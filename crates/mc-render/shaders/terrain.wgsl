// The terrain vertex and fragment stages.
//
// Both stages live in this one file and may not move out of it: WGSL has no
// preprocessor, so a helper shared between two shaders would have to be
// duplicated across them, and one file per pass is what keeps that from
// happening quietly.
//
// The vertex stage has a budget of exactly one storage binding, counted at build
// time -- the section table, for the world origins the packed corners are
// section-local to. Everything else it needs arrives as a uniform. That budget
// is the reason indices are compacted rather than quads instanced: a packed
// vertex reaches the stage as a conventional vertex buffer addressed by the
// compacted indices, not as a third storage buffer read by hand.
//
// Nothing here shades. A face's colour is its texel and no more, which is what
// makes the declared placeholder mean colours the values a captured frame can be
// clustered against -- a directional term would move every one of them by an
// amount nothing has declared.

struct Frame {
    view_projection: mat4x4<f32>,
    planes: array<vec4<f32>, 6>,
};

// The section table, field for field as `SceneGeometry::section_bytes` writes
// it: three origin components, the first quad, the quad count, how many of those
// quads are opaque, then the box's minimum and maximum corner. Spelled as
// scalars rather than as vectors because a `vec3` carries sixteen-byte alignment
// and the record on the other side is forty-eight bytes of tightly packed
// fields.
//
// This stage reads only the origin. The rest of the record is declared because
// the two stages read the same buffer and a struct short of a field would slide
// every later one.
struct Section {
    origin_x: i32,
    origin_y: i32,
    origin_z: i32,
    first_quad: u32,
    quad_count: u32,
    opaque_quad_count: u32,
    min_x: f32,
    min_y: f32,
    min_z: f32,
    max_x: f32,
    max_y: f32,
    max_z: f32,
};

@group(0) @binding(0) var<uniform> frame: Frame;
@group(0) @binding(1) var<storage, read> sections: array<Section>;

@group(1) @binding(0) var terrain_textures: texture_2d_array<f32>;
@group(1) @binding(1) var terrain_sampler: sampler;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    // Flat: a layer index is a choice of texture, and interpolating between two
    // of them would sample a third that nothing resolved.
    @location(1) @interpolate(flat) layer: u32,
    // Flat for the same reason one step removed: the degree is a property of the
    // block, identical at all four corners, and interpolating it would produce
    // a value no declaration states wherever the rasteriser rounds.
    @location(2) @interpolate(flat) opacity: f32,
};

// The bit layout of `PackedVertex`, field by field, as the shift each starts at
// and the width it occupies.
//
// These are `mc_render::geometry::vertex`'s own constants written out because a
// shader cannot read a Rust one, and `build/validate.rs` compares every value
// below against them and fails the build when any disagrees. The decode reads
// them rather than repeating their numbers, so the check is over the numbers the
// decode actually uses.
const X_SHIFT: u32 = 0u;
const COORDINATE_BITS: u32 = 5u;
const FACING_SHIFT: u32 = 15u;
const FACING_BITS: u32 = 3u;
const LAYER_SHIFT: u32 = 18u;
const LAYER_BITS: u32 = 8u;
const SECTION_SHIFT: u32 = 26u;
const SECTION_BITS: u32 = 10u;
const OPACITY_SHIFT: u32 = 36u;
const OPACITY_BITS: u32 = 8u;

// How wide one word of the packed pair is.
const WORD_BITS: u32 = 32u;

// The largest value a quantised opacity carries, which is also what it is
// divided by. `Opacity::from_quantised` is the same division on the CPU side.
const STORED_MAX: f32 = 255.0;

// The section index is the one field that crosses the boundary between the two
// words, so it is assembled from both. The opacity sits wholly above the
// boundary and is read out of the high word alone -- `vertex.rs` asserts that at
// compile time, so a layout that ever moved it below thirty-two fails the Rust
// build rather than decoding to nonsense here.
@vertex
fn vertex_main(@location(0) packed: vec2<u32>) -> VertexOutput {
    let low = packed.x;
    let high = packed.y;

    let coordinate = (1u << COORDINATE_BITS) - 1u;
    let local = vec3<f32>(
        f32((low >> X_SHIFT) & coordinate),
        f32((low >> (X_SHIFT + COORDINATE_BITS)) & coordinate),
        f32((low >> (X_SHIFT + 2u * COORDINATE_BITS)) & coordinate),
    );
    let facing = (low >> FACING_SHIFT) & ((1u << FACING_BITS) - 1u);
    let layer = (low >> LAYER_SHIFT) & ((1u << LAYER_BITS) - 1u);
    let section_index =
        ((low >> SECTION_SHIFT) | (high << (WORD_BITS - SECTION_SHIFT))) & ((1u << SECTION_BITS) - 1u);
    let opacity = (high >> (OPACITY_SHIFT - WORD_BITS)) & ((1u << OPACITY_BITS) - 1u);

    let section = sections[section_index];
    let origin = vec3<f32>(
        f32(section.origin_x),
        f32(section.origin_y),
        f32(section.origin_z),
    );

    var out: VertexOutput;
    out.clip_position = frame.view_projection * vec4<f32>(origin + local, 1.0);
    out.uv = plane_coordinates(facing, local);
    out.layer = layer;
    out.opacity = f32(opacity) / STORED_MAX;
    return out;
}

