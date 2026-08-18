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
//!
//! # The refused scenario's control was rebuilt when entry clearing shipped, and why
//! it was rebuilt rather than dropped
//!
//! Its named mutation — the clearing search called from the *refusal* path — bites
//! only while the player's box already stands in something solid **and** the search
//! has somewhere to put them. That state used to be reached by declaring a spawn
//! inside a block. Entry clearing removed it: the admission door now moves such a
//! player before any reload happens, so the fixture asserted a position the run no
//! longer produced, and its premise guard went on passing because it measured the
//! declared spawn rather than the seated one.
//!
//! **Weakening it was refused.** This is the only instrument in the project for "the
//! clearing search is never reached from the refusal path", and the spec that broke
//! it is the one that would have been paying for it — the cost would have landed on
//! whoever ships the next change to that path, reading a green suite and believing
//! something nobody had checked since the day it was weakened. A control that is
//! cheap to restore and expensive to lose is restored.
//!
//! So the state is reached the way a player reaches it. The pocket is packed solid
//! through the whole of the search, which is a world entry itself leaves the player
//! embedded in — nothing within eight blocks is clear, so the door reports that and
//! moves nobody. Their feet are in stone and their head is in water, which is *not*
//! solid while the shipped content is serving: that is what leaves their eye in a
//! cell they can aim out of, since `crates/mc-sim/src/world/action/trace.rs` gives an
//! eye inside a solid block a target at distance zero and no face. One break
//! straight up empties the cell over their head, and the position one block up — head
//! cell and broken cell — becomes the somewhere the wrongly-reached search would take
//! them to. Nothing clears a player on a break, so they are still standing in stone
//! when the refused candidate arrives.

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
use mc_sim::action::EditReport;
use mc_world::world::VoxelWorld;
use winit::event::MouseButton;

use input::InputHarness;
use reload_world::AIM_AT_THE_CEILING;

use reload::{GRASS, GRASS_FILE, STONE, WATER, WATER_FILE, candidate, restating, shipped};
use reload_clearing::{
    Clearance, at, clearance_of, holding_blocks_it_does_not_declare, standing_of,
};
use reload_trap::{
    A_SEARCH_OF, FEET_ROW, HEAD_ROW, ON_THE_FLOOR, Shape, a_client_over, a_world,
    require_a_refusal_could_have_moved_them, require_the_reload_misses, water_that_is_solid,
    within_the_search,
};
use reload_watch::solidity_of;
use reload_world::{ACROSS, Cell, Edit, edit, registry_of, standing_at};
use support::content::ContentRoot;
use support::{TestResult, content_root};

/// How many chunk columns square the worlds here are.
const ONE_COLUMN: u32 = 1;

/// Where the player whose reload misses them stands: inside the column at `(8, 8)`
/// but at neither its centre in `x` nor in `z`, an eighth of a block off each.
const OFF_CENTRE: Vec3 = Vec3::new(8.625, ON_THE_FLOOR, 8.375);

/// Where the run seats the player whose candidate is refused: the centre of the same
/// column, feet inside a solid cell and head inside water.
///
/// **The declared spawn and the seated position are the same value here, and that is
/// a property of the fixture rather than an assumption.** Entry clearing leaves this
/// player exactly where they are because the pocket has nothing clear anywhere inside
/// the search; nothing below reads this constant as if it were the seated position —
/// the premise guard reads the published snapshot, and the assertion compares against
/// what the run answered.
const EMBEDDED: Vec3 = Vec3::new(8.5, ON_THE_FLOOR, 8.5);

