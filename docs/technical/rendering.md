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
the **drawn voxel that emitted the face** — never of the face itself, so
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
because block identity and what each block declares are resolved into
**contents-ordered** keys before the sweep ever runs: one pass over the
section's own voxels in their linear (x-fastest) order builds one entry per
*distinct block the section actually holds*, in first-encounter order, and
the six shared faces are then keyed into the same table. Nothing registry-local or
history-dependent survives into that pass's output, so nothing
registry-local or history-dependent can reach the emitted quads either.

**Golden frames captured against this mesh are reproducible only because of
this property.** A renderer capturing goldens is trusting that two sections
with the same voxels — however they got written, compacted, or reordered by
a registry swap — mesh identically every time; this is why that trust is
warranted structurally rather than by convention.

### An empty cell is answered before the registry

A cell holds a block or nothing (`technical/world-format.md`), and a
palette entry holding nothing resolves to **neither drawn nor occluding,
without the registry being consulted at all**. That is one arm in one place, and it covers both
the meshed section and every supplied neighbour — which is what keeps "an
empty cell shows no face" a single rule rather than two that could drift
apart at a chunk boundary, where the difference would show as a seam only
under a specific neighbour supply.

Two consequences worth stating, because both are load-bearing:

- **A section holding nothing anywhere meshes into a mesh carrying no
  quads, even against a registry holding no block at all.** Emptiness is
  not an unresolved block, so it earns no `UnresolvedBlock` refusal.
- **Nothing about the sweep changes.** Emptiness reaches it as a cell that
  is not drawn and does not occlude, and an undrawn cell already emitted no
  face. `visible_face`, the
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
adjacent voxel held nothing — deliberately, because treating an unloaded
neighbour as solid would hide the streaming edge behind a silently missing
face, and a visible wall at the world edge is correctable in a way a silently
missing surface at every chunk boundary is not.

A supplied neighbour resolves **only the palette entries its 256
facing-plane voxels reference** — narrower than the rule for the meshed
section itself, which resolves every palette entry any of its 4096 voxels
reference. A block a neighbour holds away from the shared face is never read
and can never fail the mesh.

### Which faces exist: three questions, and one of them is an engine rule

A face is emitted at a cell only when all three of these answer yes. The first
two are read off the declarations either side of the face; the third is derived
by the engine and stated in `visible_face` and nowhere else.

```
drawn(self) && !occludes(beyond) && key(self) != key(beyond)
```

1. **Is this block drawn?** `BlockDefinition::drawn` — the only question asked
   of the cell showing the face. A block declared solid and undrawn is an
   invisible barrier: it reaches the sweep and emits nothing.
2. **Does whatever is beyond it fail to occlude?** `BlockDefinition::occludes`
   of the neighbouring cell. Separate from the first because a block may be seen
   without hiding what is behind it, which is the whole of what makes water look
   like water — and a block may hide what is behind it without being seen or
   being solid.
3. **Is whatever is beyond it a different block?** **A block never draws a face
   against its own kind.** Without this a body of water is a stack of visible
   sheets, one interior face per cell; with it, a sea shows only its surface and
   its edges.

None of the three is derived from `is_solid`, which now means collision and
nothing else. They coincide across the four blocks the base game ships, and
deriving any of them from another would put that accident in the engine where
content could not override it.

**The third is a rule, not a declaration — and PRO-952 is its named breaker.**
It is evaluated over **key identity**: keys are handed out per distinct
*contents* over a table deduplicated by name, so `key(self) != key(beyond)`
compares identity and reads neither a block name nor a runtime id. `visible_face`'s
standing property — that no name and no runtime id is looked at anywhere in the
file, which is what makes a mod's block behave exactly as the base game's — is
therefore preserved by the comparison rather than weakened by it.

What that costs a mod author, stated because it is not currently askable: **two
adjacent cells holding the same non-occluding block show no seam between them,
and there is no field that turns the rule off.** The value that would do it is
`merges_with_self = false`, whose one identified use is drawing the interior
faces of a translucent volume — per-pane glass — and translucency is PRO-952.
The day that spec lands, this rule has to become a declaration; until then the
engine's answer is the only answer.

Until the shipped water declaration made water drawn, no shipped content ever
reached this third question: `occludes(beyond)` culled first every time. That
was measured rather than assumed — the clause was deleted and `scene_contract`,
`replay_world` and all four of the then-committed **`r1`** captures stayed green,
over a world whose sea emitted no faces at all. **That measurement cannot be
re-run against the set in front of you.** The `r2` captures were minted from a
world in which water is drawn, and from that declaration onward the sea is
exactly the case where the first two questions hold and the third decides.
**So a mutation that removes this clause and moves the goldens is the rule
beginning to bind, not a defect** — which is the reading that measurement was
taken for, and the one a future engineer will otherwise get backwards.

#### The boundary plane carries a key, not a flag

`Boundaries` is `[[Key; 256]; 6]` — one key per cell of each of the six faces a
section shares with its neighbours, over **the same key table as the section
being meshed**, resolved second. A plane of booleans has no room for the third
question, and the boundary is exactly where it has to be asked: a body of water
spans sections, so a mesher that could only answer it inside a section shows a
sheet at every chunk edge.

Four properties a future change must not break, recorded here rather than
reconstructed later from the code:

1. **The meshed section is keyed before its boundaries.** This is what makes
   `UnresolvedBlock` outrank `UnresolvedNeighbourBlock` when a section and a
   neighbour both hold something unresolvable, and what keeps a refusal naming
   the lowest voxel of the meshed section's *own* linear order rather than the
   lowest of the two sections' union.
2. **Key 0 is `Contents::Empty`**, seeded before any voxel is read. An
   unsupplied neighbour's plane is therefore all zeros, which is "nothing
   beyond" — it occludes nothing and is not the same kind as anything. Absence
   stays a value the sweep reads rather than a branch it tests for.
