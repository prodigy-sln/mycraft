//! The three fields a declaration may leave out, and the id the third one
//! names.
//!
//! `replaceable`, `breakable` and `breaks_into` are optional, independent, and
//! carry the defaults they have always carried: a block nobody may build over,
//! that anybody may break, leaving the cell empty. A declaration that says
//! nothing about any of them gets exactly that.
//!
//! # Absent and stated wrongly are two different things
//!
//! An optional field is optional in the sense that leaving it out is a
//! declaration; it is not optional in the sense that any value will do.
//! `replaceable = 1` is a mistake, and reading it as the absent-means-false
//! default is how a mod author never finds out they made it — their block
//! behaves exactly as it would have if they had written nothing, so there is
//! nothing to notice. Two of the tests below are that distinction and nothing
//! else.
//!
//! # A residue is resolved where a break reads it
//!
//! `breaks_into` names a block, and the block it names need not exist yet.
//! Definitions arrive in batches and a mod may legitimately name a residue that
//! another file, or another content root, declares — so the loader records the
//! name and does not go looking for it. What it *does* check is the same rule
//! every other id obeys: exactly one separator, both sides non-empty. Those two
//! pull in opposite directions, which is why both have a test: a loader strict
//! enough to refuse `ash` and lenient enough to accept a residue nobody declared
//! is the only one that passes both.
//!
//! # Why the unbreakable block below has a residue that really exists
//!
//! Its subject is the independence of the two fields, not resolution. If its
//! residue were undeclared it would fail beside the residue test for one shared
//! reason, and a single defect would read as two. Declared here, undeclared
//! there, the two tests discriminate.

mod common;
mod luau_common;

use std::error::Error;
use std::path::PathBuf;

use common::{TestResult, content_root};
use luau_common::{
    AMBER, AMBER_FILE, ASH, ASH_FILE, Behaviour, Blamed, QUARTZ, behaviour_of, blamed_by, blaming,
    declaration_of, declaring, judged, raw_field, registered, registry_from, text_field,
    the_documented_defaults,
};
use tempfile::TempDir;

/// An id carrying no namespace, which is a rule every id in this engine obeys.
const A_RESIDUE_WITHOUT_A_NAMESPACE: &str = "ash";

/// The three required fields, correctly stated, for fixtures varying an
/// optional one.
fn the_required_three() -> Vec<String> {
    vec![
        text_field("name", AMBER),
        text_field("texture", QUARTZ),
        raw_field("solid", "true"),
    ]
}

/// The required three followed by `extra`.
fn the_required_three_and(extra: &[String]) -> Vec<String> {
    let mut fields = the_required_three();
    fields.extend_from_slice(extra);
    fields
}

/// A root holding one declaration file, written from `fields`.
fn root_declaring(directory: &TempDir, fields: &[String]) -> Result<PathBuf, Box<dyn Error>> {
    content_root(directory, &[(AMBER_FILE, declaration_of(fields))])
}

/// A root holding that declaration and a second file declaring the residue it
/// names.
fn root_declaring_the_residue_too(
    directory: &TempDir,
    fields: &[String],
) -> Result<PathBuf, Box<dyn Error>> {
    content_root(
        directory,
        &[
            (AMBER_FILE, declaration_of(fields)),
            (ASH_FILE, declaring(ASH)),
        ],
    )
}

/// What a refusal about an optional field owes: what it blamed, and whether it
/// says which rule the value broke rather than only which field held it.
#[derive(Debug, PartialEq, Eq)]
struct Refusal {
    blamed: Blamed,
    states_the_rule: bool,
}

#[test]
fn a_declaration_stating_none_of_the_optional_fields_gets_the_documented_defaults() -> TestResult {
    let directory = TempDir::new()?;
    let root = root_declaring(&directory, &the_required_three())?;

    let registry = registry_from(&root)?;

    assert_eq!(
        behaviour_of(&registry, AMBER)?,
        the_documented_defaults(),
        "the defaults are a documented contract a mod author reads before writing anything, and \
         all three are asserted in one comparison so a loader that resolved two of them the way \
         the pages promise is not mistaken for one that resolved all three that way. Each is \
         the conservative answer for its own reason: a block that never said it may be built \
         over cannot be, a sandbox whose blocks were indestructible until each said otherwise \
         would be the wrong burden to put on content, and the absence of a block is not a \
         residue worth naming"
    );
    Ok(())
}

