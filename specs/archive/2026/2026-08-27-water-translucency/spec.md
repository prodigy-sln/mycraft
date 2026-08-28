---
id: SPEC-031
title: Blocks declare translucency and the terrain draws a blended pass
status: implemented
work-type: feature
rigor: high
branch: feature/PRO-993-water-translucency
issue: PRO-993
created: 2026-08-27
updated: 2026-08-28
approved: 2026-08-27
completed: 2026-08-28
author: spec-PRO-993
---

# Specification: Blocks declare translucency and the terrain draws a blended pass

## Goal

Water is drawn opaque because nothing can say otherwise: no block declaration
states how much light passes through it, and the terrain pipeline sets
`blend: None` so it could not honour such a statement if one existed. This spec
adds a **content-declared degree of opacity** to the block declaration and a
**blended terrain draw** that honours it, so a player standing at the shore sees
the lakebed through the sea and a mod author can declare their own see-through
block without touching Rust.

## User Stories

- As a **player**, I want to stand on the shore and see the lakebed through the
  water, so the sea reads as water rather than as a blue floor.
- As a **mod author**, I want to declare how much light passes through my own
  block, so I can ship glass or a coloured liquid with no engine change.
- As a **mod author**, I want a mistyped or out-of-range translucency to be
  refused in a sentence naming my file, my block and the bound, so I can fix it
  mid-edit without reading Rust.
- As an **engine reader**, I want the draw-ordering model and its failure mode
  written down, so the next see-through block is added against a stated contract
  rather than against whatever water happened to need.

## Functional Requirements

### FR-1 — A block declares how much light passes through it

