//! What the event loop does with a window event, and how the run it ends is
//! reported to the shell.
//!
//! Both are pure functions, which is what keeps `winit` out of everything but
//! one adapter file in the client. The loop translates an event into one of
//! four actions and asks nothing else about it.
//!
//! **There is no replay wrap to test here, and there is none anywhere.** The
//! client renders the tick `mc_sim::Simulation` publishes, and that counter is
//! free-running: 120 is the length of the declared intent script, not a period
//! of the simulation. A wrap in this crate would be a function the product never
//! calls, which is exactly what a test cannot detect.

//! **Cursor capture is a policy here and an acceptance elsewhere.** Whether the
//! operating system actually took the pointer cannot be asked without a window,
//! a compositor and a person looking at the screen, and it is recorded as manual
//! acceptance in `docs/technical/testing.md`. What *is* decidable is the ladder:
//! which capture is asked for first, what each refusal falls back to, that the
//! bottom rung is a state the game carries on in rather than a failure, and
//! where Escape leaves each of the three. That is what is asserted below, and
//! the four functions are total and infallible so that "and SHALL NOT exit" is a
//! property of their type rather than a thing a test has to catch them not
//! doing.
//!
//! Losing focus and admitting pointer motion are decided here too, but their
//! consequences are `mc-sim`'s — a cleared key and an unaccumulated look delta —
//! so those two scenarios are asserted end to end in
//! `crates/mc-client/tests/window_input.rs`, the one crate that resolves both
//! halves of the seam.

use super::{
    CaptureState, Ending, LoopAction, WindowEventKind, capture_after_click, capture_after_escape,
    exit_code, first_capture_attempt, next_capture_attempt, window_event_action,
};

#[test]
fn a_close_request_leaves_the_event_loop_and_the_process_ends_successfully() {
    assert_eq!(
        (
            window_event_action(&WindowEventKind::CloseRequested),
            exit_code(&Ending::Closed)
        ),
        (LoopAction::Exit, 0),
        "closing the window is the one ending that is not a failure: the loop is left and the \
         shell is told the run succeeded"
    );
}

#[test]
fn the_first_cursor_the_client_asks_for_is_a_locked_one() {
    assert_eq!(
        first_capture_attempt(),
        CaptureState::Locked,
        "a locked pointer is the one that reports motion with no position and warps back to the \
         window's centre, which is what lets the player keep turning past the edge of the screen. \
         It is what the client asks for before anything has refused it; starting at a lesser \
         capture would settle for a degraded look on every platform that would have granted the \
         better one"
    );
}

#[test]
fn a_refused_lock_falls_back_to_confining_the_cursor() {
    assert_eq!(
        next_capture_attempt(CaptureState::Locked),
        CaptureState::Confined,
        "a platform that will not lock the pointer may still keep it inside the window, which is \
         the next best thing and not nothing: the player can still turn until the cursor reaches \
         an edge. A refusal that gave up here would hand a working desktop the uncaptured cursor \
         it did not have to have"
    );
}

#[test]
fn a_refused_confinement_carries_on_uncaptured_rather_than_ending_the_run() {
    assert_eq!(
        (
            next_capture_attempt(CaptureState::Confined),
            next_capture_attempt(CaptureState::Uncaptured)
        ),
        (CaptureState::Uncaptured, CaptureState::Uncaptured),
        "the ladder bottoms out in a state the game is played in: a platform that grants no \
         capture at all gets a client that runs with a free cursor, and there is no rung below it \
         to fall to and nothing here that ends the run. A window the player can still walk around \
         in beats a window that closed itself because the pointer would not be held"
    );
}

#[test]
fn escape_releases_the_cursor_however_it_was_captured() {
    assert_eq!(
        (
            capture_after_escape(CaptureState::Locked),
            capture_after_escape(CaptureState::Confined)
        ),
        (CaptureState::Uncaptured, CaptureState::Uncaptured),
        "Escape is how the player gets their desktop back, and it has to work from either capture \
         — a confined cursor is as trapped as a locked one from the player's side. A release that \
         only understood the capture it asked for first would leave every player it fell back for \
         with no way out but killing the process"
    );
}

#[test]
fn a_click_with_the_cursor_free_asks_for_the_capture_the_ladder_starts_at() {
    assert_eq!(
        capture_after_click(CaptureState::Uncaptured),
        first_capture_attempt(),
        "Escape gives the cursor back and a click is how the player takes it again — without \
         one, the first Escape ends looking around for the session and the game is unplayable \
         after a single keypress. It re-enters the ladder at its *first* rung rather than at \
         the one that was granted before, so a platform that refused the lock once falls back \
         the same way again instead of being remembered as unable to lock. Asserted against \
         the ladder's own head rather than against `Locked` spelled twice: which capture the \
         ladder starts at is a separate decision, pinned separately, and restating it here \
         would leave both tests agreeing about a wrong answer"
    );
}

#[test]
fn escape_pressed_with_the_cursor_already_free_leaves_it_free() {
    assert_eq!(
        capture_after_escape(CaptureState::Uncaptured),
        CaptureState::Uncaptured,
        "Escape releases and never takes: pressed with the cursor already the desktop's, it \
         changes nothing. Toggling here would grab the pointer from a player who just asked to be \
         let go of, and would do it on the second press of a key they pressed twice"
    );
}
