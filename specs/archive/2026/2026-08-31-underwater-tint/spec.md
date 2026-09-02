---
id: SPEC-032
title: The medium the eye stands in reaches the image
status: implemented
work-type: feature
rigor: high
branch: feature/PRO-998-underwater-tint
issue: PRO-998
created: 2026-08-31
updated: 2026-09-02
approved: 2026-08-31
completed: 2026-09-02
author: spec-PRO-998
---

# Specification: The medium the eye stands in reaches the image

## Goal

A camera under the sea draws a frame identical to a dry one, because the only
thing water does to a picture today is drawn as *geometry* — and the volume the
eye is inside contributes none, its exit faces being back-facing along every ray
that leaves the eye. This spec makes **submersion a state the frame knows
about**: the block occupying the eye's cell declares a colour and how far you
can see through it, and everything the frame draws is carried toward that colour
with distance. The colour and the distance are **content**, so the player goes
under and sees water, and a mod author ships a gas, an acid pool or a nebula
with no engine change.

## User Stories

- As a **player**, I want the world to look like water when my head goes under
  the sea, so that being submerged is something I can see rather than something
  I have to infer from not falling.
- As a **player**, I want the sky to stop being sky while I am under, so the
  surface above me reads as a boundary rather than as a window.
- As a **mod author**, I want to declare the colour my own block is seen through
  and how far it lets me see, so I can ship a medium of my own without touching
  Rust.
- As a **mod author**, I want a mistyped or unusable tint refused in a sentence
  naming my file, my block, the field and the bound, so I can fix it mid-edit.
- As an **engine reader**, I want the tint law, where its distance is measured
  from, and what it deliberately does not reach written down, so the next medium
  is added against a stated contract.

## Functional Requirements

### FR-1 — A block declares the medium it is, seen from inside

Two fields, two claims, following the declaration's existing habit of keeping
separate claims separate: **what colour the medium carries**, and **how far it
lets you see**. The spelling below is the architect's to settle; the semantics,
the bounds and the refusals are fixed here.

- The colour is a hexadecimal string behind a `#`, case-insensitive, in either
  the six-digit `#RRGGBB` form `content/base/materials/*.toml` writes or the
  eight-digit `#RRGGBBAA` form `content/base/hud/*.toml` writes with its alpha
  at `FF`. It carries **no alpha**: how strongly the medium acts is the
  distance's job, and a second strength would be two fields answering one
  question. **Both forms are accepted because shipped content already speaks
  both dialects, each behind a reader that claims to be the only one.**
  `crates/mc-core/src/hud/element.rs:148` takes eight digits and refuses six —
  "must be `#RRGGBBAA` with 8 hex digits and no shorthand"; `tools/voxforge/src/material.rs:210`
  takes six and refuses eight — "the colour `{text}` is not written `#rrggbb`,
  which is the only form a colour takes". Two shipped refusals, each asserting
  exclusivity, each false as written, over `content/base/hud/*.toml` in
  uppercase and `content/base/materials/*.toml` in lowercase. A third rule would
  make three; accepting both makes this the one field in the tree an author can
  write either dialect into. An alpha other than `FF` is refused rather than
  ignored, so a strength stated in the wrong place is named instead of silently
  dropped.
- The distance is in **blocks**, finite and **strictly greater than zero**, and
  it is the distance at which the medium hides what lies beyond it completely.
  The floor is exclusive, unlike `opacity`'s inclusive `0.0`, because a medium
  that hides everything at no distance hides the inside of the eye.
- **Both or neither.** A colour with no distance and a distance with no colour
  are each refused, naming the missing one. The alternative is an engine
  constant standing in for content, which Invariant 1 forbids.
- A block declaring neither tints nothing, whatever else it declares. There is
  no default tint and no default distance anywhere in Rust.

