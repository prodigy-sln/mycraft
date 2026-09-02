# Test map: 2026-08-31-underwater-tint

One line per scenario→test mapping. Scenario IDs live here and in commit
messages, never in test names or code.

## Phase 1

| Scenario | Test file | Test name |
|---|---|---|
| FR-1.1-S1 | `crates/mc-world/tests/luau_declaration_tint.rs` | `one_colour_written_three_ways_registers_as_one_pair_of_values` |
| FR-1.1-S2 | `crates/mc-world/tests/luau_declaration_tint.rs` | `a_declaration_stating_neither_field_registers_carrying_no_tint_at_all` |
| FR-1.1-S3 | `crates/mc-world/tests/luau_declaration_tint.rs` | `two_blocks_stating_two_media_are_each_registered_with_their_own` |
| FR-1.1-S4 | `crates/mc-world/tests/luau_declaration_tint.rs` | `a_block_that_stops_all_the_light_may_still_declare_what_it_looks_like_from_inside` |
| FR-1.2-S1 | `crates/mc-world/tests/luau_declaration_tint_refusals.rs` | `a_colour_stated_as_a_number_is_refused_naming_the_kind_a_colour_is` |
| FR-1.2-S2 | `crates/mc-world/tests/luau_declaration_tint_refusals.rs` | `a_colour_that_is_neither_accepted_form_is_refused_naming_both_of_them` |
| FR-1.2-S3 | `crates/mc-world/tests/luau_declaration_tint_refusals.rs` | `an_eight_digit_colour_stating_a_partial_alpha_is_told_where_strength_lives` |
| FR-1.3-S1 | `crates/mc-world/tests/luau_declaration_tint_refusals.rs` | `a_distance_at_or_under_zero_is_refused_naming_the_field_and_its_own_floor` |
| FR-1.3-S2 | `crates/mc-world/tests/luau_declaration_tint_refusals.rs` | `a_distance_that_is_not_a_finite_number_is_refused_for_that_and_never_for_the_floor` |
| FR-1.3-S3 | `crates/mc-world/tests/luau_declaration_tint_refusals.rs` | `a_distance_written_as_a_word_is_refused_naming_the_kind_a_number_is` |
| FR-1.4-S1 | `crates/mc-world/tests/luau_declaration_tint_refusals.rs` | `each_of_the_two_fields_stated_without_the_other_is_refused_naming_the_one_to_add` |
| FR-1.5-S1 | `crates/mc-world/tests/luau_declaration_keys.rs` | `a_field_one_letter_past_a_real_one_is_refused_quoting_every_field_in_declaration_order` |
| FR-2.6-S1 | `crates/mc-sim/tests/a_player_resting_on_the_sea_bed_is_under_the_surface.rs` | `the_eye_of_a_player_resting_on_the_deepest_bed_of_the_shipped_sea_stands_inside_the_water` |
| FR-2.6-S2 | `crates/mc-sim/tests/a_player_resting_on_the_sea_bed_is_under_the_surface.rs` | `a_sea_one_voxel_shallower_leaves_that_eye_over_the_surface_and_says_by_how_much` |
| FR-5.1-S1 | `crates/mc-world/tests/save_folds_a_declared_tint.rs` | `a_block_declaring_no_medium_records_one_absent_marker_and_reopens_unchanged` |
| FR-5.1-S2 | `crates/mc-world/tests/save_folds_a_declared_tint.rs` | `a_declared_medium_joins_the_appearance_record_and_moves_only_that_revision` |
| FR-5.1-S3 | `crates/mc-world/tests/save_folds_a_declared_tint.rs` | `the_appearance_record_folded_under_todays_revision_disagrees_rather_than_passing` |
| FR-6.2-S1 | `crates/mc-client/tests/documented_declaration_fields.rs` | `the_guide_tabulates_every_field_with_the_value_its_absence_means` |
| FR-6.2-S1 | `crates/mc-client/tests/documented_declaration_fields.rs` | `every_refusal_the_modding_pages_quote_lists_every_field_a_declaration_may_state` |
| FR-6.2-S2 | `crates/mc-client/tests/documented_declaration_fields.rs` | `a_page_quoting_a_list_a_field_short_is_reported_with_the_list_it_quotes` |
| FR-2.1-S1 | `crates/mc-client/tests/a_surface_is_carried_toward_the_mediums_colour_by_how_far_away_it_is.rs` | `a_wall_half_the_mediums_reach_away_is_drawn_as_the_even_mix_of_its_colour_and_the_mediums` |
| FR-2.1-S2 | `crates/mc-client/tests/a_surface_is_carried_toward_the_mediums_colour_by_how_far_away_it_is.rs` | `a_wall_at_the_mediums_full_reach_is_drawn_wholly_at_the_declared_colour` |
| FR-2.1-S3 | `crates/mc-client/tests/a_surface_is_carried_toward_the_mediums_colour_by_how_far_away_it_is.rs` | `a_wall_a_tenth_of_the_mediums_reach_away_is_drawn_a_tenth_of_the_way_toward_it` |
| FR-2.1-S4 | `crates/mc-client/tests/the_tint_measures_how_far_a_pixel_is_from_the_eye_and_not_how_deep_it_is.rs` | `a_pixel_away_from_the_centre_of_a_squarely_faced_wall_is_carried_further_than_the_centre_is` |
| FR-2.1-S5 | `crates/mc-client/tests/a_surface_is_carried_toward_the_mediums_colour_by_how_far_away_it_is.rs` | `the_same_wall_seen_through_two_declared_colours_is_drawn_at_two_distinct_colours` |
| FR-2.1-S6 | `crates/mc-client/tests/two_layers_are_each_carried_toward_the_medium_by_their_own_distance.rs` | `a_layer_and_what_stands_behind_it_are_each_carried_by_how_far_away_they_are` |
| FR-2.2-S1 | `crates/mc-client/tests/a_camera_inside_the_sea_draws_what_the_sea_declares.rs` | `a_pixel_the_frame_draws_no_terrain_at_is_the_declared_colour_and_not_the_sky` |
| FR-2.3-S1 | `crates/mc-client/tests/a_camera_inside_the_sea_draws_what_the_sea_declares.rs` | `a_camera_inside_a_sea_declaring_no_tint_draws_the_frame_a_world_with_no_tints_draws` |
| FR-2.4-S1 | `crates/mc-client/tests/a_camera_inside_the_sea_draws_what_the_sea_declares.rs` | `an_eye_in_the_open_air_over_a_sea_that_tints_is_untouched_by_it` |
| FR-2.4-S2 | `crates/mc-client/tests/the_medium_changes_across_the_cells_own_face.rs` | `an_eye_a_hair_under_the_seas_top_face_is_inside_it_and_one_a_hair_over_is_not` |
| FR-2.5-S1 | `crates/mc-client/tests/the_hud_keeps_its_declared_colours_under_the_sea.rs` | `a_hud_over_a_submerged_frame_is_drawn_at_the_colours_its_declarations_state` |
| FR-3.1-S1 | `crates/mc-client/tests/a_dry_judged_frame_is_unmoved_by_a_declared_sea_tint.rs` | `every_committed_capture_is_unmoved_by_a_sea_that_declares_a_tint` |
| FR-3.1-S2 | `crates/mc-client/tests/a_dry_judged_frame_is_unmoved_by_a_declared_sea_tint.rs` | `a_judged_camera_standing_in_a_drawn_cell_is_reported_with_its_tick_rather_than_passing` |
| FR-4.1-S1 | `crates/mc-client/tests/a_reloaded_tint_reaches_the_frame.rs` | `a_medium_that_gains_a_tint_thereafter_draws_the_wall_at_the_mix_that_tint_states` |
| FR-4.1-S2 | `crates/mc-client/tests/a_reloaded_tint_reaches_the_frame.rs` | `a_medium_whose_reload_removes_both_fields_thereafter_draws_the_wall_untinted` |
| FR-4.1-S3 | `crates/mc-client/tests/a_reloaded_tint_reaches_the_frame.rs` | `a_reload_stating_a_reach_of_no_distance_is_refused_and_leaves_the_tint_in_force` |
| FR-4.1-S4 | `crates/mc-client/tests/a_reloaded_tint_reaches_the_frame.rs` | `a_medium_whose_reload_narrows_only_its_reach_thereafter_draws_the_wall_wholly_at_its_colour` |

