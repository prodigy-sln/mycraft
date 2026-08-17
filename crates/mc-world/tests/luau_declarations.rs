//! A declaration is a chunk that runs, and the table it returns is the block.
//!
//! This is the half of the contract a mod author meets first: a file under
//! `blocks/` is evaluated by the scripting host, and whatever table it hands
//! back is the declaration. Two consequences follow and both are asserted here —
//! a declaration may *compute* what it declares, because it is code rather than
//! a document; and a chunk that hands back something other than a table is
//! refused with nothing to blame but the file, there being neither a block nor a
//! field to name.
//!
//! # The one declaration in this suite that must not be trivial
//!
//! Evaluating a declaration is an entry into script and runs under the limits
//! the engine ships. Every hostile-declaration test in this suite is
//! all-unwanted by nature, so a loader that ran content under an absurdly small
//! budget or an absurdly small memory cap would satisfy every one of them while
//! refusing perfectly ordinary content. The declaration below is what stops
//! that: it does work no test-sized host could afford, and it has to register.

mod common;
mod luau_common;

use std::error::Error;
use std::path::PathBuf;

use common::{TestResult, content_root};
use luau_common::{
    AMBER, AMBER_FILE, QUARTZ, amber_after, attribution_of, declaring, fault_from, names_yielded,
    naming_the_file_alone, registered, registry_from,
};
use mc_world::content::LuauFileDefinitionSource;
use tempfile::TempDir;

/// Work sized against the limits the engine ships, run before the declaration
/// returns.
///
/// Measured against those limits: it evaluates in about 4 ms. Measured against a
/// budget of 100,000 interrupt ticks it is refused for budget exhaustion, and
/// against a 16 KiB per-entry memory cap it is refused for allocation — so a
/// loader that quietly gave itself a test-sized host reddens here rather than
/// passing the hostile tests for the wrong reason.
///
/// 200,000 loop edges against the shipped 1,000,000-tick budget and 64 KiB
/// against the 256 KiB one entry may add: a fifth and a quarter, so ordinary
/// margins remain and this is not a test that trips a real limit by accident.
///
/// The `assert` is what keeps the work alive. Without it, neither local is ever
/// read and a compiler is free to notice.
const WORK_A_TEST_SIZED_HOST_COULD_NOT_AFFORD: &str = "local tally = 0\n\
     for step = 1, 200000 do tally = tally + 1 end\n\
     local held = string.rep('x', 65536)\n\
     assert(tally == 200000 and #held == 65536, 'the declaration did not do its own work')\n";

/// A declaration whose texture key is assembled a syllable at a time, so that
/// the value it declares appears nowhere in the file's text.
///
/// A loader that read the file rather than running it can only produce
/// `example:` and three fragments.
const AMBER_ASSEMBLING_ITS_TEXTURE: &str = "local parts = { 'qu', 'ar', 'tz' }\n\
     local assembled = ''\n\
     for index = 1, #parts do assembled = assembled .. parts[index] end\n\
     return {\n\
     \tname = 'example:amber',\n\
     \ttexture = 'example:' .. assembled,\n\
     \tsolid = true,\n\
     }\n";

/// A chunk that hands back a callable rather than a declaration.
const A_CHUNK_RETURNING_A_FUNCTION: &str = "return function()\n\treturn 1\nend\n";

/// A chunk that runs to the end without returning anything at all.
const A_CHUNK_RETURNING_NOTHING: &str = "local unused = 1\n";

/// The three blocks the multi-declaration fixture declares.
const THREE_DECLARED: [&str; 3] = ["example:amber", "example:cobalt", "example:zinc"];

/// A root of three well-formed declarations, one block each.
fn three_declaration_root(directory: &TempDir) -> Result<PathBuf, Box<dyn Error>> {
    content_root(
        directory,
        &[
            ("amber.luau", declaring("example:amber")),
            ("cobalt.luau", declaring("example:cobalt")),
            ("zinc.luau", declaring("example:zinc")),
        ],
    )
}

