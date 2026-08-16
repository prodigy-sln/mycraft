//! Nothing in this crate turns a failure into text of its own.
//!
//! Every refusal a player or a mod author reads is a failure and every failure
//! beneath it, rendered in one place. The defect that made this worth guarding was
//! the opposite: a typed failure flattened with `to_string()` at the moment it was
//! reported, so the cause naming the file, the block and the field existed in the
//! value and never reached the terminal — with every behavioural test green,
//! because each of them asked the value rather than the print.
//!
//! # What this guard proves, and what it does not
//!
//! The first two needles — the raw `Ending::Failed` spelling and `.to_string()` —
//! carry the invariant together with `#[non_exhaustive]` on that variant: a
//! reported failure cannot be *composed* in this crate at all, because the
//! compiler refuses the struct literal and the only remaining way to flatten a
//! failure by hand is the call the second needle names.
//!
//! **The last three needles are weaker and are a different kind of thing.** They
//! are a naming-convention guard over a narrow residual hole: a site that handed
//! `failed_under` a context it had built by interpolating an error under some
//! *other* binding name — `{trouble}`, say — commits the offence and escapes every
//! needle here. That hole is narrow and it is real, and it is written down rather
//! than papered over. A guard claiming a totality it does not have is this
//! feature's own defect one level up: something that reads as evidence and is not.
//!
//! It is also blind in a second direction, and its other half lives in
//! `tests/shipped_binary.rs`. A scan can say nothing composes a report; it cannot
//! say a report is ever reached and printed. A `main` that dropped its `report`
//! call entirely passes this file with the whole suite green.
//!
//! # An enumerated verdict, not an absence
//!
//! `hits.is_empty()` cannot tell a clean tree from a walk that broke, a filter
//! that skipped everything, or a root that has moved. So the scan answers with one
//! of three verdicts and the tests compare against the whole answer, which rejects
//! every other one — "I could not look" included — for free.
//!
//! # Shape
//!
//! `tests/seam_boundaries.rs`'s: a file's production text with its doc comments
//! removed, sibling `*_test.rs` unit files skipped, and a `tempfile` fixture as the
//! positive control. The exemption slot is filled in and holds nothing, which is
//! the point of Decision 6 rather than an oversight — a guard whose scope is a
//! hand-maintained list of permitted sites goes green with one more entry on the
//! day a new site stops reporting, and that is how the original defect survived.

use std::error::Error;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

type TestResult = Result<(), Box<dyn Error>>;

/// One text guard: where it reads, what it passes over, and what it refuses to
/// find.
struct Guard {
    /// Directories to walk, each relative to the crate root.
    roots: &'static [&'static str],
    /// Whether a file is passed over, judged on its whole path relative to the
    /// crate root in `/`-separated spelling — never on its bare name.
    exempt: fn(&str) -> bool,
    /// The spellings whose presence in production text is the offence.
    needles: &'static [&'static str],
}

/// One place a needle was named.
#[derive(Debug, PartialEq, Eq)]
struct Site {
    /// Where it sits, relative to the crate root, `/`-separated on every platform.
    file: String,
    /// The spelling that gave it away.
    names: String,
}

/// What a walk of one guard's roots found, before it is turned into a verdict.
#[derive(Debug, Default)]
struct Scan {
    files_read: usize,
    sites: Vec<Site>,
}

/// What the scan came to.
///
/// Three answers rather than a list and a count, because "nothing was found" and
/// "nothing could be looked at" must never compare equal.
#[derive(Debug, PartialEq, Eq)]
enum Verdict {
    /// No file under the roots composes a report of its own.
    EveryReportedFailureIsRenderedByTheRenderer,
    /// These places do.
    ComposedItsOwnReport(Vec<Site>),
    /// No production source was read at all, so nothing above could be said.
    NoSourceWasRead,
}

/// Every production source of this crate, with nothing exempt.
///
/// **`exempt: |_| false`, and that is the decision rather than a default.** After
/// the reporting moved out of this crate there is nothing left in it with a
/// legitimate reason to turn a failure into text, so the one guard whose whole
/// purpose is having no exemption list has none.
///
/// The needles, and what each is for:
///
/// - `Ending::Failed` — the raw variant spelling. Constructing it outside
///   `mc-render` is already a compile error; this catches the day somebody
///   re-opens the door.
/// - `.to_string()` — the flattening itself, which is where the chain was lost.
/// - `{failure}`, `{cause}`, `{refused}` — the three bindings this tree names an
///   error by. See the header: these three watch a naming convention, not an
///   invariant.
const REPORTING_GUARD: Guard = Guard {
    roots: &["src"],
    exempt: |_| false,
    needles: &[
        "Ending::Failed",
        ".to_string()",
        "{failure}",
        "{cause}",
        "{refused}",
    ],
};

#[test]
fn every_failure_this_client_reports_is_rendered_by_the_one_renderer() -> TestResult {
    let scanned = verdict_over(&crate_root(), &REPORTING_GUARD)?;

    assert_eq!(
        scanned,
        Verdict::EveryReportedFailureIsRenderedByTheRenderer,
        "a site that turns a failure into text here reports the outermost sentence and drops \
         everything beneath it — which is the whole defect: the file, the block and the field a \
         mod author needs are in the value and never reach the terminal, while every test that \
         asks the value stays green"
    );
    Ok(())
}

