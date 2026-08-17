//! The pipeline every golden frame is shot through watches no content root.
//!
//! A golden frame is a claim about **one** content set: this world, meshed
//! against these declarations, packed against these layers, and committed as the
//! picture that content produces. A capture run that could re-read content while
//! it ran would commit an image whose ground truth is whatever happened to be on
//! disk between two frames — and it would do so silently, because a reload that
//! lands mid-capture produces a *correct* picture of a *different* content set.
//! No pixel comparison can see that; nothing in the pipeline would report it; and
//! the mismatch would land on whoever next edits a declaration.
//!
//! `prepare_scene` therefore gains no watcher, and neither does anything the
//! goldens reach it through.
//!
//! # A different root from `client_names_no_content_door.rs`, not an extension
//! of it
//!
//! That guard reads the client's **production** sources and asks whether they
//! reach content at all. This one reads the **capture pipeline's** sources — the
//! one statement of the capture sequence, the suites that shoot the committed
//! frames, the fixtures they compose through, and the harness that captures and
//! compares them — and asks a narrower question of a wider set: does anything on
//! that path *watch*. Most of what it reads is test code, which the other guard
//! deliberately skips, and `crates/mc-testkit` is not `mc-client` at all.
//!
//! # Four spellings, and they are chokepoints rather than type names
//!
//! - `ContentWatch` — the port itself, and every adapter named after it
//!   (`NotifyContentWatch` carries it). Naming the port is how anything comes to
//!   hold a watch without naming a vendor.
//! - `watching_shipped_content` — the one door a client goes through, which is
//!   exactly the door a capture must *not*.
//! - `ContentReload` — what turns a reported change into an attempt. A pipeline
//!   holding one has a reload whether or not anything is watching yet.
//! - `attach_reload` — the only way a session comes to drive one.
//!
//! Not one of them is a source's own name, so renaming a file greens nothing.
//!
//! # The verdict is enumerated, and one arm is "I could not look"
//!
//! A scan whose sources have moved, whose walk broke, or whose file filter grew to
//! skip everything finds no watcher named — which is exactly what a clean pipeline
//! looks like. So every source this reads is **listed**, a listed source that
//! could not be read is its own answer, and the good answer is compared whole:
//! that rejects both other arms, including the one meaning there was nothing to
//! look at.
//!
//! **Two renames inside this spec are why the missing-source arm carries its
//! weight rather than being ceremony.** `crates/mc-client/src/app.rs` became
//! `src/app/mod.rs` and `src/session.rs` became `src/session/mod.rs` while it was
//! being written. A guard listing a file that no longer exists reads a shorter
//! tree every run and says nothing about it.
//!
//! Precedence, stated because it decides what the fixtures expect: a named door
//! is reported ahead of a missing source. Both are red, and the door is the more
//! specific fact.
//!
//! # Shape
//!
//! `tests/client_names_no_content_door.rs`, down to the doc-comment strip, the
//! sibling-unit-test filter and the `/`-spelled relative report. Its own header
//! records why an exemption is never the answer on a door a guard exists to
//! watch, and nothing here is exempt either. What is *not* repeated here are its
//! controls over the strip and the filter: a fixture proving one twice proves it
//! once.

use std::error::Error;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

type TestResult = Result<(), Box<dyn Error>>;

/// Every source of the capture pipeline that is one file, relative to the
/// repository root.
///
/// `startup.rs` holds `prepare_scene`, which its own doc comment calls the one
/// statement of the capture sequence; the three suites are what shoot the
/// committed frames and what holds the launch and the capture to one picture; the
/// three support modules are what those suites compose a frame through.
const PIPELINE_FILES: [&str; 8] = [
    "crates/mc-client/src/startup.rs",
    "crates/mc-client/tests/launch_and_capture_agree.rs",
    "crates/mc-client/tests/terrain_goldens.rs",
    "crates/mc-client/tests/hud_goldens.rs",
    "crates/mc-client/tests/support/goldens.rs",
    "crates/mc-client/tests/support/frames.rs",
    "crates/mc-client/tests/support/hud_frames.rs",
    "crates/mc-client/tests/support/handed.rs",
];

