---
id: SPEC-021
title: One bit answers four questions — solid, drawn, occludes and targetable split, plus swimmable and density
status: active
rigor: high
rigor-reason: >
  This spec re-decides two things that already have written answers in the tree
  and no task-list visibility — which of a save's two folds each new property
  joins, and how the ray-marched golden oracle stays independent of the mesher
  once water is drawn.
branch: feature/PRO-904-solid-split-properties
issue: PRO-904
created: 2026-08-22
updated: 2026-08-22
author: Sebastian Grunow
---

# Specification: One bit answers four questions

## Goal

`BlockDefinition::is_solid` is one boolean answering four unrelated questions —
is this drawn, does it stop me, can I aim at it, is it sensible to hold — so
water, the one shipped block that declares itself non-solid, is **completely
invisible**: `visible_face` (`crates/mc-world/src/mesh/sweep.rs:275`) emits no
face at all unless the voxel is solid. Split the bit into separately declared
properties and add the two a liquid needs, so that a block states what it *is*
rather than having three further facts derived from one, and a player can see
water and swim in it.

## Stakeholder capability delivered

Named, per Key Principle 7:

- **Player** — the sea is visible and swimmable. You can walk to the coast, see
  the water surface, wade in, and swim up out of it. Today the sea is a hole in
  the world that you walk through and cannot see.
- **Mod author** — you can declare a block that is visible without being solid,
  solid without being visible, aimable without being solid, or swimmable with a
  declared density, and each of those is a field you write in a `*.luau` file
  with a refusal that names it when you get it wrong.

## User Stories

- As a **player**, I want to see and swim in the sea, so that water is part of
  the world rather than an invisible hole in it.
- As a **mod author**, I want to declare separately whether my block is drawn,
  occludes, collides, can be aimed at and can be swum in, so that a decorative
  block is not forced to be an obstacle and an invisible barrier is expressible.
- As a **mod author**, I want a numeric field's refusal to name the field, the
  bound and what I wrote, so that a mistyped density is one edit to fix.
- As a **player**, I want a world I saved before this change to open and tell me
  which blocks now behave differently, so that a content update does not cost me
  my world.

## Functional Requirements

Scenario rules: `standards/global/scenario-guidelines.md`. Each scenario gets at
least one test, mapped in this folder's `test-map.md` — that mapping is a floor,
not a ceiling.

**Scenario count: 78.** The one command that measures it, so that no later stage
counts differently — the anchor is what keeps prose references to a scenario out
of the total:

```sh
grep -cE "^  - FR-[0-9]+\.[0-9]+-S[0-9]+: " spec.md
```

Per group: FR-1 17, FR-2 18, FR-3 9, FR-4 11, FR-5 3, FR-6 8, FR-7 5, FR-8 6,
FR-9 1. `grep -oE "^  - FR-[0-9]+\.[0-9]+-S[0-9]+" spec.md | sort | uniq -d`
must print nothing.

**This is a large scenario set and the size is a stated risk, not an oversight.**
`product/roadmap.md:186-193` records that the 82-scenario terrain-render spec
found eighteen instances of something passing for the wrong reason, and that "a
spec is exactly where that failure hides, because nobody holds the whole of it at
once". 78 sits next to that number. A recommendation to split this spec in two is
recorded under Open Questions; until that is answered, the whole of PRO-904 is
specified here.

### FR-1 — A block declares five new properties

- **FR-1.1**: A declaration may state `drawn`, `occludes`, `targetable` and
  `swimmable` as booleans. Each is optional and each defaults to the value the
  declaration states for `solid`, so no existing declaration becomes invalid.
  `solid` remains required and now means collision and nothing else.
  - FR-1.1-S1: WHEN a declaration states `solid = true` and none of the four new booleans THE SYSTEM SHALL read `drawn`, `occludes`, `targetable` and `swimmable` as true
  - FR-1.1-S2: WHEN a declaration states `solid = false` and none of the four new booleans THE SYSTEM SHALL read `drawn`, `occludes`, `targetable` and `swimmable` as false
  - FR-1.1-S3: WHEN a declaration states `solid = false` and `drawn = true` THE SYSTEM SHALL read `drawn` as true and `occludes`, `targetable` and `swimmable` as false
  - FR-1.1-S4: IF a declaration states `drawn = 1` THEN THE SYSTEM SHALL refuse the content naming the file, the block, the field `drawn`, and that it must be `true or false` but is `a number`

- **FR-1.2**: A declaration may state `density` as a number in kilograms per
  cubic metre. It is optional, absent means `0.0`, and a declared value must be
  finite and between `0.0` and `100000.0` inclusive. Both Luau's whole-number and
  fractional forms are accepted.
  - FR-1.2-S1: WHEN a declaration states `density = 1000.0` THE SYSTEM SHALL read the density as `1000.0`
  - FR-1.2-S2: WHEN a declaration states `density = 1` THE SYSTEM SHALL read the density as `1.0`
  - FR-1.2-S3: WHEN a declaration states no `density` THE SYSTEM SHALL read the density as `0.0`
  - FR-1.2-S4: WHEN a declaration states `density = 0.0` THE SYSTEM SHALL accept it
  - FR-1.2-S5: WHEN a declaration states `density = 100000.0` THE SYSTEM SHALL accept it
  - FR-1.2-S6: IF a declaration states `density = -0.1` THEN THE SYSTEM SHALL refuse the content naming the field `density`, the bound `0.0` to `100000.0`, and the value written
  - FR-1.2-S7: IF a declaration states `density = 100000.1` THEN THE SYSTEM SHALL refuse the content naming the field `density`, the bound `0.0` to `100000.0`, and the value written
  - FR-1.2-S8: IF a declaration states `density = 0/0` THEN THE SYSTEM SHALL refuse the content naming the field `density` and that it must be a finite number
  - FR-1.2-S9: IF a declaration states `density = "heavy"` THEN THE SYSTEM SHALL refuse the content naming the field `density` and that it must be a number but is `a string`

