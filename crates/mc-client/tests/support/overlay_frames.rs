//! One frame of a client that is still waiting for its world, with a HUD over it
//! and — when a scenario asks for one — the debug overlay's readout over that.
//!
//! # A sibling of `hud_frames`, and the separation is the point
//!
//! The overlay's lines are rasterised text, and drivers do not agree about
//! rasterised text. The moment a **committed golden** holds a glyph, whatever
//! rasterised it becomes the ground truth every other machine has to reproduce —
//! which is a reference nobody can judge and a re-shoot nobody can explain. So
//! the module that shoots this repository's goldens never learns what a readout
//! is: [`hud_frames`](super::hud_frames) hardcodes the absent one, and the
//! parameter that can carry a present one lives here instead.
//!
//! Nothing here names a golden, a golden's settings, or the environment variable
//! that mints one, and that is asserted rather than trusted —
//! `tests/overlay_over_content.rs` reads this file and the three others that may
//! name a readout, and refuses to go on if any of them could commit a frame.
//!
//! # The waiting frame rather than the prepared one
//!
//! Every scenario served here asks what reaches the frame *over* whatever was
//! drawn under it, and the client draws before its world lands through the same
//! one call it draws afterwards. So a frame that needs no meshed world is the
//! cheaper half of an identical claim — and it is the phase a client is in at the
//! moment it starts, which is the state one of those scenarios is about.

use std::error::Error;

use mc_client::startup::empty_scene;
use mc_render::camera::waiting_view;
use mc_render::gpu::{FrameRenderer, FrameSnapshot, RecordTarget};
use mc_render::hud::HudFrame;
use mc_render::overlay::OverlayReadout;
use mc_render::pass::TerrainPassConfig;
use mc_render::snapshot::{ScenePhase, TerrainSnapshot};
use mc_testkit::frame::Rgba8Image;
use mc_testkit::frame::gpu::{CaptureContext, CaptureRequest, draw_fn};

use super::frames::CAPTURE_SIZE;

/// What a capture reports when the draw work never ran at all.
const DRAW_WORK_NEVER_RAN: &str = "the capture returned a frame without ever running the draw work, so every pixel below would \
     be about a target nothing drew into";

/// A client that has opened its window and is still waiting for a world, drawing
/// through the one frame call the windowed client makes.
///
/// The renderer and the terrain snapshot are held rather than handed in per call,
/// so two frames a scenario compares cannot differ in anything but the HUD and
/// the overlay it asked for.
#[derive(Debug)]
pub struct OverlayFrames<'a> {
    context: &'a CaptureContext,
    renderer: FrameRenderer,
    snapshot: TerrainSnapshot,
}

impl<'a> OverlayFrames<'a> {
    /// A waiting client: no scene uploaded, no array texture filled, and the
    /// phase that says so.
    ///
    /// # Errors
    ///
    /// Returns the pipeline failure.
    pub fn waiting(context: &'a CaptureContext) -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            context,
            renderer: FrameRenderer::new(
                context.device(),
                context.queue(),
                &TerrainPassConfig::offscreen(),
                &super::frames::no_supplied_texels(),
            )?,
            snapshot: TerrainSnapshot {
                tick: 0,
                camera: waiting_view(),
                scene: empty_scene(),
                // A client still waiting for a world has none to stand in, and
                // this is the state the shipped client's own first frames are
                // drawn in rather than a value a fixture chose.
                tint: None,
            },
        })
    }

    /// One frame with `hud` over the waiting world and `overlay` over that,
    /// recorded through the client's own frame call and read back.
    ///
    /// `overlay` is what the client publishes for this frame: `None` is a client
    /// not showing it, which is every frame of a run nobody has asked for it on.
    ///
    /// # Errors
    ///
    /// Returns the recording failure the renderer reported, the capture failure,
    /// or the absence of any drawn frame at all.
    pub fn capture(
        &mut self,
        hud: &HudFrame,
        overlay: Option<&OverlayReadout>,
        request: &CaptureRequest,
    ) -> Result<Rgba8Image, Box<dyn Error>> {
        let context = self.context;
        let renderer = &mut self.renderer;
        let frame = FrameSnapshot {
            terrain: &self.snapshot,
            hud,
            overlay,
        };
        let mut recorded = false;
        let mut work = draw_fn(|encoder, color| {
            let target = RecordTarget {
                device: context.device(),
                queue: context.queue(),
                encoder,
                color,
                size: CAPTURE_SIZE,
            };
            renderer.record_frame(target, &ScenePhase::Preparing, &frame)?;
            recorded = true;
            Ok(())
        });
        let captured = context.capture(request, &mut work)?.image;
        drop(work);
        if !recorded {
            return Err(DRAW_WORK_NEVER_RAN.into());
        }
        Ok(captured)
    }
}
