# Rendering

The section mesher — turning a 16³ chunk section into the visible, merged
quads a renderer draws — is built. It lives in `mc_world::mesh`, not in
`mc-render`: meshing is pure data transformation with no GPU involvement, so
it belongs on the `mc-world → mc-core` edge rather than adding a new one.
`mc-render` itself — the draw path, GPU vertex packing, culling and lighting
— does not exist yet. PRO-852's terrain renderer is built directly on top of
the quad contract below, which is why it is recorded here rather than left
for that spec to reverse-engineer from `mc-world`'s source.

The other half of this file — the capture orientation and pixel-format
contract — predates the renderer for the same reason the mesher contract now
does: established and asserted ahead of time, by the headless frame-capture
harness in `crates/mc-testkit` (module `frame`), so the first line of
`mc-render` inherits it rather than discovering it.

## The section mesher

`mc_world::mesh` exposes one free function:

```rust
pub fn mesh_section(
    section: &Section,
    neighbours: &Neighbours<'_>,
    registry: &BlockRegistry,
) -> Result<SectionMesh, MeshError>;
```

A `Quad` is a `facing`, a `plane` (the coordinate, along the facing's axis, of
the **solid voxel that emitted the face** — never of the face itself, so
every plane stays inside `0..16` and matches `world-format.md`'s
exclusive-bound convention), an origin `PlanePos` and an extent
`PlaneExtent` (both components ≥ 1), and a `block: BlockName`. A
`SectionMesh` hands them back as `quads() -> &[Quad]` or `into_quads() ->
Vec<Quad>`, so a consumer need not clone to take ownership.

The output is quads, **not** vertices. No triangulation, no index buffer, no
winding order, no UV assignment, no bit packing — all of those are the
renderer's decisions, made against the quad contract rather than baked into
it. A quad names its block by `BlockName`, not `BlockId`: ADR-011's argument
is that a runtime id is valid only for the registry that assigned it, and a
mesh still in flight across a future registry hot swap would otherwise
resolve to a different block. The cost is one `Arc<str>` clone per **quad**,
never per voxel. A quad never spans two sections — merging across a section
boundary is not attempted, even where the content on both sides would allow
it.

### Determinism — the property golden frames depend on

Emission order **is** the loop nesting, and no sort exists anywhere in the
mesher: facing in `Facing::ALL` order (−X, +X, −Y, +Y, −Z, +Z), then plane
ascending, then secondary ascending, then primary ascending. Primary and
secondary are the plane's two remaining axes taken in x < y < z order.

Identical section contents produce a byte-identical quad sequence regardless
of write history, palette order, palette length, index width, compaction
state, reference counts, or the order blocks were registered in. This holds
because solidity and block identity are resolved into **contents-ordered**
keys before the sweep ever runs: one pass over the section's own voxels in
their linear (x-fastest) order builds one entry per *distinct block the
section actually holds*, in first-encounter order. Nothing registry-local or
history-dependent survives into that pass's output, so nothing
registry-local or history-dependent can reach the emitted quads either.

**Golden frames captured against this mesh are reproducible only because of
this property.** A renderer capturing goldens is trusting that two sections
with the same voxels — however they got written, compacted, or reordered by
a registry swap — mesh identically every time; this is why that trust is
warranted structurally rather than by convention.

### The neighbour supply model

`Neighbours<'a>` is opaque over `[Option<&'a Section>; 6]`, built by
`Neighbours::none().with(facing, section)`. The slot a neighbour occupies
**is** `facing as usize`, which is also `Facing::ALL`'s declaration order,
the emission order above, and `Facing`'s derived `Ord`. There is one mapping,
not several written down separately — a swapped neighbour slot and a
reordered emission are the same mistake, and they fail together rather than
independently.

Every per-facing fact — the plane's axis, its primary and secondary axes, the
boundary plane a neighbour is consulted at (15 for a positive facing, 0 for a
negative one), the mirrored coordinate inside that neighbour (0 and 15
respectively), and the adjacency step — derives from one exhaustive
`(Axis, is_positive)` match. None of them is written out per facing
separately.

