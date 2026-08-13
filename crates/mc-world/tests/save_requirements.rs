//! Asking a save what it needs, before anything is loaded.
//!
//! This is the capability the whole name table exists for. Resolving the table
//! once, up front, is what lets a load report *every* missing block before a
//! chunk is touched, instead of failing on whichever section happens to
//! reference a removed mod first. It is also what makes the table observable to
//! a caller at all: without it, most of what this phase writes could only be
//! asserted by reading file bytes.
//!
//! "Without reading any of its chunk data" is asserted rather than commented. A
//! save whose chunk data has been cut away entirely still answers the question
//! completely — which a reader that decoded the world first could not do.
//!
//! The last scenario is the one that keeps the rest honest. A table that cannot
//! be read has to come back as a refusal, because an empty answer from a corrupt
//! save is indistinguishable from a save that genuinely needs nothing — the same
//! way a reviewer who returned nothing is indistinguishable from a reviewer who
//! found nothing. What it must *not* do is name the decoder's complaint: which
//! way a library refused arbitrary bytes is not part of anything this crate
//! promises.

mod common;

use std::fs;

use common::persistence::{
    STANDING_SOMEWHERE, declared_names, required_names, save_in, saved_requirements, world_at,
    world_holding,
};
use common::{TestResult, registry_of};
use mc_core::block::BlockRegistry;
use mc_world::persistence::{self, LoadError};
use mc_world::world::{VoxelWorld, WorldPos};
use tempfile::TempDir;

/// The four blocks the worlds below hold.
const HELD: [&str; 4] = [
    "fixture:andesite",
    "fixture:basalt",
    "fixture:chert",
    "fixture:diorite",
];

/// Five further blocks a registry holds and no world here puts anywhere.
///
/// Declared *ahead* of [`HELD`], so every block a world holds sits at a runtime
/// id of 5 or more — a save that reported what the registry held, or that stored
/// a runtime id, would be visible as a wrong answer rather than as a coincidence.
const UNPLACED: [&str; 5] = [
    "fixture:felsite",
    "fixture:gabbro",
    "fixture:hornfels",
    "fixture:ironstone",
    "fixture:jasper",
];

/// Where each of [`HELD`] sits.
const CELLS: [WorldPos; 4] = [
    world_at(1, 1, 1),
    world_at(2, 3, 4),
    world_at(5, 8, 13),
    world_at(15, 200, 15),
];

/// How much of a save is kept when the chunk data is cut away.
///
/// Derived from the format rather than sampled: thirty bytes of preamble and a
/// table of four short names with two recorded declarations each come to under
/// 200 bytes, so this keeps the whole table and none of the world. The assertion
/// carries the other half — that a whole save is longer than this — as something
/// it observes rather than as arithmetic to be trusted.
const KEPT_WITHOUT_THE_CHUNK_DATA: usize = 1024;

/// How much of a save is kept when its table is cut off part-way through.
///
/// Two bytes past the thirty-byte preamble: enough for the table to declare how
/// many names it holds and nothing like enough for it to hold them. It has to
/// stay past the preamble, or the file stops being a save with an unreadable
/// table and becomes one that is too short to be a save at all — a different
/// refusal, about the format rather than about the question being asked.
const KEPT_WITHOUT_A_WHOLE_TABLE: usize = 32;

/// Every block a registry declares for these fixtures: the unplaced ones first,
/// so nothing a world holds is registered early.
fn every_declared_block() -> Vec<&'static str> {
    let mut names = UNPLACED.to_vec();
    names.extend_from_slice(&HELD);
    names
}

/// Each of [`HELD`] at its cell.
fn every_block_placed() -> Vec<(WorldPos, &'static str)> {
    CELLS.into_iter().zip(HELD).collect()
}

/// A world holding each of [`HELD`] once, and the registry it was built against.
fn a_world_of_four_blocks() -> Result<(VoxelWorld, BlockRegistry), Box<dyn std::error::Error>> {
    let registry = registry_of(&every_declared_block())?;
    let world = world_holding(&every_block_placed(), &registry)?;
    Ok((world, registry))
}

#[test]
fn a_world_holding_four_blocks_is_saved_needing_exactly_those_four_names() -> TestResult {
    let directory = TempDir::new()?;
    let (world, registry) = a_world_of_four_blocks()?;

    let required = saved_requirements(&directory, &world, &registry)?;

    assert_eq!(
        required_names(&required),
        HELD.map(str::to_owned).to_vec(),
        "the registry declares nine blocks and the world holds four of them, at runtime ids five \
         through eight. What the save needs is those four names and nothing else — not what the \
         registry happened to hold, and nothing carrying which registry resolved it or in what \
         order that registry was built"
    );
    Ok(())
}

