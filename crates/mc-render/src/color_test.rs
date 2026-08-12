//! The sRGB transfer function, which is wrong in a direction nothing else can
//! see.
//!
//! The colour target is sRGB-encoded and `wgpu` clear values are linear, so the
//! declared clear colour has to be decoded before it is handed to a pass. Get
//! that decode wrong — apply a plain 2.2 gamma, apply the encode instead of the
//! decode, or skip it entirely — and the goldens and the derived probes move
//! **together**: the frame is cleared to the wrong colour, the golden records
//! that colour, and every probe comparing "the declared clear colour" against a
//! frame that was cleared with the same wrong function agrees with itself. There
//! is no end-to-end assertion that can catch it, which is why this one is
//! against arithmetic done by hand.
//!
//! The expectations below are computed from the sRGB EOTF as written in the
//! standard,
//!
//! ```text
//! c = channel / 255
//! linear = c / 12.92                        for c <= 0.04045
//! linear = ((c + 0.055) / 1.055) ^ 2.4      otherwise
//! ```
//!
//! and **not** from any run of this crate. Both segments are sampled, because
//! the piecewise join is where a transfer function is usually got half right:
//! 10 is the largest channel value in the linear toe (0.04045 * 255 = 10.31) and
//! 128 sits in the middle of the power segment. Both endpoints are sampled too —
//! a decode that is only wrong in scale still maps 0 to 0, and a decode that is
//! only wrong in shape still maps 255 to 1.
//!
//! `clippy::float_cmp` is denied, so every check below is a bound on the
//! deviation rather than an equality.

use super::srgb8_to_linear;

/// How far a converted channel may sit from the hand-computed value.
///
/// Six orders of magnitude tighter than the difference between the correct
/// function and any of the plausible wrong ones — the toe and the power segment
/// differ by 2.3% at their join, and a 2.2-gamma decode differs from the correct
/// one by more than 1% across the middle of the range.
const TOLERANCE: f64 = 1e-6;

/// Channel values and the linear value the standard's own formula gives them.
///
/// - 0: both segments agree, and the answer is exactly 0.
/// - 10: the last value in the toe. `10 / (255 * 12.92) = 0.003 035 269 8`.
/// - 128: the power segment. `((128 / 255 + 0.055) / 1.055) ^ 2.4 = 0.215 860 5`.
/// - 255: the top of the range, which the power segment maps to exactly 1.
const SAMPLES: [(u8, f64); 4] = [
    (0, 0.0),
    (10, 0.003_035_269_8),
    (128, 0.215_860_5),
    (255, 1.0),
];

#[test]
fn an_srgb_channel_decodes_through_both_segments_of_the_standards_transfer_function() {
    let mut adrift = Vec::new();

    for (channel, expected) in SAMPLES {
        // The same value in all three channels, so a conversion that decodes
        // one channel and copies the other two is caught here rather than in a
        // frame nobody can look at.
        adrift.extend(
            srgb8_to_linear([channel, channel, channel])
                .into_iter()
                .enumerate()
                .filter(|&(_, converted)| (converted - expected).abs() > TOLERANCE)
                .map(|(position, converted)| (channel, position, converted, expected)),
        );
    }

    assert!(
        adrift.is_empty(),
        "every channel must decode to the value the sRGB EOTF gives it, within {TOLERANCE}; \
         these did not, as (channel, position, converted, expected): {adrift:?}"
    );
}
