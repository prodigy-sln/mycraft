//! A player resuming a save is put somewhere they can move, and left exactly
//! where they were when they already could.
//!
//! # Four traps, each of which reddens a *correct* implementation if written the
//! obvious way
//!
//! 1. **The first snapshot is read at tick 0, not tick 1.** A cleared player
//!    arrives with `on_ground: false` — `resuming` sets it, and the clearing move
//!    touches position and velocity only — so a player put on a cell floor at
//!    entry is standing on it while claiming no contact, and **the first tick
//!    settles that by falling a fraction and landing**. Read at tick 1 the height
//!    differs, and the cheapest green would be to ground the player or to set
//!    `on_ground` in the search: both are Out of Scope. The scenario says *the
//!    first snapshot the simulation publishes*, and a simulation publishes one at
//!    construction, so tick 0 is what there is to read — which is why
//!    [`first_snapshot`] reports the tick beside the position.
//! 2. **A cell centre puts the feet on the cell's *floor*.** The horizontal
//!    coordinates gain a half and the vertical one does not.
//!    [`centred_on`] states that rule; deriving `y + 0.5` from "at that cell's
//!    centre" reddens a correct search, and the cheapest green is editing the
//!    search.
//! 3. **The move scenario asserts the exact destination and must never be
//!    weakened to "covers no solid cell".** It is doing duty as the
//!    off-the-map scenario's positive control *through the extent argument*: the
//!    destination it names is the last column inside this world, so an extent one
//!    cell too small rejects it while an extent that is too large passes it and
//!    fails next door. A bare "covers no solid cell" takes it off the only path
//!    where it can fail.
//! 4. **A scenario asserting the player was *not* moved is vacuously satisfied by
//!    a search that never moves anybody.** The two directions live in this file
//!    together for that reason: the abutting player and the wedged player must
//!    stay put, and the trapped player must be moved, and no implementation
//!    satisfies all three by accident.
//!
//! # What is deliberately not re-asserted
//!
//! The reach, the ring order, the cell-centre candidates, the absence of
//! downward offsets and the eligibility rule are the reload's, pinned by the
//! integration tests in `tests/reload_*.rs`. Re-proving them here would be
//! re-proving them through the same code path. What is new is the wiring: the
//! extent the entry caller passes, and a saved position near an edge, which no
//! reload fixture supplies. The second of those lives next door in
//! `tests/entry_will_not_clear_a_player_off_the_map.rs`.
//!
//! # Every world here is solid exactly where it holds a block
//!
//! The fixture registry declares one solid block and the replay's four, and every
//! cell these fixtures write holds the solid one — so "holds something" and "is
//! solid" are the same question of these worlds, which is what lets the premises
//! below be stated against the blocks rather than against a solidity view.

#[path = "support/entry.rs"]
mod entry;
#[path = "support/persistence.rs"]
mod persistence;

use std::error::Error;
use std::sync::Arc;

use glam::Vec3;
use mc_world::persistence::SavedPlayer;
use mc_world::section::Contents;
use mc_world::world::{VoxelWorld, WorldPos};

use entry::{
    A_SEARCH_OF, ASave, FEET_ROW, NO_ARGUMENT, at, cells_a_box_covers, eye_over, filling,
    first_snapshot, floor_of, ground_registry, inside_a_world, recorded_at, require, resumed,
    the_cube_around, without, written,
};
use persistence::{GROUND, TestResult, facing, refusal, stood_at};

/// How many chunk columns square each world here is.
///
/// One column is sixteen blocks across. The trapped player's one way out is the
/// **last column inside it**, which is what makes an extent one cell too small
/// reject the destination this file asserts.
const ONE_COLUMN: u32 = 1;

/// Three columns is forty-eight blocks across, which holds the whole seventeen
/// blocks of the search cube around the wedged player with room to spare — the
/// premise that keeps "nothing within eight blocks is clear" a claim about solid
/// ground rather than about the edge of the world.
const THREE_COLUMNS: u32 = 3;

/// Which tick a simulation publishes before any intent has been submitted.
const BEFORE_ANY_INTENT: u32 = 0;

/// Where the save records the trapped player.
///
/// **The height is a quarter of a block off the cell floor**, so the destination
/// differs from the recorded position on the vertical axis as well as the
/// horizontal one — which is what makes "feet on that cell's floor" an assertion
/// rather than a coincidence.
const TRAPPED_FEET: Vec3 = Vec3::new(14.5, FEET_ROW as f32 + 0.25, 8.5);

