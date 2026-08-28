# Test map — SPEC-031

Scenario → test file → test name. Scenario IDs live here and in commit messages
only. Extra tests are under **Additional coverage** with what each catches.

Test commands:

```
cargo nextest run -p mc-world --no-fail-fast --test luau_declaration_opacity \
  --test luau_declaration_opacity_refusals --test luau_declaration_keys \
  --test save_folds_a_declared_opacity --test save_per_face_appearance \
  --test shipped_declarations_and_a_revision_3_save
cargo nextest run -p mc-client --no-fail-fast --test documented_declaration_fields \
  --test documented_property_refusals --test the_all_opaque_recording_is_of_the_world_it_names
```

## T01 — the all-opaque recording (no scenario; FR-5.1-S3 is claimed by T12 in phase 3)

T01 → `crates/mc-client/tests/fixtures/all_opaque/` + `tests/support/all_opaque.rs` → the recording taken at `74794b9` and the harness that produced it. The comparison `predicted() == recorded()` belongs to T12, where the oracle's second rule gives it something to say; written in phase 1 it asserts that a reading of this tree matches this tree. Six guards keep the fixture from rotting, all green on the tree that made it — a fixture guard's red state is a tree that damaged the fixture.

### Additional coverage — `crates/mc-client/tests/the_all_opaque_recording_is_of_the_world_it_names.rs`

- → `the_recording_covers_every_declared_sample_of_every_declared_viewpoint` — truncation, a dropped viewpoint, a camera the values were never marched from, reordered samples: four failures of one enumerated comparison.
- → `the_recording_reads_back_as_the_one_format_it_is_written_in` — a line the parser absorbs without it reaching a comparison. Measured: a duplicated `eye` line reddens only this; a deleted sample line reddens only the reading above. A pair, neither subsuming the other.
- → `the_recording_names_exactly_the_classes_the_all_opaque_world_can_offer` — both directions at once: a collapsed recording and one naming an unknown block.
- → `the_all_opaque_root_generates_the_world_the_shipped_root_generates` — the fixture root drifting, which would leave the recording about a private world. Measured: block **ids** may differ without moving a voxel, so only the four names bind.
- → `no_block_the_all_opaque_root_declares_states_an_opacity` — a later edit adding the field, which would silently change what T12 compares rather than failing it.
- → `the_same_reading_reports_a_declaration_that_does_state_an_opacity` — the positive control: a blinded scan answers "no" as loudly as a clean root.

## Phase 1 — a block declares an opacity

**FR-1.3-S2 was written after the tests were.** It began as a test this author added while ruling on an FR-1.3-S1 dispute — a solid block passing light owes a *different* sentence, because the one for a written `occludes = true` quotes a line such a file does not contain. The lead made it contract rather than leave it a test asserting more than the spec required, which is the collision this file exists to surface. Scenario count 44 → 45.

### T04 — the loader

- FR-1.1-S1 → `mc-world/tests/luau_declaration_opacity.rs` → `a_declaration_stating_half_a_degree_registers_at_exactly_that_degree`
- FR-1.1-S2 → `mc-world/tests/luau_declaration_opacity.rs` → `a_declaration_that_states_no_degree_registers_stopping_all_the_light`
- FR-1.1-S3 → `mc-world/tests/luau_declaration_opacity.rs` → `two_blocks_stating_two_degrees_are_each_registered_at_their_own`
- FR-1.1-S4 → `mc-world/tests/luau_declaration_opacity.rs` → `a_degree_at_either_end_of_the_range_registers_and_the_root_is_refused_neither_time`
- FR-1.2-S1 → `mc-world/tests/luau_declaration_opacity_refusals.rs` → `a_degree_written_as_a_string_is_refused_naming_the_kind_a_number_is`
- FR-1.2-S2 → `mc-world/tests/luau_declaration_opacity_refusals.rs` → `a_degree_below_zero_is_refused_naming_the_field_and_the_floor`
- FR-1.2-S3 → `mc-world/tests/luau_declaration_opacity_refusals.rs` → `a_degree_above_one_is_refused_naming_the_field_and_the_ceiling`
- FR-1.2-S4 → `mc-world/tests/luau_declaration_opacity_refusals.rs` → `a_degree_that_is_not_a_number_is_refused_for_that_and_never_for_the_floor`
- FR-1.2-S5 → `mc-world/tests/luau_declaration_opacity_refusals.rs` → `a_degree_without_bound_is_refused_for_finiteness_and_never_for_the_ceiling`
- FR-1.3-S1 → `mc-world/tests/luau_declaration_opacity_refusals.rs` → `a_block_that_passes_light_and_also_hides_what_is_behind_it_is_refused_naming_both_fields`
- FR-1.3-S2 → `mc-world/tests/luau_declaration_opacity_refusals.rs` → `a_solid_block_that_passes_light_is_refused_and_told_which_line_makes_it_occlude`
- FR-1.4-S1 → `mc-world/tests/luau_declaration_keys.rs` → `a_field_one_letter_past_a_real_one_is_refused_quoting_every_field_in_declaration_order`

### T05 — persistence

- FR-5.3-S1 → `mc-world/tests/shipped_declarations_and_a_revision_3_save.rs` → `a_world_saved_before_the_degree_existed_opens_with_every_block_stopping_all_the_light`
- FR-5.3-S2 → `mc-world/tests/save_folds_a_declared_opacity.rs` → `a_declared_degree_joins_the_appearance_record_and_moves_only_that_revision`

### T06 — the modding pages

- FR-7.2-S1 → `mc-client/tests/documented_declaration_fields.rs` → `every_refusal_the_modding_pages_quote_lists_every_field_a_declaration_may_state`
- FR-7.2-S1 → `mc-client/tests/documented_declaration_fields.rs` → `the_guide_tabulates_every_field_with_the_value_its_absence_means`
- FR-7.2-S2 → `mc-client/tests/documented_declaration_fields.rs` → `a_page_quoting_a_list_a_field_short_is_reported_with_the_list_it_quotes`

### Additional coverage

