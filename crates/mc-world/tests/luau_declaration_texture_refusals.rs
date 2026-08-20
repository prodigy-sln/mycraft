//! A texture table that is not exactly the six facings, and what its refusal
//! owes the person who wrote it.
//!
//! There are two shapes of refusal here and they are the same fact from opposite
//! sides. A table that **left facings out** names the ones it left out, because
//! that is the edit its author has to make. A table carrying a **name that is not
//! a facing** names the six a table may state, because its author has to see what
//! they nearly typed. A single message doing both jobs does neither of them: a
//! reader told only "a texture table states all six of …" over a table missing
//! `west` has to work out which one is missing by reading their own file back,
//! and a reader told only "`top` was not stated" over a table that says `top` is
//! being told the opposite of what is wrong.
//!
//! # `Up` is not `up`, and that is why two scenarios look like one
//!
//! `top` and `Up` are the same test twice unless the matching is exact. An
//! implementation that lower-cased the word before matching passes the `top` case
//! and accepts the `Up` one, drawing a block whose `up` facing was never declared.
//! So both are here, both state the other five facings correctly, and the pair is
//! the only thing separating exact matching from case-folded matching.
//!
//! # Which of two faults a table with both wins
//!
//! Those two fixtures state a name that is not a facing **and**, by consequence,
//! leave `up` unstated. The refusal is about the unrecognised name, on the same
//! reasoning `only_recognised_fields` is asked before any field is read: a
//! declaration whose word is misspelled is refused for the misspelling rather than
//! for the field the misspelling was meant to be. Reporting `up` missing sends its
//! author to add a facing they already wrote.
//!
//! # What is asserted, and what is deliberately not
//!
//! **Which words a refusal names, and in what order.** Not the sentence around
//! them: the final wording is the implementer's, held line for line against
//! `docs/modding/blocks-items.md` by `crates/mc-client/tests/documented_refusals.rs`,
//! and a second copy of it here would be a second place to disagree.
//!
//! **The blamed field is asserted only where the requirement names one.** A table
//! that is the wrong shape is wrong as `texture`, and that is asserted. A facing
//! holding the wrong kind of value, or an id that breaks the namespacing rule, is
//! wrong somewhere inside `texture` — the requirement says what the refusal must
//! *report*, not what it must blame, so these two compare a record that carries no
//! field at all rather than pinning a choice the spec left open.

mod common;
mod luau_common;

use std::error::Error;
use std::path::{Path, PathBuf};

use common::{TestResult, content_root};
use luau_common::{
    AMBER, AMBER_FILE, Blamed, SIX_FACINGS, blaming, declaration_of, facing_table, judged,
    raw_field, text_field,
};
use mc_core::id::TextureKey;
use tempfile::TempDir;

/// The field a texture is stated in.
const TEXTURE_FIELD: &str = "texture";

/// The three facings the short table states, and the three it does not.
const THREE_STATED: [&str; 3] = ["up", "down", "north"];
const THREE_UNSTATED: [&str; 3] = ["south", "east", "west"];

/// A name that is not a facing, and the same mistake made by capitalising one
/// that is.
const NOT_A_FACING: &str = "top";
const A_CAPITALISED_FACING: &str = "Up";

/// The facing whose value is the wrong kind, and the value it holds.
const WRONGLY_TYPED_FACING: &str = "up";
const A_NUMBER: &str = "7";

/// The facing whose value is not a namespaced id, and the id it states.
const BADLY_KEYED_FACING: &str = "north";
const TWO_SEPARATORS: &str = "base:grass:top";

/// One key per facing, in [`SIX_FACINGS`] order.
///
/// Distinct from each other and from [`AMBER`] for the reason
/// `luau_declaration_textures.rs` states: a fixture repeating a key cannot see a
/// resolver that answers one facing for all six, and one naming the block cannot
/// see a loader that read the name into a facing. Every fixture below is these
/// six with exactly one thing done to them, so that whatever the refusal is about
/// is the one thing that was done.
const SIX_KEYS: [&str; 6] = [
    "example:quartz",
    "example:ash",
    "example:basalt",
    "example:chert",
    "example:diorite",
    "example:gabbro",
];

/// What a refusal about a texture said, without the field it blamed.
///
/// Two of the scenarios below name what a refusal must **report** and say nothing
/// about what it must blame; comparing the field there would pin a decision the
/// requirement left to whoever writes it, and an over-tight assertion invites a
/// real defect — the cheapest way to green it is to change working code.
#[derive(Debug, PartialEq, Eq)]
enum About {
    /// The root was accepted, so nothing was refused for anything.
    NothingRefused,
    /// A declaration the loader would not accept, naming this file and this block.
    Declaration {
        names_the_file: bool,
        block: Option<String>,
    },
    /// Refused, but not as a malformed declaration.
    SomethingElse(String),
}

