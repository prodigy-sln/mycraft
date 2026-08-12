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
#[path = "readback_test.rs"]
mod tests;
