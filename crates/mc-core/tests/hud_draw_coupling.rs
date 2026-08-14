//! Which fields a draw kind requires, which it forbids, and what a refusal
//! offers whoever wrote a spelling nobody publishes.
//!
//! Four of these are two mirrored pairs — a missing companion and a companion
//! that can have no effect — and two of them name `color` while the other two
//! name `source`. Each fixture is therefore built so the mirror's rule cannot
//! be what fired: the declaration that forbids `source` states a valid `color`,
//! and the one that forbids `color` states a valid `source`.
//!
//! A field that can have no effect is a fault and not an ignored field. Content
//! that states `source` beside `draw = "fill"` believes something is reading it,
//! and registering the element anyway ships that belief.

mod common;

use common::TestResult;
use common::hud::{
    FIXTURE_NAME, OPAQUE_WHITE, listed_words, minimal_block_texture, minimal_fill, refused, text,
    with, without,
};
use mc_core::hud::{DRAW_KINDS, READABLE_VALUES};

#[test]
fn a_fill_declaration_omitting_its_colour_is_refused_naming_the_colour_field() -> TestResult {
    let fault = refused(without(minimal_fill(), "color"))?;

    assert_eq!(
        (fault.element.as_deref(), fault.field.as_deref()),
        (Some(FIXTURE_NAME), Some("color")),
        "a fill has nothing to draw without a colour, so the colour is required and its absence \
         names it: {fault}"
    );
    Ok(())
}

#[test]
fn a_block_texture_declaration_omitting_its_source_is_refused_naming_the_source_field() -> TestResult
{
    let fault = refused(without(minimal_block_texture(), "source"))?;

    assert_eq!(
        (fault.element.as_deref(), fault.field.as_deref()),
        (Some(FIXTURE_NAME), Some("source")),
        "a block texture has nothing to read without a source, so the source is required and its \
         absence names it: {fault}"
    );
    Ok(())
}

#[test]
fn a_fill_declaration_also_stating_a_source_is_refused_naming_the_source_field() -> TestResult {
    let fault = refused(with(minimal_fill(), "source", text("held-block")))?;

    assert_eq!(
        (fault.element.as_deref(), fault.field.as_deref()),
        (Some(FIXTURE_NAME), Some("source")),
        "a fill reads no source, so a declaration stating one is refused naming it rather than \
         registered with a field that can have no effect: {fault}"
    );
    Ok(())
}

#[test]
fn a_block_texture_declaration_also_stating_a_colour_is_refused_naming_the_colour_field()
-> TestResult {
    let fault = refused(with(minimal_block_texture(), "color", text(OPAQUE_WHITE)))?;

    assert_eq!(
        (fault.element.as_deref(), fault.field.as_deref()),
        (Some(FIXTURE_NAME), Some("color")),
        "a block texture draws no colour, so a declaration stating one is refused naming it \
         rather than registered with a field that can have no effect: {fault}"
    );
    Ok(())
}

/// The listing is read out of the published set rather than typed here, so the
/// message cannot drift away from what the model actually accepts.
#[test]
fn a_declaration_naming_an_unpublished_draw_kind_is_refused_listing_every_published_kind()
-> TestResult {
    let fault = refused(with(minimal_fill(), "draw", text("glow")))?;

    assert_eq!(
        fault.field.as_deref(),
        Some("draw"),
        "an unknown draw kind is refused naming the field that stated it: {fault}"
    );
    let listed = listed_words(&fault.cause);
    for kind in DRAW_KINDS {
        assert!(
            listed.contains(&kind),
            "the refusal offers every draw kind the model publishes, and `{kind}` is missing \
             from it: {fault}"
        );
    }
    Ok(())
}

/// `held-block` is the only readable value this MVP publishes, and the refusal
/// is where a content author learns that. Read out of the published set for the
/// same reason the draw kinds are.
#[test]
fn a_declaration_naming_an_unpublished_source_is_refused_listing_every_published_readable_value()
-> TestResult {
    let fault = refused(with(minimal_block_texture(), "source", text("hotbar")))?;

    assert_eq!(
        fault.field.as_deref(),
        Some("source"),
        "an unpublished source is refused naming the field that stated it: {fault}"
    );
    let listed = listed_words(&fault.cause);
    for published in READABLE_VALUES {
        assert!(
            listed.contains(&published),
            "the refusal offers every readable value the model publishes, and `{published}` is \
             missing from it: {fault}"
        );
    }
    Ok(())
}
