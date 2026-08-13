//! Every camera this client renders comes from the snapshot the simulation
//! published, and this is what says so.
//!
//! Invariant 4 in structural form, on the other end from the intent's field set.
//! The simulation decides where the player is and therefore where the camera is;
//! the client's only job is to hand the renderer what it was published. The
//! cheapest way to lose that is not a bug in the physics — it is a second call to
//! the renderer's camera constructor somewhere in this crate, built from a pose
//! the client made up, drawing a frame from a viewpoint nothing authoritative
//! ever agreed to. One call site cannot do that; two can, and nobody would notice
//! which one won.
//!
//! It is asserted about the *source* rather than about a frame because a frame
//! cannot see it: a picture drawn from an invented camera looks exactly like a
//! picture, and ADR-013 excludes this crate from coverage on the stated grounds
//! that it holds no policy. A camera invented here is a policy.
//!
//! **An absence assertion goes green forever the day the thing it guards is
//! quietly removed**, and this one has three ways to do that: a walk that broke
//! reads no source, a filter that drifted skips every file, and a constructor
//! renamed out from under the scan is named nowhere. The first is caught by
//! asking whether the scan read anything at all; the second and third by
//! requiring *exactly one* file rather than at most one — a client that named the
//! constructor nowhere would be one whose camera comes from somewhere this scan
//! cannot see. The control below points the same function at a fixture directory
//! whose two files both name it, so a scan that reported nothing and a scan that
//! stopped at its first hit are both caught.
//!
//! The scan reads a file's production text — the file minus its doc comments —
//! and skips sibling `*_test.rs` unit files, exactly as this workspace's other
//! source scans do and for the same two reasons: unit tests live in sibling files
//! so skipping them is a file-name filter rather than a parse, and a rustdoc
//! example is a doc test, so prose about the camera is not a use of it.

use std::error::Error;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

type TestResult = Result<(), Box<dyn Error>>;

/// The renderer's camera-view constructor: the one door a pose goes through to
/// become something a frame is drawn from.
const CAMERA_CONSTRUCTOR: &str = "camera_view";

/// How many files of the client's own source may name it.
const ALLOWED_FILES: usize = 1;

/// How many files of the control's fixture directory name it.
const FIXTURE_FILES: usize = 2;

/// What a scan of one directory tree found.
#[derive(Debug, Default)]
struct Scan {
    files_read: usize,
    naming: Vec<String>,
}

/// A `.rs` file that is not a sibling unit-test file.
fn is_guarded_source(path: &Path) -> bool {
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

/// Reads every guarded Rust source under `root` and reports each file whose
/// production text names the camera constructor.
///
/// One entry per file rather than per mention: a file that imports the
/// constructor and calls it names it in one place as far as this is concerned,
/// and what is being counted is call sites' worth of decision, not occurrences.
fn scan_for_camera_constructor(root: &Path) -> Result<Scan, Box<dyn Error>> {
    let mut scan = Scan::default();
    scan_directory(root, &mut scan)?;
    scan.naming.sort();
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
    if text.contains(CAMERA_CONSTRUCTOR) {
        scan.naming.push(
            path.file_name()
                .and_then(OsStr::to_str)
                .unwrap_or_default()
                .to_owned(),
        );
    }
    Ok(())
}

/// A path inside this crate.
fn crate_path(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(relative)
}

#[test]
fn exactly_one_source_of_the_client_names_the_renderers_camera_constructor() -> TestResult {
    let scanned = scan_for_camera_constructor(&crate_path("src"))?;

    assert!(
        scanned.files_read > 0,
        "the scan read no source at all, so the check below would be vacuous"
    );
    assert_eq!(
        scanned.naming.len(),
        ALLOWED_FILES,
        "the camera a frame is drawn from comes from the published snapshot and is built in \
         one file of this crate, so that no second viewpoint can be invented beside it — \
         these files name `{CAMERA_CONSTRUCTOR}`: {:?}",
        scanned.naming
    );
    Ok(())
}

/// The control for the assertion above, in both directions at once.
///
/// A scan whose walk broke, whose filter skipped everything, or which stopped at
/// its first hit would report a client with one call site forever. The fixture
/// directory holds two files that both name the constructor — one of them called
/// `app.rs`, which is the name the real crate's single call site has, so a scan
/// that had quietly hard-coded an exemption for it is caught too.
#[test]
fn the_same_scan_reports_both_fixture_files_that_name_the_constructor() -> TestResult {
    let fixture = crate_path("tests/fixtures/camera_source");

    let scanned = scan_for_camera_constructor(&fixture)?;

    assert_eq!(
        (scanned.files_read, scanned.naming),
        (
            FIXTURE_FILES,
            vec!["app.rs".to_owned(), "overlay.rs".to_owned()]
        ),
        "the fixture's two files both build a camera, and both have to come back — a scan \
         that reported one of them would report a client with a single call site whatever \
         its second one did"
    );
    Ok(())
}
