//! What `tint` and `tint_distance` mean, and the rule that neither of them may
//! be stated without the other.
//!
//! The second field on a declaration whose acceptance depends on **another
//! field**, so it is arranged the way [`super::opacity`] is: the rule deciding
//! it lives here rather than in the middle of the check that reads every field
//! in turn, and its refusals are the parent's [`FieldFault`]s so the vocabulary
//! a mod author reads stays one vocabulary.
//!
//! # The colour reader is new work and reuses neither shipped parser
//!
//! `mc-core`'s HUD takes eight digits and says in its refusal that eight is the
//! only form a colour takes; `voxforge` takes six and says the same of six. Both
//! claims are false about this tree, and a field a mod author may reach from
//! either direction cannot inherit either claim — an author copying a shipped
//! material file and an author copying a shipped HUD file both have to get a
//! block that works. Reconciling those two is filed separately; neither is
//! touched from here.
//!
//! # A colour states no alpha, and the eight-digit form is still accepted
//!
//! How strongly a medium acts is how far it lets an eye see, which is the other
//! field. So `#RRGGBBAA` is a *form* this reader accepts and `AA` is a *value*
//! it refuses unless it is `FF` — a distinction worth a refusal of its own,
//! because somebody who wrote a half alpha reaching for a weaker tint has
//! written a well-formed colour and needs to be told where strength lives.
//!
//! # Case is a property of the parse
//!
//! Digits go through [`u8::is_ascii_hexdigit`] and `u8::from_str_radix`, and the
//! alpha is compared as the parsed **byte** — so `ff`, `Ff` and `FF` are one
//! value rather than three spellings something has to remember to fold. The
//! alphabet is checked separately because `from_str_radix` admits a sign, and
//! `+F` is not a channel anybody wrote.

use mc_core::block::MediumTint;
use mc_script::{ScriptHost, ScriptTable, ScriptValue};

use super::number::{self, Bounds};
use super::{FieldFault, TINT_DISTANCE_FIELD, TINT_FIELD};

/// The lead every accepted colour is written behind.
const COLOUR_LEAD: char = '#';

/// How many hexadecimal digits each of the two accepted forms carries.
const DIGITS_WITHOUT_ALPHA: usize = 6;
const DIGITS_WITH_ALPHA: usize = 8;

/// How many digits spell one channel.
const DIGITS_PER_CHANNEL: usize = 2;

/// The base those digits are read in.
const HEXADECIMAL: u32 = 16;

/// The one alpha an eight-digit colour may state.
const AN_ALPHA_THAT_TAKES_NOTHING_AWAY: u8 = 0xFF;

/// The medium this declaration says its block is, or nothing where it says it is
/// no medium at all.
///
/// **Both fields are read before either absence is judged**, so a declaration
/// that states one of them badly is refused for the value rather than for the
/// half it is missing: an author who wrote `#GG0000` has a colour to fix, not a
/// distance to add.
pub(super) fn declared(
    host: &ScriptHost,
    declaration: &ScriptTable,
) -> Result<Option<MediumTint>, FieldFault> {
    let colour = declared_colour(host.read_field(declaration, TINT_FIELD))?;
    let distance = number::stated_number_within(
        host.read_field(declaration, TINT_DISTANCE_FIELD),
        TINT_DISTANCE_FIELD,
        Bounds::above_zero(),
    )?;
    match (colour, distance) {
        (None, None) => Ok(None),
        // Unreachable as a `None`: `Bounds::above_zero` refuses exactly what
        // `MediumTint::new` refuses, so a colour and a distance that reached
        // here are a medium the engine keeps. Written as a fallback rather than
        // unwrapped for the reason `super::FIELD_NAMES_READ` is.
        (Some(colour), Some(distance)) => Ok(MediumTint::new(colour, distance)),
        (Some(_), None) => Err(FieldFault::invalid(
            TINT_DISTANCE_FIELD,
            &a_distance_is_required_beside_a_colour(),
        )),
        (None, Some(_)) => Err(FieldFault::invalid(
            TINT_FIELD,
            &a_colour_is_required_beside_a_distance(),
        )),
    }
}