- **FR-1.1**: A declaration states a medium's colour and its distance, and the registry keeps both.
  - FR-1.1-S1: WHEN a block declaration states a tint distance of `12.0` and a tint of `#3A6EA5`, and separately of `#3a6ea5`, and separately of `#3A6EA5FF`, THE SYSTEM SHALL register that block with the same pair of values in each of the three cases, readable from the registry.
  - FR-1.1-S2: WHEN a block declaration states neither field THE SYSTEM SHALL register that block as carrying no tint, and SHALL NOT give it a colour or a distance of any value.
  - FR-1.1-S3: WHEN a content root declares two blocks, one tinting `#3A6EA5` at `12.0` and one tinting `#8A4400` at `3.0`, THE SYSTEM SHALL register each with its own pair and neither with the other's.
  - FR-1.1-S4: WHEN a declaration states a tint together with an opacity of `1.0` THE SYSTEM SHALL register both, refusing neither, so that what a block looks like from outside and what it does from inside stay separate claims.
- **FR-1.2**: A stated colour the engine cannot keep is refused, and the refusal names what to fix.
  - FR-1.2-S1: IF a declaration states the tint as the number `5` THEN THE SYSTEM SHALL refuse the whole content root and name the file, the block, the field, and that a colour string was expected.
  - FR-1.2-S2: IF a declaration states the tint as `"#GG0000"`, and separately as `"3A6EA5"` with no leading `#`, and separately as `"#3A6EA"` at seven digits, THEN THE SYSTEM SHALL refuse the whole content root in each case, naming the file, the block, the field and both accepted forms.
  - FR-1.2-S3: IF a declaration states the tint as `"#3A6EA580"` THEN THE SYSTEM SHALL refuse the whole content root naming that a tint states no alpha and that the distance carries its strength, in a cause distinct from the one raised against `"#GG0000"`.
- **FR-1.3**: A stated distance the engine cannot keep is refused, and the refusal names the bound it broke.
  - FR-1.3-S1: IF a declaration states a tint distance of `0.0`, and separately of `-1.0`, THEN THE SYSTEM SHALL refuse the whole content root in each case, naming the file, the block, the field and that the distance must be greater than zero.
  - FR-1.3-S2: IF a declaration states a tint distance of `math.huge`, and separately of `0/0`, THEN THE SYSTEM SHALL refuse the whole content root in each case naming finiteness, and SHALL NOT report either as breaking the floor — both values being named because one passes a `> 0.0` comparison and the other fails it, so a wrong ordering of the two checks is caught by one of the pair.
  - FR-1.3-S3: IF a declaration states the tint distance as the string `"far"` THEN THE SYSTEM SHALL refuse the whole content root and name the file, the block, the field, and that a number was expected.
- **FR-1.4**: The two fields are stated together or not at all.
  - FR-1.4-S1: IF a declaration states a tint of `#3A6EA5` and no tint distance, and separately a tint distance of `12.0` and no tint, THEN THE SYSTEM SHALL refuse the whole content root in each case, naming the missing field as the thing to add — the distance in the first and the colour in the second, in two distinct sentences.
- **FR-1.5**: The fields join the declaration vocabulary a mod author is shown.
  - FR-1.5-S1: IF a declaration states a field name that is not recognised THEN THE SYSTEM SHALL quote back the whole recognised-field list, fifteen names in the loader's own order, with both new fields in the documented position — read out of the refusal and compared entire, so that a missing name, an extra name and a reordering are three distinct failures.

### FR-2 — What the eye stands in reaches the image

Four constraints bind every scenario in this requirement.

1. **Absolute, never comparative.** Each expected colour is derived by
   arithmetic from the declared colour, the declared distance, the surface's own
   colour and the surface's measured distance from the eye. **The two scenarios
   claiming identity are not an exception to this and may not become one**: two
   frames can agree while both are wrong, which is a failure this project has
   shipped, so FR-2.3-S1 and FR-2.4-S1 each carry an absolute per-sample
   prediction *in the same assertion* as the frame comparison. The superseded
   test made both claims for exactly this reason and keeping only its
   comparative half would be a strict weakening of a shipped instrument.
2. **Computed in linear light and re-encoded.** The target is `Rgba8UnormSrgb`;
   an expectation computed on sRGB bytes does not match the frame.
