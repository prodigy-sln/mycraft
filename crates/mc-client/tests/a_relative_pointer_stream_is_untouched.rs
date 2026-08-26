//! What a physical mouse on a local console does, before and after the client
//! learned to recognise a stream of screen positions.
//!
//! # The oracle is the pose the renderer is handed
//!
//! Every assertion below reads the `CameraPose` the simulation publishes, which
//! is the value the renderer is actually given — not a look delta, a yaw or any
//! other intermediate the product could stop consulting while still drawing the
//! same wrong picture. A camera's facing is `target - eye`, and "right" is taken
//! from the facing the run's own no-motion control published rather than written
//! down, so the question survives a fixture aimed somewhere else.
//!
//! # Why equality is exact here and not approximate
//!
//! Two samples of 50 counts and one of 100 reach the accumulator as
//! `50s + 50s` and `100s` for one `f32` sensitivity `s`. Doubling is exact in
//! binary floating point and commutes with rounding, so `100s` *is*
//! `2 × round(50s)` — the two runs agree bit for bit, and a tolerance here would
//! only hide a client that had started rounding its accumulation.
//!
//! # A lone position-shaped sample gets a test of its own
//!
//! The regression scenarios say what two consecutive screen positions mean. The
//! third test below says what *one* means when an ordinary delta follows it: one
//! freak report must not leave the client differencing positions, and no
//! scenario would have reddened if it did. It is the state machine's
//! "Relative, pending / not position-shaped" row, and it is the row a physical
//! mouse would land on if a single packet ever arrived malformed.
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
use mc_sim::camera::CameraPose;

use input::InputHarness;

type TestResult = Result<(), Box<dyn Error>>;

/// How much pointer motion a whole sample carries, in raw device counts.
///
/// +x is right and +y is *down*, which is the screen's convention and the one
/// the operating system reports in.
const RAW_COUNTS: f64 = 100.0;

/// The same travel split across two samples.
const HALF_THE_COUNTS: f64 = 50.0;

/// A sample that looks like a screen position: both components inside the
/// absolute range, one of them far above the reading threshold.
const A_SCREEN_POSITION: (f64, f64) = (30_000.0, 20_000.0);

#[test]
fn two_samples_turn_the_camera_as_far_as_one_sample_of_their_sum() -> TestResult {
    let still = camera_after_one_tick(InputHarness::started(), |_| {})?;
    let split = camera_after_one_tick(InputHarness::started(), |harness| {
        harness.move_pointer(HALF_THE_COUNTS, 0.0);
        harness.move_pointer(HALF_THE_COUNTS, 0.0);
    })?;
    let whole = camera_after_one_tick(InputHarness::started(), |harness| {
        harness.move_pointer(RAW_COUNTS, 0.0);
    })?;

    assert!(
        rightward_lean(&whole, &still) > 0.0,
        "the control this scenario needs: {RAW_COUNTS} counts in one sample has to turn the \
         camera right, or the sameness below is two runs of a client that looks nowhere. It \
         leaned {}",
        rightward_lean(&whole, &still)
    );
    assert_eq!(
        split, whole,
        "a physical mouse reports a flick as several samples before one tick, and every one of \
         them is a movement to be spent: two samples of {HALF_THE_COUNTS} counts have to leave \
         the camera exactly where one of {RAW_COUNTS} leaves it. A client that had started \
         treating a sample as a position would difference these two instead of adding them, and \
         the second one alone would turn the camera by nothing at all"
    );
    Ok(())
}

#[test]
fn a_sample_with_a_negative_component_turns_the_camera_the_way_its_sign_names() -> TestResult {
    let still = camera_after_one_tick(InputHarness::started(), |_| {})?;
    let right = camera_after_one_tick(InputHarness::started(), |harness| {
        harness.move_pointer(RAW_COUNTS, 0.0);
    })?;
    let left = camera_after_one_tick(InputHarness::started(), |harness| {
        harness.move_pointer(-RAW_COUNTS, 0.0);
    })?;

    assert!(
        rightward_lean(&right, &still) > 0.0,
        "the control this scenario is read against: the same magnitude with a positive sign has \
         to turn the camera right. It leaned {}",
        rightward_lean(&right, &still)
    );
    assert!(
        rightward_lean(&left, &still) < 0.0,
        "a negative component is the pointer moving left, and no screen position is ever \
         negative — so this sample is a movement whichever way the client is reading the stream, \
         and the camera turns left by it. It leaned {} instead, which is a client that has \
         stopped reading the sign, or one that dropped the sample for failing a range check it \
         was never supposed to be measured against",
        rightward_lean(&left, &still)
    );
    Ok(())
}

#[test]
fn a_lone_screen_position_does_not_stop_the_delta_after_it_being_spent() -> TestResult {
    let still = camera_after_one_tick(InputHarness::started(), |_| {})?;
    let delta_alone = camera_after_one_tick(InputHarness::started(), |harness| {
        harness.move_pointer(RAW_COUNTS, 0.0);
    })?;
    let after_one_position = camera_after_one_tick(InputHarness::started(), |harness| {
        harness.move_pointer(A_SCREEN_POSITION.0, A_SCREEN_POSITION.1);
        harness.move_pointer(RAW_COUNTS, 0.0);
    })?;

    assert!(
        rightward_lean(&delta_alone, &still) > 0.0,
        "the control this test is read against: {RAW_COUNTS} counts on their own turn the camera \
         right. It leaned {}",
        rightward_lean(&delta_alone, &still)
    );
    assert_eq!(
        after_one_position, delta_alone,
        "one sample that looks like a screen position is not evidence that the stream reports \
         positions, and the ordinary delta that follows it settles the question the other way. \
         The camera therefore ends where that delta alone would leave it: the position is spent \
         on nothing and the delta is spent whole. A client that acted on one report would be \
         differencing every later sample against a stale position for the rest of the run, which \
         is the defect this whole fix exists to stop — arrived at from the other direction"
    );
    Ok(())
}

/// The camera one tick publishes, with `dispatched` delivered to a client over
/// the declared ground plane before that tick is taken.
///
/// Dispatching nothing is the control every assertion here is read against: the
/// same platform, the same world, the same tick, and no motion at all.
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
///
/// Zero is a camera that did not turn, and negative is one that turned the other
/// way.
fn rightward_lean(turned: &CameraPose, control: &CameraPose) -> f32 {
    let ahead = facing(control);
    facing(turned).dot(Vec3::new(-ahead.z, 0.0, ahead.x))
}