- **FR-1.3**: The set of fields a declaration may state is quoted back verbatim
  when an unrecognised one is written, and the modding guide prints the same
  refusal the engine produces.
  - FR-1.3-S1: IF a declaration states a field named `drawnn` THEN THE SYSTEM SHALL refuse it and quote exactly the eleven recognised field names, in declaration order — `name`, `texture`, `solid`, `replaceable`, `breakable`, `breaks_into`, `drawn`, `occludes`, `targetable`, `swimmable`, `density`
  - FR-1.3-S2: THE SYSTEM SHALL produce, for that refusal, exactly the message the modding guide prints for it

- **FR-1.4**: The shipped content declares water's new properties, and declares
  nothing new for the other three blocks. **Without this, every scenario in FR-2
  through FR-5 can be satisfied by synthetic fixtures while the shipped game is
  unchanged** — the shipped declaration is the only thing that makes the player
  capability real.
  - FR-1.4-S1: WHEN the shipped content root is read THE SYSTEM SHALL read `base:water` as not solid, drawn, non-occluding, targetable, swimmable, unbreakable, replaceable, and of density `1000.0`
  - FR-1.4-S2: WHEN the shipped content root is read THE SYSTEM SHALL read `base:dirt`, `base:grass` and `base:stone` as solid, drawn, occluding, targetable, not swimmable, and of density `0.0`

### FR-2 — What is drawn is declared, not derived from collision

- **FR-2.1**: The mesher emits a face for a voxel because its block is declared
  `drawn`, never because it is declared `solid`.
  - FR-2.1-S1: WHEN a section holds a block declaring `solid = false, drawn = true` with empty space above it THE SYSTEM SHALL emit an upward face for that voxel
  - FR-2.1-S2: IF a section holds a block declaring `solid = true, drawn = false` THEN THE SYSTEM SHALL emit no face for that voxel in any direction

- **FR-2.2**: A face is culled because the neighbouring block is declared
  `occludes`, never because it is declared `solid` — and that holds for a
  neighbour in the same section, a neighbour across a section boundary, and a
  neighbouring section that was never supplied.
  - FR-2.2-S1: WHEN a drawn voxel's neighbour in the same section holds a block declaring `occludes = false, solid = true` THE SYSTEM SHALL emit the face toward that neighbour
  - FR-2.2-S2: WHEN a drawn voxel's neighbour in the same section holds a block declaring `occludes = true, solid = false` THE SYSTEM SHALL cull the face toward that neighbour
  - FR-2.2-S3: WHEN a drawn voxel sits against a section boundary and the neighbouring section holds, across it, a block declaring `occludes = false, solid = true` THE SYSTEM SHALL emit the face across that boundary
  - FR-2.2-S4: IF a drawn voxel sits against a section boundary for which no neighbouring section was supplied THEN THE SYSTEM SHALL emit the face across that boundary

- **FR-2.3**: Two adjacent cells holding the same drawn, non-occluding block show
  no face on the boundary between them; two adjacent cells holding *different*
  drawn, non-occluding blocks each show theirs. Every scenario here states
  `solid`, because leaving it unstated is what would let an implementation that
  ignored `occludes` altogether satisfy one half or the other.
  - FR-2.3-S1: WHEN two horizontally adjacent cells in one section hold one block declaring `drawn = true, occludes = false, solid = false` THE SYSTEM SHALL emit no face on the boundary they share
  - FR-2.3-S2: WHEN two vertically adjacent cells in one section hold one block declaring `drawn = true, occludes = false, solid = false` THE SYSTEM SHALL emit no face on the horizontal boundary between them
  - FR-2.3-S3: WHEN two horizontally adjacent cells in two different sections hold one block declaring `drawn = true, occludes = false, solid = false` THE SYSTEM SHALL emit no face on the section boundary between them
  - FR-2.3-S4: WHEN two adjacent cells hold two different blocks each declaring `drawn = true, occludes = false, solid = false` THE SYSTEM SHALL emit a face on that boundary for each of them

- **FR-2.4**: `drawn`, `solid` and `occludes` are three independent answers, and a
  fixture in which they disagree meshes accordingly. **These three scenarios
  exist because every default in FR-1.1 makes every pre-existing fixture pass by
  construction** (`standards/global/testing.md` §2) — this is the one fixture in
  which the three cannot be each other, so it is split into three scenarios
  rather than one, so that a failure says which third failed.
  - FR-2.4-S1: WHEN a section holds a block declaring `solid = true, drawn = false, occludes = true` THE SYSTEM SHALL emit no face for it, in any direction, however its neighbours are declared
  - FR-2.4-S2: WHEN that block stands beside one declaring `solid = false, drawn = true, occludes = false` THE SYSTEM SHALL emit the second block's faces toward empty space
  - FR-2.4-S3: WHEN that block stands beside one declaring `solid = false, drawn = true, occludes = false` THE SYSTEM SHALL cull the second block's face toward the first

- **FR-2.5**: A block that does not occlude does not hide what is under it, so
  the world's existing surfaces are meshed exactly as before.
  - FR-2.5-S1: WHEN the shipped replay world is meshed with water drawn and non-occluding THE SYSTEM SHALL report the same upward face area per block as before the split — 4 095 of grass and 1 of stone

