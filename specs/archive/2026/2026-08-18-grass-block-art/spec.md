---
id: SPEC-019
title: The grass block looks like a grass block — per-face keys, baked art, real pixels from disk
status: implemented
rigor: high
branch: feature/PRO-947-grass-block-art
issue: PRO-947 (absorbs PRO-914, PRO-869 part 1, PRO-902)
created: 2026-08-18
updated: 2026-08-18
completed: 2026-08-21
author: spec-PRO-947
---

# Specification: The grass block looks like a grass block

## Goal

No pixel in this project has ever come from disk. Every block face is filled
from a hash of the block's *name*, so grass renders tan and stone and dirt
render teal. This spec gives a block declaration the ability to name a different
texture per face, teaches VoxForge to bake a model's six faces into a texture
set as an explicit pre-build step, and makes the client draw those pixels
through a mipped, filtered sampler — so that a player starting the game sees
grass, dirt and stone instead of a teal and tan world.

## User Stories

- As a **player**, I want the world to be made of blocks that look like what
  they are, so that I can tell grass from stone by looking at it.
- As a **mod author**, I want to give my block a different texture on its top,
  its sides and its bottom, so that I can ship a block like grass without the
  engine having to know it is special.
- As a **mod author**, I want a block whose texture key I have not drawn art for
  to draw *something* and keep running, so that my first block is not a crash.
- As a **mod author**, I want to author a voxel model and see it on a block in
  the world, so that shipping art is a path I can walk end to end.
- As a **contributor**, I want a build with no generated art to refuse with a
  sentence telling me what to run, so that I never debug a texture-less world.

## Functional Requirements

### FR-1 — A block declares a texture per face

- **FR-1.1**: A block declaration states its texture either as one key for all
  six faces, or as a table naming a key for each of the six facings `up`,
  `down`, `north`, `south`, `east` and `west`.
  - FR-1.1-S1: WHEN a declaration states `texture` as the string `"base:stone"`
    THE SYSTEM SHALL register that block with `base:stone` as the key of all six
    of its facings.
  - FR-1.1-S2: WHEN a declaration states `texture` as a table giving `up`,
    `down`, `north`, `south`, `east` and `west` six different keys THE SYSTEM
    SHALL register that block with each facing holding the key written against
    it.
  - FR-1.1-S3: WHEN a declaration states `texture` as a table giving `up` and
    `down` the key `base:dirt` and its four side facings four other keys THE
    SYSTEM SHALL register `base:dirt` against both `up` and `down`.
  - FR-1.1-S4: WHEN that same declaration is loaded into content spending no
    layers THE SYSTEM SHALL report five layers spent.

- **FR-1.2**: A texture table that does not state exactly the six facings, or
  that states a value which is not a texture key, is refused.
  - FR-1.2-S1: IF a declaration states `texture` as a table naming only `up`,
    `down` and `north` THEN THE SYSTEM SHALL refuse the declaration and name
    `south`, `east` and `west` as the facings that were not stated.
  - FR-1.2-S2: IF a declaration states `texture` as an empty table THEN THE
    SYSTEM SHALL refuse the declaration and name all six facings as not stated.
  - FR-1.2-S3: IF a declaration states `texture` as a table carrying a key
    against the name `top` THEN THE SYSTEM SHALL refuse the declaration and name
    the six facings a texture table may state.
  - FR-1.2-S4: IF a declaration states `texture` as a table carrying a key
    against the name `Up` THEN THE SYSTEM SHALL refuse the declaration and name
    the six facings a texture table may state.
  - FR-1.2-S5: IF a declaration states `texture` as a table whose `up` holds the
    number `7` THEN THE SYSTEM SHALL refuse the declaration and report that `up`
    must be a string.
  - FR-1.2-S6: IF a declaration states `texture` as a table whose `north` holds
    `"base:grass:top"` THEN THE SYSTEM SHALL refuse the declaration and report
    that the value has more than one namespace separator.
  - FR-1.2-S7: IF a declaration states `texture` as the boolean `true` THEN THE
    SYSTEM SHALL refuse the declaration and report that `texture` must be a
    string or a table of six facings.
  - FR-1.2-S8: THE SYSTEM SHALL state every refusal this requirement raises in
    the modding guide's list of refusals, in the field order that guide already
    states.

- **FR-1.3**: The six facing names map to the world's axes by a published
  contract that content states and the engine never infers.
  - FR-1.3-S1: WHEN a block declaring `up = "base:grass_top"` and five other
    keys is placed in the world THE SYSTEM SHALL draw `base:grass_top` on the
    face pointing along positive Y and on no other face.
  - FR-1.3-S2: WHEN a block declaring `north = "base:a"` and `south = "base:b"`
    is placed in the world THE SYSTEM SHALL draw `base:a` on the face pointing
    along negative Z and `base:b` on the face pointing along positive Z.
  - FR-1.3-S3: WHEN a block declaring `east = "base:c"` and `west = "base:d"` is
    placed in the world THE SYSTEM SHALL draw `base:c` on the face pointing
    along positive X and `base:d` on the face pointing along negative X.

- **FR-1.4**: Per-face keys spend layers from the same budget single keys spend,
  and exhausting it refuses the load whole.
  - FR-1.4-S1: WHEN content already spending 250 layers is loaded with a block
    declaring six facing keys none of which is assigned THE SYSTEM SHALL report
    256 layers spent.
  - FR-1.4-S2: IF content already spending 251 layers is loaded with a block
    declaring six unassigned facing keys THEN THE SYSTEM SHALL refuse the load
    and report 257 layers needed against 256 assignable.
  - FR-1.4-S3: WHEN that refusal is raised THE SYSTEM SHALL leave every layer
    already assigned holding the key it held before the load.

### FR-2 — A face draws the key its block declared, not the key its name spells

