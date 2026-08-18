//! A player a save records inside solid rock near the world's edge is left where
//! they are, or moved inward — never put where the world does not exist.
//!
//! # The extent has to reach the search, and this is the only fixture that says so
//!
//! `Solidity::is_solid` answers `false` for every cell past the loaded footprint,
//! because nothing is there and not because it is clear. The search reaches eight
//! blocks, so a player trapped within eight of an edge has candidates outside the
//! world — and in a wedge those are among the nearest ones the ring order meets.
//! What refuses them is the played world's own extent, and the entry caller is
//! what has to pass it. **A saved position near an edge is an input no reload
//! fixture supplies**, which is why this scenario exists here rather than being
//! inherited.
//!
//! # Two directions, and the second is mandatory
//!
//! The scenario asserts a **refusal** — the player stays put — so it is vacuously
//! satisfied by a search that finds nothing ever, and by an extent so small that
//! nothing is eligible. The control below is the same wedge with one position left
//! clear inside the world, and it requires the move. An extent that is too large
//! fails the first and passes the second; an extent that is too small passes the
//! first and fails the second; only the right one passes both.
//!
//! # Everything outside this world is at a *positive* coordinate
//!
//! The eligibility check refuses a candidate in two steps: a negative coordinate
//! names nothing any world holds, and only then is the extent asked. A fixture
//! whose out-of-world candidates were all negative would be carried entirely by
//! the first step and would prove nothing about the second. The player here stands
//! three blocks in from the far corner of a one-column world, so the cube spans
//! `[5, 21]` against an edge at sixteen: nothing in it is negative, and everything
//! outside the world is past the far edge.

#[path = "support/entry.rs"]
mod entry;
#[path = "support/persistence.rs"]
mod persistence;

use std::error::Error;
use std::sync::Arc;

use glam::Vec3;
use mc_world::section::Contents;
use mc_world::world::{VoxelWorld, WorldPos};

use entry::{
    A_SEARCH_OF, ACROSS, ASave, FEET_ROW, NO_ACCEPTANCE, at, filling, floor_of, ground_registry,
    inside_a_world, recorded_at, require, resumed, the_cube_around, without, written,
};
use persistence::{GROUND, TestResult, refusal, stood_at};

/// How many chunk columns square this world is.
///
/// One column is sixteen blocks across, which is narrower than the search is
/// wide: that is the whole premise.
const ONE_COLUMN: u32 = 1;

/// Where the save records the player: three blocks in from the far corner, feet a
/// quarter of a block above the row they are in.
const NEAR_THE_FAR_EDGE: Vec3 = Vec3::new(13.5, FEET_ROW as f32 + 0.25, 13.5);

/// The two cells the control leaves clear: four blocks along −x, inside the
/// world.
///
/// **Four rather than one, and that is what makes the control about the extent.**
/// The rings at one, two and three lie inside this world on both horizontal axes
/// except at three, where the ring reaches past the far edge — so a search reaching
/// this position has already had to refuse candidates the world does not hold, and
/// an extent covering the whole coordinate space would have stopped at one of
/// them.
const THE_WAY_OUT_CELLS: [(u32, u32, u32); 2] = [(9, FEET_ROW, 13), (9, FEET_ROW + 1, 13)];

/// Which ring the way out sits in, counted the way the search counts distance
/// sideways: the larger of the two horizontal offsets.
const THE_WAY_OUTS_RING: i32 = 4;

#[test]
fn a_resumed_player_whose_only_clear_position_lies_partly_outside_the_world_starts_where_the_save_recorded()
-> TestResult {
    let save = a_wedge_at_the_edge(&[])?;

    let launched = resumed(&save, &NO_ACCEPTANCE)?;

    assert_eq!(
        stood_at(&launched),
        Ok(at(NEAR_THE_FAR_EDGE)),
        "every position inside this world that the search may look at is blocked, and the ones \
         that are clear are outside it — where nothing is solid because nothing is loaded. A \
         candidate is a place to put somebody only if the world holds every cell their box would \
         cover, so this player has nowhere to go and stays exactly where the save recorded them, \
         at {NEAR_THE_FAR_EDGE:?}, and is told so. Putting them at the clear-looking position \
         instead drops them out of the world on the next tick, and it is reachable by saving next \
         to an edge and editing a block's solidity. The launch answered: {}",
        refusal(&launched)
    );
    Ok(())
}

