//! A player the reload traps near the world's edge is left where they are and told —
//! never put somewhere the world does not exist.
//!
//! # The defect, and it is reachable by playing
//!
//! `Solidity::is_solid` answers `false` for every cell past the footprint, so the
//! clearing search reads *outside the loaded world* as clear ground. The shipped world
//! is 64 blocks square and the search reaches 8, so **any player trapped within eight
//! blocks of an edge has candidates outside the world** — and in a wedge those are the
//! nearest "clear" ones the ring order meets. The player is then put where nothing is
//! solid and falls out of the world. Walk to an edge, save a `solid` change, and it
//! happens.
//!
//! **Eligibility is what this file is about.** A candidate is eligible only if every
//! cell the player's box would cover is *known and clear*. Outside is **unknown, not
//! clear**, and a search over unknown ground is not a search. Treating outside as
//! solid was refused: it is a lie in a model that collision, meshing and physics all
//! read, and it inverts the moment the world streams.
//!
//! # Two directions, and the second is mandatory
//!
//! The scenario here asserts a **refusal**, so it is vacuously satisfied by a search
//! that
//! finds nothing ever — deleting the candidate generator outright would leave it
//! green. The paired control is therefore in its own test function below: the same
//! wedge, in a world wide enough to hold an eligible candidate, and a move must
//! result.
//!
//! # What is asserted about the destination, and why "moved" would not do
//!
//! Today's behaviour *is* a move, so an assertion that the player moved is satisfied
//! by the defect and an assertion that they did not is satisfied by a broken search.
//! What is compared is an enumerated verdict over **where** they ended up:
//! [`Cleared::MovedOffTheMap`] and [`Cleared::MovedInsideTheWorld`] are different
//! answers, and the failure text says which one arrived. The classification is done by
//! decoding the destination the client reported and asking whether it lies inside the
//! footprint — an oracle that shares no code with the search.
//!
//! # No new vocabulary
//!
//! A boundary wedge with no eligible candidate takes the same refusal path a wedge with
//! nowhere to go anywhere else takes — the reload stands, the player stays, and the
//! verdict says how far was looked. `tests/reload_reports_nowhere_to_clear_to.rs` grades
//! that path, and nothing here asks for a fourth arm.
//!
//! # Driven through the reload, not against the search's signature
//!
//! Every reading is the client's own report, as phase 5's clearing scenarios are. The
//! world's extent has to reach the search for this to be fixable at all, and how it
//! gets there is the implementer's to choose — a test written against `cleared`'s
//! arguments would fail to compile until they picked, and a compile error is not a RED
//! for a behaviour scenario.

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
use reload_clearing::{Clearance, Feet, at, feet_at, standing_of, until_cleared};
use reload_trap::{
    A_SEARCH_OF, FEET_ROW, ON_THE_FLOOR, Overlap, SOLID_ONCE_WATER_IS, Shape, a_client_watching,
    a_world, each_holding, overlap_at, require, require_a_clear_position_at,
    require_the_reload_traps, water_that_is_solid, within_the_search,
};
use reload_watch::{Reports, block_path, restating_raw, solidity_of};
use reload_world::{ACROSS, Cell, standing_at};
use support::TestResult;
use support::content::ContentRoot;

/// How many chunk columns square each world is.
///
/// One column is sixteen blocks across, which is **narrower than the search is wide**:
/// a player standing two blocks from a corner has most of the cube outside the world,
/// which is the premise of the first scenario. Two columns is thirty-two and holds the
/// whole cube around the position the control uses, which is the premise of the
/// second — the same rule `tests/reload_reports_nowhere_to_clear_to.rs`'s fixture
/// follows.
const ONE_COLUMN: u32 = 1;
const TWO_COLUMNS: u32 = 2;

/// Where the trapped player stands: two blocks in from a corner, feet on the floor's
/// top face.
///
/// Two rather than eight, so the cube leaves the world on both horizontal axes and the
/// nearest positions the ring order reaches are outside it.
const NEAR_THE_CORNER: Vec3 = Vec3::new(2.5, ON_THE_FLOOR, 2.5);

/// The cell their feet are in, which the filling is centred on.
const NEAR_THE_CORNER_CELL: Cell = (2, FEET_ROW, 2);

