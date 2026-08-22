---
id: SPEC-021
title: One bit answers four questions — solid, drawn, occludes and targetable split
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
properties, so that a block states what it *is* rather than having three further
facts derived from one, and a player can see water.

## Stakeholder capability delivered

Named, per Key Principle 7:

- **Player** — the sea is visible. You can walk to the coast and see the water
  surface; today the sea is a hole in the world that you walk through and cannot
  see. You can aim at water, and a swing at it is refused rather than breaking
  the block behind it.
- **Mod author** — you can declare a block that is visible without being solid,
  solid without being visible, or aimable without being solid, and each of those
  is a field you write in a `*.luau` file with a refusal that names it when you
  get it wrong.

## User Stories

- As a **player**, I want to see the sea, so that water is part of the world
  rather than an invisible hole in it.
- As a **player**, I want a swing aimed at water to leave the water and the block
  behind it alone, so that what the content says about water is what happens.
- As a **mod author**, I want to declare separately whether my block is drawn,
  occludes, collides and can be aimed at, so that a decorative block is not
  forced to be an obstacle and an invisible barrier is expressible.
- As a **player**, I want a world I saved before this change to open and tell me
  which blocks now behave differently, so that a content update does not cost me
  my world.

## Functional Requirements

Scenario rules: `standards/global/scenario-guidelines.md`. Each scenario gets at
least one test, mapped in this folder's `test-map.md` — that mapping is a floor,
not a ceiling.

**Scenario count: 54.** The one command that measures it, so that no later stage
counts differently — the anchor is what keeps prose references to a scenario out
of the total:

```sh
grep -cE "^  - FR-[0-9]+\.[0-9]+-S[0-9]+: " spec.md
```

Per group: FR-1 8, FR-2 18, FR-3 9, FR-4 3, FR-5 6, FR-6 5, FR-7 4, FR-8 1.
`grep -oE "^  - FR-[0-9]+\.[0-9]+-S[0-9]+" spec.md | sort | uniq -d` must print
nothing.

**This spec was cut from 78 scenarios to 54 on 2026-08-22, and the 24 that left
went to PRO-957 rather than being dropped.** See Out of Scope. The reason for the
cut is `product/roadmap.md:186-193`: it refused to fold PRO-904 into PRO-947
because an 82-scenario spec hid eighteen instances of something passing for the
wrong reason, and applied consistently that argues against 78 just as much.

### FR-1 — A block declares three new properties

- **FR-1.1**: A declaration may state `drawn`, `occludes` and `targetable` as
  booleans. Each is optional and each defaults to the value the declaration states
  for `solid`, so no existing declaration becomes invalid. `solid` remains
  required and now means collision and nothing else.
  - FR-1.1-S1: WHEN a declaration states `solid = true` and none of the three new booleans THE SYSTEM SHALL read `drawn`, `occludes` and `targetable` as true
  - FR-1.1-S2: WHEN a declaration states `solid = false` and none of the three new booleans THE SYSTEM SHALL read `drawn`, `occludes` and `targetable` as false
  - FR-1.1-S3: WHEN a declaration states `solid = false` and `drawn = true` THE SYSTEM SHALL read `drawn` as true and `occludes` and `targetable` as false
  - FR-1.1-S4: IF a declaration states `drawn = 1` THEN THE SYSTEM SHALL refuse the content naming the file, the block, the field `drawn`, and that it must be `true or false` but is `a number`

- **FR-1.2**: The set of fields a declaration may state is quoted back verbatim
  when an unrecognised one is written, and the modding guide prints the same
  refusal the engine produces.
  - FR-1.2-S1: IF a declaration states a field named `drawnn` THEN THE SYSTEM SHALL refuse it and quote exactly the nine recognised field names, in declaration order — `name`, `texture`, `solid`, `replaceable`, `breakable`, `breaks_into`, `drawn`, `occludes`, `targetable`
  - FR-1.2-S2: THE SYSTEM SHALL produce, for that refusal, exactly the message the modding guide prints for it

