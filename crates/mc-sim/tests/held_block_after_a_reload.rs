//! Which block a player holds after a reload has published a different registry.
//!
//! The rule is the first *colliding* block in registration order, and
//! `held_block.rs` is where the rule itself is stated. What this asks is whether
//! a reload re-derives it rather than carrying the answer the launch computed —
//! a held block that never moved would leave a player holding something the
//! candidate may not even declare any more.
//!
//! # Restated, not removed, and that is the whole of what makes it falsifiable
//!
//! A candidate that stopped declaring dirt and grass would leave stone the first
//! block registered *as well as* the first block that stops a player, so a rule
//! reading plain registration order would answer stone too and this would be
//! green over either. Restating them as blocks that stop nobody keeps dirt first
//! in registration order — content is read in file-name order and `dirt.luau`
//! still sorts first — and makes stone the first that collides. Those are two
//! different answers and only one of them is the rule.
//!
//! # Two things the fixture holds that no assertion can
//!
//! **The world holds blocks the candidate still declares.** Dirt and grass are
//! restated rather than dropped, so the admission check about blocks the world
//! holds has nothing to refuse and this is a scenario about the held block
//! rather than about that check.
//!
//! **The player stands well clear of everything.** Taking solidity away from
//! dirt and grass would otherwise put the reload's own clearing search into the
//! picture, and the answer being asked for here has nothing to do with where the
//! player ends up.

#[path = "support/roots.rs"]
mod roots;
mod support;

use std::error::Error;
use std::sync::Arc;

use glam::Vec3;
use mc_core::id::BlockName;
use mc_sim::player::PlayerState;
use mc_sim::simulation::{Simulation, seat};
use mc_sim::world::World;
use mc_world::world::{VoxelWorld, WorldPos};

use roots::{
    DIRT_FILE, DIRT_THAT_IS_NOT_SOLID, GRASS_FILE, GRASS_THAT_IS_NOT_SOLID, accepted, adoption,
    shipped,
};
use support::{DIRT, GRASS, STONE, TestResult, content_registry, published_content};

/// How many chunk columns the fixture world spans on each axis.
const COLUMNS: u32 = 1;

/// Where the fixture writes the two blocks the candidate restates.
const THE_DIRT: WorldPos = WorldPos { x: 1, y: 9, z: 1 };
const THE_GRASS: WorldPos = WorldPos { x: 1, y: 10, z: 1 };

/// Where the player stands. Nothing here is about the player, and the height is
/// well clear of everything the fixture writes.
const ABOVE_EVERYTHING: Vec3 = Vec3::new(8.5, 40.0, 8.5);

#[test]
fn a_reload_that_takes_solidity_off_the_first_two_blocks_puts_the_third_in_hand() -> TestResult {
    let mut simulation = playing()?;
    let candidate = shipped()?
        .restating(DIRT_FILE, DIRT_THAT_IS_NOT_SOLID)?
        .restating(GRASS_FILE, GRASS_THAT_IS_NOT_SOLID)?;

    let answered = adoption(mc_sim::reload::adopt_at_tick_boundary(
        &mut simulation,
        candidate.candidate()?,
    ));

    assert_eq!(
        answered,
        accepted(STONE),
        "the candidate is read in file-name order, so it registers `{DIRT}`, `{GRASS}`, `{STONE}` \
         and water in that order and the first two of them now stop nobody. The block a player \
         finds in hand is the first that *collides*, so it is stone — an answer of `{DIRT}` is a \
         rule reading registration order alone, and an answer of `{DIRT}` is also what a reload \
         that published the candidate and kept the held block the launch had computed would give"
    );
    Ok(())
}

/// A simulation of one cell of dirt with one cell of grass standing on it.
fn playing() -> Result<Simulation, Box<dyn Error>> {
    let registry = Arc::new(content_registry()?);
    let mut blocks = VoxelWorld::empty(COLUMNS);
    blocks.set_block(THE_DIRT, &BlockName::parse(DIRT)?, &registry)?;
    blocks.set_block(THE_GRASS, &BlockName::parse(GRASS)?, &registry)?;
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
