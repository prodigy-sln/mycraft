//! How long a value one declaration may state, and how many fields it may
//! carry.
//!
//! # These two bound different things, and only one of them is about the script
//!
//! The **declared-text** bound is about what a definition *retains*. A block
//! keeps three strings — its name, its texture key and its residue — and a
//! content root keeps four thousand blocks, so an unbounded string is an
//! unbounded allocation the engine holds for the life of the process. It does
//! not protect the copy out of the script state: by the time the loader sees a
//! value the copy is already made, and that transient one is bounded by the
//! host's own memory ceiling instead.
//!
//! The **field-count** bound is the opposite case and has to bind one level
//! down, inside the enumeration itself. A table of a hundred thousand
//! one-character keys is copied out in full before any refusal naming one of
//! them could be written, so a bound applied to the list afterwards arrives
//! after the allocation it exists to refuse.
//!
//! # Characters, not bytes, and the accepting test is what says so
//!
//! The bound is stated in characters, and a name of 256 characters written in
//! anything but ASCII is longer than 256 bytes. So the accepting fixture below
//! is deliberately accented: a loader measuring bytes refuses it, while both
//! readings agree about the 257-character refusals next door. Without that the
//! two measures are indistinguishable across this whole file, and a mod author
//! writing their own language finds a bound the documentation does not describe.

mod common;
mod luau_common;

use std::error::Error;
use std::path::PathBuf;

use common::{TestResult, content_root};
use luau_common::{
    AMBER, AMBER_FILE, Blamed, QUARTZ, blaming, blaming_the_declaration, declaration_of, declaring,
    judged, mentioning, raw_field, registration_order_or_refusal, text_field,
};
use tempfile::TempDir;

/// How many characters one declared value may hold.
const CHARACTERS_A_DECLARED_VALUE_MAY_HOLD: usize = 256;

/// One more than that, which is the shortest value the bound refuses.
const ONE_CHARACTER_TOO_MANY: usize = CHARACTERS_A_DECLARED_VALUE_MAY_HOLD + 1;

/// How many field names the loader will read out of one declaration.
const FIELDS_A_DECLARATION_MAY_HOLD: usize = 64;

/// One more than that, which is the smallest declaration the bound refuses.
const ONE_FIELD_TOO_MANY: usize = FIELDS_A_DECLARATION_MAY_HOLD + 1;

/// The key a declaration names itself by.
const NAME_FIELD: &str = "name";

/// The key a declaration names its texture by.
const TEXTURE_FIELD: &str = "texture";

/// The key a declaration names its residue by.
const BREAKS_INTO_FIELD: &str = "breaks_into";

/// The namespace every generated id here carries.
///
/// A namespaced id is the only kind the engine accepts, so a fixture about
/// length has to stay a legal id or it would be refused for the wrong rule.
const NAMESPACE: &str = "example:";

/// A namespaced id of exactly `characters` characters, in ASCII.
///
/// Used for both refusals, where the two possible measures agree: 257 ASCII
/// characters are 257 bytes, so a loader counting either one refuses it and the
/// test is about the bound rather than about the unit.
fn an_id_of(characters: usize) -> Result<String, Box<dyn Error>> {
    let path = characters
        .checked_sub(NAMESPACE.chars().count())
        .ok_or("an id shorter than its own namespace cannot be built")?;
    Ok(format!("{NAMESPACE}{}", "a".repeat(path)))
}

/// A namespaced id of exactly `characters` characters, half of them outside
/// ASCII.
///
/// The accepting fixture, and the accent is the point: this is a legal id of
/// exactly the length the bound allows and comfortably more bytes than it, so a
/// loader measuring bytes refuses a declaration the documentation promises to
/// accept.
fn an_accented_id_of(characters: usize) -> Result<String, Box<dyn Error>> {
    let remaining = characters
        .checked_sub(NAMESPACE.chars().count())
        .ok_or("an id shorter than its own namespace cannot be built")?;
    let path: String = "aé".chars().cycle().take(remaining).collect();
    Ok(format!("{NAMESPACE}{path}"))
}

/// A root whose one declaration states exactly `fields`.
fn root_declaring(directory: &TempDir, fields: &[String]) -> Result<PathBuf, Box<dyn Error>> {
    content_root(directory, &[(AMBER_FILE, declaration_of(fields))])
}

