//! Recording one frame: the compute cull pass, and the two indirect draws.
//!
//! The order is the whole design. The CPU writes the frame's camera, its six
//! frustum planes and what the medium at the eye does to the light into a
//! uniform and returns the indirect arguments to their
//! declared state; the compute pass runs one workgroup per section, flags the
//! ones the frustum admits and compacts their indices into the two halves of the
//! index buffer while raising each half's index count with an atomic add; the
//! render pass then issues **one `draw_indexed_indirect` per half**, opaque
//! first, over whatever those counts came to. Nothing between those steps is a
//! decision — which is why the visible set can be compared against a pure
//! function that never saw a device.
//!
//! While the scene is still being prepared there is nothing to draw and the
//! frame is a bare clear. A surface texture that was acquired and left unwritten
//! shows whatever the driver last had in it, which reads as a crash rather than
//! as waiting.

use glam::Mat4;
use mc_core::block::MediumTint;

use crate::camera::{Frustum, Plane, projection_for, view_projection};
use crate::color::srgb8_to_linear;
use crate::snapshot::{FrameStats, FrameWork, ScenePhase, TerrainSnapshot, frame_work};

use super::buffers::{DRAW_ARGS_BYTES, SceneBuffers};
use super::pipeline::Pipelines;
use super::{FrameError, RecordTarget, TerrainRenderer, clear_color, opaque};

/// Everything one frame is recorded *from*.
#[derive(Debug)]
pub(super) struct Frame<'a> {
    pub(super) phase: &'a ScenePhase,
    pub(super) snapshot: &'a TerrainSnapshot,
}

/// How many draw calls a frame that draws no terrain issues.
const NO_DRAW: u32 = 0;

/// What a frame carries where the eye stands in nothing that tints.
///
/// **The literal, never the reciprocal of a sentinel distance.** `mix(a, b, 0)`
/// returns `a` bit for bit under every form a backend compiles it into, so a dry
/// frame is the tinted arithmetic with a factor of nought and not a second code
/// path — which is what lets a capture taken before any medium was declared be
/// compared against one taken after, byte for byte.
const NO_TINT: f32 = 0.0;

/// The colour a frame carries alongside [`NO_TINT`], and the four declared bytes
/// the record ends on. Both are read by nothing; both are written so the buffer
/// holds no leftover.
const NO_COLOUR: [f32; 3] = [0.0; 3];
const DECLARED_PADDING: u32 = 0;

/// The frame's medium, decoded once.
///
/// **One decode feeds the uniform and the clear.** A surface at or beyond the
/// medium's reach is drawn wholly at its colour by the shader's mix, and a pixel
/// no surface reaches is drawn at that colour by the clear; the two are the same
/// colour by two routes, and a second call to the transfer function is a second
/// place for them to part. `rendering.md` records that failure at this exact
/// site, where a unit test of the conversion and a test comparing two
/// configurations to each other were both green while every frame shipped wrong.
#[derive(Debug, Clone, Copy)]
struct Tinting {
    /// `1 / D`, so the shader multiplies where it would otherwise divide.
    reach: f32,
    linear: [f64; 3],
}

impl Tinting {
    fn of(tint: MediumTint) -> Self {
        Self {
            reach: 1.0 / tint.distance(),
            linear: srgb8_to_linear(tint.color()),
        }
    }

    /// The colour the shader reads, narrowed from the one the clear reads.
    fn narrowed(self) -> [f32; 3] {
        self.linear.map(|channel| channel as f32)
    }
}