The field states a **degree**: `0.0` passes all light (invisible), `1.0` passes
none (today's behaviour), and an absent field means `1.0`. Both bounds are
**inclusive**. The field's spelling is the architect's; its semantics, bounds and
refusals are fixed here.

- **FR-1.1**: A declaration states a degree of opacity and the registry keeps it.
  - FR-1.1-S1: WHEN a block declaration states an opacity of `0.5` THE SYSTEM SHALL register that block with an opacity of `0.5`, readable from the registry.
  - FR-1.1-S2: WHEN a block declaration omits the opacity field THE SYSTEM SHALL register that block with an opacity of `1.0`.
  - FR-1.1-S3: WHEN a content root declares two blocks with opacities `0.25` and `0.75` THE SYSTEM SHALL register each with its own stated value and neither with the other's.
  - FR-1.1-S4: WHEN a declaration states an opacity of exactly `0.0`, and when one states exactly `1.0` THE SYSTEM SHALL register each at the stated value and refuse neither content root.
- **FR-1.2**: A stated opacity the engine cannot keep is refused, and the refusal names what to fix.
  - FR-1.2-S1: IF a declaration states the opacity as the string `"half"` THEN THE SYSTEM SHALL refuse the whole content root and name the file, the block, the field, and that a number was expected.
  - FR-1.2-S2: IF a declaration states an opacity of `-0.1` THEN THE SYSTEM SHALL refuse the whole content root and name the file, the block, the field and the floor `0.0`.
  - FR-1.2-S3: IF a declaration states an opacity of `1.5` THEN THE SYSTEM SHALL refuse the whole content root and name the file, the block, the field and the ceiling `1.0`.
  - FR-1.2-S4: IF a declaration states an opacity of `0/0` THEN THE SYSTEM SHALL refuse the whole content root naming the file, the block, the field and finiteness, and SHALL NOT report it as below the floor `0.0`.
  - FR-1.2-S5: IF a declaration states an opacity of `math.huge` THEN THE SYSTEM SHALL refuse the whole content root naming the file, the block, the field and finiteness, and SHALL NOT report it as above the ceiling `1.0`.
- **FR-1.3**: A declaration cannot state a translucency that the mesher would contradict. The
  condition is the occlusion the block **resolves to**, not the line it was written on:
  `occludes` falls back to `solid`, and the mesher reads the resolved value. S2 was added during
  phase 1, after the dispute over S1 exposed that a solid block stating a degree and no
  `occludes` was refused in a sentence quoting a line its author never wrote.
  - FR-1.3-S1: IF a declaration states an opacity below `1.0` together with `occludes = true` THEN THE SYSTEM SHALL refuse the whole content root and name both fields, and that a block light passes through cannot also hide what lies beyond it.
  - FR-1.3-S2: IF a declaration states an opacity below `1.0` and states `solid = true` without stating `occludes` at all THEN THE SYSTEM SHALL refuse the whole content root naming `solid` as the line that makes the block occlude, in a cause distinct from the one raised against a written `occludes = true`.
- **FR-1.4**: The field joins the declaration vocabulary a mod author is shown.
  - FR-1.4-S1: IF a declaration states a field name that is not recognised THEN THE SYSTEM SHALL quote back a recognised-field list containing the opacity field, in the documented position.

### FR-2 — Terrain draws a declared translucency, blended

Three constraints bind every scenario in this requirement, and each was paid for
by a defect recorded in `standards/global/testing.md`:

1. **Absolute, never comparative.** Each expected colour is derived by arithmetic
   from the declared opacity and the two layers' own colours. No scenario here
   compares one rendering to another; a comparison cannot see a change that moved
   both.
2. **Computed in linear light and re-encoded.** The array is `Rgba8UnormSrgb` and
   the hardware blends after decoding, so an expectation computed on sRGB bytes
   does not match the frame — at a `0.25`/`0.75` split the gap is tens of code
   values, not units.
3. **The fixture's two blocks must stand far enough apart** that the tolerance
   band — above the layers' own texel spread, below the distance to either
   unblended colour — is non-empty, and the fixture states both measured
   distances. No assertion can enforce this; it is held by whoever builds the
   fixture and whoever reads it.

- **FR-2.1**: A translucent block shows what is behind it.
  - FR-2.1-S1: WHEN the camera looks along a ray crossing one cell of a block declared at opacity `0.5` and then meeting an opaque block THE SYSTEM SHALL draw that pixel as the even blend of the two blocks' own colours, and SHALL in the same frame and the same assertion draw a pixel whose ray meets that opaque block directly at that block's own colour, unblended.
  - FR-2.1-S2: WHEN the camera looks along a ray crossing a block declared at opacity `1.0` and then meeting an opaque block THE SYSTEM SHALL draw that pixel as the nearer block's own colour, unblended.
  - FR-2.1-S3: WHEN the camera looks along a ray crossing a block declared at opacity `0.25` and then meeting sky THE SYSTEM SHALL draw that pixel as one quarter that block's colour composited over three quarters the clear colour, in linear light.
- **FR-2.2**: A translucent block does not erase what stands in front of it.
  - FR-2.2-S1: WHEN an opaque block stands between the camera and a block declared at opacity `0.5` THE SYSTEM SHALL draw that pixel as the opaque block's own colour, unblended.
  - FR-2.2-S2: WHEN the camera looks along a ray crossing one cell declared at opacity `0.0` and then meeting an opaque block THE SYSTEM SHALL draw that pixel as that opaque block's own colour, in a frame drawing at least one hundred pixels of that block.
- **FR-2.3**: A declared opacity is the only thing that makes terrain blend.
  - FR-2.3-S1: WHEN every block a world declares stands at an opacity of `1.0` THE SYSTEM SHALL classify each pixel of the frame as exactly one of the clear colour, a declared layer's own colour, or a sample between two layers adjoining in screen space, and SHALL return a verdict naming every pixel accounted for, over a frame in which the layer under test covers at least one hundred pixels.
  - FR-2.3-S2: WHEN one block of that same world is redeclared at an opacity of `0.5` THE SYSTEM SHALL return a verdict naming at least one hundred pixels at a value that is no layer's own colour.

### FR-3 — Alpha survives baking, mipping and sampling

`crates/mc-render/src/texture/mip.rs:74-78` records that no test discriminates
the alpha treatment and that the first translucent texture must bring one. This
requirement is that test's home.

- **FR-3.1**: Alpha reduces where it stands, and colour reduces in linear light.
  - FR-3.1-S1: WHEN four texels whose alphas are `0`, `0`, `255` and `255` are reduced to one THE SYSTEM SHALL give the reduced texel an alpha of `128` — the stored-byte mean rounded half up — and not the value that averaging the same four bytes in linear light and re-encoding gives, which the module's own transfer functions put above `180`.
  - FR-3.1-S2: WHEN four texels whose alphas are each `255` are reduced to one THE SYSTEM SHALL give the reduced texel an alpha of `255`.
  - FR-3.1-S3: WHEN a texture whose texels carry alphas of `0`, `64`, `192` and `255` is baked into the array THE SYSTEM SHALL carry each of those four values to level zero of the array unchanged and in place.
- **FR-3.2**: A source image without an alpha channel is opaque, not transparent.
  - FR-3.2-S1: IF a baked source image carries no alpha channel THEN THE SYSTEM SHALL give every texel of its level-zero array entry an alpha of `255`.
- **FR-3.3**: A translucent texture's alpha reaches the sampler.
  - FR-3.3-S1: WHEN two blocks declaring the same opacity of `0.5` are drawn over one opaque block in the same frame, one whose texture is opaque and one whose texture carries an alpha of `128`, THE SYSTEM SHALL draw the first as the even blend of that block's colour with the opaque block's, and the second as one quarter of that block's colour over three quarters of the opaque block's, at the tolerance FR-2.1-S1 states.

### FR-4 — Two translucent surfaces, and the ordering model's stated limit

The ordering model is the architect's. Whichever is chosen, these scenarios state
what a player can observe, and the model's **stated behaviour** — artefact or
correctness, whichever it has — is itself a deliverable.

- **FR-4.1**: A translucent surface over another translucent surface composes.
  - FR-4.1-S1: WHEN the camera looks along a ray crossing two separated cells each declared at opacity `0.5` and then meeting an opaque block THE SYSTEM SHALL draw that pixel as the composition of both over that block, which is a distinct colour from the one either cell alone produces.
  - FR-4.1-S2: WHEN `docs/technical/rendering.md` is read THE SYSTEM SHALL name the chosen ordering model, the order the opaque and blended passes run in, each pass's depth-write setting, and either a camera position with the artefact it produces and the frame showing it, or the statement that the model composes any two translucent surfaces correctly together with the evidence for it.
- **FR-4.2**: Depth against opaque geometry is exact regardless of order.
  - FR-4.2-S1: WHEN a translucent block is drawn in a frame also holding opaque blocks nearer the camera THE SYSTEM SHALL leave every pixel of those nearer opaque blocks at their own unblended colour, having examined at least one such pixel at each of at least three camera positions on the declared capture path.
  - FR-4.2-S2: WHEN two translucent cells of the same block kind stand adjacent THE SYSTEM SHALL draw no seam between them, and the same reading SHALL report a seam over a fixture in which one stands.
- **FR-4.3**: A camera inside a translucent volume gains no whole-frame tint.
  - FR-4.3-S1: WHEN the camera stands inside a cell declared at opacity `0.5` THE SYSTEM SHALL draw each surface outside that volume at the colour it draws from a dry camera at the same pose, and SHALL apply no tint to the frame as a whole.

### FR-5 — The instruments that watch this, and the scene contract

- **FR-5.1**: The replay oracle stops assuming every drawn block is opaque.
  - FR-5.1-S1: WHEN the oracle judges a sample whose ray crosses a block of opacity below `1.0` before reaching an opaque one THE SYSTEM SHALL have the oracle predict the blend rather than the nearer block's own colour.
  - FR-5.1-S2: WHEN the oracle predicts a blended sample THE SYSTEM SHALL derive that prediction from the declared opacity and the two layers' own colours alone, sharing no code with the draw path it grades.
  - FR-5.1-S3: WHEN the oracle judges a world in which every drawn block is opaque THE SYSTEM SHALL predict, for every sample, the value recorded from the oracle on the pre-change tree and committed before this spec's first implementation commit.
  - FR-5.1-S4: IF the oracle is given a world whose declared samples classify into neither sky, nor a block the world holds, nor a blend of two of them THEN THE SYSTEM SHALL report that rather than passing.
- **FR-5.2**: The re-shot golden set carries its own provenance.
  - FR-5.2-S1: WHEN the golden set has been re-shot THE SYSTEM SHALL have every provenance sidecar name the capture id of the directory holding it.
- **FR-5.3**: A save written before the field exists still opens.
  - FR-5.3-S1: WHEN a world saved before the opacity field existed is opened THE SYSTEM SHALL load every block it holds, each at an opacity of `1.0`, and refuse nothing.
  - FR-5.3-S2: WHEN a world is saved after the field exists THE SYSTEM SHALL have moved the appearance revision byte and left the behaviour revision byte at the value it held.

### FR-6 — A declared opacity reloads live, in both directions

- **FR-6.1**: Changing a declared opacity on a running server takes effect without a restart.
  - FR-6.1-S1: WHEN a block's declared opacity changes from `1.0` to `0.5` and the content root is reloaded THE SYSTEM SHALL thereafter draw that block at the blended colour FR-2.1-S1 states, with no restart.
  - FR-6.1-S2: WHEN a reload raises a block's opacity back to `1.0`, whether by stating it or by removing the field, THE SYSTEM SHALL thereafter draw that block at its own unblended colour.
  - FR-6.1-S3: IF a reload states an opacity of `1.5` THEN THE SYSTEM SHALL refuse that reload, keep the previously loaded opacity in force, and name the file, the block, the field and the ceiling.

### FR-7 — Invariant 1 holds, and both stakeholders reach it without reading Rust

- **FR-7.1**: No block name reaches the mesher or the renderer.
  - FR-7.1-S1: WHEN the mesher and renderer sources are scanned for a comparison against a block name THE SYSTEM SHALL return the verdict that no such comparison exists.
  - FR-7.1-S2: WHEN that same scan is run over a fixture holding one name comparison THE SYSTEM SHALL report that comparison rather than returning the clean verdict.
- **FR-7.2**: The mod author's page carries the field with its bound, its refusal and a worked example.
  - FR-7.2-S1: WHEN the modding pages are read THE SYSTEM SHALL find the opacity field named in each page's field table beside the value its absence means, and in each quoted recognised-field refusal, in the order the loader holds.
  - FR-7.2-S2: IF a modding page's field list is short by one name THEN the reading SHALL report it as a missing name rather than passing.
- **FR-7.3**: The player's page says what changed.
  - FR-7.3-S1: WHEN `docs/user/gameplay.md` is read THE SYSTEM SHALL name water as see-through and name at least one thing a player can see through it from the shore.
  - FR-7.3-S2: IF the gameplay page names neither THEN the reading SHALL report which of the two is missing rather than passing.

## Architecture Delta

This feature adds a **published block-declaration field** and a **second terrain
draw**, and it sets the shape every future see-through block inherits. Both are
new binding contracts. **The architect chooses; this section enumerates the space
and the couplings measured while writing it.**

### A. Where the per-pixel alpha comes from

| Candidate | Loader cost | Mod-author cost | Note |
|---|---|---|---|
| A1 — declared scalar alone | a **new bounded numeric reader** (below) | readable refusal, hot-reloadable, greppable | uniform per block |
| A2 — texture alpha alone | none | needs an image editor; no refusal to read; no declaration to reload | the array is already `Rgba8UnormSrgb` |
| A3 — their product | as A1 | as A1, plus per-texel control | the only one giving a stained-glass texel |

**Measured:** `crates/mc-world/src/content/luau_declaration/number.rs:116`
`optional_number_at_least_zero` is the loader's **only** numeric reader and it
has **no upper bound** — `grep -n "at_least\|at_most\|MAX"` over that file returns
only the `at_least_zero` name. A1 and A3 therefore need a *new* reader with a
ceiling and a fifth refusal branch beside the four that exist (wrong kind, below
floor, non-finite, too wide for `f32`). That cost is real and is why A2 is on the
list at all.

**Measured:** the array texture is `Rgba8UnormSrgb`, and `mip.rs:180`
`mean_of_stored` already averages alpha where it stands rather than in linear
light — deliberately, with a written note that nothing tests it. A2 needs no
format change; it needs the test.

### B. The draw-ordering model — the expensive-to-reverse choice

| Candidate | Correctness | Per-frame cost | Moves the mesh contract? |
|---|---|---|---|
| B1 — unsorted, depth-test on, depth-write off | wrong where two translucent surfaces overlap | one extra pass | only if faces are partitioned |
| B2 — sorted per section, back to front | wrong within a section and at its boundaries | a sort of the section list | as B1 |
| B3 — sorted per quad | correct | a re-sort and an index re-upload as the camera moves | as B1, plus a per-frame buffer write |
| B4 — weighted blended order-independent | approximate everywhere, order-free | a second target and a resolve | as B1 |

**Couplings the architect must weigh, not assumed here:**

1. **Does the mesher partition faces into an opaque set and a translucent set?**
   Every candidate needs the renderer to know which faces are translucent. If the
   mesher answers, the **merge predicate** moves and `SCENE_REVISION` moves with
   it. If a per-layer lookup at draw time answers, the mesh contract may hold.
   This is the single question that decides item D below.
2. **Does `merges_with_self` become necessary?**
   `crates/mc-world/src/mesh/sweep.rs:285-290` names it as the field that would
   let content ask for the interior faces of its own volume, and records that
   today "two adjacent cells of one non-occluding block show no seam and a mod
   author cannot ask for one". FR-4.2-S2 keeps that behaviour. If the chosen
   model needs interior faces, the field arrives here rather than in PRO-952 —
   and that is a scope change to raise, not to absorb.
3. **What survives the second see-through block?** B1's artefact is invisible
   while exactly one translucent kind exists and appears the day glass meets
   water. Choosing B1 is choosing to pay later; that is legitimate at MVP 2
   quality and must be *written down* (FR-4.1-S2), not discovered.

### C. Which save-format revision byte moves

**Assumed appearance, not behaviour**, on SPEC-025's precedent: how much light
passes through a block does not change what it is to stand on, and routing a
rendering field through the behaviour byte would tell every player in existence
that every block they built with behaves differently. A new field is **appended,
never inserted** — `postcard` encodes positionally.

### D. Whether `SCENE_REVISION` moves `r4` → `r5`

**Not assumed.** `docs/technical/rendering.md` binds the revision to the *scene
contract* — pose, world, camera path, tick list, merge predicate, vertex format —
and records two re-shoots (2026-08-19, 2026-08-26) where art moved and the
revision deliberately held, because bumping for an art edit "would redefine the
revision as *something visible changed*". A blended pass alone is a renderer
change. A partitioned mesh or a widened vertex is a contract change. **B decides
D.** The goldens are re-shot either way: the sea covers 9.58 %–21.57 % of a frame
(measured, `rendering.md`), so every capture moves.

**Binding either way:** a revision bump renames the set by **deletion and a fresh
mint, never `git mv`**. Measured on the 2026-08-27 re-shoot, a `git mv` passed the
comparison, `golden_mismatch` and `golden_inventory` while two directories still
held sidecars naming `r3`.

## Technical Considerations

- **Invariant 1.** Nothing in the mesher or the renderer may branch on a block
  name. `visible_face` states this property about itself and holds it today; a
  translucency reached through the key table keeps it. An `if name == "water"` is
  a Blocker.
- **Three stale citations, a deliverable rather than a scenario.**
  `crates/mc-world/src/mesh/sweep.rs:287`,
  `crates/mc-client/tests/support/oracle.rs:69` and
  `crates/mc-render/src/texture/mip.rs:78` each name a debt this spec discharges;
  the first two cite **PRO-952**, which is the wrong key. Correcting all three is
  in scope. It carries no scenario deliberately: a comment's issue key is not a
  behaviour, and a grep for it cannot date itself — the complete phase's doc
  consolidation is where it is checked.
- **The goldens can witness this, measured, and the reading still holds at `r4`.**
  `rendering.md` measures **77 987** water pixels at tick 0 of 921 600 (8.46 %) in
  the committed `r3` set, and tick 0 is the *dry, inland* capture — so a golden
  does see the sea from outside it. The `r3`→`r4` re-shoot records tick 0 as
  matching before the mint and coming back **byte-identical** after it, so the
  figure carries to the current set rather than describing a superseded one. What
  a golden cannot do is tell a correct blend from a wrong one: it is a photograph
  of whatever shipped, which is exactly how SPEC-029's stand-in survived every
  committed reference image. The **probes and the oracle** are the derived
  instruments; the goldens bound *when* anyone looks, and the replay's inputs
  bound *what* they can exercise.
- **A blend expectation is computed in linear light.** Three of the four
  instruments this spec leans on report sRGB bytes while the hardware blends
  after decoding. FR-2's preamble binds this; it is repeated here because it is
  the single arithmetic mistake most likely to produce a red test whose cheapest
  green is to loosen a tolerance.

## Existing Code to Leverage

| What | Location | Reuse |
|------|----------|-------|
| The one blended target in the workspace | `crates/mc-render/src/gpu/hud_pass.rs:269` | the `BlendState` shape, already proven on this backend |
| The loader's numeric vocabulary | `crates/mc-world/src/content/luau_declaration/number.rs` | four refusal branches; a ceiling is the fifth |
| The optional-field defaulting pattern | `crates/mc-world/src/content/luau_declaration/mod.rs` | `RECOGNISED_FIELDS`, and how a field's absence is given meaning |
| Alpha-aware mip reduction | `crates/mc-render/src/texture/mip.rs:180` | `mean_of_stored` exists and is untested — FR-3.1 is its test |
| The three-question face predicate | `crates/mc-world/src/mesh/sweep.rs` | key-identity comparison, no name and no runtime id |
| Doc/loader agreement guards | `crates/mc-client/tests/documented_declaration_fields.rs` | reads lists whole and in order; a field addition reddens it |

## Out of Scope

Binding. Recorded as deferred observations, never built.

- **Refraction** — the sea does not bend what is seen through it.
- **Caustics** — no light patterning on the lakebed.
- **Underwater fog or a submerged colour grade** — no tint is applied to the
  frame *as a whole* when the camera stands inside a translucent volume. The
  surfaces it sees still blend per-surface exactly as they do from a dry camera,
  which is what FR-4.3-S1 pins. **The submerged camera is not on the declared
  path**: the *player* wades at ticks 59 and 119 and the *eye* does not go under,
  which `the_camera_of_every_judged_frame_stands_in_open_air` asserts at all
  three declared ticks. So this distinction is **not** baked into the goldens —
  FR-4.3-S1 is read at a pose the reading declares for itself over the shipped
  world, and what the goldens carry is the dry half of it. Stating it before the
  re-shoot still mattered, because the alternative was discovering afterwards
  that no committed frame was ever going to answer the question.
- **Sky reflection on the water surface.**
- **The surface inset and the wave** — PRO-952 keeps both, with the
  `render = "<method>"` framing they belong to.
- **`merges_with_self` as a declarable field**, unless coupling B2 forces it, in
  which case it is raised as a scope change rather than absorbed.
- **Sorting between translucent blocks of different kinds** where the chosen
  model does not need it — its absence is written down under FR-4.1-S2 rather
  than fixed.
- **Cutout transparency (`alpha_cutoff`) for leaves and foliage** — a different
  mechanism needing no blending and no ordering.

## Dependencies

- **PRO-904 / SPEC-025** (merged) — the `drawn` / `occludes` / `targetable`
  split. A block cannot declare *how* it draws while one bit still answers four
  questions.
- **PRO-972 / SPEC-029** (merged) — `base:water` has baked art. Without it the sea
  draws a stand-in and there is nothing whose alpha means anything.
- **PRO-952** (Backlog) — takes the surface inset, the wave and the render-method
  framing. This spec must not foreclose it: an opacity field and a later `render`
  method must be able to coexist on one declaration.

## Assumptions

- The device supports alpha blending on the terrain target. The HUD pass already
  blends on this backend, so the capability is present.
- MVP 2 quality tolerates a named and documented ordering artefact. Correctness
  for the *single* translucent kind that ships is required; correctness for two
  overlapping kinds is a stated property of whichever model is chosen.
- A capture path that wades through the sea still shows it from outside at tick
  0, measured above.

## Open Questions

None. The two questions the ledger opened are answered: the goldens **can**
witness translucency (tick 0, measured), and `merges_with_self` is coupling B2,
handed to the architect with a scope-change rule attached.