- **FR-2.6**: Water is drawn in the shipped world, and both its area and the
  world's quad count are what an independent per-voxel walk of the same world
  says, rather than a number copied from a run of the mesher.
  - FR-2.6-S1: WHEN the shipped replay world is meshed THE SYSTEM SHALL report a non-zero meshed area for `base:water`
  - FR-2.6-S2: WHEN the shipped replay world is meshed THE SYSTEM SHALL report, for every block including `base:water`, the area an independent per-voxel walk of the same world computes
  - FR-2.6-S3: WHEN the shipped replay world is meshed THE SYSTEM SHALL report a quad count derived from an independent walk of the same world rather than one snapshotted from a run of the mesher

- **FR-2.7**: A section answers "is this solid" and "is this drawn" as two
  separate questions. **The mesh fixtures and the mesher's own bench oracle both
  decide drawnness today by asking a section whether a cell is solid**
  (`crates/mc-world/src/section/mod.rs:176`), so an implementation that left that
  site answering one question would keep oracle and subject agreeing while both
  ignored `drawn`.
  - FR-2.7-S1: WHEN a section holds a block declaring `solid = true, drawn = false` THE SYSTEM SHALL report that cell as solid and as not drawn

### FR-3 — What can be aimed at is declared

- **FR-3.1**: A ray stops at the first cell whose block is declared `targetable`,
  never at the first cell whose block is declared `solid`.
  - FR-3.1-S1: WHEN a ray crosses a cell holding a block declaring `targetable = true, solid = false` THE SYSTEM SHALL report that cell as the hit
  - FR-3.1-S2: IF a ray crosses a cell holding a block declaring `targetable = false, solid = true` THEN THE SYSTEM SHALL pass through it and report whatever lies beyond

- **FR-3.2**: What can be aimed at follows an edit to the world, not only its
  load. **Targeting and collision read one pre-resolved view of the world that is
  written on every write** (`crates/mc-sim/src/world/mod.rs:251`), so a
  targetability view built once at load and never re-written satisfies FR-3.1
  entirely.
  - FR-3.2-S1: WHEN a block declaring `targetable = true, solid = false` is placed into a previously empty cell THE SYSTEM SHALL report that cell as the hit for a ray crossing it
  - FR-3.2-S2: WHEN the block in a cell that was reported as a hit is broken THE SYSTEM SHALL report the next targetable cell along the same ray instead

- **FR-3.3**: The shipped water is aimable, and only within reach.
  - FR-3.3-S1: WHEN a player aims at a cell holding `base:water` with a solid block behind it THE SYSTEM SHALL report the water's cell rather than the block behind it
  - FR-3.3-S2: IF the nearest cell holding `base:water` along a player's aim lies beyond their reach THEN THE SYSTEM SHALL report no target at all

- **FR-3.4**: **The scenario SPEC-020 could not write.** `base:water` declares
  `breakable = false`, and that declaration is inert for the player today because
  `targeted` stops only at a solid cell — measured, and recorded as a fuse in
  `crates/mc-sim/tests/shipped_water_is_not_broken_and_is_built_through.rs:138`,
  in `Refusal::Indestructible`'s doc comment, in `targeted`'s, and in
  `docs/technical/architecture.md:830-841`. Making water targetable is what makes
  the refusal live, and this spec owes the scenario.
  - FR-3.4-S1: WHEN a break is swung at a cell holding `base:water` THE SYSTEM SHALL refuse it as indestructible and leave the water in the cell
  - FR-3.4-S2: WHEN a break is swung at a cell holding `base:water` THE SYSTEM SHALL leave the solid block behind the water untouched

- **FR-3.5**: A placement aimed at water still builds through it, because
  `replaceable` is a separate declaration and is unchanged.
  - FR-3.5-S1: WHEN a placement is aimed at a cell holding `base:water` THE SYSTEM SHALL replace the water with the block being placed

### FR-4 — What stops a player, and what a player swims through

- **FR-4.1**: Collision reads `solid` and nothing else.
  - FR-4.1-S1: WHEN a player walks into a cell holding a block declaring `solid = true, drawn = false` THE SYSTEM SHALL stop them at its boundary
  - FR-4.1-S2: IF a player walks into a cell holding a block declaring `solid = false, drawn = true` THEN THE SYSTEM SHALL let them pass through it

- **FR-4.2**: A player whose body overlaps a cell holding a `swimmable` block can
  move upward through it from rest, without standing on anything.
  - FR-4.2-S1: WHILE a player's body overlaps a cell holding a block declaring `swimmable = true, solid = false` THE SYSTEM SHALL raise the player when they request upward movement, with no solid cell under their feet
  - FR-4.2-S2: IF a player's body overlaps only cells holding blocks declaring `swimmable = false` THEN THE SYSTEM SHALL not raise them when they request upward movement with no solid cell under their feet
  - FR-4.2-S3: WHEN a block declaring `swimmable = true, solid = false` is placed around a player at rest in previously empty cells THE SYSTEM SHALL raise them on their next upward request

- **FR-4.3**: `density` is the number a swimmable block's resistance is computed
  from, and it comes from the declaration rather than from the engine.
  - FR-4.3-S1: WHEN a player falls through a cell holding a swimmable block declaring `density = 1000.0` THE SYSTEM SHALL leave them with a lower downward speed after one tick than the same fall through one declaring `density = 0.0`
  - FR-4.3-S2: WHEN two swimmable blocks declare different densities THE SYSTEM SHALL produce a different fall speed through each, so no single engine-side constant can satisfy both
  - FR-4.3-S3: IF a player falls through a cell holding a swimmable block declaring `density = 100000.0` THEN THE SYSTEM SHALL leave their vertical speed at exactly zero after one tick, never upward