- **FR-2.1**: A drawn face resolves its texture layer from its block's
  declaration and its own facing, and never from the block's name.
  - FR-2.1-S1: WHEN a block whose `name` is `example:amber` declares `texture`
    as the single key `example:gold` is placed in the world THE SYSTEM SHALL
    draw all six of its faces from the layer assigned to `example:gold`.
  - FR-2.1-S2: WHEN two blocks with different names declare the same single
    texture key are loaded into content spending no layers THE SYSTEM SHALL
    report one layer spent.
  - FR-2.1-S3: IF a block's declaration names a facing key that the session's
    layer assignment does not cover THEN THE SYSTEM SHALL refuse to build that
    section's geometry and name the block and the facing, rather than drawing
    layer zero.
  - FR-2.1-S4: WHEN sections meshed while a block declared `north = "base:a"`
    are packed again, without being re-meshed, against content declaring
    `north = "base:b"` THE SYSTEM SHALL draw those sections' negative-Z faces
    from the layer assigned to `base:b`.
  - FR-2.1-S5: IF a reload gives a block a facing key the session's layer
    assignment does not cover THEN THE SYSTEM SHALL refuse the reload and leave
    those sections drawing the keys they drew before it.

- **FR-2.2**: The held-block indicator resolves its image the same way the world
  does.
  - FR-2.2-S1: WHEN the held block declares `north = "base:grass_side_north"`
    and five other keys THE SYSTEM SHALL draw the indicator from the layer
    assigned to `base:grass_side_north`.
  - FR-2.2-S2: WHEN the held block's `name` is `example:amber` and its declared
    `north` key is `example:gold` THE SYSTEM SHALL draw the indicator from
    `example:gold`'s layer rather than drawing nothing.
  - FR-2.2-S3: IF the held block's `north` key has no layer in the session's
    assignment THEN THE SYSTEM SHALL report the indicator as unresolved, naming
    the block and the key, rather than drawing layer zero.
  - FR-2.2-S4: WHEN no block is held THE SYSTEM SHALL report that nothing is
    held, rather than the indicator of the block last held.

### FR-3 — VoxForge bakes a declared texture set

- **FR-3.1**: `voxforge build <manifest>` reads a manifest naming, for each
  texture key, a model and one of that model's six faces, and writes one image
  per entry into the manifest's output directory together with an index.
  - FR-3.1-S1: WHEN `build` is run against a manifest with seven entries THE
    SYSTEM SHALL write seven images, each named for the key its entry states.
  - FR-3.1-S2: WHEN `build` is run against a manifest with seven entries THE
    SYSTEM SHALL write one index naming all seven keys.
  - FR-3.1-S3: WHEN `build` is run against a manifest with seven entries THE
    SYSTEM SHALL report each written path on its output.
  - FR-3.1-S4: WHEN `build` is run twice against an unchanged manifest, models
    and materials THE SYSTEM SHALL produce byte-identical images both times.
  - FR-3.1-S5: WHEN a manifest entry names the `bottom` face of
    `grass-block.mcvox` against the key `base:dirt` THE SYSTEM SHALL write that
    face's image as `base:dirt`'s art.
  - FR-3.1-S6: WHEN `build` is run against a manifest with zero entries THE
    SYSTEM SHALL write an index naming zero keys and report zero images written.

- **FR-3.2**: A build that finds its output current does no work.
  - FR-3.2-S1: WHEN `build` is run against a manifest whose output and index are
    already current THE SYSTEM SHALL leave every image's bytes unchanged.
  - FR-3.2-S2: WHEN `build` is run against a manifest whose output and index are
    already current THE SYSTEM SHALL report that nothing needed rebuilding.
  - FR-3.2-S3: WHEN a model the manifest names is edited and `build` is run
    again THE SYSTEM SHALL rewrite the images derived from that model.

- **FR-3.3**: A manifest the tool cannot honour refuses the build whole, naming
  what is wrong.
  - FR-3.3-S1: IF the path given to `build` names no file THEN THE SYSTEM SHALL
    refuse the build and name the path it was given.
  - FR-3.3-S2: IF the manifest is not readable as TOML THEN THE SYSTEM SHALL
    refuse the build, report where parsing stopped, and write no images.
  - FR-3.3-S3: IF the manifest names a model file that does not exist THEN THE
    SYSTEM SHALL refuse the build, name the missing path and the key whose entry
    named it, and write no images.
  - FR-3.3-S4: IF a manifest entry names `"side"` as a face THEN THE SYSTEM
    SHALL refuse the build and name the six faces a manifest entry may select.
  - FR-3.3-S5: IF two manifest entries name the same texture key THEN THE SYSTEM
    SHALL refuse the build and name the key stated twice, rather than letting
    one entry overwrite the other.
  - FR-3.3-S6: IF a manifest entry states the key `"base:grass:top"` THEN THE
    SYSTEM SHALL refuse the build and report that the value has more than one
    namespace separator.
  - FR-3.3-S7: IF a manifest entry selects a face of a model whose opposite
    edges disagree by more than that face's own largest interior step THEN THE
    SYSTEM SHALL refuse the build and name the disagreeing edge.
  - FR-3.3-S8: WHEN every face a manifest selects tiles across all four of its
    edges THE SYSTEM SHALL complete the build.
  - FR-3.3-S9: IF a manifest names a model that is 16 voxels on X, 16 on Y and
    15 on Z THEN THE SYSTEM SHALL refuse the build and name Z as the axis that
    disagrees.
  - FR-3.3-S10: IF the fourth of seven entries is refused THEN THE SYSTEM SHALL
    leave the images and index written by the previous build unchanged.
  - FR-3.3-S11: WHEN the manifest states a key that no loadable block declares
    THE SYSTEM SHALL name that key as unused on its output and complete the
    build.
  - FR-3.3-S12: IF a manifest entry states a key whose image file name would not
    be a single ordinary file name THEN THE SYSTEM SHALL refuse the build, name
    the key, and state the name a key's image is written under.
  - FR-3.3-S13: IF a manifest entry states a key carrying a line break THEN THE
    SYSTEM SHALL refuse the build and name the key as unwritable to an index.
  - FR-3.3-S14: IF the manifest names a model whose declared scale multiplied by
    the manifest's pixels per voxel is not the edge a block texture has THEN THE
    SYSTEM SHALL refuse the build and name the model, the product and the edge.

- **FR-3.4**: The index records a single value folded over exactly the sources
  the manifest reached, so a consumer can tell a current set from a stale one.
  - FR-3.4-S1: WHEN a material file the manifest reached is edited THE SYSTEM
    SHALL record a different value on the next build.
  - FR-3.4-S2: WHEN a model file the manifest names is edited THE SYSTEM SHALL
    record a different value on the next build.
  - FR-3.4-S3: WHEN a model file under `content/base/models/` that no manifest
    entry names is edited THE SYSTEM SHALL record the same value on the next
    build.
  - FR-3.4-S4: WHEN the manifest, models and materials are unchanged THE SYSTEM
    SHALL record the value an FNV-1a-64 fold over the stated byte sequence
    computes, rather than a value derived from the standard library's hasher.
  - FR-3.4-S5: IF a source the manifest reaches cannot be read while the value
    is folded THEN THE SYSTEM SHALL refuse the build and name that source.

