//! The client's own sources reach content through none of the doors that read
//! it.
//!
//! Content is the simulation's. A client never evaluates anything another
//! participant has to agree with, and a content set is the sharpest case there
//! is: a layer index rides inside every packed vertex, so one block the server
//! does not have shifts every index after it and the whole world is textured
//! wrong — silently, with no error anywhere.
//!
//! # Six spellings, and they are chokepoints rather than type names
//!
//! A guard whose needles were type names would go green the day somebody renamed
//! a source, because renaming a source does not rename the door. What these six
//! name is where content actually gets in:
//!
//! - `registry.apply(` — the only way to put a definition into a block registry
//!   at all, which `crates/mc-core/src/block/registry.rs` says about itself.
//! - `HudLayout::load` — the only door into a layout, which
//!   `crates/mc-core/src/hud/layout.rs` says about itself.
//! - `BlockRegistry::new` — where a registry comes into existence, which catches
//!   a client building an empty one to fill by some other route.
//! - `content_root` — where a content directory is resolved, which catches a
//!   client that never reads content but still goes looking for it.
//! - `NotifyContentWatch::watching` — where a content root comes to be *watched*.
//!   A client that builds its own watcher decides for itself when content is read
//!   again, which is the same decision as reading it, taken repeatedly.
//! - `notify` — the vendor behind that watcher, in the one spelling every route
//!   to it shares: the crate's own name, which `notify_debouncer_full` carries
//!   too. `mc_sim::reload::watching_shipped_content` is the one door a client
//!   goes through, and it is what makes both of these absent rather than exempt.
//!
//! **The watcher's half is weaker than the other four and it is worth knowing
//! which way.** `mc-client` does not depend on `notify`, so `use notify::…` would
//! not compile before it matched here — Rust's own extern-crate rules hold that
//! half, exactly as `crates/mc-world/Cargo.toml` records for the crate that does
//! depend on it. What these two needles add over the compiler is the manifest
//! entry *plus* the use arriving together, and the adapter reached through
//! `mc-world`'s re-export, which compiles perfectly well.
//!
//! **The scan wants no exemption at all.** If one ever seems to be needed, that
//! is a door left behind rather than a licence to write the exemption: an
//! exemption on the one door a guard exists to watch is how a guard stops being
//! one.
//!
//! # Known residual, and no text scan closes it
//!
//! Somebody adding a *second* door — a new public registration call somewhere —
//! bypasses every needle here, and a scan that reads spellings cannot see
//! reachability. The instrument that would is a dependency-closure guard
//! asserting that this crate's resolved closure excludes the scripting host, and
//! **that cannot pass while one binary hosts both halves**: a binary's closure is
//! the union of everything inside it, and in singleplayer this binary is also the
//! server. A guard green exactly when the rule is broken is inverted rather than
//! weak, so it belongs to the spec that moves the composition root, and this
//! scan is the weaker instrument carrying the property meanwhile. That is
//! recorded here rather than left for a reader to work out from its absence.
//!
//! # An enumerated verdict, not an absence
//!
//! A scan that read no file, whose walk broke, or whose exemption grew to swallow
//! the tree reports "no door was named" just as loudly as a clean client does. So
//! the answer is one of three, and each test compares the whole of it — which
//! rejects the other two, including the one meaning "there was nothing to look
//! at", for free.
//!
//! # Shape
//!
//! `tests/seam_boundaries.rs` is the shape this follows: a file's production text
//! with its doc comments removed, sibling `*_test.rs` unit files skipped, and a
//! `tempfile` fixture as the positive control. Its own doc comment records why an
//! exemption is compared against the whole path relative to the crate root and
//! never against a bare file name, and that form is kept here even though nothing
//! is exempt — a guard whose exemption is written as a bare name is one rename
//! away from excusing exactly the file it was watching.

use std::error::Error;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

type TestResult = Result<(), Box<dyn Error>>;

/// Where the client's own production sources live, relative to the crate root.
const SOURCES: &str = "src";

/// The six doors content is read or watched through, as they are spelled.
const DOORS: [&str; 6] = [
    "registry.apply(",
    "HudLayout::load",
    "BlockRegistry::new",
    "content_root",
    "NotifyContentWatch::watching",
    "notify",
];

