//! Where a placement lands when the cell the ray stopped at is itself declared
//! replaceable.
//!
//! A placement ordinarily lands one step back from the cell the ray stopped at,
//! on the side the ray came from — so it goes on the near face of what you are
//! looking at, never inside it. **A cell holding something a placement may
//! overwrite is the exception**, and it has to be, because otherwise there is no
//! ray at all that lands a block in such a cell: the cell one step back is the
//! cell the ray occupied immediately *before* the hit, and if that cell had held
//! something replaceable the walk would have stopped there instead.
//!
//! # Why there are two fixtures here rather than one
//!
//! The first is the fixture registry's own block that is **solid and replaceable
//! at once**, which nothing content ships is. It states the rule with no refusal
//! anywhere in it: the block being placed goes into the cell that was hit, and
//! the cell one step back — which is empty, and which the player is nowhere near
//! — stays empty. That is the reading that separates the two rules and nothing
//! else about it can go wrong.
//!
//! The second is the shipped water, and it is about the guard rather than the
//! rule: choosing a different cell is not licence to skip the check that the cell
//! is not one the player is standing in. Picking a cell and then not asking that
//! question is the obvious way for this to be got wrong, and no scenario about
//! water covers it.
//!
//! # The player's box, and the two cells it stands in
//!
//! The box is 0.6 blocks across and 1.8 tall, so with the feet at (8.5, 10.0,
//! 8.5) it spans x and z from 8.2 to 8.8 — voxel column 8 on both — and y from
//! 10.0 to 11.8, which is voxel rows 10 **and** 11. The eye sits at 11.62,
//! inside the head's own cell.
//!
//! The **downward** run is pitched 80 degrees below level: it leaves row 11 after
//! 0.62 blocks of descent, 0.63 blocks along the ray and still inside column 8,
//! and enters the water declared at (8, 10, 8) through that cell's upward face.
//! That cell is the one the feet are in. The **level** run looks along +x and
//! crosses x = 9 half a block along, entering the water declared at (9, 11, 8)
//! through its −x face — a cell one column clear of the box on every axis.

mod support;

use std::error::Error;
use std::sync::Arc;

use glam::Vec3;
use mc_core::id::BlockName;
use mc_sim::action::{ActionIntent, EditReport, Refusal, TickIntent};
use mc_sim::player::{BlockPos, MovementIntent, PlayerState};
use mc_sim::simulation::seat;
use mc_sim::world::World;
use mc_world::section::Contents;
use mc_world::world::{VoxelWorld, WorldPos};

use support::chamber::{BUILDABLE, BlockChamber, at, differences, fixture_content};
use support::{DIRT, STONE, TestResult, WATER, content_registry, published_content};

/// Every cell at which a run differs from the fixture as declared.
type Changes = Vec<(WorldPos, String, String)>;

/// What one action answered, and what it did to the world.
type Acted = (Option<EditReport>, Changes);

/// How many chunk columns each fixture world spans on each axis.
const COLUMNS: u32 = 1;

/// The layer the floor occupies, so its top face is at `FLOOR_LAYER + 1`.
const FLOOR_LAYER: u32 = 9;

/// How far the shipped-content floor runs on each horizontal axis.
const FLOOR_SPAN: u32 = 16;

/// The voxel row the eye is in, `floor(10.0 + 1.62)`.
const EYE_ROW: u32 = 11;

/// The solid, replaceable block the level ray stops at, four blocks from an eye
/// at x = 8.0. The two cells in front of it are empty, so the rule that lands a
/// placement one step back has somewhere to put it.
const THE_REPLACEABLE_BLOCK: WorldPos = at(12, EYE_ROW, 8);

/// Where the feet stand for the fixture-registry run.
const LINED_UP: Vec3 = Vec3::new(8.0, 10.0, 8.5);

/// Where the feet stand for the shipped-water runs.
const ON_THE_FLOOR: Vec3 = Vec3::new(8.5, 10.0, 8.5);

/// The cell of water the player is standing in — the row the feet are in, in the
/// box's own column.
const UNDER_THE_PLAYER: WorldPos = at(8, 10, 8);

/// The cell of water beside the player, one column clear of the box.
const BESIDE_THE_PLAYER: WorldPos = at(9, EYE_ROW, 8);

/// Yaw facing +x, and how far below level each run aims, in degrees.
const ALONG_THE_ROW: f32 = 0.0;
const LEVEL: f32 = 0.0;
const AT_THE_PLAYERS_OWN_FEET: f32 = -80.0;