- → `luau_declaration_opacity.rs::a_degree_of_signed_nothing_is_retained_as_the_unsigned_zero_a_save_folds` — the `-0.0 → 0.0` normalisation, which no value comparison can see and which a save folds by bits. Nothing else watches whether this field still reaches it once the reader has a ceiling.
- → `luau_declaration_opacity.rs::a_degree_written_as_a_whole_number_registers_as_the_same_degree_a_fraction_does` — a reader taking one `ScriptValue` numeric variant, which refuses half of what an author types.
- → `luau_declaration_opacity_refusals.rs::each_half_of_that_contradiction_registers_on_its_own` — the cross-field rule over-firing. Its second half is the ordinary opaque occluding block, so an over-wide rule refuses every content root in existence.
- → `luau_declaration_keys.rs::a_declaration_stating_every_recognised_field_and_nothing_else_registers` — an unrecognised-field check that over-fires; states all thirteen so it moves when the contract grows.
- → `luau_declaration_keys.rs::a_field_the_loader_does_not_recognise_is_refused_beside_the_ones_it_does` — the refusal naming the offender at all, a different path from the ordered-list comparison.
- → `documented_property_refusals.rs::the_guide_introduces_the_declaration_fields_in_the_order_a_refusal_quotes_them` — the guide's hand-written order against a **real run**, which is what stops the mirror agreeing with whatever the loader becomes. Goes green at T04, not T06.
- → `documented_refusals.rs::every_refusal_the_modding_pages_quote_is_a_refusal_the_client_prints` — a standing guard this spec newly obligates: the pages now quote three opacity refusals, so `support/opacity_refusals.rs` produces all three from real runs. It carries no scenario of this spec; without the producers a page could quote a sentence the program never says.
- → `documented_declaration_fields.rs::the_guide_states_the_bound_an_opacity_is_kept_within` — the page promising a bound the loader does not keep; the first ceiling on this declaration an author can reach.
- → `mc-world persistence::format::tests::every_shipped_blocks_recorded_appearance_is_the_fold_an_independent_oracle_computes` — the second of the two hand-built byte oracles; it is what caught the appearance bump landing with its oracle still at revision 3. Blind to a fold hardcoding `1.0`, because every shipped block states nothing; that witness is `save_folds_a_declared_opacity.rs`.
- → `shipped_declarations_and_an_appearance_revision_3_save.rs::every_block_of_that_save_is_reported_retextured_and_not_one_of_them_as_behaving_differently` — the **`retextured` arm, never exercised before**: every other committed save is stale in behaviour too, so the list stayed empty in all of them and a wrongly-computed one looked identical to a correct one.
- → `shipped_declarations_and_an_appearance_revision_3_save.rs::that_save_loads_for_a_player_who_asked_to_be_stopped_if_anything_moved` — the point of the whole split: route the degree onto the behaviour list and this exact call is refused for every player at once, over a rendering number. No other save can show it.
- → `shipped_declarations_and_an_appearance_revision_3_save.rs::the_committed_appearance_revision_3_save_really_does_hold_all_four_of_the_shipped_blocks` — a save needing nothing produces the empty lists the reading above asserts for two of its three.
- → `shipped_declarations_and_a_revision_2_save.rs::every_block_of_that_save_is_judged_to_look_different_as_well_as_to_behave_differently` — as its revision-3 twin; both asserted `looks the same` across two behaviour moves and this spec is what ends it.
- → `save_per_face_appearance.rs::an_unchanged_declaration_records_the_appearance_the_stated_byte_sequence_folds_to` — the degree appended among the six keys rather than after the flags, which moves every byte after it.
- → `shipped_declarations_and_a_revision_3_save.rs::every_block_of_that_save_is_judged_to_look_different_as_well_as_to_behave_differently` — an appearance byte bumped without its list growing, and a list grown without the byte moving.

### Mutations, with what actually happened

Window announced and closed by the implementer; `git diff --exit-code` clean after each revert. **Both outcomes recorded, including the one that did not bite.**

- **`OPACITY_BY_DEFAULT` `1.0` → `0.0`** → `1614 tests run: 1262 passed, 352 failed`. Blunt in proportion, and the finding is the coupling: with the cross-field refusal landed, a default below one makes every shipped solid block a contradiction and the whole content root is refused. The absent-field default and FR-1.3-S1 became load-bearing for each other, and **no mutation separates them**. `a_declaration_that_states_no_degree_registers_stopping_all_the_light` reddened (FR-1.1-S2).
- **The same mutation did *not* reach FR-5.3-S1**, because its registry was assembled from `BlockDefinition` literals and the Luau loader never ran — a stated falsifier the test could not serve. **Repaired rather than re-labelled**: the registry is now built by reading four declarations that state no degree, so the loader's default is in the path and the mutation reddens it on the value. `occludes = false` is stated on all four so it reddens on the degree and not on a refusal.
- **`declared_degree` made to discard the reader's value** → baseline + 6: the four registration readings, plus **both** cross-field refusals, which were not predicted and belong on the list — they confirm the refusal depends on the stated degree reaching it rather than on the field merely being present. `a_declaration_that_states_no_degree…` stayed green, which is the point: this mutation separates the default from the retention of a stated value where the one above conflates them.

### Green at authoring

`testing.md` §2 replaces RED with a mutation where a scenario is already satisfied, and three readings here were never red: **FR-1.1-S2** and **FR-5.3-S1** (T02 hardcoded `Opacity::OPAQUE`; both now have a falsifier that reaches them), and **FR-7.2-S2**'s control (`a_page_quoting_a_list_a_field_short…`), which is green by design and must stay green. The three `shipped_declarations_and_an_appearance_revision_3_save.rs` readings are guards over a fixture that could only be minted mid-spec, authored after T05 and green from the first run; their red state is a tree that moved the wrong revision byte.

## Phase 2 — the renderer gains a blended pass

Test commands:

```
cargo nextest run -p mc-render --lib --no-fail-fast -E 'test(texture::mip::tests)'
cargo nextest run -p mc-client --no-fail-fast \
  --test an_image_without_an_alpha_channel_is_opaque \
  --test a_declared_translucency_shows_what_is_behind_it \
  --test only_a_declared_opacity_makes_the_terrain_blend \
  --test two_translucent_surfaces_compose_without_a_seam
```

### T08 — alpha through the mip chain and the decoder

- FR-3.1-S1 → `mc-render/src/texture/mip_test.rs` → `two_clear_texels_and_two_opaque_ones_reduce_to_the_stored_mean_and_not_the_lit_one`
- FR-3.1-S2 → `mc-render/src/texture/mip_test.rs` → `four_opaque_texels_reduce_to_one_that_is_still_opaque`
- FR-3.1-S3 → `mc-render/src/texture/mip_test.rs` → `every_alpha_a_supplied_image_carries_reaches_level_zero_where_it_stood`
- FR-3.2-S1 → `mc-client/tests/an_image_without_an_alpha_channel_is_opaque.rs` → `a_source_image_carrying_no_alpha_channel_fills_its_layer_opaque`

### T08 — what a declared degree draws (T10 makes these green)

- FR-2.1-S1 → `mc-client/tests/a_declared_translucency_shows_what_is_behind_it.rs` → `a_pane_at_half_a_degree_shows_the_wall_through_it_and_leaves_the_rest_of_the_wall_alone`
- FR-2.1-S2 → `mc-client/tests/a_declared_translucency_shows_what_is_behind_it.rs` → `a_pane_at_a_whole_degree_hides_the_wall_behind_it`
- FR-2.1-S3 → `mc-client/tests/a_declared_translucency_shows_what_is_behind_it.rs` → `a_pane_at_a_quarter_of_a_degree_shows_three_quarters_of_the_sky_behind_it`
- FR-2.2-S1 → `mc-client/tests/a_declared_translucency_shows_what_is_behind_it.rs` → `an_opaque_block_in_front_of_a_pane_is_drawn_with_nothing_mixed_into_it`
- FR-2.2-S2 → `mc-client/tests/a_declared_translucency_shows_what_is_behind_it.rs` → `a_pane_at_no_degree_at_all_leaves_the_wall_behind_it_exactly_as_it_was`
- FR-2.3-S1 → `mc-client/tests/only_a_declared_opacity_makes_the_terrain_blend.rs` → `a_world_whose_every_block_stops_all_the_light_leaves_no_pixel_unaccounted_for`
- FR-2.3-S2 → `mc-client/tests/only_a_declared_opacity_makes_the_terrain_blend.rs` → `redeclaring_one_block_at_half_a_degree_puts_pixels_at_a_colour_no_layer_holds`
- FR-3.3-S1 → `mc-client/tests/two_translucent_surfaces_compose_without_a_seam.rs` → `a_textures_own_alpha_multiplies_the_degree_its_block_declares`
- FR-4.1-S1 → `mc-client/tests/two_translucent_surfaces_compose_without_a_seam.rs` → `two_separated_panes_over_one_wall_compose_to_a_colour_neither_reaches_alone`
- FR-4.2-S2 → `mc-client/tests/two_translucent_surfaces_compose_without_a_seam.rs` → `two_adjacent_cells_of_one_kind_draw_one_unbroken_run_and_a_painted_seam_is_reported`

### FR-3.3-S1's wording moved to meet the model, and the test did not move

