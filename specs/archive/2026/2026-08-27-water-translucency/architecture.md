# Architecture Delta — SPEC-031

Spec: `spec.md` (45 scenarios, approved 2026-08-27; FR-1.3-S2 added in phase 1) · issue **PRO-993** ·
work-type feature · rigor **high** · branch `feature/PRO-993-water-translucency`.

Standing architecture (`docs/technical/architecture.md`, `docs/technical/rendering.md`)
supplies the crate map, the tick model, the capture procedure and the golden
revision rule. Nothing here re-derives them; this document designs only the
delta the spec declares.

**Every figure below names the command or the arithmetic that produced it.** Where
a line says *derived*, it is arithmetic — over the sRGB transfer function or over
`src-over` — and not a reading of any renderer. Where it says *measured*, a
command is named.

**This draft has been through the architecture review round** (`persona-architect`,
Mode B). Section 10 records every verdict and what it changed; three blockers and
eight majors were raised and all are folded above rather than noted below.

---

## 1. Drivers

| # | Driver | Where it comes from |
|---|---|---|
| D1 | A block declaration must state a degree of opacity, bounded `0.0..=1.0` inclusive, refused in a sentence naming file, block, field and bound, hot-reloadable in both directions | FR-1, FR-6 |
| D2 | A declared opacity must reach the pixel, and the blend must be the arithmetic composition of two layers **in linear light** | FR-2 |
| D3 | The alpha channel must survive baking, mipping and sampling, and the mip module's alpha treatment must become testable | FR-3, `crates/mc-render/src/texture/mip.rs:74-78` |
| D4 | The ordering model's behaviour — correctness or a named artefact — is itself a deliverable | FR-4.1-S2 |
| D5 | Depth against opaque geometry stays exact; adjacent cells of one kind still show no seam; a submerged camera gains no whole-frame tint | FR-4.2, FR-4.3 |
| D6 | The replay oracle must stop assuming every drawn block is opaque | FR-5.1, `crates/mc-client/tests/support/oracle.rs:55-72` |
| D7 | A save written before the field exists must still open, at opacity `1.0`, moving the **appearance** revision byte only | FR-5.3 |
| D8 | Invariant 1: no block name reaches the mesher or the renderer | FR-7.1, CLAUDE.md |
| D9 | An opacity field and PRO-952's later `render = "<method>"` must coexist on one declaration | Dependencies, `docs/planning/block-render-methods.md:140` |

### Constraints the tree imposes, measured

| Constraint | Command / reading | Consequence |
|---|---|---|
| **Four storage buffers per shader entry point**, enforced at build time | `grep -n "STORAGE_BUDGET" crates/mc-render/build/validate.rs` → `166: const STORAGE_BUDGET: usize = 4;`, counted over globals in `AddressSpace::Storage` reachable from an entry point at `:363-390` | The cull pass already binds four (sections, visible, indices, args). **No new storage buffer may be added to it.** This single constraint writes the shape of Decision 4 |
| The terrain draw is **one** `draw_indexed_indirect` over a GPU-compacted index buffer | `crates/mc-render/src/gpu/record.rs:163` | A second pass means a second index range and a second `DrawArgs`, both inside buffers that already exist |
| Terrain pipeline cannot blend | `grep -n "blend" crates/mc-render/src/gpu/pipeline.rs` → one hit, `272: blend: None` | Re-confirmed on this tree |
| The only blended target in the workspace | `grep -rn "blend: Some" crates/ --include=*.rs` → one hit, `crates/mc-render/src/gpu/hud_pass.rs:269` | Re-confirmed. Its `BlendState` is reused verbatim |
| **Every setting of the terrain pass except the colour format is a one-variant type**, and the colour format is the only thing a caller may choose | `crates/mc-render/src/pass.rs:6-11` and `:13-21` | Decision 4's parameter may not enter `TerrainPassConfig` |
| The loader's only numeric reader has **no ceiling** | `crates/mc-world/src/content/luau_declaration/number.rs:116` `optional_number_at_least_zero` | Decision 7 |
| The mesher's quad order **may not be re-sorted** | `crates/mc-world/src/mesh/sweep.rs:11-17`: "The order is the loop nesting and never a sort… putting them back in order afterwards would write the order down a second time" | Decision 5 — the partition may not live in the mesher |
| Back faces are culled, unconditionally | `crates/mc-render/src/gpu/pipeline.rs` `front_face: Ccw`, `cull_mode: Back`; `pass.rs:41-55` gives `CullMode` and `FrontFace` exactly one variant each; corners wound outward per `crates/mc-render/src/geometry/mod.rs:11-25` | Decision 3's correctness argument |
| Compaction order is **not** reproducible between runs | `crates/mc-render/shaders/cull.wgsl:22-26`, verbatim: the reasoning "expires" the day a transparency pass arrives | Decision 3's stated limit, and a **fourth standing debt** the brief did not name |
| **Three** modding pages quote the recognised-field list, and a test-owned mirror holds it too | `crates/mc-world/src/content/luau_declaration/mod.rs:80-83`; `docs/modding/{blocks-items,hot-reload,README}.md`; `crates/mc-client/tests/support/quoted_refusals.rs:66` `[&str; 12]` | Decision 1 |
| **A session's texture-layer budget only ever goes up** | `crates/mc-core/src/content.rs:280` `let mut next = self.spent;` — `spent` never decreases; the refusal at `:216` says relaunching is what reclaims them | Decision 6 — this is what kills the per-layer alternative |
| A declaration may hold 64 enumerated field names | `crates/mc-world/src/content/luau_declaration/mod.rs:106` `FIELD_NAMES_READ = 64` | A 13th recognised field needs no change here |
| The packed vertex uses 36 of 64 bits, and the spare ones are **explicitly reserved** | `crates/mc-render/src/geometry/vertex.rs:16-18`: "not a design margin to be spent casually — ambient occlusion and per-vertex light will want them" | Decision 6 spends 8 of 28, and must justify it against that sentence |
| The shipped content declares four blocks and no translucency | `ls content/base/blocks/` → `dirt, grass, stone, water`; `content/base/blocks/water.luau` states no opacity field | A3 |

---

## 2. Boundaries

No new external dependency, no new crate, no new port. Every change is inside
boundaries `docs/technical/architecture.md` already draws.

| Seam | Today | After |
|---|---|---|
| `mc_core::block::BlockDefinition` | 11 declared fields | + `opacity: Opacity` |
| `mc_core::block::Opacity` | — | **new**: a finite `0.0..=1.0` scalar, and the one definition of its 8-bit quantisation |
| `mc_core::content::ResolvedBlock` | name, textures, is_solid | + `opacity` |
| `mc_render::texture::TextureResolution` | block → six keys, plus layers | block → six keys **and its opacity**, plus layers |
| `mc_world::mesh` | unchanged | **unchanged** — see Decision 5 |
| `mc_render::geometry` | quads → corners | quads → corners, **partitioned**, plus `opaque_quad_count` |
| `mc_render::gpu` | one pipeline, one draw | two pipelines, two draws, one render pass |
| `mc_render::pass::TerrainPassConfig` | unchanged | **unchanged** — see Decision 4 |

