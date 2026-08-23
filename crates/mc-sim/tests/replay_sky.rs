//! What the generated world puts above its ground, which is nothing at all.
//!
//! The declaration fills a column from the world's floor to its surface, and
//! fills it again from the surface up to the sea where the surface stands under
//! the sea. Everything over that is **not filled with anything** — it is not a
//! block, not a name, and not a stratum the generator has to remember to write.
//!
//! **The first two tests here are each other's controls and neither is worth
//! anything alone.** A world that generated empty throughout satisfies "every
//! cell above the ground holds nothing" perfectly; a world that filled every
//! cell it can reach satisfies "every cell at or below the surface holds a
//! block" just as perfectly. Only the pair rules out both, so a reviewer meeting
//! either on its own should not read it as vacuous — the unit of judgement is
//! the set.
//!
//! **The sea test names a column whose surface stands below the sea on
//! purpose.** In a column standing above it there is no sea, and the boundary
//! between the sea's top and the empty cell over it does not exist — so the
//! assertion would be about ordinary sky and would pass against a fixture that
//! could not have failed it. The column is found by searching the surface
//! heights rather than written down, because a coordinate read off a run is a
//! coordinate that commits whatever the generator did that day.
//!
//! **The falling player is judged through the collision view, and everything it
//! is judged against is re-read from the world and the registry.** The resting
//! height is the surface height the world reports plus one, and that the surface
//! block holds the player up at all is asked of the registry — never of the
//! bitset the physics reads, which is the thing under test here. A judge that
//! borrowed the subject's own reasoning would agree with a subject that reported
//! an empty cell solid.

mod support;

use std::error::Error;

use glam::Vec3;
use mc_core::block::BlockRegistry;
use mc_sim::player::{MovementIntent, PlayerState, advance_player};
use mc_sim::replay::{ReplayWorld, ResolvedVoxels};
use mc_world::column::COLUMN_HEIGHT;
use mc_world::section::Contents;

use support::{
    LANDMARK, LANDMARK_TOP, NOTHING, SEA_LEVEL, TestResult, WATER, block_at, content_registry,
    every_column, replay_world, surface_height,
};

/// How far two heights this suite calls equal may differ, in blocks. The
/// specification's declared comparison epsilon.
const EPSILON: f32 = 1e-4;

/// How long a fall is given to land and settle.
///
/// A two-block fall takes 22 ticks and a player that has landed stays landed, so
/// a longer watch cannot change the answer.
const SETTLE_TICKS: u32 = 60;

/// How far above a column's surface a fall onto it starts, in blocks.
///
/// Three, so the feet begin two blocks clear of the surface's own top face and
/// one clear of the cell that would hold them up if that cell were reported
/// solid. Both wrong answers are a whole block away from the right one.
const FALL_HEIGHT: u32 = 3;

/// How many departures from the declaration a failure message lists.
///
/// A sky reported cell by cell is a quarter of a million lines nobody reads, so
/// the count is carried whole and the first few are shown.
const REPORTED_FAULTS: usize = 8;

/// What a walk over part of the world saw: how many cells it looked at, how many
/// of them departed from the declaration, and the first few of those.
#[derive(Debug, Default)]
struct Walk {
    inspected: usize,
    faults: usize,
    first: Vec<String>,
}

impl Walk {
    /// One more cell looked at, and what was wrong with it if anything was.
    fn saw(&mut self, fault: Option<String>) {
        self.inspected += 1;
        let Some(fault) = fault else {
            return;
        };
        self.faults += 1;
        if self.first.len() < REPORTED_FAULTS {
            self.first.push(fault);
        }
    }
}

#[test]
fn every_cell_above_a_columns_surface_and_the_sea_holds_no_block() -> TestResult {
    let world = replay_world(&content_registry()?)?;
    let mut walk = Walk::default();

    for (x, z) in every_column() {
        above_the_ground(&world, (x, z), &mut walk)?;
    }

    assert!(
        walk.inspected > 0,
        "the walk reached no cell above any column's ground, so the assertion below is about \
         nothing at all"
    );
    assert!(
        walk.faults == 0,
        "the sky is not made of anything. The generator fills a column to its surface and to \
         the sea, and writes nothing whatsoever above that — a cell there holds no block, not a \
         block that stands for empty space. {} of the {} cells above the ground hold one: {:?}",
        walk.faults,
        walk.inspected,
        walk.first
    );
    Ok(())
}

#[test]
fn every_cell_at_or_below_a_columns_surface_holds_a_block() -> TestResult {
    let world = replay_world(&content_registry()?)?;
    let mut walk = Walk::default();

    for (x, z) in every_column() {
        down_to_the_world_floor(&world, (x, z), &mut walk)?;
    }

    assert!(
        walk.inspected > 0,
        "the walk reached no cell of any column's ground, so the assertion below is about \
         nothing at all"
    );
    assert!(
        walk.faults == 0,
        "the ground is solid all the way down: every cell from the world's floor to a column's \
         surface holds one of the blocks the strata name. This is what an emptiness spreading \
         past the sky would show up in, and it is the reason the emptiness test above is not \
         satisfied by a world that generated nothing anywhere. {} of the {} cells at or below a \
         surface hold no block: {:?}",
        walk.faults,
        walk.inspected,
        walk.first
    );
    Ok(())
}

