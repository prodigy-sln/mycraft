//! Where an image sits on the face it is drawn on.
//!
//! **A separate question from where a quad's two extents go**, which is
//! [`PLANE_AXES`](super::PLANE_AXES) and lives with the geometry. Reading that
//! table alone as if it answered this one is what shipped: east and west turned
//! a quarter, north and south and the underside flipped, only the top correct.
//! Five of six faces, while three hand-written copies of one table agreed with
//! each other exactly. The split is that defect written into the module tree, so
//! the two questions cannot be reached for interchangeably.
//!
//! Both tables here are **derived** from each facing's own outward normal rather
//! than tabulated, for the reason that defect gives: a six-row table of
//! conventions cannot be checked by reading it. `build/validate.rs` compares the
//! shader's hand-written copies against these values at build time.

use mc_world::mesh::Facing;

use super::plane_axes_of;

/// Whether a face's image runs its own horizontal along the **secondary** of
/// [`PLANE_AXES`](super::PLANE_AXES)' pair rather than the primary, as `1` for exchanged.
///
/// One row per facing, same order. The two `X` facings are the exchanged ones:
/// their plane pair is `(y, z)` with `y` primary, and an image standing on such
/// a face has its horizontal along `z` and its vertical along `y`. Every other
/// facing's pair already lists the horizontal first.
///
/// **Derived, not tabulated, and that is the whole of why this is right now.**
/// The shader read [`PLANE_AXES`](super::PLANE_AXES) alone for five increments and drew five of six
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