/// The capture harness, whole, because every part of it is on the path from a
/// declared scene to a committed image.
const PIPELINE_TREES: [&str; 1] = ["crates/mc-testkit/src"];

/// The four spellings a watcher is reached through.
const WATCHER_DOORS: [&str; 4] = [
    "ContentWatch",
    "watching_shipped_content",
    "ContentReload",
    "attach_reload",
];

/// What a scan of the capture pipeline's own sources came to.
#[derive(Debug, PartialEq, Eq)]
enum Verdict {
    /// Every listed source was read and none of them names a watcher door.
    NoWatcherDoorIsNamed,
    /// These sources name these doors.
    WatcherDoorsNamed(Vec<String>),
    /// These listed sources could not be read, so nothing above could be said of
    /// them.
    SourcesMissing(Vec<String>),
}

#[test]
fn the_capture_pipelines_own_sources_name_no_door_that_watches_a_content_root() -> TestResult {
    let verdict = verdict_over(&repository_root()?)?;

    assert_eq!(
        verdict,
        Verdict::NoWatcherDoorIsNamed,
        "a golden frame is a claim about one content set, and a capture run that could read \
         content again while it ran would commit a correct picture of a different one — with no \
         mismatch, no error and nothing to attribute it to. The picture a golden asserts is the \
         picture the content on disk when the run started produces, and that is only true while \
         nothing on this path watches"
    );
    Ok(())
}

/// The control for the guard above, and the only direction it has.
///
/// A walk that broke, a filter that skipped everything or a mistyped needle would
/// report a clean pipeline forever. The fixture names **every** door the guard
/// carries rather than one of them, and the expected report is derived from that
/// list rather than written out: a needle added without a fixture committing it is
/// a needle nobody has watched match anything.
#[test]
fn the_same_scan_reports_a_capture_source_that_watches_and_says_which_door_it_named() -> TestResult
{
    let fixture = tempfile::tempdir()?;
    let offending = a_capture_source_naming_every_watcher_door(fixture.path())?;

    let verdict = verdict_over(fixture.path())?;

    assert_eq!(
        verdict,
        Verdict::WatcherDoorsNamed(
            WATCHER_DOORS
                .iter()
                .map(|door| format!("{offending} names `{door}`"))
                .collect()
        ),
        "whoever has to repair a capture pipeline that started watching needs the file and the \
         spelling in front of them. A guard that reported only that something was wrong leaves the \
         repair to be guessed at, and a guard that reports a clean pipeline over a source naming \
         every door is not a guard"
    );
    Ok(())
}

/// The vacuity control, and the reason a source that could not be read is its own
/// answer.
///
/// A pipeline whose sources have moved finds no watcher named — which is exactly
/// what a clean pipeline looks like. The two must never compare equal, and the
/// expectation is derived from the two lists rather than written out, so a source
/// added to either without being reachable here shows up as soon as it is listed.
#[test]
fn a_scan_that_could_not_read_the_pipelines_sources_says_which_ones_rather_than_reporting_clean()
-> TestResult {
    let nothing = tempfile::tempdir()?;

    let verdict = verdict_over(nothing.path())?;

    assert_eq!(
        verdict,
        Verdict::SourcesMissing(every_listed_source()),
        "an empty answer and an answer nobody could look for are different facts. Two files this \
         guard reads were renamed while this spec was being written, and a guard that lists a file \
         which no longer exists reads a shorter tree every run and never mentions it"
    );
    Ok(())
}

/// Every source this guard reads, listed, in the order a report gives them.
fn every_listed_source() -> Vec<String> {
    PIPELINE_FILES
        .iter()
        .chain(PIPELINE_TREES.iter())
        .map(|source| (*source).to_owned())
        .collect()
}

/// This repository's own directory, which every listed source is relative to.
///
/// # Errors
///
/// Returns an error if this crate's manifest directory has no grandparent, which
/// is a crate that has left the workspace.
fn repository_root() -> Result<PathBuf, Box<dyn Error>> {
    Ok(Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .ok_or("this crate is no longer two directories below the repository root")?
        .to_path_buf())
}

