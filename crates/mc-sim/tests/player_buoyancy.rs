//! What a block a player can hold itself up in does to a jump, and what it does
//! not do to anything else.
//!
//! **The two medium answers are independent declarations and the pair of tests
//! that says so is the point of this file.** A block declaring a resistance and
//! no buoyancy must not lift, and a block declaring buoyancy and no resistance
//! must not slow — and neither failure is visible from inside the physics, which
//! reads whatever the table hands it. Nothing else in the suite can tell a
//! medium that came from one field from a medium that came from both.
//!
//! **The jump is widened at the one site that already answers "may this tick
//! launch", never added as a second path.** So a player overlapping nothing
//! swimmable is asked the same question it has always been asked, and the test
//! for it compares a tick that asked to jump against an identical tick that did
//! not — which reports a second launch path even where the fall it competes with
//! happens to end at the same height.
//!
//! Every expected figure is arithmetic over the declared constants — the jump
//! speed, the gravity and the tick duration — written as the arithmetic rather
//! than as its result. Gravity takes its bite before the position moves, so a
//! tick that launches rises by `(jump − g·dt)·dt` and not by `jump·dt`.

mod support;

use std::error::Error;

use glam::Vec3;
use mc_sim::player::{MovementIntent, PlayerState, advance_player};

use support::TestResult;
use support::medium::{
    BUOYANT, EXTENT, FEET, FLOOR_TOP, PLAIN_STONE, THICKER, THICKER_RESISTANCE, flooded, floored,
    hollow, resolved,
};
use support::volume::Cells;

/// How far two figures this feature calls equal may differ, in blocks. The
/// specification's declared comparison epsilon.
const EPSILON: f32 = 1e-4;

/// How long one tick simulates, in seconds. Declared, never measured.
const TICK_DURATION: f32 = 1.0 / 60.0;

/// How fast a walk carries the player, in blocks per second. Declared.
const WALK_SPEED: f32 = 4.5;

/// How fast falling accelerates, in blocks per second squared. Declared.
const GRAVITY: f32 = 30.0;

/// How fast a jump leaves whatever launched it, in blocks per second. Declared.
const JUMP_SPEED: f32 = 9.0;

/// How far one tick of a launch carries the feet upward, in blocks.
///
/// Gravity is taken from the velocity before the velocity moves the position, so
/// the tick that launches rises by less than the jump speed alone would give.
const ONE_TICK_OF_RISE: f32 = (JUMP_SPEED - GRAVITY * TICK_DURATION) * TICK_DURATION;

/// How far one tick of an unpowered fall carries the feet downward, in blocks.
const ONE_TICK_OF_FALL: f32 = -GRAVITY * TICK_DURATION * TICK_DURATION;

/// A player at rest with nothing holding it up, at [`FEET`].
fn adrift() -> PlayerState {
    PlayerState {
        position: FEET,
        velocity: Vec3::ZERO,
        yaw: 0.0,
        pitch: 0.0,
        on_ground: false,
    }
}

/// A player standing still on the floor at yaw 0.
fn standing() -> PlayerState {
    PlayerState {
        on_ground: true,
        ..adrift()
    }
}

/// An intent that asks for a jump and nothing else.
fn jumping() -> MovementIntent {
    MovementIntent {
        jump: true,
        ..MovementIntent::default()
    }
}

/// An intent asking for a walk at full deflection and nothing else, which at
/// yaw 0 is a walk along +x.
fn walking_forward() -> MovementIntent {
    MovementIntent {
        forward: 1.0,
        ..MovementIntent::default()
    }
}

/// Where one submission of `intent` leaves `state`, in the world `volume`
/// declares.
fn one_tick(
    volume: &Cells,
    state: PlayerState,
    intent: &MovementIntent,
) -> Result<PlayerState, Box<dyn Error>> {
    Ok(advance_player(state, intent, &resolved(volume)?))
}

/// A volume with nothing solid anywhere, `block` filling every row of it.
fn swum(block: &str) -> Result<Cells, Box<dyn Error>> {
    flooded(hollow(), 0, EXTENT.y, block)
}

/// The same floor, with `block` filling every row above it.
fn over_the_floor(block: &str) -> Result<Cells, Box<dyn Error>> {
    flooded(floored(PLAIN_STONE)?, FLOOR_TOP, EXTENT.y, block)
}

/// Whether a tick that asked to jump ended higher than an identical tick that
/// asked for nothing.
///
/// The comparison and not the height, because a tick held against a block can
/// end higher than it began without any jump having been honoured.
fn a_jump_lifts(volume: &Cells, from: PlayerState) -> Result<bool, Box<dyn Error>> {
    let jumped = one_tick(volume, from, &jumping())?;
    let unjumped = one_tick(volume, from, &MovementIntent::default())?;
    Ok(jumped.position.y > unjumped.position.y)
}

