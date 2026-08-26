//! The only advance the client's frame path can name takes an elapsed duration,
//! and the scan that says so is asked whether it could have said otherwise.
//!
//! # What this guards, and what already guards more
//!
//! The per-tick step is private to the client's core, so a frame path that spent
//! one tick per frame — the defect this fix removes — no longer *compiles*. That
//! is the strong guard and it needs no test. What this file adds is the one thing
//! visibility cannot hold: the day somebody makes the step public again for a
//! reason that seems good at the time, the ratio can come back with every
//! behavioural scenario still green, because the frame path is the half of this
//! client no in-process test can construct.
//!
//! # The residual hole, stated plainly rather than dressed up
//!
//! **A scan reads text, and text is not execution.** The only harness that runs
//! the real frame path is `tests/shipped_binary.rs`, which spawns the client as a
//! subprocess — and it cannot be pointed at this. The reason is decisive rather
//! than about cost: the surface takes wgpu's default `Fifo` present mode, so the
//! child's frame rate *is* the display refresh, and a broken client spends
//! `f · T` ticks against a fixed one's `60 · T`. Those two are equal exactly when
//! `f` is 60, which is the commonest configuration there is. Such a test would be
//! green on a 60 Hz panel whether or not the fix worked, red on a headless machine
//! for an unrelated reason, and discriminating only above 60 Hz — green on the
//! likely configurations and red on another is flaky by hardware, and it reads as
//! evidence while being none. A documented hole is worth more than a test that
//! appears to close it.
//!
//! # An absence proves nothing on its own
//!
//! The needle occurs in exactly one production source under the root below — the
//! file that is exempt — so this scan has never had a hit and never will while it
//! is right. That is the guard that quietly goes green forever, so three things
//! are asked rather than one:
//!
//! 1. the tree is clean — asserted as an **exact verdict**, not as an empty list,
//!    so a reading that could not look cannot arrive under the good verdict's
//!    name;
//! 2. the same scan, pointed at a fixture whose frame path *does* advance a tick,
//!    reports it — and passes over the core, whose job that is;
//! 3. the refusal in (1) is reachable: a scan that read nothing answers with it.
//!
//! (3) is not covered by (1), for a reason that looks as though it should make it
//! redundant. An exact-verdict assertion rejects every verdict meaning "I could
//! not look" — but only while the code can still *produce* one. Sources are read
//! today, so a refusal arm that had become unreachable would leave (1) green, and
//! the day the root moved the answer would be "nothing names a tick" about a scan
//! that opened no file.
//!
//! # Whole paths, never bare names
//!
//! The exemption is the core's own file and never its directory. `src/session/`
//! holds a sibling — the pacing — and a prefix would excuse it and everything ever
//! put beside it. It is compared whole for the further reason that the core is a
//! `mod.rs`: judged by name, an exemption on a file called `mod.rs` excuses nearly
//! every directory there is.

use std::error::Error;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

type TestResult = Result<(), Box<dyn Error>>;

/// The one production source of this client that may advance a tick, relative to
/// the crate root.
const THE_CORE: &str = "src/session/mod.rs";

/// The spelling of a per-tick advance.
///
/// The call and not the bare word: a snapshot has a `tick` field, a doc comment
/// says "tick" constantly, and a needle that matched either would be a needle
/// nobody could keep clean. What is forbidden is *asking for one*.
const A_PER_TICK_ADVANCE: &str = ".tick(";

/// Where the scan reads, what it passes over, and what it refuses to find.
#[derive(Debug)]
struct Guard {
    /// Directories to walk, each relative to the crate root.
    roots: &'static [&'static str],
    /// Whether a file is passed over, judged on its whole path relative to the
    /// crate root in `/`-separated spelling — never on its bare name.
    exempt: fn(&str) -> bool,
    /// The spellings whose presence in production text is the offence.
    needles: &'static [&'static str],
}

/// Nothing this client ships advances the simulation a tick at a time, save the
/// core whose job that is.
const FRAME_PACING_GUARD: Guard = Guard {
    roots: &["src"],
    exempt: |path| path == THE_CORE,
    needles: &[A_PER_TICK_ADVANCE],
};

/// What a scan of the guard's root found.
#[derive(Debug, Default)]
struct Scan {
    files_read: usize,
    hits: Vec<String>,
}

/// What one scan came to.
///
/// An enumerated answer rather than a list a caller checks for emptiness: an
/// empty list of hits is what a broken walk, a moved root and a clean tree all
/// produce, and only one of those three is good news.
#[derive(Debug, PartialEq, Eq)]
enum Verdict {
    /// Every production source under the root was read, and the only advance any
    /// of them names takes an elapsed duration.
    EveryAdvanceTakesAnElapsedDuration,
    /// Nothing was read at all: the root has moved, or the exemption has grown to
    /// cover it.
    ReadNoProductionSource,
    /// The sources that ask for a tick, each with the spelling it asks in.
    APerTickAdvanceIsNamed(Vec<String>),
}

