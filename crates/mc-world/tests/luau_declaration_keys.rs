//! Which fields a declaration carries, asked of the table rather than of the
//! mod's code, and what happens to the one nobody recognises.
//!
//! # A misspelling is a mistake, not a silent loss
//!
//! `replacable` is a word a mod author types once. Under the format this loader
//! replaces, an unrecognised key was refused by name; a loader that reads the
//! six keys it knows and never asks what else is there cannot tell a typo from
//! an absence, so the block registers, the field the author wrote does nothing,
//! and nothing anywhere says why. That is what these tests are about, and it is
//! why the refusal owes the author **the fields it does recognise** as well as
//! the one it does not — a name is only a typo once you can see what it was
//! nearly.
//!
//! # The order that list comes out in is a contract, not a detail
//!
//! Nothing in a script table's key order is defined. Measured on this
//! toolchain, a declaration written `slid, replacable, name, texture, solid`
//! hands its keys back `solid, name, slid, replacable, texture` — neither the
//! order it was written in nor any order a person could predict. A refusal
//! quoting whichever unrecognised field came back first would therefore be
//! text that varies with the backend's hashing, and the guard that holds the
//! modding pages true compares a quoted refusal against a real run **line for
//! line**. So the order is settled by the loader, and settled lexicographically.
//!
//! **A run repeated inside one process cannot show that.** Measured: a fixed set
//! of keys in a fresh script state comes back in the same order every time, so
//! stability across runs is satisfied by an implementation that simply passes
//! the backend's order through. The test that can fail is the one asserting
//! *which* order, against a fixture built so that state order, written order and
//! lexicographic order are three different orders. Both halves are asserted
//! together: the scenario asks for the first and only the second can catch
//! anything today.
//!
//! # The metamethod an enumeration meets is not the one a field read meets
//!
//! `__index` is already defended against and tested where fields are read. What
//! answers *which keys exist* is `__iter`, and a declaration can carry one
//! because `pairs` is a name a chunk may reach. Two fixtures below carry one: an
//! `__iter` that hides a field the table really holds, and an `__iter` that
//! reports a field the table does not hold. Neither may be believed.

mod common;
mod luau_common;

use std::error::Error;
use std::path::{Path, PathBuf};

use common::{TestResult, content_root};
use luau_common::{
    AMBER, AMBER_FILE, ASH, Blamed, QUARTZ, blamed_by, blaming, declaration_of, judged,
    named_in_order, raw_field, registered, registry_from, text_field,
};
use tempfile::TempDir;

/// Every field name a declaration is allowed to state.
///
/// Written out here rather than read from the loader: an expectation derived
/// from the value under test agrees with whatever that value becomes.
const RECOGNISED_FIELDS: [&str; 6] = [
    "name",
    "texture",
    "solid",
    "replaceable",
    "breakable",
    "breaks_into",
];

/// A field name nobody recognises, and the shape of the mistake that produces
/// one: a letter short of `solid`.
const A_MISSPELLING_OF_SOLID: &str = "slid";

/// A second one, a letter short of `replaceable`, so that a refusal has two
/// names to put in an order.
const A_MISSPELLING_OF_REPLACEABLE: &str = "replacable";

/// A field the table does not hold, for the metatable that invents one.
const A_FIELD_NOBODY_DECLARED: &str = "hardness";

/// How many times the ordering fixture is read, to answer the scenario's "on
/// every run".
const RUNS: usize = 8;

/// The three required fields, correctly stated.
fn the_required_three() -> Vec<String> {
    vec![
        text_field("name", AMBER),
        text_field("texture", QUARTZ),
        raw_field("solid", "true"),
    ]
}

/// The required three followed by `extra`, in that order.
fn the_required_three_and(extra: &[String]) -> Vec<String> {
    let mut fields = the_required_three();
    fields.extend_from_slice(extra);
    fields
}

/// A root holding one declaration file, written from `fields`.
fn root_declaring(directory: &TempDir, fields: &[String]) -> Result<PathBuf, Box<dyn Error>> {
    content_root(directory, &[(AMBER_FILE, declaration_of(fields))])
}

