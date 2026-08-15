//! The argument surface: which document, which materials, and which state.
//!
//! Three of the four scenarios here are the same shape, and it is the shape
//! `testing.md` §2 calls "policy is not wiring". The library already resolves a
//! materials directory, already selects a part's state and already refuses a
//! scale of zero, and every one of those is tested — against the library. None
//! of that says the command line ever passes the flag along. So each assertion
//! below is made against **the bytes that reached the file**, compared with what
//! the library encodes from the same inputs: a `--materials` the tool parsed and
//! dropped, or a `StateSelection` it built and never handed to the assembler,
//! writes a picture that differs and is caught here rather than nowhere.
//!
//! Every fixture material is fully emissive on purpose. A face factor below 1
//! would still be graded — the comparison is against the library's own
//! encoding — but a fully emissive material makes the picture the *declared*
//! colour on every face, so a failure prints a difference an author can read
//! rather than one they have to decode.

mod common;

use std::error::Error;
use std::fs;
use std::num::NonZeroU32;
use std::path::Path;

use common::cli::{
    Rendered, document_at, invoke, material_file, materials_at, nothing_missing, repository_path,
    unnamed_in,
};
use common::preview::{Encodings, Paint, compared, solid};
use common::{TestResult, shown, solid_y_layers};
use tempfile::TempDir;
use voxforge::fault::Origin;
use voxforge::format::{PartName, StateName, load_document};
use voxforge::inspect::ExitCode;
use voxforge::material::load_materials;
use voxforge::render::{View, render, to_png};
use voxforge::volume::{StateSelection, assemble};

/// The scale every preview in this file is rendered at.
///
/// Two rather than the tool's default of eight: nothing here grades the raster,
/// and a sixteenth of the pixels is a sixteenth of the time.
const TWO_PER_VOXEL: NonZeroU32 = match NonZeroU32::new(2) {
    Some(scale) => scale,
    None => NonZeroU32::MIN,
};

/// The view every preview in this file is taken from.
const FROM: View = View::IsoFl;

/// A colour no file under `content/base/materials` declares.
const CYAN: &str = "#00c8ff";

/// A second such colour, so that two material directories disagree about the
/// same key by more than a rounding.
const AMBER: &str = "#ffb000";

#[test]
fn a_document_outside_the_models_directory_renders_the_same_image_as_the_one_inside_it()
-> TestResult {
    let temp = TempDir::new()?;
    let inside = repository_path("content/base/models/reference-asymmetric.mcvox");
    let outside = document_at(&temp, "elsewhere.mcvox", &fs::read_to_string(&inside)?)?;
    let materials = repository_path("content/base/materials");

    let within = preview_of(&inside, &temp.path().join("within.png"), &materials, &[])?;
    let beyond = preview_of(&outside, &temp.path().join("beyond.png"), &materials, &[])?;

    assert_eq!(
        (
            within.code,
            beyond.code,
            compared(&within.image, &beyond.image),
            within.image.is_empty(),
        ),
        (
            ExitCode::Success,
            ExitCode::Success,
            Encodings::Identical,
            false,
        ),
        "the same document under two paths is the same model; the emptiness check is what stops two pictures nobody wrote comparing equal"
    );
    Ok(())
}

#[test]
fn a_document_path_that_does_not_exist_is_reported_by_name_with_a_failing_exit() -> TestResult {
    let temp = TempDir::new()?;
    let absent = temp.path().join("no-such-model.mcvox");
    let named = shown(&absent);

    let run = invoke(&["inspect", &named])?;

    assert_eq!(
        (run.code, unnamed_in(&run.err, &[&named])),
        (ExitCode::Defective, nothing_missing()),
        "an agent repairing its own command line has only the diagnostic to repair from, so the path it got wrong has to be in it; stderr was: {}",
        run.err
    );
    Ok(())
}

#[test]
fn naming_a_materials_directory_resolves_every_key_from_that_directory() -> TestResult {
    let temp = TempDir::new()?;
    let document = document_at(&temp, "cube.mcvox", &solid((2, 2, 2), Paint::Red))?;
    let cyan = materials_at(&temp, "cyan", &palette_declaring(CYAN))?;
    let amber = materials_at(&temp, "amber", &palette_declaring(AMBER))?;

    let under_cyan = preview_of(&document, &temp.path().join("cyan.png"), &cyan, &[])?;
    let under_amber = preview_of(&document, &temp.path().join("amber.png"), &amber, &[])?;
    let from_cyan = encoded(&document, &cyan, &StateSelection::default())?;
    let from_amber = encoded(&document, &amber, &StateSelection::default())?;

    assert_eq!(
        (
            under_cyan.code,
            compared(&under_cyan.image, &from_cyan),
            compared(&under_amber.image, &from_amber),
            from_cyan == from_amber,
        ),
        (
            ExitCode::Success,
            Encodings::Identical,
            Encodings::Identical,
            false,
        ),
        "each run must take its colours from the directory it was handed, and the last element is the fixture's own guard that the two directories are distinguishable at all"
    );
    Ok(())
}

