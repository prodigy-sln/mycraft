# Architecture: One bit answers four questions

Spec: `spec.md` (SPEC-021, PRO-904, rigor `high`, 54 scenarios).
Requirements: `requirements.md`.

Every figure below was produced by a command run in this session against
`feature/PRO-904-solid-split-properties` at `1765433`, or is arithmetic over
such a figure and labelled *derived*. Figures the spec and requirements
recorded earlier were **re-run** rather than copied; where a re-run disagreed
with the spec it is said so.

The four decisions the spec already made — fold membership, the render
oracle's independence, `ResolvedBlock` gaining nothing, and the golden
re-shoot — are taken as given and are not re-argued. This document owes the
*mechanism* for each, and the two open questions.

---

## The two open questions, answered

### Open Question 1 — no declared sample pixel sees water, and none can

**Answer: no, and the reason is not the camera's aim.** Measured three ways.

| Measurement | Result |
|---|---|
| Water census over `ReplayWorld::generate(REPLAY_SEED, …)` | 178 voxels; 84 columns one deep, 47 two deep; every top open to air; `y ∈ [33, 34]` |
| Water surface voxels' footprint | 131 voxels at `x ∈ [60, 63]`, `z ∈ [0, 34]` — a strip on the far `+x` edge |
| The 32 × 18 grid at the three declared capture ticks, marching to the first **drawn** voxel with water counted drawn | tick 0: 441 grass / 135 sky. tick 59: 544 grass / 32 sky. tick 119: 539 grass / 3 stone / 34 sky. **Water: 0, 0, 0** |
| Unoccluded line of sight from the published eye to *any* of the 131 water surface voxels, at **every one of the 120 ticks** | 0 at every tick |
| The declared spawn column (32, 32) at its declared spawn height, swept over 72 yaws at 5° | water visible at **no yaw at all** |

The first census reproduces `requirements.md` §2 exactly (178 / 84 / 47 / all
tops open), so that figure is confirmed rather than relayed.

The spawn stands at `(32.5, 41.62, 32.5)` and faces `225°` — forward
`(−0.707, 0, −0.707)`, *away* from the sea by construction, because
`SPAWN_YAW_DEGREES` exists to put the landmark pillar at column (12, 12) in
the first frame (`crates/mc-sim/src/replay/spawn.rs:47`). Over the whole
120-tick script the eye travels about 3 blocks. So this is not an aiming
problem and no added capture tick, pitch or yaw fixes it: from that eye the
sea is 28–31 blocks away behind terrain that rises above it, and the line of
sight is blocked for all 131 voxels at all 120 ticks.

**FR-6.2-S1 and FR-7.1-S1 are unsatisfiable against the fixture as it
stands.** The spec's own Assumptions section flagged this; it is now measured
and the assumption is falsified.

**One correction to the brief and to `spec.md`'s Open Questions.** Both say
the remedies "each move the scene contract a second time". Only one of them
does. `scene_contract` (`crates/mc-sim/src/replay/contract.rs:44`) takes
`&[SectionQuads]` and reads no camera; `SCENE_QUAD_COUNT`, `total_face_area`
and `area_by_block` are properties of the world's mesh alone. **Moving the
camera moves the goldens, which move once in this spec regardless. Moving the
world moves the scene contract.** That distinction decides the option below.

#### The options, against the drivers

| | What changes | Scene contract | Census | FR-2.5-S1's `4 095` grass / `1` stone | Existing derived probes |
|---|---|---|---|---|---|
| **A. Raise `SEA_LEVEL`** | the world | **moves** | **moves** | survives (grass is placed at the surface under the water, so its upward face is still emitted once water does not occlude) | survive |
| **B. Move the declared spawn** (`SPAWN_COLUMN`, `SPAWN_YAW_DEGREES`) | the camera path | unchanged | unchanged | unchanged | survive |
| **C. A second declared spawn plus a third capture set** | additive | unchanged | unchanged | unchanged | survive |

Option A's cost, measured over the height field (`min 32`, `max 48`, 4 096
columns):

| `SEA_LEVEL` | submerged columns | water voxels |
|---|---|---|
| 34 (today) | 131 | 178 |
| 35 | 321 | 499 |
| 36 | 590 | 1 089 |
| 37 | 932 | 2 021 |
| 38 | 1 535 | 3 556 |

The spawn column's own surface is 37, so nothing below 38 puts water at the
player's feet, and 38 submerges 37% of the world. A is rejected: it moves the
scene contract and the census that `requirements.md` recorded, it changes the
world a player plays for a test's benefit, and it is the one option that
cannot be checked against the existing area assertions because those
assertions' inputs move with it.

**Decision: B.** The declared spawn column and yaw are already declarations in
one file with doc comments stating what they are for, `terrain_probes.rs`
judges its own declared observation pose (`eye = [44, 56, 44]`, deliberately
"not a pose the player ever reaches") and is therefore untouched, and the
goldens are re-shot once either way. C is the fallback if B cannot be made to
hold water at all three declared ticks; it costs a parameterised `spawn`, a
parameterised capture-id prefix, a fourth golden directory and a second replay
scene to maintain, and it buys only the preservation of frames that are being
re-shot anyway.

#### What B has to satisfy, and that it can

The script turns `+1°` per tick for ticks 60–89, so the yaw at tick 119 is the
spawn yaw `+30°`. A candidate must therefore hold water in frame at both the
spawn yaw and yaw `+30`. Screened over every dry column with `x ≥ 30`
(`surface ≥ 34`, so the column is above the sea) at 10° of yaw — 73 620 poses,
position drift over the walk ignored, yaw arithmetic exact:

**4 715 poses hold water in frame at both ends of the turn.** Representative,
as `(water, sky, grass, landmark)` at the spawn yaw → at yaw + 30:

| column | yaw | at spawn yaw | after the turn |
|---|---|---|---|
| (63, 35) | 190° | 46 / 361 / 169 / 0 | 56 / 241 / 275 / 4 |
| (58, 33) | 160° | 45 / 402 / 129 / 0 | 44 / 285 / 247 / 0 |
| (63, 38) | 200° | 22 / 350 / 198 / 6 | 13 / 265 / 293 / 5 |

A separate sweep found **616 (dry column, yaw) pairs** that hold water, sky,
grass **and** the landmark pillar in one frame, so the property
`SPAWN_YAW_DEGREES` exists for is preservable rather than merely sacrificable
— e.g. (52, 41) at 190° gives 20 water / 9 landmark / 309 sky / 238 grass.

**This is a screen, not the final derivation.** The screen stands a player at
`surface + 3 + EYE_HEIGHT` and ignores the ≈3 blocks the walk covers and the
fall at the start. The implementation derives the pair from the real
simulation — `support::frames::player_pose` at each declared tick, which needs
no GPU — and records the derivation and the resulting per-tick water counts in
`test-map.md`, exactly as `oracle.rs`'s `SAMPLE_SPACING` doc comment requires
of a moved fixture: *"a grid quietly nudged until a suite went green is the
same defect as a threshold quietly lowered."* The pair is **derived, never
tuned to green**, and the derivation is what makes it a fixture rather than a
magic number.

