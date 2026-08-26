---
id: SPEC-025
title: Water has no baked texture, so the sea is a stand-in checkerboard
status: implemented
work-type: fix
rigor: high
branch: feature/PRO-972-water-texture
issue: PRO-972
created: 2026-08-26
approved: 2026-08-26
completed: 2026-08-26
author: Sebastian Grunow
---

# Fix: Water has no baked texture, so the sea is a stand-in checkerboard

## Defect

- **Observed**: a human launched a shipped build and reported the sea rendering
  as a magenta-and-black checkerboard. It is drawing the *generated stand-in*
  for `base:water`, which is a pure function of the key: FNV-1a over
  `"base:water"` is `0x65098d12c9940ca1`, so its declared mean is
  **RGB (150, 48, 141)** and its texels alternate **(160, 58, 151)** and
  **(140, 38, 131)** on a checkerboard
  (`crates/mc-render/src/texture/placeholder.rs:44-128`). The reported "black" is
  the darker of that pair against a lit sky, not a second colour.
- **Expected**: the sea draws a baked image, the way grass, dirt and stone do.
  The generated stand-in is *correct behaviour* for a key nothing has baked and
  must keep working exactly as it does — it is what stops a mod author's first
  block refusing a launch.
- **Reproduced**: 2026-08-26, on `a9c6663`, three independent readings.
  1. `content/base/textures.toml` declares seven `[[texture]]` entries and none
     of them names `base:water`; the four shipped block declarations name
     **eight** distinct keys, so exactly one is uncovered.
  2. `cargo run -p voxforge --quiet -- build content/base/textures.toml` →
     `nothing needed rebuilding`, and `content/base/textures/index.txt` records
     seven `key` lines, none of them `base:water`.
  3. **The stand-in is in the committed reference images.** Decoding the four
     golden captures under `crates/mc-render/goldens/` and counting the two
     stand-in texel values verbatim:

     | capture | `(160,58,151)` | `(140,38,131)` | mip mean `(150,49,141)` | share of frame |
     |---|---|---|---|---|
     | `player-walk-t000-r3` | 30 681 | 30 657 | 16 649 | 9.58 % |
     | `player-walk-hud-t000-r3` | 30 681 | 30 657 | 16 649 | 9.58 % |
     | `player-walk-t059-r3` | 72 885 | 72 751 | 19 596 | 18.96 % |
     | `player-walk-t119-r3` | 89 334 | 89 199 | 13 259 | 21.57 % |

     The near-equal counts of the two are the checkerboard's own parity, and the
     third column is the same colour after minification. **All four committed
     goldens show it.**

## Root Cause

`base:water` is a **declared** texture key with no **baked** image, and every
link in the chain behaves as designed:

1. `content/base/blocks/water.luau:48` declares `texture = "base:water"`, and
   `water.luau:52-53` declares `drawn = true, occludes = false`, so the mesher
   emits faces for it.
2. `content/base/textures.toml` states seven `[[texture]]` entries — the six
   grass-block faces and stone — and no entry keyed `base:water`.
3. `voxforge build` bakes exactly what the manifest names, so no
   `base__water.png` is written and `index.txt` records no such key.
4. **An uncovered key is an ordinary answer, not a refusal.**
   `crates/mc-client/src/textures/mod.rs:211` returns `SetVerdict::Current` and
   the launch proceeds; `crates/mc-render/src/texture/mip.rs:111-128` says so in
   as many words and falls back to `placeholder_texels(key, size)`.
5. `crates/mc-render/src/texture/placeholder.rs:83` generates the magenta
   checkerboard from the key alone.

Nothing here is broken. The fault is a **missing manifest entry and the art
behind it** — the one input no code can supply.

## Regression Scenarios

Rules: `standards/global/scenario-guidelines.md`. Two scenarios below —
FR-2.2-S1 and FR-4.1-S2 — assert behaviour that is **already green and must stay
green**; they are preservation scenarios, marked as such, and are the only two
that do not go red before the fix.

