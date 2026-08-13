//! What the world reads as and what the player collides with, after a break.
//!
//! These two are a pair and neither half works alone. The first alone goes green
//! against a world whose solidity was never blocking anybody, so the second runs
//! the identical drive over the identical fixture with the break left out and
//! requires the player to be stopped. Between them a store written without its
//! collision view, and a collision view that never blocked, are both red.
//!
//! # The fixture, and the arithmetic every number here comes from
//!
//! One block stands at (10, 10, 8), directly in the path of a player whose feet
//! start at (8.5, 10.0, 8.5) on a floor whose top face is y = 10. The player's box
//! is 0.6 blocks across and 1.8 tall, so it spans voxel rows 10 and 11; the block
//! occupies row 10 alone and row 11 above it is empty, which is what lets a single
//! break open the way through — there is no stepping up in this model, so a block
//! at the feet's own row stops the player as surely as a wall would.
//!
//! The break is aimed from a view pitched 30° below level: the ray leaves the eye
//! at (8.5, 11.62, 8.5) along (0.866, −0.5, 0), crosses into the next column at
//! 0.58 blocks, drops into row 10 at 1.24, and enters the block's own cell at
//! 1.73 — three cells of air, then the target, well inside the reach.
//!
//! Afterwards the player walks +x for a fixed number of ticks, and the assertion
//! is **which voxel column its feet ended in**. Three answers are distinguishable
//! and each means something different: column 8 is a player that never moved,
//! column 9 is a player held at the block's own near face, and column 10 is a
//! player standing inside the cell that block occupied. Nothing here is compared
//! against a written-down coordinate.

mod support;

use std::error::Error;

use glam::Vec3;
use mc_sim::action::{ActionIntent, TickIntent};
use mc_sim::player::{MovementIntent, PlayerState};
use mc_sim::simulation::Simulation;
use mc_world::world::WorldPos;

use support::chamber::{BlockChamber, at};
use support::{AIR, STONE, TestResult};

/// How many chunk columns the fixture world spans on each axis.
const COLUMNS: u32 = 1;

/// The layer the floor occupies, so its top face is at `FLOOR_LAYER + 1`.
const FLOOR_LAYER: u32 = 9;

/// The block standing directly in the player's path, in the feet's own voxel
/// row.
const IN_THE_WAY: WorldPos = at(10, 10, 8);

/// Where the feet start: on the floor, two columns short of the block.
const START: Vec3 = Vec3::new(8.5, 10.0, 8.5);

/// How far below level the view is aimed, in degrees, to put the ray into the
/// block's own row before it reaches its column.
const AIMED_DOWN: f32 = -30.0;

/// How many ticks the player walks after the first one.
///
/// A tick walks `4.5 / 60 = 0.075` blocks, so thirty walk 2.25 — far enough past
/// the block's near face at x = 10.0 to land the feet inside its cell rather than
/// on its edge, and short of the next column boundary at x = 11.0.
const WALK_TICKS: u32 = 30;

#[test]
fn a_player_walks_into_the_cell_a_block_in_its_path_occupied_once_that_block_is_broken()
-> TestResult {
    let walked = walked_after(Some(ActionIntent::Break))?;

    assert_eq!(
        column_reached(&walked),
        i32::try_from(IN_THE_WAY.x)?,
        "the block that stood in the way was broken through the same request path a click \
         reaches, so the walk that follows has to carry the player into the cell it occupied. A \
         store that was written while the collision view was not leaves the player held at a \
         face the world no longer has anything behind, one column short of this"
    );
    Ok(())
}

#[test]
fn a_player_walking_at_a_block_that_has_not_been_broken_is_stopped_short_of_its_cell() -> TestResult
{
    let walked = walked_after(None)?;

    assert_eq!(
        column_reached(&walked),
        i32::try_from(IN_THE_WAY.x)? - 1,
        "the same drive over the same fixture with the break left out: the player is held at the \
         block's own near face, in the column before it. This is what says the fixture was \
         blocking before anything was broken — a player that never moved ends one column further \
         back, and one that walked through ends inside the block's cell, so all three outcomes \
         are told apart by this one number"
    );
    Ok(())
}

/// A floor and one block in the path, one tick of `action`, then a walk along
/// +x.
fn walked_after(action: Option<ActionIntent>) -> Result<Simulation, Box<dyn Error>> {
    let chamber = BlockChamber::filled_with(COLUMNS, AIR)
        .run(at(0, FLOOR_LAYER, 0), at(16, FLOOR_LAYER + 1, 16), STONE)
        .cell(IN_THE_WAY, STONE);
    let mut simulation = Simulation::new(aiming_at_it(), chamber.build()?);

    simulation.advance(TickIntent {
        movement: MovementIntent::default(),
        action,
    });
    for _ in 0..WALK_TICKS {
        simulation.advance(TickIntent {
            movement: MovementIntent {
                forward: 1.0,
                ..MovementIntent::default()
            },
            action: None,
        });
    }
    Ok(simulation)
}

/// A player standing on the floor, facing +x with its view aimed down at the
/// block in its path.
fn aiming_at_it() -> PlayerState {
    PlayerState {
        position: START,
        velocity: Vec3::ZERO,
        yaw: 0.0,
        pitch: AIMED_DOWN.to_radians(),
        on_ground: true,
    }
}

/// Which voxel column the feet ended in.
fn column_reached(simulation: &Simulation) -> i32 {
    simulation.latest().player.position.x.floor() as i32
}