3. **Keys never reach the output.** They order nothing a caller sees. Their
   numeric values are deliberately unobservable, and the seeding in (2) shifts
   every key by one relative to a table that did not seed — which is harmless
   only for as long as this stays true.
4. **Only the 256 voxels of each shared face are ever read.** A block a
   neighbour holds away from that face still never reaches the registry and
   still cannot refuse a mesh it could not have appeared in.

`Key` is `u16`. Its ceiling is the 4096 voxels of the meshed section plus the
256 of each of six shared faces — 4096 + 6 × 256 = **5632** distinct blocks,
*derived* from those shapes rather than measured, and far inside the type.

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

**Read that chain in the right direction: AO invalidates the goldens because AO
shades corners, not because the merge predicate narrowed.** A change to *how the
same visible faces are cut into rectangles* moves no pixel at all — texture
coordinates come from a corner's own section-local position and the terrain
sampler repeats on every axis, so one 4×1 quad and four 1×1 quads emit the same
texels at the same depths. **The goldens are not the witness for merge shape**; the
exact-partition property tests are, and `technical/testing.md` §"Four instruments
guard the mesh" sets out which instrument sees what. AO is the change that makes
merge shape visible for the first time, and that is precisely why it invalidates
them.

### Terrain magnifies with `Nearest` and minifies with `Linear`

`mc_render::texture::sampler::TERRAIN_SAMPLER` is the whole of it: `Nearest`
magnification, `Linear` minification, `Linear` interpolation between mip levels,
an anisotropy clamp of one, and `Repeat` addressing on every axis. ADR-027 holds
the reasoning; what matters when reading a frame is the two halves.

**Nearest magnification** because a block texture is sixteen texels of
deliberate pattern, and linear magnification interpolates between texel centres
— which blurs a near face towards its own mean, and the mean is the value the
frame probes cluster captured pixels against. A magnification-filtered frame
would agree with the probes for a reason that has nothing to do with the texture
being right. `crates/mc-render/tests/terrain_sampling.rs` measures the
consequence rather than the request: a scanline across a face magnified to 9.2
screen pixels a texel shows both of the layer's colours and **nothing between
them**, where linear magnification turns 116 of 116 scanned pixels into blends
that are neither.

**Linear minification with a full mip chain** because a distant face falls under
a pixel, and point sampling then answers with *one* texel — so a sub-texel camera
movement flips whichever pixels crossed a texel boundary by the whole contrast
between two texels. That is a shimmering hillside. The same suite measures it
from the other side: two captures half a texel apart move far fewer pixels past
ΔE 10 through the terrain sampler than through a request that minifies without
filtering, and the unfiltered pair is **required to move at all** before the
comparison is read, so a fixture that drifted out of the regime fails loudly
instead of reporting that filtering helped when neither configuration could see
anything.

**Anisotropy is refused, and the device is what refuses it.** wgpu accepts an
`anisotropy_clamp` above one only when magnification, minification and mip
interpolation are all `Linear`, in three separate arms of its own validation, so
anisotropy and crisp voxel magnification cannot both be had. `terrain_sampler`
builds the request inside a `wgpu::ErrorFilter::Validation` scope and maps a
captured error to `RendererError::TerrainSampler { requested }`, which carries
the **whole** request because the vendor's rule is over the combination and names
no single field. There is deliberately no pre-check on this side: a rule copied
out of a vendor is a second copy that drifts silently the day the vendor changes
it.

**The sampler is a value, and that is what makes the pair above writable.**
`SamplerRequest` is threaded from the composition root as one half of
`gpu::TerrainTextures`, so a capture can ask for a *second* configuration and the
difference filtering makes is measured against a run that does not have it.
Without the parameter, `buffers::terrain_sampler` would be a private free
function reaching for a constant, and a test could only read back the descriptor
it caused to be built — which is agreement between two copies of one decision,
not a statement about a picture. Both halves are owed:
`crates/mc-render/src/texture/sampler_test.rs` asserts what is asked for, with a
positive control on the anisotropy inspection; `tests/terrain_sampling.rs`
asserts what a device does with it.

#### An image's basis is not the geometry's plane pair

`PLANE_AXES` says which two components of a corner's position a quad's primary
and secondary extents were written into. **It is the geometry's table**, read by
`placed()`, so a row changed in it moves the mesh rather than the texture.

Where an image's own left-to-right and top-to-bottom directions go is a separate
question, and `IMAGE_SWAPS` and `IMAGE_SIGNS` answer it: whether a face's image
runs its horizontal along the pair's secondary rather than its primary, and
whether either coordinate runs against its axis. **A pair of axis indices cannot
express either**, and reading the pair alone as though it were an image basis is
what drew five of six faces turned.

Three facts make all three tables differ per facing, and each is why one of them
exists:

- An image's rows run **downward** while the world's vertical axis runs up, so
  every face with world up in it needs its vertical coordinate negated.
- Two faces looking at each other along one axis see their in-plane axes in
  **opposite** horizontal order, so one of each pair needs its horizontal
  negated — without which north and south are forced to share a direction and
  one of them draws laterally reversed.
- An `X` face's plane pair is `(y, z)` with `y` primary, so its image's
  horizontal runs along the **secondary**.

Both image tables are **derived** from each facing's own outward normal rather
than tabulated: a viewer outside a face looks along its inward direction with the
world's up as their up, and the image's right edge is forward crossed with up.
The two horizontal facings have no world up in them, so theirs is chosen to match
what `voxforge` bakes — top image's top edge toward `-z`, bottom image's toward
`+z` — and **the bottom row was measured wrong and corrected on the strength of
the bake rather than of any test**, because a block's underside is never seen in
this world. No test discriminates either horizontal row today; the first
anisotropic top or bottom texture owes one.