3. **Distance is radial from the eye**, not depth along the view direction. The
   two disagree away from the frame's centre, and FR-2.1-S4 is what tells them
   apart.
4. **The fixture's declared colour must be one no engine constant could
   supply** — not the clear colour `#87CEEB`, not black, not white, not a colour
   any layer in the fixture shows — and must stand far enough from every colour
   the scene otherwise holds that an absent tint and a wrong tint are each
   distinguishable. The fixture states both measured distances. No assertion can
   enforce this; it is held by whoever builds the fixture and whoever reads it.

**The law.** A surface at distance `d` from the eye, in a medium declaring
colour `T` at distance `D`, is drawn as its own colour carried toward `T` by
`min(1, d / D)` in linear light: untinted at the eye, wholly `T` at `D` and
beyond. A linear ramp rather than an exponential falloff, because "you can see
twelve blocks through this" is a statement an author can make and a reading can
check, and an exponential never actually hides anything.

**The rule is over the eye's own cell and nothing else.** Whether that cell's
block passes light, occludes, is solid or is swimmable does not enter into it:
a block declaring a tint tints from inside, and one declaring none does not.
The state FR-1.1-S4 admits — a tint declared beside `opacity = 1.0` — therefore
draws the whole frame at the declared colour, that block's own faces being
back-facing along every ray leaving the eye. No shipped block reaches that
state and no scenario spends a slot on it; it is written here so an implementer
does not have to guess.

- **FR-2.1**: A surface is carried toward the medium's colour by how far away it is.
  - FR-2.1-S1: WHEN the eye stands inside a cell whose block declares a tint at a distance of `12.0`, and an opaque surface stands `6.0` blocks from the eye, THE SYSTEM SHALL draw that surface's pixels as the even mix, in linear light, of that surface's own colour and the declared colour.
  - FR-2.1-S2: WHEN that same eye looks at an opaque surface standing `12.0` blocks away or further THE SYSTEM SHALL draw those pixels at the declared colour, with none of the surface's own colour remaining.
  - FR-2.1-S3: WHEN that same eye looks at an opaque surface standing `1.2` blocks away THE SYSTEM SHALL draw those pixels one tenth of the way from that surface's own colour toward the declared colour, a value the fixture shows to be distinguishable from both the untinted colour and the even mix.
  - FR-2.1-S4: WHEN that same eye looks squarely at a flat opaque wall standing `6.0` blocks along the view direction THE SYSTEM SHALL draw the wall's centre pixel at the mix for `6.0` blocks and a pixel a quarter of the frame's width from the centre at the mix for that pixel's own radial distance — which the fixture derives from the declared camera as `6.74` blocks — the two predictions standing further apart than the tolerance the fixture states.
  - FR-2.1-S5: WHEN the eye stands inside a cell declaring `#3A6EA5` at `12.0`, and then inside one declaring `#8A4400` at `12.0`, THE SYSTEM SHALL draw the same surface at the mix predicted for each declared colour, which are two distinct colours.
  - FR-2.1-S6: WHEN that same eye looks along a ray crossing one cell of a block declared at opacity `0.5` standing `3.0` blocks away and then meeting an opaque surface `9.0` blocks away THE SYSTEM SHALL draw that pixel as the two layers each carried toward the declared colour by its own distance and then blended, which is a distinct colour from blending the two untinted layers and carrying the result by `3.0`.
- **FR-2.2**: A pixel the frame draws no terrain at is the medium, not the sky.
  - FR-2.2-S1: WHEN the eye stands inside a cell whose block declares a tint, and a pixel's ray leaves the world without meeting a drawn face, THE SYSTEM SHALL draw that pixel at the declared colour rather than at the colour a dry camera's sky is given, over a frame holding at least one hundred such pixels.
- **FR-2.3**: A medium that declares no tint tints nothing.
  - FR-2.3-S1: WHEN the eye stands inside a cell whose block passes light and declares no tint THE SYSTEM SHALL draw every pixel of the frame exactly as a camera at the same pose draws it in a world whose blocks declare no tint at all, and SHALL in the same assertion draw every declared sample at the colour predicted for it from the world's own voxels and declarations alone, sharing no code with the draw path.
