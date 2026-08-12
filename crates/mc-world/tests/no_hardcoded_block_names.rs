//! No Rust source names a block the base game ships.
//!
//! Invariant 1 in test form. The base game is a mod, so the engine may know the
//! *shape* of a block definition and nothing about any particular block; the
//! moment a name appears in Rust, the base game has a privilege a third-party mod
//! does not.
//!
//! The scan reads every `.rs` file under a crate's `src/` except the sibling
//! `*_test.rs` unit files, and looks at its **production text**: the file minus
//! its doc comments. Both halves of that are deliberate. Unit tests live in
//! sibling files (`docs/technical/testing.md`), so skipping test code is a
//! file-name filter rather than a parse; a rustdoc example is a doc test, so it
//! is test code that does live in a production file, and dropping doc comments is
//! what lets `/// BlockName::parse("base:stone")` say the most natural thing.
//! Tests under `tests/` are not scanned at all — which is why this one may say
//! the names out loud.

mod common;

use std::error::Error;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

use common::{TestResult, repository_root};
use tempfile::TempDir;

/// The blocks this repository ships as content.
const SHIPPED_NAMES: [&str; 5] = [
    "base:air",
    "base:stone",
    "base:dirt",
    "base:grass",
    "base:water",
];

/// What a scan of one directory tree found.
#[derive(Debug, Default)]
struct Scan {
    files_read: usize,
    hits: Vec<String>,
}

/// A `.rs` file that is not a sibling unit-test file.
fn is_production_source(path: &Path) -> bool {
    path.file_name()
        .and_then(OsStr::to_str)
        .is_some_and(|file_name| file_name.ends_with(".rs") && !file_name.ends_with("_test.rs"))
}

/// A file's text with its doc comments removed.
///
/// Line doc comments are the only form this repository uses; a `/** */` block
/// would be read whole, which errs toward reporting rather than toward silence.
fn production_text(source: &str) -> String {
    source
        .lines()
        .filter(|line| {
            let trimmed = line.trim_start();
            !trimmed.starts_with("///") && !trimmed.starts_with("//!")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Reads every production Rust source under `root` and reports each place a
/// shipped block name appears in one's production text.
fn scan_for_shipped_names(root: &Path) -> Result<Scan, Box<dyn Error>> {
    let mut scan = Scan::default();
    scan_directory(root, &mut scan)?;
    Ok(scan)
}

fn scan_directory(directory: &Path, scan: &mut Scan) -> Result<(), Box<dyn Error>> {
    for entry in fs::read_dir(directory)? {
        let path = entry?.path();
        if path.is_dir() {
            scan_directory(&path, scan)?;
        } else if is_production_source(&path) {
            scan_file(&path, scan)?;
        }
    }
    Ok(())
}

fn scan_file(path: &Path, scan: &mut Scan) -> Result<(), Box<dyn Error>> {
    let text = production_text(&fs::read_to_string(path)?);
    scan.files_read += 1;
    for name in SHIPPED_NAMES {
        if text.contains(name) {
            scan.hits.push(format!("{} names `{name}`", path.display()));
        }
    }
    Ok(())
}

/// Every crate's production source directory.
fn source_directories() -> Result<Vec<PathBuf>, Box<dyn Error>> {
    let mut directories = Vec::new();
    for entry in fs::read_dir(repository_root()?.join("crates"))? {
        let sources = entry?.path().join("src");
        if sources.is_dir() {
            directories.push(sources);
        }
    }
    Ok(directories)
}

/// A scan of the given files, written into a temporary directory.
fn scan_of(files: &[(&str, &str)]) -> Result<(TempDir, Scan), Box<dyn Error>> {
    let directory = TempDir::new()?;
    for (file_name, source) in files {
        fs::write(directory.path().join(file_name), source)?;
    }
    let scanned = scan_for_shipped_names(directory.path())?;
    Ok((directory, scanned))
}

#[test]
fn no_production_rust_source_names_a_block_the_base_game_ships() -> TestResult {
    let mut scanned = Scan::default();
    for directory in source_directories()? {
        let found = scan_for_shipped_names(&directory)?;
        scanned.files_read += found.files_read;
        scanned.hits.extend(found.hits);
    }

    assert!(
        scanned.files_read > 0,
        "the scan read no Rust source at all, so the check below would be vacuous"
    );
    assert!(
        scanned.hits.is_empty(),
        "a block's name belongs to content, never to the engine: {:?}",
        scanned.hits
    );
    Ok(())
}

/// A guard rather than a scenario, and the reason the check above cannot go
/// quiet. A scan whose directory walk or whose matcher broke would report nothing
/// forever — including on the day the invariant it guards is actually violated.
/// The fixture is nested one directory deep on purpose: a walk that stopped at
/// the top level would otherwise still look healthy here.
#[test]
fn the_scan_reports_a_source_that_does_name_a_block_the_base_game_ships() -> TestResult {
    let directory = TempDir::new()?;
    let nested = directory.path().join("nested");
    fs::create_dir_all(&nested)?;
    fs::write(
        nested.join("blocks.rs"),
        "const FILL: &str = \"base:grass\";\n",
    )?;

    let scanned = scan_for_shipped_names(directory.path())?;

    assert!(
        !scanned.hits.is_empty(),
        "a source that does name a shipped block must be reported, or this scan proves nothing"
    );
    Ok(())
}

/// The second half of that guard: the file-name filter must skip test code and
/// *nothing else*. A filter that had drifted into skipping too much — matching
/// `test` anywhere in the name, say — would leave the real check above green
/// while scanning almost nothing. So the fixture puts a shipped name in a sibling
/// unit-test file and another in the module it tests.
#[test]
fn a_name_in_a_sibling_unit_test_file_is_skipped_and_one_beside_it_is_still_found() -> TestResult {
    let (_directory, scanned) = scan_of(&[
        (
            "blocks_test.rs",
            "const NAMED_IN_A_TEST: &str = \"base:dirt\";\n",
        ),
        (
            "blocks.rs",
            "const NAMED_IN_PRODUCTION: &str = \"base:stone\";\n",
        ),
    ])?;

    assert!(
        scanned.hits.len() == 1 && scanned.hits.join(" ").contains("base:stone"),
        "the sibling file is test code and must be skipped; the module beside it is production \
         source and must still be found. Exactly one hit, and it is the second: {:?}",
        scanned.hits
    );
    Ok(())
}

/// Why a rustdoc example may say `base:stone` out loud. A doc example is a doc
/// test — test code that happens to live in a production file — and the most
/// natural example for a namespaced-id type is the real namespace. This asserts
/// the scan agrees, so the rule stays a decision rather than a trap someone
/// rediscovers by turning the suite red.
#[test]
fn a_name_in_a_doc_example_is_not_a_hardcoded_block_name() -> TestResult {
    let (_directory, scanned) = scan_of(&[(
        "names.rs",
        concat!(
            "/// ```\n",
            "/// let name = BlockName::parse(\"base:water\")?;\n",
            "/// ```\n",
            "pub fn parse_a_name() {}\n",
        ),
    )])?;

    assert!(
        scanned.hits.is_empty(),
        "a doc example is a doc test, so naming a shipped block in one is not the engine knowing \
         about it: {:?}",
        scanned.hits
    );
    Ok(())
}
