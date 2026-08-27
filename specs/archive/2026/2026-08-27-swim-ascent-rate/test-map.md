# Test map: SPEC-030 — Water carries a swimmer at rates content declares

Scenario → test file → test name. Test names carry no scenario ID; this file is
the whole of the mapping. Each phase appends its own section.

## Phase 1 — a declaration states a swim ascent

**9 scenarios, 11 scenario tests, plus 5 additional-coverage tests.** Command:

```
cargo nextest run -p mc-world -p mc-client --all-features --no-fail-fast -E 'binary(luau_declaration_medium) or binary(luau_declaration_medium_refusals) or binary(luau_declaration_keys) or binary(documented_declaration_fields) or binary(documented_property_refusals)'
```

- FR-1.1-S1 → `mc-world/tests/luau_declaration_medium.rs` → `an_ascent_written_as_a_fraction_registers_as_that_number`
- FR-1.1-S1 → `mc-world/tests/luau_declaration_medium.rs` → `an_ascent_written_as_a_whole_number_registers_as_that_number`
- FR-1.1-S3 → `mc-world/tests/luau_declaration_medium.rs` → `an_ascent_of_zero_registers_rather_than_meaning_the_field_was_never_written`
- FR-1.1-S4 → `mc-world/tests/luau_declaration_medium.rs` → `an_ascent_stated_without_a_swimmability_registers_and_holds_nobody_up`
- FR-2.1-S1 → `mc-world/tests/luau_declaration_medium_refusals.rs` → `an_ascent_below_zero_is_refused_naming_the_field_and_the_floor`
- FR-2.1-S2 → `mc-world/tests/luau_declaration_medium_refusals.rs` → `an_ascent_that_is_not_a_number_at_all_is_refused_as_not_finite`
- FR-2.1-S2 → `mc-world/tests/luau_declaration_medium_refusals.rs` → `an_ascent_without_bound_is_refused_as_not_finite`
- FR-2.1-S3 → `mc-world/tests/luau_declaration_medium_refusals.rs` → `an_ascent_written_as_a_boolean_is_refused_naming_the_kind_it_found`
- FR-2.1-S4 → `mc-world/tests/luau_declaration_medium_refusals.rs` → `an_ascent_written_as_a_string_is_refused_rather_than_parsed`
- FR-2.1-S5 → `mc-world/tests/luau_declaration_medium_refusals.rs` → `a_declaration_stating_two_refusable_numbers_registers_nothing_and_blames_one_of_them`
- FR-3.1-S1 → `mc-world/tests/luau_declaration_keys.rs` → `a_field_one_letter_past_a_real_one_is_refused_quoting_every_field_in_declaration_order`

S1 and FR-2.1-S2 take two tests each: the host carries `3.5` and `4` as
different `ScriptValue` variants, and a floor comparison catches an infinity but
never a NaN. FR-1.1-S2 is a physics scenario, landing with FR-4 in phase 2.
**FR-3.1-S2 needs no assertion of its own** — checked, not assumed: three
existing tests already read the field list *out of* the artefact and compare it
whole and in order, and all three redden from the mirror bump alone.

- FR-3.1-S2 → `mc-client/tests/documented_declaration_fields.rs` → `every_refusal_the_modding_pages_quote_lists_every_field_a_declaration_may_state`
- FR-3.1-S2 → `mc-client/tests/documented_declaration_fields.rs` → `the_guide_tabulates_every_field_with_the_value_its_absence_means`
- FR-3.1-S2 → `mc-client/tests/documented_property_refusals.rs` → `the_guide_introduces_the_declaration_fields_in_the_order_a_refusal_quotes_them`

