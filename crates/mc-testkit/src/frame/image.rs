//! The capture pixel format, and validation of a requested frame size.

use thiserror::Error;

/// Bytes per pixel in the harness's capture format.
const BYTES_PER_PIXEL: usize = 4;

/// An image in the harness's one capture format.
///
/// **Format contract:** 8-bit RGBA, sRGB-encoded, straight (non-premultiplied)
/// alpha, row 0 = top. No stage of this crate flips rows or touches alpha —
/// straight alpha is achieved by not writing the code that would scale it, and
/// the orientation is preserved by there being no flip to compensate for.
///
/// Fixing one format is what makes captures comparable across runs, and what
/// lets every consumer of this crate treat a frame as plain bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rgba8Image {
    width: u32,
    height: u32,
    pixels: Vec<u8>,
}

/// A pixel buffer that does not match the dimensions declared alongside it.
#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum ImageShapeError {
    #[error("a {width}x{height} RGBA image needs {expected} bytes of pixel data, got {actual}")]
    PixelCount {
        width: u32,
        height: u32,
        expected: usize,
        actual: usize,
    },
}

impl Rgba8Image {
    /// Builds an image from a row-major RGBA buffer.
    ///
    /// # Errors
    ///
    /// Returns [`ImageShapeError::PixelCount`] unless the buffer holds exactly
    /// `width * height * 4` bytes.
    pub fn from_rgba(width: u32, height: u32, pixels: Vec<u8>) -> Result<Self, ImageShapeError> {
        let expected = expected_byte_count(width, height);
        if expected != Some(pixels.len()) {
            return Err(ImageShapeError::PixelCount {
                width,
                height,
                expected: expected.unwrap_or(usize::MAX),
                actual: pixels.len(),
            });
        }
        Ok(Self {
            width,
            height,
            pixels,
        })
    }

    #[must_use]
    pub const fn width(&self) -> u32 {
        self.width
    }

    #[must_use]
    pub const fn height(&self) -> u32 {
        self.height
    }

    /// The pixel at `(x, y)`, with `(0, 0)` at the **top** left, or `None` when
    /// the coordinate lies outside the frame.
    #[must_use]
    pub fn pixel(&self, x: u32, y: u32) -> Option<[u8; 4]> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let offset = (y as usize)
            .checked_mul(self.width as usize)?
            .checked_add(x as usize)?
            .checked_mul(BYTES_PER_PIXEL)?;
        let pixel = self
            .pixels
            .get(offset..offset.checked_add(BYTES_PER_PIXEL)?)?;
        <[u8; BYTES_PER_PIXEL]>::try_from(pixel).ok()
    }

    /// The whole frame as row-major RGBA bytes.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.pixels
    }
}

/// A frame size that has been checked against the limits of the device that
/// will render it. Private fields: the only way to hold one is to have passed
/// [`validate_frame_size`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameSize {
    width: u32,
    height: u32,
}

impl FrameSize {
    #[must_use]
    pub const fn width(&self) -> u32 {
        self.width
    }

    #[must_use]
    pub const fn height(&self) -> u32 {
        self.height
    }
}

/// A requested frame size that no device could render.
#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum FrameSizeError {
    #[error("frame {dimension} must not be zero")]
    ZeroDimension { dimension: &'static str },
    #[error(
        "frame {dimension} {requested} exceeds the adapter's maximum 2D texture \
         dimension of {maximum}"
    )]
    TooLarge {
        dimension: &'static str,
        requested: u32,
        maximum: u32,
    },
}

/// Checks a requested capture size against an adapter's maximum 2D texture
/// dimension.
///
/// Pure by design, and called before any device work is recorded: wgpu's own
/// validation of an oversized texture is panic-shaped, and `panic!` is
/// lint-denied workspace-wide. `maximum` is a parameter rather than a device
/// query so that this decision stays testable without hardware.
///
/// The maximum is a strict bound: a dimension exactly at `maximum` is accepted.
///
/// # Errors
///
/// Returns [`FrameSizeError::ZeroDimension`] naming the zero dimension, or
/// [`FrameSizeError::TooLarge`] naming the offending dimension, the requested
/// value and the maximum.
pub const fn validate_frame_size(
    width: u32,
    height: u32,
    maximum: u32,
) -> Result<FrameSize, FrameSizeError> {
    if width == 0 {
        return Err(FrameSizeError::ZeroDimension { dimension: "width" });
    }
    if height == 0 {
        return Err(FrameSizeError::ZeroDimension {
            dimension: "height",
        });
    }
    if width > maximum {
        return Err(FrameSizeError::TooLarge {
            dimension: "width",
            requested: width,
            maximum,
        });
    }
    if height > maximum {
        return Err(FrameSizeError::TooLarge {
            dimension: "height",
            requested: height,
            maximum,
        });
    }
    Ok(FrameSize { width, height })
}

/// The byte count a `width` × `height` RGBA buffer must have, or `None` when
/// that count is not addressable on this platform.
fn expected_byte_count(width: u32, height: u32) -> Option<usize> {
    (width as usize)
        .checked_mul(height as usize)?
        .checked_mul(BYTES_PER_PIXEL)
}