> **Correction, measured during Phase 3 implementation. The screen above is
> wrong about the quantity it was built to decide, and the caveat in the
> paragraph before this one understates it by calling that imprecision.**
>
> All four representatives were run through the **real simulation** — seated,
> advanced under the declared script, camera read at ticks 0, 59 and 119, grid
> marched to the first drawn voxel:
>
> | candidate | water at t0 / t59 / t119 |
> |---|---|
> | (52, 41) at 190° | 0 / 0 / 0 |
> | (63, 35) at 190° | 0 / 0 / 0 |
> | (58, 33) at 160° | 0 / 0 / 0 |
> | (63, 38) at 200° | 0 / 0 / 1 |
>
> **Not one of them would have worked.** The mechanism is the one this section
> already names and then discounts: the screen stands the player at
> `surface + 3 + EYE_HEIGHT`, and the player *falls two blocks and then walks*,
> after which terrain rising above the sea hides it again. A line of sight to a
> strip of water 28–31 blocks away does not survive a two-block drop in eye
> height.
>
> **The screen was simultaneously accurate about terrain and wrong about
> water**, which is worse than being uniformly wrong: it predicted 300–500
> terrain samples and the real simulation gives 323–367, so anybody checking the
> screen against reality on the coarse quantity would have confirmed it and
> proceeded. Terrain fraction is a bulk property that survives a fall and a walk;
> water visibility is a narrow line of sight through a gap in rising terrain and
> does not.
>
> **So the instruction to derive from the real simulation was load-bearing, not
> ceremonial.** It is the only reason this spec has a working spawn, and the
> screen's numbers would have produced a confident wrong answer. The general form
> is now in `standards/global/testing.md` §2.
>
> **One thing in the screen was right and nearly lost**: its restriction to *dry*
> columns with `surface ≥ 34`. The implementation's own candidate filter omitted
> it, and the corrected ranking's top was consequently a column *below* sea
> level — a spawn in the water. Restoring the constraint gives the pair actually
> shipped: **`SPAWN_COLUMN = (63, 35)`, `SPAWN_YAW_DEGREES = 230.0`**, surface 34,
> water at **56 / 200 / 111** of 576 samples across the three declared ticks.

#### The scenario-wording question this left to the caller — **answered**

This document quoted FR-7.1-S1 as *"…show water in the captured frame at every
sample pixel where the judge predicts water, and at no fewer than one"* and asked
whether "the captured frame", singular, binds per declared terrain capture tick or
across the capture set. The strict reading — **per tick, all three** — was
recommended here on the grounds that a frame predicting no water witnesses
nothing, and the 4 715 surviving poses say it is satisfiable.

**The question is closed and the spec closed it, not this document.**
`spec.md:248` now reads *"…show water in **each** captured frame at every sample
pixel where the judge predicts water, and at no fewer than one sample pixel in
**every one of those frames**"* — the strict per-tick reading, stated twice and
unambiguously. The wording this document quoted is no longer the wording in
`spec.md`, and the recommendation and the requirement now agree. **The design
targets per tick; nothing here is left open.**

Recorded rather than deleted, so a reader meeting the singular phrasing in an
older copy can see what settled it. The consequence for the implementation is in
`tasks.md` T12, whose fallback ladder walks the declared capture *ticks* before
any second scene precisely because moving which frames are captured leaves this
reading strict.

### Open Question 2 — self-merging is an engine rule, and the boundary plane carries a key

**Answer: an engine rule, evaluated over block identity; and `Boundaries`
carries one `Key` per cell in the meshed section's own key table instead of one
`bool`.**

#### Why a rule and not a declared field

- **Invariant 1 forbids hardcoded block *definitions*, not general
  derivations.** "A block never draws a face against its own kind" names no
  block and no runtime id. `visible_face`'s own doc comment already claims the
  property this has to preserve: *"no block name and no runtime id is looked at
  anywhere in this file, which is what makes a block a mod ships behave exactly
  as one the base game ships does."* A `Key` comparison compares *identity*
  under a table deduplicated by name; it does not read a name. The property
  survives verbatim, and the base game gets no treatment a mod does not.
- **`merges_with_self = false` has one identified use and it is out of scope.**
  The value means "draw the interior faces of my own volume", which is a
  translucency want — per-pane glass — and translucency is PRO-952 by the
  spec's Out of Scope. `code-quality.md` §1 exempts a published extension API
  from the three-uses rule, but the discipline it substitutes is *"breadth of
  capability, narrowness of commitment"*: publishing a field whose only
  meaningful `false` belongs to a spec that has not decided its rendering model
  commits a name and a semantics that PRO-952 would then have to redefine.
  Deferring is the narrow commitment; shipping it now is the broad one.
- **Deferring costs no revision bump.** Verified rather than assumed:
  `docs/planning/block-render-methods.md:190-195` states that adding `render`
  to `DeclaredAppearance` bumps the appearance byte in PRO-952 and that this
  "costs a player nothing", because a changed appearance is neither reported
  nor refused. A `merges_with_self` field arriving with PRO-952 rides that bump.
- **Consequence for the folds:** an engine rule folds nothing, so
  `APPEARANCE_REVISION` moves to 3 for `drawn` and `occludes` and for nothing
  else. The spec's ruling table entry for self-merging is thereby vacated
  rather than contradicted.

**The strongest honest argument against this**, in the requirements' own words,
is that an engine rule is *"a derivation content cannot override"*. The day a
block wants its interior faces drawn, the rule has to become a field — a
change to the published declaration surface, not just to the engine. I accept
that and mitigate it by naming it: **the rule's site carries the note that
PRO-952 is the named breaker**, in the same shape as the oracle's
first-drawn-voxel assumption. Recorded now rather than reconstructed later.

#### What the boundary plane carries, and what it costs

Today (measured by reading the types):
`Boundaries { planes: [[bool; plane::CELLS]; BOUNDARIES] }` where
`plane::CELLS = 16 × 16 = 256` and `BOUNDARIES = 6` — **1 536 bytes**, filled
by `shared_face` from `solidity_at`, carrying no identity. So FR-2.3-S3 (two
cells in two different sections holding the same drawn non-occluding block)
cannot be evaluated at all, and the shipped sea spans sections, so the case is
reachable rather than hypothetical.

Three ways to widen it were considered:

| | Per section | Notes |
|---|---|---|
| A second plane of names, `[[Option<BlockName>; 256]; 6]` | ≥ 36 KiB plus 1 536 heap clones | Allocates in the meshing hot path and re-imports the very thing `solidity_at` was written to keep out ("a neighbour's block names never leave this call"). Rejected. |
| A bool plane **plus** a key plane | 1 536 + 3 072 = **4 608 bytes** | Works, but keeps two lookup paths and needs a second `occludes` answer for a neighbour block the meshed section does not hold. |
| **One key plane over a table shared with the meshed section** | **3 072 bytes** | Chosen. Fewer bytes than today's bool plane plus identity, one lookup path, and it reuses the dedup-by-name machinery that already exists. |

