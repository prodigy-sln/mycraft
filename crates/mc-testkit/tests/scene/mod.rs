//! The device fixture and the self-verification scene.
//!
//! Two things live here that deliberately do not live in the library. The
//! **scene** — a solid-colour pipeline and its WGSL — is caller-supplied draw
//! work: the harness hands out a canvas and never a scene, which is what keeps
//! it ignorant of the renderer it exists to verify. The library ships no
//! shaders. The **device fixture** is the acquisition every test in this suite
//! starts from, and a machine with no adapter fails here rather than skipping:
//! a green run that verified nothing is the one outcome this harness may not
//! have.
//!
//! Orientation is the caller's to get right, and this module is where that is
//! settled: **framebuffer row 0 is the top, and clip-space y is up**, so the
//! draw work that fills the top half writes y > 0. The two point in opposite
//! directions, which is where the ecosystem's flipped frames come from.

// Each test binary links this whole module and uses a subset of it.
#![allow(dead_code)]

use std::error::Error;

use mc_testkit::frame::gpu::{
    AcquireOptions, Acquisition, CaptureContext, CaptureRequest, DrawWork, draw_fn,
};
use mc_testkit::frame::{CaptureId, OptIns, validate_frame_size, wgpu};

/// The error type every test in this suite propagates with `?`.
pub type TestResult = Result<(), Box<dyn Error>>;

/// Bytes per pixel in the harness's capture format.
pub const BYTES_PER_PIXEL: usize = 4;
/// How many of those bytes are colour: every channel but alpha.
pub const COLOUR_CHANNELS: usize = 3;

/// The clear a test uses when the frame's colour is beside the point.
pub const OPAQUE_RED: wgpu::Color = wgpu::Color {
    r: 1.0,
    g: 0.0,
    b: 0.0,
    a: 1.0,
};
/// White at a quarter alpha. Straight alpha means the colour channels come back
/// unscaled; nothing premultiplies them.
pub const WHITE_AT_QUARTER_ALPHA: wgpu::Color = wgpu::Color {
    r: 1.0,
    g: 1.0,
    b: 1.0,
    a: 0.25,
};
/// The **linear** value the hardware sRGB-encodes to 128:
/// `((128 / 255 + 0.055) / 1.055) ^ 2.4`. The encode is the hardware's, not
/// this crate's, which is the whole point of rendering into an sRGB target.
pub const LINEAR_MID_GREY: wgpu::Color = wgpu::Color {
    r: 0.215_858_4,
    g: 0.215_858_4,
    b: 0.215_858_4,
    a: 1.0,
};

/// The bytes an opaque red pixel comes back as.
pub const OPAQUE_RED_BYTES: [u8; BYTES_PER_PIXEL] = [255, 0, 0, 255];
/// The bytes the top-half fill comes back as.
pub const OPAQUE_WHITE_BYTES: [u8; BYTES_PER_PIXEL] = [255, 255, 255, 255];
/// The bytes the clear under that fill comes back as.
pub const OPAQUE_BLACK_BYTES: [u8; BYTES_PER_PIXEL] = [0, 0, 0, 255];

/// The one colour format the harness captures in, and therefore the format a
/// caller's pipeline must render to. Fixed by the design so that captures are
/// comparable across runs, and the reason the sRGB encode is the hardware's.
const TARGET_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

/// The single colour target every scene here writes to. A constant rather than
/// a temporary, so its lifetime is not something a pipeline builder has to
/// reason about.
const COLOUR_TARGETS: [Option<wgpu::ColorTargetState>; 1] = [Some(wgpu::ColorTargetState {
    format: TARGET_FORMAT,
    blend: None,
    write_mask: wgpu::ColorWrites::ALL,
})];

/// Two triangles' worth of vertices, generated in the shader.
const QUAD_VERTICES: u32 = 6;

/// The self-verification scene: a full-width quad over the top half of the
/// frame, in flat white, with no vertex buffer.
///
/// Clip-space y is up and framebuffer row 0 is the top, so the quad spans
/// `y = 0.0 ..= 1.0` — the **top** half of the captured image. A shader that
/// wrote `y < 0` would fill the bottom instead, which is exactly the confusion
/// this scene exists to settle.
const TOP_HALF_WGSL: &str = r#"
@vertex
fn vertex_main(@builtin(vertex_index) vertex: u32) -> @builtin(position) vec4<f32> {
    var corners = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, 0.0),
        vec2<f32>( 1.0, 0.0),
        vec2<f32>(-1.0, 1.0),
        vec2<f32>(-1.0, 1.0),
        vec2<f32>( 1.0, 0.0),
        vec2<f32>( 1.0, 1.0),
    );
    return vec4<f32>(corners[vertex], 0.0, 1.0);
}

