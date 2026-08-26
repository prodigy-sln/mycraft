# Test map — PRO-972

**Over the artifact-lint budget, deliberately.** The mapping is one line per
scenario as the budget intends; what overflows it is the evidence
`standards/global/testing.md` §2 requires to be written down — the RED reading
with its invocation, the two places the spec's own figures disagreed with what
the instrument measures, and the mutation results including the ones that did
not bite. None of it is deletable to reach a line count.

## Scenario → test

Paths relative to `crates/mc-client/tests/` unless stated.

- FR-1.1-S1 → `the_shipped_set_covers_every_key_it_declares.rs::the_shipped_roots_declared_keys_and_its_built_sets_covered_keys_are_the_same_keys`
- FR-1.1-S2 → `the_shipped_set_covers_every_key_it_declares.rs::a_key_the_manifest_bakes_no_entry_for_is_named_in_the_verdict_as_uncovered`
- FR-2.1-S1 → `the_sea_draws_its_baked_art.rs::waters_layer_is_filled_with_the_image_the_manifest_bakes_for_it`
- FR-2.1-S2 → `the_sea_draws_its_baked_art.rs::waters_layer_holds_no_texel_of_the_generated_stand_in`
- FR-2.2-S1 → `an_unauthored_key_draws_a_generated_texture.rs::a_declared_key_the_set_does_not_cover_is_filled_from_the_texture_generated_for_it` *(preservation; existing, unchanged)*
- FR-3.1-S1 → `the_sea_draws_its_baked_art.rs::every_texel_of_waters_baked_image_is_more_blue_than_it_is_red_or_green`
- FR-3.1-S2 → `the_sea_draws_its_baked_art.rs::waters_mean_stands_clear_of_every_other_shipped_images_mean`
- FR-3.2-S1 → `the_sea_draws_its_baked_art.rs::waters_tones_are_distinguishable_and_no_more_mottled_than_dirt`
- FR-4.1-S1 → `no_committed_golden_shows_the_stand_in.rs::no_committed_reference_image_holds_a_pixel_of_the_generated_stand_in`
- FR-4.1-S2 → `crates/mc-sim/tests/scene_contract.rs::the_meshed_quad_count_matches_the_committed_scene_contract_snapshot`, `::the_meshed_area_equals_an_independent_per_voxel_walk_of_the_world`, `::every_blocks_meshed_area_equals_the_independent_walks_area_for_that_block` *(preservation; existing, unchanged — the three together are quad count, total face area and per-block face area)*

## Additional coverage

- `the_sea_draws_its_baked_art.rs::waters_baked_image_is_made_of_the_colours_the_water_materials_declare` → the one reading in this spec with a reference **outside** the renderer. Every other colour assertion in the client suite is judged against a value the renderer or its own generator produced; this one compares the decoded image against the `#rrggbb` a person wrote in `content/base/materials/`. It catches a decoder that swapped two channels, applied a transfer function it should not have, or shaded a face — none of which would move a triple snapshotted from a run.
- The control inside `waters_layer_holds_no_texel_of_the_generated_stand_in`: the two colours being ruled out are read out of `placeholder_texels` and compared against the two a human reported off the screen. A generator change would otherwise leave the scan hunting colours nothing draws.
- The control inside `no_committed_reference_image_holds_a_pixel_of_the_generated_stand_in`: the same check, plus the capture ids read from `declared_capture_ids` rather than listed, so a capture added at this revision is judged the day it is added and a vanished directory fails at the decode.

## RED

`cargo nextest run -p mc-client --no-fail-fast --test the_shipped_set_covers_every_key_it_declares --test the_sea_draws_its_baked_art --test no_committed_golden_shows_the_stand_in`, on `db9a4cb` plus the test files:

```
Summary [   0.162s] 9 tests run: 0 passed, 9 failed, 0 skipped
```

Bare `9 tests run` — a complete run, not a cancelled one. **Each failure is the
defect and not merely a failure:**

- FR-1.1-S1 — `left: TheyDiffer { uncovered: ["base:water"], unused: [] }`. The scan names the one uncovered key, which is the whole defect in one value.
- FR-1.1-S2 — `left: TheyDiffer { uncovered: ["base:water", "example:undrawn"], unused: [] }`. The positive control's fixture root is the shipped root plus one block, so it carries the shipped root's own uncovered key beside the one the fixture added. That second name is what makes this red rather than green-on-arrival.
- FR-2.1, FR-3.1, FR-3.2 — all five return `the shipped built set covers no image for 'base:water', so its layer is filled from the texture generated out of the key itself`. A refusal rather than an assertion failure, and it is the right shape: the oracle these readings would otherwise use *is* the fallback being ruled out, so there is nothing to assert against until art exists. What makes the assertions themselves falsifiable is the mutation table below, not this run.
- FR-4.1-S1 — `left: [Showing { capture: "player-walk-t000-r3", pixels: 77987 }, ... t059: 165232, ... t119: 191792, ... hud-t000: 77987 }]`.

## Two figures where the spec and the instrument disagree, and which is right