- **FR-4.4**: A pairing a player cannot exercise is accepted rather than refused.
  Refusing it would put a game rule in the engine that content could not
  override.
  - FR-4.4-S1: WHEN a block declares `density = 1000.0` and `swimmable = false` THE SYSTEM SHALL accept the declaration and leave a player's fall through that cell unchanged
  - FR-4.4-S2: WHEN a block declares `swimmable = true` and `solid = true` THE SYSTEM SHALL accept the declaration and stop a player at its boundary

- **FR-4.5**: A reload that changes only what is drawn does not move anybody. The
  reload's clearing search exists to move a player the *new solidity* left inside
  a block, and `drawn` is not solidity.
  - FR-4.5-S1: WHEN a reload changes nothing but one block's `drawn` and a player's body overlaps a cell holding it THE SYSTEM SHALL leave the player exactly where they are and move nobody

### FR-5 — Which block a new player holds

- **FR-5.1**: The block a player finds in hand is the first *colliding* block in
  registration order. The fourth consumer of the old bit is answered explicitly
  rather than left to fall out of the split: a held block is one you place to
  build with, and building means an obstacle.
  - FR-5.1-S1: WHEN the shipped content is registered THE SYSTEM SHALL put `base:dirt` in a new player's hand, unchanged by this split
  - FR-5.1-S2: IF a registry holds only blocks declaring `solid = false` THEN THE SYSTEM SHALL offer no held block at all, even where some of them are drawn
  - FR-5.1-S3: WHEN a reload publishes a registry whose first colliding block in registration order is `base:stone` THE SYSTEM SHALL report `base:stone` as the block the player holds

### FR-6 — A save records the split without confusing a retexture for a rebalance

- **FR-6.1**: `targetable`, `swimmable` and `density` are recorded as declared
  **behaviour**; `drawn` and `occludes` are recorded as declared **appearance**.
  The two lists keep separate revision bytes so that making water visible cannot
  claim every block in a save behaves differently.
  - FR-6.1-S1: WHEN two content roots differ in nothing but one block's `targetable` THE SYSTEM SHALL record different declared behaviour for that block and identical declared appearance
  - FR-6.1-S2: WHEN two content roots differ in nothing but one block's `swimmable` THE SYSTEM SHALL record different declared behaviour for that block and identical declared appearance
  - FR-6.1-S3: WHEN two content roots differ in nothing but one block's `density` THE SYSTEM SHALL record different declared behaviour for that block and identical declared appearance
  - FR-6.1-S4: WHEN two content roots differ in nothing but one block's `drawn` THE SYSTEM SHALL record identical declared behaviour for that block and different declared appearance
  - FR-6.1-S5: THE SYSTEM SHALL state a behaviour fold whose leading byte is `2` and an appearance fold whose leading byte is `3`, each asserted as a byte sequence built by hand

- **FR-6.2**: A world saved before this spec opens, and the verdict it is opened
  under is asserted whole rather than as an absence. **The committed fixture
  already reports `base:water` as behaviour-changed today**, so a scenario that
  asked only for "some block is named" would stay green against an implementation
  that folded no new field and never bumped the revision byte.
  - FR-6.2-S1: WHEN the committed pre-spec save is loaded against the shipped content THE SYSTEM SHALL report a verdict whose changed list is exactly `base:dirt`, `base:grass`, `base:stone`, `base:water` in ascending order, whose missing list is empty and whose retextured list is empty, and SHALL open the world naming those blocks on the error stream in one line
  - FR-6.2-S2: IF that load is asked to refuse changed blocks THEN THE SYSTEM SHALL refuse it, naming all four

- **FR-6.3**: An appearance change is classified as an appearance change and
  reported to nobody. Asserted as a whole verdict, because "no changed block was
  named" is also what an `occludes` folded into *neither* list would produce.
  - FR-6.3-S1: WHEN two content roots differ in nothing but one block's `occludes` THE SYSTEM SHALL report a verdict whose retextured list names exactly that block and whose changed and missing lists are empty

### FR-7 — The golden set and its judge move together

- **FR-7.1**: Drawing water changes what the mesher emits, so the scene revision
  is bumped, the previous revision's goldens are deleted, and the committed set
  is exactly what the new revision declares. Measured, not inferred: the shipped
  replay world holds 178 water voxels in 131 of its 4 096 columns, and zero quads
  name water today.
  - FR-7.1-S1: THE SYSTEM SHALL hold, under the committed golden root, exactly the capture directories the current scene revision declares and no directory of any other revision
  - FR-7.1-S2: IF the scene revision names a capture for which no golden is committed THEN THE SYSTEM SHALL fail naming the missing path

- **FR-7.2**: The ray-marched judge behind the golden set marches to the first
  **drawn** voxel, its classification of every sample is enumerated rather than
  asserted as an absence, and its prediction of water is proven non-vacuous.
  - FR-7.2-S1: WHEN the judge marches from the player's camera through the shipped replay THE SYSTEM SHALL classify every declared sample pixel as exactly one of sky, `base:grass`, `base:stone`, `base:dirt` or `base:water`, with at least one classified `base:water`
  - FR-7.2-S2: WHEN the judge marches through a world holding a block declaring `drawn = false, solid = true` THE SYSTEM SHALL pass through that block rather than predicting it
  - FR-7.2-S3: WHEN every sample the judge predicts as terrain is compared against the captured frame THE SYSTEM SHALL find each of them something other than sky