#[test]
fn an_unbreakable_block_may_still_name_what_it_would_have_left_behind() -> TestResult {
    let directory = TempDir::new()?;
    let root = root_declaring_the_residue_too(
        &directory,
        &the_required_three_and(&[
            raw_field("breakable", "false"),
            text_field("breaks_into", ASH),
        ]),
    )?;

    let registry = registry_from(&root)?;

    assert_eq!(
        behaviour_of(&registry, AMBER)?,
        Behaviour {
            replaceable: false,
            breakable: false,
            breaks_into: Some(ASH.to_owned()),
        },
        "the three optional fields are independent, and this is the pair whose independence a \
         loader is most likely to quietly decide for itself — a residue on a block that cannot \
         be broken looks like a contradiction worth refusing or worth dropping, and it is \
         neither. It is simply never reached, and a block later made breakable by an author \
         editing one line finds its residue still there. The third field is asserted alongside \
         so that a loader which read `breakable` by overwriting the whole record has nowhere \
         to hide"
    );
    Ok(())
}

#[test]
fn a_replaceability_written_as_a_number_is_refused_rather_than_defaulted() -> TestResult {
    let directory = TempDir::new()?;
    let root = root_declaring(
        &directory,
        &the_required_three_and(&[raw_field("replaceable", "1")]),
    )?;

    assert_eq!(
        blamed_by(&root, AMBER_FILE),
        Blamed::Declaration(blaming(AMBER, "replaceable")),
        "optional means a declaration may leave the field out, never that any value will do. \
         Falling back to the absent-means-false default here is the worst available outcome: \
         the block behaves exactly as it would have if the author had written nothing at all, \
         so there is no symptom to notice and nothing to search for. It is also what makes \
         `absent` and `stated wrongly` two facts rather than one"
    );
    Ok(())
}

#[test]
fn a_residue_written_as_a_number_is_refused_rather_than_read_as_no_residue() -> TestResult {
    let directory = TempDir::new()?;
    let root = root_declaring(
        &directory,
        &the_required_three_and(&[raw_field("breaks_into", "3")]),
    )?;

    assert_eq!(
        blamed_by(&root, AMBER_FILE),
        Blamed::Declaration(blaming(AMBER, "breaks_into")),
        "a residue of the wrong kind is not a residue, and the default for this field is \
         `nothing` — so reading it as the default hands the author a block that breaks into \
         empty air with no diagnostic anywhere. The refusal names the field, because a mod \
         author who wrote a number here needs to be sent to the line rather than told the \
         declaration is bad"
    );
    Ok(())
}

#[test]
fn a_residue_carrying_no_namespace_is_refused_naming_the_rule_it_broke() -> TestResult {
    let directory = TempDir::new()?;
    let root = root_declaring(
        &directory,
        &the_required_three_and(&[text_field("breaks_into", A_RESIDUE_WITHOUT_A_NAMESPACE)]),
    )?;

    let (blamed, cause) = judged(&root, AMBER_FILE);

    assert_eq!(
        Refusal {
            blamed,
            states_the_rule: cause.contains("namespace:path"),
        },
        Refusal {
            blamed: Blamed::Declaration(blaming(AMBER, "breaks_into")),
            states_the_rule: true,
        },
        "every id in this engine obeys one rule and a residue is an id like any other — a \
         loader that checked the block's own name and its texture key and then took whatever \
         this field held would let `ash` through, and it would resolve to nothing on the first \
         break with nothing to explain why. The rule is quoted rather than merely the field, \
         because `ash` looks entirely reasonable to whoever wrote it"
    );
    Ok(())
}

#[test]
fn a_residue_nothing_in_the_root_declares_still_registers() -> TestResult {
    let directory = TempDir::new()?;
    let root = root_declaring(
        &directory,
        &the_required_three_and(&[text_field("breaks_into", ASH)]),
    )?;

    let registry = registry_from(&root)?;

    assert_eq!(
        (
            registered(&registry, AMBER)?,
            behaviour_of(&registry, AMBER)?
        ),
        (
            format!("textured {QUARTZ}, solid true"),
            Behaviour {
                replaceable: false,
                breakable: true,
                breaks_into: Some(ASH.to_owned()),
            }
        ),
        "this is the scenario that constrains the design most: a residue is resolved where a \
         break reads it, not where it is declared, so the loader must not go looking for \
         `example:ash` — content arrives in batches and a block may legitimately name a residue \
         that a later file, or a later mod, declares. The name it retained is asserted and not \
         only that the block registered, because a loader that dropped the field entirely also \
         registers, and dropping it is the other way to make a residue unresolvable"
    );
    Ok(())
}