- **FR-2.4**: An eye outside the medium is untouched by it.
  - FR-2.4-S1: WHEN the eye stands in a cell holding nothing, in a world whose sea declares a tint, THE SYSTEM SHALL draw every pixel of the frame exactly as the same pose draws it in a world whose sea declares no tint, and SHALL in the same assertion draw every declared sample at the colour predicted for it from the world's own voxels and declarations alone.
  - FR-2.4-S2: WHEN the eye stands at `y = 34.98`, inside a cell of a block declaring `#3A6EA5` at `12.0` whose top face is at `y = 35.0`, and then at `y = 35.02` in the empty cell above it, THE SYSTEM SHALL draw the first frame at the colours FR-2.1-S1's law predicts and the second at the colours FR-2.4-S1 predicts, so that the medium changes across that cell's own face and not at a distance from it.
- **FR-2.5**: The tint reaches the world and stops at the overlay.
  - FR-2.5-S1: WHEN a frame is drawn with the HUD composited and the eye inside a tinting cell THE SYSTEM SHALL draw every pixel of every HUD element at its declared colour — the crosshair's fill at `#FFFFFF` and its outline at `#000000` — over a frame in which at least one hundred HUD pixels were examined.
- **FR-2.6**: A player of the shipped world can get under the surface.
  - FR-2.6-S1: WHEN a player enters the shipped world, swims into the deepest column of the sea and comes to rest on the bed THE SYSTEM SHALL report the cell holding that player's eye as holding the sea's own block, naming the column and how far the eye stands below the surface.
  - FR-2.6-S2: IF no column of the shipped sea puts a resting player's eye inside a cell of it THEN the reading SHALL report the deepest column and how far the eye stands above the surface rather than passing.

### FR-3 — The committed golden set, and why it should not move

No judged frame's camera is submerged: `the_camera_of_every_judged_frame_stands_in_open_air`
asserts it at all three declared ticks, and the HUD capture is tick 0. So a
correct implementation leaves every committed capture unmoved, and **that
reading is a deliverable rather than an assumption** — it is the only thing
standing between this spec and a tint leaking into a dry frame.

- **FR-3.1**: A dry judged frame is unmoved by a declared sea tint.
  - FR-3.1-S1: WHEN each capture committed before this spec's first implementation commit is compared against a frame rendered from the tree carrying the sea's declared tint THE SYSTEM SHALL report every declared capture as matching, byte for byte.
  - FR-3.1-S2: IF the camera of any judged frame stands in a cell holding a drawn block THEN the reading SHALL report which capture and which tick rather than passing.

### FR-4 — A declared tint reloads live, in both directions

- **FR-4.1**: Changing a declared tint on a running server takes effect without a restart.
  - FR-4.1-S1: WHEN a block's declaration gains a tint of `#3A6EA5` at `12.0` and the content root is reloaded THE SYSTEM SHALL thereafter draw an eye inside that block's cells at the mix FR-2.1-S1 states, with no restart.
  - FR-4.1-S2: WHEN a reload removes both fields from a block that declared them THE SYSTEM SHALL thereafter draw an eye inside that block's cells untinted.
  - FR-4.1-S3: IF a reload states a tint distance of `0.0` THEN THE SYSTEM SHALL refuse that reload, keep the previously loaded tint in force, and name the file, the block, the field and the bound.
  - FR-4.1-S4: WHEN a reload changes only a block's tint distance, from `12.0` to `6.0`, leaving its colour unchanged, THE SYSTEM SHALL thereafter draw a surface `6.0` blocks from an eye inside that block wholly at the declared colour, with no restart.

### FR-5 — A world saved before the fields existed still opens

