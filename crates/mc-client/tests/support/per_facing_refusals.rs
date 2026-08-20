//! The seven refusals a per-facing texture table raises, each as a person
//! running the client from their own game directory reads it.
//!
//! **Seven roots and not one**, for the reason [`crate::printed_refusals`] builds
//! eight: a root is refused whole, so a root carrying two mistakes is refused for
//! whichever the loader reaches first and the second refusal would be one no run
//! ever prints.
//!
//! Every one of them is produced by the client's own preparation over a real
//! content root and rendered through the shipped reporting, then normalised and
//! rewritten so that it names the root a person actually runs against. Nothing
//! here writes out what a refusal is expected to say: the wording is the
//! implementer's, and the whole point of the guard these feed is that the page and
//! the program are compared against each other rather than each against somebody's
//! belief about the other.
//!
//! # Why this is a module of its own
//!
//! [`crate::printed_refusals`] is within fifty non-blank lines of the size the
//! gate allows a test file, and seven more producers do not fit. The split is also
//! a responsibility boundary: everything here is one requirement's refusals, and
//! the guard that reads them asks a question about that requirement alone —
//! whether the modding guide states every one of them.
//!
//! # The fixture root's own directory never reaches a page
//!
//! A refusal names the declaration by the path the run was given, and a run over a
//! temporary copy names that copy. The rewrite is [`crate::printed_refusals`]'s
//! and is checked rather than hoped for: a refusal that does not name the fixture
//! root fails rather than being compared against a page it could never match.

// Each test binary linking this module drives a subset of it.
#![allow(dead_code)]

use std::error::Error;

use crate::printed_refusals::{BLOCK_FILE, as_read_from_a_game_directory};
use crate::support::content;

/// The block every declaration here names itself.
const AMBER: &str = "example:amber";

/// The six facing words, in the order a declaration writes them.
const SIX_FACINGS: [&str; 6] = ["up", "down", "north", "south", "east", "west"];

/// One key per facing, in that same order.
///
/// Pairwise distinct and none of them the block's own name, so that a refusal
/// quoting a key back cannot be quoting the block instead.
const SIX_KEYS: [&str; 6] = [
    "example:quartz",
    "example:ash",
    "example:basalt",
    "example:chert",
    "example:diorite",
    "example:gabbro",
];

/// The three facings the short table states.
const THREE_STATED: usize = 3;

/// A name that is not a facing, and the same mistake made by capitalising one
/// that is.
const NOT_A_FACING: &str = "top";
const A_CAPITALISED_FACING: &str = "Up";

/// The facing whose value each of the two value faults is about, and what it
/// holds.
const WRONGLY_TYPED_FACING: &str = "up";
const A_NUMBER: &str = "7";
const BADLY_KEYED_FACING: &str = "north";
const TWO_SEPARATORS: &str = "base:grass:top";

/// Every refusal a texture table raises, in the order the modding guide is
/// expected to introduce them: the table that is the wrong shape first, then the
/// values inside it, then `texture` being neither form.
///
/// # Errors
///
/// Returns an error if a fixture root cannot be built, or if a root that must
/// refuse is accepted.
pub fn per_facing_refusals() -> Result<Vec<String>, Box<dyn Error>> {
    let declarations = [
        table_stating(&pairs_for(&SIX_FACINGS[..THREE_STATED])),
        table_stating(&[]),
        table_stating(&with_word_misspelled(WRONGLY_TYPED_FACING, NOT_A_FACING)),
        table_stating(&with_word_misspelled(
            WRONGLY_TYPED_FACING,
            A_CAPITALISED_FACING,
        )),
        table_with_a_raw_value(WRONGLY_TYPED_FACING, A_NUMBER),
        table_stating(&with_key_replaced(BADLY_KEYED_FACING, TWO_SEPARATORS)),
        "texture = true".to_owned(),
    ];
    let mut refusals = Vec::with_capacity(declarations.len());
    for texture in declarations {
        let root = content::shipped_copy()?.declaring_block(BLOCK_FILE, &declaring(&texture))?;
        refusals.push(as_read_from_a_game_directory(&root)?);
    }
    Ok(refusals)
}

/// A declaration of [`AMBER`] whose `texture` is `texture`, correct in every
/// other field.
///
/// Correct elsewhere on purpose: a declaration with a second thing wrong with it
/// is refused for whichever the loader reaches first, and the refusal a page
/// quotes would then be about something else.
fn declaring(texture: &str) -> String {
    format!("return {{\n\tname = '{AMBER}',\n\t{texture},\n\tsolid = true,\n}}\n")
}

/// A `texture` field stating a table of `facings`.
fn table_stating(facings: &[(&str, &str)]) -> String {
    let stated: String = facings
        .iter()
        .map(|(word, key)| format!("\t\t{word} = '{key}',\n"))
        .collect();
    format!("texture = {{\n{stated}\t}}")
}

/// A `texture` table stating the well-formed six, except that `facing` holds
/// `value` written verbatim rather than as text.
fn table_with_a_raw_value(facing: &str, value: &str) -> String {
    let stated: String = the_six()
        .into_iter()
        .map(|(word, key)| {
            if word == facing {
                format!("\t\t{word} = {value},\n")
            } else {
                format!("\t\t{word} = '{key}',\n")
            }
        })
        .collect();
    format!("texture = {{\n{stated}\t}}")
}

/// The six facings, each holding its own key.
fn the_six() -> Vec<(&'static str, &'static str)> {
    SIX_FACINGS.into_iter().zip(SIX_KEYS).collect()
}

/// The first `facings` of the six, each holding its own key.
fn pairs_for(facings: &[&'static str]) -> Vec<(&'static str, &'static str)> {
    the_six()
        .into_iter()
        .filter(|(word, _)| facings.contains(word))
        .collect()
}

/// The six with the word `replaced` spelled `misspelled` instead, its key kept.
fn with_word_misspelled(
    replaced: &str,
    misspelled: &'static str,
) -> Vec<(&'static str, &'static str)> {
    the_six()
        .into_iter()
        .map(|(word, key)| {
            if word == replaced {
                (misspelled, key)
            } else {
                (word, key)
            }
        })
        .collect()
}

/// The six with `facing` holding `key` instead of its own.
fn with_key_replaced(facing: &str, key: &'static str) -> Vec<(&'static str, &'static str)> {
    the_six()
        .into_iter()
        .map(|(word, own)| {
            if word == facing {
                (word, key)
            } else {
                (word, own)
            }
        })
        .collect()
}
