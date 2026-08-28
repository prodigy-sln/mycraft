//! Neither the mesher nor the renderer compares anything against a block name.
//!
//! # Why this invariant is the one worth guarding here
//!
//! The base game is a mod: no block, item or recipe is defined in Rust, and a
//! missing hook is fixed in the API rather than special-cased. A translucency
//! feature is exactly where that gets broken, because the cheap way to make
//! water see-through is one comparison against `base:water` in the mesher or in
//! the packer — after which every mod's own glass draws opaque, and the reason
//! is a line no mod author can reach.
//!
//! # What a comparison against a block name looks like, and the two shapes
//!
//! A name reaches code in one of two forms, and both are looked for:
//!
//! - **written down**, as a namespaced literal — two runs of lower-case letters,
//!   digits and underscores with one colon between them, which is the shape
//!   `BlockName` and `TextureKey` both parse. A module that never spells one
//!   cannot compare against one;
//! - **parsed**, through `BlockName::parse` or `TextureKey::parse` — the doors
//!   that turn text into an identifier. A module holding no literal but calling
//!   one of these is building a name from somewhere, and where it got the text
//!   is a question the invariant does not want asked in these two crates at all.
//!
//! **Comparing two names against each other is not the offence and is not
//! looked for.** A mesher asking whether two adjacent cells hold the same block
//! is deciding nothing about *which* block, and a guard that banned it would be
//! standing in the way of the merge rule this spec deliberately did not need.
//!
//! # What is scanned, and what is not
//!
//! `crates/mc-world/src/mesh` is the mesher — the sweep, the occlusion resolve
//! and the facings — and `crates/mc-render/src` is the renderer whole.
//! `tests/client_names_no_content_door.rs` carries the same property for the
//! client, and this is its sibling rather than its replacement.
//!
//! **The shaders and the build script are outside both, and the reason is that
//! the scan could not fail on them.** A WGSL stage has no string type to compare
//! a name with, so a shader cannot commit this offence in the form the scan
//! detects; `crates/mc-render/build/validate.rs` reads *shader source* rather
//! than content, so it is not a content door either. Adding them would raise the
//! file count and buy nothing — a scan reporting "no name is compared" over
//! files it was never able to report anything else about is worse than one with
//! a stated boundary, because the reader cannot tell the two apart. **A boundary
//! with its reason attached survives; a boundary without one gets moved by the
//! next person who notices it**, and the green they would get back is
//! meaningless.
//!
//! # What this scan cannot see, stated rather than left to silence
//!
//! It is a scan for **names**, so it would not detect a shader special-casing a
//! block by its **layer index**. That is structurally hard rather than
//! forbidden: a fragment stage has no block identity at all, the layer a key
//! takes is its position in the ascending key list `LayerAssignment::appending`
//! walks rather than anything chosen, and the partition is decided CPU-side
//! before anything reaches a shader — so a shader would have to hard-code an
//! index whose meaning moves the moment a mod declares one more texture that
//! sorts before it. Hard is not impossible, and the instrument that would see it
//! is a reading of the shaders against the assignment rather than a text scan of
//! either.
//!
//! # An enumerated verdict, not an absence
//!
//! A scan that read no file, whose walk broke, or whose filter grew to swallow
//! the tree reports "no name is compared" just as loudly as a clean engine does.
//! So the answer is one of three and each reading compares the whole of it,
//! which rejects the other two — including the one meaning "there was nothing to
//! look at" — for free. The fixture that commits the offence is committed beside
//! them rather than run once in a mutation window: an absence assertion needs a
//! control that stands as long as it does.
//!
//! # Shape
//!
//! `tests/client_names_no_content_door.rs` is the shape this follows, down to
//! the exemption being compared against a whole path relative to the repository
//! root rather than a bare file name. Comments are stripped whole-line, and one
//! step wider than that file's doc-comment strip: prose naming a block is prose,
//! and a mesher explaining which block motivated a rule is documenting the seam
//! rather than crossing it.

mod support;

use std::error::Error;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

use support::{TestResult, repository_root};

/// The two source trees this invariant is about, relative to the repository
/// root.
const SCANNED: [&str; 2] = ["crates/mc-world/src/mesh", "crates/mc-render/src"];

/// The two doors text becomes an identifier through.
const DOORS: [&str; 2] = ["BlockName::parse", "TextureKey::parse"];

/// What a scan of the mesher's and the renderer's own sources came to.
#[derive(Debug, PartialEq, Eq)]
enum Verdict {
    /// Neither tree names a block, nor builds a name out of text.
    NoBlockNameIsCompared,
    /// These sources name these things.
    NamesCompared(Vec<String>),
    /// No source was read at all, so nothing above could be said.
    NoMesherOrRendererSourceWasRead,
}