/// What `blamed` says, with the field dropped.
fn about(blamed: Blamed) -> About {
    match blamed {
        Blamed::NothingRefused => About::NothingRefused,
        Blamed::Declaration(attribution) => About::Declaration {
            names_the_file: attribution.names_the_file,
            block: attribution.block,
        },
        Blamed::SomethingElse(rendered) => About::SomethingElse(rendered),
    }
}

/// A refusal that named this file and [`AMBER`], whatever field it blamed.
fn about_the_declaration() -> About {
    About::Declaration {
        names_the_file: true,
        block: Some(AMBER.to_owned()),
    }
}

/// A root holding one declaration whose `texture` is `field`.
fn root_texturing(directory: &TempDir, field: String) -> Result<PathBuf, Box<dyn Error>> {
    let fields = vec![text_field("name", AMBER), field, raw_field("solid", "true")];
    content_root(directory, &[(AMBER_FILE, declaration_of(&fields))])
}

/// A root whose `texture` table states `facings`.
fn root_stating(directory: &TempDir, facings: &[(&str, &str)]) -> Result<PathBuf, Box<dyn Error>> {
    root_texturing(directory, facing_table(facings))
}

/// The six facings, each holding its own key: the well-formed table every
/// fixture below is one edit away from.
fn the_six_well_formed() -> Vec<(&'static str, &'static str)> {
    SIX_FACINGS.into_iter().zip(SIX_KEYS).collect()
}

