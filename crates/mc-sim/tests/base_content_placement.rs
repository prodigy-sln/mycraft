//! What a placement may name, once the blocks the base game ships are the only
//! blocks there are.
//!
//! **The world here is built over the content root this repository ships**, not
//! over a fixture registry that imitates it. The question is precisely which
//! names a running game knows after loading `content/base`, and a registry
//! assembled in Rust would be the engine answering on content's behalf.
//!
//! **The refusal is asserted by name, and it is asserted against a run that
//! succeeds.** Two things would otherwise pass for the same answer. A request
//! naming a block the registry does not know is refused before anything is
//! written, and so is a request that never found anything to place against —
//! and both leave the world exactly as it was declared, so a test comparing
//! worlds alone could not tell "this name is not a block" from "this fixture was
//! never aimed at anything". The second run is the same drive over the same
//! world, one requested name apart.
//!
//! # The aim, and the arithmetic the two coordinates come from
//!
//! The feet stand at (8.5, 10.0, 8.5) on a floor whose top face is y = 10, so
//! the eye is at (8.5, 11.62, 8.5). The view is pitched 30° below level along
//! +x and meets the upward face of the solid block standing at (9, 10, 8), so
//! the cell the placement lands in is the one the ray came from: (9, 11, 8),
//! which holds nothing and which nothing else in the fixture reaches.

mod support;

use std::error::Error;
use std::sync::Arc;

use glam::Vec3;
use mc_core::id::BlockName;
use mc_sim::action::{ActionIntent, EditReport, Refusal, TickIntent};
use mc_sim::player::{MovementIntent, PlayerState};
use mc_sim::simulation::Simulation;
use mc_sim::world::World;
use mc_world::world::{VoxelWorld, WorldPos};

use support::chamber::{at, differences};
use support::{DIRT, NOTHING, STONE, TestResult, content_registry};

/// Every cell at which a run differs from the world as declared.
type Changes = Vec<(WorldPos, String, String)>;

/// What one placement answered, and what it did to the world.
type Placement = (Option<EditReport>, Changes);

/// How many chunk columns the world spans on each axis.
const COLUMNS: u32 = 1;

/// The layer the floor occupies, so its top face is at `FLOOR_LAYER + 1`.
const FLOOR_LAYER: u32 = 9;

/// How far the floor runs on each horizontal axis: the whole column.
const FLOOR_SPAN: u32 = 16;

/// The block whose upward face the ray meets.
const TARGET: WorldPos = at(9, 10, 8);

/// The cell directly above it, which is where the placement lands.
const ABOVE_THE_TARGET: WorldPos = at(TARGET.x, TARGET.y + 1, TARGET.z);

/// Where the feet stand: on the floor, one column short of the target.
const STANDING: Vec3 = Vec3::new(8.5, 10.0, 8.5);

/// Yaw facing +x, and how far below level the view is aimed, in degrees.
const ALONG_THE_ROW: f32 = 0.0;
const AIMED_DOWN: f32 = -30.0;

/// A namespaced name the base game once shipped for a block whose whole job was
/// to mean empty space, and ships no longer.
///
/// It parses — an unparseable name is a different refusal — and after the base
/// content set is loaded it resolves to nothing at all, which is the one thing
/// this scenario is about.
const NO_LONGER_SHIPPED: &str = "base:air";

#[test]
fn a_place_naming_a_block_the_shipped_content_no_longer_declares_changes_nothing() -> TestResult {
    let (answer, refused) = placing(NO_LONGER_SHIPPED)?;
    let (_, accepted) = placing(DIRT)?;

    assert_eq!(
        (answer, refused, accepted),
        (
            Some(EditReport::Refused(Refusal::UnknownBlock {
                name: BlockName::parse(NO_LONGER_SHIPPED)?
            })),
            nothing(),
            placed(ABOVE_THE_TARGET, NOTHING, DIRT)
        ),
        "the same request over the same world, differing only in the name it carries: one the \
         shipped content declares and one it does not. `{NO_LONGER_SHIPPED}` is not a block a \
         player can be handed, because it is not a block — the cell it would have gone into is \
         empty, and empty is not something content declares. The accepting run is what says the \
         refusal came from the name rather than from a world nothing was ever aimed at"
    );
    Ok(())
}

/// One tick over a fresh world asking for one placement of `block`, and what
/// that did to the world compared with the same world as declared.
fn placing(block: &str) -> Result<Placement, Box<dyn Error>> {
    let declared = floored_world()?;
    let mut simulation = Simulation::new(looking_down_at_the_target(), floored_world()?);
    let report = simulation.advance(TickIntent {
        movement: MovementIntent::default(),
        action: Some(ActionIntent::Place {
            block: BlockName::parse(block)?,
        }),
    });
    Ok((report, differences(&declared, simulation.world())))
}

/// A world of nothing, over the blocks this repository ships as content, with
/// one layer of floor and one solid block standing on it.
fn floored_world() -> Result<World, Box<dyn Error>> {
    let registry = Arc::new(content_registry()?);
    let mut blocks = VoxelWorld::empty(COLUMNS);
    let stone = BlockName::parse(STONE)?;
    for z in 0..FLOOR_SPAN {
        for x in 0..FLOOR_SPAN {
            blocks.set_block(
                WorldPos {
                    x,
                    y: FLOOR_LAYER,
                    z,
                },
                &stone,
                &registry,
            )?;
        }
    }
    blocks.set_block(TARGET, &stone, &registry)?;
    Ok(World::new(blocks, registry)?)
}

/// A player standing on the floor, looking down at the target.
fn looking_down_at_the_target() -> PlayerState {
    PlayerState {
        position: STANDING,
        velocity: Vec3::ZERO,
        yaw: ALONG_THE_ROW,
        pitch: AIMED_DOWN.to_radians(),
        on_ground: true,
    }
}

/// The one change a placement into `cell` is expected to make.
fn placed(cell: WorldPos, from: &str, into: &str) -> Changes {
    vec![(cell, from.to_owned(), into.to_owned())]
}

/// No cell of the world moved.
fn nothing() -> Changes {
    Vec::new()
}