### FR-1 - The shipped set covers every key the shipped content declares

- **FR-1.1**: The shipped content root's declared texture keys and its built
  set's covered keys are the same eight.
  - FR-1.1-S1: WHEN the shipped content root's block declarations are read
    against its built set THE SYSTEM SHALL report every one of `base:dirt`,
    `base:grass_side_east`, `base:grass_side_north`, `base:grass_side_south`,
    `base:grass_side_west`, `base:grass_top`, `base:stone` and `base:water` as
    covered, as a single enumerated verdict naming all eight in order.
  - FR-1.1-S2: IF a content root declares a texture key its manifest bakes no
    entry for THEN THE SYSTEM SHALL name that key in the verdict as uncovered,
    rather than reporting the root as fully covered.

### FR-2 - Water draws its baked art, and an uncovered key still draws a stand-in

- **FR-2.1**: `base:water`'s layer is filled from the shipped set.
  - FR-2.1-S1: WHEN a launch fills `base:water`'s layer from the shipped built
    set THE SYSTEM SHALL fill it with the 256 texels of the image the manifest
    bakes for that key, decoded by the client's own reader.
  - FR-2.1-S2: WHEN `base:water`'s filled layer is inspected THE SYSTEM SHALL
    hold neither `(160, 58, 151)` nor `(140, 38, 131)` at any of its 256 texels.
- **FR-2.2**: The stand-in is unchanged. *(preservation)*
  - FR-2.2-S1: IF a block declares a texture key the built set covers no entry
    for THEN THE SYSTEM SHALL fill that key's layer from the generated stand-in
    and let the launch proceed, reporting the set as current.

### FR-3 - The art reads as water and sits in the shipped set

Every bound below is **derived from the seven images already shipped**, measured
on 2026-08-26, not chosen to fit a candidate.

- **FR-3.1**: `base:water`'s baked image is unambiguously blue.
  - FR-3.1-S1: WHEN `base:water`'s baked image is inspected THE SYSTEM SHALL
    hold, at every one of its 256 texels, a blue channel strictly greater than
    both that texel's red and its green channel.
  - FR-3.1-S2: WHEN `base:water`'s linear-light mean is compared against the
    linear-light mean of each of the other seven shipped images THE SYSTEM SHALL
    stand at least dE 10 from every one of them - the figure this project
    already calls two colours told apart. *(The nearest existing mean is
    `base:stone`; the approved palette below stands dE 25.34 from it and dE 51+
    from the rest. The bound may not be restated as a set-wide invariant: the
    four grass sides stand dE 0.47-1.05 from each other and dirt stands dE 9.59
    from `base:grass_side_west`.)*
- **FR-3.2**: Water is the smoothest surface in the set, and still not flat.
  - FR-3.2-S1: WHEN the widest pairwise dE among `base:water`'s distinct colours
    is measured THE SYSTEM SHALL report a value greater than 2 and no greater
    than 16.10 - above the floor at which this project calls two texels
    distinguishable, and no more mottled than `base:dirt`, the flattest image
    the set already ships. *(Measured over the shipped seven: dirt 16.10,
    grass_top 17.01, stone 22.87, the four grass sides 55.51.)*

### FR-4 - The committed goldens are re-shot deliberately

- **FR-4.1**: No committed reference image shows the stand-in.
  - FR-4.1-S1: WHEN each of the four committed golden captures is decoded THE
    SYSTEM SHALL hold no texel equal to `(160, 58, 151)`, `(140, 38, 131)` or
    `(150, 49, 141)`. *(Currently 88 280, 88 280, 174 744 and 198 828 such
    pixels respectively - see the Defect table.)*
  - FR-4.1-S2: WHEN the scene contract is evaluated over the world the captures
    are shot from THE SYSTEM SHALL report the same quad count, total face area
    and per-block face area as it does before this fix. *(preservation - this is
    the evidence that no `SCENE_REVISION` bump is owed.)*

## Which goldens move, and why the revision does not

