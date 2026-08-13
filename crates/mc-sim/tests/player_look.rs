//! Looking around, and the camera the player's own state implies.
//!
//! Two things are asserted here and they are deliberately kept apart. *Where the
//! eye is* is arithmetic over the declared eye height and nothing else — the
//! camera is derived from the player rather than moved alongside it, so a
//! displacement of the feet is a displacement of the eye by construction. *Which
//! way it looks* is the accumulated view, and that is where the interesting
//! failures live.
//!
//! **The declared basis is the contract, not a convention.** Yaw 0 faces +x, yaw
//! +π/2 faces +z, and positive pitch looks up. Every projected figure this
//! feature is verified by is computed from that basis, so a camera that
//! exchanged its sine and cosine, or that negated its pitch, would be a quarter
//! turn or an upside-down world away while still being smooth, total and
//! reproducible. The two scenarios that pin the basis therefore assert a
//! *signed* answer: the quarter turn has to land on +z specifically, and a
//! positive pitch delta has to raise the target rather than merely move it.
//!
//! **Yaw and pitch are asked different questions because they are different
//! things.** Yaw is a direction on a circle: past a full turn is the same
//! direction, so it wraps, and the two range scenarios assert only that it came
//! back inside the turn while a third pins the exact value the wrap produces.
//! Pitch past the vertical would put the look direction on the world's up axis,
//! where a view matrix has no unique answer, so it is clamped short of it — and
//! the scenario that matters most there is the one asserting the camera still
//! has a horizontal component at the limit, because that is the property the
//! clamp exists for rather than the number it is spelled with.
//!
//! Every expected quantity below is arithmetic over the specification's declared
//! constants — the 1.62-block eye height, the ±89° pitch limit, a full turn —
//! and none is a value read off a run. Comparisons use the declared 1 × 10⁻⁴
//! epsilon, except where "unchanged" is meant: that is a question about bits,
//! which is both its exact form and the form `clippy::float_cmp` has no quarrel
//! with (`tests/support/mod.rs::exactly` is the same idiom).

use std::error::Error;
use std::f32::consts::{FRAC_PI_2, TAU};

use glam::Vec3;
use mc_sim::camera::CameraPose;
use mc_sim::player::{Look, MovementIntent, PlayerState, eye_pose};

type TestResult = Result<(), Box<dyn Error>>;

/// How far two figures this feature calls equal may differ, in blocks or in
/// radians. The specification's declared comparison epsilon.
const EPSILON: f32 = 1e-4;

/// How far above the feet the eyes sit, in blocks. Declared, not measured.
const EYE_HEIGHT: f32 = 1.62;

/// How far from level the view may tilt, in degrees. Declared, not measured.
const PITCH_LIMIT_DEGREES: f32 = 89.0;

/// A pitch delta large enough to drive the view past the limit in one tick.
///
/// 2.0 radians is 114.6°, comfortably past the 1.5533 radians the limit sits
/// at, so nothing here rests on how many ticks it took to get there.
const PAST_THE_LIMIT: f32 = 2.0;

/// Where the player's feet stand for every camera question below.
///
/// Off-lattice on all three axes and negative on one, so a camera that rounded,
/// truncated or took an absolute value somewhere has nowhere to hide.
const FEET: Vec3 = Vec3::new(10.25, 64.0, -3.5);

/// How far the player moves between the two poses the eye is compared across.
///
/// Non-zero and different on every axis, and negative on one: a camera that
/// moved by the displacement's length, or that dropped an axis, disagrees.
const DISPLACEMENT: Vec3 = Vec3::new(1.5, -2.25, 0.75);

/// Where a yaw of 6.2 radians ends up when 0.2 more is added to it, in radians.
///
/// The specification's own hand derivation, written down rather than recomputed
/// here: 6.2 + 0.2 = 6.4, and 6.4 − 2π = 0.116815. Taking it from the
/// declaration rather than from `6.4 - TAU` in this file keeps the expected
/// value independent of the same arithmetic the subject performs.
const WRAPPED_YAW: f32 = 0.116815;

/// A view that is level and axis-aligned in neither yaw nor pitch, so that
/// "the eye moved by the same displacement" is not asked of a degenerate pose.
const OFF_AXIS: Look = Look {
    yaw: 1.1,
    pitch: 0.3,
};

/// The view a non-finite delta is asked to leave alone.
const SETTLED: Look = Look {
    yaw: 1.25,
    pitch: 0.5,
};