Absence is **per neighbour**, never all-or-nothing: each of the six slots is
independently `Some` or `None`. An absent neighbour is decided as if the
adjacent voxel were non-solid — deliberately, because treating an unloaded
neighbour as solid would hide the streaming edge behind a silently missing
face, and a visible wall at the world edge is correctable in a way a silently
missing surface at every chunk boundary is not.

A supplied neighbour resolves **only the palette entries its 256
facing-plane voxels reference** — narrower than the rule for the meshed
section itself, which resolves every palette entry any of its 4096 voxels
reference. A block a neighbour holds away from the shared face is never read
and can never fail the mesh.

### The merge predicate, as built

Coplanar faces of the same facing holding the same block merge by a scanline
greedy sweep: a run grows along the primary axis while the face exists, the
block matches, and the cell is unconsumed, then extends along the secondary
axis while a whole row matches. This is the scanline-greedy result, and
deliberately **not** the minimum rectangle count — that is a different and
harder problem, not pursued here. The predicate is "same block **by name**",
not "same palette slot": two distinct palette entries can name
the same block (a section's palette is not guaranteed unique by name — see
"Known limitations" below), and a position-keyed predicate would refuse a
legitimate merge between them.

### Ambient occlusion is out, and adding it later is not free

There is no lighting system, and MVP 1 ships flat-shaded placeholder
terrain, so the mesher emits no AO — even though `crates/mc-render/CLAUDE.md`
lists AO in the packed vertex format. This is a standing fact about the
system, not a historical note that stops mattering once AO eventually
arrives:

**AO is per-vertex. Once it exists, two coplanar same-block faces with
differing corner occlusion may no longer merge. Adding AO narrows the merge
predicate, changes quad counts, and invalidates every golden frame captured
against today's mesh.** A renderer capturing goldens today is capturing them
against a merge predicate that is already known to be going to change.

### The error contract

`mesh_section` returns a complete mesh or `Err` — never a partial mesh, never
a plausible substitute for something it could not resolve, and it never
panics.

An unresolvable block fails the **whole** mesh. `MeshError::UnresolvedBlock
{ name, position }` names the lowest such voxel in the meshed section's own
linear order. `MeshError::UnresolvedNeighbourBlock { name, facing, position
}` names the position in the *neighbour's own frame* — not mirrored, and not
translated into the meshed section's frame. The meshed section is always
resolved before any neighbour, so its error wins if both are unresolvable;
neighbours are resolved in `Facing::ALL` order, so the lowest-ordered
facing's error wins between two bad neighbours.

The refusal is deliberate: treating an unresolvable block as non-solid
punches a hole in the world; treating it as solid seals a cavity; both are
silent and indistinguishable from a correct mesh at the call site. A failed
mesh is not a breach of the "a bad mod never takes down the server"
invariant — nothing panics, and a caller running meshing on a worker simply
keeps its previous mesh. A failed mesh must **not** cascade once meshing is
threaded onto workers; that is a requirement on the future threading
integration, not something built here.

Only palette entries a voxel actually references are resolved — never a
vacated (zero-refcount) entry. This is forced by determinism, not chosen for
convenience: failing on a vacated entry naming a since-de-registered block
would make identical section contents mesh differently before and after
`compact()`. The resolution pass never consults reference counts at all, so
this property survives even a refcount bug elsewhere in the section.

### Purity

Meshing is a pure read, on both the success path and the failure path. Every
parameter is a shared reference, `Section` has no interior mutability, and
the returned mesh is owned. Calling `compact()` on the input section to
simplify the sweep — a tempting, observable shortcut — is forbidden, and
forbidden at **compile time**: the mesher only ever holds a shared reference
to a section, so a call to `compact()` (which needs `&mut self`) does not
type-check. Purity is not a convention here; it is what lets the mesher move
onto `rayon` workers later as an integration rather than a rewrite.
Scheduling, mesh caching, dirty-section tracking, and re-meshing on a block
edit are not built.

### Known limitations, as built

Current behaviour, not a roadmap.

- **Only solid blocks produce faces.** A section of water (`solid = false`)
  meshes to nothing, and water is invisible in MVP 1. That is the stated
  consequence of one solidity bit and no transparency pass, not an
  oversight.
