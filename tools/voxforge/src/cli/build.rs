//! `voxforge build` — one command from a manifest to a named texture set.
//!
//! **The whole set is delivered together or not at all.** Every source is read,
//! every model rendered, every image encoded and the index rendered before any
//! file is opened, so a manifest refused on its fourth entry leaves the previous
//! build's set exactly as it was — which matters because a consumer that met a
//! half-written set could not tell it from a finished one.
//!
//! **Entries are grouped by model, and each model is emitted whole.** Six faces
//! come out of one load, one assembly and one render pass, so a manifest naming
//! six faces of one block costs one render rather than six. It is also what
//! makes a model that is not a cube refusable at all: that precondition belongs
//! to a face *set*, and a per-entry emission of one face would never ask it.
//!
//! **The cache key is the fold, and it is whole-set.** If the value folded over
//! the sources matches the one the index records and every image the index names
//! is present, nothing is opened and nothing is written. Per-entry caching would
//! need a second, finer-grained record that the client reading the index would
//! then also have to understand, for seven small images.

use std::collections::BTreeSet;
use std::io::Write;
use std::num::NonZeroU32;
use std::path::{Path, PathBuf};

use mc_core::art::{INDEX_FILE_NAME, IndexEntry, TextureSetIndex, folded_sources};
use mc_core::content::TEXTURE_EDGE;
use mc_core::id::TextureKey;

use super::{said, written_together};
use crate::fault::{Fault, Origin};
use crate::format::load_document;
use crate::inspect::ExitCode;
use crate::material::{MaterialTable, load_materials};
use crate::render::to_png;
use crate::texture::manifest::{Manifest, ManifestEntry, load_manifest};
use crate::texture::{
    EmittedFace, FaceSelection, SeamPolicy, SeamVerdict, TextureRequest, TextureSet, emit,
};
use crate::volume::{StateSelection, assemble};

/// What a build says when it found its output current.
///
/// Contract rather than prose: whoever runs the build reads this line to tell a
/// set that was rebuilt from one that did not need to be.
const NOTHING_REBUILT: &str = "nothing needed rebuilding";

/// The extension a material declaration is written with.
const MATERIAL_EXTENSION: &str = "toml";

/// The extension a block declaration is written with.
const DECLARATION_EXTENSION: &str = "luau";

/// A source the build reaches: where it is, and how the index records it.
#[derive(Debug)]
struct Reaching {
    /// Where the file sits on disk.
    at: PathBuf,
    /// The path the index records, relative to the manifest's own directory.
    recorded: String,
    /// The key whose entry named it, where an entry did.
    key: Option<TextureKey>,
}

/// One source, read.
#[derive(Debug)]
struct Source {
    /// The path the index records.
    recorded: String,
    /// The bytes the fold is taken over.
    bytes: Vec<u8>,
}

/// One image the build baked, before anything is written.
#[derive(Debug)]
struct Image {
    /// The key the image is the art for.
    key: TextureKey,
    /// The file name it is written under.
    name: String,
    /// The encoded PNG.
    bytes: Vec<u8>,
}

/// Bakes the manifest at `document` into the set it names.
///
/// # Errors
///
/// Returns a [`Fault`] naming what is wrong when the manifest cannot be read, a
/// source it reaches cannot be read, a model will not bake to a block texture,
/// or a file cannot be written.
pub fn build(document: &Path, out: &mut dyn Write) -> Result<ExitCode, Fault> {
    let manifest = load_manifest(document)?;
    let directory = document.parent().unwrap_or_else(|| Path::new("."));
    let sources = read_all(reached(document, directory, &manifest)?)?;
    let fold = folded_sources(&folding(&sources));
    let output = directory.join(&manifest.output);
    let code = if current(&output, fold) {
        said(writeln!(out, "{NOTHING_REBUILT}"), &Origin::new(&output))?;
        ExitCode::Success
    } else {
        let images = baked(directory, &manifest)?;
        let index = index_text(fold, &sources, &images, document)?;
        delivered(&output, &images, &index, out)?
    };
    // Last, and after every path this build promised: whether a key is drawn
    // with is not something the build did, and a reader looking for what was
    // written should not have to step over advice to find it.
    reported_unused(directory, &manifest, out)?;
    Ok(code)
}

