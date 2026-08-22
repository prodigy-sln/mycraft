//! What the base game declares about breaking water, and what a player who
//! swings at it or builds into it actually gets.
//!
//! **The world here is built over the content root this repository ships**, for
//! the reason `base_content_placement.rs` gives: the question is what a running
//! game does with the declaration `content/base/blocks/water.luau` carries, and a
//! registry assembled in Rust would be the engine answering on content's behalf.
//!
//! # `breakable = false` on water is inert today, and this file is where that is
//! written down
//!
//! Measured rather than reasoned: a break aimed at a cell holding water reaches
//! the **solid block behind it**, and breaks that. `targeted`
//! (`src/world/action/trace.rs:62`) returns a hit only where
//! `world.is_solid(met.cell)`, water declares `solid = false`, so the walk steps
//! straight through the water cell and `broken` is never called on one.
//! `Refusal::Indestructible` is therefore unreachable for water however the
//! declaration reads.
//!
//! So the third reading below asserts what a break really does, and it is a
//! **fuse rather than a description**. The moment `solid` is split into drawn,
//! occludes and targetable — which is PRO-904's whole subject — water becomes
//! targetable, `broken` is called on a water cell for the first time, and this
//! reading goes red. That red is the signal that `breakable = false` has just
//! acquired a player-visible consequence and now owes the scenario it could not
//! have here: *a break swung at water is refused and leaves the water in the
//! cell*. Whoever makes water targetable writes that test.
//!
//! What the declaration *does* do today is move water's recorded behaviour fold,
//! which is why an existing save reports `base:water` as changed — see
//! `crates/mc-world/tests/shipped_declarations_and_an_older_save.rs`.
//!
//! # The aim, and the arithmetic the cells come from
//!
//! The feet stand at (8.5, 10.0, 8.5) on a floor whose top face is y = 10, so the
//! eye is at (8.5, 11.62, 8.5). The view is pitched 30° below level along +x. It
//! crosses x = 9 while still above y = 11, so it enters the cell holding the
//! water first and meets the upward face of the stone standing at (9, 10, 8)
//! after it. So the water cell is both the cell the ray passes through and the
//! cell a placement against that face lands in.

mod support;

use std::error::Error;
use std::sync::Arc;

use glam::Vec3;
use mc_core::id::BlockName;
use mc_sim::action::{ActionIntent, EditReport, TickIntent};
use mc_sim::player::{BlockPos, MovementIntent, PlayerState};
use mc_sim::simulation::seat;
use mc_sim::world::World;
use mc_world::section::Contents;
use mc_world::world::{VoxelWorld, WorldPos};

use support::chamber::{at, differences};
use support::{DIRT, NOTHING, STONE, TestResult, WATER, content_registry, published_content};

/// Every cell at which a run differs from the world as declared.
type Changes = Vec<(WorldPos, String, String)>;

/// What one action answered, and what it did to the world.
type Acted = (Option<EditReport>, Changes);

/// How many chunk columns the world spans on each axis.
const COLUMNS: u32 = 1;

/// The layer the floor occupies, so its top face is at `FLOOR_LAYER + 1`.
const FLOOR_LAYER: u32 = 9;

/// How far the floor runs on each horizontal axis: the whole column.
const FLOOR_SPAN: u32 = 16;

/// The stone block whose upward face the ray meets.
const TARGET: WorldPos = at(9, 10, 8);

/// The cell directly above it, which holds the water.
const THE_WATER_CELL: WorldPos = at(TARGET.x, TARGET.y + 1, TARGET.z);

/// Where the feet stand: on the floor, one column short of the target.
const STANDING: Vec3 = Vec3::new(8.5, 10.0, 8.5);

/// Yaw facing +x, and how far below level the view is aimed, in degrees.
const ALONG_THE_ROW: f32 = 0.0;
const AIMED_DOWN: f32 = -30.0;

#[test]
fn the_shipped_content_declares_water_unbreakable_and_stone_breakable_still() -> TestResult {
    let registry = content_registry()?;

    let declared = (
        registry.resolve(&BlockName::parse(WATER)?)?.breakable,
        registry.resolve(&BlockName::parse(STONE)?)?.breakable,
    );

    assert_eq!(
        declared,
        (false, true),
        "`{WATER}` is declared unbreakable by the shipped content and that declaration has to \
         reach the registry a running game resolves names against — a reader that dropped the \
         field, or resolved its default over the stated value, would answer `true` here. \
         `{STONE}` is the control in the other direction: a reader that answered `false` for \
         everything would satisfy the first half on its own"
    );
    Ok(())
}

#[test]
fn a_placement_into_the_shipped_water_replaces_it_with_the_block_being_placed() -> TestResult {
    let (answer, changes) = acting(ActionIntent::Place {
        block: BlockName::parse(DIRT)?,
    })?;

    assert_eq!(
        (answer, changes),
        (
            Some(EditReport::Changed {
                cell: signed(THE_WATER_CELL),
                from: Contents::Holds(BlockName::parse(WATER)?),
                to: Contents::Holds(BlockName::parse(DIRT)?),
            }),
            vec![(THE_WATER_CELL, WATER.to_owned(), DIRT.to_owned())]
        ),
        "the shipped content declares `{WATER}` replaceable, so a player builds straight through \
         it rather than having to break it first. The cell held water before and holds the placed \
         block after, and no other cell of the world moved — a placement that took the right cell \
         and another one as well is not a correct placement"
    );
    Ok(())
}

/// What a break aimed at the water cell really does — the fuse this file's header
/// is about.
///
/// It reddens when water becomes targetable, which is the point. Read the header
/// before changing it: the repair is a new scenario, not a new expectation.
#[test]
fn a_break_aimed_through_the_shipped_water_reaches_the_solid_block_behind_it() -> TestResult {
    let (answer, changes) = acting(ActionIntent::Break)?;

    assert_eq!(
        (answer, changes),
        (
            Some(EditReport::Changed {
                cell: signed(TARGET),
                from: Contents::Holds(BlockName::parse(STONE)?),
                to: Contents::Empty,
            }),
            vec![(TARGET, STONE.to_owned(), NOTHING.to_owned())]
        ),
        "the ray stops only at a solid cell and `{WATER}` is not solid, so a swing at the water \
         goes through it and empties the `{STONE}` behind. That is what makes `breakable = false` \
         inert for a player today. If this has gone red because water is now targetable, the \
         change owes the scenario this could not be: the break is refused and the water stays"
    );
    Ok(())
}

/// One tick over a fresh world asking for `action`, and what that did to the
/// world compared with the same world as declared.
fn acting(action: ActionIntent) -> Result<Acted, Box<dyn Error>> {
    let declared = a_world_with_water_above_the_target()?;
    let played = a_world_with_water_above_the_target()?;
    let content = published_content(played.registry())?;
    let mut simulation = seat(looking_down_at_the_target(), played, content).simulation;
    let report = simulation.advance(TickIntent {
        movement: MovementIntent::default(),
        action: Some(action),
    });
    Ok((report, differences(&declared, simulation.world())))
}

/// A world of nothing over the shipped blocks, with a stone floor, one stone
/// block standing on it, and water in the cell directly above that block.
fn a_world_with_water_above_the_target() -> Result<World, Box<dyn Error>> {
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
    blocks.set_block(THE_WATER_CELL, &BlockName::parse(WATER)?, &registry)?;
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

/// The same cell in the signed spelling an edit report carries.
const fn signed(cell: WorldPos) -> BlockPos {
    BlockPos {
        x: cell.x as i32,
        y: cell.y as i32,
        z: cell.z as i32,
    }
}