Dependency direction is unchanged and still flows inward: `mc-core` gains a type
and depends on nothing new; `mc-render` reads it; `mc-world` reads it only to
fold a save record.

---

## 3. Decisions

### Decision 1 — BINDING: the field is `opacity`, a number, appended to the recognised list

`opacity = 1.0` passes no light (today's behaviour), `0.0` passes all, absent
means `1.0`. Both bounds inclusive. Spelling is the architect's per FR-1.

**Why `opacity` and not `transparency` / `alpha`.** The spec's own prose says
opacity throughout, and the sense runs the right way for a default: a declaration
that says nothing gets `1.0`, and `1.0` is the value that means "as it always
was". `alpha` names the implementation (a channel) rather than the property.
`transparency` would invert the default to `0.0`, which reads as "no
transparency" and is the same number a mod author would reach for meaning
"invisible".

**Position in `RECOGNISED_FIELDS`: appended, 13th.** The list stands at 12
(`crates/mc-world/src/content/luau_declaration/mod.rs:85`) and has grown twice by
appending.

**Four mirrors move together, and one of them is not the implementer's to move.**
`mod.rs:80-83` states it: "Three pages quote this list today:
`docs/modding/blocks-items.md`, `docs/modding/hot-reload.md` and
`docs/modding/README.md`. Growing it means editing all three, and the guard
sweeps every page under `docs/modding/` rather than a named one, so a page missed
is a page reported." The quotations are at `blocks-items.md:457` and `:468`,
`README.md:183`, and `hot-reload.md:437`. The fourth mirror is
`crates/mc-client/tests/support/quoted_refusals.rs:66`
`FIELDS_IN_THE_ORDER_THE_GUIDE_STATES: [&str; 12]`, compared **whole and in
order** — and at rigor `high` it is **test-author-owned**, so the implementation
context may not edit it.

> **Sequencing, binding on the tasks phase:** the 12 → 13 move in
> `quoted_refusals.rs` belongs in the **test author's** commit, together with the
> page edits' expected text. An implementer who finds it at 12 sends a dispute,
> never an edit.

*Rejected:* inserting after `occludes` to group the appearance fields. It reads
better, and it costs four quoted-string edits **plus** a re-ordering failure in a
guard whose entire purpose is to notice re-orderings — which is the wrong guard to
spend. Deferred as F5.

**Non-foreclosure of PRO-952 (D9).** `opacity` answers *how much light passes
through*; `render = "<method>"` answers *what shape and motion a face is drawn
with*. They are orthogonal and compose on one declaration:
`opacity = 0.6, render = "liquid"` is meaningful and neither field's reader has
to know the other exists. The one place they could collide is a render method
that wants to choose its own alpha source; Decision 2 keeps that open by making
the *product* the rule rather than the scalar alone.

### Decision 2 — BINDING: per-pixel alpha is the **product** of the declared opacity and the texel's alpha (candidate **A3**)

**A2 is foreclosed, not merely rejected.** FR-1.1-S1 requires a declared opacity
of `0.5` to be readable from the registry and FR-2.1-S1 requires that same
declared `0.5` to produce an even blend at a pixel. A model taking alpha from the
texture alone cannot satisfy either. This is an elimination on the spec's own
text, and it is why the axis is A1 vs A3.

**A1 vs A3, and the evidence that separates them.** A1 (declared scalar alone)
ignores the texel's alpha entirely. Under A1, **every scenario of FR-3 tests dead
code**: `mean_of_stored` (`crates/mc-render/src/texture/mip.rs:180`) would reduce
an alpha channel nothing reads, `levels_for` would carry it to a layer nothing
samples, and FR-3.3-S1's `WHERE` arm would never be entered. FR-3 exists because
`mip.rs:74-78` records a treatment nothing can tell right from wrong; A1 leaves it
exactly that way while adding tests that look like they closed it. A3 gives that
whole chain a consumer.

The cost of A3 over A1 is **one multiply in the fragment shader**. Both need the
identical loader work (Decision 7), the identical vertex work (Decision 6) and the
identical pass work (Decision 4).

**The rule, stated once because both halves are load-bearing:**

> The **declared** opacity alone decides which pass a face is drawn in — a face is
> translucent iff its block declares `opacity < 1.0`. The **texel's** alpha
> modulates within the blended pass and nowhere else.

So a block declaring `opacity = 1.0` whose texture happens to carry alpha below
255 draws opaque. That is deliberate: it keeps the partition a function of a
declared, greppable, hot-reloadable number rather than of art, and cutout
transparency is Out of Scope. It is one sentence in `docs/modding/blocks-items.md`.

**Measured, so that A3 is safe today:** every texel reaching the array is opaque
by two independent roads. Generated stand-ins:
`crates/mc-render/src/texture/placeholder.rs:60` `const OPAQUE: u8 = 255;` used at
`:123` as the fourth component of every texel. Baked art:
`crates/mc-client/src/textures/decode.rs:83` `.to_rgba8()`, which the `image`
crate defines as filling alpha with 255 for a source without an alpha channel.
FR-3.2-S1 is the test for the second of those, and it is a test of a property
that already holds rather than a change.

### Decision 3 — BINDING: **B1**, one unsorted blended pass with depth-test on and depth-write off

Rejected most closely: **B2** (sorted per section).

**What separates B1 from all three rivals at once: for the content this project
ships, B1, B2, B3 and B4 produce the same image to within a layer's own texel
spread.** Three steps. Step 1 is a reading of two source files; steps 2 and 3 are
algebra. **None of them is a measurement of a renderer**, and the section heading
says so because an earlier draft of this document called them one.

1. **A ray crosses exactly one drawn face per maximal *run* of translucent cells
   of one kind.** The mesher's third question
   (`crates/mc-world/src/mesh/sweep.rs:311`,
   `if occludes(resolved, beyond)? || beyond == key`) emits no face between two
   cells of one kind, so a run has only its two end faces. Corners are wound
   outward (`crates/mc-render/src/geometry/mod.rs:11-25`) and back faces are
   culled unconditionally (`pipeline.rs`; `pass.rs:41-55`), so the face a ray
   *leaves* a run through has its normal along the ray and is discarded. One
   drawn face per run.

   **It is per *run*, not per *volume*, and the distinction is real.** A volume
   concave along the ray — a bay, water either side of a wall — is entered,
   left, and entered again, giving two drawn faces from one body. Runs and drawn
   faces stay 1:1 regardless, because an air gap ends a run.

2. **Overlapping layers of equal colour compose order-independently.** For
   `src-over` with two layers of colour `C`, opacities `a` and `b`, over
   background `D`, both orders give `C(1-(1-a)(1-b)) + D(1-a)(1-b)` — symmetric
   in `a` and `b`.
3. **B4 collapses to the same answer for equal colours.** Weighted-blended OIT
   resolves `accum.rgb / accum.a`, which for `Ci = C` is exactly `C` whatever the
   weights, leaving `C(1-Π(1-ai)) + D·Π(1-ai)` — the sorted answer. B2 and B3
   differ from B1 only by an ordering step 2 has just shown does not matter.

**"Equal colour" is a premise, so the residual is bounded rather than zero.**
Under A3 two overlapping layers of one kind sample different texels of the same
art. For two layers of equal opacity `a` and colours `A`, `B`, the two orders
differ by exactly `a²(B − A)` — derived from step 2's expansion. At `a = 0.5`
that is a quarter of the layer's own texel spread, which `rendering.md:1656`
puts at **ΔE 3.16** for `base:water`, so the residual is at most about **ΔE
0.79** — inside R1's bracket by a factor of thirty against the ΔE 25.34 at its
far end. The honest claim is *identical within a layer's own texel spread*, not
*pixel-identical*.

The shipped content declares **one** translucent kind. With correctness equal to
within that bound, cost decides:

| Candidate | Cost over B1 | Verdict |
|---|---|---|
| B1 | — | **chosen** |
| B2 | an ordered reservation the atomic cannot give — a prefix sum or a second dispatch, in a compute pass whose four-binding budget is already full | rejected |
| B3 | B2's cost plus a per-frame GPU sort and index re-upload | rejected |
| B4 | a second colour target, a resolve pass, a change to the offscreen capture target that the whole golden procedure is written against, and a **device**-level question about `Rgba16Float` blendability that this project has never had to ask | rejected |

> **Correction folded from review:** an earlier draft disqualified B4 on "float
> blending at `Capabilities::empty()`, the profile `build/validate.rs` validates
> against". That is a category error — `naga::valid::Capabilities`
> (`build/validate.rs:36-39`) is a **shader** validation profile and never sees a
> `wgpu::ColorTargetState`. B4 still loses, on the three costs above; it does not
> lose on the reason first given.

**The stated limit, which is a deliverable and not a footnote (D4, FR-4.1-S2).**
B1's artefact appears the day two translucent surfaces of **different colours**
overlap. Worse than wrong, it is **nondeterministic**: `cull.wgsl:22-26` says the
order visible sections land in the index buffer "is whatever the atomic hands
out, and therefore not reproducible between runs", and the same comment names a
transparency pass as the event that expires its own reasoning. Within one section
quads land at deterministic offsets (`reserved_base + 6*quad`); it is the
*inter-section* order that is free.

So `docs/technical/rendering.md` owes, and FR-4.1-S2 requires: the model, the pass
order, each pass's depth-write setting, **and a camera position with a frame
showing the artefact** — which needs a fixture holding two differently-coloured
translucent kinds. That fixture is the evidence; a prose warning is not.

**Deferred, with its trigger written down:** order-stable compaction plus a
back-to-front section order. The condition that forces it is *a second translucent
block kind whose colour differs from the first reaching shipped content or a
committed golden* — not "someday". Until then it is Out of Scope by the spec's own
listing.

**`merges_with_self` is NOT needed.** Step 1 is the whole reason: the model wants
exactly the faces the mesher emits today. FR-4.2-S2 (no seam between adjacent
cells of one kind) is preserved unchanged, and preserving it is what makes step 1
true. **No scope change is raised on coupling B2.**

**FR-4.2-S2's positive control cannot be a rendered world.** The scenario requires
the same reading to *report* a seam over a fixture in which one stands — and the
engine cannot draw one, because `sweep.rs:281-289` states the rule and F3 defers
the field that would override it. The control is therefore a **synthetic frame**
carrying a seam, fed to the same reading. Legitimate and cheap, but it has to be
said, or a test author reaches for a world fixture and finds it unbuildable.

**FR-4.3-S1's mechanism, stated correctly.** A camera inside a translucent run is
past that run's entry face and in front of its exit face, which is back-facing
and culled — so **the run the eye stands in contributes no fragment**. It does
*not* follow that a wet camera draws no water at all: a further run along the ray
still draws its entry face and blends exactly as it would from a dry camera. What
FR-4.3-S1 is owed is therefore *the same per-surface blend wet or dry, and no
whole-frame tint* — never "zero water fragments from a wet camera", which the
engine does not owe and a test author could easily assert.

**And the sentence that followed this one was wrong, which is worth leaving
visible rather than deleting.** It read "the declared path is submerged at ticks
59 and 119, so this distinction reaches the goldens". The *player* wades at those
ticks; the *eye* does not go under, at any of the three, and
`replay_oracle.rs::the_camera_of_every_judged_frame_stands_in_open_air` was
already the standing assertion of it when the sentence was written. A submerged
camera is therefore a pose a reading has to **declare for itself** over the
shipped world, and FR-4.3-S1 reaches no committed frame at all. The error is the
ordinary one of taking a fact about the player for a fact about the camera, and
it is the kind that survives review because both halves are true of something.

### Decision 4 — BINDING: two draws, one render pass, one index buffer in two fixed halves, one args buffer holding two `DrawArgs`

**The four-binding budget writes this design.** The cull pass may not gain a fifth
storage buffer, so the translucent range cannot be a second buffer and the
per-quad translucency bit cannot be read from the vertex buffer (which the cull
pass does not bind at all). The only thing the cull pass can learn a partition
from is the **section table it already binds**.

| Resource | Today | After |
|---|---|---|
| `SectionRecord` / `Section` (WGSL) | origin(3), first_quad, quad_count, aabb(6) — **44 bytes** | + `opaque_quad_count` — **48 bytes** |
| indices buffer | `6 · MAX_QUADS` indices = 6 MiB | **two fixed halves**, 12 MiB |
| args buffer | one `DrawArgs`, 20 bytes | `array<DrawArgs, 2>`, 40 bytes |
| cull storage bindings | 4 | **4** |
| vertex-stage storage bindings | 1 | **1** |
| draws per frame | 1 | 2 |

- Quads `[0, opaque_quad_count)` of a section compact into the **lower** half at
  `args[0]`; quads `[opaque_quad_count, quad_count)` into the **upper** half at
  `args[1]`. Two atomics, no prefix sum, no second dispatch.
- **The upper half's base is read out of `args[1].first_index`, never duplicated
  into the shader.** `cull.wgsl` writes at `reserved_base + 6·quad` where
  `reserved_base` is an `atomicAdd` result counted from **zero**, so the
  translucent half needs a base — and writing `6 · MAX_QUADS` into WGSL would be a
  **fourth** hand-duplicated CPU/GPU number, the one whose drift writes
  translucent indices into the opaque range. Instead the CPU writes that constant
  into `args[1].first_index` in `reset_draw`, exactly as it already writes
  `index_count = 0`, and the shader reads it back:
  `base = args[1].first_index + atomicAdd(&args[1].index_count, …)`.
  **Race-free and ordered, measured:** `record.rs:57-60` calls
  `reset_draw(queue)` — a `queue.write_buffer` — before `cull` records into the
  encoder, and a queue write is ordered before the command buffer that follows
  it. That is the same guarantee `index_count = 0` already rests on. Mixing an
  `atomic<u32>` member with plain `u32` members in one WGSL struct is legal, and
  nothing writes `first_index` during the pass.
- The index buffer doubles because the two halves need statically-known bases and
  the split between them is not statically known. 6 MiB of desktop VRAM is the
  price the binding budget charges; the alternative — an `atomicMin` on
  `first_index` so the upper range grows downward — buys the 6 MiB back at the
  cost of a reservation nobody can read.
- `draw_indexed_indirect(&args, 0)` then `draw_indexed_indirect(&args, 20)`.
  `AlignOf(DrawArgs) = 4` and `SizeOf = 20`, so the WGSL array stride is
  `roundUp(4, 20) = 20` and the second offset lands exactly on `args[1]`. **If
  R4 forces padding to 32, the WGSL struct must gain the same explicit padding**
  or the two sides silently disagree about where `args[1]` starts.
- Both draws are in the **same** render pass, so the second reads the depth the
  first wrote. Rasterization order between draws in one pass is what makes
  "opaque first" mean what it says.

**Where the two pipelines' difference lives, and why not in `TerrainPassConfig`.**
`pass.rs:6-11` says "there is one descriptor, one function that fills it in, and
**the colour target is the only thing a caller may choose**", and `:13-21` says
every other setting is a one-variant type. That invariant is about **what a
caller chooses**, and a caller chooses neither pipeline — both are always built,
from one config. So the difference is a private `enum TerrainLayer { Opaque,
Translucent }` threaded through `terrain_pipeline`, `color_target` and
`depth_state` in `pipeline.rs`. `TerrainPassConfig` is untouched, `pass.rs`'s
invariant is preserved rather than amended, and `pipeline.rs`'s own header — "There
is **one** render-pipeline builder and there is no second one" — is honoured,
because the builder is parameterised rather than copied. The blended target's
`BlendState` is `hud_pass.rs:269` verbatim: colour `SrcAlpha / OneMinusSrcAlpha /
Add`, alpha `One / OneMinusSrcAlpha / Add`.

**The opaque pipeline is unchanged in every respect** — same blend (`None`), same
depth state, same write mask. Consequence, and it is the mint's strongest check:
*every pixel not covered by a translucent face must be byte-identical to `r4`.*

### Decision 5 — BINDING: the partition is built in `mc-render`, and the mesher does not move

`build_section_geometry` (`crates/mc-render/src/geometry/mod.rs`) already resolves
`quad.block` through a `TextureResolution` to get a texture layer. It emits opaque
quads first, then translucent, and `SectionGeometry` reports `opaque_quad_count`.

*Rejected:* partitioning in `mc_world::mesh::sweep`. It **contradicts a stated
invariant of that module**, verbatim at `sweep.rs:11-17`: "The order is the loop
nesting and never a sort… The tempting fast variant… is forbidden for exactly that
reason: it emits all of one block's quads before another's within a plane, and
putting them back in order afterwards would write the order down a second time."
A partition is precisely that reordering. It would also hand the mesher a reason
to read a rendering field, and `mc-world` would then own half a draw-order
decision.

**Invariant 1 (D8) holds by construction.** Opacity is reached through the same
`BlockName → declaration` lookup the texture key already uses, in the same call,
and `sweep.rs` gains nothing at all. No name comparison exists in either module
before or after; FR-7.1-S1's scan and FR-7.1-S2's positive control are what say so.

### Decision 6 — BINDING: the packed vertex grows an 8-bit opacity field, and **`SCENE_REVISION` moves `r4` → `r5`**

The alpha the blend uses is per **block**. The only per-fragment quantity the
fragment stage has today is `layer` — a per **texture key** index. Three
candidates for getting a per-block number to a fragment, and the third is the one
that costs the re-mint:

**(i) A per-layer opacity table, indexed by `layer`. Unsound.**
`crates/mc-render/src/texture/resolution.rs:40-42` is
`blocks: BTreeMap<BlockName, FaceTextures>` over `layers: TextureLayers`, so two
blocks may declare the same key and share one layer — `glass` and `tinted_glass`
sharing art is the obvious case. The second block's entry silently overwrites the
first's and the world draws a plausible wrong picture with no error anywhere,
which is the exact failure that module exists to prevent. Making it sound needs a
refusal nobody asked for.

**(ii) Key the layer assignment by `(TextureKey, Opacity)` and bake the declared
opacity into the layer's alpha at pack time. Sound, and it would hold the revision
at `r4` — rejected on a measured cost.** It invents no refusal (it reaches the
existing layer-budget refusal sooner), keeps the packed vertex at 36 bits, and by
this document's own §D rule would need no bump at all. It loses on hot reload,
which is FR-6 and is this project's headline capability:

> **Measured:** `crates/mc-core/src/content.rs:280` — `LayerAssignment::appending`
> does `let mut next = self.spent;` and hands out `next += 1` per newly
> introduced key. `spent` is never decremented anywhere in the file, and the
> budget refusal's own message (`:216`) says relaunching is what reclaims a
> layer. So **a session's layer budget is monotonic.**

Under (ii), every distinct opacity a mod author types is a new `(key, opacity)`
pair, which is a newly introduced key, which **permanently spends one of 256
layers**. Editing `opacity = 0.5` → `0.4` → `0.45` while the server runs — which
is precisely the loop FR-6 exists to make possible — burns the budget until the
reload is refused and the author is told to relaunch. It also turns a
content-authoring choice into a texture-memory cost and hands a mod author with
thirty images a refusal about having too many textures. Secondary costs: an
opacity edit becomes a texture re-upload rather than a re-pack, and the mip chain
for one image is recomputed once per opacity.

**(iii) Chosen: the opacity rides in the vertex.** 8 bits at shift 36, taking the
packed vertex from 36 bits used to 44 of 64, moving no existing field. The
fragment stage returns `vec4(texel.rgb, texel.a * opacity)`.

**Spending 8 of the 28 spare bits, against the sentence that reserves them.**
`vertex.rs:16-18` says they "are not a design margin to be spent casually —
ambient occlusion and per-vertex light will want them". Both of those are
**per-corner** quantities and want the remaining 20 bits (AO is conventionally
2 bits per corner, per-vertex light 4–8); this field is per-block and replicated
across all four corners, which is genuinely the wasteful shape. It is spent
anyway because (i) is unsound and (ii) breaks hot reload, and because 20 bits
still covers both reserved uses. **If a later spec needs those bits back**, the
route is (ii) plus a refusal on a key shared at two opacities — recorded as F7 so
the option is not lost.

**Quantisation is not a problem, and here is the arithmetic rather than a claim.**
A declared `0.5` becomes `128/255 = 0.50196`, an error of `1.96e-3`. Blending a
`(40,40,40)` block over a `(206,…)` background: the exact-`0.5` result is
**153.15** and the quantised result is **152.89** — a difference of **0.26 of one
code value**. Derived, by evaluating the sRGB transfer function; it is not a
reading of any frame.

**The oracle must NOT use the quantisation.** FR-5.1-S2 requires the prediction to
share no code with the draw path. It predicts from the declared `0.5`, and the
0.26-code-value quantisation error is one term on the *measured error* side of
the tolerance derivation. Writing that down here is the point: reaching for
`Opacity::quantised()` in the oracle would be the independence quietly lost.

#### §D — the contract argument, stated as a rule and not as an outcome

The authority is the constant's own doc comment
(`crates/mc-render/src/capture.rs:29-33`), which is newer than
`rendering.md:1391`'s six-item list and was rewritten when `r3` falsified the
older wording:

> bumped whenever a change makes a same-named capture incomparable with the
> frames already committed under that name — whether the change is to the **mesh
> contract** or to the **declared camera path**

**The rule the log actually applies, read off the four entries.** `r1`→`r2` bumped
because the merge predicate's output moved *and* the camera path moved
(`rendering.md:1476`). `r2`→`r3` and `r3`→`r4` bumped because the declared physics
moved the walk (`:1547`, `:1665`). The revision **held** twice, on 2026-08-19 and
2026-08-26, and the second entry states the mechanism precisely
(`rendering.md:1620`): `base:water` "was **already** one of those eight keys and
**already** sorted last, so baking an image for it adds no key and moves no
index — which is the whole difference from 2026-08-19's four-keys-to-eight change,
where every index after `base:dirt` renumbered and **a layer index rides inside
every packed vertex**".

**So the operative test is not "did pixels move" — 88 280 to 198 828 of them moved
in the held case. It is: did anything change about what a vertex carries, which
faces exist, or where the camera stands?**

Applying it to this delta, item by item:

| Contract item | Moves? | Reading |
|---|---|---|
| pose · world · camera path · tick list | **no** | nothing here touches spawn, script or physics |
| merge predicate | **no** | `visible_face` is untouched (Decision 5). `SCENE_QUAD_COUNT`, `total_face_area` and `area_by_block` all hold |
| **vertex format** | **YES** | an 8-bit field is added to `PackedVertex` |

**A blended pass alone would be a renderer change and would hold the revision.**
Decision 4 on its own moves no vertex and no face — the section record grows, but
a section record is a renderer-side index into buffers, not something a capture is
a photograph of, and the same faces are drawn at the same depths in the same
places. That is not a hypothetical: candidate (ii) above **is** that design, and
its honest §D answer is **no bump**. It was rejected for hot reload, not for the
revision.

**It is Decision 6(iii), and only Decision 6(iii), that bumps it.** The vertex
format is the one contract item this delta moves, and it is named in the
revision's definition in both the constant's doc and `rendering.md:1391`.
**`SCENE_REVISION: "r4"` → `"r5"`.**

**And the tripwire cannot see it — the third time, which makes it a pattern
rather than an anecdote.** `mc-sim`'s `scene_contract.rs` compares quad count and
per-block area, both properties of the mesh alone
(`crates/mc-sim/src/replay/contract.rs:47-49`). The mesh does not move here, so
all five of its tests stay green through a change that invalidates every
committed frame. `SCENE_REVISION`'s doc already records that this case "has no
guard at all"; `r3` and `r4` were the first two instances and this delta is the
third. **Three is the point at which "the guard is blind here" stops being a
property of one change and becomes a property of the guard**, and
`rendering.md` must record it in those terms — not as a third footnote in a log.
What the pattern says is narrow and worth stating exactly: **every bump so far
whose cause was neither the mesh nor the spawn has been invisible to the only
automated instrument that runs before an image is compared.** F6 keeps the guard
itself deferred; naming the pattern is what stops the fourth instance being
discovered rather than expected.

**The re-mint is certain regardless of the bump**, and the figure is measured:
`rendering.md:1608` records **77 987** water pixels at tick 0 of 921 600 **in the
committed `r3` blobs**, counting the stand-in's own colours; `:1633` is the set
comparison that carries the figure forward to `r4` unchanged in *position*. Tick 0
is the dry, inland capture — so a golden does see the sea from outside it and
every one of those pixels blends after this change.

**The rename is by DELETION and a fresh mint, never `git mv`.** Measured on the
2026-08-27 re-shoot (`rendering.md:1690`): a `git mv` passed the comparison,
`golden_mismatch` and `golden_inventory` while two directories still held sidecars
reading `"capture": "player-walk-t000-r3"`. The mint writes nothing for a capture
that still matches, the inventory reads directory *names*, and the comparison
reads *pixels* — so all three agree while the provenance is stale. FR-5.2-S1 is
the scenario for it.

**A derived check the mint must make, stronger than "the frames changed".** Because
the opaque pipeline is untouched (Decision 4), the set of pixels that **differ**
between the `r4` blobs and the `r5` mint must be a **subset of the water region**
in the old blobs — position-identical, in both directions, exactly the set
comparison `rendering.md:1633` made on 2026-08-26. A changed pixel outside that
region is a wrong partition or a wrong vertex field, and no count can report it.

### Decision 7 — BINDING: one bounded numeric reader, four refusal branches, in the order the spec's discriminators require

`number.rs`'s header states the rule this must not break: "a second reader would
be a second place for the modding guide and the program to disagree about what a
number may be." So there is **one** reader taking a floor and a ceiling:

```
optional_number_within(declared, field, bounds: RangeInclusive<f32>, absent) -> Result<f32, FieldFault>
```

> **Correction folded during implementation, and the signature above is the
> corrected one.** This document first wrote the bounds as two parameters,
> `floor` and `ceiling`, which makes five. **Measured:** `clippy.toml:10` sets
> `too-many-arguments-threshold = 4`, so `-D warnings` refuses the five-parameter
> spelling and the gate cannot pass with it. `code-quality.md` §2 names the
> remedy — *"max 4 parameters (use an object beyond that)"* — and a
> `RangeInclusive<f32>` is that object. Nothing about the four branches, their
> order, the visibility or the one-reader rule changes; only the arity does.

`pub(super)` within `luau_declaration`, like the reader it replaces —
`FieldFault` is `pub(super)` (`luau_declaration/mod.rs:330`) and may not appear in
a crate-public signature. `optional_number_at_least_zero` becomes a call with
`ceiling = f32::MAX`. **That is a no-op for `move_resistance` and `swim_ascent`,
by two readings:** a `f64` above `f32::MAX` becomes `inf` on the narrowing cast
and is caught by the finiteness branch *before* any ceiling test, and
`docs/modding/blocks-items.md:80` already documents their bound as "at most
`3.4e38`" — which is `f32::MAX`. The unified reader makes the page and the program
agree rather than diverge.

**Four branches, and the order is load-bearing.** The existing reader has three
(`number.rs:116-140`): wrong kind, non-finite, below floor — "too wide for `f32`"
is folded into finiteness by the narrowing cast, which is why the spec's four
*causes* are three *branches*. A ceiling makes four:

1. wrong kind → FR-1.2-S1 (`"half"`)
2. **not finite** → FR-1.2-S4 (`0/0`) and FR-1.2-S5 (`math.huge`)
3. below floor → FR-1.2-S2 (`-0.1`)
4. above ceiling → FR-1.2-S3 (`1.5`)

Finiteness **before** floor is already there (`NaN >= 0.0` is false); finiteness
**before ceiling** is the new half, and it is what makes `math.huge` report
non-finiteness rather than "above `1.0`" — which is exactly why the audit replaced
`-math.huge` with `math.huge` as the discriminator for the ordering *this* spec
introduces. The `-0.0 → 0.0` normalisation (`stated + 0.0`) is kept: a save folds
this number by its bits.

**FR-1.3-S1 is a cross-field refusal, the loader's first.** `opacity < 1.0`
together with `occludes = true` is refused after both are read, as a
`FieldFault::invalid` on `opacity` whose cause names both fields — `FieldFault` is
`{ field: Option<String>, cause: String }` (`mod.rs:330`), so no new fault type is
needed. **It is also load-bearing for Decision 3:** it is what guarantees a
translucent block never hides what lies beyond it, which is the premise of step 1.

### Decision 8 — BINDING: `APPEARANCE_REVISION` 3 → 4, `BEHAVIOUR_REVISION` stays 4, appended never inserted

`opacity` is appended to `DeclaredAppearance`
(`crates/mc-world/src/persistence/format.rs:410`), after `occludes`.
`APPEARANCE_REVISION` (`:311`) moves `3 → 4`; `BEHAVIOUR_REVISION` (`:310`) stays
at `4`.

The precedent is stated in that file rather than assumed: "`drawn` and `occludes`
are on the *other* list: routing a rendering field through this byte would buy
that cost again for a change no player can act on" (`:292`). How much light
passes through a block does not change what it is to stand on. A pre-field save
loads with every block at `1.0` and is reported **retextured**, which refuses
nothing; the behaviour byte's move would have reported every block **changed**,
which `Acceptance::OnlyUnchangedBlocks` refuses.

`format_test.rs:143-144` states both bytes by hand, and `format.rs:305` records
the measurement that **only** a test stating the byte sequence can see one move.
FR-5.3-S2 is that test.

### Decision 9 — BINDING: the oracle composes per translucent **run**, not per translucent voxel

D6's trap, and it is not the obvious one. The oracle marches **voxel by voxel**
(`oracle.rs:474`), so a naive second rule accumulating every translucent voxel
would predict **ten** layers through ten cells of sea where the renderer draws
**one**. The prediction would be wrong in a way that looks like a renderer defect.

So: `Sighted` gains a third arm carrying the ordered translucent layers a ray
crossed before it met an opaque block or the march limit, where a **maximal run of
one block kind contributes exactly one layer**. That is a restatement of the
engine's own rule — "a block never draws a face against its own kind", plus
back-face culling — arrived at independently, which is the same relationship
`oracle.rs:41-52` already documents for `drawn`. Being a *run* rule rather than a
*volume* rule is what makes it correct for concave water as well (Decision 3,
step 1).

- **FR-5.1-S2** (share no code with the draw path): the composition is
  `src-over` written out in linear light in the test support module, from the
  declared `f32` opacity and the two layers' own declared mean colours. It calls
  nothing in `mc-render`'s draw path and does not use `Opacity::quantised()`.
- **FR-5.1-S3** (an all-opaque world predicts what the pre-change tree predicted):
  the recorded values are committed **before** the first implementation commit,
  which makes them a reading of a tree the change had not touched. A value
  captured afterwards would be a snapshot of whatever the new code does.
- **FR-5.1-S4**: the classification stays total. An enumerated verdict — sky, a
  block the world holds, or a blend of two of them, summing to `SAMPLE_COUNT` —
  keeps "I could not look" from reading as "all accounted for".

### Decision 10 — BINDING: a whole-frame classifier for FR-2.3, and it must return a total verdict

FR-2.3-S1 demands **every pixel** of a frame classified as exactly one of the
clear colour, a declared layer's own colour, or a sample between two layers
adjoining in screen space, with a verdict naming every pixel accounted for. The
nearest instrument today is `crates/mc-client/tests/support/swatch.rs`, which is
**region**-scoped (`:9-13`). Nothing whole-frame exists.

This is a new test-support instrument, and it is the largest single item in the
delta that is neither loader nor renderer. Its required shape follows from
`standards/global/testing.md` §2 and from FR-2.3-S2 being its rival:

- a **total enumerated verdict** (`EveryPixelAccounted` / a named
  disagreement), never `assert!(strays.is_empty())` — an absence assertion
  cannot tell an empty answer from a scan that can no longer look;
- it reports **how many pixels it looked at** beside its verdict, the property
  `swatch.rs:9-13` already insists on for regions;
- the layer colours come from `support::art`, never from the frame under test.

FR-2.3-S2 (one block redeclared at `0.5` yields ≥100 pixels at no layer's own
colour) is its positive control and must be driven through the same reading.

### Decision 11 — BINDING: the build validator gains the hand-duplicated layouts this delta edits

**Measured:** `grep -n "SHIFT\|shift\|bit\|LAYER_BITS" crates/mc-render/build/validate.rs`
returns **zero** matches. The packed vertex's bit layout is written a second time
by hand in `crates/mc-render/shaders/terrain.wgsl:56-58`, and the section record's
field list a second time in both `terrain.wgsl` and `cull.wgsl` — and **none of
the three is checked**, while `QUAD_INDEX_PATTERN`, `PLANE_AXES`, `IMAGE_SWAPS`
and `IMAGE_SIGNS` all are.

This delta edits both of those unchecked duplicates. `validate.rs:29-34` records
what that shape costs when it goes wrong: five of six faces drawn wrong "while
three hand-written copies of one table agreed with each other exactly", invisible
to every probe and minted into a golden as ground truth.

So the validator gains the vertex field shifts and widths, and the section
record's field offsets and stride. The translucent index base needs **no** entry,
because Decision 4 reads it out of the args buffer rather than duplicating it —
which is the cheaper closure of the same risk and is why it was preferred.

It carries **no scenario** — it is a build-time guard on a duplication this spec
creates, of exactly the kind `validate.rs` exists for. **Flagged for the lead
rather than absorbed silently**, because it is a guard rather than a behaviour.
It is a *build script* extension and not a gate stage, so it is inside a spec's
gift — but the judgement is stated so it can be vetoed.

### DEFERRED

| # | Deferred | Trigger that forces it |
|---|---|---|
| F1 | Order-stable compaction and a back-to-front section order | a second translucent kind whose colour differs from the first reaching content or a golden |
| F2 | Sorting between translucent blocks of different kinds | as F1 |
| F3 | `merges_with_self` as a declarable field | a model that needs the interior faces of a translucent volume. **Not this one** |
| F4 | Renaming `TextureResolution` to match its widened role | it now carries opacity as well as keys; the rename touches ~20 files including test-author-owned ones. The type's *role* — everything about a block's appearance a packer needs, travelling as one value — is unchanged, which is why the name is tolerable |
| F5 | Grouping `opacity` beside `occludes` in `RECOGNISED_FIELDS` | a deliberate re-ordering of the whole list, with all four mirrors and the order-comparing guard moved together |
| F6 | A tripwire over the declared camera path | already deferred by SPEC-027; unchanged here |
| F7 | Reclaiming the 8 vertex bits via candidate (ii) plus a refusal on a key shared at two opacities | ambient occlusion or per-vertex light needing more than the 20 bits that remain |

---

## 4. Interfaces

```
mc-core
  Opacity                      // finite, 0.0..=1.0; OPAQUE; new()->Option; get()->f32;
                               // quantised()->u8 — the ONE definition of the vertex encoding
  BlockDefinition.opacity: Opacity
  ResolvedBlock.opacity: Opacity

mc-world (content), pub(super) within luau_declaration
  OPACITY_FIELD = "opacity"    // 13th of RECOGNISED_FIELDS
  optional_number_within(declared, field, bounds: RangeInclusive<f32>, absent)
                                               -> Result<f32, FieldFault>   // see Decision 7
  // FR-1.3-S1: refused after both fields are read, naming both

mc-world (persistence)
  DeclaredAppearance { input_version, name, textures, drawn, occludes, opacity }
  APPEARANCE_REVISION: 3 -> 4          BEHAVIOUR_REVISION: 4 (unmoved)

mc-render
  TextureResolution::opacity_of(&BlockName) -> Option<Opacity>
  SectionGeometry::opaque_quad_count() -> usize
  SectionRecord { origin, first_quad, quad_count, opaque_quad_count, aabb }   // 48 bytes
  Vertex.opacity: Opacity      // packed at shift 36, width 8
  enum TerrainLayer { Opaque, Translucent }   // private to gpu::pipeline

shaders
  terrain.wgsl   opacity decoded from bit 36; fragment returns vec4(rgb, a * opacity)
  cull.wgsl      two ranges, two DrawArgs, base read from args[1].first_index,
                 four storage bindings (unchanged count)
```

---

## 5. Integration points

| Point | What changes |
|---|---|
| `luau_declaration/mod.rs` | reads `opacity`; the cross-field refusal; `RECOGNISED_FIELDS` 12 → 13 |
| `luau_declaration/number.rs` | the bounded reader; the existing one delegates |
| `persistence/format.rs` | `DeclaredAppearance` + `APPEARANCE_REVISION` |
| `mc-client/src/content.rs` | `ContentView::of` carries opacity into `TextureResolution` |
| `mc-render/geometry/mod.rs` | partition + `opaque_quad_count` |
| `mc-render/geometry/vertex.rs` | the new packed field |
| `mc-render/geometry/scene.rs` | `SECTION_RECORD_BYTES` 44 → 48 |
| `mc-render/gpu/buffers.rs` | `SECTION_BYTES` 44 → 48 (**a second literal of the same number**); index buffer doubles; `ARGS_BYTES` 20 → 40; `reset_draw` writes two records including `args[1].first_index` |
| `mc-render/gpu/pipeline.rs` | `TerrainLayer`; one builder, parameterised; a second pipeline value |
| `mc-render/gpu/record.rs` | a second `draw_indexed_indirect` in the same pass |
| `shaders/cull.wgsl`, `shaders/terrain.wgsl` | as above |
| `mc-render/build/validate.rs` | Decision 11 |
| `mc-render/src/capture.rs` | `SCENE_REVISION` `r4` → `r5` |
| `mc-client/tests/support/oracle.rs` | Decision 9; and the stale `PRO-952` citation at `:69` |
| **`mc-client/tests/support/quoted_refusals.rs`** | `FIELDS_IN_THE_ORDER_THE_GUIDE_STATES` 12 → 13. **Test-author-owned** — see Decision 1's sequencing note |
| **`mc-client/tests/support/` (new)** | the whole-frame classifier, Decision 10 |
| `mc-world/src/mesh/sweep.rs:287`, `mc-render/src/texture/mip.rs:78` | stale `PRO-952` citations corrected |
| **`crates/mc-render/shaders/cull.wgsl:22-26`** | the fourth stale comment: its own stated expiry has arrived |
| `docs/modding/blocks-items.md` | field-table row with its bound (`:80` is the pattern), the absence meaning, the refusal, a worked example, and the two quoted lists at `:457` and `:468` |
| `docs/modding/README.md:183` | the quoted list |
| **`docs/modding/hot-reload.md`** | the quoted list at `:437`, **and the `Field \| Visible after a reload?` table at `:156`** — FR-6 is entirely about that row |
| `docs/user/gameplay.md` | FR-7.3 |
| `docs/technical/rendering.md` | the ordering model, the pass order, the depth-write settings, the artefact with its frame, the `r5` log entry, and the third instance of the unguarded case |

**Hot reload (FR-6) needs no new machinery.** `app/reload.rs:71` calls
`remesher.retire(uploaded, serial)` on every accepted reload, and
`texture/resolution.rs` records that the worker "keeps the whole meshed list for
the run and re-packs all of it on every batch, against whatever resolution it
currently holds". A changed opacity is a changed `TextureResolution`, so the next
batch re-packs — moving faces between the two partitions and rewriting the vertex
field — with **no re-mesh, no texture upload and no restart**. That is the row
`hot-reload.md:156` records. FR-6.1-S3's refusal is the loader's, and a refused
reload leaves the serving resolution in place by the same path every other
refusal already takes.

---

## 6. Assumptions

| # | Assumption | Basis |
|---|---|---|
| A1 | The device blends on the terrain target | the HUD already blends on this backend (`hud_pass.rs:269`); the terrain target is the same format |
| A2 | An `Rgba8UnormSrgb` colour attachment blends in **linear** light — decode, blend, re-encode | the WebGPU/Vulkan definition of an sRGB attachment. **This is the spec's named trap and it is why FR-2's expectations are computed in linear light**; derived, for a `(40,40,40)` block at opacity `0.25` over `CLEAR_COLOR_SRGB = [135,206,235]` (`crates/mc-render/src/color.rs:42`), the sRGB-byte answer and the correct answer differ by **8.51, 17.57 and 21.40** code values on R, G and B |
| A3 | The shipped content declares exactly one translucent kind | `ls content/base/blocks/` → four files; `water.luau` is the only candidate and states no opacity today. Decision 3's limit is stated *because* this is an assumption about content, not a property of the engine |
| A4 | A capture path that wades the sea still shows it from outside at tick 0 | measured, `rendering.md:1608`, 77 987 px of 921 600 in the committed `r3` blobs, carried to `r4` by the set comparison at `:1633` |
| A5 | Doubling the index buffer (6 MiB) is free on the declared hardware range | desktop target, `docs/technical/architecture.md` |

---

## 7. Risks

| # | Risk | Severity | Mitigation |
|---|---|---|---|
| R1 | A blend expectation computed on sRGB **bytes** goes red against a correct frame, and the cheapest green is a looser tolerance | **high** | A2's derived figures are in this document *before* any test exists. Every tolerance is derived from both directions, **stated in one unit**: in ΔE, the floor is the layer's own texel spread **3.16** (`rendering.md:1656`) plus the ordering residual **≤0.79** (Decision 3) plus the quantisation term, which is 0.26 of one code value and under **ΔE 0.3** at these magnitudes — so a floor near **ΔE 4.3**; the ceiling is the nearest wrong answer, `base:stone` at **ΔE 25.34**. Never loosened until green |
| R2 | The oracle composes per **voxel** rather than per **run** and predicts ten layers where one is drawn | **high** | Decision 9 states the rule and its derivation. The failure would look like a renderer defect |
| R3 | A `git mv` rename of the golden directories passes every check with stale provenance | **high** | Deletion then fresh mint, FR-5.2-S1, measured at `rendering.md:1690` |
| R4 | `wgpu` 30 refuses a 20-byte indirect offset | medium | 4-byte alignment is the WebGPU requirement and 20 satisfies it; **confirm at the first draw**. Fallback: pad each `DrawArgs` to 32 bytes — **and the WGSL struct must gain the same padding**, or the two sides disagree about where `args[1]` starts |
| R5 | The vertex bit layout and the section record layout are hand-duplicated across three files with **no** build-time check, and this delta edits both | medium | Decision 11. The new field is appended at the top of the used range, so no existing field moves |
| R6 | `SECTION_BYTES = 44` in `buffers.rs:54` and `SECTION_RECORD_BYTES = 44` in `scene.rs:41` are two literals of one number | low | Both move together; Decision 11's stride check is what would catch a half-move |
| R7 | Nondeterministic compaction becomes visible the day a second translucent kind ships, and looks like a flaky test | medium | Written into `rendering.md` with its frame (FR-4.1-S2); F1's trigger stated |
| R8 | The `scene_contract` tripwire stays green through a change that invalidates every golden | medium | Cannot be fixed here — it is a property of what the tripwire measures. Recorded in `rendering.md` as the third instance, and F6 keeps the guard deferred |
| R9 | The implementer edits `quoted_refusals.rs` to make the field list agree | medium | Decision 1's sequencing note: the 12 → 13 move is the test author's commit. An implementer sends a dispute |

---

## 8. What was NOT contested

The lead reserved a `/sdd-discuss` round for two candidates that measurement
cannot separate. **There is no such pair here, and no tie was manufactured to
obtain one.**

- **Axis A**: A2 is foreclosed by FR-1.1-S1 and FR-2.1-S1 on the spec's own text.
  A1 and A3 are separated by a consequence, not a preference: under A1 every
  scenario of FR-3 tests code no shipped path reads.
- **Axis B**: B1, B2, B3 and B4 are identical to within a layer's own texel
  spread for the content that ships, by the three-step argument in Decision 3.
  With correctness equal to that bound, cost decides, and B1 is cheapest by a
  wide margin. The one thing that would make the axis contested — a second
  translucent kind — is Out of Scope by the spec's own listing, and F1 records
  the trigger that reopens it.
- **The one genuinely close call is inside Decision 6**, between (ii) and (iii),
  and it was settled by a measurement rather than a preference: `spent` is
  monotonic, so (ii) trades a golden re-mint for a hot-reload loop that runs out
  of texture layers. That is not a tie.

## 9. What in the spec looks wrong

Nothing is wrong. Two things are **incomplete**, both in the spec's favour:

1. **A fourth standing debt exists that the spec does not name.**
   `crates/mc-render/shaders/cull.wgsl:22-26` states that the index buffer's
   compaction order is not reproducible between runs and that "the day a
   transparency pass arrives this reasoning expires and compaction has to become
   order-stable." This spec is that day. It is discharged here by *stating the
   limit* rather than by making compaction order-stable, which is legitimate
   because Decision 3 step 2 shows the order is unobservable to within a texel
   spread for the content that ships — but the comment needs correcting alongside
   the three the spec already names, and FR-4.1-S2 is where the limit gets
   written down.

2. **FR-4.1-S1's fixture is under-determined in a way that matters.** "Two
   separated cells each declared at opacity `0.5`" is order-independent if the two
   cells hold the **same** block kind and order-*dependent* if they hold two kinds
   of different colours — which is the artefact FR-4.1-S2 asks to be documented.
   Both readings are legitimate English. The test author needs the same-kind
   reading for FR-4.1-S1 and the different-kind fixture for FR-4.1-S2's frame, and
   Decision 3 states which is which so it is not discovered from a flaky run.

---

## 10. Architecture review round

One round with `persona-architect` (Mode B) against the complete draft. Three
blockers, eight majors, six minors. **Every finding is folded above; none is
overridden.** Recorded here because the review is what makes this binding.

| Finding | Verdict | What changed |
|---|---|---|
| **B1** — three pages quote the field list, not two, and `quoted_refusals.rs:66` is a test-owned fourth mirror | upheld; verified at `mod.rs:80-83` and `quoted_refusals.rs:66` | Decision 1 rewritten with all four mirrors and a **binding sequencing note**; `hot-reload.md` and `quoted_refusals.rs` added to §5; R9 added; F5's cost corrected |
| **B2** — the translucent half needs a base the cull shader does not have, and hard-coding `6·MAX_QUADS` would be a fourth CPU/GPU duplicate | upheld | Decision 4 now reads the base from `args[1].first_index`, which the CPU already writes — **no duplication at all**, better than the reviewer's uniform suggestion. Ordering verified at `record.rs:57-60`. Decision 11 correspondingly does not need to cover it |
| **M1** — a `(TextureKey, Opacity)`-keyed layer assignment would hold the revision at `r4`, and the draft rejected it by never naming it | upheld | Decision 6 restructured into three candidates. (ii) is now named, granted its §D consequence explicitly, and **rejected on a measured cost**: `content.rs:280` shows `spent` is monotonic, so an opacity edit loop exhausts the layer budget. F7 keeps the option recoverable. The reserved-spare-bits sentence at `vertex.rs:16-18` is now engaged directly |
| **M2** — step 1 is false for a concave volume, and FR-4.3-S1's "no water fragment from a wet camera" does not follow | upheld | Step 1 restated as a **run** rule; the concave case named. FR-4.3-S1's mechanism corrected to "the run the eye stands in contributes no fragment", with the warning that a test author must not assert zero water fragments |
| **M3** — algebra called a measurement, and "pixel-identical" overstates an equal-colour premise | upheld | Heading and prose corrected; the residual is now **derived and bounded** at `a²(B−A)` ≤ ΔE 0.79, and §8 restated on the weaker claim |
| **M4** — B4 disqualified on a category error (`naga::valid::Capabilities` is shader validation) | upheld | Reason replaced, and the correction left visible rather than silently rewritten |
| **M5** — the two pipelines' difference has no stated home, and `pass.rs:6-11` forbids the obvious one | upheld | Decision 4 places it: a private `TerrainLayer` in `pipeline.rs`, `TerrainPassConfig` untouched, with the reason written against `pass.rs` |
| **M6** — R1's derivation adds a ΔE to a code value | upheld | R1 restated wholly in ΔE, floor ≈ 4.3, ceiling 25.34 |
| **M7** — FR-2.3 needs a whole-frame classifier that does not exist and was un-costed | upheld | New **Decision 10**, with its required shape and its positive control |
| **M8** — FR-4.2-S2's positive control cannot be a rendered world | upheld | Stated in Decision 3: a synthetic frame, not a world fixture |
| Minors: `record.rs:163` not `:180`; `vertex.rs:16-18`; "five branches" is four; `FieldFault` is `pub(super)`; A4 dropped its `r3` qualifier; A2 never named its source colour; `hot-reload.md:156` reload table | all upheld | all corrected |

**Upheld against the reviewer, with its evidence:** the four-storage-binding count
holds at 4 whether `args` is `DrawArgs` or `array<DrawArgs, 2>`
(`build/validate.rs:363-390` counts *globals* in `AddressSpace::Storage`); the
`array<DrawArgs, 2>` stride is 20 so `draw_indexed_indirect(&args, 20)` lands
exactly on `args[1]`; Decision 5's rejection of a mesher-side partition; Decision
6's key-sharing premise; and Decision 9. The reviewer confirmed each
independently.