**`build/validate.rs` checks all three against the shader's literals and that is
not evidence they are right.** It text-compared one of them for five increments
while three hand-written copies agreed with each other and all three were wrong.
What can say a table is right is a reading of a drawn face: FR-8.1-S7 for where
its bands sit, FR-8.1-S8 for which way it runs.

**Changing any of this re-shoots every golden.** Same shape as the AO warning
above and it belongs beside it: a sampler change moves every terrain pixel. It
is a deliberate, measured change that re-shoots the set and says so in its commit
message, never a quiet quality improvement.

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

The refusal is deliberate: treating an unresolvable block as neither drawn nor
occluding punches a hole in the world; treating it as both seals a cavity; both
are silent and indistinguishable from a correct mesh at the call site. A failed
mesh is not a breach of the "a bad mod never takes down the server"
invariant — nothing panics, and a caller running meshing on a worker simply
keeps its previous mesh.

`MeshError::EmptyBlockFace { key }` is a fourth variant and an **internal
invariant**, not anything a caller did: emptiness resolves to undrawn and
a face is emitted only where the voxel is drawn, so no quad can name the
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

- **Only `drawn` blocks produce faces, and drawnness is declared.** A block
  is drawn because its declaration says so, never because it is solid — see
  "Which faces exist" above. `base:water` declares `solid = false, drawn =
  true, occludes = false`, so the sea is meshed and shows its surface and
  edges while the lakebed shows through. It is drawn **opaque**: there is
  still no transparency pass, no alpha and no sorted draw, and that is
  PRO-952 rather than a limitation of the mesher.
- **The merge predicate keys on a deduplicated key, and the deduplication is
  held by one module and by review, not by the test suite.** A run merges
  while the next cell shows an uncovered face of the same `Key`, and keys are
  handed out one per *distinct block the section actually holds*, over a table
  deduplicated by name. Handing out a key per palette *position* instead
  leaves the whole suite green: `Section::set_block` cannot build a palette
  naming one block twice, and no test builds one through `Section::import`
  either. It matters because `Section::import` *does* accept such a palette —
  its registration check verifies every named block is registered, not that
  the palette's names are unique — so a position-keyed table would refuse a
  legitimate merge and make the mesh depend on import history rather than on
  content. Whether `import` should reject a duplicate-name palette outright is
  owned by the section import/export work, not settled here.
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

### Mip levels are averaged in linear light, and every one of them is uploaded

**Status: wired.** The array texture is created with `mip_level_count:
MIP_LEVELS` and `write_layer` writes **every** level of the chain, each at its
own edge — a level nobody wrote is whatever the allocator left there, and a
minified face samples it. The sampler interpolates between the two levels either
side of the detail it wants, so the whole chain is live in every frame a distant
face appears in.

**The rule the arithmetic exists to enforce, and item 4 of what a future change
must not break.** The array texture is `Rgba8UnormSrgb`, so a texel is decoded
to linear light on sample. A level averaged over the **stored** bytes is
therefore not the average of what the sampler will see. Stored 0 and 255 average
to stored 128, which decodes to linear 0.216 rather than 0.5; every level comes
out darker than the one above it, and a minified surface darkens as it recedes.
Decoding first, averaging in linear light, and re-encoding puts the same pair at
stored **188**. The fault is plausible-looking and wrong in the direction
nothing notices, which is why the byte is pinned exactly rather than as "midway
between": a test written to accept anything in the middle accepts both
implementations.

**188 separates three implementations, not two.** Averaging the stored bytes
gives 128. A gamma-2.2 approximation of the transfer function — the shortcut
that looks close enough — gives 186. Only IEC 61966-2-1 itself gives 188, and
`to_linear`/`to_stored` implement that curve including its linear segment near
black. The pair round-trips all 256 stored bytes exactly, which is what lets a
uniform image survive every level unchanged; `to_stored` rounds to nearest and
clamps, because truncating loses that round trip and darkens a flat colour by up
to a byte per level.

**Why exact byte equality is safe here, and what the margin is against.** These
are `f32` round trips through a transfer function, so the assertions rest on a
measurement rather than on the arithmetic looking clean. The **tightest margin in
the whole chain is 0.016 of a byte**: 188's pre-rounding value is 187.516, and
187.5 is the boundary that would round it to 187 instead. What that is a margin
*against* is the transfer pair's own error — the worst pre-rounding error over a
round trip of all 256 stored bytes is **1.53e-5 of a byte**, at stored 132. Three
orders of magnitude, so exact equality is what the arithmetic supports and a
tolerance would have to span 187 to 188 to matter — which would then admit the
stored-byte fault this whole section exists to exclude. **Anyone changing the
transfer functions has 0.016 of a byte of room at that point**, not the byte a
reader might assume.

**Colour is averaged in linear light; alpha is not.** `Rgba8UnormSrgb` decodes
RGB through the transfer function and alpha linearly, so averaging alpha where
it stands is what the format means rather than what anybody preferred. Nothing
in the test suite discriminates the two treatments, because every texture the
project ships is fully opaque and both answer 255 for a constant 255. **The
first translucent texture must bring a test with it.**

**Why the chain is built on the CPU.** wgpu has no built-in mip generation, so
the alternative is a blit or a compute pass — which would put the arithmetic
inside `src/gpu/`, the one subtree excluded from coverage thresholds
(ADR-008, narrowed by ADR-013), where golden frames are the only defence. A box
filter over a 16×16 chain is five levels of trivial arithmetic and a pure
function, so it sits outside `src/gpu/` under normal coverage and its correctness
is read directly rather than inferred from a picture. `crates/mc-render/CLAUDE.md`:
anything expressible as a pure function is not exempt.

**The surface, as built.**