/// The cell their feet are in, and the two cells the one way out occupies.
const TRAPPED_CELL: (u32, u32, u32) = (14, FEET_ROW, 8);
const THE_WAY_OUT_CELLS: [(u32, u32, u32); 2] = [(15, FEET_ROW, 8), (15, FEET_ROW + 1, 8)];

/// Where the save records the abutting player, and the solid cell their box
/// touches without entering.
///
/// The box reaches `x = 13.0` exactly, and a voxel fills `[v, v + 1)` — so the
/// face lands on the near side of the cell beginning there and covers it not at
/// all. It is the boundary the half-open rule is about, and the position a
/// rounding of it in either direction would move.
const ABUTTING_FEET: Vec3 = Vec3::new(12.7, FEET_ROW as f32, 12.4);
const ABUTTED_CELL: (u32, u32, u32) = (13, FEET_ROW, 12);

/// Where the save records the wedged player: the centre of a three-column world,
/// with the whole search cube inside it.
const WEDGED_FEET: Vec3 = Vec3::new(24.5, FEET_ROW as f32 + 0.25, 24.5);

/// Which way the save records the moved player looking.
///
/// **Both angles, and neither of them zero, and neither a value a launch could
/// arrive at by inventing one.** A resume that restored where somebody stood and
/// forgot which way they were looking is the defect this is smallest against.
const RECORDED_YAW: f32 = 135.0;
const RECORDED_PITCH: f32 = -30.0;

#[test]
fn a_resumed_player_inside_a_solid_cell_starts_centred_on_the_clear_cell_one_step_sideways()
-> TestResult {
    let save = a_trap_with_one_way_out(recorded_at(TRAPPED_FEET, 0.0, 0.0))?;

    let launched = resumed(&save, &NO_ARGUMENT)?;

    assert_eq!(
        stood_at(&launched),
        Ok(at(the_way_out())),
        "the save records feet at {TRAPPED_FEET:?}, inside a cell that is solid, and the one \
         position within reach that is both inside this world and clear is one cell along +x. So \
         the player starts horizontally at that cell's centre with their feet on its floor, at \
         {:?} — **the exact destination and not merely somewhere clear**: this is the only \
         scenario an extent one cell too small can fail, and weakened to \"covers no solid cell\" \
         it goes green under a search that may not consider the last column of the world at all. \
         The launch answered: {}",
        the_way_out(),
        refusal(&launched)
    );
    Ok(())
}

#[test]
fn a_resumed_player_whose_box_abuts_a_solid_cell_without_overlapping_it_starts_exactly_where_the_save_recorded()
-> TestResult {
    let save = a_solid_cell_the_player_only_touches()?;

    let launched = resumed(&save, &NO_ARGUMENT)?;

    assert_eq!(
        stood_at(&launched),
        Ok(at(ABUTTING_FEET)),
        "the box reaches x = 13.0 exactly and the cell at {ABUTTED_CELL:?} begins there, so it is \
         touched and not entered — a player who needs no moving, who therefore loses neither their \
         sub-block position nor their place on the floor. An entry that cell-centred everybody, or \
         that read the boundary the other way round, moves them by three tenths of a block for no \
         reason and changes every committed golden frame with it. The launch answered: {}",
        refusal(&launched)
    );
    Ok(())
}

#[test]
fn a_player_moved_at_entry_still_faces_the_yaw_and_pitch_the_save_recorded() -> TestResult {
    let looking = recorded_at(
        TRAPPED_FEET,
        RECORDED_YAW.to_radians(),
        RECORDED_PITCH.to_radians(),
    );
    let save = a_trap_with_one_way_out(looking)?;

    let launched = resumed(&save, &NO_ARGUMENT)?;

    assert_eq!(
        (
            facing(&launched),
            stood_at(&launched) == Ok(at(TRAPPED_FEET))
        ),
        (
            Ok((
                RECORDED_YAW.to_radians().to_bits(),
                RECORDED_PITCH.to_radians().to_bits()
            )),
            false
        ),
        "entry moves where a player stands and never which way they are looking: they come back \
         facing {RECORDED_YAW} degrees round and {RECORDED_PITCH} degrees down, exactly as the save \
         records. The second half is this scenario's control — the player really was moved, so \
         \"when entry moves a resumed player\" is a premise that held rather than a case that \
         never arose. The launch answered: {}",
        refusal(&launched)
    );
    Ok(())
}

