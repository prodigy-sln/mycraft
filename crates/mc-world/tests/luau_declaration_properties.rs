//! The three properties a declaration may state about how a block is *seen*,
//! and what it means by leaving them out.
//!
//! `drawn`, `occludes` and `targetable` are three separate answers, and each
//! defaults to whatever the declaration says about `solid`. That default is the
//! reason this file exists rather than a paragraph in
//! `luau_declaration_options.rs`: the other three optional fields carry a
//! *constant* default, so a fixture stating nothing pins one value. These carry a
//! **derived** one, so a fixture stating nothing pins whatever `solid` says — and
//! a loader that resolved all three to `false` unconditionally satisfies every
//! non-solid fixture in the workspace by construction. Measured with
//! `grep -rn "BlockDefinition {" --include=*.rs crates/ | grep -v "pub struct"`:
//! 22 constructions, 21 of them in fixtures, none of which states any of the
//! three.
//!
//! # Which fixture can fail, and which cannot
//!
//! Stated rather than left for a reader to work out, because it decides how these
//! four tests are read:
//!
//! - The **solid** block saying nothing more cannot pass against a loader
//!   answering `false`, and passes vacuously against one answering `true`.
//! - The **non-solid** block saying nothing more is the exact mirror: vacuous
//!   against `false`, falsifying against `true`.
//! - The block that says `solid = false, drawn = true` is the one fixture in this
//!   file where no two of the three can be each other. It fails against both, and
//!   it is the only one here that does.
//!
//! So neither an under-eager nor an over-eager loader is caught by one fixture,
//! and a phase that watched only one of them go red has measured half of what
//! these say (`standards/global/testing.md` §2, "One skeleton is often not
//! enough").
//!
//! # Solidity travels in every comparison
//!
//! Each reading below carries `solid` beside the three, so a mismatch reads as
//! the whole declaration the loader built rather than as three booleans with no
//! stated reason to be what they are — and so a loader that resolved the three
//! correctly *from the wrong field* has somewhere to be wrong.

mod common;
mod luau_common;

use std::error::Error;
use std::path::PathBuf;

use common::{TestResult, content_root};
use luau_common::{
    AMBER, AMBER_FILE, Blamed, QUARTZ, blaming, declaration_of, judged, raw_field, registry_from,
    text_field,
};
use mc_core::block::BlockRegistry;
use mc_core::id::BlockName;
use tempfile::TempDir;

/// The key a declaration states its drawnness in.
const DRAWN_FIELD: &str = "drawn";

/// A drawnness written as a number, which is the mistake this field invites: the
/// two values it accepts are spelled `true` and `false`, and every other language
/// a mod author has written spells one of them `1`.
const A_DRAWNNESS_WRITTEN_AS_A_NUMBER: &str = "1";

/// What a refusal about a drawnness of the wrong kind owes: the file, the block,
/// the field, and the sentence a mod author reads.
///
/// The cause travels whole rather than as a substring check. A refusal naming the
/// field and then saying something else entirely about it — the loader's own
/// `a string` sentence, say, or the sentence for a field nobody recognises — is
/// what a `contains` on the field name cannot see, and it is the likelier of the
/// two failures while the field is new.
#[derive(Debug, PartialEq, Eq)]
struct Refusal {
    blamed: Blamed,
    cause: String,
}

/// What a declaration said about how a block is seen, beside the field the three
/// of them default from.
///
/// A record rather than four readings, so one comparison reports all four at once
/// and a loader that resolved three of them correctly is not mistaken for one
/// that resolved them all correctly.
#[derive(Debug, PartialEq, Eq)]
struct Seen {
    solid: bool,
    drawn: bool,
    occludes: bool,
    targetable: bool,
}

/// What `registry` holds for `name` in those four fields.
///
/// # Errors
///
/// Returns an error if `name` is not a namespaced id or the registry does not
/// hold it.
fn seen(registry: &BlockRegistry, name: &str) -> Result<Seen, Box<dyn Error>> {
    let definition = registry.resolve(&BlockName::parse(name)?)?;
    Ok(Seen {
        solid: definition.is_solid,
        drawn: definition.drawn,
        occludes: definition.occludes,
        targetable: definition.targetable,
    })
}

/// The two fields every fixture here states, plus the solidity it is about.
fn declared_solid(solid: bool) -> Vec<String> {
    vec![
        text_field("name", AMBER),
        text_field("texture", QUARTZ),
        raw_field("solid", if solid { "true" } else { "false" }),
    ]
}

