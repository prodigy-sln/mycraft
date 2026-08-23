//! What the base game declares about breaking water, and what a player who
//! swings at it or builds into it actually gets.
//!
//! **The world here is built over the content root this repository ships**, for
//! the reason `base_content_placement.rs` gives: the question is what a running
//! game does with the declaration `content/base/blocks/water.luau` carries, and a
//! registry assembled in Rust would be the engine answering on content's behalf.
//!
//! # `breakable = false` on water is live, and this file is where that is written
//! down
//!
//! It was not always. A break aimed at a cell holding water used to reach the
//! **solid block behind it** and break that, because the walk stopped only where
//! a block was declared `solid` and water declares `solid = false` — so `broken`
//! was never called on a water cell and `Refusal::Indestructible` was
//! unreachable for water however the declaration read. This file carried a fuse
//! recording that debt, and the fuse has been blown: water declares
//! `targetable = true`, the walk stops at it, and the refusal below is what a
//! player meets. Aiming and yielding are separate claims, and it is the first
//! that makes the second reachable at all.
//!
//! What the declaration also does is move water's recorded behaviour fold, which
//! is why an existing save reports `base:water` as changed — see
//! `crates/mc-world/tests/shipped_declarations_and_an_older_save.rs`.
//!
//! # The aim, and the arithmetic the cells come from
//!
//! The feet stand at (8.5, 10.0, 8.5) on a floor whose top face is y = 10, so the
//! eye is at (8.5, 11.62, 8.5). The view is pitched 30 degrees below level along
//! +x, which is the direction (0.866, -0.5, 0). It crosses x = 9 at 0.577 blocks
//! along, where y = 11.33 and so still inside row 11, so it enters the cell
//! holding the water through that cell's **West** face; the upward face of the
//! stone standing at (9, 10, 8) lies a further 0.66 blocks beyond it.
//!
//! **The water cell is both the cell a swing stops at and the cell a placement
//! lands in, and the two reach it by different routes.** A swing stops at the
//! water and is refused. A placement lands in the water cell because that cell is
//! itself declared `replaceable`, so it goes into the cell the ray stopped at
//! rather than one step back — and one step back from a West face is (8, 11, 8),
//! which is inside the player's own box. Before water was aimable the same
//! placement reached the same cell by stepping back from the stone's upward face:
//! the expectation is unchanged and the route to it is not.

mod support;

use std::error::Error;
use std::sync::Arc;

use glam::Vec3;
use mc_core::block::BlockRegistry;
use mc_core::id::BlockName;
use mc_sim::action::{ActionIntent, EditReport, Refusal, TickIntent};
use mc_sim::player::{BlockPos, MovementIntent, PlayerState};
use mc_sim::simulation::seat;
use mc_sim::world::World;
use mc_world::section::Contents;
use mc_world::world::{VoxelWorld, WorldPos};

use support::chamber::{at, differences};
use support::{
    DIRT, NOTHING, STONE, TestResult, WATER, content_registry, described, published_content,
};

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

/// What a swing aimed through the water reaches, and what the same swing reaches
/// with the water taken out.
///
/// The second half is what stops the first being satisfied by a fixture nobody
/// aimed at: one declared cell removed, everything else untouched, and the
/// identical aim then empties the stone.
#[test]
fn a_swing_aimed_through_the_shipped_water_stops_at_it_and_not_at_the_stone_behind_it() -> TestResult
{
    let refused = acting(ActionIntent::Break)?;
    let reaching = acting_without_the_water(ActionIntent::Break)?;

    assert_eq!(
        (refused, reaching),
        (
            (
                Some(EditReport::Refused(Refusal::Indestructible)),
                nothing()
            ),
            (
                Some(EditReport::Changed {
                    cell: signed(TARGET),
                    from: Contents::Holds(BlockName::parse(STONE)?),
                    to: Contents::Empty,
                }),
                vec![(TARGET, STONE.to_owned(), NOTHING.to_owned())]
            )
        ),
        "the ray crosses the water cell before it reaches the stone, and `{WATER}` declares that a \
         ray may stop at it — so the swing stops there and never asks the stone anything. Take \
         that one cell of water out and the same player, from the same place, looking the same \
         way, empties the stone: that is what says the first half is about where the ray stopped \
         rather than about a fixture nothing was ever aimed at"
    );
    Ok(())
}

