//! What the model publishes to content, and what it deliberately does not.
//!
//! A bad mod must not be able to disable the instrument used to diagnose it, and
//! the guarantee is held here rather than by review: content reaches the engine
//! only through the names below and the fields a declaration may spell, so if
//! neither surface names the debug overlay, no declaration can refer to it.
//!
//! Both surfaces are asserted as **sets that are exactly what the specification
//! names**, never as a list of absences. `!published.contains("debug-overlay")`
//! would be satisfied by a model publishing every name in the world; a set
//! equality is broken by adding one, which is the falsifier this file exists
//! for.

mod common;

use common::TestResult;
use mc_core::hud::{ACCEPTED_FIELDS, DRAW_KINDS, READABLE_VALUES};

/// What the specification's declaration table lets a declaration spell.
const DECLARED_FIELDS: [&str; 8] = [
    "name", "anchor", "offset", "size", "draw", "color", "source", "outline",
];

/// The published surface, read through a check that refuses to answer for an
/// empty readable-value set.
///
/// The guard is the whole reason this is a function rather than two
/// comparisons: "no published name reaches the debug overlay" is *satisfied* by
/// publishing nothing, so a surface check that accepted an empty set would go
/// green the day the set was emptied, having stopped being able to fail.
///
/// # Errors
///
/// Returns a message naming the empty surface when `readable_values` is empty.
fn published_surface(
    readable_values: &[&str],
    draw_kinds: &[&str],
) -> Result<(Vec<String>, Vec<String>), String> {
    if readable_values.is_empty() {
        return Err(
            "the published readable-value set is empty, so a check that no published name \
             reaches the debug overlay would pass without reading anything"
                .to_owned(),
        );
    }
    Ok((sorted(readable_values), sorted(draw_kinds)))
}

fn sorted(names: &[&str]) -> Vec<String> {
    let mut owned: Vec<String> = names.iter().map(|name| (*name).to_owned()).collect();
    owned.sort();
    owned
}

#[test]
fn the_published_surface_is_exactly_one_readable_value_and_two_draw_kinds() -> TestResult {
    let (readable_values, draw_kinds) = published_surface(&READABLE_VALUES, &DRAW_KINDS)?;

    assert_eq!(
        (readable_values, draw_kinds),
        (sorted(&["held-block"]), sorted(&["fill", "block-texture"])),
        "content may name exactly the live state and the drawing capabilities the specification \
         publishes, and nothing else — anything a declaration could reach beyond these is a name \
         the debug overlay has to be defended from"
    );
    Ok(())
}

/// The vacuity guard for the assertion above, and a test in its own right
/// rather than a branch inside it: as one test, "the guard stopped working
/// while the surface assertion still passed" is not something a run can show
/// you happening; as two, it is.
#[test]
fn the_published_surface_check_refuses_an_empty_readable_value_set() -> TestResult {
    let refusal = published_surface(&[], &DRAW_KINDS)
        .err()
        .ok_or("an empty published set must be refused, not reported as reaching no overlay")?;

    assert!(
        refusal.contains("empty"),
        "the refusal says the published set was empty rather than reporting a clean surface: \
         {refusal}"
    );
    Ok(())
}

/// The structural half of the same guarantee. A declaration reaches the engine
/// through the fields it may spell, so the accepted set *being exactly these
/// eight* is what makes "no field names the debug overlay" a fact rather than a
/// hope: an `overlay` field cannot be added without this comparison breaking.
#[test]
fn the_accepted_field_set_is_exactly_the_eight_the_declaration_table_names() -> TestResult {
    assert_eq!(
        sorted(&ACCEPTED_FIELDS),
        sorted(&DECLARED_FIELDS),
        "a declaration may spell exactly the fields the specification's table declares; a field \
         outside that set is a way for content to name something the engine owns"
    );
    Ok(())
}
