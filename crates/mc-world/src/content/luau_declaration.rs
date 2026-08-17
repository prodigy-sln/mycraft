//! What a declaration must say, and what it means by leaving something out.
//!
//! The contract half of the Luau block loader: given a table a chunk returned,
//! this decides whether it is a block definition and produces the fault naming
//! what is wrong when it is not. It opens no file and knows nothing about
//! directories — its sibling [`super::luau_source`] does that, and the two are
//! separate because they change for different reasons. A new field or a changed
//! default is a change here; a change to which files are declarations is a
//! change there.
//!
//! It is the Luau counterpart of [`super::raw`], which does the same job for the
//! HUD's TOML declarations. Where that one leans on `serde` to reject a field
//! nobody recognises, this one has to ask the table what fields it holds —
//! `deny_unknown_fields` was TOML's, and a host that can read a named field but
//! cannot ask what fields exist can never tell a typo from an absence.
//!
//! # Nothing here runs the mod's code
//!
//! Every read goes through [`ScriptHost::read_field`] and every enumeration
//! through [`ScriptHost::field_names`], both raw. A declaration's own metatable
//! therefore never runs on the host's schedule, never observes which fields were
//! looked at, cannot supply a field the declaration did not state, and cannot
//! hide one it did.

use std::fmt;
use std::num::NonZeroUsize;

use mc_core::block::source::DefinitionFault;
use mc_core::block::{BlockDefinition, DefinitionOrigin};
use mc_core::id::{BlockName, TextureKey};
use mc_script::{FieldNames, ScriptHost, ScriptTable, ScriptValue};

/// The key a declaration names itself by.
const NAME_FIELD: &str = "name";
/// The key a declaration names its texture by.
const TEXTURE_FIELD: &str = "texture";
/// The key a declaration states its solidity in.
const SOLID_FIELD: &str = "solid";
/// The key a declaration states being buildable-over in.
const REPLACEABLE_FIELD: &str = "replaceable";
/// The key a declaration states being breakable in.
const BREAKABLE_FIELD: &str = "breakable";
/// The key a declaration names its residue in.
const BREAKS_INTO_FIELD: &str = "breaks_into";

/// Every field name a declaration may state, in the order the documentation
/// introduces them.
///
/// The order is fixed because a refusal quotes this list back, and
/// `documented_refusals.rs` compares a quoted refusal against a real run **line
/// for line** — a list ordered by anything that can vary makes that guard
/// intermittently red and the page it guards unwritable.
const RECOGNISED_FIELDS: [&str; 6] = [
    NAME_FIELD,
    TEXTURE_FIELD,
    SOLID_FIELD,
    REPLACEABLE_FIELD,
    BREAKABLE_FIELD,
    BREAKS_INTO_FIELD,
];

/// How many field names the loader will read out of one declaration.
///
/// The enumeration this bounds copies every key name out of the script state, so
/// a table of a hundred thousand one-character keys would otherwise be allocated
/// in full before the refusal that names one of them. That is why the bound is a
/// parameter of the enumeration rather than a check applied to its result.
const FIELD_NAMES_READ: NonZeroUsize = match NonZeroUsize::new(64) {
    Some(bound) => bound,
    // Unreachable for a non-zero literal, and written rather than unwrapped
    // because this crate denies panicking conversions at its root.
    None => NonZeroUsize::MIN,
};

/// How many characters one declared value may hold.
///
/// It bounds what a `BlockDefinition` **retains** — three strings apiece across
/// a whole content root — rather than the copy out of the script state, which is
/// already made by the time `read_field` hands back text and is separately
/// bounded by the host's memory backstop.
///
/// **Characters, not bytes.** The documentation says characters, and counting
/// bytes would refuse a non-ASCII id at a different length than the page states.
const CHARACTERS_A_DECLARED_VALUE_MAY_HOLD: usize = 256;

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

/// Checks a declaration table and turns it into a definition attributed to
/// `origin`.
pub(crate) fn checked_declaration(
    host: &ScriptHost,
    declaration: &ScriptTable,
    origin: &DefinitionOrigin,
) -> Result<BlockDefinition, DefinitionFault> {
    let block = declared_name(host, declaration);
    check(host, declaration, origin).map_err(|fault| DefinitionFault {
        origin: origin.clone(),
        block,
        field: fault.field,
        cause: fault.cause,
    })
}

/// The name this declaration gives itself, as written.
///
/// Read before anything is checked, so that a refusal can still say which block
/// it is about. `None` where the declaration named itself nothing, or named
/// itself something that is not text — in both cases there is genuinely nothing
/// to quote back, and inventing something would report a block nobody declared.
fn declared_name(host: &ScriptHost, declaration: &ScriptTable) -> Option<String> {
    match host.read_field(declaration, NAME_FIELD) {
        Some(ScriptValue::Text(name)) => Some(name),
        _ => None,
    }
}

