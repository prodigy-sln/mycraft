# Rendering

The section mesher — turning a 16³ chunk section into the visible, merged
quads a renderer draws — lives in `mc_world::mesh`, not in `mc-render`:
meshing is pure data transformation with no GPU involvement, so it belongs
on the `mc-world → mc-core` edge rather than adding a new one. The terrain
draw path in `mc-render` is built directly on the quad contract below, and
consumes it unchanged.

The orientation and pixel-format contract in this file predates both: it was
established and asserted ahead of time by the headless frame-capture harness
in `crates/mc-testkit` (module `frame`), so the first line of `mc-render`
inherited it rather than discovering it.

There is no lighting model. Terrain is flat-shaded: a fragment's colour is
its texture sample and nothing else — no ambient occlusion, no directional
term, no shadows, no transparency pass.

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

### An empty cell is answered before the registry

A cell holds a block or nothing (`technical/world-format.md`), and a
palette entry holding nothing resolves to **non-solid without the registry
being consulted at all**. That is one arm in one place, and it covers both
the meshed section and every supplied neighbour — which is what keeps "an
empty cell shows no face" a single rule rather than two that could drift
apart at a chunk boundary, where the difference would show as a seam only
under a specific neighbour supply.

Two consequences worth stating, because both are load-bearing:

- **A section holding nothing anywhere meshes into a mesh carrying no
  quads, even against a registry holding no block at all.** Emptiness is
  not an unresolved block, so it earns no `UnresolvedBlock` refusal.
- **Nothing about the sweep changes.** Emptiness reaches it as a non-solid
  cell, and a non-solid cell already emitted no face. `visible_face`, the
  merge predicate, the loop nesting and the emission order are untouched by
  it — which is the reason retiring the base game's former empty block
  moved no pixel of any committed golden frame.

Keys are per distinct **contents**, so two palette entries both holding
nothing deduplicate to one key exactly as two entries naming the same block
already do.

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

### Terrain sampling is `Nearest`, and switching to filtered invalidates goldens

The terrain sampler is `Nearest` on minification, magnification and mipmaps,
with `Repeat` addressing. That is not a preference about sharpness. A
placeholder texture is sixteen texels of deliberate pattern whose **declared
mean colour** is the value the frame probes cluster captured pixels against;
linear filtering blurs a face towards that mean, so a filtered frame would
agree with the probes for a reason that has nothing to do with the texture
being correct — passing while carrying no evidence.

Same shape as the AO warning above, and it belongs beside it: **a later switch
to filtered sampling changes every terrain pixel and invalidates every golden
frame captured against today's renderer.** It is a deliberate, measured change
that re-shoots the goldens and says so, never a quiet quality improvement.

Texture coordinates are a corner's two in-plane axes in **whole blocks**, so a
face merged across four blocks shows the texture four times rather than
stretched once. That, too, is baked into every golden.

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
keeps its previous mesh.

`MeshError::EmptyBlockFace { key }` is a fourth variant and an **internal
invariant**, not anything a caller did: emptiness resolves to non-solid and
a face is emitted only where the voxel is solid, so no quad can name the
empty entry. It exists because a `Quad` names a block and nothing is not a
block — there is no honest quad to build for one, and dropping it silently
would remove geometry nobody asked to remove. Like `CorruptMeshIndex`
below, it is a `None` that needs somewhere to go rather than a condition
any input can produce.

**What a caller does with that error depends on the lifecycle stage, and the
two rules are different.** The non-cascade rule above carries its own
premise: "keeps its *previous* mesh" presupposes a previous mesh exists —
that is, a live world being re-meshed after an edit or a stream-in.

| Stage | Policy |
|---|---|
| Initial preparation of a fixed fixture | Fail the whole preparation, naming the column and section index |
| Incremental re-mesh of a live or streaming world | Must not cascade; keep the previous mesh |

The client's startup path is the first stage and fails outright, because
there is no previous mesh to keep, the block set is exactly what
`content/base/` declares, and the only way a section fails is a defect in
the renderer's own code or content. Continuing would render a world that is
not the declared one, and every golden and every probe would then measure
that world. The second rule is still a requirement on the streaming
integration, which is not built.

**The reported failure must be deterministic.** Meshing runs on `rayon`, and
`collect::<Result<Vec<_>, _>>()` short-circuits: with two failing sections it
surfaces whichever one lost the race, so the error message is not
reproducible. The preparation path collects `Vec<Result<…>>` and takes the
**first `Err` in section index order** instead. More generally, only
`IndexedParallelIterator::collect` into a `Vec` may be used on that path —
`for_each` into a shared sink, and collecting into a set or map, are what
would make output order lucky rather than guaranteed.

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

## The terrain draw path

Terrain reaches the screen as **one `draw_indexed_indirect` call** whose index
count a compute shader writes. Nothing between the mesher's quads and that call
is per-chunk CPU work.

### Quads become vertices and indices

`mc-render`'s pure layer turns each `Quad` into **four corner vertices and six
indices**, expressed in the section's own frame. The world frame is
reconstructed on the GPU from the section table's origin, which is what keeps
the packed view and the world view from drifting apart.

`QUAD_INDEX_PATTERN = [0, 1, 2, 0, 2, 3]` is one constant, not six:
**facing-dependent winding lives entirely in the order the four corners are
emitted**, so the index pattern is facing-independent by construction. The
winding is fixed on the CPU by a geometric-normal test. **If culling ever looks
inverted, the fix is `front_face`, never re-winding the geometry** — re-winding
makes the picture look right while breaking the property the normal test
asserts, which is the worst available outcome. The pipeline runs
`front_face: Ccw`, `cull_mode: Some(Face::Back)`.

Scene assembly order is declared once and is binding, because packed bytes are
compared for determinism and goldens are captured against them: columns in
`(cz, cx)` ascending, then section index ascending, then the mesher's own quad
order, untouched.