Recorded here because a reader reaches for this file rather than for the archive. The scenario originally asked for *the even blend* from a texture carrying alpha `128`, which under the product model is unreachable: an even blend from that texture needs a declared degree of one, and a block at one degree draws in the opaque pass and never blends. Worse, its `WHERE` clause failed under both readings — as *takes alpha from the texture at all* the scenario contradicts a binding decision, and as *takes alpha from the texture alone* it fires never and leaves FR-3.3 with no falsifier. **It was amended to the product model (D-I5) and the test was not changed**, because the test had been written to `architecture.md` Decision 2 from the start.

What survives from that reasoning, and is the point of the reading's shape: **two panes differing in exactly one byte is what makes the texel's alpha falsifiable.** One pane says nothing — whichever colour it draws is consistent with an alpha taken from the declaration alone, from the texture alone, or from the product, because a single number can be arrived at three ways. Two panes of one colour at one degree, separated only by the alpha their textures carry, separate all three: the declaration alone puts both on one colour, the texture alone puts the sheer pane at an even blend rather than at a quarter, and only the product puts each where the census expects. The two expected colours stand **ΔE 26.43** apart, better than four times the tolerance.

### The fixture, and the two numbers no assertion can enforce

Every reading above draws flat-coloured panes standing one behind another in one section, through a content root written for that reading. Three block colours: a wall at `(32, 200, 90)`, a pane at `(235, 120, 40)`, a blocker at `(120, 40, 160)`, against the sky at `(135, 206, 235)`. **The closest two colours any reading must tell apart stand ΔE 15.40 apart** — the sky against a quarter of the pane over it. The next two are the pane's own colour against two panes composited over the wall at ΔE 25.06, and one pane over the wall against two at ΔE 25.38. `pixel_census::require_told_apart` asserts that separation against twice the tolerance on every run, so the fixture's own premise is checked rather than quoted.

The tolerance is **ΔE 6.0**, derived from both directions and stated at `translucency::TELLS_THEM_APART`. Floor near **3.2**: flat layers give a texel spread of 0.00; the byte the declared degree quantises to moves the two-pane composite by ΔE 0.47 and every other composite by 0.00; a code value or two of rounding through the eight-bit sRGB attachment is worth ΔE 2.68 at the very worst. Ceiling **7.70**, which is half the ΔE 15.40 separation — past it a pixel can belong to two named colours at once. The sRGB-byte-space answer, the trap FR-2's preamble names, stands **ΔE 15.60** from the correct half blend and **ΔE 9.72** from the correct quarter blend, so an expectation computed in the wrong space is red rather than merely imprecise.

Every layer is one flat colour, deliberately. It removes the texel spread from the error budget, makes each expectation an exact triple, and makes the reading independent of minification — nearest magnification, linear minification and mip interpolation all answer the same byte for a texture of one colour.

### Additional coverage

- → `mc-render/src/geometry/vertex_test.rs::packing_and_unpacking_a_vertex_returns_every_field_it_went_in_with` — renamed from `…returns_its_position_facing_and_layer` and widened to assert the **section** and the **opacity** as well. The new field is packed directly above the section index, which is the one edit that can shift its neighbour without shifting anything else; the old reading carried section 7 and never asserted it, so a great many wrong shifts came back looking correct. The opacity is asserted as `128 / 255` and not as the declared `0.5`, because the round trip through eight bits is deliberately not the identity — a reading demanding the declared number back would be red against a correct packer.
- → `mip_test.rs::two_clear_texels_and_two_opaque_ones…`, its second element — the linear-light answer is computed on the spot from the module's own transfer pair and required to stand above 180. A pair that ever brought the two averaging rules within a byte of each other reports itself, instead of leaving an assertion that names one number and can no longer tell it from the other.
- → `an_image_without_an_alpha_channel_is_opaque.rs::require_no_alpha_channel` — reads the committed PNG's own IHDR colour-type byte, sharing no code with the decoder under test. A fixture re-saved *with* an alpha channel would otherwise leave that reading green while asking nothing at all.
- → the same reading asserts the whole level-zero image rather than the alpha channel alone: a decoder that filled alpha correctly and then dropped the picture, falling back to the generated texture, satisfies every alpha-only assertion.
- → `pixel_census::require_told_apart` is driven by all ten frame readings and is the assertion that a fixture's colours are far enough apart — the half of a tolerance that is otherwise held only by whoever wrote the palette.

### Mutations, with what actually happened

Window announced to the conductor and closed by message; both reverted **by hand**, never `git checkout --`. Baseline immediately before, unscoped and complete: `cargo nextest run --workspace --no-fail-fast` → `1617 tests run: 1617 passed, 1 skipped`.

- **`mip.rs` `covered_average`: alpha averaged in linear light instead of `mean_of_stored`** → `1621 tests run: 1620 passed, 1 failed, 1 skipped`. Exactly one red, `two_clear_texels_and_two_opaque_ones_reduce_to_the_stored_mean_and_not_the_lit_one` (FR-3.1-S1), and nothing else moved. Reverted; the file's remaining diff is documentation only.
- **That mutation did *not* reach FR-3.1-S2 or FR-3.1-S3, and that is a finding rather than a gap.** Both averaging rules answer 255 for a constant 255, so S2 cannot discriminate them by construction — what it is for is that a level of ordinary opaque art does not fade as it minifies, which an alpha dropped or scaled would break. S3 reads level zero, which is the input verbatim and reaches no reduction at all. Three scenarios, three different code paths.
- **`decode.rs`: `.to_rgba8()` → `.to_rgb8()` with the alpha written as 0** → `1621 tests run: 1616 passed, 5 failed, 1 skipped`. Red: `a_source_image_carrying_no_alpha_channel_fills_its_layer_opaque` (FR-3.2-S1), plus four HUD readings — `hud_goldens`, both `hud_held_block` readings, and `hud_prediction`. Reverted; `git diff --exit-code` on that file is clean.
- **The four collateral reds are all HUD, and that is the second finding.** The HUD pass is the only blended target in the workspace today, so it is the only thing that can see a zeroed alpha. Every terrain reading and every terrain golden stayed green, because the terrain fragment ignores alpha. Once the blended terrain draw lands that stops being true, and `terrain_goldens` begins carrying evidence it does not carry now.

### The draw count moved one → two, and what the repair had to preserve

`architecture.md` Decision 4 gives terrain two `draw_indexed_indirect` calls in one pass, so `TERRAIN_DRAW_CALLS` moved `1` → `2` and four assertions pinned the old number. Verdict **`test-wrong`**: the design moved deliberately and by a binding decision, and a statistic reporting `1` would be false about the frame.

**Four sites, in two files** — three in `terrain_offscreen.rs` (`:161`, `:215`, `:263`) and one in `frame_statistics.rs` (`:65`). Two of the four sit inside composite tuples, where the field and its expected value are lines apart, so a search for the pairing walks past them; the `frame_statistics.rs` one is in a different binary again and was missed by a reading scoped to `terrain_offscreen.rs`.

**The repair is an equality and never a range.** `:215`'s purpose was never "terrain costs one draw" — it is *a call per section is the regression this counts*, and that purpose is untouched at two, because two is still constant in the number of sections while the regression answers 64. Loosening to `>=` or to a band would accept exactly what the reading exists to catch. Three test names carried the old count and moved with it; the prose now interpolates the constant, so the sentence cannot go stale against the number.

**`TERRAIN_DRAWS` is written by hand in both files and deliberately not imported from `mc_render::snapshot`.** It is a control: an assertion comparing a reported statistic against the very constant production computed it from agrees with itself and can never fail. The duplication is the point, and both declarations say so, because a constant whose reason is unwritten is one somebody tidies away.

### Additional coverage — two gaps the moved constant exposed

