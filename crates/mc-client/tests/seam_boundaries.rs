//! Four text guards on where a decision is allowed to live, and their controls.
//!
//! A seam is only worth cutting if the parts left outside it decide nothing.
//! Otherwise the decisions move one file over and become unobservable again, with
//! every behavioural scenario still green — and the harness that drives the seam
//! can commit the same sin, by re-implementing the wiring it is supposed to be
//! watching. Each guard below is one of the three places that can happen:
//!
//! 1. **The harness** must open no window and acquire no adapter, *and* must not
//!    ask a policy the question the client asks it. The three vendor needles catch
//!    the first; the seven beside them catch the second, which is the failure this
//!    whole feature exists to remove. A harness that gated its own pointer motion
//!    on the capture policy would agree with the client by construction and pass a
//!    three-needle scan while the client's own gate was deleted.
//! 2. **The window-facing adapter** must spell events and grabs, and decide
//!    nothing: it may name neither the player's input accumulator, nor the
//!    simulation, nor any of the five capture-policy functions. What it *is*
//!    allowed to name is what it is asked to grab — the capture states themselves
//!    and the loop's own actions.
//! 3. **Everything outside the drivable core** — the frame path and the harness
//!    included — must not drain the accumulator, build a movement intent, or
//!    advance the simulation.
//! 4. **The judge must not become the thing judged.** `tests/support/oracle.rs`
//!    marches its own ray through the world's voxels and is what every golden
//!    frame is compared against. The simulation has a traversal of its own now,
//!    and it decides what a player is looking at — so an oracle, or a suite the
//!    oracle serves, that reached for *that* would be judging the renderer's
//!    picture against the very code the picture came from. The other direction is
//!    sealed by the module system, since a test module of another crate cannot be
//!    called from `mc-sim`; this one is not sealed at all, because this crate's
//!    tests may name `mc_sim` freely.
//!
//! # An absence proves nothing on its own
//!
//! Each of the four guards is an assertion that a scan found nothing, and a scan
//! that read no file, whose walk broke, or whose exemption grew to swallow the
//! tree reports exactly that. So each is asked two further questions: whether it
//! read any source at all, and whether the same function pointed at a fixture that
//! *does* commit the offence reports it while the file that is allowed to commit
//! it is passed over.
//!
//! The third guard's control is asked a third question, because its exemption is
//! the one with a trap in it. It walks two roots, and the file it must exempt is
//! the core itself — so an exemption compared by bare file name would silently
//! excuse a harness file called `session.rs`, which is precisely what a harness
//! re-implementing the core would be called. The fixture therefore carries both:
//! a `src/session.rs` that must be passed over and a
//! `tests/support/input/session.rs` that must not.
//!
//! # Shape, not filter
//!
//! These follow `tests/winit_boundary.rs`'s shape — a file's production text with
//! its doc comments removed, sibling `*_test.rs` unit files skipped, a
//! `files_read > 0` guard, and a `tempfile` fixture as the positive control — and
//! deliberately **not** its file filter. Two of the three walk more than one root
//! and must judge a file by where it sits rather than by what it is called, so
//! every exemption below is a comparison of the whole path relative to the crate
//! root.
//!
//! A root that does not exist contributes no files rather than an error, which is
//! what leaves the `files_read > 0` guard — and not an I/O failure — to report a
//! harness directory that has moved or gone.

use std::error::Error;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

type TestResult = Result<(), Box<dyn Error>>;

/// One text guard: where it reads, what it passes over, and what it refuses to
/// find.
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

/// What a scan of one guard's roots found.
#[derive(Debug, Default)]
struct Scan {
    files_read: usize,
    hits: Vec<String>,
}

/// The harness may name no window, no event loop, no graphics API — and no
/// decision it exists to observe.
///
/// `winit::window` rather than a bare `Window`: the harness legitimately builds a
/// `winit::event::WindowEvent`, and `Window` is a substring of that. The module
/// spelling catches the window type and the cursor-grab mode together, and
/// `EventLoop` catches `ActiveEventLoop` with it. `CaptureState` is deliberately
/// absent from this list — the harness spells what a platform grants and what it
/// was asked for, and neither is a decision.
///
/// `MouseButtonKind` joins `KeyKind` for the same reason it does: the harness
/// dispatches the *library's* button and the client decides what it means, so a
/// harness naming the client's own vocabulary would be translating on the client's
/// behalf — and the one thing on the click path the adapter is responsible for
/// would have nothing watching it.
const HARNESS_GUARD: Guard = Guard {
    roots: &["tests/support/input"],
    exempt: |_| false,
    needles: &[
        "EventLoop",
        "winit::window",
        "wgpu",
        "accepts_pointer_motion",
        "capture_after_click",
        "capture_after_escape",
        "first_capture_attempt",
        "next_capture_attempt",
        "bound_action",
        "KeyKind",
        "MouseButtonKind",
    ],
};