**Chosen mechanism.** `resolve_section` and `resolve_boundaries` share one key
table, in that order — the order `mesh_section` already calls them in
(`sweep.rs:75-76`). The meshed section's 4 096 voxels are keyed first, so
`MeshError::UnresolvedBlock` still outranks `UnresolvedNeighbourBlock` and
still names the lowest voxel in linear order. The six shared faces are then
keyed into the same table, appending any block the section did not itself hold.
`Boundaries` becomes `[[Key; plane::CELLS]; BOUNDARIES]`. Then:

> **Correction, made during Phase 2 implementation.** This paragraph originally
> justified the ordering with *"every key the meshed section's voxels hold is
> byte-identical to today"*. **That is false**, and it contradicts the seeding
> two bullets below: keying `Contents::Empty` at key 0 before any voxel is read
> shifts every key by one for a section holding no empty voxel, and grows
> `distinct_blocks()` by one. **The decision is unchanged and the seeding is
> safe — but for the measured reason, not that one: key values are
> unobservable.** Assumption 3 below is what carries it, and it was verified
> rather than assumed —
> `grep -rn "distinct_blocks" --include=*.rs crates/` finds the definition at
> `resolve.rs` and two uses in `sweep.rs`, both only as the `length` field of a
> `CorruptMeshIndex` that nothing asserts, and
> `grep -rn "Key" --include=*.rs crates/ | grep -v "/src/"` finds nothing
> at all. The ordering earns its keep on refusal precedence alone, which
> `a_section_and_a_neighbour_both_holding_something_unresolvable_refuse_by_the_sections_own`
> now asserts and a hand mutation of the order was observed to redden.
> **A future change that made a key value observable would turn this surplus
> claim into a real constraint**, which is why property (3) in
> `docs/technical/rendering.md` records it.

- `occludes(beyond)` — a table lookup on the boundary cell's key.
- "the same kind" — `key_of_self == key_beyond`, one `u16` compare, and it
  works identically inside a section and across a boundary because the table is
  one table deduplicated by name. The existing invariant *"the same key means
  the same block"* is what makes this correct, and it extends rather than bends.
- An **absent** neighbour must still be a value rather than a branch, which the
  current design achieves by leaving the plane all-`false`. The table is
  therefore **seeded with `Contents::Empty` at key 0**, so an absent
  neighbour's plane is `[0; 256]` — "nothing", which occludes nothing and is
  the same kind as nothing. FR-2.2-S4 falls out of that seeding, not out of a
  branch.

**Cost, derived.** 3 072 bytes per section being meshed, against 8 192 bytes
for `Resolved.keys` in the same frame — so the widened plane is still the
smaller of the two, and both are inline arrays rather than allocations.
Meshing runs on 16 `rayon` workers (`crates/mc-render/CLAUDE.md`), so the live
figure is 16 × 3 072 = 48 KiB against 16 × 1 536 = 24 KiB today.

**Cost in work, as designed.** Measured by reading
`Resolver::solidity_at` (`resolve.rs:~318`): it already calls `key_for`, which
already performs the dedup and returns exactly the key this design wants, and
then **throws the key away to return a bool**. Sharing the table consumes a
value that is already computed. Neither the vertex format, the section table,
nor the number of draw calls is touched, so `mc-render/CLAUDE.md`'s
one-indirect-draw rule is unaffected by the widening — what does move is the
quad count, because water now emits faces, and that is the change the spec is
for.

> **Correction, measured during Phase 2 implementation.** This paragraph
> originally claimed *"none added, and one lookup removed … per face the sweep
> gains one `u16` comparison and loses nothing"*, and that the `< 200 µs`
> per-section budget was unaffected. **The reasoning was sound and the quantity
> is false.** `crates/mc-world/benches/meshing.rs` — named above as the
> instrument if a number were wanted — was run, and a number was wanted:
>
> | fixture | before (`83b0baf`) | after (`281ff43`) | Δ |
> |---|---|---|---|
> | terrain | 139.4 µs | 160.2 µs | **+20.8 µs (+14.9 %)** |
> | solid | 164.5 µs | 183.9 µs | **+19.4 µs (+11.8 %)** |
> | checkerboard | 405.9 µs | 434.6 µs | +28.7 µs (+7.1 %) |
>
> Criterion point estimates, three rounds **interleaved** between two worktrees
> on one machine to cancel drift. `terrain` therefore spends roughly 21 µs of
> the 55 µs of margin it held under the declared 200 µs budget. **The budget is
> not breached and the decision is unchanged** — what is corrected is the
> claim that the change was free.
>
> **The mechanism was looked for and not found.** What it is *not*: the third
> clause, measured directly at **−0.5 to +4.5 µs across the three fixtures**,
> which straddles zero; and anything proportional to quad count, since
> `checkerboard` carries roughly eight times `terrain`'s quads and shows the
> same *absolute* delta. Plane initialisation and the extra struct moves were
> excluded by arithmetic rather than by experiment: 1 536 extra bytes of memset
> plus a few 8–11 KiB moves is ≈ 2 µs against a measured ≈ 20 µs.
>
> **The bench cannot narrow it further, and that is a property of the bench.**
> `visible_face` runs exactly 6 × 16 × 256 = 24 576 times per section for
> *every* fixture — a constant of section geometry, not of content — so a fixed
> per-section cost and a per-face-decision cost predict identical absolute
> deltas. ≈20 µs / 24 576 ≈ 0.8 ns, two or three cycles, is as consistent with
> the measurement as a fixed slab is. Distinguishing them needs an instrument
> that varies the call count, which no fixture here does.

**What a future change must not break.** Recorded so it is not reconstructed:
(1) the meshed section is keyed **before** its boundaries, or refusal
precedence and every existing key value move; (2) key 0 is `Contents::Empty`,
or an absent neighbour needs a branch again; (3) keys never reach the output —
they order nothing a caller sees; (4) only the 256 voxels of each shared face
are ever read, so a block a neighbour holds away from that face still never
reaches the registry and still cannot refuse a mesh it could not appear in.
`Key` is `u16` and the ceiling rises from 4 096 to 4 096 + 6 × 256 = 5 632
distinct blocks — *derived*, and still far inside `u16`, but the type's doc
comment says 4 096 today and must be corrected.

---

## Drivers

**Quality attributes that matter here, with the evidence.**

- **Evolvability of the declaration surface.** One bool answers four questions
  at seven production sites (`requirements.md` §1). The whole spec is the cost
  of that conflation coming due, and PRO-957, PRO-952 and PRO-944 each add
  fields to the same surface. Evidence: `visible_face` derives visibility from
  physics, so the one shipped non-solid block is invisible.
