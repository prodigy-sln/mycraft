//! What happens when the shape of the pointer stream changes under a client that
//! is already playing, and what a pointer the game does not hold teaches it.
//!
//! # These are not hypothetical transitions
//!
//! A Remote Desktop session resumed on the local console hands a client that has
//! been reading screen positions a physical mouse's deltas, mid-run. Reconnecting
//! over Remote Desktop does the reverse. Neither restarts the game, so both
//! changes arrive as a change in the numbers and nothing else.
//!
//! # Every expected turn is a run of the untouched relative path
//!
//! Each expectation below is a *second run of the same client*, handed the travel
//! the first run should have measured as ordinary deltas — never a camera pose
//! copied out of a green run. 4608 absolute units are exactly 135 device counts
//! horizontally, which is the only arithmetic these tests do.
//!
//! # Both absence scenarios carry a control
//!
//! A free cursor teaching the client nothing, and a lone contrary sample being
//! spent on nothing, are each satisfied by a client that does nothing whatever.
//! So each test below runs the same motion under a pointer the game holds, or
//! against the travel it should have made, and requires that run to have turned
//! the camera.
//!
//! # A sample dispatched after a lost window is one seen after focus came back
//!
//! winit registers raw input under its default `DeviceEvents::WhenFocused`
//! filter, without `RIDEV_INPUTSINK`, and nothing in this workspace asks for
//! anything else — so while the window is unfocused no `MouseMotion` is
//! delivered at all. Every sample the two focus tests dispatch after
//! `lose_focus()` is therefore one the shipped client could only see once focus
//! had returned, which is why they need no way to hand focus back. What the
//! player did with the pointer in between reaches this client as nothing
//! whatever, and that is the whole of the problem: the position it last saw is
//! the one it kept.
//!
//! # The harness is included by path
//!
//! Not through `tests/support/mod.rs`, which links `support/frames.rs` and with
//! it the whole graphics stack — into a binary whose entire premise is that no
//! adapter is acquired.

#[path = "support/input/mod.rs"]
mod input;

use std::error::Error;

use glam::Vec3;
use mc_render::window::CaptureState;
use mc_sim::camera::CameraPose;
use winit::event::MouseButton;
use winit::keyboard::KeyCode;

use input::InputHarness;

type TestResult = Result<(), Box<dyn Error>>;

/// A screen position to start from, well inside the absolute range.
const ORIGIN: (f64, f64) = (30_000.0, 20_000.0);

/// Absolute units of horizontal travel worth [`TRAVEL_IN_COUNTS`].
const ACROSS: f64 = 4_608.0;

/// What that travel is worth in raw device counts.
const TRAVEL_IN_COUNTS: f64 = 135.0;

/// A screen position far from [`ORIGIN`], where a pointer that spent time on the
/// desktop comes back.
const ELSEWHERE: (f64, f64) = (60_000.0, 20_000.0);

/// The gap a client that kept its last position would difference against: from
/// where the pointer was when the game stopped seeing it to where it turns up
/// when the game sees it again.
///
/// Named rather than written down, so it stays the gap between the positions the
/// scenarios actually dispatch.
const UNSEEN_UNITS: f64 = ELSEWHERE.0 - (ORIGIN.0 + ACROSS);

/// Two samples an ordinary mouse could produce, neither of them a position.
const A_SMALL_STEP: (f64, f64) = (3.0, 2.0);
const A_STEP_BACK: (f64, f64) = (-4.0, 1.0);

/// One sample inside the absolute range that is still nobody's screen position:
/// both components sit below the reading threshold.
const TOO_SMALL_TO_BE_A_POSITION: (f64, f64) = (800.0, 600.0);

/// A platform that refuses every capture it is asked for.
///
/// The only route to a free cursor at construction: a client refused the whole
/// ladder is the one state in which the desktop still owns the pointer from the
/// first frame.
const REFUSES_EVERYTHING: [CaptureState; 0] = [];

#[test]
fn two_ordinary_deltas_hand_the_stream_back_to_the_relative_reading() -> TestResult {
    let still = camera_after_one_tick(InputHarness::started(), |_| {})?;
    let expected = camera_after_one_tick(InputHarness::started(), |harness| {
        harness.move_pointer(TRAVEL_IN_COUNTS, 0.0);
        harness.move_pointer(A_STEP_BACK.0, A_STEP_BACK.1);
    })?;
    let resumed = camera_after_one_tick(InputHarness::started(), |harness| {
        travelled_across(harness);
        harness.move_pointer(A_SMALL_STEP.0, A_SMALL_STEP.1);
        harness.move_pointer(A_STEP_BACK.0, A_STEP_BACK.1);
    })?;

    assert!(
        rightward_lean(&expected, &still) > 0.0,
        "the control this scenario is read against: the travel it expects has to turn the camera. \
         It leaned {}",
        rightward_lean(&expected, &still)
    );
    assert_eq!(
        resumed, expected,
        "one sample that is not a screen position is not evidence that the stream has stopped \
         reporting them, and it is the *second* that settles it — spent as the delta it is, while \
         the first is spent on nothing. So the run ends where {TRAVEL_IN_COUNTS} counts of travel \
         followed by {A_STEP_BACK:?} leave it. A client that spent the deciding sample as the \
         regime it was leaving would difference {A_STEP_BACK:?} against a screen position and \
         turn the camera thirty thousand counts at the moment the player plugged a mouse in"
    );
    Ok(())
}