/// Whether two figures this feature calls equal are within the declared epsilon.
fn near(measured: f32, declared: f32) -> bool {
    (measured - declared).abs() <= EPSILON
}

#[test]
fn a_jump_asked_for_in_midair_inside_a_swimmable_block_carries_the_player_upward() -> TestResult {
    let start = adrift();

    let swimming = one_tick(&swum(BUOYANT)?, start, &jumping())?;

    assert!(
        near(swimming.position.y, start.position.y + ONE_TICK_OF_RISE),
        "a jump is honoured on every tick the box overlaps a swimmable block, ground contact or \
         no ground contact, so the tick ends higher than it began by one tick of a launch that \
         has already paid its gravity: {} rather than {}",
        swimming.position.y,
        start.position.y + ONE_TICK_OF_RISE
    );
    Ok(())
}

#[test]
fn asking_for_no_jump_inside_a_swimmable_block_still_sinks() -> TestResult {
    let start = adrift();

    let lifted = one_tick(&swum(BUOYANT)?, start, &jumping())?;
    let sinking = one_tick(&swum(BUOYANT)?, start, &MovementIntent::default())?;

    assert!(
        lifted.position.y > start.position.y,
        "the control: this is a block a jump *is* honoured in, at {} against the {} it started \
         from, so what follows is a claim about a tick that asked for nothing rather than about \
         a fixture nothing could ever have lifted",
        lifted.position.y,
        start.position.y
    );
    assert!(
        near(sinking.position.y, start.position.y + ONE_TICK_OF_FALL),
        "being swimmable holds nobody up by itself — it widens what a *request* may do and \
         nothing else — so a tick that asks for no jump inside one falls by exactly the tick of \
         gravity it would fall through air: {} rather than {}",
        sinking.position.y,
        start.position.y + ONE_TICK_OF_FALL
    );
    Ok(())
}

#[test]
fn a_jump_asked_for_in_midair_outside_any_swimmable_block_changes_nothing_about_the_tick()
-> TestResult {
    let start = adrift();

    let lifted = (
        a_jump_lifts(&swum(BUOYANT)?, start)?,
        a_jump_lifts(&hollow(), start)?,
    );

    assert_eq!(
        lifted,
        (true, false),
        "a jump off the ground and clear of any swimmable block is refused exactly as it is \
         today. Read against the control beside it rather than as an absence: the first reading \
         is a block a jump is honoured in and the second is empty air, so an implementation \
         honouring neither and one honouring both each land somewhere other than {lifted:?}"
    );
    Ok(())
}

#[test]
fn a_jump_asked_for_in_midair_inside_a_resistant_block_that_nobody_can_swim_in_still_sinks()
-> TestResult {
    let start = adrift();

    let sinking = one_tick(&swum(THICKER)?, start, &jumping())?;
    let expected = start.position.y + ONE_TICK_OF_FALL / (1.0 + THICKER_RESISTANCE);

    assert!(
        near(sinking.position.y, expected),
        "a resistance is not a buoyancy: a block declaring {THICKER_RESISTANCE} and no swimmable \
         refuses the jump the way empty air does, and slows the fall that follows by its own \
         divisor rather than reversing it. {} rather than {}",
        sinking.position.y,
        expected
    );
    Ok(())
}

#[test]
fn a_walk_through_a_swimmable_block_that_states_no_resistance_covers_exactly_what_air_does()
-> TestResult {
    let air = one_tick(&floored(PLAIN_STONE)?, standing(), &walking_forward())?;
    let buoyant = one_tick(&over_the_floor(BUOYANT)?, standing(), &walking_forward())?;
    let thicker = one_tick(&over_the_floor(THICKER)?, standing(), &walking_forward())?;

    assert!(
        near(
            thicker.velocity.x * (1.0 + THICKER_RESISTANCE),
            air.velocity.x
        ),
        "the control: a block written over these very cells by this very route does slow this \
         walk, at {} against {}",
        thicker.velocity.x,
        air.velocity.x
    );
    assert!(
        near(air.velocity.x, WALK_SPEED),
        "the unresisted walk this is measured against is the declared {WALK_SPEED} blocks per \
         second rather than a figure read off a run: {}",
        air.velocity.x
    );
    assert_eq!(
        buoyant.velocity.to_array().map(f32::to_bits),
        air.velocity.to_array().map(f32::to_bits),
        "a buoyancy is not a resistance: a block declaring swimmable and stating no \
         `move_resistance` divides by one, which is the velocity itself in every bit, so the walk \
         through it is not nearly but exactly the walk through air. {:?} against {:?}",
        buoyant.velocity,
        air.velocity
    );
    Ok(())
}