/// The window-facing adapter spells; it does not decide.
///
/// Expressed as an exemption of everything else so that one walker serves all
/// three guards: the one file of `src/` this reads is the adapter itself.
/// `CaptureState` and `LoopAction` are permitted, because an adapter that could
/// not name what it is asked to grab could not ask for it.
const WINDOW_FACING_GUARD: Guard = Guard {
    roots: &["src"],
    exempt: |path| path != "src/events.rs",
    needles: &[
        "InputState",
        "Simulation",
        "first_capture_attempt",
        "next_capture_attempt",
        "capture_after_escape",
        "capture_after_click",
        "accepts_pointer_motion",
    ],
};

/// Only the drivable core drains the input, builds an intent, or advances the
/// simulation.
///
/// `.advance(` and not `advance(`, so that `advance_player` — the simulation's own
/// pure step, which anything may name — is not caught by the guard aimed at the
/// tick. The roots are this crate's, which leaves `mc-sim`'s own definitions and
/// the golden suites that legitimately advance a replay outside the scan: a
/// bounded limitation, and the reason the core exposes no borrow of what it owns.
///
/// **The action vocabulary is on the list because it is a second vocabulary.**
/// A tick now carries what the player asked the *world* for as well as how they
/// asked to move, and a frame path that assembled one of those itself would be the
/// same failure as one that drained the accumulator — the tick running under a
/// request nothing dispatched, with every click scenario still green. `.advance(`
/// alone would not catch it, because the offending file could hand the request to
/// the one call that is allowed to make it. `pending_action` is the drain's own
/// name, and it is deliberately the name the implementation gives it rather than a
/// method the design never declared: a needle that matches nothing even when the
/// offence is committed passes its scan forever.
const OUTSIDE_THE_CORE_GUARD: Guard = Guard {
    roots: &["src", "tests/support/input"],
    exempt: |path| path == "src/session.rs",
    needles: &[
        "take_intent",
        "MovementIntent",
        ".advance(",
        "ActionIntent",
        "TickIntent",
        "pending_action",
    ],
};

/// Nothing under this crate's `tests/` reaches for the simulation's own
/// traversal.
///
/// The whole tree rather than the harness alone: `tests/support/oracle.rs` is
/// where the offence would land first, but a golden suite that judged a frame
/// against `mc-sim`'s targeting instead of against the oracle collapses the same
/// two things one level up, and both sit in this directory. Two exemptions, each
/// for a reason: this file, whose needle list would otherwise be its own hit; and
/// the fixtures directory, whose files exist to be *found* by the scans above and
/// say so in their own text.
///
/// `targeted` names the traversal itself and `::Hit` catches an import of what it
/// answers with, so a suite that reached for either — by call or by name — is
/// reported. Neither spelling occurs anywhere in this crate today, which is what
/// the control below is for: a needle that matches nothing even when the offence
/// is committed passes its scan forever.
const ORACLE_INDEPENDENCE_GUARD: Guard = Guard {
    roots: &["tests"],
    exempt: |path| path == "tests/seam_boundaries.rs" || path.starts_with("tests/fixtures/"),
    needles: &["targeted", "::Hit"],
};

#[test]
fn the_harness_names_neither_the_windowing_stack_nor_a_decision_it_exists_to_watch() -> TestResult {
    let scanned = scan(&crate_root(), &HARNESS_GUARD)?;

    assert!(
        scanned.files_read > 0,
        "the scan read no harness source at all, so the check below would be vacuous — the \
         harness directory has moved, or it does not exist yet"
    );
    assert!(
        scanned.hits.is_empty(),
        "a harness that opens a window proves nothing about a client that must run without one, \
         and a harness that asks the capture policy or the binding table the question the client \
         asks them agrees with the client by construction — which is the exact shape this feature \
         exists to delete: {:?}",
        scanned.hits
    );
    Ok(())
}