### FR-4 — The client draws the baked pixels

- **FR-4.1**: Every texture key the session assigns a layer is filled from the
  built set's image for that key when the set covers it.
  - FR-4.1-S1: WHEN the client starts with a built set whose `base:stone` image
    holds texels differing from the texture generated for `base:stone` THE
    SYSTEM SHALL fill `base:stone`'s layer with the image's decoded texels.
  - FR-4.1-S2: WHEN the client starts with a built set covering `base:grass_top`
    and `base:dirt` with different images THE SYSTEM SHALL fill each key's layer
    with its own image's decoded texels.
  - FR-4.1-S3: WHEN the client starts against the shipped content THE SYSTEM
    SHALL assign `base:water` a layer and draw no face from it.
  - FR-4.1-S4: IF the index names a key the session assigned no layer THEN THE
    SYSTEM SHALL complete the launch and leave that image unsampled.

- **FR-4.2**: A key the built set does not cover falls back to the texture
  generated from the key, and the run continues.
  - FR-4.2-S1: WHEN the client starts with a current built set that does not
    cover the key `example:undrawn`, which a loaded block declares, THE SYSTEM
    SHALL fill that key's layer with the texture generated from the key and
    SHALL complete the launch.
  - FR-4.2-S2: WHEN one declared key is covered by the current set and another
    is not THE SYSTEM SHALL fill the first from its image and the second from
    its generated texture in the same run.
  - FR-4.2-S3: IF the built set is present and current and covers no key any
    loaded block declares THEN THE SYSTEM SHALL complete the launch, drawing
    every face from a generated texture.

- **FR-4.3**: An image the set offers that the array texture cannot hold is
  refused rather than uploaded.
  - FR-4.3-S1: IF an image named by the index is 32×32 rather than 16×16 THEN
    THE SYSTEM SHALL refuse the launch and name the key, 32×32 and 16×16.
  - FR-4.3-S2: IF an image named by the index is not readable as a PNG THEN THE
    SYSTEM SHALL refuse the launch and name the key and the file.

### FR-5 — A missing or stale set refuses the launch by name

- **FR-5.1**: The client judges the built set against its sources and reports one
  of a fixed set of verdicts, refusing the launch on every verdict but the two
  that name a set it can proceed without.
  - FR-5.1-S1: IF the built set's index is absent THEN THE SYSTEM SHALL report
    the set as absent, refuse the launch, and name the command that builds it.
  - FR-5.1-S2: IF a model the manifest reaches has been edited since the set was
    built THEN THE SYSTEM SHALL report the set as stale against its sources,
    refuse the launch, and name the command that rebuilds it.
  - FR-5.1-S3: IF the manifest has gained an entry since the set was built THEN
    THE SYSTEM SHALL report the set as stale against its sources and refuse the
    launch.
  - FR-5.1-S4: IF a source the index recorded is no longer present at launch
    THEN THE SYSTEM SHALL report the set as stale against its sources and name
    the missing source, rather than reporting it as current.
  - FR-5.1-S5: IF the index names an image that is not present THEN THE SYSTEM
    SHALL refuse the launch and name the missing image and the key it belongs
    to.
  - FR-5.1-S6: WHEN the set is present and current THE SYSTEM SHALL report the
    set as current and complete the launch.
  - FR-5.1-S7: WHEN the index names zero keys and is current against its sources
    THE SYSTEM SHALL report the set as current and complete the launch.
  - FR-5.1-S8: WHEN a content root states no texture manifest THE SYSTEM SHALL
    report that the root declares no art, complete the launch, and draw every
    face from a generated texture.

- **FR-5.2**: The refusal for an unbuilt set and the fallback for an unauthored
  key are distinguishable.
  - FR-5.2-S1: IF the built set is absent entirely THEN THE SYSTEM SHALL emit
    the refusal naming the build command and SHALL NOT emit the wording used for
    a key the set does not cover.
  - FR-5.2-S2: WHEN the built set is present and current but covers no key a
    given block declares THE SYSTEM SHALL complete the launch without emitting
    the refusal naming the build command.

### FR-6 — Terrain is sampled through a mip chain

- **FR-6.1**: The array texture carries a full mip chain, each level the box
  average of the level above it computed in linear light.
  - FR-6.1-S1: WHEN a 16×16 image is prepared for upload THE SYSTEM SHALL
    produce five levels sized 16, 8, 4, 2 and 1.
  - FR-6.1-S2: WHEN a 2×2 image holding two texels of stored byte 0 and two of
    stored byte 255 in one channel is reduced THE SYSTEM SHALL produce a single
    texel of stored byte 188 in that channel.
  - FR-6.1-S3: WHEN an image whose every texel is one colour is reduced THE
    SYSTEM SHALL produce that same colour at every level.
  - FR-6.1-S4: WHEN a 4×4 image is reduced THE SYSTEM SHALL set each texel of
    the 2×2 level to the linear-light average of exactly the four texels it
    covers, and not of any other four.
  - FR-6.1-S5: IF a layer is offered for upload with fewer levels than the array
    texture declares THEN THE SYSTEM SHALL refuse the upload and name the key
    and the number of levels offered.

- **FR-6.2**: Terrain magnifies with nearest sampling and minifies with linear
  sampling across and between mip levels.
  - FR-6.2-S1: WHEN a face is drawn large enough that one texel covers more than
    one screen pixel THE SYSTEM SHALL draw a hard edge between two neighbouring
    texels of differing colour, rather than a gradient across them.
  - FR-6.2-S2: WHEN a distant face is captured at two camera positions differing
    by half a texel THE SYSTEM SHALL produce two captures differing in fewer
    pixels than the same pair captured through unfiltered minification.
  - FR-6.2-S3: WHEN the terrain sampler is requested THE SYSTEM SHALL request
    exactly nearest magnification, linear minification, linear mip interpolation
    and no anisotropy.
  - FR-6.2-S4: WHEN the same inspection is applied to a sampler request that
    does ask for anisotropy THE SYSTEM SHALL report that request as asking for
    anisotropy.
  - FR-6.2-S5: IF the device refuses the terrain sampler THEN THE SYSTEM SHALL
    refuse the launch and name the filter combination it requested.

