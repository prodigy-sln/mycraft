# Phase 3 test-author brief — PRO-904, "One bit answers four questions"

**Written by the Phase 3 implementer. Assume you remember nothing of any earlier
conversation; everything you need is here or is named by path below.**

Spec: `SPEC-021`, rigor `high`. Branch `feature/PRO-904-solid-split-properties`.
Repository root `E:\_PROJEKTE\MyCraft`. HEAD when this was written: `bec35db`
(`refactor: split the re-mesh fixture by responsibility`), working tree clean,
gate green — 1410 tests run, 1410 passed, coverage 93.63 % / 92.13 %.

---

## 0. What you are and what you are not

You are the **test author** for Phase 3. You write the phase's failing tests
from the spec's scenarios, you **own every test file for the whole phase**, and
you arbitrate disputed failures. The implementation context never edits a test
file; when it disagrees with one of yours it sends you the failure and you
answer with exactly one of `test-correct` (the implementation conforms and must
change), `test-wrong` (you fix and commit the test), or `scenario-ambiguous`
(the user decides).

**You do not write implementation.** In particular you do not touch:

| Path | Why it is the implementer's |
|---|---|
| `content/base/blocks/*.luau` | T09 — the shipped declaration |
| `crates/mc-sim/src/replay/contract.rs` | `SCENE_QUAD_COUNT` is minted at T10, after your area assertions are green |
| `crates/mc-sim/src/replay/spawn.rs` | `SPAWN_COLUMN`, `SPAWN_YAW_DEGREES` — T12 |
| `crates/mc-render/src/capture.rs` | `SCENE_REVISION`, `DECLARED_CAPTURE_TICKS` — T13 |
| `crates/mc-render/goldens/**` | T13 — the re-shoot |
| `docs/**` | T14 |
| anything else under `crates/*/src/**` or `crates/*/benches/**` | implementation |

You **do** own, outright, every file under `crates/*/tests/**`, including the two
support modules this phase turns inside out:

- `crates/mc-client/tests/support/oracle.rs` — the ray-marched frame judge
- `crates/mc-sim/tests/support/oracle.rs` — the independent per-voxel walk

and everything that consumes them.

---

## 1. What Phase 3 is

Phase 3 makes the shipped game draw its sea, and re-shoots the golden set
exactly once. It closes **12 scenarios**:

`FR-1.3-S1`, `FR-1.3-S2`, `FR-2.5-S1`, `FR-2.6-S1`, `FR-2.6-S2`, `FR-2.6-S3`,
`FR-6.1-S1`, `FR-6.1-S2`, `FR-6.2-S1`, `FR-6.2-S2`, `FR-6.2-S3`, `FR-7.1-S1`.

The implementation tasks, in order, are:

- **T09** — `content/base/blocks/water.luau` declares `drawn = true`,
  `occludes = false`, `targetable = true`; dirt, grass and stone declare none of
  the three and keep defaulting to their `solid = true`.
- **T10** — the independent per-voxel walk asks `is_drawn_at`, and the shipped
  world's per-block face areas and quad count are derived from it.
- **T11** — the ray-marched judge marches to the first **drawn** voxel.
- **T12** — `SPAWN_COLUMN` and `SPAWN_YAW_DEGREES` are re-derived from the real
  simulation so that water is in frame at every declared capture tick.
- **T13** — `SCENE_REVISION` `"r1"` → `"r2"`; four golden directories deleted,
  four minted.
- **T14** — documentation.

**The gate is RED from T09 until T13, by construction and unavoidably.** This is
the gate-atomic unit of the spec. Do not treat a broad red tree in the middle of
this phase as a defect signal on its own, and do not weaken anything to keep it
green. Green at phase end.

---

## 2. Read these from disk before you write a line

1. `CLAUDE.md` (repository root) — invariants, key principles.
2. `standards/global/testing.md` — especially §1 (what counts as RED) and §2
   (falsifiability). It is the constitution for what you are about to do.
3. `standards/global/code-quality.md`, `standards/global/git-workflow.md`.
4. `specs/active/2026-08-22-solid-split-properties/spec.md` — FR-1.3, FR-2.5,
   FR-2.6, FR-6, FR-7.1, and the "Technical Considerations" section on the
   render oracle's independence.