| Item | What it is |
|---|---|
| `texture::MIP_LEVELS: u32` | `TEXTURE_EDGE.ilog2() + 1`, five for a 16-texel edge. **Derived, never written as `5`** — a size and a level count that can disagree is a copy that overruns |
| `to_linear(stored: u8) -> f32` | one stored sRGB byte as linear light on `0.0..=1.0` |
| `to_stored(linear: f32) -> u8` | the inverse, rounding to nearest and clamping |
| `reduced(level, size) -> Vec<[u8; 4]>` | one level below `level`, where `size` is the **source** edge. Row-major: the output texel at `(r, c)` covers sources `(2r, 2c)`, `(2r, 2c+1)`, `(2r+1, 2c)` and `(2r+1, 2c+1)`, and no other four |
| `chain(level_zero, size) -> Vec<Vec<[u8; 4]>>` | every level, the first being `level_zero` **verbatim**, halving to a single texel |
| `levels_for(key, supplied, size)` | the levels a layer is filled from: the built set's art where it covers `key`, and `placeholder_texels(key, size)` where it does not |

**An uncovered key is an ordinary answer, not a failure.** A mod author's first
block declares a texture nothing has drawn yet, and `levels_for` generates one
from the key rather than refusing the launch. The refusal a launch does make is
about the *set*, and it is the client's. This is why `placeholder_texels` is not
deleted when real art arrives: it becomes the documented per-key fallback.

**The two refusals, both naming the key.** `TextureError::WrongTexelCount`
answers supplied texels whose count is not `size · size`, and
`TextureError::TooFewLevels` answers a chain shorter than the array texture
declares. Both carry `{ key, offered, declared }`, where `offered` is what the
layer has and `declared` is what was wanted — two different questions that
happen to share a shape, since one counts levels and the other counts texels. A
layer refused without its key leaves a reader with every key in the content root
to choose from.

**Where the texels come from.** `SuppliedTexels` — a key, and the `[R, G, B, A]`
stored bytes of its level zero — is the only thing that crosses between the
client's decoder and this arithmetic. `mc-render` has no `std::fs`, no `PathBuf`
and no image decoder anywhere in `src/`; it is *handed* level-zero texels and
computes the rest. The read and the PNG decode live in
`crates/mc-client/src/textures/decode.rs`, the one file of the composition root
that names `image::`, and
`crates/mc-client/tests/the_decode_stays_at_the_composition_root.rs` holds both
halves of that boundary with a positive control on each scan.

**The supply is given once, at construction, and no reload carries one.**
`TerrainRenderer::new` takes `gpu::TerrainTextures` — the texels and the sampler
request together — and `SceneBuffers` keeps the texels for the whole run. A
reload hands over *layers*, never a supply, so the second upload re-fills every
layer from the same texels the first did. The alternative was a value that can
arrive **empty**: a world drawing its baked art would go back to hash-derived
colours the moment somebody saved a block file, and no reading that takes its
texels from a launch would report it.

### A face draws what its block declared, and a `Quad` carries no key

`build_section_geometry` takes a `TextureResolution` — every registered block's six declared keys
beside the layer assignment, as one value — and answers block + facing → key → layer. **Nothing on
this path parses a block's name.** The facing is carried into the vocabulary a declaration writes by
`Facing::face` (`mc-world`), which is the one place in the workspace where a compass word meets an
axis; the key comes out of the block's own declaration; the layer comes out of the assignment.

**A `Quad` carries no resolved texture key, and that absence is the load-bearing part of the design.**
Resolution happens where vertices are built, not where quads are meshed, **because a retained mesh is
re-packed and never re-resolved at mesh time**: `Retained::rebuilt` re-packs the *entire* retained
list on every batch, against whatever resolution the re-mesh worker currently holds. Stamping a key
into a `Quad` when it is meshed re-introduces a stale key on the one path built not to re-mesh —
silently, as a plausible wrong picture, with no error anywhere. A future change may add fields to a
`Quad`; a resolved key or layer is not one of them.

For the same reason `TextureResolution` carries **no `ContentSerial`**. A bundled value invites being
stamped with one so that "packed against the content serving" becomes checkable, and it must not be:
retained quads are packed against a *newer* resolution than the one they were meshed under on
purpose, and a serial checked at the packer would refuse exactly the case that path exists for.

**The refusal names the block, the facing and the key**, and fails the whole section:
`GeometryError::UnresolvedTexture { block, face, key }`, where `key: None` means the content states no
such block at all — a section still holding quads for a block a reload dropped — and `Some` means the
key that block declares there occupies no layer. Falling back to layer 0 would draw whichever block
owns layer 0, which is the plausible wrong picture nothing downstream can report.

**One question, two consumers.** The held-block indicator resolves through the same
`TextureResolution`, at `INDICATOR_FACE = Face::North` (`src/hud/held.rs`) — a side face, because a
side is what makes the canonical block recognisable, and stated once rather than implied by whichever
facing a lookup happened to reach for. `FrameRenderer::texture_resolution()` is what a swatch is
looked up in, lent from the renderer, because it has to be what the array texture was filled from.
Closing one site and leaving the other would draw a block correctly in the world with a blank
indicator beside it, which reads as a HUD fault and sends whoever chases it to the wrong module.

What a key's *pixels* are is a separate question and is still answered by the procedural placeholder
generator, one texture per key from the key's own spelling. Baked art from disk arrives later in this
spec.

The `Srgb` in the format is load-bearing: a texel is decoded to linear on sample
and the sRGB colour target encodes it back on write, so the byte a capture reads
back is the byte the texture generator produced. `Rgba8Unorm` would skip the
decode and every frame would come back lighter than any declared mean colour —
plausible-looking, and wrong in the same invisible direction everywhere.

The generator that produces those texels is still here and is still the fallback
for a key nothing has baked, and its colours are deliberately implausible (teal
stone, tan grass). They are not to be "corrected" per block: a per-key colour
table in Rust is a block definition in Rust, which invariant 1 forbids.