fn check(
    host: &ScriptHost,
    declaration: &ScriptTable,
    origin: &DefinitionOrigin,
) -> Result<BlockDefinition, FieldFault> {
    only_recognised_fields(host, declaration)?;
    let name = declared_text(host.read_field(declaration, NAME_FIELD), NAME_FIELD)?;
    let texture = declared_text(host.read_field(declaration, TEXTURE_FIELD), TEXTURE_FIELD)?;
    let is_solid = required_boolean(host.read_field(declaration, SOLID_FIELD), SOLID_FIELD)?;
    let replaceable = optional_boolean(
        host.read_field(declaration, REPLACEABLE_FIELD),
        REPLACEABLE_FIELD,
        REPLACEABLE_BY_DEFAULT,
    )?;
    let breakable = optional_boolean(
        host.read_field(declaration, BREAKABLE_FIELD),
        BREAKABLE_FIELD,
        BREAKABLE_BY_DEFAULT,
    )?;
    let breaks_into = optional_residue(host.read_field(declaration, BREAKS_INTO_FIELD))?;
    Ok(BlockDefinition {
        name: BlockName::parse(&name).map_err(|error| FieldFault::invalid(NAME_FIELD, &error))?,
        texture: TextureKey::parse(&texture)
            .map_err(|error| FieldFault::invalid(TEXTURE_FIELD, &error))?,
        is_solid,
        replaceable,
        breakable,
        breaks_into,
        origin: origin.clone(),
    })
}

/// Nothing, once every field `declaration` states is one the loader has a
/// meaning for.
///
/// **Asked before any field is read**, so that a declaration whose fields are
/// misspelled is refused for the misspelling rather than for the required field
/// the misspelling was meant to be.
///
/// The enumeration is raw: a declaration's own `__iter` neither hides a field
/// from this nor invents one for it. Believing that metamethod would hand a
/// declaration the power to decide what the loader is allowed to notice about
/// it, which is the silent loss this whole check exists to prevent.
fn only_recognised_fields(host: &ScriptHost, declaration: &ScriptTable) -> Result<(), FieldFault> {
    let FieldNames::Enumerated(stated) = host.field_names(declaration, FIELD_NAMES_READ) else {
        return Err(FieldFault::more_fields_than_read(FIELD_NAMES_READ.get()));
    };
    let offenders: Vec<String> = stated
        .into_iter()
        .filter(|field| !RECOGNISED_FIELDS.contains(&field.as_str()))
        .collect();
    match offenders.first() {
        None => Ok(()),
        Some(first) => Err(FieldFault::unrecognised(first, &offenders)),
    }
}

/// A field a declaration may leave out, which has to be a boolean whenever it is
/// stated.
///
/// `absent` is what leaving it out means. A value of the wrong kind is refused
/// rather than falling back to it: falling back is the worst available outcome,
/// because the block then behaves exactly as it would have if the author had
/// written nothing at all, so there is no symptom to notice and nothing to
/// search for.
fn optional_boolean(
    declared: Option<ScriptValue>,
    field: &str,
    absent: bool,
) -> Result<bool, FieldFault> {
    match declared {
        None => Ok(absent),
        Some(ScriptValue::Boolean(flag)) => Ok(flag),
        Some(found) => Err(FieldFault::wrong_kind(field, &found, "true or false")),
    }
}

/// What a declaration leaves behind when it is broken, where it names anything.
///
/// The id is checked by the same rule every other id in the engine obeys, and it
/// is **not resolved**: a residue is resolved where a break reads it, not where
/// it is declared, so a block naming something no content root declares
/// registers. Looking it up here would make the order two declarations are read
/// in decide whether either of them loads.
fn optional_residue(declared: Option<ScriptValue>) -> Result<Option<BlockName>, FieldFault> {
    match declared {
        None => Ok(None),
        Some(ScriptValue::Text(text)) => {
            BlockName::parse(&within_the_text_bound(text, BREAKS_INTO_FIELD)?)
                .map(Some)
                .map_err(|error| FieldFault::invalid(BREAKS_INTO_FIELD, &error))
        }
        Some(found) => Err(FieldFault::wrong_kind(
            BREAKS_INTO_FIELD,
            &found,
            "a string",
        )),
    }
}

/// One thing that is wrong with a declaration, before it is known which block or
/// file it belongs to. Kept separate so that every check reads as a plain
/// question about a value and the attribution is written once.
///
/// `field` is optional because not every fault is about one: a declaration
/// holding more fields than the loader will read is wrong as a whole, and there
/// is no single key to send its author to.
#[derive(Debug)]
struct FieldFault {
    field: Option<String>,
    cause: String,
}

impl FieldFault {
    /// A field that is present and of the right kind, but whose value is not
    /// acceptable.
    fn invalid(field: &str, cause: &impl fmt::Display) -> Self {
        Self {
            field: Some(field.to_owned()),
            cause: cause.to_string(),
        }
    }

    /// A required field that was not declared at all.
    fn missing(field: &str) -> Self {
        Self {
            field: Some(field.to_owned()),
            cause: format!("`{field}` is required and was not declared"),
        }
    }