**All four directories are re-shot, in place, at `r3`.** Every one of them holds
the stand-in checkerboard today (Defect, reading 3), so every one of them is a
photograph of the defect.

**`SCENE_REVISION` stays at `r3`.** The constant identifies the *scene
contract* - pose, world, camera path, tick list, merge predicate, vertex format
(`crates/mc-render/src/capture.rs:11-47`) - and this fix moves none of them:

- **The key set does not change, so no layer renumbers.** Layers are assigned
  positionally over the keys the *block declarations* name, in lexicographic
  order (`crates/mc-client/tests/launch_texture_layers.rs:195-198`).
  `base:water` is already one of those eight keys and already sorts last;
  baking an image for it adds no key and moves no index. A layer index rides
  inside every packed vertex, which is what made the 2026-08-19 art spec's
  four-keys-to-eight change re-shoot the set - and that change is not this one.
- **No physics, spawn or camera-path input moves**, which is what forced the
  `r1` to `r2` and `r2` to `r3` bumps on record (`docs/technical/rendering.md`,
  "Re-shoots on record").
- This is therefore the same shape as the 2026-08-19 entry, which re-shot four
  directories in place and held the revision: *art changed, the contract did
  not.* Bumping for an art edit would redefine the revision as "something
  visible changed" and oblige a bump for every future art edit - the reading the
  ids exist not to carry.

The re-shoot follows `docs/technical/rendering.md` "Re-shooting a golden set"
verbatim: probes, then oracle, then HUD prediction, then a mint naming only
`--test terrain_goldens --test hud_goldens`, then a verification run with
`MYCRAFT_UPDATE_GOLDENS` **unset** and `golden_mismatch` selected, then
`golden_inventory`. **`MYCRAFT_UPDATE_GOLDENS` is never set for a run that can
reach `golden_mismatch`, including a bare `cargo nextest run`** - that is the run
that corrupts the set.

## The palette

**Binding**, approved 2026-08-26. Derived from the set's own materials
rather than chosen: the
three shipped terrain bases sit at S about 50 %, V 49-66 %, and their tone
spread narrows as the surface smooths (stone +/-29 per channel, dirt +/-18,
grass +/-11, with `grass_dark.toml` stating the rule in as many words). Water is
the smoothest surface in the set, so it takes the narrowest step, +/-8.

| material | colour | role |
|---|---|---|
| `base:water` | `#4c799e` (76, 121, 158) | H 207 deg, S 52 %, V 62 % |
| `base:water_dark` | `#447196` (68, 113, 150) | -8 per channel |
| `base:water_light` | `#5481a6` (84, 129, 166) | +8 per channel |

Measured: widest pairwise dE 6.29 among the three (FR-3.2-S1 admits it), dE
25.34 from `base:stone` and at least 51 from every other shipped mean (FR-3.1-S2
admits it), dE 59.20 from the stand-in it replaces, and blue exceeds both other
channels on all three (FR-3.1-S1 admits it).

Shape: `content/base/models/water-block.mcvox`, a 16-cubed model at `scale = 16`
with a *generated sparse* speckle over the three tones - mostly base, occasional
accents - following `stone-block.mcvox`'s structure but at lower density,
because a sea surface has no authored shape the way a sod line does. Baked from
the `top` face, which is the face a player sees.

## Out of Scope

Binding.

- **Anything about water *behaviour***: swimming, `move_resistance`, targeting
  from inside water (PRO-961). No field of `water.luau` other than the file's
  art is touched.
- **Transparency or alpha blending.** Water is drawn opaque; `occludes = false`
  is what lets the lakebed show through by leaving faces unculled, and no
  blending path exists or is added.
- **The stand-in generator**, its colours, its checkerboard, and the launch
  notice that describes it. Unchanged.
- **Any `SCENE_REVISION` change**, and any change to the capture ticks, the
  declared camera path, the spawn column or the world.
- Frame pacing (PRO-971) and the RDP pointer fallback (PRO-962).
- The four bare test counts in `docs/` that rot (PRO-985), and every other open
  issue. No sweeps.

