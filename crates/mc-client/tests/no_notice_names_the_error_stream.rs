//! Nothing in this crate writes to a process stream by name, except the one
//! place that chooses which stream everything goes to.
//!
//! # Why a scan, and why it is the only instrument that covers all nine
//!
//! Nine non-fatal notices called `eprintln!` directly, and the count was wrong
//! every time anybody took it — four in the issue, seven in the as-built record,
//! eight in the spec, nine in the tree once this spec's own Defect 2 added one.
//! **A hand-maintained list of these sites is the thing that keeps failing**, so
//! what guards them is a scan for the spelling rather than a list of the places
//! it may appear.
//!
//! Four of the nine are free functions a test can call with a sink and read back
//! — `src/notice_test.rs` does exactly that. The other five need a window: three
//! reporters and a reload refusal live on `App`, behind a `wgpu::Surface` nothing
//! in this workspace constructs, and the sixth is a `PointerPlatform` the session
//! boxes. For those, this is the whole of the evidence that they go through the
//! sink, and it is worth being plain about what that does and does not buy:
//! **it catches a site going back to the stream, and it cannot catch a site that
//! stops saying anything at all.** That residual is recorded in the spec rather
//! than papered over.
//!
//! # An enumerated verdict, not an absence
//!
//! `hits.is_empty()` cannot tell a clean crate from a walk that broke, a filter
//! that skipped every file, or a source root that has moved — and this scan runs
//! over a directory whose name is a constant, so all three are reachable by
//! ordinary edits. The answer is one of four verdicts and each reading compares
//! the whole of it, which rejects every other one, "I could not look" included.
//!
//! # The positive control is a fixture, not a claim about this crate
//!
//! `reporting_seam.rs`'s shape: the same scan is pointed at a temporary directory
//! holding a file that *does* write to the stream, and has to report it. A scan
//! whose recogniser silently stopped matching would otherwise pass this crate
//! forever, on the day somebody put an `eprintln!` back.
//!
//! # `main.rs` names the stream, once, and that is the design
//!
//! One place decides where everything goes. The scan does not exempt it — an
//! exemption list is how the defect it guards against survived elsewhere — it
//! *requires* it: a crate in which nothing names a stream at all is a crate whose
//! notices go nowhere, and that verdict is a failure here rather than a pass.

use std::error::Error;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

type TestResult = Result<(), Box<dyn Error>>;

/// The production sources this reads, below the crate root.
const SOURCES: &str = "src";

/// The one file that may name a stream, and the spelling it names it by.
const THE_COMPOSITION_ROOT: &str = "main.rs";
const NAMES_THE_STREAM: &str = "io::stderr()";

/// The spellings that write to a process stream without going through the sink.
///
/// `println!` is here beside `eprintln!` because the notice this spec replaced
/// was a `println!` — a client that put a notice back on standard output would be
/// out of reach of a guard that watched only the error stream, and a person
/// piping the client's output past a pager loses it either way.
const WRITES_TO_A_STREAM: [&str; 4] = ["eprintln!", "eprint!", "println!", "print!"];

/// What the scan found.
#[derive(Debug, PartialEq, Eq)]
enum Verdict {
    /// Every notice goes through the sink, and one place names the stream.
    OneFileNamesTheStreamAndNothingWritesToOne,
    /// These files write to a stream directly, each with the spelling found.
    WritesToAStream(Vec<String>),
    /// Nothing names a stream at all, so the notices go nowhere.
    NoFileNamesTheStream,
    /// The walk read no source at all — a moved root, or a filter that skipped
    /// everything.
    NoSourceWasRead,
}

#[test]
fn no_production_source_in_this_crate_writes_to_a_process_stream() -> TestResult {
    let verdict = verdict_over(&sources()?)?;

    assert_eq!(
        verdict,
        Verdict::OneFileNamesTheStreamAndNothingWritesToOne,
        "every non-fatal notice this client emits goes through the sink its caller supplied, which \
         is what lets a harness read one, a caller route one elsewhere and anybody silence the \
         lot. Nine sites wrote to the error stream by name and nothing could do any of the three. \
         `main.rs` names the stream once and hands it in, which is why `NoFileNamesTheStream` is a \
         failure here and not a cleaner answer"
    );
    Ok(())
}

