# Tasks: One bit answers four questions

**Spec**: [spec.md](spec.md) (SPEC-021) · **Architecture**: [architecture.md](architecture.md), binding ·
**Branch**: `feature/PRO-904-solid-split-properties` · **Created**: 2026-08-22

54 scenarios, every one assigned to exactly one task. Measured, not relayed:

```sh
grep -cE "^  - FR-[0-9]+\.[0-9]+-S[0-9]+: " spec.md          # 54
grep -oE "^  - FR-[0-9]+\.[0-9]+-S[0-9]+" spec.md | sort | uniq -d   # prints nothing
```

Per group: FR-1 8, FR-2 18, FR-3 9, FR-4 3, FR-5 6, FR-6 5, FR-7 4, FR-8 1.
Per phase: 7 · 14 · 12 · 12 · 9 = 54.

`[P]` = independent of other `[P]` tasks in the same phase. Rigor is `high`, so a
phase's tests are authored and owned by a test author who has not seen the
implementation, and the implementation context never edits a test file — each
phase below states the boundary that makes that possible.

---

## Why the phases split where they do

**The only hard constraint is the gate** (Key Principle 4: `scripts/sdd-gate.ps1`
exits 0 at every phase end). The gate runs `cargo nextest run --workspace`, which
includes `mc-client --test terrain_goldens` and `--test hud_goldens`, and those
acquire a **real device** on this machine — measured: `MYCRAFT_ALLOW_NO_GPU` is
unset and `cargo nextest run -p mc-client --test terrain_goldens` ran 1 test to
PASS in 1.72 s against the four committed `…-r1` directories. So the goldens are
in the gate, and **anything that changes a rendered pixel leaves the gate red
until the set is re-minted.**

That makes the water quads, the moved spawn, the judge and the mint **one
gate-atomic unit**, which is what `architecture.md` Decision 8 already required
in the words *"the spawn move and the water quads land in the same increment"*.
Phase 3 is that unit, and it is the **only** phase that moves a shipped number or
a shipped pixel.

The split that keeps it to twelve scenarios instead of twenty-four rests on one
property, derived by reading the predicate: while the shipped water still
declares nothing, every shipped block has `drawn == occludes == solid`, so
`drawn(self) && !occludes(beyond) && key(self) != key(beyond)` reduces to today's
`solid(self) && !solid(beyond)` — the third clause only ever fires where
`!occludes(beyond)` already held, and an absent or empty neighbour is key 0,
which is not the key of any block. **Phase 2 therefore rebuilds the whole mesher
and moves no shipped quad**, and `scene_contract`'s committed
`SCENE_QUAD_COUNT = 2759`, `replay_world`'s zero-water-quads test and the four
`r1` goldens staying green *untouched* is that phase's own evidence that the
widening is behaviour-preserving.

**One consequence to look at rather than discover: FR-1.3 sits in Phase 3, not
Phase 1.** `spec.md` and `architecture.md` both name FR-1.3 among the scenarios
that stop FR-2 through FR-4 being satisfied by synthetic fixtures while the
shipped game is unchanged, so it must not land late — and Phase 3 of 5 is not
late. It cannot be earlier: the moment `content/base/blocks/water.luau` states
`drawn = true` *and* the mesher reads `drawn`, the shipped mesh moves and the
goldens go red, so FR-1.3 and the re-shoot are inseparable. Putting FR-1.3 in
Phase 1 would force Phase 2 to carry all of FR-2, FR-6 and FR-7.1 — twenty-four
scenarios, three unrelated test-support modules, and one phase in which a
surprise is attributable to nothing.

The two anti-default instruments that **are** in Phase 2, at its head, are
FR-2.4's three-way fixture and FR-2.7's two-method section, per
`architecture.md` §Risks: *"A phase that reaches green on FR-2.1 and FR-2.2 alone
has proved nothing."*

Phase 4 depends only on Phase 1 and could run before Phase 3. It is placed after
so that Phase 3's whole-suite run over `mc-sim` (T12) meets a tree in which
nothing else has changed — which is the whole point of running it there.

---

## Phase 1: A block declares three new properties

**Closes**: FR-1.1-S1..S4, FR-1.2-S1, FR-1.2-S2, FR-8.1-S1 (7).
**Depends on**: nothing.
**Gate at phase end**: green. Nothing reads the three new fields yet — the
mesher still reads `Resolved::is_solid`, `targeted` still takes `&dyn Solidity`,
and neither fold has gained a field — so no shipped quad, pixel, verdict or
revision byte moves in this phase.

**Test author's boundary**: `spec.md` FR-1.1, FR-1.2 and FR-8.1 plus
`architecture.md` Decision 1 and the `RECOGNISED_FIELDS` order, which is quoted
verbatim in FR-1.2-S1. Nothing here needs a mesher, a camera or a save.

