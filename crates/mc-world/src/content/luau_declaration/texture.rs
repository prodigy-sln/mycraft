//! What a declaration's `texture` may say, and which key each of a block's six
//! faces ends up drawing from.
//!
//! Two forms and no third. **One string** means all six faces draw that key,
//! which is what every declaration in this repository states and what a mod
//! author writes for a block that looks the same all over. **A table naming a
//! key against each of `up`, `down`, `north`, `south`, `east` and `west`** means
//! a key per face, and it has to state exactly those six — a partial table would
//! leave faces drawing something nobody wrote, and there is no sensible key to
//! invent for one.
//!
//! # Two shapes of refusal, and why a single message cannot do both jobs
//!
//! A table that **left facings out** is told which ones, because that is the
//! edit its author has to make. A table carrying a **name that is not a facing**
//! is told the six a table may state, because its author has to see what they
//! nearly typed. Reciting all six over a table missing `west` is true and
//! useless; saying `top` was not stated over a table that says `top` is the
//! opposite of what is wrong.
//!
//! # A child of [`super`] rather than a module beside it
//!
//! Every refusal here is one of the parent's [`FieldFault`]s, which a child may
//! reach and a sibling could not without widening that type's visibility to the
//! whole of `content`. The refusals a mod author reads are one vocabulary, held
//! against `docs/modding/blocks-items.md` line for line, and a second fault type
//! here would be a second place for the page and the program to disagree.

use std::collections::BTreeSet;

use mc_core::content::{Face, FaceTextures};
use mc_core::id::TextureKey;
use mc_script::{FieldNames, ScriptHost, ScriptTable, ScriptValue};

use super::{
    FIELD_NAMES_READ, FieldFault, TEXTURE_FIELD, kind_of, listed, required_text,
    within_the_text_bound,
};

/// How many names the loader will read out of one texture table.
///
/// The same allowance a declaration's own fields get, and for the same reason:
/// the bound is on the copy out of the script state, not on how many names are
/// useful. A tighter one would refuse a table for its **size** where naming the
/// word its author misspelled is the refusal they can act on.
const FACING_NAMES_READ: std::num::NonZeroUsize = FIELD_NAMES_READ;

/// Which key each of a block's six faces draws from, in either form a
/// declaration may write it.
pub(super) fn declared_textures(
    host: &ScriptHost,
    declaration: &ScriptTable,
) -> Result<FaceTextures, FieldFault> {
    match host.read_field(declaration, TEXTURE_FIELD) {
        None => Err(FieldFault::missing(TEXTURE_FIELD)),
        Some(ScriptValue::Text(key)) => Ok(FaceTextures::uniform(parsed_key(key, TEXTURE_FIELD)?)),
        Some(ScriptValue::Table(stated)) => stated_facings(host, &stated),
        Some(found) => Err(FieldFault::neither_texture_form(&found)),
    }
}

/// The six keys a texture table states, or the fault naming what is wrong with
/// it.
///
/// **Every read of the table is raw**, exactly as the declaration around it is:
/// its metatable neither supplies a facing it did not state nor hides one it
/// did. A table that could decide what the loader is allowed to notice about it
/// could decide it was well formed.
///
/// A name the loader does not recognise beats a facing that was not stated, on
/// the reasoning `only_recognised_fields` is asked first: a misspelled word is
/// refused for the misspelling rather than for the facing the misspelling was
/// meant to be, so its author is not sent to add something they already wrote.
fn stated_facings(host: &ScriptHost, stated: &ScriptTable) -> Result<FaceTextures, FieldFault> {
    let FieldNames::Enumerated(names) = host.field_names(stated, FACING_NAMES_READ) else {
        return Err(FieldFault::more_facings_than_read(FACING_NAMES_READ.get()));
    };
    if let Some(unrecognised) = names.iter().find(|name| Face::named(name).is_none()) {
        return Err(FieldFault::not_a_facing(unrecognised));
    }
    let named: BTreeSet<&str> = names.iter().map(String::as_str).collect();
    let unstated: Vec<&str> = facing_words()
        .into_iter()
        .filter(|word| !named.contains(word))
        .collect();
    if !unstated.is_empty() {
        return Err(FieldFault::facings_not_stated(&unstated));
    }
    let mut keys = Vec::with_capacity(Face::ALL.len());
    for face in Face::ALL {
        keys.push(facing_key(host, stated, face)?);
    }
    match <[TextureKey; 6]>::try_from(keys) {
        Ok(read) => Ok(FaceTextures::stating(read)),
        // One key was read for each of the six faces just above, so this cannot
        // be reached. Written as an arm rather than an unwrap because a panic
        // here would end a load over a declaration the host is in the middle of
        // refusing, and reported as the table being short because that is the
        // only thing too few keys could mean.
        Err(_) => Err(FieldFault::facings_not_stated(&facing_words())),
    }
}