#[test]
fn one_ordinary_delta_between_screen_positions_is_spent_on_nothing() -> TestResult {
    let still = camera_after_one_tick(InputHarness::started(), |_| {})?;
    let expected = camera_after_one_tick(InputHarness::started(), |harness| {
        harness.move_pointer(TRAVEL_IN_COUNTS, 0.0);
        harness.move_pointer(TRAVEL_IN_COUNTS, 0.0);
    })?;
    let interrupted = camera_after_one_tick(InputHarness::started(), |harness| {
        travelled_across(harness);
        harness.move_pointer(TOO_SMALL_TO_BE_A_POSITION.0, TOO_SMALL_TO_BE_A_POSITION.1);
        harness.move_pointer(ORIGIN.0 + ACROSS + ACROSS, ORIGIN.1);
    })?;

    assert!(
        rightward_lean(&expected, &still) > 0.0,
        "the control this scenario is read against: two travels of {TRAVEL_IN_COUNTS} counts have \
         to turn the camera. It leaned {}",
        rightward_lean(&expected, &still)
    );
    assert_eq!(
        interrupted, expected,
        "a single {TOO_SMALL_TO_BE_A_POSITION:?} between screen positions is one freak report, \
         not a change of stream: it turns the camera by nothing, and the position after it is \
         measured from the last position rather than from it. Two travels of \
         {TRAVEL_IN_COUNTS} counts, and no third. A client that flipped on one sample would then \
         difference the next screen position against {TOO_SMALL_TO_BE_A_POSITION:?} and spend \
         nearly the whole absolute range as a turn"
    );
    Ok(())
}

#[test]
fn screen_positions_arriving_while_the_cursor_is_free_leave_the_camera_alone() -> TestResult {
    let held = camera_after_one_tick(InputHarness::started(), travelled_across)?;
    let held_still = camera_after_one_tick(InputHarness::started(), |_| {})?;
    let still = camera_after_one_tick(InputHarness::granting(&REFUSES_EVERYTHING), |_| {})?;
    let free = camera_after_one_tick(
        InputHarness::granting(&REFUSES_EVERYTHING),
        travelled_across,
    )?;

    assert!(
        rightward_lean(&held, &held_still) > 0.0,
        "the control this scenario needs: the same two screen positions under a pointer the game \
         holds have to turn the camera, or the sameness below is a client that never looks \
         anywhere. It leaned {}",
        rightward_lean(&held, &held_still)
    );
    assert_eq!(
        free, still,
        "an uncaptured pointer belongs to the desktop, and a screen position it reports is where \
         the player's cursor is over somebody else's window. The published camera is left exactly \
         as a tick with no motion at all leaves it — and, because nothing was measured, there is \
         no position left behind for the next turn to be measured from either"
    );
    Ok(())
}

#[test]
fn taking_the_pointer_back_turns_only_by_the_travel_since_it_came_back() -> TestResult {
    let still = camera_after_one_tick(InputHarness::started(), |_| {})?;
    let expected = camera_after_one_tick(InputHarness::started(), |harness| {
        harness.move_pointer(TRAVEL_IN_COUNTS, 0.0);
        harness.move_pointer(TRAVEL_IN_COUNTS, 0.0);
    })?;
    let returned = camera_after_one_tick(InputHarness::started(), |harness| {
        travelled_across(harness);
        harness.press(KeyCode::Escape);
        harness.move_pointer(ELSEWHERE.0, ELSEWHERE.1);
        harness.click(MouseButton::Left);
        harness.move_pointer(ELSEWHERE.0, ELSEWHERE.1);
        harness.move_pointer(ELSEWHERE.0 + ACROSS, ELSEWHERE.1);
    })?;

    assert!(
        rightward_lean(&expected, &still) > 0.0,
        "the control this scenario is read against: two travels of {TRAVEL_IN_COUNTS} counts have \
         to turn the camera. It leaned {}",
        rightward_lean(&expected, &still)
    );
    assert_eq!(
        returned, expected,
        "Escape gives the pointer to the desktop and a click takes it back, and the journey it \
         made in between is the player's, not the game's. The camera turns by the travel before \
         it left and the travel after it returned — two travels of {TRAVEL_IN_COUNTS} counts — \
         and by nothing for the first position after the pointer came back, because there is \
         nothing left to measure that one from. A client that kept its old position would spend \
         the distance across the desktop as one enormous turn the instant the player clicked back \
         in, which is the failure the player meets every time they alt-tab"
    );
    Ok(())
}

