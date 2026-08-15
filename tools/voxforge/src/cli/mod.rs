//! The whole command line, over injected writers.
//!
//! **Every decision this tool makes lives here rather than in `main.rs`**, which
//! is three lines. The gate excludes the binary crates from coverage wholesale
//! because nothing runs `App`; a CLI whose argument parsing, dispatch, rendered
//! text and exit-code selection lived in its binary would earn the same
//! exclusion and the same blindness. The writers are injected for the same
//! reason: what a tool prints is a decision, and a decision nothing can observe
//! is a decision nothing grades.
//!
//! **A preview builds its whole byte vector before it opens the output file.**
//! A late refusal therefore cannot leave a truncated image where a good one was
//! — the file is opened once, with everything already in hand.

mod args;
mod report;

use std::ffi::OsString;
use std::io::Write;
use std::num::NonZeroU32;
use std::path::Path;

use args::{Cli, Command, PreviewArgs, TextureArgs};

use crate::fault::{Fault, Origin};
use crate::format::load_document;
use crate::inspect::{ExitCode, inspect_document};
use crate::material::{MaterialTable, load_materials};
use crate::render::{View, contact_sheet, pixels_per_voxel, render, to_png, view_named};
use crate::texture::{
    AxisAlignedView, EmittedFace, FaceSelection, SeamPolicy, SeamVerdict, TextureRequest,
    TextureSet, emit,
};
use crate::volume::{StateSelection, Volume, assemble};

/// Runs the tool over `argv`, writing to `out` and `err`.
///
/// `argv` is the whole command line as the process received it, program name
/// first.
#[must_use]
pub fn run(argv: Vec<OsString>, out: &mut dyn Write, err: &mut dyn Write) -> ExitCode {
    match dispatch(argv, out) {
        Ok(code) => code,
        // Deliberately the only place a refusal reaches a stream: stdout has had
        // nothing written to it on this path, which is what lets an agent read
        // stdout as the answer and stderr as the repair.
        //
        // A stderr that will not accept the diagnostic leaves nowhere to report
        // that it did not — the exit code is the whole of what survives, and it
        // already says the run failed.
        Err(fault) => match writeln!(err, "{fault}") {
            Ok(()) | Err(_) => ExitCode::Defective,
        },
    }
}

/// Does what `argv` asks, writing anything it has to say to `out`.
fn dispatch(argv: Vec<OsString>, out: &mut dyn Write) -> Result<ExitCode, Fault> {
    let cli: Cli = args::parse(argv)?;
    let states = cli.command.state_selection()?;
    match &cli.command {
        Command::Preview(asked) => preview(&Preview::of(asked), &states, out),
        Command::Inspect(asked) => inspect(&asked.document, &states, out),
        Command::Texture(asked) => texture(&Texture::of(asked), &states, out),
    }
}

/// Everything a preview needs to know before it reads anything.
struct Preview<'a> {
    /// The document to render.
    document: &'a Path,
    /// Where the image goes.
    destination: &'a Path,
    /// Which view, or every view tiled into a sheet.
    view: Option<&'a str>,
    /// How many pixels one voxel spans.
    scale: Option<u32>,
    /// Where material keys resolve from.
    materials: std::path::PathBuf,
}

impl<'a> Preview<'a> {
    /// What `asked` asks a preview for.
    fn of(asked: &'a PreviewArgs) -> Self {
        Self {
            document: &asked.document,
            destination: &asked.out,
            view: asked.view.as_deref(),
            scale: asked.pixels_per_voxel,
            materials: args::materials_of(asked.materials.as_ref()),
        }
    }
}

/// Renders `request` and writes the image where it asks.
fn preview(
    request: &Preview<'_>,
    states: &StateSelection,
    out: &mut dyn Write,
) -> Result<ExitCode, Fault> {
    let origin = Origin::new(request.document);
    let scale = pixels_per_voxel(args::scale_of(request.scale), origin.clone())?;
    let model = load_document(request.document)?;
    let materials = load_materials(&request.materials)?;
    model.bind_materials(&materials)?;
    let volume = assemble(&model, states)?;

    // The whole encoding exists before the file is opened, so a refusal from
    // anything above cannot leave a partial image behind.
    let scene = Scene {
        volume: &volume,
        materials: &materials,
        scale,
        origin: &origin,
    };
    let (bytes, legend) = match request.view {
        Some(spelled) => scene.single_view(spelled)?,
        None => scene.sheet()?,
    };
    deliver(request.destination, &bytes, &legend, out)
}

