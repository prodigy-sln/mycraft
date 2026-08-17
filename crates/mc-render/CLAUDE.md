# mc-render — wgpu renderer

Detail: `docs/technical/rendering.md`. Stack rationale: `docs/technical/decisions.md` ADR-002.

## Why this crate is custom

We took `bevy_ecs` standalone and wrote the renderer (ADR-002) because voxel rendering bypasses
general-purpose mesh/material abstractions anyway. That choice only pays off if this crate stays
voxel-specific. Do not grow a general scene graph here.

## Verification — read this before writing render code

**The `src/gpu/` subtree of this crate is excluded from coverage thresholds (ADR-008, narrowed by
ADR-013), which means golden-frame tests are the only thing standing between a regression there and
shipping it.** Everything outside `src/gpu/` is counted like any other library code. The exclusion
is a bet that the frame harness is good. Keep that bet honest:

- Every visual feature gets a golden frame in `mc-testkit`, captured from a fixed seed, fixed camera
  path, and fixed tick count.
- Golden frames are compared perceptually, not byte-wise — GPU drivers differ.
- A deliberate visual change updates the golden and says so in the commit. An unexplained golden
  update in a diff is a review stop.
- Anything expressible as a pure function — meshing, packing, culling maths, atlas layout, light
  propagation — is unit-tested normally and is **not** exempt. Only GPU-resident work gets the
  exclusion.

Correspondingly: keep logic out of the GPU-touching layer. A meshing bug should be catchable without
a GPU. If a function needs a device to test, ask whether the maths inside it could live in a
testable free function first.

## Performance rules

- Terrain draws through **one indirect draw call**; per-chunk commands are built by a compute shader
  doing frustum and occlusion culling. Adding a per-chunk CPU draw call is a regression.
  **This rule binds whether or not the current increment measures draw calls or frame time.** The
  absence of a benchmark is not licence to drop the property — build it properly from the start.
  **Occlusion culling is deferred, frustum culling is not.** The compute pass, the atomic index
  compaction and the single `draw_indexed_indirect` are built (PRO-852); a hierarchical depth
  pyramid is a self-contained later feature that slots into the same compaction step and brings its
  own goldens. Deferred with a reason, not quietly unimplemented — so the day it is wanted, nothing
  about the draw path changes.
- Vertices are bit-packed (position, normal, AO, UV). Growing the vertex format is a deliberate,
  measured decision — a 16³ section should stay cache-resident.
- Meshing runs on `rayon` workers, never on the render or tick thread. Budget: < 200 µs per section.
- Textures use an **array texture**, not an atlas — no bleeding, mipmaps work, and a single block
  texture can hot-reload without rebuilding everything.
- Benchmark with `criterion` before optimizing. `code-quality.md` §8: make it work, make it right,
  make it fast, in that order.

## Robustness

- **No panics in the render loop.** `unwrap`, `expect`, `panic!` and `indexing_slicing` are
  lint-denied workspace-wide; this is where the denial matters most. A dropped frame is recoverable,
  a crash is not.
- Handle surface loss, device loss, and window minimize/resize to zero. These happen routinely on
  real machines, especially on laptop GPU switches.
- Target hardware spans an RTX 4090 and an Intel UHD 770. Features that only work on discrete GPUs
  need a fallback path, and the fallback needs its own golden frame.
- Shaders are validated at build time via `naga`, not at first draw.

## Boundaries

- This crate renders. It does not simulate, own game state, or decide anything authoritative — it
  reads a snapshot produced by `mc-sim`.
- It must never block the tick thread. Reading a stale snapshot is correct; stalling the simulation
  is not.
- `egui` is for debug and tooling UI **only**. Anything a player sees during normal play is custom
  and follows `standards/global/ui-design.md`.

## Never take `glam::camera::rh::proj::vulkan::perspective`

The camera uses `glam::camera::rh::proj::directx::perspective` and
`glam::camera::rh::view::look_at_mat4` — right-handed, NDC z 0..1, clip-space y **up**. The
`vulkan::` module of the same shape is y-**down**, and swapping it in compiles, runs, and renders a
vertically mirrored world. That is precisely the failure FR-8.2-S1 and its negative control
FR-8.2-S6 exist to catch, so it will be caught — but by a golden diff whose cause is not obvious,
not by the type system.

(The older `Mat4::perspective_rh` / `Mat4::look_at_rh` pair is deprecated as of the pinned glam
0.33.3 and does not compile under `-D warnings`. Documents naming that pair are stale, not wrong
about the intent.)

## Known gap — texture resolution does not consult the registry

`build_section_geometry` matches a quad's `BlockName` against a `TextureKey` **by identical
spelling**. The registry is the real authority for which texture a block draws, and the builder's
signature passes no registry, so the match is a coincidence that holds only because every
`content/base/blocks/*` happens to declare `texture` equal to `name`.

**There are two such sites, not one.** `hud::held::held_swatch` (`src/hud/held.rs`) resolves the
held-block indicator by parsing the block's own name as a texture key, on exactly the same
coincidence. Whoever closes this gap must find both — one fixed and one left would show a block
drawing correctly in the world while its indicator draws nothing, which reads as a HUD bug.

Deferred deliberately for MVP 1, on two grounds: the spec contemplates the failure (FR-1.1-S5), and
the failure mode is `UnresolvedTexture` naming the block — loud, never a wrong picture. Nothing
silently renders the wrong thing.

**MVP 2 must close it, and will hit it immediately.** A mod that draws two blocks with one texture,
or names a texture differently from its block, is valid content this cannot express — and invariant
1 says a missing hook is fixed in the API, never special-cased. The fix routes resolution through
the registry and changes a binding signature, which is why it was not taken mid-implementation with
five phases already broken down against it.