**FR-4.1's four are the only instrument in the suite that can see a cache**, and
what makes them that is *where* they read: `InputHarness::published` hands back
the `SimSnapshot` the simulation wrote, which is the field `App::snapshot` copies
into the frame. Each names the published tint on **both sides of the reload** and
the colours the frame drew, because the field alone would be satisfied by a
renderer that never wrote the uniform and the frame alone by a simulation that
published a stale tint into a draw path carrying it faithfully. The `after` value
is the publish **one tick past** the boundary that reported the uptake: that
boundary publishes what it resolved before adopting, so reading its snapshot
would be reading the old registry's answer and calling it the new one.

**Two of the seventeen are green on arrival, and both are reported rather than
contrived into failing.** FR-4.1-S3 asserts that a reload stating a reach of zero
is refused and leaves the loaded tint in force — the loader's floor landed in
phase 1 and the keep-in-force path is the one every refused reload already takes,
so no new machinery is owed and T12's claim that none is needed is now measured
rather than predicted. FR-3.1-S2 is a positive control on a scan this phase
introduces, so it is green the moment the scan exists; it is what distinguishes
FR-3.1-S1's empty answer from a scan that could no longer look.

**Five scenarios are red *only* on a control, and without those controls all five
would pass against a renderer with no tint in it.** FR-2.3-S1, FR-2.4-S1,
FR-2.5-S1, FR-3.1-S1 and FR-4.1-S2 each assert that something does **not** move,
and the colour predicted for an untinted eye is the untinted colour — so their
absolute halves pass too. Each therefore carries, inside the same enumerated
verdict, a case that must move: the same pose over two roots differing only in
whether the eye's own cell declares a tint. **It is never two poses over one
root** — two poses differ whatever any declaration says, and that spelling was
written first and went green.