- **FR-1.3**: The shipped content declares water's new properties, and declares
  nothing new for the other three blocks. **Without this, every scenario in FR-2
  through FR-4 can be satisfied by synthetic fixtures while the shipped game is
  unchanged** — the shipped declaration is the only thing that makes the player
  capability real.
  - FR-1.3-S1: WHEN the shipped content root is read THE SYSTEM SHALL read `base:water` as not solid, drawn, non-occluding, targetable, unbreakable and replaceable
  - FR-1.3-S2: WHEN the shipped content root is read THE SYSTEM SHALL read `base:dirt`, `base:grass` and `base:stone` as solid, drawn, occluding and targetable

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

### FR-4 — Which block a new player holds

- **FR-4.1**: The block a player finds in hand is the first *colliding* block in
  registration order. The fourth consumer of the old bit is answered explicitly
  rather than left to fall out of the split: a held block is one you place to
  build with, and building means an obstacle.
  - FR-4.1-S1: WHEN the shipped content is registered THE SYSTEM SHALL put `base:dirt` in a new player's hand, unchanged by this split
  - FR-4.1-S2: IF a registry holds only blocks declaring `solid = false` THEN THE SYSTEM SHALL offer no held block at all, even where some of them are drawn
  - FR-4.1-S3: WHEN a reload publishes a registry whose first colliding block in registration order is `base:stone` THE SYSTEM SHALL report `base:stone` as the block the player holds

### FR-5 — A save records the split without confusing a retexture for a rebalance

- **FR-5.1**: `targetable` is recorded as declared **behaviour**; `drawn` and
  `occludes` are recorded as declared **appearance**. The two lists keep separate
  revision bytes so that making water visible cannot claim every block in a save
  behaves differently.
  - FR-5.1-S1: WHEN two content roots differ in nothing but one block's `targetable` THE SYSTEM SHALL record different declared behaviour for that block and identical declared appearance
  - FR-5.1-S2: WHEN two content roots differ in nothing but one block's `drawn` THE SYSTEM SHALL record identical declared behaviour for that block and different declared appearance
  - FR-5.1-S3: THE SYSTEM SHALL state a behaviour fold whose leading byte is `2` and an appearance fold whose leading byte is `3`, each asserted as a byte sequence built by hand

- **FR-5.2**: A world saved before this spec opens, and the verdict it is opened
  under is asserted whole rather than as an absence. **The committed fixture
  already reports `base:water` as behaviour-changed today**, so a scenario that
  asked only for "some block is named" would stay green against an implementation
  that folded no new field and never bumped the revision byte.
  - FR-5.2-S1: WHEN the committed pre-spec save is loaded against the shipped content THE SYSTEM SHALL report a verdict whose changed list is exactly `base:dirt`, `base:grass`, `base:stone`, `base:water` in ascending order, whose missing list is empty and whose retextured list is empty, and SHALL open the world naming those blocks on the error stream in one line
  - FR-5.2-S2: IF that load is asked to refuse changed blocks THEN THE SYSTEM SHALL refuse it, naming all four

- **FR-5.3**: An appearance change is classified as an appearance change and
  reported to nobody. Asserted as a whole verdict, because "no changed block was
  named" is also what an `occludes` folded into *neither* list would produce.
  - FR-5.3-S1: WHEN two content roots differ in nothing but one block's `occludes` THE SYSTEM SHALL report a verdict whose retextured list names exactly that block and whose changed and missing lists are empty

### FR-6 — The golden set and its judge move together

- **FR-6.1**: Drawing water changes what the mesher emits, so the scene revision
  is bumped, the previous revision's goldens are deleted, and the committed set
  is exactly what the new revision declares. Measured, not inferred: the shipped
  replay world holds 178 water voxels in 131 of its 4 096 columns, and zero quads
  name water today.
  - FR-6.1-S1: THE SYSTEM SHALL hold, under the committed golden root, exactly the capture directories the current scene revision declares and no directory of any other revision
  - FR-6.1-S2: IF the scene revision names a capture for which no golden is committed THEN THE SYSTEM SHALL fail naming the missing path

