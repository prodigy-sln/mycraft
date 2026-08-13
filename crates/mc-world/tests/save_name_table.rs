//! What ends up in a save's table of names: one entry per distinct block the
//! world actually holds, written as the namespaced name and nothing else.
//!
//! Two of the four fixtures here are about size and two are about shape.
//!
//! The size ones bound the file-local identifier from below. A save-wide table
//! is bounded by the distinct names across the whole save, which is a different
//! bound entirely from a section palette's — and mis-sizing it is the single
//! highest-risk mistake persistence can make. 4097 is here to assert the table's
//! *count* directly, so that the fixture a later phase builds at 65 537 cannot
//! quietly contain forty names instead of the number it claims. A count cannot
//! see shape, so something has to see it first.
//!
//! The shape ones are about what a name is and what does not deserve one. A
//! block's identity in a save is its namespaced name, because a runtime id is
//! reassigned the moment the block set changes and a palette position means
//! nothing outside the section that minted it. And an entry no voxel refers to
//! is not something the world holds: it is a palette's book-keeping, kept
//! because reclaiming it on the edit path would put work into a tick loop for a
//! benefit only persistence collects. The save is where that debt is paid.

mod common;

use std::collections::BTreeSet;

use common::persistence::{
    FIXTURE_FOOTPRINT, required_names, saved_requirements, world_at, world_holding,
};
use common::{TestResult, generated_block_name, registry_of, registry_of_size};
use mc_core::id::BlockName;
use mc_world::world::{VoxelWorld, WorldPos};
use tempfile::TempDir;

/// The block a save must name in full, and three that make a partial match
/// plausible: one sharing its path under another namespace, one whose path is
/// its own with a letter added, and one unrelated.
const NAMED_IN_FULL: &str = "fixture:andesite";
const NEAR_MISSES: [&str; 3] = ["other:andesite", "fixture:andesites", "fixture:basalt"];

/// Where the one block of the naming fixture sits.
const A_SINGLE_CELL: WorldPos = world_at(4, 40, 4);

/// How many columns a side the single-block world spans, and the block filling
/// every one of its cells.
const COLUMNS_A_SIDE: u32 = 4;
const FILL: &str = "fixture:stone";

/// The three blocks the palette of the vacated-entry fixture ends up holding,
/// and the one of them any voxel still refers to.
const KEPT: &str = "fixture:alpha";
const OVERWRITTEN: [&str; 2] = ["fixture:beta", "fixture:gamma"];

/// The cell the vacated-entry fixture writes to three times over.
const A_REVISITED_CELL: WorldPos = world_at(0, 0, 0);

/// How many distinct blocks the large fixture holds.
///
/// One past a compacted section's 4096, so it cannot be held in a single
/// section and the table is genuinely a save-wide one rather than a section's
/// palette under another name.
const DISTINCT_BLOCKS: u32 = 4097;

/// Where the `nth` block of the large fixture goes: x fastest, then z, then the
/// height, so the first 4096 fill one section exactly and the 4097th starts the
/// next one.
///
/// Shifts rather than divisions: `clippy::integer_division` is a gate error and
/// it applies to test targets too.
const fn nth_cell(nth: u32) -> WorldPos {
    world_at(nth & 15, nth >> 8, (nth >> 4) & 15)
}

#[test]
fn a_saved_block_is_needed_by_its_whole_namespaced_name() -> TestResult {
    let directory = TempDir::new()?;
    let mut declared: Vec<&str> = NEAR_MISSES.to_vec();
    declared.push(NAMED_IN_FULL);
    let registry = registry_of(&declared)?;
    let world = world_holding(&[(A_SINGLE_CELL, NAMED_IN_FULL)], &registry)?;

    let required = saved_requirements(&directory, &world, &registry)?;

    assert_eq!(
        required_names(&required),
        vec![NAMED_IN_FULL.to_owned()],
        "a block's identity in a save is the name it was registered under, written out whole. \
         The registry beside it holds the same path in another namespace and the same namespace \
         with a longer path, so a save recording anything less than the full name — a bare path, \
         a prefix, the runtime id 3 this block happens to have here — resolves to the wrong \
         block or to none"
    );
    Ok(())
}

#[test]
fn sixteen_columns_holding_only_one_block_need_exactly_one_name() -> TestResult {
    let directory = TempDir::new()?;
    let registry = registry_of(&[FILL])?;
    let world = VoxelWorld::filled(COLUMNS_A_SIDE, &BlockName::parse(FILL)?, &registry)?;

    let required = saved_requirements(&directory, &world, &registry)?;

    assert_eq!(
        required_names(&required),
        vec![FILL.to_owned()],
        "a million cells hold the same block, and the table holds it once. The whole reason the \
         names live in a table addressed by a file-local identifier, rather than beside every \
         cell, is that a world repeats itself — and one entry per distinct name is what makes \
         the missing-block report a list a player can read"
    );
    Ok(())
}

#[test]
fn a_world_of_4097_distinct_blocks_needs_4097_names() -> TestResult {
    let directory = TempDir::new()?;
    let registry = registry_of_size(DISTINCT_BLOCKS)?;
    let placed: Vec<(WorldPos, String)> = (0..DISTINCT_BLOCKS)
        .map(|nth| (nth_cell(nth), generated_block_name(nth)))
        .collect();
    let borrowed: Vec<(WorldPos, &str)> = placed
        .iter()
        .map(|(at, name)| (*at, name.as_str()))
        .collect();
    let world = world_holding(&borrowed, &registry)?;

    let required = saved_requirements(&directory, &world, &registry)?;

    let reported: BTreeSet<String> = required_names(&required).into_iter().collect();
    let expected: BTreeSet<String> = (0..DISTINCT_BLOCKS).map(generated_block_name).collect();
    assert_eq!(
        (reported.len(), reported == expected),
        (DISTINCT_BLOCKS as usize, true),
        "one past what a compacted section's palette can address, so these 4097 names cannot all \
         live in one section and the table is measured as the save-wide thing it is. The count is \
         asserted here, on a fixture small enough to enumerate, precisely so that the far larger \
         fixture the identifier's width is decided by cannot pass while holding a fraction of \
         the names it claims"
    );
    Ok(())
}

#[test]
fn a_palette_entry_no_voxel_refers_to_any_more_is_not_a_name_the_save_needs() -> TestResult {
    let directory = TempDir::new()?;
    let mut declared: Vec<&str> = vec![KEPT];
    declared.extend_from_slice(&OVERWRITTEN);
    let registry = registry_of(&declared)?;
    let mut world = VoxelWorld::filled(FIXTURE_FOOTPRINT, &BlockName::parse(KEPT)?, &registry)?;
    for name in OVERWRITTEN {
        world.set_block(A_REVISITED_CELL, &BlockName::parse(name)?, &registry)?;
    }
    world.set_block(A_REVISITED_CELL, &BlockName::parse(KEPT)?, &registry)?;

    let required = saved_requirements(&directory, &world, &registry)?;

    assert_eq!(
        required_names(&required),
        vec![KEPT.to_owned()],
        "one cell was written three times over, so the section's palette holds three entries and \
         exactly one of them is referred to by any voxel. A save carrying all three would need \
         two blocks nothing in the world is made of — and the day one of those mods is uninstalled \
         the world refuses to load over a block that is not in it"
    );
    Ok(())
}