### Additional coverage — what each catches
- signed zero → `mc-world/tests/luau_declaration_medium.rs` → `an_ascent_of_signed_zero_is_retained_as_the_unsigned_zero_a_save_folds` — a `-0.0` retained where `0.0` was meant, which the save folds by its bits and `==` cannot see. The resistance's twin proves the *shared reader* normalises; it cannot prove `declared_ascent` routes through it.
- whole contract → `mc-world/tests/luau_declaration_keys.rs` → `a_declaration_stating_every_recognised_field_and_nothing_else_registers` — an unrecognised-field check that over-fires. Pre-existing; it gained `swim_ascent` to stay a control over the whole contract, and its reading was made total (it bailed on `?` before, reporting a refusal without comparing it).
- absent means → `mc-client/tests/documented_declaration_fields.rs` → `the_guide_tabulates_every_field_with_the_value_its_absence_means` (extended with `what_a_missing_ascent_means`) — a table row present but silent about its default. `swim_ascent` is the only row on that page whose default is not the value an absent field resembles: it is the jump speed, and an author left to guess writes `0.0`.
- page's ceiling → `mc-world/tests/luau_declaration_medium_ceiling.rs` → `the_largest_ascent_the_guide_promises_is_registered_at_the_width_the_engine_keeps` — a value at the top of the scale clamped or refused, against a page that promises it.
- page's ceiling → `mc-world/tests/luau_declaration_medium_ceiling.rs` → `an_ascent_a_step_past_that_promise_is_refused_as_not_finite` — a reader judging what the script *said* rather than what is *kept*. Added after T04 put `at most 3.4e38` on a second row of the `Bound` column; both pairs share one reading, told which field to read back.

### RED, measured

`1555 tests run: 1538 passed, 17 failed` — a bare count under `--no-fail-fast`,
so nothing was cancelled — 17 `assertion left == right failed` lines, no `Error:`
bail. **Not the reason the brief predicted:** `swim_ascent = 3.5` is refused, not
registered as `9.0`, because `only_recognised_fields` runs before any value is
read. Still an assertion failure — what these files' *total* readings buy — and
unavoidable. Once T03 bumps the list but before `declared_ascent` exists the same
tests fail on `9.0`, so a half-landed T03 is caught too.

**The blind window** opened at T01, closed at T02 (`b8279a3`). Three instrument
lessons, all measured. Clippy at `-D warnings` the moment it closed reported
`clippy::too_many_lines` on `registry_texturing` (30 lines to 31). That clean
clippy said nothing about `cargo fmt --check`, which named two other files.
Fixing *that* reflowed eight fixture tuples and took `media_registry` from 13
lines to 36, which clippy reported and no suite could. **`fmt` and `clippy` move
each other's input: re-run both after the last edit, not once each.**

### Mutation outcomes
Both on `16a2ca4`, announced, reverted by hand, `git diff --exit-code` clean
after. Workspace-wide under `--no-fail-fast` — bare counts, nothing cancelled —
because "nothing else moved" is the load-bearing half.

- **`declared_ascent` alone judges what the script said** (finite as an `f64`,
  infinite as an `f32`, returned rather than refused) → `1557 tests run: 1556
  passed, 1 failed`. **Exactly one**: `an_ascent_a_step_past_that_promise_is_
  refused_as_not_finite`. Every other `swim_ascent` fixture stayed green, and so
  did the resistance's pair. This is what justifies the ascent pair existing.
- **The shared reader stops rejecting infinities** (`!stated.is_finite()` →
  `stated.is_nan()`) → `1557 tests run: 1552 passed, 5 failed`: both
  `_a_step_past_that_promise_` tests, both `_without_bound_` tests, and
  `a_resistance_too_large_for_the_width_the_engine_keeps_is_refused_as_not_finite`.
  It bites, but reddens **both** pairs together — evidence the shared reader is
  under test, and *not* an argument for the second pair. Recorded anyway: a
  mutation that does not discriminate is still a fact about the suite.

## Phase 2 — the medium carries an ascent to a tick

**16 scenarios, 16 tests, plus 2 repairs carrying no scenario.** Four tests carry
a **control** beside the scenario's own — FR-4.1-S4, S6, FR-5.1-S1 and S4, each of
which a fixture the physics cannot tell apart would otherwise satisfy. Command:

```
cargo nextest run -p mc-sim --all-features --no-fail-fast -E 'binary(player_swim_ascent) or binary(player_ground_beats_medium) or binary(non_lifting_medium) or binary(medium_table_width) or binary(player_buoyancy)'
```