- → `mc-render/src/geometry/mod_test.rs::quads_that_stop_all_the_light_are_emitted_before_those_that_pass_some_of_it` — **nothing pinned the partition.** Under one draw there was nothing to split; under two there is a new observable, and every draw-*count* assertion is satisfied at `2` by a packer that put every quad in one half. The frame readings in `mc-client` do catch both directions, but they need a device and are skipped by `MYCRAFT_ALLOW_NO_GPU` — which left the partition with zero witnesses on a machine without one. This is the direct assertion on the shared derivation: three blocks whose sorted order is deliberately not their emission order, so no partition gives `[1, 2, 0, 1]`, a sort by layer gives `[0, 1, 1, 2]`, a reorder within a half gives `[0, 2, 1, 1]`, and only the correct answer gives `[2, 0, 1, 1]`.
- → `mc-render/tests/shader_validation.rs::the_validators_vertex_layout_is_checked_against_the_bits_packing_actually_writes` — `build/validate.rs` gained `VERTEX_LAYOUT`, a private copy of the packed vertex's shifts, with no agreement test; that file's own header states why one is owed, and `QUAD_INDEX_PATTERN` and `PLANE_AXES` both have one. Tied back to **what packing emits** rather than to `vertex.rs`'s private constants: each field is set to its own lowest step with every other field zero, so the word that comes out is a single bit and that bit names the shift. `Opacity::CLEAR` is not that step — it is the smallest *degree* and encodes to the byte zero, leaving the field empty and the reading vacuous; the smallest *byte* is what the fixture uses.
- → `…::the_validators_section_record_is_checked_against_the_stride_a_scene_writes` — the same for `SECTION_RECORD`, compared against the bytes `section_bytes` actually writes rather than against `SECTION_RECORD_BYTES`, because two constants agreeing with each other is the shape being guarded against. A one-section scene and a two-section one are both read, so a table off by a field is a different failure from a writer that emitted one record and stopped.

### Mutations on the validator's private copies, and what they actually showed

Window announced and closed; every revert by hand, `git diff --exit-code` confirmed on `build/validate.rs` and both shaders. Baseline `cargo nextest run -p mc-render --no-fail-fast` → `138 tests run: 138 passed` — **scoped to `mc-render`**, defensible because the fault is local to that crate and its build script, and said here rather than left to be assumed.

- **`VERTEX_LAYOUT`'s `OPACITY_SHIFT` 36 → 35, alone** → **the crate failed to compile.** The build script compares its table against the shader, so the build check fires first and the agreement test never runs. **That is not evidence the agreement test bites** — an uncompiled suite cannot report a test red, only fail to report it green — and it is recorded as a build failure rather than as a test result.
- **The same shift moved in `terrain.wgsl` too, so the validator and the shader agree with each other and only the Rust packer differs** → `138 tests run: 137 passed, 1 failed`. Exactly one red, `the_validators_vertex_layout_is_checked_against_the_bits_packing_actually_writes`, and nothing else moved. This is the drift the test exists for and it is the only thing in the suite that sees it.
- **A field dropped from `SECTION_RECORD` and from both shaders** → naga refuses `cull.wgsl` with `invalid field accessor`, because that stage *reads* the field. Not isolable in this direction, and the finding is which half has an independent witness: a field a stage reads is covered by naga, a field it only declares is not.
- **What none of these can separate**, and it is left deliberately unrun: moving the **real** `Vertex` or `SectionRecord` and leaving `validate.rs` untouched. Every mutation above reddens a test anchored to the real type *and* one anchored to a third copy alike, so it proves the test reads the validator's table — which was never in doubt. The complementary mutation is the decisive half and belongs to whoever runs it cold, without knowing this answer.

### The content seam, asserted at the boundary rather than inferred from a picture

`mc-sim/src/content.rs`'s `resolved_from` copies a `BlockDefinition` into a `ResolvedBlock`, and it is the one line deciding whether a declared degree reaches anything that draws. **Measured before the test was written: every `ResolvedBlock` in the suite was a hand-built literal — not one came out of `resolved_from`** — so the shipped construction was observed only by rendered frames, all of which need a device.

- → `mc-sim/tests/resolved_client_content.rs::every_blocks_declared_degree_of_opacity_reaches_the_content_a_client_receives` — through the public door, `mc_sim::content::load` over a root declaring three blocks: one at `0.25` with `occludes = false` written out, one stating `1.0`, one stating no degree at all. Asserted as **the value beside the name that declared it**, never as "not opaque" and never as a difference between two blocks — a copy handing back a constant, a default, or one block's degree under another block's name is then three distinct failures rather than one silence. `0.25` rather than `0.5` so the expectation cannot be confused with a midpoint default.

**`occludes = false` is written out on the block below one degree**, or `defaulting_to_solidity` makes it a block that both passes light and hides what lies behind it, FR-1.3-S2 refuses the whole root, and the reading fails for a reason that has nothing to do with the seam.

### Mutation on the seam, with the prediction written down first

Prediction recorded before the run: **eight red**, seven of them frame readings needing a device and one device-free; and with `MYCRAFT_ALLOW_NO_GPU` set, **exactly one**. No second-order path was predicted, because this mutation substitutes a value rather than moving a field — unlike a shift moved into a neighbour's bits, it cannot corrupt an unrelated quantity.

Baseline `cargo nextest run --workspace --no-fail-fast` → `1635 tests run: 1635 passed, 1 skipped`.

- **`resolved_from`'s `opacity: definition.opacity` → `Opacity::OPAQUE`** → `1635 tests run: 1627 passed, 8 failed, 1 skipped`. Eight red, and they are the eight predicted, name for name: the seam reading above, `redeclaring_one_block_at_half_a_degree…`, the three `a_declared_translucency…` readings at a quarter, a half and none at all, and all three of `two_translucent_surfaces_compose_without_a_seam`.
- **What stayed green, and each for the reason predicted**: every `mc-world` loader reading (they read `BlockDefinition`, which the mutation does not touch); every golden and every terrain probe (no shipped block declares a degree below one); FR-2.1-S2, FR-2.2-S1 and FR-2.3-S1 (none needs a degree below one to reach a fragment); the two older `resolved_client_content` readings (neither states an opacity); the save readings (they fold `BlockDefinition`).
- **The device-free half, measured rather than derived.** With the seam mutation still in place, `frames::device()` was stacked to offer no device at all: `1635 tests run: 1634 passed, 1 failed, 1 skipped`. **Exactly one** — the seam reading. Seven of the eight witnesses vanish with the GPU, and before this test the count on a machine without one was zero.

Both mutations reverted by hand; `git diff --exit-code` clean on `mc-sim/src/content.rs` and `mc-client/tests/support/frames.rs`.

### What the build-time layout check is blind to

Recorded here because it is a property of the guard rather than of one change, and because `build/validate.rs`'s header carries only the other half. Three directions, all measured:

| What moves | Build | What reddens |
|---|---|---|
| `VERTEX_LAYOUT` alone | **fails** | nothing — an uncompiled suite reports no test at all |
| `VERTEX_LAYOUT` and `terrain.wgsl` together | green | the agreement test alone |
| **`vertex.rs`'s own shift alone** | **green** | the agreement test, and six frame readings |

The third is the direction real drift travels, because nobody edits the validator's copy by accident — and it is the one the build check cannot see, since the table and the shader go on agreeing with each other about a number neither reads from the type. With `MYCRAFT_ALLOW_NO_GPU` set, `the_validators_vertex_layout_is_checked_against_the_bits_packing_actually_writes` is the only test in the workspace that observes the real packed opacity. That is stated in the test itself and in `build/validate.rs`'s header, on both sides deliberately, so neither can be deleted in ignorance of the other.

### Green at authoring

`testing.md` §2 replaces RED with a mutation where a scenario is already satisfied, and five readings here were never red.

