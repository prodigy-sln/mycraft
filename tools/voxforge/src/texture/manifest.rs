//! What a texture manifest says, and every refusal it can earn before a model
//! is opened.
//!
//! A manifest is the file that says which face of which model becomes which
//! texture key. It is content: committed, reviewable in a diff, and written by
//! hand — so every refusal here is in the author's own terms, naming the value
//! they typed and what a legal one looks like.
//!
//! Every DTO field is `Option<toml::Value>` under `deny_unknown_fields`, which
//! is the shape the document reader already uses: serde answers exactly one
//! question — *is this key one we recognise?* — and never what a value has to
//! be, so `pixels_per_voxel` being zero is refused in this tool's words rather
//! than as `invalid type`.

use std::collections::BTreeSet;
use std::num::NonZeroU32;
use std::path::{Path, PathBuf};

use mc_core::art::is_an_ordinary_image_name;
use mc_core::id::TextureKey;
use serde::Deserialize;
use toml::Value;

use crate::fault::{Fault, Origin};
use crate::format::dto::{from_text, from_value};
use crate::render::pixels_per_voxel;
use crate::texture::AxisAlignedView;

/// A whole manifest.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ManifestDto {
    /// Where the built set is written, relative to the manifest.
    output: Option<Value>,
    /// Where material keys resolve from, relative to the manifest.
    materials: Option<Value>,
    /// Where the block declarations sit, relative to the manifest.
    blocks: Option<Value>,
    /// How many pixels one voxel spans.
    pixels_per_voxel: Option<Value>,
    /// Every entry the manifest states.
    texture: Option<Value>,
}

/// One `[[texture]]` table.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct EntryDto {
    /// The texture key this entry bakes to.
    key: Option<Value>,
    /// The model the face is taken from.
    model: Option<Value>,
    /// Which of that model's six faces.
    face: Option<Value>,
}

/// What one entry asks to be baked.
#[derive(Debug, Clone)]
pub struct ManifestEntry {
    /// The key a block declaration names this art by.
    pub key: TextureKey,
    /// The model, as the manifest spells it, relative to the manifest.
    pub model: PathBuf,
    /// Which of the model's six faces becomes the key's art.
    pub face: AxisAlignedView,
    /// The file the key's art is written under, derived from the key here and
    /// nowhere else.
    pub image: String,
}

/// Everything a manifest states.
///
/// The three directories stay exactly as written: they are relative to the
/// manifest's own directory, and that is what lets a copied content root fold
/// to the same value somewhere else on disk.
#[derive(Debug, Clone)]
pub struct Manifest {
    /// Where the built set is written.
    pub output: PathBuf,
    /// Where material keys resolve from.
    pub materials: PathBuf,
    /// Where the block declarations sit.
    pub blocks: PathBuf,
    /// How many pixels one voxel spans.
    pub pixels_per_voxel: NonZeroU32,
    /// The entries, in the order the manifest states them.
    pub entries: Vec<ManifestEntry>,
}

/// The manifest at `path`.
///
/// # Errors
///
/// Returns a [`Fault`] naming the path when there is no file there or it is not
/// TOML, and one naming the offending value when an entry states a key that is
/// not a namespaced id, a key no index could record, a key whose art would not
/// be written under an ordinary file name, a face that is not one of the six, or
/// a key some other entry has already claimed.
pub fn load_manifest(path: &Path) -> Result<Manifest, Fault> {
    let origin = Origin::new(path);
    let text = std::fs::read_to_string(path).map_err(|cause| {
        Fault::about(
            origin.clone(),
            format!("no texture manifest could be read there: {cause}"),
        )
    })?;
    let declared: ManifestDto = from_text(&text, &origin)?;
    let entries = read_entries(declared.texture.as_ref(), &origin)?;
    refuse_a_key_stated_twice(&entries, &origin)?;
    Ok(Manifest {
        output: read_directory(declared.output.as_ref(), "output", &origin)?,
        materials: read_directory(declared.materials.as_ref(), "materials", &origin)?,
        blocks: read_directory(declared.blocks.as_ref(), "blocks", &origin)?,
        pixels_per_voxel: read_pixels_per_voxel(declared.pixels_per_voxel.as_ref(), &origin)?,
        entries,
    })
}

/// One directory the manifest names, relative to itself.
fn read_directory(
    declared: Option<&Value>,
    field: &str,
    origin: &Origin,
) -> Result<PathBuf, Fault> {
    let spelled = declared.and_then(Value::as_str).ok_or_else(|| {
        refusal(
            origin,
            field,
            format!("a manifest states `{field}`, a directory relative to the manifest itself"),
        )
    })?;
    Ok(PathBuf::from(spelled))
}

/// How many pixels one voxel spans.
fn read_pixels_per_voxel(declared: Option<&Value>, origin: &Origin) -> Result<NonZeroU32, Fault> {
    let stated = declared.and_then(Value::as_integer).ok_or_else(|| {
        refusal(
            origin,
            "pixels_per_voxel",
            "a manifest states `pixels_per_voxel`, how many pixels one voxel spans",
        )
    })?;
    let held = u32::try_from(stated).ok().ok_or_else(|| {
        refusal(
            origin,
            "pixels_per_voxel",
            format!("`pixels_per_voxel` must be at least 1, but is {stated}"),
        )
    })?;
    pixels_per_voxel(held, origin.clone())
}