FR-2.1's six readings stand over a **fixture world** and every other FR-2
reading stands over the shipped one, which is a measured split rather than a
preference. The shipped sea is 178 cells; its footprint at `y = 34` is
`x 60..63 × z 0..34` and the two-deep channel `x 62..63 × z 0..30` over a flat
bed at `y = 33.0`. From an eye inside it the bed is 1.5 blocks below and the
opaque distances sideways are the half-integers 2.5 … 34.5 — six blocks is not
among them — and the widest squarely faced wall at a whole distance runs **two
columns** across the centre row, against the 3.08 blocks a quarter-frame-width
pixel stands off centre at six. There is no wall in that world both six blocks
away and wide enough to look at, and FR-2.1-S6 needs a second run along the ray
that the superseded reading measured absent over all 19 767 admitted
candidates. The fixture world is meshed by `World::mesh`, packed by
`startup::scene_of` and drawn by `TerrainRenderer` — only its voxels and its
declarations are the fixture's, which is the shape `support/reload_opacity.rs`
already takes. FR-2.4-S2's two heights are the shipped sea's own top face at
`y = 35.0`.

Each of FR-2.3-S1 and FR-2.4-S1 carries a **control inside the same assertion**,
and without it neither could be red at all: a build that never writes the tint
into a frame satisfies both their identity halves *and* both their absolute
halves, because the colour predicted for an untinted eye is the untinted
colour. The control is the same pose over two roots — one declaring a tint in
the eye's own cell and one declaring none — which must draw different pictures.
It is never the two poses over one root: two poses differ whatever any
declaration says.

FR-6.2-S1 maps to two tests because the scenario names two artefacts a reader
meets separately: the page's own field table beside the value each absence
means, and the list quoted back inside a pasted refusal. Neither covers the
other — a page can carry the row and quote a stale list, or the reverse.

### Additional coverage