5. `specs/active/2026-08-22-solid-split-properties/architecture.md` — **Open
   Question 1** (the measured impossibility and the spawn move), **Open Question
   2** (the engine's three-question face predicate and the key plane),
   **Decision 7** (the judge), **Decision 8** (the re-shoot), **Decision 9** (the
   spawn).
6. `specs/active/2026-08-22-solid-split-properties/tasks.md`, the section headed
   `## Phase 3: The shipped game draws its sea, and the golden set re-shoots
   once`.
7. `specs/active/2026-08-22-solid-split-properties/test-map.md` — where your
   mapping and your measurements go. Read the Phase 1 and Phase 2 sections for
   the shape expected of you.
8. `docs/technical/testing.md` — test placement rules and gate stages.
9. `docs/technical/rendering.md` § "Re-shooting a golden set" (line 1220 onward)
   — you are not performing it, but FR-6.1 is about its guard rails.

---

## 3. Verified facts about the tree, with their locations

Every value below was read from disk at `bec35db`. If one disagrees with what
you find, **the tree wins and you tell the implementer**.

### The shipped content

`content/base/blocks/water.luau` today, verbatim in its declaration table:

```lua
return {
	name = "base:water",
	texture = "base:water",
	solid = false,
	breakable = false,
	replaceable = true,
}
```

Its header currently explains `solid = false` as what *"keeps the mesher from
culling against it"*. **T09 falsifies that sentence and rewrites it.**
`dirt.luau`, `grass.luau` and `stone.luau` state `solid = true` and nothing
about the three new fields.

### The engine, after Phase 2

- `crates/mc-core/src/block/definition.rs:84` `pub drawn: bool`, `:91`
  `pub occludes: bool`, `:99` `pub targetable: bool`. Each defaults, when a
  declaration is silent, to whatever that declaration says about `is_solid`.
- `crates/mc-world/src/section/mod.rs:176` `is_solid_at`, `:210` `is_drawn_at` —
  two separate questions on `Section`, already shipped.
- `crates/mc-world/src/mesh/sweep.rs`, `visible_face` — the face predicate is
  now **three questions**: `drawn(self) && !occludes(beyond) && beyond != key`,
  computed over a resolved key table and a boundary plane carrying one `Key` per
  cell. An absent neighbour resolves to the key of `Contents::Empty` (key 0),
  which occludes nothing and is the same kind as nothing.
- **Not yet moved, and not this phase's business:** the save folds
  (`crates/mc-world/src/persistence/format.rs:275-276`, `BEHAVIOUR_REVISION = 1`,
  `APPEARANCE_REVISION = 2`) are Phase 5's, and the reload geometry check
  (`crates/mc-sim/src/world/reload.rs`, `drawn_of`, still reading
  `declared.is_solid`) is Phase 5's. Targetability wiring is Phase 4's — so
  water declaring `targetable = true` at T09 changes nothing a player can aim at
  until Phase 4 lands.

### Constants this phase moves or must not move

| Constant | Verified value | Location | Whose |
|---|---|---|---|
| `SCENE_QUAD_COUNT` | `2759` | `crates/mc-sim/src/replay/contract.rs:28` | implementer, minted at T10 |
| `SCENE_REVISION` | `"r1"` | `crates/mc-render/src/capture.rs:32` | implementer, T13 |
| `DECLARED_CAPTURE_TICKS` | `[0, 59, 119]` | `crates/mc-render/src/capture.rs` | implementer |
| `HUD_CAPTURE_TICKS` | `[0]` | `crates/mc-render/src/capture.rs` | implementer |
| `SPAWN_COLUMN` | `(32, 32)` | `crates/mc-sim/src/replay/spawn.rs:38` | implementer, T12 |
| `SPAWN_ABOVE_SURFACE` | `3` | `crates/mc-sim/src/replay/spawn.rs:45` | **unchanged** |
| `SPAWN_YAW_DEGREES` | `225.0` | `crates/mc-sim/src/replay/spawn.rs:49` | implementer, T12 |
| `GRASS_UPWARD` | `4095` | `crates/mc-sim/tests/scene_contract.rs:38` | **yours** |
| `STONE_UPWARD` | `1` | `crates/mc-sim/tests/scene_contract.rs:42` | **yours** |
| `DOWNWARD` | `4096` | `crates/mc-sim/tests/scene_contract.rs:47` | **yours** |
| `OPENING` / `WALKED` / `CLOSING` | `0` / `59` / `119` | `crates/mc-client/tests/support/goldens.rs:37-39` | **yours** |
| `DECLARED_TICKS` | `[0, 59, 119]` | `crates/mc-client/tests/support/goldens.rs:42` | **yours** |
| `SECOND_REVISION` | `"r2"` | `crates/mc-render/tests/golden_inventory.rs:52` | **yours — see §6, trap 1** |
| `DISAGREEMENT_BUDGET` | `2` | `crates/mc-client/tests/replay_oracle.rs:79` | **yours, and it is not raised** |
| `PREDICTION_FLOOR` | `100` | `crates/mc-client/tests/replay_oracle.rs:87` | **yours, re-derived not loosened** |
| `JUDGED_TICKS` | `[0, 59, 119]` | `crates/mc-client/tests/replay_oracle.rs:67` | **yours** |
| `CONTROL_PITCH_DEGREES` | `3.0` | `crates/mc-client/tests/replay_oracle.rs:90` | **yours** |
| `SAMPLE_COLUMNS` / `SAMPLE_ROWS` | `32` / `18` | `crates/mc-client/tests/support/oracle.rs:65-66` | **yours** |
| `SAMPLE_SPACING` / `SAMPLE_ORIGIN` | `40` / `20` | `crates/mc-client/tests/support/oracle.rs:78-79` | **yours** |
| `SAMPLE_COUNT` | `576` (derived) | `crates/mc-client/tests/support/oracle.rs:82` | **yours** |

The committed golden set today, verified by `ls crates/mc-render/goldens/`:
`player-walk-hud-t000-r1`, `player-walk-t000-r1`, `player-walk-t059-r1`,
`player-walk-t119-r1`. Each directory holds `default.png` and
`default.provenance.json`.

### The world, as measured in `architecture.md` and confirmed there against
`requirements.md`

- The replay world is `64 × 64` columns (`FOOTPRINT = 64`), surface band
  `[32, 48]`, sea level `34`, landmark stone pillar at column `(12, 12)` up to
  `y = 64`.
- **178 water voxels** in **131 columns**: 84 columns one deep, 47 two deep,
  `y ∈ [33, 34]`, **every top open to air**. So there are **131 water surface
  voxels**, at `x ∈ [60, 63]`, `z ∈ [0, 34]` — a strip on the far `+x` edge.
- **Zero quads name water today.**
- From the declared spawn `(32.5, 41.62, 32.5)` facing `225°`: **0 water pixels
  at all three declared capture ticks**; **0 unoccluded lines of sight to any of
  the 131 water surface voxels at every one of the 120 ticks**; water visible at
  **no yaw at all** from that column.

These are input to your reasoning. **They are not values to copy into an
assertion** — see §5.

---

## 4. Scenario by scenario

The scenario↔test mapping is a **floor, not a ceiling**. Add any test that would
catch something real, and record it in `test-map.md` under the scenario it
strengthens or under an "additional coverage" heading with one line stating what
it catches.

### FR-1.3-S1 — the shipped water declaration

> WHEN the shipped content root is read THE SYSTEM SHALL read `base:water` as
> not solid, drawn, non-occluding, targetable, unbreakable and replaceable

### FR-1.3-S2 — the other three

> WHEN the shipped content root is read THE SYSTEM SHALL read `base:dirt`,
> `base:grass` and `base:stone` as solid, drawn, occluding and targetable

Both read the **real shipped content root** (`content/base/`), never a fixture —
that is the whole point of FR-1.3: without it, all of FR-2 through FR-4 is
satisfiable by synthetic fixtures while the shipped game is unchanged.

The natural home is `crates/mc-client/tests/shipped_blocks_are_declared_in_luau.rs`,
which already reads the four shipped blocks through `prepare_launch` and asserts
each block's `name`, declaring **file**, `solid` and `replaceable` as one
comparison against a hand-written `SHIPPED: [Declared; 4]` table
(`:40-66`). Extending that table with the three new fields keeps the
"which file declared it" discipline that file exists for. If you would rather
write a new file, say why in `test-map.md`.

**Trap.** Assert all six facts of S1 and all four of S2 as **one comparison per
block against a hand-written expectation**, not as six separate `assert!`s and
not by filtering. A per-field assertion cannot see a field that stopped being
read at all, and a filtered comparison cannot see an extra member — see §6,
trap 3.

RED here is immediate and arrives before T09: water is not drawn today.

### FR-2.5-S1 — the world's existing surfaces are unmoved

> WHEN the shipped replay world is meshed with water drawn and non-occluding THE
> SYSTEM SHALL report the same upward face area per block as before the split —
> 4 095 of grass and 1 of stone

Home: `crates/mc-sim/tests/scene_contract.rs`,
`the_surface_shows_one_upward_face_per_column_and_the_landmark_caps_exactly_one`.

Today that test asserts the whole upward map equals
`{base:grass: 4095, base:stone: 1}`. Once water is drawn, water's own upward
faces join that map and the assertion **fails as a map inequality** — which is
correct RED and is exactly why it is asserted as a whole map rather than by
lookup.

**Do not copy `4 095` and `1` from the spec. Re-derive them**, and derive water's
entry too: the arithmetic already in that file's header is one upward face per
column of the 64 × 64 footprint less the landmark's stone cap. Water's upward
area is derivable the same way — from the census, not from a run. Write the
derivation in the constants' doc comments the way the existing three do.

### FR-2.6-S1 — water is drawn at all

> WHEN the shipped replay world is meshed THE SYSTEM SHALL report a non-zero
> meshed area for `base:water`

This is the positive control that makes S2 and S3 non-vacuous, and it **inverts
an existing test**: `crates/mc-sim/tests/replay_world.rs`,
`no_quad_of_the_meshed_replay_names_the_block_that_fills_its_sea` (`:186`),
which today asserts `watery == 0`. That test is yours to replace. **Its own
positive control survives and must be kept** — it asserts, before the quad
assertion, that some column of the world is below sea level, so that "no quad
names water" cannot be satisfied by a world with no water in it. The inverted
test needs the same guard for the same reason.

### FR-2.6-S2 — every block's area agrees with an independent walk

> WHEN the shipped replay world is meshed THE SYSTEM SHALL report, for every
> block including `base:water`, the area an independent per-voxel walk of the
> same world computes

Home: `crates/mc-sim/tests/scene_contract.rs`,
`every_blocks_meshed_area_equals_the_independent_walks_area_for_that_block`, over
`crates/mc-sim/tests/support/oracle.rs`.

**This is the largest single piece of work in the phase and the one with the
sharpest vacuity risk.** The walk today decides a face by

```
self.registry.resolve(name)?.is_solid          // is this voxel solid
&& !is_solid_at(neighbour)                     // is the neighbour not solid
```

at `add_visible_sides_of` and `is_solid_at`. The mesher's predicate is now three
questions, not two. **The walk must answer the same three, written from the
spec's own wording and never copied from `visible_face`:**

1. **Is this block drawn?** — `resolve(name)?.drawn`, replacing the `is_solid`
   read in `add_visible_sides_of`.
2. **Does whatever is beyond it fail to occlude?** — `resolve(name)?.occludes`,
   replacing `is_solid_at`. Outside the world, and a cell holding nothing, both
   occlude nothing — unchanged in shape from today's two arms.
3. **Is whatever is beyond it a different block?** — the engine rule that a block
   never draws a face against its own kind. **The walk has no key table and must
   not acquire one**: compare the two cells' block *names*. A cell holding
   nothing is not the same kind as any block; a cell outside the world is the
   same kind as nothing. Files under `tests/` are outside `mc-world`'s
   hardcoded-name scan and this module's own header already says block names
   appear in it in full, so comparing names here is legitimate and is what keeps
   the walk from being a second copy of the key comparison.

Without clause 3 the walk will disagree with the mesher over the sea by a large
margin, and the failure will look like a mesher defect. Without clause 2 written
as `occludes` the walk agrees with the mesher for the wrong reason.

**The vacuity trap, stated plainly.** Before T09, water declares
`drawn = false, occludes = false`, so a walk asking `drawn`/`occludes` and a
mesher asking the same produce identical answers to today's, and this equality
stays **green through your whole change**. That green is not evidence. What
makes the pair meaningful is FR-2.6-S1's non-zero water area (the control) plus
the fact that clause 3 is a genuine reimplementation the mesher does not share.
Say this in `test-map.md`; a reviewer meeting a green equality otherwise cannot
tell it from a working one.

Keep the module's existing header claims true, and update them: it shares no
code with `mc_world::mesh` beyond the `Facing` enum, it re-derives adjacency from
its own six signed offsets, and its `Side` → `Facing` translation is one
hand-written match. The paragraph headed "**Area, not quad count.**" stays.

### FR-2.6-S3 — the quad count is derived, not snapshotted

> WHEN the shipped replay world is meshed THE SYSTEM SHALL report a quad count
> derived from an independent walk of the same world rather than one snapshotted
> from a run of the mesher

Home: `crates/mc-sim/tests/scene_contract.rs`,
`the_meshed_quad_count_matches_the_committed_scene_contract_snapshot`.

**The ordering here is binding and is the whole point of the task.**
`SCENE_QUAD_COUNT` is minted by the implementer **only after your area
assertions are green**, per the constant's own doc comment
(`crates/mc-sim/src/replay/contract.rs:20-27`), and it is **never edited to reach
green**. Minting it first and then deriving the walk to match inverts the only
ordering that makes either meaningful.

So: while you are working, this test will be **red on the stale `2759`**, and
that red is expected and correct. Keep its failure message — it names the
remedy (bump `SCENE_REVISION`, delete the previous revision's directories,
re-shoot, justify in the commit) and that message is what stops somebody simply
editing the number.

**Red for a known reason hides red for an unknown one** (`testing.md` §2). This
test being red on a stale count is exactly the state in which a *second* defect
in the same test goes unnoticed. It is fixed — by the implementer minting the
new value — before the phase closes, never annotated and left.

### FR-6.1-S1 — the committed set is exactly what the revision declares

> THE SYSTEM SHALL hold, under the committed golden root, exactly the capture
> directories the current scene revision declares and no directory of any other
> revision

### FR-6.1-S2 — a declared capture with no golden fails naming the path

> IF the scene revision names a capture for which no golden is committed THEN THE
> SYSTEM SHALL fail naming the missing path

Both are already asserted, in `crates/mc-render/tests/golden_inventory.rs`:
`the_committed_goldens_are_exactly_the_directories_the_current_revision_declares`
and `a_revision_whose_goldens_were_never_captured_fails_naming_the_path_it_looked_for`.

**Decide whether those existing tests close the two scenarios as written, and
record the measurement in `test-map.md`** — Phase 1 did exactly this for
FR-1.2-S2, and there is a heading in `test-map.md` showing the shape expected.
"An existing test already covers it" is an acceptable answer *only* with the
measurement attached; assert it and say what would redden.

**See §6 trap 1 — `SECOND_REVISION` in that file collides with the new
`SCENE_REVISION` and must move. That is yours.**

### FR-6.2-S1 — the judge classifies every sample, and predicts water

> WHEN the judge marches from the player's camera through the shipped replay THE
> SYSTEM SHALL classify every declared sample pixel as exactly one of sky,
> `base:grass`, `base:stone`, `base:dirt` or `base:water`, with at least one
> classified `base:water`

This is a **new capability** in `crates/mc-client/tests/support/oracle.rs`. Today
the judge answers a single boolean per sample (`marches_into_terrain`) and
`predicted_terrain` returns the sub-list it called terrain. FR-6.2-S1 needs an
**enumerated verdict per sample** — five arms, no sixth, no "unknown", no
`Option`.

**An enumerated verdict beats an absence assertion** (`testing.md` §2 and the
project's own memory of it): a total enum rejects every other answer *including*
"I could not look", so a vanished world or a collapsed march reddens for free.
Assert the whole classification of all 576 samples — the count per class summing
to `SAMPLE_COUNT`, and at least one `base:water` — rather than filtering for the
class you care about.

`sky` is a **prediction**, meaning the march met no drawn voxel inside
`MARCH_LIMIT`. It is not a reading of the frame. Nothing in FR-6.2-S1 touches a
rendered image, which is what lets it run with no GPU.

**FR-6.2-S1 is unsatisfiable until T12 and that is the point.** With the spawn
where it is today the judge sees 0 water at every declared tick, measured.
**If you see this go green before T12, that is a defect in your test, not
progress** — it has measured something other than what the scenario says.

### FR-6.2-S2 — the judge marches *through* a drawn-false, solid-true block

> WHEN the judge marches through a world holding a block declaring
> `drawn = false, solid = true` THE SYSTEM SHALL pass through that block rather
> than predicting it

**No judge reading `solid` can pass this**, which is what makes it the phase's
sharpest single assertion. It fails as an assertion — not a compile error —
against the current judge, immediately, before T09 or T11.

Mechanism note, verified: `mc_sim::replay::ReplayWorld` has exactly one
constructor, `ReplayWorld::generate(seed, &BlockRegistry)`
(`crates/mc-sim/src/replay/world.rs:95`); there is no hand-placement API. So the
"world holding a `drawn = false, solid = true` block" is built by generating a
replay world against a **registry read from a content root in which one shipped
block restates `drawn = false`** — `support::reload::restating(root, file_name,
&Declaration)` (`crates/mc-client/tests/support/reload.rs:276`) copies the
shipped root and rewrites one block file, and refuses if the shipped root does
not already declare that file. If a different construction serves better, take
it; this one is known to exist.

### FR-6.2-S3 — every predicted terrain sample is something other than sky

> WHEN every sample the judge predicts as terrain is compared against the
> captured frame THE SYSTEM SHALL find each of them something other than sky

Home: `crates/mc-client/tests/replay_oracle.rs`,
`every_sample_a_marched_ray_calls_terrain_is_drawn_as_something_other_than_sky`,
with its three existing companions:

- `every_frames_march_predicts_terrain_at_a_hundred_of_the_declared_samples_or_more`
  (the collapse detector — the one-sided comparison's answer to "an oracle
  predicting nothing satisfies it perfectly"),
- `a_prediction_marched_three_degrees_below_the_camera_disagrees_with_the_frame`
  (the control, which needs a terrain horizon in the perturbed frame),
- `a_frame_of_nothing_but_sky_disagrees_with_the_prediction_the_world_gives`.

`tasks.md` and the phase brief both list FR-6.2-S3 among the scenarios
**unsatisfiable until T12**. Measure what it actually does at each stage and
**record the measurement in `test-map.md` either way**, including if it is green
before T12 — a green there is a fact about how much of the assertion depends on
water being in frame, and it is never a licence to skip T12 or to weaken S1.

**`DISAGREEMENT_BUDGET` is not raised.** Its own doc comment says so. If a sample
lands within a pixel of a silhouette after the spawn moves, the remedy is to move
that sample — `SAMPLE_SPACING` / `SAMPLE_ORIGIN` — and record the move and its
reason in `test-map.md`, per the constant's own doc comment: *"a grid quietly
nudged until a suite went green is the same defect as a threshold quietly
lowered."*

`PREDICTION_FLOOR` is **re-derived** for the new pose, not loosened. The
architecture's screen says candidate poses predict 300–500 samples as terrain, so
a floor of 100 holds with slack — but derive it, from both directions, and say so.

### FR-7.1-S1 — the shipped game draws its sea

> WHEN the shipped client runs the declared capture ticks THE SYSTEM SHALL show
> water in each captured frame at every sample pixel where the judge predicts
> water, and at no fewer than one sample pixel in every one of those frames

The strict, per-tick reading. `architecture.md` records that the spec closed this
question deliberately: **all three declared ticks, each independently**. A frame
predicting no water witnesses nothing.

This one needs a **device** and it needs an instrument for "shows water" that
does not come from the renderer. The precedent is
`crates/mc-client/tests/the_grass_top_the_camera_sees_is_its_baked_image.rs`,
which chooses its pixel by marching the world (never by looking at the frame),
then judges the pixel's colour against the **built PNG decoded by the client's
own reader** via `support::art::{drawn_texels, linear_mean, means_agree}` — with a
tolerance derived in both directions (above the measured spread of the image's own
texels, below the nearest wrong answer) and stated in the file's header.

Do the same for water: derive the tolerance from `base:water`'s built image and
from the ΔE to every other thing that pixel could be showing (sky, grass top,
grass side, dirt, stone, the generated stand-in), and **write both bounds down**.
**Do not loosen until green** — an over-tight assertion inviting a real defect and
an over-loose one proving nothing are both live failure modes here
(`testing.md` §2).

Like FR-6.2-S1: **unsatisfiable until T12, and a green FR-7.1-S1 before T12 is a
defect in the test.**

---

## 5. What RED must look like, and the falsifier built into this phase

`testing.md` §1: *"A compile error is acceptable RED only when the scenario is
genuinely about a type or function existing. For a behaviour scenario, get an
**assertion** failure."* Almost every scenario here is behavioural. Display the
failing output before any implementation begins, and put it in `test-map.md`
**with the invocation beside every count**.

This phase has no stub tree to redden against — the subject is the shipped
content and the existing production code. So RED arrives in three distinct
waves, and you must be able to say which wave each of your tests is in:

| Wave | Reddens | Scenarios |
|---|---|---|
| **Immediately, on the tree as it stands** | before T09 | FR-1.3-S1, FR-1.3-S2, FR-2.6-S1, FR-6.2-S2 |
| **The moment T09 lands** | committed area and quad numbers move | FR-2.5-S1, FR-2.6-S2, FR-2.6-S3 |
| **Not satisfiable until T12** | the camera cannot see water at all today | FR-6.2-S1, FR-6.2-S3, FR-7.1-S1 |

The third row is the falsifier built into the phase, and it is worth restating
because it is the one thing here that cannot be recovered later: the architecture
measured **0 water pixels at all three declared capture ticks**, **0 unoccluded
lines of sight to any of the 131 water surface voxels at every one of the 120
ticks**, and water visible at **no yaw at all** from the declared spawn column.
**A green FR-6.2-S1 or FR-7.1-S1 before T12 has measured something other than
what its scenario says.**

**Prefer a derived oracle to a committed number.** No expected quantity in this
phase may be copied from a run of the code under test, and that includes the
figures quoted in §3 of this brief — they are context for your reasoning, not
values to paste. Water's meshed area, water's upward face area, the per-tick
water sample counts and the new quad count are all **derived**, by arithmetic
over the declared world or by an oracle sharing no code with the subject.

---

## 6. Traps that will cost this phase real defects

Each of these was found by reading the tree. They are not hypothetical.

### Trap 1 — `SECOND_REVISION` collides with the new `SCENE_REVISION`

`crates/mc-render/tests/golden_inventory.rs:52` declares
`const SECOND_REVISION: &str = "r2"` — *"a revision nobody has captured anything
for … `r2` is what the day after ambient occlusion looks like"*. T13 makes
`SCENE_REVISION` **`"r2"`**. The test
`the_capture_ids_of_a_second_scene_revision_all_carry_it_and_none_repeats_the_first`
then compares `declared_capture_ids("r2")` against itself: `repeated` becomes 4
where the expectation is 0, and it fails for a reason that has nothing to do with
this spec. `SECOND_REVISION` must move to a revision nothing has captured — and
the doc comment explaining *why that string* moves with it. **This is yours,
because the file is a test file.** Coordinate the moment: it must land with T13,
not before.

### Trap 2 — renaming `Voxels::is_solid` changes the question three other callers were asking

T11 renames `Voxels::is_solid` → `is_drawn` (reading `resolve(name)?.drawn`) and
`first_solid_face` → `first_drawn_face`, at
`crates/mc-client/tests/support/oracle.rs:114`, `:306`. Three other places call
them, and **they are not all asking the same question**:

- `crates/mc-client/tests/the_grass_top_the_camera_sees_is_its_baked_image.rs:202`
  calls `first_solid_face` to ask *"what block face is this pixel of"*. That is
  the drawn question; the rename is right for it.
- `crates/mc-client/tests/support/faces.rs:293` `reachable` calls
  `voxels.is_solid` to ask *"can the eye stand here without being inside
  terrain"*. **That is an occupancy question, not a drawn one.** Under `is_drawn`
  a cell of water would newly disqualify a pose. Decide deliberately which
  question that helper wants, and write the answer into its doc comment. Do not
  let the rename silently change it.
- `crates/mc-client/tests/support/faces.rs:279` `first_exposed` searches for a
  grass block with an exposed face and clear cells in front. Same consideration.

### Trap 3 — a hand-maintained list compared by *filtering* cannot see an extra member

This project has already been bitten twice by exactly this: two mirrors of a
nine-name field list were each held at six while the loader grew, and **neither
reddened**, because one filtered its needles by presence in the observed output
and the other skipped observed items it could not rank. FR-1.3-S1 and S2 are the
same shape — a hand-written expectation of what the shipped content declares.
**Read the list out of the observed output and compare the whole thing, in
order**, so that a missing field, an extra field and a reordering are three
distinct failures.

### Trap 4 — the walk must not become a second copy of the mesher

`crates/mc-sim/tests/support/oracle.rs` is *"the judge, never the thing judged"*.
Its independence is a constraint no assertion can enforce, so it is held by
whoever reads the file. Keep it: six explicit signed offsets of its own, one
hand-written `Side` → `Facing` match, one registry lookup per voxel side, no
bitmask, no resolution pass, **no key table and no boundary plane**. Same for the
client's judge: `architecture.md` Decision 7 is explicit that the judge is given
no boundary plane, no key table and no `occludes` answer, *"and that is what
keeps it a judge rather than a second copy of the culling predicate"*.

### Trap 5 — the "# Water" paragraph in the client judge is deleted, not softened

`crates/mc-client/tests/support/oracle.rs:41-47` currently reads, in part, that
*"A ray passes straight through water, because water's definition is not solid —
and the renderer draws the lakebed for the same reason… The two agree about a
submerged surface by construction rather than by luck."* T11 makes that false.
The architecture's ruling is that the paragraph is **deleted and replaced** by two
things: the positive control (FR-6.2-S1 and S2, the two directions), and the
**named breaker** — a first-drawn-voxel march is right only while every drawn
block is opaque, and **PRO-952 (translucency) is the day it needs a second rule**.
Record the breaker in the module header. The judge comes out of this spec with
more falsifiability than it went in with; say so where a reader meets it.

The header's paragraph at `:30-39` claiming it *"reads solidity through
`BlockDefinition::is_solid`"* also stops being true and moves to `drawn`. The
independence claim two paragraphs earlier — that it never reads the pre-resolved
bitset the physics uses — stays true and stays.

### Trap 6 — the error contract does not change

Both oracles report a `RegistryError` for a block the registry does not
register, and **never read it as "not drawn"**. The reason is written in
`crates/mc-client/tests/support/oracle.rs:110-113`: *"a silent non-solid would
shrink the prediction, and a shrinking prediction is exactly what a one-sided
comparison cannot see."* That reasoning transfers verbatim to `drawn`. Keep both
`# Errors` sections and update the word.

### Trap 7 — a count is only a count with `--no-fail-fast`

In `cargo nextest`, a slashed `N/M tests run` is a **cancelled** run and says
nothing whatever about the remaining tests; a bare `N tests run` is a complete
one. Quote the invocation beside every count you record. This bites hardest on
mutation checks, whose load-bearing half is usually the *green* one.

### Trap 8 — a green suite is no evidence about a lint

A nesting-threshold defect once survived 697 passing tests and two rounds of
falsification. Run

```
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

**directly**, yourself, on your own tree. A lower severity asks a different
question, and without `-D warnings` cargo attributes a diagnostic to the first
binary and marks the rest `(1 duplicate)` — which means *this same diagnostic
repeated*, not *a pre-existing one lives elsewhere*. Note that
`crates/mc-sim/tests/support/oracle.rs` already carries a `Face` struct and a
split helper **purely to stay inside `clippy.toml`'s four-argument and
two-nesting-level caps**; adding a third question to the predicate will push
against both.

### Trap 9 — a reading names the tree it was taken on

An observation of a shared working tree ages exactly as fast as anybody else's,
and this tree may be held by more than one agent. Re-read the tree
(`git stash list && git status --short`, and check whether `HEAD` moved) before
concluding anything from a failure. Prefer a reading that dates itself — a run
naming a specific test as FAILED cannot have come from a tree where that test
passes.

### Trap 10 — before writing a fixture, ask what the shipped caller supplies

Every defect that shipped past six phases of green tests in the hot-reload spec
was one of two things: a fixture supplying a value in a form no caller uses, or a
product path no caller takes. For this phase the question has teeth in one place
in particular: **`support::frames::player_pose`
(`crates/mc-client/tests/support/frames.rs:149`) reaches a camera by *advancing*
the real simulation under the declared script's own intents, never by asking** —
because *"an integrated player cannot have that property, and pretending
otherwise is how a frame comes to be shot through a camera the product never
reaches."* Anything you assert about where the player is or what they can see
goes through that call, not through arithmetic on the spawn constants.

---

## 7. What is *not* yours in this phase

Recorded so you do not write a test for it and find it green for a reason that
belongs to another phase:

- **Targetability wiring** is Phase 4. Water declares `targetable = true` at T09,
  but `crates/mc-sim` still resolves aim through solidity, so nothing a player
  can aim at changes in Phase 3. `docs/user/gameplay.md`'s sentence that the
  crosshair only targets solid blocks is still true at the end of this phase.
- **The save folds and the revision bytes** are Phase 5. `BEHAVIOUR_REVISION` and
  `APPEARANCE_REVISION` do not move here, and `drawn`/`occludes` are not folded
  yet — so `crates/mc-world/tests/shipped_declarations_and_an_older_save.rs` and
  its `BEHAVES_DIFFERENTLY` / `LOOK_DIFFERENT` constants should be unaffected.
  **Measure that rather than assuming it**, and tell the implementer if it moves.
- **The reload geometry check** (`crates/mc-sim/src/world/reload.rs`, `drawn_of`)
  is Phase 5.
- **`crates/mc-client/tests/terrain_probes.rs` is untouched.** It judges its own
  declared observation pose `eye = [44, 56, 44]` looking at `[12, 52, 20]`,
  deliberately *"not a pose the player ever reaches"*, so the spawn move does not
  reach it. `crates/mc-client/tests/edit_geometry.rs`'s landmark constants are
  hand-placed world positions, not spawn-derived, and are untouched too.
- **`crates/mc-world/benches/meshing.rs`** — the implementer re-baselines it
  before T09, to keep water's cost separable from the mesher regression Phase 2
  left behind (PRO-959: terrain +14.9 %, solid +11.8 %). Its fixtures are
  synthetic and declare `drawn == solid`, so nothing you write should move it.

---

## 8. What you record, and where

`specs/active/2026-08-22-solid-split-properties/test-map.md`, in a
`## Phase 3 — …` section following the shape of the Phase 1 and Phase 2 sections
already in that file. It must carry:

1. **The scenario↔test mapping** — every one of the 12 scenarios against at least
   one test, by test function name and file. Test names stay behavioral and
   **never carry a spec or scenario ID**.
2. **Any additional coverage**, under its own heading, one line each stating what
   the test catches.
3. **The failing output**, with the exact invocation beside every count, and the
   wave (§5) each red belongs to.
4. **Any moved sample**, with the reason — required by `SAMPLE_SPACING`'s own doc
   comment.
5. **The re-derivation of `PREDICTION_FLOOR`**, from both directions.
6. **The FR-7.1-S1 tolerance**, both bounds and the nearest wrong answer.
7. **Where a scenario is closed by a test that already existed** — with the
   measurement that says so, not just the claim.
8. **Mutations run and what each proved, including the ones that did not bite.**
   A mutation that does not bite is evidence about the code's structure, not
   automatically a test gap — record the outcome either way. Revert a mutation by
   **re-editing the line by hand**, never with `git checkout --`, and confirm with
   `git diff --exit-code`.
9. **The clippy invocation and its result**, per trap 8.

The implementer separately records the T12 derivation of `SPAWN_COLUMN` and
`SPAWN_YAW_DEGREES` and the resulting per-tick water sample counts in the same
file. Leave room for it; do not write it for them.

---

## 9. Working rules

- **Announce a mutation window before deliberately breaking the tree**, with a
  baseline test count and how many failures to expect. Two agents mutating one
  tree produces exactly the signature of a flaky test, and the failing-test count
  is what distinguishes them afterwards.
- **Never `git add -A` or `git add .`** — stage explicit paths, with no "the tree
  is clean, I checked" exception. `git branch --show-current` before every commit.
- **Conventional Commits.** Test code and implementation code never in one
  commit. Your commits are `test: add failing tests for [behavior]`. Reference
  scenario IDs (`FR-x.y-Sz`) **in the commit message on the branch, never in code
  or test names**.
- **No `.gitignore` rule for `*.proptest-regressions`.** Delete seeds written by a
  deliberate mutation; **commit** seeds written by a genuine failure.
- **Do not run `scripts/sdd-gate.ps1`** — the team lead owns the gate.
- **Nothing you write may cite a path inside `specs/active/`.** That folder is
  archived at completion and the citation would dangle. State a rule by its
  failure, not by a path prefix. (This brief is itself inside `specs/active/` and
  is disposable; nothing in the tree may point at it.)
- The machine's owner has freed it; heavy runs are permitted. Run suites with
  `--no-fail-fast`.

---

## 10. Reporting and arbitration

- Report via `SendMessage` to `main`. Every claim carries the command that
  produced it.
- **Never end a turn silently.** If a turn ends with work outstanding, send one
  line of prose. `[DONE]` only when nothing of yours is running and the tree holds
  none of your uncommitted work.
- If you hit a decision you cannot resolve from the spec, the standards or the
  repo — competing viable approaches, an ambiguous scenario, a conflict with an
  invariant — **do not guess and do not fail the phase**. Send the question with
  the options you see and your recommendation, and wait.
- During implementation, disputed failures come to you. Judge against the spec
  scenario and answer with exactly one of `test-correct`, `test-wrong`,
  `scenario-ambiguous`. Nothing is resolved by a quiet edit in either direction.
