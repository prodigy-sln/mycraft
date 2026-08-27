//! Which of the two things that may launch a tick wins when both are available,
//! and what a jump asked for by neither does.
//!
//! **A jump made from ground contact leaves at the player's own jump speed even
//! while the box is submerged.** A declared ascent governs only the jump the
//! *medium alone* admits — so every reading here is taken in pairs, one bit of
//! the state apart, over one volume and one medium. Two volumes would let a
//! difference be attributed to the fixture; one volume and one flag cannot.
//!
//! **A jump asked for in mid-air outside any swimmable block is the control this
//! whole feature is measured against**, and it is stated over a block rather
//! than over empty air on purpose. The absolute it ends at is `−2.5` only where
//! the resistance is zero: a resistant non-swimmable fixture satisfies the same
//! sentence, divides by `1 + r`, and lands somewhere else entirely — and five
//! such blocks already exist in this suite's own fixtures. Pinning the
//! declaration is what buys the equality, because a division by `1.0` is
//! bit-exact identity, and the assertion is written as one rather than as a
//! tolerance for exactly that reason.
//!
//! **It also reaches `launched` by a path nothing else here does.**
//! `player_ground.rs` asks the same question through a solidity-only fixture
//! whose medium is "nothing" unconditionally; this asks it through the resolved
//! medium view, which is the path a declared ascent travels and the one every
//! scenario about a medium takes.

mod support;

use std::error::Error;

use glam::Vec3;
use mc_sim::player::{MovementIntent, PlayerState, advance_player};

use support::TestResult;
use support::medium::{
    CLEAR, EXTENT, FEET, FLOOR_TOP, LIFTING, LIFTING_ASCENT, LIFTING_NOT_AT_ALL,
    LIFTING_RESISTANCE, PLAIN_STONE, flooded, floored, hollow, resolved,
};
use support::volume::Cells;

/// How far two figures this feature calls equal may differ, in blocks per
/// second. The specification's declared comparison epsilon.
const EPSILON: f32 = 1e-4;

/// How long one tick simulates, in seconds. Declared, never measured.
const TICK_DURATION: f32 = 1.0 / 60.0;

/// How fast falling accelerates, in blocks per second squared. Declared.
const GRAVITY: f32 = 30.0;

/// How fast the player's own jump leaves whatever launched it, in blocks per
/// second. Declared.
const JUMP_SPEED: f32 = 9.0;

/// What one tick of gravity takes from a launch before anything reads it.
const ONE_TICK_OF_GRAVITY: f32 = GRAVITY * TICK_DURATION;

/// Where the player's own jump leaves a tick in open air: `9.0 − 0.5`.
const A_JUMP_IN_OPEN_AIR: f32 = JUMP_SPEED - ONE_TICK_OF_GRAVITY;

/// Where the player's own jump leaves a tick under [`LIFTING_RESISTANCE`]:
/// `(9.0 − 0.5) / 1.5`, which the specification states as `5.6667`.
const A_JUMP_UNDER_THAT_RESISTANCE: f32 = 17.0 / 3.0;

/// The downward speed the mid-air control begins its tick at.
const FALLING_AT: f32 = -2.0;

/// Where one tick of gravity leaves that fall, through a volume that resists
/// nothing: `−2.0 − 0.5`, divided by one.
const STILL_FALLING: f32 = FALLING_AT - ONE_TICK_OF_GRAVITY;

/// A player standing on the floor at [`FEET`], at yaw 0.
fn standing() -> PlayerState {
    PlayerState {
        position: FEET,
        velocity: Vec3::ZERO,
        yaw: 0.0,
        pitch: 0.0,
        on_ground: true,
    }
}

