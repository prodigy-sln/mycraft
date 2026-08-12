//! Row-padding arithmetic for texture-to-buffer copies.
//!
//! A GPU texture copy aligns every destination row to 256 bytes, so a frame
//! whose width defeats that alignment comes back with filler on each row that
//! is not part of the image. Both halves of that arithmetic live here, as pure
//! functions over plain bytes, so the first device-backed capture is testing
//! the device rather than the maths.

use thiserror::Error;

/// `wgpu::COPY_BYTES_PER_ROW_ALIGNMENT`. Stated as a plain constant so this
/// module carries no dependency on the GPU layer.
///
/// The `expect` here and on the three items below states the seam as a compile
/// condition: everything the capture path needs from this module is reachable
/// from the GPU layer or from a test, and from nowhere else. With the feature
/// off there is genuinely no caller — and `expect` rather than `allow` means
/// that if one ever appears in the core, the annotation becomes a warning
/// rather than rotting quietly.
#[cfg_attr(all(not(test), not(feature = "gpu")), expect(dead_code))]
const COPY_ROW_ALIGNMENT: u32 = 256;

/// Bytes per pixel in the harness's capture format.
#[cfg_attr(all(not(test), not(feature = "gpu")), expect(dead_code))]
pub(crate) const BYTES_PER_PIXEL: u32 = 4;

/// A readback buffer that cannot be interpreted as the frame it claims to hold.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ReadbackError {
    #[error("a frame {width} pixels wide needs a padded row longer than the addressable range")]
    RowTooWide { width: u32 },
    /// The device never handed the frame over.
    ///
    /// The cause is a string rather than a driver error type because this enum
    /// sits on the GPU-free side of the seam: it is an *input* to
    /// [`super::clock::poll_until_deadline`], which must stay compilable with no
    /// wgpu in the dependency graph.
    #[error("the device did not hand over the captured frame: {cause}")]
    DeviceLost { cause: String },
    #[error("a padded row of {padded_row_bytes} bytes cannot carry a row of {row_bytes} bytes")]
    RowLayout {
        row_bytes: usize,
        padded_row_bytes: usize,
    },
    #[error(
        "a readback buffer of {actual} bytes cannot hold {height} rows of \
         {padded_row_bytes} bytes"
    )]
    ShortBuffer {
        actual: usize,
        height: u32,
        padded_row_bytes: usize,
    },
}

/// The row stride a texture-to-buffer copy of a `width`-pixel frame must use.
///
/// `next_multiple_of` rather than `(x + 255) / 256 * 256`: `integer_division`
/// is lint-denied workspace-wide, and the rounding is the whole point of the
/// function.
///
/// # Errors
///
/// Returns [`ReadbackError::RowTooWide`] when the padded stride does not fit in
/// a `u32`, which is the only reason this is fallible.
#[cfg_attr(all(not(test), not(feature = "gpu")), expect(dead_code))]
pub(crate) fn padded_row_bytes(width: u32) -> Result<u32, ReadbackError> {
    width
        .checked_mul(BYTES_PER_PIXEL)
        .and_then(|row| row.checked_next_multiple_of(COPY_ROW_ALIGNMENT))
        .ok_or(ReadbackError::RowTooWide { width })
}

/// Strips the copy alignment's filler, turning a padded readback buffer into a
/// tightly packed row-major frame.
///
/// # Errors
///
/// Returns [`ReadbackError::RowLayout`] if a padded row could not carry a
/// content row, or [`ReadbackError::ShortBuffer`] if `padded` is too small to
/// hold `height` padded rows.
pub fn unpad_rows(
    padded: &[u8],
    row_bytes: usize,
    padded_row_bytes: usize,
    height: u32,
) -> Result<Vec<u8>, ReadbackError> {
    if padded_row_bytes == 0 || row_bytes > padded_row_bytes {
        return Err(ReadbackError::RowLayout {
            row_bytes,
            padded_row_bytes,
        });
    }

    let rows = height as usize;
    let required = padded_row_bytes
        .checked_mul(rows)
        .ok_or(ReadbackError::ShortBuffer {
            actual: padded.len(),
            height,
            padded_row_bytes,
        })?;
    if padded.len() < required {
        return Err(ReadbackError::ShortBuffer {
            actual: padded.len(),
            height,
            padded_row_bytes,
        });
    }

    let mut unpadded = Vec::with_capacity(row_bytes.saturating_mul(rows));
    for row in padded.chunks_exact(padded_row_bytes).take(rows) {
        let content = row.get(..row_bytes).ok_or(ReadbackError::RowLayout {
            row_bytes,
            padded_row_bytes,
        })?;
        unpadded.extend_from_slice(content);
    }
    Ok(unpadded)
}