**Every key the base game declares now draws baked art instead**, and it arrived
as content rather than as a patch — eight images baked by
`voxforge build content/base/textures.toml` from models and materials under
`content/base/`. Seven landed on 2026-08-19; `base:water` landed on 2026-08-26,
three days late, and what it drew in the meantime is recorded in
`technical/testing.md` under "An oracle can be right and useless".

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

The rule is **binary**: a candidate that changes some block's declared `drawn` or
`occludes`, or **any of the six keys its faces draw from**, or adds or removes a
block, marks every section; one that changes none of those marks none. `solid`,
`targetable`, `replaceable`, `breakable` and `breaks_into` change no geometry.

The key is a **map from name to `(drawn, occludes, textures)`** rather than a
list, so a re-ordered declaration file is not mistaken for a geometry change.

All six keys and not one: a block whose `north` alone was re-pointed is a block
that draws differently, and a comparison reading a single key would accept that
edit and mark nothing at all — the reload would succeed and the world would never
be built again to show it.

**Solidity left this key when `solid` was split, and its removal reads as a
regression until you see what replaced it.** While one bit answered every question
about a block, keying on `is_solid` stood in for *drawnness* rather than meaning
collision. Now that a declaration states the two apart, keeping solidity here would
rebuild all 256 sections for a physics edit that changes not one pixel. **A
declaration that edits `solid` and states nothing else still marks the world** —
because `drawn` and `occludes` default to whatever that declaration says about
`solid`, so editing it moves them too. That is the loader's default doing the work,
not this key, and the distinction matters to whoever changes either: a declaration
that states `drawn` explicitly and then edits `solid` alone now marks nothing, and
that is correct.

**`targetable` is not among them either**, and it is the clearest case: what a
swing can find is not something a section could show. A reload changing only
`targetable` is accepted, publishes an advanced serial, and rebuilds **zero**
sections.

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

**Two facts together are what make a retained-but-not-re-packed section unreachable
today, and only one of them is this rule.** A texture-key change marks every section,
and `Session::take_remesh_work` drains the whole dirty set into **one** batch — so
there is not even a partial-drain window in which a section is retained, not
re-meshed, and drawn against the new resolution. Neither fact on its own is enough.

**Bounding the re-mesh batch turns that state into a production path.** A whole-world
re-mesh measured at 9.1 ms is exactly the thing somebody will one day bound, and the
moment a batch is capped, sections retained out of it are drawn while the content
they were meshed under has stopped serving. **Whoever bounds it must make the
retained sections re-pack against the serving resolution before that batch is
drawn.** `Retained::rebuilt` does that today by re-packing the whole retained list on
every batch; a bounded batch that re-packed only what it re-meshed would leave the
rest drawing keys nobody is playing, with no error anywhere. That is why resolution
lives where vertices are built and not in a `Quad` — see §"A face draws what its
block declared, and a `Quad` carries no key".

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
the most sky. **That ranking held at `r1` and has since reversed twice** — the
two wet frames move whenever the sea's declaration does, and the margin is now
around two points either way. `mc_render::capture::HUD_CAPTURE_TICKS` carries
the measured figure for every revision; do not read the sentence above as
current.

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

**A revision bump renames the set, and the rename is a deletion and a fresh mint
— never a `git mv`.** Step 4 writes nothing for a capture that still *matches*
its golden (`GoldenOutcome::GoldenUnchanged`), and it only ever looks for a
golden under the current revision's id. So moving the old directories into the
new names and minting leaves every unmoved frame carrying a provenance sidecar
that names the **superseded** capture — an `r3` sidecar sitting inside an `r4`
directory — and nothing reports it: `golden_inventory` reads directory names and
the comparison reads pixels, so neither instrument looks at the sidecar. Delete
the old directories first and let step 4 mint all of them through `on_missing`.
Measured on the 2026-08-27 re-shoot below, where two of four frames were
unchanged and both kept their `r3` sidecars until they were deleted and
re-minted.

### Re-shoots on record

**2026-08-19, the spec that gave the base game's blocks real art.** All four
committed captures were re-shot at `r1`, in place, and **`SCENE_REVISION` was not
bumped.** Neither half of that is a shortcut and both are worth the paragraph.

**Why the pixels moved.** Two independent changes, landed together. The shipped
key set went from four keys to eight — `grass.luau` declares six facings, so
`base:grass` stopped being a key at all and every layer index after `base:dirt`
renumbered: stone 2 to 6, water 3 to 7. A layer index rides inside every packed
vertex, so that alone re-shoots the set. And every one of those layers is now
filled from a baked PNG rather than from the generator, read through a sampler
that magnifies with nearest and minifies with linear over a full mip chain
(ADR-027).

**Why `SCENE_REVISION` stayed at 1.** It identifies the *scene contract* — pose,
world, camera path, tick list, merge predicate, vertex format — and this spec
changes none of them. Bumping it would redefine the revision as "something
visible changed" and oblige a bump for every future art edit, which is exactly
the reading the ids exist not to carry. The set is therefore overwritten rather
than renamed, and the diff shows four modified files and no added or removed
directory. `golden_inventory` was run after the mint and reports the committed
directories as exactly the ones the current revision declares.

**What was verified before the mint, in the order this section prescribes**, all
at `861d093`: `terrain_probes`, `replay_oracle` and `hud_prediction` — nineteen
tests, nineteen passed. Then the mint, naming only the two golden binaries. Then
the set re-verified with the opt-in unset and `golden_mismatch` selected — four
passed — and `golden_inventory` — three passed. The provenance sidecars are
byte-identical: the same adapter, backend and driver produced both sets.

#### The second re-shoot, 2026-08-19, and why there was one

**The first mint of this set photographed a renderer drawing five of its six
faces turned.** East and west were a quarter turn out, north and south and the
underside were flipped, north was laterally reversed as well, and only the top
was right. The goldens then enshrined it, which is the exact failure the
procedure above exists to prevent — arriving through the procedure, with every
step of it followed.