`SceneGeometry::assemble` is the **only** capacity gate — `MAX_SECTIONS = 1024`,
`MAX_QUADS = 1 << 18` — so an over-capacity scene cannot be constructed and
nothing downstream re-checks.

**Breaking blocks pushes toward that ceiling, not away from it.** Every break exposes new faces, so a
heavily excavated world meshes to *more* quads than the world it started as — the opposite direction
from what intuition suggests a "hole" would cost. Measured against the replay world's 262 144-quad
capacity: a fully checkerboarded 16³ section costs 12 288 quads (every voxel isolated, nothing merges,
2048 solid voxels × 6 faces), so capacity covers roughly 21 of the world's 256 sections in that worst
state. A single break costs at most 18 quads in the pessimal case — up to 6 newly exposed 1×1 faces on
neighbours, plus splitting up to six already-merged rectangles at up to 3 quads each — which puts the
crossing point at roughly 15 000 to 45 000 pessimally-placed edits depending on pattern. No scripted
replay reaches it (`docs/technical/testing.md` §"The 10 000-block exit criterion"), because a replay that
never uploads to the GPU never asks `SceneGeometry::assemble` anything; a long, excavation-heavy play
session is what would. `RendererError::SceneTooLarge` is the existing, loud failure that session would
hit — sizing the buffers for it is tracked as **PRO-883**, not left as a silent gap.

### The packed vertex, and the section table

A vertex is a **single `u64`, 8 bytes**, and each field is cut to the width its
own domain needs rather than the width of the Rust type it arrives in: 5 bits
per coordinate (corners run `0..=16`, seventeen values), 3 for the facing, 8 for
the texture layer (the downlevel `max_texture_array_layers` is 256), and 10 for
the section index (`MAX_SECTIONS` is 1024). Thirty-six bits of sixty-four. The
spare bits are not margin to spend casually — ambient occlusion and per-vertex
light will want them.

**Packing refuses rather than truncates.** A coordinate of 17 masked into five
bits becomes 1: a corner at the far side of the section, geometrically
plausible, and indistinguishable at every later stage from one somebody meant.
There is no honest packed form for it, so there is an error instead. The bound
checked is the *section's*, not the field's — five bits hold 31, and a corner at
20 is still a bug even though it fits.

**The vertex carries no UV.** UVs are derived in the shader from a corner's two
in-plane coordinates in whole blocks, which is what makes the texture-coordinate
convention above — a face merged across four blocks shows the texture four times
— a property of the format rather than of the mesher.

A section record is **44 tightly packed bytes, eleven scalars**: three origin
components, the first quad index, the quad count, and the AABB's two corners.
Both WGSL shaders declare it as scalars rather than `vec3` because `vec3` carries
16-byte alignment and would disagree with the CPU's layout — silently, and in a
way that surfaces as a culling bug.

Every uploaded buffer is built with explicit `to_le_bytes`. **`bytemuck` is
deliberately absent from `mc-render`**: `cast_slice` is native-endian, and a byte
order that is a property of the build host is not a byte order a determinism test
can compare.

There is **no CPU index buffer**. Indices are produced on the GPU from the
section table's `first_quad`/`quad_count`; the assembled scene exposes vertex
bytes and section bytes and nothing else.

### Textures are array layers, never an atlas

One 16×16 RGBA8 layer per texture key, in a `Rgba8UnormSrgb` array texture.
An atlas is not an alternative here — layers give no bleeding, working mipmaps,
and a single block texture that can hot-reload without rebuilding everything.

**Layer indices are stated by whoever read the content, and honoured here.** A layer index sits
inside every packed vertex, so the committed goldens depend on the assignment and nothing else pins
it — which is exactly why the renderer does not work one out. `TextureLayers::stated` takes the
assignment as given and checks it against no sort: checking would be the same derivation written a
second time and would refuse precisely the assignments the mechanism exists to accept.
`TextureLayers::resolve`, which does assign lexicographically over a key set, survives as a test
convenience and has no production caller. `technical/architecture.md` §"The layer assignment is
stated, not derived" holds the reasoning and is not repeated here.

**The set the assignment covers is every block the content registers, never the set a particular
world's quads happen to reference.** `BlockRegistry::texture_keys()` (`mc-core`) declares it, reading
each definition's `texture` field and never its `name`, and `mc_sim::content::resolved_from` is the
only place a shipped assignment is built — it takes a registry, not a world, so it cannot be handed
one. No expression in the tree derives a layer index from a world: a save cannot move one because
nothing can, which is stronger than any single test guarding the property. The order the simulation
assigns today is lexicographic, and that is an implementation detail rather than a contract — nothing
downstream may reproduce it. The shipped root declares four blocks, all with `texture` equal to
`name`: `base:dirt`, `base:grass` and `base:stone` sort before `base:water`, so water appends at layer
3 and the other three keep 0/1/2 — the fact every committed golden's layer indices depend on.

**A block whose declared `texture` is not its own name is refused at packing time rather than drawn
from another block's layer**, because the entry is selected by the block's *name* today while the
assignment is keyed by the declared `texture`. The two agree only because every shipped block declares
them identically. That gap and its two call sites are recorded in `technical/architecture.md`
§"Making an edit visible: the remesh transport"; `modding/blocks-items.md` states the consequence in
an author's own terms.

The `Srgb` in the format is load-bearing: a texel is decoded to linear on sample
and the sRGB colour target encodes it back on write, so the byte a capture reads
back is the byte the texture generator produced. `Rgba8Unorm` would skip the
decode and every frame would come back lighter than any declared mean colour —
plausible-looking, and wrong in the same invisible direction everywhere.

