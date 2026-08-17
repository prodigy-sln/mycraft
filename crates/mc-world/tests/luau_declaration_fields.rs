//! The three fields every declaration must state, and what a refusal owes
//! whoever wrote one wrongly.
//!
//! `name`, `texture` and `solid` are required, independent, and typed. A field
//! left out, a field of the wrong kind and an id that breaks the namespacing
//! rule are three different mistakes, and a mod author gets told which one they
//! made, in which file, about which block.
//!
//! # Why every fixture states a texture that is not the block's own name
//!
//! Because a loader that read `name` into both fields would otherwise be green
//! throughout — which is what the first draft of these scenarios permitted, and
//! what `BlockRegistry::texture_keys` warns about in its own doc comment. One
//! test below asserts the distinction head-on; the rest simply never give it
//! anywhere to hide.
//!
//! # What a refusal may name, and what it may not
//!
//! The block is read before anything is checked, so a refusal can say which
//! declaration it is about. Where the name is missing or is not text there is
//! genuinely nothing to quote back, and the refusal says so by naming no block
//! rather than by inventing one.

mod common;
mod luau_common;

use std::error::Error;
use std::path::PathBuf;

use common::{TestResult, content_root};
use luau_common::{
    AMBER, AMBER_FILE, Attribution, QUARTZ, attribution_of, blaming, blaming_field_alone,
    declaration_of, fault_from, raw_field, registered, registry_from, text_field, texture_keys,
};
use tempfile::TempDir;

/// The name field, correctly stated, for the fixtures that vary something else.
fn stated_name() -> String {
    text_field("name", AMBER)
}

/// The texture field, correctly stated, naming a key that is not the block's own
/// name.
fn stated_texture() -> String {
    text_field("texture", QUARTZ)
}

/// A root holding one declaration file, written from `fields`.
fn root_declaring(directory: &TempDir, fields: &[String]) -> Result<PathBuf, Box<dyn Error>> {
    content_root(directory, &[(AMBER_FILE, declaration_of(fields))])
}

/// What a refusal owes its reader: where it happened, and whether it says why
/// rather than only where.
///
/// The two travel together because a refusal that located itself perfectly and
/// explained nothing is the failure this format is judged on.
#[derive(Debug, PartialEq, Eq)]
struct Refusal {
    attribution: Attribution,
    states_the_reason: bool,
}

#[test]
fn a_declaration_stating_it_is_not_solid_registers_a_block_that_reports_non_solid() -> TestResult {
    let directory = TempDir::new()?;
    let root = root_declaring(
        &directory,
        &[stated_name(), stated_texture(), raw_field("solid", "false")],
    )?;

    let registry = registry_from(&root)?;

    assert_eq!(
        registered(&registry, AMBER)?,
        format!("textured {QUARTZ}, solid false"),
        "solidity is read from the declaration rather than assumed. A loader that registered \
         every block solid would be invisible until the first block somebody could walk through"
    );
    Ok(())
}

#[test]
fn a_declaration_naming_a_texture_other_than_itself_registers_both_as_stated() -> TestResult {
    let directory = TempDir::new()?;
    let root = root_declaring(
        &directory,
        &[stated_name(), stated_texture(), raw_field("solid", "true")],
    )?;

    let registry = registry_from(&root)?;

    assert_eq!(
        (registered(&registry, AMBER)?, texture_keys(&registry)),
        (
            format!("textured {QUARTZ}, solid true"),
            vec![QUARTZ.to_owned()]
        ),
        "a block's name and its texture key are two independent fields that happen to coincide \
         across everything this repository ships, which is exactly why one fixture has to make \
         them differ. The key is read back twice by different routes — resolved from the block, \
         and enumerated out of the whole registry — so a loader that wrote the name into both \
         fields has nowhere to be right"
    );
    Ok(())
}

#[test]
fn a_declaration_that_states_no_solidity_is_refused_naming_the_file_the_block_and_the_field()
-> TestResult {
    let directory = TempDir::new()?;
    let root = root_declaring(&directory, &[stated_name(), stated_texture()])?;

    let fault = fault_from(&root)?;

    assert_eq!(
        attribution_of(&fault, AMBER_FILE),
        blaming(AMBER, "solid"),
        "a required field left out is refused, never defaulted. All three parts of the \
         attribution are what makes the message usable: the file to open, the block to look \
         for in it, and the field to write: {fault:?}"
    );
    Ok(())
}

