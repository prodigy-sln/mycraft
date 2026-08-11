//! Comparison in a process that holds no graphics device.
//!
//! Deliberately hermetic: this file names nothing behind the `gpu` feature and
//! builds its own fixtures, so it compiles and runs in the configuration where
//! no adapter *can* exist. Its whole worth is what it links, which is why it is
//! kept apart from the rest of the comparison suite.

use mc_testkit::frame::{
    Comparison, ImageShapeError, MismatchReason, Rgba8Image, Thresholds, Verdict, compare,
};

const OPAQUE: u8 = 255;
const EDGE: u32 = 64;
const BASELINE: u8 = 128;
/// Twelve levels from the baseline: a distance of about 4.67.
const DRIFTED: u8 = 140;
const DRIFTED_PIXELS: usize = 3;

fn grey_frame(level: u8) -> Result<Rgba8Image, ImageShapeError> {
    let pixel_count = (EDGE * EDGE) as usize;
    let mut pixels = Vec::with_capacity(pixel_count * 4);
    for _ in 0..pixel_count {
        pixels.extend_from_slice(&[level, level, level, OPAQUE]);
    }
    Rgba8Image::from_rgba(EDGE, EDGE, pixels)
}

fn drifted_frame(level: u8) -> Result<Rgba8Image, ImageShapeError> {
    let mut pixels = grey_frame(BASELINE)?.as_bytes().to_vec();
    for pixel in pixels.chunks_exact_mut(4).take(DRIFTED_PIXELS) {
        pixel.copy_from_slice(&[level, level, level, OPAQUE]);
    }
    Rgba8Image::from_rgba(EDGE, EDGE, pixels)
}

#[test]
fn two_caller_supplied_frames_produce_a_verdict_with_no_device_in_the_process()
-> Result<(), Box<dyn std::error::Error>> {
    let expected = grey_frame(BASELINE)?;
    let actual = drifted_frame(DRIFTED)?;

    let comparison: Comparison = compare(&expected, &actual, &Thresholds::default());

    assert_eq!(
        comparison.verdict,
        Verdict::Mismatch(MismatchReason::AreaBudget),
        "3 of 4096 pixels is 0.073%, past the default 0.01% budget"
    );
    assert_eq!(
        comparison.failing_pixels, 3,
        "the verdict is computed, not stubbed"
    );
    Ok(())
}