## Three premises this fix falsifies, and must repair

Binding, ruled in scope by the approving authority on 2026-08-26. Each is a
statement in the tree that **stops being true the moment `base:water` is baked**,
so each belongs to the commit that makes the change rather than to a later sweep.

1. **`docs/modding/voxel-models.md:354-358` is false and this spec's own subject
   is what falsifies it.** It tells a mod author *"What it does not do yet is
   draw from it — the faces you bake are checked, named and then not painted,
   and the pictures on screen are still the generated placeholder textures. So a
   set you build today is a set the client will hold you to and not yet one you
   can see."* That stopped being true on 2026-08-19. It is the exact sentence
   that lets this defect read as intended behaviour: somebody bakes a set, sees
   a checkerboard, reads that page and concludes the client works as documented.
   Rewrite it to say a built set is drawn. *(The paragraph's opening sentence at
   353-354 — the client judges a set at every launch and refuses a missing or
   stale one — is true and stays.)*
2. **`crates/mc-client/tests/support/art.rs:55-62` states a tolerance as
   "Measured, not chosen. Over the **seven** images the shipped manifest bakes
   the widest separation is dE 2.38."** An eighth image makes that sentence's own
   premise false. **Re-measure over eight at implement and rewrite the sentence
   to match**, whether or not `MEANS_AGREE_WITHIN = 3.0` still holds. A tolerance
   whose stated derivation no longer describes the set is the same defect class
   as everything else on this branch.
3. **The detection gap belongs in `docs/technical/testing.md`, not only in this
   folder.** The oracle at `support/art.rs:71-75` falls back to the same
   generator the product does, and the goldens are minted from the renderer they
   grade — a closed loop with no outside reference anywhere in it. It is the
   sharpest instance this project has of **an oracle that is correct and
   useless**, and it earns a named entry beside the existing falsifiability
   rules rather than an archived spec nobody reads.

## Notes

Out-of-scope observations recorded, not built.

1. **`specs/archive/2026/2026-08-26-framerate-independent-speed/spec.md`
   carries `id: PRO-971` where `docs/INDEX.md` attributes that work to
   SPEC-023.** The two places that hold spec identity disagree. SPEC-025 is
   nonetheless free: SPEC-024 is `2026-08-26-absolute-pointer-fallback`
   (PRO-962), confirmed from its own frontmatter, and no folder or INDEX row
   claims SPEC-025.

## RCA

### Causal chain

| # | Link | Evidence |
|---|---|---|
| 1 | Symptom: the sea draws a magenta checkerboard | four committed goldens, 9.58-21.57 % of frame (Defect, reading 3) |
| 2 | `base:water`'s layer is filled from the generated stand-in | `crates/mc-render/src/texture/mip.rs:126-128` |
| 3 | It is filled that way because the built set covers no such key | `crates/mc-client/src/textures/mod.rs:211` returns `SetVerdict::Current`; `mip.rs:111` states the rule |
| 4 | The set covers no such key because the manifest states no entry | `content/base/textures.toml`, seven `[[texture]]` tables |
| 5 | Origin: `9fc98d71` (2026-08-23) gave `water.luau` `drawn = true, occludes = false` and did not extend the manifest | `git show --stat 9fc98d71` touches `content/base/blocks/water.luau` and one Rust file, and **not** `content/base/textures.toml` |

**The manifest was not wrong when it was written.** `bdbb0214` (2026-08-21)
created `content/base/textures.toml` with seven entries at a moment when
`water.luau` declared no `drawn` field at all
(`git show bdbb0214:content/base/blocks/water.luau` names only `texture`), so
there was no water face to bake art for. The defect was introduced two days
later by the commit that made water visible while leaving the manifest alone.
The gap between an art build and the declaration set it must cover is what has
no keeper.

### Detection gap

Four instruments could have seen this. Each is correct, each is green, and none
of them is *about* this.