/// The cell the embedded player's feet are in, the cell their head is in, and the
/// cell they break open above it.
///
/// Breaking the third is what turns the position one block up — the head cell and the
/// broken one — into somewhere the search could put them, without taking away the
/// stone their feet are in.
const FEET_CELL: Cell = (8, FEET_ROW, 8);
const HEAD_CELL: Cell = (8, HEAD_ROW, 8);
const BROKEN_OPEN: Cell = (8, HEAD_ROW + 1, 8);

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
    let root = content_root()?;
    let (mut client, _) = a_client_over(&root, standing_at(EMBEDDED), |registry| {
        a_pocket_opened_at(registry, &[])
    })?;
    let broke = edit(a_break_straight_up(&mut client));
    require_a_refusal_could_have_moved_them(
        &a_pocket_opened_at(&registry_of(&root)?.clone(), &[BROKEN_OPEN])?,
        client.published(),
    )?;

    let said = clearance_of(client.adopt(candidate(water_solid_and_grass_dropped()?.path())?));
    let after = standing_of(client.tick());
    let water_now = solidity_of(&client, WATER)?;

    assert_eq!(
        (broke, said, after, water_now),
        (
            Edit::Emptied(BROKEN_OPEN),
            holding_blocks_it_does_not_declare(&[GRASS]),
            at(EMBEDDED),
            Some(false)
        ),
        "the candidate declares `base:water` solid — which would put the player further inside solid rock — and stops declaring `base:grass`, which the floor under the pocket holds, so it is turned away. Nobody is moved because the search never runs. **The player is standing in stone with exactly one way out, and that is the whole point of this fixture**: a search wrongly reached from the refusal path runs against the solidity the world still has, so unless there is both something to clear them out of and somewhere to clear them to, that call answers `Unneeded` or `NoClearSpaceWithin`, moves nobody, and the defect this scenario exists to catch leaves it green. The break is asserted here because it is what opens the way out — a run where it was refused or landed elsewhere is a run whose premise the guard above measured against a world the client is not playing"
    );
    Ok(())
}

/// The one break: aimed straight up from an eye standing in water, spent on the next
/// tick.
///
/// Their head cell is the non-solid one precisely so this is possible. An eye inside
/// a solid block has a target at distance zero, so an embedded player whose head cell
/// were stone could only ever break the cell they are standing in — which would let
/// them out of the very thing this scenario needs them to be in.
fn a_break_straight_up(client: &mut InputHarness) -> Option<EditReport> {
    client.move_pointer(0.0, AIM_AT_THE_CEILING);
    client.click(MouseButton::Left);
    client.edit()
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

/// A grass floor under a pocket packed solid through the whole of the search, with
/// `opened` left empty.
///
/// **Packed rather than sparse, because that is what makes entry leave the player
/// where the fixture puts them.** Nothing anywhere inside the eight blocks the search
/// looks at is clear, so the admission door reports that it found nowhere and moves
/// nobody — and the player is standing in stone when the reload arrives, which is the
/// state the named mutation needs.
///
/// The head cell holds water rather than stone, for two reasons that happen to be the
/// same cell. It is what the refused candidate would make solid, so the candidate
/// really is one that would have trapped them further; and it leaves their eye in a
/// cell they can aim out of.
///
/// The player is stable in it — a box overlapping a block is resolved back onto the
/// face below it, not pushed out — so no tick moves them and the readings are of the
/// position the run seated them at.
fn a_pocket_opened_at(
    registry: &BlockRegistry,
    opened: &[Cell],
) -> Result<VoxelWorld, Box<dyn Error>> {
    let cells: Vec<(Cell, &str)> = the_pocket()
        .into_iter()
        .filter(|cell| !opened.contains(cell))
        .map(|cell| (cell, if cell == HEAD_CELL { WATER } else { STONE }))
        .collect();
    a_floor_holding(registry, &cells)
}

/// Every cell of the pocket: the whole of what the search may look at from the
/// player's own cell, clipped to the world that holds it.
///
/// Derived from the declared reach on every axis rather than from a count, so a
/// fixture that has to fill everything the search can see follows the bound and goes
/// on following it if it ever changes. The clip is what makes the world writable at
/// all — a write past an edge is refused — and the cells it drops are outside the
/// world, which the search reads as unknown rather than clear.
fn the_pocket() -> Vec<Cell> {
    let reach = A_SEARCH_OF as i32;
    let rows: Vec<i32> = (FEET_ROW..=FEET_ROW + reach).collect();
    let across = (ONE_COLUMN * ACROSS) as i32;
    within_the_search(FEET_CELL, &rows)
        .into_iter()
        .filter(|(x, _, z)| (0..across).contains(x) && (0..across).contains(z))
        .collect()
}
