//! Capturing a frame with no window: the geometry that comes back, and what
//! the harness reports about the readback that produced it.
//!
//! The process running these tests has no window, no surface and no display
//! server — a plain test binary is exactly the context an SSH session or a
//! service gives, which is what "headless" means here.

mod scene;

use std::time::Duration;

use scene::TestResult;

/// A frame the size of a single pixel: the smallest capture that can be asked
/// for, and the one that proves the whole path runs at all.
const SMALLEST_FRAME: u32 = 1;

/// Both dimensions defeat the 256-byte row alignment wgpu requires of a
/// texture-to-buffer copy: 257 pixels is 1028 bytes of content in a 1280-byte
/// padded row, and 129 rows is an odd count of them.
const UNALIGNED_WIDTH: u32 = 257;
const UNALIGNED_HEIGHT: u32 = 129;
const UNALIGNED_PIXELS: usize = 33_153;

const EDGE: u32 = 64;

#[test]
fn a_headless_context_captures_a_single_pixel_frame() -> TestResult {
    let context = scene::device_context()?;
    let request = scene::request(&context, "smallest-frame", SMALLEST_FRAME, SMALLEST_FRAME)?;
    let mut draw = scene::clear(scene::OPAQUE_RED);

    let capture = context.capture(&request, &mut draw)?;

    assert_eq!(
        scene::pixel_count(capture.image.as_bytes()),
        1,
        "a 1x1 capture must come back as exactly one pixel"
    );
    Ok(())
}

#[test]
fn a_frame_whose_rows_defeat_the_copy_alignment_comes_back_unpadded() -> TestResult {
    let context = scene::device_context()?;
    let request = scene::request(
        &context,
        "unaligned-rows",
        UNALIGNED_WIDTH,
        UNALIGNED_HEIGHT,
    )?;
    let mut draw = scene::clear(scene::OPAQUE_RED);

    let image = context.capture(&request, &mut draw)?.image;

    assert_eq!(
        (image.width(), image.height()),
        (UNALIGNED_WIDTH, UNALIGNED_HEIGHT),
        "the capture must have exactly the requested dimensions"
    );
    assert_eq!(
        scene::pixel_count(image.as_bytes()),
        UNALIGNED_PIXELS,
        "the frame must carry {UNALIGNED_WIDTH} pixels per row and none of the copy padding"
    );
    Ok(())
}

#[test]
fn a_completed_capture_reports_how_long_its_readback_took() -> TestResult {
    let context = scene::device_context()?;
    let request = scene::request(&context, "readback-elapsed", EDGE, EDGE)?;
    let mut draw = scene::clear(scene::OPAQUE_RED);

    let capture = context.capture(&request, &mut draw)?;

    assert_eq!(
        (capture.image.width(), capture.image.height()),
        (EDGE, EDGE),
        "a completed capture must return its image"
    );
    // Not a timing assertion — capture speed is Out of Scope. What is asserted
    // is that the duration was measured at all: a zero readback is a field
    // nobody filled in.
    assert!(
        capture.readback > Duration::ZERO,
        "the elapsed readback time must be measured, got {:?}",
        capture.readback
    );
    Ok(())
}