@fragment
fn fragment_main() -> @location(0) vec4<f32> {
    return vec4<f32>(1.0, 1.0, 1.0, 1.0);
}
"#;

/// Acquires the device this suite renders on.
///
/// `MYCRAFT_ALLOW_NO_GPU` is never set — no test in this project sets an
/// environment variable — so a machine with no adapter reaches the `Err` arm
/// and fails the run. That is the contract: absence of hardware is a red gate,
/// never a quiet skip.
///
/// # Errors
///
/// Returns the acquisition failure, or a failure of its own if the harness
/// reports a skip nobody asked for.
pub fn device_context() -> Result<Box<CaptureContext>, Box<dyn Error>> {
    match CaptureContext::acquire(&OptIns::default(), &AcquireOptions::default())? {
        Acquisition::Ready(context) => Ok(context),
        Acquisition::Skipped(notice) => Err(format!(
            "a capture may only be skipped when the opt-in asked for it, got `{}`",
            notice.message()
        )
        .into()),
    }
}

/// A request for a `width` × `height` capture on `context`'s device, carrying
/// the harness's own readback deadline.
///
/// # Errors
///
/// Returns the name failure for an invalid capture name, or the size failure
/// when the device cannot render a frame that large.
pub fn request(
    context: &CaptureContext,
    name: &str,
    width: u32,
    height: u32,
) -> Result<CaptureRequest, Box<dyn Error>> {
    let maximum = context.limits().max_texture_dimension_2d;
    let size = validate_frame_size(width, height, maximum)?;
    Ok(CaptureRequest::new(CaptureId::new(name)?, size))
}

/// Draw work that clears the whole frame to `color` and draws nothing.
///
/// The caller owns the render pass, including its load op, so a clear is a
/// scene like any other.
pub fn clear(color: wgpu::Color) -> impl DrawWork {
    draw_fn(move |encoder, target| {
        drop(encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("clear"),
            color_attachments: &[Some(attachment(target, wgpu::LoadOp::Clear(color)))],
            ..wgpu::RenderPassDescriptor::default()
        }));
        Ok(())
    })
}

/// Draw work that clears the frame to opaque black and fills its **top** half
/// with opaque white.
///
/// The one scene in this suite that can witness a row inversion in the capture
/// path: a harness that flipped rows would return the white half at the bottom,
/// and every golden this project ever commits would be upside-down in the same
/// direction — consistently, and therefore invisibly.
pub fn top_half_white_over_black(device: &wgpu::Device) -> impl DrawWork {
    let pipeline = top_half_pipeline(device);
    draw_fn(move |encoder, target| {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("top half over black"),
            color_attachments: &[Some(attachment(
                target,
                wgpu::LoadOp::Clear(wgpu::Color::BLACK),
            ))],
            ..wgpu::RenderPassDescriptor::default()
        });
        pass.set_pipeline(&pipeline);
        pass.draw(0..QUAD_VERTICES, 0..1);
        drop(pass);
        Ok(())
    })
}

/// The one colour attachment every scene here renders into.
fn attachment(
    target: &wgpu::TextureView,
    load: wgpu::LoadOp<wgpu::Color>,
) -> wgpu::RenderPassColorAttachment<'_> {
    wgpu::RenderPassColorAttachment {
        view: target,
        depth_slice: None,
        resolve_target: None,
        ops: wgpu::Operations {
            load,
            store: wgpu::StoreOp::Store,
        },
    }
}

/// The flat-white pipeline that fills the top half, built once per scene.
fn top_half_pipeline(device: &wgpu::Device) -> wgpu::RenderPipeline {
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("top half"),
        source: wgpu::ShaderSource::Wgsl(TOP_HALF_WGSL.into()),
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("top half"),
        layout: None,
        vertex: wgpu::VertexState {
            module: &module,
            entry_point: Some("vertex_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[],
        },
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &module,
            entry_point: Some("fragment_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &COLOUR_TARGETS,
        }),
        multiview_mask: None,
        cache: None,
    })
}

/// How many pixels a captured frame carries.
///
/// `integer_division` is lint-denied workspace-wide, so the count comes from
/// walking the buffer rather than dividing its length.
#[must_use]
pub fn pixel_count(bytes: &[u8]) -> usize {
    bytes.chunks_exact(BYTES_PER_PIXEL).count()
}

/// How many colour channels — alpha excluded — fall outside `tolerance` of
/// `expected`.
#[must_use]
pub fn channels_away_from(bytes: &[u8], expected: u8, tolerance: u8) -> usize {
    bytes
        .chunks_exact(BYTES_PER_PIXEL)
        .filter_map(|pixel| pixel.get(..COLOUR_CHANNELS))
        .flatten()
        .filter(|channel| channel.abs_diff(expected) > tolerance)
        .count()
}
