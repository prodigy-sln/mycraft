//! The order a save's report comes back in, which is not the order the save
//! holds its names in.
//!
//! A save's table may be in any order. This build's writer sorts it, but a save
//! is a file — written by another build, by an older one, or by a tool nobody
//! here has seen — and a reader that required ascending order would refuse files
//! that are perfectly readable. So the reader takes the table as it finds it and
//! **the report is sorted when it is reported**, which is the only place the
//! order is anybody's to choose.
//!
//! Both fixtures here are two saves naming the same blocks in two different
//! orders, and both are written out byte by byte for the reason the writer makes
//! them unreachable: it sorts. The claim is that the two saves answer
//! identically, which is what makes the answer a *report* rather than an echo of
//! whatever order the file happened to carry — and what lets the two lists a
//! refusal carries be read, compared and acted on rather than merely scanned.
//!
//! Ascending lexicographic is byte order over the whole namespaced name, which
//! is what a block name already sorts by.

mod common;

use std::error::Error;

use common::TestResult;
use common::handbuilt::{self, Entry, HandBuilt};
use common::registry_declaring;
use mc_core::block::BlockRegistry;
use mc_core::id::BlockName;
use mc_world::persistence::{self, SaveRequirements};
use tempfile::TempDir;

/// The three names both fixtures are about, ascending.
///
/// This is the answer, not an input: neither save below holds them in this
/// order.
const ASCENDING: [&str; 3] = ["fixture:andesite", "fixture:basalt", "fixture:chert"];

/// One order a save's table might hold them in, and another.
///
/// Two rotations rather than one order and its reverse, so that a report which
/// happened to reverse what it was given would not land on the ascending answer
/// by accident, and neither save's own order is the sorted one.
const ONE_ORDER: [&str; 3] = ["fixture:chert", "fixture:andesite", "fixture:basalt"];
const ANOTHER_ORDER: [&str; 3] = ["fixture:basalt", "fixture:chert", "fixture:andesite"];

/// What the unregistered fixtures record against each name.
///
/// Arbitrary, and it may be: nothing compares a declaration against a registry
/// that does not hold the name at all. They differ from each other so that two
/// entries of one table cannot be mistaken for each other.
const A_DECLARATION: [(u64, u64); 3] = [(11, 12), (13, 14), (15, 16)];

/// `names` as block names, in the order given.
fn block_names(names: &[&str]) -> Result<Vec<BlockName>, Box<dyn Error>> {
    let mut parsed = Vec::with_capacity(names.len());
    for name in names {
        parsed.push(BlockName::parse(name)?);
    }
    Ok(parsed)
}

/// A table naming `order`, with a declaration recorded against each name that no
/// registry here will ever be asked about.
fn table_of<'a>(order: &[&'a str]) -> Vec<Entry<'a>> {
    order
        .iter()
        .zip(A_DECLARATION)
        .map(|(&name, (behaviour, appearance))| (name, behaviour, appearance))
        .collect()
}

/// What a save whose table is `entries` says it needs.
fn a_save_holding(
    directory: &TempDir,
    file_name: &str,
    entries: &[Entry<'_>],
) -> Result<SaveRequirements, Box<dyn Error>> {
    let path = handbuilt::written(
        directory,
        file_name,
        HandBuilt {
            table: entries,
            ..HandBuilt::default()
        },
    )?;
    Ok(persistence::requirements(&path)?)
}

/// Every name a registry makes nothing of, out of a save whose table is
/// `entries`.
fn missing_from(
    directory: &TempDir,
    file_name: &str,
    entries: &[Entry<'_>],
) -> Result<Vec<BlockName>, Box<dyn Error>> {
    let required = a_save_holding(directory, file_name, entries)?;
    Ok(persistence::resolve(&required, &BlockRegistry::new()).missing)
}

/// Every name `registry` holds and no longer declares the way the save records
/// it, out of a save whose table is `entries`.
fn changed_from(
    directory: &TempDir,
    file_name: &str,
    entries: &[Entry<'_>],
    registry: &BlockRegistry,
) -> Result<Vec<BlockName>, Box<dyn Error>> {
    let required = a_save_holding(directory, file_name, entries)?;
    Ok(persistence::resolve(&required, registry).changed)
}

#[test]
fn two_saves_naming_the_same_unheld_blocks_in_two_orders_report_them_ascending() -> TestResult {
    let directory = TempDir::new()?;

    let one = missing_from(&directory, "one_order.mcw", &table_of(&ONE_ORDER))?;
    let other = missing_from(&directory, "another_order.mcw", &table_of(&ANOTHER_ORDER))?;

    assert_eq!(
        (one, other),
        (block_names(&ASCENDING)?, block_names(&ASCENDING)?),
        "the list of what is missing is the one thing a player acts on, and they act on it by \
         reading it — so it comes back in an order they can scan, the same order every time, \
         whatever order the file it came out of happened to be in. Two different orders answering \
         identically is what says the report is sorted rather than passed through, and neither of \
         the two is already the answer"
    );
    Ok(())
}

#[test]
fn two_saves_naming_the_same_redeclared_blocks_in_two_orders_report_them_ascending() -> TestResult {
    let directory = TempDir::new()?;
    let declared: Vec<(&str, bool)> = ASCENDING.iter().map(|name| (*name, true)).collect();
    let registry = registry_declaring(&declared)?;
    let one = handbuilt::recorded_as_changed(&ONE_ORDER, &registry)?;
    let other = handbuilt::recorded_as_changed(&ANOTHER_ORDER, &registry)?;

    let first = changed_from(&directory, "changed_one_order.mcw", &one, &registry)?;
    let second = changed_from(&directory, "changed_another_order.mcw", &other, &registry)?;

    assert_eq!(
        (first, second),
        (block_names(&ASCENDING)?, block_names(&ASCENDING)?),
        "the changed list is read the same way and by the same person, under more pressure: they \
         are deciding whether to open a world at all, and a list whose order moved between two \
         launches would look like the answer moved. Each of these three records a declaration one \
         bit away from what the registry declares, so all three really have changed and none of \
         them changed by accident"
    );
    Ok(())
}