/// The three required fields, correctly stated, followed by `extra`.
fn the_required_three_and(extra: &[String]) -> Vec<String> {
    let mut fields = vec![
        text_field(NAME_FIELD, AMBER),
        text_field(TEXTURE_FIELD, QUARTZ),
        raw_field("solid", "true"),
    ];
    fields.extend_from_slice(extra);
    fields
}

/// A declaration stating its three required fields and enough keys nobody
/// recognises to carry `total` in all.
///
/// The filler keys are unrecognised on purpose, and that is what makes this
/// fixture discriminate: a loader that checks which fields it knows before it
/// checks how many there are refuses one of them by name, which is a different
/// refusal than the one this bound owes.
fn a_declaration_of(total: usize) -> Result<Vec<String>, Box<dyn Error>> {
    let filler = total
        .checked_sub(3)
        .ok_or("a declaration carries its three required fields before any filler")?;
    let extra: Vec<String> = (0..filler)
        .map(|position| raw_field(&format!("filler_{position:04}"), "true"))
        .collect();
    Ok(the_required_three_and(&extra))
}

/// The two quantities a refusal about declared text owes its reader.
fn the_length_and_its_bound() -> [String; 2] {
    [
        ONE_CHARACTER_TOO_MANY.to_string(),
        CHARACTERS_A_DECLARED_VALUE_MAY_HOLD.to_string(),
    ]
}

/// Those two, sorted the way [`mentioning`] answers.
fn both_stated(quantities: &[String; 2]) -> Vec<String> {
    let mut stated = quantities.to_vec();
    stated.sort();
    stated
}

/// What a refusal about a value that is too long managed to say.
#[derive(Debug, PartialEq, Eq)]
struct TooLong {
    characters_declared: usize,
    blamed: Blamed,
    quantities_named: Vec<String>,
}

/// What a refusal about a declaration carrying too many fields managed to say.
#[derive(Debug, PartialEq, Eq)]
struct TooMany {
    fields_declared: usize,
    blamed: Blamed,
    the_bound_named: Vec<String>,
}

/// What became of a name of exactly the length allowed.
#[derive(Debug, PartialEq, Eq)]
struct AtTheLimit {
    characters: usize,
    is_longer_in_bytes: bool,
    registered: Result<Vec<String>, String>,
}

#[test]
fn a_name_of_one_character_too_many_is_refused_naming_that_field_and_the_length_it_allows()
-> TestResult {
    let directory = TempDir::new()?;
    let name = an_id_of(ONE_CHARACTER_TOO_MANY)?;
    let root = content_root(&directory, &[(AMBER_FILE, declaring(&name))])?;
    let quantities = the_length_and_its_bound();
    let (blamed, cause) = judged(&root, AMBER_FILE);

    assert_eq!(
        TooLong {
            characters_declared: name.chars().count(),
            blamed,
            quantities_named: mentioning(&cause, &[&quantities[0], &quantities[1]]),
        },
        TooLong {
            characters_declared: ONE_CHARACTER_TOO_MANY,
            blamed: Blamed::Declaration(blaming(&name, NAME_FIELD)),
            quantities_named: both_stated(&quantities),
        },
        "a name is one of the three strings a definition keeps for the life of the process, and \
         four thousand definitions keep three apiece — so a name nobody bounded is memory the \
         engine holds because a content file asked it to. This id is a perfectly legal namespaced \
         id and breaks no other rule, so the only thing that can refuse it is its length, and the \
         refusal owes the author both numbers: the one they wrote and the one they may write"
    );
    Ok(())
}

#[test]
fn a_texture_key_of_one_character_too_many_is_refused_naming_that_field_and_the_length_it_allows()
-> TestResult {
    let directory = TempDir::new()?;
    let texture = an_id_of(ONE_CHARACTER_TOO_MANY)?;
    let root = root_declaring(
        &directory,
        &[
            text_field(NAME_FIELD, AMBER),
            text_field(TEXTURE_FIELD, &texture),
            raw_field("solid", "true"),
        ],
    )?;
    let quantities = the_length_and_its_bound();
    let (blamed, cause) = judged(&root, AMBER_FILE);

    assert_eq!(
        TooLong {
            characters_declared: texture.chars().count(),
            blamed,
            quantities_named: mentioning(&cause, &[&quantities[0], &quantities[1]]),
        },
        TooLong {
            characters_declared: ONE_CHARACTER_TOO_MANY,
            blamed: Blamed::Declaration(blaming(AMBER, TEXTURE_FIELD)),
            quantities_named: both_stated(&quantities),
        },
        "the same bound on the second of the three retained strings, and the block is nameable \
         here where the test next door has only an over-long name to quote back — so a loader \
         that bounded whichever string it happened to read first is refused by one of the pair \
         whichever one it chose. The refusal names `texture`, because sending an author to the \
         field they must shorten is the entire difference between a bound and a wall"
    );
    Ok(())
}

