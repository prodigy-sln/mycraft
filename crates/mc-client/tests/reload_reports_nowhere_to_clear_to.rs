//! A reload that traps the player with nowhere inside the declared search to put
//! them: the reload stands, the player stays, and a person is told.
//!
//! # The verdict is read out of the client's report and not out of a call this
//! scenario makes
//!
//! The system is required to *report* that a player could not be cleared, and a verdict
//! computed and dropped satisfies nothing. So this is the one scenario in
//! the phase driven through the watcher rather than through the client's own
//! `adopt_content` door: the author writes into the root the running client is
//! playing, a change is reported on it, and the verdict is read where it crosses out
//! of the client's core on its way to the one place that prints it. A test asking a
//! pure call what the search decided would leave the whole of that path ungraded.
//!
//! **What is left ungraded even so, stated rather than left to be found:** `App`
//! turning that report into a line somebody reads. `crates/mc-client/src/app.rs`
//! needs a real window and nothing in this workspace constructs one, so the printing
//! is held by review exactly as `report_remesh`'s and the reload refusal's already
//! are. The halves either side of it are covered — the verdict at the boundary here,
//! and the existing assertions on how a chain is rendered.
//!
//! # The world is two columns square, and it has to be
//!
//! The search may look eight blocks out horizontally, and a position outside the
//! loaded world is not solid — so it is clear, and a player closer than eight columns
//! to an edge would be cleared straight off the footprint. One column is sixteen
//! blocks across and has no such position in it. Two columns square is thirty-two,
//! and the player stands at `(12, 12)`, which leaves the whole cube inside the world.
//!
//! # Nothing here asserts a count of positions tested
//!
//! The bound is graded by reachability: this world blocks every position the search
//! may look at, and the fixture guard says so by walking the cube itself. The spec's
//! `17³ = 4 913` ceiling and the `9 × 17 × 17 = 2 601` the search spends of it are
//! both facts about cost, and an assertion on either would redden against a
//! conforming implementation.

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

use input::InputHarness;
use reload::{GRASS, WATER, WATER_FILE, shipped};
use reload_clearing::{Clearance, at, standing_of, until_cleared};
use reload_trap::{
    A_SEARCH_OF, FEET_ROW, ON_THE_FLOOR, Shape, a_client_watching, a_world, each_holding,
    require_nothing_clear_within_the_search, require_the_reload_traps, water_that_is_solid,
    within_the_search,
};
use reload_watch::{Reports, block_path, restating_raw, solidity_of};
use reload_world::{Cell, standing_at};
use support::TestResult;
use support::content::ContentRoot;

/// How many chunk columns square the world is. See this file's header.
const TWO_COLUMNS: u32 = 2;

/// Where the player stands: the centre of the column at `(12, 12)`, feet on the
/// floor's top face and eight columns clear of every edge.
const WEDGED: Vec3 = Vec3::new(12.5, ON_THE_FLOOR, 12.5);

/// The cell their feet are in, which the filling below is centred on.
const WEDGED_CELL: Cell = (12, FEET_ROW, 12);

#[test]
fn a_reload_with_nowhere_clear_inside_the_bound_leaves_the_player_and_says_so() -> TestResult {
    let root = shipped()?;
    let (mut client, reports) = a_wedged_client(&root)?;
    let root = restating_raw(root, WATER_FILE, &water_that_is_solid().text())?;

    reports.changed(&[block_path(&root, WATER_FILE)])?;
    let said = until_cleared(&mut client);
    let after = standing_of(client.tick());
    let water_now = solidity_of(&client, WATER)?;

    assert_eq!(
        (said, after, water_now),
        (
            Clearance::NoClearSpaceWithin {
                blocks: A_SEARCH_OF
            },
            at(WEDGED),
            Some(true)
        ),
        "the author saved a declaration that turns a lake into rock, and the player was in the \
         middle of it — every position the search may look at is inside the same lake, which the \
         guard above establishes by walking the cube. So the reload stands, the player is left \
         exactly where they were, and the verdict says how far was looked. **`NoClearSpaceWithin` \
         is a value rather than an absence, which is why it can be reported at all**, and reading \
         it out of the client's own report is what says it reaches a person instead of being \
         computed and dropped"
    );
    Ok(())
}

/// A client standing in the middle of a world with nowhere clear inside the search,
/// watching the root it plays.
///
/// **The two guards live here, beside the world that has to satisfy them.** They are
/// the scenario's premises — the reload is what traps the player, and every position
/// the search may look at is blocked — and the second walks the cube itself, so a gap
/// left anywhere in two and a half thousand cells is reported as the gap it is rather
/// than as a search that went looking where it should not have.
///
/// # Errors
///
/// Returns an error if the root does not read, if the world does not build, or if
/// either premise fails.
fn a_wedged_client(root: &ContentRoot) -> Result<(InputHarness, Reports), Box<dyn Error>> {
    let filled = each_holding(&every_cell_the_search_can_reach(), WATER);
    let (client, declared, reports) = a_client_watching(root, standing_at(WEDGED), |registry| {
        a_wedge(registry, &filled)
    })?;
    require_the_reload_traps(&declared, WEDGED)?;
    require_nothing_clear_within_the_search(&declared, WEDGED)?;
    Ok((client, reports))
}

/// Every cell the search can put the player's feet in: their own row and the eight
/// above it, over the whole horizontal reach.
///
/// Derived from [`A_SEARCH_OF`] on both axes, so a world built to wedge a player
/// follows the declared bound rather than a number of its own.
fn every_cell_the_search_can_reach() -> Vec<Cell> {
    let rows: Vec<i32> = (FEET_ROW..=FEET_ROW + A_SEARCH_OF as i32).collect();
    within_the_search(WEDGED_CELL, &rows)
}

/// Two columns square, a grass floor to stand on, and `filled` written above it.
///
/// The floor matters: with it, the tick after the swap resolves the player back onto
/// the same face and the reading is where they were left rather than where they were
/// left plus a fall.
fn a_wedge(
    registry: &BlockRegistry,
    filled: &[(Cell, &str)],
) -> Result<VoxelWorld, Box<dyn Error>> {
    a_world(
        registry,
        &Shape {
            columns: TWO_COLUMNS,
            floor: Some(GRASS),
            open: &[],
            cells: filled,
        },
    )
}