1. **The colour oracle falls back to the same generator the product does.**
   `crates/mc-client/tests/support/art.rs:71-75` — `drawn_texels` answers
   `supplied.covering(key)` *or else* `placeholder_texels(key, TEXTURE_EDGE)`.
   Every colour assertion in the client suite routes through it, so the probes,
   the replay oracle and the landmark comparisons all asked *"does water draw
   what an uncovered key should draw"* and were truthfully told yes. **This is
   the central hole**, and it is the shape `standards/global/testing.md` names:
   a fixture that is correct, and asserts truthfully about a world the product
   should not inhabit.
2. **The goldens are minted from the renderer they grade.** The checkerboard is
   the committed reference image, so `terrain_goldens` compares a frame of the
   defect against a photograph of the defect and matches.
3. **The layer test asserts water has a layer, not that it has an image.**
   `crates/mc-client/tests/launch_texture_layers.rs:142-145` is about layer
   assignment and is right about it.
4. **The gate checks that the set is *current*, never that it is *complete*.**
   `scripts/sdd-gate.ps1:411` runs `voxforge build` and stage 7 refuses a
   committed image. Both are about the set matching its manifest. Nothing
   anywhere compares the manifest's keys against the keys the block
   declarations name.

`voxforge build`'s own advisory scan runs in the **opposite direction** — it
reports a manifest key no block uses. The direction that mattered, a block key
no manifest bakes, is documented as deliberately never a refusal
(`docs/modding/voxel-models.md`, "why an uncovered key is never a refusal"), and
correctly so: it is a mod author's first block. What was missing is that the
*base game* is held to a stricter rule than a third-party mod, because the base
game's job is to prove the contract is complete (`content/CLAUDE.md`).

**It was found by a human playing the game** — the second time in this project
that the largest visible defect in a rendering spec was found by looking at the
picture rather than by 1 300 green tests.

### Sibling sweep

The shipped content root declares exactly eight distinct texture keys, across
four block files: `base:dirt` (`dirt.luau:13`), `base:stone` (`stone.luau:13`),
six facings in `grass.luau:27-34` naming `base:grass_top`, `base:dirt` and the
four `base:grass_side_*`, and `base:water` (`water.luau:48`). The manifest
covers seven of them.

**`base:water` is the only uncovered key, so the sweep finds no sibling.** There
is one content root under `content/`, and `content/base/hud/*.toml` declare
element names rather than texture keys. The uncovered keys in
`crates/mc-client/tests/fixtures/gate/` are deliberate fixtures for the stand-in
path and must stay uncovered.

### Prevention

**Per-defect, as a regression test in this spec: FR-1.1-S1 and FR-1.1-S2.** The
shipped root's declared keys and its built set's covered keys are compared as a
**single enumerated verdict**, not as an absence assertion — an
`assert!(uncovered.is_empty())` cannot tell an empty answer from a scan that can
no longer look, and a vanished content root would go green forever.

Two shapes are forbidden in that test, both because this project has been bitten
by them:

- **It must read the key list out of the observed output**, in order, and
  compare the whole list — never filter a hand-maintained list of eight names
  against what it observes. Two mirrors of a nine-name list were each held at
  six and neither reddened, because one filtered its needles by presence and the
  other skipped what it could not rank. A ninth key added by a later spec must
  make this test red, not slip past it.
- **It needs a positive control** — FR-1.1-S2 — over a fixture root that
  genuinely declares an uncovered key, so the day the scan stops being able to
  look, something says so.

**No gate amendment is proposed.** A project-wide "every content root's keys are
covered" check would be wrong at the gate: it is false by design for a mod, and
`scripts/sdd-gate.ps1` is amended only by a project-level decision with a
runtime budget (CLAUDE.md Key Principle 5). The rule that binds is about the
*base game* specifically, which is what a test in `mc-client`'s suite can say
and a gate stage over arbitrary roots cannot.