#[test]
fn naming_a_part_and_one_of_its_states_renders_that_states_layers() -> TestResult {
    let temp = TempDir::new()?;
    let document = document_at(&temp, "torch.mcvox", &torch())?;
    let materials = materials_at(&temp, "materials", &palette_declaring(CYAN))?;
    let chosen = StateSelection::default().with(PartName::new("flame"), StateName::new("high"));

    let run = preview_of(
        &document,
        &temp.path().join("high.png"),
        &materials,
        &["--state", "flame=high"],
    )?;
    let high = encoded(&document, &materials, &chosen)?;
    let default = encoded(&document, &materials, &StateSelection::default())?;

    assert_eq!(
        (run.code, compared(&run.image, &high), high == default),
        (ExitCode::Success, Encodings::Identical, false),
        "`low` fills one voxel of the flame and `high` fills eight, so a selection the tool built and never passed on renders a shorter model"
    );
    Ok(())
}

#[test]
fn a_scale_of_zero_pixels_per_voxel_is_refused_in_the_librarys_own_words() -> TestResult {
    let temp = TempDir::new()?;
    let document = document_at(&temp, "cube.mcvox", &solid((2, 2, 2), Paint::Red))?;
    let materials = materials_at(&temp, "materials", &palette_declaring(CYAN))?;
    let out = temp.path().join("nothing.png");

    let run = invoke(&[
        "preview",
        &shown(&document),
        "--out",
        &shown(&out),
        "--materials",
        &shown(&materials),
        "--pixels-per-voxel",
        "0",
    ])?;

    assert_eq!(
        (
            run.code,
            run.out.as_str(),
            unnamed_in(&run.err, &["minimum is 1"]),
        ),
        (ExitCode::Defective, "", nothing_missing()),
        "the refusal has to be the one the library composes, or a command line that quietly substituted a scale of 1 would render a picture and say nothing; stderr was: {}",
        run.err
    );
    Ok(())
}

/// The three fixture materials, with the red one declared as `colour`.
///
/// All three are declared because a document built by `solid` names all three in
/// its palette, and the loader binds every palette entry against the table
/// before anything is rendered.
fn palette_declaring(colour: &str) -> Vec<(&'static str, String)> {
    vec![
        (
            "ruby.toml",
            material_file(Paint::Red.material(), colour, "1.0"),
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

/// A stateless handle and a flame declaring two states, with no art yet.
const TORCH_HEADER: &str = r#"schema = 1
name = "base:torch"
scale = 16
slice = "y"

[palette]
"." = "empty"
"r" = "base:ruby"
"g" = "base:jade"

[[parts]]
name = "handle"
size = [2, 3, 2]
origin = [0, 0, 0]

[[parts]]
name = "flame"
size = [2, 2, 2]
origin = [0, 0, 0]
states = ["low", "high"]
attach = { to = "handle", at = [0, 3, 0] }
"#;

/// A torch whose flame declares two states filling one voxel and eight.
///
/// The counts differ, and so does the reach: `high` puts art a whole layer above
/// anything `low` does, so the assembled model is taller and the two states
/// cannot render to pictures of the same size. A fixture whose states differed
/// only in colour would let a wrong selection produce a plausible image.
fn torch() -> String {
    format!(
        "{TORCH_HEADER}{handle}{low}{high_floor}{high_ceiling}",
        handle = solid_y_layers("handle", (2, 3, 2), 'g'),
        low = flame_layer("low", 0, &["r.", ".."]),
        high_floor = flame_layer("high", 0, &["rr", "rr"]),
        high_ceiling = flame_layer("high", 1, &["rr", "rr"]),
    )
}

/// One layer of the flame, belonging to `state`, holding `rows`.
fn flame_layer(state: &str, plane: u32, rows: &[&str]) -> String {
    let grid = rows.join("\n");
    format!(
        "\n[[layers]]\npart = \"flame\"\nstate = \"{state}\"\ny = {plane}\ngrid = \"\"\"\n{grid}\n\"\"\"\n"
    )
}

/// What `preview` does with `document`, plus whatever landed at `out`.
fn preview_of(
    document: &Path,
    out: &Path,
    materials: &Path,
    extra: &[&str],
) -> Result<Rendered, Box<dyn Error>> {
    let (document, written_to, materials) = (shown(document), shown(out), shown(materials));
    let mut arguments = vec![
        "preview",
        &document,
        "--out",
        &written_to,
        "--materials",
        &materials,
        "--view",
        FROM.as_str(),
        "--pixels-per-voxel",
        "2",
    ];
    arguments.extend_from_slice(extra);
    let run = invoke(&arguments)?;
    Ok(Rendered {
        code: run.code,
        image: fs::read(out).unwrap_or_default(),
    })
}

/// The PNG the library itself encodes for `document` under `materials` and
/// `states`, at this file's view and scale.
///
/// The oracle every assertion here compares against. It shares no code with the
/// command line — it is the same library call the command line is supposed to
/// be making — so a flag that never arrived shows up as different bytes.
fn encoded(
    document: &Path,
    materials: &Path,
    states: &StateSelection,
) -> Result<Vec<u8>, Box<dyn Error>> {
    let model = load_document(document)?;
    let table = load_materials(materials)?;
    let volume = assemble(&model, states)?;
    Ok(to_png(
        &render(&volume, &table, FROM, TWO_PER_VOXEL),
        Origin::new("preview.png"),
    )?)
}