/// Writes the image, then says where it went.
///
/// In that order, and the order is the contract: the path on stdout is a
/// promise that the file is there, so nothing announces an image that has not
/// landed.
fn deliver(
    destination: &Path,
    bytes: &[u8],
    legend: &[String],
    out: &mut dyn Write,
) -> Result<ExitCode, Fault> {
    let origin = Origin::new(destination);
    std::fs::write(destination, bytes).map_err(|cause| {
        Fault::about(
            origin.clone(),
            format!("the image could not be written: {cause}"),
        )
    })?;
    // Propagated rather than ignored: the path on stdout *is* the answer, and a
    // run whose answer never arrived has not succeeded however good the image.
    said(writeln!(out, "{}", destination.display()), &origin)?;
    for line in legend {
        said(writeln!(out, "{line}"), &origin)?;
    }
    Ok(ExitCode::Success)
}

/// What a preview is rendered from, once the document has been read.
struct Scene<'a> {
    /// The assembled model.
    volume: &'a Volume,
    /// What its materials look like.
    materials: &'a MaterialTable,
    /// How many pixels one voxel spans.
    scale: NonZeroU32,
    /// The document the image is of, for anything that has to be refused.
    origin: &'a Origin,
}

impl Scene<'_> {
    /// One view's encoding, and no legend — a single view needs no key.
    fn single_view(&self, spelled: &str) -> Result<(Vec<u8>, Vec<String>), Fault> {
        let view: View = view_named(spelled, self.origin.clone())?;
        let preview = render(self.volume, self.materials, view, self.scale);
        Ok((to_png(&preview, self.origin.clone())?, Vec::new()))
    }

    /// Every view's encoding, tiled, and the legend naming each tile.
    ///
    /// The legend is the only thing that says which tile is which — the sheet
    /// carries no rendered text at all — so an encoding that succeeded and a
    /// legend nobody printed would be an image no reader can use.
    fn sheet(&self) -> Result<(Vec<u8>, Vec<String>), Fault> {
        let sheet = contact_sheet(self.volume, self.materials, self.scale);
        Ok((to_png(sheet.image(), self.origin.clone())?, sheet.legend()))
    }
}

/// Everything a texture emission needs to know before it reads anything.
struct Texture<'a> {
    /// The document to emit.
    document: &'a Path,
    /// The directory the images go into, one file per face.
    directory: &'a Path,
    /// Which face, when one is named.
    face: Option<&'a str>,
    /// Whether the block's whole six were asked for.
    all_faces: bool,
    /// Whether a texture that will not tile is refused rather than reported.
    seamless: bool,
    /// How many pixels one voxel spans.
    scale: Option<u32>,
    /// Where material keys resolve from.
    materials: std::path::PathBuf,
}

impl<'a> Texture<'a> {
    /// What `asked` asks a texture emission for.
    fn of(asked: &'a TextureArgs) -> Self {
        Self {
            document: &asked.document,
            directory: &asked.out,
            face: asked.face.as_deref(),
            all_faces: asked.all_faces,
            seamless: asked.seamless,
            scale: asked.pixels_per_voxel,
            materials: args::materials_of(asked.materials.as_ref()),
        }
    }
}

/// Emits `request` and writes one image per face where it asks.
fn texture(
    request: &Texture<'_>,
    states: &StateSelection,
    out: &mut dyn Write,
) -> Result<ExitCode, Fault> {
    let origin = Origin::new(request.document);
    let scale = pixels_per_voxel(args::scale_of(request.scale), origin.clone())?;
    let model = load_document(request.document)?;
    let materials = load_materials(&request.materials)?;
    model.bind_materials(&materials)?;
    let volume = assemble(&model, states)?;

    // Everything rendered, judged and encoded before any file is opened. That
    // is the all-or-nothing rule, and it is what a shell loop calling this tool
    // six times cannot give: by the time the fourth invocation refuses, three
    // files are already on disk.
    let set = emit(
        &volume,
        &materials,
        TextureRequest {
            faces: selection(request.face, request.all_faces, &origin)?,
            pixels_per_voxel: scale,
            scale: model.scale,
            seams: if request.seamless {
                SeamPolicy::Required
            } else {
                SeamPolicy::Reported
            },
            origin: origin.clone(),
        },
    )?;
    let encoded = encoded_faces(&set, &origin)?;
    deliver_faces(&encoded, request.directory, out)
}

/// Every emitted face's image as PNG bytes, before any of them is written.
fn encoded_faces<'a>(
    set: &'a TextureSet,
    origin: &Origin,
) -> Result<Vec<(&'a EmittedFace, Vec<u8>)>, Fault> {
    set.faces
        .iter()
        .map(|face| Ok((face, to_png(&face.image, origin.clone())?)))
        .collect()
}

