//! Everything the build script holds a **second copy** of.
//!
//! A build script cannot depend on the package it builds, so every value the
//! shaders are checked against has to be written again here. That is the whole
//! risk this file is: a copy agrees with itself forever. What makes the copies
//! mean anything is `tests/shader_validation.rs`, which includes the validator
//! and asserts each one still equals the `mc_render` value it stands for —
//! because an agreement test against a private copy agrees with itself.
//!
//! **Two of them are derived rather than tabulated**, for the reason the
//! geometry builder's own comment gives: a six-row table of conventions cannot
//! be checked by reading it, and this project shipped one whose three
//! hand-written copies agreed and were wrong.
//!
//! Separated from the checking so that adding a value to compare and adding a
//! comparison are two different edits in two different files.

/// The six indices four corners are drawn as.
///
/// The build script's own copy of `mc_render::geometry::QUAD_INDEX_PATTERN`.
/// `tests/shader_validation.rs` includes this file and asserts the two are
/// equal, which is the only thing making the shader check below mean anything —
/// an agreement test against a private copy agrees with itself.
pub const QUAD_INDEX_PATTERN: [u32; 6] = [0, 1, 2, 0, 2, 3];

/// Which two components of a corner's local position a face's plane coordinates
/// are written into, one row per facing.
///
/// The build script's own copy of `mc_render::geometry::PLANE_AXES`, held for
/// the same reason and closed by the same test. A shader whose copy has drifted
/// runs a texture *across* a face instead of along it: the face's mean colour is
/// unchanged, so no probe over a captured frame can see it, and a golden minted
/// from that renderer records the drift as ground truth.
///
/// **This is the geometry's table and not an image basis.** It says where a
/// quad's two extents go, which is a different question from where an image's
/// own left-to-right and top-to-bottom run — and reusing it for the second was
/// the defect that drew five of six faces wrong. That question is
/// [`IMAGE_SWAPS`] and [`IMAGE_SIGNS`].
pub const PLANE_AXES: [[u32; 2]; 6] = [[1, 2], [1, 2], [0, 2], [0, 2], [0, 1], [0, 1]];

/// Whether a face's image runs its horizontal along the **secondary** of
/// [`PLANE_AXES`]' pair rather than the primary, `1` for exchanged.
///
/// The build script cannot depend on the crate it builds, so this is its own
/// answer to `mc_render::geometry::IMAGE_SWAPS`. **Derived rather than
/// tabulated**, for the reason the geometry builder's own comment gives: a
/// six-row table of conventions cannot be checked by reading it, and this
/// project shipped one whose three hand-written copies agreed and were wrong.
pub const IMAGE_SWAPS: [u32; 6] = [
    image_swap([-1, 0, 0], [1, 2]),
    image_swap([1, 0, 0], [1, 2]),
    image_swap([0, -1, 0], [0, 2]),
    image_swap([0, 1, 0], [0, 2]),
    image_swap([0, 0, -1], [0, 1]),
    image_swap([0, 0, 1], [0, 1]),
];

/// Whether each of an image's two coordinates runs against its axis rather than
/// along it, horizontal first, `1` for negated. Same row order.
pub const IMAGE_SIGNS: [[u32; 2]; 6] = [
    image_sign([-1, 0, 0]),
    image_sign([1, 0, 0]),
    image_sign([0, -1, 0]),
    image_sign([0, 1, 0]),
    image_sign([0, 0, -1]),
    image_sign([0, 0, 1]),
];

/// The world directions a face's image runs its right edge and its top edge
/// toward, for a viewer standing outside it.
///
/// A viewer outside a face looks along its inward direction with the world's up
/// as their up, and the image's right edge is then forward crossed with up. The
/// two horizontal faces have no world up in them, so theirs is chosen to match
/// what `voxforge` bakes: the top image's top edge runs toward `-z`, the bottom
/// image's toward `+z`.
///
/// The six outward normals are written at the call sites above and are the only
/// hand-written input here. A normal says which way a face points and nothing
/// about how an image sits on it, so there is no convention in one to get wrong.
const fn image_basis(normal: [i32; 3]) -> ([i32; 3], [i32; 3]) {
    let forward = [-normal[0], -normal[1], -normal[2]];
    let up = if normal[1] == 0 {
        [0, 1, 0]
    } else {
        [0, 0, -normal[1]]
    };
    (cross(forward, up), up)
}

/// Whether the face with this `normal` and this plane `pair` runs its image's
/// horizontal along the pair's secondary.
const fn image_swap(normal: [i32; 3], pair: [u32; 2]) -> u32 {
    let (right, _) = image_basis(normal);
    let (horizontal, _) = axis_of(right);
    let [_, secondary] = pair;
    (horizontal == secondary) as u32
}

/// Whether each of this face's image coordinates is negated.
const fn image_sign(normal: [i32; 3]) -> [u32; 2] {
    let (right, up) = image_basis(normal);
    let (_, horizontal_is_negative) = axis_of(right);
    let (_, up_is_negative) = axis_of(up);
    // An image's rows run downward, so its vertical coordinate always runs
    // against the direction its top edge points.
    [horizontal_is_negative as u32, !up_is_negative as u32]
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

/// The packed vertex's bit layout: the shift each field starts at and the width
/// it occupies, named exactly as `terrain.wgsl` declares them.
///
/// The build script's own copy of `mc_render::geometry::vertex`'s constants,
/// held for the reason every other copy here is and closed by the same
/// agreement test. **This delta is what made it worth writing down**: a field
/// was added at the top of the used range, so the one edit that can shift a
/// neighbour without shifting anything else was made in a layout that
/// `validate.rs` did not look at at all — while four other tables were checked.
/// A vertex decoded a bit out draws the whole world at a plausible wrong
/// texture, degree or section, and no mean colour reports it.
pub const VERTEX_LAYOUT: [(&str, u32); 10] = [
    ("const X_SHIFT", 0),
    ("const COORDINATE_BITS", 5),
    ("const FACING_SHIFT", 15),
    ("const FACING_BITS", 3),
    ("const LAYER_SHIFT", 18),
    ("const LAYER_BITS", 8),
    ("const SECTION_SHIFT", 26),
    ("const SECTION_BITS", 10),
    ("const OPACITY_SHIFT", 36),
    ("const OPACITY_BITS", 8),
];

/// The section table record's fields, in the order
/// `SceneGeometry::section_bytes` writes them and with the WGSL scalar each is
/// read as.
///
/// Both shaders declare this struct and neither can check it for itself: a
/// record whose fields have slid by one reads a coordinate as a quad count, and
/// a struct short of a field reads every section's box out of the next section's
/// origin. Every scalar here is four bytes wide, so the list's own length is the
/// record's stride — so `SECTION_RECORD.len() * 4` is what `scene.rs` must state
/// as `SECTION_RECORD_BYTES` and what the section buffer is allocated at.
/// `tests/shader_validation.rs` is where those two are compared.
pub const SECTION_RECORD: [(&str, &str); 12] = [
    ("origin_x", "i32"),
    ("origin_y", "i32"),
    ("origin_z", "i32"),
    ("first_quad", "u32"),
    ("quad_count", "u32"),
    ("opaque_quad_count", "u32"),
    ("min_x", "f32"),
    ("min_y", "f32"),
    ("min_z", "f32"),
    ("max_x", "f32"),
    ("max_y", "f32"),
    ("max_z", "f32"),
];