- FR-1.1-S2 → `mc-sim/tests/player_swim_ascent.rs` → `a_block_that_states_no_lift_at_all_carries_a_swimmer_at_the_speed_its_own_jump_would`
- FR-4.1-S1 → `mc-sim/tests/player_swim_ascent.rs` → `a_jump_inside_a_block_that_declares_a_lift_leaves_at_the_lift_the_block_declares`
- FR-4.1-S2 → `mc-sim/tests/player_swim_ascent.rs` → `a_declared_lift_of_exactly_one_tick_of_gravity_holds_a_swimmer_at_its_depth`
- FR-4.1-S3 → `mc-sim/tests/player_swim_ascent.rs` → `a_declared_lift_of_zero_arrests_a_sink_without_reversing_it`
- FR-4.1-S4 → `mc-sim/tests/player_swim_ascent.rs` → `a_declared_lift_carries_nobody_the_volume_does_not_hold_up`
- FR-4.1-S5 → `mc-sim/tests/player_swim_ascent.rs` → `a_box_across_two_declared_lifts_rises_at_the_greater_of_them`
- FR-4.1-S6 → `mc-sim/tests/player_swim_ascent.rs` → `a_sink_is_governed_by_resistance_alone_whatever_lift_the_block_declares`
- FR-4.1-S7 → `mc-sim/tests/player_swim_ascent.rs` → `a_lift_past_what_one_tick_may_spend_raises_the_feet_by_the_ticks_own_bound`
- FR-4.1-S8 → `mc-sim/tests/player_swim_ascent.rs` → `an_empty_cell_beside_a_lifting_one_contributes_no_lift_of_its_own`
- FR-4.1-S9 → `mc-sim/tests/player_swim_ascent.rs` → `a_cell_nobody_can_be_held_up_in_contributes_no_lift_to_one_that_holds_a_swimmer`
- FR-4.1-S10 → `mc-sim/tests/non_lifting_medium.rs` → `a_block_nobody_can_be_held_up_in_resolves_to_the_medium_a_cell_holding_nothing_does`
- FR-4.1-S11 → `mc-sim/tests/medium_table_width.rs` → `the_shipped_registry_spends_exactly_one_bit_a_voxel_on_what_medium_it_is`
- FR-5.1-S1 → `mc-sim/tests/player_ground_beats_medium.rs` → `a_jump_from_the_ground_inside_a_lifting_block_leaves_at_the_players_own_jump_speed`
- FR-5.1-S2 → `mc-sim/tests/player_ground_beats_medium.rs` → `a_jump_from_the_ground_in_open_air_leaves_at_the_players_own_jump_speed`
- FR-5.1-S3 → `mc-sim/tests/player_ground_beats_medium.rs` → `a_jump_asked_for_while_falling_through_a_block_nobody_swims_in_neither_lifts_nor_arrests`
- FR-5.1-S4 → `mc-sim/tests/player_ground_beats_medium.rs` → `a_jump_from_the_ground_inside_a_block_declaring_no_lift_at_all_still_leaves_at_the_jump_speed`

### Repairs and fixtures
- `player_buoyancy.rs`'s `a_jump_asked_for_in_midair_outside_any_swimmable_block_changes_nothing_about_the_tick`
  starts from a fall: from rest a `launched` dropping its buoyancy condition
  zeroes both ticks and `(true, false)` held for the wrong reason; from `−2.0` it
  gives `(true, true)`. `adrift()` untouched — three tests read it.
- `medium_table_width.rs` keeps `<= SHIPPED_CEILING` and loses the message's claim
  that the index *"fits in one bit"*, which it never checked; it now names the two
  readings that hold it. Module prose: "both medium questions" → three.
- Seven `solid = false` declarations in `support/medium.rs`, never `chamber.rs`
  (`medium_at` is `NOTHING` there, greening FR-4.1-S9 vacuously): `LIFTING`
  3.5/r0.5, `LIFTING_LESS` 1.5/r0.5, `LIFTING_NOT_AT_ALL` 0/r0.5,
  `LIFTING_BY_DEFAULT` unstated/r0.5, `HOLDING_DEPTH` 0.5/r0, `LIFTING_ABSURDLY`
  9000/r0, `HOLDS_NOBODY_UP` the same unbuoyant.

