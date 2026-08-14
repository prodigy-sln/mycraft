//! What a content root's `hud/` directory gives a layout, and what it says when
//! it cannot give one.
//!
//! Three groups, and the middle one is why the other two are written the way
//! they are.
//!
//! *What is picked up* — the declarations directly under `hud/`, in file-name
//! order, ignoring anything that is not a declaration file.
//!
//! *What is refused* — a declaration file that is not TOML, and a root where
//! `hud` is a regular file. **The second is the falsifier for the third group
//! and the third group is its falsifier.** A loader that answered "no elements"
//! to everything it could not read passes every zero-element scenario below and
//! fails the regular-file one; a loader that refused everything it could not
//! find does the reverse. Neither test is evidence without the other, so they
//! live in one binary and each says so.
//!
//! *What is a valid, empty answer* — a root with no `hud/` at all, and a `hud/`
//! holding no declaration. Deliberately unlike `blocks/`, where declaring
//! nothing is an error: a world with no blocks cannot be played, and a game with
//! no HUD is merely bare.

mod common;

use std::fs;

use common::TestResult;
use common::hud::{
    HUD_DIRECTORY, ROOT_WITH_A_FILE_NAMED_HUD, declared_by, hud_content_root, hud_file, refusal,
    registered_names, root_with_a_file_named_hud, root_with_an_empty_hud_directory,
    root_without_a_hud_directory,
};
use tempfile::TempDir;

/// The eight files the determinism fixture declares, in the order it creates
/// them — which is deliberately not their file-name order.
///
/// Eight rather than two: the falsifier is a directory-iteration order leaking
/// through into the registered order, and an iteration order that happened to
/// agree with the sort on two entries is not much of a coincidence. Eight
/// entries created in a shuffled order make an unsorted read plausibly differ
/// from a sorted one, both between two reads and against the expectation below.
const CREATED_IN_THIS_ORDER: [&str; 8] = [
    "zulu", "mike", "alpha", "tango", "bravo", "sierra", "charlie", "romeo",
];

/// The declaration files [`CREATED_IN_THIS_ORDER`] writes, in creation order.
fn declaration_files() -> Vec<(String, String)> {
    CREATED_IN_THIS_ORDER
        .iter()
        .map(|stem| (format!("{stem}.toml"), hud_file(&declared_by(stem))))
        .collect()
}

/// The element names those files declare, in the order the contract requires —
/// derived here by sorting the **file names** this suite wrote and mapping each
/// to the element it declares.
///
/// Sorted by this test rather than read back from a load: an expectation taken
/// from the subject is an expectation the subject cannot fail.
fn in_file_name_order() -> Vec<String> {
    let mut files: Vec<String> = CREATED_IN_THIS_ORDER
        .iter()
        .map(|stem| format!("{stem}.toml"))
        .collect();
    files.sort();
    files
        .iter()
        .map(|file| declared_by(file.trim_end_matches(".toml")))
        .collect()
}

#[test]
fn a_hud_directory_registers_its_declarations_in_file_name_order_rather_than_creation_order()
-> TestResult {
    let directory = TempDir::new()?;
    // `zulu.toml` is written first and `alpha.toml` second, so creation order and
    // file-name order disagree. A fixture built in file-name order could not
    // falsify the sort at all — it would pass for a reader that did nothing.
    let root = hud_content_root(
        &directory,
        &[
            ("zulu.toml", hud_file(&declared_by("zulu"))),
            ("alpha.toml", hud_file(&declared_by("alpha"))),
        ],
    )?;

    assert_eq!(
        registered_names(&root)?,
        vec![declared_by("alpha"), declared_by("zulu")],
        "a `hud/` directory registers exactly the declarations it holds, ordered by file name and \
         not by the order they were created in"
    );
    Ok(())
}

#[test]
fn a_declaration_in_a_subdirectory_of_hud_is_not_registered() -> TestResult {
    let directory = TempDir::new()?;
    let root = hud_content_root(
        &directory,
        &[("alpha.toml", hud_file(&declared_by("alpha")))],
    )?;
    let nested = root.join(HUD_DIRECTORY).join("nested");
    fs::create_dir_all(&nested)?;
    fs::write(nested.join("nested.toml"), hud_file(&declared_by("nested")))?;

    assert_eq!(
        registered_names(&root)?,
        vec![declared_by("alpha")],
        "the search is one directory deep: a declaration below `hud/` is not registered, and \
         finding it would make where a file sits a thing content authors have to reason about"
    );
    Ok(())
}

