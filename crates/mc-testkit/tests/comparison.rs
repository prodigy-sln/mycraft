//! Perceptual comparison: per-pixel tolerance, area budget and hard ceiling.
//!
//! Every fixture is a neutral grey, where the perceptual distance reduces to a
//! lightness step and the expected numbers can be stated up front:
//!
//! | step | distance |
//! |------|----------|
//! | 128 → 129 | ≈ 0.39 |
//! | 128 → 140 | ≈ 4.67 |
//! | 128 → 180 | ≈ 19.73 |
//!
//! The area-budget fixtures are 320 × 180 (57 600 pixels) because a hundredth
//! of a percent of a 64 × 64 image is 0.41 pixels — a boundary no single pixel
//! can sit either side of.

mod common;

use common::{TestResult, assert_near, grey, uniform, with_leading_pixels};
use mc_testkit::frame::{MismatchReason, ThresholdError, Thresholds, Verdict, compare};

/// The grey every fixture drifts away from.
const BASELINE: u8 = 128;
/// One level away from the baseline: a distance of about 0.39.
const ONE_LEVEL: u8 = 129;
/// Twelve levels away: about 4.67, over the default tolerance, under the ceiling.
const TWELVE_LEVELS: u8 = 140;
/// Fifty-two levels away: about 19.73, well over the ceiling.
const FIFTY_TWO_LEVELS: u8 = 180;

/// The spec states its distances to two decimals; this window is "the same
/// number" without pinning an arithmetic accident.
const DISTANCE_WINDOW: f64 = 0.02;
/// Thresholds are stored, not computed, so they come back exactly.
const EXACT: f64 = 1e-12;

#[test]
fn a_one_level_grey_drift_stays_inside_the_per_pixel_tolerance() -> TestResult {
    let expected = uniform(64, 64, grey(BASELINE))?;
    let actual = with_leading_pixels(&expected, grey(ONE_LEVEL), 3)?;

    let comparison = compare(&expected, &actual, &Thresholds::new(2.0, 0.0001, 10.0)?);

    assert_eq!(
        comparison.verdict,
        Verdict::Match,
        "a drift of about 0.39 is under a tolerance of 2.0"
    );
    Ok(())
}

#[test]
fn a_twelve_level_grey_drift_counts_those_pixels_as_failing() -> TestResult {
    let expected = uniform(64, 64, grey(BASELINE))?;
    let actual = with_leading_pixels(&expected, grey(TWELVE_LEVELS), 3)?;

    let comparison = compare(&expected, &actual, &Thresholds::new(2.0, 0.0001, 10.0)?);

    assert_eq!(
        comparison.failing_pixels, 3,
        "a drift of about 4.67 is over a tolerance of 2.0"
    );
    Ok(())
}

#[test]
fn a_failing_share_inside_the_area_budget_is_a_match_that_still_counts_the_pixels() -> TestResult {
    let expected = uniform(320, 180, grey(BASELINE))?;
    let actual = with_leading_pixels(&expected, grey(TWELVE_LEVELS), 5)?;

    let comparison = compare(&expected, &actual, &Thresholds::new(2.0, 0.0001, 10.0)?);

    assert_eq!(
        comparison.verdict,
        Verdict::Match,
        "5 of 57 600 pixels is 0.0087%, inside a 0.01% budget"
    );
    assert_eq!(comparison.failing_pixels, 5, "the count is reported anyway");
    Ok(())
}

#[test]
fn a_failing_share_past_the_area_budget_is_a_mismatch_stating_the_count_and_the_budget()
-> TestResult {
    let expected = uniform(320, 180, grey(BASELINE))?;
    let actual = with_leading_pixels(&expected, grey(TWELVE_LEVELS), 6)?;

    let comparison = compare(&expected, &actual, &Thresholds::new(2.0, 0.0001, 10.0)?);

    assert_eq!(
        comparison.verdict,
        Verdict::Mismatch(MismatchReason::AreaBudget),
        "6 of 57 600 pixels is 0.0104%, past a 0.01% budget"
    );
    assert_eq!(comparison.failing_pixels, 6, "the count is stated");
    assert_near(comparison.thresholds.max_failing_fraction(), 0.0001, EXACT);
    Ok(())
}

