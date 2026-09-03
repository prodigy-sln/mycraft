//! What a swimmer in the water this repository ships can aim at, break and
//! build against.
//!
//! **The world here is built over the content root this repository ships**, for
//! the reason `the_shipped_water_is_aimable_only_within_reach.rs` gives: the
//! question is what a running game does with the declaration
//! `content/base/blocks/water.luau` carries — `targetable = true` beside
//! `occludes = false` — and a registry assembled in Rust would be the engine
//! answering on content's behalf.
//!
//! # What the fixture is, and what it deliberately is not
//!
//! It is **the water the player's head is in**, over a stone lakebed that rises
//! into the eye's own row four cells along. It is not a sea, and the cells
//! between the eye and that rise hold nothing on purpose: water declares that a
//! ray may stop at it, so a water cell standing in the ray is a target in its own
//! right, at its own distance and through its own face. That is deliberate and is
//! not what these scenarios are about — they are about the cell the eye is
//! *inside*, which is the one cell whose answer was wrong for every swimmer.
//!
//! **The consequence is worth stating rather than leaving to be discovered.**
//! In water deeper than the cell the eye occupies, the cell a swimmer's ray meets
//! next is more water, and a swing at it is still refused as indestructible —
//! water is `breakable = false`. What the origin-cell rule restores is that a
//! swimmer aims at *something else than the block their own head is in*: at the
//! lakebed where it stands beside or below them, and at the water beside them
//! with a face to build against. Before it, every swing and every placement was
//! answered by the cell the eye occupied, at distance 0 and with no face at all.
//!
//! # Two things the fixture holds that no assertion can
//!
//! **The eye is inside a shipped water voxel**, which is the whole precondition
//! and is checked by the builder rather than assumed: a fixture that came to hold
//! something else there would satisfy both scenarios below for the wrong reason.
//!
//! **The cell a placement lands in is one the player's box does not occupy.** The
//! box is 0.6 blocks wide about the feet, so the cell three along the row is
//! clear of it while the eye's own cell never is — a placement that landed in the
//! eye's cell would be refused as inside the player, and that refusal would look
//! exactly like the one this scenario exists to see gone.

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

/// The layer the lakebed occupies, so its top face is at `LAKEBED_LAYER + 1`.
const LAKEBED_LAYER: u32 = 9;

/// How far the lakebed runs on each horizontal axis: the whole column.
const LAKEBED_SPAN: u32 = 16;

/// Where the feet rest, on the lakebed's top face.
const FEET: Vec3 = Vec3::new(8.0, 10.0, 8.5);

/// The voxel row the eye is in, `floor(10.0 + 1.62)`.
const EYE_ROW: u32 = 11;

/// The cell the eye is inside, which the fixture fills with shipped water.
const EYE_CELL: WorldPos = at(8, EYE_ROW, 8);

/// The other cell the player's box reaches into at the eye's height, filled with
/// the same water so that the head is in water and not beside it.
const BESIDE_THE_EYE: WorldPos = at(7, EYE_ROW, 8);

/// Where the lakebed rises into the eye's own row, four cells along the ray.
const THE_RISE: WorldPos = at(12, EYE_ROW, 8);

/// The cell a placement against the rise lands in, one step back through the
/// face the ray entered by.
const AGAINST_THE_RISE: WorldPos = at(11, EYE_ROW, 8);

/// Yaw facing +x, which is where both rays go.
const ALONG_THE_ROW: f32 = 0.0;

#[test]
fn a_swing_from_inside_the_shipped_water_breaks_the_lakebed_rather_than_the_water() -> TestResult {
    let swung = acting(ActionIntent::Break)?;

    assert_eq!(
        swung,
        (
            Some(EditReport::Changed {
                cell: signed(THE_RISE),
                from: Contents::Holds(BlockName::parse(STONE)?),
                to: Contents::Empty,
            }),
            vec![(THE_RISE, STONE.to_owned(), NOTHING.to_owned())]
        ),
        "the water the swimmer's head is in declares that it can be seen through, so the swing \
         travels past the cell the eye occupies and takes the lakebed where it rises into that \
         row. A walk that stops at the origin cell answers with the water instead, and water \
         declares `breakable = false`, so the swimmer is told the thing they are standing in \
         cannot be broken and no cell moves at all"
    );
    Ok(())
}

#[test]
fn a_placement_from_inside_the_shipped_water_goes_against_the_lakebed() -> TestResult {
    let placed = acting(ActionIntent::Place {
        block: BlockName::parse(DIRT)?,
    })?;

    assert_eq!(
        placed,
        (
            Some(EditReport::Changed {
                cell: signed(AGAINST_THE_RISE),
                from: Contents::Empty,
                to: Contents::Holds(BlockName::parse(DIRT)?),
            }),
            vec![(AGAINST_THE_RISE, NOTHING.to_owned(), DIRT.to_owned())]
        ),
        "a placement goes on the near side of what the ray met, one step back through the face it \
         entered by. A walk that stops at the cell the eye is inside crossed no face at all, so \
         there is no near side to build on and the request is refused for want of one — which is \
         every placement a swimmer ever asked for"
    );
    Ok(())
}

/// One tick over a fresh world asking for one action, and what that did to the
/// world compared with the same world as declared.
fn acting(action: ActionIntent) -> Result<Acted, Box<dyn Error>> {
    let declared = the_water_a_head_is_in_over_a_lakebed()?;
    let played = the_water_a_head_is_in_over_a_lakebed()?;
    let content = published_content(played.registry())?;
    let mut simulation = seat(swimming(), played, content).simulation;
    let report = simulation.advance(TickIntent {
        movement: MovementIntent::default(),
        action: Some(action),
    });
    Ok((report, differences(&declared, simulation.world())))
}

/// A stone lakebed across the whole column, the two cells of shipped water the
/// player's head reaches into, and the lakebed rising into the eye's row four
/// cells along.
///
/// Refuses to build a world whose eye cell holds anything but water: the
/// precondition of both scenarios is that the eye is *inside* the shipped water,
/// and a fixture that had drifted off it would answer both of them for a reason
/// neither is about.
fn the_water_a_head_is_in_over_a_lakebed() -> Result<World, Box<dyn Error>> {
    let registry = Arc::new(content_registry()?);
    let mut blocks = VoxelWorld::empty(COLUMNS);
    let stone = BlockName::parse(STONE)?;
    for z in 0..LAKEBED_SPAN {
        for x in 0..LAKEBED_SPAN {
            blocks.set_block(
                WorldPos {
                    x,
                    y: LAKEBED_LAYER,
                    z,
                },
                &stone,
                &registry,
            )?;
        }
    }
    blocks.set_block(THE_RISE, &stone, &registry)?;
    let water = BlockName::parse(WATER)?;
    for cell in [EYE_CELL, BESIDE_THE_EYE] {
        blocks.set_block(cell, &water, &registry)?;
    }
    let world = World::new(blocks, registry)?;
    match world.block_at(signed(EYE_CELL)) {
        Some(Contents::Holds(name)) if name.as_str() == WATER => Ok(world),
        held => Err(format!(
            "the eye's own cell has to hold the shipped water for either scenario below to be \
             about a swimmer at all, and it holds {held:?}"
        )
        .into()),
    }
}

/// A player resting on the lakebed with their head in the water, facing along
/// the row with a level view and asking for no movement.
fn swimming() -> PlayerState {
    PlayerState {
        position: FEET,
        velocity: Vec3::ZERO,
        yaw: ALONG_THE_ROW,
        pitch: 0.0,
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