/// Look deltas that are not finite numbers, in every position one can arrive in.
///
/// Both axes and both kinds: the rule is that a single non-finite delta leaves
/// *both* accumulators untouched, so each row pairs one bad delta with a
/// perfectly good one on the other axis. A guard that sanitised only the axis
/// the NaN arrived on would pass two of these four and fail the other two.
const NON_FINITE_DELTAS: [(f32, f32); 4] = [
    (f32::NAN, 0.3),
    (0.4, f32::NAN),
    (f32::INFINITY, 0.3),
    (0.4, f32::NEG_INFINITY),
];

/// A player standing still at `position` and looking along `look`.
fn standing_at(position: Vec3, look: Look) -> PlayerState {
    PlayerState {
        position,
        velocity: Vec3::ZERO,
        yaw: look.yaw,
        pitch: look.pitch,
        on_ground: true,
    }
}

/// An intent that asks for nothing but a change of view.
fn looking(yaw_delta: f32, pitch_delta: f32) -> MovementIntent {
    MovementIntent {
        yaw_delta,
        pitch_delta,
        ..MovementIntent::default()
    }
}

/// The vector from `from` to `to`.
fn separation(from: [f32; 3], to: [f32; 3]) -> [f32; 3] {
    let [from_x, from_y, from_z] = from;
    let [to_x, to_y, to_z] = to;
    [to_x - from_x, to_y - from_y, to_z - from_z]
}

/// Which way the camera looks: its target minus its eye.
fn direction(camera: &CameraPose) -> [f32; 3] {
    separation(camera.eye, camera.target)
}

/// The largest disagreement between two positions on any one axis.
fn furthest_axis(placed: [f32; 3], declared: [f32; 3]) -> f32 {
    placed
        .iter()
        .zip(declared.iter())
        .map(|(placed, declared)| (placed - declared).abs())
        .fold(0.0, f32::max)
}

/// A view as the integers its floats are. "Unchanged" means the same value, not
/// a nearly equal one.
fn exactly(look: Look) -> (u32, u32) {
    (look.yaw.to_bits(), look.pitch.to_bits())
}

#[test]
fn the_eye_stands_over_the_feet_at_eye_height() -> TestResult {
    let player = standing_at(FEET, Look::default());

    let camera = eye_pose(&player);

    let declared = [FEET.x, FEET.y + EYE_HEIGHT, FEET.z];
    assert!(
        furthest_axis(camera.eye, declared) <= EPSILON,
        "the eye stands at the feet's own horizontal position, {EYE_HEIGHT} blocks up, so it \
         belongs at {declared:?} and not at {:?}",
        camera.eye
    );
    Ok(())
}

#[test]
fn moving_the_player_moves_the_eye_by_the_same_displacement() -> TestResult {
    let before = eye_pose(&standing_at(FEET, OFF_AXIS));

    let after = eye_pose(&standing_at(FEET + DISPLACEMENT, OFF_AXIS));

    let travelled = separation(before.eye, after.eye);
    assert!(
        furthest_axis(travelled, DISPLACEMENT.to_array()) <= EPSILON,
        "the camera is derived from the player rather than driven beside it, so displacing the \
         feet by {:?} displaces the eye by exactly that and not by {travelled:?}",
        DISPLACEMENT.to_array()
    );
    Ok(())
}

#[test]
fn a_quarter_turn_of_yaw_points_the_camera_along_positive_z() -> TestResult {
    let turned = Look::default().accumulate(&looking(FRAC_PI_2, 0.0));

    let camera = eye_pose(&standing_at(FEET, turned));

    let looking_along = direction(&camera);
    let [sideways, _, forward] = looking_along;
    assert!(
        sideways.abs() <= EPSILON && forward > EPSILON,
        "yaw 0 faces +x and a quarter turn takes it to +z, so a quarter turn from level looks \
         along +z with no x left in it — not along {looking_along:?}, which is where a basis \
         with its sine and cosine exchanged, or its yaw negated, ends up"
    );
    Ok(())
}

#[test]
fn a_positive_pitch_delta_lifts_the_camera_target_above_its_eye() -> TestResult {
    let tilted = Look::default().accumulate(&looking(0.0, 0.1));

    let camera = eye_pose(&standing_at(FEET, tilted));

    let [_, rise, _] = direction(&camera);
    assert!(
        rise > EPSILON,
        "positive pitch looks up, so a target above the eye is the whole of it: this one sits \
         {rise} blocks above it"
    );
    Ok(())
}

#[test]
fn a_look_delta_that_is_not_a_finite_number_leaves_both_angles_untouched() -> TestResult {
    let mut moved = Vec::new();

    for (yaw_delta, pitch_delta) in NON_FINITE_DELTAS {
        let after = SETTLED.accumulate(&looking(yaw_delta, pitch_delta));
        if exactly(after) != exactly(SETTLED) {
            moved.push(format!(
                "({yaw_delta}, {pitch_delta}) left the view at ({}, {})",
                after.yaw, after.pitch
            ));
        }
    }

    assert!(
        moved.is_empty(),
        "a client's intent is the one thing that arrives from outside, and a NaN reaching either \
         accumulator poisons the player's view for the rest of the run and every frame after it. \
         A single non-finite delta leaves both angles at ({}, {}): {moved:?}",
        SETTLED.yaw,
        SETTLED.pitch
    );
    Ok(())
}

