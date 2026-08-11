//! Writing a frame to disk as a PNG, and reading one back.
//!
//! The encoder's compression and filter settings are pinned explicitly rather
//! than left to the `image` crate's defaults, so the byte-identity the diff
//! artifact guarantees does not silently depend on an upstream default.
//!
//! **No stage here flips rows.** PNG row 0 is the top of the image and so is
//! [`Rgba8Image`]'s, so the correct amount of reorientation is none. A
//! compensating pair of flips on write and read would cancel in a round trip
//! while leaving the file upside-down for whoever opens it.

use std::fs;
use std::path::{Path, PathBuf};

use image::codecs::png::{CompressionType, FilterType, PngEncoder};
use image::{ExtendedColorType, ImageEncoder, ImageFormat};
use thiserror::Error;

use super::image::{ImageShapeError, Rgba8Image};

/// Pinned, not defaulted: byte-identity must not depend on an upstream choice.
const COMPRESSION: CompressionType = CompressionType::Best;
const FILTER: FilterType = FilterType::Adaptive;

/// A PNG that could not be produced, written or read.
#[derive(Debug, Error)]
pub enum ImageIoError {
    #[error("could not create the directory `{path}` to hold a PNG")]
    Directory {
        path: PathBuf,
        #[source]
        cause: std::io::Error,
    },
    #[error("could not write the PNG `{path}`")]
    Write {
        path: PathBuf,
        #[source]
        cause: std::io::Error,
    },
    #[error("could not read `{path}`")]
    Read {
        path: PathBuf,
        #[source]
        cause: std::io::Error,
    },
    #[error("could not decode `{path}` as a PNG")]
    Decode {
        path: PathBuf,
        #[source]
        cause: image::ImageError,
    },
    #[error("could not encode a {width}x{height} frame as a PNG")]
    Encode {
        width: u32,
        height: u32,
        #[source]
        cause: image::ImageError,
    },
    #[error("`{path}` decoded to a buffer that is not a whole number of RGBA pixels")]
    DecodedShape {
        path: PathBuf,
        #[source]
        cause: ImageShapeError,
    },
}

/// Encodes a frame as PNG bytes.
///
/// Deterministic: the same frame encodes to the same bytes every time.
///
/// # Errors
///
/// Returns [`ImageIoError::Encode`] if the encoder rejects the frame.
pub fn encode_png(frame: &Rgba8Image) -> Result<Vec<u8>, ImageIoError> {
    let mut encoded = Vec::new();
    PngEncoder::new_with_quality(&mut encoded, COMPRESSION, FILTER)
        .write_image(
            frame.as_bytes(),
            frame.width(),
            frame.height(),
            ExtendedColorType::Rgba8,
        )
        .map_err(|cause| ImageIoError::Encode {
            width: frame.width(),
            height: frame.height(),
            cause,
        })?;
    Ok(encoded)
}

/// Writes a frame to `path` as a PNG, creating the parent directory if it does
/// not exist.
///
/// # Errors
///
/// Returns [`ImageIoError::Directory`] if the parent directory does not exist
/// and cannot be created, [`ImageIoError::Encode`] if encoding fails, or
/// [`ImageIoError::Write`] if the file cannot be written.
pub fn write_png(frame: &Rgba8Image, path: &Path) -> Result<(), ImageIoError> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent).map_err(|cause| ImageIoError::Directory {
            path: parent.to_path_buf(),
            cause,
        })?;
    }

    let encoded = encode_png(frame)?;
    fs::write(path, encoded).map_err(|cause| ImageIoError::Write {
        path: path.to_path_buf(),
        cause,
    })
}

/// Reads a PNG from `path` into the harness's capture format.
///
/// # Errors
///
/// Returns [`ImageIoError::Read`] if the file cannot be read,
/// [`ImageIoError::Decode`] if its contents are not a decodable PNG, or
/// [`ImageIoError::DecodedShape`] if the decoded buffer does not match the
/// dimensions the file declares.
pub fn read_png(path: &Path) -> Result<Rgba8Image, ImageIoError> {
    let encoded = fs::read(path).map_err(|cause| ImageIoError::Read {
        path: path.to_path_buf(),
        cause,
    })?;
    let decoded =
        image::load_from_memory_with_format(&encoded, ImageFormat::Png).map_err(|cause| {
            ImageIoError::Decode {
                path: path.to_path_buf(),
                cause,
            }
        })?;

    let rgba = decoded.to_rgba8();
    let (width, height) = (rgba.width(), rgba.height());
    Rgba8Image::from_rgba(width, height, rgba.into_raw()).map_err(|cause| {
        ImageIoError::DecodedShape {
            path: path.to_path_buf(),
            cause,
        }
    })
}