- **Falsifiability.** `drawn` defaults to `is_solid`, which makes all 34
  existing solidity fixtures pass by construction, and no count can see it
  (`testing.md` §2). This is the dominant risk in the spec and it is why the
  scenario set has FR-2.3, FR-2.4, FR-2.7, FR-6.2-S2 and FR-1.3 in it. It
  drives the design wherever a choice exists between a shape a test can see
  through and one it cannot.
- **Meshing latency.** Budget `< 200 µs` per section on 16 `rayon` workers,
  and terrain draws through one indirect draw call — a rule that binds whether
  or not this increment measures it (`crates/mc-render/CLAUDE.md`). The
  per-face decision and the boundary plane are both in that path.
- **Save compatibility, and the honesty of what a player is told.** Two folds
  with separate revision bytes exist precisely so a rendering change cannot
  claim every block behaves differently. Bumping the behaviour byte tells every
  player about all four shipped blocks, once.
- **Operability of the golden set.** The re-shoot procedure has a known
  corrupting failure mode (a run reaching `golden_mismatch` with the opt-in
  set) and is followed as written.

**Constraints.**

- Invariant 1 — zero hardcoded block behaviour in Rust; a default is chosen in
  the declaration loader as a documented default.
- Invariant 4 — the server is authoritative; targetability is recomputed
  server-side, never taken from a client.
- `Out of Scope` is binding: no transparency, no render methods, no light
  occlusion, no `ResolvedBlock` change, no rename of `solid`, no wire format.
- No sensitive data and no regulatory exposure anywhere in this change.

**What is volatile, and what is expensive to reverse.**

- Expensive: the published declaration field names (`drawn`, `occludes`,
  `targetable`) — a mod author writes them and a rename breaks every third-party
  block. The two revision bytes — each move is paid by every existing save.
- Cheap: the internal shapes (`Resolved`, `Boundaries`, the bitsets), which no
  content and no save sees.

---

## Boundaries

The design touches no network, no vendor and no new source of nondeterminism.
The table is not empty because two boundaries already exist and both are
crossed by this change.

| External dependency | Volatility (Vendor / Regulatory / API / Substitutability) | Port | Adapter location | Direct-use justification |
|---|---|---|---|---|
| The save file on disk (`redb` + `postcard` encoding) | none / none / low / low | already ported — the persistence layer *is* the port; `DeclaredBehaviour` / `DeclaredAppearance` are its written-out-by-hand contract, deliberately not derived from `BlockDefinition` | `crates/mc-world/src/persistence/format.rs` | — |
| Content declarations in `*.luau`, read through the sandboxed host | none / none / low / low | already ported — `DefinitionSource` (`mc_core::block::source`); the engine never learns whether a definition came from a file or a script | `crates/mc-world/src/content/luau_declaration/` | — |
| GPU / `wgpu` | — | not reached by this change | — | The mesher produces `Quad`s on `rayon` workers; nothing here touches a device. |
| Clock, random, filesystem | — | not reached | — | The mesher, the folds and the traces are pure functions of their inputs. |

**Nothing in this change introduces a dependency that needs a new port.** The
three new fields ride the two ports that already exist, and that is the reason
the change is as contained as it is. `architecture-principles.md` §3's litmus
test — "if this vendor disappeared tomorrow, how many files change?" — has no
new vendor to ask it of.

Two *internal* boundaries do move, and they are the design's substance:

- **`Solidity` is one question and becomes two.** `targeted` must stop asking
  "does this stop a player". See Decision 4.
- **The mesher's per-face predicate is one question and becomes three.** See
  Decision 3.

---

## Decisions

Every decision below is **BINDING** unless marked otherwise. Tasks and
implementation are handed this document and nothing else.

### 1. The three new fields are read by the existing optional-boolean reader, defaulting to the declaration's own `solid` — BINDING

`optional_boolean(declared, field, absent)`
(`crates/mc-world/src/content/luau_declaration/mod.rs:204`) already does
exactly this and already refuses a wrong kind rather than falling back to the
default, with the reason written in its doc comment. The three fields pass the
declaration's resolved `solid` as `absent`. Forced rather than chosen: the
spec fixes the defaults and the reader exists.

`RECOGNISED_FIELDS` grows from `[&str; 6]` to `[&str; 9]` in the order FR-1.2-S1
states — `name`, `texture`, `solid`, `replaceable`, `breakable`, `breaks_into`,
`drawn`, `occludes`, `targetable`. **Order is load-bearing**: a refusal quotes
the list back and `documented_refusals.rs` compares a real run against
`docs/modding/blocks-items.md:396` line for line. Three sites move together —
the constant, its hand-maintained mirror at
`crates/mc-world/tests/luau_declaration_keys.rs:60`, and the documented refusal.
`FIELD_NAMES_READ` is 64 and needs no change.

**Invariant 1 is satisfied here and only here.** Every default in this change is
a documented constant in the declaration loader. No engine module derives a
property from a block's name or id.

### 2. `BlockDefinition` gains three `bool` fields; the folds gain two and one — BINDING

`BlockDefinition` gains `drawn`, `occludes`, `targetable`, each with a doc
comment stating the one question it answers, in the shape the existing fields
already use. `is_solid`'s doc comment narrows to collision alone.

The folds, per the spec's ruling, with the mechanism:

- `DeclaredBehaviour` gains `targetable`; `BEHAVIOUR_REVISION` 1 → 2.
- `DeclaredAppearance` gains `drawn` and `occludes`; `APPEARANCE_REVISION`
  2 → 3.
- Both structs stay **written out by hand and never derived from
  `BlockDefinition`** — the existing doc comment's reason holds unchanged: a
  derive would bind every save to a struct that changes for other reasons.
- Field position in the `Serialize` struct is part of the record (`postcard`
  encodes positionally). The new fields are appended after the existing ones so
  the diff reads as an addition; a rename changes no byte and an insertion in
  the middle changes every one.
- `FR-5.1-S3` is the only witness that can see either byte move, because every
  other witness compares one fold to another. The format module's own doc
  comment records this as measured; the two hand-built byte-sequence guards are
  the test.

**Why not put `drawn` or `occludes` on the behaviour list:** it would tell every
player in existence that every block they built with behaves differently, on the
strength of a rendering field — the exact ambiguity the two bytes exist to
prevent. **Why `targetable` is behaviour:** it is what makes `breakable = false`
change what a break *does*.

### 3. The mesher's per-face predicate becomes three questions, and the boundary plane carries a key — BINDING

`visible_face` today is `solidity(self) && !solid_beyond(...)`. It becomes:

```
drawn(self) && !occludes(beyond) && key(self) != key(beyond)
```

- `Resolved.blocks` changes from `Vec<(Contents, bool)>` to
  `Vec<(Contents, Drawn, Occludes)>` — the mesher needs `drawn` of the voxel and
  `occludes` of the neighbour and needs solidity nowhere. `Resolved::is_solid`
  is read only by `sweep.rs` (measured by grep across `crates/mc-world/src/mesh/`),
  so it is replaced rather than added to.
