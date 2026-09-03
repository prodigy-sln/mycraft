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

use crate::notice::Notices;
use crate::session::reload::Remeshing;

use super::{App, WORKER_GONE};

#[cfg(test)]
#[path = "report_test.rs"]
mod tests;

/// A line said once, however many times the fault that produces it recurs.
///
/// **A value rather than a field and a comparison at each site**, which is what
/// makes the dedup assertable: the three reporters below and the reload's own all
/// live behind a window nothing in this workspace constructs, so "said once" was
/// a property nothing could ask about. This is a plain value a test drives
/// directly.
///
/// It compares the composed *line* rather than the fault it came from. Same
/// fault, same line — and a fault that renders differently is a different thing
/// to tell somebody about.
#[derive(Debug, Default)]
pub(crate) struct SaidOnce {
    last: Option<String>,
}

impl SaidOnce {
    /// Writes `line` to `notices` unless it is exactly what was written last.
    pub(crate) fn say(&mut self, notices: &Notices, line: &str) {
        if self.last.as_deref() != Some(line) {
            notices.say(line);
            self.last = Some(line.to_owned());
        }
    }
}

/// What a player is told about `collected`, or `None` where there is nothing to
/// tell them.
///
/// **A function of the verdict rather than a `match` inside the frame path**, and
/// that is the whole of what PRO-949 reported missing. `Collecting::WorkerGone`
/// has existed since PRO-918 and `App::exchange_remesh` has reported it — inside
/// a redraw, needing a graphics device and a display server, so the sentence a
/// player reads when meshing stops was asserted by nothing at all. The arms in
/// test support proved the variant existed, never that anything reached it.
///
/// Three of the five say nothing, and two of those three happen on most frames of
/// an ordinary run.
pub(crate) fn said_about(collected: &Remeshing) -> Option<String> {
    match collected {
        Remeshing::NothingYet | Remeshing::Show(_) | Remeshing::Discarded => None,
        Remeshing::Report(failure) => Some(unshowable_edit(&rendered(failure))),
        Remeshing::WorkerGone => Some(unshowable_edit(WORKER_GONE)),
    }
}

/// How an edit that could not be shown reads, whatever stopped it.
fn unshowable_edit(reason: &str) -> String {
    format!("mycraft: an edit could not be shown: {reason}")
}

impl App {
    /// States a frame failure once, however many frames it goes on to affect.
    pub(super) fn report(&mut self, failure: FrameError) {
        let said = format!("mycraft: a frame was dropped: {}", rendered(&failure));
        self.reported.say(&self.notices, &said);
    }

    /// States a held block that draws no indicator once, however many frames go
    /// on to hold it.
    ///
    /// It never ends the run. A HUD element that cannot be drawn is the rest of
    /// the game still being playable, which is the same trade a failed re-mesh
    /// makes.
    pub(super) fn report_swatch(&mut self, report: &str) {
        let said = format!("mycraft: {report}");
        self.reported_swatch.say(&self.notices, &said);
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
    pub(super) fn report_remesh(&mut self, said: &str) {
        self.reported_remesh.say(&self.notices, said);
    }

    /// Tells the player whatever `collected` is worth telling them, once.
    ///
    /// **The words are [`said_about`]'s and never this path's**, which is what
    /// makes them assertable: a decision taken inside a redraw is a decision
    /// behind a device nothing in this workspace constructs.
    pub(super) fn say_about(&mut self, collected: &Remeshing) {
        if let Some(said) = said_about(collected) {
            self.report_remesh(&said);
        }
    }
}