/// The control for the guard above, and the only direction it has.
///
/// A walk that broke, a filter that skipped everything, or a needle list that
/// stopped matching would report a clean harness forever. The fixture names the
/// event loop in a harness file, which the same function has to read and report.
///
/// It names the client's own button vocabulary beside it, because a needle added
/// to the list and never committed by any fixture is a needle nobody has watched
/// match anything — a mistyped one would report a clean scan for as long as it
/// stood there.
#[test]
fn the_same_scan_reports_a_harness_file_that_names_the_event_loop() -> TestResult {
    let fixture = tempfile::tempdir()?;
    let harness = fixture.path().join("tests/support/input");
    fs::create_dir_all(&harness)?;
    fs::write(
        harness.join("harness.rs"),
        "use winit::event_loop::EventLoop;\nfn kind(button: MouseButtonKind) -> bool { true }\n",
    )?;

    let scanned = scan(fixture.path(), &HARNESS_GUARD)?;
    let reported = |needle: &str| scanned.hits.iter().any(|hit| hit.contains(needle));

    assert_eq!(
        (
            scanned.files_read,
            scanned.hits.len(),
            reported("EventLoop"),
            reported("MouseButtonKind")
        ),
        (1, 2, true, true),
        "the scan has to reach into the harness directory, read the file it finds there, and \
         report both the event loop it opens and the client vocabulary it translates into: {:?}",
        scanned.hits
    );
    Ok(())
}

#[test]
fn the_window_facing_adapter_names_no_accumulator_no_simulation_and_no_capture_policy() -> TestResult
{
    let scanned = scan(&crate_root(), &WINDOW_FACING_GUARD)?;

    assert!(
        scanned.files_read > 0,
        "the scan read no adapter source at all, so the check below would be vacuous"
    );
    assert!(
        scanned.hits.is_empty(),
        "the side of the seam a windowless test cannot reach is allowed to spell events and \
         grabs and to decide nothing: an accumulator, a simulation or a capture policy named \
         here is a decision that has moved back out of reach, with every behavioural scenario \
         still green: {:?}",
        scanned.hits
    );
    Ok(())
}

/// The control for the guard above, in both directions at once.
///
/// The fixture names a capture-policy function twice — once in the adapter, which
/// must be reported, and once in the core beside it, which must be passed over —
/// so a scan that reported nothing and a scan whose filter had grown to cover the
/// core are both caught here.
#[test]
fn the_same_scan_reports_a_capture_policy_named_in_the_adapter_and_passes_the_core_over()
-> TestResult {
    let fixture = tempfile::tempdir()?;
    let sources = fixture.path().join("src");
    fs::create_dir_all(&sources)?;
    fs::write(
        sources.join("events.rs"),
        "if accepts_pointer_motion(capture) { look(x, y) }\n",
    )?;
    fs::write(
        sources.join("session.rs"),
        "let wanted = first_capture_attempt();\n",
    )?;

    let scanned = scan(fixture.path(), &WINDOW_FACING_GUARD)?;

    assert_eq!(
        (
            scanned.files_read,
            scanned.hits.len(),
            scanned.hits.iter().any(|hit| hit.contains("src/events.rs"))
        ),
        (1, 1, true),
        "the scan has to read the adapter, report the policy it consults, and leave the core — \
         whose whole job is to consult that policy — alone: {:?}",
        scanned.hits
    );
    Ok(())
}

#[test]
fn nothing_outside_the_core_drains_the_input_builds_an_intent_or_advances_the_simulation()
-> TestResult {
    let scanned = scan(&crate_root(), &OUTSIDE_THE_CORE_GUARD)?;

    assert!(
        scanned.files_read > 0,
        "the scan read no source at all, so the check below would be vacuous"
    );
    assert!(
        scanned.hits.is_empty(),
        "a frame path that drains the accumulator itself, or a harness that advances the \
         simulation itself, is the same failure one level up: the tick keeps running under an \
         intent nothing dispatched, and every scenario driving a key stays green while the \
         client submits nothing: {:?}",
        scanned.hits
    );
    Ok(())
}

/// The control for the guard above, in all four of its directions.
///
/// The third is the one that matters most: the exemption is compared against the
/// whole path, so the harness file sharing the core's *name* is reported rather
/// than excused. A bare-name comparison would place the exemption on exactly the
/// file a harness re-implementing the core would be called.
///
/// The fourth is new and is why the offending file below names **every** needle
/// the guard carries rather than one of them. A needle no fixture ever commits is
/// a needle nobody has watched match anything: mistype one and it reports a clean
/// scan for as long as it stands there, which is the failure the whole file is
/// about, one level up. The expected count is therefore derived — one hit per
/// needle from the frame path, and one more from the harness file that advances
/// the simulation — so a needle added without a fixture to catch it fails here
/// rather than passing silently.
#[test]
fn the_same_scan_reports_a_non_core_file_that_advances_the_simulation_wherever_it_sits()
-> TestResult {
    let fixture = tempfile::tempdir()?;
    a_tree_that_advances_the_simulation(fixture.path())?;

    let scanned = scan(fixture.path(), &OUTSIDE_THE_CORE_GUARD)?;
    let reported = |file: &str| scanned.hits.iter().any(|hit| hit.starts_with(file));
    let every_needle = OUTSIDE_THE_CORE_GUARD
        .needles
        .iter()
        .all(|needle| scanned.hits.iter().any(|hit| hit.contains(needle)));

    assert_eq!(
        (
            scanned.files_read,
            scanned.hits.len(),
            reported("src/app.rs"),
            reported("tests/support/input/session.rs"),
            every_needle
        ),
        (
            2,
            OUTSIDE_THE_CORE_GUARD.needles.len() + 1,
            true,
            true,
            true
        ),
        "the scan has to report the frame path assembling and advancing a tick — every needle the \
         guard carries, one hit apiece — report a harness file advancing the simulation wherever \
         it sits and whatever it is called, and pass over the one file whose job that is, judged \
         on its whole path and never on its name: {:?}",
        scanned.hits
    );
    Ok(())
}