- **FR-6.2**: The ray-marched judge behind the golden set marches to the first
  **drawn** voxel, its classification of every sample is enumerated rather than
  asserted as an absence, and its prediction of water is proven non-vacuous.
  - FR-6.2-S1: WHEN the judge marches from the player's camera through the shipped replay THE SYSTEM SHALL classify every declared sample pixel as exactly one of sky, `base:grass`, `base:stone`, `base:dirt` or `base:water`, with at least one classified `base:water`
  - FR-6.2-S2: WHEN the judge marches through a world holding a block declaring `drawn = false, solid = true` THE SYSTEM SHALL pass through that block rather than predicting it
  - FR-6.2-S3: WHEN every sample the judge predicts as terrain is compared against the captured frame THE SYSTEM SHALL find each of them something other than sky

### FR-7 — The player sees water, and a mod author can turn it off

The two capabilities Key Principle 7 requires, asserted rather than claimed.

- **FR-7.1**: The shipped game draws its sea.
  - FR-7.1-S1: WHEN the shipped client runs the declared capture ticks THE SYSTEM SHALL show water in each captured frame at every sample pixel where the judge predicts water, and at no fewer than one sample pixel in every one of those frames

- **FR-7.2**: A reload that changes what is drawn re-meshes the world, and one
  that changes only how a block behaves does not. **This is the wiring, not the
  policy** (`standards/global/testing.md` §2): the geometry-change test today
  keys on `(is_solid, textures)` (`crates/mc-sim/src/world/reload.rs:87`), so a
  correct `drawn` field that this site never learns about would leave an edited
  block looking unchanged until relaunch, with every other scenario green.
  - FR-7.2-S1: WHEN a reload candidate changes nothing but one block's `drawn` THE SYSTEM SHALL re-mesh the world so the change is visible without a relaunch
  - FR-7.2-S2: WHEN a reload candidate changes nothing but one block's `occludes` THE SYSTEM SHALL re-mesh the world so the change is visible without a relaunch
  - FR-7.2-S3: WHEN a reload candidate changes nothing but one block's `targetable` THE SYSTEM SHALL report an accepted reload whose published serial advances and whose rebuilt-section count is zero

### FR-8 — The refusals a mod author meets are the ones the guide prints

- **FR-8.1**: Every refusal this spec adds is quoted in the modding guide exactly
  as the engine produces it.
  - FR-8.1-S1: THE SYSTEM SHALL produce, for each of the two refusals FR-1.1-S4 and FR-1.2-S1 name, exactly the message the modding guide prints for it

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
| self-merging | **appearance** | Same, if it becomes a declared field at all — see Open Question 2. |
| `targetable` | **behaviour** | This is the property that makes `breakable = false` live (FR-3.4). A block that becomes aimable changes what a break and a placement *do* to a world — exactly what the behaviour list exists to report. |
| `solid` | **behaviour**, unchanged | Already there, and its meaning narrows to the one thing the list already recorded it for. |

PRO-957's `swimmable` and `move_resistance` are **behaviour** on the same
grounds — they change how a player moves through a cell — and that ruling is
recorded here because it was made here, even though the fields are that spec's.
It costs PRO-957 no second revision bump: the behaviour byte moves in this spec
and PRO-957 lands on the same revision.

Consequences, stated rather than discovered:

