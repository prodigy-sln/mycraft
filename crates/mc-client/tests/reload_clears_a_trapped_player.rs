//! Where a reload puts a player whose box it made solid: sideways before upward,
//! never downward, and with nothing of the move left in their velocity.
//!
//! # Every reading is taken one tick after the swap
//!
//! The swap publishes no tick of its own, so the snapshot standing at the moment a
//! candidate is taken up was written by the *previous* `advance` and nothing the
//! clearing search does could have reached it. Each scenario below hands over its
//! candidate and then advances exactly one tick, which is the first snapshot the
//! move could have been written into — and every destination here is a cell whose
//! own floor holds the player up, so that one tick of gravity resolves back onto
//! the same face and the reading is the move rather than the move plus a fall.
//!
//! # Every destination is worked out from the declared order, and each is chosen so
//! that the horizontal metric cannot decide it
//!
//! The order is `(dy, max(|dx|, |dz|), dz, dx)` ascending over cell centres, with
//! `dy ∈ [0, 8]` and `dx, dz ∈ [-8, 8]`. Each world below blocks every candidate
//! ordered before the one it names, so the expected cell is the first clear one —
//! and in each case the cells blocked include every diagonal that a Chebyshev reach
//! would consider at that distance and a Euclidean one would not. **So the
//! expectations do not turn on which horizontal metric the search uses**, only on
//! `dy` coming first and on the declared `(dz, dx)` tie-break, both of which the
//! architecture states.
//!
//! # No expectation here is a count of positions tested
//!
//! The spec's declared ceiling is `17³ = 4 913` and the search spends
//! `9 × 17 × 17 = 2 601` of it. Both are numbers about cost rather than behaviour,
//! and an assertion on either would redden against a conforming implementation.
//! What these scenarios grade is which cell the player ends up in.

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
use mc_sim::player::PlayerState;
use mc_world::world::VoxelWorld;

use reload::{GRASS, WATER, WATER_FILE, candidate, restating, shipped};
use reload_clearing::{at, clearance_of, moved_to, standing_of};
use reload_trap::{
    FEET_ROW, HEAD_ROW, ON_THE_FLOOR, Shape, a_client_over, a_world, airborne_at, each_holding,
    require_a_clear_position_at, require_rising, require_the_reload_traps, rising_at,
    water_that_is_solid, within_the_search,
};
use reload_world::{Cell, standing_at};
use support::content::ContentRoot;
use support::{TestResult, content_root};

/// How many chunk columns square the small worlds here are.
///
/// One is sixteen blocks across, which reaches every candidate the scenarios below
/// name — the furthest is two columns from the player.
const ONE_COLUMN: u32 = 1;

/// How many square the wide world is.
///
/// The scenario that has to block the whole of the search's own horizontal reach
/// needs a world in which every position inside that reach is a position the world
/// holds: a player eight columns from an edge is not, and a cell past the edge is
/// not solid, so it would be clear and the search would find it.
const TWO_COLUMNS: u32 = 2;

/// Where the player stands in the small worlds: the centre of the column at
/// `(8, 8)`, feet on the floor's top face.
const TRAPPED: Vec3 = Vec3::new(8.5, ON_THE_FLOOR, 8.5);

/// Where the player stands in the wide world, eight columns clear of every edge.
const WEDGED: Vec3 = Vec3::new(12.5, ON_THE_FLOOR, 12.5);

/// The cell the player's feet are in, and the column beside it the searches below
/// come out in.
const OWN_CELL: Cell = (8, FEET_ROW, 8);

/// How fast the rising player is moving, in blocks per second.
///
/// As fast as a jump leaves the ground (`crates/mc-sim/src/player/physics.rs`,
/// where the constant is private). What the scenario needs of it is only that it be
/// strictly positive: a rise survives the tick after the swap, and a fall is spent
/// by that tick's own collision whether the search zeroed it or not.
const RISING: f32 = 9.0;

#[test]
fn a_reload_that_makes_the_players_own_cell_solid_puts_them_somewhere_clear() -> TestResult {
    /// The first candidate in the declared order whose box is clear: `dy` 0,
    /// one column along `-z`, which the four diagonals beside it are blocked out
    /// of. Its box occupies the two rows of column `(8, 7)`, both empty.
    const ONE_ALONG_MINUS_Z: Vec3 = Vec3::new(8.5, ON_THE_FLOOR, 7.5);

    let (mut client, declared) = a_client_over(&content_root()?, standing_at(TRAPPED), a_pocket)?;
    require_the_reload_traps(&declared, TRAPPED)?;
    require_a_clear_position_at(&declared, ONE_ALONG_MINUS_Z)?;

    let said = clearance_of(client.adopt(candidate(water_declared_solid()?.path())?));
    let after = standing_of(client.tick());

    assert_eq!(
        (said, after),
        (moved_to(ONE_ALONG_MINUS_Z), at(ONE_ALONG_MINUS_Z)),
        "the author has made `base:water` solid and the player was standing in two cells of it, so \
         the box they had is now inside solid rock. The guards above are the scenario's own \
         premise, asserted rather than described: that box overlapped nothing solid before the \
         reload and overlaps something after it, and the cell named here is clear. **The verdict \
         and the position are both compared because either alone is satisfiable on its own** — a \
         search that reported a move it never made, and a swap that moved the player without \
         saying so"
    );
    Ok(())
}

