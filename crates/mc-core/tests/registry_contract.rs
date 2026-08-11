//! What a block registry promises its callers.
//!
//! Three questions, and they are the whole contract every later crate binds to:
//! what a name resolves to and what runtime id it gets, what a name is allowed
//! to be, and what a registry holds before anything has been applied to it. The
//! last one is invariant 1 in test form — an engine that ships blocks of its own
//! fails it.

mod common;

use std::error::Error;

use common::{TestResult, definition, registry_from, source};
use mc_core::block::{BlockDefinition, BlockRegistry, RegistryError};
use mc_core::id::{BlockName, NamespacedIdError, TextureKey};

/// A registry holding three blocks, registered in the order they are written.
fn three_block_registry() -> Result<BlockRegistry, Box<dyn Error>> {
    Ok(registry_from(
        "fixture-content",
        vec![
            definition("base:air", "base:air", "air.toml")?,
            definition("base:stone", "base:stone", "stone.toml")?,
            definition("base:grass", "base:grass", "grass.toml")?,
        ],
    )?)
}

/// Where the surviving declaration of `base:stone` was made.
const FIRST_STONE_ORIGIN: &str = "stone.toml";
/// Where the refused declaration of `base:stone` was made.
const SECOND_STONE_ORIGIN: &str = "stone_again.toml";

/// The first definition of `base:stone`, and the one that must survive.
fn first_stone_definition() -> Result<BlockDefinition, NamespacedIdError> {
    definition("base:stone", "base:stone_a", FIRST_STONE_ORIGIN)
}

/// A later definition of the same name, textured differently so that which of
/// the two survived is observable, and declared somewhere else so that which of
/// the two a rejection is talking about is observable too.
fn second_stone_definition() -> Result<BlockDefinition, NamespacedIdError> {
    definition("base:stone", "base:stone_b", SECOND_STONE_ORIGIN)
}

#[test]
fn a_registered_name_resolves_to_its_own_definition_under_an_id_no_other_block_shares() -> TestResult
{
    let registry = three_block_registry()?;
    let stone = BlockName::parse("base:stone")?;

    let resolved = registry.resolve(&stone)?;
    assert_eq!(
        resolved.name, stone,
        "a name resolves to the definition registered under it"
    );

    let stone_id = registry.id_of(&stone)?;
    let air_id = registry.id_of(&BlockName::parse("base:air")?)?;
    let grass_id = registry.id_of(&BlockName::parse("base:grass")?)?;
    assert!(
        stone_id != air_id && stone_id != grass_id,
        "a runtime id is unique within its registry: stone {stone_id:?}, air {air_id:?}, grass {grass_id:?}"
    );
    Ok(())
}

#[test]
fn resolving_a_name_that_was_never_registered_reports_that_name() -> TestResult {
    let registry = three_block_registry()?;
    let absent = BlockName::parse("base:diamond")?;

    let error = registry
        .resolve(&absent)
        .err()
        .ok_or("a name that was never registered must not resolve to anything")?;

    let RegistryError::UnknownName { name } = &error else {
        return Err(format!("expected an unknown-name rejection, got {error:?}").into());
    };
    assert_eq!(
        name.as_str(),
        "base:diamond",
        "the rejection names the block that could not be resolved"
    );
    Ok(())
}

#[test]
fn a_block_name_carrying_no_namespace_is_rejected_naming_the_text() -> TestResult {
    let error = BlockName::parse("stone")
        .err()
        .ok_or("a name with no namespace separator must not parse")?;

    let NamespacedIdError::MissingNamespace { text } = &error else {
        return Err(format!("expected a missing-namespace rejection, got {error:?}").into());
    };
    assert_eq!(
        text.as_str(),
        "stone",
        "the rejection names the text it refused"
    );
    Ok(())
}

/// A guard rather than a scenario, and it covers both newtypes deliberately.
/// The rule is "exactly one separator", so a parser that split on the *first*
/// one would turn `base:stone:top` into the namespace `base` and the path
/// `stone:top` — a typo becoming a plausible-looking block that resolves to
/// nothing, with no diagnostic pointing at the colon. Both newtypes are covered
/// because a check written into one of them rather than into the validation
/// core they share would leave the other silently permissive.
#[test]
fn a_block_name_carrying_more_than_one_separator_is_rejected_naming_the_text() -> TestResult {
    let error = BlockName::parse("base:stone:top")
        .err()
        .ok_or("a name with two namespace separators must not parse")?;

    let NamespacedIdError::MultipleSeparators { text } = &error else {
        return Err(format!("expected a multiple-separator rejection, got {error:?}").into());
    };
    assert_eq!(
        text.as_str(),
        "base:stone:top",
        "the rejection names the text it refused"
    );
    Ok(())
}

#[test]
fn a_texture_key_carrying_more_than_one_separator_is_rejected_naming_the_text() -> TestResult {
    let error = TextureKey::parse("base:blocks:stone")
        .err()
        .ok_or("a texture key with two namespace separators must not parse")?;

    let NamespacedIdError::MultipleSeparators { text } = &error else {
        return Err(format!("expected a multiple-separator rejection, got {error:?}").into());
    };
    assert_eq!(
        text.as_str(),
        "base:blocks:stone",
        "the rejection names the text it refused"
    );
    Ok(())
}

