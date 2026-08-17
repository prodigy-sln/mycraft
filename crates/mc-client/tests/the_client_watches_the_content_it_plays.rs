//! The shipped client watches the content root it is playing.
//!
//! # Why an absence guard's mirror image is owed, and why it is owed here
//!
//! `tests/client_names_no_content_door.rs` asserts that the client's own sources
//! name none of the doors content is read or watched through. That guard is
//! satisfied just as well by a client that **never reloads at all** — and at the
//! head this was written against, that is exactly the client this workspace ships:
//! `Session::attach_reload` has no caller outside test fixtures,
//! `App::collect_preparation` attaches a simulation and spawns the re-mesh worker
//! and attaches no reload, and `main::run` hands its content root to the
//! preparation and keeps no copy. Every reload scenario in this spec is green and
//! `cargo run -p mc-client` watches nothing.
//!
//! What that costs is the capability the specification opens with: leave the client
//! running, edit `content/base/blocks/stone.luau`, save, and walk through stone.
//! None of it is reachable by the person it is for.
//!
//! # Why this is a scan, and what would be better
//!
//! A behavioural reading is unavailable: the wiring belongs to
//! `crates/mc-client/src/app`, which needs a real window, and nothing in this
//! workspace constructs one. A compiler-held obligation would be better still —
//! that is how the reloaded texture upload was closed, once a mutation showed that
//! deleting it left 234 of 234 green — but an obligation needs a value the wiring
//! must consume, and a session that is simply never told to watch consumes nothing.
//! A source-level scan beats no instrument at all for a silent, player-visible
//! absence, and it is the shape this phase already builds three of.
//!
//! # Two spellings, and they are the chokepoints the other guard leaves open
//!
//! - `watching_shipped_content(` — the one door a client goes through, and the
//!   reason the sibling guard's watcher needles are absent rather than exempt. A
//!   client reaching the adapter any other way reddens *there*.
//! - `.attach_reload(` — the only way a session comes to drive a reload at its tick
//!   boundaries. A client that builds a watcher and drops it names the first and
//!   not the second.
//!
//! **Both are calls and neither is a name, and the first draft of this guard got
//! that wrong.** `attach_reload` on its own matched `pub fn attach_reload` — the
//! method's own definition, which is in this crate — so the needle was satisfied by
//! the very declaration whose caller is missing. Written as a call it cannot be:
//! a definition carries no receiver before it. That is the same reason the sibling
//! guard spells its own registry door `registry.apply(`.
//!
//! # A presence assertion needs no positive control, and does need the other one
//!
//! A needle that stopped matching reddens this immediately — which is the whole
//! difference from an absence guard, and why no fixture here has to prove that a
//! spelling can be found. What it does need is the arm that separates "no source
//! names it" from "no source was read at all": a walk that has lost the tree it
//! reads answers the first while meaning the second.

use std::error::Error;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

type TestResult = Result<(), Box<dyn Error>>;

/// Where the client's own production sources live, relative to the crate root.
const SOURCES: &str = "src";

/// The two spellings that put a content root under watch for a run.
const WIRING: [&str; 2] = ["watching_shipped_content(", ".attach_reload("];

/// What a scan of the client's own sources came to.
#[derive(Debug, PartialEq, Eq)]
enum Verdict {
    /// The client reaches the reload through the one door and hands it to a
    /// session.
    TheReloadIsDrivenFromTheOneDoor,
    /// No production source names these.
    NotReached(Vec<String>),
    /// No source was read at all, so nothing above could be said.
    NoClientSourceWasRead,
}

#[test]
fn the_clients_own_sources_put_the_root_it_plays_under_watch() -> TestResult {
    let verdict = verdict_over(&crate_root())?;

    assert_eq!(
        verdict,
        Verdict::TheReloadIsDrivenFromTheOneDoor,
        "hot reload exists so that a mod author can edit a declaration while the game is running, \
         and a client that never puts its content root under watch delivers none of it however \
         complete the machinery behind it is. The whole reload path can be correct, tested and \
         unreachable at the same time — which is what it is until something in the client's own \
         sources reaches `watching_shipped_content` and hands the result to its session"
    );
    Ok(())
}