/// What a scan of the client's own sources came to.
#[derive(Debug, PartialEq, Eq)]
enum Verdict {
    /// Every one of the six doors is unnamed in the client's own sources.
    EveryContentDoorIsUnnamed,
    /// These sources name these doors.
    DoorsNamed(Vec<String>),
    /// No source was read at all, so nothing above could be said.
    NoClientSourceWasRead,
}

#[test]
fn the_clients_own_sources_name_none_of_the_doors_content_is_read_through() -> TestResult {
    let verdict = verdict_over(&crate_root())?;

    assert_eq!(
        verdict,
        Verdict::EveryContentDoorIsUnnamed,
        "the simulation loads a content root and the client is handed what came back. A client \
         that resolves the directory, builds the registry, applies a source to it or opens a \
         layout is a client deciding for itself what blocks exist — and two participants deciding \
         that separately texture the whole world wrong with no error anywhere"
    );
    Ok(())
}

/// The control for the guard above, and the only direction it has.
///
/// A walk that broke, a filter that skipped everything, or a needle that stopped
/// matching would report a clean client forever. The fixture names **every** door
/// the guard carries rather than one of them, and the expected report is derived
/// from that list rather than written out: a needle added here without a fixture
/// committing it is a needle nobody has ever watched match anything, and a
/// mistyped one would report a clean scan for as long as it stood there.
#[test]
fn the_same_scan_reports_a_client_source_that_names_a_door_and_says_which_door_it_named()
-> TestResult {
    let fixture = tempfile::tempdir()?;
    let offending = a_source_naming_every_door(fixture.path())?;

    let verdict = verdict_over(fixture.path())?;

    assert_eq!(
        verdict,
        Verdict::DoorsNamed(
            DOORS
                .iter()
                .map(|door| format!("{offending} names `{door}`"))
                .collect()
        ),
        "whoever has to repair a client that reached for content needs the file and the spelling \
         in front of them, and a guard that reported only that something was wrong leaves the \
         repair to be guessed at"
    );
    Ok(())
}

/// The vacuity control, and the reason the verdict is enumerated at all.
///
/// A client whose sources have moved, or a walk that can no longer reach them,
/// finds no door named — which is exactly what a clean client looks like. The two
/// must never compare equal.
#[test]
fn a_scan_that_read_no_client_source_says_so_rather_than_reporting_a_clean_client() -> TestResult {
    let nothing = tempfile::tempdir()?;

    let verdict = verdict_over(nothing.path())?;

    assert_eq!(
        verdict,
        Verdict::NoClientSourceWasRead,
        "an empty answer and an answer nobody could look for are different facts, and a guard \
         that cannot tell them apart goes green the day it stops being able to see"
    );
    Ok(())
}

/// The file filter, in both of its directions at once.
///
/// Unit tests live in a sibling file beside the code they test and may name
/// whatever they are testing; the module beside them may not. A filter that
/// skipped too much would leave the control above green while scanning almost
/// nothing, so the fixture puts the same spelling either side of the filter.
#[test]
fn a_door_named_in_a_sibling_unit_test_file_is_passed_over_and_the_module_beside_it_is_not()
-> TestResult {
    let fixture = tempfile::tempdir()?;
    let sources = fixture.path().join(SOURCES);
    fs::create_dir_all(&sources)?;
    fs::write(
        sources.join("startup_test.rs"),
        "let mut it = BlockRegistry::new();\n",
    )?;
    fs::write(sources.join("startup.rs"), "let root = content_root()?;\n")?;

    let verdict = verdict_over(fixture.path())?;

    assert_eq!(
        verdict,
        Verdict::DoorsNamed(vec![format!("{SOURCES}/startup.rs names `content_root`")]),
        "a unit test naming what it tests is not the client reaching for content, and a filter \
         that swallowed the module beside it would report a clean client having read almost \
         nothing"
    );
    Ok(())
}

