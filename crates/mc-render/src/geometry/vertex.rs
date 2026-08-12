//! What one corner of a quad costs in a vertex buffer.
//!
//! A 16³ section is meant to stay cache-resident, so a vertex is a single `u64`
//! with every field cut to the width its own domain actually needs rather than
//! to the width of the Rust type it arrives in. The widths below are each
//! derived from something, and the derivation is the reason the number is what
//! it is:
//!
//! | field | bits | why that many |
//! |-------|------|---------------|
//! | x, y, z | 5 each | corners run `0..=16`, which needs 17 values |
//! | facing | 3 | six of them |
//! | texture layer | 8 | `wgpu`'s downlevel `max_texture_array_layers` is 256 |
//! | section | 10 | `MAX_SECTIONS` is 1024 |
//!
//! Thirty-six bits used of sixty-four. The spare bits are not a design margin
//! to be spent casually — ambient occlusion and per-vertex light will want them
//! — but they do mean none of the widths above had to be shaved to fit.
//!
//! **Packing refuses rather than truncates.** A coordinate of 17 masked into
//! five bits becomes 1: a corner at the far side of the section, geometrically
//! plausible, and indistinguishable at every later stage from one somebody
//! meant. There is no honest packed form for it, so there is an error instead.
//! The coordinate bound is the section's, not the field's — five bits hold 31,
//! and a corner at 20 is still a bug even though it fits.

use mc_world::mesh::Facing;
use mc_world::section::{Axis, SECTION_SIZE};
use thiserror::Error;

/// How many bits one corner coordinate occupies.
const COORDINATE_BITS: u32 = 5;

/// How many bits a facing occupies.
const FACING_BITS: u32 = 3;

/// How many bits a texture layer index occupies.
const LAYER_BITS: u32 = 8;

/// How many bits a section index occupies.
const SECTION_BITS: u32 = 10;

const X_SHIFT: u32 = 0;
const Y_SHIFT: u32 = X_SHIFT + COORDINATE_BITS;
const Z_SHIFT: u32 = Y_SHIFT + COORDINATE_BITS;
const FACING_SHIFT: u32 = Z_SHIFT + COORDINATE_BITS;
const LAYER_SHIFT: u32 = FACING_SHIFT + FACING_BITS;
const SECTION_SHIFT: u32 = LAYER_SHIFT + LAYER_BITS;

/// The last section-local corner coordinate.
///
/// One further than the last *voxel* coordinate, because the face a voxel at
/// plane 15 emits along `+X` sits at x = 16. Derived from the section's own size
/// rather than written as 16, so a section that ever changes size cannot leave a
/// stale bound behind.
const MAX_COORDINATE: u32 = SECTION_SIZE;

/// The highest texture layer index an array texture can offer on the weakest
/// declared adapter.
///
/// Public because the array texture is allocated to hold exactly this many
/// layers plus one, and a second literal on that side would be a second place
/// the packed field's width is written down.
pub const MAX_LAYER: u32 = (1 << LAYER_BITS) - 1;

/// The highest section index a scene holds. See `MAX_SECTIONS` in `scene`.
const MAX_SECTION: u32 = (1 << SECTION_BITS) - 1;

/// One corner of a quad, in the frame the vertex buffer speaks.
///
/// `local` runs `0..=16` per axis, `section` indexes the scene's section table,
/// and `layer` indexes the array texture. Nothing here is a world coordinate:
/// the world frame is reconstructed from the section's origin, which is what
/// keeps the packed and world views from drifting apart.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Vertex {
    pub local: [u8; 3],
    pub facing: Facing,
    pub layer: u16,
    pub section: u16,
}

/// A [`Vertex`] in the form the vertex buffer holds.
///
/// The inner value is private and [`pack`](Self::pack) is its only constructor,
/// which is what makes [`unpack`](Self::unpack) total: no bit pattern outside
/// the ones packing writes can exist to be decoded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PackedVertex(u64);

impl PackedVertex {
    /// Packs `vertex` into its buffer form.
    ///
    /// # Errors
    ///
    /// Returns [`PackError`] naming the field and the value whenever one is
    /// outside the range its bits can hold. Truncating instead would produce a
    /// vertex that no later stage could tell from a deliberate one.
    pub fn pack(vertex: &Vertex) -> Result<Self, PackError> {
        let [x, y, z] = vertex.local;
        let bits = coordinate(Axis::X, x)? << X_SHIFT
            | coordinate(Axis::Y, y)? << Y_SHIFT
            | coordinate(Axis::Z, z)? << Z_SHIFT
            | (vertex.facing as u64) << FACING_SHIFT
            | layer(vertex.layer)? << LAYER_SHIFT
            | section(vertex.section)? << SECTION_SHIFT;
        Ok(Self(bits))
    }