/// The well-formed six with the word `replaced` spelled `misspelled` instead.
///
/// The key stays where it was, so the table holds six entries and six keys and
/// differs from a well-formed one in one word.
fn the_six_with_a_word_misspelled(
    replaced: &str,
    misspelled: &'static str,
) -> Vec<(&'static str, &'static str)> {
    the_six_well_formed()
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

/// The well-formed six with `facing` holding `key` instead of its own.
fn the_six_with_a_key_replaced(
    facing: &str,
    key: &'static str,
) -> Vec<(&'static str, &'static str)> {
    the_six_well_formed()
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

/// A `texture` table stating the well-formed six, except that `facing` holds
/// `value` written verbatim rather than as text.
///
/// Verbatim because the fixture's subject is a facing holding something that is
/// not a string at all, which a text field cannot express.
fn the_six_with_a_raw_value(facing: &str, value: &str) -> String {
    let stated: String = the_six_well_formed()
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

/// Which facing words `cause` names, in the order it names them.
///
/// Matched **quoted**, the way every other refusal in this loader writes a field
/// name, so that `Up` in a refusal about `Up` is not read as a mention of `up`.
fn facings_named_in_order(cause: &str) -> Vec<&'static str> {
    let mut found: Vec<(usize, &'static str)> = SIX_FACINGS
        .into_iter()
        .filter_map(|facing| cause.find(&format!("`{facing}`")).map(|at| (at, facing)))
        .collect();
    found.sort_by_key(|&(at, _)| at);
    found.into_iter().map(|(_, facing)| facing).collect()
}

/// The first `most` facing words `cause` names.
fn the_first_facings_named(cause: &str, most: usize) -> Vec<&'static str> {
    facings_named_in_order(cause)
        .into_iter()
        .take(most)
        .collect()
}

/// What reading `root` blamed, and what it said.
fn refusal_of(root: &Path) -> (About, String) {
    let (blamed, cause) = judged(root, AMBER_FILE);
    (about(blamed), cause)
}

/// What the namespaced-id rule says about a key holding two separators.
///
/// Taken from the rule itself rather than written out, because the requirement is
/// that the existing sentence is **reused**: a second wording here would be a
/// second place for the two to disagree, and it is precisely the disagreement
/// this comparison exists to catch.
///
/// # Errors
///
/// Returns an error if that key parses, in which case the fixture is not the one
/// the scenario names.
fn the_namespacing_sentence() -> Result<String, Box<dyn Error>> {
    Ok(TextureKey::parse(TWO_SEPARATORS)
        .err()
        .ok_or("this fixture has to state a texture key the namespacing rule refuses")?
        .to_string())
}

#[test]
fn a_table_stating_three_facings_is_refused_naming_the_three_it_did_not_state() -> TestResult {
    let directory = TempDir::new()?;
    let stated: Vec<(&str, &str)> = THREE_STATED.into_iter().zip(SIX_KEYS).collect();
    let root = root_stating(&directory, &stated)?;

    let (about, cause) = refusal_of(&root);

    assert_eq!(
        (about, the_first_facings_named(&cause, THREE_UNSTATED.len())),
        (about_the_declaration(), THREE_UNSTATED.to_vec()),
        "the author left three facings out and the three they left out are the edit they have to \
         make. A refusal that recited all six instead would be true and useless — they would have \
         to read their own file back to work out which three it meant: {cause}"
    );
    Ok(())
}

#[test]
fn an_empty_table_is_refused_naming_all_six_facings_as_unstated() -> TestResult {
    let directory = TempDir::new()?;
    let root = root_stating(&directory, &[])?;

    let (about, cause) = refusal_of(&root);

    assert_eq!(
        (about, facings_named_in_order(&cause)),
        (about_the_declaration(), SIX_FACINGS.to_vec()),
        "an empty table is the one case where naming what was not stated and naming what may be \
         stated are the same six words, which is why the three-facing scenario above exists to \
         separate them. What this one is for is that `texture = {{}}` is refused at all: a loader \
         accepting an empty table registers a block with no key on any face: {cause}"
    );
    Ok(())
}

#[test]
fn a_table_carrying_top_is_refused_naming_the_six_facings_a_table_may_state() -> TestResult {
    let directory = TempDir::new()?;
    let stated = the_six_with_a_word_misspelled(WRONGLY_TYPED_FACING, NOT_A_FACING);
    let root = root_stating(&directory, &stated)?;

    let (about, cause) = refusal_of(&root);

    assert_eq!(
        (
            about,
            facings_named_in_order(&cause),
            cause.contains(NOT_A_FACING)
        ),
        (about_the_declaration(), SIX_FACINGS.to_vec(), true),
        "`top` is the word somebody reaches for before they have read the page, and the refusal \
         has to show them the six words that exist as well as quoting back the one that does not \
         — a name is only recognisable as a near miss once you can see what it was nearly: {cause}"
    );
    Ok(())
}

#[test]
fn a_table_carrying_a_capitalised_facing_is_refused_naming_the_six_a_table_may_state() -> TestResult
{
    let directory = TempDir::new()?;
    let stated = the_six_with_a_word_misspelled(WRONGLY_TYPED_FACING, A_CAPITALISED_FACING);
    let root = root_stating(&directory, &stated)?;

    let (about, cause) = refusal_of(&root);

    assert_eq!(
        (
            about,
            facings_named_in_order(&cause),
            cause.contains(A_CAPITALISED_FACING)
        ),
        (about_the_declaration(), SIX_FACINGS.to_vec(), true),
        "the facing words are matched exactly and are not case-folded. An implementation that \
         lower-cased the word passes the `top` scenario and accepts this one — registering a block \
         whose `up` facing nobody declared and whose top face draws whatever `up` fell back to. \
         The pair is the only thing in this suite that separates the two: {cause}"
    );
    Ok(())
}

#[test]
fn a_facing_holding_a_number_is_refused_reporting_that_it_must_be_a_string() -> TestResult {
    let directory = TempDir::new()?;
    let root = root_texturing(
        &directory,
        the_six_with_a_raw_value(WRONGLY_TYPED_FACING, A_NUMBER),
    )?;

    let (about, cause) = refusal_of(&root);

    assert_eq!(
        (
            about,
            cause.contains(&format!("`{WRONGLY_TYPED_FACING}`")),
            cause.contains("string")
        ),
        (about_the_declaration(), true, true),
        "a facing holding a number is refused rather than rendered into one, and the author is \
         told which facing and what it has to hold. Rendering the value would honour `__tostring` \
         — the mod's own code running on the host's schedule at the moment the host is reporting \
         the mod's mistake: {cause}"
    );
    Ok(())
}

#[test]
fn a_facing_holding_two_namespace_separators_is_refused_reporting_that() -> TestResult {
    let directory = TempDir::new()?;
    let stated = the_six_with_a_key_replaced(BADLY_KEYED_FACING, TWO_SEPARATORS);
    let root = root_stating(&directory, &stated)?;

    let (about, cause) = refusal_of(&root);

    assert_eq!(
        (about, cause.contains(&the_namespacing_sentence()?)),
        (about_the_declaration(), true),
        "`base:grass:top` is the mistake somebody makes reaching for a hierarchy the id rule does \
         not have, and the sentence that explains it already exists and is quoted on the modding \
         page. Writing a second one for facings would be a second place for the two to disagree, \
         and an author would be told two different things about one rule: {cause}"
    );
    Ok(())
}

#[test]
fn a_texture_stated_as_a_boolean_is_refused_reporting_the_two_forms_it_may_take() -> TestResult {
    let directory = TempDir::new()?;
    let root = root_texturing(&directory, raw_field(TEXTURE_FIELD, "true"))?;

    let (blamed, cause) = judged(&root, AMBER_FILE);

    assert_eq!(
        (
            blamed,
            cause.contains("string"),
            cause.contains("table"),
            cause.contains("six")
        ),
        (
            Blamed::Declaration(blaming(AMBER, TEXTURE_FIELD)),
            true,
            true,
            true
        ),
        "`texture` being neither a string nor a table is a different mistake from anything inside \
         a table, and the refusal has to state both forms. An author told only that it must be a \
         string learns that the feature they came here for does not exist: {cause}"
    );
    Ok(())
}