- `Boundaries` becomes `[[Key; plane::CELLS]; BOUNDARIES]` over a table shared
  with the meshed section, resolved second, seeded with `Contents::Empty` at
  key 0. Mechanism, costs and the four properties a future change must not
  break are in Open Question 2 above.
- `Resolver::solidity_at` is deleted; `key_for` is called directly and its key
  kept.

**The third clause is the engine rule, and it is stated in the sweep and
nowhere else.** It compares identity, never a name, so `visible_face`'s
doc-comment property — that a mod's block is treated exactly as the base game's
— is preserved verbatim and the doc comment is updated to say *why* it is still
true of a key comparison.

**What defeats the default-equals-`solid` trap here** (`testing.md` §2, and the
spec names these as the countermeasures rather than leaving them to review):
FR-2.4's fixture, where `drawn`, `solid` and `occludes` are three different
answers and no two can be each other; FR-2.3, which states `solid` in every
scenario so an implementation ignoring `occludes` cannot satisfy one half;
FR-2.3-S4, two *different* drawn non-occluding blocks; FR-2.3-S3, the section
boundary that a bool plane cannot answer at all; and FR-2.7, which separates the
two questions a section answers.

### 4. Targetability is a second pre-resolved view, written at the single existing write site — BINDING

`targeted` (`crates/mc-sim/src/world/action/trace.rs:56`) takes
`&dyn Solidity` and stops at `world.is_solid(cell)`. FR-3.1 needs it to stop at
the first *targetable* cell, and FR-3.2 needs that to follow an edit rather than
only a load.

Options:

| | Shape | Verdict |
|---|---|---|
| Widen `Solidity::is_solid` to mean targetable | one bitset, one meaning changed | Rejected. `collide.rs` reads the same trait at five sites and means collision by it. Silently changing what one method means at nine call sites is the shape of a defect nothing can see. |
| One trait with two methods | `fn is_solid`, `fn is_targetable` | Rejected. `collide.rs` would gain access to a question it must never ask, and a collision test could exercise targetability by accident. |
| **A second narrow trait over the same type** | `Targetable { fn is_targetable(&self, at: BlockPos) -> bool }`, implemented by the same `SolidVoxels`; `targeted` takes `&dyn Targetable` | **Chosen.** Each consumer depends on the one question it asks; one type answers both; one place still writes both. |

`SolidVoxels` gains a second `Bitset`. **Both bits are written at the one site
that already writes solidity** — `World::write` and `World::adopt`
(`crates/mc-sim/src/world/mod.rs:249, 274`), whose doc comments say *"the one
place either view is written, and there is no other"* and *"the other place
either view is written, and there is no third"*. Those two sentences become
"either" → "any", and the property they name is what makes FR-3.2 hold: a view
built at load and never re-written would satisfy FR-3.1 entirely and FR-3.2 not
at all, and FR-3.2 exists because that failure is invisible from inside FR-3.1.

`SolidVoxels` now resolves two properties per voxel, so `LastResolved` caches
the pair rather than the bool — the run-coherence optimisation is unchanged in
shape and its argument (an `Arc`-backed name, consecutive voxels sharing one
allocation) is unchanged in substance.

**Cost, derived.** The shipped world is 64 × 64 × 256 = 1 048 576 voxels at one
bit each = **+128 KiB** for the second view, once, at world scale.

**The type's name.** `SolidVoxels` now answers two questions and should not be
named for one of them; the same argument the spec makes about `is_solid` itself.
Renamed to `ResolvedVoxels`, with the two traits carrying the capability names.
This is a rename inside `mc-sim` and reaches no content and no save.

### 5. `Section` answers "solid" and "drawn" as two methods — BINDING

`Section::is_solid_at` (`crates/mc-world/src/section/mod.rs:176`) gains a
sibling `is_drawn_at`, same shape, same empty-cell arm, same registry-refusal
contract. FR-2.7-S1 is the scenario, and the reason it exists is measured in the
spec: **the mesh fixtures and the mesher's own bench oracle both decide
drawnness today by asking a section whether a cell is solid** — six call sites
across `crates/mc-world/tests/` and `crates/mc-world/benches/`, plus
`crates/mc-sim/tests/support/oracle.rs:211`. An implementation that left that
one site answering a single question would keep oracle and subject agreeing
while both ignored `drawn`. The two methods are what make the oracle able to
disagree.

### 6. The reload's geometry check learns `drawn` and `occludes` and stays ignorant of `targetable` — BINDING

`drawn_of` (`crates/mc-sim/src/world/reload.rs:87`) keys today on
`(is_solid, &textures)` per name. It becomes `(drawn, occludes, &textures)` —
and **not** `targetable`, which changes no geometry. FR-7.2-S1 and S2 are the
two positives; **FR-7.2-S3 is the negative and it is the one that matters**: a
reload changing only `targetable` reports an accepted reload whose published
serial advances and whose rebuilt-section count is **zero**. An implementation
that folded all five fields into the geometry key would pass S1 and S2 and fail
only S3.

`solid` leaves this key. That is correct and worth stating because it looks like
a regression: solidity changes no geometry once drawnness is its own field, and
keeping it here would re-mesh the world for a physics edit.

### 7. The judge marches to the first `drawn` voxel and gains a positive control — BINDING (mechanism only; the ruling is the spec's)

`Voxels::is_solid` in `crates/mc-client/tests/support/oracle.rs` becomes
`is_drawn`, reading `resolve(name)?.drawn`. `first_solid_face` becomes
`first_drawn_face`; `marches_into_terrain` is unchanged in shape.

The mechanism that keeps the module's independence claim true — and the brief
is explicit that keeping it true is this stage's job:

- The claim is about **derivation**, not about which field is read. The header
  says so itself: the judge reads a declared field out of the registry and never
  the pre-resolved bitset the physics uses. Judge and mesher already read the
  same declared field today; moving both from `solid` to `drawn` changes nothing
  about that relationship.
- **`drawn` is not the mesher's decision.** After Decision 3 the mesher computes
  `drawn(self) && !occludes(beyond) && key(self) != key(beyond)` per face over a
  resolved key table and a boundary plane. The judge computes "nearest drawn
  surface along this ray" by its own hand-built DDA and **never looks at a
  neighbour at all**. Two questions, two implementations. The judge is not a
  second copy of the culling predicate, and the design keeps it that way by
  giving the judge no access to a boundary plane, a key table or an `occludes`
  answer.
- FR-6.2-S1 (every sample classified as exactly one of sky, grass, stone, dirt or
  water, at least one water) and FR-6.2-S2 (marching *through* a
  `drawn = false, solid = true` block, which no judge reading `solid` can pass)
  are the two directions. FR-6.2-S1's *enumerated* classification is what makes
  it more than an absence assertion — it rejects "I could not look" for free.
- The module header's "Water" paragraph — the coincidence that a ray passes
  through water because water is not solid, and the renderer draws the lakebed
  for the same reason — is **deleted and replaced** by the positive control and
  by the named PRO-952 breaker (a first-drawn-voxel march is right only while
  every drawn block is opaque).