- **`BEHAVIOUR_REVISION` moves to 2.** Every block of every existing save reports
  as changed. That is survivable rather than fatal because PRO-956 shipped a
  fortnight ago: a save whose blocks report changed loads and names them instead
  of refusing. It is nonetheless a real cost, paid once, deliberately, and FR-5.2
  asserts what a player sees for it. Worth stating precisely, because the
  near-miss version of this sentence is false: PRO-956's load path **has** already
  reported a real content edit — `base:water`'s `breakable = false` moves its
  behaviour fold today, which is what
  `crates/mc-world/tests/shipped_declarations_and_an_older_save.rs` and
  `docs/user/gameplay.md:68` record. What has never moved is the revision *byte*,
  and that is the difference between one block being named and all four being
  named. It is also exactly why FR-5.2 asserts the verdict **whole**: a scenario
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
  FR-5.1-S3 is that test, for both bytes.

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
  FR-6.2-S1 asserts the judge classifies every sample and predicts `base:water`
  for at least one, and FR-6.2-S2 asserts it marches *through* a
  `drawn = false, solid = true` block — which no judge reading `solid` can pass.
  Together they are the two directions `testing.md` §2 asks for: the prediction is
  non-vacuous, and it is reached by the new field rather than the old one. **The
  judge comes out of this spec with more falsifiability than it went in with, not
  less.**
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
  `drawn` and `occludes` in and keep `targetable` out. But measured:
  `ResolvedBlock::is_solid` reaches only `crates/mc-client/src/content.rs:73`,
  which builds `ClientContent::solidity`, and `ClientContent::is_solid` has **no
  production caller at all** — the mesher reaches solidity through
  `BlockRegistry` (`mesh/resolve.rs:324`), not through resolved content.
  Extending the struct now would add surface with no production reader, which is
  the "policy is not wiring" trap in `testing.md` §2 and the half-a-spec failure
  in Key Principle 7. So the struct is left alone and the question is handed to
  PRO-944, which moves the composition root and is the spec that gives the client
  a reason to hold any of this. **The absence is stated here rather than left
  silent.**
- **The three boolean defaults are knowingly the trap.** A `drawn` default equal
  to `solid` makes all 34 existing solidity fixtures pass by construction, and no
  count can see it. The countermeasures are named rather than left to review:
  **FR-2.4** is the fixture where `drawn`, `solid` and `occludes` cannot be each
  other; **FR-2.3** states `solid` in every one of its scenarios, because leaving
  it unstated lets an implementation ignoring `occludes` satisfy one half or the
  other; **FR-2.7** separates the two questions a section answers; **FR-6.2-S2**
  applies the same discipline to the judge; and **FR-1.3** is what stops the
  whole of FR-2 through FR-4 being satisfiable by synthetic fixtures while the
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
  printed at `docs/modding/blocks-items.md:396`. It becomes `[&str; 9]` here and
  `[&str; 11]` in PRO-957.

## Existing Code to Leverage

| What | Location | Reuse |
|------|----------|-------|
| Optional boolean field reader with a default | `crates/mc-world/src/content/luau_declaration/mod.rs:204` | the pattern the three new booleans follow |
| Refusal construction naming field and expectation | `crates/mc-world/src/content/luau_declaration/mod.rs:272` `FieldFault::wrong_kind` | FR-1.1-S4 |
| The recognised-field list and its refusal | same file, `:64` `RECOGNISED_FIELDS` | grows to nine; order is load-bearing |
| Behaviour / appearance folds and their revision bytes | `crates/mc-world/src/persistence/format.rs:275-365` | both lists gain fields; both bytes bump |
| Changed-block load and report | shipped by SPEC-020 / PRO-956 | FR-5.2 exercises it for the first revision bump |
| Total three-list verdict | `crates/mc-world/src/persistence/table.rs:54` `RegistryVerdict` | FR-5.2 and FR-5.3 assert the whole verdict, never an absence |
| Independent per-voxel walk of a meshed world | `crates/mc-sim/tests/support/oracle.rs`, `crates/mc-sim/tests/scene_contract.rs` | FR-2.6's oracle, extended to water |
| Ray-marched frame judge | `crates/mc-client/tests/support/oracle.rs` | FR-6.2; marches on `drawn` |
| The golden re-shoot procedure | `docs/technical/rendering.md`, "Re-shooting a golden set" | FR-6.1; the procedure has a known corrupting failure mode and is followed as written |
| The inherited fuse | `crates/mc-sim/tests/shipped_water_is_not_broken_and_is_built_through.rs:138` | goes red in this spec; FR-3.4 is what replaces it |
| Reload geometry-change test | `crates/mc-sim/src/world/reload.rs:87` `drawn_of` | FR-7.2; must learn `drawn` and `occludes` |

