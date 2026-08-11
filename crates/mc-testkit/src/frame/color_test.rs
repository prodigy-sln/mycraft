//! The three reference distances the whole comparison suite is calibrated on.
//!
//! Neutral greys reduce the perceptual distance to a lightness difference, so
//! these numbers are checkable by hand and are the ones the comparison
//! thresholds were chosen against. If the conversion cannot reproduce them, the
//! thresholds mean something other than what they say — that is an escalation,
//! not a licence to adjust the expected values here.

use super::{delta_e, srgb8_to_lab};

/// The expected values below are the exact distances the spec now records. This
/// window treats a value as the same number without pinning an arithmetic
/// accident down to the last bit.
const WINDOW: f64 = 0.02;

fn neutral_grey_distance(from: u8, to: u8) -> f64 {
    delta_e(srgb8_to_lab([from; 3]), srgb8_to_lab([to; 3]))
}

fn assert_distance(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < WINDOW,
        "expected a distance of about {expected}, got {actual}"
    );
}

#[test]
fn adjacent_neutral_greys_are_a_fraction_of_a_unit_apart() {
    assert_distance(neutral_grey_distance(128, 129), 0.39168);
}

#[test]
fn a_twelve_level_neutral_grey_step_is_a_few_units() {
    assert_distance(neutral_grey_distance(128, 140), 4.66505);
}

#[test]
fn a_fifty_two_level_neutral_grey_step_is_far_past_the_hard_ceiling() {
    assert_distance(neutral_grey_distance(128, 180), 19.72703);
}
