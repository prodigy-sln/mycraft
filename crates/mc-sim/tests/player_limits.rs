//! What the tick refuses: a movement request larger than full deflection, a
//! request that is not a number at all, and a displacement larger than the bound
//! per-axis resolution is only exact within.
//!
//! **Every limit here is on the receiving side, and that is the whole point.**
//! The client accumulates input and hands over a request; what comes of it is
//! the simulation's answer. So a client asking to walk a thousand times as hard
//! as the keyboard can express, and a client whose arithmetic produced a NaN,
//! must get the same answer a well-behaved one gets — otherwise the authority
//! boundary is a promise rather than a structure, and a single non-finite intent
//! poisons the player state permanently and every frame after it.
//!
//! **The magnitude clamp is a cap, not a normalisation.** A request is a
//! direction *and* a magnitude: full deflection on two axes is still one walk
//! (`min(1, ‖(forward, strafe)‖)`), and half deflection on one is half a walk.
//! An implementation that normalised instead would satisfy the diagonal and the
//! absurd request and turn a half-pressed stick into a sprint, so the half
//! request below is what separates the two and is not decoration.
//!
//! **The displacement clamp is on the displacement and never on the velocity.**
//! The velocity is what a scenario reads back off the state, so clamping it
//! would report a fall that is not happening. What the bound protects is
//! resolution: a tick that moves the box less than a block on each axis can only
//! newly overlap the adjacent voxel layer, which is what makes resolving one
//! axis at a time exact rather than approximately exact.
//!
//! That distinction is only *observable* on the vertical axis, which is what the
//! declared state below is stated against. The tick **sets** horizontal velocity
//! from the intent, so a declared horizontal velocity is discarded before the
//! bound can see it and clamping the velocity would be indistinguishable from
//! clamping the displacement. A declared vertical velocity survives step 3, so
//! the tick can be caught reporting a speed the bound quietly took from it.
//!
//! Nothing below is read off a run. The declared constants are the walk speed,
//! the tick duration, the terminal speed and the one-block bound; every expected
//! figure is written as arithmetic over them.

mod support;

use std::error::Error;

use glam::Vec3;
use mc_sim::player::{MovementIntent, PlayerState, advance_player};

use support::solidity::Ground;

type TestResult = Result<(), Box<dyn Error>>;

/// How far two figures this feature calls equal may differ, in blocks. The
/// specification's declared comparison epsilon.
const EPSILON: f32 = 1e-4;

/// How long one tick simulates, in seconds. Declared, never measured.
const TICK_DURATION: f32 = 1.0 / 60.0;

/// How fast a walk carries the player, in blocks per second. Declared.
const WALK_SPEED: f32 = 4.5;

/// The fastest a fall ever goes, in blocks per second. Declared.
const TERMINAL_SPEED: f32 = 48.0;

/// How far a tick may displace the player on any one axis, in blocks. Declared.
const DISPLACEMENT_LIMIT: f32 = 1.0;

/// How far one tick of a walk at full deflection carries the player, in blocks.
///
/// The declared speed for the declared quantum, written as that product rather
/// than as the 0.075 it comes to.
const FULL_DEFLECTION: f32 = WALK_SPEED * TICK_DURATION;

/// A request half way to full deflection — a stick pushed half over.
const HALF_REQUEST: f32 = 0.5;

/// A request no input device can produce and no honest client would send.
const ABSURD_REQUEST: f32 = 1000.0;

/// How fast falling accelerates, in blocks per second squared. Declared.
const GRAVITY: f32 = 30.0;

/// The upward velocity the declared state below carries, in blocks per second.
///
/// Nearly five blocks a tick, so the bound has something to bite on. No intent
/// can ask for it — full deflection is 4.5 blocks per second and a jump leaves
/// at 9.0 — which is why this scenario is stated against a declared state rather
/// than against a request.
const DECLARED_SPEED: f32 = 300.0;

/// What that state's vertical velocity reads after the tick, in blocks per
/// second.
///
/// Gravity's one bite and nothing else. The bound takes nothing from it: it acts
/// on the displacement the velocity asks for, not on the velocity itself, so the
/// state keeps reporting how fast the player is actually going. A tick reporting
/// 60 here has clamped the wrong thing and would tell every later scenario — the
/// terminal fall, the jump's arc — a speed the player does not have.
const UNDIMINISHED_SPEED: f32 = DECLARED_SPEED - GRAVITY * TICK_DURATION;