- **FR-3.1-S1, FR-3.1-S2, FR-3.1-S3 and FR-3.2-S1** describe behaviour this tree already has. Their falsifiers are the two mutations above; S2 and S3 are named there with why neither mutation reaches them.
- **FR-2.1-S2** (a pane at a whole degree draws unblended) and **FR-2.2-S1** (an opaque blocker in front of a pane) hold because nothing blends yet. What reddens them is a blended pass that blends the wrong faces: one partitioning on anything other than the declared degree puts FR-2.1-S2's pane into the blended draw, and one ignoring the depth the opaque pass wrote mixes FR-2.2-S1's hidden pane into the blocker.
- **FR-2.3-S1** (an all-opaque world classifies with every pixel accounted for) is FR-2.3-S2's control and must stay green. It reddens if the blended pass blends a face no declaration asked it to.
- **FR-4.2-S2's positive-control half is already live**: the painted-seam frame is reported as a seam on today's tree, so that reading is red only on its rendered half — which is the half T10 makes green.

## Phase 3 — the sea goes see-through

Test commands:

```
cargo nextest run -p mc-client --no-fail-fast \
  --test the_judge_composes_what_a_ray_passes_through --test replay_oracle \
  --test the_all_opaque_recording_is_of_the_world_it_names --test a_camera_inside_the_sea_tints_nothing \
  --test the_sea_the_camera_sees_is_the_water_layer --test an_opaque_face_nearer_than_the_sea_keeps_its_own_colour
```

### T12 — the judge composes what a ray passes through

- FR-5.1-S1 → `mc-client/tests/the_judge_composes_what_a_ray_passes_through.rs` → `a_ray_crossing_many_cells_of_one_sea_is_predicted_as_one_layer_over_what_lies_beyond`
- FR-5.1-S1 → `mc-client/tests/the_judge_composes_what_a_ray_passes_through.rs` → `the_colour_predicted_for_a_ray_crossing_the_sea_is_neither_layers_own`
- FR-5.1-S2 → `mc-client/tests/the_judge_composes_what_a_ray_passes_through.rs` → `a_prediction_composes_from_the_declared_degree_and_never_from_the_byte_a_vertex_carries`
- FR-5.1-S2 → `mc-client/tests/the_judge_composes_what_a_ray_passes_through.rs` → `the_judge_and_its_composition_name_nothing_from_the_draw_path_they_grade`
- FR-5.1-S2 → `mc-client/tests/the_judge_composes_what_a_ray_passes_through.rs` → `that_same_reading_reports_a_module_that_does_name_one`
- FR-5.1-S3 → `mc-client/tests/the_all_opaque_recording_is_of_the_world_it_names.rs` → `the_oracles_second_rule_moves_no_prediction_over_a_world_that_stops_all_the_light`
- FR-5.1-S4 → `mc-client/tests/replay_oracle.rs` → `a_class_the_declared_list_does_not_hold_is_named_rather_than_passed_over`
- FR-4.2-S1 → `mc-client/tests/an_opaque_face_nearer_than_the_sea_keeps_its_own_colour.rs` → `every_declared_capture_draws_the_shore_in_front_of_the_sea_with_nothing_of_the_sea_in_it`
- FR-4.3-S1 → `mc-client/tests/a_camera_inside_the_sea_tints_nothing.rs` → `a_camera_standing_in_the_sea_draws_every_surface_and_the_sky_exactly_as_a_dry_one_does`

**FR-5.1-S2 is one scenario with two clauses and each has its own falsifier**, which is why it carries three lines. The clause *"from the declared opacity"* is arithmetic, falsified by a composition at a degree of `0.002` over black: the byte a vertex would carry moves the answer from `[7, 7, 7]` to `[13, 13, 13]` — six code values, because near black the transfer function is a straight line of slope 12.92 and the rounding is amplified rather than lost. **No pair of colours this project ships can show that difference**; over them it is under a third of one code value, so a reading that would notice needs a pair chosen for it. The clause *"sharing no code with the draw path"* is structural, falsified by a source scan with a positive control over a doctored copy of the same module.

### FR-4.3-S1 fires at no declared tick, and the pose is declared instead

`spec.md:293` says the declared capture path is *"submerged at ticks 59 and 119"*. **Measured: the camera is in open air at all three**, and `replay_oracle.rs::the_camera_of_every_judged_frame_stands_in_open_air` is the green test that says so — eye `(63.5, 38.62, 35.5)`, `(62.34, 36.38, 34.12)` and `(61.72, 35.62, 31.31)`, with `base:water` two cells below at tick 59 and one below at tick 119. The player wades; the eye does not go under. Written at those ticks the scenario's `WHEN` fires never and the reading asserts nothing, which is the D-I5 shape.

So the reading declares its own pose over the shipped world, as `terrain_probes.rs` and `support/all_opaque.rs` both already do. The lead ruled that option, and ruled separately that **`0.5` is chosen and not derived**: `tasks.md` T13 derives it through this same broken chain, and the honest label is *chosen, and binding under FR-4.3-S1*. Both corrections belong to the implementation side and are not this author's to write.

### The filter and the ranking that chose the submerged pose, stated apart

**The filter**: the eye's cell holds a block that passes light; the eye stands at that cell's centre, so it is strictly inside on all three axes; the forward direction is not parallel to the world's up axis; and the grid classifies at least one sample as sky and one as a surface reached without crossing the sea. **What the filter does not say, measured**: it does not ask the eye's six neighbours to hold water — the shipped sea is **178 cells, 47 at height 33 and 131 at height 34**, so no cell of it has water on all six sides and such a filter admits nothing at all; and it does not ask for a *further* run of the sea along any ray, because over all **19 767** admitted candidates not one has a sample that crosses one. **The ranking**: the most even split of the 576 samples between sky and surfaces. The chosen pose gives 288 and 288, and no candidate does better.

### The sea's colour reading was re-derived and its tolerance was not touched

`the_sea_the_camera_sees_is_the_water_layer.rs` judged the sea against the water layer's own mean and reddens the moment water blends — measured, under a one-line mutation of `content/base/blocks/water.luau`, at 57, 114 and 205 pixels across the three captures. **`SHOWS_THE_LAYER` is still 8.0.** What changed is the expectation: a predicted-water sample is judged against what the march predicts for it, which is the sea composed over whatever the ray meets beyond. Measured under that same mutation: worst residual **ΔE 1.29**; nearest a composite stands from a colour one of its own operands draws unblended **ΔE 11.95**. The bracket is (1.29, 11.95) and 8.0 sits inside it. A tolerance widened to fit the composite would have had to pass **16.26**, which is where an *unblended* sea is accepted too.

**`the_sea_draws_its_baked_art.rs` and `terrain_probes.rs` are clean, measured rather than inferred from their names.** Under the same mutation the workspace ran `1635 tests run: 1632 passed, 3 failed, 1 skipped`: `terrain_goldens`, the sea's colour reading, and — **unpredicted** — `hud_goldens`. The HUD capture composites over the terrain frame, so a content change that moves terrain moves the HUD set even though the renderer rework moved no HUD pixel; a reader who has absorbed the second sentence must not carry it into the first. All six readings of `the_sea_draws_its_baked_art` stayed green because they inspect texels and mip levels and never touch a frame; all nine of `terrain_probes` stayed green because its three per-key floors are floors and its landmark pixel is the pillar's stone.

### Additional coverage