/// Where the far-edge player stands: three blocks in from the far corner of a
/// one-column world.
///
/// **The whole cube is at non-negative coordinates and part of it is past the far
/// edge**, which is the one thing this position is chosen for. The reach is eight, so
/// the cube spans `[5, 21]` on both horizontal axes against an edge at sixteen: nothing
/// in it is negative, and everything outside the world is *positive*. A sign check
/// alone therefore says every candidate is fine, and only an extent can refuse them.
const NEAR_THE_FAR_EDGE: Vec3 = Vec3::new(13.5, ON_THE_FLOOR, 13.5);

/// The cell their feet are in.
const NEAR_THE_FAR_EDGE_CELL: Cell = (13, FEET_ROW, 13);

/// Where the control's player stands: the centre of the column at `(12, 12)` in a
/// two-column world, which leaves the whole cube inside it.
const WELL_INSIDE: Vec3 = Vec3::new(12.5, ON_THE_FLOOR, 12.5);

/// The cell their feet are in.
const WELL_INSIDE_CELL: Cell = (12, FEET_ROW, 12);

/// The one position the control leaves clear: four blocks along `+x`, inside the world.
///
/// Four rather than one, so it is not the first place the search looks — the ring order
/// has to reach it past three rings of blocked ground, which is what makes the control
/// about a search that works rather than about a lucky neighbour.
const THE_ONE_WAY_OUT: Vec3 = Vec3::new(16.5, ON_THE_FLOOR, 12.5);

/// The two cells a player standing there occupies, which the filling leaves empty.
const THE_WAY_OUT_CELLS: [Cell; 2] = [(16, FEET_ROW, 12), (16, FEET_ROW + 1, 12)];

/// Where the player ended up, judged against the world that was loaded.
///
/// **A total verdict, and the two `Moved` arms are the whole point.** Today's defect is
/// a *move*, so a verdict that only said whether one happened would be satisfied by it;
/// what separates the defect from the fix is whether the destination is somewhere the
/// world exists.
#[derive(Debug, PartialEq, Eq)]
enum Cleared {
    /// The player was left where they were and told nothing inside this many blocks was
    /// clear.
    ToldNothingWasClear { blocks: u32 },
    /// The player was moved, and every cell their box covers is inside the world.
    MovedInsideTheWorld(Feet),
    /// The player was moved off the map — the destination is outside the footprint,
    /// where nothing is solid because nothing is loaded.
    MovedOffTheMap(Feet),
    /// Anything else the client said about clearing them.
    Otherwise(Clearance),
}

#[test]
fn a_reload_that_traps_a_player_at_the_world_edge_leaves_them_rather_than_putting_them_outside()
-> TestResult {
    let root = shipped()?;
    let (mut client, reports) = a_client_wedged_near_the_corner(&root)?;
    let root = restating_raw(root, WATER_FILE, &water_that_is_solid().text())?;

    reports.changed(&[block_path(&root, WATER_FILE)])?;
    let said = until_cleared(&mut client);
    let after = standing_of(client.tick());
    let water_now = solidity_of(&client, WATER)?;

    assert_eq!(
        (cleared_within(&said, ONE_COLUMN), after, water_now),
        (
            Cleared::ToldNothingWasClear {
                blocks: A_SEARCH_OF
            },
            at(NEAR_THE_CORNER),
            Some(true)
        ),
        "the player is two blocks from a corner and the search reaches eight, so most of the cube \
         it may look at is outside the loaded world — where `is_solid` answers false because \
         nothing is there, not because it is clear. A candidate is eligible only if every cell the \
         box would cover is *known* and clear, so a wedge at the edge has nowhere to go and takes \
         the same refusal path a wedge in the middle of a lake takes: the reload stands, the player \
         stays, and a person is told. Putting them outside instead drops them out of the world, \
         and it is reachable by walking to an edge and saving a solidity change"
    );
    Ok(())
}