/// Every source the manifest reaches, in fold order.
///
/// The manifest first, then each model in the order of its first mention,
/// de-duplicated, then every material declaration sorted by file name. Exactly
/// what the build consumes and nothing else: folding the whole content tree
/// would turn an edit to a model nobody bakes into a client that refuses to
/// launch until somebody rebuilds textures that did not change.
fn reached(document: &Path, directory: &Path, manifest: &Manifest) -> Result<Vec<Reaching>, Fault> {
    let mut reaching = vec![Reaching {
        at: document.to_owned(),
        recorded: manifest_named(document),
        key: None,
    }];
    let mut mentioned: BTreeSet<String> = BTreeSet::new();
    for entry in &manifest.entries {
        let recorded = as_recorded(&entry.model);
        if !mentioned.insert(recorded.clone()) {
            continue;
        }
        reaching.push(Reaching {
            at: directory.join(&entry.model),
            recorded,
            key: Some(entry.key.clone()),
        });
    }
    reaching.extend(material_declarations(directory, manifest)?);
    Ok(reaching)
}

/// Every material declaration the build reads, sorted by file name.
///
/// Every `*.toml` in the directory and nothing that is not one, because that is
/// exactly what the material loader reads: folding a stray note beside them
/// would make the set stale for an input nothing consumed.
fn material_declarations(directory: &Path, manifest: &Manifest) -> Result<Vec<Reaching>, Fault> {
    let at = directory.join(&manifest.materials);
    let listing = std::fs::read_dir(&at).map_err(|cause| {
        Fault::about(
            Origin::new(&at),
            format!("the materials directory could not be read: {cause}"),
        )
    })?;
    let mut named: Vec<String> = listing
        .filter_map(Result::ok)
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| is_a_declaration(name))
        .collect();
    named.sort();
    let held = as_recorded(&manifest.materials);
    Ok(named
        .into_iter()
        .map(|name| Reaching {
            at: at.join(&name),
            recorded: format!("{held}/{name}"),
            key: None,
        })
        .collect())
}

/// Whether `name` is a material declaration.
fn is_a_declaration(name: &str) -> bool {
    Path::new(name)
        .extension()
        .is_some_and(|extension| extension == MATERIAL_EXTENSION)
}

/// Reads every source, in order.
fn read_all(reaching: Vec<Reaching>) -> Result<Vec<Source>, Fault> {
    reaching.into_iter().map(read_one).collect()
}

/// Reads one source.
///
/// A source that cannot be read refuses the build rather than folding as empty:
/// a value that is not a function of what the build consumed would report the
/// set current forever against a file nobody can open.
fn read_one(reaching: Reaching) -> Result<Source, Fault> {
    let bytes = std::fs::read(&reaching.at).map_err(|cause| {
        let named = match &reaching.key {
            Some(key) => format!(", named by the entry for `{key}`", key = key.as_str()),
            None => String::new(),
        };
        Fault::about(
            Origin::new(&reaching.at),
            format!("this source could not be read{named}: {cause}"),
        )
    })?;
    Ok(Source {
        recorded: reaching.recorded,
        bytes,
    })
}

/// Every source as the fold takes it.
fn folding(sources: &[Source]) -> Vec<(&str, &[u8])> {
    sources
        .iter()
        .map(|source| (source.recorded.as_str(), source.bytes.as_slice()))
        .collect()
}

/// Whether the set at `output` was built from sources folding to `fold`, and is
/// all still there.
///
/// The images are checked for **presence** and not for content. A tampered image
/// therefore survives a build, which is the stated consequence of a whole-set
/// cache key: what says the set is current is the fold over its sources.
fn current(output: &Path, fold: u64) -> bool {
    let Ok(text) = std::fs::read_to_string(output.join(INDEX_FILE_NAME)) else {
        return false;
    };
    let Ok(index) = TextureSetIndex::parse(&text) else {
        return false;
    };
    index.fold() == fold
        && index
            .entries()
            .iter()
            .all(|entry| output.join(&entry.image).is_file())
}