MVP 1 ships procedurally generated placeholder textures whose colours are
deliberately implausible (teal stone, tan grass). They are not to be "corrected"
per block: a per-key colour table in Rust is a block definition in Rust, which
invariant 1 forbids. Real textures arrive as content, not as a patch.

### Appended, never renumbered: what a layer index costs across a reload

Hot reload replaces the registry whole while packed vertices the renderer already
holds carry layer indices inside them. **So a layer, once assigned, keeps its index
for the whole session.** A reload appends assignments for keys it has not seen and
changes none that it has.

The alternative was renumbering — re-deriving the assignment lexicographically over
whatever the new content declares — and it fails for a reason worth stating plainly:
a key inserted anywhere but last shifts every key after it, and every vertex the
renderer is holding then draws from somebody else's texture until the whole world has
been re-meshed and re-uploaded. That is not a transient artefact, it is a wrong
picture with no error, and it lasts as long as any un-re-meshed section does.

**The bound is 256 and it is not configurable.** Eight bits of the packed vertex
carry a layer index, so 256 is the content-to-renderer contract rather than either
side's preference. `mc-render` asserts agreement with its own capacity at compile
time; the constant is declared in `mc-core` and restated nowhere.

**The budget is spent by distinct keys ever seen, not by keys live at once.** A key
that stops being declared keeps its layer *and its texels* for the session — retired
but not reclaimed — because reclaiming means renumbering. What that costs is a
session that renames a key repeatedly running out of layers while declaring only a
handful; **relaunching reclaims every layer retired since the client started**, and
the arithmetic is exactly `spent − live`.

**Appending writes one layer rather than re-creating the array**, but the write path
iterates every live entry per call, so "append one layer" is a rewrite of every live
layer's texels. That is accepted and it is why the per-reload upload cost is what it
is. A reload's texture upload happening at all is held by the type system rather than
by review: the layers reach the frame path wrapped, and the only route to a value the
re-mesh worker will accept runs through the upload.

### A reload that changes what is drawn re-meshes the whole world

The rule is **binary**: a candidate that changes some block's declared solidity or
declared texture key, or adds or removes a block, marks every section; one that
changes neither marks none. `replaceable`, `breakable` and `breaks_into` change no
geometry.

**Selective marking was measured and refused, and the measurement is the reason.**
The shipped world's highest occupied section is 3 in fifteen columns and 4 in one, so
marking only the sections whose palettes hold a changed name, plus their neighbours,
marks about **82 of 256** — which fails the bound the spec states outright. What the
binary rule adds over a selective one is exactly the *empty* sections, and those mesh
to no quads. Narrowing this is a specification change, not an optimisation somebody
may take while passing.

**A drawn observable cannot see the difference, which is why the counting
assertions are the guard.** Marking only the lowest four sections of each column —
64 of 256 — still covers every occupied section of the shipped world, so the scenario
that watches a culled face appear once stone stops being solid passes against a
selective rule. Measured. Anything that weakens the section-count assertions removes
the only instrument that can see this.

### The compute cull pass and the single indirect draw

One entry point, **one workgroup per section**, `workgroup_size(64)`:

1. Lane 0 tests the section's world AABB against the six frustum planes — read
   from a **uniform** buffer, so they consume no storage binding — and writes
   `visible[section_index]`.
2. If visible, lane 0 reserves an index range with
   `atomicAdd(&indirect_args.index_count, 6u * quad_count)`. The atomic counter
   **is** the indirect arguments' `index_count` field, so there is no second
   dispatch and no prefix sum; the CPU zeroes that one `u32` before the pass.
3. All 64 lanes stride the section's quads, writing six indices each. Striping
   the writes is what keeps a dense section from serialising on one lane.

The indirect arguments are `instance_count: 1`, `first_index: 0`,
`base_vertex: 0`, `first_instance: 0` — **exactly one field is dynamic**. That
shape is chosen for portability, not tidiness: `first_instance = 0` avoids the
`INDIRECT_FIRST_INSTANCE` device feature and `instance_count = 1` avoids
`MULTI_DRAW_INDIRECT`, so the optional-feature set stays empty. The compaction
invariant is `index_count == 6 × Σ quad_count` over the admitted sections.

**Compaction order is nondeterministic, and that is safe only for a stated
reason.** `atomicAdd` gives no ordering guarantee, so visible sections' index
runs land in an arbitrary order. Terrain is fully opaque and depth-tested, and
the mesher's property tests assert no two quads cover the same (voxel, facing)
pair — so no two fragments ever contend for the same depth and the image is
order-independent. **The day a transparency pass arrives, or anything emits a
second quad for one (voxel, facing) pair, this reasoning expires and compaction
must become order-stable.**

### The frustum test exists twice, deliberately

The pure Rust `Frustum::admits` and the WGSL test in the cull shader are the same
maths written twice. That is duplication on purpose: one is the draw path and the
other is its independent oracle, compared by a test that reads the visible buffer
back. Merging them would delete the oracle.

Both must therefore behave identically in two respects that are easy to get
wrong. **Planes are deliberately unnormalised** — the type carries a `normal` and
an `offset`, not a normal and a distance, because nothing asks how far a box is
from a plane and the sign of `normal · p + offset` is scale-independent. And the
test is **conservative in the corners**: a box clearing all six half-spaces
individually is admitted even when it lies outside the frustum. That is the safe
direction — a section drawn contributing nothing, never a hole — and the shader
must use the same test, not a tighter one.

### The storage-buffer budget, enforced at build time

`downlevel_defaults()` allows **four storage buffers per shader stage**, and that
is the budget this design is drawn to fit. The cull shader binds exactly four
(section table, visible, destination indices, indirect arguments); the vertex
stage binds one (the section table, for per-section world origins), because
compaction lets the packed vertices be a conventional vertex buffer rather than a
third storage binding.