2. **This spec's own two figures that did not survive meeting the code, both
   recorded rather than silently corrected.** FR-4.1-S1's parenthetical states
   88 280 / 88 280 / 174 744 / 198 828 while naming exactly three colours; those
   three colours count **77 987 / 165 232 / 191 792 / 77 987**, which is what the
   Defect table's own three columns sum to. The larger figures are those plus the
   trilinear blends between mip levels (10 293 / 9 512 / 7 036). Both are correct
   measurements of different things. The test asserts the three named colours and
   says so; `test-map.md` carries the arithmetic.
3. **The Detection-gap section cites `docs/modding/voxel-models.md`, "why an
   uncovered key is never a refusal".** No section of that name exists. The
   material is under `### Two things a missing texture can mean`, which is what
   the repaired paragraph now links to.
4. **The launch notice never names the key that drew a stand-in**
   (`crates/mc-client/src/main.rs`). It prints one general sentence at every
   launch saying stand-ins exist and mean nothing is wrong, whether or not any
   key is uncovered — so it could not have surfaced this defect, and
   `docs/user/gameplay.md` now says so plainly rather than implying the terminal
   reports one. Naming uncovered keys at launch would be a real remedy and is
   **explicitly Out of Scope** here ("the launch notice that describes it.
   Unchanged"). Worth an issue.
5. **`docs/planning/block-render-methods.md:51` says water "is therefore
   completely invisible right now".** Stale since 2026-08-23, before this spec,
   and a planning document rather than as-built — left alone under the no-sweeps
   rule.
6. **`docs/INDEX.md`'s routing rows for the four documents this phase edited are
   unchanged**, and the SPEC-025 attribution is not on them. That is the complete
   phase's consolidation step, not a gap in the edits.
7. **`gen_water.py` runs and writes the model the repository tracks;
   `gen_stone.py` deliberately does not.** The divergence is stated in the new
   script's own docstring, with the reason the sibling's non-working state was
   left unrepaired still holding: neither is a way to regenerate art.
8. **A fourth falsified premise, ruled in scope by the approving authority on
   2026-08-26 as premise 2's own shape.**
   `crates/mc-client/tests/the_sea_the_camera_sees_is_the_water_layer.rs`'s
   header derived its dE 8 tolerance from the *stand-in's* texel spread and said
   in as many words that `base:water` is not one of the seven baked keys. Both
   ends of the bracket moved when the art landed — spread 3.71 to **3.16**,
   nearest wrong answer `base:stone` 62.40 to **25.34** — and 8 sits inside both,
   so the constant did not have to move. That is precisely premise 2: a tolerance
   whose *stated derivation* stops describing the set while the number itself
   stays adequate. A reader checking the derivation finds it false; a reader
   checking only the number finds nothing. Three was the count the spec could
   see, not a cap.

   **The irony belongs on the record, because it is the closed loop written out
   in prose inside its own test file.** That header is the document that first
   told this spec's author the water key was unbaked. It did not merely describe
   the defect — it **built a tolerance around it and asserted a guard on that
   tolerance**: `require_nothing_else_is_that_colour` checked that nothing else
   in the frame stood within dE 8 of the stand-in's mean, and passed, because the
   stand-in is magenta and magenta is far from everything. Prose and a passing
   guard, both accurate, correctly bracketed in both directions, and neither able
   to act — about a world the product should not have been in. It is the sharpest
   single artefact behind `docs/technical/testing.md`'s "An oracle can be right
   and useless".
9. **A fifth, repaired for the same reason.** `docs/technical/rendering.md` said
   MVP 1 ships procedurally generated placeholder textures. Every key the base
   game declares now draws baked art, and the generator is stated as what remains
   for a key nothing baked.
10. **`tools/voxforge/tests/build_writes_a_set.rs` was a consequence the spec did
    not predict.** Three readings hold the shipped manifest's entry count by hand
    — correct design, since a derived count would agree with a manifest that had
    quietly lost an entry — so all three went red at seven. Moved to eight, the
    two test names carrying the count renamed, and the constant's doc now says it
    is hand-maintained and why.
