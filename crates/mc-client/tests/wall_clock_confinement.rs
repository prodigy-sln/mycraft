//! The wall clock is confined to one file, and the scan that says so is asked
//! whether it could have said otherwise.
//!
//! `src/app.rs` states that no wall clock is read anywhere in this client, and
//! that is what makes a fixed replay the same run on a machine drawing 300 frames
//! a second and on one managing 30. The debug overlay has to report a frame rate,
//! which needs a clock. **The two are reconciled by confinement rather than by
//! dropping either**: one file holds the port and its one adapter, every other
//! path takes an interval through that port, and this is where "every other path"
//! stops being a convention.
//!
//! # An absence proves nothing on its own, and here it proves less than usual
//!
//! Both needles occur in **exactly one** production source under the two roots
//! below — the file that is exempt — so this scan has never had a hit and never
//! will while it is right. That is precisely the guard that quietly goes green
//! forever: a walk that broke, a root that moved, an exemption that grew, or a
//! needle mistyped all report a clean tree indistinguishably from a clean tree.
//! So three things are asked rather than one:
//!
//! 1. the tree is clean — asserted as an **exact verdict**, not as an empty list,
//!    so a reading that could not look cannot arrive under the good verdict's
//!    name;
//! 2. the same scan, pointed at a fixture that *does* read a clock, reports it —
//!    and passes over the one file that is allowed to;
//! 3. the refusal in (1) is reachable: a scan that read nothing answers with it.
//!
//! (3) is not covered by (1) for a reason worth stating, because it looks as
//! though it should be. An exact-verdict assertion rejects every verdict meaning
//! "I could not look" — but only if the code can still *produce* one. Sources are
//! read today, so a refusal arm that had become unreachable would leave (1) green,
//! and the day a root moved the answer would be "nothing reads a clock" about a
//! scan that read no file.
//!
//! # Whole paths, never bare names
//!
//! The exemption is compared against the whole path relative to the repository,
//! spelled with `/` on every platform. A bare-name comparison would excuse any
//! future `clock.rs` anywhere under either root — and "the client grew a clock of
//! its own" is exactly the change this guard exists to report. The fixture below
//! carries a file wearing the exempt file's *name* at a different path for that
//! reason.
//!
//! # Doc comments are stripped, and nothing measures that today
//!
//! A file's text is read with its `///` and `//!` lines removed, because prose
//! about a clock is not a use of one — and the exempt file's own documentation
//! says, in as many words, why it takes a monotonic clock rather than a settable
//! one. That file is exempt in any case, so no assertion here can tell the filter
//! from its absence. Recorded rather than dressed up: the day a source under
//! either root explains in prose why it reads no clock, this filter is what keeps
//! it from being reported for saying so.

use std::error::Error;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

type TestResult = Result<(), Box<dyn Error>>;

/// The one production source that may name a wall clock, relative to the
/// repository.
const EXEMPT_FILE: &str = "crates/mc-render/src/overlay/clock.rs";

/// One text guard: where it reads, what it passes over, and what it refuses to
/// find.
#[derive(Debug)]
struct Guard {
    /// Directories to walk, each relative to the repository root.
    roots: &'static [&'static str],
    /// Whether a file is passed over, judged on its whole path relative to the
    /// repository in `/`-separated spelling — never on its bare name.
    exempt: fn(&str) -> bool,
    /// The spellings whose presence in production text is the offence.
    needles: &'static [&'static str],
}

/// Nothing the client or the renderer ships names a wall clock, save the one file
/// that is the clock.
///
/// Two roots because the confinement is one claim about two crates: the client is
/// where a frame path would reach for elapsed time, and the renderer is where the
/// overlay that legitimately needs it lives. The bare type names rather than
/// their `std::time::` spelling, so an aliased import is caught too.
const WALL_CLOCK_GUARD: Guard = Guard {
    roots: &["crates/mc-client/src", "crates/mc-render/src"],
    exempt: |path| path == EXEMPT_FILE,
    needles: &["Instant", "SystemTime"],
};

/// What a scan of the guard's roots found.
#[derive(Debug, Default)]
struct Scan {
    files_read: usize,
    hits: Vec<String>,
}

/// What one scan came to.
///
/// An enumerated answer rather than a list a caller checks for emptiness: an empty
/// list of hits is what a broken walk, a moved root and a clean tree all produce,
/// and only one of those three is good news. Naming the refusals means an
/// assertion of the good one rejects them.
#[derive(Debug, PartialEq, Eq)]
enum Verdict {
    /// Every production source under the roots was read, and none names a clock.
    NoProductionSourceReadsAWallClock,
    /// Nothing was read at all: the roots have moved, or the exemption has grown
    /// to cover them.
    ReadNoProductionSource,
    /// The sources that name one, each with the spelling it names.
    WallClockRead(Vec<String>),
}

/// What the wall-clock guard says about the tree under `root`.
///
/// # Errors
///
/// Returns the read failure when a directory under a root cannot be walked.
fn verdict_over(root: &Path) -> Result<Verdict, Box<dyn Error>> {
    let scanned = scan(root, &WALL_CLOCK_GUARD)?;
    if scanned.files_read == 0 {
        return Ok(Verdict::ReadNoProductionSource);
    }
    if scanned.hits.is_empty() {
        return Ok(Verdict::NoProductionSourceReadsAWallClock);
    }
    Ok(Verdict::WallClockRead(scanned.hits))
}