## Out of Scope

Binding.

### Moved to PRO-957, not dropped

**The 24 scenarios below were specified, audited and then cut from this spec on
2026-08-22 when it was split at 78.** They are named individually so that a
reviewer meeting the gap can tell scope *moved* from scope *dropped* — the same
distinction SPEC-019 recorded when it retired an FR rather than deleting it.

- **`swimmable`, and everything that reads it.** A player raised through a
  swimmable block from rest, a player *not* raised where nothing is swimmable, a
  swimmable block placed around a player at rest, the `swimmable = true` plus
  `solid = true` pairing accepted rather than refused, `swimmable`'s membership of
  the behaviour fold, and a player swimming to the surface of the shipped sea at
  `y = 34`. **Capability carried: a player can swim.**
- **The block declaration's first numeric field.** Nine scenarios covering the
  reader, both of Luau's numeric forms, the absent default, both accepted bounds,
  and four refusals — negative, above bound, non-finite, and not a number. Plus
  the field's membership of the behaviour fold, the reload that must not re-mesh
  for it, and its presence in the shipped water declaration. **Capability
  carried: a mod author can state a validated number in a declaration.**
- **The field is `move_resistance`, not `density`.** This spec's draft called it
  `density`, as an assumption flagged rather than buried. The project owner ruled
  against the name: drag and density are independent properties and one number
  cannot express both — honey and water have nearly the same density and very
  different viscosity — and the name is what mod authors write, so it is the most
  expensive part to change later. `density` stays reserved for mass-per-volume if
  buoyancy is ever simulated, which would need the player to have one too.
  Minetest's `move_resistance` is the prior art. Recorded here because the
  reasoning was produced here, and because a reader of this spec's history will
  otherwise meet `density` with no explanation.

PRO-957 re-shoots no goldens and lands on the behaviour revision this spec bumps.

### Never in PRO-904

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
- **PRO-957 depends on this spec**, not the other way round: `swimmable` and
  `move_resistance` join the behaviour fold this spec bumps, and a swimmable
  block a player cannot see is not a capability anybody can exercise.

## Assumptions

- **Every drawn block is opaque.** The judge's first-drawn-voxel march depends on
  it, and PRO-952 is the named breaker.
- **The player's camera sees water in at least one declared sample pixel.**
  Assumed by FR-6.2-S1 and FR-7.1-S1. **Measured, and false against the fixture as
  it stood** — 0 water pixels at all three ticks, and 0 unoccluded lines of sight
  to the sea at every one of the 120 ticks. It becomes true only because the
  declared spawn moves; see Open Questions 1. **So this is an assumption the
  implementation has to establish rather than inherit**: until the new spawn pair
  is derived from the real simulation, both scenarios are unsatisfiable, and a
  phase that reports them green without a moved spawn has measured something other
  than what they say.

## Definition of Done — documentation

Key Principle 3, for all three audiences. Every item below is a **surface this
spec implements**, so none of it is deferrable and none of it is "not applicable".
FR-1.2-S2 and FR-8.1-S1 make the mod-author refusals mechanically checkable; the
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

- **Player** (`docs/user/gameplay.md`) — the sea is visible; a swing at water is
  refused and the water stays, and the block behind it is untouched; a save from
  before this build names all four shipped blocks rather than one.
- **Mod author** (`docs/modding/blocks-items.md`) — three new fields, each with
  its type, its default and the refusal it produces; the sentence "the three
  optional fields are independent of one another" becomes six; the field table and
  the recognised-field refusal at `:396` are rewritten; and a worked example that
  runs — a declaration that is drawn without being solid.
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

