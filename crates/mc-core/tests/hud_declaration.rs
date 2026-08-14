//! What a HUD declaration has to say, and what it may leave unsaid.
//!
//! Presence, kind, range and the element name. Every refusal here is graded on
//! *which field it names*, never on the bare fact that it refused: "the load
//! failed" is the one thing every one of these fixtures has in common, so a test
//! asserting only that would pass for fifteen different reasons.
//!
//! The name is read before anything else is checked, which is what lets a
//! declaration whose `size` holds a string still be refused *by name*. A test
//! that accepted an anonymous refusal there would let that ordering be lost
//! silently.

mod common;

use common::TestResult;
use common::hud::{
    FIXTURE_NAME, FIXTURE_ORIGIN, extents, integer, minimal_fill, refused, registered, text, with,
    without,
};
use mc_core::hud::DeclaredValue;

#[test]
fn a_declaration_stating_neither_offset_nor_outline_registers_with_neither() -> TestResult {
    let element = registered(minimal_fill())?;

    assert_eq!(
        (element.offset, element.outline),
        ([0, 0], None),
        "a declaration silent about its offset and its outline means no displacement and no \
         outline, not an unregistered element"
    );
    Ok(())
}

#[test]
fn a_declaration_omitting_its_size_is_refused_naming_the_origin_the_element_and_that_field()
-> TestResult {
    let fault = refused(without(minimal_fill(), "size"))?;

    assert_eq!(
        (
            fault.origin.as_str(),
            fault.element.as_deref(),
            fault.field.as_deref(),
        ),
        (FIXTURE_ORIGIN, Some(FIXTURE_NAME), Some("size")),
        "a missing size is refused naming where it was declared, which element it was, and which \
         field is at fault: {fault}"
    );
    Ok(())
}

/// The accepted field set is the check, so this is what reports a model that
/// grew a field nobody declared acceptable — including one by which a
/// declaration could name something the engine owns.
#[test]
fn a_declaration_stating_a_field_the_model_does_not_accept_is_refused_naming_that_field()
-> TestResult {
    let fault = refused(with(minimal_fill(), "wobble", DeclaredValue::Boolean(true)))?;

    assert_eq!(
        fault.field.as_deref(),
        Some("wobble"),
        "a field outside the accepted set is refused by its own name, rather than ignored: {fault}"
    );
    Ok(())
}

#[test]
fn a_declaration_whose_size_has_a_zero_extent_is_refused_naming_the_size_field() -> TestResult {
    let fault = refused(with(minimal_fill(), "size", extents(0, 4)))?;

    assert_eq!(
        (fault.element.as_deref(), fault.field.as_deref()),
        (Some(FIXTURE_NAME), Some("size")),
        "both extents are strictly positive, and a zero one is refused naming the element and \
         the field: {fault}"
    );
    Ok(())
}

/// The element is named even though the field that is wrong is not the name —
/// which only holds if the name is read before anything else is checked.
#[test]
fn a_declaration_whose_size_holds_a_string_extent_is_refused_naming_the_element_and_that_field()
-> TestResult {
    let fault = refused(with(
        minimal_fill(),
        "size",
        DeclaredValue::List(vec![text("9"), integer(1)]),
    ))?;

    assert_eq!(
        (fault.element.as_deref(), fault.field.as_deref()),
        (Some(FIXTURE_NAME), Some("size")),
        "an extent of the wrong kind still names the element it belongs to: {fault}"
    );
    Ok(())
}

/// The sibling of the `size` scenario above, and not redundant with it.
///
/// A checker that read a malformed pair as `[0, 0]` rather than refusing it is
/// still caught on `size` — by the strictly-positive extent rule sitting behind
/// it, not by the wrong-kind check itself. Nothing sits behind `offset`, so the
/// same defect there registers an element at a displacement nobody declared,
/// with no fault at all. This is the test that reports it.
#[test]
fn a_declaration_whose_offset_holds_a_string_displacement_is_refused_naming_the_element_and_that_field()
-> TestResult {
    let fault = refused(with(
        minimal_fill(),
        "offset",
        DeclaredValue::List(vec![text("9"), integer(1)]),
    ))?;

    assert_eq!(
        (fault.element.as_deref(), fault.field.as_deref()),
        (Some(FIXTURE_NAME), Some("offset")),
        "a displacement of the wrong kind is refused naming the element and that field, rather \
         than silently displacing the element by nothing: {fault}"
    );
    Ok(())
}

#[test]
fn a_declaration_omitting_its_name_is_refused_naming_the_origin_and_the_name_field() -> TestResult {
    let fault = refused(without(minimal_fill(), "name"))?;

    assert_eq!(
        (fault.origin.as_str(), fault.field.as_deref()),
        (FIXTURE_ORIGIN, Some("name")),
        "an unnamed declaration can only be identified by where it was declared, so the refusal \
         names that and the field: {fault}"
    );
    Ok(())
}

#[test]
fn a_declaration_named_with_one_namespace_separator_registers_under_that_name() -> TestResult {
    let element = registered(minimal_fill())?;

    assert_eq!(
        element.name.as_str(),
        FIXTURE_NAME,
        "a namespaced name registers the element under exactly the name it was written as"
    );
    Ok(())
}

#[test]
fn a_declaration_whose_name_carries_no_namespace_is_refused_naming_the_name_field() -> TestResult {
    let fault = refused(with(minimal_fill(), "name", text("crosshair")))?;

    assert_eq!(
        fault.field.as_deref(),
        Some("name"),
        "an element name follows the namespaced-id rule, and a bare word is refused naming the \
         field that broke it: {fault}"
    );
    Ok(())
}

#[test]
fn a_declaration_whose_name_carries_a_second_separator_is_refused_naming_the_separator()
-> TestResult {
    let fault = refused(with(minimal_fill(), "name", text("base:hud:crosshair")))?;

    assert_eq!(
        fault.field.as_deref(),
        Some("name"),
        "a second separator is refused naming the field: {fault}"
    );
    assert!(
        fault.cause.contains("separator"),
        "the refusal says what was wrong with the name — a second separator — rather than only \
         that it was wrong: {fault}"
    );
    Ok(())
}
