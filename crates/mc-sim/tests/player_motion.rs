//! What one tick does to the player's velocity: the basis a walk is expressed
//! in, and the gravity that acts on a body with nothing under it.
//!
//! **The movement basis is the contract, and it is asserted signed.** Yaw 0
//! faces +x, strafe-right at yaw 0 is +z, and a quarter turn of yaw takes
//! forward to +z. A strafe implemented along −z, and a basis with its sine and
//! cosine exchanged, are both smooth, total, reproducible and wrong — and both
//! satisfy any assertion written about how *far* the player went. Every walk
//! below therefore asserts the displacement as a signed pair on both horizontal
//! axes: a sign flip lands 9 blocks from the expectation, and a transposed basis
//! lands on the wrong axis entirely.
//!
//! **The basis is horizontal, and it stays horizontal.** Where the player looks
//! decides which way forward points and never whether forward has a vertical
//! component: forward is `(cos yaw, 0, sin yaw)` with no pitch anywhere in it.
//! The two walks taken at the pitch limits are what say so, and they assert the
//! height as well as the distance — a basis built from the full look direction
//! flies the player into the sky at one limit and into the floor at the other,
//! and covers a fraction of the ground at both, so a test asserting only how far
//! it went along x would report the shortfall without ever naming the cause.
//!
//! **Nothing here is read off a run.** The declared constants are the walk speed,
//! the tick duration, the gravitational acceleration and the terminal speed;
//! every expected figure is arithmetic over them, written as the arithmetic
//! rather than as its result. The absence of acceleration, friction and inertia
//! is what makes that possible — horizontal displacement is exactly
//! `speed × ticks × tick duration`, with no integration to reproduce.
//!
//! Comparisons use the declared 1 × 10⁻⁴ epsilon, except where *unchanged* is
//! the claim: that is a question about bits, which is both its exact form and
//! the form `clippy::float_cmp` has no quarrel with
//! (`tests/support/mod.rs::exactly` is the same idiom).

mod support;

use std::error::Error;
use std::f32::consts::FRAC_PI_2;

use glam::Vec3;
use mc_sim::player::{MovementIntent, PlayerState, Traversal, advance_player};

use support::solidity::Ground;

type TestResult = Result<(), Box<dyn Error>>;

/// How far two figures this feature calls equal may differ, in blocks or in
/// blocks per second. The specification's declared comparison epsilon.
const EPSILON: f32 = 1e-4;

/// How long one tick simulates, in seconds. Declared, never measured.
const TICK_DURATION: f32 = 1.0 / 60.0;

/// How fast a walk carries the player, in blocks per second. Declared.
const WALK_SPEED: f32 = 4.5;

/// How fast falling accelerates, in blocks per second squared. Declared.
const GRAVITY: f32 = 30.0;

/// The fastest a fall ever goes, in blocks per second. Declared.
const TERMINAL_SPEED: f32 = 48.0;

/// How long each walk below is held for: one second of ticks.
const HELD_TICKS: u32 = 60;

/// How far a walk held for [`HELD_TICKS`] covers, in blocks.
///
/// The declared speed for the declared time, written as that product rather than
/// as the 4.5 it comes to — a distance copied from a run of the subject would
/// commit whatever the subject did on the day it was copied.
const WALK_DISTANCE: f32 = WALK_SPEED * TICK_DURATION * HELD_TICKS as f32;

/// How much speed one tick of gravity adds to a fall, in blocks per second.
const FALL_PER_TICK: f32 = GRAVITY * TICK_DURATION;

/// How many ticks of gravity reach the terminal speed.
///
/// The declaration's own figure: 48.0 blocks per second at 0.5 of them per tick
/// is 96 ticks, and the boundary is what makes it worth asking — a tick earlier
/// the fall is still accelerating.
const TICKS_TO_TERMINAL: u32 = 96;

/// How far pitch is allowed from the horizon, in radians. The declared ±89°,
/// written as the conversion rather than as the 1.5533 it comes to.
const PITCH_LIMIT: f32 = 89.0 * std::f32::consts::PI / 180.0;

