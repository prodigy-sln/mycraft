//! Two things a save's table of names may not be, and what saying so out loud
//! costs.
//!
//! Both fixtures here are written out byte by byte, and they have to be: the
//! writer builds its table from an ordered set of names it has already parsed,
//! so it can produce neither a name repeated nor text that is not a name. A save
//! on a player's disk can be both — edited, truncated and re-joined, written by
//! a build that was wrong, or simply damaged — and a reader that trusted the
//! writer's discipline would meet each of them for the first time in the middle
//! of a load.
//!
//! **Neither refusal is the decoder's**, and that is why they are ours to
//! assert. A table naming the same block twice decodes perfectly well; so does
//! one holding the string "not a namespaced name". Both are refusals about what
//! the decoded values *mean*, raised where we raise them, and each names the
//! value that was wrong so that a player is told which entry to go and look at
//! rather than that a file could not be read.
//!
//! **The reader must not require ascending order.** The writer sorts, but two
//! other requirements load saves naming the same blocks in different table
//! orders and expect both to report ascending — so the reader accepts any order,
//! refuses only a duplicate, and sorts when it reports. A reader that refused an
//! unsorted table would make those unreachable, and it would pass both tests
//! here.

mod common;

use common::TestResult;
use common::handbuilt::{self, Entry, HandBuilt};
use mc_core::id::{BlockName, NamespacedIdError};
use mc_world::persistence::{self, LoadError};
use tempfile::TempDir;

/// The name a table holds twice.
const NAMED_TWICE: &str = "fixture:andesite";

/// A table naming the same block twice, with a different declaration recorded
/// against each.
///
/// The two declarations differ so that the duplicate is a real ambiguity rather
/// than a harmless repetition: a reader that quietly kept the last entry it saw
/// would resolve this save against a declaration the earlier half of the file
/// disagrees with, and every cell pointing at the first entry would be read as
/// the second block.
const A_NAME_TWICE: [Entry<'static>; 3] = [
    (NAMED_TWICE, 11, 12),
    ("fixture:basalt", 13, 14),
    (NAMED_TWICE, 15, 16),
];

/// Text a table holds where a name belongs.
///
/// No separator at all, which is the malformation a hand-edited file most often
/// carries: the namespace dropped, leaving something that still looks like a
/// word.
const NOT_A_NAME: &str = "andesite";

/// A table holding one entry that is not a namespaced name, between two that
/// are.
///
/// Between, so that a reader which stopped at the first entry and one which
/// checked only the last would both have to reach it.
const A_TABLE_HOLDING_TEXT: [Entry<'static>; 3] = [
    ("fixture:andesite", 11, 12),
    (NOT_A_NAME, 13, 14),
    ("fixture:basalt", 15, 16),
];

/// A save at `path` whose table is `entries`, and what it says it needs.
fn what_a_save_naming(
    directory: &TempDir,
    file_name: &str,
    entries: &[Entry<'_>],
) -> Result<Result<persistence::SaveRequirements, LoadError>, Box<dyn std::error::Error>> {
    let path = handbuilt::written(
        directory,
        file_name,
        HandBuilt {
            table: entries,
            ..HandBuilt::default()
        },
    )?;
    Ok(persistence::requirements(&path))
}

#[test]
fn a_save_naming_the_same_block_twice_is_refused_naming_the_block() -> TestResult {
    let directory = TempDir::new()?;

    let asked = what_a_save_naming(&directory, "named_twice.mcw", &A_NAME_TWICE)?;

    assert_eq!(
        asked,
        Err(LoadError::DuplicateName {
            name: BlockName::parse(NAMED_TWICE)?
        }),
        "a name occurring twice makes the table two answers to the same question, and every cell \
         in the save points at one of them by number — so whichever entry the reader kept, half \
         the world would be read against a declaration the file disagrees with, silently and \
         everywhere. Naming the block is what turns that into something a player can go and look \
         at"
    );
    Ok(())
}

#[test]
fn a_save_whose_table_holds_text_that_is_not_a_name_is_refused_naming_the_text() -> TestResult {
    let directory = TempDir::new()?;

    let asked = what_a_save_naming(&directory, "not_a_name.mcw", &A_TABLE_HOLDING_TEXT)?;

    assert_eq!(
        asked,
        Err(LoadError::MalformedName {
            text: NOT_A_NAME.to_owned(),
            source: NamespacedIdError::MissingNamespace {
                text: NOT_A_NAME.to_owned()
            }
        }),
        "the table is the one place a save says what its blocks are called, and text that is not \
         a name cannot be resolved against any registry — so the alternative to refusing is a \
         reader that drops the entry and reports a world needing one block fewer than it holds. \
         Quoting the text back is the whole of the diagnosis: it is the only thing that says which \
         entry, and the reason beside it says what is wrong with it"
    );
    Ok(())
}