/// The key `face` draws from, as `stated` writes it.
fn facing_key(
    host: &ScriptHost,
    stated: &ScriptTable,
    face: Face,
) -> Result<TextureKey, FieldFault> {
    let word = face.as_str();
    parsed_key(required_text(host.read_field(stated, word), word)?, word)
}

/// `text` as a texture key, once it is short enough to retain.
///
/// The bound is the parent's and is checked before the id rule, so an author who
/// pasted a paragraph into a facing is told about the length rather than about a
/// separator somewhere inside it.
fn parsed_key(text: String, field: &str) -> Result<TextureKey, FieldFault> {
    TextureKey::parse(&within_the_text_bound(text, field)?)
        .map_err(|error| FieldFault::invalid(field, &error))
}

/// The six words a texture table may state, in the order a refusal lists them.
///
/// Read from [`Face`] rather than written out, so the list a refusal quotes and
/// the faces a block has are one fact and not two.
fn facing_words() -> [&'static str; 6] {
    Face::ALL.map(Face::as_str)
}

impl FieldFault {
    /// A texture table that names no key against every facing.
    ///
    /// Names **the facings it left out**, and then the six a table states.
    fn facings_not_stated(unstated: &[&str]) -> Self {
        Self {
            field: Some(TEXTURE_FIELD.to_owned()),
            cause: format!(
                "`{TEXTURE_FIELD}` states no key for {}; a texture table states all six of {}",
                listed(unstated),
                listed(&facing_words())
            ),
        }
    }

    /// A name in a texture table that is not one of the six facings.
    ///
    /// Quotes the word back **and** names the six, for the reason
    /// `FieldFault::unrecognised` does: a name is only recognisable as a near
    /// miss once its author can see what it was nearly.
    fn not_a_facing(name: &str) -> Self {
        Self {
            field: Some(TEXTURE_FIELD.to_owned()),
            cause: format!(
                "`{name}` is not a facing a texture table may state; a texture table may state {}",
                listed(&facing_words())
            ),
        }
    }

    /// A texture table holding more names than the loader will read.
    ///
    /// Names no count of its own for the reason the declaration-wide bound does
    /// not: the enumeration stops one name past the allowance rather than
    /// counting a table it is refusing to allocate.
    fn more_facings_than_read(allowed: usize) -> Self {
        Self {
            field: Some(TEXTURE_FIELD.to_owned()),
            cause: format!(
                "a texture table may hold at most {allowed} names, and this one holds more"
            ),
        }
    }

    /// A `texture` that is neither of the two forms one may take.
    ///
    /// States both forms. An author told only that it must be a string learns
    /// that the feature they came here for does not exist.
    fn neither_texture_form(found: &ScriptValue) -> Self {
        Self {
            field: Some(TEXTURE_FIELD.to_owned()),
            cause: format!(
                "`{TEXTURE_FIELD}` must be a string or a table of six facings, but is {}",
                kind_of(found)
            ),
        }
    }
}