**A fifth storage binding in either stage is a portability break, not a
refactor** — pack into an existing buffer instead. The build script counts each
entry point's storage globals from naga's module info and fails the build over
four, so this is a compile-time fact rather than a surprise on the weakest
supported adapter.

Required *downlevel capabilities* are `COMPUTE_SHADERS`, `INDIRECT_EXECUTION` and
`VERTEX_STORAGE`, asserted against the adapter at startup. No optional device
feature is used, which is why there is **no fallback path and no second golden
set**. The day an adapter lacks one of the three, the fallback needs its own
golden.

### Depth

`mc-render` allocates and owns its depth attachment on **both** the windowed and
the offscreen path: `Depth32Float`, `depth_compare: Less`, cleared to 1.0, depth
writes enabled. The frame-capture harness supplies a colour target and never a
depth one, and a caller attaching its own depth texture in its own render pass is
adding a draw path rather than extending the harness's — which is what the
harness's one-colour-target contract permits.

The depth texture is cached and reallocated only when the surface size changes.
That decision is a pure function of the current and requested sizes, which is what
makes it testable without a device.

### One pass configuration, two targets

`TerrainPassConfig::{offscreen, windowed}` is the **single source of every pass
setting**, and there is exactly one parameterised pipeline builder. The two
configurations differ in **colour format alone** — that is the property that
keeps the path the goldens are shot through from drifting away from the path a
player sees.

`offscreen()` declares its format itself rather than reading the capture
harness's constant, so that `mc-testkit` does not become a runtime dependency of
the client; an agreement test asserts the two constants are equal.

**Clear colours are specified in linear space.** `wgpu::Color` is linear while the
target is sRGB, and the hardware performs the encode on write, so clearing to a
declared sky colour's sRGB bytes reads back visibly wrong. This is the one place
where a unit test of the conversion and a test comparing the two configurations
to each other can both pass while every shipped frame is wrong — neither looks at
the value that reaches the device. Only an assertion on a captured frame closes
it.

The camera matrices are `glam::camera::rh::proj::directx::perspective` (NDC z in
`0..1`, clip-space y **up**) and `glam::camera::rh::view::look_at_mat4`. The
`rh::proj::vulkan` sibling of the same shape is y-**down** and renders a
vertically mirrored world that compiles and runs; `rh::proj::opengl` is the
`−1..1` depth variant. Both are traps, recorded in `crates/mc-render/CLAUDE.md`.

### Shaders are validated when the crate is built

One WGSL file per pass, validated by `naga` in a build script — not at first
draw. Validation runs at the **downlevel capability profile**
(`Capabilities::empty()`), not at naga's defaults, so a shader using a capability
the supported hardware range does not offer fails on a development machine's
build rather than on the weakest adapter's first frame. The validator refuses an
empty shader directory, so a broken glob cannot pass by validating nothing.

The build script and its tests include **one source file** by `#[path]`, so the
tests exercise the exact code the build runs. Beyond validation, that code closes
the two duplications this design forces: the cull shader's six-element winding
literal must equal the Rust index pattern, and the shader's plane-axis table must
equal what `Facing` declares. The table is six rows in `Facing` declaration order
and the shader derives nothing from a facing value — a three-row axis-indexed
table would only be reachable by the very expression being guarded, and
reordering the enum would move four of six shader rows while leaving every suite
green.

### Refusals, and what recovers

- **Recording terrain before a scene has been uploaded is a refusal, not an
  empty frame.** An empty picture and a picture of a world that has not arrived
  are the same frame, and only one of them is a defect. While the scene is still
  being prepared the frame is an explicit clear reporting zero draw calls, not an
  unwritten surface texture.
- **Surface loss recovers; device loss does not.** `Lost` and `Outdated`
  reconfigure and continue; a lost device is fatal, reported, and exits non-zero
  — never retried. wgpu 30's surface result cannot distinguish the two on its
  own, so the client arms a flag from `set_device_lost_callback` and asks it when
  a surface reports `Lost`. Getting that backwards in the recoverable direction
  spins forever on a window that will never draw again.
- **A missing adapter is fatal for the binary**, regardless of
  `MYCRAFT_ALLOW_NO_GPU`. That opt-in downgrades adapter absence to an announced
  skip for GPU *tests* only; a player without a usable GPU needs an error.

### What the frame statistics observe, and what they only predict

`sections_admitted` is computed by calling the pure frustum function on the frame
path. It is a **prediction**, named so that nothing reads it as an observation of
what the GPU did; the observation is a test reading `index_count` back and
checking it equals `6 × Σ quad_count` over the admitted sections, which ties the
GPU's compaction to the CPU's admitted set quantitatively.

`terrain_draw_calls` is the constant `1`. **Nothing distinguishes one indirect
draw from one indirect draw**: a per-section CPU loop that reported `1` would
satisfy any assertion over that field. The single-draw property rests on the draw
path being built this way, not on a test that could catch its loss.

## The HUD pass

A frame is **three passes in one call**: terrain (cleared, depth-tested),
then the content-declared HUD, then the debug overlay when one is supplied.
The second and third both use `LoadOp::Load` on colour and **no depth
attachment**, so each is additive over what is already there —
which is what makes "content cannot obscure the overlay" a property of the
pass order rather than a check
(`technical/architecture.md` §"The HUD", `modding/hud.md`).

The HUD pass is **one instanced draw whatever the layout holds**: a rectangle
is an instance, its corners come from the vertex index, and nothing is read
from a vertex buffer. It binds **one uniform and zero storage buffers**, so
the four-per-stage budget above is untouched. `cull_mode: None` —
a screen-space quad has no back to face away from the camera, and a cull mode
would turn a winding mistake into an element that silently does not draw.