1. **FR-4.1-S1's parenthetical says 88 280 / 88 280 / 174 744 / 198 828; the scan measures 77 987 / 165 232 / 191 792 / 77 987.** Both are correct measurements of different things, and the spec's own Defect table is the tell: its three columns sum to exactly the scan's figures. The larger numbers count the **trilinear blends** as well — sampling interpolates between two mip levels, so the frames carry a tail: the RGB box the two texels span, `(140,38,131)` through `(160,58,151)`, minus the three themselves (10 293 at tick 0 and in the HUD capture, 9 512 at tick 59, 7 036 at tick 119, reaching ten bytes from the mean). The scenario names three exact colours, so the scan names three exact colours; reaching the tail would need a tolerance, and a tolerance would start deciding how near a colour is allowed to be. The tail cannot exist without the three.
2. **The share of frame follows.** 8.46% / 17.93% / 20.81% for the three named colours over 1280×720; 9.58% / 18.96% / 21.57% counting the blends, which is what the spec reports.

## Mutations

Run by hand, reverted by hand, `git diff --exit-code` confirmed clean between
rows, `voxforge build` re-run wherever the art changed. Baseline before and after
the M1–M5 window: `cargo nextest run -p mc-client -p mc-render --no-fail-fast` →
**539 tests run, 539 passed, 0 skipped** — a bare count both times, so both are
complete runs. M6 was run later, over the frozen tree whose baseline is **1544
tests run, 1544 passed, 1 skipped**.

**Five of the nine scenarios reached RED through a *refusal* rather than an
assertion**, because the oracle they would otherwise use is the fallback being
ruled out and there was nothing to assert against until art existed. That is what
this table is paid for: the RED run says the tests notice the defect, and only
these say the assertions themselves can fail.

| # | Mutation | Result |
|---|---|---|
| M1 | `content/base/textures.toml`: the water entry's key renamed `base:waterX`, set rebuilt — the defect restored, plus one baked key nothing declares | **10 of 539.** All six of `the_sea_draws_its_baked_art`, both coverage readings, `terrain_goldens` and `hud_goldens`. The coverage verdict named **both** directions at once: `uncovered: ["base:water"], unused: ["base:waterX"]`. `no_committed_golden_shows_the_stand_in` stayed **green**, correctly — it judges committed bytes, and the committed bytes are the re-shot ones |
| M2 | `content/base/blocks/stone.luau`: `texture = "base:granite"` | **2 of 2** in `the_shipped_set_covers_every_key_it_declares`. `TheyDiffer { uncovered: ["base:granite"], unused: ["base:stone"] }`. The scan sees a key that is not water, which the RED run alone could not establish |
| M3 | `water_light.toml` → `#a6a6a6`, set rebuilt — one tone made grey | **2 of 6.** FR-3.1-S1 (a texel whose blue no longer dominates) and FR-3.2-S1 (spread past dirt's 16.10). FR-2.1-S1/S2 and FR-3.1-S2 green, and so is the material-colour reading — **the documented weakness, measured**: it reads the same TOML the bake reads, so a deliberate palette edit flows through both sides at once. It catches a decoder, never a palette |
| M4 | All three water materials moved next to `base:stone` while keeping blue dominant (`#7a7d82` / `#72757a` / `#82858a`), set rebuilt | **1 of 6**, and it is FR-3.1-S2 alone: `["base:stone" at ΔE 3.14]`. The cleanest isolation in the table — the separation bound bites on its own, with blue dominance and the spread bound both still satisfied |
| M5 | `crates/mc-render/src/texture/mip.rs`: `levels_for` filling every layer from `placeholder_texels`, ignoring the supply | **3 of 11.** FR-2.1-S1, FR-2.1-S2, and `an_unauthored_key_draws_a_generated_texture`'s mixed-run reading |
| M6 | M5's mutation **and** FR-2.1-S2 put back to its pre-`e93aa17` form, reading `covering(key)` instead of the filled layer | **1 of 6, and it is not FR-2.1-S2.** `waters_layer_holds_no_texel_of_the_generated_stand_in` reports **PASS** while `levels_for` fills every layer from the generator; only FR-2.1-S1 reddens. This is the measurement that justifies `e93aa17` |

**M6 exists because the claim it settles was first written down as reasoning.**
The row above it originally read "*before it read the filled layer it read
`covering(key)`, which this mutation does not touch, so it would have been green
here*" — derived by reading the code and never run. `testing.md`'s standing rule
is that every wrong premise in this project was reached by reasoning and every
correction by running something, so it was run. It held, and the precise shape
matters more than the verdict: the *file* would still have caught the defect,
through FR-2.1-S1 — but **FR-2.1-S2's own witness was gone**, leaving a
single-witness path where the scenario set says there are two. That is the
`testing.md` §1 case for a second witness, arriving as a scenario silently
delegating its evidence to its neighbour rather than as a missing test.

**FR-1.1-S1, FR-1.1-S2, FR-2.1-S1, FR-2.1-S2, FR-3.1-S1, FR-3.1-S2, FR-3.2-S1 and
FR-4.1-S1 additionally have a dated RED→GREEN transition**, which is the strongest
form of this evidence and cannot be taken at the wrong moment: the RED run above
names each of them FAILED on a tree where the art did not exist, and the same
invocation names them PASSED on the tree that has it. FR-4.1-S1's is the sharpest
— 77 987 / 165 232 / 191 792 / 77 987 before the re-shoot, 0 / 0 / 0 / 0 after.

**Two preservation scenarios were held green throughout** and are not mutated
here, because a mutation of them would be a mutation of somebody else's subject:
FR-2.2-S1 (`an_unauthored_key_draws_a_generated_texture`) and FR-4.1-S2
(`scene_contract.rs`, three tests — quad count, total face area, per-block face
area). M5 does redden FR-2.2-S1's *sibling*, which is evidence that binary is
alive.
