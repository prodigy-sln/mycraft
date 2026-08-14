//! The two vocabularies a declaration writes values in: colours and anchors.
//!
//! `#RRGGBBAA` with eight hex digits and no shorthand, on `color` and `outline`
//! alike — strict now, relaxable later, the direction the namespaced-id rule
//! already takes. A rule can be loosened without invalidating content that was
//! already written; it cannot be tightened.

mod common;

use common::TestResult;
use common::hud::{FIXTURE_NAME, listed_words, minimal_fill, refused, text, with};
use mc_core::hud::ANCHOR_NAMES;

#[test]
fn a_colour_written_in_shorthand_is_refused_naming_the_colour_field() -> TestResult {
    let fault = refused(with(minimal_fill(), "color", text("#FFF")))?;

    assert_eq!(
        (fault.element.as_deref(), fault.field.as_deref()),
        (Some(FIXTURE_NAME), Some("color")),
        "a colour is eight hex digits and shorthand is not accepted, naming the field that used \
         it: {fault}"
    );
    Ok(())
}

/// The same rule on the second colour field, because a rule written into one of
/// them rather than into the check they share would leave the other silently
/// permissive.
#[test]
fn an_outline_written_in_shorthand_is_refused_naming_the_outline_field() -> TestResult {
    let fault = refused(with(minimal_fill(), "outline", text("#000")))?;

    assert_eq!(
        (fault.element.as_deref(), fault.field.as_deref()),
        (Some(FIXTURE_NAME), Some("outline")),
        "an outline is a colour and obeys the colour rule, naming the field that used shorthand: \
         {fault}"
    );
    Ok(())
}

/// The listing is read out of the published anchor names rather than typed
/// here, so a tenth anchor cannot arrive without this refusal offering it.
#[test]
fn a_declaration_naming_an_unknown_anchor_is_refused_listing_every_accepted_anchor() -> TestResult {
    let fault = refused(with(minimal_fill(), "anchor", text("middle")))?;

    assert_eq!(
        fault.field.as_deref(),
        Some("anchor"),
        "an unknown anchor is refused naming the field that stated it: {fault}"
    );
    let listed = listed_words(&fault.cause);
    for anchor in ANCHOR_NAMES {
        assert!(
            listed.contains(&anchor),
            "the refusal offers every anchor the model accepts, and `{anchor}` is missing from \
             it: {fault}"
        );
    }
    Ok(())
}