**The pass is recorded even when the plan is empty, and a zero-instance draw
is issued. That is a falsifiability decision, not a performance one.** With
an early return on an empty plan — the obvious optimisation, one pass cheaper
per frame — a pass that *cleared* the colour attachment instead of loading it
would leave the one assertion about "the HUD stage did nothing" green:
measured, a clearing pass alone reddens it at all 921 600 pixels, and a
clearing pass plus that early return does not redden it at all. The early
return, not the load op, would then be what preserved the picture.

**A layout of more than 256 rectangles loses the ones past the ceiling,
silently.** `MAX_HUD_RECTS` is the uniform's array length, the first 256 are
taken, and no error variant reports it. An element contributes one rectangle
for its fill plus four for its ring, so `content/base/`'s three elements
cannot reach it; a third-party mod's HUD could. Revisit when one does, at
which point the answer is a storage buffer rather than a bigger uniform.

### The rectangle derivation, pinned

Target `W × H`, `scale = H / 720.0`, `round` = half away from zero:

- `w = max(1, round(size.x × scale))`, and the same for `h`.
- `ox = round(offset.x × scale)`, `oy = round(offset.y × scale)`; +x right, +y down.
- `inset_x = round(0.05 × W)`, `inset_y = round(0.05 × H)` — **per axis, from
  that axis's own extent**. At 1280×720 that is 64 and 36.
- `center` is centred on `(W/2, H/2)` and is **not** inset. Every other
  anchor puts its named edges on the safe-area box and centres its free axis
  on the target.
- Origin from a centre `c` and extent `e` is `left = round(c − e/2)` — not
  `round(c) − round(e/2)`, which is a distinct and wrong answer for an odd
  extent.
- `H == 0` yields an empty plan and no error.
- Every rectangle is intersected with `0..W × 0..H` after offsetting, so
  nothing is written outside the target and nothing wraps to the opposite
  edge.
- An outline is one UI unit thick, scaled by the same `max(1, round(·))` rule.

**Three of those rules are graded by nothing, and a later simplification
should know which.** The one-pixel floor never fires on its own numbers — at
640×360 the scale is 0.5 and a declared height of 1 gives `round(0.5) = 1`
by half-away-from-zero *before* the floor is consulted, so `max(1, …)` can be
deleted outright with the whole suite green; what the small-target scenario
actually grades is the rounding *direction*. The `H == 0` early return cannot
be told from its absence by any observable, because a scale of 0 floors every
extent to one pixel and the clip to `0..0` then drops every rectangle anyway;
it is kept because a render pass over a zero extent is a validation error
rather than a frame nobody sees. And "a free axis centres on the target, not
on the safe-area box" is a distinction with **no** difference at any target
size: the box is `inset .. span − inset`, so its centre is `span/2`
identically. That last one is pinned so two independent implementations agree
on a spelling, and it carries no evidence either way — nothing that agrees
with it has verified anything.

### Outlines compose as a ring, in their own prior pass

Pass 1, file-name sorted: for every element declaring an outline, the four
strips of (expanded rectangle − fill rectangle), in the outline colour. Pass
2, file-name sorted: every fill.

**One pass would let a later element's outline cut a black notch through an
earlier element's fill** — which is exactly what the two crossing bars of the
base crosshair would do to each other. This is not polish; with a single pass
the shipped crosshair is visibly wrong.

**A ring rather than a solid rectangle under the fill**, because a translucent
fill over a solid under-rectangle would blend against its own outline colour
instead of the scene, and whether an element declared an outline would then
change what its own alpha means. Recorded honestly: replacing the four strips
with one solid expanded rectangle keeps the whole suite green, because no
fixture declares both an outline and a translucent fill. The rule is kept
because the day a mod declares one the difference is visible, not because
something measures it.

**An element whose paint does not resolve composes nothing at all, ring
included.** Each element's paint is resolved once, before either pass, and
both passes iterate only what resolved — one list rather than two opinions.
This came from a real defect: with the outline treated as a property of the
rectangle rather than of what fills it, the held-block swatch drew a black
26 × 26 ring around nothing at bottom-centre for every frame before the world
landed — exactly **100** pixels, which is `26² − 24²`. **The rule is keyed on
the paint being unavailable and never on clipping**: an element pushed off the
target still has a fill and still rings as far as the target reaches.

### Alpha composites in linear space, and the expected byte is derived

The colour target is `Rgba8UnormSrgb`, so the blend hardware decodes the
destination to linear, blends, and re-encodes on write. Declared colours are
converted with `mc_render::color::srgb8_to_linear` on the CPU and handed to
the shader as linear floats — the identical discipline this file records for
clear colours. **Alpha is not a colour and passes through undecoded.**

Computed, never read back from a frame: `#FFFFFF80` over `#000000FF` is
`α·1.0 + (1−α)·0.0` at `α = 128/255`, giving linear **0.50196**, which
sRGB-encodes to **0.7366469 → 187.845 → byte 188**. Not the 128 that reading
the hex digits suggests; the gap is ΔE 22.66, which no tolerance reaches.

**This is the second time this project has met this exact edge in a shipped
path, and the first — the clear colour, above — is why the second was caught
before it did damage.** The scenarios covering this blend were first drafted
expecting 128, and **they would have gone red against a correct renderer**;
the cheapest way to green them is to skip the sRGB decode of the declared
colour, which is precisely the "plausible-looking, wrong in the same
invisible direction everywhere" defect the clear-colour paragraph describes,
arriving through a test instead of through the renderer. Deriving the
expected byte from `α` and the two declared colours is what separated the two
readings.

**Byte 188 means two opposite things here, so it is never a verdict on its
own.** For a 50%-white-over-black blend it is the *correct* answer. For a
fully opaque `#808080FF`, whose correct answer is **128**, byte 188 is the
signature of a *missing* sRGB decode — the same arithmetic arriving from the
other direction, since an un-decoded 128 hands the shader the same 0.50196
that blend produces legitimately. Which one a 188 means depends entirely on
which fixture produced it.

