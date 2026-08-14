//! What a layout does with the declarations a source hands it, when one of them
//! cannot be accepted and when two of them claim the same name.
//!
//! Both fixtures here are held in memory rather than on disk, because neither
//! rule is about files: the all-or-nothing property and the duplicate refusal
//! belong to the layout, and grading them through a directory read would put a
//! second thing under test in every assertion.
//!
//! The source holds **declarations**, not checked elements. A scenario about a
//! declaration stating `size = [0, 4]` is not expressible over a source of
//! already-accepted elements — the test would have to hand-build the very fault
//! it claims the model produces, and would then pass against a model that
//! produced no faults at all.

mod common;

use std::error::Error;

use common::TestResult;
use common::hud::{Declaration, declared, extents, minimal_fill, text, with};
use mc_core::hud::source::{HudElementSource, InMemoryHudSource};
use mc_core::hud::{DeclaredValue, HudLayout, HudLoadError, HudOrigin};

/// What the source as a whole is called, for a refusal that is about the source
/// rather than about any one declaration it handed over.
const SOURCE_LABEL: &str = "a fixture content root";

/// The three files the all-or-nothing fixture declares, in the order a reader of
/// a directory would hand them over.
const FIRST_FILE: &str = "alpha.toml";
const MIDDLE_FILE: &str = "mike.toml";
const LAST_FILE: &str = "zulu.toml";

/// The three names those files declare.
const FIRST_NAME: &str = "base:alpha";
const MIDDLE_NAME: &str = "base:mike";
const LAST_NAME: &str = "base:zulu";

/// A source labelled [`SOURCE_LABEL`] handing over `declarations`, each
/// attributed to the file that stated it, in the order given.
fn source(declarations: Vec<(&str, Declaration)>) -> InMemoryHudSource {
    InMemoryHudSource::new(
        HudOrigin::new(SOURCE_LABEL),
        declarations
            .into_iter()
            .map(|(origin, declaration)| (HudOrigin::new(origin), declared(declaration)))
            .collect(),
    )
}

/// A declaration named `name`, and otherwise the smallest the spec's table
/// accepts.
fn named(name: &str) -> Declaration {
    with(minimal_fill(), "name", text(name))
}

/// Every element name loading `source` registers, in the order the layout holds
/// them — and nothing at all where the load was refused.
///
/// The two answers are deliberately collapsed, because "registered none of them"
/// is precisely what a refusal leaves behind: there is no layout to ask. What
/// keeps the collapse honest is that every assertion below pairs a refused
/// fixture with an accepted one, so an implementation that refuses everything
/// reports an empty second half and fails.
fn registered_names(source: &dyn HudElementSource) -> Vec<String> {
    match HudLayout::load(source) {
        Ok(layout) => layout
            .elements()
            .iter()
            .map(|element| element.name.as_str().to_owned())
            .collect(),
        Err(_) => Vec::new(),
    }
}

/// The refusal loading `source` produced.
///
/// # Errors
///
/// Fails if the layout accepted the source, because an assertion about a
/// refusal that never happened has learned nothing.
fn refusal(source: &dyn HudElementSource) -> Result<HudLoadError, Box<dyn Error>> {
    match HudLayout::load(source) {
        Ok(layout) => Err(format!(
            "this source must not be accepted, or the assertion below is vacuous, but it \
             registered {} element(s)",
            layout.elements().len()
        )
        .into()),
        Err(error) => Ok(error),
    }
}

/// Three declarations, of which the middle one states `size` as `stated`.
///
/// The perturbed declaration is the **middle** one on purpose: a layout that
/// registered as it went would already be holding the first by the time it
/// reached the fault, and would still have the third to hand over afterwards.
/// Every other field of all three is identical, so the two fixtures below differ
/// in exactly the thing the scenario is about.
fn three_declarations(stated: DeclaredValue) -> InMemoryHudSource {
    source(vec![
        (FIRST_FILE, named(FIRST_NAME)),
        (MIDDLE_FILE, with(named(MIDDLE_NAME), "size", stated)),
        (LAST_FILE, named(LAST_NAME)),
    ])
}

#[test]
fn three_declarations_of_which_one_states_a_zero_extent_register_none_of_the_three() -> TestResult {
    let with_a_zero_extent = three_declarations(extents(0, 4));
    let with_every_extent_positive = three_declarations(extents(9, 1));

    assert_eq!(
        (
            registered_names(&with_a_zero_extent),
            registered_names(&with_every_extent_positive),
        ),
        (
            Vec::new(),
            vec![
                FIRST_NAME.to_owned(),
                MIDDLE_NAME.to_owned(),
                LAST_NAME.to_owned(),
            ],
        ),
        "one declaration the model will not accept registers none of the three — and the same \
         three with that one extent repaired register all of them, so the empty answer is the \
         refusal rather than a fixture nothing could have loaded"
    );
    Ok(())
}

#[test]
fn two_declarations_claiming_one_name_are_refused_naming_both_of_the_files() -> TestResult {
    // The two differ in their extents, so a layout that quietly accepted a
    // declaration identical to one it already held would still be refused here.
    let both = source(vec![
        (FIRST_FILE, named(FIRST_NAME)),
        (LAST_FILE, with(named(FIRST_NAME), "size", extents(1, 9))),
    ]);

    let error = refusal(&both)?;

    let HudLoadError::AlreadyDeclared { first, second, .. } = &error else {
        return Err(format!("expected an already-declared refusal, got {error:?}").into());
    };
    assert_eq!(
        (
            first.as_str().contains(FIRST_FILE),
            second.as_str().contains(LAST_FILE),
        ),
        (true, true),
        "the refusal tells the two declarations apart, in the order the source handed them over: \
         first `{}`, second `{}`",
        first.as_str(),
        second.as_str()
    );
    Ok(())
}