/// Every image the manifest asks for, encoded, before any of them is written.
fn baked(directory: &Path, manifest: &Manifest) -> Result<Vec<Image>, Fault> {
    let materials = load_materials(&directory.join(&manifest.materials))?;
    let mut baked: Vec<Image> = Vec::new();
    for model in models_named(manifest) {
        let at = directory.join(&model);
        let set = set_of(&at, &materials, manifest)?;
        for entry in manifest.entries.iter().filter(|entry| entry.model == model) {
            baked.push(image_of(&set, entry, &Origin::new(&at))?);
        }
    }
    Ok(baked)
}

/// Every distinct model the manifest names, in the order of its first mention.
fn models_named(manifest: &Manifest) -> Vec<PathBuf> {
    let mut named: Vec<PathBuf> = Vec::new();
    for entry in &manifest.entries {
        if !named.contains(&entry.model) {
            named.push(entry.model.clone());
        }
    }
    named
}

/// The six faces of the model at `at`.
fn set_of(at: &Path, materials: &MaterialTable, manifest: &Manifest) -> Result<TextureSet, Fault> {
    let origin = Origin::new(at);
    let model = load_document(at)?;
    refuse_an_edge_that_is_not_a_blocks(model.scale, manifest.pixels_per_voxel, &origin)?;
    model.bind_materials(materials)?;
    let volume = assemble(&model, &StateSelection::default())?;
    emit(
        &volume,
        materials,
        TextureRequest {
            faces: FaceSelection::All,
            pixels_per_voxel: manifest.pixels_per_voxel,
            scale: model.scale,
            // Reported, not required: which faces have to tile is decided by
            // what the manifest selected, and a verdict on a face nobody asked
            // for is not this build's business.
            seams: SeamPolicy::Reported,
            origin,
        },
    )
}

/// Refuses a model that would not bake to a block texture's own edge.
///
/// A model's declared scale, the manifest's pixels per voxel and the edge a
/// block texture has are three numbers with nothing connecting them, and this is
/// the only place they meet. Left unchecked, a `scale = 32` model bakes a 32x32
/// set that builds cleanly, commits cleanly, passes the gate, and refuses the
/// launch with a message about an *image* — pointing an author at a file they
/// never authored.
fn refuse_an_edge_that_is_not_a_blocks(
    scale: NonZeroU32,
    pixels_per_voxel: NonZeroU32,
    origin: &Origin,
) -> Result<(), Fault> {
    let baked = scale.get().saturating_mul(pixels_per_voxel.get());
    if baked == TEXTURE_EDGE {
        return Ok(());
    }
    Err(Fault::about(
        origin.clone(),
        format!(
            "a block texture is {TEXTURE_EDGE} pixels on an edge, but this model's declared scale of {scale} at {pixels_per_voxel} pixel(s) per voxel bakes {baked}",
            scale = scale.get(),
            pixels_per_voxel = pixels_per_voxel.get()
        ),
    )
    .in_field("scale"))
}

/// The image one entry asks for, encoded.
fn image_of(set: &TextureSet, entry: &ManifestEntry, origin: &Origin) -> Result<Image, Fault> {
    let Some(emitted) = set.faces.iter().find(|face| face.face == entry.face) else {
        return Err(Fault::about(
            origin.clone(),
            format!(
                "this model emitted no `{face}` face",
                face = entry.face.as_str()
            ),
        )
        .in_field("face"));
    };
    refuse_a_face_that_will_not_tile(emitted, entry, origin)?;
    Ok(Image {
        key: entry.key.clone(),
        name: entry.image.clone(),
        bytes: to_png(&emitted.image, origin.clone())?,
    })
}

/// Refuses a selected face whose verdict says it will not tile.
///
/// Judged **per entry** and never per emitted face. The build renders all six so
/// that the cubic precondition is asked at all, but a verdict on a face no entry
/// selected is not this build's business: refusing on one would refuse a set for
/// a face the manifest never wanted, and every positive scenario would still
/// pass. The first failing leg is the one reported, so the diagnostic does not
/// depend on which of several a scan happened to reach first.
fn refuse_a_face_that_will_not_tile(
    emitted: &EmittedFace,
    entry: &ManifestEntry,
    origin: &Origin,
) -> Result<(), Fault> {
    let failing = emitted
        .verdicts
        .iter()
        .find(|verdict| **verdict != SeamVerdict::TilesAcrossEveryEdge);
    let Some(verdict) = failing else {
        return Ok(());
    };
    Err(Fault::about(
        origin.clone(),
        format!(
            "the {face} face, which the entry for `{key}` bakes, will not tile: {verdict}",
            face = entry.face.as_str(),
            key = entry.key.as_str()
        ),
    )
    .in_field("face"))
}