### FR-7 — Generated art is built, never committed

- **FR-7.1**: The built set is absent from version control and the quality gate
  enforces it.
  - FR-7.1-S1: IF the gate runs against a tree in which a built image has been
    committed THEN THE SYSTEM SHALL fail that stage and name the committed path.
  - FR-7.1-S2: WHEN the gate runs against a tree carrying the manifest, the
    models and the materials and no built image THE SYSTEM SHALL pass that
    stage.
  - FR-7.1-S3: WHEN the gate runs its instrumented path THE SYSTEM SHALL build
    the set before the stage that runs the tests.
  - FR-7.1-S4: WHEN the gate runs its coverage-skipping path THE SYSTEM SHALL
    build the set before the stage that runs the tests.
  - FR-7.1-S5: IF the set build fails while the gate runs THEN THE SYSTEM SHALL
    fail that stage and reproduce the build's refusal.
  - FR-7.1-S6: IF the set build fails while the gate runs THEN THE SYSTEM SHALL
    not run the test stage against the set built previously.

- **FR-7.2**: A change to the art's sources fails distinguishably from a change
  to the renderer.
  - FR-7.2-S1: IF a model the manifest reaches is edited and the tests are run
    without rebuilding THEN THE SYSTEM SHALL report the set as stale against its
    sources rather than as a golden-frame mismatch.
  - FR-7.2-S2: IF the tests are run with no set built at all THEN THE SYSTEM
    SHALL report the set as absent rather than as a golden-frame mismatch.

### FR-8 — The shipped blocks carry their real art

- **FR-8.1**: The base game's grass, dirt and stone draw their baked textures.
  - FR-8.1-S1: WHEN the client starts against the shipped content and the built
    set THE SYSTEM SHALL draw the grass block's positive-Y face from the image
    baked from the grass model's `top` face.
  - FR-8.1-S2: WHEN the client starts against the shipped content and the built
    set THE SYSTEM SHALL draw the grass block's negative-Y face from the same
    image the dirt block draws on all six of its faces.
  - FR-8.1-S3: WHEN the client starts against the shipped content THE SYSTEM
    SHALL draw the grass block's four side faces from four images whose decoded
    texels are pairwise unequal.
  - FR-8.1-S6: (added 2026-08-19, after the owner found the defect below)
    WHEN each of the grass block's six baked images is compared against the
    model's own outermost voxels along that face's view axis THE SYSTEM SHALL
    agree texel for texel under the identity transform, and disagree under at
    least one other of the eight dihedral transforms. **The second clause is the
    discrimination proof**: a laterally symmetric image would agree under a
    mirror too, and an assertion that cannot tell the transforms apart is not an
    orientation claim. Measured on the shipped art: identity scores 0 on all six
    faces, every other transform 82–210 of 256, and **between 82 and 170 of 256
    texels differ from each image's own horizontal mirror** — east 82, south 90,
    the underside 96, north 100, west 106, the top 170 — so no shipped image is
    palindromic and the art can see a reversal.

    **Corrected 2026-08-19.** This clause first read "224 of 256", which no
    shipped image gives. The figure came from a report, went into this file
    unmeasured, and was caught by the test author measuring it twice while
    writing S8's fixture. The claim it was supporting survives; the number did
    not. **Third figure in this spec to be stated from a report rather than from
    a measurement** — the two before it are in `docs/technical/testing.md` under
    "An independent oracle is not a checked one", and this one is mine.
  - FR-8.1-S8: (same) WHEN a frame of the shipped replay world is captured
    THE SYSTEM SHALL draw one grass block's side face with the image's own
    left-to-right order rather than reversed. **FR-8.1-S7 cannot catch this** —
    turf above and dirt below is preserved by a pure lateral reversal — so the
    horizontal half of a face's orientation needs its own claim.
  - FR-8.1-S7: (same) WHEN a frame of the shipped replay world is captured
    THE SYSTEM SHALL draw, high and low on one side face of one grass block, a
    turf colour above and a dirt colour below.

  **Why these two exist, recorded because it is worth more than the scenarios.**
  The first mint of the golden set drew the grass sides **rotated** — turf along
  the bottom edge on some faces and running vertically on others — and **not one
  of 1366 tests could see it.** FR-8.1-S3 asks the four sides to be pairwise
  *unequal*, which a rotation preserves; FR-8.1-S5 judges the *top* face, which
  was correct; `terrain_probes`, the ΔE figures in `## Notes`, the
  distinct-colour counts and `share_within_any` all compare **means, histograms
  or set membership**, and **every one of those is invariant under rotation,
  reflection and permutation**. The goldens were minted from the broken output
  and so enshrined it.

  **What the defect turned out to be, measured.** Not the bake — the bake agrees
  with the model texel for texel on all six faces. **`mc_render::geometry::PLANE_AXES`
  was wrong in five of its six rows**: east and west had their axis pair
  exchanged so image-u carried world-up, north and south had both signs wrong,
  the bottom row's v sign was wrong, and **only the top row was correct — which
  is exactly why FR-8.1-S5 was green and honestly so.** Four distinct
  wrongnesses across six faces from one table, which is why the report was
  "turned in all directions" rather than "upside down". **The bottom row is the
  one nobody would have found**: a block's underside is never seen in this
  world, so it was corrected on the strength of the bake rather than of any
  test.

  **And the guard over that table could not have caught it.** `build/validate.rs`
  text-compares the Rust constant against its `terrain.wgsl` copy, so the two
  were held in agreement **by a machine** — and both agreed on the wrong answer.
  **A mechanical check that two copies of a constant match is not evidence about
  the constant.** The general form is in `docs/technical/testing.md`.

  **A colour-based reading cannot see geometry.** Every instrument this spec
  built measures *which colours are present*; none measured *where they sit*.
  These two scenarios are the only orientation claims in the feature, and the
  general form belongs in `docs/technical/testing.md`, which outlives this
  folder. **It was found by the project owner looking at the picture** — which
  is the standing rule about a green suite being no evidence, collecting.
  - FR-8.1-S4: WHEN a frame of the shipped replay world is captured THE SYSTEM
    SHALL match the golden frame committed for that capture.
  - FR-8.1-S5: WHEN a frame of the shipped replay world is captured THE SYSTEM
    SHALL draw, at the pixel the declared camera projects the grass block's top
    face onto, a colour within the declared tolerance of the mean of the built
    `base:grass_top` image's texels.

