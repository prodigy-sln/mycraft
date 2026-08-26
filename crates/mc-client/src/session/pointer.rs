//! What the client can ask a platform to do with the pointer.
//!
//! A port and the vocabulary of one ask, and nothing that decides anything. The
//! ladder — which attempt follows a refusal, when the walk stops, and what is
//! left when nothing was granted — is the session's, for the reason the module
//! above gives: a port that walked it would put those decisions on the side of
//! the seam no test can reach.
//!
//! Its own file for the same reason the keyboard vocabulary has one: a port is
//! not a decision, and it needs nothing the session owns.

use mc_render::window::CaptureState;

/// What the platform can be asked to do with the pointer.
///
/// One attempt at a time, deliberately: a port that took a capture and answered
/// with the one it settled on would be walking the ladder, and the ladder is the
/// decision this trait exists to keep out of the platform.
pub trait PointerPlatform {
    /// Asks for `capture` and reports whether the platform granted it.
    ///
    /// Never called with [`CaptureState::Uncaptured`] — that is the bottom of
    /// the ladder, which is a state rather than something to ask a window for.
    fn grab(&mut self, capture: CaptureState) -> bool;

    /// Gives the pointer back to the desktop.
    fn release(&mut self);

    /// Shows or hides the cursor.
    fn show_cursor(&mut self, visible: bool);
}

/// One thing a platform was asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PointerAsk {
    Grab(CaptureState),
    Release,
    CursorVisible(bool),
}
