//! A placement may not put a block where the player already is.
//!
//! The player's box is 0.6 blocks across and 1.8 tall, so with the feet at
//! (0.5, 10.0, 0.5) it spans x and z from 0.2 to 0.8 — voxel column 0 on both —
//! and y from 10.0 to 11.8, which is voxel rows 10 **and** 11. Two cells,
//! (0, 10, 0) and (0, 11, 0), and a check that looked only at the one the feet
//! are in would let a block be built through the player's head while every other
//! placement scenario in the suite stayed green. That is the whole reason these
//! two tests are a pair and why their numbers are the specification's.
//!
//! # The two rays, and where each of them comes from
//!
//! The eye is the feet plus 1.62 blocks, at (0.5, 11.62, 0.5), inside the head's
//! own cell.
//!
//! The **head** run looks level along +x: it crosses x = 1 half a block along
//! and meets the −x face of the block declared at (1, 11, 0). The cell it came
//! from is (0, 11, 0) — the head.
//!
//! The **feet** run looks 60° below level, along (0.5, −0.866, 0). It drops out
//! of row 11 after 0.62 blocks of descent, 0.716 blocks along the ray and still
//! inside column 0, then crosses x = 1 at exactly 1.0 blocks along while at
//! y = 10.754 — the middle of row 10 — and meets the −x face of the block
//! declared at (1, 10, 0). The cell it came from is (0, 10, 0) — the feet.
//!
//! # What the accepting halves are, and why they cannot be nearer
//!
//! A block adjacent to the player's own box always yields a placement cell the
//! player occupies; that is the scenario, not an accident of the fixture. So an
//! accepting run has to target something one cell further away, which each half
//! does by declaring **one cell less**: with the near block gone the same ray
//! carries on to a block the fixture already holds, and the cell it then comes
//! from is one the player is nowhere near.
//!
//! The floor is deliberately two cells wide rather than a layer. The feet run
//! descends past y = 10 at x ≈ 1.44, so a floor spanning the whole world would
//! be what its accepting half met — and the fixture would stop being able to say
//! which block was aimed at.

mod support;

use std::error::Error;

use glam::Vec3;
use mc_core::id::BlockName;
use mc_sim::action::{ActionIntent, EditReport, Refusal, TickIntent};
use mc_sim::player::{MovementIntent, PlayerState};
use mc_sim::simulation::Simulation;
use mc_world::world::WorldPos;

use support::chamber::{BlockChamber, at, differences};
use support::{AIR, DIRT, STONE, TestResult};

/// Every cell at which a run differs from the fixture as declared.
type Changes = Vec<(WorldPos, String, String)>;

/// What one placement answered, and what it did to the world.
type Placement = (Option<EditReport>, Changes);

/// How many chunk columns the fixture world spans on each axis.
const COLUMNS: u32 = 1;

/// The layer the floor occupies, so its top face is at `FLOOR_LAYER + 1`.
const FLOOR_LAYER: u32 = 9;

/// Where the feet stand, in the world's own corner column.
const IN_THE_CORNER: Vec3 = Vec3::new(0.5, 10.0, 0.5);

/// The block whose −x face the feet run meets, one column along in the feet's
/// own row.
const AGAINST_THE_FEET: WorldPos = at(1, 10, 0);

/// The block whose −x face the head run meets, one column along in the head's
/// own row.
const AGAINST_THE_HEAD: WorldPos = at(1, 11, 0);

/// The block the head run meets once the near one is gone, one column further
/// along the same row.
const BEYOND_THE_HEAD: WorldPos = at(2, 11, 0);

/// The far cell of the floor, which is what the feet run meets once the near
/// block is gone: its upward face, not its side.
const THE_FLOORS_FAR_CELL: WorldPos = at(1, FLOOR_LAYER, 0);

/// Yaw facing +x, which is where both rays go.
const ALONG_THE_ROW: f32 = 0.0;

/// How far below level each run aims, in degrees.
const AIMED_AT_THE_FEETS_ROW: f32 = -60.0;
const LEVEL: f32 = 0.0;

