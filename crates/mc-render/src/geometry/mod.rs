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

use mc_core::content::Face;
use mc_core::id::{BlockName, TextureKey};
use mc_world::mesh::{Facing, Quad};
use mc_world::section::Axis;
use thiserror::Error;

use crate::texture::TextureResolution;

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
///
/// **This is the geometry's table and it is not an image basis.** `placed`
/// reads it to put a quad's primary and secondary extents into world
/// components, so a row changed here *moves the mesh* — `mc_world`'s `Quad`
/// defines those two extents as the facing's other two axes in `x < y < z`
/// order and this is the mapping that honours it. Where an image's own left-to-
/// right and top-to-bottom directions go is [`IMAGE_SWAPS`] and [`IMAGE_SIGNS`],
/// which is a separate question and was the one this project got wrong. Reusing
/// one table for both is what made a rotated face look like a correct one.
pub const PLANE_AXES: [[u32; 2]; 6] = [[1, 2], [1, 2], [0, 2], [0, 2], [0, 1], [0, 1]];

/// Whether a face's image runs its own horizontal along the **secondary** of
/// [`PLANE_AXES`]' pair rather than the primary, as `1` for exchanged.
///
/// One row per facing, same order. The two `X` facings are the exchanged ones:
/// their plane pair is `(y, z)` with `y` primary, and an image standing on such
/// a face has its horizontal along `z` and its vertical along `y`. Every other
/// facing's pair already lists the horizontal first.
///
/// **Derived, not tabulated, and that is the whole of why this is right now.**
/// The shader read [`PLANE_AXES`] alone for five increments and drew five of six
/// faces wrong — east and west turned a quarter, north and south and the
/// underside flipped, only the top correct — while three hand-written copies of
/// that table agreed with each other exactly. A six-row table of conventions
/// cannot be checked by reading it, so both of these come out of
/// `image_basis`'s two lines of vector arithmetic.
pub const IMAGE_SWAPS: [u32; 6] = image_swaps();

/// Whether each of an image's two coordinates runs *against* its axis rather
/// than along it, horizontal first, `1` for negated. Same row order.
///
/// **A sign is what an axis index cannot express**, and its absence is the other
/// half of the same defect. An image's rows run downward while the world's
/// vertical axis runs up, so every face with world up in it needs its vertical
/// coordinate negated. And two faces looking at each other along one axis see
/// their in-plane axes in opposite horizontal order, so one of each pair needs
/// its horizontal negated too — without which north and south are forced to
/// share a horizontal direction and one of them draws laterally reversed.
pub const IMAGE_SIGNS: [[u32; 2]; 6] = image_signs();

/// The world directions the image's right edge and its top edge run toward, for
/// a viewer standing outside `facing` and looking at it.
///
/// **The convention, stated for all six facings rather than left to be inferred
/// from a table's values.** A viewer outside a face looks along the face's
/// inward direction with the world's up as their up, and the image's right is
/// then forward crossed with up. That fixes four of the six rows outright.
///
/// **The two horizontal facings have no world up in them, so theirs is chosen**,
/// and it is chosen to match what `voxforge` bakes rather than by preference:
/// the top image's top edge runs toward `-z` and the bottom image's toward `+z`.
/// Measured against `grass-block.mcvox`'s own outermost voxels, the shipped
/// images agree with that convention texel for texel on all six faces.
///
/// **No test discriminates the two horizontal rows today** — a top or bottom
/// texture has no world up in it, so every orientation of one looks equally
/// plausible, and `base:grass_top`'s noise is near-uniform under rotation. **The
/// bottom row was measured wrong and corrected on the strength of the bake
/// rather than of any test**, which is the clearest case there is for not
/// leaving a row to rest on nobody being able to see it. The first anisotropic
/// top or bottom texture owes a scenario.
const fn image_basis(facing: Facing) -> ([i32; 3], [i32; 3]) {
    let normal = facing.step();
    let forward = [-normal[0], -normal[1], -normal[2]];
    let up = if normal[1] == 0 {
        [0, 1, 0]
    } else {
        [0, 0, -normal[1]]
    };
    (cross(forward, up), up)
}

/// The cross product of two unit axis directions.
const fn cross(one: [i32; 3], other: [i32; 3]) -> [i32; 3] {
    [
        one[1] * other[2] - one[2] * other[1],
        one[2] * other[0] - one[0] * other[2],
        one[0] * other[1] - one[1] * other[0],
    ]
}

/// The axis index a unit `direction` lies along, and whether it points the
/// negative way down it.
const fn axis_of(direction: [i32; 3]) -> (u32, bool) {
    if direction[0] != 0 {
        (0, direction[0] < 0)
    } else if direction[1] != 0 {
        (1, direction[1] < 0)
    } else {
        (2, direction[2] < 0)
    }
}

