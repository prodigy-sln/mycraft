//! The diff artifact: what a mismatch looks like when an agent opens it.
//!
//! The diff carries the expected image everywhere it agreed, so the marked
//! positions read as an overlay on the frame rather than as a bare mask.

mod common;

use common::{TestResult, grey, half_split, with_leading_pixels};
use mc_testkit::frame::{Rgba8Image, Thresholds, compare, encode_png, render_diff};

const EDGE: u32 = 64;
/// A dark left half and a light right half: "carries the expected pixel" is only
/// an assertion if the expected image is not uniform.
const SHADOW: [u8; 3] = [10, 20, 30];
const HIGHLIGHT: [u8; 3] = [200, 210, 220];
/// The leading pixels the actual image drifts on. All sit in the dark half, far
/// past any tolerance.
const FAILING_PIXELS: usize = 12;
const DRIFTED: u8 = 140;
const MAGENTA: [u8; 4] = [255, 0, 255, 255];

fn drifting_pair() -> Result<(Rgba8Image, Rgba8Image), Box<dyn std::error::Error>> {
    let expected = half_split(EDGE, EDGE, SHADOW, HIGHLIGHT)?;
    let actual = with_leading_pixels(&expected, grey(DRIFTED), FAILING_PIXELS)?;
    Ok((expected, actual))
}

/// The index of the first pixel at which two buffers of the same length differ.
fn first_differing_pixel(left: &[u8], right: &[u8]) -> Option<usize> {
    left.chunks_exact(4)
        .zip(right.chunks_exact(4))
        .position(|(left_pixel, right_pixel)| left_pixel != right_pixel)
}

#[test]
fn the_diff_marks_every_failing_position_and_carries_the_expected_pixel_elsewhere() -> TestResult {
    let (expected, actual) = drifting_pair()?;
    let comparison = compare(&expected, &actual, &Thresholds::default());

    let diff =
        render_diff(&expected, &comparison).ok_or("a same-sized pair must produce a diff")?;

    let mut required = expected.as_bytes().to_vec();
    for pixel in required.chunks_exact_mut(4).take(FAILING_PIXELS) {
        pixel.copy_from_slice(&MAGENTA);
    }
    assert_eq!(
        diff.as_bytes().len(),
        required.len(),
        "the diff covers the whole frame"
    );
    let difference = first_differing_pixel(diff.as_bytes(), &required);
    assert!(
        difference.is_none(),
        "the diff must be magenta at exactly the {FAILING_PIXELS} failing positions and the \
         expected image everywhere else, but it first differs at pixel {difference:?}"
    );
    Ok(())
}

#[test]
fn rendering_and_encoding_the_same_diff_twice_produces_identical_bytes() -> TestResult {
    let (expected, actual) = drifting_pair()?;
    let thresholds = Thresholds::default();

    let first = render_diff(&expected, &compare(&expected, &actual, &thresholds))
        .ok_or("a same-sized pair must produce a diff")?;
    let second = render_diff(&expected, &compare(&expected, &actual, &thresholds))
        .ok_or("a same-sized pair must produce a diff")?;

    assert!(
        encode_png(&first)? == encode_png(&second)?,
        "the same pair and thresholds must encode to the same bytes twice"
    );
    Ok(())
}

#[test]
fn no_diff_is_produced_for_images_of_different_sizes() -> TestResult {
    let expected = half_split(EDGE, EDGE, SHADOW, HIGHLIGHT)?;
    let wider = half_split(EDGE + 1, EDGE, SHADOW, HIGHLIGHT)?;
    let comparison = compare(&expected, &wider, &Thresholds::default());

    assert!(
        render_diff(&expected, &comparison).is_none(),
        "there is no position-by-position diff between frames of different sizes"
    );
    Ok(())
}