#[test]
fn the_same_scan_reports_a_source_that_writes_to_a_stream() -> TestResult {
    let elsewhere = tempfile::tempdir()?;
    fs::write(
        elsewhere.path().join(THE_COMPOSITION_ROOT),
        NAMES_THE_STREAM,
    )?;
    fs::write(
        elsewhere.path().join("chatty.rs"),
        "fn say() {\n    eprintln!(\"mycraft: straight to the stream\");\n}\n",
    )?;

    assert_eq!(
        verdict_over(elsewhere.path())?,
        Verdict::WritesToAStream(vec!["chatty.rs names `eprintln!`".to_owned()]),
        "the control, and without it the reading above goes green forever the day the recogniser \
         stops matching — which is the same day somebody puts an `eprintln!` back and nothing \
         says so"
    );
    Ok(())
}

#[test]
fn a_scan_that_read_no_source_says_so_rather_than_reporting_a_clean_crate() -> TestResult {
    let empty = tempfile::tempdir()?;

    assert_eq!(
        verdict_over(empty.path())?,
        Verdict::NoSourceWasRead,
        "a directory that has moved reads exactly like a crate with nothing wrong in it, and a \
         scan that could not look must never answer the question it was asked"
    );
    Ok(())
}

/// What the sources under `root` come to.
///
/// # Errors
///
/// Returns an error if the tree cannot be walked or a file cannot be read.
fn verdict_over(root: &Path) -> Result<Verdict, Box<dyn Error>> {
    let mut read = 0_usize;
    let mut writing = Vec::new();
    let mut names_the_stream = false;
    for source in rust_sources(root)? {
        read += 1;
        let text = production_text(&fs::read_to_string(&source)?);
        let named = source
            .file_name()
            .and_then(OsStr::to_str)
            .unwrap_or_default()
            .to_owned();
        if text.contains(NAMES_THE_STREAM) {
            names_the_stream = true;
        }
        writing.extend(
            WRITES_TO_A_STREAM
                .into_iter()
                .filter(|spelling| names(&text, spelling))
                .map(|spelling| format!("{named} names `{spelling}`")),
        );
    }
    writing.sort();
    Ok(match (read, writing.is_empty(), names_the_stream) {
        (0, _, _) => Verdict::NoSourceWasRead,
        (_, false, _) => Verdict::WritesToAStream(writing),
        (_, true, false) => Verdict::NoFileNamesTheStream,
        (_, true, true) => Verdict::OneFileNamesTheStreamAndNothingWritesToOne,
    })
}

/// Whether `text` names `spelling` as a macro of its own.
///
/// **The character before it has to end a name**, and the positive control is
/// what found that: `eprintln!` *contains* `println!`, so a plain substring test
/// reported every site twice and called the error stream the standard one. A
/// recogniser that cannot tell those two apart is worse than none here, because
/// the whole question is which stream a line went to.
fn names(text: &str, spelling: &str) -> bool {
    text.match_indices(spelling).any(|(at, _)| {
        text[..at]
            .chars()
            .next_back()
            .is_none_or(|before| !before.is_alphanumeric() && before != '_')
    })
}

/// `text` with every doc-comment line dropped.
///
/// A module header explaining why the stream is named in one place would
/// otherwise be indistinguishable from a site naming it, and this crate's headers
/// discuss exactly that. Sibling `*_test.rs` files are skipped in
/// [`rust_sources`] for the same reason: a test may spell anything it likes.
fn production_text(text: &str) -> String {
    text.lines()
        .filter(|line| {
            let trimmed = line.trim_start();
            !trimmed.starts_with("///") && !trimmed.starts_with("//!")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Every production `.rs` file under `root`, sibling unit-test files excluded.
///
/// # Errors
///
/// Returns an error if a directory cannot be read.
fn rust_sources(root: &Path) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    let mut found = Vec::new();
    if !root.is_dir() {
        return Ok(found);
    }
    for entry in fs::read_dir(root)? {
        let path = entry?.path();
        if path.is_dir() {
            found.extend(rust_sources(&path)?);
            continue;
        }
        let named = path
            .file_name()
            .and_then(OsStr::to_str)
            .unwrap_or_default()
            .to_owned();
        if named.ends_with(".rs") && !named.ends_with("_test.rs") {
            found.push(path);
        }
    }
    found.sort();
    Ok(found)
}

/// This crate's own production sources.
///
/// # Errors
///
/// Returns an error if the manifest directory cannot be read.
fn sources() -> Result<PathBuf, Box<dyn Error>> {
    Ok(Path::new(env!("CARGO_MANIFEST_DIR")).join(SOURCES))
}
