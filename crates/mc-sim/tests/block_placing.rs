//! Where a placed block lands, and what it is allowed to be.
//!
//! A placement goes in the cell **against the face the ray met**, which is the
//! last cell the ray passed through before it stopped — never the cell it
//! stopped in, and never the one on the far side of it. Those three are one step
//! apart and an edit that took any of them changes exactly one cell, so a test
//! that only counted changes would report all three as correct. Every assertion
//! here therefore names the cell that moved *and* what the cell it was aimed at
//! still holds.
//!
//! # The arithmetic every number here comes from
//!
//! The floor's top face is at y = 10, the feet stand on it, and the eye is the
//! feet plus 1.62 blocks — so the eye is at (8.5, 11.62, 8.5) and the voxel row
//! it sits in is 11. The view is pitched 30° below level, which is the direction
//! (0.866, −0.5, 0): it crosses into the next column at 0.577 blocks, and drops
//! out of row 11 after 0.62 blocks of descent, i.e. 1.24 blocks along the ray, at
//! x = 8.5 + 1.074 = 9.574. That is inside column 9 with more than four tenths of
//! a block of margin either side, and it is the **top face** of the block
//! declared at (9, 10, 8) that the ray comes in through.
//!
//! So the placement lands at (9, 11, 8), the cell directly above the block the
//! ray met. The two cells a wrong step would take are declared as something else
//! on purpose: (9, 10, 8) holds the target itself, and (9, 9, 8) is floor. Three
//! cells, three different answers, and the one a placement is meant for holds
//! nothing at all — which is why it accepts the block: because it is empty, not
//! because content declared it overwritable.
//!
//! # A block that is not solid is placeable, and that is a rule with a history
//!
//! A place naming a non-solid block used to be refused outright, which made
//! water unplaceable. The rule it was guarding — a client placing air to delete a
//! block it could not break — is already carried by replaceability, since a block
//! content does not declare replaceable may not be overwritten by *anything*, air
//! included. The second test here is what catches that refusal being
//! reintroduced.

mod support;

use std::error::Error;

use glam::Vec3;
use mc_core::id::BlockName;
use mc_sim::action::{ActionIntent, EditReport, TickIntent};
use mc_sim::player::{BlockPos, MovementIntent, PlayerState};
use mc_sim::simulation::Simulation;
use mc_sim::world::World;
use mc_world::section::Contents;
use mc_world::world::WorldPos;

use support::chamber::{BlockChamber, at, differences};
use support::{DIRT, NOTHING, STONE, TestResult, WATER, described};

/// How many chunk columns the fixture world spans on each axis.
const COLUMNS: u32 = 1;

/// The layer the floor occupies, so its top face is at `FLOOR_LAYER + 1`.
const FLOOR_LAYER: u32 = 9;

/// Where the feet stand: on the floor's top face.
const FEET_Y: f32 = 10.0;

/// The block whose upward face every ray in this file comes in through.
///
/// It stands on the floor in the row the feet are in, one column along from
/// them, and the ray reaches its top face 1.24 blocks from the eye.
const TARGET: WorldPos = at(9, 10, 8);

/// The cell directly above the target, which is where a placement lands.
///
/// Derived from the target rather than written out, so the two cannot be moved
/// apart by an edit to one of them.
const ABOVE_THE_TARGET: WorldPos = at(TARGET.x, TARGET.y + 1, TARGET.z);

/// Where the feet stand, one column short of the target and level with it.
const STANDING: Vec3 = Vec3::new(8.5, FEET_Y, 8.5);

/// Yaw facing +x, which is where every ray in this file goes.
const ALONG_THE_ROW: f32 = 0.0;

/// How far below level the view is aimed, in degrees.
///
/// Steep enough that the ray leaves the eye's own row inside the target's column
/// and meets its top face, and shallow enough that it meets the target long
/// before it would reach the floor.
const AIMED_DOWN: f32 = -30.0;

#[test]
fn a_place_against_a_blocks_upward_face_leaves_the_requested_block_in_the_cell_above_it()
-> TestResult {
    let chamber = one_block_on_the_floor();
    let declared = chamber.build()?;
    let placed = after_a_place(&chamber, DIRT)?;

    assert_eq!(
        (
            differences(&declared, placed.world()),
            held_at(placed.world(), TARGET)?
        ),
        (
            vec![(ABOVE_THE_TARGET, NOTHING.to_owned(), DIRT.to_owned())],
            STONE.to_owned()
        ),
        "the ray comes in through the target's upward face, so the requested block belongs in the \
         cell directly above it and the target itself has to be left exactly as it was declared. \
         Both halves are named because both are one step from the right answer and both change \
         the same number of cells: overwriting the cell the ray stopped in moves the target, and \
         taking the cell on the far side of the face moves the floor beneath it"
    );
    Ok(())
}