#[cfg(test)]
mod tests {
    //! Row-padding arithmetic for texture-to-buffer copies.
    //!
    //! A copy's rows are aligned to 256 bytes, so a frame whose width defeats that
    //! alignment comes back with filler on every row. 257 × 129 is the shape that
    //! defeats it in both directions: 1028 content bytes padded to 1280, 129 times
    //! over. Proving this here means the first device-backed capture is testing the
    //! device, not the arithmetic.

    use super::{ReadbackError, padded_row_bytes, unpad_rows};

    const WIDTH: u32 = 257;
    const HEIGHT: u32 = 129;
    const ROW_BYTES: usize = 1028;
    const PADDED_ROW_BYTES: usize = 1280;
    /// A byte that must never survive into the unpadded output.
    const FILLER: u8 = 0xAB;

    /// One marker byte per row, so a dropped, duplicated or reordered row is
    /// visible in the output rather than hidden by uniform content.
    fn row_marker(row: u32) -> u8 {
        u8::try_from(row % 251).unwrap_or(0)
    }

    fn padded_rows() -> Vec<u8> {
        let mut buffer = Vec::with_capacity(PADDED_ROW_BYTES * HEIGHT as usize);
        for row in 0..HEIGHT {
            buffer.extend(std::iter::repeat_n(row_marker(row), ROW_BYTES));
            buffer.extend(std::iter::repeat_n(FILLER, PADDED_ROW_BYTES - ROW_BYTES));
        }
        buffer
    }

    fn expected_rows() -> Vec<u8> {
        let mut buffer = Vec::with_capacity(ROW_BYTES * HEIGHT as usize);
        for row in 0..HEIGHT {
            buffer.extend(std::iter::repeat_n(row_marker(row), ROW_BYTES));
        }
        buffer
    }

    #[test]
    fn a_row_that_defeats_the_copy_alignment_is_padded_up_to_it() -> Result<(), ReadbackError> {
        assert_eq!(
            padded_row_bytes(WIDTH)?,
            1280,
            "1028 content bytes round up to the next 256-byte multiple"
        );
        Ok(())
    }

    #[test]
    fn a_row_that_already_fills_the_copy_alignment_is_left_alone() -> Result<(), ReadbackError> {
        assert_eq!(
            padded_row_bytes(64)?,
            256,
            "256 content bytes are already aligned and gain no padding"
        );
        Ok(())
    }

    #[test]
    fn a_width_whose_row_cannot_be_addressed_is_rejected() {
        assert!(
            padded_row_bytes(u32::MAX).is_err(),
            "a row of 4 × u32::MAX bytes has no representable padded length"
        );
    }

    #[test]
    fn unpadding_strips_the_filler_from_every_row() -> Result<(), ReadbackError> {
        let unpadded = unpad_rows(&padded_rows(), ROW_BYTES, PADDED_ROW_BYTES, HEIGHT)?;

        assert_eq!(
            unpadded.len(),
            ROW_BYTES * HEIGHT as usize,
            "the output holds exactly the content bytes of every row"
        );
        assert!(
            unpadded == expected_rows(),
            "every row must come back in order with its filler removed"
        );
        Ok(())
    }

    #[test]
    fn a_buffer_shorter_than_its_padded_rows_is_rejected() {
        let truncated = vec![0_u8; PADDED_ROW_BYTES * (HEIGHT as usize - 1)];

        assert!(
            unpad_rows(&truncated, ROW_BYTES, PADDED_ROW_BYTES, HEIGHT).is_err(),
            "a buffer that cannot hold every padded row is not a frame"
        );
    }
}