/// [`IMAGE_SWAPS`], computed per facing.
///
/// The six facings are named rather than walked, so the declaration order these
/// rows depend on is visible in the source instead of implied by an index.
const fn image_swaps() -> [u32; 6] {
    [
        image_swap(Facing::NegX),
        image_swap(Facing::PosX),
        image_swap(Facing::NegY),
        image_swap(Facing::PosY),
        image_swap(Facing::NegZ),
        image_swap(Facing::PosZ),
    ]
}

/// Whether `facing`'s image runs its horizontal along the secondary of its plane
/// pair.
const fn image_swap(facing: Facing) -> u32 {
    let (right, _) = image_basis(facing);
    let (horizontal, _) = axis_of(right);
    let [_, secondary] = plane_axes_of(facing);
    (horizontal == secondary) as u32
}

/// [`IMAGE_SIGNS`], computed per facing.
const fn image_signs() -> [[u32; 2]; 6] {
    [
        image_sign(Facing::NegX),
        image_sign(Facing::PosX),
        image_sign(Facing::NegY),
        image_sign(Facing::PosY),
        image_sign(Facing::NegZ),
        image_sign(Facing::PosZ),
    ]
}

/// Whether each of `facing`'s image coordinates is negated.
const fn image_sign(facing: Facing) -> [u32; 2] {
    let (right, up) = image_basis(facing);
    let (_, horizontal_is_negative) = axis_of(right);
    let (_, up_is_negative) = axis_of(up);
    // An image's rows run downward, so its vertical coordinate always runs
    // against the direction its top edge points.
    [horizontal_is_negative as u32, !up_is_negative as u32]
}

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

    /// The array-texture layer the `vertex_index`-th emitted corner draws
    /// from, or `None` past the last one.
    ///
    /// **The only way to read a packed layer back from outside this crate**, and
    /// it exists so that the property "the stated assignment is honoured" can be
    /// asserted where the index actually lands rather than one step before it. A
    /// reading that asked the layer table what it holds would leave the packer
    /// free to derive an index of its own, which is exactly the failure the
    /// property is about; and decoding the bit layout at the caller would be a
    /// second copy of the packing decision, free to drift from the first.
    #[must_use]
    pub fn layer_at(&self, vertex_index: usize) -> Option<u16> {
        self.vertices.get(vertex_index).map(|vertex| vertex.layer)
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
/// Returns [`GeometryError::UnresolvedTexture`] when a quad's block draws, on
/// the facing that quad points, a key `resolution` gives no array layer — or no
/// key at all, because the content states no such block. [`GeometryError::Pack`]
/// is a corner landing outside the section. Either fails the whole section: a
/// section that emitted some of its faces is a hole in the world, and
/// substituting a fallback layer draws stone-coloured grass that nothing
/// downstream can tell from a deliberate choice.
pub fn build_section_geometry(
    quads: &[Quad],
    origin: SectionOrigin,
    resolution: &TextureResolution,
) -> Result<SectionGeometry, GeometryError> {
    let mut vertices = Vec::with_capacity(quads.len() * CORNERS_PER_QUAD as usize);
    let mut indices = Vec::with_capacity(quads.len() * QUAD_INDEX_PATTERN.len());
    let mut first_corner: u32 = 0;

    for quad in quads {
        let layer = layer_for(quad, resolution)?;
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

/// The array layer `quad`'s block draws with on the facing it points.
///
/// The facing is carried into the compass vocabulary a declaration writes, and
/// the key is read out of the declaration. Nothing here parses the block's name.
fn layer_for(quad: &Quad, resolution: &TextureResolution) -> Result<u16, GeometryError> {
    let face = quad.facing.face();
    let key = resolution.key_of(&quad.block, face);
    key.and_then(|declared| resolution.layers().layer_of(declared))
        .ok_or_else(|| GeometryError::UnresolvedTexture {
            block: quad.block.clone(),
            face,
            key: key.cloned(),
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
        "the block `{block}` draws nothing on its `{face}` face: {because}",
        block = block.as_str(),
        face = face.as_str(),
        because = unresolved_because(key.as_ref())
    )]
    UnresolvedTexture {
        block: BlockName,
        face: Face,
        key: Option<TextureKey>,
    },
    #[error(transparent)]
    Pack(#[from] PackError),
}

/// Why one facing of one block resolved to no layer.
///
/// A sentence rather than a second variant: the two cases differ only in this
/// clause, and splitting them would leave a caller matching on which of two
/// spellings of "there is nothing to draw here" it was handed.
fn unresolved_because(key: Option<&TextureKey>) -> String {
    match key {
        Some(key) => format!(
            "the key `{key}` it declares there occupies no array layer",
            key = key.as_str()
        ),
        None => "the content states no such block, so there is no key it declares there".to_owned(),
    }
}

#[cfg(test)]
#[path = "mod_test.rs"]
mod tests;