/// How long the airborne walk below is held for: half a second of ticks.
const AIRBORNE_TICKS: u32 = 30;

/// How far a walk held for [`AIRBORNE_TICKS`] covers, in blocks.
const AIRBORNE_DISTANCE: f32 = WALK_SPEED * TICK_DURATION * AIRBORNE_TICKS as f32;

/// The topmost solid voxel of the flat floor every walk below happens on.
const FLOOR_SURFACE: i32 = 63;

/// Where that floor's top face is, and so where a standing player's feet are.
const FLOOR_TOP: f32 = (FLOOR_SURFACE + 1) as f32;

/// Where the player's feet start.
///
/// Off-lattice on both horizontal axes and different on each, so that a walk
/// resolved on the axis it did not mean has nowhere to hide, and so that no
/// distance below is measured from a coordinate small enough to flatter the
/// arithmetic.
const START: Vec3 = Vec3::new(10.5, FLOOR_TOP, 3.5);

/// The flat floor every walk below happens on.
fn floor() -> Ground {
    Ground::Flat {
        surface: FLOOR_SURFACE,
    }
}

/// A player standing still on the floor, facing `yaw`.
fn standing(yaw: f32) -> PlayerState {
    PlayerState {
        position: START,
        velocity: Vec3::ZERO,
        yaw,
        pitch: 0.0,
        on_ground: true,
    }
}

/// A player standing still on the floor at yaw 0, looking `pitch` radians away
/// from the horizon.
fn looking(pitch: f32) -> PlayerState {
    PlayerState {
        pitch,
        ..standing(0.0)
    }
}

/// A player already falling at `speed`, with nothing at all beneath it.
fn falling(speed: f32) -> PlayerState {
    PlayerState {
        position: START,
        velocity: Vec3::new(0.0, speed, 0.0),
        yaw: 0.0,
        pitch: 0.0,
        on_ground: false,
    }
}

/// An intent that asks only to walk.
fn walking(forward: f32, strafe: f32) -> MovementIntent {
    MovementIntent {
        forward,
        strafe,
        ..MovementIntent::default()
    }
}

/// Where `ticks` submissions of `intent` leave `state`.
fn advance(
    state: PlayerState,
    intent: &MovementIntent,
    world: &dyn Traversal,
    ticks: u32,
) -> PlayerState {
    (0..ticks).fold(state, |state, _| advance_player(state, intent, world))
}

/// How far the player travelled horizontally, x then z.
fn travel(from: &PlayerState, to: &PlayerState) -> (f32, f32) {
    (
        to.position.x - from.position.x,
        to.position.z - from.position.z,
    )
}

/// Whether a travelled displacement is the declared one on both axes.
fn arrives_at(travelled: (f32, f32), declared: (f32, f32)) -> bool {
    (travelled.0 - declared.0).abs() <= EPSILON && (travelled.1 - declared.1).abs() <= EPSILON
}

/// A horizontal position as the integers its floats are. "Unchanged" means the
/// same value, not a nearly equal one.
fn exactly(state: &PlayerState) -> (u32, u32) {
    (state.position.x.to_bits(), state.position.z.to_bits())
}

/// Whether a walk left the height it started at exactly where it was.
fn stayed_level(from: &PlayerState, to: &PlayerState) -> bool {
    to.position.y.to_bits() == from.position.y.to_bits()
}

#[test]
fn holding_forward_for_a_second_walks_the_walk_speed_along_positive_x() -> TestResult {
    let floor = floor();
    let start = standing(0.0);

    let walked = advance(start, &walking(1.0, 0.0), &floor, HELD_TICKS);

    let travelled = travel(&start, &walked);
    assert!(
        arrives_at(travelled, (WALK_DISTANCE, 0.0)),
        "yaw 0 faces +x, and a walk is the declared speed for as long as it is held with no \
         acceleration to build up and no inertia to shed, so {HELD_TICKS} ticks of forward is \
         ({WALK_DISTANCE}, 0) and not {travelled:?}"
    );
    Ok(())
}