/// Every entry the manifest states, in the order it states them.
fn read_entries(declared: Option<&Value>, origin: &Origin) -> Result<Vec<ManifestEntry>, Fault> {
    let Some(stated) = declared else {
        return Ok(Vec::new());
    };
    let tables = stated.as_array().ok_or_else(|| {
        refusal(
            origin,
            "texture",
            "a manifest states its entries as `[[texture]]` tables",
        )
    })?;
    tables
        .iter()
        .map(|table| read_entry(table.clone(), origin))
        .collect()
}

/// The one entry `table` states.
fn read_entry(table: Value, origin: &Origin) -> Result<ManifestEntry, Fault> {
    let declared: EntryDto = from_value(table, origin)?;
    let spelled = declared
        .key
        .as_ref()
        .and_then(Value::as_str)
        .ok_or_else(|| {
            refusal(
                origin,
                "key",
                "a texture entry states a namespaced `key`, written `namespace:path`",
            )
        })?;
    let key =
        TextureKey::parse(spelled).map_err(|cause| refusal(origin, "key", cause.to_string()))?;
    Ok(ManifestEntry {
        image: image_of(&key, origin)?,
        key,
        model: read_model(declared.model.as_ref(), origin)?,
        face: read_face(declared.face.as_ref(), origin)?,
    })
}

/// The file one entry's art is written under.
///
/// A key has no character set imposed on it, so this is where a content string
/// would otherwise become a filesystem path. Both refusals are raised here,
/// while the author can still see the manifest they typed, rather than by the
/// index renderer at the end of a bake — which would quote back a derived name
/// for a key the author never wrote down.
///
/// **The two are ordered and the order is the message's.** A key carrying a
/// control character cannot be recorded at all, and saying so first keeps an
/// author from being told about a file name when what is wrong is a line break.
fn image_of(key: &TextureKey, origin: &Origin) -> Result<String, Fault> {
    if key.as_str().chars().any(|held| held.is_ascii_control()) {
        return Err(refusal(
            origin,
            "key",
            format!(
                "`{key}` carries a control character, so it cannot be written to an index, which states one record to a line",
                key = key.as_str()
            ),
        ));
    }
    let named = image_named(key);
    if !is_an_ordinary_image_name(&named) {
        return Err(refusal(
            origin,
            "key",
            format!(
                "`{key}` would have its art written as `{named}`, which is not a single ordinary file name — a key's image is its text with each `:` replaced by `__` and `.png` appended, and the result may hold only letters, digits, `.`, `-` and `_`",
                key = key.as_str()
            ),
        ));
    }
    Ok(named)
}

/// A key's text as a file name: a colon is not a file-name character everywhere
/// this runs, so it becomes two underscores.
fn image_named(key: &TextureKey) -> String {
    format!("{name}.png", name = key.as_str().replace(':', "__"))
}

/// The model one entry names.
fn read_model(declared: Option<&Value>, origin: &Origin) -> Result<PathBuf, Fault> {
    let spelled = declared.and_then(Value::as_str).ok_or_else(|| {
        refusal(
            origin,
            "model",
            "a texture entry states a `model`, a path relative to the manifest itself",
        )
    })?;
    Ok(PathBuf::from(spelled))
}

/// The face one entry selects.
///
/// Read through [`AxisAlignedView`]'s own vocabulary and never through the whole
/// view list: an isometric view is not a face, and a refusal offering one would
/// send an author to write a manifest entry this tool cannot serve.
fn read_face(declared: Option<&Value>, origin: &Origin) -> Result<AxisAlignedView, Fault> {
    let offered = AxisAlignedView::ALL
        .iter()
        .map(|face| face.as_str())
        .collect::<Vec<&str>>()
        .join(", ");
    let spelled = declared.and_then(Value::as_str).ok_or_else(|| {
        refusal(
            origin,
            "face",
            format!("a texture entry states a `face` — a block has six, {offered}"),
        )
    })?;
    AxisAlignedView::named(spelled).ok_or_else(|| {
        refusal(
            origin,
            "face",
            format!(
                "`{spelled}` is not a face a texture entry may select — a block has six, {offered}"
            ),
        )
    })
}

/// Refuses a manifest two of whose entries claim one key.
///
/// One key names one image, so letting the later entry win would bake whichever
/// face came second and say nothing at all about the one that was overwritten.
fn refuse_a_key_stated_twice(entries: &[ManifestEntry], origin: &Origin) -> Result<(), Fault> {
    let mut claimed: BTreeSet<&str> = BTreeSet::new();
    for entry in entries {
        if !claimed.insert(entry.key.as_str()) {
            return Err(refusal(
                origin,
                "key",
                format!(
                    "`{key}` is stated by two entries, and one key names one image",
                    key = entry.key.as_str()
                ),
            ));
        }
    }
    Ok(())
}

/// A refusal about `origin`, attributed to `field`.
fn refusal(origin: &Origin, field: &str, cause: impl Into<String>) -> Fault {
    Fault::about(origin.clone(), cause).in_field(field)
}