#[test]
fn a_resumed_player_with_nothing_clear_within_eight_blocks_starts_where_the_save_recorded()
-> TestResult {
    let save = a_wedge_with_no_way_out()?;

    let launched = resumed(&save, &NO_ARGUMENT)?;

    assert_eq!(
        stood_at(&launched),
        Ok(at(WEDGED_FEET)),
        "every position the search may look at covers at least one solid cell, and every one of \
         them is inside a world wide enough to hold the whole cube — so this is a wedge and not an \
         edge, and there is nowhere to put the player that is any better than where they are. They \
         start where the save recorded them, and the launch proceeds: a refused launch would take \
         away the edit-and-relaunch escape along with the save. The launch answered: {}",
        refusal(&launched)
    );
    Ok(())
}

#[test]
fn the_first_snapshot_a_simulation_publishes_reports_the_position_entry_moved_the_player_to()
-> TestResult {
    let save = a_trap_with_one_way_out(recorded_at(TRAPPED_FEET, 0.0, 0.0))?;

    let launched = resumed(&save, &NO_ARGUMENT)?;

    assert_eq!(
        first_snapshot(&launched),
        Ok((
            BEFORE_ANY_INTENT,
            at(the_way_out()),
            eye_over(the_way_out())
        )),
        "the first frame a run draws is drawn from the snapshot a simulation publishes at \
         construction, before any intent has been submitted — so the move has to be in *that* \
         snapshot and not merely in the state a later tick catches up to. The camera is asserted \
         beside the position because the camera is what the frame is actually drawn through: a \
         snapshot carrying the moved player and an eye derived from where they would have been \
         shows the world from inside the rock they were taken out of. **The tick is part of the \
         answer**, because a reading that arrived one tick later would satisfy a comparison that \
         left it out. The launch answered: {}",
        refusal(&launched)
    );
    Ok(())
}

/// Where the search must put the trapped player: horizontally centred on the one
/// clear cell, feet on its floor.
///
/// Derived from the cell through [`centred_on`] rather than written out as a
/// coordinate, so the expectation states the rule and cannot be a number
/// transcribed from a run.
fn the_way_out() -> Vec3 {
    centred_on(THE_WAY_OUT_CELLS[0])
}

/// Where a player standing at the centre of `cell` has their feet.
///
/// **Horizontally centred, feet on the cell's floor** — a half on x and z and
/// nothing at all on y. The search's own declaration, restated here: a fixture
/// reading the function it judges would agree with a rule that moved.
fn centred_on(cell: (u32, u32, u32)) -> Vec3 {
    let (x, y, z) = cell;
    Vec3::new(x as f32 + 0.5, y as f32, z as f32 + 0.5)
}

/// A save recording `player` inside solid rock, with one clear cell one step
/// along +x and nothing else clear beside them.
///
/// # Errors
///
/// Returns an error if the world cannot be built or written, or if either premise
/// fails: the recorded box has to be inside something solid, and the way out has
/// to be the only thing next to them that is not.
fn a_trap_with_one_way_out(player: SavedPlayer) -> Result<ASave, Box<dyn Error>> {
    let registry = ground_registry()?;
    let mut blocks = floor_of(&registry, ONE_COLUMN, GROUND)?;
    let walls = without(&beside(TRAPPED_CELL), &THE_WAY_OUT_CELLS);
    filling(&mut blocks, &registry, &walls, GROUND)?;

    require_the_save_traps_the_player(&blocks, TRAPPED_FEET)?;
    require_the_only_way_out_is_beside_them(&blocks)?;
    written(blocks, &registry, Arc::clone(&registry), player)
}

/// A save recording a player whose box touches one solid cell and enters none.
///
/// # Errors
///
/// Returns an error if the world cannot be built or written, or if the premise
/// fails: the box has to cover no solid cell while the cell it abuts holds one.
fn a_solid_cell_the_player_only_touches() -> Result<ASave, Box<dyn Error>> {
    let registry = ground_registry()?;
    let mut blocks = floor_of(&registry, ONE_COLUMN, GROUND)?;
    filling(&mut blocks, &registry, &[ABUTTED_CELL], GROUND)?;

    require_the_box_touches_without_entering(&blocks)?;
    written(
        blocks,
        &registry,
        Arc::clone(&registry),
        recorded_at(ABUTTING_FEET, 0.0, 0.0),
    )
}

