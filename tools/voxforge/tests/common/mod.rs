//! Fixtures and verdicts shared by VoxForge's document and material tests.
//!
//! Every fixture here is text: a `.mcvox` document is text, a material file is
//! text, and the thing under test is precisely the reading of them. Where a
//! directory is needed it is a real temporary one, per `testing.md` §5 — a mock
//! of a directory read would assert nothing about the read.
//!
//! The refusal helpers return a **verdict** rather than a boolean, because
//! "this cause named everything it had to" and "this cause could not be
//! examined" must never read the same.

// Each test binary links this whole module and uses a subset of it.
#![allow(dead_code)]

pub mod cli;
pub mod preview;
pub mod texture;
pub mod tiles;

use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use tempfile::TempDir;
use voxforge::fault::{Fault, Origin};
use voxforge::format::{Model, Voxel, parse_document};
use voxforge::inspect::{Report, inspect};
use voxforge::name::MaterialKey;
use voxforge::volume::{StateSelection, Volume, assemble};

/// The error type every test in this suite propagates with `?`.
pub type TestResult = Result<(), Box<dyn Error>>;

/// The file name every parsed document fixture is attributed to.
///
/// A name and not a path: a refusal quotes its origin back, and an assertion on
/// an absolute path would be a Windows-only or Unix-only test.
pub const FIXTURE_FILE: &str = "fixture.mcvox";

/// The model `text` describes.
///
/// # Errors
///
/// Returns the refusal, when the document was not accepted.
pub fn loaded(text: &str) -> Result<Model, Box<dyn Error>> {
    Ok(parse_document(text, Origin::new(FIXTURE_FILE))?)
}

/// The refusal `text` earns.
///
/// # Errors
///
/// Returns an error when the document was accepted — a scenario about a refusal
/// asserts nothing if the document loaded.
pub fn refusal(text: &str) -> Result<Fault, Box<dyn Error>> {
    match parse_document(text, Origin::new(FIXTURE_FILE)) {
        // Named rather than dumped: a whole `Model` is pages of resolved grid
        // cells, and a failure nobody reads is a failure nobody acts on.
        Ok(model) => Err(format!(
            "this document must be refused, but loaded as `{}` with {} part(s)",
            model.name.as_str(),
            model.parts.len()
        )
        .into()),
        Err(fault) => Ok(fault),
    }
}

/// The volume `text` assembles to under `states`.
///
/// # Errors
///
/// Returns the refusal, when the document was not accepted or the model was not
/// assembled.
pub fn assembled(text: &str, states: &StateSelection) -> Result<Volume, Box<dyn Error>> {
    Ok(assemble(&loaded(text)?, states)?)
}

/// The refusal assembling `text` under `states` earns.
///
/// # Errors
///
/// Returns an error when the model assembled — a scenario about a refusal
/// asserts nothing if it did not happen.
pub fn assembly_refusal(text: &str, states: &StateSelection) -> Result<Fault, Box<dyn Error>> {
    match assemble(&loaded(text)?, states) {
        Ok(volume) => Err(format!(
            "this model must be refused, but assembled to {extent:?} holding {filled} filled voxel(s)",
            extent = volume.extent(),
            filled = volume.filled().len()
        )
        .into()),
        Err(fault) => Ok(fault),
    }
}

/// The report `text` earns, assembled with every part in its default state.
///
/// Both halves of the inspection come from one reading of the document, which
/// is what a caller has: a report whose stats and whose defects were computed
/// from two separate parses could disagree without either being wrong.
///
/// # Errors
///
/// Returns the refusal, when the document was not accepted or the model was not
/// assembled.
pub fn inspected(text: &str) -> Result<Report, Box<dyn Error>> {
    let model = loaded(text)?;
    let volume = assemble(&model, &StateSelection::default())?;
    Ok(inspect(&volume, &model))
}

/// Where every voxel of `material` sits in `volume`, ascending by position.
///
/// # Errors
///
/// Returns an error when `material` is not a namespaced key, which would make
/// the answer an empty list for a reason that has nothing to do with the model.
pub fn positions_of(volume: &Volume, material: &str) -> Result<Vec<Voxel>, Box<dyn Error>> {
    let key = MaterialKey::parse(material)?;
    Ok(volume
        .filled()
        .into_iter()
        .filter(|cell| cell.material == key)
        .map(|cell| cell.position)
        .collect())
}

/// The voxel at `x`, `y`, `z`.
#[must_use]
pub fn at(x: u32, y: u32, z: u32) -> Voxel {
    Voxel { x, y, z }
}

