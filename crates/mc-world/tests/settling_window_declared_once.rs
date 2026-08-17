//! The settling window is declared in exactly one place in this repository.
//!
//! A value spelled twice is two places for one decision to disagree with itself,
//! and this one decides how a burst of writes becomes a single reload attempt.
//! **It is also the dominant term in the spec's one-second target**, so a second
//! declaration is not a tidiness question: an adapter debouncing on 150 ms while a
//! second constant says something else gives two different answers to "how long
//! does an editor's save take to settle", and the one a *person* experiences is
//! whichever the shipped door happens to reach.
//!
//! # What counts as a declaration, and why it is two spellings rather than one
//!
//! - `from_millis(150)` — the **value**. This is what catches a second window
//!   declared under any name at all, including one inlined at a call.
//! - `SETTLING_WINDOW:` — the **name**, in the one shape a declaration has and a
//!   use does not: a use spells `watch::SETTLING_WINDOW` or
//!   `SETTLING_WINDOW * 2`, and only a `const` writes a colon after it. Without
//!   this half, a second constant of the same name holding a *different* duration
//!   would be invisible here.
//!
//! One line matching either is one declaration site, and the shipped declaration
//! matches both on one line — which is why sites are counted by line and not by
//! needle.
//!
//! # This does NOT cover the window reaching the debouncer, and must not be read
//! as covering it
//!
//! A window declared exactly once and then handed to the debouncer builder as
//! `Duration::ZERO` leaves this scan **green**, the domain's coalescing test
//! green, and the shipped client beginning one attempt per filesystem event. That
//! was measured rather than reasoned: phase 3 ran exactly that mutation and it
//! reddened the boundary assertion and nothing else. The boundary assertion lives
//! beside the adapter, in `crates/mc-world/tests/content_watch.rs`, and neither of
//! the two covers the other.
//!
//! # The verdict is enumerated, and two of its arms mean "I could not look"
//!
//! `assert!(sites.len() == 1)` cannot tell one declaration from a walk that read
//! one file and stopped, nor from a root that has moved out from under it. So the
//! answer is one of four and the good one is compared whole, which rejects the
//! other three for free — including a root that contributed nothing.
//!
//! **A root contributing nothing is its own arm because the total cannot see it.**
//! `crates/` alone holds some four hundred production files, so a walk that lost
//! `tools/` entirely would still read plenty and report exactly what a clean tree
//! reports. That is the defect `crates/mc-world/tests/no_hardcoded_block_names.rs`
//! exists to refuse one level down, and the same guard is owed here for the same
//! reason.
//!
//! # Shape
//!
//! `tests/no_hardcoded_block_names.rs`: the member roots stated in one place, a
//! per-root count, `read_dir` failing loudly on a root that is not there, and a
//! file's text with its doc comments stripped — prose about the window (this file
//! is full of it) is not a declaration of it.

use std::error::Error;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

type TestResult = Result<(), Box<dyn Error>>;

/// The workspace member directories, each holding crates whose `src` is
/// production code.
const MEMBER_ROOTS: [&str; 2] = ["crates", "tools"];

/// The two spellings a declaration of this window has.
const DECLARATIONS: [&str; 2] = ["from_millis(150)", "SETTLING_WINDOW:"];

/// Where the one declaration is, relative to the repository root and spelled with
/// `/` on every platform.
///
/// **Written out rather than discovered**, because the point of the scenario is
/// that this decision has one home: a scan that reported wherever it found the
/// value would agree with a declaration that had drifted into another crate. The port,
/// the relevance rule and this window are sited together in `mc_world::content::watch`
/// deliberately — the rule has to be built from the loaders' own constants or it
/// silently narrows — so moving the window is a change to that decision rather than a
/// refactor, and this reddening is the correct answer to a move.
const DECLARED_IN: &str = "crates/mc-world/src/content/watch/mod.rs";

/// What a scan of this repository's production sources came to.
#[derive(Debug, PartialEq, Eq)]
enum Verdict {
    /// Exactly one production line declares the window, and this is the file.
    DeclaredExactlyOnce { site: String },
    /// Several lines declare it, and these are they.
    DeclaredIn(Vec<String>),
    /// Every root was read and nothing declares it at all.
    DeclaredNowhere,
    /// These member roots contributed no production source, so nothing above
    /// could be said of them.
    RootsThatContributedNothing(Vec<String>),
}

