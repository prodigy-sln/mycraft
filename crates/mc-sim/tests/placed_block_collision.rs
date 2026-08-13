//! What the world reads as and what the player collides with, after a place.
//!
//! These two are a pair and neither half works alone. The first alone goes green
//! against a world that was already blocking the player before anything was
//! placed, so the second runs the identical drive over the identical fixture with
//! the placement left out and requires the player to walk through. Between them a
//! store written without its collision view, and a fixture that was never open in
//! the first place, are both red.
//!
//! # The fixture, and the arithmetic every number here comes from
//!
//! The player's feet start at (8.5, 10.0, 8.5) on a floor whose top face is
//! y = 10, so the eye is at (8.5, 11.62, 8.5). The view is pitched 40° below
//! level, along (0.766, −0.643, 0): it descends the 1.62 blocks to the floor's
//! own face over 1.931 blocks of x, arriving at x = 10.431 after 2.520 blocks of
//! ray. That is the **upward** face of the floor cell at (10, 9, 8), four tenths
//! of a block inside column 10 either side of the boundary, so the placement
//! lands at (10, 10, 8) — the feet's own row, two columns ahead of where the
//! player is standing.
//!
//! There is no stepping up in this model, so a block in the feet's row stops the
//! player as surely as a wall would.
//!
//! Afterwards the player walks +x for a fixed number of ticks, and the assertion
//! is **which voxel column its feet ended in**. A tick walks 4.5 / 60 = 0.075
//! blocks, so thirty walk 2.25: unobstructed the feet reach 10.75, and held at
//! the placed block's near face they stop 0.3 short of x = 10 at 9.7. Three
//! answers are distinguishable and each means something different — column 8 is a
//! player that never moved, column 9 a player held at the block that was placed,
//! and column 10 a player that walked through the cell it was placed in. Nothing
//! here is compared against a written-down coordinate.

mod support;

use std::error::Error;

use glam::Vec3;
use mc_core::id::BlockName;
use mc_sim::action::{ActionIntent, TickIntent};
use mc_sim::player::{MovementIntent, PlayerState};
use mc_sim::simulation::Simulation;
use mc_world::world::WorldPos;

use support::chamber::{BlockChamber, at};
use support::{STONE, TestResult};

/// How many chunk columns the fixture world spans on each axis.
const COLUMNS: u32 = 1;

/// The layer the floor occupies, so its top face is at `FLOOR_LAYER + 1`.
const FLOOR_LAYER: u32 = 9;

/// The floor cell whose upward face the ray comes in through.
const AIMED_AT: WorldPos = at(10, FLOOR_LAYER, 8);

/// The cell the placement lands in: directly above that floor cell, which is the
/// feet's own row and directly in the player's path.
const IN_THE_PATH: WorldPos = at(AIMED_AT.x, AIMED_AT.y + 1, AIMED_AT.z);

/// Where the feet start: on the floor, two columns short of the placement.
const START: Vec3 = Vec3::new(8.5, 10.0, 8.5);

/// Yaw facing +x, which is both where the ray goes and where the walk goes.
const ALONG_THE_ROW: f32 = 0.0;

/// How far below level the view is aimed, in degrees, to meet the floor's face
/// two columns ahead rather than the one underfoot.
const AIMED_DOWN: f32 = -40.0;

/// How many ticks the player walks after the first one.
const WALK_TICKS: u32 = 30;

#[test]
fn a_player_is_stopped_at_a_block_placed_in_the_cell_directly_in_its_path() -> TestResult {
    let walked = walked_after(Some(ActionIntent::Place {
        block: BlockName::parse(STONE)?,
    }))?;

    assert_eq!(
        column_reached(&walked),
        i32::try_from(IN_THE_PATH.x)? - 1,
        "a block was put in the player's path through the same request path a click reaches, so \
         the walk that follows has to be held at its near face, in the column before it. A store \
         written while the collision view was not leaves the world reading as blocked and the \
         player walking straight through the cell it says holds a block"
    );
    Ok(())
}

#[test]
fn a_player_walks_through_the_cell_in_its_path_while_no_block_has_been_placed_there() -> TestResult
{
    let walked = walked_after(None)?;

    assert_eq!(
        column_reached(&walked),
        i32::try_from(IN_THE_PATH.x)?,
        "the same drive over the same fixture with the placement left out: the way is open and \
         the player ends inside the cell the other half stops it short of. This is what says the \
         fixture was not already blocking — a player that never moved ends two columns further \
         back, so all three outcomes are told apart by this one number"
    );
    Ok(())
}

/// A floor, one tick of `action`, then a walk along +x.
fn walked_after(action: Option<ActionIntent>) -> Result<Simulation, Box<dyn Error>> {
    let chamber =
        BlockChamber::empty(COLUMNS).run(at(0, FLOOR_LAYER, 0), at(16, FLOOR_LAYER + 1, 16), STONE);
    let mut simulation = Simulation::new(aiming_at_the_floor(), chamber.build()?);

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
/// floor two columns ahead.
fn aiming_at_the_floor() -> PlayerState {
    PlayerState {
        position: START,
        velocity: Vec3::ZERO,
        yaw: ALONG_THE_ROW,
        pitch: AIMED_DOWN.to_radians(),
        on_ground: true,
    }
}

/// Which voxel column the feet ended in.
fn column_reached(simulation: &Simulation) -> i32 {
    simulation.latest().player.position.x.floor() as i32
}
