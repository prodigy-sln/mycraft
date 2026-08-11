//! Image fixtures shared by the harness's behavioural tests.
//!
//! Every helper here builds an `Rgba8Image` by hand. That is the seam working:
//! the core of this crate consumes plain values, so no test in this suite needs
//! a graphics device to produce its inputs.

// Each test binary links this whole module and uses a subset of it.
#![allow(dead_code)]

use mc_testkit::frame::{ImageShapeError, Rgba8Image};

/// The error type every test in this suite propagates with `?`.
pub type TestResult = Result<(), Box<dyn std::error::Error>>;

/// Alpha of a fully opaque pixel.
pub const OPAQUE: u8 = 255;

/// A neutral grey, where the perceptual distance reduces to a lightness step.
#[must_use]
pub fn grey(level: u8) -> [u8; 3] {
    [level; 3]
}

/// A `width` × `height` image whose every pixel is the opaque colour `rgb`.
///
/// # Errors
///
/// Returns [`ImageShapeError`] if the built buffer does not match the declared
/// dimensions.
pub fn uniform(width: u32, height: u32, rgb: [u8; 3]) -> Result<Rgba8Image, ImageShapeError> {
    let [red, green, blue] = rgb;
    let pixel_count = width as usize * height as usize;
    let mut pixels = Vec::with_capacity(pixel_count * 4);
    for _ in 0..pixel_count {
        pixels.extend_from_slice(&[red, green, blue, OPAQUE]);
    }
    Rgba8Image::from_rgba(width, height, pixels)
}

/// `base` with its first `count` pixels, in row-major order, recoloured to the
/// opaque colour `rgb`.
///
/// # Errors
///
/// Returns [`ImageShapeError`] if the rebuilt buffer does not match `base`'s
/// dimensions.
pub fn with_leading_pixels(
    base: &Rgba8Image,
    rgb: [u8; 3],
    count: usize,
) -> Result<Rgba8Image, ImageShapeError> {
    let [red, green, blue] = rgb;
    let mut pixels = base.as_bytes().to_vec();
    for pixel in pixels.chunks_exact_mut(4).take(count) {
        pixel.copy_from_slice(&[red, green, blue, OPAQUE]);
    }
    Rgba8Image::from_rgba(base.width(), base.height(), pixels)
}

/// One row of opaque pixels, all the same colour.
fn solid_row(width: u32, rgb: [u8; 3]) -> Vec<u8> {
    let [red, green, blue] = rgb;
    let mut row = Vec::with_capacity(width as usize * 4);
    for _ in 0..width {
        row.extend_from_slice(&[red, green, blue, OPAQUE]);
    }
    row
}

/// An opaque image split down the middle **by column**: `left` on the left,
/// `right` on the right. Vertically symmetric — a row-order flip leaves it
/// unchanged, so never use it to assert row order.
///
/// # Errors
///
/// Returns [`ImageShapeError`] if the built buffer does not match the declared
/// dimensions.
pub fn split_by_column(
    width: u32,
    height: u32,
    left: [u8; 3],
    right: [u8; 3],
) -> Result<Rgba8Image, ImageShapeError> {
    // `integer_division` is lint-denied workspace-wide; `midpoint` is the halving
    // that survives it.
    let midpoint = u32::midpoint(0, width);
    let mut row = Vec::with_capacity(width as usize * 4);
    for column in 0..width {
        let [red, green, blue] = if column < midpoint { left } else { right };
        row.extend_from_slice(&[red, green, blue, OPAQUE]);
    }
    let mut pixels = Vec::with_capacity(row.len() * height as usize);
    for _ in 0..height {
        pixels.extend_from_slice(&row);
    }
    Rgba8Image::from_rgba(width, height, pixels)
}

/// An opaque image split across the middle **by row**: `top` above, `bottom`
/// below. PNG is row-ordered, so this is the fixture that can tell a
/// right-way-up file from an inverted one.
///
/// # Errors
///
/// Returns [`ImageShapeError`] if the built buffer does not match the declared
/// dimensions.
pub fn split_by_row(
    width: u32,
    height: u32,
    top: [u8; 3],
    bottom: [u8; 3],
) -> Result<Rgba8Image, ImageShapeError> {
    let midpoint = u32::midpoint(0, height);
    let top_row = solid_row(width, top);
    let bottom_row = solid_row(width, bottom);
    let mut pixels = Vec::with_capacity(top_row.len() * height as usize);
    for row in 0..height {
        pixels.extend_from_slice(if row < midpoint {
            &top_row
        } else {
            &bottom_row
        });
    }
    Rgba8Image::from_rgba(width, height, pixels)
}

/// Asserts two floating-point values agree within `tolerance`.
///
/// `float_cmp` is lint-denied, and every quantity these tests compare is the
/// result of arithmetic, so equality is always a windowed comparison.
pub fn assert_near(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        (actual - expected).abs() < tolerance,
        "expected {expected} within {tolerance}, got {actual}"
    );
}