#[test]
fn the_settling_window_is_declared_in_exactly_one_production_source() -> TestResult {
    let verdict = verdict_over(&repository_root()?)?;

    assert_eq!(
        verdict,
        Verdict::DeclaredExactlyOnce {
            site: DECLARED_IN.to_owned()
        },
        "how long an editor's save is given to settle is one decision, and the whole of the reload \
         budget it is the dominant term in rests on there being one answer. A second declaration is \
         a second answer, and which one a person meets is whichever the shipped door reaches"
    );
    Ok(())
}

/// One control, and it is the one that matters: several declarations must not read
/// as one.
///
/// The fixture puts them in two different crates, which is where the drift
/// actually happens — an adapter growing a window of its own beside the port's.
/// The expectation is derived from what the fixture wrote rather than from a run.
#[test]
fn the_same_scan_reports_a_second_declaration_in_a_second_module_and_names_both() -> TestResult {
    let fixture = tempfile::tempdir()?;
    let first = a_source_declaring(fixture.path(), "crates/mc-world/src/watch.rs")?;
    let second = a_source_declaring(fixture.path(), "crates/mc-sim/src/reload.rs")?;
    // Every member root has to contribute something, or the barren-root arm
    // answers first — which is the precedence this guard states and not an
    // inconvenience to work around.
    a_source_declaring_nothing(fixture.path(), "tools/voxforge/src/main.rs")?;

    let verdict = verdict_over(fixture.path())?;

    assert_eq!(
        verdict,
        Verdict::DeclaredIn(vec![second, first]),
        "two declarations are two places for one decision to disagree, and a guard that answered \
         `declared once` over them would be the reason nobody found out. Both are named, in the \
         order the walk reads them, because a repair needs to know which one to delete"
    );
    Ok(())
}

/// The vacuity control, in the form the total cannot see.
///
/// A walk that read nothing reports no second declaration just as loudly as a tree
/// with one — and a walk that lost one member root reports it while still reading
/// hundreds of files. Both are the same defect and this is the arm that separates
/// them from a clean answer.
#[test]
fn a_scan_whose_member_roots_hold_no_source_says_so_rather_than_reporting_one_declaration()
-> TestResult {
    let nothing = tempfile::tempdir()?;
    empty_member_roots(nothing.path())?;

    let verdict = verdict_over(nothing.path())?;

    assert_eq!(
        verdict,
        Verdict::RootsThatContributedNothing(
            MEMBER_ROOTS.iter().map(|root| (*root).to_owned()).collect()
        ),
        "an empty answer and an answer nobody could look for are different facts. This one is \
         worse than most: `crates/` holds hundreds of production files, so a walk that lost a root \
         goes on reading plenty and reports exactly what a clean tree reports"
    );
    Ok(())
}

/// This repository's own directory.
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

/// What the production sources under `repository` came to.
///
/// # Errors
///
/// Returns the I/O failure when a member root cannot be read — which is what a
/// root named in [`MEMBER_ROOTS`] but absent from the tree produces. Failing
/// loudly is what keeps a mistyped root from narrowing this walk in silence.
fn verdict_over(repository: &Path) -> Result<Verdict, Box<dyn Error>> {
    let mut sites = Vec::new();
    let mut barren = Vec::new();
    for member_root in MEMBER_ROOTS {
        let mut read = 0;
        for sources in source_directories_under(&repository.join(member_root))? {
            let spelled = relative_spelling(&sources, repository)?;
            read += walked(&sources, &spelled, &mut sites)?;
        }
        if read == 0 {
            barren.push(member_root.to_owned());
        }
    }
    if !barren.is_empty() {
        return Ok(Verdict::RootsThatContributedNothing(barren));
    }
    match sites.as_slice() {
        [] => Ok(Verdict::DeclaredNowhere),
        [only] => Ok(Verdict::DeclaredExactlyOnce {
            site: only.file.clone(),
        }),
        _ => Ok(Verdict::DeclaredIn(
            sites.iter().map(Site::spelled).collect(),
        )),
    }
}

/// One line that declares the window.
#[derive(Debug)]
struct Site {
    file: String,
    line: usize,
}

impl Site {
    /// The site as a repair reads it.
    fn spelled(&self) -> String {
        format!(
            "{file} declares it, on line {line}",
            file = self.file,
            line = self.line
        )
    }
}