**No stage could see it, and that is the part worth keeping.** Every reading this
spec built measures *which colours* a face holds: FR-8.1-S3 asks the four side
images to be pairwise unequal, and a rotation keeps them unequal; FR-8.1-S5
judges the top face's colour, and the top face was correct; the terrain probes
compare means, and a mean is invariant under rotation, reflection and
permutation; so are the distinct-colour counts, the pairwise ΔE figures and the
landmark shares. **None of them measured where a colour sits.** The one property
that mattered — which edge of a side face carries the turf — had no witness
anywhere in 1370 tests. Not a defect in any test: a hole in the scenario set.

**It was found by the project owner looking at the picture.** That is the
standing rule in `technical/testing.md` about a green suite being no evidence,
collecting on the largest thing this project has built.

`FR-8.1-S6`, `S7` and `S8` are what close it: the bake against the model it is a
view of under all eight dihedral transforms, where a drawn face's bands sit, and
which way a drawn face runs. The fix is `IMAGE_SWAPS` and `IMAGE_SIGNS`, and
`decisions.md` ADR-028 records why an image basis is not the geometry's plane
pair.

**This re-shoot was forced by a defect and not chosen**, which is the difference
from the first one and the reason the one-re-shoot bound was not violated by it:
the bound exists so that a set is never minted from a renderer nobody has judged,
and re-minting is what serves that rather than what breaks it. Same procedure,
same order, `SCENE_REVISION` again untouched, four files modified and no directory
added or removed.

#### Why no reference capture was taken between the two pixel changes, this once

The paragraph above names two independent changes, and this section's own advice
would be to land one, capture an uncommitted reference frame, then land the other
— so a bad diff has a bisect point. **That was not done, and the reason is
recorded here rather than left to read as a step somebody skipped.**

The two do not compile apart. `gpu::TerrainTextures` is one borrowed parameter
carrying both the texels a layer is filled from and the sampler request it is
read through, and it is the parameter that makes either of them expressible: the
sampler was a private free function taking no request, so nothing under `tests/`
could build a renderer that sampled any other way. Splitting the commit would
have meant an intermediate tree whose sampler constant is still the old one,
reddening the scenario that asserts the new one, for no gain — the reference
frame would have been a picture of a tree the branch never had.

**What stands in its place is stronger than a bisect point, and that is the part
worth keeping.** A reference capture is unattributable on its own: it tells you
the two frames differ and nothing about which is right.
`crates/mc-client/tests/the_grass_top_the_camera_sees_is_its_baked_image.rs`
marches a ray through the world's own voxels from the declared pose, picks the
first pixel whose ray — and all four of its neighbours' — meets the upward face of
one `base:grass` voxel, and judges the colour there against a mean computed from
the built PNG, **decoded by the client's decoder and never by the draw**. It was
green before the mint. So was every probe, over strata re-derived from the real
art. That is an independent judgement of *this* picture, made before any golden
existed to agree with — which is the thing a reference frame cannot give you and
the thing the ordering rule at the top of this section is really about.

**The advice is not withdrawn.** Where two pixel changes *can* be landed apart,
land them apart. Where they cannot, the substitute is a judgement of the picture
that shares no path with the golden — not a frame nobody can attribute.

#### 2026-08-23, the spec that made the sea visible — and the first bump of `SCENE_REVISION`

**`r1` → `r2`, four directories deleted and four added.** This is the first entry
in this log where the revision moves, and the two above explain why they held it
at `r1`: *art* changed while the scene contract did not, and bumping for an art
edit would have redefined the revision as "something visible changed". **This
time the contract itself moved**, in two of the six things it names.

**What the merge predicate emits moved first, and moved no pixel.** The shipped
water declaration gained `drawn = true`, `occludes = false`, so the sea began
emitting faces and the replay's quad count went **2759 → 2770**. Every committed
`r1` golden still matched afterwards — thirteen golden and probe tests green on a
real device — because from the declared spawn the sea was behind terrain at every
capture tick. **The sea emitted faces no declared capture could see.** A reader
who expects "water became visible, so the goldens moved" will find that it is not
what happened, and could check it against `r1` and be right.

**The camera path moved second, and that is what re-shot the set.** The declared
spawn moved from an inland column to the coast, because no yaw from the old one
saw water at any of the 120 ticks. That is the change every pixel in the `r2` set
comes from.

**`r2` was minted twice on this branch, and the account matters more than the
tidiness.** The first mint was taken from a spawn that was **not** the
derivation's own output: the candidate ranking put an optional property above the
water margin the requirement rests on. Re-ranking exposed a worse fault — the
candidate filter never required the spawn column to stand above the sea, so the
corrected ranking's top was a submerged column. The set was re-minted from the
corrected column once that constraint was restored. **The fault was a dropped
constraint, not a bad candidate**, which is the part worth carrying: a criterion
can be wrong in a way re-sorting it cannot reveal.

The revision does not move again for either mint. `r1` was already deleted, and a
spec branch squashes, so `main` goes `r1` → `r2` exactly once however many times
`r2` is re-minted before merge. The one-re-shoot bound is a statement about what
`main` carries and what a later spec must not redo, not a budget on in-branch
corrections.

**What was verified before each mint, in the order this section prescribes:**
`terrain_probes` 9 of 9, `replay_oracle` 5 of 5, `hud_prediction` 6 of 6 — twenty
tests before a pixel became ground truth. Then the mint, naming only the two
golden binaries. Then the set re-verified with the opt-in unset and
`golden_mismatch` selected — 4 passed — and `golden_inventory` — 3 passed. One
adapter across the set.

**No intermediate reference capture was taken, and the first reason is that an
extra capture is not free.** The one failure this procedure exists to prevent is a
run reaching `golden_mismatch` with `MYCRAFT_UPDATE_GOLDENS` set; every further
occasion on which that opt-in is set is another chance to reach it. Declining the
intermediate capture **reduces** that exposure rather than merely saving effort.
The second is that nothing was destroyed: the changes are separate commits, so
anyone debugging the minted set can check out the earlier one and capture from
there. The capture was not forgone — it was left uncomputed until somebody needs
it. And in this instance it would have carried **no information at all**: the
declaration change was demonstrated to move no pixel, so a capture taken between
the two would have been byte-identical to the `r1` set it was meant to be
compared against.

