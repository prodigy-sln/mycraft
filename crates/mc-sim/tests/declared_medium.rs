//! A declaration that says a block can be swum in, and one that says nothing,
//! asked through the door the running game actually uses.
//!
//! **The world, not a view a test resolved itself.** `simulation.rs` hands
//! `advance_player` a [`World`](mc_sim::world::World); a `ResolvedVoxels` built
//! in a test is a different object, and a world that built its view correctly
//! and then answered a medium question from somewhere else — the registry, a
//! constant, nothing at all — would leave every scenario asserted through the
//! view green. A second entry point onto a tested path is untested until
//! something asserts through it, so these three do.
//!
//! **What each fixture is shaped to catch.** A buoyancy the loader derived from
//! solidity, and one it derived from the absence of solidity, are two different
//! defects and neither is visible to the other's fixture: a block declaring no
//! `swimmable` is asked here once while stating `solid = true` and once while
//! stating `solid = false`, and exactly one of the two reddens under each.
//!
//! **The solid fixture seats the player's box overlapping a solid voxel, which
//! is a state the shipped game does not reach** — two rules keep the box out of
//! solid cells, and neither of them binds a test. It is stated anyway because it
//! is the only arrangement in which "a solid block does not confer buoyancy" is
//! observable at all, and its geometry is chosen so that the wrong answer moves:
//! the block sits in the row the box's feet share, low enough that a launched
//! tick's rise clears that row entirely (`7.9 + 0.1417 > 8.0`) and high enough
//! that the box shares it to begin with (`7.9 < 8.0`). A tick that wrongly
//! launches therefore ends free and higher, while both correct ticks are held
//! against the block's own top face at exactly `8.0`.

mod support;

use std::error::Error;

use glam::Vec3;
use mc_sim::player::{MovementIntent, PlayerState, advance_player};
use mc_sim::world::World;
use mc_world::world::WorldPos;

use support::TestResult;
use support::medium::{BUOYANT, CLEAR, FEET, PLAIN_STONE, world_holding};

/// How far two figures this feature calls equal may differ, in blocks. The
/// specification's declared comparison epsilon.
const EPSILON: f32 = 1e-4;

/// How long one tick simulates, in seconds. Declared, never measured.
const TICK_DURATION: f32 = 1.0 / 60.0;

/// How fast falling accelerates, in blocks per second squared. Declared.
const GRAVITY: f32 = 30.0;

/// How fast a jump leaves whatever launched it, in blocks per second. Declared.
const JUMP_SPEED: f32 = 9.0;

/// How far one tick of a launch carries the feet upward, in blocks. Gravity
/// takes its bite before the velocity moves the position.
const ONE_TICK_OF_RISE: f32 = (JUMP_SPEED - GRAVITY * TICK_DURATION) * TICK_DURATION;

/// The box a declared block fills in the worlds below: `x` in `[8, 12)`, `z` in
/// `[0, 6)`, sixteen rows up from the floor of the world.
///
/// [`FEET`]'s own columns lie inside it and their transpose does not, so a world
/// that answered about a box's z where it meant its x reads a column holding
/// nothing.
const FILLED_LOW: WorldPos = WorldPos { x: 8, y: 0, z: 0 };
const FILLED_HIGH: WorldPos = WorldPos { x: 12, y: 16, z: 6 };

/// The one solid voxel the last fixture's box shares a row with.
const SHARED_CELL: WorldPos = WorldPos { x: 10, y: 7, z: 3 };

/// Where the feet stand while the box overlaps [`SHARED_CELL`].
///
/// On the column boundary at `x = 11`, so the box spans columns 10 and 11 and
/// overlaps the block in the first while the second stays clear.
const FEET_SHARING_A_ROW: Vec3 = Vec3::new(11.0, 7.9, 3.5);

/// A player at rest with nothing holding it up, standing at `at`.
fn adrift(at: Vec3) -> PlayerState {
    PlayerState {
        position: at,
        velocity: Vec3::ZERO,
        yaw: 0.0,
        pitch: 0.0,
        on_ground: false,
    }
}

/// An intent that asks for a jump and nothing else.
fn jumping() -> MovementIntent {
    MovementIntent {
        jump: true,
        ..MovementIntent::default()
    }
}

/// A world holding `block` over the declared box and nothing else.
fn world_filled_with(block: &str) -> Result<World, Box<dyn Error>> {
    world_holding(&[(FILLED_LOW, FILLED_HIGH, block)])
}

/// The one-cell box beginning at `at`.
const fn one_cell_beyond(at: WorldPos) -> WorldPos {
    WorldPos {
        x: at.x + 1,
        y: at.y + 1,
        z: at.z + 1,
    }
}

/// Whether a tick that asked to jump ended higher than an identical tick that
/// asked for nothing.
///
/// The comparison and not the height, because a box held against a block can end
/// a tick higher than it began without any jump having been honoured — which is
/// exactly the arrangement the last fixture below is in.
fn a_jump_lifts(world: &World, from: PlayerState) -> bool {
    let jumped = advance_player(from, &jumping(), world).position.y;
    let unjumped = advance_player(from, &MovementIntent::default(), world)
        .position
        .y;
    jumped > unjumped
}

#[test]
fn a_block_declared_swimmable_carries_a_jump_asked_for_off_the_ground() -> TestResult {
    let world = world_filled_with(BUOYANT)?;
    let start = adrift(FEET);

    let swimming = advance_player(start, &jumping(), &world);

    assert!(
        (swimming.position.y - (start.position.y + ONE_TICK_OF_RISE)).abs() <= EPSILON,
        "a declaration stating swimmable registers a block a player can hold itself up in, and \
         the world the game runs on is what has to say so: while the box overlaps it, a jump off \
         the ground ends the tick higher than it began, at {} rather than {}",
        swimming.position.y,
        start.position.y + ONE_TICK_OF_RISE
    );
    Ok(())
}

#[test]
fn a_block_that_stops_nobody_and_declares_no_swimmable_refuses_a_jump_off_the_ground() -> TestResult
{
    let start = adrift(FEET);

    let lifted = (
        a_jump_lifts(&world_filled_with(BUOYANT)?, start),
        a_jump_lifts(&world_filled_with(CLEAR)?, start),
    );

    assert_eq!(
        lifted,
        (true, false),
        "a declaration that says nothing about swimmable registers a block nobody can swim in, so \
         a block that stops nobody is not swimmable by virtue of being the sort of thing a player \
         passes through. Read against the control beside it rather than as an absence: the first \
         reading is a block declared swimmable at this very position, so a world that honours \
         neither jump lands somewhere other than {lifted:?} just as one that honours both does"
    );
    Ok(())
}

#[test]
fn a_block_that_stops_a_player_and_declares_no_swimmable_refuses_a_jump_off_the_ground()
-> TestResult {
    let stopping = world_holding(&[(SHARED_CELL, one_cell_beyond(SHARED_CELL), PLAIN_STONE)])?;
    let start = adrift(FEET_SHARING_A_ROW);

    let lifted = (
        a_jump_lifts(&world_filled_with(BUOYANT)?, start),
        a_jump_lifts(&stopping, start),
    );

    assert_eq!(
        lifted,
        (true, false),
        "the other half of the same rule: a block that stops a player is not swimmable by virtue \
         of being solid either. Both readings are taken from the one position where the box \
         shares a row with the solid block, so the control says a jump is reachable from exactly \
         here — and a tick that wrongly launched would clear that row and end free, above the \
         tick that asked for nothing. {lifted:?}"
    );
    Ok(())
}
