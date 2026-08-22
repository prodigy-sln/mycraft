# Requirements — PRO-904

Source: [PRO-904](https://linear.app/prodigy-solutions/issue/PRO-904/one-bit-answers-four-questions-split-solid-drawn-occludes-and),
description corrected 2026-08-22. `product/roadmap.md:124` states the feature as
*"Solid, drawn, occludes and targetable split, plus swimmable and density — you can
see water and swim in it"*.

Everything below marked **measured** was produced by a command run on
2026-08-22 against `main` at `57327a9` / branch
`feature/PRO-904-solid-split-properties`. Everything marked **derived** is
arithmetic or a reading of the tree, labelled as such.

---

## 1. What one bit answers today

`BlockDefinition::is_solid` (`crates/mc-core/src/block/definition.rs:50`) is read
by seven production sites, each asking a different question:

| Site | The question |
|------|--------------|
| `crates/mc-world/src/mesh/resolve.rs:324` → `mesh/sweep.rs:275,298,317` | is this drawn, and does it cull its neighbour's face? |
| `crates/mc-sim/src/player/collide.rs:251` | does this stop me? |
| `crates/mc-sim/src/world/action/trace.rs:69` | can I aim at this? |
| `crates/mc-sim/src/world/action/mod.rs:348` | is this sensible to put in a new player's hand? |
| `crates/mc-sim/src/world/mod.rs:251`, `replay/solid.rs:186`, `mc-world/src/section/mod.rs:189` | the collision bitset, packed from the same bit |
| `crates/mc-sim/src/world/reload.rs:87` | did a reload change what is drawn? (`(is_solid, &textures)`) |
| `crates/mc-world/src/persistence/format.rs:343` | what a save records as declared behaviour |

The mesher makes the conflation explicit: `visible_face`
(`mesh/sweep.rs:275`) returns no face at all unless the voxel is solid, so
**visibility is derived from physics**.

## 2. Measured facts

- **The replay world holds 178 water voxels, in 131 of its 4 096 columns**, at
  depths of one block (84 columns) and two blocks (47 columns). Measured with a
  throwaway integration test over `ReplayWorld::generate(REPLAY_SEED, …)`, since
  deleted; the census printed
  `{"base:dirt": 12288, "base:grass": 4095, "base:stone": 146927, "base:water": 178}`.
  Independently reproduced by a script replicating `replay/height.rs` in `f32`,
  which agreed exactly on the census and on the surface histogram.
- **Every one of those 131 water columns has its top face open to air**, because
  water is placed only up to `SEA_LEVEL = 34` and nothing is placed above it
  (`crates/mc-sim/src/replay/world.rs:260`). Derived from the placement loop.
- **Zero quads name water today, and the world genuinely holds some.** Measured:
  `cargo nextest run -p mc-sim --test replay_world --test scene_contract` — 13
  passed, including `no_quad_of_the_meshed_replay_names_the_block_that_fills_its_sea`,
  whose own positive control asserts a column below sea level exists before
  asserting the quad count is zero (`crates/mc-sim/tests/replay_world.rs:200-204`).
- **Drawing water therefore moves the committed quad count and re-shoots the
  golden set.** `SCENE_QUAD_COUNT = 2759`
  (`crates/mc-sim/src/replay/contract.rs:28`) is asserted by
  `the_meshed_quad_count_matches_the_committed_scene_contract_snapshot`
  (`crates/mc-sim/tests/scene_contract.rs:122`), whose failure message names the
  whole remedy. This closes the issue's second open question: **yes**, the
  goldens re-shoot. `SCENE_REVISION` is `"r1"`
  (`crates/mc-render/src/capture.rs:32`) and the committed set is four
  directories of two files each under `crates/mc-render/goldens/`.
- **`breakable = false` on water is inert for the player.** Measured previously
  by SPEC-020 and recorded in
  `crates/mc-sim/tests/shipped_water_is_not_broken_and_is_built_through.rs:138`,
  `a_break_aimed_through_the_shipped_water_reaches_the_solid_block_behind_it`,
  which asserts the break reaches the stone behind and is written as a **fuse**:
  its header states it goes red the moment water becomes targetable, and that the
  spec which makes it red owes the scenario it could not write.
- **No numeric field is read anywhere in the Luau declaration parser today.**
  `grep -rn "Integer\|Number\|f32\|f64" crates/mc-world/src/content/luau_declaration/`
  returns exactly one match, inside `kind_of`, which maps both numeric variants
  to the refusal word `"a number"`. There is no `required_number` or
  `optional_number`.
- **`density = 1` and `density = 1.0` arrive as different variants.**
  `ScriptValue::Integer(i64)` and `ScriptValue::Number(f64)`
  (`crates/mc-script/src/value.rs:22,24`), translated at
  `crates/mc-script/src/luau/translate.rs:75-76`.
- **`ResolvedBlock::is_solid` is dead in production.** Measured:
  `grep -rn "is_solid" crates/*/src --include=*.rs | grep -v _test.rs` shows it
  read only by `crates/mc-client/src/content.rs:73`, which builds
  `ClientContent::solidity`; and `ClientContent::is_solid`
  (`crates/mc-client/src/content.rs:107`) has **no production caller** —
  `grep -rn "\.is_solid(" crates/mc-client/src crates/mc-render/src` returns
  nothing. The mesher reaches solidity through `BlockRegistry`
  (`mesh/resolve.rs:324`), not through `ResolvedContent`.
- **`mc-render` never reads solidity, and `mc-proto` / `mc-net` are empty
  skeletons.** So the only cross-process surface this change crosses is the save
  file. Measured by grep across those crates.
- **Fixture blast radius: 34 non-production files construct a definition or a
  `ResolvedBlock` with a solidity value.** Command:
  `grep -rlE "is_solid[:,]" crates --include=*.rs | grep -E "/tests/|_test\.rs|/benches/" | wc -l`.
  42 files carry a Luau declaration text containing `solid = `
  (`grep -rlE "solid = " crates --include=*.rs | wc -l`), and 24 test files
  reference `base:water`. The leverage points are the shared helpers: eleven
  `tests/support/` and `tests/common/` modules, of which four expose the boolean
  in a *signature* — `crates/mc-client/tests/support/persistence.rs:125`
  `declared(name, is_solid)`, `crates/mc-client/tests/support/changed_blocks.rs:200`
  `block(…, is_solid, …)`, and three copies of
  `registry_declaring(blocks: &[(&str, bool)])`
  (`crates/mc-sim/tests/support/volume.rs:113`,
  `crates/mc-world/tests/common/mod.rs:201`,
  `crates/mc-world/tests/mesh_common/mod.rs:190`).
- **The declaration's recognised-field list is duplicated and doc-checked.**
  `RECOGNISED_FIELDS: [&str; 6]`
  (`crates/mc-world/src/content/luau_declaration/mod.rs:64`) is mirrored by hand
  in `crates/mc-world/tests/luau_declaration_keys.rs:60`, and the refusal message
  quoting it is compared line-for-line against
  `docs/modding/blocks-items.md:396` by
  `crates/mc-client/tests/documented_refusals.rs`. Three places move together.
- **A section-boundary plane carries one bool per cell and no block identity.**
  `Boundaries { planes: [[bool; plane::CELLS]; BOUNDARIES] }`
  (`crates/mc-world/src/mesh/resolve.rs:93`), filled by `shared_face` from
  `solidity_at`. So the "no face against its own kind" behaviour cannot be
  evaluated across a section boundary as the mesher is built, and the shipped sea
  spans many sections — the case is reachable in the shipped world. Whichever
  mechanism is chosen, the plane has to widen. Measured by reading the type.
- **The verdict a save is loaded under is a total three-list classification.**
  `RegistryVerdict { missing, changed, retextured }`
  (`crates/mc-world/src/persistence/table.rs:54`), whose own doc comment states
  the `retextured` arm exists so that "a test can compare a whole verdict instead
  of asserting an absence". Every scenario about what a load reports asserts the
  whole verdict.
- **Three sentences on the player-facing page become false.**
  `docs/user/gameplay.md:60-72` states that water "draws nothing: no face of it
  is ever emitted", that "the crosshair only ever targets *solid* blocks … so a
  swing at a cell of water goes straight through it", and that "water is the one
  shipped block whose recorded behaviour moved". All three are falsified by this
  spec. Measured by reading them.
- **The two save folds carry separate revision bytes.** `BEHAVIOUR_REVISION = 1`
  over `{input_version, name, is_solid, replaceable, breakable, breaks_into}`;
  `APPEARANCE_REVISION = 2` over `{input_version, name, textures[6]}`
  (`crates/mc-world/src/persistence/format.rs:275-365`). The behaviour byte has
  not moved since the format was written.

## 3. Decisions taken here, with their reasons

### 3.1 `solid` keeps its name and narrows to collision

The alternative was renaming it `collides`. Rejected: it invalidates all four
shipped declarations, 42 test files carrying declaration text, and the
line-for-line doc check, in exchange for a clearer name on a field whose
documentation is being rewritten anyway. `solid` is what a mod author has
already written and what `docs/modding/blocks-items.md` already explains as
"does this block stop a player" (`:94`) — that sentence becomes the *whole* of
what it means rather than one of four things.

### 3.2 Every new field is optional, and the defaults come from `solid`

So that no existing declaration becomes invalid — the same discipline PRO-917
applied to the loader swap. **This is knowingly the trap `testing.md` §2 names**:
a `drawn` default equal to `solid` makes every existing fixture pass by
construction, and a count cannot see it. The countermeasures are written into the
scenario set rather than left to review: **FR-2.4** requires a fixture in which
`drawn`, `solid` and `occludes` are three *different* answers, **FR-2.3-S4**
requires a drawn non-occluding block whose neighbour is a *different* drawn
non-occluding block, **FR-2.3** states `solid` explicitly in every one of its
scenarios, **FR-2.7** separates the two questions a section answers, **FR-7.2-S2**
applies the same discipline to the frame judge, and **FR-1.4** asserts the
shipped declarations so that none of FR-2 through FR-5 can be satisfied by
synthetic fixtures while the shipped game is unchanged.

### 3.3 `density` is a non-negative bounded number in kilograms per cubic metre

`0.0` to `100000.0` inclusive; water declares `1000.0`; absent means `0.0`. Read
as "positive" in the non-negative sense so that `0.0` stays expressible — a
swimmable block offering no resistance is a thing a mod may want to declare, and
refusing `0.0` while accepting an absent field would make the two forms differ
for no reason. Both `ScriptValue::Integer` and `ScriptValue::Number` are
accepted; `density = 0/0` and `density = 1/0` are expressible in Luau and are
refused, which is what makes the finiteness check non-vacuous.

### 3.4 Fold membership, per property

See `## The two architecture decisions` in `spec.md` §Technical Considerations.
Summary: `targetable`, `swimmable` and `density` join the **behaviour** list and
bump `BEHAVIOUR_REVISION` to 2; `drawn` and `occludes` (and self-merging, if it
becomes a declared field) join the **appearance** list and bump
`APPEARANCE_REVISION` to 3.

### 3.5 The render oracle stays independent, and gains a positive control

See the same section. The oracle marches to the first **drawn** voxel rather
than the first solid one, which is a change of *declaration read* and not a
sharing of code — it already reads `is_solid` from the same registry the mesher
reads, and the module header already says the independence is in the
*derivation*. What the change costs is the water coincidence the header
describes, and that coincidence was never evidence: the header itself calls the
agreement "by construction rather than by luck".

## 4. Assumptions carried, each with what falsifies it

- **Density is the swim parameter.** Neither the issue nor the roadmap states
  what `density` *does*. It arrives in the same breath as `swimmable` and the
  exit criterion is "swim in it", so it is specified here as the resistance a
  block's volume offers to a player moving through it. If the intended meaning
  was mass-for-physics or light attenuation, FR-4.3 and FR-4.4 are wrong and
  nothing else in this spec is. **Raised with the requester rather than settled
  silently.**
- **Water is drawn opaque.** Transparency, sorting and the inset/wavy surface are
  PRO-952 and are out of scope. The oracle's first-drawn-voxel march is correct
  only while every drawn block is opaque; the day a translucent block exists,
  the oracle needs a second rule, and that is recorded as the named breaker.
- **A break swung at water is refused, and the water stays.** That is the
  scenario SPEC-020 could not write and this spec inherits (FR-3.3). It follows
  from `breakable = false` once water is targetable; it is asserted, not assumed.

## 5. Questions put to the requester

0. **One spec or two?** The audited scenario set is 78, next to the 82 the
   roadmap names as the spec that hid eighteen wrong-reason passes. The seam is
   clean and counted: 54 scenarios for *"you can see water"*, 24 for *"you can
   swim in it"*, each delivering a named player capability, and the golden
   re-shoot is paid once either way. **Recommendation: split.** Not acted on
   unilaterally, because the roadmap and the issue scope them together. See
   `spec.md` Open Question 1.
1. **What does `density` mean?** (default taken: resistance to movement, kg/m³ —
   see §4.)
2. **Is self-merging a declared field or an engine rule?** A drawn non-occluding
   block abutting itself must not draw an internal face, or a 47-column
   two-deep sea draws faces inside itself. That can be a declared
   `merges_with_self` field or an engine rule "a block never draws a face against
   its own kind". No use for `merges_with_self = false` has been identified, and
   invariant 1 argues against engine-side derivation. Recommendation: specify the
   *behaviour* here and let `/sdd-architect` choose the mechanism. Fold
   membership is **appearance** under either.
3. **Does the replay world hold enough water for a golden frame to see any?**
   131 of 4 096 columns, all one or two blocks deep, against a 32 × 18 sample
   grid. If no sample sees water, the golden set does not witness the spec's
   headline change and the world's sea level may need raising — which moves the
   scene contract again. Named as the first thing `/sdd-architect` must measure.
