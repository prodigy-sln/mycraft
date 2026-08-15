//! What `texture` writes, what it says, and on which stream.
//!
//! An agent branches on the exit code, reads the paths off stdout and reads the
//! repair off stderr, so every assertion here grades **both halves**. "Nothing
//! on stdout" is satisfied by a tool that writes nothing anywhere, and "a
//! diagnostic on stderr" by one that also dumped it on stdout; only the pair
//! says the streams are separated.
//!
//! The contract this file fixes, because an agent parses it:
//!
//! - `--out` names a **directory**, and each emitted face lands in it as
//!   `<face>.png`.
//! - One stdout line per emitted face, in the fixed order front, back, left,
//!   right, top, bottom, each naming — in this order — the face, the path
//!   written, the two model axes its columns and rows run along as `(columns,
//!   rows)`, and its seam verdict.
//! - A refusal is the `Fault`'s own text on stderr, with stdout left empty.
//!
//! **Two scenarios here exist because `emit` cannot see them.** It takes no
//! paths and writes nothing, so the all-or-nothing rule — all six verdicts
//! computed and all six byte vectors built before any file is opened — is only
//! observable from out here, against files that were already on disk.

mod common;

use std::error::Error;
use std::fs;
use std::path::PathBuf;

use common::cli::{
    Exited, Survival, Written, built_binary, document_at, invoke, nothing_missing, survival,
    unnamed_in, written,
};
use common::preview::EIGHT_PER_VOXEL;
use common::texture::{GRADIENT, GREY, Leg, Tone, material_text, named_in_order};
use common::tiles::{gradient_depth, narrow_slab, solid_block};
use common::{Mention, TestResult, shown};
use tempfile::TempDir;
use voxforge::fault::Origin;
use voxforge::format::load_document;
use voxforge::inspect::ExitCode;
use voxforge::material::load_materials;
use voxforge::render::{View, render_texture, to_png};
use voxforge::texture::AxisAlignedView;
use voxforge::volume::{StateSelection, assemble};

/// The palette a one-grey fixture is painted from.
const PLAIN: [Tone; 1] = [GREY];

/// What a pre-existing file at an output path holds before a run that must not
/// touch it.
///
/// Long enough that a truncation is visible as a length rather than only as a
/// difference, and not a PNG, so that a tool overwriting it with a picture is
/// `Rewritten` rather than accidentally equal.
const ALREADY_THERE: &[u8] = b"this file was here first, and a refused emission leaves it alone.";

/// A document, its materials and an output directory, all on disk.
struct Workspace {
    /// The directory everything sits under, alive for as long as this is.
    _directory: TempDir,
    /// The document to emit.
    document: PathBuf,
    /// Where material keys resolve from.
    materials: PathBuf,
    /// Where the images go.
    out: PathBuf,
}

impl Workspace {
    /// A workspace holding `text` painted from `palette`.
    fn holding(text: &str, palette: &[Tone]) -> Result<Self, Box<dyn Error>> {
        let directory = TempDir::new()?;
        let document = document_at(&directory, "fixture.mcvox", text)?;
        let materials = directory.path().join("materials");
        fs::create_dir_all(&materials)?;
        for tone in palette {
            let leaf = tone.key.split(':').next_back().unwrap_or(tone.key);
            fs::write(materials.join(format!("{leaf}.toml")), material_text(*tone))?;
        }
        let out = directory.path().join("textures");
        fs::create_dir_all(&out)?;
        Ok(Self {
            _directory: directory,
            document,
            materials,
            out,
        })
    }

    /// Where this face's image is written.
    fn face_path(&self, face: AxisAlignedView) -> PathBuf {
        self.out.join(format!("{face}.png", face = face.as_str()))
    }