- **FR-8.2**: ~~The shipped models are reproducible from the generators the
  repository carries, on any checkout.~~ **Retired 2026-08-18, with all seven of
  its scenarios: FR-8.2-S1, S2, S3, S4, S5, S6 and S7.** The spec's scenario
  total drops from 108 to **101**.

  **What the seven asked for**, kept visible so no reader mistakes a retirement
  for an omission: S1 and S2 asked each generator, run from a repository root, to
  write bytes equal to the tracked model; S3 asked the grass assembler to take
  its header from the tracked model rather than from a second copy; S4 and S5
  asked a scan over every shipped model header to report that each generator path
  cited exists, with a control feeding it a path that does not; S6 and S7 asked a
  new gate stage to refuse a generator naming an absolute output path, with a
  control naming only relative ones.

  **Why they are retired.** The three generators are one-off scripts. They ran
  once, produced the models the repository tracks, and nothing re-runs them — so
  a standing test would re-prove a fact nothing can change. That fact is already
  measured: `architecture.md`'s `## The generator spike` records both models
  reproducing from their generators **byte for byte**, and S1 and S2 were to be
  that same comparison run again, permanently. Two narrower measurements point
  the same way. Exactly one shipped model header cites a generator at all
  (`grep -l "\.py" content/base/models/*.mcvox`), so S4/S5's scan, its positive
  control, its fixture and its mutation would guard one line in one file — a
  corpus of one does not earn a scan. And nothing downstream depends on FR-8.2:
  P5's T29 cited S3 rhetorically only, and P9's goldens come from the tracked
  model through voxforge. The cost avoided is five tasks, seven scenarios, six
  mutations and the workspace's first dependency on a Python interpreter.

  **On whose authority.** The project owner, 2026-08-18: *"Phase 4 is bogus. The
  shipped textures dont need to be reproducible. Voxforge already tests enough of
  the modelling. The python scripts were one-off scripts for generation. The
  generated artifact is sufficient. The models are committed in git. Nothing
  needs to validate that either git works or voxforge works."*

  **Recorded rather than deleted**, for the same reason the "Test premises this
  spec invalidates" list above is kept: a reviewer meeting a requirement that is
  simply gone cannot tell scope-**retired** from scope-**dropped**, and only one
  of those readings is a decision somebody made.

  **What survives, with no scenarios and no tests.** Phase 4 becomes a
  documentation-only phase. `grass-block.mcvox:57` cites `scratchpad/gen_grass.py`,
  a path that is not in the tree, and that citation is repaired. Each generator
  gains a header saying what it is — it ran once, produced the tracked model, and
  is kept as provenance rather than maintained or runnable as-is. The scripts are
  **kept**: the five hand-authored courses in `assemble_grass.py` (`y = 10`…`14`,
  the sod shadow and three lone blades at deliberately different depths) are
  design intent recorded nowhere else and unrecoverable from pixels. And
  `docs/modding/voxel-models.md` states that the tracked models are the source of
  truth, that the generators are provenance and not a build step, and that
  model → generator is not a claim of any kind.

### FR-9 — A save records what a block looked like, per face

- **FR-9.1**: A block's recorded appearance folds every key it declares.
  - FR-9.1-S1: WHEN two blocks differ only in the key declared for their `north`
    facing THE SYSTEM SHALL record different appearances for them.
  - FR-9.1-S2: WHEN a block's declaration is unchanged THE SYSTEM SHALL record
    the appearance an FNV-1a-64 fold over the stated byte sequence computes,
    rather than a value derived from the standard library's hasher.
  - FR-9.1-S3: IF a save was written before appearances folded per-facing keys
    THEN THE SYSTEM SHALL report its blocks' appearances as changed rather than
    comparing values folded over different fields.
  - FR-9.1-S4: WHEN a save written before this spec is loaded THE SYSTEM SHALL
    report every hash it stores other than a block appearance as unchanged.

## Technical Considerations

**Quads stay geometric; resolution happens where vertices are built.** The
mesher continues to emit a `Quad` carrying a facing and a block name and no
resolved content. The packer takes a per-block, per-facing layer source
alongside the layer table, and the held-block indicator takes the same source.
This is option (b) of the two the issue posed. The decisive argument is in the
tree: `PreparedScene` retains `meshed: Vec<SectionQuads>` across a reload
deliberately, so a `Quad` carrying a resolved key would carry a stale one
exactly on the path built not to re-mesh. **FR-2.1-S4 is the scenario that
distinguishes the two options** — under option (a) it fails. Full reasoning and
the rejected option (a) are in `requirements.md` §3.

**FR-2.1-S4's witness sits at the packer, not at a reload, and that is
deliberate.** An earlier wording said "WHEN a reload changes a block's `north`
key … and the sections were retained rather than re-meshed". A reload that
changes any texture key marks *every* section, and the dirty set is drained
whole into one batch, so no section is left retained-but-not-re-meshed by a
reload — the conjunction that wording described is unreachable in a running
client, and a reader would have believed a path was covered that is not. The
property itself is real and is exercised where it lives: the retained mesh is
re-packed against the resolution the packer currently holds, and a quad that
remembered a key would defeat that. Narrowing the marking rule so that a reload
*does* leave sections retained is a change to a rule
`docs/technical/rendering.md` states outright, and it belongs to whichever spec
takes it — not to this one.

**PRO-902 closes here, at both sites.** `geometry/mod.rs:186` and
`hud/held.rs:109` both parse a block's name as a texture key today;
`crates/mc-render/CLAUDE.md` warns that closing one alone leaves a block drawing
in-world with a blank indicator. Both are replaced by the same lookup, and
neither parses a block name afterwards. FR-2.1-S1 and FR-2.2-S2 are the two
witnesses, one per site.

