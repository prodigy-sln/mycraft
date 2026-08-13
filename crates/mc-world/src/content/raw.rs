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
/// The key a declaration states whether it may be built over in.
const REPLACEABLE_FIELD: &str = "replaceable";
/// The key a declaration states whether it can be broken at all in.
const BREAKABLE_FIELD: &str = "breakable";
/// The key a declaration names what it breaks into by.
const BREAKS_INTO_FIELD: &str = "breaks_into";

/// What a declaration means by saying nothing about being built over.
///
/// The conservative half: a block that does not say so cannot be built through,
/// so a content author who forgets the key loses a placement rather than a
/// block.
const REPLACEABLE_BY_DEFAULT: bool = false;

/// What a declaration means by saying nothing about being breakable.
///
/// Breakable is the ordinary case, and a sandbox whose blocks were
/// indestructible until each said otherwise would be the wrong default to make
/// content carry.
const BREAKABLE_BY_DEFAULT: bool = true;

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
    /// Optional in a way the three above are not: absent is a meaningful state
    /// — a block nothing may be built over — rather than a declaration somebody
    /// forgot to write. The same is true of the two below, and all three still
    /// have to be spelled here, because `deny_unknown_fields` would otherwise
    /// refuse every file that declared one.
    replaceable: Option<Value>,
    /// Absent means breakable. The three fields are independent claims: a block
    /// may be indestructible and still name a residue, and a breakable block
    /// need not name one.
    breakable: Option<Value>,
    /// Absent means breaking this block leaves the cell empty.
    breaks_into: Option<Value>,
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
        let replaceable = optional_boolean(
            self.replaceable.as_ref(),
            REPLACEABLE_FIELD,
            REPLACEABLE_BY_DEFAULT,
        )?;
        let breakable = optional_boolean(
            self.breakable.as_ref(),
            BREAKABLE_FIELD,
            BREAKABLE_BY_DEFAULT,
        )?;
        let breaks_into = optional_text(self.breaks_into.as_ref(), BREAKS_INTO_FIELD)?;
        Ok(BlockDefinition {
            name: BlockName::parse(&name)
                .map_err(|error| FieldFault::invalid(NAME_FIELD, &error))?,
            texture: TextureKey::parse(&texture)
                .map_err(|error| FieldFault::invalid(TEXTURE_FIELD, &error))?,
            is_solid,
            replaceable,
            breakable,
            breaks_into: breaks_into
                .map(|declared| BlockName::parse(&declared))
                .transpose()
                .map_err(|error| FieldFault::invalid(BREAKS_INTO_FIELD, &error))?,
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

/// A field that need not be declared, but that has to be text where it is.
///
/// A field left out is `None` and a field written as something other than a
/// string is still a fault: "the author did not declare this" and "the author
/// declared it wrongly" are different mistakes, and collapsing the second into
/// the first is how a typed `breaks_into = 3` becomes an indestructible block.
fn optional_text(
    declared: Option<&Value>,
    field: &'static str,
) -> Result<Option<String>, FieldFault> {
    declared
        .map(|value| {
            value
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| FieldFault::wrong_kind(field, value, "a string"))
        })
        .transpose()
}

/// A boolean field that need not be declared, but that has to be a boolean where
/// it is.
///
/// The default carries a meaning of its own, which is why it is a parameter and
/// not a `false`: a block saying nothing about being breakable is breakable, and
/// one saying nothing about being built over is not replaceable. A value of the
/// wrong kind is still a fault — collapsing `breakable = "no"` into the default
/// would silently ship a breakable block to an author who declared the opposite.
fn optional_boolean(
    declared: Option<&Value>,
    field: &'static str,
    absent: bool,
) -> Result<bool, FieldFault> {
    declared.map_or(Ok(absent), |value| {
        value
            .as_bool()
            .ok_or_else(|| FieldFault::wrong_kind(field, value, "true or false"))
    })
}

fn required_boolean(declared: Option<&Value>, field: &'static str) -> Result<bool, FieldFault> {
    let value = declared.ok_or_else(|| FieldFault::missing(field))?;
    value
        .as_bool()
        .ok_or_else(|| FieldFault::wrong_kind(field, value, "true or false"))
}