**Before implementation, RED looks like**: for FR-1.1-S1..S3 an *assertion*
failure, not a compile error — implement the three fields returning `false`
unconditionally first so the assertion actually runs (`testing.md` §2, "What
counts as RED"). For FR-1.1-S4 and FR-1.2-S1 a refusal-message mismatch naming
the message actually produced.

**This phase opens the widest adaptation window in the spec.** `requirements.md`
measured 34 non-production files constructing a definition with a solidity value
and four shared helpers exposing the boolean in a *signature*; re-measured here,
the helpers taking a single bool are `crates/mc-sim/tests/support/volume.rs:113`,
`crates/mc-world/tests/common/mod.rs:216`,
`crates/mc-world/tests/mesh_common/mod.rs:190` (all `registry_declaring(blocks:
&[(&str, bool)])`) and a fourth the requirements did not name,
`crates/mc-world/tests/save_resolution.rs:117` `registry_declaring_all(names,
solid)`. While the tree does not compile the gate cannot run, so **whoever
authors tests in that window runs
`cargo clippy --workspace --all-targets --all-features -- -D warnings` directly**
(`testing.md` §2: a green suite is no evidence about a lint).

- [x] **T01** `BlockDefinition` gains `drawn`, `occludes` and `targetable`, each read by the existing `optional_boolean` with `absent` = the declaration's own resolved `solid`; `is_solid`'s doc comment narrows to collision and nothing else — `crates/mc-core/src/block/definition.rs:50`, `crates/mc-world/src/content/luau_declaration/mod.rs:204` (`optional_boolean`), `:272` (`FieldFault::wrong_kind`)
      Scenarios: FR-1.1-S1, FR-1.1-S2, FR-1.1-S3, FR-1.1-S4
      Each field gets a doc comment stating the one question it answers, in the shape the existing fields use. Invariant 1 is satisfied here and only here: every default in this change is a documented constant in the declaration loader, and no engine module derives a property from a name or an id.

- [x] **T02** `RECOGNISED_FIELDS` grows from `[&str; 6]` to `[&str; 9]` in the order FR-1.2-S1 quotes, and the three sites that carry the list move together — `crates/mc-world/src/content/luau_declaration/mod.rs:64` (verified `[&str; 6]`), its hand-maintained mirror at `crates/mc-world/tests/luau_declaration_keys.rs:60` (verified `[&str; 6]`), and the refusal message printed in `docs/modding/blocks-items.md` (verified: the fenced refusal block quoting the six names sits at `:397`, and `crates/mc-client/tests/documented_refusals.rs` compares a real run against it line for line)
      Scenarios: FR-1.2-S1, FR-1.2-S2
      Depends on: T01
      Order is load-bearing — a refusal quotes the list back. `FIELD_NAMES_READ` is 64 and needs no change for nine fields (`architecture.md` Assumption 5).

- [x] **T03** Mod-author documentation: the three fields with type, default and the refusal each produces; the field table rewritten; the sentence "the three optional fields are independent of one another" becomes six; and a worked example that runs — a declaration that is drawn without being solid — `docs/modding/blocks-items.md`
      Scenarios: FR-8.1-S1
      Depends on: T01, T02
      A reference listing names without a working example is not documentation (Key Principle 3). FR-1.2-S2 and FR-8.1-S1 are what make this page mechanically checkable; the rest of the page is prose no test asserts, which is why it is a task rather than a reviewer's memory.

---

## Phase 2: The mesher decides by what is declared drawn

**Closes**: FR-2.1-S1, FR-2.1-S2, FR-2.2-S1..S4, FR-2.3-S1..S4, FR-2.4-S1..S3,
FR-2.7-S1 (14).
**Depends on**: Phase 1 (the three fields must exist to be read).
**Gate at phase end**: green, and that is this phase's substance. See "Why the
phases split where they do": with the shipped water still declaring nothing, this
whole rebuild moves no shipped quad. `crates/mc-sim/tests/scene_contract.rs`,
`crates/mc-sim/tests/replay_world.rs` and the four committed `r1` golden
directories are **not touched in this phase** and must stay green on their
committed numbers.

**Test author's boundary**: `spec.md` FR-2.1, FR-2.2, FR-2.3, FR-2.4, FR-2.7 and
`architecture.md` Decisions 3 and 5 plus Open Question 2. Every fixture here is
synthetic; nothing needs the shipped content, a camera or a save.

**Before implementation, RED looks like**: `testing.md` §1 warns that one
skeleton is often not enough, and this phase needs both directions. FR-2.1-S2 and
FR-2.4-S1 assert that **no** face is emitted, so an emit-nothing skeleton passes
them for the wrong reason; FR-2.1-S1, FR-2.2-S1, FR-2.2-S3 and FR-2.3-S4 assert
that a face **is** emitted, so an over-eager skeleton passes those. Drive the
phase red with the skeleton that makes *this* scenario fail, per scenario.

**The existing helper cannot express this phase's fixtures.**
`registry_declaring(blocks: &[(&str, bool)])` takes one boolean per block and
FR-2.4 needs three that disagree, so the helper widens before any of these tests
can be written. That widening is the test author's, in all three copies.

- [ ] **T04** `Section::is_drawn_at` beside `is_solid_at` — same shape, the same empty-cell arm short-circuiting before the registry, the same refusal contract — `crates/mc-world/src/section/mod.rs:176`
      Scenarios: FR-2.7-S1
      This is the instrument that lets an oracle disagree with the subject. Measured: **seven files decide drawnness today by asking a section whether a cell is solid** — `crates/mc-world/tests/block_semantics.rs`, `crates/mc-world/tests/empty_cell_solidity.rs`, `crates/mc-world/tests/mesh_fixtures.rs`, `crates/mc-world/tests/mesh_fixture_scale.rs`, `crates/mc-world/benches/meshing.rs`, `crates/mc-world/benches/support/oracle.rs`, and `crates/mc-sim/tests/support/oracle.rs:199,211`. Not all of them *mean* drawnness — `empty_cell_solidity.rs` and parts of `block_semantics.rs` are about solidity and stay — so each site is read before it moves. Those are test and bench files: the implementation supplies `is_drawn_at` and reports the site list; the test author moves the sites.

- [ ] **T05** The three-way fixture, where `drawn`, `solid` and `occludes` are three different answers and no two can be each other — `crates/mc-world/tests/`, `crates/mc-world/tests/mesh_common/mod.rs:190`
      Scenarios: FR-2.4-S1, FR-2.4-S2, FR-2.4-S3
      **Written first, and green here is what makes green in T06 and T07 mean anything.** `architecture.md` §Drivers: `drawn` defaulting to `is_solid` makes all 34 existing solidity fixtures pass by construction and no count can see it. Split into three scenarios rather than one so a failure says which third failed.
      **Conductor amendment: measure this population here, and do not carry a figure into it.** Shared test helpers exposing a solidity boolean in a *signature* were counted at **21** before Phase 1 and at **24** on `c14de5e` — Phase 1's own 21-site adaptation grew the population it was measured against, which is exactly why neither number may be trusted forward. `requirements.md` says "four" and lists five; an earlier brief said six. Run it: `grep -rn "fn registry_declaring\|solid: bool\|is_solid: bool\|&\[(&str, bool)\]" --include=*.rs crates/ | grep -v "/src/"`. Phase 1 left every one of them alone, and that was correct rather than cautious — Phase 1 can state no fixture where the three properties diverge from solidity **and observe anything**. This task is the first place the count bites, so run the command against the tree you are holding.

- [ ] **T06** The per-face predicate becomes `drawn(self) && !occludes(beyond) && key(self) != key(beyond)` for a neighbour inside the section: `Resolved.blocks` carries `(Contents, drawn, occludes)` instead of `(Contents, bool)`, and `Resolver::solidity_at` is deleted in favour of keeping the key `key_for` already computes and today throws away — `crates/mc-world/src/mesh/resolve.rs:64` (`Resolved::is_solid`), `:253` (`solidity_at`), `crates/mc-world/src/mesh/sweep.rs:266` (`visible_face`), `:317` (`solidity`)
      Scenarios: FR-2.1-S1, FR-2.1-S2, FR-2.2-S1, FR-2.2-S2, FR-2.3-S1, FR-2.3-S2, FR-2.3-S4
      Depends on: T05
      Verified before writing this task: `Resolved::is_solid` is read at exactly one site, `sweep.rs:317`, so it is replaced rather than added to. The third clause is the engine rule and it is stated in the sweep and nowhere else; it compares identity under a table deduplicated by name, never a name, so `visible_face`'s doc-comment property — no block name and no runtime id read anywhere in the file — survives verbatim.

- [ ] **T07** `Boundaries` widens from `[[bool; 256]; 6]` to `[[Key; 256]; 6]` over a table **shared with the meshed section and resolved second**, seeded with `Contents::Empty` at key 0 — `crates/mc-world/src/mesh/resolve.rs:93` (verified `planes: [[bool; plane::CELLS]; BOUNDARIES]`), `:98` (`Boundaries::is_solid` → `key_at`), `crates/mc-world/src/mesh/sweep.rs:281` (`solid_beyond`), `:301`
      Scenarios: FR-2.2-S3, FR-2.2-S4, FR-2.3-S3
      Depends on: T06
      **Two checks before anything else in this task.** (1) Refusal precedence: the existing `MeshError::UnresolvedBlock` / `UnresolvedNeighbourBlock` scenarios must still refuse in the same order and still name the lowest voxel in the meshed section's own linear order — a shared table resolved in the wrong order breaks both, and those scenarios are the cheapest instrument that can see it. (2) Before seeding key 0, grep for any test asserting a `Key` value or a `distinct_blocks` count: `architecture.md` Assumption 3 names exactly that as what falsifies the seeding.
      FR-2.2-S4 falls out of the seeding rather than out of a branch — an absent neighbour's plane is `[0; 256]`, which occludes nothing and is the same kind as nothing. `sweep.rs:288`'s doc comment already states that absence needs no branch of its own, and it must still be true afterwards.

- [ ] **T08** Documentation owed by this phase — `crates/mc-world/src/mesh/sweep.rs` (`visible_face`'s doc comment), `crates/mc-world/src/mesh/resolve.rs:45`, `docs/technical/rendering.md`
      Scenarios: — (Key Principle 3; no scenario asserts prose)
      Depends on: T06, T07
      Three items, the first of which `architecture.md` §Integration adds to the spec's own list: (a) the engine rule "a block never draws a face against its own kind", that it is evaluated over key identity, why a key comparison still reads no name, and **PRO-952 named as its breaker** — a mod author needs to know that two adjacent identical non-occluding blocks show no seam and that they cannot currently ask for one; (b) the four properties a future change must not break (meshed section keyed first; key 0 is `Contents::Empty`; keys reach no output; only the 256 voxels of each shared face ever reach the registry); (c) `Key`'s stated ceiling — verified, `resolve.rs:45` says *"A section has 4096 voxels, so it can hold at most 4096 distinct blocks"* — corrected to 4 096 + 6 × 256 = **5 632** (*derived*), still far inside `u16`.

---

## Phase 3: The shipped game draws its sea, and the golden set re-shoots once

**Closes**: FR-1.3-S1, FR-1.3-S2, FR-2.5-S1, FR-2.6-S1..S3, FR-6.1-S1,
FR-6.1-S2, FR-6.2-S1..S3, FR-7.1-S1 (12).
**Depends on**: Phase 2.
**Gate**: red from T09 until T13, by construction and unavoidably — this is the
gate-atomic unit. Green at phase end. **Nothing that moves world geometry or the
camera may land after T13**, in this phase or any later one, or the set moves
twice.

**Test author's boundary**: `spec.md` FR-1.3, FR-2.5, FR-2.6, FR-6, FR-7.1 plus
`architecture.md` Decisions 7, 8, 9 and Open Question 1. Two of the files here are
test-support modules the test author owns outright —
`crates/mc-client/tests/support/oracle.rs` (the ray-marched judge) and
`crates/mc-sim/tests/support/oracle.rs` (the independent per-voxel walk).

**Before implementation, RED looks like**: FR-2.5-S1 and FR-2.6 fail on the
committed area and quad numbers the moment T09 lands. FR-6.2-S2 fails as an
assertion against a synthetic world holding a `drawn = false, solid = true` block
— no judge reading `solid` can pass it. FR-6.2-S1, FR-6.2-S3 and FR-7.1-S1 are
**unsatisfiable until T12**, and that is the point: `architecture.md` measured 0
water pixels at all three declared capture ticks, 0 unoccluded lines of sight to
any of the 131 water surface voxels at every one of the 120 ticks, and water
visible at no yaw at all from the declared spawn column. **A green FR-6.2-S1 or
FR-7.1-S1 before T12 has measured something other than what the scenario says.**

- [ ] **T09** The shipped water declaration states `drawn = true`, `occludes = false`, `targetable = true`; dirt, grass and stone state none of the three and default to their `solid = true` — `content/base/blocks/water.luau` (verified: today it states `name`, `texture`, `solid = false`, `breakable = false`, `replaceable = true`, and its header explains `solid = false` as what "keeps the mesher from culling against it" — a sentence this task falsifies and rewrites)
      Scenarios: FR-1.3-S1, FR-1.3-S2
      **This is the task that makes the whole of FR-2 through FR-4 about the shipped game rather than about fixtures.** From here the shipped mesh moves and the gate is red until T13.

- [ ] **T10** The shipped world's per-block face areas and its quad count, both derived from an independent per-voxel walk rather than snapshotted from a run of the mesher — `crates/mc-sim/tests/support/oracle.rs`, `crates/mc-sim/tests/scene_contract.rs`, `crates/mc-sim/tests/replay_world.rs`, `crates/mc-sim/src/replay/contract.rs:28`
      Scenarios: FR-2.5-S1, FR-2.6-S1, FR-2.6-S2, FR-2.6-S3
      Depends on: T09, T04 (the walk asks `is_drawn_at`, not `is_solid_at`)
      `SCENE_QUAD_COUNT` (verified `= 2759` today) is minted **only after** the area assertions are green, per the constant's own doc comment, and **never edited to reach green** — minting it first and then deriving the walk to match inverts the only ordering that makes either meaningful. **Do not copy FR-2.5-S1's `4 095` grass / `1` stone from the spec: re-measure it.** `replay_world.rs`'s `no_quad_of_the_meshed_replay_names_the_block_that_fills_its_sea` inverts here — it is the test author's to replace, and its positive control (a column below sea level exists before the quad count is asserted) is what made it non-vacuous and survives.

- [ ] **T11** The judge marches to the first **drawn** voxel: `Voxels::is_solid` → `is_drawn` reading `resolve(name)?.drawn`, `first_solid_face` → `first_drawn_face`, error contract unchanged (`RegistryError` reported, never read as "not drawn" — a shrinking prediction is exactly what a one-sided comparison cannot see) — `crates/mc-client/tests/support/oracle.rs:114`, `:132`, `:306`, header `:36-47`
      Scenarios: FR-6.2-S2
      Depends on: T01
      The module header's "# Water" paragraph — verified present at `:41-47`, the coincidence that a ray passes through water because water is not solid and the renderer draws the lakebed for the same reason — is **deleted and replaced** by the positive control and by the named PRO-952 breaker: a first-drawn-voxel march is right only while every drawn block is opaque. The judge is given no boundary plane, no key table and no `occludes` answer; that is what keeps it a judge rather than a second copy of the culling predicate.

- [ ] **T12** Derive `SPAWN_COLUMN` and `SPAWN_YAW_DEGREES` from the real simulation — `support::frames::player_pose` (`crates/mc-client/tests/support/frames.rs:149`) at each of ticks 0, 59 and 119, which needs no GPU — and record the derivation and the resulting per-tick water sample counts in `test-map.md` — `crates/mc-sim/src/replay/spawn.rs:38` (verified `SPAWN_COLUMN: (u32, u32) = (32, 32)`), `:49` (verified `SPAWN_YAW_DEGREES: f32 = 225.0`)
      Scenarios: FR-6.2-S1, FR-6.2-S3, FR-7.1-S1
      Depends on: T09, T10, T11
      **The pair is derived, never tuned to green**, and the derivation is what makes it a fixture rather than a magic number — `oracle.rs`'s `SAMPLE_SPACING` doc comment states the rule: *"a grid quietly nudged until a suite went green is the same defect as a threshold quietly lowered."* `architecture.md` §Open Question 1 holds a screen over 73 620 poses and reports how many survive the script's `+30°` turn and how many also hold sky, grass and the landmark pillar; those are a *screen*, and are deliberately not repeated here so that no candidate figure in a task list becomes the thing somebody tunes to. Derive, then record.
      `SPAWN_ABOVE_SURFACE = 3` is unchanged (verified `:45`), so the spawn still falls and the first frame still shows a fall. Both spawn doc comments are rewritten: the yaw's stated purpose becomes the sea, and whether the landmark pillar stays in frame is recorded as measured either way.
      **Run the whole `mc-client` and `mc-sim` suites immediately after the two constants change and before anything else in this task**, so a surprise arrives attributable — `architecture.md` §Risks calls the spawn move the widest-radius change in the spec and the one not behind a test's own fixture.
      Re-derive rather than assume: `replay_oracle.rs`'s `PREDICTION_FLOOR`, and re-run its 3° downward-pitch control, which needs a terrain horizon in the perturbed frame. **`DISAGREEMENT_BUDGET` is not raised** — if a sample lands within a pixel of a silhouette, move that sample and record why. `terrain_probes.rs` is untouched (it judges its own declared observation pose `eye = [44, 56, 44]`, deliberately not a pose the player reaches), and `edit_geometry.rs`'s landmark constants are hand-placed world positions, not spawn-derived.
      **If no (column, yaw) pair holds water at ticks 0, 59 and 119**, the fallback is a ladder and it is walked in this order. **Option C is not pre-authorised.**
      (a) **Re-derive with the declared capture ticks as an additional free variable inside the existing 120-tick script.** Still three ticks, still one scene, still one golden set, FR-7.1-S1 untouched and strict — moving *which* frames are captured changes nothing about the requirement's strictness, and the set is being re-minted in T13 regardless, so inside this phase it is nearly free. `DECLARED_TICKS` is `[0, 59, 119]` in `crates/mc-client/tests/support/goldens.rs:37-42`, where each constant carries the meaning it was chosen for — `OPENING` the spawn before it has fallen, `WALKED` the end of the straight walk, `CLOSING` the last tick of the script. **Record why the ticks moved and what each one is now for.** Tick 0 is the fall and tick 119 is the end of the script; losing either meaning is a cost to state, not one to absorb silently. `hud_goldens.rs` picks tick 0 because it is the frame with the least terrain coverage (77.91%, its own measured figure) so the crosshair stands against the most sky — that reason moves with the tick or is re-measured.
      (b) **Only if (a) also fails, escalate before spending `architecture.md` Decision 1's option C** (a second declared spawn plus a third capture set). A second scene is a scope expansion and is not the implementer's call to make.
      (c) **Never a loosened FR-7.1-S1.**

- [ ] **T13** `SCENE_REVISION` `"r1"` → `"r2"`, the four committed `r1` directories deleted and four `r2` minted, following `docs/technical/rendering.md` §"Re-shooting a golden set" **as written and in its stated order** — `crates/mc-render/src/capture.rs:32` (verified `pub const SCENE_REVISION: &str = "r1"`), `crates/mc-render/goldens/` (verified: `player-walk-hud-t000-r1`, `player-walk-t000-r1`, `player-walk-t059-r1`, `player-walk-t119-r1`)
      Scenarios: FR-6.1-S1, FR-6.1-S2
      Depends on: T09, T10, T11, T12 — **all of them, and nothing after.**
      The procedure, verified as written at `docs/technical/rendering.md:1159-1185`: `terrain_probes`, then `replay_oracle`, then `hud_prediction`, then mint through `terrain_goldens` and `hud_goldens` **only**, then verify with `MYCRAFT_UPDATE_GOLDENS` unset including `golden_mismatch` and `mc-render --test golden_inventory`. **The known corrupting failure mode is a run reaching `golden_mismatch` with the opt-in set** — it writes a tick-59 frame as tick 0's ground truth — and the procedure's narrowness is what prevents it. Minting before the probes and the oracle are green photographs a broken renderer permanently.
      `declared_capture_ids` stays the single authority on the set, and `golden_inventory` fails on a stale directory as much as on a missing one (FR-6.1-S1 and S2).

- [ ] **T14** Documentation owed by this phase — `docs/technical/rendering.md`, `docs/user/gameplay.md:60-62`
      Scenarios: —
      Depends on: T13
      `rendering.md` records the re-shoot, why the revision moved and that the declared spawn moved with it. On the player page, the first of the three sentences this spec falsifies — verified verbatim: *"**Water** is declared by the world's content and draws nothing: no face of it is ever emitted, so it has no picture on screen even though it holds a place in the texture set like every other block."* — is replaced by what a player now sees and how to get to it.
      **Conductor amendment: this task also owns `docs/modding/blocks-items.md`, which no later task names.** Phase 1 added a paragraph beginning *"No base block states `drawn`, `occludes` or `targetable`"* (`:675` on `c14de5e`), plus a base-game table stating the same. Both become false the moment `water.luau` declares them, and the paragraph must be **deleted, not edited** — it has no true successor. An editing instruction here invites somebody to soften a sentence that has simply stopped being true.

---

## Phase 4: What can be aimed at, and what a new player holds

**Closes**: FR-3.1-S1, FR-3.1-S2, FR-3.2-S1, FR-3.2-S2, FR-3.3-S1, FR-3.3-S2,
FR-3.4-S1, FR-3.4-S2, FR-3.5-S1, FR-4.1-S1..S3 (12).
**Depends on**: Phase 1 only. Placed after Phase 3 so that T12's whole-suite run
over `mc-sim` meets a tree in which nothing else has changed.
**Gate at phase end**: green. Verified that nothing here moves a golden: the
capture path (`crates/mc-client/tests/support/frames.rs`) composites no targeting
outline, the only "overlay" in the client's draw path is the frame-time debug
reading (`app/mod.rs:277`, `session/mod.rs:415`), and FR-4.1-S1 keeps the held
block `base:dirt`, which is what the HUD golden draws.

**Test author's boundary**: `spec.md` FR-3 and FR-4 plus `architecture.md`
Decision 4 and Decision 11. No camera, no mesh, no save.

**Before implementation, RED looks like**: FR-3.1-S2 as an assertion that the ray
reported the cell *beyond* a `targetable = false, solid = true` block — a
`targeted` still reading solidity reports the block itself. FR-3.2-S1 and S2 are
the wiring scenarios and must fail against a targetability view that is built at
load and never re-written; a view built once satisfies all of FR-3.1.

- [ ] **T15** A second narrow trait over the same type, and a second bitset written where solidity already is: `Targetable { fn is_targetable(&self, at: BlockPos) -> bool }`; `SolidVoxels` renamed `ResolvedVoxels` and gaining a second `Bitset`; `LastResolved` caching the pair rather than the bool; `targeted` taking `&dyn Targetable` — `crates/mc-sim/src/replay/solid.rs`, `crates/mc-sim/src/world/mod.rs:232` and `:262` (the two write sites), `crates/mc-sim/src/world/action/trace.rs:56`
      Scenarios: FR-3.1-S1, FR-3.1-S2, FR-3.2-S1, FR-3.2-S2
      **`Solidity` keeps its meaning everywhere it is read.** Re-measured: `&dyn Solidity` appears at `player/collide.rs:101,125,222,250,259` (the five `collide.rs` sites `architecture.md` names), `player/physics.rs:74`, and `world/clearing.rs:68,98,114` — nine sites that mean collision, none of which change — plus `world/action/trace.rs:56`, which is the one that moves. This is why widening `is_solid` to mean targetable was rejected: silently changing what one method means at nine call sites is the shape of a defect nothing can see.
      **Both bits are written at the two sites that already write solidity and nowhere else.** Verified doc comments: `:232` *"**The one place either view is written**, and there is no other"* and `:262` *"**The other place either view is written**, and there is no third"* — "either" becomes "any", and the property they name is what FR-3.2 rests on. Ask of the second bitset what calls it and what would go red if the write site stopped setting it: the answer must be FR-3.2-S1 and S2 and nothing else in the suite (`testing.md` §2, "policy is not wiring").
      Cost, *derived*: 64 × 64 × 256 = 1 048 576 voxels at one bit = +128 KiB for the second view, once, at world scale.

- [ ] **T16** The shipped water is aimable and only within reach, a break swung at it is refused as indestructible with the water left in the cell and the block behind it untouched, and a placement aimed at it still builds through — `crates/mc-sim/tests/`
      Scenarios: FR-3.3-S1, FR-3.3-S2, FR-3.4-S1, FR-3.4-S2, FR-3.5-S1
      Depends on: T15, T09
      **FR-3.4 is the scenario SPEC-020 could not write, and this task blows the fuse that recorded the debt.** `crates/mc-sim/tests/shipped_water_is_not_broken_and_is_built_through.rs:138` — verified, `a_break_aimed_through_the_shipped_water_reaches_the_solid_block_behind_it`, whose doc comment says *"It reddens when water becomes targetable, which is the point. Read the header before changing it: the repair is a new scenario, not a new expectation."* FR-3.4 is that new scenario. `replaceable` is a separate declaration and is unchanged, which is why FR-3.5 still holds.

- [ ] **T17** The block a new player finds in hand is the first *colliding* block in registration order — reads `solid`, unchanged — `crates/mc-sim/src/world/action/mod.rs:348`
      Scenarios: FR-4.1-S1, FR-4.1-S2, FR-4.1-S3
      Depends on: T01
      The fourth consumer of the old bit, answered explicitly rather than left to fall out of the split: a held block is one you place to build with, and building means an obstacle. **FR-4.1-S2 is the one that needs the split to exist** — a registry of only non-solid blocks offers no held block *even where some of them are drawn*.

- [ ] **T18** Documentation owed by this phase: the four sites that describe a fuse this spec blows, all four rewritten — `docs/technical/architecture.md:828-841`, `Refusal::Indestructible`'s doc comment (`crates/mc-sim/src/world/action/mod.rs:129`), `targeted`'s (`crates/mc-sim/src/world/action/trace.rs:53`), and the header of `crates/mc-sim/tests/shipped_water_is_not_broken_and_is_built_through.rs`. Plus the second falsified player sentence — `docs/user/gameplay.md:64-68`
      Scenarios: —
      Depends on: T15, T16
      Verified: `architecture.md:829-841` states *"One shipped variant is unreachable, and the shipped content is why"* and hands the debt over in those words — *"whoever makes that split owes the scenario that cannot be written now"*. It becomes reachable here. The player sentence, verified verbatim: *"The crosshair only ever targets *solid* blocks, and water is not one, so a swing at a cell of water goes straight through it and breaks whatever solid block is behind."*
      **Conductor amendment: `docs/modding/blocks-items.md` is a fifth site and no task named it.** Two statements there fall due here — *"Only a *solid* block can be aimed at — the ray a break travels stops at the first solid cell"*, and the sentence that it *"will start mattering the day a non-solid block can be targeted"*. Both were true when Phase 1 read them and both become false in this phase; they wrap across lines, so grep a fragment rather than the whole sentence (`:683` and `:686` on `c14de5e`).

---

## Phase 5: What a declaration change is classified as — in a save, and in a live reload

**Closes**: FR-5.1-S1..S3, FR-5.2-S1, FR-5.2-S2, FR-5.3-S1, FR-7.2-S1..S3 (9).
**Depends on**: Phase 1. Placed last because the behaviour-revision bump reports
every block of every existing save as changed, and this is the phase that proves
the report rather than a later one that inherits it.
**Gate at phase end**: green. Nothing here moves a pixel.

**The two halves are each other's cross-check, which is why they are one phase.**
`targetable` is **in** the behaviour fold and **out** of the geometry key;
`drawn` and `occludes` are **in** the appearance fold and **in** the geometry
key. An implementation that folded all five fields into one key passes FR-7.2-S1
and S2 and fails only FR-7.2-S3; an implementation that put `drawn` on the
behaviour list passes FR-5.2 and fails FR-5.1-S2 and FR-5.3.

**Test author's boundary**: `spec.md` FR-5 and FR-7.2 plus `architecture.md`
Decisions 2 and 6. The committed pre-spec save fixture already exists.

**Before implementation, RED looks like**: FR-5.1-S3 as a byte-sequence
mismatch built by hand, in both directions — this is the **only** witness that
can see either revision byte move, because every other witness compares one fold
to another and cannot see a leading byte that moved in both. FR-5.2-S1 as a
whole-verdict inequality naming which of the four blocks is missing from the
changed list; the committed fixture already reports `base:water` as
behaviour-changed today, so a scenario asking only that *some* block is named
would stay green against an implementation that folded no new field at all.

- [ ] **T19** `DeclaredBehaviour` gains `targetable` and `BEHAVIOUR_REVISION` moves 1 → 2; `DeclaredAppearance` gains `drawn` and `occludes` and `APPEARANCE_REVISION` moves 2 → 3 — `crates/mc-world/src/persistence/format.rs:275` (verified `BEHAVIOUR_REVISION: u8 = 1`), `:276` (verified `APPEARANCE_REVISION: u8 = 2`), `:341`, `:362`
      Scenarios: FR-5.1-S1, FR-5.1-S2, FR-5.1-S3
      Depends on: T01
      Both structs stay **written out by hand and never derived** from `BlockDefinition` — a derive would bind every save to a struct that changes for other reasons. The new fields are **appended after the existing ones**, because `postcard` encodes positionally: a rename changes no byte and an insertion in the middle changes every one. **Self-merging is not a field in this spec** (`architecture.md` Decision 10), so it joins neither fold and the spec's ruling-table row for it is vacated rather than contradicted.
      Why not put `drawn` or `occludes` on the behaviour list: it would tell every player in existence that every block they built with behaves differently, on the strength of a rendering field — the exact ambiguity the two bytes exist to prevent.

- [ ] **T20** The whole verdict a save is opened under, asserted whole rather than as an absence — `crates/mc-world/src/persistence/table.rs:54` (`RegistryVerdict`), `crates/mc-world/tests/shipped_declarations_and_an_older_save.rs`, `crates/mc-client/tests/changed_blocks_named_on_the_error_stream.rs`
      Scenarios: FR-5.2-S1, FR-5.2-S2, FR-5.3-S1
      Depends on: T19
      FR-5.2 is what makes the behaviour bump survivable rather than fatal, and it is the first exercise of PRO-956's changed-save load path against a revision byte that actually moved: changed = `base:dirt`, `base:grass`, `base:stone`, `base:water` in ascending order, missing empty, retextured empty, and one line on the error stream. FR-5.3 is asserted whole for the mirror reason — "no changed block was named" is also what an `occludes` folded into *neither* list would produce.
      **Fallout sweep, measured** — the files that name a `RegistryVerdict` or a `changed` / `retextured` list, and therefore the set to walk before this task is called done: `crates/mc-world/tests/{save_acceptance,save_changed_blocks,save_declarations,save_resolution,save_scale,shipped_declarations_and_an_older_save,mesh_determinism}.rs`, `crates/mc-world/tests/common/mod.rs`, `crates/mc-sim/tests/publication.rs`, `crates/mc-client/tests/{shipped_binary,art_and_renderer_failures_are_told_apart}.rs`, `crates/mc-client/tests/support/{changed_blocks,reload}.rs`. Most write and read a save through one helper and stay self-consistent across a revision move; the ones that cannot are those reading a **committed** fixture, of which `shipped_declarations_and_an_older_save.rs` is the one the spec names.

- [ ] **T21** The reload's geometry key learns `drawn` and `occludes` and stays ignorant of `targetable`: `drawn_of` moves from `(is_solid, &textures)` to `(drawn, occludes, &textures)` — `crates/mc-sim/src/world/reload.rs:83-89` (verified: `fn drawn_of(...) -> BTreeMap<&BlockName, (bool, &FaceTextures)>` mapping `(declared.is_solid, &declared.textures)`)
      Scenarios: FR-7.2-S1, FR-7.2-S2, FR-7.2-S3
      Depends on: T01
      **`solid` leaves this key, and that is correct even though it looks like a regression**: solidity changes no geometry once drawnness is its own field, and keeping it here would re-mesh the world for a physics edit. **FR-7.2-S3 is the negative and the one that matters** — a reload changing only `targetable` reports an accepted reload whose published serial advances and whose rebuilt-section count is **zero**. This is the wiring, not the policy: a correct `drawn` field this site never learned about would leave an edited block looking unchanged until relaunch, with every other scenario green.

- [ ] **T22** Documentation owed by this phase — `docs/technical/world-format.md`, `docs/user/gameplay.md:68-70`
      Scenarios: —
      Depends on: T19, T20
      `world-format.md` carries both fold revisions and their numbers. The third falsified player sentence, verified verbatim: *"water is the one shipped block whose recorded behaviour moved, which is why a world saved before this build reports it by name on the terminal"* — after the bump all four shipped blocks report as changed, and the page says so.
      **Conductor amendment: `docs/modding/blocks-items.md` is a third site and no task named it.** On `c14de5e` it is `:689`, and line numbers on this page drift as earlier phases edit it, so match the words rather than the number: *"`breakable` is one of the **five** fields a save folds into a block's recorded behaviour, alongside `name`, `solid`, `replaceable` and `breaks_into`"* — six once `targetable` joins the behaviour fold.

---

## Notes

**Mutation checks owed** (`testing.md` §2 — break by hand, observe, revert by
hand, confirm `git diff --exit-code` is clean, and record the outcome **either
way**, including the mutations that did not bite). One per phase, on the pass
that would otherwise be the least falsifiable:

- Phase 1 — make `optional_boolean`'s `absent` argument a literal `false` for all three fields; FR-1.1-S1 must redden.
- Phase 2 — drop the third clause from the predicate; FR-2.3-S1, S2 and S3 must redden and nothing else should.
- Phase 3 — make `Voxels::is_drawn` read `is_solid` again; FR-6.2-S2 must redden.
- Phase 4 — stop writing the targetable bit in `World::adopt` only; FR-3.2 must redden while FR-3.1 stays green. That asymmetry is the whole reason FR-3.2 exists.
- Phase 5 — put `targetable` into `drawn_of`'s key; FR-7.2-S3 must redden alone.

**A test red for a known reason is fixed before its phase closes, never
annotated** (`testing.md` §2). Three are expected and each is named in its task:
`scene_contract` and `replay_world` from T09 until T10, the four `r1` goldens
from T09 until T13, and the inherited fuse in T16.

**Announce a mutation window before deliberately breaking the tree, and record
the failing-test count** — more than one agent may hold this working tree, and
two agents mutating one tree produces exactly the signature of a flaky test
(`git-workflow.md` §5).

**Figures deliberately not carried into this list.** The candidate-spawn screen's
counts and FR-2.5-S1's grass/stone areas are named in `spec.md` and
`architecture.md` and are **re-measured** in T10 and T12 rather than copied — a
screened figure sitting in a task list is the thing somebody tunes to.

[Deferred observations and follow-ups discovered during implementation go below.
Never delete task text; append status markers only.]

### Phase 1 — T01, T02, T03 done at `45a945c`

**The gate result is not in this line yet.** An earlier revision of this heading
said "gate green" while `scripts/sdd-gate.ps1` was still inside its
`tests + coverage` stage — a status claim standing in for an observation, written
into the durable record. What was actually observed at the time it was written:
`cargo nextest run -p mc-world -p mc-core` at 388 passed / 0 failed,
`cargo clippy --workspace --all-targets --all-features -- -D warnings` at exit 0,
and the gate's earlier stages (sast, secrets, art) reporting ok.

**Gate: green.** `scripts/sdd-gate.ps1` exited **0**, log written outside the
repo and redirected rather than piped. Every stage ok — format, lint + complexity
(clippy, zero warnings), gpu-free, docs, size, deps, sast, secrets, both art
stages. `tests + coverage (llvm-cov nextest)`: **1394 tests run, 1394 passed, 1
skipped** in 117.6 s, and **coverage 93.63%**. The gpu-free stage separately ran
69 passed / 1 skipped and 106 passed / 0 skipped. Measured on the tree at
`d40ad42`, working tree clean.

**Gated a second time, on the tree actually being closed.** The first reading was
taken at `d40ad42`; two spec-folder commits followed (`67fe007` recording that
reading, and the test author's `53f076f` re-running M1–M3 with `--no-fail-fast`).
A gate reading is a statement about a **tree object**, not about a commit, and the
objects differ — `git rev-parse d40ad42^{tree} 53f076f^{tree}` gives
`315e073f79d04ba436ff1c261781df54f802e212` and
`5181de87dfdc18daf142c3244f8d8116536035a7` — so the first reading does not
transfer, whatever changed. **The rule, worth applying mechanically rather than
re-deriving: if the tree object moved, re-gate; if it did not, do not.** No
argument about *which* files changed can make a reading transfer; it can only
predict what the re-run will say.

Re-run on `53f076f`: `scripts/sdd-gate.ps1` exited **0**, every stage ok,
**1394 run, 1394 passed, 1 skipped** in 119.1 s, **lines 93.63%, regions 92.1%,
10744 lines tracked**. Log written to a *second* file so the `d40ad42` evidence
was not overwritten — two trees need two logs.

**Why recording this does not demand a third run.** Committing these words moves
the tree object again, which is a real regress rather than a technicality. What
bounds it is that the gate's inputs are unchanged, measured rather than argued:

```sh
git diff --stat d40ad42..53f076f -- crates docs tools content scripts Cargo.toml
# prints nothing — byte-identical
```

Every stage reads only those paths; the size stage is narrowest and explicit
about it (`$SizeRoots = @('crates', 'tools')` with `-Filter '*.rs'`), and no stage
in the script names `specs` at all (`grep -n "specs" scripts/sdd-gate.ps1` prints
nothing). The two runs above are the corroboration: two different tree objects,
differing only in spec-folder markdown, produced identical figures down to
`93.63%`. So a spec-folder commit cannot move a gate reading — which is a claim
about this gate script, and stops being true the day a stage learns to read
`specs/`.

**One thing the size stage is worth knowing for, beyond this phase.** It fails
when a declared root measures **zero** files, because a total is vacuous at the
granularity that matters: `crates/` contributes ~400 files, so a mistyped `tools`
root contributes nothing, the total barely moves, and the stage would otherwise
pass while a whole tree went unmeasured. That is a positive control living inside
a gate stage, and the reason to check *what* a stage measures rather than that it
printed `ok`.

**Mutation, Phase 1 — it bit.** `optional_boolean`'s `absent` made a literal
`false` inside `defaulting_to_solidity`. `cargo nextest run -p mc-world -p mc-core
--no-fail-fast`: **388 run, 387 passed, 1 failed** — exactly
`a_solid_block_that_states_nothing_more_is_drawn_occludes_and_can_be_aimed_at`
(FR-1.1-S1), as the task predicted, and nothing else. Reverted by re-editing the
line; `git diff --exit-code` returned 0. Note that a plain run without
`--no-fail-fast` reports `186/388 tests run` and proves nothing about the other
202 — the count is only a count with fail-fast disabled.

**T02 has more sites than it names, measured.** The task names the loader, the
mirror at `luau_declaration_keys.rs:60`, and one documented refusal. Actually
five things moved:

- `crates/mc-world/src/content/luau_declaration/mod.rs` — the constant.
- `crates/mc-world/tests/luau_declaration_keys.rs` — the mirror (test author's).
- `crates/mc-client/tests/support/quoted_refusals.rs` — a **second** test-side
  mirror the task does not name, formerly
  `documented_refusals.rs:308 FIELDS_IN_THE_ORDER_THE_GUIDE_STATES`. It was
  silently blind: `ranked_field` returns `Option` and its caller skips what it
  cannot rank, so a list left at six compiled and reddened nothing.
- `docs/modding/blocks-items.md`, `docs/modding/hot-reload.md` and
  `docs/modding/README.md` — **three** pages quoted the six-name refusal, not
  one. `every_refusal_the_modding_pages_quote_is_a_refusal_the_client_prints`
  sweeps every page under `docs/modding/`, and its failing output does not name
  which page a stale quotation came from. Command:
  `grep -rn "may state \`name\`" docs/`.

**The line number in T02 is off by one.** `awk 'NR>=394 && NR<=398'` on the
pre-change page: 395 is the opening fence, **396 is the refusal text**, 397 is
the closing fence. `spec.md:426`, `spec.md:552`, `architecture.md:355`,
`architecture.md:739` and `requirements.md:115` all say `:396` and are right.

**Where a `drawn` quotation may sit on the blocks page is tighter than recorded.**
The last `texture`-blaming quotation on the page is **not** in the texture-table
section — it is in the bounds section (`texture` holds 257 characters), now at
`docs/modding/blocks-items.md:496`, which sits *below* "Reading a refusal". A
`drawn` quotation anywhere above it returns
`OutOfFieldOrder { field: "texture", after: "drawn" }` from
`the_modding_guide_states_every_per_facing_refusal_in_the_recognised_field_order`.
The `drawn = 1` quotation therefore lives in a new section placed after the
bounds, at `:528`. Both numbers are from the committed tree, checked with
`grep -n`: an earlier revision of this note said `:502`, which was measured while
the `drawn` quotation was still in "Reading a refusal" and six lines above the
bounds section had not yet been removed. A line number in a note is read as
verified, so it is worth saying which tree it came from. `slid` and `drawnn` are unranked — neither is a recognised field — so
they order nothing.

**Three statements on `docs/modding/blocks-items.md` fall in later phases, and no
later task names this page.** Recorded here rather than fixed: each is true today
and none of them is Phase 1's.

- Phase 3 (FR-1.3): the base-game table and the paragraph beginning "**No base
  block states `drawn`, `occludes` or `targetable`**" both go stale the moment
  water declares them. T14 names `docs/technical/rendering.md` and
  `docs/user/gameplay.md:60-62`.
- Phase 4 (FR-3): "Only a *solid* block can be aimed at — the ray a break travels
  stops at the first solid cell", and the sentence that it "will start mattering
  the day a non-solid block can be targeted", both become false. T18 names four
  sites plus `docs/user/gameplay.md:64-68`, none of them this page.
- Phase 5 (FR-5.1): "`breakable` is one of the **five** fields a save folds into a
  block's recorded behaviour, alongside `name`, `solid`, `replaceable` and
  `breaks_into`" becomes six when `targetable` joins the behaviour fold. T22 names
  `docs/technical/world-format.md` and `docs/user/gameplay.md:68-70`.

**The three tasks landed as one commit, and no split of them is green.** T01's own
RED needs T02's list growth, because `only_recognised_fields`
(`luau_declaration/mod.rs:145`, doc comment at `:174-176`) runs ahead of every
field read — so a fixture stating `drawn` is refused as unrecognised and never
reaches `optional_boolean`. And the page quotations are what FR-1.2-S2 and
FR-8.1-S1 assert, verbatim against a real run. Code-then-docs leaves
`documented_refusals` red; docs-then-code leaves it red the other way. One commit
was the only ordering with no red commit in it (`git-workflow.md` §2).

**`check` broke the nesting/length lint and the extraction is load-bearing.**
Three `optional_boolean` calls inline give
`error: this function has too many lines (35/30)` under
`cargo clippy --workspace --all-targets --all-features -- -D warnings`. Extracted
as `defaulting_to_solidity`, which also names the one derived default the loader
has. No `#[allow]`.