#[test]
fn a_place_that_would_land_in_the_cell_holding_the_players_feet_changes_nothing() -> TestResult {
    let blocked = a_floor_two_cells_wide().cell(AGAINST_THE_FEET, STONE);
    let open = a_floor_two_cells_wide();
    let (answer, refused) = placing(&blocked, looking(AIMED_AT_THE_FEETS_ROW), DIRT)?;
    let (_, accepted) = placing(&open, looking(AIMED_AT_THE_FEETS_ROW), DIRT)?;

    assert_eq!(
        (answer, refused, accepted),
        (
            Some(EditReport::Refused(Refusal::InsidePlayer)),
            nothing(),
            placed(AGAINST_THE_FEET, AIR, DIRT)
        ),
        "the block this ray meets stands directly beside the feet, so the cell against the face \
         it comes in through is the cell the feet are standing in and nothing may be built there. \
         With that one declared cell removed the identical ray carries on to the floor beyond it \
         and comes in through its upward face instead, and the placement then lands in exactly \
         the cell the first half refused to fill"
    );
    Ok(())
}

#[test]
fn a_place_that_would_land_in_the_cell_holding_the_players_head_changes_nothing() -> TestResult {
    let blocked = a_floor_two_cells_wide()
        .cell(AGAINST_THE_HEAD, STONE)
        .cell(BEYOND_THE_HEAD, STONE);
    let open = a_floor_two_cells_wide().cell(BEYOND_THE_HEAD, STONE);
    let (answer, refused) = placing(&blocked, looking(LEVEL), DIRT)?;
    let (_, accepted) = placing(&open, looking(LEVEL), DIRT)?;

    assert_eq!(
        (answer, refused, accepted),
        (
            Some(EditReport::Refused(Refusal::InsidePlayer)),
            nothing(),
            placed(AGAINST_THE_HEAD, AIR, DIRT)
        ),
        "the player's box is 1.8 blocks tall, so it stands in two voxel rows and this ray's \
         placement lands in the upper one. An overlap test that asked only about the row the feet \
         are in calls that cell free and builds a block through the player's head, while every \
         other placement scenario in the suite — including the one about the feet — stays green. \
         One declared cell further on, the same ray reaches a block whose near cell the player is \
         nowhere near, and the placement goes through"
    );
    Ok(())
}

/// Two cells of floor under and beside the player, and nothing else.
///
/// Enough to stand on — the box's footprint is entirely inside the corner
/// column — and short enough that the descending ray leaves it behind rather
/// than landing on it.
fn a_floor_two_cells_wide() -> BlockChamber {
    BlockChamber::filled_with(COLUMNS, AIR).run(
        at(0, FLOOR_LAYER, 0),
        at(THE_FLOORS_FAR_CELL.x + 1, FLOOR_LAYER + 1, 1),
        STONE,
    )
}

/// A player standing in the corner, facing +x, pitched `pitch` degrees from
/// level.
fn looking(pitch: f32) -> PlayerState {
    PlayerState {
        position: IN_THE_CORNER,
        velocity: Vec3::ZERO,
        yaw: ALONG_THE_ROW,
        pitch: pitch.to_radians(),
        on_ground: true,
    }
}

/// One tick over a fresh build of `chamber` asking for one placement of `block`,
/// and what that did to the world compared with the same chamber as declared.
fn placing(
    chamber: &BlockChamber,
    player: PlayerState,
    block: &str,
) -> Result<Placement, Box<dyn Error>> {
    let declared = chamber.build()?;
    let mut simulation = Simulation::new(player, chamber.build()?);
    let report = simulation.advance(TickIntent {
        movement: MovementIntent::default(),
        action: Some(ActionIntent::Place {
            block: BlockName::parse(block)?,
        }),
    });
    Ok((report, differences(&declared, simulation.world())))
}

/// The one change a placement into `cell` is expected to make.
fn placed(cell: WorldPos, from: &str, into: &str) -> Changes {
    vec![(cell, from.to_owned(), into.to_owned())]
}

/// No cell of the fixture moved.
fn nothing() -> Changes {
    Vec::new()
}