/// Every layer of a part sliced on `y` and filled solid with `paint`.
///
/// A solid part is what makes an assembled extent derivable from the declared
/// one: the assembled volume is normalised onto the art, so art that did not
/// reach its own declared corners would make every expected bound a second
/// calculation rather than the size written in the fixture.
#[must_use]
pub fn solid_y_layers(part: &str, size: (u32, u32, u32), paint: char) -> String {
    let (x, y, z) = size;
    let row: String = (0..x).map(|_| paint).collect();
    let art: Vec<String> = (0..z).map(|_| row.clone()).collect();
    let grid = art.join("\n");
    (0..y)
        .map(|plane| {
            format!("\n[[layers]]\npart = \"{part}\"\ny = {plane}\ngrid = \"\"\"\n{grid}\n\"\"\"\n")
        })
        .collect()
}

/// A torch: a solid handle, and a solid flame whose pivot sits inside its own
/// volume so that the flame reaches one voxel past the handle in both `−x` and
/// `−z`.
///
/// Shared by the attachment scenarios and the normalisation ones because they
/// are two readings of one placement, and two fixtures claiming to be this one
/// could drift apart without either failing. Its numbers:
///
/// - `flame`'s translation is `at − origin = (1, 10, 1) − (2, 0, 2) =
///   (−1, 10, −1)`, so its art spans `x −1..2`, `y 10..15`, `z −1..2`.
/// - `handle`'s art spans `x 0..1`, `y 0..9`, `z 0..1`.
/// - The model's lowest art is therefore `(−1, 0, −1)` and normalising adds
///   `(1, 0, 1)`, giving an assembled extent of `[4, 16, 4]`.
#[must_use]
pub fn torch() -> String {
    format!(
        r#"schema = 1
name = "base:torch"
scale = 16
slice = "y"

[palette]
"f" = "base:flame"
"w" = "base:oak_plank"

[[parts]]
name = "handle"
size = [2, 10, 2]
origin = [1, 0, 1]

[[parts]]
name = "flame"
size = [4, 6, 4]
origin = [2, 0, 2]
attach = {{ to = "handle", at = [1, 10, 1] }}
{handle}{flame}"#,
        handle = solid_y_layers("handle", (2, 10, 2), 'w'),
        flame = solid_y_layers("flame", (4, 6, 4), 'f'),
    )
}

/// Every one of `expected` that the fault's cause does not name.
///
/// Empty is the passing answer, and the assertion reads as an equality against
/// [`all_named`] so that a failure prints exactly which token was missing.
#[must_use]
pub fn unnamed<'a>(fault: &Fault, expected: &[&'a str]) -> Vec<&'a str> {
    expected
        .iter()
        .copied()
        .filter(|token| !fault.cause.contains(token))
        .collect()
}

/// Nothing — what [`unnamed`] answers for a cause that named everything it had
/// to.
#[must_use]
pub fn all_named() -> Vec<&'static str> {
    Vec::new()
}

/// Whether a cause names several things, and in which order.
///
/// An enumerated verdict rather than a comparison of two `Option`s: `None` sorts
/// below `Some`, so a missing first token would compare as "in order" and pass.
#[derive(Debug, PartialEq, Eq)]
pub enum Mention {
    /// Each token is present, and each follows the one before it.
    Ordered,
    /// A token is not named at all.
    Missing(String),
    /// Every token is named, but not in the order asked for.
    OutOfOrder(String),
}

/// Whether the fault's cause names `expected` in that order.
#[must_use]
pub fn mentioned_in_order(fault: &Fault, expected: &[&str]) -> Mention {
    let mut reached = 0;
    for token in expected {
        let Some(found) = fault.cause.find(token) else {
            return Mention::Missing((*token).to_owned());
        };
        if found < reached {
            return Mention::OutOfOrder((*token).to_owned());
        }
        reached = found;
    }
    Mention::Ordered
}

/// Writes `files` into `directory` and hands the directory back.
///
/// # Errors
///
/// Returns the I/O failure when a file cannot be written.
pub fn directory_holding(
    directory: &TempDir,
    files: &[(&str, &str)],
) -> Result<PathBuf, Box<dyn Error>> {
    let root = directory.path().join("materials");
    fs::create_dir_all(&root)?;
    for (name, text) in files {
        fs::write(root.join(name), text)?;
    }
    Ok(root)
}

/// Writes `text` as a document file inside `directory` and hands its path back.
///
/// # Errors
///
/// Returns the I/O failure when the file cannot be written.
pub fn document_file(directory: &TempDir, text: &str) -> Result<PathBuf, Box<dyn Error>> {
    let path = directory.path().join(FIXTURE_FILE);
    fs::write(&path, text)?;
    Ok(path)
}

/// The same text with every line ending written the way a Windows editor writes
/// it.
#[must_use]
pub fn as_crlf(text: &str) -> String {
    text.replace('\n', "\r\n")
}

/// A path as a refusal quotes it back.
#[must_use]
pub fn shown(path: &Path) -> String {
    path.display().to_string()
}
