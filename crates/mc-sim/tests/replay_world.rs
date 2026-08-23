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

use mc_core::id::BlockName;
use mc_sim::REPLAY_SEED;
use mc_sim::replay::contract::scene_contract;
use mc_sim::replay::{ReplayWorld, mesh_all};
use mc_world::column::{COLUMN_HEIGHT, SECTIONS_PER_COLUMN};
use mc_world::section::SectionData;

use support::{
    DIRT, DIRT_DEPTH, FOOTPRINT, GRASS, HIGHEST_SURFACE, LANDMARK, LANDMARK_TOP, LOWEST_SURFACE,
    NOTHING, SEA_LEVEL, STONE, TestResult, WATER, block_at, content_registry, every_column,
    heightmap, replay_world, submerged_columns, surface_height,
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

/// The sea, as it stood before `base:water` declared anything — a **tripwire and
/// not an oracle**, in the shape `SCENE_QUAD_COUNT` uses and for its reason.
///
/// Recorded at commit `bc7541a`, ahead of the measurement they predict. No
/// arithmetic derives them: the count is a sum over submerged columns of
/// `SEA_LEVEL − surface` and the heightmap is a hash, so these are the one part
/// of the census that is committed rather than computed. What verifies the
/// census is the two enumerations agreeing and the sides summing to six a
/// voxel; what *these* do, and nothing else can, is notice the world itself
/// moved.
const WATER_VOXELS: usize = 178;

/// Water sides open to the air or to the world's edge.
///
/// **The two are counted as one because the walk answers them as one** — its
/// `shows` collapses "the step left the world" and "the cell holds nothing" into
/// a single arm, so a bucket that split them would be finer than the thing it is
/// a yardstick for.
const OPEN_SIDES: usize = 201;

/// Water sides meeting another water voxel — the interior of the sea, which the
/// engine's own rule that a block draws no face against its own kind is what
/// culls.
const WATERY_SIDES: usize = 662;

/// Water sides meeting a named block that is **not** water.
///
/// **Named for what is counted, not for what is inferred.** Every such block is
/// the lakebed or its shore, and each of them does occlude — but that is a
/// reading of the shipped declarations, where `base:dirt`, `base:grass` and
/// `base:stone` state `solid = true` and nothing about occlusion, so `occludes`
/// answers from it. The census establishes only that the neighbour is some
/// named block other than water.
const OTHER_SIDES: usize = 205;

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
        if held != STONE {
            faults.push(format!("y = {y} holds `{held}`"));
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

/// The guard below is the point of this test, and it survives the inversion it
/// was written for. It used to hold up the claim that *no* quad names water —
/// a world holding no water at all satisfies that while measuring nothing — and
/// it holds up the opposite claim for the same reason: a lower bound counted
/// over a sea that is not there is a lower bound of zero, which any mesh meets.
///
/// The bound itself is counted rather than committed. Each submerged column owes
/// one upward water face, so the sea's meshed area cannot be under the number of
/// them however greedily the faces are merged into rectangles — merging changes
/// how visible faces are grouped and never which faces are visible.
#[test]
fn the_meshed_replay_shows_the_block_that_fills_its_sea() -> TestResult {
    let registry = content_registry()?;
    let world = replay_world(&registry)?;
    let sections = mesh_all(&world, &registry)?;

    let quads: usize = sections.iter().map(|section| section.quads.len()).sum();
    let submerged = submerged_columns(&world)?;
    let watery = scene_contract(&sections)
        .area_by_block
        .get(&BlockName::parse(WATER)?)
        .copied()
        .unwrap_or_default();

    assert!(quads > 0, "the meshed replay emitted no quad at all");
    assert!(
        submerged > 0,
        "no column of the world is below the declared sea level at {SEA_LEVEL}, so there \
         is no water for a quad to name and this assertion is vacuous"
    );
    assert!(
        watery >= submerged,
        "the sea covers {submerged} columns of the declared world and every one of them is \
         open to the air above it, so the mesh owes at least that much water area — and it \
         meshed {watery}. Zero is a mesher that still decides what to draw by what a player \
         would walk into; anything short of the floor is a sea drawn in part"
    );
    Ok(())
}

/// A census of the sea, and the three layers it is built in.
///
/// # What it is for
///
/// It is the yardstick the walk's own questions are measured against. Deleting
/// one of the three questions the independent per-voxel walk asks should move
/// its answer by an amount this census predicts, and a **delta of the wrong
/// size** is as much a finding as no delta at all. So the numbers have to come
/// from somewhere the walk is not.
///
/// # It is independent of the walk, and here is exactly how
///
/// It resolves **no block definition at all** — no `drawn`, no `occludes`, no
/// registry read beyond the one that generates the world — and it calls none of
/// `visible_face_area`, the walk's `Beyond`, its `shows`, its `Side::step` or
/// its `inside_the_world`. Its six offsets and its bounds are written out below,
/// its enumeration comes from the surface heights and [`SEA_LEVEL`] alone, and
/// it classifies a neighbour by **block name text**.
///
/// What it does share with the walk is [`block_at`] and [`surface_height`] —
/// the *subject world's* own public accessors. That is reading the subject, not
/// sharing a derivation of visibility, and it is the same relationship
/// [`submerged_columns`] already has.
///
/// # Three layers, and each is useless for the other's job
///
/// 1. **Two enumerations compared.** The heightmap rule against a scan of all
///    `64 × 64 × 256` cells. Neither is snapshotted; a disagreement means the
///    rule missed water the generator placed, and *that* is the finding.
/// 2. **The arithmetic.** The three buckets sum to `voxels × 6`, which catches a
///    miscount of the buckets that the enumerations agreeing cannot.
/// 3. **The four committed numbers**, which are a **tripwire and not an
///    oracle** — the same shape, and for the same reason, as
///    `SCENE_QUAD_COUNT`. No arithmetic derives them: 178 is a sum over
///    submerged columns of `SEA_LEVEL − surface`, and the heightmap is a hash.
///
/// **Layer 3 earns its place because layers 1 and 2 cannot do its job.** Both
/// are internally consistent about *whatever world they are handed*: if the
/// generator changed and the sea moved, both stay green. The snapshot is the
/// only layer that notices the world itself changed. That is why four
/// underivable numbers are worth committing, and it is the argument against
/// deleting them later as "not derived".
///
/// # This census may CHECK the mesher and must never be SET by it
///
/// The mesher reports water's area as part of its own contract, and the
/// independent walk agrees with it, so those two corroborate each other. If this
/// census ever disagrees with them the fault is **here** — two implementations
/// that share nothing but a registry lookup do not both drift the same way. The
/// repair is then to find the water cells this enumeration missed and understand
/// why, **never to adopt their figure**: doing that would make this a
/// restatement of the subject, which is the one thing it must not be. Their
/// value says where to look. It never says what the answer is.
///
/// # Measured before the declaration it is used across, and that is the point
///
/// These figures were taken **before `base:water` declared anything** —
/// recorded at commit `bc7541a`, ahead of the measurement they predict — as
/// **178** voxels and **201 / 662 / 205**. The census measures *voxels*, and
/// declaring a block drawn changes what is *drawn*, so the numbers should be
/// invariant across that change.
///
/// **They are: this test was first run after the declaration landed, after the
/// spawn moved twice and after the golden set was re-shot, and it reads the
/// figures recorded before any of it.** That invariance is a result rather than
/// an absence — it says world generation does not depend on what a block
/// declares, which was an assumption until this test existed. The day it fails
/// while nobody moved the sea, that assumption is what broke.
#[test]
fn the_declared_sea_is_the_one_the_heightmap_implies_and_its_sides_add_up() -> TestResult {
    let world = replay_world(&content_registry()?)?;

    let implied = water_the_heightmap_implies(&world)?;
    let found = every_water_voxel_in_the_world(&world)?;
    let sides = sides_of(&world, &implied)?;

    assert_eq!(
        implied,
        found,
        "the heightmap's prediction of where the sea is against a scan of every cell the \
         world holds. Water the rule misses is what would make every figure below a floor \
         rather than a total — {}",
        first_absentee(&implied, &found)
    );
    assert_eq!(
        sides.open + sides.watery + sides.other,
        implied.len() * 6,
        "six sides a voxel, each in exactly one bucket, so this holds over any world and \
         is the one check here not about *this* sea. It sees a side skipped or counted \
         twice, which the enumerations cannot: they never look at a side"
    );
    assert_eq!(
        counted(&implied, &sides),
        DECLARED_SEA,
        "a disagreement means one of two things and they are not the same: generation \
         changed and the sea moved by accident, or somebody moved it on purpose. Find out \
         which, and only if it was intended update all four **together** — they are one \
         census. **Do not settle it with the mesher's water area**; this test's own doc \
         says why that would make the census a restatement of what it exists to measure"
    );
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

/// What the census comes to: voxels, then sides open, watery and other.
///
/// **The total is deliberately not here.** The sides summing to `voxels × 6` is
/// an invariant of any world and is asserted on its own, so that a defect in the
/// classification reports itself as that and not as the sea having moved. These
/// four are the tripwire and nothing else.
type Census = (usize, usize, usize, usize);

/// The census the declared sea has always given.
const DECLARED_SEA: Census = (WATER_VOXELS, OPEN_SIDES, WATERY_SIDES, OTHER_SIDES);

/// What one enumeration and its sides count up to.
fn counted(water: &Cells, sides: &Sides) -> Census {
    (water.len(), sides.open, sides.watery, sides.other)
}

/// A cell of the world, as the census names one.
///
/// Its own tuple alias rather than the walk's `Voxel`: the two describe the same
/// thing and must not share the type that decides what "the same cell" means.
type Cell = (u32, u32, u32);

/// A set of cells, which is what an enumeration of the sea comes to.
type Cells = BTreeSet<Cell>;

/// How the census classifies the six sides of every water voxel.
///
/// Three counts and no fourth: a side reaches the air or the world's edge, or
/// another water voxel, or some other named block. Nothing else is reachable,
/// which is what makes the sum a check.
#[derive(Debug, Default, PartialEq, Eq)]
struct Sides {
    open: usize,
    watery: usize,
    other: usize,
}

/// Every cell the declared sea fills, from the surface heights and the declared
/// sea level alone.
///
/// The rule, stated: water fills a submerged column from one block above its own
/// surface up to the sea level, and no column that is not submerged holds any.
/// **No cell contents are read** — this is the declaration's own prediction of
/// where the sea is, which is what makes comparing it against a scan of the
/// world worth doing.
fn water_the_heightmap_implies(world: &ReplayWorld) -> Result<Cells, Box<dyn Error>> {
    let mut implied = BTreeSet::new();
    for (x, z) in every_column() {
        implied.extend(sea_the_heightmap_puts_over(world, x, z)?);
    }
    Ok(implied)
}

/// The cells the sea fills in one column, which is none at all unless that
/// column's surface stands below the declared sea level.
fn sea_the_heightmap_puts_over(
    world: &ReplayWorld,
    x: u32,
    z: u32,
) -> Result<Vec<Cell>, Box<dyn Error>> {
    let surface = surface_height(world, x, z)?;
    if surface >= SEA_LEVEL {
        return Ok(Vec::new());
    }
    Ok(((surface + 1)..=SEA_LEVEL).map(|y| (x, y, z)).collect())
}

/// Every cell of the whole world that actually holds water.
///
/// All `FOOTPRINT × COLUMN_HEIGHT × FOOTPRINT` of them, and deliberately not the
/// sea's bounding volume: the question this answers is whether the rule above
/// misses water the generator placed, and a scan narrowed to where the rule says
/// water is could not answer it.
fn every_water_voxel_in_the_world(world: &ReplayWorld) -> Result<Cells, Box<dyn Error>> {
    let mut found = BTreeSet::new();
    for (x, z) in every_column() {
        found.extend(water_held_anywhere_in_column(world, x, z)?);
    }
    Ok(found)
}

/// Every cell of one column, top to bottom, that holds water — asked of the
/// world rather than predicted, which is the whole point of this half.
fn water_held_anywhere_in_column(
    world: &ReplayWorld,
    x: u32,
    z: u32,
) -> Result<Vec<Cell>, Box<dyn Error>> {
    let mut found = Vec::new();
    for y in 0..COLUMN_HEIGHT {
        if block_at(world, x, y, z)? == WATER {
            found.push((x, y, z));
        }
    }
    Ok(found)
}

/// How the six sides of every voxel in `water` are classified.
///
/// **Its own six offsets and its own bounds**, written out here rather than
/// taken from the walk's, so that a sign inversion or a swapped axis in either
/// one is a disagreement rather than a shared mistake.
fn sides_of(world: &ReplayWorld, water: &Cells) -> Result<Sides, Box<dyn Error>> {
    const STEPS: [(i64, i64, i64); 6] = [
        (-1, 0, 0),
        (1, 0, 0),
        (0, -1, 0),
        (0, 1, 0),
        (0, 0, -1),
        (0, 0, 1),
    ];
    let mut sides = Sides::default();
    for (x, y, z) in water {
        for (along_x, along_y, along_z) in STEPS {
            let stepped = (
                i64::from(*x) + along_x,
                i64::from(*y) + along_y,
                i64::from(*z) + along_z,
            );
            match beyond(world, stepped)? {
                None => sides.open += 1,
                Some(held) if held == NOTHING => sides.open += 1,
                Some(held) if held == WATER => sides.watery += 1,
                Some(_) => sides.other += 1,
            }
        }
    }
    Ok(sides)
}

/// What a stepped position holds, or nothing where it left the world.
fn beyond(world: &ReplayWorld, stepped: (i64, i64, i64)) -> Result<Option<String>, Box<dyn Error>> {
    let (x, y, z) = stepped;
    let inside = (0..i64::from(FOOTPRINT)).contains(&x)
        && (0..i64::from(COLUMN_HEIGHT)).contains(&y)
        && (0..i64::from(FOOTPRINT)).contains(&z);
    if !inside {
        return Ok(None);
    }
    Ok(Some(block_at(world, x as u32, y as u32, z as u32)?))
}

/// The first cell one enumeration names that the other does not, named.
fn first_absentee(implied: &Cells, found: &Cells) -> String {
    if let Some(cell) = found.difference(implied).next() {
        return format!("the scan found water at {cell:?} that the heightmap rule does not imply");
    }
    match implied.difference(found).next() {
        Some(cell) => {
            format!("the heightmap rule implies water at {cell:?} that the world does not hold")
        }
        None => "the two enumerations name the same cells".to_owned(),
    }
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
        if held != block {
            faults.push(format!("({x}, {y}, {z}) holds `{held}`, not `{block}`"));
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
