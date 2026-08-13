//! What a registry makes of a save, decided before a single chunk is read.
//!
//! This is what the whole name table is for. Resolving it once, up front, is
//! what lets a load report **every** block that is missing or changed in one
//! answer, instead of failing on whichever section happens to mention a removed
//! mod first — the difference between a player putting back what is needed in
//! one step and discovering it one failed load at a time.
//!
//! The verdict is reached by a pure function over a save's requirements and a
//! registry: no path, no file, no chunk data. That is why the report a user
//! interface will one day render is testable here with nothing but those two
//! values, and it is why the last test in this file can cut a save's chunk data
//! away entirely and still expect a complete answer.
//!
//! **Fixture constraint no assertion can enforce.** Every fixture below builds
//! the save's registry and the loading registry from the same builder, so two
//! registries naming the same block declare it *identically*. That is what makes
//! "missing" the only thing separating them: a fixture that varied solidity
//! between the two would legitimately report the shared block as changed, the
//! test would fail, and the failure would read as a resolver bug rather than as
//! the fixture saying something it did not mean. Where a fixture *does* mean to
//! change a declaration, it says so at the builder.
//!
//! Two of these carry the group's weight in opposite directions. The four-block
//! test is its **positive control**: a resolver that reported everything missing
//! satisfies every other test here and fails only there. And the near-miss test
//! is its **discrimination test**: a comparison that matched on a prefix, or
//! ignored the namespace, passes every other test here and fails only there.

mod common;

use std::error::Error;

use common::persistence::{STANDING_SOMEWHERE, saved_requirements, world_at, world_holding};
use common::{TestResult, registry_declaring, registry_of};
use mc_core::block::BlockRegistry;
use mc_core::id::BlockName;
use mc_world::persistence::{
    self, Acceptance, LoadError, RegistryVerdict, SaveRequirements, requirements,
};
use mc_world::world::WorldPos;
use std::fs;
use tempfile::TempDir;

/// Three blocks a save names, written in the order they sort in.
const THREE: [&str; 3] = ["fixture:andesite", "fixture:basalt", "fixture:chert"];

/// Four blocks a save names, for the control.
const FOUR: [&str; 4] = [
    "fixture:andesite",
    "fixture:basalt",
    "fixture:chert",
    "fixture:diorite",
];

/// The one of [`THREE`] the shrunken registry keeps.
const STILL_REGISTERED: [&str; 1] = ["fixture:andesite"];

/// The two of [`THREE`] it does not.
const NO_LONGER_REGISTERED: [&str; 2] = ["fixture:basalt", "fixture:chert"];

/// The name a save asks for, and the two names a registry holds instead.
///
/// One shares the path under another namespace and one shares the namespace with
/// a longer path, so a comparison that matched on either half alone — or on a
/// prefix — finds a block here and reports nothing missing.
const ASKED_FOR: [&str; 1] = ["fixture:stone"];
const NEAR_MISSES: [&str; 2] = ["other:stone", "fixture:stones"];

/// The two blocks whose declared behaviour changes between writing and reading.
const REDECLARED: [&str; 2] = ["fixture:andesite", "fixture:basalt"];

/// Where the blocks of a fixture world sit.
const CELLS: [WorldPos; 4] = [
    world_at(1, 1, 1),
    world_at(2, 3, 4),
    world_at(5, 8, 13),
    world_at(15, 200, 15),
];

/// How much of a save is kept when its chunk data is cut away.
///
/// Derived from the format rather than sampled: thirty bytes of preamble and a
/// table of two short names with two recorded declarations each come to well
/// under a hundred, so this keeps the whole table and none of the world. The
/// assertion carries the other half — that a whole save is longer than this — as
/// something it observes.
const KEPT_WITHOUT_THE_CHUNK_DATA: usize = 1024;

/// `names` as block names, in the order given.
fn block_names(names: &[&str]) -> Result<Vec<BlockName>, Box<dyn Error>> {
    let mut parsed = Vec::with_capacity(names.len());
    for name in names {
        parsed.push(BlockName::parse(name)?);
    }
    Ok(parsed)
}

/// What a save of a world holding `held` says it needs.
///
/// Written against a registry declaring exactly those blocks, because a writer
/// refuses a world holding a name the registry it is saved against does not
/// declare — the save has to be written by somebody who had the mod.
fn a_save_naming(directory: &TempDir, held: &[&str]) -> Result<SaveRequirements, Box<dyn Error>> {
    let registry = registry_of(held)?;
    let placed: Vec<(WorldPos, &str)> = CELLS.into_iter().zip(held.iter().copied()).collect();
    let world = world_holding(&placed, &registry)?;
    saved_requirements(directory, &world, &registry)
}

/// A registry declaring each of `names` with the solidity given, and nothing
/// else.
///
/// The declaration is what a save records, so flipping solidity is how a fixture
/// says "this block is not what it was" without touching its name or its texture
/// — a change of behaviour and not of appearance.
fn registry_declaring_all(names: &[&str], solid: bool) -> Result<BlockRegistry, Box<dyn Error>> {
    let declared: Vec<(&str, bool)> = names.iter().map(|name| (*name, solid)).collect();
    registry_declaring(&declared)
}