/// A save recording a player every one of whose candidate positions is blocked,
/// in a world wide enough to hold the whole search cube.
///
/// # Errors
///
/// Returns an error if the world cannot be built or written, or if either premise
/// fails: the cube has to lie entirely inside the world, and every position in it
/// has to cover something solid.
fn a_wedge_with_no_way_out() -> Result<ASave, Box<dyn Error>> {
    let registry = ground_registry()?;
    let mut blocks = floor_of(&registry, THREE_COLUMNS, GROUND)?;
    let cube = the_cube_around(WEDGED_FEET);
    let filled = inside_a_world(&cube, THREE_COLUMNS);
    filling(&mut blocks, &registry, &filled, GROUND)?;

    require(
        filled.len() == cube.len(),
        format!(
            "this scenario needs every position the search may look at to be inside the world, or \
             it is about an edge rather than about a wedge — {outside} of the {total} positions \
             around {WEDGED_FEET:?} lie outside a world {THREE_COLUMNS} columns square",
            outside = cube.len() - filled.len(),
            total = cube.len()
        ),
    )?;
    require_every_position_in_the_cube_is_blocked(&blocks, &cube)?;
    written(
        blocks,
        &registry,
        Arc::clone(&registry),
        recorded_at(WEDGED_FEET, 0.0, 0.0),
    )
}

/// The cells of the nine columns around `cell`, at the two rows a standing box
/// covers.
fn beside(cell: (u32, u32, u32)) -> Vec<(u32, u32, u32)> {
    let (x, y, z) = cell;
    let mut around = Vec::new();
    for along in [z - 1, z, z + 1] {
        for across in [x - 1, x, x + 1] {
            around.push((across, y, along));
            around.push((across, y + 1, along));
        }
    }
    around
}

/// Refuses unless the box of a player standing at `feet` covers a cell holding a
/// block.
fn require_the_save_traps_the_player(
    blocks: &VoxelWorld,
    feet: Vec3,
) -> Result<(), Box<dyn Error>> {
    let covered = cells_a_box_covers(feet);
    let inside_something = covered.iter().filter(|cell| holds(blocks, **cell)).count();
    require(
        inside_something > 0,
        format!(
            "this scenario needs the recorded position to be inside solid rock, and none of the \
             {count} cells the box at {feet:?} covers holds a block",
            count = covered.len()
        ),
    )
}

/// Refuses unless the way out is clear and everything else beside the player is
/// not.
fn require_the_only_way_out_is_beside_them(blocks: &VoxelWorld) -> Result<(), Box<dyn Error>> {
    let clear: Vec<(u32, u32, u32)> = beside(TRAPPED_CELL)
        .into_iter()
        .filter(|cell| !holds(blocks, *cell))
        .collect();
    require(
        clear == THE_WAY_OUT_CELLS.to_vec(),
        format!(
            "this scenario needs exactly one way out of the trap, and the cells beside the player \
             that hold nothing are {clear:?} where {THE_WAY_OUT_CELLS:?} was laid out — with a \
             second one clear the destination is whichever the ring order meets first, and the \
             assertion stops being about the rule"
        ),
    )
}

/// Refuses unless the box at the recorded position covers no solid cell while the
/// cell it abuts holds one.
fn require_the_box_touches_without_entering(blocks: &VoxelWorld) -> Result<(), Box<dyn Error>> {
    let covered = cells_a_box_covers(ABUTTING_FEET);
    require(
        holds(blocks, ABUTTED_CELL),
        format!("this scenario needs {ABUTTED_CELL:?} to hold a block, and it holds nothing"),
    )?;
    require(
        !covered.contains(&ABUTTED_CELL),
        format!(
            "this scenario needs the box at {ABUTTING_FEET:?} to touch {ABUTTED_CELL:?} without \
             entering it, and the cells it covers are {covered:?} — a fixture whose box really did \
             overlap would be asserting that a trapped player is left where they are"
        ),
    )
}

/// Refuses unless every position the search may look at covers something solid.
fn require_every_position_in_the_cube_is_blocked(
    blocks: &VoxelWorld,
    cube: &[(i32, i32, i32)],
) -> Result<(), Box<dyn Error>> {
    let clear: Vec<(i32, i32, i32)> = cube
        .iter()
        .copied()
        .filter(|(x, y, z)| {
            let feet = Vec3::new(*x as f32 + 0.5, *y as f32, *z as f32 + 0.5);
            !cells_a_box_covers(feet)
                .iter()
                .any(|cell| holds(blocks, *cell))
        })
        .collect();
    require(
        clear.is_empty(),
        format!(
            "this scenario needs every one of the {total} positions within {A_SEARCH_OF} blocks to \
             cover at least one solid cell, and {count} of them are clear — the first at {first:?}, \
             which is where the search would rightly put the player",
            total = cube.len(),
            count = clear.len(),
            first = clear.first()
        ),
    )
}

/// Whether the world holds a block at `cell`.
fn holds(blocks: &VoxelWorld, cell: (u32, u32, u32)) -> bool {
    let (x, y, z) = cell;
    blocks
        .block_at(WorldPos { x, y, z })
        .is_ok_and(|contents| matches!(contents, Contents::Holds(_)))
}
