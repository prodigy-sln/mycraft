//! The event loop's vocabulary: what a window event asks the client to do, what
//! tick follows the one just drawn, and how the client's run ends.
//!
//! `winit` is not named here and must not be. The client's `events.rs`
//! translates a `winit::event::WindowEvent` into a [`WindowEventKind`] and
//! nothing else in the workspace ever sees the original — which is what lets
//! every decision the loop makes be a pure function tested without a window, a
//! compositor or a display server.
//!
//! **Cursor capture is a ladder here and an acceptance elsewhere.** Whether the
//! operating system actually took the pointer cannot be asked without a window,
//! a compositor and a person looking at the screen, and it is recorded as manual
//! acceptance in `docs/technical/testing.md`. What is decidable is the policy —
//! which capture is asked for first, what each refusal falls back to, that the
//! bottom rung is a state the game carries on in, where Escape leaves each of
//! the three, and where a click leaves them — and all five functions are total
//! and infallible, so "SHALL NOT exit" is a property of their types rather than
//! something a client has to remember not to do.
//!
//! **Escape and a click are a pair and only make sense as one.** A release with
//! nothing that re-acquires is a game that ends looking around at the first
//! keypress, with every scenario about the release still green.
//!
//! **The replay's length is a parameter, never a constant read here.**
//! `mc_sim::replay::SCRIPT_TICKS` is where it is declared and this crate may not resolve
//! `mc-sim` in any dependency kind, so the wrap is a function of the count it is
//! handed. That also makes it testable without the simulation existing.

use crate::surface::{FatalReason, StartupError, SurfaceSize};

/// What the window told the client, in the renderer's own vocabulary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowEventKind {
    CloseRequested,
    Resized(SurfaceSize),
    RedrawRequested,
    FocusLost,
    Other,
}

/// What the event loop does about it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoopAction {
    Exit,
    Redraw,
    Resize(SurfaceSize),
    ClearInput,
    Ignore,
}

/// How firmly the pointer is held, or is being asked to be held.
///
/// Three states rather than a boolean because the two captured ones are not
/// interchangeable: a locked pointer is warped back to the window's centre and
/// reports motion with no position at all, and a confined one keeps a position
/// that runs out at the window's edge. What the client does with a pointer that
/// is merely confined is not this module's business; which one it asks for, and
/// what it does when it cannot have it, is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureState {
    Locked,
    Confined,
    Uncaptured,
}

/// The capture the client asks for before anything has refused it.
///
/// The head of the ladder [`next_capture_attempt`] walks down, and a function
/// rather than a constant so that the client reads its first attempt out of the
/// same policy as every later one — a client spelling the first attempt inline
/// would be the decision moving into the adapter.
#[must_use]
pub const fn first_capture_attempt() -> CaptureState {
    CaptureState::Locked
}

/// What to ask for after `refused` was refused.
///
/// One rung down, and the bottom rung answers itself: a platform that grants no
/// capture at all gets a client that runs with a free cursor, because there is
/// nothing below it to fall to. Every answer is a state the game is played in,
/// which is what makes a refused pointer a degraded game rather than a failed
/// start.
#[must_use]
pub const fn next_capture_attempt(refused: CaptureState) -> CaptureState {
    match refused {
        CaptureState::Locked => CaptureState::Confined,
        CaptureState::Confined | CaptureState::Uncaptured => CaptureState::Uncaptured,
    }
}

/// Where Escape leaves a cursor that is currently in `state`.
///
/// It releases and never takes, from either capture: a confined cursor is as
/// trapped as a locked one from the player's side, and a release that only
/// understood the capture asked for first would leave every player it fell back
/// for with no way out but killing the process. Pressed with the cursor already
/// free it changes nothing, because a toggle here would grab the pointer back
/// from somebody who just asked to be let go of.
#[must_use]
pub const fn capture_after_escape(_state: CaptureState) -> CaptureState {
    CaptureState::Uncaptured
}

