//! What may name a capture.
//!
//! One identifier fills three roles — the timeout error, the golden directory
//! and the artifact directory — and two of those turn it into a path segment.
//! That makes validation input validation on a path-forming public input, not
//! decoration.

mod common;

use common::TestResult;
use mc_testkit::frame::{CaptureId, CaptureIdError};

#[test]
fn a_lowercase_name_with_digits_and_separators_is_accepted() -> TestResult {
    let capture = CaptureId::new("clear-red_64")?;

    assert_eq!(
        capture.as_str(),
        "clear-red_64",
        "an accepted name is carried through unchanged"
    );
    Ok(())
}

#[test]
fn a_nameless_capture_is_rejected() {
    assert!(
        matches!(CaptureId::new(""), Err(CaptureIdError::Empty)),
        "an empty segment would put a capture's files directly in the root"
    );
}

#[test]
fn a_name_carrying_a_path_separator_is_rejected_naming_the_character() -> TestResult {
    let error = CaptureId::new("goldens/clear-red")
        .err()
        .ok_or("a name spanning two path segments must not be accepted")?;

    let CaptureIdError::IllegalCharacter { character, .. } = error else {
        return Err(format!("expected an illegal-character rejection, got {error:?}").into());
    };
    assert_eq!(
        character, '/',
        "the rejection names the character it refused"
    );
    Ok(())
}

#[test]
fn a_parent_directory_reference_is_rejected() {
    assert!(
        CaptureId::new("..").is_err(),
        "`..` must never survive into a path the harness writes to"
    );
}

#[test]
fn an_uppercase_name_is_rejected() {
    assert!(
        CaptureId::new("ClearRed").is_err(),
        "the identifier alphabet is lowercase, so one capture cannot collide \
         with another on a case-insensitive filesystem"
    );
}
