//! The two reloads that move nobody: one whose candidate was taken up and did not
//! reach the player, and one whose candidate was turned away.
//!
//! # Not moved at all, rather than moved to where they already were
//!
//! Being cleared costs a player their sub-block position, because the search puts
//! them at a cell centre. Not being cleared leaves it exactly. So the player in the
//! first scenario stands at neither the centre of their column in `x` nor in `z` —
//! an eighth of a block off each, both exact in binary — and a move to the centre of
//! the cell they are already in is therefore a move this reading can see. A fixture
//! that spawned them at `8.5` would have made "left exactly where they are" and
//! "moved to the middle of their own cell" the same answer.
//!
//! # Both readings are taken one tick after the swap
//!
//! The swap publishes no tick of its own, so an assertion reading the snapshot that
//! stands when the candidate is handed over cannot see anything the swap did to the
//! player — which is exactly how a scenario asserting that nothing moved passes
//! while something did. One tick later is the first snapshot a clearing move could
//! have been written into, and the player in each of these worlds is standing on a
//! floor, so that tick's gravity resolves back onto the same face.
//!
//! # Each carries its own control, in the same comparison
//!
//! "The player did not move" is satisfied for good by a reload that never happened,
//! so each assertion reads what the client is now serving alongside the position:
//! the taken-up candidate must have made `base:water` solid, and the refused one
//! must have left it as it was.

#[path = "support/input/mod.rs"]
mod input;
#[path = "support/reload.rs"]
mod reload;
#[path = "support/reload_clearing.rs"]
mod reload_clearing;
#[path = "support/reload_trap.rs"]
mod reload_trap;
#[path = "support/reload_watch.rs"]
mod reload_watch;
#[path = "support/reload_world.rs"]
mod reload_world;
mod support;

use std::error::Error;

use glam::Vec3;
use mc_core::block::BlockRegistry;
use mc_world::world::VoxelWorld;

use reload::{GRASS, GRASS_FILE, STONE, WATER, WATER_FILE, candidate, restating, shipped};
use reload_clearing::{
    Clearance, at, clearance_of, holding_blocks_it_does_not_declare, standing_of,
};
use reload_trap::{
    FEET_ROW, HEAD_ROW, ON_THE_FLOOR, Shape, a_client_over, a_world,
    require_a_refusal_could_have_moved_them, require_the_reload_misses, water_that_is_solid,
};
use reload_watch::solidity_of;
use reload_world::{Cell, standing_at};
use support::content::ContentRoot;
use support::{TestResult, content_root};

/// How many chunk columns square the worlds here are.
const ONE_COLUMN: u32 = 1;

/// Where the player whose reload misses them stands: inside the column at `(8, 8)`
/// but at neither its centre in `x` nor in `z`, an eighth of a block off each.
const OFF_CENTRE: Vec3 = Vec3::new(8.625, ON_THE_FLOOR, 8.375);

/// Where the player whose candidate is refused stands: the centre of the same
/// column, with their head already inside a solid block.
const EMBEDDED: Vec3 = Vec3::new(8.5, ON_THE_FLOOR, 8.5);

/// The cell a candidate makes solid that no player's box here reaches — four
/// columns away from either of them.
const WELL_CLEAR_OF_THEM: Cell = (12, FEET_ROW, 12);

#[test]
fn a_reload_that_makes_a_cell_no_part_of_the_player_stands_in_solid_moves_them_nowhere()
-> TestResult {
    let (mut client, declared) = a_client_over(&content_root()?, standing_at(OFF_CENTRE), a_floor)?;
    require_the_reload_misses(&declared, OFF_CENTRE)?;

    let said = clearance_of(client.adopt(candidate(water_declared_solid()?.path())?));
    let after = standing_of(client.tick());
    let water_now = solidity_of(&client, WATER)?;

    assert_eq!(
        (said, after, water_now),
        (Clearance::NoMoveNeeded, at(OFF_CENTRE), Some(true)),
        "the author has made `base:water` solid and the cell that holds it is four columns from the \
         player, so there is nothing to move them out of. **Not moved at all, and that is what the \
         off-centre spawn is for**: a search answering `MovedTo` with the centre of the cell they \
         are already standing in would put them at (8.5, _, 8.5) and lose the eighth of a block \
         they were carrying on each axis. The solidity read back is what says the reload happened, \
         without which this whole comparison is satisfied by a candidate nobody ever took up"
    );
    Ok(())
}

#[test]
fn a_candidate_that_would_have_trapped_the_player_and_was_refused_moves_them_nowhere() -> TestResult
{
    let (mut client, declared) = a_client_over(&content_root()?, standing_at(EMBEDDED), a_pocket)?;
    require_a_refusal_could_have_moved_them(&declared, EMBEDDED)?;

    let said = clearance_of(client.adopt(candidate(water_solid_and_grass_dropped()?.path())?));
    let after = standing_of(client.tick());
    let water_now = solidity_of(&client, WATER)?;

    assert_eq!(
        (said, after, water_now),
        (
            holding_blocks_it_does_not_declare(&[GRASS]),
            at(EMBEDDED),
            Some(false)
        ),
        "the candidate declares `base:water` solid — which would put the player inside solid rock — \
         and stops declaring `base:grass`, which the floor under them holds, so it is turned away. \
         Nobody is moved because the search never runs. **The player's head is already inside a \
         solid block, and that is the whole point of this fixture**: a search wrongly reached from \
         the refusal path runs against the solidity the world still has, so unless there is already \
         something to clear them out of, that call answers `Unneeded` and the defect this scenario \
         exists to catch leaves it green"
    );
    Ok(())
}

/// A copy of the shipped root whose `water.luau` declares `base:water` solid.
fn water_declared_solid() -> Result<ContentRoot, Box<dyn Error>> {
    restating(shipped()?, WATER_FILE, &water_that_is_solid())
}

/// The same, also stopping declaring `base:grass` — which the floor holds, so the
/// candidate is refused for it.
///
/// **Both edits are needed and neither is decoration.** Without the solid water the
/// candidate could never have trapped anybody, and the scenario would be about a
/// refusal with no clearing anywhere near it. Without the dropped declaration it
/// would be accepted.
fn water_solid_and_grass_dropped() -> Result<ContentRoot, Box<dyn Error>> {
    water_declared_solid()?.not_declaring_blocks(&[GRASS_FILE])
}

/// One column of grass floor with `cells` written above it.
fn a_floor_holding(
    registry: &BlockRegistry,
    cells: &[(Cell, &str)],
) -> Result<VoxelWorld, Box<dyn Error>> {
    a_world(
        registry,
        &Shape {
            columns: ONE_COLUMN,
            floor: Some(GRASS),
            open: &[],
            cells,
        },
    )
}

/// A grass floor holding one cell of water, well clear of where the player stands.
fn a_floor(registry: &BlockRegistry) -> Result<VoxelWorld, Box<dyn Error>> {
    a_floor_holding(registry, &[(WELL_CLEAR_OF_THEM, WATER)])
}

/// A grass floor with water in the cell the player's feet are in and stone in the
/// one their head reaches.
///
/// The stone is what the shipped content already calls solid, so the box the player
/// carries overlaps something before anybody edits anything; the water is what the
/// candidate would add to that. The player is stable there — a box overlapping a
/// block is resolved back onto the face below it, not pushed out.
fn a_pocket(registry: &BlockRegistry) -> Result<VoxelWorld, Box<dyn Error>> {
    a_floor_holding(
        registry,
        &[((8, FEET_ROW, 8), WATER), ((8, HEAD_ROW, 8), STONE)],
    )
}
