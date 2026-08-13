//! The identifier a save numbers its names by is the file's, and it does not
//! survive being read.
//!
//! A save's table is a name against a position local to that one file. Nothing
//! outside the file may depend on which position a name landed at: not the
//! registry it is read against, not the order that registry was built in, and
//! not the runtime ids it hands out. What a cell holds is decided by the name
//! the table carries and by nothing else.
//!
//! **The three fixtures below attack that from three sides.** One save read
//! twice against two registration orders must produce the same world; a world
//! that has been read and written again must store the same bytes, so that the
//! identifier survives a whole trip through memory unchanged; and a registry
//! that registers four further blocks *ahead* of a save's two must not shift
//! what the save's cells hold.
//!
//! That last one is here in place of a scan asserting the identifier never
//! reaches the protocol crate. `mc-proto` is an empty stub today, so such a scan
//! would be green because there is nothing there, and would stay green forever —
//! an absent reviewer and a clean reviewer look identical. Extra registrations
//! ahead of a save's own names is the same property made consequential.
//!
//! **Fixture constraint no assertion can enforce.** The two registries a save is
//! read against declare the *same definitions* and differ in order alone. A
//! fixture varying a block's solidity between them would be describing a changed
//! block, which is a refusal about declarations rather than anything about the
//! identifier, and the failure would read as a resolver bug.

mod common;

use common::persistence::{
    AGREES, STANDING_SOMEWHERE, answer_at, disagreement, loaded_from, save_in, stored_data,
    world_at, world_holding,
};
use common::{TestResult, registry_of};
use mc_core::block::BlockRegistry;
use mc_core::id::BlockName;
use mc_world::persistence::{self, Acceptance};
use mc_world::world::{VoxelWorld, WorldPos};
use tempfile::TempDir;

/// The four blocks the reordering fixtures name, and the block their world is
/// otherwise made of.
const NAMED: [&str; 4] = [
    "fixture:andesite",
    "fixture:basalt",
    "fixture:chert",
    "fixture:dacite",
];
const BASE: &str = "fixture:andesite";

/// Where each of the four blocks sits.
const CELLS: [WorldPos; 4] = [
    world_at(1, 1, 1),
    world_at(2, 30, 3),
    world_at(4, 100, 5),
    world_at(6, 200, 7),
];

/// The two blocks a save of its own names, and the four a loading registry
/// declares ahead of them.
const ITS_OWN: [&str; 2] = ["fixture:quartz", "fixture:rhyolite"];
const REGISTERED_FIRST: [&str; 4] = [
    "fixture:ash",
    "fixture:basalt",
    "fixture:chalk",
    "fixture:diorite",
];

/// Where the two blocks of that save sit.
const TWO_CELLS: [WorldPos; 2] = [world_at(3, 3, 3), world_at(12, 200, 14)];

/// A world of one column made of `BASE`, with each of `NAMED` written into it.
fn a_world_of_four_blocks() -> Result<(VoxelWorld, BlockRegistry), Box<dyn std::error::Error>> {
    let registry = registry_of(&NAMED)?;
    let mut world = VoxelWorld::filled(1, &BlockName::parse(BASE)?, &registry)?;
    for (at, name) in CELLS.into_iter().zip(NAMED) {
        world.set_block(at, &BlockName::parse(name)?, &registry)?;
    }
    Ok((world, registry))
}

#[test]
fn one_save_read_against_two_registration_orders_holds_the_same_contents_both_times() -> TestResult
{
    let directory = TempDir::new()?;
    let (world, registry) = a_world_of_four_blocks()?;
    let path = save_in(&directory);
    persistence::save_world(&path, &world, STANDING_SOMEWHERE, &registry)?;
    let mut reversed: Vec<&str> = NAMED.to_vec();
    reversed.reverse();

    let one_way = loaded_from(
        &path,
        &registry_of(&NAMED)?,
        Acceptance::OnlyUnchangedBlocks,
    )?;
    let the_other = loaded_from(
        &path,
        &registry_of(&reversed)?,
        Acceptance::OnlyUnchangedBlocks,
    )?;

    assert_eq!(
        (
            disagreement(&one_way, &the_other)?,
            CELLS.map(|at| answer_at(&one_way, at)).to_vec()
        ),
        (AGREES.to_owned(), NAMED.map(str::to_owned).to_vec()),
        "the same file, read twice, against two registries that differ in nothing but the order \
         they were built in. A load that resolved a stored number through the registry — a runtime \
         id, a registration position — would produce two different worlds here, each one \
         internally consistent and one of them wrong. The four cells are the control: two empty \
         worlds agree with each other perfectly, so agreement alone says nothing until something \
         says the worlds have blocks in them"
    );
    Ok(())
}

#[test]
fn a_world_saved_again_after_being_read_stores_the_bytes_it_was_read_from() -> TestResult {
    let directory = TempDir::new()?;
    let (world, registry) = a_world_of_four_blocks()?;
    let first = stored_data(&directory, "first.mcw", &world, &registry)?;

    let read_back = loaded_from(
        &directory.path().join("first.mcw"),
        &registry,
        Acceptance::OnlyUnchangedBlocks,
    )?;
    let second = stored_data(&directory, "second.mcw", &read_back, &registry)?;

    assert_eq!(
        (first == second, first.is_empty()),
        (true, false),
        "a save read into memory and written straight back out has to be the same save. The \
         identifier is minted from the names the world holds, so a world whose names arrived in \
         the order the file listed them — rather than in the order the table is written in — comes \
         back numbered differently and stores different bytes, while holding exactly the same \
         blocks. The second half is what stops a writer that emits nothing from passing: two empty \
         byte strings are equal"
    );
    Ok(())
}

#[test]
fn a_save_read_against_a_registry_holding_four_blocks_first_still_holds_its_own_two() -> TestResult
{
    let directory = TempDir::new()?;
    let written_against = registry_of(&ITS_OWN)?;
    let placed: Vec<(WorldPos, &str)> = TWO_CELLS.into_iter().zip(ITS_OWN).collect();
    let world = world_holding(&placed, &written_against)?;
    let path = save_in(&directory);
    persistence::save_world(&path, &world, STANDING_SOMEWHERE, &written_against)?;
    let mut declared: Vec<&str> = REGISTERED_FIRST.to_vec();
    declared.extend_from_slice(&ITS_OWN);

    let read_back = loaded_from(
        &path,
        &registry_of(&declared)?,
        Acceptance::OnlyUnchangedBlocks,
    )?;

    assert_eq!(
        TWO_CELLS.map(|at| answer_at(&read_back, at)).to_vec(),
        ITS_OWN.map(str::to_owned).to_vec(),
        "the save numbers its two names 0 and 1, and the registry it is read against numbers those \
         same two blocks 4 and 5 — because four other blocks were registered ahead of them. A load \
         that treated the file's number as a registry number puts the wrong block in both cells \
         and refuses nothing, which is the failure a stub crate's dependency graph could never \
         have caught"
    );
    Ok(())
}