/// A root whose one declaration states `fields` and carries `metatable`.
fn root_declaring_under(
    directory: &TempDir,
    fields: &[String],
    metatable: &str,
) -> Result<PathBuf, Box<dyn Error>> {
    let table = declaration_of(fields).replacen("return {", "local declaration = {", 1);
    let chunk = format!("{table}return setmetatable(declaration, {metatable})\n");
    content_root(directory, &[(AMBER_FILE, chunk)])
}

/// A metatable whose `__iter` reports exactly `shown` and nothing else.
///
/// It reads each key back raw, so what it reports is genuinely the table's own
/// value for a key it chose to admit to — a metamethod that lied about the
/// values as well would leave it open which lie the loader believed.
fn an_iter_reporting(shown: &[&str]) -> String {
    let listed = shown
        .iter()
        .map(|key| format!("'{key}'"))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "{{\n\
         \t__iter = function(self)\n\
         \t\tlocal shown = {{ {listed} }}\n\
         \t\tlocal position = 0\n\
         \t\treturn function()\n\
         \t\t\tposition = position + 1\n\
         \t\t\tlocal key = shown[position]\n\
         \t\t\tif key then return key, rawget(self, key) end\n\
         \t\t\treturn nil\n\
         \t\tend\n\
         \tend,\n\
         }}"
    )
}

/// What a refusal about an unrecognised field managed to say.
///
/// What it blamed and the recognised list travel together because a refusal
/// that named the offending field perfectly and left the author guessing what
/// they nearly typed is the failure this format is judged on.
#[derive(Debug, PartialEq, Eq)]
struct Unrecognised {
    blamed: Blamed,
    recognised_fields_named: Vec<&'static str>,
}

/// The recognised field names `cause` mentions, in a fixed order so that the
/// comparison is about *which* it named rather than about the order it chose.
fn recognised_fields_named(cause: &str) -> Vec<&'static str> {
    let mut named = named_in_order(cause, &RECOGNISED_FIELDS);
    named.sort_unstable();
    named
}

/// Every recognised field name, in that same fixed order.
fn all_six_recognised() -> Vec<&'static str> {
    let mut every = RECOGNISED_FIELDS.to_vec();
    every.sort_unstable();
    every
}

/// What repeated reads of one content root said about the fields it does not
/// recognise.
#[derive(Debug, PartialEq, Eq)]
struct Listing {
    unrecognised_in_order: Vec<&'static str>,
    every_run_agreed: bool,
}

/// Reads `root` [`RUNS`] times and reports the order it named the two
/// misspellings in, and whether every run said exactly the same thing.
fn listing_of(root: &Path) -> Result<Listing, Box<dyn Error>> {
    let mut causes = Vec::with_capacity(RUNS);
    for _ in 0..RUNS {
        causes.push(judged(root, AMBER_FILE).1);
    }
    let first = causes
        .first()
        .ok_or("this fixture must be read at least once")?;
    Ok(Listing {
        unrecognised_in_order: named_in_order(
            first,
            &[A_MISSPELLING_OF_REPLACEABLE, A_MISSPELLING_OF_SOLID],
        ),
        every_run_agreed: causes.iter().all(|cause| cause == first),
    })
}

#[test]
fn a_field_the_loader_does_not_recognise_is_refused_beside_the_ones_it_does() -> TestResult {
    let directory = TempDir::new()?;
    let root = root_declaring(
        &directory,
        &the_required_three_and(&[raw_field(A_MISSPELLING_OF_SOLID, "true")]),
    )?;

    let (blamed, cause) = judged(&root, AMBER_FILE);

    assert_eq!(
        Unrecognised {
            blamed,
            recognised_fields_named: recognised_fields_named(&cause),
        },
        Unrecognised {
            blamed: Blamed::Declaration(blaming(AMBER, A_MISSPELLING_OF_SOLID)),
            recognised_fields_named: all_six_recognised(),
        },
        "a key the loader has no meaning for is a mistake, and refusing it is the only thing \
         that tells a mod author `slid` was never going to do anything. Ignoring it registers a \
         block whose author believes they declared its solidity twice. The recognised six are \
         owed with it: a name is only recognisable as a typo once you can see what it was \
         nearly, and the author reading this refusal has no other list to compare against"
    );
    Ok(())
}