#[test]
fn an_intent_asking_for_nothing_leaves_the_walk_exactly_where_it_stopped() -> TestResult {
    let floor = floor();
    let moved = advance_player(standing(0.0), &walking(1.0, 0.0), &floor);
    let mut state = moved;
    let mut drifted = Vec::new();

    for tick in 1..=HELD_TICKS {
        state = advance_player(state, &MovementIntent::default(), &floor);
        if exactly(&state) != exactly(&moved) {
            drifted.push(format!(
                "tick {tick} left it at ({}, {})",
                state.position.x, state.position.z
            ));
        }
    }

    assert!(
        drifted.is_empty(),
        "horizontal velocity is set from the intent every tick rather than accumulated, so a \
         player that moved and is then asked for nothing stops dead at ({}, {}) — it does not \
         coast, and it does not creep: {} of {HELD_TICKS} ticks moved it, the first {:?}",
        moved.position.x,
        moved.position.z,
        drifted.len(),
        drifted.first()
    );
    Ok(())
}

#[test]
fn holding_strafe_right_walks_the_same_distance_along_positive_z() -> TestResult {
    let floor = floor();
    let start = standing(0.0);

    let walked = advance(start, &walking(0.0, 1.0), &floor, HELD_TICKS);

    let travelled = travel(&start, &walked);
    assert!(
        arrives_at(travelled, (0.0, WALK_DISTANCE)),
        "strafe-right at yaw 0 is +z, so this walk belongs at (0, {WALK_DISTANCE}) and not at \
         {travelled:?} — a right vector negated lands the same distance on the wrong side, which \
         is exactly the defect a test asserting only how far it went would let through"
    );
    Ok(())
}

#[test]
fn holding_back_walks_the_same_distance_the_other_way_along_x() -> TestResult {
    let floor = floor();
    let start = standing(0.0);

    let walked = advance(start, &walking(-1.0, 0.0), &floor, HELD_TICKS);

    let travelled = travel(&start, &walked);
    assert!(
        arrives_at(travelled, (-WALK_DISTANCE, 0.0)),
        "walking back is walking forward with the sign of the request, so it belongs at \
         ({}, 0) and not at {travelled:?}",
        -WALK_DISTANCE
    );
    Ok(())
}

#[test]
fn a_quarter_turn_of_yaw_sends_forward_along_positive_z() -> TestResult {
    let floor = floor();
    let start = standing(FRAC_PI_2);

    let walked = advance(start, &walking(1.0, 0.0), &floor, HELD_TICKS);

    let travelled = travel(&start, &walked);
    assert!(
        arrives_at(travelled, (0.0, WALK_DISTANCE)),
        "forward is (cos yaw, 0, sin yaw), so a quarter turn takes it from +x to +z: this walk \
         belongs at (0, {WALK_DISTANCE}) and not at {travelled:?}, which is where a basis with \
         its sine and cosine exchanged ends up"
    );
    Ok(())
}

#[test]
fn one_tick_of_free_fall_leaves_the_player_falling_at_half_a_block_a_second() -> TestResult {
    let fallen = advance_player(falling(0.0), &MovementIntent::default(), &Ground::Void);

    assert!(
        (fallen.velocity.y + FALL_PER_TICK).abs() <= EPSILON,
        "one tick of the declared {GRAVITY} blocks per second squared is {FALL_PER_TICK} blocks \
         per second of fall, so a player let go from rest is falling at that and not at {}",
        fallen.velocity.y
    );
    Ok(())
}

#[test]
fn a_fall_of_ninety_six_ticks_reaches_the_terminal_speed() -> TestResult {
    let fallen = advance(
        falling(0.0),
        &MovementIntent::default(),
        &Ground::Void,
        TICKS_TO_TERMINAL,
    );

    assert!(
        (fallen.velocity.y + TERMINAL_SPEED).abs() <= EPSILON,
        "{TICKS_TO_TERMINAL} ticks of {FALL_PER_TICK} blocks per second each is the declared \
         terminal speed of {TERMINAL_SPEED} exactly, so that is what a fall of that length \
         reports — not {}",
        fallen.velocity.y
    );
    Ok(())
}