| Test file | Test name | What it catches |
|---|---|---|
| `crates/mc-sim/tests/a_player_resting_on_the_sea_bed_is_under_the_surface.rs` | `a_declared_pool_as_deep_as_the_shipped_sea_puts_that_eye_back_under_the_surface` | The positive control for FR-2.6-S2. Without it the dry verdict could be attributable to the fixture being declared rather than generated, to the stone under it, or to anything else the two seas do not share; same builder, same registry, same surface height, one number changed. Also a second witness on the column scan, reaching it through `Cells` where FR-2.6-S1 reaches it through `ReplayWorld`. |
| `crates/mc-core/src/block/medium_tint_test.rs` | `a_finite_distance_greater_than_zero_is_held_with_its_colour_unchanged` | The accepting half of the constructor, without which the two refusal readings beside it are satisfied by a constructor that holds nothing. Reads `f32::MIN_POSITIVE` as well as an ordinary distance, because the floor is exclusive and what a floor drifted upward takes away is not zero but the smallest value above it. |
| `crates/mc-core/src/block/medium_tint_test.rs` | `a_distance_that_is_not_greater_than_zero_is_refused_rather_than_held` | The only witness in the workspace on `MediumTint::new`'s floor. Measured: making `new` accept everything reddens this and one other test and **nothing else in 1689** — every declared distance is refused by the loader's numeric reader a layer above, so no loader scenario can see this guard. The draw path divides by the distance with no zero check on the strength of this promise. |
| `crates/mc-core/src/block/medium_tint_test.rs` | `a_distance_that_is_not_a_finite_number_is_refused_rather_than_held` | The other half of that measurement, and the one that separates the two checks: a positive infinity passes `> 0.0`, so a constructor asking only about the sign holds it and hands the draw path a reciprocal of zero. |
| `crates/mc-render/tests/shader_frame_record.rs` | `the_shipped_shaders_declare_the_record_the_build_expects_and_are_accepted` | The control on the two below: a validator that refuses everything satisfies both doctored readings. Green on today's two-field, 160-byte record, which is Invariant 5 — the instrument is known good on the shape that exists before the record grows. |
| `crates/mc-render/tests/shader_frame_record.rs` | `a_terrain_frame_with_two_fields_exchanged_fails_the_build_naming_that_shader` | A uniform whose fields are transposed compiles, binds and draws. `min_binding_size` sees only an undersized buffer, and the CPU that filled it is correct, so there is no error at any layer — at six fields this is a plausible wrong picture with no symptom. |
| `crates/mc-render/tests/shader_frame_record.rs` | `a_cull_frame_declaring_a_field_terrain_does_not_fails_the_build_naming_that_shader` | The prefix invariant this spec introduces, in the direction nothing else can see: two structs diverging after their common fields both compile and both bind, and cull then decides which quads are drawn on bytes the CPU never wrote for it. |
| `crates/mc-world/tests/luau_declaration_keys.rs` | `a_declaration_stating_every_recognised_field_and_nothing_else_registers` | The control on the unrecognised-field refusal, now stating both new fields. A check that over-fires takes the whole-contract declaration down with the misspelled one, and a control exercising a subset of the contract stops being a control over the part it left out. |
| `crates/mc-world/tests/luau_declaration_keys.rs` | `a_field_the_loader_does_not_recognise_is_refused_beside_the_ones_it_does` | The membership half of the mirror, beside FR-1.5-S1's ordered half. It reddens when a name is missing from the refusal; the ordered one additionally reddens on an extra name and on a reordering. |
| `crates/mc-client/tests/documented_declaration_fields.rs` | `the_guide_states_what_a_medium_may_be_written_as_and_how_far_it_may_reach` | The page-side twin of the loader's refusals. An author who reads a bound cell naming a single colour form believes the file they copied out of the other content directory is malformed; one who reads `#RRGGBBAA` with no alpha clause writes a strength there. The distance carries this page's only **exclusive** floor, and a cell borrowing the shared `not less than zero` wording promises a value the loader refuses. |
| `crates/mc-client/tests/documented_property_refusals.rs` | `the_guide_introduces_the_declaration_fields_in_the_order_a_refusal_quotes_them` | Pre-existing, and the fourth consumer of the field-list mirror. Listed here because this phase reddens it: it ranks a refusal's blamed field by the guide's own order, so a page short of the two new names cannot rank them. |
| `crates/mc-render/tests/frame_uniform_size.rs` | `the_frame_uniform_is_allocated_at_the_size_the_shader_declares_for_the_record` | The buffer is allocated at `FRAME_UNIFORM_BYTES` and the stage reads at the shader's declaration, and `min_binding_size: None` means an undersized buffer is caught at no layer. The table check in `build/validate_tables.rs` compares the shader against a copy, never against the constant, so it cannot see somebody grow the record, update both shaders and the table, and forget the constant three modules away. Measured: changing the constant reddens this and **nothing else in 1691**. |
| `crates/mc-client/tests/reload_marks_no_section.rs` | `a_candidate_that_only_changes_what_a_volume_does_to_the_light_marks_no_section` | A tint-only reload marks no section. Every sibling non-drawing field — `swimmable`, `move_resistance`, `swim_ascent`, `targetable`, `breakable` — had a named reading in that file and the field this spec introduced had none, so `reload.rs`'s promise that it reddens if a property joins the geometry key was **false for `tint`**. Measured: adding `declared.tint` to `drawn_of`'s key moved the workspace by **zero** tests before this reading existed, and by **exactly one — this one — after** (`1710 tests run: 1709 passed, 1 failed`, `--no-fail-fast`). The property is true by construction today; what the reading defends against is a future contributor putting a medium field into that key, which rebuilds all 256 sections on every retune of a colour nobody can see in a frame. Adding it took `reload_marks_sections.rs` past the 600-line cap, so that file was split by the question each half answers — the fields the marking rule is keyed on, and the fields it excludes — with both controls kept in the half that needs them and the shared launch in `support/marks_sections.rs`. A move, not a rewrite: same fourteen names, `14 tests run: 14 passed`. |
| `crates/mc-render/tests/frame_uniform_size.rs` | `the_same_reading_gives_the_section_record_its_own_declared_size` | The control on the reading above. A lookup that stopped reading the declaration and returned a fixed number would agree with the constant today and go on agreeing after the record grew — silently, at the moment the check exists for. Measured: stubbing the lookup to return `160` leaves the frame test **green** and reddens only this one. |
| `crates/mc-client/tests/the_committed_captures_are_unmoved_by_a_declared_sea_tint.rs` | `every_committed_capture_is_drawn_byte_for_byte_as_the_frame_committed_for_it` | Two leaks FR-3.1-S1's own reading forgives, because it judges through the golden lifecycle and this compares channel bytes. `Thresholds::default()` is ΔE 2.0 per pixel over a 0.01% area budget, so a tint reaching **up to 92 pixels at any strength** passes it, and so does one reaching **every** pixel at ΔE under 2.0 — which is the shape a dry frame would actually leak in, a strength that failed to reach the literal `0.0` applying to the whole frame at once. It also judges the **fourth** committed capture: FR-3.1-S1 walks `DECLARED_TICKS`, which is the three terrain ones, so the HUD capture is judged by `hud_goldens.rs` under the same tolerance and by nothing else. Measured: all four redraw 0 of 3 686 400 channel bytes apart. Does **not** repeat FR-3.1-S1's control — a submerged frame over the same tree moving — which lives next door and which this rests on; a second copy through the same code path would read as strength and be none. |
| `crates/mc-client/tests/the_committed_captures_are_unmoved_by_a_declared_sea_tint.rs` | `that_same_comparison_reports_a_capture_drawn_against_another_captures_committed_frame` | The positive control on the byte comparator above, without which it goes green the day it stops being able to look — a decoder handing back the frame it was given, a blob opened out of the wrong directory, a loop that compared nothing. Driven over one capture's frame against another tick's committed bytes, which cannot be one picture, and asserted as an enumerated verdict naming the capture rather than as a count. |

