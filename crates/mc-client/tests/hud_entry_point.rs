//! The client draws its frame through one call, and its sources name no other
//! way of putting a HUD on the screen.
//!
//! A frame test that composes the HUD through a path the windowed client never
//! takes verifies a composition the product does not perform. This scan is the
//! half of that claim a text search can hold; `hud_frame_path.rs` drives the
//! other half on the object the windowed client owns, and its header says why
//! neither is sufficient alone.
//!
//! # What counts as drawing a HUD somewhere else
//!
//! Three shapes, all on one list:
//!
//! - a second **composition** — the composition entry point called from anywhere
//!   in the client, or the plan it composes built by hand from the pure module;
//! - a second **frame path** — terrain recorded outside the one frame call, or a
//!   terrain-only renderer owned beside it. A frame that draws the world with no
//!   HUD over it is the same failure as one that draws two HUDs: the product
//!   would then have a path the HUD never reaches, and every scenario about the
//!   composition itself would stay green.
//! - anything painting a rectangle the client derived itself.
//!
//! The one call the client is allowed to make is spelled `record_frame`, and it
//! is on no list here. **So this guard has no exemption at all**, which is worth
//! saying out loud: an exemption is the part of a scan that grows quietly, and a
//! guard with none cannot be widened without deleting a needle — which its
//! control below would report.
//!
//! # An absence proves nothing on its own
//!
//! An assertion that a scan found nothing is satisfied by a scan whose walk
//! broke, whose filter skipped the tree, or whose needles stopped matching. So
//! the same function is asked two further questions, each its own test rather
//! than an assertion inside the first: whether it reports a fixture that does
//! commit the offence, and whether it refuses outright when it read no source at
//! all.
//!
//! The fixture commits **every** needle the guard carries, for the reason
//! `seam_boundaries.rs` gives: a needle no fixture ever matches is a needle
//! nobody has watched work, and a mistyped one reports a clean scan for as long
//! as it stands there. The expected hit count is derived from the list rather
//! than written down, so a needle added without a fixture line to catch it fails
//! here instead of standing unwatched.
//!
//! Shape follows `seam_boundaries.rs`: production text with doc comments
//! removed, sibling `*_test.rs` unit files skipped, and every path compared
//! whole and relative to the crate root rather than by its bare name.

use std::error::Error;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

use tempfile::TempDir;

type TestResult = Result<(), Box<dyn Error>>;

/// Where the guard reads, and what it refuses to find there.
#[derive(Debug)]
struct Guard {
    /// Directories to walk, each relative to the crate root.
    roots: &'static [&'static str],
    /// The spellings whose presence in production text is the offence.
    needles: &'static [&'static str],
}

/// The client's own sources name no HUD drawing and no frame path but the one.
///
/// `record_terrain` and `TerrainRenderer` are here beside the two composition
/// spellings because a second frame path is the same failure arriving one level
/// up: the terrain pass is what "the HUD stage not run at all" means, and a
/// client that still reached for it directly would have a frame the HUD never
/// reaches while every composition scenario stayed green.
///
/// `hud::compose` rather than a bare `compose`, so the entry point's own name is
/// not caught by the needle aimed at the pure plan builder; `PaintedRect` is
/// what a client assembling its own plan would have to name to paint one.
const HUD_DRAWING_GUARD: Guard = Guard {
    roots: &["src"],
    needles: &[
        "compose_hud",
        "hud::compose",
        "PaintedRect",
        "record_terrain",
        "TerrainRenderer",
    ],
};

/// What a scan of one guard's roots found.
#[derive(Debug, Default)]
struct Scan {
    files_read: usize,
    hits: Vec<String>,
}

/// What a scan amounts to.
///
/// A verdict rather than two booleans a caller has to remember to check, so that
/// "the scan read nothing" cannot be mistaken for "the scan found nothing" by
/// anybody reading either the code or a failure message.
#[derive(Debug, PartialEq, Eq)]
enum Verdict {
    /// Sources were read, and the HUD is drawn through the one frame call.
    OneEntryPoint,
    /// Nothing was read, so nothing could have been reported.
    ReadNothing,
    /// Every place the client draws a HUD, or a frame, outside that call.
    DrawnElsewhere(Vec<String>),
}

/// The verdict `scanned` amounts to.
fn verdict(scanned: &Scan) -> Verdict {
    if scanned.files_read == 0 {
        return Verdict::ReadNothing;
    }
    if scanned.hits.is_empty() {
        return Verdict::OneEntryPoint;
    }
    Verdict::DrawnElsewhere(scanned.hits.clone())
}

#[test]
fn the_client_draws_no_hud_and_no_frame_outside_its_one_frame_call() -> TestResult {
    let scanned = scan(&crate_root(), &HUD_DRAWING_GUARD)?;

    assert_eq!(
        verdict(&scanned),
        Verdict::OneEntryPoint,
        "the client composes the HUD through one call and records terrain through no other: a \
         second composition draws an element over itself, and a second frame path draws the \
         world with the HUD missing — the drift a frame test cannot see, because the frame test \
         is not the thing that shipped"
    );
    Ok(())
}