#[test]
fn a_file_that_is_not_a_declaration_is_ignored_rather_than_refused() -> TestResult {
    let directory = TempDir::new()?;
    // The notes are not TOML either, so a reader that ignored the extension would
    // refuse the whole root rather than quietly register something extra — the
    // assertion below reports both mistakes.
    let root = hud_content_root(
        &directory,
        &[
            (
                "crosshair-horizontal.toml",
                hud_file(&declared_by("crosshair-horizontal")),
            ),
            (
                "notes.md",
                "These are notes about the crosshair, not a declaration.\n".to_owned(),
            ),
        ],
    )?;

    assert_eq!(
        registered_names(&root)?,
        vec![declared_by("crosshair-horizontal")],
        "a file that is not a declaration is passed over, and the root still loads"
    );
    Ok(())
}

#[test]
fn a_declaration_file_that_is_not_valid_toml_is_refused_naming_that_file() -> TestResult {
    const BROKEN_FILE: &str = "broken.toml";
    let directory = TempDir::new()?;
    // Sorted after `alpha.toml`, so the unreadable file is genuinely read second
    // and a loader that registered as it went would already hold one element.
    let root = hud_content_root(
        &directory,
        &[
            ("alpha.toml", hud_file(&declared_by("alpha"))),
            (BROKEN_FILE, "this line is not toml at all\n".to_owned()),
        ],
    )?;

    let error = refusal(&root)?;

    assert!(
        error.to_string().contains(BROKEN_FILE),
        "a content author is told which file could not be read, not merely that the HUD failed \
         to load: {error}"
    );
    Ok(())
}

/// The counterpart of the two zero-element scenarios below, and each is the
/// other's falsifier.
///
/// A loader that answered "no elements" to everything it could not read would
/// pass both of those and fail this one; a loader that refused everything it
/// could not find would pass this one and fail both of those. Only the three
/// together say that a missing `hud/` is an empty answer and a mis-shaped one is
/// a fault.
#[test]
fn a_content_root_whose_hud_is_a_regular_file_is_refused_naming_that_path() -> TestResult {
    let directory = TempDir::new()?;
    let root = root_with_a_file_named_hud(&directory)?;

    let error = refusal(&root)?;

    assert!(
        error.to_string().contains(ROOT_WITH_A_FILE_NAMED_HUD),
        "a root that cannot be read is refused naming the path that could not be read, rather \
         than degrading to zero elements and hiding a mis-shaped content root: {error}"
    );
    Ok(())
}

/// See the regular-file scenario above: on its own, this test is passed by a
/// loader that registers nothing for any root whatsoever. What stops that
/// reading is the rest of this binary — three tests here require an exact,
/// non-empty list of names, and the regular-file test requires a refusal.
#[test]
fn a_content_root_with_no_hud_directory_loads_with_no_elements() -> TestResult {
    let directory = TempDir::new()?;
    let root = root_without_a_hud_directory(&directory)?;

    assert_eq!(
        registered_names(&root)?,
        Vec::<String>::new(),
        "a content root declaring no HUD is a valid, empty answer — a mod that ships none is \
         ordinary, unlike one that declares no blocks"
    );
    Ok(())
}

/// The sibling of the scenario above, and a different fact: there the directory
/// listing fails outright, here it succeeds and yields nothing. An
/// implementation can get one right and the other wrong.
#[test]
fn a_hud_directory_holding_no_declaration_file_loads_with_no_elements() -> TestResult {
    let directory = TempDir::new()?;
    let root = root_with_an_empty_hud_directory(&directory)?;

    assert_eq!(
        registered_names(&root)?,
        Vec::<String>::new(),
        "a `hud/` directory that declares nothing is a valid, empty answer rather than a refusal"
    );
    Ok(())
}

#[test]
fn loading_one_content_root_twice_registers_the_same_elements_in_the_same_order() -> TestResult {
    let directory = TempDir::new()?;
    let files = declaration_files();
    let declarations: Vec<(&str, String)> = files
        .iter()
        .map(|(file, body)| (file.as_str(), body.clone()))
        .collect();
    let root = hud_content_root(&directory, &declarations)?;

    let first = registered_names(&root)?;
    let second = registered_names(&root)?;

    // Compared against an order derived from the file names this test wrote, not
    // merely against each other: two loads that agree on the wrong order agree
    // just as loudly as two that agree on the right one.
    assert_eq!(
        (first, second),
        (in_file_name_order(), in_file_name_order()),
        "loading one content root twice in a process registers the same elements in the same \
         order, and that order is the file-name order — a directory-iteration order leaking \
         through would show up here as a difference from the sorted expectation, whether or not \
         the two reads happened to agree with each other"
    );
    Ok(())
}