#### 2026-08-25, the spec that made the sea swimmable — and the bump the tripwire could not ask for

**`r2` → `r3`, four directories deleted and four added.** The second bump, and
the first whose cause is neither the mesh nor the spawn. **The world is
unchanged**, so `SCENE_QUAD_COUNT`, `total_face_area` and `area_by_block` all
hold and `scene_contract.rs` stays green throughout — **5 tests run, 5 passed**,
taken on the tree carrying the re-shot set — and two committed frames are
invalidated anyway. What moved is the **physics the declared walk runs
under**: `base:water` gained `swimmable` and a `move_resistance`, the divisor
acts on carried-forward velocity, and the scripted player wades through the sea
from tick 44 onward.

**This is what falsified the constant's own documentation**, which said the
revision is bumped for a change to the *mesh contract* and that the scene
contract "is the tripwire that fails first". Both halves were false here. The
sentence had been true when written — the camera derived from the spawn and the
spawn from the world, so the trajectory *was* a function of the mesh contract —
and content-declared physics is a third input that severs that chain. The doc was
**stale, not permissive**, and a reader taking it as permissive would have
concluded no bump was needed. It now states what the tripwire can and cannot see,
and says plainly that **the second case has no guard at all**. A tripwire over
the declared camera path is recorded as deferred, not built: correcting a false
sentence is repair, adding a guard is a feature.

**All four directories re-shot, including the two that could not move**, and the
premise was measured rather than argued. Hashing the minted `r3` images against
the `r2` blobs they replace: `player-walk-t000` **byte-identical**, and so is
`player-walk-hud-t000` — git recorded both as 100 % renames — while
`player-walk-t059` and `player-walk-t119` both moved. Tick 0 is dry, and so is
tick 44, the first *wet* tick, because the medium is read at the **start** of a
tick and the first tick whose outcome differs is 45. **That sentence is what makes
the unmoved frame a derivation rather than a coincidence**, and it is what should
stop a later reader filing an unmoved first-wet capture as a bug. The declared
walk's pose moves **1.0832 blocks at tick 59 and 3.2973 at tick 119**.

**The churn this ruling accepted is therefore measured rather than estimated, and
it is exactly two frames.** The decision to re-shoot all four was taken knowing
two would reproduce byte-identically but before anyone had confirmed the other two
were the *only* ones that moved; a third moving frame would have meant the physics
reached a capture the architecture had measured as dry, which is a finding about
the simulation rather than a reason to re-shoot fewer. It did not. So the
asymmetry the ruling turns on is real: bumping cost two captures that reproduce identical
images, while not bumping would have left `player-walk-t059-r2` meaning one thing
before this spec and another after — a name silently redefined, permanently, in a
repository that archives its specs and expects them to be re-readable.

**What was verified before the mint, in the order this section prescribes:**
`terrain_probes`, `replay_oracle` and `hud_prediction` selected together —
**21 tests run, 21 passed**, a bare count and so a complete run. Then the mint,
naming only the two golden binaries. Then the set re-verified with the opt-in
**unset** and `golden_mismatch` selected — 4 passed — and `golden_inventory` —
3 passed. One adapter across the set. The whole of `mc-client` and `mc-render`
afterwards: **493 tests run, 493 passed**.

**The oracle survived the moved poses untouched, and that is worth recording
rather than passing over.** `PREDICTION_FLOOR` is slack by design — 100 against a
tightest judged frame in the low three hundreds — and `DISAGREEMENT_BUDGET` is 2,
so both held across a camera that moved several blocks. The reading that could
have needed a moved sample, `the_sea_the_camera_sees_is_the_water_layer`, passed
as well: no predicted sample straddles the sea's edge at the new poses, so the
remedy that section prescribes had nothing to apply to. **A green pre-mint suite
across a moved camera is evidence those tolerances are honest**, not evidence they
were never asked anything.

#### 2026-08-26, the spec that gave the sea art — and why the revision held at `r3`

**All four directories re-shot in place, `SCENE_REVISION` unmoved.** The same
shape as 2026-08-19 and the opposite of the two bumps between: *art* changed and
the scene contract did not.

**What every one of the four was a photograph of.** `base:water` was a declared
texture key with no baked image, so its layer was filled from the generated
stand-in and the sea drew a checkerboard of two magentas — `(140,38,131)` and
`(160,58,151)`, only ΔE 7.21 apart and ΔE 70 from black, the darker of which
reads as near-black against a lit sky. Counting those two values and the
`(150,49,141)` they minify to — the colour observed in the blobs, one off the
pair's arithmetic mean `(150,48,141)`, which occurs 42 times and gives 61 380 —
verbatim, in the committed `r3` blobs: **77 987 pixels at tick 0 and in the HUD
capture, 165 232 at tick 59, 191 792 at tick 119** — and counting the trilinear
blends between them as well, **88 280 at tick 0 and in the HUD capture, 174 744
at tick 59, 198 828 at tick 119**, which is 9.58 % to 21.57 % of the frame.
Every committed reference image showed it, which is exactly why
`terrain_goldens` could not: it compared a frame of the defect against a
photograph of the defect and matched. The detection gap is recorded in
`technical/testing.md` under "An oracle can be right and useless".

**Why the revision held, and it is not the same argument as "the pixels barely
moved".** Layers are assigned positionally over the keys the *block declarations*
name, lexicographically. `base:water` was **already** one of those eight keys and
**already** sorted last, so baking an image for it adds no key and moves no index
— which is the whole difference from 2026-08-19's four-keys-to-eight change, where
every index after `base:dirt` renumbered and a layer index rides inside every
packed vertex. No spawn, physics or camera-path input moved either, which is what
forced `r1`→`r2` and `r2`→`r3`. Bumping for an art edit would redefine the
revision as "something visible changed" and oblige a bump for every future art
edit, which is the reading the ids exist not to carry.