#[test]
fn the_sea_fills_a_submerged_column_to_sea_level_and_the_cell_over_it_holds_no_block() -> TestResult
{
    let world = replay_world(&content_registry()?)?;
    let (x, z) = a_submerged_column(&world)?;

    assert_eq!(
        (
            block_at(&world, x, SEA_LEVEL, z)?,
            block_at(&world, x, SEA_LEVEL + 1, z)?
        ),
        (WATER.to_owned(), NOTHING.to_owned()),
        "column ({x}, {z}) stands its surface below the declared sea level of {SEA_LEVEL}, so \
         the sea reaches that level and stops there. The cell above it is where the water ends \
         and the sky begins, and the sky is nothing — a block of any name there is a generator \
         that filled the space it left over, and a cell at the sea level holding nothing is one \
         that never filled the sea"
    );
    Ok(())
}

#[test]
fn the_cell_one_step_above_a_columns_surface_holds_no_block_and_does_not_stop_a_falling_player()
-> TestResult {
    let registry = content_registry()?;
    let world = replay_world(&registry)?;
    let voxels = ResolvedVoxels::resolve(&world, &registry)?;
    let column = a_column_open_to_the_sky(&world)?;
    let standing_on_the_surface = (column.surface + 1) as f32;

    let landed = settled(dropped_over(column), &voxels);

    assert!(
        the_surface_block_is_solid(&world, &registry, column)?,
        "the block at column {column:?}'s own surface has to be one the registry calls solid, or \
         a player that comes to rest on its top face did so for a reason that has nothing to do \
         with what the cell above it holds"
    );
    assert!(
        block_at(&world, column.x, column.surface + 1, column.z)? == NOTHING
            && (landed.position.y - standing_on_the_surface).abs() <= EPSILON
            && landed.on_ground,
        "the cell one step above the surface of column {column:?} holds nothing, and nothing \
         stops nobody: a player let go over it falls through to stand on the surface's own top \
         face at {standing_on_the_surface}. It came to rest at {} with ground contact {}, and \
         the cell holds `{}` — a rest one block higher is an empty cell the collision view \
         reported solid, which every read of that view answers alike and only the world itself \
         can contradict",
        landed.position.y,
        landed.on_ground,
        block_at(&world, column.x, column.surface + 1, column.z)?
    );
    Ok(())
}

/// Looks at every cell of one column above everything the declaration fills.
///
/// # Errors
///
/// Returns an error if the world reaches no cell the walk asks about.
fn above_the_ground(
    world: &ReplayWorld,
    column: (u32, u32),
    walk: &mut Walk,
) -> Result<(), Box<dyn Error>> {
    let (x, z) = column;
    for y in (highest_filled_cell(world, x, z)? + 1)..COLUMN_HEIGHT {
        let held = a_block_at(world, x, y, z)?;
        walk.saw(held.map(|name| format!("({x}, {y}, {z}) holds `{name}`")));
    }
    Ok(())
}

/// Looks at every cell of one column from its surface down to the world's floor.
///
/// # Errors
///
/// Returns an error if the world reaches no cell the walk asks about.
fn down_to_the_world_floor(
    world: &ReplayWorld,
    column: (u32, u32),
    walk: &mut Walk,
) -> Result<(), Box<dyn Error>> {
    let (x, z) = column;
    for y in 0..=surface_height(world, x, z)? {
        let empty = a_block_at(world, x, y, z)?.is_none();
        walk.saw(empty.then(|| format!("({x}, {y}, {z}) holds nothing")));
    }
    Ok(())
}

/// What the generated world holds at a position: the block's own name, or
/// nothing where the cell holds none.
///
/// **Three answers and three arms, never two.** A position the world does not
/// reach is an error rather than one of the other two: a world answering
/// "outside" everywhere would otherwise satisfy every assertion written about
/// its contents by having none of them, and folding that answer into "this cell
/// holds nothing" is exactly how a position past the edge of the world becomes
/// ordinary empty space.
///
/// # Errors
///
/// Returns an error if the world reaches no cell there.
fn a_block_at(
    world: &ReplayWorld,
    x: u32,
    y: u32,
    z: u32,
) -> Result<Option<String>, Box<dyn Error>> {
    match world.block_at(x, y, z) {
        None => Err(format!("the replay world reaches no cell at ({x}, {y}, {z})").into()),
        Some(Contents::Empty) => Ok(None),
        Some(Contents::Holds(name)) => Ok(Some(name.as_str().to_owned())),
    }
}

