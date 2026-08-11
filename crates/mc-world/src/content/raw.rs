//! A block declaration exactly as a file spells it, and the checking that turns
//! one into a definition.
//!
//! This type and [`BlockDefinition`] are deliberately separate. If the domain
//! type derived `serde` the accepted file format would silently follow every
//! change made to it, and the registry contract would inherit a serialization
//! dependency it has no use for.

use std::fmt;

use mc_core::block::source::DefinitionFault;
use mc_core::block::{BlockDefinition, DefinitionOrigin};
use mc_core::id::{BlockName, TextureKey};
use serde::Deserialize;
use toml::Value;

/// The key a declaration names itself by.
pub(super) const NAME_FIELD: &str = "name";
/// The key a declaration names its texture by.
const TEXTURE_FIELD: &str = "texture";
/// The key a declaration states its solidity in.
const SOLID_FIELD: &str = "solid";

/// A block declaration as read, before anything about it has been checked.
///
/// Every field is an untyped value rather than its eventual type, and that is
/// what makes the diagnostics possible: a declaration whose solidity reads
/// `"yes"` has to be refused *naming the block and the field*, and a parser that
/// failed on the type would have thrown the block's name away before anyone
/// could ask for it. Unknown fields are refused outright — a silently ignored
/// typo is a debugging trap for whoever wrote the file.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RawBlockDefinition {
    name: Option<Value>,
    texture: Option<Value>,
    solid: Option<Value>,
}

impl RawBlockDefinition {
    /// Checks the declaration and turns it into a definition attributed to
    /// `origin`.
    ///
    /// # Errors
    ///
    /// Returns a [`DefinitionFault`] naming `origin`, the block as it named
    /// itself, and the field at fault, if a required field is missing, holds the
    /// wrong kind of value, or is not a namespaced id.
    pub(super) fn into_definition(
        self,
        origin: &DefinitionOrigin,
    ) -> Result<BlockDefinition, DefinitionFault> {
        let declared = self.declared_name();
        self.check(origin).map_err(|fault| DefinitionFault {
            origin: origin.clone(),
            block: declared,
            field: Some(fault.field.to_owned()),
            cause: fault.cause,
        })
    }

    /// The name this declaration gives itself, as written.
    ///
    /// Read before anything is checked, so that a refusal can still say which
    /// block it is about — the alternative is telling a mod author only that
    /// something in the file is wrong.
    fn declared_name(&self) -> Option<String> {
        self.name
            .as_ref()
            .and_then(Value::as_str)
            .map(str::to_owned)
    }

    fn check(self, origin: &DefinitionOrigin) -> Result<BlockDefinition, FieldFault> {
        let name = required_text(self.name.as_ref(), NAME_FIELD)?;
        let texture = required_text(self.texture.as_ref(), TEXTURE_FIELD)?;
        let is_solid = required_boolean(self.solid.as_ref(), SOLID_FIELD)?;
        Ok(BlockDefinition {
            name: BlockName::parse(&name)
                .map_err(|error| FieldFault::invalid(NAME_FIELD, &error))?,
            texture: TextureKey::parse(&texture)
                .map_err(|error| FieldFault::invalid(TEXTURE_FIELD, &error))?,
            is_solid,
            origin: origin.clone(),
        })
    }
}

/// One field that is wrong, before it is known which block or file it belongs
/// to. Kept separate so that every check reads as a plain question about a value
/// and the attribution is written once.
#[derive(Debug)]
struct FieldFault {
    field: &'static str,
    cause: String,
}

impl FieldFault {
    /// A field that is present and of the right kind, but whose value is not
    /// acceptable.
    fn invalid(field: &'static str, cause: &impl fmt::Display) -> Self {
        Self {
            field,
            cause: cause.to_string(),
        }
    }

    /// A required field that was not declared at all.
    fn missing(field: &'static str) -> Self {
        Self {
            field,
            cause: format!("`{field}` is required and was not declared"),
        }
    }

    /// A field holding something other than the kind of value it is declared in.
    fn wrong_kind(field: &'static str, found: &Value, expected: &str) -> Self {
        Self {
            field,
            cause: format!("`{field}` must be {expected}, but is {}", found.type_str()),
        }
    }
}

fn required_text(declared: Option<&Value>, field: &'static str) -> Result<String, FieldFault> {
    let value = declared.ok_or_else(|| FieldFault::missing(field))?;
    value
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| FieldFault::wrong_kind(field, value, "a string"))
}

fn required_boolean(declared: Option<&Value>, field: &'static str) -> Result<bool, FieldFault> {
    let value = declared.ok_or_else(|| FieldFault::missing(field))?;
    value
        .as_bool()
        .ok_or_else(|| FieldFault::wrong_kind(field, value, "true or false"))
}
