//! Quads become drawable geometry: corners, triangles and outward winding.
//!
//! A quad names the *voxel* that emitted the face, never the face itself, so a
//! face pointing at higher coordinates sits one step past its plane and a face
//! pointing at lower coordinates sits exactly on it. That single asymmetry is
//! where a whole class of plausible-looking bugs lives — a build that offsets
//! every facing puts the world's top surfaces exactly where they belong and its
//! bottom surfaces one block low, which reads as a lighting artefact rather than
//! as an arithmetic error.
//!
//! **Winding is where the picture disappears rather than degrades.** A quad
//! wound the wrong way round is culled entirely, so the index pattern is
//! deliberately facing-independent — one six-element constant, not six — and
//! every facing's winding lives in the order its four corners are emitted. The
//! rule for that order is derived here rather than tabulated: the corner
//! sequence `(p₀,s₀) (p₁,s₀) (p₁,s₁) (p₀,s₁)` has the normal
//! `primary × secondary`, and with the plane axes taken in `x < y < z` order
//! that cross product points along the positive third axis for `X` and `Z`
//! faces and along the negative one for `Y` faces, because `(X, Z)` is the one
//! anti-cyclic pair of the three. Where it disagrees with the facing, the order
//! is reversed.
//!
//! If culling ever turns out inverted, the fix is the pipeline's `front_face`
//! and never the corner order here: re-winding would make the picture right
//! while breaking the property that says it is right.

pub mod scene;
pub mod vertex;

use mc_core::id::{BlockName, TextureKey};
use mc_world::mesh::{Facing, Quad};
use mc_world::section::Axis;
use thiserror::Error;

use crate::texture::TextureLayers;

use vertex::{PackError, Vertex};

/// The six indices four corners are drawn as: two triangles sharing a diagonal.
///
/// Facing-independent by construction, which is what lets the cull shader hold
/// the same six numbers without also holding a facing. `build/validate.rs`
/// asserts the shader's copy still matches this one at build time — the one
/// duplication the storage-binding budget forces, closed mechanically rather
/// than by review.
pub const QUAD_INDEX_PATTERN: [u32; 6] = [0, 1, 2, 0, 2, 3];

/// Which two components of a corner's section-local position a face's plane
/// coordinates are written into: the primary first, then the secondary.
///
/// One row per **facing**, in `Facing`'s own declaration order, rather than one
/// row per axis. A packed vertex carries that discriminant, so the terrain
/// shader reads the same table with no arithmetic of its own — deriving the axis
/// as `facing >> 1` would be a second copy of the declaration order, and a
/// reordering of the enum would move four of the six rows while every answer
/// computed from `Facing::axis()` stayed correct. `build/validate.rs` asserts
/// the shader's copy still matches this one at build time.
///
/// The drift this closes is invisible to every other assertion: a face whose two
/// plane axes are exchanged still draws, still shows its own texture and still
/// averages to the same colour. It runs that texture *across* the face instead
/// of along it, which no mean colour and no golden shot from the drifted
/// renderer can report.
pub const PLANE_AXES: [[u32; 2]; 6] = [[1, 2], [1, 2], [0, 2], [0, 2], [0, 1], [0, 1]];

/// How many corners one quad has.
const CORNERS_PER_QUAD: u32 = 4;

/// Where a section sits in the world.
///
/// A section's corners are stored section-locally and this is the only thing
/// that turns them into world positions, so the two views cannot drift apart.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SectionOrigin {
    world: [i32; 3],
}

impl SectionOrigin {
    /// The origin at `world`, in world-space block coordinates.
    #[must_use]
    pub const fn new(world: [i32; 3]) -> Self {
        Self { world }
    }
}

/// One section's quads, as corners and the triangles over them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SectionGeometry {
    origin: SectionOrigin,
    vertices: Vec<Vertex>,
    indices: Vec<u32>,
}