### 8. The golden set re-shoots once, and the revision names what changed — BINDING

`SCENE_REVISION` `"r1"` → `"r2"`. `SCENE_QUAD_COUNT` moves; its new value is
minted **only after** the area assertions are green, per the constant's own doc
comment, and never edited to reach green. The four committed directories under
`crates/mc-render/goldens/` are deleted and four are added, so the commit reads
as a rename. `declared_capture_ids` is the authority on the set and
`crates/mc-render/tests/golden_inventory.rs` fails on a stale directory as much
as on a missing one — FR-6.1-S1 and S2.

The re-shoot follows `docs/technical/rendering.md` §"Re-shooting a golden set"
**as written**, in order: `terrain_probes`, then `replay_oracle`, then
`hud_prediction`, then mint through `terrain_goldens` and `hud_goldens` only,
then verify with the opt-in unset plus `golden_inventory`. The known corrupting
failure mode is a run that reaches `golden_mismatch` with the opt-in set; the
procedure's narrowness is what prevents it.

**One re-shoot, not two.** The spawn move (Decision 9) and the water quads land
in the same increment, and neither the spawn nor the capture set moves again
afterwards. PRO-957 re-shoots nothing.

### 9. The declared spawn moves to a derived coast column and yaw — BINDING

`SPAWN_COLUMN` and `SPAWN_YAW_DEGREES` (`crates/mc-sim/src/replay/spawn.rs:37, 49`)
are re-derived so that every declared terrain capture tick predicts water. The
derivation, the candidate screen and the recording obligation are in Open
Question 1. `SPAWN_ABOVE_SURFACE` is unchanged, so the spawn still falls and the
first frame still shows a fall.

Consequences to re-derive rather than discover, each already located:

- `replay_oracle.rs`'s `PREDICTION_FLOOR` (100 of 576) and
  `DISAGREEMENT_BUDGET` (2) — the screen's candidates predict 300–500 samples as
  terrain, so the floor holds with slack; the budget is **not** raised, per its
  own doc comment, and if a sample lands within a pixel of a silhouette the
  remedy is to move that sample and record why.
- The 3° downward-pitch control needs a terrain horizon in the perturbed frame.
  The candidates carry 240–400 sky samples, so a horizon is present; the control
  is re-run rather than assumed.
- `terrain_probes.rs` is **untouched** — it judges its own declared observation
  pose `eye = [44, 56, 44]`, deliberately not a pose the player reaches.
- `edit_geometry.rs`'s landmark constants are hand-placed world positions, not
  spawn-derived, and are untouched.
- The spawn's own doc comments state what the column and the yaw are *for*. Both
  sentences are rewritten: the yaw's stated purpose becomes the sea, and whether
  the landmark stays in frame is recorded as measured either way.

**The strongest argument against Decision 9** is that it changes where a player
starts in the shipped game to make a test's assertion reachable — a fixture
driving the product. Two things answer it and neither is a dismissal. First, the
alternative that does not touch the product (option C) preserves frames that are
being re-shot in this same commit, so it buys nothing durable for real
complexity. Second, the spec's own player capability is *"walk to the coast and
see the water surface"*; a spawn from which the sea is unreachable and invisible
is a fixture that contradicts the capability the spec exists to deliver. The
change makes the shipped game match its own spec. It is still a product change
made for a test's benefit, and it is recorded as one.

### 10. `merges_with_self` is not a field in this spec — DEFERRED

Revisit when PRO-952 decides its rendering model. The condition is concrete: the
day a block wants the interior faces of its own volume drawn. PRO-952 already
bumps `APPEARANCE_REVISION` for `render`, so the field costs no bump of its own.

### 11. Trivial, one line each