/// How long the declared fall and the declared walk that follows it each run.
const FALLING_TICKS: u32 = 200;
const WALKING_TICKS: u32 = 200;

/// How far one tick at the terminal speed carries a fall, in blocks.
///
/// The largest displacement the declared constants can produce, and the figure
/// the bound has to sit above for per-axis resolution to be exact by derivation
/// rather than by clamping.
const TERMINAL_DISPLACEMENT: f32 = TERMINAL_SPEED * TICK_DURATION;

/// The topmost solid voxel of the flat floor every walk below happens on, and
/// where its top face — and so a standing player's feet — therefore is.
const FLOOR_SURFACE: i32 = 63;
const FLOOR_TOP: f32 = (FLOOR_SURFACE + 1) as f32;

/// Where the player's feet start.
///
/// Off-lattice on both horizontal axes and different on each, so a move resolved
/// on the axis it did not mean has nowhere to hide.
const START: Vec3 = Vec3::new(10.5, FLOOR_TOP, 3.5);

/// Every way a client can ask to walk with a number that is not one.
///
/// Each pairs the non-finite value with a *finite* request on the other axis,
/// and that pairing is what makes the table discriminating: sanitising the
/// offending axis alone would leave the finite one to move the player, so an
/// implementation that zeroed only what it could not read is caught here rather
/// than in some later phase. Both infinities appear because a magnitude clamp
/// written as `min(1, ‖·‖)` swallows `+∞` while leaving `−∞` and `NaN` through.
const NOT_A_NUMBER: [(f32, f32); 4] = [
    (f32::NAN, 1.0),
    (1.0, f32::NAN),
    (f32::INFINITY, 1.0),
    (1.0, f32::NEG_INFINITY),
];

/// The flat floor every walk below happens on.
fn floor() -> Ground {
    Ground::Flat {
        surface: FLOOR_SURFACE,
    }
}