- **FR-5.1**: The fields are appearance, and a save that predates them loads.
  - FR-5.1-S1: WHEN a world saved before the tint fields existed is opened THE SYSTEM SHALL load every block it holds, each carrying no tint, and refuse nothing.
  - FR-5.1-S2: WHEN a world is saved after the fields exist THE SYSTEM SHALL have moved the appearance revision byte and left the behaviour revision byte at the value it held.
  - FR-5.1-S3: IF the appearance revision byte is left at the value it holds today while the two fields join the list it is folded over THEN the reading SHALL report the byte sequence as disagreeing rather than passing — a reading that builds the expected bytes by hand being the only witness that can see a leading byte which moved in neither fold.

### FR-6 — Invariant 1 holds, and all three audiences reach it without reading Rust

- **FR-6.1**: No block name reaches the path that decides what the eye is in, or the path that draws it.
  - FR-6.1-S1: WHEN the sources deciding the eye's medium and drawing the frame are scanned for a comparison against a block name THE SYSTEM SHALL return the verdict that no such comparison exists.
  - FR-6.1-S2: WHEN that same scan is run over a fixture holding one name comparison THE SYSTEM SHALL report that comparison rather than returning the clean verdict.
- **FR-6.2**: The mod author's page carries both fields with their bounds, their refusals and a worked example.
  - FR-6.2-S1: WHEN the modding pages are read THE SYSTEM SHALL find both fields named in each page's field table beside the value their absence means, and in each quoted recognised-field refusal, in the order the loader holds.
  - FR-6.2-S2: IF a modding page's field list is short by one name THEN the reading SHALL report it as a missing name rather than passing.
- **FR-6.3**: The player's page says what is different.
  - FR-6.3-S1: WHEN `docs/user/gameplay.md` is read THE SYSTEM SHALL name that going under the sea changes what the player sees, and name where the water is deep enough for that to happen.
  - FR-6.3-S2: IF the gameplay page names neither THEN the reading SHALL report which of the two is missing rather than passing.
- **FR-6.4**: The engine reader's page states the law and its limits.
  - FR-6.4-S1: WHEN `docs/technical/rendering.md` is read THE SYSTEM SHALL name the tint law, the point the distance is measured from, what a pixel drawing no terrain is given, that the HUD is not tinted, and whether the scene revision moved.
  - FR-6.4-S2: IF that page names four of those five and omits the fifth THEN the reading SHALL report which one is missing rather than reporting that the page names them all.

## Architecture Delta

This feature adds **two published block-declaration fields** and a **new
per-frame input to the draw path**, and it sets the shape every future medium
inherits. The architect chooses; this section enumerates the space and the
couplings measured while writing it.

### A. Where the eye's medium comes from

`mc-render` **never names `mc-sim` in any dependency of any kind**
(`architecture.md` §"The simulation/renderer seam"), and only tick, camera pose
and player state cross `SimSnapshot`. Geometry reaches the renderer via the
client. The tint has to arrive by one of:

| Candidate | Seam cost | Note |
|---|---|---|
| A1 — a field on `SimSnapshot`, carried into `TerrainSnapshot` by the client | one field on each of two existing types | the simulation already owns the world and the registry; `VoxelMedium` is the established word for what a volume is to a body inside it |
| A2 — the client queries the world for the eye's cell each frame | no new snapshot field; a second reader of the world beside the snapshot | the client would own a rule the simulation already owns |
| A3 — the renderer resolves it from the scene it holds | none across the seam | the renderer holds geometry, not a registry; it would have to learn what a block is |

**Measured:** `crates/mc-sim/src/player/mod.rs:128` `VoxelMedium` already carries
`swimmable`, `resistance` and `swim_ascent`, joined across the cells a *body*
occupies. A tint is a property of the **one cell the eye is in**, not a join over
a volume, so reusing the type is a decision rather than an obvious win.

### B. How the tint reaches a fragment