#[test]
fn pitch_driven_above_the_limit_reports_the_upper_limit() -> TestResult {
    let tilted = Look::default().accumulate(&looking(0.0, PAST_THE_LIMIT));

    let limit = PITCH_LIMIT_DEGREES.to_radians();
    assert!(
        (tilted.pitch - limit).abs() <= EPSILON,
        "a pitch delta of {PAST_THE_LIMIT} radians asks for more than the view may tilt, and the \
         answer is the limit itself — {PITCH_LIMIT_DEGREES}° is {limit} radians, not {}",
        tilted.pitch
    );
    Ok(())
}

#[test]
fn pitch_driven_below_the_limit_reports_the_lower_limit() -> TestResult {
    let tilted = Look::default().accumulate(&looking(0.0, -PAST_THE_LIMIT));

    let limit = -PITCH_LIMIT_DEGREES.to_radians();
    assert!(
        (tilted.pitch - limit).abs() <= EPSILON,
        "the clamp is symmetric, so looking as far down as it goes reaches −{PITCH_LIMIT_DEGREES}° \
         — {limit} radians, not {}",
        tilted.pitch
    );
    Ok(())
}

/// Why the limit is 89° and not 90°, asserted as the property rather than as the
/// number. At exactly the up axis the look direction has no horizontal component
/// at all and a view matrix has no unique answer; one degree short of it, the
/// horizontal component is `cos 89° = 0.0175`, which is small but is not zero
/// and is a hundred and seventy times this epsilon.
#[test]
fn at_either_pitch_limit_the_camera_still_looks_horizontally_as_well_as_vertically() -> TestResult {
    let mut degenerate = Vec::new();

    for driven in [PAST_THE_LIMIT, -PAST_THE_LIMIT] {
        let tilted = Look::default().accumulate(&looking(0.0, driven));
        let camera = eye_pose(&standing_at(FEET, tilted));
        let [sideways, rise, forward] = direction(&camera);
        if sideways.hypot(forward) <= EPSILON || rise.abs() <= EPSILON {
            degenerate.push(format!(
                "a pitch driven by {driven} looks along ({sideways}, {rise}, {forward})"
            ));
        }
    }

    assert!(
        degenerate.is_empty(),
        "the clamp stops the view short of the world's up axis, so at either limit the camera \
         still looks somewhere horizontally as well as up or down: {degenerate:?}"
    );
    Ok(())
}

#[test]
fn yaw_raised_past_a_full_turn_wraps_into_the_first_turn() -> TestResult {
    let turning = Look {
        yaw: 6.0,
        pitch: 0.0,
    };

    let turned = turning.accumulate(&looking(1.0, 0.0));

    assert!(
        (0.0..TAU).contains(&turned.yaw),
        "6.0 radians plus 1.0 is past a full turn, and a full turn is the same direction it \
         started from, so the answer belongs in [0, {TAU}) rather than growing to {}",
        turned.yaw
    );
    Ok(())
}

#[test]
fn yaw_lowered_below_zero_wraps_into_the_first_turn() -> TestResult {
    let turning = Look {
        yaw: 0.1,
        pitch: 0.0,
    };

    let turned = turning.accumulate(&looking(-0.5, 0.0));

    assert!(
        (0.0..TAU).contains(&turned.yaw),
        "turning left past zero is turning, not stopping: 0.1 radians less 0.5 belongs in \
         [0, {TAU}) rather than at {}",
        turned.yaw
    );
    Ok(())
}

/// The exact value the wrap produces, so that "inside the turn" above cannot be
/// satisfied by a clamp to zero or by any other answer that happens to be in
/// range. Derived rather than read off a run: 6.2 + 0.2 = 6.4, and 6.4 − 2π =
/// 0.116815.
#[test]
fn a_yaw_of_six_point_two_advanced_by_two_tenths_wraps_just_past_zero() -> TestResult {
    let turning = Look {
        yaw: 6.2,
        pitch: 0.0,
    };

    let turned = turning.accumulate(&looking(0.2, 0.0));

    assert!(
        (turned.yaw - WRAPPED_YAW).abs() <= EPSILON,
        "6.2 radians plus 0.2 is 6.4, which is {WRAPPED_YAW} past a full turn, so that is where \
         the view faces — not {}, and not the 6.4 an accumulator that never wrapped reports",
        turned.yaw
    );
    Ok(())
}
