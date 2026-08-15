//! What each character in a grid means.
//!
//! The namespaced id rule is reused rather than restated: `MaterialKey` is a
//! newtype over `mc_core::id::NamespacedId`, so the diagnostic an author reads
//! here is the same one the block loader gives them, and there is no second
//! opinion about what a namespaced id is.

use std::collections::BTreeMap;

use toml::Value;

use crate::fault::{Fault, Origin};
use crate::format::PaletteEntry;
use crate::name::MaterialKey;

/// The value a palette maps a character to when the character means "no voxel".
const EMPTY_MARKER: &str = "empty";

/// The field a refusal about the palette is attributed to.
const FIELD: &str = "palette";

/// The palette `declared` describes.
///
/// # Errors
///
/// Returns a [`Fault`] naming the palette when it is absent, empty, keyed by
/// anything but one ASCII character, or valued by anything but the empty marker
/// or a namespaced material key.
pub fn resolve(
    declared: Option<&Value>,
    origin: &Origin,
) -> Result<BTreeMap<u8, PaletteEntry>, Fault> {
    let table = declared
        .and_then(Value::as_table)
        .ok_or_else(|| refusal(origin, "a document declares a `palette` mapping each grid character to a material or to the empty marker, and this one declares no palette table"))?;
    if table.is_empty() {
        return Err(refusal(
            origin,
            "the palette declares no entry at all, so no grid character could be spelled with it",
        ));
    }
    let mut palette = BTreeMap::new();
    for (key, value) in table {
        palette.insert(character(key, origin)?, entry(key, value, origin)?);
    }
    Ok(palette)
}

/// The single ASCII byte `key` spells.
fn character(key: &str, origin: &Origin) -> Result<u8, Fault> {
    match key.as_bytes() {
        [only] if key.is_ascii() => Ok(*only),
        _ => Err(refusal(
            origin,
            format!(
                "the palette key `{key}` is not one character — a grid spells one character per cell, so a key of any other length could never appear in one"
            ),
        )),
    }
}

/// What `key` maps to.
fn entry(key: &str, value: &Value, origin: &Origin) -> Result<PaletteEntry, Fault> {
    let text = value.as_str().ok_or_else(|| {
        refusal(
            origin,
            format!(
                "the palette key `{key}` maps to something that is not text — an entry is either `\"{EMPTY_MARKER}\"` or a namespaced material key"
            ),
        )
    })?;
    if text == EMPTY_MARKER {
        return Ok(PaletteEntry::Empty);
    }
    MaterialKey::parse(text)
        .map(PaletteEntry::Material)
        .map_err(|cause| refusal(origin, cause.to_string()))
}

/// A refusal about the palette.
fn refusal(origin: &Origin, cause: impl Into<String>) -> Fault {
    Fault::about(origin.clone(), cause).in_field(FIELD)
}
