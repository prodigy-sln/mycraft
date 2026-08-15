//! Which stream carries what, and what the process answers.
//!
//! The separation is the whole point: an agent branches on the exit code, reads
//! the output path off stdout, and reads the repair off stderr. So every
//! assertion here grades **both halves**. "Nothing on stdout" is satisfied by a
//! tool that writes nothing anywhere, and "a diagnostic on stderr" is satisfied
//! by a tool that also dumped it on stdout; only the pair says the streams are
//! actually separated.
//!
//! The stream contract these tests fix:
//!
//! - `preview` writes the output path, alone, as the first line of stdout. When
//!   it rendered a contact sheet, that path is followed by the sheet's own
//!   legend, one line per tile.
//! - `inspect` writes its report to stdout, one fact per line. The filled-voxel
//!   count sits behind `filled `, and a defect sits on a line saying `defect`
//!   and naming the palette key it is about in backticks.
//! - Every refusal is the `Fault`'s own text, on stderr, and stdout stays empty.

mod common;

use std::error::Error;
use std::fs;
use std::num::NonZeroU32;
use std::path::Path;

use common::cli::{
    Filled, Survival, Written, document_at, filled_in, invoke, material_file, materials_at,
    nothing_missing, survival, unnamed_in, written,
};
use common::preview::{Paint, paints, png_of, solid};
use common::{TestResult, assembled, shown};
use tempfile::TempDir;
use voxforge::fault::Origin;
use voxforge::format::load_document;
use voxforge::inspect::ExitCode;
use voxforge::material::load_materials;
use voxforge::render::{View, contact_sheet, render, to_png};
use voxforge::volume::{StateSelection, assemble};

/// The scale every preview in this file is rendered at.
const TWO_PER_VOXEL: NonZeroU32 = match NonZeroU32::new(2) {
    Some(scale) => scale,
    None => NonZeroU32::MIN,
};

/// A document that is not well-formed TOML, and so fails before any of its
/// meaning is looked at.
const UNPARSEABLE: &str = "schema = 1\nname =\n";

/// A model whose palette declares one key no grid ever spells.
///
/// Seven of its eight voxels are filled, `.` is spelled once so that the empty
/// marker is not itself unused, and `x` is spelled nowhere — exactly one defect,
/// so the report's own words about it can be graded without ambiguity.
const ONE_UNUSED_ENTRY: &str = r#"schema = 1
name = "base:probe"
scale = 16
size = [2, 2, 2]
origin = [0, 0, 0]
slice = "y"

[palette]
"." = "empty"
"r" = "base:ruby"
"x" = "base:lapis"

[[layers]]
y = 0
grid = """
rr
rr
"""

[[layers]]
y = 1
grid = """
rr
r.
"""
"#;

#[test]
fn a_preview_that_succeeds_writes_the_image_and_prints_its_path_and_nothing_else() -> TestResult {
    let temp = TempDir::new()?;
    let document = document_at(&temp, "cube.mcvox", &solid((2, 2, 2), Paint::Blue))?;
    let materials = materials_at(&temp, "materials", &fixture_materials())?;
    let out = temp.path().join("cube.png");
    let named = shown(&out);

    let run = single_view(&document, &out, &materials)?;

    assert_eq!(
        (
            run.code,
            run.out.trim_end(),
            run.err.as_str(),
            written(&out, &view_encoding(&document, &materials)?),
        ),
        (ExitCode::Success, named.as_str(), "", Written::ThePicture),
        "the path on stdout is what an agent opens next, so it is the whole of stdout and the picture behind it is the one the library encodes"
    );
    Ok(())
}

#[test]
fn a_document_that_fails_to_load_is_diagnosed_on_stderr_with_stdout_left_empty() -> TestResult {
    let temp = TempDir::new()?;
    let document = document_at(&temp, "broken.mcvox", UNPARSEABLE)?;
    let materials = materials_at(&temp, "materials", &fixture_materials())?;
    let out = temp.path().join("never-written.png");

    let run = single_view(&document, &out, &materials)?;

    assert_eq!(
        (
            run.code,
            run.out.as_str(),
            unnamed_in(&run.err, &[&shown(&document)]),
        ),
        (ExitCode::Defective, "", nothing_missing()),
        "an empty stdout on its own is satisfied by a tool that says nothing anywhere; the diagnostic naming the file is the other half. stderr was: {}",
        run.err
    );
    Ok(())
}

#[test]
fn a_document_that_fails_to_load_leaves_a_pre_existing_image_byte_for_byte_as_it_was() -> TestResult
{
    let temp = TempDir::new()?;
    let document = document_at(&temp, "broken.mcvox", UNPARSEABLE)?;
    let materials = materials_at(&temp, "materials", &fixture_materials())?;
    let out = temp.path().join("already-here.png");
    let before = a_larger_picture()?;
    fs::write(&out, &before)?;

    let run = single_view(&document, &out, &materials)?;

    assert_eq!(
        (run.code, survival(&out, &before)),
        (ExitCode::Defective, Survival::Untouched),
        "an author's last good render is what they compare against; a tool that opened the file before it knew it had a picture leaves them nothing"
    );
    Ok(())
}

