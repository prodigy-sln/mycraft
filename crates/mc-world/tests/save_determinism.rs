//! A save is a deterministic image of what the world holds, and of nothing
//! about the registry that happened to be loaded when it was written.
//!
//! Four of the six assertions here are byte *equalities*, and every one of them
//! is satisfied by a writer that emits a constant — so two of the six are
//! positive controls and the group is worthless without them. Two worlds
//! differing in one cell must produce different bytes, and two registries
//! differing in one block's declared solidity must too. The first falsifies a
//! writer that ignores the world; the second falsifies a definition hash that
//! returns the same number for everything, which would otherwise satisfy every
//! absence assertion in this file.
//!
//! **The equality fixtures carry a constraint no assertion can enforce.** Two
//! registries produce the same bytes only while they declare the *same
//! definitions* and differ in the order they declare them, because what a save
//! records is a block's declaration and a declaration is registry content rather
//! than registry order. A fixture that varied solidity between the two would
//! legitimately produce different bytes — and would read as a determinism bug in
//! the writer rather than as a mistake in the fixture. Each builder below says
//! which of the two it is doing.
//!
//! The comparand is the save's *stored world data* rather than the whole file,
//! so the requirement does not silently decide what the container is allowed to
//! carry of its own.

mod common;

use std::error::Error;

use common::persistence::{stored_data, world_at, world_holding};
use common::{FIXTURE_ORIGIN, TestResult, registry_from, registry_of};
use mc_core::block::BlockRegistry;
use mc_world::world::{VoxelWorld, WorldPos};
use tempfile::TempDir;

/// The four blocks the world of this file's equality fixtures is made of.
const BLOCKS: [&str; 4] = [
    "fixture:andesite",
    "fixture:basalt",
    "fixture:chert",
    "fixture:diorite",
];

/// The same four names, registered the other way round. **Identical
/// definitions, different registration order** — which is what an update that
/// adds or reorders content does, and the only difference these fixtures are
/// allowed to have.
const REVERSED: [&str; 4] = [
    "fixture:diorite",
    "fixture:chert",
    "fixture:basalt",
    "fixture:andesite",
];

/// Where each of [`BLOCKS`] sits.
const CELLS: [WorldPos; 4] = [
    world_at(1, 1, 1),
    world_at(2, 3, 4),
    world_at(5, 8, 13),
    world_at(15, 200, 15),
];

/// The block whose runtime id the two registries below disagree about, and the
/// three that push it to the back of one of them.
const MOVED: &str = "fixture:diorite";
const AHEAD_OF_IT: [&str; 3] = ["fixture:andesite", "fixture:basalt", "fixture:chert"];

/// The one block whose declared solidity the two registries of the last test
/// disagree about.
const REBALANCED: &str = "fixture:andesite";

/// The two cells the one-cell-apart worlds hold, and the two blocks they hold
/// between them.
///
/// Both worlds hold both blocks, so their name tables are identical and the only
/// thing left to differ is which block sits in the first cell. A fixture whose
/// edit introduced a name the other world lacked would be caught by the table
/// alone, and would say nothing about the stored cells.
const FIRST_CELL: WorldPos = world_at(1, 1, 1);
const SECOND_CELL: WorldPos = world_at(2, 2, 2);
const THIRD_CELL: WorldPos = world_at(3, 3, 3);
const ONE: &str = "fixture:andesite";
const OTHER: &str = "fixture:basalt";

/// Two saves of `world`, one against each registry, as the world data each
/// stored.
fn saved_against_each(
    world: &VoxelWorld,
    first: &BlockRegistry,
    second: &BlockRegistry,
) -> Result<(Vec<u8>, Vec<u8>), Box<dyn Error>> {
    let directory = TempDir::new()?;
    let one = stored_data(&directory, "first.mcw", world, first)?;
    let other = stored_data(&directory, "second.mcw", world, second)?;
    Ok((one, other))
}

/// Each of [`BLOCKS`] at its cell.
fn every_block_placed() -> Vec<(WorldPos, &'static str)> {
    CELLS.into_iter().zip(BLOCKS).collect()
}

/// Every block textured by its own name, with [`REBALANCED`] carrying
/// `is_solid` and the rest solid.
///
/// **One field differs between the two registries this builds and nothing else
/// does.** That is what makes a difference in the saved bytes attributable to
/// the declared solidity rather than to the fixture.
fn declared_with(is_solid: bool) -> Vec<(&'static str, &'static str, bool)> {
    BLOCKS
        .into_iter()
        .map(|name| (name, name, name != REBALANCED || is_solid))
        .collect()
}

