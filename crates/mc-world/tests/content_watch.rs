//! The window a content watch settles for, and which paths under a root are
//! content at all.
//!
//! # The window is asserted where it crosses into the vendor, not where it is
//! declared
//!
//! A test that read `SETTLING_WINDOW` and compared it against 150 ms would be the
//! constant agreeing with itself, and a scan that finds it declared once
//! (`settling_window_declared_once.rs`) says nothing about which duration the
//! debouncer was built with. Both stay green while the adapter hands the vendor
//! `Duration::ZERO` — and the shipped client then begins one attempt per
//! filesystem event, which is the whole failure the window exists to prevent.
//! What closes it is asking the adapter which window it handed over. No
//! filesystem, no timer.
//!
//! **What it does not close, stated rather than left to be found**: an adapter
//! that recorded the window it was given and handed the builder a different
//! literal. That takes two spellings at one call site, and the parameterised door
//! below is what makes the shipped one the only place the declared window is
//! supplied. The residue is held by review.
//!
//! # The relevance rule is graded against the directory it is about
//!
//! Its expectation is the shipped root's own listing rather than a list written
//! here: every declaration the two loaders read has to be content, and everything
//! else under the same root has to be nothing. That is an oracle from outside the
//! code under test — a rule keyed on the extension alone accepts the material
//! files, a rule that forgot one of the two directories rejects declarations that
//! are read, and either way the listing disagrees with it.
//!
//! Because it asserts an absence over one half of its input, it carries the
//! premise of that half: a run over a root holding nothing but declarations would
//! have nothing to reject, and says so rather than passing.

mod common;

use std::error::Error;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use common::{TestResult, repository_root};
use mc_world::content::watch::{NotifyContentWatch, SETTLING_WINDOW, declares_content};
use tempfile::TempDir;

/// The two directories the loaders read, and the extension each reads there.
///
/// Spelled here rather than imported: these are the four values the rule under
/// test is required to be *built from*, and a test importing them would be the
/// rule agreeing with its own source. What grades the rule is the listing below,
/// and these say which files that listing is expected to sort which way.
const BLOCKS: (&str, &str) = ("blocks", "luau");
const HUD: (&str, &str) = ("hud", "toml");

/// The window the declaration says a save is allowed to settle over.
///
/// Written out as the spec's Declared Quantities table states it, and deliberately
/// **not** read from `SETTLING_WINDOW`: that constant is what is under test on the
/// other side of this comparison, and an expectation assembled from it would read
/// back whatever it became.
const A_SAVE_SETTLES_FOR: Duration = Duration::from_millis(150);

#[test]
fn the_window_a_watch_hands_its_debouncer_is_the_declared_settling_window() -> TestResult {
    let directory = TempDir::new()?;

    let watching = NotifyContentWatch::watching(directory.path());
    let asked_for_zero = NotifyContentWatch::settling_for(directory.path(), Duration::ZERO);

    assert_eq!(
        (
            watching.settling_window(),
            asked_for_zero.settling_window(),
            SETTLING_WINDOW
        ),
        (A_SAVE_SETTLES_FOR, Duration::ZERO, A_SAVE_SETTLES_FOR),
        "the shipped door is the one place the declared window is supplied and this is the \
         boundary it crosses: everything else about a save settling is the vendor's own. The \
         second reading is what makes the first falsifiable — an adapter reporting a constant \
         rather than the window it was handed answers 150 ms to both — and the third is what \
         stops the pair agreeing at zero, which would leave the coalescing test green, the \
         declaration scan green, and the shipped client beginning one attempt per filesystem \
         event"
    );
    Ok(())
}

#[test]
fn every_declaration_the_shipped_root_holds_is_content_and_nothing_else_under_it_is() -> TestResult
{
    let root = repository_root()?.join("content").join("base");
    let (declared, beside) = sorted_by_the_listing(&root)?;
    require_something_to_reject(&beside)?;

    let nothing: Vec<&PathBuf> = Vec::new();
    let unread: Vec<&PathBuf> = declared
        .iter()
        .filter(|declaration| !declares_content(&root, declaration.as_path()))
        .collect();
    let claimed: Vec<&PathBuf> = beside
        .iter()
        .filter(|other| declares_content(&root, other.as_path()))
        .collect();

    assert_eq!(
        (unread, claimed),
        (nothing.clone(), nothing),
        "a change begins an attempt exactly when the loader would read the file that changed, so \
         the shipped root's own listing is what the rule is graded against: {declared} \
         declarations under `{blocks}/` and `{hud}/`, and {beside} other files beneath the same \
         root that no loader on the content path opens. A rule keyed on an extension alone claims \
         the second set; a rule that lost one of the two directories disclaims part of the first",
        declared = declared.len(),
        beside = beside.len(),
        blocks = BLOCKS.0,
        hud = HUD.0
    );
    Ok(())
}

/// Every file under `root`, sorted into the ones the loaders read and the ones
/// beside them.
///
/// The sort is by directory and extension, which is how both loaders decide what
/// they are looking at — one directory deep, and on the extension rather than on
/// the name.
///
/// # Errors
///
/// Returns an error if the root cannot be walked.
fn sorted_by_the_listing(root: &Path) -> Result<(Vec<PathBuf>, Vec<PathBuf>), Box<dyn Error>> {
    let found = files_under(root)?;
    Ok(found
        .into_iter()
        .partition(|path| is_a_declaration(root, path)))
}

/// Whether the loaders read `path`: directly under one of the two declaration
/// directories, with that directory's own extension.
fn is_a_declaration(root: &Path, path: &Path) -> bool {
    [BLOCKS, HUD].iter().any(|(directory, extension)| {
        path.parent() == Some(&root.join(directory))
            && path.extension() == Some(OsStr::new(extension))
    })
}

/// Every file at any depth under `directory`.
///
/// # Errors
///
/// Returns an error if a directory cannot be read.
fn files_under(directory: &Path) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    let mut found = Vec::new();
    for entry in fs::read_dir(directory)? {
        let path = entry?.path();
        if path.is_dir() {
            found.extend(files_under(&path)?);
        } else {
            found.push(path);
        }
    }
    Ok(found)
}

/// Refuses unless the shipped root holds a file the loaders do not read.
///
/// Half of this scenario is an absence, and an absence over an empty set is free.
/// The shipped root carries block textures the loader has never opened, so there
/// is something to reject; a root that stopped carrying any would make that half
/// vacuous, and this says so rather than passing.
///
/// # Errors
///
/// Returns an error when every file under the root is a declaration.
fn require_something_to_reject(beside: &[PathBuf]) -> Result<(), Box<dyn Error>> {
    if beside.is_empty() {
        return Err(NOTHING_TO_REJECT.into());
    }
    Ok(())
}

/// What a run over a root holding nothing but declarations is told.
const NOTHING_TO_REJECT: &str = "this scenario needs the shipped content root to hold at least one file no loader on the \
     content path reads, and every file under it is a declaration. The half of the assertion \
     about what begins no attempt would then hold over nothing";