#[test]
fn a_reload_that_traps_a_player_at_the_far_edge_refuses_the_ground_past_it_too() -> TestResult {
    let root = shipped()?;
    let (mut client, reports) = a_client_wedged_near_the_far_edge(&root)?;
    let root = restating_raw(root, WATER_FILE, &water_that_is_solid().text())?;

    reports.changed(&[block_path(&root, WATER_FILE)])?;
    let said = until_cleared(&mut client);
    let after = standing_of(client.tick());
    let water_now = solidity_of(&client, WATER)?;

    assert_eq!(
        (cleared_within(&said, ONE_COLUMN), after, water_now),
        (
            Cleared::ToldNothingWasClear {
                blocks: A_SEARCH_OF
            },
            at(NEAR_THE_FAR_EDGE),
            Some(true)
        ),
        "the same wedge at the other end of the world, and it is a different test: this player's          whole candidate cube is at non-negative coordinates, so the sign check that refuses          `x = -1` says yes to every one of them and only the extent can refuse `x = 16`. **The          scenario beside this one is carried entirely by that sign check** — its cube spans          `[-6, 10]` against an edge at sixteen and never reaches the far side — so with this test          absent, an extent that answered `true` for everything would be proved wrong by nothing,          and a player near the far edge of the shipped 64-block world would be teleported off it          exactly as the near-edge player was"
    );
    Ok(())
}

/// The paired positive control, and it is mandatory rather than thorough.
///
/// The scenario above asserts a refusal, so **a search that never found anything would
/// satisfy it**: deleting the candidate generator, or refusing every candidate outright,
/// leaves it green. This is the same wedge in a world wide enough to hold an eligible
/// candidate, and it requires the move. It is green today and has to stay green — a fix
/// that made outside ineligible by making *everything* ineligible fails here.
#[test]
fn the_same_wedge_in_a_world_wide_enough_moves_the_player_to_a_position_inside_it() -> TestResult {
    let root = shipped()?;
    let (mut client, reports) = a_client_wedged_well_inside(&root)?;
    let root = restating_raw(root, WATER_FILE, &water_that_is_solid().text())?;

    reports.changed(&[block_path(&root, WATER_FILE)])?;
    let said = until_cleared(&mut client);
    let water_now = solidity_of(&client, WATER)?;

    assert_eq!(
        (cleared_within(&said, TWO_COLUMNS), water_now),
        (
            Cleared::MovedInsideTheWorld(feet_at(THE_ONE_WAY_OUT)),
            Some(true)
        ),
        "the wedge is the same and the world is wider, so the one position left clear is inside it \
         and four blocks away — past three rings of blocked ground, which is what makes this a \
         search rather than a lucky neighbour. Without this, the scenario beside it is satisfied by \
         a search that finds nothing ever, and the repair for putting a player off the map would be \
         to stop clearing anybody"
    );
    Ok(())
}

/// A client trapped three blocks from the **far** corner of a one-column world,
/// watching the root it plays.
///
/// # Errors
///
/// Returns an error if the root does not read, if the world does not build, or if any
/// premise fails.
fn a_client_wedged_near_the_far_edge(
    root: &ContentRoot,
) -> Result<(InputHarness, Reports), Box<dyn Error>> {
    let filled = each_holding(
        &inside_the_world(&the_whole_cube_around(NEAR_THE_FAR_EDGE_CELL)),
        WATER,
    );
    let (client, declared, reports) =
        a_client_watching(root, standing_at(NEAR_THE_FAR_EDGE), |registry| {
            a_wedge(registry, ONE_COLUMN, &filled)
        })?;
    require_the_reload_traps(&declared, NEAR_THE_FAR_EDGE)?;
    require_every_candidate_outside_is_past_the_far_edge(NEAR_THE_FAR_EDGE_CELL, ONE_COLUMN)?;
    require_nothing_inside_the_world_is_clear(&declared, NEAR_THE_FAR_EDGE, ONE_COLUMN)?;
    Ok((client, reports))
}

/// A client trapped two blocks from a corner of a one-column world, watching the root
/// it plays.
///
/// **Three premises, and the second is this scenario's own.** The reload has to be what
/// traps the player; some position the search may look at has to lie outside the world,
/// or the fixture is about a wedge rather than about a boundary; and every position
/// inside the world has to be blocked, or the search would rightly find one.
///
/// # Errors
///
/// Returns an error if the root does not read, if the world does not build, or if any
/// premise fails.
fn a_client_wedged_near_the_corner(
    root: &ContentRoot,
) -> Result<(InputHarness, Reports), Box<dyn Error>> {
    let filled = each_holding(
        &inside_the_world(&the_whole_cube_around(NEAR_THE_CORNER_CELL)),
        WATER,
    );
    let (client, declared, reports) =
        a_client_watching(root, standing_at(NEAR_THE_CORNER), |registry| {
            a_wedge(registry, ONE_COLUMN, &filled)
        })?;
    require_the_reload_traps(&declared, NEAR_THE_CORNER)?;
    require_some_candidate_lies_outside(NEAR_THE_CORNER_CELL, ONE_COLUMN)?;
    require_nothing_inside_the_world_is_clear(&declared, NEAR_THE_CORNER, ONE_COLUMN)?;
    Ok((client, reports))
}