/// What the frame-pacing guard says about the tree under `crate_root`.
///
/// # Errors
///
/// Returns the read failure when a directory under the root cannot be walked.
fn verdict_over(crate_root: &Path) -> Result<Verdict, Box<dyn Error>> {
    let scanned = scan(crate_root, &FRAME_PACING_GUARD)?;
    if scanned.files_read == 0 {
        return Ok(Verdict::ReadNoProductionSource);
    }
    if scanned.hits.is_empty() {
        return Ok(Verdict::EveryAdvanceTakesAnElapsedDuration);
    }
    Ok(Verdict::APerTickAdvanceIsNamed(scanned.hits))
}

#[test]
fn no_source_under_the_clients_frame_path_names_a_per_tick_advance() -> TestResult {
    assert_eq!(
        verdict_over(&crate_root())?,
        Verdict::EveryAdvanceTakesAnElapsedDuration,
        "a frame does not buy a tick; a sixtieth of a second does. The client used to advance one \
         tick per rendered frame, which ran the world at the display's rate divided by sixty — \
         right by coincidence at 60 Hz and 2.4× fast at 144, and a player reported it as warping \
         around with super speed. The only door the frame path has takes elapsed time, and \
         anything here asking for a tick instead has put the ratio back where nothing in this \
         workspace executes it"
    );
    Ok(())
}

/// The control for the guard above, in both of its directions at once.
///
/// A walk that broke, a filter that skipped everything or a mistyped needle would
/// report a clean tree forever. The fixture asks for a tick twice: once from a
/// frame path, which has to be reported, and once from the core, which has to be
/// passed over. Expecting exactly one hit is what asserts both — a second is an
/// exemption that stopped working, and none is a scan that stopped looking.
#[test]
fn the_same_scan_reports_a_frame_path_that_advances_a_tick_and_passes_the_core_over() -> TestResult
{
    let fixture = tempfile::tempdir()?;
    let frame_path = fixture.path().join("src/app");
    let core = fixture.path().join(THE_CORE);
    fs::create_dir_all(&frame_path)?;
    fs::create_dir_all(core.parent().ok_or("the core has no directory")?)?;
    fs::write(
        frame_path.join("mod.rs"),
        "self.gpu.queue.present(acquired);\nsession.tick();\n",
    )?;
    fs::write(core, "let edited = self.tick();\n")?;

    assert_eq!(
        verdict_over(fixture.path())?,
        Verdict::APerTickAdvanceIsNamed(vec![format!(
            "src/app/mod.rs names `{A_PER_TICK_ADVANCE}`"
        )]),
        "the scan has to walk into the source tree, report the frame path that asks for a tick — \
         wherever it sits and whatever the file is called — and pass over the one file whose job \
         that is. A needle no fixture ever commits is a needle nobody has watched match anything, \
         and an exemption compared by directory would excuse the pacing sitting beside the core \
         along with everything ever put there afterwards"
    );
    Ok(())
}

/// The vacuity guard, and it is a scenario rather than an assertion tucked inside
/// the first, for the reason this file's header gives: the refusal has to be
/// *reachable*, and no tree that has sources in it can show that.
#[test]
fn a_pacing_scan_that_read_no_production_source_refuses_rather_than_reporting_no_occurrences()
-> TestResult {
    let fixture = tempfile::tempdir()?;

    assert_eq!(
        verdict_over(fixture.path())?,
        Verdict::ReadNoProductionSource,
        "a scan whose root holds nothing has found nothing, which is not the same as there being \
         nothing to find. Without this refusal the headline claim above reads 'no frame path asks \
         for a tick' about a walk that opened no file — the day the crate is laid out differently, \
         the guard goes green and stays green while the pacing it was watching quietly ends"
    );
    Ok(())
}

/// This crate's own directory, which the root above is relative to.
fn crate_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

/// Reads every production source under `guard`'s roots and reports each place one
/// of its needles is named.
///
/// A root that does not exist contributes no files rather than an error, which is
/// what leaves the verdict's own refusal — and not an I/O failure — to report a
/// root that has moved or gone.
fn scan(crate_root: &Path, guard: &Guard) -> Result<Scan, Box<dyn Error>> {
    let mut scanned = Scan::default();
    for named in guard.roots {
        let directory = crate_root.join(named);
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

/// Reads one file, unless the guard exempts it — an exempt file is not read, so
/// it can neither be reported nor counted toward the vacuity refusal.
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
            scanned.hits.push(format!("{relative} names `{needle}`"));
        }
    }
    Ok(())
}

/// A `.rs` file that is not a sibling unit-test file.
fn is_production_source(path: &Path) -> bool {
    path.file_name()
        .and_then(OsStr::to_str)
        .is_some_and(|file_name| file_name.ends_with(".rs") && !file_name.ends_with("_test.rs"))
}

/// A file's text with its doc comments removed.
///
/// Prose about a tick is not an advance of one, and the sources under this root
/// have a great deal of prose about ticks — including the note on the private step
/// explaining why it is private.
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
/// platform so an exemption can be written once and compared whole.
fn relative_spelling(path: &Path, crate_root: &Path) -> Result<String, Box<dyn Error>> {
    Ok(path
        .strip_prefix(crate_root)?
        .components()
        .filter_map(|component| component.as_os_str().to_str())
        .collect::<Vec<_>>()
        .join("/"))
}