#[test]
fn the_same_scan_reports_a_client_source_that_draws_a_hud_outside_that_call() -> TestResult {
    let fixture = TempDir::new()?;
    a_source_that_draws_its_own_hud(fixture.path())?;

    let scanned = scan(fixture.path(), &HUD_DRAWING_GUARD)?;
    let every_needle = HUD_DRAWING_GUARD
        .needles
        .iter()
        .all(|needle| scanned.hits.iter().any(|hit| hit.contains(needle)));
    let reported_where = scanned
        .hits
        .iter()
        .all(|hit| hit.starts_with("src/frame/second_path.rs"));

    assert_eq!(
        (
            scanned.files_read,
            scanned.hits.len(),
            every_needle,
            reported_where
        ),
        (1, HUD_DRAWING_GUARD.needles.len(), true, true),
        "the scan has to walk into a nested source directory, read the file it finds there, and \
         report every spelling of drawing a HUD outside the one frame call — one hit per needle, \
         each naming the whole path it sits at: {:?}",
        scanned.hits
    );
    Ok(())
}

#[test]
fn a_scan_that_read_no_client_source_refuses_rather_than_reporting_no_occurrences() -> TestResult {
    let fixture = TempDir::new()?;

    let scanned = scan(fixture.path(), &HUD_DRAWING_GUARD)?;

    assert_eq!(
        (scanned.files_read, scanned.hits.len(), verdict(&scanned)),
        (0, 0, Verdict::ReadNothing),
        "a scan that read no source found no offence for a reason that has nothing to do with \
         the client: the sources moved, the walk broke, or the roots are wrong. Reporting \
         nothing there is how a guard goes green forever"
    );
    Ok(())
}

/// A client tree that draws a HUD everywhere but through its one frame call,
/// written under `root`.
///
/// One file, nested a directory deep so a walk that stopped at the top level is
/// reported rather than looking healthy, and naming **every** needle the guard
/// carries so a mistyped one is caught here rather than standing unwatched.
fn a_source_that_draws_its_own_hud(root: &Path) -> Result<(), Box<dyn Error>> {
    let sources = root.join("src").join("frame");
    fs::create_dir_all(&sources)?;
    fs::write(
        sources.join("second_path.rs"),
        "let mut second: TerrainRenderer = TerrainRenderer::new(device, queue, config)?;\n\
         second.record_terrain(target, &phase, &snapshot)?;\n\
         let planned: Vec<PaintedRect> = mc_render::hud::compose(&frame, size, &layers);\n\
         self.renderer.compose_hud(target, &frame)?;\n",
    )?;
    Ok(())
}

/// This crate's own directory, which every root above is relative to.
fn crate_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

/// Reads every production source under `guard`'s roots and reports each place
/// one of its needles is named.
fn scan(crate_root: &Path, guard: &Guard) -> Result<Scan, Box<dyn Error>> {
    let mut scanned = Scan::default();
    for root in guard.roots {
        let directory = crate_root.join(root);
        if directory.is_dir() {
            walk(&directory, crate_root, guard, &mut scanned)?;
        }
    }
    Ok(scanned)
}

fn walk(
    directory: &Path,
    crate_root: &Path,
    guard: &Guard,
    scanned: &mut Scan,
) -> Result<(), Box<dyn Error>> {
    for entry in fs::read_dir(directory)? {
        let path = entry?.path();
        if path.is_dir() {
            walk(&path, crate_root, guard, scanned)?;
        } else if is_production_source(&path) {
            read(&path, crate_root, guard, scanned)?;
        }
    }
    Ok(())
}

/// Reads one file and records every needle its production text names.
fn read(
    path: &Path,
    crate_root: &Path,
    guard: &Guard,
    scanned: &mut Scan,
) -> Result<(), Box<dyn Error>> {
    let relative = relative_spelling(path, crate_root)?;
    let text = production_text(&fs::read_to_string(path)?);
    scanned.files_read += 1;
    for needle in guard.needles {
        if text.contains(needle) {
            scanned.hits.push(format!("{relative} names `{needle}`"));
        }
    }
    Ok(())
}

/// A `.rs` file that is not a sibling unit-test file.
///
/// Unit tests live beside the code they test, so skipping them is a file-name
/// filter rather than a parse.
fn is_production_source(path: &Path) -> bool {
    path.file_name()
        .and_then(OsStr::to_str)
        .is_some_and(|file_name| file_name.ends_with(".rs") && !file_name.ends_with("_test.rs"))
}

/// A file's text with its doc comments removed.
///
/// A rustdoc example is a doc test, so prose about the frame call — or about
/// what used to be recorded and no longer is — is not a second frame path.
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

/// Where a file sits relative to the crate root, spelled with `/` on every
/// platform so a path can be compared whole.
fn relative_spelling(path: &Path, crate_root: &Path) -> Result<String, Box<dyn Error>> {
    Ok(path
        .strip_prefix(crate_root)?
        .components()
        .filter_map(|component| component.as_os_str().to_str())
        .collect::<Vec<_>>()
        .join("/"))
}