/// The paired positive control, and it is mandatory rather than thorough.
///
/// This is a *move* the extent constrained, where the scenario above is a refusal
/// the extent caused, and an extent that is wrong in either direction fails
/// exactly one of them. Without it, the repair for putting a player off the map
/// would be to stop clearing anybody.
#[test]
fn a_resumed_player_near_the_worlds_edge_is_moved_inward_rather_than_over_it() -> TestResult {
    let save = a_wedge_at_the_edge(&THE_WAY_OUT_CELLS)?;

    let launched = resumed(&save, &NO_ACCEPTANCE)?;

    assert_eq!(
        stood_at(&launched),
        Ok(at(centred_on(THE_WAY_OUT_CELLS[0]))),
        "the same wedge with one position left clear inside the world, four blocks inward. The \
         search has to walk past the ring that reaches over the far edge — every candidate there \
         looks clear and none of them is ground this world holds — and come to rest at {:?}, \
         which is inward. A search told it may consider the whole coordinate space stops at the \
         first of those and puts the player outside; one told it may consider nothing finds this \
         position no more than it finds the others. The launch answered: {}",
        centred_on(THE_WAY_OUT_CELLS[0]),
        refusal(&launched)
    );
    Ok(())
}

/// Where a player standing at the centre of `cell` has their feet: horizontally
/// centred, feet on the cell's floor.
fn centred_on(cell: (u32, u32, u32)) -> Vec3 {
    let (x, y, z) = cell;
    Vec3::new(x as f32 + 0.5, y as f32, z as f32 + 0.5)
}

/// A save recording a player wedged near the far edge, with every position inside
/// the world blocked except `left_clear`.
///
/// # Errors
///
/// Returns an error if the world cannot be built or written, or if any premise
/// fails.
fn a_wedge_at_the_edge(left_clear: &[(u32, u32, u32)]) -> Result<ASave, Box<dyn Error>> {
    let registry = ground_registry()?;
    let mut blocks = floor_of(&registry, ONE_COLUMN, GROUND)?;
    let cube = the_cube_around(NEAR_THE_FAR_EDGE);
    let inside = inside_a_world(&cube, ONE_COLUMN);
    filling(
        &mut blocks,
        &registry,
        &without(&inside, left_clear),
        GROUND,
    )?;

    require_the_save_traps_the_player(&blocks)?;
    require_every_candidate_outside_is_past_the_far_edge(&cube)?;
    require_a_nearer_candidate_lies_outside(&cube)?;
    require_nothing_inside_the_world_is_clear_but(&blocks, &inside, left_clear)?;
    written(
        blocks,
        &registry,
        Arc::clone(&registry),
        recorded_at(NEAR_THE_FAR_EDGE, 0.0, 0.0),
    )
}

/// Refuses unless the box at the recorded position covers something solid.
fn require_the_save_traps_the_player(blocks: &VoxelWorld) -> Result<(), Box<dyn Error>> {
    let covered = entry::cells_a_box_covers(NEAR_THE_FAR_EDGE);
    require(
        covered.iter().any(|cell| holds(blocks, *cell)),
        format!(
            "this scenario needs the recorded position to be inside solid rock, and none of the \
             cells the box at {NEAR_THE_FAR_EDGE:?} covers holds a block"
        ),
    )
}

