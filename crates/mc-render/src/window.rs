//! The event loop's vocabulary: what a window event asks the client to do, what
//! tick follows the one just drawn, and how the client's run ends.
//!
//! `winit` is not named here and must not be. The client's `events.rs`
//! translates a `winit::event::WindowEvent` into a [`WindowEventKind`] and
//! nothing else in the workspace ever sees the original — which is what lets
//! every decision the loop makes be a pure function tested without a window, a
//! compositor or a display server.
//!
//! **The replay's length is a parameter, never a constant read here.**
//! `mc_sim::TICK_COUNT` is where it is declared and this crate may not resolve
//! `mc-sim` in any dependency kind, so the wrap is a function of the count it is
//! handed. That also makes it testable without the simulation existing.

use crate::surface::{FatalReason, StartupError, SurfaceSize};

/// What the window told the client, in the renderer's own vocabulary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowEventKind {
    CloseRequested,
    Resized(SurfaceSize),
    RedrawRequested,
    Other,
}

/// What the event loop does about it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoopAction {
    Exit,
    Redraw,
    Resize(SurfaceSize),
    Ignore,
}

/// What `event` asks the event loop to do.
///
/// Every window event the client cares about reaches exactly one of four
/// answers, and everything else is ignored explicitly rather than by falling off
/// the end of a match somebody has to remember to extend.
#[must_use]
pub const fn window_event_action(event: &WindowEventKind) -> LoopAction {
    match event {
        WindowEventKind::CloseRequested => LoopAction::Exit,
        WindowEventKind::Resized(size) => LoopAction::Resize(*size),
        WindowEventKind::RedrawRequested => LoopAction::Redraw,
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