- → `support/composite.rs`'s use of `art::landmarks_at_every_scale` — a face at middle distance shows a *reduced* texel that is neither a texel nor the layer's mean, and a grass side's stands **ΔE 16.14** from every colour `art::landmarks` offers. Judging against the narrow set and widening a tolerance to fit would have been the tolerance doing the layer's work. Measured: the worst blended residual falls from ΔE 7.13 to 1.29 and the worst unblended from 16.14 to 7.42. `art::landmarks` is deliberately left alone, so no existing reading's accepted set moves.
- → `an_opaque_face_nearer_than_the_sea_keeps_its_own_colour.rs`'s stronger reading is **not available and that is measured**: over all three captures, **zero** declared samples have an opaque face and the sea on one ray, so a depth test switched off has no witness on the declared path. What witnesses that is FR-2.2-S1's pane fixture. Recorded because a reader would otherwise assume this reading covers it.
- → `replay_oracle.rs::every_declared_sample_of_every_judged_frame_is_sky_or_a_block_the_world_places_and_some_is_sea` was repaired rather than left: a class is split back into its parts on ` over `, so a composite is not reported as a class the world does not place. Without the repair every reading in that file would have reddened the day the sea declared a degree, for a reason that is not a defect.
- → `support/march.rs` is `oracle.rs`'s voxel walker and lens moved out whole, with no behaviour change: the judge was 701 non-blank lines against the 600 the gate allows for a test file, and the walk is a separate question from the judging.

### Green at authoring

`testing.md` §2 replaces RED with a mutation where a scenario is already satisfied, and seven of these nine readings were never red.

- **FR-5.1-S1..S4** are about an instrument this author writes, so there is no implementation for them to fail against; their falsifiers are the mutations below.
- **The sea's colour reading** is green before and after the sea declares a degree, which is the point of a standing guard. Its falsifier is the content mutation above, which reddens its old form and leaves its new one green.
- **FR-4.2-S1** and **FR-4.3-S1** are the two that are red, both on their stated premise: no frame of the declared path draws anything that passes light, and the cell the submerged eye stands in stops all the light. Both become readings the day `water.luau` declares a degree.

### Mutations, with the prediction written down first

Window announced to the lead and closed by message. **The tests were committed at `1191f89` before anything was broken**, which is D-I8's precondition: `git diff --exit-code` isolates a revert only where the file's other work is already committed. Every revert by hand, never `git checkout --`; each confirmed by `git diff --exit-code` on the file and by `git hash-object` against `git rev-parse :<path>`, and the tree came back to `git status --short` empty.

All three are **value substitutions** — a predicate's answer, a rule's branch, a number's source. None moves a packed field, so none can corrupt a neighbouring quantity and produce collateral reds that answer a different question.

**Baseline on this tree, unscoped, `--no-fail-fast`, bare count: `1644 tests run: 1642 passed, 2 failed, 1 skipped`.** The two failures are this phase's RED, and **for the length of the window those two are blind instruments**: a mutation reddening one of them for a *different* reason is indistinguishable from the state it is already in, which is the shape that once let a test failing on a stale count swallow a revision-substitution defect. No prediction below rests on observing either of them move, and both are counted out of every figure.

- **The run rule made per-voxel** — `oracle::opening` pushes a layer for every cell rather than only where the run changes, which is Decision 9's named trap. Predicted: both judge readings red, nothing else moved. → `1644 tests run: 1640 passed, 4 failed, 1 skipped`. **Exactly the two predicted**, `a_ray_crossing_many_cells_of_one_sea_is_predicted_as_one_layer_over_what_lies_beyond` and `the_colour_predicted_for_a_ray_crossing_the_sea_is_neither_layers_own`, and nothing else moved — because no shipped block passes light yet, so the rule fires nowhere else.
- **The composition reading the quantised byte** — `composite::degree_of` answers `opacity.quantised() / 255.0`. Predicted **two** red: the arithmetic falsifier, and the source scan, because `quantised` would then stand outside a comment. → `1644 tests run: 1639 passed, 5 failed, 1 skipped`. **Three**, and the third was not predicted: the scan's own **positive control** reddened too. Its expectation is an exact one-element list over a doctored copy of the real module, and with the real module already naming `quantised` the doctored copy names it twice. That is the control being **right**, not defective — it says the premise it rests on, a clean module, is gone — and the pair together is more informative than the scan alone. Recorded because a reader meeting two reds would otherwise look for two faults.
- **`passes_light` widened so every drawn block is a layer** — `oracle::met` asks `degree.get() <= 1.0`, which every degree satisfies, so the second rule fires where nothing declares anything. Predicted: the committed all-opaque recording red, both judge readings red, the sea's colour reading red, **and `replay_oracle`'s enumerated class verdict GREEN**. → `1644 tests run: 1637 passed, 7 failed, 1 skipped`. All four predictions held. **The predicted green is the finding**: every part of a composite is still a class the world places, so the verdict that enumerates them cannot see a rule that fires where it should not, and the committed recording is the only witness there is. One unpredicted red, `the_judge_marches_through_a_block_nothing_draws`, whose subject is a `drawn = false` block and whose reported class moves when everything becomes a layer.

**What none of these separates**, left deliberately unrun: the composition's *colours*. Every mutation above moves a rule or a number the composition reads, and the colours come from `art.rs`, which this spec did not otherwise touch. A mutation of `linear_mean` or of the transfer pair would redden a great deal of this suite for reasons that predate this phase, so it says nothing about the work here.

## T12 continued — FR-4.1-S2, the model's stated limit

**Why it had no test until now, recorded because it is a decomposition defect.** `tasks.md` assigns FR-4.1-S2 to **T14**, a documentation task the implementer owns — but satisfying it needs a fixture and a reading, which is a test file the implementer may not write. A scenario whose satisfaction requires a test was assigned to a task whose owner is forbidden from writing one. The second arm — *"the model composes any two translucent surfaces correctly"* — is false under B1, so only the first is available: a camera position, the artefact it produces, and the frame.

- FR-4.1-S2 → `mc-client/tests/two_translucent_kinds_compose_in_emission_order.rs` → `two_translucent_kinds_take_their_weights_from_emission_order_and_not_from_depth`
- FR-4.1-S2 → `mc-client/tests/two_translucent_kinds_compose_in_emission_order.rs` → `the_two_frames_differ_at_exactly_the_pixels_where_the_two_kinds_overlap`
- FR-4.1-S2 → `mc-client/tests/the_rendering_document_records_the_ordering_model.rs` → `the_rendering_document_names_the_model_the_passes_the_camera_and_both_frames`
- FR-4.1-S2 → `mc-client/tests/the_rendering_document_records_the_ordering_model.rs` → `a_document_short_of_any_one_of_those_reports_that_one`
- FR-4.1-S2 → `mc-client/tests/the_rendering_document_records_the_ordering_model.rs` → `the_colours_the_rendering_document_states_are_the_colours_the_model_draws`
- FR-4.1-S2 → `mc-client/tests/the_rendering_document_records_the_ordering_model.rs` → `the_figures_the_rendering_document_states_are_the_ones_the_art_and_the_captures_give`

A camera position alone cannot show B1's artefact: it is not a property of where the eye stands but of the order two faces reach the index buffer. So the fixture draws **one scene from one eye twice**, handing the packer two differently-coloured translucent panes in the two possible orders. Measured: with the farther pane emitted first the overlap draws `(182, 135, 98)` against a prediction of `(182, 135, 99)`; with the nearer pane first it draws `(150, 124, 125)` exactly. The two stand **ΔE 24.52** apart and the closest pair among all six colours either frame may hold is **ΔE 24.02**, four times the ΔE 6 tolerance. **The heavier share goes to whichever composites last, and which one that is has nothing to do with depth.** Deterministic by construction: one section, where quads land at fixed offsets. Between sections the compaction order is whatever the atomic hands out, and a fixture spanning two would be demonstrating that nondeterminism instead — a different claim, and not one a test may assert. **Both frames are committed** under `docs/technical/images/`, and the guard compares each against the frame the fixture still draws, so the picture a reader is shown cannot be of an engine that has moved on. Their only writer was a throwaway, as the all-opaque recording's was.