#[test]
fn a_player_already_at_the_terminal_speed_falls_no_faster() -> TestResult {
    let fallen = advance_player(
        falling(-TERMINAL_SPEED),
        &MovementIntent::default(),
        &Ground::Void,
    );

    assert!(
        (fallen.velocity.y + TERMINAL_SPEED).abs() <= EPSILON,
        "the terminal speed is where gravity stops adding: a player already at {TERMINAL_SPEED} \
         blocks per second is still at it a tick later, and not at {} — which is also what keeps \
         a tick's displacement inside the one block per axis that makes per-axis resolution exact",
        fallen.velocity.y
    );
    Ok(())
}

#[test]
fn a_player_resting_on_the_floor_reports_no_vertical_velocity() -> TestResult {
    let floor = floor();
    let mut state = standing(0.0);
    let mut accumulated = Vec::new();

    for tick in 1..=HELD_TICKS {
        state = advance_player(state, &MovementIntent::default(), &floor);
        if state.velocity.y.abs() > EPSILON {
            accumulated.push(format!("tick {tick} reports {}", state.velocity.y));
        }
    }

    assert!(
        accumulated.is_empty(),
        "gravity acts on a resting player every tick and the floor answers it every tick, so the \
         velocity a tick ends with is zero rather than a downward speed that grows for as long as \
         the player stands still: {} of {HELD_TICKS} ticks reported one, the first {:?}",
        accumulated.len(),
        accumulated.first()
    );
    Ok(())
}

#[test]
fn a_walk_taken_looking_at_the_sky_covers_the_same_ground_at_the_same_height() -> TestResult {
    let floor = floor();
    let start = looking(PITCH_LIMIT);

    let walked = advance(start, &walking(1.0, 0.0), &floor, HELD_TICKS);

    let travelled = travel(&start, &walked);
    assert!(
        arrives_at(travelled, (WALK_DISTANCE, 0.0)) && stayed_level(&start, &walked),
        "forward is (cos yaw, 0, sin yaw) and pitch is nowhere in it, so a player looking as far \
         up as the limit allows walks the same ({WALK_DISTANCE}, 0) along the same floor. This \
         one travelled {travelled:?} and ended at height {} rather than {FLOOR_TOP} — a basis \
         taking the whole look direction flies where it looks and covers a fraction of the ground \
         doing it",
        walked.position.y
    );
    Ok(())
}

#[test]
fn a_walk_taken_looking_at_the_floor_covers_the_same_ground_at_the_same_height() -> TestResult {
    let floor = floor();
    let start = looking(-PITCH_LIMIT);

    let walked = advance(start, &walking(1.0, 0.0), &floor, HELD_TICKS);

    let travelled = travel(&start, &walked);
    assert!(
        arrives_at(travelled, (WALK_DISTANCE, 0.0)) && stayed_level(&start, &walked),
        "the other limit answers the same way, and it is the one a floor would hide: a basis \
         built from the look direction drives the player down into a floor that stops it every \
         tick, so its height stays right while its distance quietly shrinks. This walk belongs at \
         ({WALK_DISTANCE}, 0) and height {FLOOR_TOP}, not at {travelled:?} and {}",
        walked.position.y
    );
    Ok(())
}

#[test]
fn holding_forward_in_mid_air_covers_the_same_ground_as_holding_it_on_foot() -> TestResult {
    let start = falling(0.0);

    let walked = advance(start, &walking(1.0, 0.0), &Ground::Void, AIRBORNE_TICKS);

    let travelled = travel(&start, &walked);
    assert!(
        arrives_at(travelled, (AIRBORNE_DISTANCE, 0.0)),
        "horizontal velocity is set from the intent every tick and the floor has no say in it, so \
         {AIRBORNE_TICKS} ticks of forward carries a falling player the same \
         {AIRBORNE_DISTANCE} blocks it carries a standing one — not {travelled:?}, which is where \
         a walk that needed the ground under it to work ends up"
    );
    Ok(())
}