- **The name-based merge predicate is held by one module and by review, not
  by the test suite.** Keying merges by palette position instead of
  deduplicating by name leaves the whole suite green: `Section::set_block`
  cannot build a palette naming one block twice, and no test builds one
  through `Section::import` either. It matters because `Section::import`
  *does* accept such a palette — its registration check verifies every named
  block is registered, not that the palette's names are unique — so a
  position-keyed predicate would refuse a legitimate merge and make the mesh
  depend on import history rather than on content. Whether `import` should
  reject a duplicate-name palette outright is owned by the section
  import/export work, not settled here.
- **No non-cube geometry, block models, rotations, or per-block shape data.**
  No lighting of any kind. No level of detail. No merging across a section
  boundary. No column- or world-level meshing — a caller assembles a
  section's neighbours itself, since there is no world or column map yet.
  `ChunkColumn::section(index)` is what reaches a section's vertical
  neighbours; the horizontal four have no lookup until streaming exists.
- **`MeshError::CorruptMeshIndex` is unreachable by design**, mirroring
  `SectionError::CorruptPaletteIndex`. Raw indexing, `unwrap` and `panic!`
  are lint-denied workspace-wide, so the sweep's roughly 6 × 4096 reads of
  its own fixed-size arrays are all `get`-shaped; every index is composed
  from coordinates already inside `0..16`, and the residual `None` collapses
  at one private call site into this variant rather than misreporting a
  mesher bug as a storage one. Its one line is deliberately uncovered.

## Orientation: row 0 is the top, clip-space y is up

**Framebuffer row 0 is the top of the image, and stays the top through
readback, comparison, PNG encode and PNG decode. No stage flips rows.**

**Clip-space y is up.** wgpu's framebuffer origin and clip-space y point in
opposite directions — this mismatch is the single most common source of
flipped output across the wgpu/WebGPU ecosystem, and resolving it is the
caller's responsibility, not the graphics API's.

Consequently: **a caller filling the top half of a render target writes
y > 0**. Any draw work — the capture harness's self-verification scene today,
`mc-render`'s vertex shaders tomorrow — must place geometry accordingly.

### Why this is written down before there is a renderer

A capture path that silently inverted rows would make every golden this
project ever commits wrong in the same direction — consistently, and
therefore invisibly. Against a solid-colour test fixture that is easy to
catch. Against terrain it is not: a vertically mirrored world looks entirely
plausible on a screenshot, and the first instinct when it appears is to
suspect worldgen, the camera, or the mesher, not the capture path that has
been silently upside-down since before the renderer existed. Chasing that
bug against terrain would be expensive and confusing; chasing it against a
computed 64×64 fixture with analytically known pixel values is neither.
That is why the convention above was settled and asserted by the capture
harness itself, before any renderer draws a single triangle, rather than
left as an assumption for `mc-render` to get right on faith.

## Capture pixel format

The offscreen render target the capture harness allocates is
`Rgba8UnormSrgb`. The hardware performs the sRGB encode on write — there is
no CPU-side sRGB encoding step anywhere in the capture path, and readback
copies texels verbatim. This is the standard path and the one a renderer is
expected to use for its own colour target.

Captured pixels are therefore 8-bit sRGB-encoded RGBA with **straight
(non-premultiplied) alpha**: nothing in the capture path multiplies a colour
by its alpha on write, and nothing divides it back out on read. A clear to
`(1.0, 1.0, 1.0, 0.25)` reads back as `(255, 255, 255, 64)` — the RGB
channels are untouched by the alpha value.

One colour target only: no depth buffer, no stencil, no MSAA. A renderer
that needs depth or multisampling is adding a new draw path, not extending
this one.

## Relationship to the frame-capture harness

These conventions are asserted, not merely stated: `docs/technical/testing.md`
describes the harness that enforces them and the tolerance model comparisons
are judged against. See that file for how a captured frame is verified, and
`docs/technical/decisions.md` (ADR-008) for why golden-frame comparison is
the coverage strategy for GPU-resident rendering code at all.