### FR-8 — The player sees water and swims in it, and a mod author can turn it off

The two capabilities Key Principle 7 requires, asserted rather than claimed.

- **FR-8.1**: The shipped game draws its sea.
  - FR-8.1-S1: WHEN the shipped client runs the declared capture ticks THE SYSTEM SHALL show water in the captured frame at every sample pixel where the judge predicts water, and at no fewer than one

- **FR-8.2**: The shipped game lets a player swim.
  - FR-8.2-S1: WHEN a player standing in the shipped replay world's sea requests upward movement with no solid cell under their feet THE SYSTEM SHALL raise them to the water surface at `y = 34`

- **FR-8.3**: A reload that changes what is drawn re-meshes the world, and one
  that changes only how a block behaves does not. **This is the wiring, not the
  policy** (`standards/global/testing.md` §2): the geometry-change test today
  keys on `(is_solid, textures)` (`crates/mc-sim/src/world/reload.rs:87`), so a
  correct `drawn` field that this site never learns about would leave an edited
  block looking unchanged until relaunch, with every other scenario green.
  - FR-8.3-S1: WHEN a reload candidate changes nothing but one block's `drawn` THE SYSTEM SHALL re-mesh the world so the change is visible without a relaunch
  - FR-8.3-S2: WHEN a reload candidate changes nothing but one block's `occludes` THE SYSTEM SHALL re-mesh the world so the change is visible without a relaunch
  - FR-8.3-S3: WHEN a reload candidate changes nothing but one block's `targetable` THE SYSTEM SHALL report an accepted reload whose published serial advances and whose rebuilt-section count is zero
  - FR-8.3-S4: WHEN a reload candidate changes nothing but one block's `density` THE SYSTEM SHALL report an accepted reload whose published serial advances and whose rebuilt-section count is zero

### FR-9 — The refusals a mod author meets are the ones the guide prints

- **FR-9.1**: Every refusal this spec adds is quoted in the modding guide exactly
  as the engine produces it. The guide holds three declaration refusals today and
  gains six.
  - FR-9.1-S1: THE SYSTEM SHALL produce, for each of the six refusals FR-1.1-S4, FR-1.2-S6, FR-1.2-S7, FR-1.2-S8, FR-1.2-S9 and FR-1.3-S1 name, exactly the message the modding guide prints for it

## Technical Considerations

### The two architecture decisions this spec re-decides

Both are answered here and both are binding on `/sdd-architect`, which owes the
mechanism rather than the ruling.

#### 1. Fold membership, per property

`crates/mc-world/src/persistence/format.rs:275-365` folds a save's declared
behaviour over `{input_version, name, is_solid, replaceable, breakable,
breaks_into}` at `BEHAVIOUR_REVISION = 1`, and its appearance over
`{input_version, name, textures[6]}` at `APPEARANCE_REVISION = 2`. The doc
comment states the test: *"a block whose texture changed is the same block to
stand on, and a block whose solidity or drop changed is not."* Applied to each
new property:

| Property | List | Why |
|----------|------|-----|
| `drawn` | **appearance** | A block that stops being drawn is still the same block to stand on, to build through and to break. Nothing about mutating the world changes. |
| `occludes` | **appearance** | Whether a face behind it is culled is what the world looks like and nothing else. |
| self-merging | **appearance** | Same, if it becomes a declared field at all — see the open mechanism question below. |
| `targetable` | **behaviour** | This is the property that makes `breakable = false` live (FR-3.4). A block that becomes aimable changes what a break and a placement *do* to a world — exactly what the behaviour list exists to report. |
| `swimmable` | **behaviour** | It changes how a player moves through a cell. Physics. |
| `density` | **behaviour** | Same. |
| `solid` | **behaviour**, unchanged | Already there, and its meaning narrows to the one thing the list already recorded it for. |

Consequences, stated rather than discovered:

- **`BEHAVIOUR_REVISION` moves to 2, and this is the first time it ever has.**
  Every block of every existing save reports as changed. That is survivable
  rather than fatal because PRO-956 shipped a fortnight ago: a save whose blocks
  report changed loads and names them instead of refusing. It is nonetheless a
  real cost, paid once, deliberately, and FR-6.2 asserts what a player sees for
  it. Worth stating precisely, because the near-miss version of this sentence is
  false: PRO-956's load path **has** already reported a real content edit —
  `base:water`'s `breakable = false` moves its behaviour fold today, which is
  what `crates/mc-world/tests/shipped_declarations_and_an_older_save.rs` and
  `docs/user/gameplay.md:68` record. What has never moved is the revision *byte*,
  and that is the difference between one block being named and all four being
  named. It is also exactly why FR-6.2 asserts the verdict **whole**: a scenario
  asking only that some block is named would stay green against an
  implementation that folded no new field at all.
- **`APPEARANCE_REVISION` moves to 3, and no player is told anything.** That is
  the designed behaviour of the split and it is why the two bytes are separate.
  `docs/planning/block-render-methods.md` §4 already ruled the same way for
  PRO-952's `render` field, on the same grounds.
- **The separation is preserved, not undone.** Putting `drawn` or `occludes` on
  the behaviour list would tell every player in existence that every block they
  built with behaves differently, on the strength of a rendering field. That is
  precisely the ambiguity the two revision bytes exist to prevent.
- **Field renames are free and revision bumps are not.** `postcard` encodes a
  struct positionally, so renaming `is_solid` changes no byte; adding a field or
  bumping `input_version` changes every one. This means the default values chosen
  in FR-1.1 have no bearing on save compatibility — every block reports changed
  either way — which usefully removes save compatibility from the list of things
  the default choice has to serve.