/// The `src` directory of every member directly under `root`.
fn source_directories_under(root: &Path) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    let mut entries: Vec<PathBuf> = fs::read_dir(root)?
        .map(|entry| entry.map(|found| found.path().join("src")))
        .collect::<Result<_, _>>()?;
    entries.sort();
    Ok(entries
        .into_iter()
        .filter(|sources| sources.is_dir())
        .collect())
}

/// Reads every production source under `directory`, and how many that was.
///
/// Sorted, so a report naming two sites names them in the same order on every run.
fn walked(directory: &Path, spelled: &str, sites: &mut Vec<Site>) -> Result<usize, Box<dyn Error>> {
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
            read += walked(&path, &under, sites)?;
        } else if is_production_source(file_name) {
            read_source(&path, &under, sites)?;
            read += 1;
        }
    }
    Ok(read)
}

/// Reads one source and records every line of it that declares the window.
fn read_source(path: &Path, spelled: &str, sites: &mut Vec<Site>) -> Result<(), Box<dyn Error>> {
    for (offset, line) in production_lines(&fs::read_to_string(path)?) {
        if DECLARATIONS.iter().any(|needle| line.contains(needle)) {
            sites.push(Site {
                file: spelled.to_owned(),
                line: offset,
            });
        }
    }
    Ok(())
}

/// A `.rs` file that is not a sibling unit-test file.
fn is_production_source(file_name: &str) -> bool {
    file_name.ends_with(".rs") && !file_name.ends_with("_test.rs")
}

/// Every line of a file that is not a doc comment, with the number it sits on.
///
/// Prose about the window is not a declaration of it, and this repository
/// discusses it at length. Ordinary `//` comments stay: a declaration commented
/// out is still a second answer somebody meant to have.
fn production_lines(source: &str) -> Vec<(usize, &str)> {
    source
        .lines()
        .enumerate()
        .map(|(offset, line)| (offset + 1, line))
        .filter(|(_, line)| {
            let trimmed = line.trim_start();
            !trimmed.starts_with("///") && !trimmed.starts_with("//!")
        })
        .collect()
}

/// Where a directory sits relative to `repository`, spelled with `/` on every
/// platform so a report reads the same everywhere.
fn relative_spelling(path: &Path, repository: &Path) -> Result<String, Box<dyn Error>> {
    Ok(path
        .strip_prefix(repository)?
        .components()
        .filter_map(|component| component.as_os_str().to_str())
        .collect::<Vec<_>>()
        .join("/"))
}

/// Both member roots, present and holding nothing.
///
/// A root that is missing altogether is an error rather than a verdict, so a
/// fixture about a tree with nothing in it has to be a tree.
fn empty_member_roots(repository: &Path) -> Result<(), Box<dyn Error>> {
    for member_root in MEMBER_ROOTS {
        fs::create_dir_all(repository.join(member_root))?;
    }
    Ok(())
}

/// A production source at `spelled` under `repository` that declares the window,
/// and how a report will name its one declaring line.
///
/// The declaration is written on the file's second line, under a doc comment, so
/// the fixture also exercises the strip and the line numbering rather than only
/// the needle.
///
/// # Errors
///
/// Returns an error if the directory or the file cannot be written.
fn a_source_declaring(repository: &Path, spelled: &str) -> Result<String, Box<dyn Error>> {
    let path = repository.join(spelled);
    let parent = path
        .parent()
        .ok_or("that source has no directory to write it into")?;
    fs::create_dir_all(parent)?;
    fs::write(
        &path,
        "/// How long a save is given to settle, said a second time.\n\
         pub const SETTLING_WINDOW: Duration = Duration::from_millis(150);\n",
    )?;
    Ok(Site {
        file: spelled.to_owned(),
        line: 2,
    }
    .spelled())
}

/// A production source at `spelled` under `repository` that declares no window,
/// so that a member root contributes a file without contributing a site.
///
/// # Errors
///
/// Returns an error if the directory or the file cannot be written.
fn a_source_declaring_nothing(repository: &Path, spelled: &str) -> Result<(), Box<dyn Error>> {
    let path = repository.join(spelled);
    let parent = path
        .parent()
        .ok_or("that source has no directory to write it into")?;
    fs::create_dir_all(parent)?;
    fs::write(&path, "fn main() {}\n")?;
    Ok(())
}