### RED, measured
`23 tests run: 13 passed, 10 failed`, and `230 tests run: 220 passed, 10 failed`
over the crate — bare counts under `--no-fail-fast`, so the seven fixtures moved
nothing else. Every failure is an assertion: phase 2 opens no non-compiling
window. `fmt --check` then `clippy -D warnings` clean after the last edit. **Six
are green first.** FR-1.1-S2, FR-4.1-S4, FR-5.1-S2 and S3 are the "unchanged by
this feature" controls; FR-4.1-S10 and S11 cannot redden until `VoxelMedium`
carries the field: T05 turns them red, T07 greens them. Tolerances are derived —
`GRAVITY × TICK_DURATION` rounds to exactly `0.5` in `f32`, so the error against
`2.0`, `−1/3`, `17/3` and `−2.5` is nil and the nearest wrong answer ≥ 0.6 b/s.

## Phase 3 — the shipped water, and the golden set re-shoots once

**9 scenarios, 9 tests**, every threshold absolute and every watch derived. `cargo nextest run --workspace --all-features --no-fail-fast`

- FR-6.1-S1 → `mc-sim/tests/shipped_water_declares_a_medium.rs` → `the_shipped_water_declares_the_medium_the_stated_rates_are_derived_from`
- FR-6.1-S2 → `mc-sim/tests/water_carries_a_swimmer_at_stated_rates.rs` → `a_swimmer_asking_for_nothing_sinks_a_stated_distance_in_a_second`
- FR-6.1-S3 → `mc-sim/tests/water_carries_a_swimmer_at_stated_rates.rs` → `a_swimmer_holding_jump_rises_a_stated_distance_in_a_second`
- FR-6.1-S4 → `mc-sim/tests/water_carries_a_swimmer_at_stated_rates.rs` → `a_submerged_walk_at_full_deflection_carries_a_stated_distance_in_a_second`
- FR-6.1-S5 → `mc-sim/tests/the_generated_sea_is_deep_enough_to_swim_in.rs` → `the_deepest_column_of_the_generated_sea_stands_two_water_voxels_over_its_lakebed`
- FR-6.1-S6 → `mc-sim/tests/the_generated_sea_is_deep_enough_to_swim_in.rs` → `a_swimmer_holding_jump_off_the_lakebed_clears_the_first_water_voxel_inside_a_stated_band`
- FR-6.1-S7 → `mc-sim/tests/the_generated_sea_is_deep_enough_to_swim_in.rs` → `a_swimmer_that_stops_asking_to_rise_is_back_on_the_lakebed_inside_a_stated_budget`
- FR-6.1-S8 → `mc-sim/tests/the_generated_sea_is_deep_enough_to_swim_in.rs` → `a_swimmer_holding_jump_for_ten_seconds_surfaces_rather_than_being_expelled`
- FR-7.1-S6 → `mc-render/tests/golden_inventory.rs` → `the_committed_goldens_carry_none_of_the_names_the_superseded_frames_were_shot_under`

**RED**: `1581 tests run: 1573 passed, 8 failed, 1 skipped` — bare, nothing cancelled, and the eight are
these minus S5. All assertions: sank `0.309`/`0.9667`, rose `3.269`/`2.0`, carried `1.731`/`3.0`, crossed
on tick `19` against `25..=45`, sank in `391`/`150`, `360` ticks over `35.1`, `1.6`+`9.0` against
`0.5`+`3.5`, four `r3` directories against none. **S5 is green first**: no geometry moves here, and S5
pins the depth the other three sea scenarios stand on. S1 retargets the snapshot in place, not beside it.

**Tolerance `1e-3`, both directions**: sixty `f32` position sums drift `9.0e-6`/`2.7e-5`/`1.1e-5` against
the same recurrence in `f64`, under sixty half-ulps (`6.0e-5`); the nearest wrong answer is the sink read
as a terminal speed, `1.0` against `0.9667`, 33× it. Never loosened. `support/pool.rs` is shipped
`base:water` through `content_registry()`, refusing a position not in a swimmable volume a block clear of
every solid; its rise starts at `−2.0` and its walk at `−3.0` the wrong way, because a fixture at rest
cannot tell a velocity replaced from one kept. `sea.rs`'s six ids now say what they mean behaviourally.

### T13 — the figures the moved poses touched

