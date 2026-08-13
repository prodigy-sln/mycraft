//! What the world and the registry refuse a placement, and by which name.
//!
//! # Every refusal here asserts a difference, and asserts the reason
//!
//! "No block changed" is satisfied by an implementation that refuses everything
//! and by a fixture where nothing was ever targeted, so each test below runs the
//! same request twice — once in the refusing configuration and once in the
//! minimally different accepting one, one declared cell or one requested name
//! apart — and carries both answers in one assertion.
//!
//! That is still not enough on its own, and the unregistered-name scenario is
//! why. The store refuses a name the registry does not know whatever the
//! resolution does, so deleting the check *before* the write leaves the world
//! unchanged and a test asserting only "nothing moved" reports a kill it never
//! made. Each refusal is therefore named: `UnknownBlock` and not `Storage`,
//! `Occupied` and not merely "unchanged".
//!
//! # The fixture block whose replaceability and solidity disagree
//!
//! Every block the base game ships has the two agreeing — air and water are
//! non-solid and replaceable, dirt, grass and stone are solid and not — so
//! against shipped content alone a placement check reading `!is_solid` and one
//! reading `replaceable` answer identically at every cell. The overlay's
//! unbuildable block is **not solid and not replaceable**: a ray goes straight
//! through it, so it can sit in the cell a placement would land in, and a
//! placement into it is refused under the right reading and allowed under the
//! wrong one. It is the only fixture in the suite that tells the two apart.
//!
//! # The world's ceiling, and why the player stands above it
//!
//! A placement lands in the cell the ray came *from*, which is the last cell it
//! passed through before it stopped. For that cell to lie outside the world the
//! ray has to have been outside the world, so the out-of-range fixture puts the
//! player above the world's top and aims steeply down at a block on its topmost
//! layer. The eye is at 259.612 after one tick of fall — 258.0 feet, less
//! 0.00833 of a tick's gravity, plus 1.62 of eye height — and the view is
//! pitched 75° below level, so the ray descends the 3.612 blocks to y = 256 over
//! 0.968 of a block of x and meets the target's upward face at 3.739 blocks,
//! inside column 9 and well inside the reach. The cell it came from is
//! (9, 256, 8), one layer above everything the world can store.
//!
//! The accepting half is that same fixture one block lower, so the two runs
//! differ in whether the cell the placement lands in is the world's topmost
//! layer or the first one past it — the boundary asserted from both sides, the
//! way the reach already is.

mod support;

use std::error::Error;

use glam::Vec3;
use mc_core::id::BlockName;
use mc_sim::action::{ActionIntent, EditReport, Refusal, TickIntent};
use mc_sim::player::{BlockPos, MovementIntent, PlayerState};
use mc_sim::simulation::Simulation;
use mc_world::column::COLUMN_HEIGHT;
use mc_world::world::WorldPos;

use support::chamber::{BlockChamber, UNBUILDABLE, at, differences};
use support::{AIR, DIRT, STONE, TestResult};

/// Every cell at which a run differs from the fixture as declared.
type Changes = Vec<(WorldPos, String, String)>;

/// What one placement answered, and what it did to the world.
type Placement = (Option<EditReport>, Changes);

/// How many chunk columns the fixture world spans on each axis.
const COLUMNS: u32 = 1;

/// The layer the floor occupies, so its top face is at `FLOOR_LAYER + 1`.
const FLOOR_LAYER: u32 = 9;

/// The block whose upward face the registry and replaceability scenarios aim at.
const TARGET: WorldPos = at(9, 10, 8);

/// The cell directly above the target, which is where their placement lands.
const ABOVE_THE_TARGET: WorldPos = at(TARGET.x, TARGET.y + 1, TARGET.z);

/// Where the feet stand for them: on the floor, one column short of the target.
const STANDING: Vec3 = Vec3::new(8.5, 10.0, 8.5);

/// How far below level the view is aimed at the target, in degrees.
const AIMED_DOWN: f32 = -30.0;

/// A namespaced name no batch of the fixture registry declares.
///
/// It parses — an unparseable name is a different refusal — and it resolves to
/// nothing, which is the one thing this scenario is about.
const UNREGISTERED: &str = "fixture:unregistered";

/// The block standing on the world's topmost storable layer.
const AT_THE_CEILING: WorldPos = at(9, COLUMN_HEIGHT - 1, 8);

/// The same block one layer lower, which is the accepting half's target.
const JUST_BELOW_THE_CEILING: WorldPos = at(AT_THE_CEILING.x, AT_THE_CEILING.y - 1, 8);

/// The cell a placement against the ceiling block's upward face would land in.
///
/// One layer above everything the world can store, which is what makes it the
/// scenario rather than an ordinary cell.
const PAST_THE_CEILING: BlockPos = BlockPos {
    x: AT_THE_CEILING.x as i32,
    y: (AT_THE_CEILING.y + 1) as i32,
    z: AT_THE_CEILING.z as i32,
};