/// Writes each face, then says where it went and what its seams said.
fn deliver_faces(
    encoded: &[(&EmittedFace, Vec<u8>)],
    directory: &Path,
    out: &mut dyn Write,
) -> Result<ExitCode, Fault> {
    written_together(encoded, directory)?;
    for (face, _) in encoded {
        let path = directory.join(named(face.face));
        said(
            writeln!(
                out,
                "{name} {path} ({columns}, {rows}) {verdicts}",
                name = face.face.as_str(),
                path = path.display(),
                columns = face.face.columns().as_str(),
                rows = face.face.rows().as_str(),
                verdicts = verdicts_of(&face.verdicts)
            ),
            &Origin::new(&path),
        )?;
    }
    Ok(ExitCode::Success)
}

/// Writes every face, removing the ones already written if one fails.
///
/// The all-or-nothing rule is about the *set*, and it has two halves. The
/// verdicts and the encodings are settled before anything is opened, which
/// covers every refusal this tool makes. This covers the other half: an
/// ordinary I/O failure — a full disk, a permission revoked mid-run — partway
/// through six files would otherwise leave a partial set behind, which is the
/// same outcome FR-10.5-S4 forbids arriving by a different road.
///
/// **Best-effort, and deliberately so.** Removing a file can fail too, and
/// there is nowhere left to report that: the emission is already being refused
/// for the reason that matters. A rename-into-place scheme would be atomic per
/// file and still not atomic across six, so it would buy precision rather than
/// the property. What is guaranteed is that this never leaves a partial set
/// *quietly* — the refusal names the write that failed.
fn written_together(encoded: &[(&EmittedFace, Vec<u8>)], directory: &Path) -> Result<(), Fault> {
    let mut landed: Vec<std::path::PathBuf> = Vec::new();
    for (face, bytes) in encoded {
        let path = directory.join(named(face.face));
        let Err(fault) = wrote(&path, bytes) else {
            landed.push(path);
            continue;
        };
        for written in landed {
            drop(std::fs::remove_file(written));
        }
        return Err(fault);
    }
    Ok(())
}

/// Every verdict a face carries, as one phrase.
fn verdicts_of(verdicts: &[SeamVerdict]) -> String {
    verdicts
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("; ")
}

/// The file one face's texture is written to, within the output directory.
fn named(face: AxisAlignedView) -> String {
    format!("{face}.png", face = face.as_str())
}

/// Which faces `spelled` asks for, or the block's whole six when it names none.
///
/// A name that is not a view at all is refused by `view_named` in the library's
/// own words; one that is a view but not a face — `iso-fl` — is refused by
/// `AxisAlignedView`, which is the one place that judgement lives.
fn selection(
    spelled: Option<&str>,
    all_faces: bool,
    origin: &Origin,
) -> Result<FaceSelection, Fault> {
    if let Some(spelled) = spelled {
        let view = view_named(spelled, origin.clone())?;
        return Ok(FaceSelection::One(AxisAlignedView::parse(view)?));
    }
    if all_faces {
        return Ok(FaceSelection::All);
    }
    // Neither flag. `clap` makes the two mutually exclusive but neither
    // required, so this is the one combination it lets through — and defaulting
    // it to the whole set would make `--all-faces` a word with no effect,
    // silently doing something the caller never asked for.
    Err(Fault::about(
        origin.clone(),
        "a texture emission names either one face, as `--face <name>`, or a block's whole six, as `--all-faces`",
    )
    .in_field("face"))
}

/// Puts `bytes` at `path`, as a refusal naming the path when it cannot.
fn wrote(path: &Path, bytes: &[u8]) -> Result<(), Fault> {
    let origin = Origin::new(path);
    std::fs::write(path, bytes)
        .map_err(|cause| Fault::about(origin, format!("the image could not be written: {cause}")))
}

/// Reports on `document`, writing the report to `out`.
///
/// Resolves no materials at all. Assembly does not need them, the per-material
/// counts come from the palette rather than from a table, and an author should
/// be able to inspect a document whose art directory is not to hand.
fn inspect(
    document: &Path,
    states: &StateSelection,
    out: &mut dyn Write,
) -> Result<ExitCode, Fault> {
    let report = inspect_document(document, states)?;
    said(report::write(&report, out), &Origin::new(document))?;
    Ok(report.exit_code())
}

/// The outcome of saying something, as a refusal when it could not be said.
///
/// Writing to a stream is not a formality here: what this tool prints is its
/// answer, so a write that failed is an answer nobody received.
fn said(outcome: std::io::Result<()>, origin: &Origin) -> Result<(), Fault> {
    outcome.map_err(|cause| {
        Fault::about(
            origin.clone(),
            format!("the report could not be written to the output stream: {cause}"),
        )
    })
}