/// What a save of a world holding `held`, written against `registry`, says it
/// needs once its chunk data has been cut away — and how long the whole save was
/// before the cut.
///
/// The length comes back because the cut only means something while the save was
/// longer than it: a cut that removed nothing would leave a whole save behind
/// and the scenario would be asserting nothing at all.
fn a_save_without_its_chunk_data(
    directory: &TempDir,
    held: &[&str],
    registry: &BlockRegistry,
) -> Result<(SaveRequirements, usize), Box<dyn Error>> {
    let placed: Vec<(WorldPos, &str)> = CELLS.into_iter().zip(held.iter().copied()).collect();
    let world = world_holding(&placed, registry)?;
    let path = directory.path().join("world.mcw");
    persistence::save_world(&path, &world, STANDING_SOMEWHERE, registry)?;
    let whole = fs::read(&path)?;
    let table_only = directory.path().join("table_only.mcw");
    fs::write(
        &table_only,
        whole.get(..KEPT_WITHOUT_THE_CHUNK_DATA).unwrap_or(&whole),
    )?;
    Ok((requirements(&table_only)?, whole.len()))
}

#[test]
fn a_save_naming_three_blocks_no_registry_holds_is_refused_naming_all_three() -> TestResult {
    let directory = TempDir::new()?;
    let required = a_save_naming(&directory, &THREE)?;

    let verdict = persistence::resolve(&required, &BlockRegistry::new());

    assert_eq!(
        verdict.refusal(Acceptance::OnlyUnchangedBlocks),
        Some(LoadError::Unresolvable {
            missing: block_names(&THREE)?,
            changed: Vec::new()
        }),
        "all three at once is the requirement, not one of the three: a load that failed on \
         whichever block it met first would send a player away to reinstall one mod, relaunch, \
         and be sent away again — three times over, for a fact the save could state completely \
         before it read a single chunk. Nothing can go in a cell whose block nobody has, so this \
         refusal is not a decision the player is in a position to make"
    );
    Ok(())
}

#[test]
fn a_save_naming_one_registered_and_two_unregistered_blocks_names_exactly_the_two() -> TestResult {
    let directory = TempDir::new()?;
    let required = a_save_naming(&directory, &THREE)?;
    let shrunken = registry_of(&STILL_REGISTERED)?;

    let verdict = persistence::resolve(&required, &shrunken);

    assert_eq!(
        verdict.refusal(Acceptance::OnlyUnchangedBlocks),
        Some(LoadError::Unresolvable {
            missing: block_names(&NO_LONGER_REGISTERED)?,
            changed: Vec::new()
        }),
        "exactly the two, which is both halves of the claim: a report short of one leaves a \
         player one failed load from finishing, and a report naming the block they still have \
         sends them looking for a mod that is already installed. The block both registries hold \
         is declared identically in each, so it is not merely present — it is unchanged, and the \
         changed list stays empty because of it"
    );
    Ok(())
}

#[test]
fn a_save_whose_four_blocks_are_all_registered_and_unchanged_reports_nothing() -> TestResult {
    let directory = TempDir::new()?;
    let required = a_save_naming(&directory, &FOUR)?;
    let registry = registry_of(&FOUR)?;

    let verdict = persistence::resolve(&required, &registry);

    assert_eq!(
        (
            verdict.clone(),
            verdict.refusal(Acceptance::OnlyUnchangedBlocks)
        ),
        (RegistryVerdict::default(), None),
        "this is the control the rest of the group cannot do without. A resolver that reported \
         every name as missing satisfies every other test in this file — each of those asks only \
         that certain names appear — and fails here alone. The ordinary case is a player \
         relaunching with the mods they quit with, and it has to be silent: a load that asked \
         about four blocks nobody touched would teach them to dismiss the question"
    );
    Ok(())
}

#[test]
fn a_save_naming_a_block_two_near_misses_do_not_provide_is_refused_by_that_name() -> TestResult {
    let directory = TempDir::new()?;
    let required = a_save_naming(&directory, &ASKED_FOR)?;
    let nearly = registry_of(&NEAR_MISSES)?;

    let verdict = persistence::resolve(&required, &nearly);

    assert_eq!(
        verdict.refusal(Acceptance::OnlyUnchangedBlocks),
        Some(LoadError::Unresolvable {
            missing: block_names(&ASKED_FOR)?,
            changed: Vec::new()
        }),
        "a name is the whole identity a save stores, so it has to match whole. One of the two \
         blocks on offer shares the path under a different namespace and the other shares the \
         namespace with a longer path — a comparison matching on a prefix, or on either half \
         alone, resolves this save against somebody else's block and passes every other test \
         here while doing it"
    );
    Ok(())
}

#[test]
fn a_save_with_its_chunk_data_cut_away_still_reports_both_blocks_as_changed() -> TestResult {
    let directory = TempDir::new()?;
    let written_against = registry_declaring_all(&REDECLARED, true)?;
    let (required, whole_length) =
        a_save_without_its_chunk_data(&directory, &REDECLARED, &written_against)?;
    let redeclared = registry_declaring_all(&REDECLARED, false)?;

    let verdict = persistence::resolve(&required, &redeclared);

    assert_eq!(
        (KEPT_WITHOUT_THE_CHUNK_DATA < whole_length, verdict.changed),
        (true, block_names(&REDECLARED)?),
        "the file that answered here has no chunk data left in it at all, so a resolver that \
         reached the declarations by way of the world could not have answered — which is what \
         makes `without reading any of its chunk data` a property observed rather than a comment. \
         It is also the value a dialog is later built over: the complete list of what has changed, \
         available before anything is decided and before anything is loaded"
    );
    Ok(())
}