**The extent of the sea did not change, and that was measured rather than
assumed.** The reading is a set comparison rather than a count: the pixels that
**differ** between the old committed blobs and the new ones, against the pixels
the old blobs drew from the stand-in — every colour in the RGB box the two
checkerboard values span, `(140, 38, 131)` through `(160, 58, 151)`, which
contains the minified mean and every blend between them. In all four captures the
two sets coincide in **both** directions: no changed pixel lies outside the old
stand-in region, and no stand-in pixel was left alone. The totals are **88 280,
88 280, 174 744 and 198 828**, t000 and the HUD capture agreeing exactly. What
changed is the colour of the sea and not one pixel of where it is. Being
*position*-identical rather than merely equal in count is what rules a renumbered
layer out outright rather than by inference: a wrong layer index draws water on
different surfaces, so it would have put changed pixels outside the old magenta
region, and there are none.

**What was verified before the mint, in the order this section prescribes:**
`terrain_probes`, `replay_oracle` and `hud_prediction` selected together with
`the_sea_the_camera_sees_is_the_water_layer` — **22 tests run, 22 passed**, a bare
count and so a complete run. Then the mint, naming only the two golden binaries —
3 passed. Then the set re-verified with the opt-in **unset** and `golden_mismatch`
selected — **4 passed** — and `golden_inventory` — **3 passed**. Four files
modified, no directory added or removed, and the provenance sidecars
byte-identical: one adapter, backend and driver across both sets.

**The sea reading's tolerance was re-derived and did not have to move.**
`the_sea_the_camera_sees_is_the_water_layer.rs` judges a pixel against the water
layer's own mean at ΔE 8. Both ends of its bracket moved that day — the layer's
own texel spread from ΔE 3.71 to **3.16**, and the nearest wrong answer from
`base:stone` at ΔE 62.40 to **25.34**, because a blue that belongs in the same
palette as the ground it meets stands nearer to it than a deliberately
implausible magenta did. 8 was inside both brackets. **That it still stands is a
measurement and not an omission**, which is why the file's header now says so.

#### 2026-08-27, the spec that made the sea's rates content-declared

**`r3` → `r4`, four directories deleted and four added.** The third bump, and the
second whose cause is the declared physics rather than the mesh or the spawn.
SPEC-030 moved what `content/base/blocks/water.luau` declares — `move_resistance`
`1.6` → `0.5`, and a `swim_ascent` of `3.5` where the field did not exist — and
the scripted walk wades that sea. **The world is unchanged**, so `scene_contract.rs`
holds throughout, which is the second time the case `SCENE_REVISION`'s doc comment
records as having no guard is the case that actually arrives.

**Two of the four frames moved and two did not, and the split is the same one
PRO-957 derived.** Read before the mint with the opt-in unset: tick 59 differed at
**350 085** of 921 600 pixels, worst distance **73.923**, and tick 119 at
**341 511**, worst distance **75.850**; tick 0 and the HUD capture matched. Tick 0
is dry — the spawn is still falling and inland of the coast — and the HUD does not
animate. Re-minted after deletion, both unchanged frames came back
**byte-identical**, which git records as 100 % renames. That is an independent
reading that the capture is deterministic across the change, and it is why the
unmoved pair is a derivation rather than a coincidence.

**What the mint's skip cost, measured.** The first attempt renamed the four
directories with `git mv` and then minted. It passed every check in this
section — the comparison, `golden_mismatch` and `golden_inventory` all green —
while `player-walk-t000-r4` and `player-walk-hud-t000-r4` still held sidecars
reading `"capture": "player-walk-t000-r3"`. Nothing in the suite reports that, and
it is what the deletion rule added above exists to prevent.

**Verified in the order this section prescribes**, on the tree carrying the
retune: `terrain_probes`, `replay_oracle` and `hud_prediction` — **21 tests run,
21 passed**. Then the mint, naming only the two golden binaries. Then the set
re-verified with `MYCRAFT_UPDATE_GOLDENS` unset and `golden_mismatch` selected —
**4 passed** — and `golden_inventory` — **4 passed**. The provenance sidecars are
byte-identical apart from the capture id they name: the same adapter, backend and
driver produced both sets.

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
- **What the scripted walk never does is invisible to the goldens, and that was
  measured against a declared physics field.** Sampling ticks bounds *when* the
  frames look; the replay's own inputs bound *what* they can ever exercise.
  SPEC-030 moved both of water's rate fields and the two came out opposite:
  `move_resistance` `0.5 → 0.6` reddens `terrain_goldens`, because the walk
  wades the sea and a resistance changes where it is at tick 59 and 119, while
  `swim_ascent` `3.5 → 4.0` leaves them **green** — the walk never holds jump
  under water at any captured tick, so no frame has ever depended on the rate a
  swimmer rises at. An ascent shipped without a scenario stating its rise in
  absolute blocks would have been guarded by nothing whatsoever. **Read a
  declared value against the replay's inputs before crediting a golden with
  guarding it**: that a field reaches the physics the frames are shot through
  does not mean the frames reach the field.
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
be read against. **That drift has since happened, twice**, both times because
content-declared physics moved the two wet frames while tick 0 stayed
byte-identical: `57.11 / 71.05 / 55.00` at `r2`, `57.11 / 66.63 / 50.95` at `r3`
and `57.11 / 67.62 / 59.20` at `r4`. `HUD_CAPTURE_TICKS`' own doc comment is
where that table is kept current, and it carried the `r2` row through the `r3`
bump unmeasured — a doc comment is read by nothing, so nothing reported it.

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