/// Where a click leaves a cursor that is currently in `state`.
///
/// Escape gives the pointer back and this is how the player takes it again.
/// Without it the first Escape ends looking around for the session: every rung
/// of the ladder above is satisfied, every scenario passes, and the game is
/// unplayable after a single keypress.
///
/// A free cursor re-enters the ladder at [`first_capture_attempt`] rather than
/// at whatever was granted last time, so a platform that refused the lock once
/// is asked again rather than being remembered as unable — the refusal is the
/// platform's answer to a request, not a fact about the session. A cursor that
/// is already held is left alone: the player is already playing, and walking the
/// ladder again on every click would re-ask for something they have.
#[must_use]
pub const fn capture_after_click(state: CaptureState) -> CaptureState {
    match state {
        CaptureState::Uncaptured => first_capture_attempt(),
        CaptureState::Locked | CaptureState::Confined => state,
    }
}

/// Whether pointer motion arriving in `state` is the player looking around.
///
/// An uncaptured pointer is the desktop's: the player is moving a cursor over
/// other windows, and turning the camera with it would be the game reading input
/// it was not given — and holding the turn ready for them when they came back.
#[must_use]
pub const fn accepts_pointer_motion(state: CaptureState) -> bool {
    match state {
        CaptureState::Locked | CaptureState::Confined => true,
        CaptureState::Uncaptured => false,
    }
}

/// What `event` asks the event loop to do.
///
/// Every window event the client cares about reaches exactly one of five
/// answers, and everything else is ignored explicitly rather than by falling off
/// the end of a match somebody has to remember to extend.
///
/// A window that lost focus is itself a release of everything held in it: the
/// key-up events for those keys are delivered to whatever has focus now and
/// never arrive here, so the decision to drop them is made in this table rather
/// than in the adapter that noticed.
#[must_use]
pub const fn window_event_action(event: &WindowEventKind) -> LoopAction {
    match event {
        WindowEventKind::CloseRequested => LoopAction::Exit,
        WindowEventKind::Resized(size) => LoopAction::Resize(*size),
        WindowEventKind::RedrawRequested => LoopAction::Redraw,
        WindowEventKind::FocusLost => LoopAction::ClearInput,
        WindowEventKind::Other => LoopAction::Ignore,
    }
}

// There is deliberately no tick-advancing function here. The replay's tick is
// the simulation's — D-B binds the client to render what `Simulation` publishes,
// so the wrap that runs is `mc_sim`'s own and FR-9.4-S2 tests it there. A second
// wrap in this crate would be a public function the product never calls, which
// reads as covered policy while covering nothing: the version deleted here was
// green through a mutation that made the real replay restart at the wrong tick.

/// How the client's run ended.
///
/// The one place the three endings meet, so `main` reports and returns and
/// decides nothing. A closed window is the only one that is not a failure.
/// The three the scenarios name, and one for everything else. `Failed` carries
/// text rather than a type because the things that reach it are the *client's*
/// failures — a window that would not open, content that is not there — and this
/// crate cannot name them. Keeping it here anyway is what leaves the client with
/// no exit-status decision of its own.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ending {
    Closed,
    Startup(StartupError),
    Frame(FatalReason),
    Failed { report: String },
}

/// The status the shell is told, for a run that ended in `ending`.
///
/// It lives here, beside the endings, rather than inside a `main` that no test
/// can call — which is also what keeps it inside the coverage denominator
/// ADR-013 draws around this crate.
#[must_use]
pub const fn exit_code(ending: &Ending) -> u8 {
    match ending {
        Ending::Closed => SUCCESS,
        Ending::Startup(_) | Ending::Frame(_) | Ending::Failed { .. } => FAILURE,
    }
}

/// What a run that did what it was asked reports.
const SUCCESS: u8 = 0;

/// What a run that could not reports. Any non-zero status would do; this is the
/// conventional one for "failed", and the tests ask only that it is not zero.
const FAILURE: u8 = 1;

#[cfg(test)]
#[path = "window_test.rs"]
mod tests;