**Only a mid-tone fixture can grade the decode at all**, which is worth
knowing before writing another colour assertion: **0 and 255 are fixed points
of the sRGB transfer function**, so at those two bytes a correct decode and a
bare `channel / 255` produce the identical linear value and the identical
pixel. Every colour the base HUD and most of its fixtures declare is built
from those two bytes alone, and against them `srgb8_to_linear` can be deleted
from the HUD path with everything green. `decode(128/255)` is linear
**0.2158605**, which the target re-encodes to byte **128.000**; skipping the
decode renders **188**. Sixty bytes apart. The one assertion that grades this
declares `#808080` and expects `#808080`, which reads as a tautology and is
not: the identity is the *result of two inverse operations* and fails the
moment either goes.

### The swatch borrows the terrain's own array texture

A `block-texture` element samples the **terrain's** array texture and sampler
(`Nearest`, `Repeat`), lent through a private accessor, rather than
allocating a second array filled from the same layers: a swatch of a block
has to be the texture that block is drawn with, and two arrays are two
answers waiting to disagree. The view outlives every upload into it, so a
bind group built once stays valid.

A rectangle carries its array layer as a third vector component, **negative
for a flat fill** — the shader compares against zero, so one component
carries both the question and the answer. The shader **samples
unconditionally and `select`s** rather than sampling inside the branch:
`textureSample` computes its own derivatives and wants uniform control flow,
and while a per-instance layer *is* uniform across a triangle, that is a fact
about the data rather than one the compiler must accept.

Two things about a textured rectangle are read by nothing today. The colour
it carries in the uniform is given opaque white and the shader `select`s the
sampled texel over it, so any value would render identically — white is
chosen so that the day it becomes a tint it is a neutral one rather than a
value that would annihilate the swatch. And the **alpha the pass writes to
the colour attachment** is graded by nothing: overwriting the destination
alpha rather than accumulating coverage keeps everything green, because every
colour assertion drops the alpha channel and every whole-pixel comparison is
over regions the HUD did not paint. Invisible in an offscreen capture; a
presented surface is where a destination alpha of 0.5 can reach a compositing
window manager.

### The HUD's derived prediction, and why the duplication is the oracle

The rectangle derivation above is written **here**, in a document, rather than
in either of the two places that implement it — because the frame assertions
about the shipped crosshair and swatch are graded by a **predictor computed
from the content declarations alone**, sharing no code with
`mc_render::hud`. If either implementation were the other's source, the
prediction would follow the thing it grades. This has an exact precedent one
section up: `Frustum::admits` and the WGSL frustum test are the same maths
written twice, deliberately, because merging them would delete the oracle.

The prediction is in fact derived **three** times, and the third is what
checks the second: this document's rule, the predictor over the parsed
declarations, and a hand derivation of the six rectangles at 1280×720 in the
test's own header. Every scenario refuses to run unless the second and third
agree at all three elements *and* the footprint union comes to **733**
pixels. A defect in the predictor is therefore caught by the hand derivation
rather than shared with the code it grades — the property that would be lost
the day somebody merges the predictor with `mc_render::hud::compose` to
remove the duplication.

**The standing hazard that independence buys, stated because nothing
mechanical prompts it:** the predictor keeps its **own copy** of the outline
thickness constant. Measured — with the renderer at two units and the
predictor at one, the per-pixel colour assertion stays **green** (a ring drawn
two pixels thick still paints black at every position a one-pixel ring would
have) and only the area-based assertions redden. So the day an outline
thickness legitimately changes, the predictor's constant has to be updated by
hand, or the per-pixel check goes on grading positions that are no longer the
ones drawn. Deriving it from the renderer would close this and would delete
the oracle, so it is not closed. Same shape as the retired-name hole in
`technical/architecture.md`: the entry belongs to the commit that makes the
change.

### The HUD's capture id

`mc_render::capture` declares `HUD_CAPTURE_TICKS = [0]` and
`hud_capture_id(tick, revision)`, and `declared_capture_ids` returns terrain
ids ++ HUD ids — four directories. **`SCENE_REVISION` was not bumped**:
nothing about the mesh contract changed, and bumping would have renamed and
forced a re-shoot of exactly the frames being preserved.

**One HUD golden, at tick 0, not three.** The HUD does not animate and the
held block is set once, so ticks 59 and 119 would assert the same rectangles
a third time against different terrain. Tick 0 is the frame with the least
terrain coverage (77.91%, measured below), so the crosshair stands against
the most sky.

**The three committed terrain goldens were not re-shot, and did not move.**
They were minted against a terrain path an independent ray-marched oracle had
already judged at 441/544/542 sample pixels with zero disagreements;
re-shooting would replace references with that provenance by references whose
provenance is "the day the HUD landed", and would hide any terrain regression
introduced in the same commit. Freezing them makes the merge condition a
**zero-byte diff** under `crates/mc-render/goldens/player-walk-t*/`, which is
a stronger statement than a re-shoot can make. Any movement there is a stop,
not a cost. The honest price is recorded in the re-shoot section below: the
terrain set no longer traverses the client's exact frame call, and only the
pair of sets covers the product path.

## Re-shooting a golden set

The committed goldens live in `crates/mc-render/goldens/<capture-id>/`, one
directory per capture id, each holding `default.png` and its provenance
sidecar. `mc_render::capture::declared_capture_ids` is the authority on which
directories may exist, and `crates/mc-render/tests/golden_inventory.rs` fails
when the set on disk is not exactly that list — a stale directory left behind
by a previous scene revision is as much a defect as a missing one.

