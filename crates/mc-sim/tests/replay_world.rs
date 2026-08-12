//! The replay world is a pure function of a fixed seed, and its content is the
//! one the specification declares.
//!
//! The declaration is the input to every test here and none of it is read back
//! out of a run: the footprint, the surface band, the sea level, the strata and
//! the landmark all live in `support/mod.rs` as constants copied from the
//! specification. A world that came out at the wrong heights, made of the wrong
//! blocks, or flat, would satisfy every *geometry* assertion this feature makes
//! and would then be captured into its own goldens — so these are the
//! assertions that stand between that and a green gate.
//!
//! **Two of these are unfalsifiable alone and load-bearing together.** A flat
//! heightmap satisfies the adjacent-step bound perfectly, and per-column white
//! noise satisfies the distinct-height count perfectly; only the pair rules out
//! both. A reviewer meeting either on its own should not read it as vacuous —
//! the unit of judgement is the set, exactly as
//! `docs/technical/testing.md` records for the mesher's property tests.

mod support;

use std::collections::BTreeSet;
use std::error::Error;

use mc_sim::REPLAY_SEED;
use mc_sim::replay::{ReplayWorld, mesh_all};
use mc_world::column::SECTIONS_PER_COLUMN;
use mc_world::section::SectionData;

use support::{
    DIRT, DIRT_DEPTH, FOOTPRINT, GRASS, HIGHEST_SURFACE, LANDMARK, LANDMARK_TOP, LOWEST_SURFACE,
    SEA_LEVEL, STONE, TestResult, WATER, block_at, content_registry, every_column, heightmap,
    replay_world, surface_height,
};

/// How many columns the footprint is: four by four.
const COLUMNS: usize = 16;

/// The most two adjacent surface heights may differ by.
///
/// Derived, not measured. Interpolated value noise on a lattice period of 16
/// with `3t^2 - 2t^3` has a maximum derivative of 1.5 over the interval, so the
/// field's slope is at most `1.5 * 16 / 16 = 1.5` per block and two adjacent
/// integer heights differ by at most 2.
const MAX_ADJACENT_STEP: u32 = 2;

/// How many distinct heights the footprint must show.
///
/// The one figure here that is an assumption rather than a derivation: a lattice
/// period of 16 over a 64-block footprint gives a 4 by 4 lattice interpolated
/// over a 17-value range, so many distinct integers are expected, but the count
/// depends on the hash. If it falls short the amplitude or the hash moves, never
/// this number.
const MINIMUM_DISTINCT_HEIGHTS: usize = 8;

/// How far above its four neighbours' surfaces the landmark must stand.
const MINIMUM_LANDMARK_CLEARANCE: u32 = 12;

#[test]
fn generating_the_replay_world_twice_from_one_seed_stores_identical_sections() -> TestResult {
    let registry = content_registry()?;
    let first = exported_sections(&replay_world(&registry)?)?;
    let second = exported_sections(&replay_world(&registry)?)?;

    assert_eq!(
        first.len(),
        COLUMNS * SECTIONS_PER_COLUMN as usize,
        "every column's every section has to be exported, or the comparison below \
         covers less of the world than it claims to"
    );
    assert!(
        first.iter().any(|section| section.palette.len() > 1),
        "every section exported the same single block, so two empty worlds would \
         compare equal and this assertion would prove nothing"
    );
    assert_eq!(
        first, second,
        "the same seed has to build the same world, block for block"
    );
    Ok(())
}

/// A flat heightmap passes this and fails the distinct-height count below; white
/// noise does the reverse. See the module note.
#[test]
fn no_two_adjacent_columns_of_the_replay_surface_differ_by_more_than_two_blocks() -> TestResult {
    let world = replay_world(&content_registry()?)?;

    let (step, at) = largest_adjacent_step(&world)?;

    assert!(
        step <= MAX_ADJACENT_STEP,
        "the surface has to be spatially coherent: column {at:?} steps {step} blocks to a \
         neighbour, and the lattice the heightmap is built on admits at most \
         {MAX_ADJACENT_STEP}"
    );
    Ok(())
}

