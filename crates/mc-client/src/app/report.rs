//! Saying a recurring fault once: the three reporters the frame path reaches
//! for, and the one policy behind all three.
//!
//! Their shared responsibility is one sentence: **say a recurring fault once,
//! however many frames repeat it, and never end the run for it.** None of them
//! touches `wgpu`, a [`Session`](crate::session::Session) or a snapshot; each
//! writes one line to stderr and dedups against its own last message. A child
//! module rather than a sibling because it writes the fields `App` owns, which
//! is the rule `app/reload.rs` already states.
//!
//! **Three reporters and not one, because a third recurring fault must not
//! silence the other two.** A dropped frame, an edit that could not be shown and
//! a held block that occupies no layer are three faults, and folding them into
//! one dedup would let whichever recurs fastest hide the others for the rest of
//! the run. They also have nothing to say to each other: "an edit could not be
//! shown" is the wrong sentence for a block whose texture occupies no layer, and
//! the one message a content author reads has to be about what is wrong.
//!
//! [`report_reload`](super::App) stays in `app/reload.rs` and is not gathered
//! here. That module's stated contract is `App`'s *whole* share of a reload, and
//! pulling its reporter out would break a contract to satisfy a symmetry.

use mc_render::gpu::FrameError;
use mc_render::window::rendered;

use super::App;

impl App {
    /// States a frame failure once, however many frames it goes on to affect.
    pub(super) fn report(&mut self, failure: FrameError) {
        if self.reported != Some(failure) {
            eprintln!("mycraft: a frame was dropped: {}", rendered(&failure));
            self.reported = Some(failure);
        }
    }

    /// States a held block that draws no indicator once, however many frames go
    /// on to hold it.
    ///
    /// It never ends the run. A HUD element that cannot be drawn is the rest of
    /// the game still being playable, which is the same trade a failed re-mesh
    /// makes.
    pub(super) fn report_swatch(&mut self, report: &str) {
        if self.reported_swatch.as_deref() != Some(report) {
            eprintln!("mycraft: {report}");
            self.reported_swatch = Some(report.to_owned());
        }
    }

    /// States an edit that could not be shown once, however many edits go on to
    /// meet the same fault.
    ///
    /// **It never ends the run**, which is the opposite of what a failed
    /// preparation does and deliberately so: preparation has no previous picture
    /// to fall back on, and a re-mesh has the one it drew a moment ago.
    /// `reason` is text the renderer already produced, not a failure: both
    /// callers hand it `rendered(..)`. Naming it after the value it carries
    /// rather than after the value it came from is what keeps it out of the
    /// guard that watches for an unrendered failure being interpolated.
    pub(super) fn report_remesh(&mut self, reason: &str) {
        if self.reported_remesh.as_deref() != Some(reason) {
            eprintln!("mycraft: an edit could not be shown: {reason}");
            self.reported_remesh = Some(reason.to_owned());
        }
    }
}