#[test]
fn an_inspected_model_carrying_a_defect_reports_on_stdout_and_exits_non_zero() -> TestResult {
    let temp = TempDir::new()?;
    let document = document_at(&temp, "probe.mcvox", ONE_UNUSED_ENTRY)?;

    let run = invoke(&["inspect", &shown(&document)])?;

    assert_eq!(
        (
            run.code,
            run.err.as_str(),
            unnamed_in(&run.out, &["defect", "`x`"]),
            filled_in(&run.out),
        ),
        (
            ExitCode::Defective,
            "",
            nothing_missing(),
            Filled::Voxels(SEVEN_FILLED),
        ),
        "a report is not a refusal: it belongs on stdout with stderr left clean, and the non-zero exit is what an agent branches on. stdout was:\n{}",
        run.out
    );
    Ok(())
}

#[test]
fn a_preview_naming_no_view_writes_the_contact_sheet_and_prints_every_tile() -> TestResult {
    let temp = TempDir::new()?;
    let document = document_at(&temp, "cube.mcvox", &solid((2, 2, 2), Paint::Blue))?;
    let materials = materials_at(&temp, "materials", &fixture_materials())?;
    let out = temp.path().join("sheet.png");
    let (image, legend) = sheet_encoding(&document, &materials)?;

    let run = invoke(&[
        "preview",
        &shown(&document),
        "--out",
        &shown(&out),
        "--materials",
        &shown(&materials),
        "--pixels-per-voxel",
        "2",
    ])?;

    let mut expected = vec![shown(&out)];
    expected.extend(legend);
    assert_eq!(
        (
            run.code,
            run.out.lines().map(str::to_owned).collect::<Vec<String>>(),
            written(&out, &image),
        ),
        (ExitCode::Success, expected, Written::ThePicture),
        "the legend is what makes a sheet readable without pixels; a tool that computed it and printed nothing leaves an agent guessing which tile is which"
    );
    Ok(())
}

/// How many voxels [`ONE_UNUSED_ENTRY`] fills: four on its lower layer and three
/// on its upper one.
const SEVEN_FILLED: usize = 4 + 3;

/// The three fixture materials, each fully emissive so its colour survives every
/// face factor unchanged.
fn fixture_materials() -> Vec<(&'static str, String)> {
    vec![
        (
            "ruby.toml",
            material_file(Paint::Red.material(), "#ff0000", "1.0"),
        ),
        (
            "jade.toml",
            material_file(Paint::Green.material(), "#00ff00", "1.0"),
        ),
        (
            "lapis.toml",
            material_file(Paint::Blue.material(), "#0000ff", "1.0"),
        ),
    ]
}

/// What `preview --view front` does with `document`.
fn single_view(
    document: &Path,
    out: &Path,
    materials: &Path,
) -> Result<common::cli::Invocation, Box<dyn Error>> {
    invoke(&[
        "preview",
        &shown(document),
        "--out",
        &shown(out),
        "--materials",
        &shown(materials),
        "--view",
        View::Front.as_str(),
        "--pixels-per-voxel",
        "2",
    ])
}

/// The PNG the library encodes for `document`'s front view under `materials`.
fn view_encoding(document: &Path, materials: &Path) -> Result<Vec<u8>, Box<dyn Error>> {
    let (volume, table) = loaded(document, materials)?;
    Ok(to_png(
        &render(&volume, &table, View::Front, TWO_PER_VOXEL),
        Origin::new("preview.png"),
    )?)
}

/// The contact sheet the library encodes for `document`, and the legend it
/// reports alongside.
fn sheet_encoding(
    document: &Path,
    materials: &Path,
) -> Result<(Vec<u8>, Vec<String>), Box<dyn Error>> {
    let (volume, table) = loaded(document, materials)?;
    let sheet = contact_sheet(&volume, &table, TWO_PER_VOXEL);
    let bytes = to_png(sheet.image(), Origin::new("sheet.png"))?;
    Ok((bytes, sheet.legend()))
}

/// `document` assembled in every part's default state, with `materials` read.
fn loaded(
    document: &Path,
    materials: &Path,
) -> Result<(voxforge::volume::Volume, voxforge::material::MaterialTable), Box<dyn Error>> {
    let model = load_document(document)?;
    let table = load_materials(materials)?;
    let volume = assemble(&model, &StateSelection::default())?;
    Ok((volume, table))
}

/// A real PNG, and a much bigger one than any preview these tests ask for.
///
/// Bigger on purpose. The scenario it serves is about a **pre-existing image**,
/// and both its length and its content have to matter: a tool that opened the
/// path for writing before it knew whether it had a picture leaves a file of no
/// bytes, and one that wrote a partial encoding leaves a short prefix. Either is
/// shorter than this, and neither equals it.
fn a_larger_picture() -> Result<Vec<u8>, Box<dyn Error>> {
    let volume = assembled(&solid((6, 6, 6), Paint::Green), &StateSelection::default())?;
    png_of(&volume, &paints()?, View::Front)
}