#[test]
fn a_reload_moves_the_player_sideways_where_sideways_and_upward_are_both_clear() -> TestResult {
    /// The first candidate in the declared order whose box is clear: `dy` 0, one
    /// column along `-x`, the three candidates the tie-break puts before it being
    /// blocked.
    const ONE_ALONG_MINUS_X: Vec3 = Vec3::new(7.5, ON_THE_FLOOR, 8.5);
    /// One block straight up, which is clear as well — and which the search must
    /// not take, because `dy` is the first key of the order and every one of the
    /// 289 positions at `dy` 0 is tried before any of them.
    const ONE_BLOCK_UP: Vec3 = Vec3::new(8.5, ON_THE_FLOOR + 1.0, 8.5);

    let (mut client, declared) = a_client_over(&content_root()?, standing_at(TRAPPED), a_corridor)?;
    require_the_reload_traps(&declared, TRAPPED)?;
    require_a_clear_position_at(&declared, ONE_ALONG_MINUS_X)?;
    require_a_clear_position_at(&declared, ONE_BLOCK_UP)?;

    let said = clearance_of(client.adopt(candidate(water_declared_solid()?.path())?));
    let after = standing_of(client.tick());

    assert_eq!(
        (said, after),
        (moved_to(ONE_ALONG_MINUS_X), at(ONE_ALONG_MINUS_X)),
        "both a clear cell one column sideways and a clear cell one block up are available, and the \
         two guards above are what say so rather than a comment. Sideways is the answer, and it is \
         the answer because `dy` is ranked before horizontal distance: a search that took the \
         upward cell would leave the player at {ONE_BLOCK_UP:?}, standing on the very block that \
         had just trapped them"
    );
    Ok(())
}

#[test]
fn a_reload_moves_the_player_sideways_where_the_nearest_clear_cell_is_below_them() -> TestResult {
    /// One block below, which is clear — and which is not a candidate at all: the
    /// cube is `dy ∈ [0, 8]`, so downward is absent from the candidate set rather
    /// than ranked last, and no reordering can reach it.
    const ONE_BLOCK_BELOW: Vec3 = Vec3::new(8.5, ON_THE_FLOOR - 1.0, 8.5);
    /// The first candidate in the declared order whose box is clear: two columns
    /// along `-z`, everything nearer being blocked.
    const TWO_ALONG_MINUS_Z: Vec3 = Vec3::new(8.5, ON_THE_FLOOR, 6.5);

    let (mut client, declared) = a_client_over(&content_root()?, airborne_over_a_hole(), a_shaft)?;
    require_the_reload_traps(&declared, TRAPPED)?;
    require_a_clear_position_at(&declared, ONE_BLOCK_BELOW)?;
    require_a_clear_position_at(&declared, TWO_ALONG_MINUS_Z)?;

    let said = clearance_of(client.adopt(candidate(water_declared_solid()?.path())?));
    let after = standing_of(client.tick());

    assert_eq!(
        (said, after),
        (moved_to(TWO_ALONG_MINUS_Z), at(TWO_ALONG_MINUS_Z)),
        "the floor is missing under the player, so the nearest clear cell of all is the one block \
         below them at {ONE_BLOCK_BELOW:?} — one away against the two the sideways cell costs. It \
         is not taken, and it is not taken because it is not in the candidate set: a search that \
         ranked downward last rather than leaving it out would still pass a scenario in which \
         nothing below was clear, which is why this one makes it clear and nearest"
    );
    Ok(())
}

#[test]
fn a_reload_that_moves_a_rising_player_upward_takes_their_climb_away() -> TestResult {
    /// The first candidate in the declared order whose box is clear: one block
    /// straight up. Every one of the 289 positions at `dy` 0 has its feet row
    /// filled, so the search reaches `dy` 1 with nothing horizontal left to try.
    const ONE_BLOCK_UP: Vec3 = Vec3::new(12.5, ON_THE_FLOOR + 1.0, 12.5);

    let spawn = rising_at(WEDGED, RISING);
    let (mut client, declared) = a_client_over(&content_root()?, spawn, a_ceilingless_plane)?;
    require_rising(&spawn)?;
    require_the_reload_traps(&declared, WEDGED)?;
    require_a_clear_position_at(&declared, ONE_BLOCK_UP)?;

    let said = clearance_of(client.adopt(candidate(water_declared_solid()?.path())?));
    let after = standing_of(client.tick());

    assert_eq!(
        (said, after),
        (moved_to(ONE_BLOCK_UP), at(ONE_BLOCK_UP)),
        "a cleared player has been teleported, so the climb they were in the middle of is not \
         theirs to finish. The velocity in this comparison is the whole of the scenario: with the \
         rise left in place the next tick carries them {RISING} blocks per second further up, and \
         both the velocity read back and the height it moved them to would differ"
    );
    Ok(())
}