**The golden shooter now resolves the eye's medium rather than stating it.**
`support/goldens.rs` built every declared terrain capture's snapshot through the
three-parameter `frames::snapshot`, which carries no world and writes `tint:
None`. Under that shooter FR-3.1-S1 would have passed about a renderer that
cannot tint at all — the frame would have been dry because a fixture line said
so, and nothing would have reddened the day a tint reached a dry camera, which
is the one thing the committed set exists to report. Its verify path and the
byte-comparison path above now share one `shot` helper that goes through
`snapshot_in`, so the answer is `None` because the eye stands in open air.
`terrain_goldens`, `hud_goldens`, `golden_mismatch` and the four other readings
that go through it are unchanged and green. The HUD capture's own shooter,
`HudCapture::over`, still states `None`; `support/committed_captures.rs`
resolves it for the reading above and the constructor itself is left for
whoever next owns that file.

**FR-2.1's grid is predicted per sample, and the repair is what made it
satisfiable.** As first written, `across_the_wall`'s 231-sample grid was judged
against **one** colour derived from the wall's *depth* along the view direction.
The wall is flat and faced squarely, so its distance *from the eye* differs at
every pixel of it: the grid runs to a quarter of the frame's width and a quarter
of its height from the centre, where the ray reaches the plane **1.161** times
further out. The grid's extreme sample is `(960, 360)` — `A_QUARTER_ACROSS`
itself, which FR-2.1-S4 separately asserts must draw a colour **more than** the
ΔE 3.0 tolerance away from the depth's. The two readings therefore demanded
opposite things of one pixel, and against a correct radial draw path FR-2.1-S1
was red with **carrying the tint by depth as its cheapest green** — which is the
defect FR-2.1-S4 exists to catch, reaching an implementer as "make S1 pass".
Measured before the repair, judging a grid against one colour costs ΔE **9.75**
at six blocks (FR-2.1-S1, FR-2.1-S5), **9.21** for the two-layer reading
(FR-2.1-S6), **2.23** at 1.2 blocks (FR-2.1-S3 — inside the tolerance, but by
0.77, a margin nobody chose), and **0.00** at the full reach (FR-2.1-S2, where
`min(1, d / D)` clamps every sample to one, verified at the extreme sample and
not at the centre). `support/reload_tint.rs`'s cluster reaches only ±32 pixels
and costs **0.53**, so FR-4.1's readings never had this. `straying_from` is
replaced by `owed_across_the_wall`, which pairs each sample with the colour a
closure gives for **how much further out that pixel's own ray reaches** —
a multiple rather than a distance, because every layer a ray crosses is further
by the same factor, which is what lets FR-2.1-S6 apply one factor to both of its
distances. `radially_a_quarter_across` is now that same `further_at` at
`A_QUARTER_ACROSS`, so FR-2.1-S4's derivation and the grid's are one arithmetic.
**The proof this repair is right is not available until T10**: there is no
draw-path `mix` to mutate from radial to depth yet. The prediction is recorded
in advance — carrying by depth must redden **both**
`a_wall_half_the_mediums_reach_away_...` and
`a_pixel_away_from_the_centre_...`, and reddening only the second means the
repair did not take.