- **Only a test stating the byte sequence can see a revision byte move.** The
  format module's own doc comment records this as measured: every other witness
  compares one fold to another and cannot see a leading byte that moved in both.
  FR-6.1-S5 is that test, for both bytes.

#### 2. The render oracle's independence

`crates/mc-client/tests/support/oracle.rs` is the ray-marched judge behind the
golden set. Its `Voxels::is_solid` (about line 132) reads
`self.registry.resolve(name)?.is_solid`, and its header (lines 41-47) records the
water coincidence: *"A ray passes straight through water, because water's
definition is not solid — and the renderer draws the lakebed for the same reason,
since a non-solid block is never meshed. The two agree about a submerged surface
by construction rather than by luck."*

**Ruling: the judge marches to the first `drawn` voxel, and it stays a judge.**
The reasoning, because the ruling on its own is worth nothing here:

- **The independence claim in this module is about derivation, not about which
  field is read.** The header states it itself two paragraphs earlier: *"it reads
  solidity through `BlockDefinition::is_solid` and never through the pre-resolved
  bitset the physics uses, so an oracle and a subject that were both wrong about
  a block would still have to be wrong in two separate places."* The judge and
  the mesher already read the same declared field out of the same registry today.
  Moving both from `solid` to `drawn` changes nothing about that relationship.
- **`drawn` is not the mesher's decision.** The mesher's decision is
  `drawn(self) && !occludes(neighbour) && not-the-same-block`, computed per face
  over a resolved key table and a boundary plane. The judge computes "which block
  is the nearest drawn surface along this ray" by its own hand-built DDA over the
  world's own voxels. Those are two different questions answered by two
  independent implementations. A judge marching on `drawn` is not a second copy
  of the culling predicate; it never looks at a neighbour at all.
- **The coincidence being deleted was never evidence.** The header says the two
  agree *"by construction rather than by luck"* — which is an admission that the
  agreement about submerged surfaces is not independent confirmation of anything.
  Losing it costs no evidence. What it does cost is the *silence*: today a judge
  that had quietly stopped reading the registry at all would still agree about
  water, and nothing would say so.
- **So the split adds a positive control the module does not have today.**
  FR-7.2-S1 asserts the judge predicts `base:water` for at least one sample, and
  FR-7.2-S2 asserts it marches *through* a `drawn = false, solid = true` block —
  which no judge reading `solid` can pass. Together they are the two directions
  `testing.md` §2 asks for: the prediction is non-vacuous, and it is reached by
  the new field rather than the old one. **The judge comes out of this spec with
  more falsifiability than it went in with, not less.**
- **The named breaker, recorded now rather than reconstructed later.** A
  first-drawn-voxel march is right only while every drawn block is opaque. The
  day a translucent block exists — PRO-952 — the judge needs a second rule, and
  the assumption is recorded in this spec and belongs in the module header.

**No downgrade is being shipped and nothing is being escalated on this point.**

### Other decisions

- **`solid` keeps its name.** Renaming it `collides` would invalidate all four
  shipped declarations, 42 test files carrying declaration text, and the
  line-for-line doc check in `crates/mc-client/tests/documented_refusals.rs`, to
  gain a clearer name on a field whose documentation is being rewritten in this
  spec anyway. Its meaning narrows to collision; every consumer that meant
  something else moves off it.
- **`ResolvedBlock` gains nothing.** `crates/mc-core/src/content.rs`'s written
  argument is that it carries what a client *draws and predicts with* and
  excludes the rules by which a world is *mutated*. Applying that test would put
  `drawn`, `occludes` and self-merging in and keep `targetable` out. But
  measured: `ResolvedBlock::is_solid` reaches only
  `crates/mc-client/src/content.rs:73`, which builds `ClientContent::solidity`,
  and `ClientContent::is_solid` has **no production caller at all** — the mesher
  reaches solidity through `BlockRegistry` (`mesh/resolve.rs:324`), not through
  resolved content. Extending the struct now would add surface with no production
  reader, which is the "policy is not wiring" trap in `testing.md` §2 and the
  half-a-spec failure in Key Principle 7. So the struct is left alone and the
  question is handed to PRO-944, which moves the composition root and is the
  spec that gives the client a reason to hold any of this. **The absence is
  stated here rather than left silent.**
- **The four boolean defaults are knowingly the trap.** A `drawn` default equal
  to `solid` makes all 34 existing solidity fixtures pass by construction, and no
  count can see it. The countermeasures are named rather than left to review:
  **FR-2.4** is the fixture where `drawn`, `solid` and `occludes` cannot be each
  other; **FR-2.3** states `solid` in every one of its scenarios, because leaving
  it unstated lets an implementation ignoring `occludes` satisfy one half or the
  other; **FR-2.7** separates the two questions a section answers; **FR-7.2-S2**
  applies the same discipline to the judge; and **FR-1.4** is what stops the
  whole of FR-2 through FR-5 being satisfiable by synthetic fixtures while the
  shipped game is unchanged.
- **A boundary plane carries one bool per cell and no block identity.**
  `Boundaries` is `[[bool; 256]; 6]` (`crates/mc-world/src/mesh/resolve.rs:93`),
  filled by `shared_face` from `solidity_at`. So "no face against its own kind"
  (FR-2.3) **cannot be evaluated across a section boundary as the mesher is
  built today** — and the shipped sea spans many sections, so that case is
  reachable in the shipped world rather than hypothetical. Whatever mechanism
  `/sdd-architect` picks for FR-2.3, the boundary plane has to carry more than
  one bool, and FR-2.3-S3 is the scenario that says so. Measured by reading the
  type, not inferred.