#[test]
fn a_declaration_chunk_that_returns_a_table_registers_the_block_it_states() -> TestResult {
    let directory = TempDir::new()?;
    let root = content_root(
        &directory,
        &[(
            AMBER_FILE,
            amber_after(WORK_A_TEST_SIZED_HOST_COULD_NOT_AFFORD),
        )],
    )?;

    let registry = registry_from(&root)?;

    assert_eq!(
        registered(&registry, AMBER)?,
        format!("textured {QUARTZ}, solid true"),
        "a declaration states three things and all three have to survive the trip: the name it \
         is registered under, the texture key it names — which is not its own name — and its \
         solidity. The declaration also does 200,000 loop edges' and 64 KiB's worth of work \
         before it returns, which is what makes this the test that fails if the loader gave \
         itself a smaller budget or a smaller memory cap than the one that ships"
    );
    Ok(())
}

#[test]
fn a_texture_key_the_chunk_assembled_registers_as_the_chunk_computed_it() -> TestResult {
    if AMBER_ASSEMBLING_ITS_TEXTURE.contains("quartz") {
        return Err("the assembled key must appear nowhere in the file's text".into());
    }
    let directory = TempDir::new()?;
    let root = content_root(
        &directory,
        &[(AMBER_FILE, AMBER_ASSEMBLING_ITS_TEXTURE.to_owned())],
    )?;

    let registry = registry_from(&root)?;

    assert_eq!(
        registered(&registry, AMBER)?,
        format!("textured {QUARTZ}, solid true"),
        "a declaration is code that ran and not a document that was parsed. The key registered \
         here exists only after the chunk has concatenated it, so a loader that read the file \
         instead of evaluating it has three syllables and no key"
    );
    Ok(())
}

#[test]
fn a_chunk_returning_a_function_is_refused_naming_its_file_alone() -> TestResult {
    let directory = TempDir::new()?;
    let root = content_root(
        &directory,
        &[(AMBER_FILE, A_CHUNK_RETURNING_A_FUNCTION.to_owned())],
    )?;

    let fault = fault_from(&root)?;

    assert_eq!(
        attribution_of(&fault, AMBER_FILE),
        naming_the_file_alone(),
        "a chunk that returned a callable declared no block and named no field, so the file is \
         the only thing the refusal can honestly point at. Filling either slot would send a mod \
         author looking for a block that was never declared: {fault:?}"
    );
    Ok(())
}

#[test]
fn a_chunk_returning_nothing_is_refused_naming_its_file_alone() -> TestResult {
    let directory = TempDir::new()?;
    let root = content_root(
        &directory,
        &[(AMBER_FILE, A_CHUNK_RETURNING_NOTHING.to_owned())],
    )?;

    let fault = fault_from(&root)?;

    assert_eq!(
        attribution_of(&fault, AMBER_FILE),
        naming_the_file_alone(),
        "a chunk that forgot to return is the commonest mistake in this whole format, and it is \
         refused on exactly the terms a chunk returning the wrong kind of thing is. The two are \
         different values inside the host and must not be different refusals outside it: \
         {fault:?}"
    );
    Ok(())
}

#[test]
fn a_source_asked_for_its_definitions_twice_yields_the_same_three_both_times() -> TestResult {
    let directory = TempDir::new()?;
    let root = three_declaration_root(&directory)?;
    let source = LuauFileDefinitionSource::new(&root);
    let expected: Vec<String> = THREE_DECLARED.into_iter().map(String::from).collect();

    let first = names_yielded(&source)?;
    let second = names_yielded(&source)?;

    assert_eq!(
        (first, second),
        (expected.clone(), expected),
        "a source hands back a stream and nothing says it may only be asked once. Both readings \
         are compared against the three names this test wrote, never against each other, so a \
         source that answered the same wrong thing twice is a failure here rather than an \
         agreement"
    );
    Ok(())
}