#[test]
fn a_solidity_written_as_text_is_refused_naming_the_field_and_the_kind_of_value_it_holds()
-> TestResult {
    let directory = TempDir::new()?;
    let root = root_declaring(
        &directory,
        &[stated_name(), stated_texture(), text_field("solid", "yes")],
    )?;

    let fault = fault_from(&root)?;

    assert_eq!(
        Refusal {
            attribution: attribution_of(&fault, AMBER_FILE),
            states_the_reason: fault.cause.contains("string"),
        },
        Refusal {
            attribution: blaming(AMBER, "solid"),
            states_the_reason: true,
        },
        "`solid = 'yes'` is a mistake a mod author makes once and has to be able to see, so it \
         is refused rather than read as truthy. Naming the field alone leaves them guessing what \
         was wrong with it — the refusal states the kind of value it found, in the word Luau's \
         own `type` would give them, which is `string` and not the host's internal name for it: \
         {fault:?}"
    );
    Ok(())
}

#[test]
fn a_declaration_that_states_no_texture_is_refused_naming_the_file_the_block_and_the_field()
-> TestResult {
    let directory = TempDir::new()?;
    let root = root_declaring(&directory, &[stated_name(), raw_field("solid", "true")])?;

    let fault = fault_from(&root)?;

    assert_eq!(
        attribution_of(&fault, AMBER_FILE),
        blaming(AMBER, "texture"),
        "the three required fields are independent, and a declaration missing its texture key \
         is refused on the same terms as one missing its solidity — naming the one that is \
         missing, so a mod author is not left bisecting a file: {fault:?}"
    );
    Ok(())
}

#[test]
fn a_declaration_that_states_no_name_is_refused_naming_the_field_and_no_block() -> TestResult {
    let directory = TempDir::new()?;
    let root = root_declaring(&directory, &[stated_texture(), raw_field("solid", "true")])?;

    let fault = fault_from(&root)?;

    assert_eq!(
        attribution_of(&fault, AMBER_FILE),
        blaming_field_alone("name"),
        "a declaration that never named itself leaves nothing to quote back, so the refusal \
         names the file and the field and stops there. A block slot filled with something the \
         loader invented — the file's own name, say — reads as a declaration that exists: \
         {fault:?}"
    );
    Ok(())
}

#[test]
fn a_name_written_as_a_number_is_refused_naming_the_field_and_no_block() -> TestResult {
    let directory = TempDir::new()?;
    let root = root_declaring(
        &directory,
        &[
            raw_field("name", "42"),
            stated_texture(),
            raw_field("solid", "true"),
        ],
    )?;

    let fault = fault_from(&root)?;

    assert_eq!(
        attribution_of(&fault, AMBER_FILE),
        blaming_field_alone("name"),
        "a name of the wrong kind is not a name, so there is still nothing to quote back — the \
         same answer a declaration that stated no name at all gets, reached from the other \
         direction. Rendering `42` into the block slot would report a block nobody declared: \
         {fault:?}"
    );
    Ok(())
}

#[test]
fn a_name_carrying_no_namespace_is_refused_naming_the_field_and_the_rule_it_broke() -> TestResult {
    let directory = TempDir::new()?;
    let root = root_declaring(
        &directory,
        &[
            text_field("name", "amber"),
            stated_texture(),
            raw_field("solid", "true"),
        ],
    )?;

    let fault = fault_from(&root)?;

    assert_eq!(
        Refusal {
            attribution: attribution_of(&fault, AMBER_FILE),
            states_the_reason: fault.cause.contains("namespace:path"),
        },
        Refusal {
            attribution: blaming("amber", "name"),
            states_the_reason: true,
        },
        "the id is text and readable, so the refusal quotes it back as the block it is about \
         while still refusing it. What a mod author needs beyond that is the rule — `namespace:\
         path`, one separator, both sides non-empty — because `amber` looks perfectly reasonable \
         to whoever wrote it: {fault:?}"
    );
    Ok(())
}

#[test]
fn a_texture_key_carrying_two_separators_is_refused_naming_that_field() -> TestResult {
    let directory = TempDir::new()?;
    let root = root_declaring(
        &directory,
        &[
            stated_name(),
            text_field("texture", "example:amber:top"),
            raw_field("solid", "true"),
        ],
    )?;

    let fault = fault_from(&root)?;

    assert_eq!(
        Refusal {
            attribution: attribution_of(&fault, AMBER_FILE),
            states_the_reason: fault.cause.contains("more than one namespace separator"),
        },
        Refusal {
            attribution: blaming(AMBER, "texture"),
            states_the_reason: true,
        },
        "splitting on the first separator would turn this into the path `amber:top` inside the \
         namespace `example` — a plausible-looking key that resolves to nothing, with no \
         diagnostic anywhere near the colon that caused it. The rule is exactly one separator, \
         and the refusal says which rule was broken rather than only which field: {fault:?}"
    );
    Ok(())
}
