//! What the client asks the platform to do with the pointer, and what dispatched
//! pointer motion does once it has it.
//!
//! # The oracle is the pose the renderer is handed
//!
//! Every camera assertion below reads the `CameraPose` the simulation publishes,
//! which is the value the renderer is actually given — not a look delta, a yaw or
//! any other intermediate the product could stop consulting while still drawing
//! the same wrong picture. A camera's facing is `target - eye`.
//!
//! # The frame, and why "right" is derived rather than written down
//!
//! Yaw 0 faces +x, yaw +π/2 faces +z, and positive pitch looks up
//! (`crates/mc-sim/src/player/mod.rs`), so at pitch 0 a facing of `(cos y, 0, sin y)`
//! has its right hand at `(-sin y, 0, cos y)`. Rather than spell `+z` — which is
//! only the answer while the fixture happens to spawn at yaw 0 — each scenario
//! takes the right hand *of the facing its own no-motion control published*. The
//! assertion is then that the turned camera leans toward that right, which stays
//! the same question if the fixture's spawn is ever re-aimed.
//!
//! # Direction and sign, and the size of the gap
//!
//! 100 raw counts is a fifth of a radian of turn, while the float noise on a
//! facing derived at the fixture's distance from the origin is a few parts in a
//! million. The assertions are therefore `> 0` and `< 0` against a control that
//! measures exactly zero — an unturned camera's facing has no rightward component
//! at all, and a level camera's target sits exactly level with its eye, because
//! `sin(0)` is exact. There is no tolerance to loosen: the measured error is zero
//! and the smallest difference to catch is five orders of magnitude above it.
//!
//! # Two scenarios assert an absence, and both carry a control
//!
//! A free cursor's motion changing nothing, and the ladder's first ask being
//! refused, are both satisfied by a client that does nothing whatever. So each
//! runs the same motion under a pointer the game *holds*, in the same test, and
//! requires that run to have turned the camera.
//!
//! # Why one scenario asserts an ask and a consequence together
//!
//! A test asserting only that a confined pointer was asked for would be satisfied
//! by a client that recorded a grant it never acted on; a test asserting only the
//! turn would be satisfied by a platform that granted the lock it was supposed to
//! refuse, with the ladder's loop body never running twice. Neither half is
//! evidence without the other, and splitting them would produce a second test
//! with identical setup asserting what another scenario already covers.
//!
//! # The ask is asserted against a literal, deliberately
//!
//! The capture the client opens with, and the one it falls back to, are the
//! capture policy's own answers, and a test that called that policy to find out
//! what it should have expected would agree with the client by construction: both
//! would be wrong together the day the policy changed, and neither would notice a
//! client that asked for nothing at all. So the expectations here are written
//! down.
//!
//! # The harness is included by path
//!
//! Not through `tests/support/mod.rs`, which links `support/frames.rs` and with it
//! the whole graphics stack — into a binary whose entire premise is that no
//! adapter is acquired (conductor ruling 56).

#[path = "support/input/mod.rs"]
mod input;

use std::error::Error;

use glam::Vec3;
use mc_render::window::CaptureState;
use mc_sim::camera::CameraPose;

use input::InputHarness;

type TestResult = Result<(), Box<dyn Error>>;

/// How much pointer motion is dispatched, in raw device counts.
///
/// +x is right and +y is *down*, which is the screen's convention and the one the
/// operating system reports in.
const RAW_COUNTS: f64 = 100.0;

/// A platform that refuses every capture it is asked for.
///
/// The only route to a free cursor: with the Escape path cut, a client that is
/// refused the whole ladder is the one state in which the desktop still owns the
/// pointer.
const REFUSES_EVERYTHING: [CaptureState; 0] = [];

/// A platform that refuses a locked pointer and grants a confined one.
const CONFINES_ONLY: [CaptureState; 1] = [CaptureState::Confined];

/// How many tick steps are taken before the world lands.
///
/// More than one, so the motion has to survive being carried rather than merely
/// arriving late.
const TICKS_BEFORE_THE_WORLD: u32 = 3;

#[test]
fn starting_a_session_asks_a_granting_platform_for_a_locked_pointer_exactly_once() {
    let started = InputHarness::started();

    assert_eq!(
        started.grabs(),
        vec![CaptureState::Locked],
        "a client that never asks the platform to hold the pointer has no mouselook at all, and \
         nothing about it is visible from inside the game — so starting a session against a \
         platform that grants a locked pointer has to leave exactly one ask behind it, for a \
         locked pointer, before any event has been dispatched. This one asked for {:?}",
        started.grabs()
    );
}

#[test]
fn pointer_motion_to_the_right_turns_the_camera_toward_the_players_right() -> TestResult {
    let still = camera_after_one_tick(InputHarness::started(), |_| {})?;
    let turned = camera_after_one_tick(InputHarness::started(), |harness| {
        harness.move_pointer(RAW_COUNTS, 0.0);
    })?;

    assert!(
        rightward_lean(&turned, &still) > 0.0,
        "{RAW_COUNTS} counts of the pointer moved right is the player turning right, and the \
         camera the renderer is handed has to have turned with them. Against the facing its own \
         no-motion control published, this one leaned {} — zero is a client whose pointer reaches \
         nothing, and negative is a camera that turns the wrong way, which is unplayable in a \
         way no still frame would show",
        rightward_lean(&turned, &still)
    );
    Ok(())
}