/// The index the set carries, as the text it is written as.
fn index_text(
    fold: u64,
    sources: &[Source],
    images: &[Image],
    document: &Path,
) -> Result<String, Fault> {
    let recorded = sources
        .iter()
        .map(|source| source.recorded.clone())
        .collect();
    let entries = images
        .iter()
        .map(|image| IndexEntry {
            key: image.key.clone(),
            image: image.name.clone(),
        })
        .collect();
    let index = TextureSetIndex::stating(fold, recorded, entries)
        .map_err(|cause| Fault::about(Origin::new(document), cause.to_string()).in_field("key"))?;
    Ok(index.rendered())
}

/// Writes the whole set, then says where every file went.
///
/// In that order, and the order is the contract: a path on stdout is a promise
/// that the file is there.
fn delivered(
    output: &Path,
    images: &[Image],
    index: &str,
    out: &mut dyn Write,
) -> Result<ExitCode, Fault> {
    let origin = Origin::new(output);
    std::fs::create_dir_all(output).map_err(|cause| {
        Fault::about(
            origin.clone(),
            format!("the output directory could not be made: {cause}"),
        )
    })?;
    let mut files: Vec<(PathBuf, Vec<u8>)> = images
        .iter()
        .map(|image| (output.join(&image.name), image.bytes.clone()))
        .collect();
    files.push((output.join(INDEX_FILE_NAME), index.as_bytes().to_vec()));
    written_together(&files)?;
    for (path, _) in &files {
        said(writeln!(out, "{path}", path = path.display()), &origin)?;
    }
    Ok(ExitCode::Success)
}

/// Says which of the manifest's keys no block declaration spells.
///
/// **Advisory, and never a refusal.** The manifest and the block files are
/// edited by different hands at different times, so a build that stopped because
/// a block had not been written yet would be wrong about which of the two is
/// unfinished.
fn reported_unused(
    directory: &Path,
    manifest: &Manifest,
    out: &mut dyn Write,
) -> Result<(), Fault> {
    let at = directory.join(&manifest.blocks);
    let declared = declarations(&at);
    let origin = Origin::new(&at);
    for entry in &manifest.entries {
        if declared.contains(entry.key.as_str()) {
            continue;
        }
        said(
            writeln!(
                out,
                "`{key}` is baked here and named by no block declaration",
                key = entry.key.as_str()
            ),
            &origin,
        )?;
    }
    Ok(())
}

/// Every block declaration under `at`, as one body of text.
///
/// Read as **text** rather than loaded: an art build that started a script host
/// to find out which keys are used would let one broken block declaration refuse
/// a texture bake that has nothing to do with it. The cost is that a declaration
/// *computing* its key is not seen and is reported unused — acceptable precisely
/// because the report is advisory and a false positive costs one line.
///
/// A directory that is not there, and a file that will not open, both read as no
/// declaration at all rather than as a failure: a root that ships no blocks yet
/// still has a perfectly good art build, and there is nothing here for a refusal
/// to be about.
fn declarations(at: &Path) -> String {
    let Ok(listing) = std::fs::read_dir(at) else {
        return String::new();
    };
    listing
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == DECLARATION_EXTENSION)
        })
        .filter_map(|path| std::fs::read_to_string(path).ok())
        .collect()
}

/// `path` as the index records it.
///
/// `/`-separated whatever the platform writes, because the index is read on
/// platforms other than the one that wrote it.
fn as_recorded(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

/// The manifest's own name, as the index records it.
fn manifest_named(document: &Path) -> String {
    document.file_name().map_or_else(
        || as_recorded(document),
        |name| name.to_string_lossy().into_owned(),
    )
}