Guideline 4 asks every requirement for an unwanted-behaviour scenario. Several of
these requirements are written entirely with `WHEN`, and in each the negative
direction is carried by a *sibling* scenario rather than by an `IF` clause —
FR-2.3-S4 is the case where faces **are** emitted, FR-2.4-S3 the case where one
**is** culled, FR-5.1-S2 the property that must **not** move the behaviour fold,
FR-5.3 the change that must be reported to nobody, FR-7.2-S3 the reload that must
**not** re-mesh. The substance is present and the pattern is not; recorded here so
a reviewer does not have to rediscover it, and so that a later stage does not
"fix" it by adding scenarios that re-prove a sibling through the same code path.

## Open Questions

**Both are now answered, with measurements, in `architecture.md`.** They are kept
here with their answers rather than deleted, so a reader meeting the question can
see what settled it.

1. **Does any declared sample pixel see water?** **No — measured, and it is not an
   aiming problem.** Water's surface is a strip at `x ∈ [60,63]`, `z ∈ [0,34]`;
   the declared spawn is (32,32) facing 225°, deliberately away from it so the
   landmark pillar lands in the first frame. Across all three capture ticks the
   grid sees 0 water pixels, unoccluded line of sight to any of the 131 surface
   voxels is 0 at **every one of the 120 ticks**, and a 72-yaw sweep at the spawn
   column finds water at no yaw at all. The eye travels ~3 blocks over the script
   while the sea is 28–31 blocks away behind rising terrain, so no added tick,
   pitch or yaw reaches it.

   **One claim this question originally made is false, and the distinction decides
   the remedy.** The two remedies do *not* both move the scene contract.
   `scene_contract` takes `&[SectionQuads]` and reads no camera, so
   `SCENE_QUAD_COUNT`, `total_face_area` and `area_by_block` are properties of the
   world's mesh alone. **Moving the camera moves the goldens, which move once in
   this spec regardless. Moving the world moves the scene contract.** Raising
   `SEA_LEVEL` is therefore rejected — measured, the first level putting water at
   the spawn's feet is 38, which submerges 1 535 columns, 37% of the world.

   The answer is to **move the declared spawn**, which is two declarations in one
   file: 4 715 (column, yaw) pairs hold water through the script's +30° turn, and
   616 of those hold water, sky, grass and the landmark pillar in one frame, so
   the property the yaw exists for is preserved rather than sacrificed. **This is
   a product change made for a test's benefit and is recorded as one** — a human
   player can already walk to the sea today; it is only the automated capture that
   cannot see it. Accepted by the project owner on 2026-08-22.
2. **Is "no face between two cells of the same drawn non-occluding block" a
   declared field or an engine rule, and what does the boundary plane carry?**
   **An engine rule, evaluated over key identity; the plane carries a `Key` per
   cell.** The rule names no block and no id, and a `Key` comparison compares
   identity under a table deduplicated by name, so `visible_face`'s claim that no
   name and no runtime id is read survives verbatim. `merges_with_self = false`
   has exactly one identified use — interior faces of a translucent volume — and
   that belongs to PRO-952, which bumps the appearance byte for `render` anyway,
   so deferring the field costs no extra bump. **Consequence: this spec's
   `APPEARANCE_REVISION` bump covers `drawn` and `occludes` only, and the
   ruling-table row for self-merging is vacated rather than contradicted.** The
   argument against is recorded at the rule's own site: an engine rule is a
   derivation content cannot override, and PRO-952 is named there as the change
   that must turn it back into a field.

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
- Q: One spec or two? → A: Two. Cut to 54 scenarios here; the 24 swim scenarios
  went to PRO-957. The seam was counted before the recommendation was made, and
  Key Principle 7 holds on both sides of it.
- Q: What does `density` mean? → A: It does not exist. The project owner ruled
  the field is `move_resistance`, named for the mechanic it drives, and it belongs
  to PRO-957. See Out of Scope for the reasoning.
