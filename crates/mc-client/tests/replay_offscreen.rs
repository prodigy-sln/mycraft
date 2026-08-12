//! The replay, rendered: the first point at which the world, the mesher, the
//! packing and the draw path are asked to work as one thing.
//!
//! # Why this suite is here and not beside the renderer
//!
//! It needs the world `mc-sim` generates and the terrain pass `mc-render`
//! records, and neither of those two crates may resolve the other in **any**
//! dependency kind — the seam test walks dev-dependencies too. The composition
//! root is the only crate that resolves both, so a test that needs both belongs
//! here. The lines it exercises live in the renderer and are counted there.
//!
//! # What "drew terrain" is measured by
//!
//! Not by `sections_admitted`. That figure is a *prediction* the pure frustum
//! function makes, and a renderer that drew nothing at all would still report it
//! — which is exactly the vacuous green this spec exists to remove. The
//! observation is the index count the compute pass compacted into the indirect
//! arguments, read back from the device. Both are asserted, and it is the second
//! one that makes the first mean something.

mod support;

use std::error::Error;
use std::sync::Arc;

use mc_render::camera::camera_view;
use mc_render::geometry::scene::SceneGeometry;
use mc_render::gpu::{RecordTarget, TerrainRenderer};
use mc_render::pass::TerrainPassConfig;
use mc_render::snapshot::{ScenePhase, TerrainSnapshot};
use mc_render::surface::SurfaceSize;
use mc_sim::replay::{TickIndex, pose};
use mc_testkit::frame::gpu::{
    AcquireOptions, Acquisition, CaptureContext, CaptureRequest, draw_fn,
};
use mc_testkit::frame::{CaptureId, OptIns, validate_frame_size};

use support::{TestResult, prepare_scene};

/// The ticks the client advances through before anything else happens.
const TICKS: [u32; 3] = [0, 1, 2];

/// The frame each of them is drawn into. 16:9, the declared aspect, at a
/// fraction of the declared pixels: nothing here reads a pixel.
const FRAME: SurfaceSize = SurfaceSize {
    width: 128,
    height: 72,
};

/// What a capture reports when the draw work never ran at all.
const DRAW_WORK_NEVER_RAN: &str = "the capture returned a frame without ever running the draw work, so there are no frame \
     statistics and the assertion below would be about a default value";

#[test]
fn each_of_the_replays_first_three_ticks_draws_sections() -> TestResult {
    let prepared = prepare_scene()?;
    let Some(context) = device()? else {
        return Ok(());
    };
    let scene = Arc::new(prepared.scene);
    let mut renderer = TerrainRenderer::new(
        context.device(),
        context.queue(),
        &TerrainPassConfig::offscreen(),
    )?;
    renderer.upload_textures(context.queue(), &prepared.layers)?;
    renderer.upload_scene(context.queue(), &scene)?;

    let mut drawn = Vec::new();
    for tick in TICKS {
        drawn.push(render_tick(&context, &mut renderer, &scene, tick)?);
    }

    assert!(
        drawn.len() == TICKS.len()
            && drawn
                .iter()
                .all(|(admitted, indices)| *admitted > 0 && *indices > 0),
        "each of the replay's first three frames has to draw sections, and the number that says \
         so is the index count the GPU compacted rather than the prediction that produced it: \
         (sections admitted, indices compacted) = {drawn:?}"
    );
    Ok(())
}

/// The device the frames are drawn on, or `None` when the opt-in permitted its
/// absence.
fn device() -> Result<Option<Box<CaptureContext>>, Box<dyn Error>> {
    match CaptureContext::acquire(&OptIns::from_environment(), &AcquireOptions::default())? {
        Acquisition::Ready(context) => Ok(Some(context)),
        Acquisition::Skipped(_) => Ok(None),
    }
}

/// Renders `scene` from the replay's pose at `tick`, and reports how many
/// sections the frame predicted and how many indices it actually compacted.
///
/// The `ok_or` at the end is load-bearing: draw work that never ran leaves
/// `None`, and proceeding on a default `FrameStats` is the shape this whole
/// suite exists to catch.
fn render_tick(
    context: &CaptureContext,
    renderer: &mut TerrainRenderer,
    scene: &Arc<SceneGeometry>,
    tick: u32,
) -> Result<(u32, u32), Box<dyn Error>> {
    let camera = pose(TickIndex::new(tick)?);
    let snapshot = TerrainSnapshot {
        tick,
        camera: camera_view(camera.eye, camera.target),
        scene: Arc::clone(scene),
    };
    let phase = ScenePhase::Ready(Arc::clone(scene));
    let request = request(context, tick)?;

    let mut stats = None;
    {
        let mut work = draw_fn(|encoder, color| {
            let target = RecordTarget {
                device: context.device(),
                queue: context.queue(),
                encoder,
                color,
                size: FRAME,
            };
            stats = Some(renderer.record_terrain(target, &phase, &snapshot)?);
            Ok(())
        });
        context.capture(&request, &mut work)?;
    }
    let stats = stats.ok_or(DRAW_WORK_NEVER_RAN)?;

    let indices = renderer.read_drawn_index_count(context.device(), context.queue())?;
    Ok((stats.sections_admitted, indices))
}

/// A capture request for `tick`'s frame.
fn request(context: &CaptureContext, tick: u32) -> Result<CaptureRequest, Box<dyn Error>> {
    let maximum = context.limits().max_texture_dimension_2d;
    let size = validate_frame_size(FRAME.width, FRAME.height, maximum)?;
    Ok(CaptureRequest::new(
        CaptureId::new(&format!("replay-offscreen-t{tick:03}"))?,
        size,
    ))
}
