//! sRGB → CIELAB conversion and the perceptual distance between two colours.
//!
//! # The swap point
//!
//! [`delta_e`] is **the only place in this crate where a distance is computed**,
//! and it is the single point at which CIE76 could become CIEDE2000. Comparison
//! receives a scalar and never inspects `L*`, `a*`, `b*` or a raw channel, and
//! all three comparison thresholds are expressed in ΔE units, so swapping the
//! metric is a function body rather than an architecture change. Do not inline a
//! channel comparison anywhere as a fast path.
//!
//! CIE76 is chosen over CIEDE2000 because it is straight arithmetic that fits
//! the project's complexity budget, and is accurate enough at the
//! just-noticeable-difference scale this harness works at.

use std::sync::LazyLock;

/// sRGB → linear, one entry per 8-bit level.
///
/// Computed once. Beyond removing `powf` from the per-pixel loop, the table is
/// what makes conversion bit-reproducible run to run, which the diff image's
/// byte-identity guarantee leans on.
static LINEAR_FROM_SRGB8: LazyLock<[f64; 256]> = LazyLock::new(|| {
    let mut table = [0.0_f64; 256];
    for (level, entry) in table.iter_mut().enumerate() {
        let encoded = level as f64 / 255.0;
        *entry = if encoded <= 0.040_449_936 {
            encoded / 12.92
        } else {
            ((encoded + 0.055) / 1.055).powf(2.4)
        };
    }
    table
});

/// The D65 white point the sRGB colour space is defined against.
const WHITE_POINT: [f64; 3] = [0.950_489, 1.0, 1.088_840];

/// The CIE standard's `(6/29)^3`: below it the transfer function is linear.
const LAB_EPSILON: f64 = 216.0 / 24389.0;
/// The CIE standard's `(29/3)^3`, the slope of that linear segment.
const LAB_KAPPA: f64 = 24389.0 / 27.0;

/// A colour in CIELAB.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Lab {
    pub l: f64,
    pub a: f64,
    pub b: f64,
}

/// Converts an 8-bit sRGB triple to CIELAB, via linear RGB and CIE XYZ (D65).
pub(crate) fn srgb8_to_lab(rgb: [u8; 3]) -> Lab {
    let [red, green, blue] = rgb.map(|level| {
        LINEAR_FROM_SRGB8
            .get(level as usize)
            .copied()
            .unwrap_or(0.0)
    });

    // sRGB (D65) → CIE XYZ.
    let x = 0.412_456_4 * red + 0.357_576_1 * green + 0.180_437_5 * blue;
    let y = 0.212_672_9 * red + 0.715_152_2 * green + 0.072_175_0 * blue;
    let z = 0.019_333_9 * red + 0.119_192_0 * green + 0.950_304_1 * blue;

    let [white_x, white_y, white_z] = WHITE_POINT;
    let fx = lab_transfer(x / white_x);
    let fy = lab_transfer(y / white_y);
    let fz = lab_transfer(z / white_z);

    Lab {
        l: 116.0 * fy - 16.0,
        a: 500.0 * (fx - fy),
        b: 200.0 * (fy - fz),
    }
}

/// The CIE76 distance between two colours: the plain Euclidean distance in
/// CIELAB, where a value of about 1.0 is a just-noticeable difference.
///
/// **This is the metric.** Replacing it with CIEDE2000 means replacing this
/// body and nothing else.
pub(crate) fn delta_e(from: Lab, to: Lab) -> f64 {
    let lightness = from.l - to.l;
    let green_red = from.a - to.a;
    let blue_yellow = from.b - to.b;
    lightness
        .mul_add(
            lightness,
            green_red.mul_add(green_red, blue_yellow * blue_yellow),
        )
        .sqrt()
}

/// The CIE transfer function: a cube root, linearised near zero where the cube
/// root's slope would otherwise run away.
fn lab_transfer(ratio: f64) -> f64 {
    if ratio > LAB_EPSILON {
        ratio.cbrt()
    } else {
        LAB_KAPPA.mul_add(ratio, 16.0) / 116.0
    }
}

#[cfg(test)]
mod tests {
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
}