**Run at T10, the prediction held and the property turned out to have six
witnesses rather than four.** The four predicted are the fixture readings:
FR-2.1-S1 and FR-2.1-S5 at six blocks, FR-2.1-S6 over two layers, and
FR-2.1-S4's own two pixels. The two nobody predicted are
`a_pixel_the_frame_draws_no_terrain_at_is_the_declared_colour_and_not_the_sky`
(FR-2.2-S1) and
`an_eye_a_hair_under_the_seas_top_face_is_inside_it_and_one_a_hair_over_is_not`
(FR-2.4-S2), and they are the **stronger** two: both stand over the **shipped
sea** rather than a fixture world, both predict every sample at that sample's
own radial distance through `support/oracle.rs` and `support/composite.rs`, and
both judge at **ΔE 2.0 — tighter than the fixture's 3.0**. So the property is
not held only by a fixture built to show it; it is held by two readings over
the world a player actually stands in, at a tighter tolerance, which is a
better guarantee than the map claimed and than the repair set out to buy.

**The check whose absence let the contradiction compile is now in place.**
`the_geometry_holds` asks only that the sample grid fit inside the frame and the
wall; nothing asked whether the grid could **see** the thing its readings are
about, which is why a grid predicted from one colour compiled beside a reading
requiring one of its own samples to be a different colour.
`medium::the_grid_tells_radius_from_depth` closes that. Because the predictor is
handed a *multiple*, `predict(1.0)` is exactly the colour a pass carrying the
tint by **depth** would draw at every sample, so comparing it against the widest
sample's own prediction asks on every run whether a reading is a witness on
radial distance or merely compatible with one. It takes **the same closure the
prediction uses**, so a guard that passed and a prediction that judged cannot be
about two different laws. FR-2.1-S1, FR-2.1-S5 and FR-2.1-S6 carry it.
FR-2.1-S2 and FR-2.1-S3 deliberately do not, and both are properties rather than
omissions: at the declared reach `min(1, d / D)` saturates and the spread is
ΔE 0.00, so radial distance cannot reach that reading at all; at a tenth of the
reach the spread is ΔE 2.23, inside the tolerance by 0.77, so that reading
cannot tell radius from depth and FR-2.1-S4's is what does.

**`HudCapture::over` resolves the eye's medium as well.** It built its snapshot
through `frames::snapshot`, which states `tint: None`, and it is the constructor
the **HUD golden** is minted and verified from — so that golden would have
matched whatever the draw path did, the frame being dry because a fixture line
said so. It now goes through `frames::snapshot_in` like `support/goldens.rs`.
Measured: `hud_goldens`, `hud_prediction`, `hud_held_block`, `hud_frame_path`
and `terrain_goldens` are all green across the change — resolving moved no
pixel, because the camera stands in open air, which is the point.

**The five controls depend on FR-2.2-S1, and nothing else records it.** A
control asks only that the picture *moved*, never that it moved to the right
colour: `differing(...) > 0` is satisfied by any difference at all, including a
draw path that writes something wrong the moment a tint is declared, because the
control frames are compared against each other and judged against no prediction.
What constrains the tinted picture is **FR-2.2-S1**, which drives the *same*
declared pose over the *same* tinting root and asserts every declared sample
against the colour the world's own voxels predict for it. So the five scenarios
red on a control — FR-2.3-S1, FR-2.4-S1, FR-2.5-S1, FR-3.1-S1 and FR-4.1-S2 —
would every one be hollowed by somebody simplifying FR-2.2-S1's reading, and not
one test would redden to say so. Written into
`a_camera_inside_the_sea_draws_what_the_sea_declares.rs`'s header as well, which
is where a reader editing that reading meets it.