/// What the capture pipeline's sources under `repository` came to.
///
/// # Errors
///
/// Returns an error if a directory or a file that is there cannot be read — an
/// I/O failure is not one of the three verdicts, for the same reason a source
/// that is not there is not "no door was named".
fn verdict_over(repository: &Path) -> Result<Verdict, Box<dyn Error>> {
    let mut named = Vec::new();
    let mut missing = Vec::new();
    for source in PIPELINE_FILES {
        let path = repository.join(source);
        if path.is_file() {
            read_source(&path, source, &mut named)?;
        } else {
            missing.push(source.to_owned());
        }
    }
    for tree in PIPELINE_TREES {
        if walked(&repository.join(tree), tree, &mut named)? == 0 {
            missing.push(tree.to_owned());
        }
    }
    if !named.is_empty() {
        return Ok(Verdict::WatcherDoorsNamed(named));
    }
    if !missing.is_empty() {
        return Ok(Verdict::SourcesMissing(missing));
    }
    Ok(Verdict::NoWatcherDoorIsNamed)
}

/// Reads every production source under `directory`, and how many that was.
///
/// Sorted, so the report a repair is made from is the same on every run whatever
/// order the filesystem hands its entries back in.
fn walked(
    directory: &Path,
    spelled: &str,
    named: &mut Vec<String>,
) -> Result<usize, Box<dyn Error>> {
    if !directory.is_dir() {
        return Ok(0);
    }
    let mut entries: Vec<PathBuf> = fs::read_dir(directory)?
        .map(|entry| entry.map(|found| found.path()))
        .collect::<Result<_, _>>()?;
    entries.sort();
    let mut read = 0;
    for path in entries {
        let Some(file_name) = path.file_name().and_then(OsStr::to_str) else {
            continue;
        };
        let under = format!("{spelled}/{file_name}");
        if path.is_dir() {
            read += walked(&path, &under, named)?;
        } else if is_production_source(file_name) {
            read_source(&path, &under, named)?;
            read += 1;
        }
    }
    Ok(read)
}

/// Reads one source and records every watcher door it names.
fn read_source(path: &Path, spelled: &str, named: &mut Vec<String>) -> Result<(), Box<dyn Error>> {
    let text = production_text(&fs::read_to_string(path)?);
    for door in WATCHER_DOORS {
        if text.contains(door) {
            named.push(format!("{spelled} names `{door}`"));
        }
    }
    Ok(())
}

/// A `.rs` file that is not a sibling unit-test file.
fn is_production_source(file_name: &str) -> bool {
    file_name.ends_with(".rs") && !file_name.ends_with("_test.rs")
}

/// A file's text with its doc comments removed.
///
/// The pipeline's sources discuss what a capture is a claim about, and prose about
/// a watcher is not a use of one. Ordinary `//` comments are deliberately left in:
/// a line of code commented out is still a line somebody meant to run.
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

/// A capture-pipeline source naming every watcher door the guard carries, written
/// under `repository`, and where it sits.
///
/// It is written at the first listed file's own path, so the fixture exercises the
/// same listing the real run reads rather than a path invented beside it.
///
/// # Errors
///
/// Returns an error if the directory or the file cannot be written.
fn a_capture_source_naming_every_watcher_door(repository: &Path) -> Result<String, Box<dyn Error>> {
    let spelled = PIPELINE_FILES[0];
    let path = repository.join(spelled);
    let parent = path
        .parent()
        .ok_or("the first listed pipeline source has no directory to write it into")?;
    fs::create_dir_all(parent)?;
    fs::write(
        &path,
        "let watch: Box<dyn ContentWatch> = Box::new(watching);\n\
         let reload = watching_shipped_content(root.clone());\n\
         let holding: ContentReload = reload;\n\
         session.attach_reload(holding);\n",
    )?;
    Ok(spelled.to_owned())
}