/// An intent that asks for a jump and nothing else.
fn jumping() -> MovementIntent {
    MovementIntent {
        jump: true,
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

/// A floor, with `block` filling every row above it — so a player at [`FEET`] is
/// both standing on the floor and submerged in `block`.
fn submerged_over_the_floor(block: &str) -> Result<Cells, Box<dyn Error>> {
    flooded(floored(PLAIN_STONE)?, FLOOR_TOP, EXTENT.y, block)
}

/// The vertical velocities a jump leaves the tick at over one volume, taken with
/// ground contact and without it: one volume, one medium, one bit apart.
fn with_and_without_contact(volume: &Cells) -> Result<(f32, f32), Box<dyn Error>> {
    let grounded = one_tick(volume, standing(), &jumping())?;
    let adrift = one_tick(
        volume,
        PlayerState {
            on_ground: false,
            ..standing()
        },
        &jumping(),
    )?;
    Ok((grounded.velocity.y, adrift.velocity.y))
}

/// Whether two figures this feature calls equal are within the declared epsilon.
fn near(measured: f32, declared: f32) -> bool {
    (measured - declared).abs() <= EPSILON
}

#[test]
fn a_jump_from_the_ground_inside_a_lifting_block_leaves_at_the_players_own_jump_speed() -> TestResult
{
    let (grounded, adrift) = with_and_without_contact(&submerged_over_the_floor(LIFTING)?)?;

    assert!(
        (grounded - adrift).abs() > EPSILON,
        "the control: this block declares an ascent of {LIFTING_ASCENT} that the medium alone \
         does admit, so the two readings are not the same reading — {grounded} against {adrift} — \
         and what follows is a claim about ground contact winning rather than about a fixture \
         whose declared ascent nothing ever reads"
    );
    assert!(
        near(grounded, A_JUMP_UNDER_THAT_RESISTANCE),
        "ground contact beats the medium: a jump made from the floor leaves at the player's own \
         {JUMP_SPEED}, resisted by the {LIFTING_RESISTANCE} it is submerged in, so the tick ends \
         at {A_JUMP_UNDER_THAT_RESISTANCE} — the specification's 5.6667 blocks per second — and \
         not at the {adrift} the block's own ascent gives a swimmer. Measured {grounded}"
    );
    Ok(())
}

#[test]
fn a_jump_from_the_ground_in_open_air_leaves_at_the_players_own_jump_speed() -> TestResult {
    let launched = one_tick(&floored(PLAIN_STONE)?, standing(), &jumping())?;

    assert!(
        near(launched.velocity.y, A_JUMP_IN_OPEN_AIR),
        "nothing this feature adds reaches a jump made on land: the tick sets the declared \
         {JUMP_SPEED} and gravity takes its first {ONE_TICK_OF_GRAVITY} of it before the state \
         can be read, leaving {A_JUMP_IN_OPEN_AIR} exactly as it always has. Measured {}",
        launched.velocity.y
    );
    Ok(())
}

#[test]
fn a_jump_asked_for_while_falling_through_a_block_nobody_swims_in_neither_lifts_nor_arrests()
-> TestResult {
    let volume = flooded(hollow(), 0, EXTENT.y, CLEAR)?;
    let falling = PlayerState {
        velocity: Vec3::new(0.0, FALLING_AT, 0.0),
        on_ground: false,
        ..standing()
    };

    let asked = one_tick(&volume, falling, &jumping())?;
    let unasked = one_tick(&volume, falling, &MovementIntent::default())?;

    assert_eq!(
        asked.velocity.y.to_bits(),
        unasked.velocity.y.to_bits(),
        "a jump asked for in mid-air outside any swimmable block changes nothing about the tick, \
         so the request and its absence leave the fall in the same bits: {} against {}",
        asked.velocity.y,
        unasked.velocity.y
    );
    assert_eq!(
        asked.velocity.y.to_bits(),
        STILL_FALLING.to_bits(),
        "and that is gravity's work and nothing else — a fall of {FALLING_AT} through a volume \
         declaring no resistance is divided by one, which is bit-exact identity, so the tick ends \
         at exactly {STILL_FALLING} rather than nearly it. Measured {}",
        asked.velocity.y
    );
    Ok(())
}

#[test]
fn a_jump_from_the_ground_inside_a_block_declaring_no_lift_at_all_still_leaves_at_the_jump_speed()
-> TestResult {
    let (grounded, adrift) =
        with_and_without_contact(&submerged_over_the_floor(LIFTING_NOT_AT_ALL)?)?;

    assert!(
        (grounded - adrift).abs() > EPSILON,
        "the control: a declared ascent of zero is a launch speed of zero rather than the absence \
         of one, so the medium alone does answer this tick and answers it differently — \
         {grounded} against {adrift}"
    );
    assert!(
        near(grounded, A_JUMP_UNDER_THAT_RESISTANCE),
        "an ascent of zero notwithstanding, a jump made from the floor is the player's own: it \
         leaves at {JUMP_SPEED} resisted by {LIFTING_RESISTANCE}, ending the tick at \
         {A_JUMP_UNDER_THAT_RESISTANCE} — the specification's 5.6667 — rather than being arrested \
         to the {adrift} the block would give a swimmer. Measured {grounded}"
    );
    Ok(())
}