/// Records one frame into `target`.
///
/// # Errors
///
/// Returns [`FrameError::DepthAllocation`] for a frame with no area, and
/// [`FrameError::Recording`] when terrain was asked for before any scene was
/// uploaded.
pub(super) fn record(
    renderer: &mut TerrainRenderer,
    mut target: RecordTarget<'_>,
    frame: Frame<'_>,
) -> Result<FrameStats, FrameError> {
    let predicted = TerrainRenderer::stats_for(frame.snapshot, target.size);
    let sections = match frame_work(frame.phase) {
        FrameWork::ClearOnly => None,
        FrameWork::Terrain => Some(renderer.sections.ok_or(FrameError::Recording {
            stage: "scene upload",
        })?),
    };

    let tinting = frame.snapshot.tint.map(Tinting::of);
    if let Some(count) = sections {
        upload_frame(&renderer.buffers, &target, frame.snapshot, tinting);
        renderer.buffers.reset_draw(target.queue);
        cull(&renderer.pipelines, &mut target.encoder, count);
    }
    let clear = tinting.map_or_else(
        || clear_color(&renderer.config),
        |medium| opaque(medium.linear),
    );
    draw(renderer, target, sections.is_some(), clear)?;

    Ok(FrameStats {
        terrain_draw_calls: if sections.is_some() {
            predicted.terrain_draw_calls
        } else {
            NO_DRAW
        },
        ..predicted
    })
}

/// Writes the frame's view-projection matrix, its six frustum planes, the eye
/// they were derived from, and what the medium at that eye does to the light.
///
/// The planes come from the same extraction the statistics' prediction uses, so
/// the shader and the pure function test one set of six numbers rather than two
/// that happen to agree today.
fn upload_frame(
    buffers: &SceneBuffers,
    target: &RecordTarget<'_>,
    snapshot: &TerrainSnapshot,
    tinting: Option<Tinting>,
) {
    let matrix = view_projection(&snapshot.camera, &projection_for(target.size));
    let frustum = Frustum::from_view_projection(&matrix);
    let bytes = frame_uniform_bytes(&matrix, &frustum, snapshot.camera.eye.to_array(), tinting);
    target.queue.write_buffer(&buffers.frame, 0, &bytes);
}

/// The uniform's bytes: the matrix in column-major order, the six planes as
/// `(normal, offset)` — unnormalised, exactly as extracted — then the eye, the
/// reciprocal of the medium's reach, its colour in linear light, and the four
/// bytes of padding the record declares.
///
/// **The eye is written whether or not anything tints.** It costs twelve bytes
/// and it keeps the record one shape, where a frame that wrote it only sometimes
/// would leave the fragment stage reading a stale eye out of a buffer nobody
/// rewrote.
fn frame_uniform_bytes(
    matrix: &Mat4,
    frustum: &Frustum,
    eye: [f32; 3],
    tinting: Option<Tinting>,
) -> Vec<u8> {
    let mut bytes: Vec<u8> = matrix
        .to_cols_array()
        .iter()
        .flat_map(|value| value.to_le_bytes())
        .collect();
    for plane in frustum.planes() {
        bytes.extend(plane_bytes(plane));
    }
    bytes.extend(eye.into_iter().flat_map(f32::to_le_bytes));
    let (reach, color) = tinting.map_or((NO_TINT, NO_COLOUR), |medium| {
        (medium.reach, medium.narrowed())
    });
    bytes.extend(reach.to_le_bytes());
    bytes.extend(color.into_iter().flat_map(f32::to_le_bytes));
    bytes.extend(DECLARED_PADDING.to_le_bytes());
    bytes
}

/// One plane as four little-endian floats.
fn plane_bytes(plane: &Plane) -> impl Iterator<Item = u8> {
    [plane.normal.x, plane.normal.y, plane.normal.z, plane.offset]
        .into_iter()
        .flat_map(f32::to_le_bytes)
}

/// Records the compute pass: one workgroup per section, sixty-four lanes each.
fn cull(pipelines: &Pipelines, encoder: &mut &mut wgpu::CommandEncoder, sections: u32) {
    if sections == 0 {
        return;
    }
    let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
        label: Some("mycraft terrain cull"),
        timestamp_writes: None,
    });
    pass.set_pipeline(&pipelines.cull);
    pass.set_bind_group(0, &pipelines.cull_group, &[]);
    pass.dispatch_workgroups(sections, 1, 1);
}

