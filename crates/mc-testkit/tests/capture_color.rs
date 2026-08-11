//! What the captured bytes are: channel order, alpha handling, the sRGB encode,
//! and which end of the frame the caller's draw work landed on.
//!
//! Every expectation here is computed from first principles rather than read
//! from a committed image. A harness compared against a golden it produced
//! itself proves only that it is consistent; these four assert that it is
//! *right*, so a systematically wrong capture path — swapped channels,
//! premultiplied alpha, a missing sRGB encode, flipped rows — fails here rather
//! than being baked into every golden the project ever commits.

mod scene;

use scene::TestResult;

const EDGE: u32 = 64;
const PIXELS: usize = 4096;
/// A column away from either side edge, so the assertion is about rows.
const MIDDLE_COLUMN: u32 = 32;
/// The last row of the frame, the far side of the split from row 0.
const LAST_ROW: u32 = EDGE - 1;

/// Full intensity in a colour channel, whatever the alpha beside it.
const FULL_CHANNEL: u8 = 255;
/// What the mid-tone clear must encode to.
const MID_GREY: u8 = 128;
/// Absorbs the rounding difference between backends, and nothing more. A
/// channel outside this window is cross-adapter drift worth reporting, not a
/// tolerance worth widening.
const ENCODING_TOLERANCE: u8 = 1;

#[test]
fn a_frame_cleared_to_opaque_red_comes_back_opaque_red() -> TestResult {
    let context = scene::device_context()?;
    let request = scene::request(&context, "clear-red", EDGE, EDGE)?;
    let mut draw = scene::clear(scene::OPAQUE_RED);

    let image = context.capture(&request, &mut draw)?.image;

    let red = image
        .as_bytes()
        .chunks_exact(scene::BYTES_PER_PIXEL)
        .filter(|pixel| *pixel == scene::OPAQUE_RED_BYTES.as_slice())
        .count();
    assert_eq!(
        red, PIXELS,
        "every pixel of the frame must be opaque red, in that channel order"
    );
    Ok(())
}

#[test]
fn a_fill_of_the_top_half_comes_back_at_the_top_of_the_frame() -> TestResult {
    let context = scene::device_context()?;
    let request = scene::request(&context, "top-half-fill", EDGE, EDGE)?;
    let mut draw = scene::top_half_white_over_black(context.device());

    let image = context.capture(&request, &mut draw)?.image;

    assert_eq!(
        image
            .pixel(MIDDLE_COLUMN, 0)
            .ok_or("the capture is missing its first row")?,
        scene::OPAQUE_WHITE_BYTES,
        "the first row must carry the fill the caller drew across the top half"
    );
    assert_eq!(
        image
            .pixel(MIDDLE_COLUMN, LAST_ROW)
            .ok_or("the capture is missing its last row")?,
        scene::OPAQUE_BLACK_BYTES,
        "the last row must still be the clear underneath that fill"
    );
    Ok(())
}

#[test]
fn a_quarter_alpha_clear_leaves_the_colour_channels_unscaled() -> TestResult {
    let context = scene::device_context()?;
    let request = scene::request(&context, "quarter-alpha", EDGE, EDGE)?;
    let mut draw = scene::clear(scene::WHITE_AT_QUARTER_ALPHA);

    let image = context.capture(&request, &mut draw)?.image;

    let scaled = scene::channels_away_from(image.as_bytes(), FULL_CHANNEL, 0);
    assert_eq!(
        scaled, 0,
        "alpha is straight, so every colour channel stays at {FULL_CHANNEL}; \
         {scaled} of them were scaled by it"
    );
    Ok(())
}

#[test]
fn a_mid_tone_clear_comes_back_srgb_encoded() -> TestResult {
    let context = scene::device_context()?;
    let request = scene::request(&context, "mid-tone", EDGE, EDGE)?;
    // A *linear* clear. The hardware performs the encode, which is why the
    // target is an sRGB format and why no CPU stage touches these bytes.
    let mut draw = scene::clear(scene::LINEAR_MID_GREY);

    let image = context.capture(&request, &mut draw)?.image;

    let off_target = scene::channels_away_from(image.as_bytes(), MID_GREY, ENCODING_TOLERANCE);
    assert_eq!(
        off_target, 0,
        "every colour channel must encode to within {ENCODING_TOLERANCE} of \
         {MID_GREY}; {off_target} of them did not"
    );
    Ok(())
}