- **Three places move together for the field list.**
  `RECOGNISED_FIELDS: [&str; 6]`
  (`crates/mc-world/src/content/luau_declaration/mod.rs:64`, order load-bearing),
  its hand-maintained mirror at
  `crates/mc-world/tests/luau_declaration_keys.rs:60`, and the refusal message
  printed at `docs/modding/blocks-items.md:396`.
- **The numeric reader is new machinery, at 100 % coverage.** No numeric field is
  read anywhere in the declaration parser today; the only numeric mention is
  `kind_of` rendering both variants as `"a number"` for refusal text. Luau
  delivers `density = 1` as `ScriptValue::Integer(i64)` and `density = 1.0` as
  `ScriptValue::Number(f64)`, so the reader accepts both arms. `density = 0/0`
  and `density = 1/0` are expressible in a declaration, which is what makes the
  finiteness check reachable rather than decorative.
  `standards/global/testing.md` §4 puts validation rules at 100 % coverage.

## Existing Code to Leverage

| What | Location | Reuse |
|------|----------|-------|
| Optional boolean field reader with a default | `crates/mc-world/src/content/luau_declaration/mod.rs:204` | the pattern the four new booleans follow |
| Refusal construction naming field and expectation | `crates/mc-world/src/content/luau_declaration/mod.rs:272` `FieldFault::wrong_kind` | the density refusals extend it |
| The recognised-field list and its refusal | same file, `:64` `RECOGNISED_FIELDS` | grows; order is load-bearing |
| Behaviour / appearance folds and their revision bytes | `crates/mc-world/src/persistence/format.rs:275-365` | both lists gain fields; both bytes bump |
| Changed-block load and report | shipped by SPEC-020 / PRO-956 | FR-6.2 exercises it for the first real bump |
| Independent per-voxel walk of a meshed world | `crates/mc-sim/tests/support/oracle.rs`, `crates/mc-sim/tests/scene_contract.rs` | FR-2.6's oracle, extended to water |
| Ray-marched frame judge | `crates/mc-client/tests/support/oracle.rs` | FR-7.2; marches on `drawn` |
| The golden re-shoot procedure | `docs/technical/rendering.md`, "Re-shooting a golden set" | FR-7.1; the procedure has a known corrupting failure mode and is followed as written |
| The inherited fuse | `crates/mc-sim/tests/shipped_water_is_not_broken_and_is_built_through.rs:138` | goes red in this spec; FR-3.4 is what replaces it |
| Reload geometry-change test | `crates/mc-sim/src/world/reload.rs:87` `drawn_of` | FR-8.2; must learn `drawn` and `occludes` |

## Out of Scope

Binding.

- **Transparency, draw order and sorting.** Water is drawn opaque. Alpha, a
  second pass and sorted transparency are PRO-952.
- **The inset top surface and the wave.** PRO-952. `docs/planning/block-render-methods.md`
  states this spec is its prerequisite, not its delivery.
- **Declared render methods.** A block naming an engine render method is PRO-952.
- **Light occlusion.** `occludes` here means view occlusion only. The day
  "occludes-view" and "occludes-light" part ways is named in
  `docs/planning/city-generation.md:903-906` and is not this spec.
- **Extending `ResolvedBlock` or the client's content view.** Handed to PRO-944
  with the reason recorded above.
- **Renaming `solid` to `collides`.**
- **A wire representation of any of these properties.** `mc-proto` and `mc-net`
  are empty skeletons; there is nothing to migrate.
- **Breath, drowning, swim animation, or a swimming camera.** FR-4.2 is moving
  upward through a swimmable block and nothing more.
- **New blocks, or any content beyond editing the four shipped declarations.**
- **Reclaiming the golden set's previous revision directories for history.** They
  are deleted, as the re-shoot procedure requires.

## Dependencies

- **PRO-956 / SPEC-020 must be merged first.** It is (`main` at `57327a9`). Its
  changed-save load path is what makes the behaviour-revision bump survivable
  rather than a refusal of every existing world.
- **PRO-947 / SPEC-019 must be merged first.** It is. The golden set this spec
  re-shoots is the one PRO-947 minted, and re-shooting a set that was about to
  move anyway would have been the third repetition the roadmap ruled against.

## Assumptions

- **`density` means resistance to movement through a block's volume, in kg/m³.**
  Neither PRO-904 nor `product/roadmap.md` states what it does; it arrives with
  `swimmable` and the exit criterion is "swim in it". If it was meant as mass for
  physics or as light attenuation, FR-4.3 and FR-4.4 are wrong and nothing else
  in this spec is. **Raised with the requester; see Open Questions.**
- **Every drawn block is opaque.** The judge's first-drawn-voxel march depends on
  it, and PRO-952 is the named breaker.
- **The player's camera sees water in at least one declared sample pixel.**
  Assumed by FR-7.2-S1 and FR-8.1-S1, and **not yet measured** — the sea is 131
  of 4 096 columns, one or two blocks deep, against a 32 × 18 grid. See Open
  Questions.

## Definition of Done — documentation

Key Principle 3, for all three audiences. Every item below is a **surface this
spec implements**, so none of it is deferrable and none of it is "not applicable".
FR-1.3-S2 and FR-9.1-S1 make the mod-author refusals mechanically checkable; the
rest is prose that no test can assert, which is exactly why it is enumerated here
rather than left to a reviewer's memory.

**Three sentences currently on the player page become false the day this merges**
(`docs/user/gameplay.md:60-72`), measured by reading them:

