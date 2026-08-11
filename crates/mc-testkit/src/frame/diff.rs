//! The diff image: what a mismatch looks like when someone opens the artifact.
//!
//! Failing positions are marked in opaque magenta and every other position
//! carries the **expected** image's pixel, so the marks read as an overlay on
//! the frame that was supposed to be produced rather than as a bare mask.

use super::compare::{Comparison, FailingMask};
use super::image::Rgba8Image;

/// The mark. Chosen because nothing in a rendered frame is likely to be it.
const MAGENTA: [u8; 4] = [255, 0, 255, 255];

/// Bytes per pixel in the harness's capture format.
const BYTES_PER_PIXEL: usize = 4;

/// Renders `comparison`'s failing positions over `expected`.
///
/// Returns `None` when the two compared frames were different sizes: there is
/// no position-by-position diff between frames that do not share positions.
/// That reason is recorded in the mismatch report instead.
///
/// Deterministic: the same expected frame and comparison produce the same bytes
/// every time.
#[must_use]
pub fn render_diff(expected: &Rgba8Image, comparison: &Comparison) -> Option<Rgba8Image> {
    let mask = comparison.failing_mask.as_ref()?;
    if mask.width() != expected.width() || mask.height() != expected.height() {
        return None;
    }

    let row_bytes = (expected.width() as usize).checked_mul(BYTES_PER_PIXEL)?;
    let mut pixels = expected.as_bytes().to_vec();
    if row_bytes > 0 {
        // Row-major chunks rather than an index divided by the width: integer
        // division is lint-denied, and the chunking says the same thing.
        for (row_index, row) in pixels.chunks_exact_mut(row_bytes).enumerate() {
            mark_failing_pixels(row, u32::try_from(row_index).unwrap_or(u32::MAX), mask);
        }
    }

    Rgba8Image::from_rgba(expected.width(), expected.height(), pixels).ok()
}

/// Marks every failing pixel of one row.
fn mark_failing_pixels(row: &mut [u8], row_index: u32, mask: &FailingMask) {
    for (column, pixel) in row.chunks_exact_mut(BYTES_PER_PIXEL).enumerate() {
        if mask.is_failing(u32::try_from(column).unwrap_or(u32::MAX), row_index) {
            pixel.copy_from_slice(&MAGENTA);
        }
    }
}