#[test]
fn a_declaration_stating_all_six_recognised_fields_and_nothing_else_registers() -> TestResult {
    let directory = TempDir::new()?;
    let root = root_declaring(
        &directory,
        &the_required_three_and(&[
            raw_field("replaceable", "false"),
            raw_field("breakable", "true"),
            text_field("breaks_into", ASH),
        ]),
    )?;

    let registry = registry_from(&root)?;

    assert_eq!(
        registered(&registry, AMBER)?,
        format!("textured {QUARTZ}, solid true"),
        "the control on the refusal next door, and the only thing standing between an \
         unrecognised-field check and a check that refuses everything. Every field here is one \
         the loader is documented to accept, so a declaration using the whole contract must \
         load — a check that over-fires takes the six-field declaration down with the \
         misspelled one and this is what says so"
    );
    Ok(())
}

#[test]
fn two_unrecognised_fields_are_named_in_the_same_order_every_time() -> TestResult {
    let directory = TempDir::new()?;
    let root = root_declaring(
        &directory,
        &the_required_three_and(&[
            raw_field(A_MISSPELLING_OF_SOLID, "true"),
            raw_field(A_MISSPELLING_OF_REPLACEABLE, "true"),
        ]),
    )?;

    assert_eq!(
        listing_of(&root)?,
        Listing {
            unrecognised_in_order: vec![A_MISSPELLING_OF_REPLACEABLE, A_MISSPELLING_OF_SOLID],
            every_run_agreed: true,
        },
        "the modding pages quote this refusal and a guard compares the quotation against a real \
         run line for line, so a list ordered by the backend's hashing makes that guard \
         intermittently red and the page unwritable. The two halves are asserted together \
         because only one of them can fail today: a fixed set of keys in a fresh state comes \
         back in the same order every time, so `on every run` is satisfied by passing the \
         backend's order straight through. Which order is what catches that — this fixture \
         states `slid` before `replacable`, and the state hands them back in that same order, \
         so lexicographic is the one answer neither the writing nor the backend produces"
    );
    Ok(())
}

#[test]
fn an_iterator_that_hides_a_field_does_not_stop_it_being_refused() -> TestResult {
    let directory = TempDir::new()?;
    let root = root_declaring_under(
        &directory,
        &the_required_three_and(&[raw_field(A_MISSPELLING_OF_SOLID, "true")]),
        &an_iter_reporting(&["name", "texture", "solid"]),
    )?;

    assert_eq!(
        blamed_by(&root, AMBER_FILE),
        Blamed::Declaration(blaming(AMBER, A_MISSPELLING_OF_SOLID)),
        "the metamethod that answers `which keys does this table have` is the mod's own code, \
         and a loader that asks it has handed a declaration the power to decide what the loader \
         is allowed to notice about it. This one admits to exactly the three fields that pass, \
         so a loader that believed it would register the block and never mention `slid` — the \
         silent loss the whole unrecognised-field rule exists to prevent, arrived at \
         deliberately instead of by accident"
    );
    Ok(())
}

#[test]
fn an_iterator_that_invents_a_field_does_not_stop_a_declaration_registering() -> TestResult {
    let directory = TempDir::new()?;
    let root = root_declaring_under(
        &directory,
        &the_required_three(),
        &an_iter_reporting(&[A_FIELD_NOBODY_DECLARED]),
    )?;

    let registry = registry_from(&root)?;

    assert_eq!(
        registered(&registry, AMBER)?,
        format!("textured {QUARTZ}, solid true"),
        "the same metamethod from the other direction, and the half that says the enumeration \
         reads the table rather than merely distrusting it. A loader consulting this `__iter` \
         sees one field, recognises none of it, and refuses a declaration whose own three \
         fields are perfectly stated — so a mod author is told their block carries a `hardness` \
         they never wrote. Together with the fixture above, believing the metamethod fails in \
         both directions and reading the table's own keys passes in both"
    );
    Ok(())
}
