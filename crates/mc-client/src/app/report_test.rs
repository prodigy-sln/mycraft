//! Saying a recurring fault once, asserted with no device in reach.
//!
//! The three reporters this belongs to sit behind a `wgpu::Surface` and a
//! `winit::Window` that nothing in this workspace constructs, so "said once,
//! however many frames repeat it" was a property nothing could ask about — the
//! same shape `notice_test.rs` exists to answer for the clearing sentences.
//! Pulling the dedup out as a value is what makes it a question.
//!
//! **Both halves matter and each is the other's control.** A reporter that says
//! nothing after the first line satisfies "a repeat is said once" and is useless;
//! a reporter that says everything satisfies "a changed line is said again" and
//! fills the terminal. The readings below are one of each, over one recorder.

use std::sync::Arc;

use mc_render::geometry::scene::SceneGeometry;

use super::{SaidOnce, said_about};
use crate::notice::recording::Recorder;
use crate::session::reload::Remeshing;

/// Two faults a run can meet, and the lines they compose to.
const A_DROPPED_FRAME: &str = "mycraft: a frame was dropped: the surface was lost";
const AN_UNSHOWABLE_EDIT: &str = "mycraft: an edit could not be shown: the worker is gone";

#[test]
fn a_fault_that_recurs_every_frame_is_said_once() {
    let (recorder, notices) = Recorder::listening();
    let mut said = SaidOnce::default();

    said.say(&notices, A_DROPPED_FRAME);
    said.say(&notices, A_DROPPED_FRAME);
    said.say(&notices, A_DROPPED_FRAME);

    assert_eq!(
        recorder.said(),
        format!("{A_DROPPED_FRAME}\n"),
        "a dropped frame recurs for as long as whatever dropped it lasts, which on a lost surface \
         is every frame until the run ends. Said each time it would be the only thing on the \
         terminal, and the line a player actually needed would be somewhere in the scrollback"
    );
}

#[test]
fn a_fault_that_changes_is_said_again() {
    let (recorder, notices) = Recorder::listening();
    let mut said = SaidOnce::default();

    said.say(&notices, A_DROPPED_FRAME);
    said.say(&notices, AN_UNSHOWABLE_EDIT);
    said.say(&notices, A_DROPPED_FRAME);

    assert_eq!(
        recorder.said(),
        format!("{A_DROPPED_FRAME}\n{AN_UNSHOWABLE_EDIT}\n{A_DROPPED_FRAME}\n"),
        "the dedup is against the *last* line and not against everything ever said, so a run that \
         alternates between two faults reports both every time it changes. Remembering all of them \
         would mean a fault that came back after being fixed was never mentioned again, which is \
         the more interesting event of the two"
    );
}

/// The whole of what a player reads when the worker that draws their edits has
/// stopped.
///
/// Written out rather than composed from the client's own constants, on
/// `notice_test.rs`'s rule: these are the words on a terminal, and a reading that
/// assembled them the way the client does would agree with it about a rewording
/// neither had noticed.
const EDITS_WILL_NOT_BE_SHOWN: &str = "mycraft: an edit could not be shown: the worker that draws \
                                       your edits has stopped; edits will not be shown for the \
                                       rest of this run";

#[test]
fn a_worker_that_has_stopped_tells_the_player_their_edits_will_not_be_shown() {
    assert_eq!(
        said_about(&Remeshing::WorkerGone).as_deref(),
        Some(EDITS_WILL_NOT_BE_SHOWN),
        "a world that silently stops showing what a player breaks is the worst outcome this path \
         has: they go on digging, nothing changes on screen, and nothing anywhere says why. The \
         sentence has to name both halves — that meshing has stopped, and that it will not come \
         back this run — because the first alone reads as something to wait out"
    );
}

/// The control: the three verdicts that are not a fault say nothing at all.
///
/// **A frame path that reported every collect would say something on every frame
/// it drew**, which is the ordinary case: a worker with nothing finished answers
/// `NothingYet` for as long as it is meshing. The `WorkerGone` reading above
/// passes perfectly well against an implementation that reports all five.
#[test]
fn a_collect_that_is_not_a_fault_tells_the_player_nothing() {
    let drawn = Remeshing::Show(Arc::new(SceneGeometry::default()));

    assert_eq!(
        [
            said_about(&Remeshing::NothingYet),
            said_about(&drawn),
            said_about(&Remeshing::Discarded),
        ],
        [None, None, None],
        "a worker still meshing, a scene to upload and a batch whose content stopped serving are \
         the three ordinary answers, and two of them happen on most frames of a run. A line for \
         any of them is a line on every frame, which is how the one that matters stops being read"
    );
}