#[test]
fn two_registries_declaring_the_same_blocks_in_different_orders_store_the_same_bytes() -> TestResult
{
    let world = world_holding(&every_block_placed(), &registry_of(&BLOCKS)?)?;

    let (one, other) =
        saved_against_each(&world, &registry_of(&BLOCKS)?, &registry_of(&REVERSED)?)?;

    assert_eq!(
        (one == other, one.is_empty()),
        (true, false),
        "the two registries declare the same four blocks and disagree about nothing but the \
         order they were declared in, which is the whole of what installing another mod does. A \
         world that meant something different afterwards would be a world rewritten by an \
         update nobody asked to change it"
    );
    Ok(())
}

#[test]
fn a_block_saved_at_one_runtime_id_and_then_at_another_stores_the_same_bytes() -> TestResult {
    let mut behind: Vec<&str> = AHEAD_OF_IT.to_vec();
    behind.push(MOVED);
    let mut in_front: Vec<&str> = vec![MOVED];
    in_front.extend_from_slice(&AHEAD_OF_IT);
    let world = world_holding(&[(FIRST_CELL, MOVED)], &registry_of(&behind)?)?;

    let (one, other) =
        saved_against_each(&world, &registry_of(&behind)?, &registry_of(&in_front)?)?;

    assert_eq!(
        (one == other, one.is_empty()),
        (true, false),
        "the same block is the fourth thing one registry registered and the first thing the \
         other did, so its runtime id is 3 in one and 0 in the other. An id is dense, \
         registry-local and reassigned the moment the block set changes — a save that stored one \
         would start reporting whichever block happened to be numbered the same after an update"
    );
    Ok(())
}

#[test]
fn saving_one_world_twice_in_one_process_stores_the_same_bytes() -> TestResult {
    let registry = registry_of(&BLOCKS)?;
    let world = world_holding(&every_block_placed(), &registry)?;
    let directory = TempDir::new()?;

    let one = stored_data(&directory, "first.mcw", &world, &registry)?;
    let other = stored_data(&directory, "second.mcw", &world, &registry)?;

    assert_eq!(
        (one == other, one.is_empty()),
        (true, false),
        "nothing changed between the two saves — same world, same registry, same process — so \
         anything that differs came from inside the writer. Two hash maps in one process iterate \
         in different orders, because each is seeded separately, and a single hash-ordered \
         iteration reaching the file is enough to make a save's bytes depend on nothing a player \
         did"
    );
    Ok(())
}

#[test]
fn two_worlds_differing_in_one_cell_store_different_bytes() -> TestResult {
    let registry = registry_of(&BLOCKS)?;
    let one_cell_apart = [(FIRST_CELL, OTHER), (SECOND_CELL, ONE), (THIRD_CELL, OTHER)];
    let world = world_holding(
        &[(FIRST_CELL, ONE), (SECOND_CELL, ONE), (THIRD_CELL, OTHER)],
        &registry,
    )?;
    let edited = world_holding(&one_cell_apart, &registry)?;
    let directory = TempDir::new()?;

    let one = stored_data(&directory, "first.mcw", &world, &registry)?;
    let other = stored_data(&directory, "second.mcw", &edited, &registry)?;

    assert!(
        one != other,
        "the control the three equalities above cannot do without: a writer that emits a \
         constant satisfies every one of them and fails only here. Both worlds hold both blocks, \
         so their name tables are the same and the single cell that differs is the only thing \
         left — which also rules out a writer that stores the table and forgets the cells"
    );
    Ok(())
}

#[test]
fn definitions_read_from_two_different_origins_store_the_same_bytes() -> TestResult {
    let declared: Vec<(&str, &str, bool)> =
        BLOCKS.into_iter().map(|name| (name, name, true)).collect();
    let world = world_holding(&every_block_placed(), &registry_of(&BLOCKS)?)?;

    let (one, other) = saved_against_each(
        &world,
        &registry_from(FIXTURE_ORIGIN, &declared)?,
        &registry_from("a content root somewhere else entirely", &declared)?,
    )?;

    assert_eq!(
        (one == other, one.is_empty()),
        (true, false),
        "an origin is a human-readable label derived from the file a definition was read out of, \
         so recording it would make a save written from a repository at one path refuse to load \
         from another — for a reason that has nothing to do with what the world is made of, and \
         with a refusal a player could not tell apart from corruption"
    );
    Ok(())
}

#[test]
fn one_block_declared_solid_and_then_not_stores_different_bytes() -> TestResult {
    let world = world_holding(&every_block_placed(), &registry_of(&BLOCKS)?)?;

    let (one, other) = saved_against_each(
        &world,
        &registry_from(FIXTURE_ORIGIN, &declared_with(true))?,
        &registry_from(FIXTURE_ORIGIN, &declared_with(false))?,
    )?;

    assert!(
        one != other,
        "the second control, one level down from the first: a definition hash that returns the \
         same number whatever it is given satisfies every absence assertion in this file and in \
         the declarations suite, and fails only here. A block that was solid and is not any more \
         is a world whose ground a player falls through, which is exactly the change the \
         recorded declaration exists to notice"
    );
    Ok(())
}