#[test]
fn losing_the_window_turns_only_by_the_travel_since_it_came_back() -> TestResult {
    let still = camera_after_one_tick(InputHarness::started(), |_| {})?;
    let expected = camera_after_one_tick(InputHarness::started(), |harness| {
        harness.move_pointer(TRAVEL_IN_COUNTS, 0.0);
        harness.move_pointer(TRAVEL_IN_COUNTS, 0.0);
    })?;
    let returned = camera_after_one_tick(InputHarness::started(), came_back_elsewhere)?;

    assert!(
        rightward_lean(&expected, &still) > 0.0,
        "the control this scenario is read against: two travels of {TRAVEL_IN_COUNTS} counts have \
         to turn the camera. It leaned {}",
        rightward_lean(&expected, &still)
    );
    assert_eq!(
        returned,
        expected,
        "the window going away is the client's last sight of the pointer, and where it turns up \
         when the window comes back is not a distance anybody travelled while looking at this \
         game. So the camera turns by the travel before the window went away and the travel \
         after it returned — two travels of {TRAVEL_IN_COUNTS} counts — and by nothing for the \
         first position after it came back, because nothing is left to measure that one from. A \
         client that kept the position it last saw differences {ELSEWHERE:?} against it and \
         spends the {UNSEEN_UNITS} units nobody was watching, {} times this run's whole travel, \
         as one turn at the instant the player alt-tabs back in",
        UNSEEN_UNITS / ACROSS
    );
    Ok(())
}

#[test]
fn losing_the_window_leaves_an_ordinary_delta_spent_whole() -> TestResult {
    let still = camera_after_one_tick(InputHarness::started(), |_| {})?;
    let expected = camera_after_one_tick(InputHarness::started(), |harness| {
        harness.move_pointer(TRAVEL_IN_COUNTS, 0.0);
    })?;
    let after_the_window = camera_after_one_tick(InputHarness::started(), |harness| {
        harness.lose_focus();
        harness.move_pointer(TRAVEL_IN_COUNTS, 0.0);
    })?;

    assert!(
        rightward_lean(&expected, &still) > 0.0,
        "the control this test is read against: {TRAVEL_IN_COUNTS} counts on their own turn the \
         camera right. It leaned {}",
        rightward_lean(&expected, &still)
    );
    assert_eq!(
        after_the_window, expected,
        "what a lost window costs is the position there was to measure from, and nothing else. A \
         physical mouse never had one, so the first sample after the window comes back is spent \
         whole exactly as it was before — the camera lands where that delta alone leaves it. The \
         repair this guards against is one that clears more than the position: a client left \
         waiting for two samples to corroborate a stream it never left swallows the first flick \
         of every alt-tab back, on a local console where nothing was ever wrong"
    );
    Ok(())
}

/// A player who travelled [`TRAVEL_IN_COUNTS`] counts, went to another window,
/// and came back with the pointer [`UNSEEN_UNITS`] away from where the game last
/// saw it — then travelled [`TRAVEL_IN_COUNTS`] counts again.
///
/// The two samples after the window is lost are the two the shipped client would
/// see once focus had returned: none is delivered while it is away.
fn came_back_elsewhere(harness: &mut InputHarness) {
    travelled_across(harness);
    harness.lose_focus();
    harness.move_pointer(ELSEWHERE.0, ELSEWHERE.1);
    harness.move_pointer(ELSEWHERE.0 + ACROSS, ELSEWHERE.1);
}

/// Two screen positions [`ACROSS`] apart: the pointer arriving, and then having
/// travelled [`TRAVEL_IN_COUNTS`] counts to the right.
fn travelled_across(harness: &mut InputHarness) {
    harness.move_pointer(ORIGIN.0, ORIGIN.1);
    harness.move_pointer(ORIGIN.0 + ACROSS, ORIGIN.1);
}

/// The camera one tick publishes, with `dispatched` delivered to a client over
/// the declared ground plane before that tick is taken.
fn camera_after_one_tick(
    mut harness: InputHarness,
    dispatched: impl FnOnce(&mut InputHarness),
) -> Result<CameraPose, Box<dyn Error>> {
    harness.start_world()?;
    dispatched(&mut harness);
    let published = harness
        .tick()
        .ok_or("a tick over a started world publishes a snapshot")?;
    Ok(published.camera)
}

/// The direction a published camera looks in.
fn facing(camera: &CameraPose) -> Vec3 {
    Vec3::from_array(camera.target) - Vec3::from_array(camera.eye)
}

/// How far `turned` leans toward the right hand of what `control` was facing.
fn rightward_lean(turned: &CameraPose, control: &CameraPose) -> f32 {
    let ahead = facing(control);
    facing(turned).dot(Vec3::new(-ahead.z, 0.0, ahead.x))
}