Measured on `0ef4933` through `oracle::sighted_samples`, marching the world's own voxels from the
published camera: the independent judge, never a count read off the assertion it feeds. Ticks 59 and 119
moved and tick 0 did not, so every figure of its own reports what it reported before — this reading's
own provenance, which no fresh run could supply.

- terrain samples → `mc-client/tests/replay_oracle.rs` → `PREDICTION_FLOOR`'s doc — `335/383/290` of 576 → `335/386/344`, shares `58/66/50 %` → `58/67/60 %`; moved by the retuned sink
- sky samples → `mc-client/tests/replay_oracle.rs` → module doc — `241/193/286` → `241/190/232`; tick 0 is the roomiest again by 9 of 576, having lost by 45
- control disagreements → `mc-client/tests/replay_oracle.rs` → module doc — `22/30/28` at 3° → `22/30/25`; only tick 0's is asserted and it did not move
- water samples → `the_sea_the_camera_sees_is_the_water_layer.rs` → nothing committed — PRO-904's `56/200/111`, PRO-957's `56/109/124`, now `56/112/204`; the test reports "at least one", so no figure in code moved
- HUD tick coverage → `mc-render/src/capture.rs` → `HUD_CAPTURE_TICKS`' doc — `57.11/71.05/55.00 %` → `57.11/67.62/59.20 %`, so least-covered flips from tick 119 back to tick 0 by 2.09 points. **Production code: measured and reported, not edited here**
- unmoved → `mc-client/tests/replay_oracle.rs`, `replay_determinism.rs`, `support/oracle.rs` → `JUDGED_TICKS`, `SAMPLED_TICKS`, `PREDICTION_FLOOR`, `DISAGREEMENT_BUDGET`, the 32 × 18 grid — the tick lists mirror `DECLARED_CAPTURE_TICKS`, which T12 did not move; the floor now sits 3.35× under the tightest frame rather than 2.9×; the budget of 2 stands against 0/0/0 measured