Two sets stand under that root and they are shot through different calls: the
three `player-walk-t*` captures through `record_terrain`, and
`player-walk-hud-t000-r1` through `record_frame`, the one frame call the windowed
client makes. Only the pair covers the path the product draws through, so neither
is retired in favour of the other and both are minted by the procedure below.

**Regenerate through `terrain_goldens` and `hud_goldens` and nothing wider. A run
that reaches `golden_mismatch` with the opt-in set corrupts the set.**

```
# 1. The probes first. They are derived from a declared pose, world and
#    colours, so they are the only thing that can tell a correct renderer from
#    a broken one before a broken one becomes ground truth.
cargo nextest run -p mc-client --test terrain_probes

# 2. The oracle next. The probes judge a declared pose; this judges the frames
#    the goldens are actually shot through, against the world's own voxels.
cargo nextest run -p mc-client --test replay_oracle

# 3. The HUD prediction, for the same reason and about the other half of the
#    frame. It judges every pixel the content declarations predict against a
#    derivation that shares no code with the composition, with no area budget.
#    The default tolerance step 5 applies forgives 92 wrong pixels and the base
#    crosshair's fill is 17, so this is the only thing that can tell a correct
#    HUD from a broken one before a broken one becomes ground truth.
cargo nextest run -p mc-client --test hud_prediction

# 4. Mint. Whole binaries, both of which hold only self-comparisons.
MYCRAFT_UPDATE_GOLDENS=1 cargo nextest run -p mc-client --test terrain_goldens \
    --test hud_goldens --no-tests=fail

# 5. Verify with the opt-in unset, including the mismatch path and the inventory.
cargo nextest run -p mc-client --test terrain_goldens --test hud_goldens \
    --test golden_mismatch --no-tests=fail
cargo nextest run -p mc-render --test golden_inventory
```

**`MYCRAFT_UPDATE_GOLDENS` must never be set for a run that selects
`golden_mismatch`.** That binary holds one test, which deliberately verifies the
*tick-59* capture against the *tick-0* golden, because the compare-and-fail half
of the lifecycle is the half a passing suite never exercises. Under the update
opt-in that test does not compare: it mints, and it writes a tick-59 frame as
tick 0's committed reference. Every later run then compares the right frame
against the wrong ground truth and passes forever, and the diff that would have
shown it is a binary blob nobody can read. So the danger extends to a bare
`MYCRAFT_UPDATE_GOLDENS=1 cargo nextest run`, which selects everything.

That is the failure the whole golden discipline exists to prevent — a golden of
a renderer nothing checked — arriving through the regeneration procedure rather
than through the renderer. The ordering rule that goldens are shot only after
the derived probes pass does not cover this door, which is why the mint command
is written down here rather than left to be reconstructed.

**The mint step names binaries, not tests, and that is deliberate: a document
meant to be followed verbatim must not embed an identifier that a refactor moves
silently.** This command used to carry `-E 'test(matches_its_committed_golden)'`,
a suffix three separate tests shared; when they collapsed into the one
table-driven test the scenario asks for, the filter matched nothing and minted
nothing. A binary name is a file name — `crates/mc-client/tests/terrain_goldens.rs`
and `crates/mc-client/tests/hud_goldens.rs` — so renaming one is a file move a
diff shows, and the mint-unsafe test was given its own binary rather than being
excluded by name so that this selection needs no filter at all. Both binaries
named at step 4 hold only judgements that are safe to mint: each capture is
judged against *its own* golden, and `hud_goldens`'s other scenario compares two
frames of one run against each other and reads no golden at all.
`--no-tests=fail` is what turns a selection that has decayed to zero matches into
a failure at mint time rather than a silent no-op discovered at step 5.

Any re-shoot is a deliberate change that says so in its commit message. An
unexplained golden update in a diff remains a review stop. When the cause is a
change to the mesh contract rather than to the renderer, bump
`mc_render::capture::SCENE_REVISION` instead of overwriting: the ids carry the
revision, so the set is *renamed*, the commit shows added and removed files, and
the inventory test forces the previous set out rather than letting it linger.

## What golden-frame verification cannot see

**This section is now about terrain, and no longer about the whole frame.**
Goldens and cluster-share probes verify terrain at *large scale*, and neither
can see a scattered handful of pixels — structural rather than a gap to be
tuned away. The content-declared HUD is the one part of the picture that
**is** graded per pixel, by the derived prediction above rather than by a
golden; the last part of this section says what that does and does not buy,
and what remains unseen for both halves.

- **A golden encodes whatever the renderer did.** It is minted from the renderer
  it verifies, so any artifact present at minting is baked into the reference and
  the comparison agrees with it forever. This is why the derived probes and the
  HUD prediction are made to pass before any golden is shot — but that ordering
  rule only covers artifacts they can detect.
- **The committed set samples three ticks of a 120-tick walk** — 0, 59 and 119,
  at 1280×720 — plus one HUD capture at tick 0. Anything that appears only at
  another tick passes the whole suite untouched, and anything present at a
  sampled tick is baked into that tick's reference.
- **Share-based probes are blind to sparse pixels by construction.** A coverage
  assertion with a 0.25% floor over a 1280×720 frame cannot be moved by a few
  hundred pixels spread across a replay. Measured while building SPEC-004: 231
  isolated pixels across all 120 ticks, 0–9 per frame, moved no cluster share at
  all, and two of the three terrain goldens contain some.