    /// The vertex this value was packed from.
    #[must_use]
    pub fn unpack(self) -> Vertex {
        // Every cast below narrows a value the mask has already bounded: five
        // bits cannot exceed a `u8`, and ten cannot exceed a `u16`.
        Vertex {
            local: [
                self.field(X_SHIFT, COORDINATE_BITS) as u8,
                self.field(Y_SHIFT, COORDINATE_BITS) as u8,
                self.field(Z_SHIFT, COORDINATE_BITS) as u8,
            ],
            facing: Facing::ALL
                .get(self.field(FACING_SHIFT, FACING_BITS) as usize)
                .copied()
                // Unreachable: `pack` is the only constructor and it writes a
                // discriminant of `Facing::ALL`, so the three spare bit patterns
                // never occur. A value rather than a panic because a panic on
                // the render path is the worse of the two failures.
                .unwrap_or(Facing::NegX),
            layer: self.field(LAYER_SHIFT, LAYER_BITS) as u16,
            section: self.field(SECTION_SHIFT, SECTION_BITS) as u16,
        }
    }

    /// The eight bytes this vertex occupies in a vertex buffer.
    ///
    /// Explicitly little-endian rather than `bytemuck::cast_slice`, which is
    /// native-endian: the byte order a buffer is uploaded in has to be a stated
    /// fact, not whatever the build host happened to be.
    #[must_use]
    pub const fn to_le_bytes(self) -> [u8; 8] {
        self.0.to_le_bytes()
    }

    /// The `width` bits starting at `shift`.
    const fn field(self, shift: u32, width: u32) -> u64 {
        (self.0 >> shift) & ((1 << width) - 1)
    }
}

/// `value` as a corner coordinate, or the reason it is not one.
///
/// The geometry builder computes corners in `u32` and stores them in the `u8`
/// [`Vertex`] carries, and the narrowing has to be the same check packing makes
/// — otherwise a plane of 200 would arrive at packing already saturated to 255
/// and the refusal would name a value nobody wrote.
///
/// # Errors
///
/// Returns [`PackError::CoordinateOutOfRange`] when `value` is past the last
/// corner a section has.
pub(crate) fn local_coordinate(axis: Axis, value: u32) -> Result<u8, PackError> {
    if value > MAX_COORDINATE {
        return Err(PackError::CoordinateOutOfRange {
            axis,
            value,
            max: MAX_COORDINATE,
        });
    }
    // Bounded by `MAX_COORDINATE` above, which is a section's size.
    Ok(value as u8)
}

/// A corner coordinate as packed bits, or the reason it has none.
fn coordinate(axis: Axis, value: u8) -> Result<u64, PackError> {
    let value = u32::from(value);
    bounded(value, MAX_COORDINATE).ok_or(PackError::CoordinateOutOfRange {
        axis,
        value,
        max: MAX_COORDINATE,
    })
}

/// A texture layer index as packed bits, or the reason it has none.
fn layer(value: u16) -> Result<u64, PackError> {
    let value = u32::from(value);
    bounded(value, MAX_LAYER).ok_or(PackError::LayerOutOfRange {
        value,
        max: MAX_LAYER,
    })
}

/// A section index as packed bits, or the reason it has none.
fn section(value: u16) -> Result<u64, PackError> {
    let value = u32::from(value);
    bounded(value, MAX_SECTION).ok_or(PackError::SectionOutOfRange {
        value,
        max: MAX_SECTION,
    })
}

/// `value` widened for packing, or `None` if it does not fit in `max`.
const fn bounded(value: u32, max: u32) -> Option<u64> {
    if value > max {
        return None;
    }
    Some(value as u64)
}

/// Why a vertex has no packed form.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum PackError {
    #[error("a corner's {axis} coordinate is {value}, and a section's corners run 0..={max}")]
    CoordinateOutOfRange { axis: Axis, value: u32, max: u32 },
    #[error("texture layer {value} is outside the 0..={max} an array texture offers")]
    LayerOutOfRange { value: u32, max: u32 },
    #[error("section index {value} is outside the 0..={max} a scene holds")]
    SectionOutOfRange { value: u32, max: u32 },
}

#[cfg(test)]
#[path = "vertex_test.rs"]
mod tests;