/// A client trapped in the middle of a two-column world with one way out, watching the
/// root it plays.
///
/// # Errors
///
/// Returns an error if the root does not read, if the world does not build, or if
/// either premise fails.
fn a_client_wedged_well_inside(
    root: &ContentRoot,
) -> Result<(InputHarness, Reports), Box<dyn Error>> {
    let cube = the_whole_cube_around(WELL_INSIDE_CELL);
    let filled = each_holding(&without_the_way_out(&cube), WATER);
    let (client, declared, reports) =
        a_client_watching(root, standing_at(WELL_INSIDE), |registry| {
            a_wedge(registry, TWO_COLUMNS, &filled)
        })?;
    require_the_reload_traps(&declared, WELL_INSIDE)?;
    require_a_clear_position_at(&declared, THE_ONE_WAY_OUT)?;
    Ok((client, reports))
}

/// Every cell the search can put a player's feet in around `centre`: their own row and
/// the eight above it, over the whole horizontal reach.
///
/// Derived from [`A_SEARCH_OF`] on both axes, so a fixture follows the declared bound
/// rather than a number of its own.
fn the_whole_cube_around(centre: Cell) -> Vec<Cell> {
    let rows: Vec<i32> = (FEET_ROW..=FEET_ROW + A_SEARCH_OF as i32).collect();
    within_the_search(centre, &rows)
}

/// The cells of `cube` that lie inside a one-column world.
///
/// **The filter is what makes the fixture buildable at all** — a write past an edge is
/// refused, and rightly — and it is also the shape of the defect: the cells it drops are
/// exactly the ones the search reads as clear because nothing is loaded there.
fn inside_the_world(cube: &[Cell]) -> Vec<Cell> {
    let across = i32::try_from(ONE_COLUMN * ACROSS).unwrap_or(i32::MAX);
    cube.iter()
        .copied()
        .filter(|(x, _, z)| (0..across).contains(x) && (0..across).contains(z))
        .collect()
}

/// `cube` with the two cells a player standing at the one way out would occupy left
/// empty.
fn without_the_way_out(cube: &[Cell]) -> Vec<Cell> {
    cube.iter()
        .copied()
        .filter(|cell| !THE_WAY_OUT_CELLS.contains(cell))
        .collect()
}

/// `columns` square, a grass floor to stand on, and `filled` written above it.
///
/// The floor matters: with it, the tick after the swap resolves the player back onto the
/// same face and the reading is where they were left rather than where they were left
/// plus a fall.
fn a_wedge(
    registry: &BlockRegistry,
    columns: u32,
    filled: &[(Cell, &str)],
) -> Result<VoxelWorld, Box<dyn Error>> {
    a_world(
        registry,
        &Shape {
            columns,
            floor: Some(GRASS),
            open: &[],
            cells: filled,
        },
    )
}

/// What the client said about clearing the player, judged against a world `columns`
/// square.
fn cleared_within(said: &Clearance, columns: u32) -> Cleared {
    match said {
        Clearance::NoClearSpaceWithin { blocks } => {
            Cleared::ToldNothingWasClear { blocks: *blocks }
        }
        Clearance::MovedTo(feet) => {
            if lies_inside(*feet, columns) {
                Cleared::MovedInsideTheWorld(*feet)
            } else {
                Cleared::MovedOffTheMap(*feet)
            }
        }
        other => Cleared::Otherwise(other.clone()),
    }
}

/// Whether a destination lies inside a world `columns` square.
///
/// **Decoded from what the client reported rather than matched against a set this test
/// built**, so the classification shares no code with the search that produced it.
///
/// Only the two horizontal axes are asked about. Every destination the search takes is a
/// cell centre, so the 0.6-wide box spans `[c + 0.2, c + 0.8]` and lies inside the one
/// column the centre is in — bounding the centre is therefore exact rather than
/// approximate. Vertically the search reaches eight rows above a floor at ten, which is
/// well inside a column's own height, so a destination cannot leave the world upward.
fn lies_inside(feet: Feet, columns: u32) -> bool {
    let across = (columns * ACROSS) as f32;
    let [x, _, z] = feet.map(f32::from_bits);
    (0.0..across).contains(&x) && (0.0..across).contains(&z)
}

