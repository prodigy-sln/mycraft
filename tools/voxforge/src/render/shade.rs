//! What colour a face reaches the image as.
//!
//! **The factors apply to light, and light is linear.** A colour is declared in
//! sRGB, which is an encoding rather than a quantity — multiplying the encoded
//! byte darkens the wrong thing. So every shaded channel is decoded to linear,
//! scaled, and re-encoded.
//!
//! Byte 128 against byte 188 is this project's recurring trap, met twice before
//! this crate existed. For a mid grey on a `−x` face:
//!
//! ```text
//! decode(128/255) = 0.2158605   ×0.80 = 0.1726884   encode → 0.452509 → 115
//! ```
//!
//! 102 is what multiplying the byte gives, and 188 is what a decode with no
//! matching encode gives. Only the round trip gives 115.
//!
//! The factor constants are **deferred** alongside the isometric angle, to be
//! revisited together on the first real previews at eight pixels per voxel.

use crate::material::Material;
use crate::render::Pixel;

/// Which face of a voxel a ray arrived through.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Face {
    /// The face pointing along `+x` or `−x`.
    SideX,
    /// The face pointing along `+y` — the top of the voxel.
    Up,
    /// The face pointing along `−y` — its underside.
    Down,
    /// The face pointing along `+z` or `−z`.
    SideZ,
}

impl Face {
    /// How much of a material's colour this face keeps.
    ///
    /// There is no light source in this model and no lighting engine anywhere in
    /// the project — these are a fixed convention that makes a shape legible,
    /// nothing more.
    fn factor(self) -> f64 {
        match self {
            Self::Up => 1.00,
            Self::SideX => 0.80,
            Self::SideZ => 0.65,
            Self::Down => 0.50,
        }
    }
}

/// The pixel `material` produces on any face, unlit.
///
/// The **declared** colour, regardless of the material's `emissive`. Not
/// "emissive forced to 1.0 and blended": that would reach the same bytes today
/// while coupling flatness to the emissive term specifically, so the day a
/// shading term appears that is not gated on emissive — ambient occlusion is
/// already named as a future user of the packed vertex's spare bits — flat
/// would quietly stop being flat.
#[must_use]
pub fn flat(material: &Material) -> Pixel {
    let colour = material.color;
    Pixel::opaque(colour.red, colour.green, colour.blue)
}

/// The pixel `material` produces on `face`.
#[must_use]
pub fn shade(material: &Material, face: Face) -> Pixel {
    // Self-illumination lifts the face factor toward 1: a material that makes
    // all of its own light is lit by nothing, so no face of it is darker than
    // another. Written as a blend rather than a test against 1.0 because
    // comparing floats for equality is a defect in waiting, and because a
    // partially emissive material has to mean something too.
    let emissive = f64::from(material.emissive.fraction());
    let factor = face.factor().mul_add(1.0 - emissive, emissive);
    let colour = material.color;
    Pixel::opaque(
        scaled(colour.red, factor),
        scaled(colour.green, factor),
        scaled(colour.blue, factor),
    )
}

/// One channel, scaled in linear space and re-encoded.
fn scaled(channel: u8, factor: f64) -> u8 {
    let linear = decode(f64::from(channel) / 255.0) * factor;
    let encoded = encode(linear) * 255.0;
    // `round` then clamp: the encode cannot leave `0.0 ..= 1.0` for a factor in
    // that range, and the clamp is here so that a later factor above 1 would
    // saturate rather than wrap.
    encoded.round().clamp(0.0, 255.0) as u8
}

/// The linear value an sRGB-encoded channel stands for.
fn decode(encoded: f64) -> f64 {
    if encoded <= 0.040_45 {
        return encoded / 12.92;
    }
    ((encoded + 0.055) / 1.055).powf(2.4)
}

/// The sRGB encoding of a linear value.
fn encode(linear: f64) -> f64 {
    if linear <= 0.003_130_8 {
        return 12.92 * linear;
    }
    1.055 * linear.powf(1.0 / 2.4) - 0.055
}