| Candidate | Cost | Reaches the sky? | Note |
|---|---|---|---|
| B1 — mixed in the terrain fragment stage, with the clear colour set to the tint | one uniform field, a few shader lines, a clear-colour choice | yes, via the clear | radial distance is available in the fragment stage; the pass already takes a clear colour per configuration |
| B2 — a full-screen pass after terrain, sampling depth | a new pipeline, a new bind group, depth as a sampled texture (a usage-flag change on an attachment the renderer owns) | yes | the only candidate that tints without touching terrain's shader |
| B3 — a constant full-screen wash, no distance | least | uniformly | refused by FR-2's law: a constant wash is a colour grade, not a medium |

**Constraint the choice inherits:** the HUD is composited over the terrain frame
(`rendering.md` records that anything moving terrain moves the HUD capture), so
whichever candidate is chosen must act **before** the HUD pass. FR-2.5-S1 is
what catches getting that backwards.

**Storage-buffer budget:** `downlevel_defaults()` allows four storage buffers per
stage and the cull shader binds exactly four; a fifth in either stage is a
portability break enforced at build time. A tint carried as a **uniform** costs
none of that budget, which is what B1 assumes.

### C. Whether `SCENE_REVISION` moves `r5` → `r6`

**The expectation is that it does not, and the expectation is not the
deliverable — FR-3.1-S1 is.** The revision is bound to the *scene contract*:
pose, world, camera path, tick list, merge predicate, vertex format. A tint is
per-frame and per-eye; **nothing here needs a per-face field**, so the packed
vertex is untouched, the merge predicate is untouched, and no judged camera is
submerged. If the chosen mechanism moves any of those, the bump is owed and
FR-3.1-S1 will say so.

**Binding either way:** a bump renames the set by **deletion and a fresh mint,
never `git mv`**. Measured on the 2026-08-27 re-shoot, a `git mv` passed the
comparison, `golden_mismatch` and `golden_inventory` while two directories still
held sidecars naming `r3`.

### D. Which save revision byte moves

**Assumed appearance, not behaviour**, on the same precedent `opacity` used: what
colour a block is seen through from inside does not change what it is to stand
on, and routing a rendering field through the behaviour byte tells every player
that every block they built with behaves differently. Both bytes stand at 4
today. A new field is **appended, never inserted** — the encoding is positional.

### E. The colour's representation across the seam

