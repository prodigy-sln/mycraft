//! Perceptual comparison of two frames against three thresholds.
//!
//! A pixel fails when its distance is **greater than** the per-pixel tolerance;
//! a pair fails the budget when the failing share is **greater than** the
//! budget, and the ceiling when any distance is **greater than** the ceiling.
//! All three are strictly-greater-than, so a value sitting exactly on a
//! threshold passes — which also keeps this module free of any float equality.
//!
//! Comparison is a pure function of the two frames and the thresholds. It never
//! touches a device, and its only reductions — a count and a maximum — are
//! order-independent, so the verdict is deterministic.

use thiserror::Error;

use super::color::{delta_e, srgb8_to_lab};
use super::image::Rgba8Image;

/// Bytes per pixel in the harness's capture format.
const BYTES_PER_PIXEL: usize = 4;

/// The three thresholds a comparison is judged against, all in ΔE units except
/// the area budget, which is a fraction of the frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Thresholds {
    per_pixel_delta_e: f64,
    max_failing_fraction: f64,
    hard_ceiling_delta_e: f64,
}

/// A threshold that is not a usable distance.
#[derive(Debug, Error, Clone, Copy, PartialEq)]
pub enum ThresholdError {
    #[error("threshold `{field}` must be a finite, non-negative number, got {value}")]
    Invalid { field: &'static str, value: f64 },
}

impl Thresholds {
    /// Builds a threshold set.
    ///
    /// # Errors
    ///
    /// Returns [`ThresholdError::Invalid`] naming the field and the rejected
    /// value when any threshold is negative, infinite or NaN.
    pub fn new(
        per_pixel_delta_e: f64,
        max_failing_fraction: f64,
        hard_ceiling_delta_e: f64,
    ) -> Result<Self, ThresholdError> {
        check("per_pixel_delta_e", per_pixel_delta_e)?;
        check("max_failing_fraction", max_failing_fraction)?;
        check("hard_ceiling_delta_e", hard_ceiling_delta_e)?;
        Ok(Self {
            per_pixel_delta_e,
            max_failing_fraction,
            hard_ceiling_delta_e,
        })
    }

    #[must_use]
    pub const fn per_pixel_delta_e(&self) -> f64 {
        self.per_pixel_delta_e
    }

    #[must_use]
    pub const fn max_failing_fraction(&self) -> f64 {
        self.max_failing_fraction
    }

    #[must_use]
    pub const fn hard_ceiling_delta_e(&self) -> f64 {
        self.hard_ceiling_delta_e
    }
}

impl Default for Thresholds {
    /// ΔE 2.0 per pixel, a 0.01% area budget and a ΔE 10.0 hard ceiling.
    ///
    /// The budget is deliberately an order of magnitude tighter than the 0.1%
    /// an anti-aliasing-heavy renderer would want: at 0.1% the budget at 720p
    /// exceeds the pixel count of one block face, so it could forgive an
    /// entirely wrong face. Loosening it is a per-comparison override that
    /// records its reason — never a change here.
    fn default() -> Self {
        Self {
            per_pixel_delta_e: 2.0,
            max_failing_fraction: 0.0001,
            hard_ceiling_delta_e: 10.0,
        }
    }
}

/// Why a pair of frames did not match.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MismatchReason {
    /// More of the frame drifted than the area budget allows.
    AreaBudget,
    /// One pixel drifted far enough to be a defect on its own, whatever the
    /// failing share was.
    HardCeiling,
    /// The frames are different sizes, so no pixels were compared.
    Dimensions {
        expected: (u32, u32),
        actual: (u32, u32),
    },
}

/// The outcome of a comparison.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Match,
    Mismatch(MismatchReason),
}

/// Which positions exceeded the per-pixel tolerance. Row-major, one entry per
/// pixel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FailingMask {
    width: u32,
    height: u32,
    failing: Vec<bool>,
}

impl FailingMask {
    #[must_use]
    pub const fn width(&self) -> u32 {
        self.width
    }

    #[must_use]
    pub const fn height(&self) -> u32 {
        self.height
    }

    /// Whether the pixel at `(x, y)` exceeded the per-pixel tolerance. Positions
    /// outside the frame never failed, because they were never compared.
    #[must_use]
    pub fn is_failing(&self, x: u32, y: u32) -> bool {
        if x >= self.width || y >= self.height {
            return false;
        }
        let offset = (y as usize)
            .checked_mul(self.width as usize)
            .and_then(|row| row.checked_add(x as usize));
        offset
            .and_then(|offset| self.failing.get(offset))
            .copied()
            .unwrap_or(false)
    }
}

