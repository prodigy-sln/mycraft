//! The client's own sources derive no layer assignment of their own.
//!
//! # Why a scan, when everything else in this phase is behavioural
//!
//! The simulation states a layer assignment and the client honours it. That the
//! *view* honours one is asserted through the packer, with an assignment written
//! down to disagree with the sorted position of its keys. What no reading in
//! this phase can assert is that the two **preparation paths** — the ones a
//! player launches through and every golden frame is shot through — build that
//! view rather than going on deriving an assignment from the registry they are
//! still handed.
//!
//! It cannot be asserted behaviourally, and the reason is worth stating rather
//! than leaving to be rediscovered: the assignment the simulation states over a
//! real content root agrees with the order a positional derivation produces, so
//! a client that honours and a client that derives answer identically for every
//! root that can be built today. The two stop agreeing the moment an assignment
//! is appended rather than renumbered, which is hot reload's, and at that point
//! this scan can be replaced by the reading it stands in for.
//!
//! # Two spellings, and they are chokepoints rather than type names
//!
//! - `texture_keys(` — the only way to get a key *set* out of a registry, which
//!   is where a derivation starts.
//! - `TextureLayers::resolve` — where a key set becomes positional layer
//!   indices, which is where one ends.
//!
//! A client naming either is a client deciding for itself which layer a block
//! draws from. Neither is a type name, so renaming a source does not green this.
//!
//! # An enumerated verdict, not an absence
//!
//! A scan whose walk broke, or whose sources moved, finds no derivation named —
//! which is exactly what a clean client looks like. So the answer is one of
//! three and each reading compares the whole of it, which rejects "there was
//! nothing to look at" for free.
//!
//! # Shape
//!
//! `tests/client_names_no_content_door.rs` is the shape this follows, down to
//! the whole-path spelling of a report and the doc-comment strip — the client's
//! own sources discuss how a layer index used to be handed out, and prose about
//! a derivation is not one. The file filter and the strip are the same two
//! carve-outs that guard carries, and for the same reasons; what is not repeated
//! here are its controls over them, because a fixture proving a filter twice
//! proves it once.

use std::error::Error;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

type TestResult = Result<(), Box<dyn Error>>;

/// Where the client's own production sources live, relative to the crate root.
const SOURCES: &str = "src";

/// The two spellings a derivation is made of.
const DERIVATIONS: [&str; 2] = ["texture_keys(", "TextureLayers::resolve"];

/// What a scan of the client's own sources came to.
#[derive(Debug, PartialEq, Eq)]
enum Verdict {
    /// The client derives no layer assignment anywhere in its own sources.
    NoAssignmentIsDerived,
    /// These sources derive one, this way.
    DerivedIn(Vec<String>),
    /// No source was read at all, so nothing above could be said.
    NoClientSourceWasRead,
}

#[test]
fn the_clients_own_sources_derive_no_layer_assignment_of_their_own() -> TestResult {
    let verdict = verdict_over(&crate_root());

    assert_eq!(
        verdict?,
        Verdict::NoAssignmentIsDerived,
        "the assignment is stated by whoever read the content and honoured by whoever draws it. A \
         client that builds its own key set and hands out indices positionally has two \
         participants numbering layers separately — and since a layer index rides inside every \
         packed vertex, inserting one block then renumbers every index after it and textures the \
         whole world wrong with no error anywhere"
    );
    Ok(())
}

/// The positive control, and the only direction this guard has.
///
/// A walk that broke, a filter that skipped everything, or a needle that stopped
/// matching would report a clean client forever. The fixture commits **both**
/// spellings and the expected report is derived from the needle list rather than
/// written out, so a needle added without a fixture committing it fails here
/// rather than standing unwatched.
#[test]
fn the_same_scan_reports_a_client_source_that_derives_one_and_says_how_it_derived_it() -> TestResult
{
    let fixture = tempfile::tempdir()?;
    let offending = a_source_deriving_an_assignment(fixture.path())?;

    let verdict = verdict_over(fixture.path());

    assert_eq!(
        verdict?,
        Verdict::DerivedIn(
            DERIVATIONS
                .iter()
                .map(|spelling| format!("{offending} names `{spelling}`"))
                .collect()
        ),
        "whoever has to repair a client that numbered its own layers needs the file and the \
         spelling in front of them, and a guard reporting only that something was wrong leaves \
         the repair to be guessed at"
    );
    Ok(())
}

/// The vacuity control, and the reason the verdict is enumerated at all.
#[test]
fn a_scan_that_read_no_client_source_says_so_rather_than_reporting_a_clean_client() -> TestResult {
    let nothing = tempfile::tempdir()?;

    let verdict = verdict_over(nothing.path());

    assert_eq!(
        verdict?,
        Verdict::NoClientSourceWasRead,
        "an empty answer and an answer nobody could look for are different facts, and a guard \
         that cannot tell them apart goes green the day it stops being able to see"
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
/// "no assignment was derived".
fn verdict_over(crate_root: &Path) -> Result<Verdict, Box<dyn Error>> {
    let mut read = 0_usize;
    let mut derived = Vec::new();
    let sources = crate_root.join(SOURCES);
    if sources.is_dir() {
        walk(&sources, crate_root, &mut read, &mut derived)?;
    }
    if read == 0 {
        return Ok(Verdict::NoClientSourceWasRead);
    }
    if derived.is_empty() {
        return Ok(Verdict::NoAssignmentIsDerived);
    }
    Ok(Verdict::DerivedIn(derived))
}

fn walk(
    directory: &Path,
    crate_root: &Path,
    read: &mut usize,
    derived: &mut Vec<String>,
) -> Result<(), Box<dyn Error>> {
    let mut entries: Vec<PathBuf> = fs::read_dir(directory)?
        .map(|entry| entry.map(|found| found.path()))
        .collect::<Result<_, _>>()?;
    // Sorted, so the report a repair is made from is the same on every run
    // whatever order the filesystem hands its entries back in.
    entries.sort();
    for path in entries {
        if path.is_dir() {
            walk(&path, crate_root, read, derived)?;
        } else if is_production_source(&path) {
            read_source(&path, crate_root, read, derived)?;
        }
    }
    Ok(())
}

/// Reads one source and records every derivation it names.
fn read_source(
    path: &Path,
    crate_root: &Path,
    read: &mut usize,
    derived: &mut Vec<String>,
) -> Result<(), Box<dyn Error>> {
    let relative = relative_spelling(path, crate_root)?;
    let text = production_text(&fs::read_to_string(path)?);
    *read += 1;
    for spelling in DERIVATIONS {
        if text.contains(spelling) {
            derived.push(format!("{relative} names `{spelling}`"));
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

/// A client source deriving a layer assignment both ways, written under `root`,
/// and where it sits.
///
/// # Errors
///
/// Returns an error if the directory or the file cannot be written.
fn a_source_deriving_an_assignment(root: &Path) -> Result<String, Box<dyn Error>> {
    let sources = root.join(SOURCES);
    fs::create_dir_all(&sources)?;
    fs::write(
        sources.join("startup.rs"),
        "let keys = registry.texture_keys();\n\
         TextureLayers::resolve(&keys)\n",
    )?;
    Ok(format!("{SOURCES}/startup.rs"))
}