/// The highest cell the declaration writes a block into, in one column.
///
/// A column's surface, or the sea's own level where the surface stands under it.
/// The one column the declaration stands a pillar in is measured to the pillar's
/// top instead: its fill reaches past its surface by design, and what this
/// boundary is about is where the declaration stops filling — so treating that
/// column as ordinary would report the pillar as sky rather than covering the
/// pillar's own sky at all.
///
/// # Errors
///
/// Returns an error if the world reports no surface height for the column.
fn highest_filled_cell(world: &ReplayWorld, x: u32, z: u32) -> Result<u32, Box<dyn Error>> {
    let filled_to = if (x, z) == LANDMARK {
        LANDMARK_TOP
    } else {
        surface_height(world, x, z)?
    };
    Ok(filled_to.max(SEA_LEVEL))
}

/// The first column whose surface stands below the sea, in [`every_column`]
/// order.
///
/// The landmark's column is passed over: the declaration stands a pillar of
/// stone through it from its surface upward, so whatever its surface height,
/// its sea level holds the pillar rather than the sea.
///
/// # Errors
///
/// Returns an error if no column stands under the sea, which would leave this
/// scenario measuring a boundary the world does not have.
fn a_submerged_column(world: &ReplayWorld) -> Result<(u32, u32), Box<dyn Error>> {
    for (x, z) in every_column() {
        if (x, z) != LANDMARK && surface_height(world, x, z)? < SEA_LEVEL {
            return Ok((x, z));
        }
    }
    Err(format!(
        "no column of the generated world stands its surface below the declared sea level of \
         {SEA_LEVEL}, so no column holds a sea and the boundary between the sea's top and the \
         empty cell over it is not in this world to be measured"
    )
    .into())
}

/// A column of the generated world with nothing over its surface.
#[derive(Debug, Clone, Copy)]
struct OpenColumn {
    x: u32,
    z: u32,
    surface: u32,
}

/// The first such column, in [`every_column`] order.
///
/// Three conditions, and every one of them is a constraint on the fixture's
/// *shape* that no assertion in the test can enforce. The surface must stand at
/// or above the sea, or the cell over it holds water and this scenario is about
/// the cell that holds nothing. The two coordinates must **differ**, and the
/// transposed column must stand at a **different height**, because a resting
/// height read out of column `(z, x)` instead of `(x, z)` is the one indexing
/// defect a fall onto a horizontally uniform answer cannot see. And the
/// landmark's column is passed over, because a pillar of stone stands over its
/// surface and the cell above it holds a block by declaration.
///
/// # Errors
///
/// Returns an error if the world has no such column.
fn a_column_open_to_the_sky(world: &ReplayWorld) -> Result<OpenColumn, Box<dyn Error>> {
    for (x, z) in every_column() {
        if x == z || (x, z) == LANDMARK {
            continue;
        }
        let surface = surface_height(world, x, z)?;
        if surface >= SEA_LEVEL && surface_height(world, z, x)? != surface {
            return Ok(OpenColumn { x, z, surface });
        }
    }
    Err(
        "no column of the generated world stands its surface at or above the sea with its \
         transposed column at a different height, so a fall onto one would measure neither an \
         open sky nor which column was consulted"
            .into(),
    )
}

/// Whether the block at a column's surface is one the registry calls solid.
///
/// Read back through the registry from the name the world holds, which is the
/// independent half of the judgement: the collision view under test resolved the
/// same question once at construction, and asking it again would be asking the
/// subject to grade itself.
///
/// # Errors
///
/// Returns an error if the world reaches no cell at the surface, or if the
/// registry does not know what it holds.
fn the_surface_block_is_solid(
    world: &ReplayWorld,
    registry: &BlockRegistry,
    column: OpenColumn,
) -> Result<bool, Box<dyn Error>> {
    match world.block_at(column.x, column.surface, column.z) {
        None => {
            Err(format!("the replay world reaches no cell at the surface of {column:?}").into())
        }
        Some(Contents::Empty) => Ok(false),
        Some(Contents::Holds(name)) => Ok(registry.resolve(name)?.is_solid),
    }
}

/// A player let go from rest over a column, out of contact with anything.
fn dropped_over(column: OpenColumn) -> PlayerState {
    PlayerState {
        position: Vec3::new(
            column.x as f32 + 0.5,
            (column.surface + FALL_HEIGHT) as f32,
            column.z as f32 + 0.5,
        ),
        velocity: Vec3::ZERO,
        yaw: 0.0,
        pitch: 0.0,
        on_ground: false,
    }
}

/// Where a fall watched for [`SETTLE_TICKS`] ticks leaves `state`.
fn settled(state: PlayerState, voxels: &ResolvedVoxels) -> PlayerState {
    (0..SETTLE_TICKS).fold(state, |state, _| {
        advance_player(state, &MovementIntent::default(), voxels)
    })
}
