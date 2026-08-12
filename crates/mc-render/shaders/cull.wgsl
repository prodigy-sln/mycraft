// The compute pass that decides which sections are drawn and compacts their
// indices into the one indirect draw.
//
// One workgroup per section at 64 lanes. Lane 0 tests the section's world box
// against the six frustum planes, writes the visibility flag, and reserves a
// range of the destination index buffer with a single atomic add on the indirect
// arguments' index count -- which is why there is no second dispatch and no
// prefix sum. All 64 lanes then stride the section's quads, so a dense section
// does not serialise on one lane.
//
// Two things are fixed here because the build-time validator counts them:
//
//   * at most four storage bindings for this entry point -- a fifth is the
//     weakest adapter in the declared range dropping out of the supported set,
//     not a refactor. The four are: the section table, the visibility flags, the
//     destination indices, and the indirect arguments. The frustum arrives as a
//     uniform precisely so that it costs none of them.
//   * the winding literal below, against the geometry builder's own constant.
//
// The order visible sections land in the index buffer is whatever the atomic
// hands out, and therefore not reproducible between runs. That is safe here for
// a stated reason: terrain is fully opaque, depth-tested, and no two quads cover
// the same voxel face -- so no two fragments contend for one depth value and the
// image does not depend on the order. The day a transparency pass arrives this
// reasoning expires and compaction has to become order-stable.

// The six indices a quad is drawn as. This is the CPU's QUAD_INDEX_PATTERN,
// spelled a second time because the compute pass writes the index buffer and
// reading the CPU's copy would cost the fifth storage binding there is no room
// for. The build compares this literal against the Rust constant and fails if
// they disagree, so the duplication is checked rather than trusted.
const QUAD_INDEX_PATTERN: array<u32, 6> = array<u32, 6>(0u, 1u, 2u, 0u, 2u, 3u);

// How many corners a quad contributes to the vertex buffer.
const CORNERS_PER_QUAD: u32 = 4u;

// How many indices a quad is drawn by.
const INDICES_PER_QUAD: u32 = 6u;

// How many lanes one workgroup runs.
const LANES: u32 = 64u;

struct Frame {
    view_projection: mat4x4<f32>,
    planes: array<vec4<f32>, 6>,
};

// The section table, field for field as `SceneGeometry::section_bytes` writes
// it. See the identical declaration in `terrain.wgsl` -- the two stages read the
// same buffer, and this layout is the one thing about it neither of them can
// check for itself.
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

// The arguments the one `draw_indexed_indirect` reads. Exactly one field varies:
// `index_count`, which this pass raises. `instance_count` is 1 and
// `first_instance` is 0 so that no optional device feature is required.
struct DrawArgs {
    index_count: atomic<u32>,
    instance_count: u32,
    first_index: u32,
    base_vertex: i32,
    first_instance: u32,
};

@group(0) @binding(0) var<uniform> frame: Frame;
@group(0) @binding(1) var<storage, read> sections: array<Section>;
@group(0) @binding(2) var<storage, read_write> visible: array<u32>;
@group(0) @binding(3) var<storage, read_write> indices: array<u32>;
@group(0) @binding(4) var<storage, read_write> args: DrawArgs;

// What lane 0 decided, for the other sixty-three to read after the barrier.
var<workgroup> reserved_base: u32;
var<workgroup> reserved_quads: u32;
var<workgroup> reserved_first_quad: u32;

@compute @workgroup_size(64)
fn cull_sections(
    @builtin(workgroup_id) group: vec3<u32>,
    @builtin(local_invocation_index) lane: u32,
) {
    let section_index = group.x;

    if lane == 0u {
        let section = sections[section_index];
        let seen = admits(section);
        visible[section_index] = select(0u, 1u, seen);
        reserved_quads = select(0u, section.quad_count, seen);
        reserved_first_quad = section.first_quad;
        reserved_base = atomicAdd(&args.index_count, INDICES_PER_QUAD * reserved_quads);
    }
    workgroupBarrier();

    compact(lane);
}

// Writes six indices for every quad this lane is responsible for.
//
// Lane `l` takes quads `l`, `l + 64`, `l + 128`, ... of the section, so the
// writes are spread across the workgroup rather than issued by one invocation.
fn compact(lane: u32) {
    var pattern = QUAD_INDEX_PATTERN;
    var quad = lane;
    loop {
        if quad >= reserved_quads {
            break;
        }
        let corner = CORNERS_PER_QUAD * (reserved_first_quad + quad);
        let at = reserved_base + INDICES_PER_QUAD * quad;
        for (var step = 0u; step < INDICES_PER_QUAD; step = step + 1u) {
            indices[at + step] = corner + pattern[step];
        }
        quad = quad + LANES;
    }
}

// Whether any part of the section's world box could be drawn.
//
// The same test the CPU-side frustum function makes, in the same terms: the six
// planes are **unnormalised**, and only the sign of `normal . p + offset`
// matters, so nothing here divides by a length. The corner taken is the one
// furthest along each plane's normal -- if even that corner is behind the plane,
// every other one is too. Conservative in the corners in exactly the way the CPU
// function is, because the scenario that compares them asserts they select the
// *same* set and a tighter test here would disagree by dropping sections the
// prediction kept.
fn admits(section: Section) -> bool {
    let low = vec3<f32>(section.min_x, section.min_y, section.min_z);
    let high = vec3<f32>(section.max_x, section.max_y, section.max_z);
    for (var index = 0u; index < 6u; index = index + 1u) {
        let plane = frame.planes[index];
        let normal = plane.xyz;
        let furthest = select(low, high, normal >= vec3<f32>(0.0));
        if dot(normal, furthest) + plane.w < 0.0 {
            return false;
        }
    }
    return true;
}