#[test]
fn neither_the_mesher_nor_the_renderer_compares_anything_against_a_block_name() -> TestResult {
    let verdict = verdict_over(&repository_root()?)?;

    assert_eq!(
        verdict,
        Verdict::NoBlockNameIsCompared,
        "the base game is a mod, and the whole of what makes that true is that the engine cannot \
         tell one block from another. A mesher or a packer that named `base:water` would draw the \
         shipped sea correctly and every mod's own glass opaque, with the reason in a file no mod \
         author can reach — and the declared degree these two modules now carry is exactly the \
         thing a shortcut would be taken with"
    );
    Ok(())
}

/// The control for the guard above, and the only direction it has.
///
/// A walk that broke, a filter that skipped everything, or a needle that stopped
/// matching would report a clean engine forever. The fixture commits **every**
/// shape the guard carries rather than one of them, and the expected report is
/// derived from the same lists the scan reads: a door added here without a
/// fixture committing it is a door nobody has watched match anything, and a
/// mistyped one would report a clean scan for as long as it stood there.
#[test]
fn the_same_scan_reports_a_mesher_source_that_compares_a_block_name_and_says_which_name_it_named()
-> TestResult {
    let fixture = tempfile::tempdir()?;
    let offending = a_source_comparing_a_block_name(fixture.path())?;

    let verdict = verdict_over(fixture.path())?;

    assert_eq!(
        verdict,
        Verdict::NamesCompared(
            [format!(
                "{offending} names `{THE_NAME_A_SHORTCUT_WOULD_REACH_FOR}`"
            )]
            .into_iter()
            .chain(
                DOORS
                    .iter()
                    .map(|door| format!("{offending} names `{door}`"))
            )
            .collect()
        ),
        "whoever has to repair a mesher that reached for a block by name needs the file and the \
         spelling in front of them, and a guard reporting only that something was wrong leaves the \
         repair to be guessed at"
    );
    Ok(())
}

/// The vacuity control, and the reason the verdict is enumerated at all.
///
/// A tree whose sources have moved, or a walk that can no longer reach them,
/// finds no name compared — which is exactly what a clean engine looks like. The
/// two must never compare equal.
#[test]
fn a_scan_that_read_no_source_says_so_rather_than_reporting_a_clean_engine() -> TestResult {
    let nothing = tempfile::tempdir()?;

    let verdict = verdict_over(nothing.path())?;

    assert_eq!(
        verdict,
        Verdict::NoMesherOrRendererSourceWasRead,
        "an empty answer and an answer nobody could look for are different facts, and a guard that \
         cannot tell them apart goes green the day it stops being able to see"
    );
    Ok(())
}

/// The file filter, in both of its directions at once.
///
/// A sibling unit test may name whatever it is testing; the module beside it may
/// not. A filter that skipped too much would leave the control above green while
/// scanning almost nothing, so the fixture puts a name either side of it.
#[test]
fn a_name_in_a_sibling_unit_test_file_is_passed_over_and_the_module_beside_it_is_not() -> TestResult
{
    let fixture = tempfile::tempdir()?;
    let mesher = fixture.path().join(SCANNED[0]);
    fs::create_dir_all(&mesher)?;
    fs::write(
        mesher.join("sweep_test.rs"),
        format!("let sea = BlockName::parse(\"{THE_NAME_A_SHORTCUT_WOULD_REACH_FOR}\")?;\n"),
    )?;
    fs::write(
        mesher.join("sweep.rs"),
        format!("if block.as_str() == \"{THE_NAME_A_SHORTCUT_WOULD_REACH_FOR}\" {{}}\n"),
    )?;

    let verdict = verdict_over(fixture.path())?;

    assert_eq!(
        verdict,
        Verdict::NamesCompared(vec![format!(
            "{}/sweep.rs names `{THE_NAME_A_SHORTCUT_WOULD_REACH_FOR}`",
            SCANNED[0]
        )]),
        "a unit test naming the block it builds a fixture out of is not the mesher deciding \
         anything, and a filter that swallowed the module beside it would report a clean engine \
         having read almost nothing"
    );
    Ok(())
}

/// A comment naming a block is prose about the seam rather than a crossing of
/// it.
///
/// The strip is one step wider than the doc-comment strip its sibling guard
/// uses, and it has to be: these two modules discuss the sea in ordinary
/// comments today — why a merge rule was not needed, which block motivated a
/// bound. Without the strip the guard would be unsatisfiable for a reason that
/// has nothing to do with the invariant, which is the state in which somebody
/// deletes the guard rather than the offence.
#[test]
fn a_block_named_in_a_comment_is_not_a_comparison_against_it() -> TestResult {
    let fixture = tempfile::tempdir()?;
    let renderer = fixture.path().join(SCANNED[1]);
    fs::create_dir_all(&renderer)?;
    fs::write(
        renderer.join("mip.rs"),
        format!(
            "//! Why the first translucent block mattered.\n\
             /// Written when `{THE_NAME_A_SHORTCUT_WOULD_REACH_FOR}` was the only one.\n\
             // `{THE_NAME_A_SHORTCUT_WOULD_REACH_FOR}` is the block this bound came from.\n\
             pub fn reduced() {{}}\n"
        ),
    )?;

    let verdict = verdict_over(fixture.path())?;

    assert_eq!(
        verdict,
        Verdict::NoBlockNameIsCompared,
        "a module explaining which block a rule was written for is a module documenting the seam, \
         which is the opposite of one crossing it"
    );
    Ok(())
}