/// Records the render pass: the clear, and — when there is a scene — the one
/// indirect terrain draw.
///
/// `clear` is the frame's own and not the configuration's: a pixel no surface
/// reaches is given the medium's colour here, which is what makes the far field
/// seamless against the mix. [`TerrainPassConfig::clear_color_linear`] goes on
/// meaning the dry sky and is what a frame with no medium falls back to.
fn draw(
    renderer: &mut TerrainRenderer,
    target: RecordTarget<'_>,
    terrain: bool,
    clear: wgpu::Color,
) -> Result<(), FrameError> {
    let config = renderer.config;
    let depth = renderer
        .depth
        .view_for(target.device, target.size, &config)?;
    let attachments = [Some(color_attachment(target.color, clear))];
    let mut pass = target
        .encoder
        .begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("mycraft terrain"),
            color_attachments: &attachments,
            depth_stencil_attachment: Some(depth_attachment(depth)),
            ..wgpu::RenderPassDescriptor::default()
        });
    if terrain {
        record_terrain_draw(&mut pass, &renderer.pipelines, &renderer.buffers);
    }
    Ok(())
}

/// The two terrain draws: bind once, point at the compacted indices, and let the
/// device read how many of them each half holds.
///
/// **Opaque first, blended second, in the same render pass.** Rasterization
/// order between two draws of one pass is what makes "first" mean what it says,
/// so the blended draw reads the depth the opaque one wrote and a translucent
/// face behind an opaque one is discarded rather than mixed into it. Two passes
/// would need the depth attachment loaded rather than cleared and would say the
/// same thing less directly.
///
/// Both draws read the same vertex buffer and the same index buffer; what
/// separates them is where in that buffer each half begins, which the arguments
/// carry. A frame in which nothing declares a degree below one leaves the second
/// draw's index count at zero, and a draw of zero indices is not a special case
/// anybody has to write down.
fn record_terrain_draw(
    pass: &mut wgpu::RenderPass<'_>,
    pipelines: &Pipelines,
    buffers: &SceneBuffers,
) {
    pass.set_bind_group(0, &pipelines.frame_group, &[]);
    pass.set_bind_group(1, &pipelines.texture_group, &[]);
    pass.set_vertex_buffer(0, buffers.vertices.slice(..));
    pass.set_index_buffer(buffers.indices.slice(..), wgpu::IndexFormat::Uint32);
    pass.set_pipeline(&pipelines.terrain);
    pass.draw_indexed_indirect(&buffers.args, 0);
    pass.set_pipeline(&pipelines.blended_terrain);
    pass.draw_indexed_indirect(&buffers.args, DRAW_ARGS_BYTES);
}

/// The single colour attachment, cleared to the declared colour.
const fn color_attachment(
    view: &wgpu::TextureView,
    color: wgpu::Color,
) -> wgpu::RenderPassColorAttachment<'_> {
    wgpu::RenderPassColorAttachment {
        view,
        depth_slice: None,
        resolve_target: None,
        ops: wgpu::Operations {
            load: wgpu::LoadOp::Clear(color),
            store: wgpu::StoreOp::Store,
        },
    }
}

/// The depth attachment, cleared to the far plane.
///
/// `Less` keeps the first fragment to arrive at a pixel and every nearer one
/// after it, which is what makes the order sections were compacted in — and
/// therefore the order they are drawn in — irrelevant to the picture.
const fn depth_attachment(view: &wgpu::TextureView) -> wgpu::RenderPassDepthStencilAttachment<'_> {
    wgpu::RenderPassDepthStencilAttachment {
        view,
        depth_ops: Some(wgpu::Operations {
            load: wgpu::LoadOp::Clear(1.0),
            store: wgpu::StoreOp::Store,
        }),
        stencil_ops: None,
    }
}