/// Refuses unless some position the search may look at lies outside the world.
///
/// **This scenario's own premise.** A fixture whose whole cube is inside the footprint
/// is about a wedge, not about a boundary, and the refusal it asserted would be the one
/// `tests/reload_reports_nowhere_to_clear_to.rs` already grades.
///
/// # Errors
///
/// Returns an error saying how many of the cube's positions were inside.
fn require_some_candidate_lies_outside(centre: Cell, columns: u32) -> Result<(), Box<dyn Error>> {
    let cube = the_whole_cube_around(centre);
    let across = i32::try_from(columns * ACROSS)?;
    let outside = cube
        .iter()
        .filter(|(x, _, z)| !((0..across).contains(x) && (0..across).contains(z)))
        .count();
    require(
        outside > 0,
        format!(
            "this scenario needs a player close enough to an edge that the search can look outside \
             the world, and all {inside} positions of the cube around {centre:?} are inside a world \
             {across} blocks square",
            inside = cube.len()
        ),
    )
}

/// Refuses unless the cube around `centre` leaves the world **only** past its far edge.
///
/// **The premise that makes the far-edge scenario a different test from the near-corner
/// one**, and without it the two are one test written twice. `holds` refuses a candidate
/// in two steps — a negative coordinate names nothing the world holds, and then the
/// extent is asked — so a fixture whose out-of-world candidates are all negative is
/// carried entirely by the first step and proves nothing about the second. This requires
/// the opposite: something outside, and nothing negative.
///
/// # Errors
///
/// Returns an error saying which half failed and with how many positions.
fn require_every_candidate_outside_is_past_the_far_edge(
    centre: Cell,
    columns: u32,
) -> Result<(), Box<dyn Error>> {
    let cube = the_whole_cube_around(centre);
    let across = i32::try_from(columns * ACROSS)?;
    let negative = cube.iter().filter(|(x, _, z)| *x < 0 || *z < 0).count();
    let past_the_edge = cube
        .iter()
        .filter(|(x, _, z)| *x >= across || *z >= across)
        .count();
    require(
        negative == 0,
        format!(
            "this scenario needs every out-of-world position to be past the far edge, and {negative}              of the cube around {centre:?} are at negative coordinates — where the sign check              refuses them before the extent is ever asked"
        ),
    )?;
    require(
        past_the_edge > 0,
        format!(
            "this scenario needs the cube around {centre:?} to reach past the far edge of a world              {across} blocks square, and none of its {count} positions does",
            count = cube.len()
        ),
    )
}

/// Refuses unless every position inside the world that the search may look at is blocked
/// once water is solid.
///
/// The other half of the premise: with an eligible position left anywhere inside the
/// footprint, the search would rightly move the player there and the scenario would be
/// about nothing.
///
/// # Errors
///
/// Returns an error naming how many were clear and where the first of them was.
fn require_nothing_inside_the_world_is_clear(
    blocks: &VoxelWorld,
    feet: Vec3,
    columns: u32,
) -> Result<(), Box<dyn Error>> {
    let clear: Vec<Feet> = the_whole_cube_around(cell_of(feet))
        .into_iter()
        .map(|(x, y, z)| Vec3::new(x as f32 + 0.5, y as f32, z as f32 + 0.5))
        .filter(|position| lies_inside(feet_at(*position), columns))
        .filter(|position| overlap_at(blocks, &SOLID_ONCE_WATER_IS, *position) == Overlap::Clear)
        .map(feet_at)
        .collect();
    require(
        clear.is_empty(),
        format!(
            "this scenario needs every position inside the world that the search may look at to be \
             blocked, and {count} of them are clear — the first at {first:?}",
            count = clear.len(),
            first = clear.first()
        ),
    )
}

/// The cell a player's feet at `feet` are in.
fn cell_of(feet: Vec3) -> Cell {
    (
        feet.x.floor() as i32,
        feet.y.floor() as i32,
        feet.z.floor() as i32,
    )
}