    /// The arguments a `texture` invocation of this workspace carries.
    fn arguments<'a>(&'a self, selection: &[&'a str]) -> Vec<&'a str> {
        let mut argv = vec!["texture", self.document_name(), "--out", self.out_name()];
        argv.extend_from_slice(selection);
        argv.extend(["--materials", self.materials_name()]);
        argv
    }

    /// The output directory, as a command line spells it.
    fn out_name(&self) -> &str {
        self.out.to_str().unwrap_or_default()
    }

    /// The materials directory, as a command line spells it.
    fn materials_name(&self) -> &str {
        self.materials.to_str().unwrap_or_default()
    }

    /// The document, as a command line spells it.
    fn document_name(&self) -> &str {
        self.document.to_str().unwrap_or_default()
    }

    /// The picture the library encodes for that face of this document.
    fn picture(&self, face: AxisAlignedView) -> Result<Vec<u8>, Box<dyn Error>> {
        let model = load_document(&self.document)?;
        let materials = load_materials(&self.materials)?;
        let volume = assemble(&model, &StateSelection::default())?;
        let image = render_texture(&volume, &materials, face, EIGHT_PER_VOXEL);
        Ok(to_png(&image, Origin::new(self.face_path(face)))?)
    }

    /// Puts [`ALREADY_THERE`] at every one of `faces`.
    fn occupy(&self, faces: &[AxisAlignedView]) -> Result<(), Box<dyn Error>> {
        for face in faces {
            fs::write(self.face_path(*face), ALREADY_THERE)?;
        }
        Ok(())
    }
}

/// Whether stdout carries one line per face, in the declared order, each naming
/// what a face line has to name.
#[derive(Debug, PartialEq, Eq)]
enum Reported {
    /// It does.
    OnePerFaceInOrder,
    /// Some other number of lines came out.
    Counted(usize),
    /// A line does not name what it must, or names it out of order.
    Line {
        /// Which line, counted from zero.
        at: usize,
        /// What was wrong with it.
        mention: Mention,
    },
}

/// Whether `out` carries one line per entry of `expected`, each naming its
/// tokens in order.
fn reported(out: &str, expected: &[Vec<String>]) -> Reported {
    let lines: Vec<&str> = out.lines().collect();
    if lines.len() != expected.len() {
        return Reported::Counted(lines.len());
    }
    for (at, (line, tokens)) in lines.iter().zip(expected).enumerate() {
        let borrowed: Vec<&str> = tokens.iter().map(String::as_str).collect();
        let mention = named_in_order(line, &borrowed);
        if mention != Mention::Ordered {
            return Reported::Line { at, mention };
        }
    }
    Reported::OnePerFaceInOrder
}

/// What one face's stdout line has to name, in order.
fn line_of(workspace: &Workspace, face: AxisAlignedView, verdict: &str) -> Vec<String> {
    vec![
        face.as_str().to_owned(),
        shown(&workspace.face_path(face)),
        format!(
            "({columns}, {rows})",
            columns = face.columns().as_str(),
            rows = face.rows().as_str()
        ),
        verdict.to_owned(),
    ]
}

#[test]
fn a_single_face_texture_writes_its_image_and_prints_one_line_about_it() -> TestResult {
    let workspace = Workspace::holding(&solid_block(), &PLAIN)?;
    let front = AxisAlignedView::parse(View::Front)?;

    let run = invoke(&workspace.arguments(&["--face", "front"]))?;

    assert_eq!(
        (
            run.code,
            reported(&run.out, &[line_of(&workspace, front, Leg::Tiles.token())]),
            written(&workspace.face_path(front), &workspace.picture(front)?),
        ),
        (
            ExitCode::Success,
            Reported::OnePerFaceInOrder,
            Written::ThePicture
        ),
        "the line is what an agent parses — the face it asked for, the path it opens next, the axes it maps onto a mesh, and the verdict it decides on. stderr was:\n{}",
        run.err
    );
    Ok(())
}

#[test]
fn a_texture_declared_seamless_that_does_not_tile_leaves_the_path_it_was_given_alone() -> TestResult
{
    let workspace = Workspace::holding(&narrow_slab(), &PLAIN)?;
    let front = AxisAlignedView::parse(View::Front)?;
    workspace.occupy(&[front])?;

    // Through the built binary, and the period case deliberately: nothing else
    // forces the command line to read `scale` from the document rather than from
    // the volume's own extent or a flag default.
    let run = built_binary(&workspace.arguments(&["--face", "front", "--seamless"]))?;

    assert_eq!(
        (
            run.exit,
            run.out.as_str(),
            unnamed_in(&run.err, &["period", "x axis", "3", "4"]),
            survival(&workspace.face_path(front), ALREADY_THERE),
        ),
        (
            Exited::NonZero(1),
            "",
            nothing_missing(),
            Survival::Untouched
        ),
        "a refusal is a repair on stderr and nothing on stdout, and a file that was already there is not collateral. stderr was:\n{}",
        run.err
    );
    Ok(())
}

#[test]
fn a_face_that_is_not_one_of_a_blocks_six_is_refused_listing_the_ones_that_are() -> TestResult {
    let workspace = Workspace::holding(&solid_block(), &PLAIN)?;

    let run = invoke(&workspace.arguments(&["--face", "iso-fl"]))?;

    assert_eq!(
        (
            run.code,
            run.out.as_str(),
            unnamed_in(
                &run.err,
                &["iso-fl", "front", "back", "left", "right", "top", "bottom"]
            ),
        ),
        (ExitCode::Defective, "", nothing_missing()),
        "an isometric texture is representable nonsense, and refusing it once is what keeps texture emission from being `preview` with a flag. stderr was:\n{}",
        run.err
    );
    Ok(())
}

#[test]
fn a_face_set_declared_seamless_with_one_bad_face_writes_none_of_the_six() -> TestResult {
    let workspace = Workspace::holding(&gradient_depth(), &GRADIENT)?;
    workspace.occupy(&AxisAlignedView::ALL)?;

    let run = invoke(&workspace.arguments(&["--all-faces", "--seamless"]))?;

    assert_eq!(
        (run.code, run.out.as_str(), survivals(&workspace)),
        (ExitCode::Defective, "", every_face_untouched()),
        "`front` and `back` of this model tile, so a set that wrote as it went would leave two pictures behind — all six verdicts are decided before any file is opened. stderr was:\n{}",
        run.err
    );
    Ok(())
}

#[test]
fn an_undeclared_face_set_writes_all_six_and_reports_each_faces_verdict() -> TestResult {
    let workspace = Workspace::holding(&gradient_depth(), &GRADIENT)?;

    let run = invoke(&workspace.arguments(&["--all-faces"]))?;

    assert_eq!(
        (
            run.code,
            reported(&run.out, &set_lines(&workspace)),
            pictures(&workspace)?
        ),
        (
            ExitCode::Success,
            Reported::OnePerFaceInOrder,
            every_face_written()
        ),
        "an undeclared texture that does not tile is legitimate content, so the set is written and every face's verdict is stated. stderr was:\n{}",
        run.err
    );
    Ok(())
}

/// What became of the file that was already at each of the six paths.
fn survivals(workspace: &Workspace) -> Vec<(&'static str, Survival)> {
    AxisAlignedView::ALL
        .into_iter()
        .map(|face| {
            (
                face.as_str(),
                survival(&workspace.face_path(face), ALREADY_THERE),
            )
        })
        .collect()
}

/// Six untouched files — what a refused set leaves behind.
fn every_face_untouched() -> Vec<(&'static str, Survival)> {
    AxisAlignedView::ALL
        .into_iter()
        .map(|face| (face.as_str(), Survival::Untouched))
        .collect()
}

/// What sits at each of the six paths, against the picture the library encodes.
fn pictures(workspace: &Workspace) -> Result<Vec<(&'static str, Written)>, Box<dyn Error>> {
    let mut found = Vec::new();
    for face in AxisAlignedView::ALL {
        let expected = workspace.picture(face)?;
        found.push((
            face.as_str(),
            written(&workspace.face_path(face), &expected),
        ));
    }
    Ok(found)
}

/// Six written pictures — what an undeclared set leaves behind.
fn every_face_written() -> Vec<(&'static str, Written)> {
    AxisAlignedView::ALL
        .into_iter()
        .map(|face| (face.as_str(), Written::ThePicture))
        .collect()
}

/// What each of the six lines of an undeclared set of the depth gradient says.
///
/// `front` sees only the `z = 3` layer and `back` only the `z = 0` one, so both
/// are uniform and both tile. The other four run the gradient along one of their
/// own in-plane axes, and every one of those axes is `z`.
fn set_lines(workspace: &Workspace) -> Vec<Vec<String>> {
    AxisAlignedView::ALL
        .into_iter()
        .map(|face| {
            let verdict = match face.as_str() {
                "front" | "back" => Leg::Tiles.token(),
                _ => Leg::Edges.token(),
            };
            line_of(workspace, face, verdict)
        })
        .collect()
}
