//! Frame-size validation.
//!
//! This is a pure check that runs *before* any device work is recorded, so an
//! unusable size is a returned error rather than a driver-side panic.

mod common;

use common::TestResult;
use mc_testkit::frame::{FrameSizeError, validate_frame_size};

/// A plausible `max_texture_dimension_2d` for a desktop adapter.
const MAX_DIMENSION: u32 = 8192;

#[test]
fn a_zero_extent_is_rejected_naming_the_offending_dimension() -> TestResult {
    let zero_width = validate_frame_size(0, 64, MAX_DIMENSION)
        .err()
        .ok_or("a zero width must be rejected")?;
    assert!(
        matches!(
            &zero_width,
            FrameSizeError::ZeroDimension { dimension: "width" }
        ),
        "a zero width must name the width, got {zero_width:?}"
    );

    let zero_height = validate_frame_size(64, 0, MAX_DIMENSION)
        .err()
        .ok_or("a zero height must be rejected")?;
    assert!(
        matches!(
            &zero_height,
            FrameSizeError::ZeroDimension {
                dimension: "height"
            }
        ),
        "a zero height must name the height, got {zero_height:?}"
    );
    Ok(())
}

#[test]
fn a_width_past_the_maximum_texture_dimension_is_rejected_naming_both_numbers() -> TestResult {
    let error = validate_frame_size(9000, 64, MAX_DIMENSION)
        .err()
        .ok_or("a width above the adapter maximum must be rejected")?;
    assert!(
        matches!(
            &error,
            FrameSizeError::TooLarge {
                dimension: "width",
                requested: 9000,
                maximum: MAX_DIMENSION
            }
        ),
        "the rejection must name the requested width and the maximum, got {error:?}"
    );

    assert!(
        validate_frame_size(MAX_DIMENSION, 64, MAX_DIMENSION).is_ok(),
        "a width exactly at the maximum does not exceed it"
    );
    Ok(())
}