#[test]
fn a_break_swung_at_the_shipped_water_is_refused_as_indestructible_and_leaves_it_in_the_cell()
-> TestResult {
    let played = a_world_with_water_above_the_target()?;
    let content = published_content(played.registry())?;
    let mut simulation = seat(looking_down_at_the_target(), played, content).simulation;

    let answer = simulation.advance(TickIntent {
        movement: MovementIntent::default(),
        action: Some(ActionIntent::Break),
    });

    assert_eq!(
        (answer, held(simulation.world(), THE_WATER_CELL)?),
        (
            Some(EditReport::Refused(Refusal::Indestructible)),
            WATER.to_owned()
        ),
        "`{WATER}` declares `breakable = false`, and now that a swing can arrive at it that \
         declaration has a consequence a player meets. The refusal is named rather than inferred \
         from a world that did not move: `NoTarget` leaves the world exactly as still, and it is \
         what a build in which water never became aimable produces. The cell is read back by name \
         for the other half, because a refusal that emptied the cell anyway would be a refusal in \
         the report only"
    );
    Ok(())
}

#[test]
fn a_break_swung_at_the_shipped_water_leaves_the_solid_block_behind_it_untouched() -> TestResult {
    let (answer, changes) = acting(ActionIntent::Break)?;

    assert_eq!(
        (answer, changes),
        (
            Some(EditReport::Refused(Refusal::Indestructible)),
            nothing()
        ),
        "the whole world is compared cell by cell against the same world as declared, so this says \
         the stone behind the water is where it was and that nothing else moved either. It is a \
         reading of its own so that a refusal that was right while the world moved anyway fails \
         here and nowhere else, which one test carrying both claims could not distinguish"
    );
    Ok(())
}

/// One tick over a fresh world asking for `action`, and what that did to the
/// world compared with the same world as declared.
fn acting(action: ActionIntent) -> Result<Acted, Box<dyn Error>> {
    acting_over(a_world_with_water_above_the_target, action)
}

/// The same, over the world that has no water above the target.
fn acting_without_the_water(action: ActionIntent) -> Result<Acted, Box<dyn Error>> {
    acting_over(a_world_with_nothing_above_the_target, action)
}

/// One tick over a fresh build of `world` asking for `action`, and what that did
/// to the world compared with the same world as declared.
///
/// The builder is taken rather than the world, because the comparison needs the
/// fixture twice — once to drive and once to compare against — and a copy of a
/// run is not a declaration.
fn acting_over(
    world: fn() -> Result<World, Box<dyn Error>>,
    action: ActionIntent,
) -> Result<Acted, Box<dyn Error>> {
    let declared = world()?;
    let played = world()?;
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
    let mut blocks = a_floor_of_stone(&registry)?;
    let stone = BlockName::parse(STONE)?;
    blocks.set_block(TARGET, &stone, &registry)?;
    blocks.set_block(THE_WATER_CELL, &BlockName::parse(WATER)?, &registry)?;
    Ok(World::new(blocks, registry)?)
}

/// The same world with the one cell of water left out, and nothing else changed.
///
/// **One declared cell fewer, and that is the whole of the difference** — the
/// floor, the stone, the player and the aim are the fixture above's. What the
/// same ray then meets is the stone's upward face, which is what it met before
/// water could be aimed at.
fn a_world_with_nothing_above_the_target() -> Result<World, Box<dyn Error>> {
    let registry = Arc::new(content_registry()?);
    let mut blocks = a_floor_of_stone(&registry)?;
    blocks.set_block(TARGET, &BlockName::parse(STONE)?, &registry)?;
    Ok(World::new(blocks, registry)?)
}

/// One layer of stone across the whole column, for the player to stand on.
fn a_floor_of_stone(registry: &Arc<BlockRegistry>) -> Result<VoxelWorld, Box<dyn Error>> {
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
                registry,
            )?;
        }
    }
    Ok(blocks)
}

/// What `world` holds at `cell`, by name.
///
/// # Errors
///
/// Returns an error where the world reaches no such cell, which is this fixture
/// being wrong about itself rather than an answer the world gave.
fn held(world: &World, cell: WorldPos) -> Result<String, Box<dyn Error>> {
    let contents = world
        .block_at(signed(cell))
        .ok_or_else(|| format!("the fixture world reaches no cell at {cell:?}"))?;
    Ok(described(contents))
}

/// No cell of the fixture moved.
fn nothing() -> Changes {
    Vec::new()
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