#[test]
fn neither_the_goldens_oracle_nor_any_suite_it_judges_names_the_simulations_own_traversal()
-> TestResult {
    let scanned = scan(&crate_root(), &ORACLE_INDEPENDENCE_GUARD)?;

    assert!(
        scanned.files_read > 0,
        "the scan read no test source at all, so the check below would be vacuous — the suites \
         have moved, or both exemptions have grown to cover the tree"
    );
    assert!(
        scanned.hits.is_empty(),
        "the ray this crate's oracle marches is what every golden frame is judged against, and \
         the simulation's traversal is what decides which block a player is looking at. Two \
         implementations, on purpose: promoting one to stand for the other means a frame is \
         compared against the code that drew it, and the comparison passes whatever either of \
         them does: {:?}",
        scanned.hits
    );
    Ok(())
}

/// The control for the guard above, in both of its directions.
///
/// A needle matching nothing anywhere, a walk that broke, or an exemption that
/// grew would all report a clean tree forever. The fixture reaches for the
/// traversal from a harness file, which must be reported, and names the same
/// spelling in a copy of this file, which must be passed over — the exemption
/// that keeps a needle list from being its own hit is the one most likely to be
/// widened by accident.
#[test]
fn the_same_scan_reports_a_harness_file_that_reaches_for_the_simulations_traversal() -> TestResult {
    let fixture = tempfile::tempdir()?;
    let support = fixture.path().join("tests/support");
    fs::create_dir_all(&support)?;
    fs::write(
        support.join("oracle.rs"),
        "let met = mc_sim::action::targeted(eye, ray, REACH, world);\n",
    )?;
    fs::write(
        fixture.path().join("tests/seam_boundaries.rs"),
        "needles: &[\"targeted\", \"::Hit\"],\n",
    )?;

    let scanned = scan(fixture.path(), &ORACLE_INDEPENDENCE_GUARD)?;

    assert_eq!(
        (
            scanned.files_read,
            scanned.hits.len(),
            scanned
                .hits
                .iter()
                .any(|hit| hit.contains("tests/support/oracle.rs"))
        ),
        (1, 1, true),
        "the scan has to reach into the harness, report the production traversal it reached for, \
         and pass over the guard file whose own needle list names the same spelling: {:?}",
        scanned.hits
    );
    Ok(())
}

/// The offending tree the control above scans, written under `root`.
///
/// Three files, each for one direction of that control: a frame path that
/// assembles a whole tick and advances it — naming every needle the guard carries,
/// so a mistyped one is caught rather than standing unwatched; the core beside it,
/// whose job that is and which must be passed over; and a harness file wearing the
/// core's own name, which must not be.
fn a_tree_that_advances_the_simulation(root: &Path) -> Result<(), Box<dyn Error>> {
    let sources = root.join("src");
    let harness = root.join("tests/support/input");
    fs::create_dir_all(&sources)?;
    fs::create_dir_all(&harness)?;
    fs::write(
        sources.join("app.rs"),
        "let action: Option<ActionIntent> = self.pending_action.take();\n\
         let movement: MovementIntent = self.input.take_intent();\n\
         simulation.advance(TickIntent { movement, action });\n",
    )?;
    fs::write(
        sources.join("session.rs"),
        "self.simulation.advance(self.input.take_intent());\n",
    )?;
    fs::write(harness.join("session.rs"), "simulation.advance(intent);\n")?;
    Ok(())
}

/// This crate's own directory, which every root above is relative to.
fn crate_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

/// Reads every production source under `guard`'s roots and reports each place one
/// of its needles is named.
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

/// Reads one file, unless the guard exempts it — an exempt file is not read, so
/// it can neither be reported nor be counted toward the vacuity guard.
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
/// A rustdoc example is a doc test, so prose about a window library or about the
/// policy a core consults is not a use of either.
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