#[test]
fn a_declaration_carrying_more_fields_than_are_read_is_refused_naming_the_bound_and_no_one_field()
-> TestResult {
    let directory = TempDir::new()?;
    let fields = a_declaration_of(ONE_FIELD_TOO_MANY)?;
    let root = root_declaring(&directory, &fields)?;
    let bound = FIELDS_A_DECLARATION_MAY_HOLD.to_string();
    let (blamed, cause) = judged(&root, AMBER_FILE);

    assert_eq!(
        TooMany {
            fields_declared: fields.len(),
            blamed,
            the_bound_named: mentioning(&cause, &[&bound]),
        },
        TooMany {
            fields_declared: ONE_FIELD_TOO_MANY,
            blamed: Blamed::Declaration(blaming_the_declaration(AMBER)),
            the_bound_named: vec![bound.clone()],
        },
        "this bound is the only one that has to bind *before* the thing it bounds is built: the \
         enumeration copies every key name out of the script state, so a table of a hundred \
         thousand keys is allocated in full before any refusal naming one of them could be \
         written. What that costs the refusal is the observed quantity — the loader stops \
         counting one past the bound and genuinely does not know how many there were — so it \
         states the bound alone, and blames the declaration as a whole rather than a single key, \
         because a declaration carrying sixty-five fields has no one field to send its author to. \
         The filler keys are unrecognised on purpose: a loader that checks *which* fields it \
         knows before it checks how many there are refuses one of them by name instead"
    );
    Ok(())
}

#[test]
fn a_name_of_exactly_the_characters_allowed_registers_the_block_under_that_name() -> TestResult {
    let directory = TempDir::new()?;
    let name = an_accented_id_of(CHARACTERS_A_DECLARED_VALUE_MAY_HOLD)?;
    let root = content_root(&directory, &[(AMBER_FILE, declaring(&name))])?;

    assert_eq!(
        AtTheLimit {
            characters: name.chars().count(),
            is_longer_in_bytes: name.len() > CHARACTERS_A_DECLARED_VALUE_MAY_HOLD,
            registered: registration_order_or_refusal(&root),
        },
        AtTheLimit {
            characters: CHARACTERS_A_DECLARED_VALUE_MAY_HOLD,
            is_longer_in_bytes: true,
            registered: Ok(vec![name.clone()]),
        },
        "the accepting side of the bound, and the one place in this file where the unit is \
         visible. A bound stated only from the refusing side leaves `>` and `>=` \
         indistinguishable; a bound stated in characters and measured in bytes leaves a mod \
         author writing their own language refused at a length nothing documents. This name is \
         exactly 256 characters and comfortably more than 256 bytes, so both failures redden it \
         and neither can hide behind an ASCII fixture"
    );
    Ok(())
}

#[test]
fn a_residue_of_one_character_too_many_is_refused_on_the_same_bound_as_a_name() -> TestResult {
    let directory = TempDir::new()?;
    let residue = an_id_of(ONE_CHARACTER_TOO_MANY)?;
    let root = root_declaring(
        &directory,
        &the_required_three_and(&[text_field(BREAKS_INTO_FIELD, &residue)]),
    )?;
    let quantities = the_length_and_its_bound();
    let (blamed, cause) = judged(&root, AMBER_FILE);

    assert_eq!(
        TooLong {
            characters_declared: residue.chars().count(),
            blamed,
            quantities_named: mentioning(&cause, &[&quantities[0], &quantities[1]]),
        },
        TooLong {
            characters_declared: ONE_CHARACTER_TOO_MANY,
            blamed: Blamed::Declaration(blaming(AMBER, BREAKS_INTO_FIELD)),
            quantities_named: both_stated(&quantities),
        },
        "the residue is the third of the three strings a definition retains, and no scenario \
         reaches it — the spec words the bound over `name` and `texture` alone while its \
         rationale counts three strings apiece. A loader that bounded two of the three leaves the \
         allocation the bound exists to refuse reachable through the field nobody tested, and \
         `breaks_into` is the easiest of the three to overlook because it is optional and is \
         never resolved. It is refused by the same rule and named the same way"
    );
    Ok(())
}