/// The control that separates a client which does not watch from a scan that could
/// not look.
///
/// A walk whose sources have moved names nothing — which reads exactly like a
/// client that reaches for nothing. The two must never compare equal, and the
/// failure a reader is shown must be the right one of the two.
#[test]
fn a_scan_that_read_no_client_source_says_so_rather_than_reporting_an_unwatched_root() -> TestResult
{
    let nothing = tempfile::tempdir()?;

    let verdict = verdict_over(nothing.path())?;

    assert_eq!(
        verdict,
        Verdict::NoClientSourceWasRead,
        "an answer nobody could look for is not an answer, and this guard's two failures ask for \
         opposite repairs: one is wiring a reload up, the other is a walk that has lost the tree it \
         reads"
    );
    Ok(())
}

/// The control that says a red here is about the tree rather than about a mistyped
/// needle.
///
/// It earns its place only while the assertion above is red: a presence assertion
/// that has gone green has proved its own needles match. Until then, this is what
/// tells whoever reads the failure that the two spellings are findable and the
/// client simply does not spell them.
#[test]
fn the_same_scan_reports_a_client_source_that_does_put_its_root_under_watch() -> TestResult {
    let fixture = tempfile::tempdir()?;
    let sources = fixture.path().join(SOURCES);
    fs::create_dir_all(&sources)?;
    fs::write(
        sources.join("app.rs"),
        "let watching = watching_shipped_content(root.clone());\n\
         session.attach_reload(watching);\n",
    )?;

    let verdict = verdict_over(fixture.path())?;

    assert_eq!(
        verdict,
        Verdict::TheReloadIsDrivenFromTheOneDoor,
        "a guard that cannot recognise the wiring when it is right in front of it would report a \
         missing reload forever, and whoever wired one up would have no way to tell that from a \
         guard that had stopped working"
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
/// Returns an error if a directory or a file cannot be read — an I/O failure is not
/// one of the three verdicts, for the same reason reading nothing is not "the client
/// does not watch".
fn verdict_over(crate_root: &Path) -> Result<Verdict, Box<dyn Error>> {
    let mut read = 0_usize;
    let mut named = Vec::new();
    let sources = crate_root.join(SOURCES);
    if sources.is_dir() {
        walk(&sources, &mut read, &mut named)?;
    }
    if read == 0 {
        return Ok(Verdict::NoClientSourceWasRead);
    }
    let missing: Vec<String> = WIRING
        .iter()
        .filter(|spelling| !named.contains(*spelling))
        .map(|spelling| (*spelling).to_owned())
        .collect();
    if missing.is_empty() {
        return Ok(Verdict::TheReloadIsDrivenFromTheOneDoor);
    }
    Ok(Verdict::NotReached(missing))
}

/// Reads every production source under `directory`, recording which spellings the
/// tree names.
fn walk(
    directory: &Path,
    read: &mut usize,
    named: &mut Vec<&'static str>,
) -> Result<(), Box<dyn Error>> {
    let mut entries: Vec<PathBuf> = fs::read_dir(directory)?
        .map(|entry| entry.map(|found| found.path()))
        .collect::<Result<_, _>>()?;
    entries.sort();
    for path in entries {
        if path.is_dir() {
            walk(&path, read, named)?;
        } else if is_production_source(&path) {
            *read += 1;
            record_wiring(&fs::read_to_string(&path)?, named);
        }
    }
    Ok(())
}

/// Records which of the two spellings one source names.
fn record_wiring(source: &str, named: &mut Vec<&'static str>) {
    let text = production_text(source);
    for spelling in WIRING {
        if text.contains(spelling) {
            named.push(spelling);
        }
    }
}

/// A `.rs` file that is not a sibling unit-test file.
///
/// A unit test naming the wiring is not the client doing it, exactly as the sibling
/// guard's filter has it — and here the filter runs the other way round: a test file
/// counted as production would let a fixture satisfy this for the whole crate.
fn is_production_source(path: &Path) -> bool {
    path.file_name()
        .and_then(OsStr::to_str)
        .is_some_and(|file_name| file_name.ends_with(".rs") && !file_name.ends_with("_test.rs"))
}

/// A file's text with its doc comments removed.
///
/// Prose about watching a content root is not watching one, and this crate's own
/// sources discuss the seam at length — including, once this is green, the very
/// lines that satisfy it.
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