#[test]
fn pointer_motion_downward_puts_the_cameras_target_below_its_eye() -> TestResult {
    let level = camera_after_one_tick(InputHarness::started(), |_| {})?;
    let lowered = camera_after_one_tick(InputHarness::started(), |harness| {
        harness.move_pointer(0.0, RAW_COUNTS);
    })?;

    assert!(
        rise(&level) >= 0.0,
        "the control this scenario needs: the camera has to start out level, or a target below \
         its eye is where it was looking all along. It started {} above it",
        rise(&level)
    );
    assert!(
        rise(&lowered) < 0.0,
        "a pointer pushed down is the player looking down, so the camera's target ends up below \
         its eye — it is {} above it instead. The two raw axes are one type and one call apart, \
         so a swap or a flipped vertical sign is smooth, total and reproducible while being an \
         upside-down world, and every horizontal-only scenario stays green through it",
        rise(&lowered)
    );
    Ok(())
}

#[test]
fn pointer_motion_arriving_while_the_cursor_is_free_leaves_the_camera_alone() -> TestResult {
    let held = camera_after_one_tick(InputHarness::started(), |harness| {
        harness.move_pointer(RAW_COUNTS, 0.0);
    })?;
    let still = camera_after_one_tick(InputHarness::granting(&REFUSES_EVERYTHING), |_| {})?;
    let free = camera_after_one_tick(InputHarness::granting(&REFUSES_EVERYTHING), |harness| {
        harness.move_pointer(RAW_COUNTS, 0.0);
    })?;

    assert!(
        rightward_lean(&held, &still) > 0.0,
        "the control this scenario needs: the same motion under a pointer the game holds has to \
         turn the camera, or the sameness below is a client that never looks anywhere"
    );
    assert_eq!(
        free, still,
        "an uncaptured pointer belongs to the desktop: the player is moving a cursor over other \
         windows, and the same {RAW_COUNTS} counts leave the published camera exactly as a tick \
         with no motion at all leaves it. A client that turned anyway spins the view while the \
         player is using another window, and has the spin waiting for them when they come back"
    );
    Ok(())
}

#[test]
fn pointer_motion_dispatched_before_the_world_lands_turns_the_camera_at_its_first_tick()
-> TestResult {
    let still = camera_after_one_tick(InputHarness::started(), |_| {})?;

    let mut early = InputHarness::started();
    early.move_pointer(RAW_COUNTS, 0.0);
    let published_before_the_world = early.ticks(TICKS_BEFORE_THE_WORLD);
    early.start_world();
    let first = early
        .tick()
        .ok_or("a tick over a started world publishes a snapshot")?;

    assert!(
        published_before_the_world.is_empty(),
        "the precondition this scenario is stated under: the {TICKS_BEFORE_THE_WORLD} tick steps \
         before the world lands have to have had nothing to advance, or the motion below was \
         never carried across anything. They published {} snapshots",
        published_before_the_world.len()
    );
    assert!(
        rightward_lean(&first.camera, &still) > 0.0,
        "a pointer moved while the world is still loading is a turn the player has already made, \
         and the first tick after the world lands is where it arrives — it leaned {} instead. A \
         tick that spends the pending look whether or not it has anything to advance throws the \
         turn away on a tick that drew nothing, and unlike a held key a look is spent exactly \
         once, so nothing brings it back",
        rightward_lean(&first.camera, &still)
    );
    Ok(())
}

#[test]
fn a_refused_lock_is_followed_by_a_confined_pointer_the_motion_still_reaches() -> TestResult {
    let mut confined = InputHarness::granting(&CONFINES_ONLY);
    confined.start_world();
    confined.move_pointer(RAW_COUNTS, 0.0);
    let turned = confined
        .tick()
        .ok_or("a tick over a started world publishes a snapshot")?;
    let still = camera_after_one_tick(InputHarness::granting(&CONFINES_ONLY), |_| {})?;

    assert_eq!(
        confined.grabs(),
        vec![CaptureState::Locked, CaptureState::Confined],
        "the ask this scenario is stated under: a platform that refuses a locked pointer has to \
         be asked for a confined one next, so the log reads two asks in that order. A single ask \
         means the ladder's loop body never ran twice — most likely a platform that granted what \
         it was told to refuse, under which the turn below proves nothing. It asked for {:?}",
        confined.grabs()
    );
    assert!(
        rightward_lean(&turned.camera, &still) > 0.0,
        "the capture the client fell back to is a capture it has to actually look through: the \
         same {RAW_COUNTS} counts turn the camera under a confined pointer exactly as they do \
         under a locked one, and this one leaned {}. A client that recorded the grant without \
         acting on it leaves a player with no mouselook at all on every compositor that refuses \
         a lock",
        rightward_lean(&turned.camera, &still)
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
    harness.start_world();
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
/// way. The right hand is taken from the control's own published facing rather
/// than written down, so the question survives a fixture aimed somewhere else.
fn rightward_lean(turned: &CameraPose, control: &CameraPose) -> f32 {
    let ahead = facing(control);
    facing(turned).dot(Vec3::new(-ahead.z, 0.0, ahead.x))
}

/// How far a camera's target stands above its eye.
fn rise(camera: &CameraPose) -> f32 {
    facing(camera).y
}