    /// A field holding something other than the kind of value it is declared
    /// in.
    fn wrong_kind(field: &str, found: &ScriptValue, expected: &str) -> Self {
        Self {
            field: Some(field.to_owned()),
            cause: format!("`{field}` must be {expected}, but is {}", kind_of(found)),
        }
    }

    /// Field names the loader has no meaning for.
    ///
    /// Blames `first` — the first in the enumeration's own sorted order — and
    /// names every offender **and every field a declaration may state**. The
    /// recognised list is owed to whoever reads this: a name is only
    /// recognisable as a typo once you can see what it was nearly, and
    /// `replacable` beside `replaceable` explains itself where `replacable`
    /// alone does not.
    fn unrecognised(first: &str, offenders: &[String]) -> Self {
        let stated = if offenders.len() == 1 {
            "is not a field a declaration may state"
        } else {
            "are not fields a declaration may state"
        };
        Self {
            field: Some(first.to_owned()),
            cause: format!(
                "{} {stated}; a declaration may state {}",
                listed(offenders),
                listed(&RECOGNISED_FIELDS)
            ),
        }
    }

    /// A declaration holding more field names than the loader will read.
    ///
    /// Names no field, because none of them is the one at fault — and states the
    /// bound without stating how many the declaration held, which is the one
    /// quantity the design deliberately never learns: the enumeration stops one
    /// key past the allowance rather than counting a table it is refusing to
    /// allocate.
    fn more_fields_than_read(allowed: usize) -> Self {
        Self {
            field: None,
            cause: format!(
                "a declaration may hold at most {allowed} fields, and this one holds more"
            ),
        }
    }

    /// A declared value longer than one may be.
    fn too_long(field: &str, characters: usize) -> Self {
        Self {
            field: Some(field.to_owned()),
            cause: format!(
                "`{field}` holds {characters} characters, and a declared value may \
                 hold at most {CHARACTERS_A_DECLARED_VALUE_MAY_HOLD}"
            ),
        }
    }
}

/// `values` as a comma-separated list, each quoted the way a declaration writes
/// it.
fn listed<S: AsRef<str>>(values: &[S]) -> String {
    values
        .iter()
        .map(|value| format!("`{}`", value.as_ref()))
        .collect::<Vec<_>>()
        .join(", ")
}

/// How a declared value reads when a refusal has to name what it found.
///
/// **In Luau's own `type` vocabulary**, because whoever reads this refusal is
/// looking at a Luau file: a mod author told their `solid` holds `a string`
/// knows where to look, and one told it holds the host's internal name for the
/// same thing does not. Rendering the value itself is refused for a different
/// reason — that would honour `__tostring`, which is the mod's own code running
/// on the host's schedule at the moment the host is reporting the mod's
/// mistake.
fn kind_of(value: &ScriptValue) -> &'static str {
    match value {
        ScriptValue::Nil => "nil",
        ScriptValue::Boolean(_) => "a boolean",
        ScriptValue::Integer(_) | ScriptValue::Number(_) => "a number",
        ScriptValue::Text(_) => "a string",
        ScriptValue::Table(_) => "a table",
        ScriptValue::Function(_) => "a function",
        ScriptValue::Opaque => "a value of a kind a declaration cannot state",
    }
}

/// A field that has to be declared, has to be text, and has to be short enough
/// for the engine to keep.
fn declared_text(declared: Option<ScriptValue>, field: &str) -> Result<String, FieldFault> {
    within_the_text_bound(required_text(declared, field)?, field)
}

/// `text` itself, once it is short enough to retain.
///
/// Counted in characters rather than bytes; see
/// [`CHARACTERS_A_DECLARED_VALUE_MAY_HOLD`]. Both quantities reach the refusal,
/// so an author is told the length they wrote and the length they may write.
fn within_the_text_bound(text: String, field: &str) -> Result<String, FieldFault> {
    let characters = text.chars().count();
    if characters <= CHARACTERS_A_DECLARED_VALUE_MAY_HOLD {
        return Ok(text);
    }
    Err(FieldFault::too_long(field, characters))
}

/// A field that has to be declared, and has to be text.
///
/// A field a declaration left out and one holding nothing are one state in
/// script and are one answer here, which is what the host's `None` already
/// means.
fn required_text(declared: Option<ScriptValue>, field: &str) -> Result<String, FieldFault> {
    match declared.ok_or_else(|| FieldFault::missing(field))? {
        ScriptValue::Text(text) => Ok(text),
        found => Err(FieldFault::wrong_kind(field, &found, "a string")),
    }
}

/// A field that has to be declared, and has to be a boolean.
///
/// A value of the wrong kind is refused rather than read for its truthiness:
/// `solid = 'yes'` is a mistake a mod author makes once, and reading it as true
/// is how they never find out they made it.
fn required_boolean(declared: Option<ScriptValue>, field: &str) -> Result<bool, FieldFault> {
    match declared.ok_or_else(|| FieldFault::missing(field))? {
        ScriptValue::Boolean(flag) => Ok(flag),
        found => Err(FieldFault::wrong_kind(field, &found, "true or false")),
    }
}