impl SectionGeometry {
    /// The world-frame position of the `vertex_index`-th emitted corner, or
    /// `None` past the last one.
    ///
    /// Derived from the section origin on every call rather than stored: a
    /// second, world-space copy of a corner is a second thing that can be wrong.
    #[must_use]
    pub fn world_corner(&self, vertex_index: usize) -> Option<[f32; 3]> {
        let vertex = self.vertices.get(vertex_index)?;
        let [origin_x, origin_y, origin_z] = self.origin.world;
        let [local_x, local_y, local_z] = vertex.local;
        Some([
            (origin_x + i32::from(local_x)) as f32,
            (origin_y + i32::from(local_y)) as f32,
            (origin_z + i32::from(local_z)) as f32,
        ])
    }

    /// How many quads this section emitted.
    #[must_use]
    pub const fn quad_count(&self) -> usize {
        // A shift rather than a division: `clippy::integer_division` is a gate
        // error, and four corners per quad is a power of two anyway.
        self.vertices.len() >> 2
    }
}

/// Converts `quads` into the corners and triangles that draw them, in the
/// section at `origin`.
///
/// # Errors
///
/// Returns [`GeometryError::UnresolvedTexture`] when a quad names a block for
/// which `layers` resolved no array layer, and [`GeometryError::Pack`] when a
/// corner lands outside the section. Either fails the whole section: a section
/// that emitted some of its faces is a hole in the world, and substituting a
/// fallback layer draws stone-coloured grass that nothing downstream can tell
/// from a deliberate choice.
pub fn build_section_geometry(
    quads: &[Quad],
    origin: SectionOrigin,
    layers: &TextureLayers,
) -> Result<SectionGeometry, GeometryError> {
    let mut vertices = Vec::with_capacity(quads.len() * CORNERS_PER_QUAD as usize);
    let mut indices = Vec::with_capacity(quads.len() * QUAD_INDEX_PATTERN.len());
    let mut first_corner: u32 = 0;

    for quad in quads {
        let layer = layer_for(quad, layers)?;
        for local in corners(quad)? {
            vertices.push(Vertex {
                local,
                facing: quad.facing,
                layer,
                // Assigned when the scene is assembled, which is the first point
                // at which a section has an index.
                section: 0,
            });
        }
        indices.extend(QUAD_INDEX_PATTERN.map(|offset| first_corner + offset));
        first_corner += CORNERS_PER_QUAD;
    }

    Ok(SectionGeometry {
        origin,
        vertices,
        indices,
    })
}

/// The array layer `quad`'s block draws with.
fn layer_for(quad: &Quad, layers: &TextureLayers) -> Result<u16, GeometryError> {
    // A block's texture key is its own name: placeholder textures are generated
    // per key, and the shipped content declares the two identically. Both are
    // namespaced ids with one grammar, so the parse cannot fail for a name that
    // already parsed — and if it somehow did, "this block has no resolved layer"
    // is the true answer either way, which is why the two cases fold together.
    TextureKey::parse(quad.block.as_str())
        .ok()
        .and_then(|key| layers.layer_of(&key))
        .ok_or_else(|| GeometryError::UnresolvedTexture {
            block: quad.block.clone(),
        })
}

/// The four corners of `quad`, in the order that winds its triangles outward.
fn corners(quad: &Quad) -> Result<[[u8; 3]; 4], PackError> {
    let axis = quad.facing.axis();
    let positive = points_at_higher_coordinates(quad.facing);
    // A face pointing at higher coordinates sits one step past the voxel that
    // emitted it; one pointing the other way sits on the voxel's own plane.
    let along = if positive { quad.plane + 1 } else { quad.plane };

    let first_primary = quad.origin.primary;
    let first_secondary = quad.origin.secondary;
    let last_primary = first_primary + quad.extent.primary;
    let last_secondary = first_secondary + quad.extent.secondary;
    let corner = |primary, secondary| {
        narrowed(placed(
            quad.facing,
            FaceCoordinates {
                along,
                primary,
                secondary,
            },
        ))
    };

    let (a, b, c, d) = (
        corner(first_primary, first_secondary)?,
        corner(last_primary, first_secondary)?,
        corner(last_primary, last_secondary)?,
        corner(first_primary, last_secondary)?,
    );
    Ok(if base_order_winds_outward(axis, positive) {
        [a, b, c, d]
    } else {
        [a, d, c, b]
    })
}