The lead ruled that a guard checking prose against a string list, with the prose then written to contain those strings, is an agreement test relocated into documentation — and that when a guard and a document disagree the guard is usually the one shaped wrongly, because the document has a reader and the guard does not. So the two depth-write settings are read out of the page's **own table**, a row per draw and a column per setting; the figures are looked for as **values** anywhere on the page; and only the model's name and the pass order are phrase matches, with emphasis normalised out of both sides. Every figure is checked against a measurement and each is classified: the **spread** (a property of one image) by equality at the stated precision; the **ordering residual** by the identity the page itself asserts, because an equality against a *bound* is red on a correct engine — a quarter of the spread is 0.7904 and the page states 0.79, so `measured ≤ stated` fails by four ten-thousandths and the cheapest green would be editing the bound down; the **floor** by its own arithmetic; the **ceiling** against the nearest a composition stands from a colour one of its operands draws unblended, over the crossings the declared captures hold.

**`base:stone` lies under the sea nowhere in this world.** Enumerated because a composite family and an occurring crossing are different quantities and two independent derivations disagreed about which the ceiling comes from. Terminals behind the sea: `base:grass` and the sky at all three declared ticks; **none at all** at the submerged pose, where the run the eye stands in contributes no layer; `base:grass` (106 156) and the sky (71 817) over a sweep of **103 680** poses across the whole footprint and the integer direction lattice, with `base:stone` at **zero**; and directly under a water cell, `base:grass` at 131 and more sea at the other 47. So the occurring ceiling is **ΔE 11.95** and the derived one, over a family that includes a stone lakebed, is **ΔE 9.46**. Both are stated in the reading's header because a bound over what a world could hold and a measurement over what it does are different claims. **`SHOWS_THE_LAYER` stays 8.0**, which clears either.

### Additional coverage

- → `the_sea_the_camera_sees_is_the_water_layer.rs`'s fourth list, `drawn_as_though_the_sea_were_absent` — a composition may legitimately sit near a third layer's colours: **1 111** pairs of the 478 colours these compositions may take stand within ΔE 8 of `base:stone`'s, the nearest at **2.86**. So "the drawn colour is one the composition may take" is a sentence a picture with no sea in it could satisfy. This asks the other question — is the pixel what it would show with *no sea at all* — and the nearest any of them comes is ΔE 11.95.
- → that file's ceiling derivation was **25.34 and is now 11.95**, and the correction is not a rounding: 25.34 is layer-against-layer, which asks whether a pixel is a *different layer*, while what binds a blend is composition-against-its-own-operands. Two vacuity modes, the second being the defect this whole spec fixes: the sea failed to draw, and the sea drew without blending.

## T15 — the reload, Invariant 1, and the player's page

Test command, and the only complete form of it (`N/M tests run` is a cancelled run and no verdict at all):

```
cargo nextest run -p mc-client --no-fail-fast --test a_reloaded_opacity_reaches_the_frame \
  --test a_reload_refusing_a_degree_keeps_the_one_in_force \
  --test the_mesher_and_the_renderer_name_no_block \
  --test the_gameplay_page_tells_a_player_the_sea_is_see_through
```

- FR-6.1-S1 → `mc-client/tests/a_reloaded_opacity_reaches_the_frame.rs` → `a_pane_reloaded_at_half_a_degree_is_thereafter_drawn_blended_over_the_floor_it_lies_on`
- FR-6.1-S2 → `mc-client/tests/a_reloaded_opacity_reaches_the_frame.rs` → `a_pane_whose_reload_states_a_whole_degree_again_is_thereafter_drawn_at_its_own_colour`
- FR-6.1-S2 → `mc-client/tests/a_reloaded_opacity_reaches_the_frame.rs` → `a_pane_whose_reload_removes_the_degree_is_thereafter_drawn_at_its_own_colour`
- FR-6.1-S3 → `mc-client/tests/a_reload_refusing_a_degree_keeps_the_one_in_force.rs` → `a_reload_stating_a_degree_past_the_ceiling_is_refused_naming_the_file_block_field_and_bound`
- FR-6.1-S3 → `mc-client/tests/a_reload_refusing_a_degree_keeps_the_one_in_force.rs` → `a_refused_reload_leaves_the_pane_drawing_at_the_degree_that_was_already_in_force`
- FR-7.1-S1 → `mc-client/tests/the_mesher_and_the_renderer_name_no_block.rs` → `neither_the_mesher_nor_the_renderer_compares_anything_against_a_block_name`
- FR-7.1-S2 → `mc-client/tests/the_mesher_and_the_renderer_name_no_block.rs` → `the_same_scan_reports_a_mesher_source_that_compares_a_block_name_and_says_which_name_it_named`
- FR-7.3-S1 → `mc-client/tests/the_gameplay_page_tells_a_player_the_sea_is_see_through.rs` → `the_gameplay_page_names_the_water_as_see_through_and_names_something_seen_through_it`
- FR-7.3-S2 → `mc-client/tests/the_gameplay_page_tells_a_player_the_sea_is_see_through.rs` → `a_page_naming_neither_reports_which_of_the_two_it_is_short_of`

**FR-6.1-S2 has two tests because it states two edits, and only one of them reaches a number reader.** Stating `1.0` goes through the bounded reader; deleting the line goes through the absent-field default. A loader that lost the default, or one that kept the last degree it saw for a field a declaration no longer states, satisfies one and fails the other.

**One test is red and eight are green, and the eight are green for two different reasons.** `the_gameplay_page_…` is the RED: the page names the lakebed six times and says nothing about the water passing light, so the reading answers `NotStated(["that the water is see through"])` — which is also S2's behaviour demonstrated on the real page. The five reload readings are green because **hot reload needed no new machinery**, which `tasks.md` T16 states as the thing to prove rather than build; the two Invariant 1 readings are green because the invariant holds by construction (`architecture.md` §Decision 5). `testing.md` §2's mutation check replaces RED for all eight, below.

### Additional coverage

- → `the_mesher_and_the_renderer_name_no_block.rs`'s `a_scan_that_read_no_source_says_so_rather_than_reporting_a_clean_engine` — the third arm of the verdict has to be *reachable*, not merely declared. A read counter that never fired would leave a walk that reaches nothing reporting a clean engine, and the enumerated comparison above cannot see that on its own.
- → that file's `a_name_in_a_sibling_unit_test_file_is_passed_over_and_the_module_beside_it_is_not` — both directions of the file filter at once: a filter that swallowed the module beside the unit test would leave the positive control green while scanning almost nothing.
- → that file's `a_block_named_in_a_comment_is_not_a_comparison_against_it` — the comment strip is one step wider than its sibling guard's doc-comment strip, because these two modules discuss the sea in ordinary comments today. Without it the guard is unsatisfiable for a reason that has nothing to do with the invariant, which is the state in which somebody deletes the guard rather than the offence.
- → `the_gameplay_page_tells_a_player_the_sea_is_see_through.rs`'s `a_reading_with_no_page_to_read_says_so_rather_than_reporting_a_page_short_of_both` — a page that has moved and a page short of both are the same observation to an absence check, and one is a document to fix while the other is a guard that has lost its reach.

### The mutation window, and what each half of it is worth

Window announced to the lead and closed by message. **The tests were committed at `42ef9b7` before anything was broken**, which is D-I8's precondition: `git diff --exit-code` isolates a revert only where the file's other work is already committed. Both mutations are **value substitutions** on one line — `crates/mc-sim/src/content.rs:166`, `resolved_from`'s `opacity:` — so neither moves a packed field and neither can corrupt a neighbouring quantity. Each revert was by hand, never `git checkout --`, confirmed by `git diff --exit-code` and by `git hash-object` against `git rev-parse :<path>` (`d031da4` both times), with `git status --short` empty after each.