/// Refuses unless the cube leaves the world **only** past its far edge.
///
/// Without this the fixture is carried by the sign check that refuses a negative
/// coordinate before the extent is ever asked, and it proves nothing about the
/// extent at all.
fn require_every_candidate_outside_is_past_the_far_edge(
    cube: &[(i32, i32, i32)],
) -> Result<(), Box<dyn Error>> {
    let across = i32::try_from(ONE_COLUMN * ACROSS)?;
    let negative = cube.iter().filter(|(x, _, z)| *x < 0 || *z < 0).count();
    let past_the_edge = cube
        .iter()
        .filter(|(x, _, z)| *x >= across || *z >= across)
        .count();
    require(
        negative == 0,
        format!(
            "this scenario needs every out-of-world position to be past the far edge, and \
             {negative} of the cube around {NEAR_THE_FAR_EDGE:?} are at negative coordinates — \
             where a sign check refuses them before the extent is ever asked"
        ),
    )?;
    require(
        past_the_edge > 0,
        format!(
            "this scenario needs the cube around {NEAR_THE_FAR_EDGE:?} to reach past the far edge \
             of a world {across} blocks square, and none of its {count} positions does — which \
             would make it a wedge in open ground rather than one at a boundary",
            count = cube.len()
        ),
    )
}

/// Refuses unless some position the search reaches **before** the way out lies
/// outside the world.
///
/// The control's own premise. The search walks upward last and sideways by rings,
/// so everything at the player's own height in a nearer ring is looked at first;
/// if none of those left the world, the control would be about a search that
/// simply found the nearest clear cell and would say nothing about the extent.
fn require_a_nearer_candidate_lies_outside(cube: &[(i32, i32, i32)]) -> Result<(), Box<dyn Error>> {
    let across = i32::try_from(ONE_COLUMN * ACROSS)?;
    let feet_row = i32::try_from(FEET_ROW)?;
    let (centre_x, centre_z) = (
        NEAR_THE_FAR_EDGE.x.floor() as i32,
        NEAR_THE_FAR_EDGE.z.floor() as i32,
    );
    let nearer_and_outside = cube
        .iter()
        .filter(|(_, y, _)| *y == feet_row)
        .filter(|(x, _, z)| (x - centre_x).abs().max((z - centre_z).abs()) < THE_WAY_OUTS_RING)
        .filter(|(x, _, z)| *x >= across || *z >= across)
        .count();
    require(
        nearer_and_outside > 0,
        format!(
            "this control needs the search to have to refuse ground the world does not hold before \
             it can reach the way out at ring {THE_WAY_OUTS_RING}, and every position it looks at \
             first is inside a world {across} blocks square — so an extent covering everything \
             would answer this fixture exactly as the right one does"
        ),
    )
}

/// Refuses unless the cells left clear are the only ones inside the world that
/// are.
fn require_nothing_inside_the_world_is_clear_but(
    blocks: &VoxelWorld,
    inside: &[(u32, u32, u32)],
    left_clear: &[(u32, u32, u32)],
) -> Result<(), Box<dyn Error>> {
    let clear: Vec<(u32, u32, u32)> = inside
        .iter()
        .copied()
        .filter(|cell| !holds(blocks, *cell))
        .collect();
    require(
        clear == left_clear,
        format!(
            "this fixture leaves {left_clear:?} clear inside the world and blocks everything else \
             the search may look at, and what is actually clear is {clear:?} — with a second \
             position open the destination is whichever the ring order meets first, and neither \
             direction of this pair is about the extent any more"
        ),
    )?;
    require(
        inside.len() < the_whole_cube(),
        format!(
            "this scenario needs part of the search cube to lie outside the world, and all \
             {count} of its positions are inside one",
            count = inside.len()
        ),
    )
}

/// How many positions the whole search cube holds: the horizontal reach both ways
/// on two axes, at the player's own row and the eight above it.
fn the_whole_cube() -> usize {
    let across = (2 * A_SEARCH_OF + 1) as usize;
    across * across * (A_SEARCH_OF + 1) as usize
}

/// Whether the world holds a block at `cell`.
fn holds(blocks: &VoxelWorld, cell: (u32, u32, u32)) -> bool {
    let (x, y, z) = cell;
    blocks
        .block_at(WorldPos { x, y, z })
        .is_ok_and(|contents| matches!(contents, Contents::Holds(_)))
}
