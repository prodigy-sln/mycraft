//! The permissive shape a document is read into before anything is decided.
//!
//! Every field is `Option<toml::Value>`, so serde answers exactly one question —
//! *is this key one we recognise?* — and never what a value has to be. A typed
//! field would let serde write the refusal, and serde's refusal names a Rust
//! type rather than the thing the author has to edit: `invalid type: integer` is
//! not what tells somebody their scale may not be zero.
//!
//! `deny_unknown_fields` is on every one of them, because a key nobody reads is
//! a typo that a permissive reader turns into silence.

use serde::Deserialize;
use toml::Value;

use crate::fault::{Fault, Origin};

/// A whole `.mcvox` document.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DocumentDto {
    /// The format revision the document claims.
    pub schema: Option<Value>,
    /// The namespaced name the document declares.
    pub name: Option<Value>,
    /// How many voxels span one block edge.
    pub scale: Option<Value>,
    /// The default axis layers are planes of.
    pub slice: Option<Value>,
    /// The implicit single-part form's extent.
    pub size: Option<Value>,
    /// The implicit single-part form's pivot.
    pub origin: Option<Value>,
    /// The character-to-material map.
    pub palette: Option<Value>,
    /// The explicit form's parts.
    pub parts: Option<Value>,
    /// Every layer the document declares, for every part.
    pub layers: Option<Value>,
}

/// One `[[parts]]` table.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PartDto {
    /// The name the part is declared under.
    pub name: Option<Value>,
    /// How far the part reaches on each axis.
    pub size: Option<Value>,
    /// The part's pivot in its own space.
    pub origin: Option<Value>,
    /// This part's override of the model's slice axis.
    pub slice: Option<Value>,
    /// Where the part hangs off its parent.
    pub attach: Option<Value>,
    /// The states the part declares.
    pub states: Option<Value>,
}

/// One `[[layers]]` table.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LayerDto {
    /// The part this layer belongs to, absent in the implicit single-part form.
    pub part: Option<Value>,
    /// The state this layer belongs to.
    pub state: Option<Value>,
    /// The layer's art.
    pub grid: Option<Value>,
    /// The plane, when the part is sliced on `x`.
    pub x: Option<Value>,
    /// The plane, when the part is sliced on `y`.
    pub y: Option<Value>,
    /// The plane, when the part is sliced on `z`.
    pub z: Option<Value>,
}

/// One material file.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MaterialDto {
    /// The namespaced key the material is declared under.
    pub name: Option<Value>,
    /// The material's colour, as `#rrggbb`.
    pub color: Option<Value>,
    /// How much light the material makes of its own.
    pub emissive: Option<Value>,
}

/// The field name serde names in an `unknown field \`x\`` diagnostic.
///
/// Reading it back out of the message is what lets one `deny_unknown_fields`
/// serve both the refusal and its attribution: the alternative is a hand-written
/// key list beside every DTO, which is the same knowledge in two places and one
/// of them silently wrong the day a field is added.
fn unknown_field(message: &str) -> Option<&str> {
    let (_, after) = message.split_once("unknown field `")?;
    let (field, _) = after.split_once('`')?;
    Some(field)
}

/// Reads `text` into `T`, turning serde's diagnostic into this tool's.
///
/// # Errors
///
/// Returns a [`Fault`] about `origin`, attributed to the offending field where
/// the failure was an unrecognised one.
pub fn from_text<T>(text: &str, origin: &Origin) -> Result<T, Fault>
where
    T: for<'de> Deserialize<'de>,
{
    toml::from_str(text).map_err(|cause| attributed(&cause.to_string(), origin))
}

/// Reads `value` into `T`, turning serde's diagnostic into this tool's.
///
/// # Errors
///
/// Returns a [`Fault`] about `origin`, attributed to the offending field where
/// the failure was an unrecognised one.
pub fn from_value<T>(value: Value, origin: &Origin) -> Result<T, Fault>
where
    T: for<'de> Deserialize<'de>,
{
    value
        .try_into()
        .map_err(|cause: toml::de::Error| attributed(&cause.to_string(), origin))
}

/// The refusal `message` describes, attributed to a field when it names one.
fn attributed(message: &str, origin: &Origin) -> Fault {
    let fault = Fault::about(origin.clone(), message);
    match unknown_field(message) {
        Some(field) => fault.in_field(field),
        None => fault,
    }
}