#[test]
fn the_replay_surface_holds_at_least_eight_distinct_heights_across_its_footprint() -> TestResult {
    let world = replay_world(&content_registry()?)?;

    let distinct: BTreeSet<u32> = heightmap(&world)?.into_iter().collect();

    assert!(
        distinct.len() >= MINIMUM_DISTINCT_HEIGHTS,
        "a flat or nearly flat world is not the declared one: {} distinct heights across \
         the footprint, against {MINIMUM_DISTINCT_HEIGHTS} required — {distinct:?}",
        distinct.len()
    );
    Ok(())
}

#[test]
fn every_replay_column_places_its_surface_inside_the_declared_band() -> TestResult {
    let world = replay_world(&content_registry()?)?;
    let mut outside = Vec::new();

    for (x, z) in every_column() {
        let height = surface_height(&world, x, z)?;
        if !(LOWEST_SURFACE..=HIGHEST_SURFACE).contains(&height) {
            outside.push(format!("({x}, {z}) at {height}"));
        }
    }

    assert!(
        outside.is_empty(),
        "every surface belongs in {LOWEST_SURFACE}..={HIGHEST_SURFACE}, and these do not: \
         {outside:?}"
    );
    Ok(())
}

/// The landmark column is left out on purpose: the declaration overwrites its
/// surface with stone up to the pillar's top, which is exactly why the world's
/// upward grass area is 4095 and not 4096. Its own contents are asserted by the
/// landmark test below rather than skipped.
#[test]
fn every_other_column_shows_grass_over_three_blocks_of_dirt_over_stone() -> TestResult {
    let world = replay_world(&content_registry()?)?;
    let mut faults = Vec::new();

    for (x, z) in every_column().filter(|column| *column != LANDMARK) {
        faults.extend(strata_faults(&world, x, z)?);
    }

    assert!(
        faults.is_empty(),
        "the declared strata are grass, then {DIRT_DEPTH} of dirt, then stone to the \
         world floor: {faults:?}"
    );
    Ok(())
}

#[test]
fn the_landmark_column_stands_in_stone_from_its_surface_to_the_declared_top() -> TestResult {
    let world = replay_world(&content_registry()?)?;
    let (x, z) = LANDMARK;
    let surface = surface_height(&world, x, z)?;
    let mut faults = Vec::new();

    for y in surface..=LANDMARK_TOP {
        let held = block_at(&world, x, y, z)?;
        if held.as_str() != STONE {
            faults.push(format!("y = {y} holds `{}`", held.as_str()));
        }
    }
    let clearance = landmark_clearance(&world)?;

    assert!(
        faults.is_empty(),
        "the landmark is stone all the way up: {faults:?}"
    );
    assert!(
        clearance >= MINIMUM_LANDMARK_CLEARANCE,
        "the landmark has to clear its four neighbours by {MINIMUM_LANDMARK_CLEARANCE} \
         blocks so it projects above the horizon; it clears them by {clearance}"
    );
    Ok(())
}

/// Both guards below are the point of this test. Water emits no faces because it
/// is non-solid, so a world holding no water at all, or a mesh holding no quads
/// at all, satisfies "no quad names water" while measuring nothing.
#[test]
fn no_quad_of_the_meshed_replay_names_the_block_that_fills_its_sea() -> TestResult {
    let registry = content_registry()?;
    let world = replay_world(&registry)?;
    let sections = mesh_all(&world, &registry)?;

    let quads: usize = sections.iter().map(|section| section.quads.len()).sum();
    let watery = sections
        .iter()
        .flat_map(|section| section.quads.iter())
        .filter(|quad| quad.block.as_str() == WATER)
        .count();

    assert!(quads > 0, "the meshed replay emitted no quad at all");
    assert!(
        any_column_holds_water(&world)?,
        "no column of the world is below the declared sea level at {SEA_LEVEL}, so there \
         is no water for a quad to name and this assertion is vacuous"
    );
    assert_eq!(watery, 0, "water is non-solid and shows no face");
    Ok(())
}