#[test]
fn a_world_whose_every_cell_holds_nothing_is_saved_against_an_empty_registry_needing_no_name()
-> TestResult {
    let directory = TempDir::new()?;
    let world = VoxelWorld::empty(1);

    let required = saved_requirements(&directory, &world, &BlockRegistry::new())?;

    assert_eq!(
        required_names(&required),
        Vec::<String>::new(),
        "a world of nothing names no block, so there is nothing for a registry to hold — and a \
         registry holding nothing at all is enough to read it back. Nothing is never a table \
         entry: giving emptiness a name at the one place a stored format makes it permanent \
         would put it in every missing-block report ever written"
    );
    Ok(())
}

#[test]
fn a_save_with_its_chunk_data_cut_away_still_reports_every_name_it_needs() -> TestResult {
    let directory = TempDir::new()?;
    let (world, registry) = a_world_of_four_blocks()?;
    let path = save_in(&directory);
    persistence::save_world(&path, &world, STANDING_SOMEWHERE, &registry)?;
    let whole = fs::read(&path)?;
    let table_only = directory.path().join("table_only.mcw");
    fs::write(
        &table_only,
        whole.get(..KEPT_WITHOUT_THE_CHUNK_DATA).unwrap_or(&whole),
    )?;

    let required = persistence::requirements(&table_only)?;

    assert_eq!(
        (
            KEPT_WITHOUT_THE_CHUNK_DATA < whole.len(),
            required_names(&required)
        ),
        (true, HELD.map(str::to_owned).to_vec()),
        "the file that answered here has no chunk data left in it at all, so a reader that got \
         to the names by way of the world could not have answered at all — which is what makes \
         `without reading any of its chunk data` a property this suite observes rather than a \
         sentence in a comment. It is also what the whole table is for: the complete list of \
         what a save needs, before a single section is touched"
    );
    Ok(())
}

#[test]
fn asking_a_save_of_four_blocks_what_it_needs_names_exactly_those_four() -> TestResult {
    let directory = TempDir::new()?;
    let registry = registry_of(&HELD)?;
    let world = world_holding(&every_block_placed(), &registry)?;

    let required = saved_requirements(&directory, &world, &registry)?;

    assert_eq!(
        declared_names(&required),
        HELD.map(str::to_owned).to_vec(),
        "read off the recorded declarations rather than off the names, because the two halves of \
         the answer are what a caller acts on together — a report whose names were complete and \
         whose declarations were short by one would look right through the names alone"
    );
    Ok(())
}

#[test]
fn a_save_of_a_world_holding_no_block_needs_no_name_at_all() -> TestResult {
    let directory = TempDir::new()?;
    let registry = registry_of(&every_declared_block())?;
    let world = VoxelWorld::empty(1);

    let required = saved_requirements(&directory, &world, &registry)?;

    assert_eq!(
        required_names(&required),
        Vec::<String>::new(),
        "nine blocks are registered and the world puts none of them anywhere, so the save needs \
         none of them. A writer that recorded the registry instead of the world would answer \
         with nine names here and would be right about every other fixture in this file"
    );
    Ok(())
}

#[test]
fn a_save_whose_table_cannot_be_read_refuses_the_question_rather_than_answering_it() -> TestResult {
    let directory = TempDir::new()?;
    let (world, registry) = a_world_of_four_blocks()?;
    let path = save_in(&directory);
    persistence::save_world(&path, &world, STANDING_SOMEWHERE, &registry)?;
    let whole = fs::read(&path)?;
    let cut_off_mid_table = directory.path().join("cut_off.mcw");
    fs::write(
        &cut_off_mid_table,
        whole.get(..KEPT_WITHOUT_A_WHOLE_TABLE).unwrap_or_default(),
    )?;

    let asked = persistence::requirements(&cut_off_mid_table);

    assert_eq!(
        asked,
        Err(LoadError::Malformed {
            path: cut_off_mid_table.clone()
        }),
        "the table says it holds names and the bytes that would hold them are gone, so the \
         question has no answer — and the one answer that must never come back is an empty set, \
         which a caller cannot tell apart from a save that genuinely needs nothing. What the \
         refusal must not do is quote the decoder: which way a library declined arbitrary bytes \
         is not something this crate promises anybody"
    );
    Ok(())
}