/// Where the feet start for the two ceiling runs: above the world, and one block
/// lower.
const ABOVE_THE_WORLD: Vec3 = Vec3::new(8.5, 258.0, 8.5);
const ONE_BLOCK_LOWER: Vec3 = Vec3::new(8.5, 257.0, 8.5);

/// How far below level the ceiling runs aim, in degrees.
const AIMED_STEEPLY_DOWN: f32 = -75.0;

/// Yaw facing +x, which is where every ray in this file goes.
const ALONG_THE_ROW: f32 = 0.0;

#[test]
fn a_place_naming_a_block_the_registry_does_not_know_changes_nothing() -> TestResult {
    let chamber = one_block_on_the_floor();
    let (answer, refused) = placing(&chamber, looking_down_at_the_target(), UNREGISTERED)?;
    let (_, accepted) = placing(&chamber, looking_down_at_the_target(), DIRT)?;

    assert_eq!(
        (answer, refused, accepted),
        (
            Some(EditReport::Refused(Refusal::UnknownBlock {
                name: BlockName::parse(UNREGISTERED)?
            })),
            nothing(),
            placed(ABOVE_THE_TARGET, AIR, DIRT)
        ),
        "the same request over the same fixture, differing only in the name it carries: one the \
         registry knows and one it does not. The refusal has to arrive **by name**, because the \
         store refuses an unknown name at the write in any case — so an implementation with no \
         check at all before the write leaves the world exactly as unchanged as a correct one, \
         and a test that only compared worlds could not tell them apart"
    );
    Ok(())
}

#[test]
fn a_place_into_a_cell_holding_a_block_that_is_not_solid_and_not_replaceable_changes_nothing()
-> TestResult {
    let blocked = one_block_on_the_floor().cell(ABOVE_THE_TARGET, UNBUILDABLE);
    let open = one_block_on_the_floor();
    let (answer, refused) = placing(&blocked, looking_down_at_the_target(), DIRT)?;
    let (_, accepted) = placing(&open, looking_down_at_the_target(), DIRT)?;

    assert_eq!(
        (answer, refused, accepted),
        (
            Some(EditReport::Refused(Refusal::Occupied)),
            nothing(),
            placed(ABOVE_THE_TARGET, AIR, DIRT)
        ),
        "the cell the placement lands in holds a block that stops nobody and that content does \
         not declare replaceable, and the two runs differ in that one declared cell and in \
         nothing else. This is the only place in the suite where reading `replaceable` and \
         reading `!is_solid` give different answers: the wrong reading calls this cell empty, \
         because nothing is standing in it, and overwrites a block content said may not be \
         overwritten"
    );
    Ok(())
}

#[test]
fn a_place_that_would_land_outside_the_worlds_storable_range_changes_nothing() -> TestResult {
    let against_the_ceiling = one_block_at(AT_THE_CEILING);
    let one_lower = one_block_at(JUST_BELOW_THE_CEILING);
    let (answer, refused) = placing(&against_the_ceiling, above_the_world(ABOVE_THE_WORLD), DIRT)?;
    let (_, accepted) = placing(&one_lower, above_the_world(ONE_BLOCK_LOWER), DIRT)?;

    assert_eq!(
        (answer, refused, accepted),
        (
            Some(EditReport::Refused(Refusal::OutsideWorld {
                at: PAST_THE_CEILING
            })),
            nothing(),
            placed(AT_THE_CEILING, AIR, DIRT)
        ),
        "the cell this placement would land in is the layer above the last one the world can \
         store, so no cell of the world may hold anything other than what it was declared with. \
         The same drive one block lower lands in the topmost layer the world does have, which is \
         what makes this the *edge* rather than an implementation that refuses everything — and \
         an index that wrapped instead of refusing would write into the bottom of the same \
         column, where the first half would catch it"
    );
    Ok(())
}

/// A floor, and one solid block standing on it in the eye's line of sight.
fn one_block_on_the_floor() -> BlockChamber {
    BlockChamber::filled_with(COLUMNS, AIR)
        .run(at(0, FLOOR_LAYER, 0), at(16, FLOOR_LAYER + 1, 16), STONE)
        .cell(TARGET, STONE)
}

/// Air everywhere, with one solid block at `cell` and no floor at all.
///
/// The ceiling runs need no floor: the player is above the world for the one
/// tick they last, and a tick of gravity moves the eye by 0.00833 of a block —
/// which the arithmetic in this file's header already carries.
fn one_block_at(cell: WorldPos) -> BlockChamber {
    BlockChamber::filled_with(COLUMNS, AIR).cell(cell, STONE)
}

/// A player standing on the floor, looking down at the target.
fn looking_down_at_the_target() -> PlayerState {
    standing(STANDING, AIMED_DOWN)
}

/// A player above the world's top, looking steeply down at the block below it.
fn above_the_world(feet: Vec3) -> PlayerState {
    standing(feet, AIMED_STEEPLY_DOWN)
}

/// A player at `feet` facing along +x, pitched `pitch` degrees from level.
fn standing(feet: Vec3, pitch: f32) -> PlayerState {
    PlayerState {
        position: feet,
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
