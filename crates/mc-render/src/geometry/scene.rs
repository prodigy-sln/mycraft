//! Every section's geometry, laid out the way the GPU reads it.
//!
//! Assembly is the **only** capacity gate in the renderer. A `SceneGeometry`
//! over capacity cannot be constructed, so nothing downstream re-checks and
//! nothing downstream can disagree with the answer given here. The alternative
//! — a frame that draws the first 1024 sections and drops the rest — looks like
//! a world that was generated smaller rather than like a bug, which is why the
//! refusal names both the count it was handed and the capacity that count
//! exceeded.
//!
//! **Order is preserved, never imposed.** The caller hands sections in the one
//! declared order — columns `(cz, cx)` ascending, then section index ascending,
//! then the mesher's own quad order untouched — and assembly appends them in
//! exactly that sequence. There is no sort here, so there is no comparator that
//! could disagree with the order the goldens were shot under.
//!
//! Both byte views are explicitly little-endian rather than
//! `bytemuck::cast_slice`, which is native-endian: a buffer's byte order has to
//! be a stated fact rather than whatever the build host happened to be, and the
//! replay's determinism scenario compares these bytes directly.

use thiserror::Error;

use mc_world::section::SECTION_SIZE;

use crate::aabb::Aabb;

use super::SectionGeometry;
use super::vertex::{PackError, PackedVertex, Vertex};

/// How many sections the visible-set buffer addresses.
///
/// The replay uses 256 of them, so nothing in this project discovers this
/// number by running into it.
pub const MAX_SECTIONS: usize = 1024;

/// How many quads the index buffer holds.
pub const MAX_QUADS: usize = 1 << 18;

/// How many bytes one section occupies in [`SceneGeometry::section_bytes`].
const SECTION_RECORD_BYTES: usize = 44;

/// Where one section's quads sit in the scene's buffers, and the box the
/// culling pass tests.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SectionRecord {
    pub origin: [i32; 3],
    pub first_quad: u32,
    pub quad_count: u32,
    pub aabb: Aabb,
}

/// Every section's geometry, in the form the GPU consumes.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SceneGeometry {
    packed: Vec<PackedVertex>,
    sections: Vec<SectionRecord>,
}

impl SceneGeometry {
    /// Assembles `sections`, in the order given, into one scene.
    ///
    /// This is where a section first has an index, so this is where each of its
    /// corners learns which section it belongs to.
    ///
    /// # Errors
    ///
    /// Returns [`SceneError::TooManySections`] or [`SceneError::TooManyQuads`]
    /// when the scene exceeds a buffer's capacity, and [`SceneError::Pack`] when
    /// a corner has no packed form.
    pub fn assemble(sections: Vec<SectionGeometry>) -> Result<Self, SceneError> {
        let found = sections.len();
        if found > MAX_SECTIONS {
            return Err(SceneError::TooManySections {
                found,
                capacity: MAX_SECTIONS,
            });
        }
        let quads: usize = sections.iter().map(SectionGeometry::quad_count).sum();
        if quads > MAX_QUADS {
            return Err(SceneError::TooManyQuads {
                found: quads,
                capacity: MAX_QUADS,
            });
        }

        let mut scene = Self {
            packed: Vec::with_capacity(quads * CORNERS_PER_QUAD),
            sections: Vec::with_capacity(found),
        };
        for (index, section) in sections.iter().enumerate() {
            scene.append(index, section)?;
        }
        Ok(scene)
    }

    /// Appends one section's corners and its record.
    ///
    /// Every narrowing below is bounded by a check `assemble` made before any
    /// section was appended: `index` by `MAX_SECTIONS`, which fits a `u16`, and
    /// both quad figures by `MAX_QUADS`, which fits a `u32`. A corner outside
    /// its section is the one failure that remains possible here, and it reaches
    /// the caller rather than being clamped.
    fn append(&mut self, index: usize, section: &SectionGeometry) -> Result<(), SceneError> {
        let first_quad = (self.packed.len() >> 2) as u32;
        let section_index = index as u16;
        for vertex in &section.vertices {
            self.packed.push(PackedVertex::pack(&Vertex {
                section: section_index,
                ..*vertex
            })?);
        }
        self.sections.push(SectionRecord {
            origin: section.origin.world,
            first_quad,
            quad_count: section.quad_count() as u32,
            aabb: section_box(section.origin.world),
        });
        Ok(())
    }

    /// Where each section's quads sit, and the box the culling pass tests it by.
    ///
    /// In assembly order, so a section's position here is its section index —
    /// which is what the packed vertices carry and what the visible-set buffer
    /// is addressed by.
    #[must_use]
    pub fn sections(&self) -> &[SectionRecord] {
        &self.sections
    }

    /// The packed vertex buffer's bytes.
    #[must_use]
    pub fn vertex_bytes(&self) -> Vec<u8> {
        self.packed
            .iter()
            .flat_map(|vertex| vertex.to_le_bytes())
            .collect()
    }

    /// The section table's bytes: origin, first quad, quad count, then the
    /// box's minimum and maximum corner.
    ///
    /// Field order and widths are this crate's own declaration, and the compute
    /// shader's matching struct is written against it.
    #[must_use]
    pub fn section_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(self.sections.len() * SECTION_RECORD_BYTES);
        for record in &self.sections {
            bytes.extend(record.origin.iter().flat_map(|axis| axis.to_le_bytes()));
            bytes.extend(record.first_quad.to_le_bytes());
            bytes.extend(record.quad_count.to_le_bytes());
            bytes.extend(record.aabb.min.iter().flat_map(|axis| axis.to_le_bytes()));
            bytes.extend(record.aabb.max.iter().flat_map(|axis| axis.to_le_bytes()));
        }
        bytes
    }
}

/// How many corners one quad has.
const CORNERS_PER_QUAD: usize = 4;

/// The world-space box a section at `origin` occupies.
///
/// The section's own box rather than the tight bounds of its quads: the culling
/// pass tests one box per section, and a tight box would have to be recomputed
/// every time the section is remeshed.
fn section_box(origin: [i32; 3]) -> Aabb {
    let [x, y, z] = origin;
    let size = SECTION_SIZE as i32;
    Aabb {
        min: [x as f32, y as f32, z as f32],
        max: [(x + size) as f32, (y + size) as f32, (z + size) as f32],
    }
}

/// Why a scene cannot be assembled.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum SceneError {
    #[error("a scene of {found} sections exceeds the {capacity} the visible-set buffer addresses")]
    TooManySections { found: usize, capacity: usize },
    #[error("a scene of {found} quads exceeds the {capacity} the index buffer holds")]
    TooManyQuads { found: usize, capacity: usize },
    #[error(transparent)]
    Pack(#[from] PackError),
}

#[cfg(test)]
#[path = "scene_test.rs"]
mod tests;