#[test]
fn a_neighbouring_seed_moves_at_least_one_column_of_the_replay_surface() -> TestResult {
    let registry = content_registry()?;
    let declared = heightmap(&ReplayWorld::generate(REPLAY_SEED, &registry)?)?;
    let neighbouring = heightmap(&ReplayWorld::generate(REPLAY_SEED + 1, &registry)?)?;

    assert_eq!(
        declared.len(),
        (FOOTPRINT * FOOTPRINT) as usize,
        "both heightmaps have to cover the whole footprint, or a shorter one could \
         differ for the wrong reason"
    );
    assert_ne!(
        declared, neighbouring,
        "a heightmap that ignored its seed would be identical under both"
    );
    Ok(())
}

/// Every section of every column, in the form that means the same thing under
/// any registry.
///
/// The exported form rather than the sections themselves: it carries the palette
/// and one position per voxel, so a world that reached the same voxels through a
/// different write history is still caught if its palette came out in a
/// different order.
fn exported_sections(world: &ReplayWorld) -> Result<Vec<SectionData>, Box<dyn Error>> {
    let mut exported = Vec::new();
    for column in world.columns() {
        for index in 0..SECTIONS_PER_COLUMN as usize {
            let section = column
                .section(index)
                .ok_or_else(|| format!("a column stacks no section at index {index}"))?;
            exported.push(section.export()?);
        }
    }
    Ok(exported)
}

/// The largest step between any two adjacent columns, and where it is.
fn largest_adjacent_step(world: &ReplayWorld) -> Result<(u32, (u32, u32)), Box<dyn Error>> {
    let mut largest = (0, (0, 0));
    for (x, z) in every_column() {
        let step = largest_step_from(world, x, z)?;
        if step.0 > largest.0 {
            largest = step;
        }
    }
    Ok(largest)
}

/// The largest step from one column to the neighbour on its `+x` or `+z` side.
///
/// Two of the four neighbours, because every pair is reached from one side or
/// the other and looking at both ends would only measure each pair twice.
fn largest_step_from(
    world: &ReplayWorld,
    x: u32,
    z: u32,
) -> Result<(u32, (u32, u32)), Box<dyn Error>> {
    let here = surface_height(world, x, z)?;
    let mut largest = 0;
    for (nx, nz) in [(x + 1, z), (x, z + 1)] {
        if nx < FOOTPRINT && nz < FOOTPRINT {
            largest = largest.max(here.abs_diff(surface_height(world, nx, nz)?));
        }
    }
    Ok((largest, (x, z)))
}

/// Where one column departs from the declared strata.
fn strata_faults(world: &ReplayWorld, x: u32, z: u32) -> Result<Vec<String>, Box<dyn Error>> {
    let surface = surface_height(world, x, z)?;
    let mut expected = vec![(surface, GRASS)];
    expected.extend((1..=DIRT_DEPTH).map(|below| (surface.saturating_sub(below), DIRT)));
    expected.push((surface.saturating_sub(DIRT_DEPTH + 1), STONE));
    expected.push((0, STONE));

    let mut faults = Vec::new();
    for (y, block) in expected {
        let held = block_at(world, x, y, z)?;
        if held.as_str() != block {
            faults.push(format!(
                "({x}, {y}, {z}) holds `{held}`, not `{block}`",
                held = held.as_str()
            ));
        }
    }
    Ok(faults)
}

/// How far the landmark's top stands above the highest of its four neighbours.
fn landmark_clearance(world: &ReplayWorld) -> Result<u32, Box<dyn Error>> {
    let (x, z) = LANDMARK;
    let mut clearance = u32::MAX;
    for (nx, nz) in [(x - 1, z), (x + 1, z), (x, z - 1), (x, z + 1)] {
        let neighbour = surface_height(world, nx, nz)?;
        clearance = clearance.min(LANDMARK_TOP.saturating_sub(neighbour));
    }
    Ok(clearance)
}

/// Whether any column of the world holds water at all.
fn any_column_holds_water(world: &ReplayWorld) -> Result<bool, Box<dyn Error>> {
    for (x, z) in every_column() {
        if column_holds_water(world, x, z)? {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Whether one column holds water between its surface and the declared sea
/// level.
fn column_holds_water(world: &ReplayWorld, x: u32, z: u32) -> Result<bool, Box<dyn Error>> {
    let surface = surface_height(world, x, z)?;
    for y in surface..=SEA_LEVEL {
        if block_at(world, x, y, z)?.as_str() == WATER {
            return Ok(true);
        }
    }
    Ok(false)
}