/// Those fields followed by `extra`.
fn declared_solid_and(solid: bool, extra: &[String]) -> Vec<String> {
    let mut fields = declared_solid(solid);
    fields.extend_from_slice(extra);
    fields
}

/// A root holding one declaration file, written from `fields`.
fn root_declaring(directory: &TempDir, fields: &[String]) -> Result<PathBuf, Box<dyn Error>> {
    content_root(directory, &[(AMBER_FILE, declaration_of(fields))])
}

#[test]
fn a_solid_block_that_states_nothing_more_is_drawn_occludes_and_can_be_aimed_at() -> TestResult {
    let directory = TempDir::new()?;
    let root = root_declaring(&directory, &declared_solid(true))?;

    let registry = registry_from(&root)?;

    assert_eq!(
        seen(&registry, AMBER)?,
        Seen {
            solid: true,
            drawn: true,
            occludes: true,
            targetable: true,
        },
        "every declaration written before these three fields existed states solidity and \
         nothing else, and each of them has to keep meaning what it meant — a solid block is \
         drawn, hides what is behind it, and can be aimed at, which is exactly what one bit \
         used to answer. The default is therefore not a convenience: it is the whole of what \
         makes an existing declaration still valid, and it is stated per field so that a \
         loader defaulting two of them from `solid` and the third from nothing has somewhere \
         to fail"
    );
    Ok(())
}

#[test]
fn a_non_solid_block_that_states_nothing_more_is_undrawn_transparent_and_cannot_be_aimed_at()
-> TestResult {
    let directory = TempDir::new()?;
    let root = root_declaring(&directory, &declared_solid(false))?;

    let registry = registry_from(&root)?;

    assert_eq!(
        seen(&registry, AMBER)?,
        Seen {
            solid: false,
            drawn: false,
            occludes: false,
            targetable: false,
        },
        "the other half of the default, and the half that keeps the shipped sea invisible \
         until content says otherwise: splitting one bit into four must not make a block \
         appear that nobody asked to see. A declaration that said `solid = false` was saying \
         all four things, and it goes on saying all four until it states one of them \
         separately"
    );
    Ok(())
}

#[test]
fn a_non_solid_block_stating_it_is_drawn_still_neither_occludes_nor_can_be_aimed_at() -> TestResult
{
    let directory = TempDir::new()?;
    let root = root_declaring(
        &directory,
        &declared_solid_and(false, &[raw_field(DRAWN_FIELD, "true")]),
    )?;

    let registry = registry_from(&root)?;

    assert_eq!(
        seen(&registry, AMBER)?,
        Seen {
            solid: false,
            drawn: true,
            occludes: false,
            targetable: false,
        },
        "the one fixture in this file in which no two of the four can be each other, and the \
         only one that can fail against a loader answering the same constant for all three. \
         A field stated overrides the default for itself alone: a mod author who says their \
         block is visible has said nothing about whether it hides what is behind it or \
         whether a swing can find it, and a loader that carried `drawn` across to the other \
         two would hand them three decisions for one"
    );
    Ok(())
}

#[test]
fn a_drawnness_written_as_a_number_is_refused_naming_the_field_and_the_two_values_it_accepts()
-> TestResult {
    let directory = TempDir::new()?;
    let root = root_declaring(
        &directory,
        &declared_solid_and(
            true,
            &[raw_field(DRAWN_FIELD, A_DRAWNNESS_WRITTEN_AS_A_NUMBER)],
        ),
    )?;

    let (blamed, cause) = judged(&root, AMBER_FILE);

    assert_eq!(
        Refusal { blamed, cause },
        Refusal {
            blamed: Blamed::Declaration(blaming(AMBER, DRAWN_FIELD)),
            cause: format!("`{DRAWN_FIELD}` must be true or false, but is a number"),
        },
        "`drawn = 1` is the mistake this field invites, and falling back to the default for \
         it is the worst available outcome: the default here is `solid`, which this fixture \
         states as `true`, so the block would be drawn exactly as the author intended and \
         they would never learn that the line they wrote did nothing. The next one they write \
         it in has `solid = false`, and then their block is invisible for a reason no \
         diagnostic anywhere mentions. Both values are quoted rather than only the field, \
         because an author who wrote `1` needs to be told what to write instead"
    );
    Ok(())
}