- **A golden's blindness has a precise shape, and it is not "small means
  invisible".** `Thresholds::default` carries a **ΔE 10 hard ceiling that is
  checked first**, before the 0.01% area budget, so *gross* wrongness of even
  two pixels is a mismatch whatever share of the frame it is — measured: a
  two-pixel defect at ΔE 100 failed the golden, and a one-pixel placement
  error failed it at 536 pixels. What the golden is blind to is **drift inside
  ΔE 10 over a small area**, which is exactly the crosshair case: every
  crosshair fill pixel rendered at `[242, 242, 242]` where `[255, 255, 255]`
  is declared is ΔE **4.506** — above the per-pixel tolerance of 2.0, below
  the ceiling — and 17 failing pixels sit under the 92-pixel budget, so
  **the golden returns Match while the entire crosshair is the wrong
  colour.** So "a golden-frame test asserts the crosshair is present" is
  satisfied by a frame whose crosshair is uniformly wrong, and read strictly
  it understates what the harness would still have caught.

**What the player's camera actually shows, measured rather than predicted.**
Non-sky coverage from the published camera is **77.91% at tick 0, 95.01% at
tick 59 and 94.22% at tick 119** — measured in `crates/mc-client/tests/` by
comparing each rendered frame against a uniform field of the declared clear
colour through the harness's own metric, at the commit that first shot the
frames from the player. SPEC-005's architecture predicted ≈48% and ≈75% from a
design-time re-implementation of the heightmap that answered `surface_height(32,
32) = 40`; the shipped generator answers **37**, so the eye stands at y = 41.62
rather than 44.62 and a player three blocks lower on a hillside rising to 48 is
looking *into* terrain rather than over it. The prediction was not stale, it was
wrong by 30 and 20 points, and the figures above are what a future drift should
be read against.

Those figures are what a **coverage floor** would have been checked against, and
this is why the player's camera has no coverage floor. A 15% floor against a
frame that is 78% terrain is three to five times of slack: a renderer drawing a
third of what it should would pass it. What judges these frames instead is a ray
marched from the published camera through the world's own voxels, at 576 sample
pixels per frame, predicting terrain at 441 / 544 / 542 of them and disagreeing
with the rendered frame at **none**. The budget is 2 disagreements per frame and
it may not be raised: the remedy for a sample landing on a silhouette edge is to
move that sample, which is a declared fixture. Measured sharpness of that
budget: a pitch error of 0.25° reaches 2 disagreements, 0.5° reaches 5 and 3°
reaches 24.

Those 231 pixels turned out to be *correct* geometry — the exposed vertical side
face of a dirt voxel at a terrain step of two blocks or more, or on the world's
cut edge, seen nearly edge-on so it projects to roughly one pixel and winks in
and out as the sample point crosses it. Confirmed by an independent ray-cast
sharing no code with the renderer: 38 dots checked, 38 agreements, and the voxel
beyond the entered face not solid in every case.

The lesson is not about those pixels. It is that **a defect of that size would
have been equally invisible**, so *terrain's* pixel-scale correctness rests on
reading the code and on a human looking at the window, not on the golden set.
When a change could plausibly produce sparse artifacts there, verify it with a
per-pixel oracle or accept that nothing automated is watching.

### The HUD's 733 pixels are the exception, and the exception is bounded

**The content-declared HUD is graded per pixel with no area budget**, against
the derived prediction described above — so the sentence about reading the
code and looking at the window is no longer true of it. 733 pixels are
predicted from the declarations alone: every pixel of both crosshair bars and
their ring judged within ΔE 2.0 of the predicted composite, one failing pixel
being a mismatch; every pixel *outside* the predicted footprints required
equal to the zero-HUD frame; the swatch's footprint required to be covered
exactly, with the pixel immediately outside it unchanged. The chain has both
controls a prediction needs: applied to a frame rendered with zero HUD
elements it must report a mismatch, and applied to a frame where the
crosshair's fill pixels are rendered in its declared *outline* colour it must
also report one — so "something was drawn there" does not satisfy it.

**Four things it still does not see**, each measured rather than supposed:

- **The shipped elements' contrast outline.** The prediction derives the ring
  *from the declaration*, so deleting `outline` from a shipped file makes the
  prediction predict no ring, the frame draw none, and the two agree. **A
  prediction that follows its input is not an oracle for that input.** What
  covers it instead is a content assertion over the parsed declarations
  (`technical/architecture.md` §"Mechanically enforced invariants") — and
  that is the *only* thing covering it: with the field deleted by hand, that
  one test is the sole failure in the workspace, while every frame assertion
  about the footprints stays green, because each is a frame-to-frame equality
  that a missing ring satisfies **on both sides**.
- **The overlay's text.** No committed golden may hold rasterised text
  (ADR-017): drivers disagree about glyphs, and the first golden to hold one
  makes whatever rasterised it the ground truth every machine must reproduce.
  The overlay is hidden by default and no declared capture is taken with it
  shown. What that leaves ungraded is recorded in
  `docs/technical/testing.md` §"Nothing can tell a painted readout from a
  painted rectangle".
- **A vertical flip is caught by less than it looks.** Three of the four
  rectangles the offscreen HUD fixtures assert against are *vertically
  symmetric about the target midpoint* and map to themselves under
  `y → H − y − h`; only the `9 × 1` bar does not. So an un-inverted shader y
  axis was originally caught by two assertions and nothing else, and a later
  change to that bar's extents would silently take the falsifier away. The
  shipped swatch and crossbar footprints widened it to six once they existed,
  and the HUD's own placement inversion — a different mutation from the
  clip-space inversion `crates/mc-render/CLAUDE.md` warns about — reddens
  fifteen.
- **Terrain, still.** Everything above the sub-heading applies unchanged. The
  HUD prediction says nothing about the world behind it.

## Relationship to the frame-capture harness

These conventions are asserted, not merely stated: `docs/technical/testing.md`
describes the harness that enforces them and the tolerance model comparisons
are judged against. See that file for how a captured frame is verified, and
`docs/technical/decisions.md` (ADR-008, as narrowed by ADR-013) for why
golden-frame comparison is the coverage strategy for GPU-resident rendering code
at all, and why the pure layer beside it gets no such exemption.
