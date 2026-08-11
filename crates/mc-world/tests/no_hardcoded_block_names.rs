//! No Rust source names a block the base game ships.
//!
//! Invariant 1 in test form. The base game is a mod, so the engine may know the
//! *shape* of a block definition and nothing about any particular block; the
//! moment a name appears in Rust, the base game has a privilege a third-party mod
//! does not.
//!
//! The scan is a file-level filter — every `.rs` file under a crate's `src/`,
//! read whole, except the sibling `*_test.rs` unit files. That is only a valid
//! filter because unit tests live in sibling files rather than inline
//! `#[cfg(test)]` modules; it is also why a rustdoc example may not use a `base:`
//! name and uses `example:` instead. Test files under `tests/` are not scanned,
//! which is why this one may say the names out loud.

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

/// Reads every production Rust source under `root` and reports each place a
/// shipped block name appears in one.
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

/// A `.rs` file that is not a sibling unit-test file.
fn is_production_source(path: &Path) -> bool {
    path.file_name()
        .and_then(OsStr::to_str)
        .is_some_and(|file_name| file_name.ends_with(".rs") && !file_name.ends_with("_test.rs"))
}

fn scan_file(path: &Path, scan: &mut Scan) -> Result<(), Box<dyn Error>> {
    let text = fs::read_to_string(path)?;
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