/// The three channel bytes this declaration states, or nothing where it states
/// no colour.
fn declared_colour(declared: Option<ScriptValue>) -> Result<Option<[u8; 3]>, FieldFault> {
    let stated = match declared {
        None => return Ok(None),
        Some(ScriptValue::Text(colour)) => colour,
        Some(found) => {
            return Err(FieldFault::wrong_kind(
                TINT_FIELD,
                &found,
                "a colour string",
            ));
        }
    };
    let Some(digits) = stated.strip_prefix(COLOUR_LEAD) else {
        return Err(FieldFault::invalid(TINT_FIELD, &not_one_of_the_two_forms()));
    };
    channels_of(digits)
}

/// The channels `digits` spells, refused where they are not one of the two
/// accepted forms or state an alpha that takes something away.
fn channels_of(digits: &str) -> Result<Option<[u8; 3]>, FieldFault> {
    let spelled = digits.len();
    // The alphabet is checked here and not left to `byte_of` below, which reads
    // through `u8::from_str_radix` — and that admits a **sign**. Without this
    // sweep `#+F0000` parses to three perfectly good channels, so deleting it as
    // belt-and-braces admits a colour nobody wrote.
    if !digits.bytes().all(|digit| digit.is_ascii_hexdigit())
        || (spelled != DIGITS_WITHOUT_ALPHA && spelled != DIGITS_WITH_ALPHA)
    {
        return Err(FieldFault::invalid(TINT_FIELD, &not_one_of_the_two_forms()));
    }
    let Some((channels, alpha)) = channels_and_alpha(digits) else {
        return Err(FieldFault::invalid(TINT_FIELD, &not_one_of_the_two_forms()));
    };
    if alpha.is_some_and(|stated| stated != AN_ALPHA_THAT_TAKES_NOTHING_AWAY) {
        return Err(FieldFault::invalid(TINT_FIELD, &a_colour_states_no_alpha()));
    }
    Ok(Some(channels))
}

/// The three channels `digits` spells and the alpha behind them where it spells
/// a fourth, or nothing where it spells fewer than three.
fn channels_and_alpha(digits: &str) -> Option<([u8; 3], Option<u8>)> {
    let mut pairs = digits.as_bytes().chunks_exact(DIGITS_PER_CHANNEL);
    let mut channels = [0; 3];
    for channel in &mut channels {
        *channel = byte_of(pairs.next()?)?;
    }
    match pairs.next() {
        None => Some((channels, None)),
        Some(alpha) => Some((channels, Some(byte_of(alpha)?))),
    }
}

/// The byte two hexadecimal digits spell.
fn byte_of(pair: &[u8]) -> Option<u8> {
    str::from_utf8(pair)
        .ok()
        .and_then(|pair| u8::from_str_radix(pair, HEXADECIMAL).ok())
}

/// The sentence a colour that is neither accepted form is refused in.
///
/// **Both forms are named.** Each is already written somewhere in this tree
/// behind a reader claiming to be the only one, so a refusal quoting a single
/// form would tell half of everyone copying a shipped file that the file they
/// copied is malformed.
fn not_one_of_the_two_forms() -> String {
    format!("`{TINT_FIELD}` must be written `#RRGGBB` or `#RRGGBBAA`, in upper case or lower")
}

/// The sentence an eight-digit colour whose alpha takes something away is
/// refused in.
///
/// It names where strength lives rather than describing the form, because the
/// form is correct: an author told their colour is malformed edits it into six
/// digits, loses the strength they were reaching for, and never learns which
/// field carries it.
fn a_colour_states_no_alpha() -> String {
    format!(
        "`{TINT_FIELD}` states no alpha: how strongly a medium acts is `{TINT_DISTANCE_FIELD}`, \
         so an eight-digit colour must end `FF`"
    )
}

/// The sentence a colour stated with no distance is refused in.
///
/// **It blames the missing field**, which is the line the author has to add —
/// and it names the one they already have, so nobody greps for a line their
/// file does not contain.
fn a_distance_is_required_beside_a_colour() -> String {
    format!(
        "`{TINT_DISTANCE_FIELD}` is required beside `{TINT_FIELD}`: a colour with no distance \
         does not say how far this medium lets an eye see"
    )
}

/// The sentence a distance stated with no colour is refused in.
///
/// A different sentence from the one above rather than a shared vague one,
/// because the remedy is different: this one names a colour to add where that
/// one names a distance.
fn a_colour_is_required_beside_a_distance() -> String {
    format!(
        "`{TINT_FIELD}` is required beside `{TINT_DISTANCE_FIELD}`: a distance with no colour \
         does not say what this medium carries a view toward"
    )
}