**And those five are one control observed five times, not five witnesses.** All
of them reduce to *a submerged eye over a tinting sea draws something a
submerged eye over an untinted sea does not*: they go green together and stay red
together. "Five scenarios red only on a control" is one fact counted five times,
which matters at validation, where a reviewer counting red scenarios would read
five independent witnesses where there is one.

**FR-2.5-S1's control element is named for what it checks.** It was
`the_terrain_moved_when_the_sea_declared_one` and it is a whole-frame
comparison, so it would be true if only the HUD had moved; the conjunction is
sound — a tinted HUD fails the per-pixel element beside it — so this is a naming
fix rather than a logic one, and the name now says what the assertion checks.

**Three scenarios cannot witness the wiring, not two.** `architecture.md` and
`tasks.md` constraint (b) both said FR-2.3-S1 and FR-2.4-S1 were the scenarios
a build that never writes the uniform would satisfy — a zero-filled buffer *is*
`tint_reach = 0`, and their absolute halves are dry predictions that pass too.
The mutation measured a third: forcing the reach to no tint reddens **nine of
the ten** readings, and the survivor is **FR-4.1-S3**,
`a_reload_stating_a_reach_of_no_distance_is_refused_and_leaves_the_tint_in_force`.
It compares a frame drawn before a refused reload against one drawn after, and
with no tint anywhere those two frames are equal — so *a refusal that changed
nothing* and *a renderer that draws no tint at all* are the same picture to it.
That is not a defect in the reading: what it is about is that a refused reload
leaves the loaded tint in force, and it says so correctly. It is a limit on what
it can be cited for, and the correction is to the accounting rather than to the
test. Both records now say three.

## Phase 3

| Scenario | Test file | Test name |
|---|---|---|
| FR-6.1-S1 | `crates/mc-client/tests/no_name_reaches_the_medium_the_eye_stands_in.rs` | `no_source_that_decides_or_draws_the_eyes_medium_compares_against_a_block_name` |
| FR-6.1-S2 | `crates/mc-client/tests/no_name_reaches_the_medium_the_eye_stands_in.rs` | `the_same_scan_driven_over_a_source_holding_one_name_comparison_reports_that_comparison` |
| FR-6.3-S1 | `crates/mc-client/tests/the_gameplay_page_tells_a_player_the_sea_is_see_through.rs` | `the_gameplay_page_names_the_water_as_see_through_and_names_something_seen_through_it` |
| FR-6.3-S2 | `crates/mc-client/tests/the_gameplay_page_tells_a_player_the_sea_is_see_through.rs` | `a_page_short_of_any_of_them_reports_which_ones_it_is_short_of` |
| FR-6.3-S2 | `crates/mc-client/tests/the_gameplay_page_tells_a_player_the_sea_is_see_through.rs` | `the_same_reading_over_the_real_page_with_one_thing_struck_out_reports_that_one` |
| FR-6.4-S1 | `crates/mc-client/tests/the_rendering_document_records_the_medium_the_eye_stands_in.rs` | `the_rendering_page_states_the_law_the_distance_the_clear_the_hud_and_the_revision` |
| FR-6.4-S2 | `crates/mc-client/tests/the_rendering_document_records_the_medium_the_eye_stands_in.rs` | `the_rendering_page_with_one_of_them_struck_out_reports_that_one_and_the_rest_as_stated` |

**FR-6.1-S1 is green on arrival, and it is reported rather than contrived into
failing.** It reads over the resolver, the uniform writer, the clear-colour
chooser and `terrain.wgsl` — all of which landed in phase 2 — so there is no
tree on which it could have been red for the right reason, exactly as FR-2.6-S1
had none. What stands in for RED is a **mutation over the real tree**: with
`let _shortcut = ["base:water"];` added inside `eye_medium`, `24 tests run: 23
passed, 1 failed` (`--no-fail-fast`, the four scan binaries), the single failure
being FR-6.1-S1's reading, naming the file and the spelling. The green half is
the load-bearing one — `neither_the_mesher_nor_the_renderer_compares_anything_against_a_block_name`
stayed green through it, which is the measurement showing `mc-sim` sits outside
that sibling guard's two trees and that this reading is additive rather than a
second copy of it. Reverted by re-editing; `git diff --exit-code` clean.

**FR-6.3-S2 maps to two tests because a fixture control and a doctored real page
answer different questions.** The first drives the reading over prose written to
satisfy it, which proves the reading can tell one missing item from another; it
proves nothing about whether the reading finds these things where a player
actually reads them. The second strikes each item's own spellings out of
`docs/user/gameplay.md` and requires each to come back reported alone, which is
the only thing that would notice a reading located an item only in prose of its
own writing.