**Mip levels are averaged in linear light, not over stored bytes.** The array
texture is `Rgba8UnormSrgb`, and `crates/mc-render/src/gpu/buffers.rs:10-16`
already records that this is load-bearing: a texel is decoded to linear on
sample. Averaging the stored bytes of 0 and 255 gives 128, which decodes to
linear 0.216 rather than 0.5 — every mip level would come out darker than the
level above it, which is the classic sRGB mipping fault and is, in that module's
own words about the neighbouring trap, "plausible-looking, and wrong in the
direction nothing notices". Each level therefore decodes to linear, averages,
and re-encodes; FR-6.1-S2 pins the resulting byte at 188 precisely because 128
is what the wrong implementation produces, and a scenario saying only "midway
between" would accept both.

**The client decodes; the renderer stays pure.** `mc-render` has no `std::fs`,
no `PathBuf` and no image decoder anywhere in `src/`, and this spec does not
give it one — it gains the ability to be *handed* texels for a key, falling back
to the generated texture where it is handed none. The read and the PNG decode
land in `mc-client`, which is already the composition root and already the only
crate that builds `TextureLayers`. Decoding in `mc-sim` was rejected: pixels are
the client's half of the split (`docs/planning/client-server-split.md`), and a
server that needed them would break the asymmetry that makes a texture pack a
legal client modification and a block declaration not.

**The set's verdict is a total enumeration, not an absence check.** FR-5.1 is
written as one verdict per launch — current, absent, stale against sources,
image missing — because `assert!(no_refusal_printed)` cannot tell a healthy set
from a client that lost the ability to check. An enumerated verdict reddens for
free when the check stops looking, which is the failure mode
`standards/global/testing.md` §2 names.

**The manifest is content and is committed; the set is derived and is not.**
`content/base/textures.toml` names, per entry, a model, one of its six faces,
and the texture key that face becomes. `content/base/textures/` holds the built
images and the index. Neither path is reachable by the block loader, the HUD
loader or the reload watcher — all three read one named subdirectory and one
extension, one level deep, and this was verified rather than assumed.

**One hash, in `mc-core`, so two sides cannot drift.** VoxForge writes the
index; the client verifies it; `crates/` may not depend on `tools/` (SPEC-013
FR-9.1, enforced by `crates/mc-testkit/tests/workspace_layering.rs`). The
project already carries the right primitive with its rationale written down —
`fnv_1a_64` in `crates/mc-world/src/persistence/format.rs`, hand-written
precisely because the standard library's hasher is unspecified and moves with
the toolchain. It moves to `mc-core` unchanged, so no stored save hash changes
value (FR-9.1-S4 is what holds that), and there is one implementation rather
than two that must agree forever.

**The fold covers exactly what the manifest reached.** FR-3.4-S3 is a negative
control and is the scenario that stops "fold the whole `content/` tree", which
would satisfy every positive staleness scenario while turning each unrelated
content edit into a spurious launch refusal.

**The sampler choice is forced by wgpu, not preferred.**
`wgpu-core-30.0.0/src/device/resource.rs:2288-2316` refuses a sampler with
`anisotropy_clamp != 1` unless `min_filter`, `mag_filter` **and**
`mipmap_filter` are all `Linear`. Anisotropy and crisp voxel magnification
cannot both be had. PRO-869 part 1 asks for "filtered **or** anisotropic"; this
spec takes filtered. The bind-group layout already declares `filterable: true`,
so no layout change is needed, and no discrete-GPU fallback is owed because no
optional feature is taken. FR-6.2-S3 asserts the request and FR-6.2-S4 is its
positive control; FR-6.2-S1 and S2 assert the consequence in a captured frame,
because a test that reads back the descriptor it caused to be built is agreement
between two copies of one decision.

**Mips are generated on the CPU.** wgpu has no built-in mip generation, and a
GPU blit or compute pass would land in `src/gpu/`, the one subtree excluded from
coverage thresholds where golden frames are the only defence. A box filter over
a 16×16 chain is five levels of trivial arithmetic and a pure function under
normal coverage — which is what `crates/mc-render/CLAUDE.md` asks for.

**Goldens are re-shot in place at `r1`.** `SCENE_REVISION` identifies the scene
contract — pose, world, camera path, tick list, merge predicate, vertex format —
and this spec changes none of it. Bumping would redefine the revision as
"something visible changed" and oblige a bump for every future art change. The
re-shoot follows `docs/technical/rendering.md` verbatim: probes, then oracle,
then HUD prediction, then a mint naming only the `terrain_goldens` and
`hud_goldens` **binaries**. A bare `MYCRAFT_UPDATE_GOLDENS=1 cargo nextest run`
reaches `golden_mismatch` and corrupts the set permanently. FR-8.1-S5 exists
because FR-8.1-S4 alone is a snapshot minted from the renderer it verifies: the
derived probe judges the frame against a mean computed from the built PNG, which
shares no code with the draw.

**The save format's appearance hash gains the six keys and `INPUT_VERSION` goes
1 → 2.** That is the mechanism the file documents for this case. The consequence
is player-visible and intended: a save written before this spec reports every
block's appearance as changed on next load, through the existing
`--load-changed-blocks` path. Every block's appearance really did change.

**Test premises this spec invalidates**, listed so none is loosened until green.
**This list is what tells a reviewer an intended inversion from a broken pin.** A
reader meeting a test turned inside out, with the spec silent about it, has no way
to distinguish "this spec meant that" from "somebody adjusted a test to match their
implementation" — and the second reading is one this project is right to fear. Keeping
the list true is the only thing standing between them.

- `swatch.rs`'s `TEXEL_COLORS = 2` — a real face has three to six colours, measured in
  `requirements.md` §1.2. **Amended 2026-08-19, measured in Phase 8: this is not a
  threshold to widen.** `texel_colors` *errors outright* unless a layer holds exactly two
  colours, and the shipped art holds 3 (dirt, stone), 5 (`grass_top`) or 6 (the sides).
  Two colours was a property of the placeholder *generator*; shipped art has no such
  property, so the reading has to be rebuilt rather than retuned.
- `probe.rs`'s `STRATA` means, which cluster against `placeholder_mean_color` and must
  be re-derived from real texels with the ΔE constants re-checked against art far less
  separated than three hash-derived colours.
- ~~`terrain_offscreen.rs`'s centre-pixel comparison.~~ **Re-measured in Phase 9 and it
  stands — this entry was wrong.** Its fixtures are `example:` blocks no built set covers,
  so they draw generated checkerboards whose mean *is* the declared mean at every mip
  level: a minified frame converges towards the value it is compared against.
  `SAME_TEXTURE = 10.0` is untouched and both depth readings were green in every run.
  Recorded rather than deleted, because **a reader meeting an unchanged file and a list
  claiming otherwise cannot tell "looked at" from "overlooked".**

