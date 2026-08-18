//! The two views of the world state one answer about a cell across a swap.
//!
//! # This is the only instrument in the tree that can see the disagreement
//!
//! The world keeps a block store and a bitset of what stops the player, and the
//! whole difficulty of an editable world is that the two can fall out of step.
//! The replay's overlap oracle cannot see it: that oracle re-reads the world's
//! blocks and asks the registry about every name it finds, so a registry swapped
//! without its bitset makes the oracle agree with itself and go green.
//!
//! So the question is asked of both, separately, about the same cell:
//!
//! - **What the physics reads** is the bitset, through the world's own
//!   `Solidity`. It is the only thing a tick consults, and it carries no name to
//!   look up.
//! - **What a placement's occupancy check reads** is the block store and the
//!   registry the world was resolved against — `world.block_at(cell)` and then
//!   what that name is declared to be. It is the chain every rule about a cell's
//!   contents goes down.
//!
//! One answer before the swap and one after, so a swap that changed neither is
//! caught by the second pair and a swap that changed one of them is caught by the
//! two disagreeing.
//!
//! # Why this drives the simulation rather than a client
//!
//! The second view has no client surface at all: `Session` hands out no borrow of
//! the world and none of the registry, deliberately. The scenarios asking whether
//! a client *honours* a changed solidity — that stone stops stopping the player,
//! that water starts — live in `crates/mc-client/tests/`, and this is the one
//! that asks whether the world's two answers about a cell can be made to
//! disagree at all.

#[path = "support/roots.rs"]
mod roots;
mod support;

use std::error::Error;
use std::sync::Arc;

use glam::Vec3;
use mc_core::id::BlockName;
use mc_sim::player::{BlockPos, PlayerState, Solidity};
use mc_sim::simulation::{Simulation, seat};
use mc_sim::world::World;
use mc_world::section::Contents;
use mc_world::world::{VoxelWorld, WorldPos};

use roots::{Adoption, STONE_FILE, STONE_THAT_IS_NOT_SOLID, adoption, shipped};
use support::{GRASS, STONE, TestResult, content_registry, published_content};

/// How many chunk columns the fixture world spans on each axis.
const COLUMNS: u32 = 1;

/// The one cell this scenario asks both views about. It holds stone, standing on
/// a floor of grass whose own solidity nothing here changes.
const THE_STONE: WorldPos = WorldPos { x: 1, y: 10, z: 1 };
const UNDER_IT: WorldPos = WorldPos { x: 1, y: 9, z: 1 };

/// Where the player stands. Nothing here is about the player.
const ABOVE_EVERYTHING: Vec3 = Vec3::new(8.5, 40.0, 8.5);

#[test]
fn the_physics_and_a_placements_occupancy_check_agree_a_stone_cell_stopped_being_solid()
-> TestResult {
    let mut simulation = playing()?;
    let before = both_views(&simulation, THE_STONE)?;
    let candidate = shipped()?.restating(STONE_FILE, STONE_THAT_IS_NOT_SOLID)?;

    let answered = adoption(mc_sim::reload::adopt_at_tick_boundary(
        &mut simulation,
        candidate.candidate()?,
    ));
    require_admitted(&answered)?;

    assert_eq!(
        (before, both_views(&simulation, THE_STONE)?),
        ((true, true), (false, false)),
        "the bitset a tick consults and the registry a rule about a cell's contents consults are \
         two views of one world, and this cell is the same cell in both. A swap that replaced the \
         registry and left the bitset to be refreshed by somebody else leaves them disagreeing — \
         the player walks into a block content says is not there — and the replay's overlap oracle \
         cannot report it, because that oracle re-reads the world through the registry and would \
         be agreeing with itself"
    );
    Ok(())
}

/// A simulation of a grass floor with one stone block standing on it.
fn playing() -> Result<Simulation, Box<dyn Error>> {
    let registry = Arc::new(content_registry()?);
    let mut blocks = VoxelWorld::empty(COLUMNS);
    blocks.set_block(UNDER_IT, &BlockName::parse(GRASS)?, &registry)?;
    blocks.set_block(THE_STONE, &BlockName::parse(STONE)?, &registry)?;
    let spawn = PlayerState {
        position: ABOVE_EVERYTHING,
        velocity: Vec3::ZERO,
        yaw: 0.0,
        pitch: 0.0,
        on_ground: false,
    };
    let content = published_content(&registry)?;
    Ok(seat(spawn, World::new(blocks, registry)?, content).simulation)
}

/// What each of the two views says about the cell at `at`: whether the physics
/// is stopped by it, and whether what the store says is there is declared to
/// stop anything.
///
/// # Errors
///
/// Returns an error where the cell holds no block at all, which is this fixture
/// being wrong about itself rather than an answer either view gave.
fn both_views(simulation: &Simulation, at: WorldPos) -> Result<(bool, bool), Box<dyn Error>> {
    let world = simulation.world();
    let cell = BlockPos {
        x: i32::try_from(at.x)?,
        y: i32::try_from(at.y)?,
        z: i32::try_from(at.z)?,
    };
    let held = match world.block_at(cell) {
        Some(Contents::Holds(name)) => name.clone(),
        _ => {
            return Err(format!(
                "this fixture has to leave a block in the cell both views are asked about, and \
                 ({x}, {y}, {z}) holds none",
                x = at.x,
                y = at.y,
                z = at.z
            )
            .into());
        }
    };
    Ok((
        world.is_solid(cell),
        world.registry().resolve(&held)?.is_solid,
    ))
}

/// Refuses unless the candidate was admitted at all.
fn require_admitted(answered: &Adoption) -> Result<(), Box<dyn Error>> {
    if matches!(answered, Adoption::Accepted { .. }) {
        return Ok(());
    }
    Err(format!(
        "this scenario needs the candidate that takes stone's solidity away to be admitted, and \
         it answered {answered:?}. Neither view could then have moved, and the comparison below \
         would be about a swap that never happened"
    )
    .into())
}