/// A guard rather than a scenario. The rule is one separator with *both* sides
/// non-empty, and the two ways to write a separator with nothing on one side of
/// it are the two ways to satisfy "exactly one colon" while naming nothing at
/// all. Neither is refused by the checks the scenarios cover, so without these
/// two a parser that accepted `:x` as the block `x` in the anonymous namespace
/// would pass every test in this suite.
#[test]
fn a_block_name_whose_namespace_is_empty_is_rejected_naming_the_text() -> TestResult {
    let error = BlockName::parse(":x")
        .err()
        .ok_or("a name with nothing before its separator must not parse")?;

    let NamespacedIdError::EmptyNamespace { text } = &error else {
        return Err(format!("expected an empty-namespace rejection, got {error:?}").into());
    };
    assert_eq!(
        text.as_str(),
        ":x",
        "the rejection names the text it refused"
    );
    Ok(())
}

#[test]
fn a_block_name_whose_path_is_empty_is_rejected_naming_the_text() -> TestResult {
    let error = BlockName::parse("x:")
        .err()
        .ok_or("a name with nothing after its separator must not parse")?;

    let NamespacedIdError::EmptyPath { text } = &error else {
        return Err(format!("expected an empty-path rejection, got {error:?}").into());
    };
    assert_eq!(
        text.as_str(),
        "x:",
        "the rejection names the text it refused"
    );
    Ok(())
}

#[test]
fn registering_a_name_a_second_time_is_rejected_naming_that_name() -> TestResult {
    let mut registry = registry_from("first-mod", vec![first_stone_definition()?])?;

    let error = registry
        .apply(&source("second-mod", vec![second_stone_definition()?]))
        .err()
        .ok_or("a name may be registered only once")?;

    let RegistryError::AlreadyRegistered { name, .. } = &error else {
        return Err(format!("expected an already-registered rejection, got {error:?}").into());
    };
    assert_eq!(
        name.as_str(),
        "base:stone",
        "the rejection names the block that was registered twice"
    );
    Ok(())
}

/// A guard rather than a scenario: nothing in the specification pins which
/// declaration a rejection calls first and which it calls second when the two
/// arrive in separate applications. A rejection that named the arriving origin
/// twice, or named the two the wrong way round, would pass every other test
/// here — and a mod author reading it would go and edit the wrong file.
#[test]
fn a_duplicate_rejection_names_where_each_declaration_was_made_in_arrival_order() -> TestResult {
    let mut registry = registry_from("first-mod", vec![first_stone_definition()?])?;

    let error = registry
        .apply(&source("second-mod", vec![second_stone_definition()?]))
        .err()
        .ok_or("a name may be registered only once")?;

    let RegistryError::AlreadyRegistered { first, second, .. } = &error else {
        return Err(format!("expected an already-registered rejection, got {error:?}").into());
    };
    assert_eq!(
        (first.as_str(), second.as_str()),
        (FIRST_STONE_ORIGIN, SECOND_STONE_ORIGIN),
        "a rejection tells the surviving declaration and the refused one apart, in that order"
    );
    Ok(())
}

#[test]
fn a_rejected_duplicate_leaves_the_first_definition_resolvable() -> TestResult {
    let mut registry = registry_from("first-mod", vec![first_stone_definition()?])?;
    assert!(
        registry
            .apply(&source("second-mod", vec![second_stone_definition()?]))
            .is_err(),
        "the duplicate must be rejected, or this scenario asserts nothing"
    );

    let stone = BlockName::parse("base:stone")?;
    assert_eq!(
        registry.resolve(&stone)?.texture.as_str(),
        "base:stone_a",
        "a rejected registration leaves the definition registered first in place"
    );
    Ok(())
}

#[test]
fn a_registry_to_which_no_source_has_been_applied_holds_no_blocks() {
    let registry = BlockRegistry::new();

    assert_eq!(
        registry.registered_count(),
        0,
        "the engine ships no block definitions of its own"
    );
}

#[test]
fn resolving_against_a_registry_to_which_no_source_has_been_applied_reports_the_name() -> TestResult
{
    let registry = BlockRegistry::new();
    let air = BlockName::parse("base:air")?;

    let error = registry
        .resolve(&air)
        .err()
        .ok_or("a registry holding nothing can resolve nothing")?;

    let RegistryError::UnknownName { name } = &error else {
        return Err(format!("expected an unknown-name rejection, got {error:?}").into());
    };
    assert_eq!(
        name.as_str(),
        "base:air",
        "the rejection names the block that could not be resolved"
    );
    Ok(())
}

#[test]
fn a_texture_key_is_reported_exactly_as_the_definition_declared_it() -> TestResult {
    // The block's own name deliberately differs from its texture key: a texture
    // key is a reference a block makes, never a restatement of what it is called.
    let registry = registry_from(
        "fixture-content",
        vec![definition(
            "fixture:cobblestone",
            "base:stone",
            "cobblestone.toml",
        )?],
    )?;

    let cobblestone = BlockName::parse("fixture:cobblestone")?;
    assert_eq!(
        registry.resolve(&cobblestone)?.texture.as_str(),
        "base:stone",
        "a block's texture key is reported as declared, character for character"
    );
    Ok(())
}

#[test]
fn a_texture_given_as_a_file_path_is_rejected_naming_the_value() -> TestResult {
    let error = TextureKey::parse("textures/stone.png")
        .err()
        .ok_or("a file path must not be accepted as a texture key")?;

    let NamespacedIdError::MissingNamespace { text } = &error else {
        return Err(format!("expected a missing-namespace rejection, got {error:?}").into());
    };
    assert_eq!(
        text.as_str(),
        "textures/stone.png",
        "the rejection names the value it refused"
    );
    Ok(())
}