**The two page readings are red on assertions and stay red until T14 and T15.**
Both name the items they cannot find, so the failure a doc author reads is the
list of what to write rather than a compile error.

**`the_rendering_document_records_the_medium_the_eye_stands_in.rs` is a second
file beside `the_rendering_document_records_the_ordering_model.rs` rather than an
extension of it.** That file stood at 495 lines against a 600-line cap and these
five claims plus their controls are some 250 more; the split is by the question
each half answers — how two translucent surfaces compose, and what the medium the
eye is inside does to everything drawn. Nothing was moved between them, so no doc
naming the older file needs re-pointing.

**`seam_boundaries.rs` was not the home the task named it as, for the same
reason.** It stands at 608 lines already. The Invariant 1 scan follows its shape
and `client_names_no_content_door.rs`'s, and sits beside
`the_mesher_and_the_renderer_name_no_block.rs`, which is the guard it is closest
kin to.

**Two pre-existing names in the gameplay reading changed, because the page now
owes four things rather than two**: `a_page_naming_neither_reports_which_of_the_two_it_is_short_of`
became `a_page_short_of_any_of_them_reports_which_ones_it_is_short_of`, and
`a_reading_with_no_page_to_read_says_so_rather_than_reporting_a_page_short_of_both`
became `..._short_of_all`. `specs/archive/2026/2026-08-27-water-translucency/test-map.md`
names the first under FR-7.3-S2; that folder is history and is left as it was
written.

### Additional coverage — phase 3

| Test file | Test name | What it catches |
|---|---|---|
| `crates/mc-client/tests/no_name_reaches_the_medium_the_eye_stands_in.rs` | `the_same_scan_reports_every_shape_of_reaching_for_a_name_that_it_carries` | FR-6.1-S2's fixture commits one shape; this commits all of them. A needle no fixture ever commits is a needle nobody has watched match anything — mistype `BlockName::from` and it reports a clean path for as long as it stands there. The expected report is derived from the same list the scan reads, so a door added without a fixture fails here rather than standing unwatched. |
| `crates/mc-client/tests/no_name_reaches_the_medium_the_eye_stands_in.rs` | `a_source_handling_the_type_a_name_has_is_not_a_source_comparing_against_one` | The trap the scan is shaped around, in the direction nothing else looks. A correct resolver names `BlockName` in its signature and in the pattern it matches, so a scan keyed on the *mention* would report the correct implementation as the violation — which is worse than no scan, because the repair a reader reaches for is deleting the guard. |
| `crates/mc-client/tests/no_name_reaches_the_medium_the_eye_stands_in.rs` | `a_name_written_in_a_comment_is_not_a_comparison_against_it` | The whole-line comment strip, over all three Rust comment forms and the bare `//` a shader has no doc form to be told apart from. Without it the guard is unsatisfiable for a reason that has nothing to do with the invariant, which is the state in which somebody deletes the guard rather than the offence. |
| `crates/mc-client/tests/no_name_reaches_the_medium_the_eye_stands_in.rs` | `a_scan_that_can_no_longer_reach_a_declared_source_says_which_one_rather_than_reporting_it_clean` | The vacuity control in both of its directions: a declared source that has gone, and one still present that no longer holds the thing it was declared for. The second is what a file-level declaration is otherwise blind to — the resolver moves one file over, the scan goes on reading an empty room, and its clean verdict is about nothing at all. |
| `crates/mc-client/tests/the_gameplay_page_tells_a_player_the_sea_is_see_through.rs` | `a_reading_with_no_page_to_read_says_so_rather_than_reporting_a_page_short_of_all` | Pre-existing, renamed with the list's growth. A page that moved carries none of the four, which is exactly what a page short of all four looks like; one is a document to fix and the other is a guard that has stopped being able to look. |
| `crates/mc-client/tests/the_rendering_document_records_the_medium_the_eye_stands_in.rs` | `a_reading_driven_over_a_page_short_of_each_in_turn_reports_each_alone` | The per-item discrimination, over a page written to carry everything — and the only thing saying the reading works at all while the two scenario readings beside it are red. A test red for a known reason reports nothing about anything else, so this stands beside them rather than behind them. |
| `crates/mc-client/tests/the_rendering_document_records_the_medium_the_eye_stands_in.rs` | `a_reading_with_no_page_to_read_says_so_rather_than_reporting_a_page_short_of_all_five` | The vacuity control, and the reason the verdict carries a `ThePageWasNotRead` arm rather than answering all-false. |