- *"Water … draws nothing: no face of it is ever emitted."*
- *"The crosshair only ever targets solid blocks, and water is not one, so a
  swing at a cell of water goes straight through it and breaks whatever solid
  block is behind."*
- *"water is the one shipped block whose recorded behaviour moved"* — after the
  revision bump all four shipped blocks report as changed.

Owed, per audience:

- **Player** (`docs/user/gameplay.md`) — the sea is visible; you can wade and
  swim up out of it; a swing at water is refused and the water stays; a save from
  before this build names all four shipped blocks rather than one.
- **Mod author** (`docs/modding/blocks-items.md`) — five new fields, each with its
  type, its default, its bound, and the refusal it produces; the sentence "the
  three optional fields are independent of one another" becomes eight; the field
  table and the recognised-field refusal at `:396` are rewritten; and a worked
  example that runs — a declaration that is drawn without being solid.
- **Engine reader** — `docs/technical/architecture.md:830-841` states
  `Refusal::Indestructible` is unreachable and why; it becomes reachable, and the
  handover text there, in that variant's doc comment, in `targeted`'s, and in the
  header of `crates/mc-sim/tests/shipped_water_is_not_broken_and_is_built_through.rs`
  all describe a fuse that this spec blows. Four sites, all four rewritten.
  `docs/technical/world-format.md` carries the fold revisions and their numbers.
  `docs/technical/rendering.md` records the re-shoot and why the revision moved.
  `crates/mc-client/tests/support/oracle.rs:41-47` records the water coincidence
  as the module's independence story and is replaced by the positive control and
  the named PRO-952 breaker.

## A note on scenario patterns

Guideline 4 asks every requirement for an unwanted-behaviour scenario. Eleven of
these requirements are written entirely with `WHEN`, and in each the negative
direction is carried by a *sibling* scenario rather than by an `IF` clause —
FR-2.3-S4 is the case where faces **are** emitted, FR-2.4-S3 the case where one
**is** culled, FR-4.4 the two pairings that are accepted rather than refused,
FR-6.1-S4 the property that must **not** move the behaviour fold, FR-6.3 the
change that must be reported to nobody, FR-8.3-S3 and S4 the reloads that must
**not** re-mesh. The substance is present and the pattern is not; recorded here
so a reviewer does not have to rediscover it, and so that a later stage does not
"fix" it by adding scenarios that re-prove a sibling through the same code path.

## Open Questions

Must be empty before implementation starts. Question 1 is for the requester;
2 and 3 are for `/sdd-architect`.

1. **Should PRO-904 ship as one spec or two?** 78 scenarios is the honest size of
   the issue as scoped, and it sits next to the 82-scenario spec
   `product/roadmap.md:186-193` names as the one that hid eighteen wrong-reason
   passes — the roadmap's own stated reason for **not** folding PRO-904 into
   PRO-947. The natural seam is clean, and the two halves are counted rather
   than estimated:

   | Half | Scenarios | What it is |
   |------|-----------|------------|
   | *"you can see water"* | **54** | FR-1.1 (4), FR-1.3 (2), FR-1.4 (2), FR-2 (18), FR-3 (9), FR-5 (3), FR-6.1-S1/S4/S5 (3), FR-6.2 (2), FR-6.3 (1), FR-7 (5), FR-8.1 (1), FR-8.3-S1/S2/S3 (3), FR-9 (1). One golden re-shoot, one behaviour bump, no new machinery. |
   | *"you can swim in it"* | **24** | FR-1.2 (9), FR-6.1-S2/S3 (2), FR-4 (11), FR-8.2 (1), FR-8.3-S4 (1). No re-shoot, and all of the genuinely new machinery: the first numeric validated-input path, and a physics change. |
   Each half delivers a named player capability on its own, so Key Principle 7 is
   satisfied either way. **Recommendation: split.** The re-shoot is paid once
   regardless, so the usual argument for bundling does not apply here. Not acted
   on unilaterally, because the roadmap and the issue both scope them together.
2. **Does any declared sample pixel see water?** If not, FR-7.2-S1 and FR-8.1-S1
   are unsatisfiable without changing the fixture, and the remedies — raising
   `SEA_LEVEL`, or moving the camera path — each move the scene contract a second
   time inside one spec. This is the **first thing `/sdd-architect` measures**,
   before anything is designed against it.
3. **Is "no face between two cells of the same drawn non-occluding block" a
   declared field or an engine rule, and what does the boundary plane carry?**
   FR-2.3 states the behaviour and leaves the mechanism open. A declared
   `merges_with_self` honours invariant 1 and costs a field with no identified use
   for its `false` value; an engine identity rule costs a derivation content
   cannot override. Fold membership is **appearance** either way. Either answer
   has to widen `Boundaries`, which today carries `[[bool; 256]; 6]` and no block
   identity — see Other decisions.

## Clarifications

### Session 2026-08-22

- Q: Rigor tier? → A: `high`, decided before this stage and not re-derived. Two
  binding re-decisions with existing written answers and no task-list visibility.
- Q: Are blocks TOML or Luau? → A: `*.luau`. `*.toml` has not existed since
  PRO-917; the issue description was corrected on 2026-08-22.
- Q: Does drawing water re-shoot the committed goldens? → A: Yes. Measured, not
  inferred — the replay world holds 178 water voxels and
  `no_quad_of_the_meshed_replay_names_the_block_that_fills_its_sea` passes today
  with a positive control proving the water is there.
- Q: Is `is_solid` folded into a save's declared behaviour? → A: Yes,
  `format.rs:343`. The split moves the behaviour byte for every block of every
  existing save, once.