#[test]
fn a_place_aimed_at_a_replaceable_block_lands_in_that_blocks_own_cell() -> TestResult {
    let answered = placing_over_the_fixture_registry(looking(LINED_UP, LEVEL))?;

    assert_eq!(
        answered,
        (
            Some(EditReport::Changed {
                cell: signed(THE_REPLACEABLE_BLOCK),
                from: Contents::Holds(BlockName::parse(BUILDABLE)?),
                to: Contents::Holds(BlockName::parse(DIRT)?),
            }),
            vec![(THE_REPLACEABLE_BLOCK, BUILDABLE.to_owned(), DIRT.to_owned())]
        ),
        "the block this ray stops at is one content declares may be built over, so the placement \
         goes into that cell and replaces it. The rule that lands a placement one step back puts \
         the block in the empty cell in front of it instead, which is a different cell and a \
         different `from` — and both halves of the report say so. Nothing here is refused: the \
         cell one step back is empty and the player is four blocks away from either of them"
    );
    Ok(())
}

#[test]
fn a_place_aimed_at_a_replaceable_cell_the_player_is_standing_in_changes_nothing() -> TestResult {
    let refusing = placing(looking(ON_THE_FLOOR, AT_THE_PLAYERS_OWN_FEET))?;
    let accepting = placing(looking(ON_THE_FLOOR, LEVEL))?;

    assert_eq!(
        (refusing, accepting),
        (
            (Some(EditReport::Refused(Refusal::InsidePlayer)), nothing()),
            (
                Some(EditReport::Changed {
                    cell: signed(BESIDE_THE_PLAYER),
                    from: Contents::Holds(BlockName::parse(WATER)?),
                    to: Contents::Holds(BlockName::parse(DIRT)?),
                }),
                vec![(BESIDE_THE_PLAYER, WATER.to_owned(), DIRT.to_owned())]
            )
        ),
        "landing a placement in the cell that was hit rather than one step back changes *which* \
         cell is chosen and nothing about whether the player is standing in it. Aimed down, the \
         cell chosen is the one the feet are in and the request has to be refused by name — a \
         branch that picked the cell and skipped the box check builds a block through the player \
         while the other half of this pair, and every scenario about water, stays green. Aimed \
         level at the identical fixture, the cell chosen is one column clear of the box and the \
         same request goes through"
    );
    Ok(())
}

/// A floor, and one solid block a placement may overwrite standing four blocks
/// along the ray, with the two cells in front of it empty.
fn one_replaceable_block_with_empty_space_in_front_of_it() -> BlockChamber {
    BlockChamber::empty(COLUMNS)
        .run(at(0, FLOOR_LAYER, 0), at(16, FLOOR_LAYER + 1, 16), STONE)
        .cell(THE_REPLACEABLE_BLOCK, BUILDABLE)
}

/// One tick over a fresh build of the fixture-registry world asking for one
/// placement of dirt, and what that did to the world compared with the same
/// world as declared.
fn placing_over_the_fixture_registry(player: PlayerState) -> Result<Acted, Box<dyn Error>> {
    let chamber = one_replaceable_block_with_empty_space_in_front_of_it();
    let declared = chamber.build()?;
    let mut simulation = seat(player, chamber.build()?, fixture_content()?).simulation;
    let report = simulation.advance(a_placement_of_dirt()?);
    Ok((report, differences(&declared, simulation.world())))
}

/// One tick over a fresh build of the shipped-content world asking for one
/// placement of dirt, and what that did to the world compared with the same
/// world as declared.
fn placing(player: PlayerState) -> Result<Acted, Box<dyn Error>> {
    let declared = water_under_the_player_and_beside_them()?;
    let played = water_under_the_player_and_beside_them()?;
    let content = published_content(played.registry())?;
    let mut simulation = seat(player, played, content).simulation;
    let report = simulation.advance(a_placement_of_dirt()?);
    Ok((report, differences(&declared, simulation.world())))
}

/// A stone floor across the whole column, with one cell of water in the row the
/// feet are in and one in the row the eye is in, a column along.
fn water_under_the_player_and_beside_them() -> Result<World, Box<dyn Error>> {
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
    let water = BlockName::parse(WATER)?;
    blocks.set_block(UNDER_THE_PLAYER, &water, &registry)?;
    blocks.set_block(BESIDE_THE_PLAYER, &water, &registry)?;
    Ok(World::new(blocks, registry)?)
}

/// One tick asking for no movement and one placement of dirt.
fn a_placement_of_dirt() -> Result<TickIntent, Box<dyn Error>> {
    Ok(TickIntent {
        movement: MovementIntent::default(),
        action: Some(ActionIntent::Place {
            block: BlockName::parse(DIRT)?,
        }),
    })
}

/// A player standing still on the floor at `feet`, facing along the row, pitched
/// `pitch` degrees from level.
fn looking(feet: Vec3, pitch: f32) -> PlayerState {
    PlayerState {
        position: feet,
        velocity: Vec3::ZERO,
        yaw: ALONG_THE_ROW,
        pitch: pitch.to_radians(),
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

/// No cell of the fixture moved.
fn nothing() -> Changes {
    Vec::new()
}