- `solid` keeps its name (the spec's ruling; nothing to design).
- `ResolvedBlock` gains nothing; the absence is documented and handed to
  PRO-944 (the spec's ruling; measured that `ClientContent::is_solid` has no
  production caller).
- FR-4.1's held block reads `solid`, unchanged, at
  `crates/mc-sim/src/world/action/mod.rs:348` — a held block is one you build
  with, and building means an obstacle.
- `collide.rs` keeps reading solidity, unchanged, at every one of its five
  sites.
- The shipped water declaration states `drawn = true`, `occludes = false`,
  `targetable = true`; dirt, grass and stone state none of the three and
  default to their `solid = true` (FR-1.3).

---

## Interfaces

Signatures the implementation must provide. Error contracts are the existing
ones unless stated.

```rust
// mc-core: what a block is
pub struct BlockDefinition {
    pub name: BlockName,
    pub textures: FaceTextures,
    pub is_solid: bool,   // collision, and nothing else
    pub replaceable: bool,
    pub breakable: bool,
    pub breaks_into: Option<BlockName>,
    pub drawn: bool,      // does the mesher emit a face for it
    pub occludes: bool,   // does it cull a neighbour's face toward it
    pub targetable: bool, // may a ray stop at it
    pub origin: DefinitionOrigin,
}

// mc-world: the declaration surface
const RECOGNISED_FIELDS: [&str; 9]; // declaration order, load-bearing
// three calls to the existing reader, `absent` = the declaration's own `solid`:
fn optional_boolean(declared: Option<ScriptValue>, field: &str, absent: bool)
    -> Result<bool, FieldFault>;
// refusals unchanged in shape: FieldFault::wrong_kind(field, &found, "true or false")
// and FieldFault::unrecognised(first, offenders), which quotes all nine.

// mc-world: what a section answers
impl Section {
    pub fn is_solid_at(&self, pos: LocalPos, registry: &BlockRegistry)
        -> Result<bool, SectionError>;
    pub fn is_drawn_at(&self, pos: LocalPos, registry: &BlockRegistry)
        -> Result<bool, SectionError>;   // new; Contents::Empty => false, as above
}

// mc-world: the mesher's resolved view
pub(super) struct Resolved { /* keys: [Key; VOXELS_PER_SECTION], blocks: Vec<(Contents, bool /*drawn*/, bool /*occludes*/)> */ }
impl Resolved {
    pub(super) fn key_at(&self, index: usize) -> Option<Key>;
    pub(super) fn is_drawn(&self, key: Key) -> Option<bool>;
    pub(super) fn occludes(&self, key: Key) -> Option<bool>;
    pub(super) fn contents(&self, key: Key) -> Option<Contents<&BlockName>>;
    pub(super) fn distinct_blocks(&self) -> usize;
}

pub(super) struct Boundaries { /* planes: [[Key; plane::CELLS]; BOUNDARIES] */ }
impl Boundaries {
    pub(super) fn key_at(&self, facing: Facing, cell: usize) -> Option<Key>;
}

// resolved together, meshed section first, sharing one key table:
pub(super) fn resolve_surroundings(
    section: &Section,
    neighbours: &Neighbours<'_>,
    registry: &BlockRegistry,
) -> Result<(Resolved, Boundaries), MeshError>;
// MeshError::UnresolvedBlock still outranks UnresolvedNeighbourBlock, and still
// names the lowest voxel in the meshed section's own linear order.

// mc-sim: the two questions a resolved world answers
pub trait Solidity    { fn is_solid(&self, at: BlockPos) -> bool; }      // unchanged
pub trait Targetable  { fn is_targetable(&self, at: BlockPos) -> bool; } // new
pub struct ResolvedVoxels { /* extent, solid: Bitset, targetable: Bitset */ }
impl Solidity for ResolvedVoxels { /* … */ }
impl Targetable for ResolvedVoxels { /* … */ }
pub fn targeted(origin: Vec3, direction: Vec3, reach: f32, world: &dyn Targetable)
    -> Option<Hit>;   // was &dyn Solidity

// mc-sim: the reload's geometry key
fn drawn_of(registry: &BlockRegistry)
    -> BTreeMap<&BlockName, (bool /*drawn*/, bool /*occludes*/, &FaceTextures)>;

// mc-world: the two folds
struct DeclaredBehaviour<'a> {
    input_version: u8, // BEHAVIOUR_REVISION = 2
    name: &'a str, is_solid: bool, replaceable: bool, breakable: bool,
    breaks_into: Option<&'a str>, targetable: bool,
}
struct DeclaredAppearance<'a> {
    input_version: u8, // APPEARANCE_REVISION = 3
    name: &'a str, textures: [&'a str; 6], drawn: bool, occludes: bool,
}

// mc-render: the golden set
pub const SCENE_REVISION: &str = "r2";
```

`mc-client/tests/support/oracle.rs`: `Voxels::is_solid` → `is_drawn`,
`first_solid_face` → `first_drawn_face`, both otherwise unchanged in signature
shape and error contract (`RegistryError` on a block the registry does not
register — reported, never read as "not drawn", because a shrinking prediction
is exactly what a one-sided comparison cannot see).

---

## Data

**No new persisted entity and no migration step.** What changes is the content
of two 64-bit folds, and the mechanism for that is a revision byte rather than a
migration:

| Fold | Revision | Fields | Effect on an existing save |
|---|---|---|---|
| `DeclaredBehaviour` | 1 → **2** | gains `targetable` | every block of every save reports **changed**. Survivable because PRO-956's load path names changed blocks instead of refusing. FR-5.2 asserts exactly what a player sees: changed = `base:dirt`, `base:grass`, `base:stone`, `base:water` in ascending order, missing empty, retextured empty, and one line on the error stream. |
| `DeclaredAppearance` | 2 → **3** | gains `drawn`, `occludes` | every block reports **retextured**, and **nobody is told** — the designed behaviour of the split and the reason the bytes are separate. FR-5.3 asserts a whole verdict for it, because "no changed block was named" is also what an `occludes` folded into *neither* list would produce. |

`postcard` encodes positionally, so a field **rename** changes no byte and a
field **addition** changes every one. The defaults chosen in FR-1.1 therefore
have no bearing on save compatibility — every block reports changed either way —
which usefully removes save compatibility from the list of things the default
choice has to serve.

**Retention: not applicable.** No personal data, no voice, no financial record;
a save is a local world file.

**In-memory data that grows** (all *derived*, none persisted):

| Structure | Today | After | Live multiplier |
|---|---|---|---|
| `Boundaries` | 1 536 B | 3 072 B | × 16 rayon workers |
| `ResolvedVoxels` | 128 KiB | 256 KiB | × 1, world scale |
| `Resolved.blocks` entry | `(Contents, bool)` | `(Contents, bool, bool)` | ≤ 5 632 entries, in practice single digits |

---

## Integration

Existing code touched, what connects, and what must not break.

| Site | What connects | Must not break |
|---|---|---|
| `mc-world/src/content/luau_declaration/mod.rs:64, 204, 272` | three new optional booleans; `RECOGNISED_FIELDS` → 9 | the quoted order; the refusal text, which `documented_refusals.rs` compares line-for-line against `docs/modding/blocks-items.md:396`; the mirror at `tests/luau_declaration_keys.rs:60` |
| `mc-core/src/block/definition.rs:50` | three fields; `is_solid`'s doc narrows | nothing derives a property from a name or an id |
| `mc-world/src/mesh/resolve.rs:53, 93, 140, 165, 318` | shared key table; `Boundaries` carries keys; `solidity_at` deleted | meshed section resolved first; refusal precedence and the lowest-voxel rule; only 256 voxels per shared face ever reach the registry; `Key`'s stated ceiling corrected to 5 632 |
| `mc-world/src/mesh/sweep.rs:275, 298, 317` | the three-clause predicate | no block name and no runtime id read anywhere in the file; an absent neighbour stays a value, not a branch |
| `mc-world/src/section/mod.rs:176` | `is_drawn_at` beside `is_solid_at` | the empty-cell arm short-circuits before the registry, for both |
| `mc-world/src/persistence/format.rs:275-365` | both folds, both bytes | hand-written, never derived; the origin stays excluded; field order is the record |
| `mc-sim/src/world/mod.rs:249, 274` | a second bitset written at both existing sites and nowhere else | "the one place either view is written" becomes "any view", and stays true — this is what FR-3.2 rests on |
| `mc-sim/src/replay/solid.rs:94, 146, 202` | second `Bitset`; `LastResolved` caches a pair; `Targetable` impl; renamed `ResolvedVoxels` | totality outside the extent; the two-arms-for-two-facts rule for empty vs outside |
| `mc-sim/src/world/action/trace.rs:56` | `targeted` takes `&dyn Targetable` | `Solidity` keeps its meaning at every site that reads it, and `trace.rs:56` is the only one that moves. Re-measured for the task list: `&dyn Solidity` appears at `player/collide.rs:101,125,222,250,259` (the five this row named), **`player/physics.rs:74`**, and **`world/clearing.rs:68,98,114`** — the nine that mean collision, matching the count Decision 4 already argued from. The three sites outside `collide.rs` were missing from this row and are as binding as the five |
| `mc-sim/src/world/action/mod.rs:348` | unchanged; reads `solid` | FR-4.1-S2 — a registry of only non-solid blocks offers no held block even where some are drawn |
| `mc-sim/src/world/reload.rs:87` | `(drawn, occludes, textures)` | `targetable` is **absent** from the key (FR-7.2-S3) |
| `mc-sim/src/replay/spawn.rs:37, 49` | derived coast column and yaw | the fall still happens; the derivation is recorded, not tuned |
| `mc-render/src/capture.rs:32` | `SCENE_REVISION` → `"r2"` | `declared_capture_ids` stays the single authority; the revision is a parameter everywhere and is never read from the constant inside |
| `mc-sim/src/replay/contract.rs:28` | `SCENE_QUAD_COUNT` minted after the areas are green | never edited to reach green |
| `mc-client/tests/support/oracle.rs:41-47, 132` | marches on `drawn`; header rewritten | the judge never sees a boundary plane, a key table or an `occludes` answer |
| `mc-sim/tests/shipped_water_is_not_broken_and_is_built_through.rs:138` | the inherited fuse goes red | FR-3.4 is what replaces it; the fuse's header text is rewritten, not deleted silently |
| `content/base/blocks/water.luau` | states the three fields | the other three declarations state none of them |

**Documentation owed, per Key Principle 3** — part of this spec's definition of
done, not a follow-up. The spec enumerates it and the enumeration is binding:
three now-false sentences on `docs/user/gameplay.md:60-72`; the mod-author page's
field table, defaults, refusals and a worked example that runs (a block drawn
without being solid); `docs/technical/architecture.md:830-841` plus
`Refusal::Indestructible`'s doc comment, `targeted`'s, and the fuse test's header
— four sites describing a fuse this spec blows; `docs/technical/world-format.md`
for the two revisions; `docs/technical/rendering.md` for the re-shoot, the
revision and the moved spawn. **This document adds one item to that list:** the
engine rule "a block never draws a face against its own kind", its evaluation over
key identity, the widened boundary plane, and PRO-952 as its named breaker,
belong in `docs/technical/rendering.md` and in the sweep's own doc comment — a
mod author needs to know their two adjacent identical non-occluding blocks show
no seam and cannot currently ask for one.

---

## Assumptions

Listed so a reviewer can veto them. Each is low-impact or measured; none stands
in place of a driver a binding decision needed.

1. **Every drawn block is opaque.** The judge's first-drawn-voxel march depends
   on it. PRO-952 is the named breaker and the note goes in the module header.
   The spec's assumption, carried unchanged.
2. **The candidate-spawn screen's approximation is safe.** It stands the player
   at `surface + 3 + EYE_HEIGHT` and ignores the ≈3 blocks the walk covers and
   the initial fall, while treating the yaw arithmetic exactly. 4 715 surviving
   poses is a wide enough margin that the approximation does not decide the
   outcome, but the final pair is derived from the real simulation and the
   per-tick counts are recorded. **If the derivation finds no pair holding water
   at all three declared ticks, fall back to Decision 1's option C rather than
   loosening FR-7.1-S1.**
3. **Nothing observable depends on the numeric value of a mesher `Key`.** Read
   from the module's own doc comment ("nothing ordered by palette order, palette
   length, index width, reference count or runtime id reaches the output") and
   from `distinct_blocks` being used only as the `length` of a
   `CorruptMeshIndex` that cannot occur. Seeding key 0 with `Contents::Empty`
   rests on this. **Falsified by** any test asserting a key value or a
   `distinct_blocks` count; the implementation greps for both before seeding.
4. **`Targetable` has exactly one implementor for the life of this spec.** No
   port is built for it, per `architecture-principles.md` §3 — it crosses no
   process, no vendor and no nondeterminism, and §3's second rule is to define a
   port when the first adapter is built, not before.
5. **`FIELD_NAMES_READ = 64` needs no change for nine fields**, and none for the
   eleven PRO-957 brings.

---

## Risks

What could go wrong, what to verify early, and where the blast radius is.

- **The default-equals-`solid` trap is the dominant risk and it is invisible to
  counts.** `drawn = is_solid` makes all 34 existing solidity fixtures pass by
  construction. Verify early, before any of FR-2's positive scenarios: FR-2.4's
  three-way fixture and FR-2.7's two-method section are the two that cannot pass
  vacuously. A phase that reaches green on FR-2.1 and FR-2.2 alone has proved
  nothing.
- **A green suite is no evidence about a lint** (`testing.md` §2). Decision 3
  and Decision 4 both change struct shapes across crate boundaries, so a phase
  opening with an adaptation commit has no compilable tree for the gate to run
  on. Whoever authors tests in that window runs
  `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  directly.
- **The golden re-shoot has a known corrupting failure mode.** A run reaching
  `golden_mismatch` with `MYCRAFT_UPDATE_GOLDENS` set writes a tick-59 frame as
  tick 0's ground truth. The procedure is followed as written and the mint step
  names two binaries, not a wider selector.
- **Minting before the probes and the oracle are green photographs a broken
  renderer, permanently.** `terrain_goldens.rs`'s own header calls the ordering
  binding. The spawn move means the oracle's floor and its 3° control are
  re-derived *before* the mint, not after.
- **Two revision bytes moving in one commit.** Only FR-5.1-S3's hand-built byte
  sequences can see either move; every other witness compares one fold to
  another and cannot see a leading byte that moved in both. If FR-5.1-S3 is
  written loosely, a byte that did not move is indistinguishable from one that
  did.
- **`SCENE_QUAD_COUNT` is change detection and verifies nothing.** Its new value
  is minted after FR-2.6's independent per-voxel walk agrees on the areas.
  Minting it first, then deriving the walk to match, would invert the only
  ordering that makes either meaningful.
- **A shared key table changes refusal precedence if resolved in the wrong
  order.** Verify with the existing `UnresolvedBlock` /
  `UnresolvedNeighbourBlock` scenarios before anything else in Decision 3 is
  built — they are the cheapest instrument that can see it.
- **`targeted` moving to a second bitset is policy that needs wiring**
  (`testing.md` §2). FR-3.2 exists because a targetability view built at load
  and never re-written satisfies FR-3.1 entirely. Ask of the second bitset what
  calls it and what would go red if the write site stopped setting it — the
  answer must be FR-3.2-S1 and S2, and nothing else in the suite.
- **The spawn move is the widest-radius change in the spec** and it is not
  behind a test's own fixture — it changes the shipped game. Everything that
  reads the spawn is enumerated in Integration; the risk is something not in
  that list. Verify by running the whole `mc-client` and `mc-sim` suites
  immediately after the two constants change and before anything else in
  Decision 9, so a surprise arrives attributable.
- **Vendor-failure blast radius: none.** No vendor is reached. The only
  cross-process surface this change crosses is the save file, and the revision
  bytes are the mechanism that makes crossing it safe.

---

## Questions for the caller — **both answered; kept with their answers**

Neither blocked the design; both changed a scenario's wording rather than a
decision, and both are now settled in `spec.md`. Kept visible rather than deleted
so a reader meeting either question can see what closed it.

1. **Does FR-7.1-S1 bind per declared terrain capture tick, or across the
   capture set?** → **Per tick, all three.** Asked here because the wording this
   document quoted said "the captured frame", singular. `spec.md:248` now says
   *"in **each** captured frame … and at no fewer than one sample pixel in
   **every one of those frames**"*, which is the strict reading stated twice.
   Recommendation and requirement agree; the spawn derivation does **not** relax.
   See the answered subsection under Open Question 1.
2. **`spec.md`'s Open Questions and the brief both say the camera remedy "moves
   the scene contract a second time".** → **Corrected in `spec.md`.** It does not
   — `scene_contract` (`crates/mc-sim/src/replay/contract.rs:44`) takes
   `&[SectionQuads]` and reads no camera. `spec.md` Open Question 1 now carries
   the correction in its own words (*"The two remedies do not both move the scene
   contract"*), so no later stage re-derives the wrong cost.