**Baseline at `42ef9b7`, clean tree, `cargo nextest run --workspace --no-fail-fast`: `1663 tests run: 1662 passed, 1 failed, 1 skipped`** — a bare count, so a complete run. The one failure is this phase's RED, and **for the length of the window it is a blind instrument**: no prediction below rests on watching it move, and it is counted out of every figure. The tree returned to that same reading after the window closed.

**Mutation A's predictions were formed and never written down, so A is reported as a differential and not as a prediction.** Reconstructing them afterwards would have been unfalsifiable by construction — written by somebody who already knew the answer, which is D-I10's third entry with the sign flipped. What replaces them is a comparator no hindsight can bend: **phase 2 mutated this same line, to this same value, and committed its list of eight.**

- **A — `opacity: definition.opacity` → `Opacity::OPAQUE`** → `1663 tests run: 1646 passed, 17 failed, 1 skipped`. Sixteen attributable. **Phase 2's eight are all still red, name for name.** Eight are new, and six of them are not this phase's own: the terrain and HUD goldens, `the_sea_the_camera_sees_is_the_water_layer`, the rendering document's colours, and both `two_translucent_kinds_compose_in_emission_order` readings — the last four being phase 3 instruments that did not exist to be counted then.
- **The two goldens are the finding, and they land on a dated claim.** Phase 2 recorded them green *"because no shipped block declares a degree below one"* — a reason with an expiry written into it, and T13 is the date arriving. Both are red now, the HUD capture included, which is the composite-over-terrain fact `rendering.md` carries. The line's reach extended by exactly the amount three phases of reasoning said it must, and that was checked rather than assumed.
- **A also answers a question no green could**, as a by-product: two of this phase's frame readings reddened, so a device was genuinely acquired and the census genuinely ran. A `Some(frame)` that had been `None` would have returned `Ok(())` without asserting anything, and nothing in a passing run can tell those apart.
- **What stayed green under A, and why.** Both "back to a whole degree" readings expect the pane's own colour unblended, which is what an always-opaque resolution draws — they are what B exists for. The refusal-wording reading asserts no frame and the loader reads `BlockDefinition`, which this line does not touch. All seven Invariant 1 and gameplay-page readings were unmoved: neither file reads content at all.
- **B — the same line → `Opacity::new(0.5)`, with the prediction written down first** → `1663 tests run: 1630 passed, 33 failed, 1 skipped`. Thirty-two attributable. **Every prediction about this phase's own readings held, name for name**: all four frame readings red and both other groups green. The one worth having written down is `a_pane_reloaded_at_half_a_degree…`, predicted **red against the obvious green** — the pane's degree is right by accident and the floor's is not, so a translucent floor composites over the sky behind it and the pixel becomes pane over (floor over sky).
- **B's category predictions elsewhere: four hit and three missed.** Hit: the goldens, the terrain probes, `only_a_declared_opacity…`'s all-opaque arm, and every `mc-world` loader and save reading green. Missed, all three predicted red and green: `replay_oracle` and `the_all_opaque_recording…`, because the oracle reads `BlockDefinition` through the registry rather than `ResolvedBlock` — which is FR-5.1-S2's independence showing up as an immunity; and `the_sea_draws_its_baked_art`. **Also unpredicted as a category**: five frame readings whose subject is not opacity at all — `reload_draws_the_new_block`, `a_reload_keeps_the_supply…`, and three HUD readings — red because every block blending moves any frame comparison.
- **`Opacity::new(0.5)` is water's own declared value, and that splits B's green list in two.** A reading that survives B either does not read a declared opacity at all, or reads only water's — and the second is not evidence the declaration is read. `the_sea_draws_its_baked_art` is in that second class, which is why its green is recorded as a miss rather than explained away. `redeclaring_one_block_at_half_a_degree…` is a third case: it asserts pixels at a colour no layer holds, and a world where everything blends satisfies that more strongly, so it is green for the mutation's own reason.

## T12 continued — FR-5.2-S1, the sidecar nothing read

- FR-5.2-S1 → `mc-render/tests/golden_inventory.rs` → `every_committed_sidecar_names_the_capture_of_the_directory_holding_it`
- FR-5.2-S1 → `mc-testkit/tests/golden_update.rs` → `a_written_golden_records_the_capture_the_adapter_and_the_backend_that_produced_it`

**Two tests because the scenario has two ends and they fail differently**: one reads the set that is committed, the other reads what the writer produces. Measured, and neither substitutes for the other — a wrong `capture` in a committed sidecar reddens only the first, and a writer storing the wrong string reddens only the second. **A scenario was deleted as redundant against an instrument that does not cover it.** `scenario-audit.md:31` removed an earlier duplicate FR-5.2-S1 because *"golden_inventory already proves it, and the measured hole was the sidecars"*. **`golden_inventory` did not open a sidecar at all** — its checks compared directory *names* against declared ids, and its only mention of provenance was a doc comment. The audit named the right instrument and was wrong about **what that instrument measures**, which is D-I10's wrong-quantity family applied to coverage rather than to a figure. The question that catches it is *which property does that instrument actually assert?*, and nothing else did: `grep -rn 'get("capture")' crates/ --include=*.rs` returned **zero** across the whole repository, so the value existed in four committed files and was compared by nothing. **A deletion justified by a false premise is more dangerous than an absence, because it looks reasoned.**

**The control is committed, not run once.** `tests/fixtures/sidecars-a-scan-has-to-report/` holds three directories committed to be wrong in three different ways, and the same scan is driven over them: one with no sidecar at all, one whose sidecar names another capture, and one whose sidecar states no `capture` field. **All three are told apart**, because they are three different defects — a record lost in a rename, a field a format change dropped, and an id from the revision before — and a reader has to know which. The verdict carries how many sidecars it opened in both arms, so a scan that read nothing fails on the count before its verdict is weighed.

**Three mutations, each with its prediction written down first.** Baseline, unscoped, `--no-fail-fast`, bare count: `1665 tests run: 1665 passed, 1 skipped`. All three are value substitutions. `MYCRAFT_UPDATE_GOLDENS` was verified unset before every run — a mint would have rewritten the very file two of them edit. Reverts by hand; **`git hash-object` against `git rev-parse :<path>` on all three files**, which is the check that matters here because a botched revert is self-concealing: a wrong `capture` in a committed sidecar is precisely what nothing else in the repository looks at, so a failed revert and a failed test would leave a tree that looked clean.

- **A sidecar naming another *declared* capture** — `player-walk-t059-r5`'s set to `player-walk-t000-r5`, an id rather than garbage, so the comparison is exercised instead of a parse. Predicted exactly one red. → `1664 passed, 1 failed`: `every_committed_sidecar_names_the_capture_of_the_directory_holding_it`, verdict `Disagreeing { sidecars_read: 4, faults: ["`player-walk-t059-r5`'s sidecar names `player-walk-t000-r5`"] }`. **Both golden suites, the comparison and the inventory's other four readings stayed green** — which is the gap itself, measured.
- **The `capture` key deleted from that same sidecar.** Predicted one red naming *missing* rather than mismatched, with no parse failure, because the file stays valid JSON. → `1664 passed, 1 failed`, verdict `Disagreeing { sidecars_read: 4, faults: ["`player-walk-t059-r5`'s sidecar states no `capture` at all"] }`. **Both arms of the distinction are now observed to fire**; an arm of a verdict nobody has seen fire is in the position this whole property was in before this entry.
- **The writer storing the adapter's name as the capture** — `write_golden_provenance`'s `capture: capture.as_str()` → `&provenance.name`. Predicted exactly one red, and that the committed-set reading would stay green because the committed set is untouched by it. → `1664 passed, 1 failed`: `a_written_golden_records_the_capture_the_adapter_and_the_backend_that_produced_it`, and the committed-set reading stayed green as predicted. That is the pair being two readings rather than one.