/// A player standing still on the floor, facing +x.
fn standing() -> PlayerState {
    PlayerState {
        position: START,
        velocity: Vec3::ZERO,
        yaw: 0.0,
        pitch: 0.0,
        on_ground: true,
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

/// How far the player travelled along the ground, whatever direction it took.
fn horizontal_distance(from: &PlayerState, to: &PlayerState) -> f32 {
    let travelled = to.position - from.position;
    travelled.with_y(0.0).length()
}

/// A horizontal position as the integers its floats are. "Unchanged" means the
/// same value, not a nearly equal one.
fn exactly(state: &PlayerState) -> (u32, u32) {
    (state.position.x.to_bits(), state.position.z.to_bits())
}

/// The largest distance any one axis covered between two positions.
fn largest_axis(from: Vec3, to: Vec3) -> f32 {
    (to - from).abs().max_element()
}

#[test]
fn a_walk_asked_for_on_both_axes_at_once_covers_no_more_ground_than_one_axis() -> TestResult {
    let floor = floor();
    let start = standing();

    let walked = advance_player(start, &walking(1.0, 1.0), &floor);

    let distance = horizontal_distance(&start, &walked);
    assert!(
        (distance - FULL_DEFLECTION).abs() <= EPSILON,
        "a request is a direction and a magnitude, and the magnitude is capped at one before the \
         walk speed scales it, so forward and strafe both at full deflection is still one walk of \
         {FULL_DEFLECTION} blocks — not the {} that taking the request as two independent \
         distances gives, which is a diagonal sprint no straight walk can match. This tick \
         covered {distance}",
        FULL_DEFLECTION * std::f32::consts::SQRT_2
    );
    Ok(())
}

#[test]
fn a_walk_asked_for_far_beyond_full_deflection_covers_exactly_full_deflection() -> TestResult {
    let floor = floor();
    let start = standing();

    let walked = advance_player(start, &walking(ABSURD_REQUEST, 0.0), &floor);

    let distance = horizontal_distance(&start, &walked);
    assert!(
        (distance - FULL_DEFLECTION).abs() <= EPSILON,
        "the clamp is on the receiving side, so a client asking for {ABSURD_REQUEST} gets exactly \
         what a client asking for 1.0 gets: {FULL_DEFLECTION} blocks. This tick covered \
         {distance}, and a simulation that took the request at its word would let any client \
         teleport by asking loudly enough"
    );
    Ok(())
}

#[test]
fn a_walk_asked_for_at_half_deflection_covers_half_the_ground() -> TestResult {
    let floor = floor();
    let start = standing();

    let walked = advance_player(start, &walking(HALF_REQUEST, 0.0), &floor);

    let distance = horizontal_distance(&start, &walked);
    let expected = HALF_REQUEST * FULL_DEFLECTION;
    assert!(
        (distance - expected).abs() <= EPSILON,
        "the magnitude is capped, not normalised: below full deflection the request is scaled by \
         what it asks for, so half a request is {expected} blocks and not the {FULL_DEFLECTION} a \
         normalising implementation would give — which would satisfy every other request in this \
         file while turning a half-pressed stick into a sprint. This tick covered {distance}"
    );
    Ok(())
}

#[test]
fn a_walk_asked_for_with_a_number_that_is_not_one_moves_the_player_nowhere() -> TestResult {
    let floor = floor();
    let start = standing();
    let mut moved = Vec::new();

    for (forward, strafe) in NOT_A_NUMBER {
        let after = advance_player(start, &walking(forward, strafe), &floor);
        if exactly(&after) != exactly(&start) {
            moved.push(format!(
                "forward {forward} strafe {strafe} left it at ({}, {})",
                after.position.x, after.position.z
            ));
        }
    }

    assert!(
        moved.is_empty(),
        "a magnitude that is not a finite number is a client fact, not a server error: the whole \
         request is dropped and the player stays at ({}, {}) horizontally — both axes, because a \
         request half of which cannot be read cannot be trusted on the other half either. A NaN \
         allowed through poisons the position permanently, and every tick after it. {} of {} \
         requests moved the player, the first {:?}",
        START.x,
        START.z,
        moved.len(),
        NOT_A_NUMBER.len(),
        moved.first()
    );
    Ok(())
}

#[test]
fn a_tick_moves_the_player_no_further_than_a_block_while_still_reporting_its_full_speed()
-> TestResult {
    let declared = PlayerState {
        position: START,
        velocity: Vec3::new(0.0, DECLARED_SPEED, 0.0),
        yaw: 0.0,
        pitch: 0.0,
        on_ground: false,
    };

    let moved = advance_player(declared, &MovementIntent::default(), &Ground::Void);

    let displaced = moved.position.y - declared.position.y;
    assert!(
        (displaced - DISPLACEMENT_LIMIT).abs() <= EPSILON
            && (moved.velocity.y - UNDIMINISHED_SPEED).abs() <= EPSILON,
        "resolving one axis at a time is only exact while a tick moves the box less than a block \
         on each, because only then can the box newly overlap the adjacent voxel layer. A state \
         carrying {UNDIMINISHED_SPEED} blocks per second asks for {} blocks in a tick and would \
         tunnel through everything between, so the tick moves it {DISPLACEMENT_LIMIT} and no \
         further — and still reports {UNDIMINISHED_SPEED}, because what the bound took was the \
         displacement and not the speed. This tick moved {displaced} and reported {}",
        UNDIMINISHED_SPEED * TICK_DURATION,
        moved.velocity.y
    );
    Ok(())
}

#[test]
fn neither_a_long_fall_nor_a_long_walk_ever_reaches_that_limit() -> TestResult {
    let mut state = PlayerState {
        on_ground: false,
        ..standing()
    };
    let mut largest = 0.0_f32;

    for tick in 0..FALLING_TICKS + WALKING_TICKS {
        let intent = if tick < FALLING_TICKS {
            MovementIntent::default()
        } else {
            walking(1.0, 0.0)
        };
        let after = advance_player(state, &intent, &Ground::Void);
        largest = largest.max(largest_axis(state.position, after.position));
        state = after;
    }

    assert!(
        (largest - TERMINAL_DISPLACEMENT).abs() <= EPSILON && largest < DISPLACEMENT_LIMIT,
        "under the declared constants the biggest a tick ever moves the box is one tick of the \
         terminal fall, {TERMINAL_DISPLACEMENT} blocks, which is comfortably inside the \
         {DISPLACEMENT_LIMIT} bound — so per-axis resolution is exact by derivation and the bound \
         is a guard against a constant changing later, not a thing the product runs into. The \
         largest of these {} ticks was {largest}; a value at the bound means the guard is load \
         bearing and the derivation is gone",
        FALLING_TICKS + WALKING_TICKS
    );
    Ok(())
}