#[test]
fn a_place_naming_a_block_that_is_not_solid_leaves_it_in_the_replaceable_cell_above_the_target()
-> TestResult {
    let chamber = one_block_on_the_floor();
    let declared = chamber.build()?;
    let placed = after_a_place(&chamber, WATER)?;

    assert_eq!(
        differences(&declared, placed.world()),
        vec![(ABOVE_THE_TARGET, NOTHING.to_owned(), WATER.to_owned())],
        "the server checks that a named block is registered and nothing more, so a block that \
         stops nobody is as placeable as one that stops everybody. A refusal here would be the \
         struck non-solid rule back again — the one that cost water its placeability while \
         guarding a door replaceability already locks, since a block content does not declare \
         replaceable may not be overwritten by anything at all"
    );
    Ok(())
}

#[test]
fn a_place_into_a_cell_holding_nothing_reports_it_as_replacing_nothing() -> TestResult {
    let chamber = one_block_on_the_floor();
    let mut simulation = Simulation::new(aiming_down(), chamber.build()?);

    let report = simulation.advance(TickIntent {
        movement: MovementIntent::default(),
        action: Some(ActionIntent::Place {
            block: BlockName::parse(DIRT)?,
        }),
    });

    assert_eq!(
        report,
        Some(EditReport::Changed {
            cell: voxel(ABOVE_THE_TARGET),
            from: Contents::Empty,
            to: Contents::Holds(BlockName::parse(DIRT)?),
        }),
        "the cell the block lands in held nothing, so the report has nothing to name as what \
         was replaced. A report naming some block there describes a placement that overwrote \
         something — which is a refusal's business and not a change's — and the cell-by-cell \
         comparison in the first test cannot see it, because the cell ends holding dirt either \
         way"
    );
    Ok(())
}

/// A floor, and one solid block standing on it in the eye's line of sight.
fn one_block_on_the_floor() -> BlockChamber {
    floored().cell(TARGET, STONE)
}

/// Nothing anywhere, with one layer of floor for the player to stand on.
fn floored() -> BlockChamber {
    BlockChamber::empty(COLUMNS).run(at(0, FLOOR_LAYER, 0), at(16, FLOOR_LAYER + 1, 16), STONE)
}

/// A world position as the signed cell a report names it by.
const fn voxel(cell: WorldPos) -> BlockPos {
    BlockPos {
        x: cell.x as i32,
        y: cell.y as i32,
        z: cell.z as i32,
    }
}

/// A player standing still on the floor, looking down at the target.
fn aiming_down() -> PlayerState {
    PlayerState {
        position: STANDING,
        velocity: Vec3::ZERO,
        yaw: ALONG_THE_ROW,
        pitch: AIMED_DOWN.to_radians(),
        on_ground: true,
    }
}

/// One tick over a fresh build of `chamber`, asking for no movement and one
/// placement of `block`.
///
/// A grounded player asked for no movement ends the tick exactly where it began
/// — the fall of one tick is resolved back onto the floor's own face — so the ray
/// is cast from the declared eye and not from somewhere a tick of gravity left.
fn after_a_place(chamber: &BlockChamber, block: &str) -> Result<Simulation, Box<dyn Error>> {
    let mut simulation = Simulation::new(aiming_down(), chamber.build()?);
    simulation.advance(TickIntent {
        movement: MovementIntent::default(),
        action: Some(ActionIntent::Place {
            block: BlockName::parse(block)?,
        }),
    });
    Ok(simulation)
}

/// What one cell of a world holds: a block by name, or [`NOTHING`].
///
/// A cell the world does not reach at all is an error rather than either of
/// those — the fixture declares every cell it asks about, so a world answering
/// "outside" here is the fixture being wrong about itself.
fn held_at(world: &World, cell: WorldPos) -> Result<String, Box<dyn Error>> {
    let held = world
        .block_at(voxel(cell))
        .ok_or_else(|| format!("the world reaches no cell at {cell:?}"))?;
    Ok(described(held))
}