**No sample straddles the sea's edge**, and the evidence is per pixel rather than a green run: all 372
predicted water pixels draw within ΔE 3.162 of the water layer's mean against a tolerance of 8 — exactly
the layer's own texel spread, so each sits on the layer with 4.8 ΔE to spare. `cargo nextest run
--workspace --all-features --no-fail-fast` → `1581 tests run: 1581 passed, 1 skipped`, a bare count.
`fmt --check` then `clippy -D warnings` clean after the last edit.

### Phase 3 mutation outcomes
Both on `1c593c8`, announced, reverted by hand, `git diff --exit-code` clean after; workspace-wide, `--no-fail-fast`, bare counts. `move_resistance` `0.5 → 0.6` → `1581 tests run: 1576 passed, 5 failed` — declaration, sink, rise, walk, `terrain_goldens`. `swim_ascent` `3.5 → 4.0` → `1581 tests run: 1578 passed, 3 failed` — declaration, rise, surfacing ceiling, **and `terrain_goldens` green**: the guard that was previously the only one cannot see this field at all. Each value has one sharp witness — the sink moves only with the resistance, the ceiling only with the ascent. Recorded in full in `content/base/blocks/water.luau`.

## Phase 4 — what a save records, and editing a medium while the game runs

**9 scenarios, 11 scenario tests, plus 1 additional-coverage test.** Command: `cargo nextest run -p mc-world -p mc-sim -p mc-client --all-features --no-fail-fast`.

- FR-7.1-S1 → `mc-world/src/persistence/format_test.rs` → `every_shipped_blocks_recorded_behaviour_is_the_fold_an_independent_oracle_computes`
- FR-7.1-S1 → `mc-world/tests/save_declarations.rs` → `a_solid_breakable_aimable_block_that_neither_floats_nor_slows_records_the_stated_behaviour`
- FR-7.1-S2 → `mc-world/tests/save_folds_the_medium.rs` → `a_block_that_began_carrying_a_swimmer_upward_records_a_different_behaviour_and_the_same_appearance`
- FR-7.1-S3 → `mc-world/tests/save_folds_the_medium.rs` → `a_block_that_began_carrying_a_swimmer_upward_records_a_different_behaviour_and_the_same_appearance`
- FR-7.1-S4 → `mc-world/tests/shipped_declarations_and_a_revision_3_save.rs` → `every_block_of_a_save_written_before_the_ascent_joined_the_list_is_reported_as_behaving_differently`
- FR-7.1-S4 → `mc-world/tests/shipped_declarations_and_a_revision_3_save.rs` → `that_save_loads_with_its_changed_blocks_accepted_and_the_world_it_holds_comes_back_whole`
- FR-7.1-S5 → `mc-world/tests/shipped_declarations_and_a_revision_3_save.rs` → `that_save_is_refused_and_told_every_block_when_only_unchanged_blocks_are_accepted`
- FR-7.1-S7 → `mc-world/tests/shipped_declarations_and_a_revision_3_save.rs` → `no_block_of_that_save_is_judged_to_look_different_while_every_one_of_them_behaves_differently`
- FR-8.1-S1 → `mc-sim/tests/reload_medium_views.rs` → `a_reload_that_only_lowers_a_swim_ascent_slows_the_very_next_held_jump`
- FR-8.1-S2 → `mc-client/tests/reload_marks_sections.rs` → `a_candidate_that_only_changes_how_fast_a_block_carries_a_swimmer_marks_no_section`
- FR-8.1-S3 → `mc-client/tests/reload_refuses_a_declared_ascent.rs` → `an_ascent_below_zero_leaves_the_content_serving_and_names_the_file_block_and_field`
- FR-8.1-S3 → `mc-client/tests/reload_refuses_a_declared_ascent.rs` → `an_ascent_that_is_not_a_finite_number_leaves_the_content_serving_and_names_the_field`

FR-7.1-S2 and S3 share one test on purpose: that file's instrument is `(behaviour moved, appearance moved)` compared as one value, because a verdict answers behaviour first and cannot witness the pair. Two tests would be one fixture through one door twice, and the enum already makes each half fail for its own defect. FR-7.1-S4 takes two because `resolve` and `load_world` are two entry points reporting two changed lists, and a list assembled in one and lost in the other is invisible to the other's reading. FR-8.1-S3 takes two because a floor comparison catches a negative and never a NaN — measured, both reddened under M2.

### FR-7.1-S1 has three witnesses, and the third was missed at authoring time
`save_declarations.rs` pins one behaviour fold to a hand-derived hash with the whole byte table in its doc comment, and it was **not** in the phase-4 contract — the tree went red on it after T15 landed and it was repaired under arbitration (`test-wrong`). `docs/technical/world-format.md:803-823` predicts exactly this miss: a needle spelled after `REVISION` cannot find that file, because it states its version as a hand-written `03` byte and calls it "version 3" in prose. Read the three files; do not re-derive the list from that grep. The new constant `VERSION_4_BEHAVIOUR_OF_PINNED = 0xb808_ebfd_74e6_f12a` was derived by hand over the twenty-nine-byte input in a scratch computation sharing no code with the writer, and the method was validated first against FNV's published vectors and then against **all three** constants it supersedes on their own inputs — `0x5e9d_3089_5b2e_0d5f` at 19 bytes, `0xbee1_336f_0dc4_f79d` at 20, `0x3e58_43bf_8f20_37bc` at 25 — before the run of the code under test was looked at. The four appended bytes are `00 00 10 41`, the fixture's stated `9.0`: the loader's default, not the fold identity, and masked to `0.0` for a block nothing can swim in.

### Additional coverage — what it catches
- fixture integrity → `mc-world/tests/shipped_declarations_and_a_revision_3_save.rs` → `the_committed_revision_3_save_really_does_hold_all_four_of_the_blocks_the_base_game_ships` — a name table lost from a fixture nobody can mint again. Green from birth, and it is what stops the four readings above comparing four blocks against none of them.

### The revision-3 fixture was minted before the list grew and cannot be minted again
`crates/mc-world/tests/fixtures/world_saved_against_behaviour_revision_3.mcw`, written **2026-08-27** from the tree at **`6c2ed61`** by the shipped writer of that day, against `content/base/` as it then stood — behaviour revision 3, appearance revision 3, water at `move_resistance = 0.5` and `swim_ascent = 3.5` — holding all four base blocks at `(1,1,1)`, `(2,3,4)`, `(5,8,13)` and `(15,34,15)`. `BEHAVIOUR_REVISION` is a compile-time constant, so once T15 lands no run can produce a revision-3 save again. At the moment of minting, its behaviour records were byte-identical to a save minted today, which is exactly what makes FR-7.1-S4, S5 and S7 red rather than green on arrival. The revision-2 fixture does not serve: it is stale by two list growths, so it reports the same four names whether the ascent was appended, the byte moved, or both.

### The appearance byte standing at 3 was checked, not assumed
FR-7.1-S3 is **not** that guard — it compares two appearance hashes to each other and stays green if both move together. Measured (M4): bumping `APPEARANCE_REVISION` 3 → 4 reddens exactly three tests, and the third was not predicted — `format_test.rs`'s appearance oracle, `save_per_face_appearance.rs`'s stated byte sequence, and `shipped_declarations_and_a_revision_2_save.rs`'s appearance-standing-still reading. The revision-3 file's own equivalent could not report it, being already red for its own reason: "red for a known reason hides red for an unknown one", recorded while it is still measurable.

### The tolerance on the reloaded rise, derived from both directions
`RISE_RELOADED = 0.6667` against `TOLERANCE = 1e-4` in `mc-sim/tests/reload_medium_views.rs`. From below: `GRAVITY · TICK_DURATION` is `30.0 × 0.016666668` = `0.50000003`, which rounds to exactly `0.5` in `f32` (0.4375 ulp), so both subtractions are exact and the single division is correctly rounded — one ulp near `0.667` is `6.0e-8`. What dominates is the specification's own rounding of `2/3` to four places, `3.3e-5`, so the floor is `3.4e-5` and this sits three times above it. From above: the nearest wrong answer is a rise of `1.0` — the reloaded ascent with gravity's bite skipped, or with the medium's division skipped — a gap of `0.333`, three thousand times this; a stale view reports the served `2.0` twice, a gap of `1.333`, and a fall back on the loader's default gives `5.667`. The served `2.0` is exact in `f32` and is read through the same tolerance rather than by bits, so the two readings are one comparison.

### Phase 4 mutation outcomes
All on the phase-4 test tree, announced on the shared tree, each reverted by hand with `git diff --exit-code` clean before the next. Command throughout `-p mc-world -p mc-sim -p mc-client --all-features --no-fail-fast`; every summary a bare count, so no run was cancelled. Baseline `1050 tests run: 1044 passed, 6 failed`, the six being this phase's RED.

- **M4** `APPEARANCE_REVISION` 3 → 4 (`persistence/format.rs`) → `1050 tests run: 1041 passed, 9 failed`; three added, named above. The falsifier for a constant nothing asserts directly.
- **M1** `swim_ascent` added to `drawn_of`'s key (`world/reload.rs`), the placement an implementer reaches for while editing that doc comment → `1050 tests run: 1043 passed, 7 failed`. Exactly one added: `a_candidate_that_only_changes_how_fast_a_block_carries_a_swimmer_marks_no_section`. It is the sole reporter, because every other reload fixture states no ascent and their keys compare `9.0` to `9.0`.
- **M2** `declared_ascent` falls back to the default instead of propagating the fault (`luau_declaration/number.rs`) → `1050 tests run: 1036 passed, 14 failed`; eight added, both new reload-refusal tests and the six phase-1 launch-time refusals. The pair is what says the reload's build stage still reaches the loader's own refusal rather than a second reader.
- **M3** `World::adopt` keeps the resolved views it had (`world/mod.rs`) → `1050 tests run: 1021 passed, 29 failed`; twenty-three added, including `a_reload_that_only_lowers_a_swim_ascent_slows_the_very_next_held_jump`. Broad by construction — it removes the re-resolve every reload view depends on — so it establishes that the new test sits on the reload path and **not** that it isolates the ascent. The isolating half is M1's green: nothing else in 1050 tests moved when the ascent was routed into the geometry key.

**All four FR-8.1 tests were green on arrival, reported rather than papered over.** The hot-reload half of this phase adds no mechanism: `World::adopt` already re-resolves the whole volume and rebuilds the medium table, `changes_geometry` already excludes every medium field, and the loader already refuses a bad ascent through the reader `move_resistance` shares. So they passed the moment they compiled, and their falsifiability comes from M1, M2 and M3 rather than from a red run. FR-7.1's six were red on arrival, six of six.