/// The control for the guard above, in three directions at once.
///
/// A walk that broke, a filter that skipped everything, or a needle that matches
/// nothing even when the offence is committed would all report a clean tree
/// forever. So the offending file names **every** needle the guard carries rather
/// than one of them, and the expected sites are derived from the needle list — a
/// needle added without a fixture to catch it fails here rather than standing
/// unwatched.
///
/// The third direction is the `*_test.rs` skip. A sibling unit file naming the
/// same spellings must be passed over, and comparing the whole verdict is what
/// says so: a scan that read it would report sites this expectation does not hold.
#[test]
fn the_same_scan_reports_a_source_file_that_composes_a_report_of_its_own() -> TestResult {
    let fixture = tempfile::tempdir()?;
    a_tree_that_composes_its_own_report(fixture.path())?;

    let scanned = verdict_over(fixture.path(), &REPORTING_GUARD)?;

    assert_eq!(
        scanned,
        Verdict::ComposedItsOwnReport(every_needle_named_in("src/report.rs")),
        "the scan has to reach into the source directory, read the file that flattens a failure \
         and interpolates one under each name this tree uses, report every one of them, and pass \
         over the sibling unit file that says the same words"
    );
    Ok(())
}

#[test]
fn a_scan_pointed_at_a_source_root_that_is_not_there_reports_that_it_read_nothing() -> TestResult {
    let nowhere = tempfile::tempdir()?;

    let scanned = verdict_over(nowhere.path(), &REPORTING_GUARD)?;

    assert_eq!(
        scanned,
        Verdict::NoSourceWasRead,
        "a scan with nothing to read must not answer the same way as a scan that read the tree \
         and found it clean; a source root that has moved or gone is the way this guard stops \
         being able to look, and it has to say so"
    );
    Ok(())
}

/// One site per needle, all in `file`, in the order the scan reports them.
///
/// Derived from the guard's own list rather than written out, so the expectation
/// cannot fall behind the needles it is expecting.
fn every_needle_named_in(file: &str) -> Vec<Site> {
    REPORTING_GUARD
        .needles
        .iter()
        .map(|needle| Site {
            file: file.to_owned(),
            names: (*needle).to_owned(),
        })
        .collect()
}

/// The offending tree the control above scans, written under `root`.
///
/// Two files. `report.rs` commits every needle: it builds the reported variant by
/// hand out of a flattened failure, then says an error under each of the three
/// names this crate binds one to. `report_test.rs` says the same words and must be
/// passed over, because a sibling unit file is not production text.
fn a_tree_that_composes_its_own_report(root: &Path) -> Result<(), Box<dyn Error>> {
    let sources = root.join("src");
    fs::create_dir_all(&sources)?;
    fs::write(
        sources.join("report.rs"),
        "let ending = Ending::Failed { report: failure.to_string() };\n\
         eprintln!(\"mycraft: a frame was dropped: {failure}\");\n\
         eprintln!(\"mycraft: the adapter could not be asked: {cause}\");\n\
         eprintln!(\"mycraft: the world could not be saved: {refused}\");\n",
    )?;
    fs::write(
        sources.join("report_test.rs"),
        "let ending = Ending::Failed { report: failure.to_string() };\n\
         eprintln!(\"a frame was dropped: {failure} {cause} {refused}\");\n",
    )?;
    Ok(())
}

/// This crate's own directory, which every root above is relative to.
fn crate_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

/// Reads every production source under `guard`'s roots and says what it found.
fn verdict_over(crate_root: &Path, guard: &Guard) -> Result<Verdict, Box<dyn Error>> {
    let mut scanned = Scan::default();
    for root in guard.roots {
        let directory = crate_root.join(root);
        if directory.is_dir() {
            walk(&directory, crate_root, guard, &mut scanned)?;
        }
    }
    if scanned.files_read == 0 {
        return Ok(Verdict::NoSourceWasRead);
    }
    if scanned.sites.is_empty() {
        return Ok(Verdict::EveryReportedFailureIsRenderedByTheRenderer);
    }
    Ok(Verdict::ComposedItsOwnReport(scanned.sites))
}

fn walk(
    directory: &Path,
    crate_root: &Path,
    guard: &Guard,
    scanned: &mut Scan,
) -> Result<(), Box<dyn Error>> {
    let mut entries = fs::read_dir(directory)?
        .map(|entry| entry.map(|found| found.path()))
        .collect::<Result<Vec<_>, _>>()?;
    // Read in a settled order, so the sites a failure prints are the same list
    // whatever order the filesystem hands its entries back in.
    entries.sort();
    for path in entries {
        if path.is_dir() {
            walk(&path, crate_root, guard, scanned)?;
        } else if is_production_source(&path) {
            read(&path, crate_root, guard, scanned)?;
        }
    }
    Ok(())
}

/// Reads one file, unless the guard exempts it — an exempt file is not read, so
/// it can neither be reported nor be counted toward the vacuity verdict.
fn read(
    path: &Path,
    crate_root: &Path,
    guard: &Guard,
    scanned: &mut Scan,
) -> Result<(), Box<dyn Error>> {
    let relative = relative_spelling(path, crate_root)?;
    if (guard.exempt)(&relative) {
        return Ok(());
    }
    let text = production_text(&fs::read_to_string(path)?);
    scanned.files_read += 1;
    for needle in guard.needles {
        if text.contains(needle) {
            scanned.sites.push(Site {
                file: relative.clone(),
                names: (*needle).to_owned(),
            });
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
/// A rustdoc example is a doc test, so prose about how a failure used to be
/// flattened is not a flattening.
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
/// platform so a site reads the same wherever the suite runs.
fn relative_spelling(path: &Path, crate_root: &Path) -> Result<String, Box<dyn Error>> {
    Ok(path
        .strip_prefix(crate_root)?
        .components()
        .filter_map(|component| component.as_os_str().to_str())
        .collect::<Vec<_>>()
        .join("/"))
}