/// Whether `facing` points towards higher coordinates on its own axis.
///
/// The one fact about a facing this crate authors for itself. `mc-world` keeps
/// its own copy private, and the alternative — a six-row table of axes and signs
/// — is the shape whose wrong row nobody notices. This is checked exhaustively
/// by the winding scenario, which builds a quad for each of the six facings and
/// recomputes its normal from the corners actually emitted.
const fn points_at_higher_coordinates(facing: Facing) -> bool {
    matches!(facing, Facing::PosX | Facing::PosY | Facing::PosZ)
}

/// Whether the primary-then-secondary corner order already winds outward.
///
/// That order's normal is `primary × secondary`. With the plane axes taken in
/// `x < y < z` order the pairs are `(Y, Z)`, `(X, Z)` and `(X, Y)`, of which
/// only `(X, Z)` is anti-cyclic — so the base order points along the positive
/// third axis except on `Y` faces, where it points along the negative one.
const fn base_order_winds_outward(axis: Axis, positive: bool) -> bool {
    let base_points_at_higher_coordinates = !matches!(axis, Axis::Y);
    base_points_at_higher_coordinates == positive
}

/// Where a corner sits in the frame of its own face.
#[derive(Debug, Clone, Copy)]
struct FaceCoordinates {
    /// How far along the facing's own axis the face's plane sits.
    along: u32,
    /// Where in that plane the corner sits, on the primary axis and then on the
    /// secondary one.
    primary: u32,
    secondary: u32,
}

/// A corner of a `facing` face, as a section-local position.
///
/// Driven by [`PLANE_AXES`] rather than by a match of its own: the facing's row
/// names the two components the plane coordinates are written into, and the
/// third keeps `along`, because the component the row does not name is the
/// facing's own axis. That is what makes the constant the thing corners are
/// placed by, so the shader reading the same table is reading the same fact
/// rather than a description of it.
///
/// The plane axes are the two that are not the facing's own, in `x < y < z`
/// order, which is the convention `base_order_winds_outward` is derived under.
const fn placed(facing: Facing, at: FaceCoordinates) -> [u32; 3] {
    let [primary_axis, secondary_axis] = plane_axes_of(facing);
    let along_every_axis = [at.along; 3];
    let with_primary = with_component(along_every_axis, primary_axis, at.primary);
    with_component(with_primary, secondary_axis, at.secondary)
}

/// `facing`'s own row of [`PLANE_AXES`].
///
/// The constant is destructured and matched rather than indexed by the
/// discriminant: an exhaustive match cannot read the wrong row for a facing
/// nobody updated, and adding a seventh facing becomes a compile error here
/// instead of a row silently missing from the table.
const fn plane_axes_of(facing: Facing) -> [u32; 2] {
    let [neg_x, pos_x, neg_y, pos_y, neg_z, pos_z] = PLANE_AXES;
    match facing {
        Facing::NegX => neg_x,
        Facing::PosX => pos_x,
        Facing::NegY => neg_y,
        Facing::PosY => pos_y,
        Facing::NegZ => neg_z,
        Facing::PosZ => pos_z,
    }
}

/// `local` with its `component`th coordinate replaced by `value`.
const fn with_component(local: [u32; 3], component: u32, value: u32) -> [u32; 3] {
    let [x, y, z] = local;
    match component {
        0 => [value, y, z],
        1 => [x, value, z],
        _ => [x, y, value],
    }
}

/// A corner narrowed to the width a vertex carries.
fn narrowed(local: [u32; 3]) -> Result<[u8; 3], PackError> {
    let [x, y, z] = local;
    Ok([
        vertex::local_coordinate(Axis::X, x)?,
        vertex::local_coordinate(Axis::Y, y)?,
        vertex::local_coordinate(Axis::Z, z)?,
    ])
}

/// Why a section has no drawable geometry.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum GeometryError {
    #[error(
        "the block `{block}` names a texture for which no array layer was resolved",
        block = block.as_str()
    )]
    UnresolvedTexture { block: BlockName },
    #[error(transparent)]
    Pack(#[from] PackError),
}

#[cfg(test)]
#[path = "mod_test.rs"]
mod tests;