#[test]
fn no_production_source_of_the_client_or_the_renderer_reads_a_wall_clock_outside_the_overlays_own()
-> TestResult {
    assert_eq!(
        verdict_over(&repository_root()?)?,
        Verdict::NoProductionSourceReadsAWallClock,
        "a replay is evidence only because the client reads no clock: one tick per rendered frame, \
         so the same input leaves the same world behind at 30 frames a second and at 300. The \
         overlay needs elapsed time and takes it through a port, from the one file that is allowed \
         to know what a clock is — anything else naming one is a frame path, a tick or a capture \
         that has started depending on how fast the machine it ran on was, and no test of a \
         *result* can report that"
    );
    Ok(())
}

/// The control for the guard above, in both of its directions at once.
///
/// A walk that broke, a filter that skipped everything or a mistyped needle would
/// report a clean tree forever. The fixture names a clock twice: once in a file
/// wearing the exempt file's own name at a path the exemption does not cover,
/// which has to be reported, and once in the exempt file itself, which has to be
/// passed over. Expecting exactly one hit is what asserts both — a second hit is
/// an exemption that stopped working, and none is a scan that stopped looking.
#[test]
fn the_same_scan_reports_a_source_that_names_a_wall_clock_and_passes_over_the_one_that_may()
-> TestResult {
    let fixture = tempfile::tempdir()?;
    let offending = fixture.path().join("crates/mc-client/src");
    let allowed = fixture.path().join(EXEMPT_FILE);
    fs::create_dir_all(&offending)?;
    fs::create_dir_all(allowed.parent().ok_or("the exempt file has no directory")?)?;
    fs::write(
        offending.join("clock.rs"),
        "use std::time::Instant;\nfn since(started: Instant) -> u128 { started.elapsed().as_nanos() }\n",
    )?;
    fs::write(allowed, "use std::time::Instant;\n")?;

    assert_eq!(
        verdict_over(fixture.path())?,
        Verdict::WallClockRead(vec![
            "crates/mc-client/src/clock.rs names `Instant`".to_owned()
        ]),
        "the scan has to walk into both roots, report the source that reads a clock — wherever it \
         sits and whatever it is called — and pass over the single file whose whole job is to be \
         one. A needle no fixture ever commits is a needle nobody has watched match anything, and \
         an exemption compared by bare name would excuse the very file a client growing a clock of \
         its own would be called"
    );
    Ok(())
}

/// The vacuity guard, and it is a scenario rather than an assertion tucked inside
/// the first for the reason this file's header gives: the refusal has to be
/// *reachable*, and no tree that has sources in it can show that.
#[test]
fn a_wall_clock_scan_that_read_no_production_source_refuses_rather_than_reporting_no_occurrences()
-> TestResult {
    let fixture = tempfile::tempdir()?;

    assert_eq!(
        verdict_over(fixture.path())?,
        Verdict::ReadNoProductionSource,
        "a scan whose roots hold nothing has found nothing, which is not the same as there being \
         nothing to find. Without this refusal the headline claim above reads 'no source names a \
         clock' about a walk that opened no file — the day a crate is renamed or a directory moves, \
         the guard goes green and stays green while the confinement it was watching quietly ends"
    );
    Ok(())
}

/// The repository's own root, located upwards from the crate this binary was built
/// for, because one of the two roots below is another crate's.
fn repository_root() -> Result<PathBuf, Box<dyn Error>> {
    Ok(Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .ok_or("the crate manifest directory has no repository root above it")?
        .to_owned())
}

/// Reads every production source under `guard`'s roots and reports each place one
/// of its needles is named.
///
/// A root that does not exist contributes no files rather than an error, which is
/// what leaves the verdict's own refusal — and not an I/O failure — to report a
/// root that has moved or gone.
fn scan(root: &Path, guard: &Guard) -> Result<Scan, Box<dyn Error>> {
    let mut scanned = Scan::default();
    for named in guard.roots {
        let directory = root.join(named);
        if directory.is_dir() {
            walk(&directory, root, guard, &mut scanned)?;
        }
    }
    Ok(scanned)
}

fn walk(
    directory: &Path,
    root: &Path,
    guard: &Guard,
    scanned: &mut Scan,
) -> Result<(), Box<dyn Error>> {
    for entry in fs::read_dir(directory)? {
        let path = entry?.path();
        if path.is_dir() {
            walk(&path, root, guard, scanned)?;
        } else if is_production_source(&path) {
            read(&path, root, guard, scanned)?;
        }
    }
    Ok(())
}

/// Reads one file, unless the guard exempts it — an exempt file is not read, so it
/// can neither be reported nor counted toward the vacuity refusal.
fn read(path: &Path, root: &Path, guard: &Guard, scanned: &mut Scan) -> Result<(), Box<dyn Error>> {
    let relative = relative_spelling(path, root)?;
    if (guard.exempt)(&relative) {
        return Ok(());
    }
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
/// Unit tests live beside the code they test and are not a path this client runs,
/// so skipping them is a file-name filter rather than a parse.
fn is_production_source(path: &Path) -> bool {
    path.file_name()
        .and_then(OsStr::to_str)
        .is_some_and(|file_name| file_name.ends_with(".rs") && !file_name.ends_with("_test.rs"))
}

/// A file's text with its doc comments removed. See this file's header.
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

/// Where a file sits relative to `root`, spelled with `/` on every platform so an
/// exemption can be written once and compared whole.
fn relative_spelling(path: &Path, root: &Path) -> Result<String, Box<dyn Error>> {
    Ok(path
        .strip_prefix(root)?
        .components()
        .filter_map(|component| component.as_os_str().to_str())
        .collect::<Vec<_>>()
        .join("/"))
}
