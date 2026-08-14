// The HUD pass: one instanced draw over the rectangles the composition planned.
//
// The whole pass is one draw call whatever the layout holds. A rectangle is an
// instance, its four corners come from the vertex index, and nothing is read
// from a vertex buffer — a HUD of three elements does not deserve a buffer
// upload per frame, and a per-rectangle draw loop is the shape this crate's
// terrain path exists to avoid.
//
// Colours arrive **already decoded into linear light**. The colour target is
// sRGB and the hardware performs the encode on write, so a colour decoded here
// as well would be encoded twice; the single CPU-side conversion lives in
// `src/hud/uniform.rs`, which is also where the reason is written down. Alpha is
// not a colour and is never decoded.
//
// This shader binds one uniform and no storage buffer at all, so the
// four-storage-globals-per-stage budget the weakest declared adapter offers is
// untouched by the HUD.

// How many rectangles the uniform's array holds.
//
// `src/hud/uniform.rs` declares the same number as `MAX_HUD_RECTS` and sizes the
// buffer from it. The two are not mechanically tied.
const MAX_RECTS: u32 = 256u;

struct Rect {
    // x and y in physical pixels from the target's top-left corner, then the
    // two extents.
    bounds: vec4<f32>,
    // Linear light, with straight (non-premultiplied) alpha. Read only by a
    // rectangle painted flat; a textured one carries opaque white here.
    color: vec4<f32>,
    // x is the array layer this rectangle samples, or negative where it is
    // painted with `color` alone. The other three components pad the member out
    // to the sixteen-byte alignment a uniform's array element is strided by.
    sampling: vec4<f32>,
};

struct Hud {
    // The target's extents in physical pixels. The other two components pad the
    // header out to the alignment the array member starts on.
    //
    // Not spelled `target`, which WGSL reserves.
    extents: vec4<f32>,
    rects: array<Rect, MAX_RECTS>,
};

@group(0) @binding(0) var<uniform> hud: Hud;

// The same array texture and sampler the terrain reads through, so a swatch of a
// block is the block's own texture rather than a second copy of it that could
// disagree. The sampler is nearest and repeating, which is what makes a swatch's
// texels the layer's texels exactly.
@group(0) @binding(1) var layers: texture_2d_array<f32>;
@group(0) @binding(2) var layer_sampler: sampler;

// The four corners of a unit quad, as the six vertices of two triangles.
//
// The pipeline culls no face, so the winding here decides nothing and cannot
// silently hide a rectangle the way a back-facing terrain quad would.
const CORNERS: array<vec2<f32>, 6> = array<vec2<f32>, 6>(
    vec2<f32>(0.0, 0.0),
    vec2<f32>(1.0, 0.0),
    vec2<f32>(0.0, 1.0),
    vec2<f32>(1.0, 0.0),
    vec2<f32>(1.0, 1.0),
    vec2<f32>(0.0, 1.0),
);

struct Fragment {
    @builtin(position) clip: vec4<f32>,
    @location(0) color: vec4<f32>,
    // Where in the sampled layer this corner sits: the unit quad's own corner, so
    // one whole texture covers the rectangle however large it is drawn.
    @location(1) uv: vec2<f32>,
    // Flat because a layer index is one value for the whole instance, and
    // interpolating it would put a fractional layer at every fragment but the
    // corners.
    @location(2) @interpolate(flat) layer: f32,
};

@vertex
fn vertex_main(
    @builtin(vertex_index) vertex: u32,
    @builtin(instance_index) instance: u32,
) -> Fragment {
    let rect = hud.rects[instance];
    let corner = CORNERS[vertex];
    let pixel = rect.bounds.xy + corner * rect.bounds.zw;

    // Framebuffer row 0 is the top of the target and clip-space y points up, so
    // the vertical axis is inverted exactly here and nowhere else. A plan states
    // its rectangles in framebuffer coordinates, which is what the capture
    // harness reads back and what every expected rectangle is derived in.
    let ndc = vec2<f32>(
        pixel.x / hud.extents.x * 2.0 - 1.0,
        1.0 - pixel.y / hud.extents.y * 2.0,
    );

    var fragment: Fragment;
    fragment.clip = vec4<f32>(ndc, 0.0, 1.0);
    fragment.color = rect.color;
    fragment.uv = corner;
    fragment.layer = rect.sampling.x;
    return fragment;
}

@fragment
fn fragment_main(fragment: Fragment) -> @location(0) vec4<f32> {
    // Sampled unconditionally and then chosen between, rather than sampled inside
    // the branch. `textureSample` computes its own derivatives and so requires
    // uniform control flow; a flat per-instance layer is uniform across a
    // triangle, but that is a fact about the data rather than one the compiler
    // has to accept, and `select` needs no such argument.
    //
    // The array texture is sRGB, so a texel is decoded to linear light here and
    // the target re-encodes it on write. That round trip is why a swatch's pixels
    // come back as the bytes the layer's texels were generated as.
    let sampled = textureSample(layers, layer_sampler, fragment.uv, u32(max(fragment.layer, 0.0)));
    return select(fragment.color, sampled, fragment.layer >= 0.0);
}
