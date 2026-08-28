//! Recording one frame: the compute cull pass, and the two indirect draws.
//!
//! The order is the whole design. The CPU writes the frame's camera and its six
//! frustum planes into a uniform and returns the indirect arguments to their
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

use crate::camera::{Frustum, Plane, projection_for, view_projection};
use crate::snapshot::{FrameStats, FrameWork, ScenePhase, TerrainSnapshot, frame_work};
use crate::surface::SurfaceSize;

use super::buffers::{DRAW_ARGS_BYTES, SceneBuffers};
use super::pipeline::Pipelines;
use super::{FrameError, RecordTarget, TerrainRenderer, clear_color};

/// Everything one frame is recorded *from*.
#[derive(Debug)]
pub(super) struct Frame<'a> {
    pub(super) phase: &'a ScenePhase,
    pub(super) snapshot: &'a TerrainSnapshot,
}

/// How many draw calls a frame that draws no terrain issues.
const NO_DRAW: u32 = 0;

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

    if let Some(count) = sections {
        upload_frame(&renderer.buffers, target.queue, frame.snapshot, target.size);
        renderer.buffers.reset_draw(target.queue);
        cull(&renderer.pipelines, &mut target.encoder, count);
    }
    draw(renderer, target, sections.is_some())?;

    Ok(FrameStats {
        terrain_draw_calls: if sections.is_some() {
            predicted.terrain_draw_calls
        } else {
            NO_DRAW
        },
        ..predicted
    })
}

/// Writes the frame's view-projection matrix and its six frustum planes.
///
/// The planes come from the same extraction the statistics' prediction uses, so
/// the shader and the pure function test one set of six numbers rather than two
/// that happen to agree today.
fn upload_frame(
    buffers: &SceneBuffers,
    queue: &wgpu::Queue,
    snapshot: &TerrainSnapshot,
    size: SurfaceSize,
) {
    let matrix = view_projection(&snapshot.camera, &projection_for(size));
    let frustum = Frustum::from_view_projection(&matrix);
    queue.write_buffer(&buffers.frame, 0, &frame_uniform_bytes(&matrix, &frustum));
}

/// The uniform's bytes: the matrix in column-major order, then the six planes as
/// `(normal, offset)` — unnormalised, exactly as extracted.
fn frame_uniform_bytes(matrix: &Mat4, frustum: &Frustum) -> Vec<u8> {
    let mut bytes: Vec<u8> = matrix
        .to_cols_array()
        .iter()
        .flat_map(|value| value.to_le_bytes())
        .collect();
    for plane in frustum.planes() {
        bytes.extend(plane_bytes(plane));
    }
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
fn draw(
    renderer: &mut TerrainRenderer,
    target: RecordTarget<'_>,
    terrain: bool,
) -> Result<(), FrameError> {
    let config = renderer.config;
    let depth = renderer
        .depth
        .view_for(target.device, target.size, &config)?;
    let attachments = [Some(color_attachment(target.color, clear_color(&config)))];
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