// The texel, with the block's declared degree folded into its alpha.
//
// **The product of the two**, not one or the other: the declared degree decides
// which of the two draws a face lands in and the texel's alpha modulates within
// the blended one. So a block at a whole degree draws opaque however its
// texture's alpha reads, and this multiply is the identity for it -- which is
// what keeps the partition a function of a declared, greppable, hot-reloadable
// number rather than of art.
@fragment
fn fragment_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let texel = textureSample(terrain_textures, terrain_sampler, input.uv, input.layer);
    return vec4<f32>(texel.rgb, texel.a * input.opacity);
}

// Which two components of a corner's section-local position its face's plane
// coordinates are read from -- the primary first, then the secondary -- and then
// how an image sits on that pair: whether its own horizontal runs along the
// secondary rather than the primary, and whether either coordinate runs against
// its axis. One row per facing in `mc_world::mesh::Facing`'s declaration order,
// in all three tables.
//
// These are `mc_render::geometry::PLANE_AXES`, `IMAGE_SWAPS` and `IMAGE_SIGNS`,
// written out because a shader cannot read a Rust constant. On the Rust side the
// second and third are *derived* from each facing's own normal by two lines of
// vector arithmetic; these literals are the only hand-written copies left, and
// `build/validate.rs` compares all three against the Rust values and fails the
// build when any disagrees.
//
// **The plane pair is the geometry's and it is not an image basis.** It says
// which components a quad's two extents were written into. Where an image's own
// left-to-right and top-to-bottom go is a separate question, and reading the
// pair alone as if it answered that question is what shipped: east and west
// turned a quarter, north and south and the underside flipped, only the top
// correct. Five of six faces, while three hand-written copies of one table
// agreed with each other exactly.
//
// **A swap and a sign are what a pair of axis indices cannot express.** An
// image's rows run downward while the world's vertical axis runs up, so every
// face with world up in it needs its vertical coordinate negated; two faces
// looking at each other along one axis see their in-plane axes in opposite
// horizontal order, so one of each pair needs its horizontal negated; and an X
// face's pair lists its vertical first, so its two coordinates are exchanged.
//
// **That is also why agreement is not the property to rest on.** A build-time
// comparison between two copies says they match and says nothing about whether
// either is right. What can say that is a reading of the picture: FR-8.1-S7 for
// where a face's bands sit and FR-8.1-S8 for which way it runs.
const PLANE_AXES: array<u32, 12> =
    array<u32, 12>(1u, 2u, 1u, 2u, 0u, 2u, 0u, 2u, 0u, 1u, 0u, 1u);
const IMAGE_SWAPS: array<u32, 6> = array<u32, 6>(1u, 1u, 0u, 0u, 0u, 0u);
const IMAGE_SIGNS: array<u32, 12> =
    array<u32, 12>(0u, 1u, 1u, 1u, 0u, 1u, 0u, 0u, 1u, 1u, 0u, 1u);

// A corner's coordinates within its own face's image.
//
// The same convention the geometry builder places a corner under, so an image
// arrives on a face the way a viewer standing outside it would see it: the
// image's top edge toward the face's own up, its right edge toward that viewer's
// right. Coordinates are in whole blocks and the sampler repeats, so a face
// merged across four blocks shows the texture four times rather than stretched
// once -- and a negated coordinate mirrors within each block, which is what
// makes a sign expressible at all here.
//
// Every lookup goes through a function-scope copy: a constant array and a vector
// value cannot be indexed by anything but a constant, and the index here is the
// facing the vertex was packed with.
fn plane_coordinates(facing: u32, local: vec3<f32>) -> vec2<f32> {
    var axes = PLANE_AXES;
    var swaps = IMAGE_SWAPS;
    var signs = IMAGE_SIGNS;
    var corner = local;
    let row = facing * 2u;
    let primary = corner[axes[row]];
    let secondary = corner[axes[row + 1u]];
    let exchanged = swaps[facing] == 1u;
    let horizontal = select(primary, secondary, exchanged);
    let vertical = select(secondary, primary, exchanged);
    return vec2<f32>(
        select(horizontal, -horizontal, signs[row] == 1u),
        select(vertical, -vertical, signs[row + 1u] == 1u),
    );
}