/// The doc-comment strip, which this guard cannot do without.
///
/// The client's own sources discuss these doors in prose today — where the
/// content root is looked for, why a layout is built through its loader — and
/// prose about a door is not a use of it. Without the strip the guard would be
/// unsatisfiable for a reason that has nothing to do with the seam, which is the
/// state in which somebody deletes the guard rather than the offence.
#[test]
fn prose_about_a_door_in_a_doc_comment_is_not_a_use_of_it() -> TestResult {
    let fixture = tempfile::tempdir()?;
    let sources = fixture.path().join(SOURCES);
    fs::create_dir_all(&sources)?;
    fs::write(
        sources.join("startup.rs"),
        "//! Where content is read, and by whom.\n\
         /// Answered by `content_root`, and applied with `registry.apply(`.\n\
         pub fn prepared() {}\n",
    )?;

    let verdict = verdict_over(fixture.path())?;

    assert_eq!(
        verdict,
        Verdict::EveryContentDoorIsUnnamed,
        "a client explaining where content comes from is a client documenting the seam, which is \
         the opposite of one crossing it"
    );
    Ok(())
}

/// This crate's own directory, which every path below is relative to.
fn crate_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

/// What the production sources under `crate_root` came to.
///
/// # Errors
///
/// Returns an error if a directory or a file cannot be read — an I/O failure is
/// not one of the three verdicts, for the same reason reading nothing is not
/// "no door was named".
fn verdict_over(crate_root: &Path) -> Result<Verdict, Box<dyn Error>> {
    let mut read = 0_usize;
    let mut named = Vec::new();
    let sources = crate_root.join(SOURCES);
    if sources.is_dir() {
        walk(&sources, crate_root, &mut read, &mut named)?;
    }
    if read == 0 {
        return Ok(Verdict::NoClientSourceWasRead);
    }
    if named.is_empty() {
        return Ok(Verdict::EveryContentDoorIsUnnamed);
    }
    Ok(Verdict::DoorsNamed(named))
}

fn walk(
    directory: &Path,
    crate_root: &Path,
    read: &mut usize,
    named: &mut Vec<String>,
) -> Result<(), Box<dyn Error>> {
    let mut entries: Vec<PathBuf> = fs::read_dir(directory)?
        .map(|entry| entry.map(|found| found.path()))
        .collect::<Result<_, _>>()?;
    // Sorted, so the report a repair is made from is the same on every run
    // whatever order the filesystem hands its entries back in.
    entries.sort();
    for path in entries {
        if path.is_dir() {
            walk(&path, crate_root, read, named)?;
        } else if is_production_source(&path) {
            read_source(&path, crate_root, read, named)?;
        }
    }
    Ok(())
}

/// Reads one source and records every door it names.
fn read_source(
    path: &Path,
    crate_root: &Path,
    read: &mut usize,
    named: &mut Vec<String>,
) -> Result<(), Box<dyn Error>> {
    let relative = relative_spelling(path, crate_root)?;
    let text = production_text(&fs::read_to_string(path)?);
    *read += 1;
    for door in DOORS {
        if text.contains(door) {
            named.push(format!("{relative} names `{door}`"));
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
/// platform so a report reads the same everywhere.
fn relative_spelling(path: &Path, crate_root: &Path) -> Result<String, Box<dyn Error>> {
    Ok(path
        .strip_prefix(crate_root)?
        .components()
        .filter_map(|component| component.as_os_str().to_str())
        .collect::<Vec<_>>()
        .join("/"))
}

/// A client source naming every door the guard carries, written under `root`,
/// and where it sits.
///
/// # Errors
///
/// Returns an error if the directory or the file cannot be written.
fn a_source_naming_every_door(root: &Path) -> Result<String, Box<dyn Error>> {
    let sources = root.join(SOURCES);
    fs::create_dir_all(&sources)?;
    fs::write(
        sources.join("startup.rs"),
        "use notify::RecursiveMode;\n\
         let root = content_root()?;\n\
         let mut registry = BlockRegistry::new();\n\
         registry.apply(&source)?;\n\
         let hud = HudLayout::load(&source)?;\n\
         let watch = NotifyContentWatch::watching(&root);\n",
    )?;
    Ok(format!("{SOURCES}/startup.rs"))
}
