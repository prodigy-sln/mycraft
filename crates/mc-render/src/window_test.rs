//! What the event loop does with a window event, and how the run it ends is
//! reported to the shell.
//!
//! Both are pure functions, which is what keeps `winit` out of everything but
//! one adapter file in the client. The loop translates an event into one of
//! four actions and asks nothing else about it.
//!
//! **The replay's wrap is not tested here, and deliberately so.** The client
//! renders the tick `mc_sim::Simulation` publishes, so the wrap that runs in the
//! window is the simulation's and is asserted through `advance()` in
//! `crates/mc-sim/tests/replay_camera.rs` (conductor ruling 37). A second wrap
//! in this crate would be a function the product never calls, which is exactly
//! what a test cannot detect.

use super::{Ending, LoopAction, WindowEventKind, exit_code, window_event_action};

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