/// Everything a comparison concluded, including the thresholds it applied.
#[derive(Debug, Clone, PartialEq)]
pub struct Comparison {
    pub verdict: Verdict,
    pub failing_pixels: u64,
    pub total_pixels: u64,
    pub failing_fraction: f64,
    pub max_delta_e: f64,
    pub thresholds: Thresholds,
    /// `None` if and only if the two frames were different sizes.
    pub failing_mask: Option<FailingMask>,
}

/// Compares two frames pixel by pixel in CIELAB.
///
/// **This never fails.** A dimension difference is a *mismatch* naming both
/// sizes, not an error: a resized frame is a real regression, and returning an
/// error would blur that with "could not compare".
#[must_use]
pub fn compare(expected: &Rgba8Image, actual: &Rgba8Image, thresholds: &Thresholds) -> Comparison {
    if expected.width() != actual.width() || expected.height() != actual.height() {
        return dimension_mismatch(expected, actual, thresholds);
    }

    let tally = tally_pixels(expected, actual, thresholds.per_pixel_delta_e());
    let total_pixels = tally.failing.len() as u64;
    let failing_pixels = tally.failing_pixels;
    let failing_fraction = if total_pixels == 0 {
        0.0
    } else {
        failing_pixels as f64 / total_pixels as f64
    };

    Comparison {
        verdict: verdict_for(failing_fraction, tally.max_delta_e, thresholds),
        failing_pixels,
        total_pixels,
        failing_fraction,
        max_delta_e: tally.max_delta_e,
        thresholds: *thresholds,
        failing_mask: Some(FailingMask {
            width: expected.width(),
            height: expected.height(),
            failing: tally.failing,
        }),
    }
}

/// What a per-pixel sweep found.
struct Tally {
    failing: Vec<bool>,
    failing_pixels: u64,
    max_delta_e: f64,
}

/// Walks both frames once, recording which pixels drifted past `tolerance` and
/// how far the worst one drifted.
fn tally_pixels(expected: &Rgba8Image, actual: &Rgba8Image, tolerance: f64) -> Tally {
    let pixel_pairs = expected
        .as_bytes()
        .chunks_exact(BYTES_PER_PIXEL)
        .zip(actual.as_bytes().chunks_exact(BYTES_PER_PIXEL));

    let mut failing = Vec::with_capacity(pixel_pairs.len());
    let mut failing_pixels = 0_u64;
    let mut max_delta_e = 0.0_f64;

    for (left, right) in pixel_pairs {
        let distance = pixel_distance(left, right);
        max_delta_e = max_delta_e.max(distance);
        let over_tolerance = distance > tolerance;
        if over_tolerance {
            failing_pixels = failing_pixels.saturating_add(1);
        }
        failing.push(over_tolerance);
    }

    Tally {
        failing,
        failing_pixels,
        max_delta_e,
    }
}

/// The perceptual distance between two RGBA pixels.
///
/// Alpha is not part of the metric: ΔE is defined over RGB, and alpha is
/// asserted on the capture side instead.
fn pixel_distance(left: &[u8], right: &[u8]) -> f64 {
    match (left, right) {
        ([left_red, left_green, left_blue, _], [right_red, right_green, right_blue, _]) => delta_e(
            srgb8_to_lab([*left_red, *left_green, *left_blue]),
            srgb8_to_lab([*right_red, *right_green, *right_blue]),
        ),
        _ => 0.0,
    }
}

/// The ceiling is checked first: a single pixel that far off is a defect
/// whatever the failing share was, and letting the budget answer first would
/// hide a small but severe error.
fn verdict_for(failing_fraction: f64, max_delta_e: f64, thresholds: &Thresholds) -> Verdict {
    if max_delta_e > thresholds.hard_ceiling_delta_e() {
        return Verdict::Mismatch(MismatchReason::HardCeiling);
    }
    if failing_fraction > thresholds.max_failing_fraction() {
        return Verdict::Mismatch(MismatchReason::AreaBudget);
    }
    Verdict::Match
}

/// The verdict for two frames of different sizes. No pixels are compared, so
/// every pixel statistic is zero and there is no mask.
fn dimension_mismatch(
    expected: &Rgba8Image,
    actual: &Rgba8Image,
    thresholds: &Thresholds,
) -> Comparison {
    Comparison {
        verdict: Verdict::Mismatch(MismatchReason::Dimensions {
            expected: (expected.width(), expected.height()),
            actual: (actual.width(), actual.height()),
        }),
        failing_pixels: 0,
        total_pixels: 0,
        failing_fraction: 0.0,
        max_delta_e: 0.0,
        thresholds: *thresholds,
        failing_mask: None,
    }
}

/// Rejects a threshold that is not a usable distance.
fn check(field: &'static str, value: f64) -> Result<(), ThresholdError> {
    if !value.is_finite() || value < 0.0 {
        return Err(ThresholdError::Invalid { field, value });
    }
    Ok(())
}
