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
// it: three origin components, the first quad, the quad count, then the box's
// minimum and maximum corner. Spelled as scalars rather than as vectors because
// a `vec3` carries sixteen-byte alignment and the record on the other side is
// forty-four bytes of tightly packed fields.
struct Section {
    origin_x: i32,
    origin_y: i32,
    origin_z: i32,
    first_quad: u32,
    quad_count: u32,
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
};

// The bit layout of `PackedVertex`, which this decodes:
//
//   x 0..5   y 5..10   z 10..15   facing 15..18   layer 18..26   section 26..36
//
// The section index is the one field that crosses the thirty-two bit boundary,
// which is why it is assembled from both words.
@vertex
fn vertex_main(@location(0) packed: vec2<u32>) -> VertexOutput {
    let low = packed.x;
    let high = packed.y;

    let local = vec3<f32>(
        f32(low & 31u),
        f32((low >> 5u) & 31u),
        f32((low >> 10u) & 31u),
    );
    let facing = (low >> 15u) & 7u;
    let layer = (low >> 18u) & 255u;
    let section_index = ((low >> 26u) & 63u) | ((high & 15u) << 6u);

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
    return out;
}

@fragment
fn fragment_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(terrain_textures, terrain_sampler, input.uv, input.layer);
}

// Which two components of a corner's section-local position its face's plane
// coordinates are read from: the primary first, then the secondary, one row per
// facing in `mc_world::mesh::Facing`'s declaration order.
//
// This is `mc_render::geometry::PLANE_AXES`, written out because a shader cannot
// read a Rust constant. `build/validate.rs` reads this literal as text and fails
// the build when the two disagree -- the only mechanical check there can be, and
// the reason the values are a literal rather than anything computed.
//
// Indexed by the discriminant the vertex already carries, so nothing here
// derives the facing's axis. `facing >> 1u` would be a second, unguarded copy of
// the declaration order, and a reordering of the enum would move four of these
// six rows while every Rust answer computed from `Facing::axis()` stayed
// correct. The drift that produces runs a texture *across* a face instead of
// along it, which leaves the face's mean colour untouched -- so no derived probe
// reports it and a golden shot from the drifted renderer records it as truth.
const PLANE_AXES: array<u32, 12> =
    array<u32, 12>(1u, 2u, 1u, 2u, 0u, 2u, 0u, 2u, 0u, 1u, 0u, 1u);

// A corner's coordinates within its own face's plane.
//
// The same convention the geometry builder places a corner under, so the texture
// runs along the quad's primary axis and not across it. Coordinates are in whole
// blocks and the sampler repeats, so a face merged across four blocks shows the
// texture four times rather than stretched once.
//
// Both lookups go through function-scope copies: a constant array and a vector
// value cannot be indexed by anything but a constant, and the index here is the
// facing the vertex was packed with.
fn plane_coordinates(facing: u32, local: vec3<f32>) -> vec2<f32> {
    var axes = PLANE_AXES;
    var corner = local;
    let row = facing * 2u;
    return vec2<f32>(corner[axes[row]], corner[axes[row + 1u]]);
}