#[test]
fn the_largest_distance_under_the_hard_ceiling_is_reported_with_the_match() -> TestResult {
    let expected = uniform(320, 180, grey(BASELINE))?;
    let actual = with_leading_pixels(&expected, grey(TWELVE_LEVELS), 5)?;

    let comparison = compare(&expected, &actual, &Thresholds::new(2.0, 0.0001, 10.0)?);

    assert_eq!(
        comparison.verdict,
        Verdict::Match,
        "the worst pixel is about 4.67, under a ceiling of 10.0"
    );
    assert_near(comparison.max_delta_e, 4.67, DISTANCE_WINDOW);
    Ok(())
}

#[test]
fn a_single_pixel_past_the_hard_ceiling_fails_from_inside_the_area_budget() -> TestResult {
    let expected = uniform(320, 180, grey(BASELINE))?;
    let actual = with_leading_pixels(&expected, grey(FIFTY_TWO_LEVELS), 1)?;

    let comparison = compare(&expected, &actual, &Thresholds::new(2.0, 0.0001, 10.0)?);

    assert_eq!(
        comparison.verdict,
        Verdict::Mismatch(MismatchReason::HardCeiling),
        "1 of 57 600 pixels is 0.0017% — inside the budget, but about 19.73 is past the ceiling"
    );
    Ok(())
}

#[test]
fn swapping_the_two_images_leaves_the_verdict_count_and_maximum_unchanged() -> TestResult {
    let expected = uniform(320, 180, grey(BASELINE))?;
    let actual = with_leading_pixels(&expected, grey(TWELVE_LEVELS), 6)?;
    let thresholds = Thresholds::new(2.0, 0.0001, 10.0)?;

    let forward = compare(&expected, &actual, &thresholds);
    let swapped = compare(&actual, &expected, &thresholds);

    assert_eq!(forward.verdict, swapped.verdict, "the verdict is symmetric");
    assert_eq!(
        forward.failing_pixels, swapped.failing_pixels,
        "the failing-pixel count is symmetric"
    );
    assert_near(forward.max_delta_e, swapped.max_delta_e, EXACT);
    Ok(())
}

#[test]
fn images_of_different_sizes_are_a_mismatch_naming_both_sizes() -> TestResult {
    let expected = uniform(64, 64, grey(BASELINE))?;
    let actual = uniform(65, 64, grey(BASELINE))?;

    let comparison = compare(&expected, &actual, &Thresholds::default());

    assert_eq!(
        comparison.verdict,
        Verdict::Mismatch(MismatchReason::Dimensions {
            expected: (64, 64),
            actual: (65, 64)
        }),
        "a resized frame is a regression, not a failure to compare"
    );
    assert_eq!(comparison.total_pixels, 0, "no pixels were compared");
    assert!(
        comparison.failing_mask.is_none(),
        "there is no mask when nothing was compared"
    );
    Ok(())
}

#[test]
fn a_comparison_without_explicit_thresholds_applies_the_documented_defaults() -> TestResult {
    let expected = uniform(64, 64, grey(BASELINE))?;
    let actual = with_leading_pixels(&expected, grey(TWELVE_LEVELS), 3)?;

    let comparison = compare(&expected, &actual, &Thresholds::default());

    assert_near(comparison.thresholds.per_pixel_delta_e(), 2.0, EXACT);
    assert_near(comparison.thresholds.max_failing_fraction(), 0.0001, EXACT);
    assert_near(comparison.thresholds.hard_ceiling_delta_e(), 10.0, EXACT);
    Ok(())
}

#[test]
fn a_tightened_per_pixel_tolerance_catches_a_one_level_grey_drift() -> TestResult {
    let expected = uniform(64, 64, grey(BASELINE))?;
    let actual = with_leading_pixels(&expected, grey(ONE_LEVEL), 3)?;

    let comparison = compare(&expected, &actual, &Thresholds::new(0.2, 0.0001, 10.0)?);

    assert_eq!(
        comparison.failing_pixels, 3,
        "a drift of about 0.39 is over a tolerance of 0.2"
    );
    Ok(())
}

#[test]
fn a_negative_per_pixel_tolerance_is_rejected_naming_the_value() -> TestResult {
    let error = Thresholds::new(-1.0, 0.0001, 10.0)
        .err()
        .ok_or("a negative per-pixel tolerance must be rejected")?;

    assert!(
        matches!(
            &error,
            ThresholdError::Invalid { field: "per_pixel_delta_e", value }
                if (value + 1.0).abs() < EXACT
        ),
        "the rejection must name the field and the value it rejected, got {error:?}"
    );
    Ok(())
}