**Added 2026-08-19, measured in Phase 9. T53 takes the shipped texture-key set from four
to eight and renumbers every layer after `base:dirt`** — `base:grass` stops being a key at
all, `stone` moves 2 → 6 and `water` 3 → 7. **Twelve files pinned the old set and go red
against a *correct* T53**; all twelve are turned in place, and `test-map.md` records what
each one pinned. The three below were found by reading the tree. The other nine were not,
and that is the finding:

**Reading the tree found three of twelve; running it found twelve.** A search over one
spelling cannot see a fixture that computes its expectation from `SHIPPED_KEYS.len()`, or
one that treats a list of texture keys as a list of block names because the two used to
coincide. **Neither is a careless fixture** — both are derived rather than committed, which
is what this project asks for, and **being derived is exactly what makes them invisible to a
search.** So the remedy is not to survey harder: land the change and read the failures.
The nine are `reload_appends_layers.rs`, `reload_appends_a_drawable_layer.rs`,
`reload_builds_off_the_tick.rs`, `reload_keeps_packed_layers.rs`, `reload_layer_budget.rs`,
`reload_publishes_content.rs`, `reload_hand_shows_the_new_block.rs`,
`reload_refusal_ends_one_attempt.rs`, `reload_refuses_an_uncovered_facing_key.rs`,
`saved_world_texture_layers.rs`, `documented_refusals.rs` and `hud_prediction.rs`.
The general form is in `docs/technical/testing.md`, which outlives this folder.

**Two of the twelve came back stronger than the premise they replaced**, which is worth
recording because it is the usual outcome rather than a lucky one: taking `grass.luau` away
now retires **five** keys rather than one, so "stone and water keep 6 and 7 rather than
sliding to 1 and 2" is a **wider** claim; and `reload_keeps_packed_layers.rs` had been
packing one quad per *key*, using each key as a `BlockName` — correct only while the two
vocabularies coincided — and now packs one quad per *block*, which is the join the packer
actually crosses.

The three found by reading:

- `launch_texture_layers.rs` — pins `DIRT=0, GRASS=1, STONE=2` as the layers every
  committed golden was shot with, and pins `SHIPPED_TEXTURE_KEYS` as those four.
- `mc-sim/tests/per_face_layers.rs` — `SHIPPED_KEYS: [&str; 4]`, guarded by
  `the_shipped_root_spends_one_layer_per_key`.
- **`hud_held_block.rs` — the one no plan predicted.** It paired dirt against grass as two
  blocks sharing no colour. The grass block's north facing now draws
  `base:grass_side_north`, which holds the three dirt colours **byte for byte**, so "every
  indicator pixel differs" would be red against a correct renderer. It is re-paired against
  **stone**, whose nearest colours stand ΔE 21.13 apart.
- **`stated_layers_are_honoured.rs`'s two pins on PRO-902's gap** —
  `a_block_whose_declared_texture_is_not_its_own_name_is_refused_at_packing_time_naming_the_block`
  and `the_held_block_indicator_resolves_by_the_blocks_name_too_and_shows_nothing_for_such_a_block`.
  Both assert that a block declaring a key other than its own name draws **nothing**,
  which FR-2.1 and FR-2.2 make false. They are **inverted in place rather than deleted**,
  because they reach the property through `ContentView` — the client's own construction of
  a resolution out of resolved content — and no scenario's own test takes that route.
- **The prose that described the gap as open**, in
  `crates/mc-client/tests/reload_keeps_packed_layers.rs`'s header and in
  `support/reload.rs`'s `Declaration::of` and `repointing_north`. Both said in writing that
  the red those pins turn when PRO-902 closes "is its success signal", so both are retired
  with them. **The knowledge was in the code and missing from this list**, which is the
  safer direction of the two and still a defect in the list.

The placeholder's own tests stay valid and keep guarding the fallback path, which is not
deleted.

Amended 2026-08-18, after Phase 3 met a fourth premise the list had missed:
`stated_layers_are_honoured.rs` pinned PRO-902's resolution-by-name gap **as
current behaviour** in two tests, and `reload_keeps_packed_layers.rs`'s header
and `support/reload.rs`'s `Declaration::of` both said in writing that the red
they turn when the gap closes is the success signal. Both tests are inverted in
place by this spec —
`a_block_whose_declared_texture_is_not_its_own_name_packs_from_the_key_it_declared`
and `the_held_block_indicator_shows_the_layer_of_the_key_the_block_declared` —
and the prose is retired in all three files. **The amendment is owed because an
inverted test and a test adjusted to match an implementation are
indistinguishable afterwards**; this list is what separates the two readings,
so a premise met late is recorded here rather than only in the commit that
turns it.

## Existing Code to Leverage

| What | Location | Reuse |
|------|----------|-------|
| Six-face bake, seam judgement | `tools/voxforge/src/texture/{emit,set,seam}.rs` | `build` drives the existing `emit` per entry; `SeamPolicy::Required` is FR-3.3-S7's refusal |
| All-or-nothing delivery | `tools/voxforge/src/cli/mod.rs` (`written_together`) | already deletes landed files when a later write fails — FR-3.3-S10 extends it across entries |
| PNG encoding | `tools/voxforge/src/render/mod.rs:394` | unchanged; determinism already graded by `tests/preview_determinism.rs` |
| Subcommand dispatch | `tools/voxforge/src/cli/args.rs` | `build` is a fourth `Command` variant beside `preview`, `inspect`, `texture` |
| Stable byte fold | `crates/mc-world/src/persistence/format.rs` (`fnv_1a_64`) | moves to `mc-core`; the index's recorded value and the save's hashes share it |
| Stated key→layer table | `crates/mc-core/src/content.rs` (`LayerAssignment`), `mc-render` (`TextureLayers`) | unchanged contract; per-face keys widen only what contributes keys to it |
| Layer budget refusal | `crates/mc-core/src/content.rs` (`LayerBudget`) | FR-1.4's refusal is the existing one, reached by a new route |
| Layer upload path | `crates/mc-render/src/gpu/buffers.rs:158-197` | gains supplied texels and a mip loop; the layer-range refusal stays |
| Per-key fallback | `crates/mc-render/src/texture/placeholder.rs` | not deleted, not repurposed — it becomes the documented fallback for an unauthored key |
| Held-swatch states | `crates/mc-render/src/hud/held.rs` (`HeldSwatch`) | `NothingHeld` / `Shows` / `Unresolved` already total — FR-2.2 changes how they are reached, not what they are |
| Launch refusal contract | `crates/mc-client/src/startup.rs` (`PreparationError`) | the set verdict's refusals are variants beside `NoContentRoot`, failing the run rather than degrading it |
| Declaration validation and refusal wording | `crates/mc-world/src/content/luau_declaration.rs` | the per-facing refusals follow its existing `FieldFault` shapes and its recognised-field ordering contract |
| Refusal-text conformance | `crates/mc-client/tests/documented_refusals.rs` | FR-1.2-S8's witness; the guide and the code stay line-for-line |
| Golden lifecycle and re-shoot procedure | `docs/technical/rendering.md`, `crates/mc-client/tests/{terrain,hud}_goldens.rs` | followed verbatim; no new golden machinery |