The declaration states sRGB bytes; the hardware blends in linear light and the
clear colour is specified in linear space already (`rendering.md`: "this is the
one place where a unit test of the conversion and a test comparing the two
configurations to each other can both pass while every shipped frame is wrong").
Where the decode happens, and which side of the seam carries which
representation, is the architect's — but a tint that crosses as sRGB and is
mixed as if linear is precisely the defect that section records.

## Technical Considerations

- **The superseded test is replaced, not deleted first.**
  `crates/mc-client/tests/a_camera_inside_the_sea_tints_nothing.rs` asserts the
  behaviour this spec removes, and its *name* becomes a false statement the
  moment the sea declares a tint. Its successor is FR-2.1's reading, and its
  negative half survives as FR-2.3-S1 — a camera inside a block that declares no
  tint still draws a dry frame, which is the part of the old test that stays
  true. The old file's header carries the pose filter, the ranking and the
  measured sea extent that the successor inherits; that reasoning moves with it
  rather than being re-derived.
- **Two colour vocabularies already exist, and this spec adds no third.** Both
  shipped readers claim exclusivity and both claims are false: the HUD's takes
  eight digits and refuses six, voxforge's takes six and refuses eight, and each
  says in its refusal that its own form is the only one. The tint accepts both
  and refuses only an alpha it could not honour. **Reconciling the two is out of
  scope** — it is a change to two shipped published surfaces with their own
  guards and refusal texts, and absorbing it here would put a HUD change inside
  a water spec. Filed as **PRO-999**.
- **The chosen law owes an ADR, and the architect phase owes it.**
  Linear-ramp-to-a-stated-distance is a published content surface later specs
  must not break, and this folder is archived at completion — a rationale living
  only here is one a future reader reconstructs from code. The architect records
  it in `docs/technical/decisions.md` with the exponential / Beer-Lambert
  alternative **named and rejected, with the reason**. Not written in this
  phase.
- **The successor test's own header is where the identity-claim reasoning
  lands.** A frame-to-frame identity claim is satisfied by a constant wash
  applied regardless of the eye's medium; the superseded test carried an
  absolute per-sample claim beside its pixel comparison for exactly that reason,
  and replacing it with the comparison alone would have read as a clean
  replacement while being a strict weakening. That belongs where the next reader
  meets it, which is the successor's header rather than this folder.
- **The declared pose is the fixture's, and it has to be.** The player wades and
  the eye does not go under at any judged tick, so every reading in FR-2 declares
  its own pose over the shipped world — which `terrain_probes.rs`,
  `support/all_opaque.rs` and the superseded test all already do, for the same
  reason: the world, the art, the mesher and the draw path stay the shipped ones
  and only the pose is the fixture's.
- **A player can reach this, by 0.38 blocks, and that margin is the single
  load-bearing premise of the spec.** The eye stands 1.62 blocks over the feet
  and a swimmer sinks to the bed, so a player in the 47 two-deep columns of the
  sea is submerged by `2.0 − 1.62 = 0.38` blocks; the 131 one-deep cells leave
  it dry. This is arithmetic over two recorded numbers, **not an observation** —
  FR-2.6 is what turns it into one.
  **If it turns out false, the phase that finds out escalates to the owner and
  stops.** It is not to be worked around by deepening the sea, widening the
  water, or lowering the eye: each of those is a content or physics change
  smuggled into a render spec, and all three are refused in advance.
  **Recorded as debt either way:** a margin of 0.38 blocks means any future
  change to `EYE_HEIGHT` or to the sea's depth kills submersion silently, and
  FR-2.6-S2 is the only thing that would say so.
- **`min(1, d / D)` is stated in the spec because a scenario needs an exact
  outcome.** The architect may argue the law; changing it changes FR-2.1's
  arithmetic and is an amendment to this spec rather than an implementation
  choice.
- **What a golden cannot do here.** Every capture is dry, so the golden set is
  evidence that the tint *stays out* of a dry frame and no evidence whatever
  that it is right when present. The instruments that can say that are the
  derived per-pixel readings of FR-2, which share no code with the draw path.

## Existing Code to Leverage

| What | Location | Reuse |
|------|----------|-------|
| The submerged pose, its filter and its ranking | `crates/mc-client/tests/a_camera_inside_the_sea_tints_nothing.rs` | the successor inherits the declared pose and the reasoning that chose it |
| The medium a volume is to a body | `crates/mc-sim/src/player/mod.rs:128`, `crates/mc-sim/src/replay/medium.rs` | the vocabulary, and the per-voxel medium table |
| The loader's numeric vocabulary and its bounds | `crates/mc-world/src/content/luau_declaration/number.rs`, `.../opacity.rs` | four refusal branches; an exclusive floor is a new one |
| The two shipped colour readers | `crates/mc-core/src/hud/element.rs:148` (eight digits), `tools/voxforge/src/material.rs:210` (six digits) | the shape of a colour refusal, and the two dialects to accept — **neither is reusable as a parser**: each takes one form and refuses the other, so a reader taking both is new work |
| The optional-field defaulting pattern | `crates/mc-world/src/content/luau_declaration/mod.rs` | `RECOGNISED_FIELDS`, and how a field's absence is given meaning |
| Doc/loader agreement guards | `crates/mc-client/tests/documented_declaration_fields.rs` | reads lists whole and in order; a field addition reddens it |
| The per-pixel derived oracle | `crates/mc-client/tests/support/oracle.rs`, `support/composite.rs` | predicts a pixel from the world's own voxels, sharing no code with the draw path |
| The clear colour in linear space | `crates/mc-render/src/pass.rs:83`, `crates/mc-render/src/color.rs:42` | the one place a per-frame colour already reaches the device |

## Out of Scope

Binding. Recorded as deferred observations, never built.

- **A screen-space overlay texture** — no bubbles, no droplets, no water
  distortion image over the frame.
- **Refraction and surface distortion** — the medium does not bend what is seen
  through it, and the picture does not wobble.
- **Caustics** — no light patterning on the lakebed.
- **A swim animation, a breath meter, drowning, or any sound.**
- **Tinting anything but the terrain frame** — the HUD is expressly excluded by
  FR-2.5-S1, and the debug overlay with it.
- **A view of the surface from below** — no reflection, no total-internal
  reflection, no separate treatment of the boundary plane the eye is under.
- **A tint that varies within one medium** — no depth-below-surface gradient, no
  biome colour, no time of day. One block, one colour, one distance.
- **A per-medium clear colour declared separately from the tint** — a pixel
  drawing no terrain is the tint colour by FR-2.2-S1, and a second declaration
  for the same thing is two answers to one question.
- **The sky colour becoming content** — `CLEAR_COLOR_SRGB` stays a Rust constant
  for a dry camera. That it is one is worth an issue; it is not this spec's.
- **Sorting between translucent blocks of different kinds** — unchanged from
  SPEC-031, and untouched here.
- **Relaxing the HUD's eight-digit colour rule or voxforge's six-digit one**, or
  unifying the two behind a single reader. Filed as **PRO-999** with both line
  numbers and both refusal texts; not built here.
- **`merges_with_self` and drawing a volume's interior faces** — the fix this
  spec deliberately does not make, for the reasons in `requirements.md`.

## Dependencies

- **PRO-993 / SPEC-031** (merged) — the sea passes light and the terrain draws a
  blended pass. Without it there is no translucent volume for an eye to stand
  inside and nothing to be under.
- **PRO-904 / SPEC-025** (merged) — the `drawn` / `occludes` / `targetable`
  split. A medium can only be declared once a block can state more than its
  collision.
- **PRO-952** (Backlog) — takes the surface inset, the wave, and the
  `render = "<method>"` framing. This spec must not foreclose it: a tint pair
  and a later render method must coexist on one declaration.

## Assumptions

- The sea is two cells deep in 47 columns, so a player standing on the bed there
  has its eye under the surface. Derived in `requirements.md` from two recorded
  numbers; to be confirmed by a phase that can run the world.
- A per-frame uniform is available to the draw path without touching the
  four-storage-buffer budget.
- MVP 2 quality is satisfied by one colour and one distance per medium. Nothing
  here forecloses a richer medium later; it forecloses nobody's ability to see
  water now.

## Open Questions

None. The three the ledger opened are answered or handed on: the golden question
is FR-3.1's reading rather than an argument, the submerged capture is refused
with its reason, and where the eye's medium comes from is item A of the
Architecture Delta. The fourth, raised by the scenario audit — that the engine's
only colour parser refuses the form this field accepts — is answered in FR-1's
preamble by accepting both forms, with the reconciliation of the HUD's own rule
stated as out of scope rather than left silent.

## Notes

Out-of-scope observations recorded during this phase, never built here:

- `CLEAR_COLOR_SRGB` is a Rust constant naming the sky's colour
  (`crates/mc-render/src/color.rs:42`). Invariant 1 is about blocks, items,
  recipes, NPCs and quests, so a sky colour is not a violation of it — but it is
  the same shape of thing this spec is moving into content, and it is worth an
  issue of its own.
- **Two shipped colour readers, each whose refusal claims it is the only one.**
  `crates/mc-core/src/hud/element.rs:148` takes eight hex digits and refuses six
  "with no shorthand"; `tools/voxforge/src/material.rs:210` takes six and says
  `#rrggbb` "is the only form a colour takes". Case is unpinned in both, and
  shipped content writes uppercase in one directory and lowercase in the other.
  Filed as **PRO-999**; refused here as scope.
- `a_camera_inside_the_sea_tints_nothing.rs` records **19 767** admitted
  submerged candidates over the shipped world and that **not one** has a sample
  crossing a *further* run of the sea, because there is only one body of water.
  So SPEC-031's Decision 3 — "a further run along the ray still draws its entry
  face" — still has no witness at any submerged pose, and this spec does not give
  it one.
