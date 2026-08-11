# mc-render — wgpu renderer

Detail: `docs/technical/rendering.md`. Stack rationale: `docs/technical/decisions.md` ADR-002.

## Why this crate is custom

We took `bevy_ecs` standalone and wrote the renderer (ADR-002) because voxel rendering bypasses
general-purpose mesh/material abstractions anyway. That choice only pays off if this crate stays
voxel-specific. Do not grow a general scene graph here.

## Verification — read this before writing render code

**This crate is excluded from coverage thresholds (ADR-008), which means golden-frame tests are the
only thing standing between a regression and shipping it.** The exclusion is a bet that the frame
harness is good. Keep that bet honest:

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