## Out of Scope

Binding.

- **A top/side/bottom triple form.** PRO-914's inherited shape called it the
  common case; no shipped block wants it, it is the only form needing a
  precedence rule between `side` and a named facing, and adding it later is a
  pure relaxation while removing it is not. Reasoning in `requirements.md` §4.2.
- **PRO-904's drawn/occludes solidity split.** Ruled out of this spec by the
  conductor after reading PRO-947's full scope; PRO-904 follows immediately and
  pays a second golden re-shoot deliberately. No solidity semantics change here.
- **Making water visible.** `base:water` is declared non-solid and therefore
  emits no faces at all; that is PRO-952's subject. Water keeps its key and
  spends its layer, which FR-4.1-S3 asserts, and draws nothing.
- **Anisotropic sampling.** Excluded by the wgpu constraint above, not deferred
  for cost. Revisiting it means giving up nearest magnification.
- **PRO-869 part 2** — the palette generator's pairwise separation for blocks
  that ship no texture. Untouched; the placeholder generator is unchanged.
- **A texture pack, or any client-side override of the built set.** The split
  that makes one legal is stated and relied on; the mechanism is not built.
- **Any texture resolution other than 16×16.** Decided by measurement
  (`requirements.md` §1.2), not left open.
- **Art for any block beyond grass, dirt and stone.**
- **Chasing the stale doc comment in `tools/voxforge/tests/command_line.rs`.**
  It asserts nothing, so nothing this spec adds can invalidate it
  (`requirements.md` §2.1).

## Dependencies

- **ADR-026** (`docs/technical/decisions.md`) — decided, and this spec is its
  first consumer. Its five owed items land as FR-5.1 (item 1), FR-7.1-S3 and S4
  (item 2), FR-3.4 (item 3), FR-7.1-S1 (item 4) and FR-3.2 (item 5).
- **SPEC-013 FR-9.1** — `tools/` may depend inward on `crates/`, nothing in
  `crates/` may depend on `tools/`. Not amended; the shared hash in `mc-core` is
  what respects it.
- **PRO-917's stated layer assignment** — keys are appended and never
  renumbered, because a layer index rides inside every packed vertex. Per-face
  keys widen what contributes keys; they do not change that rule.
- **VoxForge** must remain able to bake `grass-block.mcvox` and
  `stone-block.mcvox`. Verified at `8dea90d`: the bake reproduces from the
  checked-in model across six faces at two resolutions, with `--seamless`
  accepting all twelve. The sha256 of each is recorded in `requirements.md` §1.1
  so the check can be re-run.

## Assumptions

- The 16×16 art the shipped models bake to is the art that ships. Verified by
  measurement, not assumed (`requirements.md` §1.2).
- Every shipped model is exactly `scale` voxels on all three axes, which is what
  lets a six-face emit work. VoxForge already refuses otherwise, naming the
  axis.
- A contributor can run `cargo run -p voxforge -- build content/base/textures.toml`.
  ADR-026 accepts that `cargo build` alone no longer produces a complete game,
  and FR-5.1 is the whole of the mitigation.
- `assemble_grass.py` is the assembler that produced the checked-in grass model.
  Its output is not independently verifiable by any standing test, because the
  model is assembled from generated noise plus hand-authored courses that exist
  nowhere else — and since FR-8.2's retirement that is **by design**, not a gap
  awaiting a scenario: the script ran once, nothing re-runs it, and a test would
  hold nothing that can move. The tracked `grass-block.mcvox` remains the source
  of truth, which is what §1.1's recorded digests are against.

## Open Questions

None.

## Clarifications

### Session 2026-08-18

- Q: Does the art the spec ships actually come from the checked-in model? → A:
  Yes, and nothing pre-baked ships. The bake reproduces from
  `grass-block.mcvox` across six faces at two resolutions, verified by sha256
  and recorded in `requirements.md` §1.1; `--seamless` accepted all twelve.
  Model and tool are both intact.
- Q: Which of the two baked resolutions ships? → A: 16×16. The 192×192 set was
  decoded and is an exact 12× nearest-neighbour magnification — 0 non-uniform
  sub-pixels, 0 disagreements with the 16×16 — so it carries no information at
  144× the texels.
- Q: Where do the generators live? → A: Beside the model, at
  `content/base/models/generators/`, where two of the three already are. All
  three content scans were checked and none can see the directory.
- Q: Is the reserved-colour absence assertion in `command_line.rs` a trap for
  this spec? → A: No — there is no assertion, only a doc comment on a
  temp-directory fixture constant. The obligation the issue imposed dissolves.
- Q: Option (a) or (b) for texture resolution? → A: (b). Retained meshed
  sections across a reload would carry a stale resolved key under (a), which is
  what FR-2.1-S4 tests.
- Q: Anisotropic or filtered sampling? → A: Filtered. wgpu 30 refuses
  anisotropy unless magnification is also linear, which would give up the voxel
  aesthetic.
- Q: In which colour space are mip levels averaged? → A: Linear. The array
  texture is sRGB and decodes on sample, so averaging stored bytes would darken
  every level. Raised by the scenario audit; FR-6.1-S2 pins the byte that
  distinguishes the two implementations.
- Q: Re-shoot the goldens or bump `SCENE_REVISION`? → A: Re-shoot at `r1`. The
  scene contract is unchanged; only pixels and sampling change.