/// The name a shortcut would reach for, and the one every fixture here commits.
const THE_NAME_A_SHORTCUT_WOULD_REACH_FOR: &str = "base:water";

/// What the production sources under `root` came to.
///
/// # Errors
///
/// Returns an error if a directory or a file cannot be read — an I/O failure is
/// not one of the three verdicts, for the same reason reading nothing is not
/// "no name is compared".
fn verdict_over(root: &Path) -> Result<Verdict, Box<dyn Error>> {
    let mut read = 0_usize;
    let mut named = Vec::new();
    for tree in SCANNED {
        let sources = root.join(tree);
        if sources.is_dir() {
            walk(&sources, root, &mut read, &mut named)?;
        }
    }
    if read == 0 {
        return Ok(Verdict::NoMesherOrRendererSourceWasRead);
    }
    if named.is_empty() {
        return Ok(Verdict::NoBlockNameIsCompared);
    }
    Ok(Verdict::NamesCompared(named))
}

fn walk(
    directory: &Path,
    root: &Path,
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
            walk(&path, root, read, named)?;
        } else if is_production_source(&path) {
            read_source(&path, root, read, named)?;
        }
    }
    Ok(())
}

/// Reads one source and records every name it spells and every door it names.
fn read_source(
    path: &Path,
    root: &Path,
    read: &mut usize,
    named: &mut Vec<String>,
) -> Result<(), Box<dyn Error>> {
    let relative = relative_spelling(path, root)?;
    let text = production_text(&fs::read_to_string(path)?);
    *read += 1;
    for name in namespaced_names_in(&text) {
        named.push(format!("{relative} names `{name}`"));
    }
    for door in DOORS {
        if text.contains(door) {
            named.push(format!("{relative} names `{door}`"));
        }
    }
    Ok(())
}

/// Every distinct namespaced name `text` spells inside a string literal, in the
/// order it first spells them.
fn namespaced_names_in(text: &str) -> Vec<String> {
    let mut found: Vec<String> = Vec::new();
    for literal in text.split('"').skip(1).step_by(2) {
        if is_a_namespaced_name(literal) && !found.iter().any(|seen| seen == literal) {
            found.push(literal.to_owned());
        }
    }
    found
}

/// Whether `literal` is spelled the way a block name and a texture key both are:
/// two non-empty runs of lower-case letters, digits and underscores with one
/// colon between them.
fn is_a_namespaced_name(literal: &str) -> bool {
    let [namespace, name] = literal.split(':').collect::<Vec<&str>>()[..] else {
        return false;
    };
    [namespace, name].iter().all(|part| {
        !part.is_empty()
            && part.chars().all(|letter| {
                letter.is_ascii_lowercase() || letter.is_ascii_digit() || letter == '_'
            })
    })
}

/// A `.rs` file that is not a sibling unit-test file.
fn is_production_source(path: &Path) -> bool {
    path.file_name()
        .and_then(OsStr::to_str)
        .is_some_and(|file_name| file_name.ends_with(".rs") && !file_name.ends_with("_test.rs"))
}

/// A file's text with its whole-line comments removed.
fn production_text(source: &str) -> String {
    source
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<&str>>()
        .join("\n")
}

/// Where a file sits relative to `root`, spelled with `/` on every platform so a
/// report reads the same everywhere.
fn relative_spelling(path: &Path, root: &Path) -> Result<String, Box<dyn Error>> {
    Ok(path
        .strip_prefix(root)?
        .components()
        .filter_map(|component| component.as_os_str().to_str())
        .collect::<Vec<&str>>()
        .join("/"))
}

/// A mesher source committing every shape the guard carries, written under
/// `root`, and where it sits.
///
/// # Errors
///
/// Returns an error if the directory or the file cannot be written.
fn a_source_comparing_a_block_name(root: &Path) -> Result<String, Box<dyn Error>> {
    let mesher = root.join(SCANNED[0]);
    fs::create_dir_all(&mesher)?;
    fs::write(
        mesher.join("sweep.rs"),
        format!(
            "if quad.block.as_str() == \"{THE_NAME_A_SHORTCUT_WOULD_REACH_FOR}\" {{\n\
             \tlet sea = BlockName::parse(named)?;\n\
             \tlet key = TextureKey::parse(named)?;\n\
             }}\n"
        ),
    )?;
    Ok(format!("{}/sweep.rs", SCANNED[0]))
}