/// A copy of the shipped root whose `water.luau` declares `base:water` solid.
///
/// The one edit every scenario here is driven with. Nothing else about the root
/// changes, so the candidate is admitted: it still declares every block these
/// worlds hold and still declares a solid block for a player to place.
fn water_declared_solid() -> Result<ContentRoot, Box<dyn Error>> {
    restating(shipped()?, WATER_FILE, &water_that_is_solid())
}

/// A player over the column whose floor is missing, at rest and about to fall.
fn airborne_over_a_hole() -> PlayerState {
    airborne_at(TRAPPED)
}

/// One column of grass floor, with `cells` written above it.
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

/// The player's own two cells filled with water, and the four diagonals beside them
/// filled too.
///
/// So the search's first candidate is their own cell — blocked — and the four
/// diagonal neighbours at distance one are blocked as well, which leaves the first
/// clear one at `(dz, dx) = (-1, 0)`: one column along `-z`. Blocking the diagonals
/// is what makes the answer the same under a Chebyshev reach and a Euclidean one.
fn a_pocket(registry: &BlockRegistry) -> Result<VoxelWorld, Box<dyn Error>> {
    a_floor_holding(
        registry,
        &[
            (OWN_CELL, WATER),
            ((8, HEAD_ROW, 8), WATER),
            ((7, FEET_ROW, 7), WATER),
            ((9, FEET_ROW, 7), WATER),
            ((7, FEET_ROW, 9), WATER),
            ((9, FEET_ROW, 9), WATER),
        ],
    )
}

/// The player's feet cell filled, and the three candidates the tie-break ranks
/// before `-x` filled too, with the cell over their head left empty.
///
/// The head cell being empty is what makes one block up a clear position, which is
/// the half of the scenario that has to be available for the sideways answer to
/// mean anything.
fn a_corridor(registry: &BlockRegistry) -> Result<VoxelWorld, Box<dyn Error>> {
    a_floor_holding(
        registry,
        &[
            (OWN_CELL, WATER),
            ((7, FEET_ROW, 7), WATER),
            ((8, FEET_ROW, 7), WATER),
            ((9, FEET_ROW, 7), WATER),
        ],
    )
}

/// The cell over the player's head filled, every neighbouring column at distance
/// one filled, and the two candidates the tie-break ranks before `-z` at distance
/// two filled — with the floor under the player missing.
///
/// Their own feet cell is left empty, which is what makes the cell one block below
/// them clear and therefore the nearest clear position of all.
fn a_shaft(registry: &BlockRegistry) -> Result<VoxelWorld, Box<dyn Error>> {
    let mut cells = vec![((8, HEAD_ROW, 8), WATER)];
    cells.extend(each_holding(&ring_around_the_player(), WATER));
    cells.extend(each_holding(&[(6, FEET_ROW, 6), (7, FEET_ROW, 6)], WATER));
    a_world(
        registry,
        &Shape {
            columns: ONE_COLUMN,
            floor: Some(GRASS),
            open: &[(8, 8)],
            cells: &cells,
        },
    )
}

/// The eight cells at the player's own feet row in the columns around theirs.
fn ring_around_the_player() -> Vec<Cell> {
    (7..=9)
        .flat_map(|z| (7..=9).map(move |x| (x, FEET_ROW, z)))
        .filter(|cell| *cell != OWN_CELL)
        .collect()
}

/// Two columns square, no floor at all, and water filling the whole of the feet row
/// the search can reach horizontally.
///
/// Every one of the 289 positions at `dy` 0 therefore has its feet row solid once
/// water is, which is what leaves the first clear candidate one block up. The row
/// above is empty, so that cell's box is clear, and the water row under it is what
/// holds the player there for the tick that follows.
fn a_ceilingless_plane(registry: &BlockRegistry) -> Result<VoxelWorld, Box<dyn Error>> {
    let filled = within_the_search((12, FEET_ROW, 12), &[FEET_ROW]);
    a_world(
        registry,
        &Shape {
            columns: TWO_COLUMNS,
            floor: None,
            open: &[],
            cells: &each_holding(&filled, WATER),
        },
    )
}
