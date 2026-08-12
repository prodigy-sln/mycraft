//! One tick of the replay, rendered offscreen at the size the goldens are
//! declared at.
//!
//! Both phase-5 suites need the same five steps — acquire a device, build the
//! pipelines, upload the textures and the scene once, build the snapshot for a
//! tick, record the terrain pass into a capture — and they need them at
//! 1280 × 720, because every pixel the screen-space budget names is a pixel of
//! a frame that size. Written once here so that the goldens and the probes
//! cannot drift into judging two different pictures.
//!
//! **The statistics are deliberately dropped.** Phase 4 asserts what a frame
//! reports about itself; phase 5 asserts what a frame *looks like*. What is kept
//! from that pattern is the `ok_or` below: draw work that never ran leaves
//! `None`, and a suite that went on to probe a default-constructed frame would
//! be exactly the vacuous green this spec exists to remove.

use std::error::Error;
use std::sync::Arc;

use mc_render::camera::{CameraView, camera_view};
use mc_render::geometry::scene::SceneGeometry;
use mc_render::gpu::{RecordTarget, TerrainRenderer};
use mc_render::pass::TerrainPassConfig;
use mc_render::snapshot::{ScenePhase, TerrainSnapshot};
use mc_render::surface::SurfaceSize;
use mc_sim::replay::{TickIndex, pose};
use mc_testkit::frame::gpu::{
    AcquireOptions, Acquisition, CaptureContext, CaptureRequest, DrawWork, capture_and_verify,
    draw_fn,
};
use mc_testkit::frame::{
    CaptureId, GoldenOutcome, GoldenSettings, OptIns, Rgba8Image, validate_frame_size,
};

use super::PreparedScene;

/// The size every declared capture is taken at.
///
/// `spec.md`'s binding table, and the size the screen-space budget projected the
/// camera onto. A probe looking for the landmark at pixel (478, 215) is looking
/// at this frame and no other.
pub const CAPTURE_SIZE: SurfaceSize = SurfaceSize {
    width: 1280,
    height: 720,
};

/// What a capture reports when the draw work never ran at all.
const DRAW_WORK_NEVER_RAN: &str = "the capture returned a frame without ever running the draw work, so every pixel below \
     would be about a target nothing drew into";

/// The device these frames are drawn on, or `None` when the opt-in permitted its
/// absence.
///
/// # Errors
///
/// Returns the acquisition failure when no adapter answered and the opt-in did
/// not permit saying so.
pub fn device() -> Result<Option<Box<CaptureContext>>, Box<dyn Error>> {
    match CaptureContext::acquire(&OptIns::from_environment(), &AcquireOptions::default())? {
        Acquisition::Ready(context) => Ok(Some(context)),
        Acquisition::Skipped(_) => Ok(None),
    }
}

/// A renderer with the replay's array texture and scene already uploaded.
///
/// # Errors
///
/// Returns the pipeline or upload failure.
pub fn prepared_renderer(
    context: &CaptureContext,
    prepared: &PreparedScene,
) -> Result<TerrainRenderer, Box<dyn Error>> {
    let mut renderer = TerrainRenderer::new(
        context.device(),
        context.queue(),
        &TerrainPassConfig::offscreen(),
    )?;
    renderer.upload_textures(context.queue(), &prepared.layers)?;
    renderer.upload_scene(context.queue(), &prepared.scene)?;
    Ok(renderer)
}

/// A request named `name`, at the declared capture size.
///
/// # Errors
///
/// Returns the name failure for an invalid capture name, or the size failure
/// when the device cannot render a frame that large.
pub fn request(context: &CaptureContext, name: &str) -> Result<CaptureRequest, Box<dyn Error>> {
    let maximum = context.limits().max_texture_dimension_2d;
    let size = validate_frame_size(CAPTURE_SIZE.width, CAPTURE_SIZE.height, maximum)?;
    Ok(CaptureRequest::new(CaptureId::new(name)?, size))
}

/// Where the replay's camera stands at `tick`.
///
/// # Errors
///
/// Returns the refusal when `tick` is past the replay's length.
pub fn replay_camera(tick: u32) -> Result<CameraView, Box<dyn Error>> {
    let pose = pose(TickIndex::new(tick)?);
    Ok(camera_view(pose.eye, pose.target))
}

/// The snapshot the client hands the renderer for `tick`.
#[must_use]
pub fn snapshot(tick: u32, camera: CameraView, scene: &Arc<SceneGeometry>) -> TerrainSnapshot {
    TerrainSnapshot {
        tick,
        camera,
        scene: Arc::clone(scene),
    }
}

/// One frame of the replay, waiting to be drawn.
#[derive(Debug)]
pub struct ReplayFrame<'a> {
    pub context: &'a CaptureContext,
    pub renderer: &'a mut TerrainRenderer,
    pub snapshot: &'a TerrainSnapshot,
}

impl ReplayFrame<'_> {
    /// Draws the frame and reads its pixels back.
    ///
    /// # Errors
    ///
    /// Returns the recording failure the renderer reported, the capture
    /// failure, or the absence of any drawn frame at all.
    pub fn capture(&mut self, request: &CaptureRequest) -> Result<Rgba8Image, Box<dyn Error>> {
        self.drawn(|context, work| Ok(context.capture(request, work)?.image))
    }

    /// Draws the frame and judges it against the golden `settings` names.
    ///
    /// # Errors
    ///
    /// Returns the recording failure the renderer reported, the capture
    /// failure, or the absence of any drawn frame at all. A frame that did not
    /// match its golden is a [`GoldenOutcome`], not an error.
    pub fn verify(
        &mut self,
        request: &CaptureRequest,
        settings: &GoldenSettings,
    ) -> Result<GoldenOutcome, Box<dyn Error>> {
        self.drawn(|context, work| Ok(capture_and_verify(context, request, work, settings)?))
    }

    /// Records the terrain pass into draw work and hands it to `run`.
    ///
    /// The `ran` flag is the load-bearing part: a capture that returned without
    /// ever invoking the draw work would otherwise hand back a target nothing
    /// wrote into, and every probe below would then be measuring an empty
    /// texture rather than a rendered frame.
    fn drawn<T>(
        &mut self,
        run: impl FnOnce(&CaptureContext, &mut dyn DrawWork) -> Result<T, Box<dyn Error>>,
    ) -> Result<T, Box<dyn Error>> {
        let context = self.context;
        let snapshot = self.snapshot;
        let renderer = &mut *self.renderer;
        let phase = ScenePhase::Ready(Arc::clone(&snapshot.scene));
        let mut ran = false;
        let mut work = draw_fn(|encoder, color| {
            let target = RecordTarget {
                device: context.device(),
                queue: context.queue(),
                encoder,
                color,
                size: CAPTURE_SIZE,
            };
            renderer.record_terrain(target, &phase, snapshot)?;
            ran = true;
            Ok(())
        });
        let produced = run(context, &mut work)?;
        drop(work);
        if !ran {
            return Err(DRAW_WORK_NEVER_RAN.into());
        }
        Ok(produced)
    }
}
