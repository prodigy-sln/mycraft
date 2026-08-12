//! The window library is named in one file of this crate, and this is what says
//! so.
//!
//! `architecture.md`'s boundaries table binds it: `winit::event::*` is
//! translated into `mc-render`'s pure vocabulary — `WindowEventKind`,
//! `SurfaceErrorKind`, `SurfaceSize`, `AdapterFacts` — in `src/events.rs`, and
//! `winit::` appears nowhere else. That is not decoration. ADR-013 excludes this
//! whole crate from the coverage denominator on the stated grounds that it holds
//! the event-loop adapter and the composition wiring and no policy at all, so
//! every decision that leaks in here leaves the denominator along with it, and
//! nothing about the gate's number would change on the day it happened.
//!
//! **An absence proves nothing on its own**, and this one is weaker than most:
//! it is true today for the trivial reason that nothing in this crate names
//! `winit` yet, and it would stay true if the walk broke, the filter drifted, or
//! the exemption grew to swallow everything. So the scan is asked two further
//! questions — whether it read any source at all, and whether the same function
//! pointed at a tree that *does* name the library outside the adapter reports
//! it while a file that may name it is passed over.
//!
//! The scan reads a file's production text — the file minus its doc comments —
//! and skips sibling `*_test.rs` unit files, exactly as this workspace's other
//! source scans do and for the same two reasons: unit tests live in sibling
//! files so skipping them is a file-name filter rather than a parse, and a
//! rustdoc example is a doc test, so prose about the window library is not a use
//! of it.

use std::error::Error;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

type TestResult = Result<(), Box<dyn Error>>;

/// The crate whose types may cross into this one.
const WINDOW_LIBRARY: &str = "winit";

/// The one file of `src/` that may name it.
const ADAPTER: &str = "events.rs";

/// What a scan of one directory tree found.
#[derive(Debug, Default)]
struct Scan {
    files_read: usize,
    hits: Vec<String>,
}

/// A `.rs` file that is neither a sibling unit-test file nor the adapter the
/// window library is allowed to be named in.
fn is_guarded_source(path: &Path) -> bool {
    path.file_name()
        .and_then(OsStr::to_str)
        .is_some_and(|file_name| {
            file_name.ends_with(".rs") && !file_name.ends_with("_test.rs") && file_name != ADAPTER
        })
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

/// Reads every guarded Rust source under `root` and reports each place the
/// window library is named in one's production text.
fn scan_for_window_library(root: &Path) -> Result<Scan, Box<dyn Error>> {
    let mut scan = Scan::default();
    scan_directory(root, &mut scan)?;
    Ok(scan)
}

fn scan_directory(directory: &Path, scan: &mut Scan) -> Result<(), Box<dyn Error>> {
    for entry in fs::read_dir(directory)? {
        let path = entry?.path();
        if path.is_dir() {
            scan_directory(&path, scan)?;
        } else if is_guarded_source(&path) {
            scan_file(&path, scan)?;
        }
    }
    Ok(())
}

fn scan_file(path: &Path, scan: &mut Scan) -> Result<(), Box<dyn Error>> {
    let text = production_text(&fs::read_to_string(path)?);
    scan.files_read += 1;
    if text.contains(WINDOW_LIBRARY) {
        scan.hits
            .push(format!("{} names `{WINDOW_LIBRARY}`", path.display()));
    }
    Ok(())
}

/// This crate's own production source.
fn client_sources() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
}

#[test]
fn no_source_of_the_client_but_its_event_adapter_names_the_window_library() -> TestResult {
    let scanned = scan_for_window_library(&client_sources())?;

    assert!(
        scanned.files_read > 0,
        "the scan read no source at all, so the check below would be vacuous"
    );
    assert!(
        scanned.hits.is_empty(),
        "the window library is translated into the renderer's pure vocabulary in `{ADAPTER}` and \
         is named nowhere else, or the policies this crate is excluded from coverage for having \
         none of have started to live here: {:?}",
        scanned.hits
    );
    Ok(())
}

/// The control for the assertion above, in both directions at once.
///
/// A scan whose walk broke, whose filter skipped everything, or whose exemption
/// grew past the one file it is allowed to cover would report a clean crate
/// forever. The fixture names the library twice — once in the adapter, which
/// must be passed over, and once beside it, which must not be — so a scan that
/// reported nothing and a scan that exempted everything are both caught here.
#[test]
fn the_same_scan_reports_the_window_library_named_outside_the_event_adapter() -> TestResult {
    let fixture = tempfile::tempdir()?;
    let allowed = fixture.path().join(ADAPTER);
    let forbidden = fixture.path().join("app.rs");
    fs::write(&allowed, "use winit::event::WindowEvent;\n")?;
    fs::write(&forbidden, "use winit::window::Window;\n")?;

    let scanned = scan_for_window_library(fixture.path())?;

    assert_eq!(
        (
            scanned.files_read,
            scanned
                .hits
                .iter()
                .any(|hit| hit.contains("app.rs") && !hit.contains(ADAPTER)),
            scanned.hits.len()
        ),
        (1, true, 1),
        "the scan has to read the file beside the adapter, report it, and pass the adapter over: \
         {:?}",
        scanned.hits
    );
    Ok(())
}
