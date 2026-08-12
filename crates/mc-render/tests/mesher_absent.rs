//! No frame path meshes.
//!
//! Meshing runs on rayon workers during world preparation and never on the
//! render thread. That is a rule about where a call may appear, so it is checked
//! where it can be: by reading this crate's own production source and finding no
//! call to the section mesher in it.
//!
//! **An absence proves nothing on its own.** A scan whose directory walk broke,
//! whose file filter drifted, or whose matcher no longer matches anything would
//! report a clean renderer forever — including on the day meshing creeps onto
//! the frame path. So the second test points the same scan at the source that
//! *must* contain the call, the replay's world preparation, and the pair is
//! deliberately in one file: split across two crates they could rot apart.
//!
//! The scan reads a file's production text — the file minus its doc comments —
//! and skips sibling `*_test.rs` unit files, exactly as `mc-world`'s
//! hardcoded-name scan does and for the same two reasons: unit tests live in
//! sibling files so skipping them is a file-name filter rather than a parse, and
//! a rustdoc example is a doc test, so prose describing the mesher is not a call
//! to it.

use std::error::Error;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

type TestResult = Result<(), Box<dyn Error>>;

/// The call the frame path must not contain and world preparation must.
const MESHER: &str = "mesh_section";

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

/// Reads every production Rust source under `root` and reports each place the
/// section mesher is named in one's production text.
fn scan_for_mesher(root: &Path) -> Result<Scan, Box<dyn Error>> {
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
    if text.contains(MESHER) {
        scan.hits
            .push(format!("{} names `{MESHER}`", path.display()));
    }
    Ok(())
}

/// This crate's own production source.
fn renderer_sources() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
}

/// The replay's world-preparation source, resolved from this crate's manifest
/// directory so the pair of tests below reads two roots and one scan.
fn preparation_sources() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("mc-sim")
        .join("src")
}

#[test]
fn no_production_source_of_the_renderer_calls_the_section_mesher() -> TestResult {
    let scanned = scan_for_mesher(&renderer_sources())?;

    assert!(
        scanned.files_read > 0,
        "the scan read no source at all, so the check below would be vacuous"
    );
    assert!(
        scanned.hits.is_empty(),
        "meshing belongs on the preparation workers, never on a frame path: {:?}",
        scanned.hits
    );
    Ok(())
}

/// The control for the assertion above, and the reason it cannot go quiet. A
/// broken walk, a filter that skipped everything, or a matcher looking for a
/// name nothing uses any more would each leave the absence trivially true.
#[test]
fn the_replays_world_preparation_does_call_the_section_mesher() -> TestResult {
    let scanned = scan_for_mesher(&preparation_sources())?;

    assert!(
        scanned.files_read > 0,
        "the scan read no source at all, so the check below cannot be about anything"
    );
    assert!(
        !scanned.hits.is_empty(),
        "the source that is supposed to mesh has to name the mesher, or the scan above is \
         reporting an absence it would report over any directory"
    );
    Ok(())
}
