# Test Map: Headless Frame-Capture Harness

**Spec**: [spec.md](spec.md) · **Tasks**: [tasks.md](tasks.md) ·
**Owner**: test author (rigor `high`) · **Updated**: 2026-08-11

Each acceptance scenario maps to exactly one test. Scenario IDs live here and in
commit messages only — never in test names, file names or code.

Test command (phase 1 and 2):

```
cargo nextest run -p mc-testkit --no-default-features
```

## Phase 1 — GPU-free pixel pipeline (21 scenarios)

| Scenario | Test file | Test name |
|---|---|---|
| FR-2.2-S2 | `crates/mc-testkit/tests/frame_size.rs` | `a_zero_extent_is_rejected_naming_the_offending_dimension` |
| FR-2.2-S3 | `crates/mc-testkit/tests/frame_size.rs` | `a_width_past_the_maximum_texture_dimension_is_rejected_naming_both_numbers` |
| FR-2.4-S1 | `crates/mc-testkit/tests/png_io.rs` | `an_image_written_to_a_png_decodes_back_to_the_same_pixels` |
| FR-2.4-S2 | `crates/mc-testkit/tests/png_io.rs` | `a_written_png_keeps_the_white_half_at_the_top_where_it_was_drawn` |
| FR-2.4-S3 | `crates/mc-testkit/tests/png_io.rs` | `a_target_whose_directory_cannot_be_created_names_the_path_and_the_cause` |
| FR-3.1-S1 | `crates/mc-testkit/tests/comparison.rs` | `a_one_level_grey_drift_stays_inside_the_per_pixel_tolerance` |
| FR-3.1-S2 | `crates/mc-testkit/tests/comparison.rs` | `a_twelve_level_grey_drift_counts_those_pixels_as_failing` |
| FR-3.2-S1 | `crates/mc-testkit/tests/comparison.rs` | `a_failing_share_inside_the_area_budget_is_a_match_that_still_counts_the_pixels` |
| FR-3.2-S2 | `crates/mc-testkit/tests/comparison.rs` | `a_failing_share_past_the_area_budget_is_a_mismatch_stating_the_count_and_the_budget` |
| FR-3.3-S1 | `crates/mc-testkit/tests/comparison.rs` | `the_largest_distance_under_the_hard_ceiling_is_reported_with_the_match` |
| FR-3.3-S2 | `crates/mc-testkit/tests/comparison.rs` | `a_single_pixel_past_the_hard_ceiling_fails_from_inside_the_area_budget` |
| FR-3.4-S1 | `crates/mc-testkit/tests/comparison.rs` | `swapping_the_two_images_leaves_the_verdict_count_and_maximum_unchanged` |
| FR-3.4-S2 | `crates/mc-testkit/tests/comparison.rs` | `images_of_different_sizes_are_a_mismatch_naming_both_sizes` |
| FR-3.4-S3 | `crates/mc-testkit/tests/comparison_without_adapter.rs` | `two_caller_supplied_frames_produce_a_verdict_with_no_device_in_the_process` |
| FR-3.5-S1 | `crates/mc-testkit/tests/comparison.rs` | `a_comparison_without_explicit_thresholds_applies_the_documented_defaults` |
| FR-3.5-S2 | `crates/mc-testkit/tests/comparison.rs` | `a_tightened_per_pixel_tolerance_catches_a_one_level_grey_drift` |
| FR-3.5-S3 | `crates/mc-testkit/tests/comparison.rs` | `a_negative_per_pixel_tolerance_is_rejected_naming_the_value` |
| FR-4.3-S1 | `crates/mc-testkit/tests/diff_image.rs` | `the_diff_marks_every_failing_position_and_carries_the_expected_pixel_elsewhere` |
| FR-4.3-S2 | `crates/mc-testkit/tests/diff_image.rs` | `rendering_and_encoding_the_same_diff_twice_produces_identical_bytes` |
| FR-4.3-S3 | `crates/mc-testkit/tests/diff_image.rs` | `no_diff_is_produced_for_images_of_different_sizes` |
| FR-6.1-S1 | `crates/mc-testkit/tests/dependency_graph.rs` | `the_harness_resolves_without_the_crates_it_exists_to_verify` |

### Supporting unit tests (no scenario)

Enabling work whose correctness the scenarios above depend on. Private items, so
they are tested through a `*_test.rs` sibling rather than from `tests/`.

| Task | Test file | Test name | Why it exists |
|---|---|---|---|
| T04 | `crates/mc-testkit/src/frame/color_test.rs` | `adjacent_neutral_greys_are_a_fraction_of_a_unit_apart` | Pins ΔE 0.39168 for (128,128,128) vs (129,129,129) |
| T04 | `crates/mc-testkit/src/frame/color_test.rs` | `a_twelve_level_neutral_grey_step_is_a_few_units` | Pins ΔE 4.66505 for (128,128,128) vs (140,140,140) |
| T04 | `crates/mc-testkit/src/frame/color_test.rs` | `a_fifty_two_level_neutral_grey_step_is_far_past_the_hard_ceiling` | Pins ΔE 19.72703 for (128,128,128) vs (180,180,180) |
| T05 | `crates/mc-testkit/src/frame/readback_test.rs` | `a_row_that_defeats_the_copy_alignment_is_padded_up_to_it` | 257 px wide → 1028 content bytes → 1280 padded |
| T05 | `crates/mc-testkit/src/frame/readback_test.rs` | `a_row_that_already_fills_the_copy_alignment_is_left_alone` | 64 px wide → 256 bytes, no padding added |
| T05 | `crates/mc-testkit/src/frame/readback_test.rs` | `a_width_whose_row_cannot_be_addressed_is_rejected` | Justifies the fallible return of the padding function |
| T05 | `crates/mc-testkit/src/frame/readback_test.rs` | `unpadding_strips_the_filler_from_every_row` | The 257 × 129 shape, 129 rows, in order |
| T05 | `crates/mc-testkit/src/frame/readback_test.rs` | `a_buffer_shorter_than_its_padded_rows_is_rejected` | A short mapped range is an error, not a truncated frame |

### Shared fixtures

`crates/mc-testkit/tests/common/mod.rs` — hand-built `Rgba8Image` fixtures
(`uniform`, `with_leading_pixels`, `split_by_column`, `split_by_row`), the
shared `TestResult` alias and the windowed float assertion. No test in phase 1
needs a device to build its inputs, which is the data seam working.

The two split helpers name their axis on purpose. `split_by_column` is
vertically symmetric and therefore cannot witness row order — using it for the
on-disk orientation assertion is the defect the FR-2.4-S2 amendment corrected.
`split_by_row` is the fixture for anything about orientation.

**FR-2.4-S2 decodes the written file without `read_png`** (validation pass 1,
Minor 5). The row-split fixture alone was not enough: writing with `write_png`
and reading with `read_png` is a round trip, and a round trip is exactly what a
compensating flip pair defeats — flip on write, flip on read, the buffer that
comes back is the one that went in, and the file on disk is upside-down. Since
that pair is the *sole* property `spec.md` gives S2, the test decoded the file
through `image` directly instead, with no `frame::png` code on the read side.
Verified by injecting the compensating pair into `encode_png` and `read_png`:
FR-2.4-S1 and the previous FR-2.4-S2 both still passed, and the independent
decode failed on `[0, 0, 0, 255]` at the top of the file. Do not "simplify" the
read side back to `read_png`.

`crates/mc-testkit/tests/comparison_without_adapter.rs` deliberately builds its
own fixtures instead of using `common`, so the file names nothing outside the
core.

## Phase 2 — GPU-free policy, lifecycle and reporting (20 scenarios)

| Scenario | Test file | Test name |
|---|---|---|
| FR-1.1-S5 | `crates/mc-testkit/src/frame/selection_test.rs` | `a_hardware_adapter_is_chosen_over_a_software_one` |
| FR-1.2-S1 | `crates/mc-testkit/src/frame/selection_test.rs` | `a_failed_acquisition_without_the_opt_in_is_an_error_rather_than_a_skip` |
| FR-1.2-S2 | `crates/mc-testkit/src/frame/selection_test.rs` | `a_failed_acquisition_with_the_opt_in_skips_with_a_warning_naming_the_variable` |
| FR-1.2-S3 | `crates/mc-testkit/src/frame/selection_test.rs` | `a_successful_acquisition_with_the_opt_in_set_still_runs_the_capture` |
| FR-2.3-S2 | `crates/mc-testkit/tests/readback_deadline.rs` | `a_readback_that_outlives_its_deadline_times_out_naming_the_capture_and_the_bound` |
| FR-4.1-S1 | `crates/mc-testkit/tests/golden_pass.rs` | `a_capture_matching_its_golden_passes_and_leaves_its_artifact_directory_empty` |
| FR-4.1-S2 | `crates/mc-testkit/tests/golden_pass.rs` | `a_pass_removes_the_artifacts_an_earlier_mismatch_left_behind` |
| FR-4.1-S3 | `crates/mc-testkit/tests/golden_pass.rs` | `clearing_one_captures_artifacts_leaves_another_captures_alone` |
| FR-4.2-S1 | `crates/mc-testkit/tests/golden_mismatch.rs` | `a_mismatching_capture_writes_the_whole_artifact_set` |
| FR-4.2-S2 | `crates/mc-testkit/tests/golden_mismatch.rs` | `a_reported_mismatch_names_the_directory_holding_its_artifacts` |
| FR-4.2-S3 | `crates/mc-testkit/tests/golden_mismatch.rs` | `an_artifact_set_that_cannot_reach_disk_still_reports_the_mismatch` |
| FR-4.4-S1 | `crates/mc-testkit/tests/golden_missing.rs` | `a_missing_golden_fails_naming_the_path_and_is_not_created` |
| FR-4.4-S2 | `crates/mc-testkit/tests/golden_missing.rs` | `a_missing_golden_writes_the_captured_frame_into_the_artifact_directory` |
| FR-4.4-S5 | `crates/mc-testkit/tests/golden_missing.rs` | `a_golden_that_is_not_a_decodable_png_fails_without_being_replaced` |
| FR-4.4-S3 | `crates/mc-testkit/tests/golden_update.rs` | `the_update_path_overwrites_a_mismatching_golden_and_reports_every_path_it_wrote` |
| FR-4.4-S4 | `crates/mc-testkit/tests/golden_update.rs` | `the_update_path_leaves_a_matching_golden_byte_for_byte_alone` |
| FR-5.1-S4 | `crates/mc-testkit/tests/golden_update.rs` | `a_written_golden_records_the_adapter_and_backend_that_produced_it` |
| FR-5.1-S1 | `crates/mc-testkit/tests/frame_report.rs` | `a_written_mismatch_report_parses_as_json_whose_failing_pixel_count_is_a_number` |
| FR-5.1-S2 | `crates/mc-testkit/tests/frame_report.rs` | `a_mismatch_report_records_the_environment_and_the_thresholds_that_judged_it` |
| FR-5.1-S3 | `crates/mc-testkit/tests/frame_report.rs` | `an_adapter_reporting_no_driver_description_records_the_field_as_unknown` |

**FR-4.2-S3 covers both of its conditions and asserts the path as a whole**
(validation pass 2, correctness PARTIAL). Its test was renamed from
`an_artifact_directory_that_cannot_be_created_still_reports_the_mismatch`, for
two reasons.

The scenario says "cannot be created **or written**", and only creation was
provoked. The write half is now provoked by pre-creating `expected.png` as a
*directory* inside an artifact directory that is created without trouble — the
same portable trick the sidecar test uses, needing no permission bits. Both
obstructions run through one test, so the scenario keeps its single mapping.

And the naming assertion had to be able to fail. `<dir>` is a substring of
`<dir>/expected.png`, so a plain `contains` cannot tell "the failure named the
directory" from "the failure named a file inside it", and would stay green for
an implementation that never named the directory at all. `names_whole_path`
requires an occurrence that is **not** followed by a path separator. Verified
against three mutations of the implementation, each reverted:
`ArtifactError::Directory` carrying `expected.png` instead of the directory
(directory half fails on the naming assertion); `GoldenFailure::Display` using
`{cause}` instead of `describe(cause)` (both halves fail on the cause
assertion); and the image write reporting its parent directory with
`ImageIoError::Write` dropping its path (write half fails on the naming
assertion, which also confirms the second obstruction really executes). The
superseded test survived the first mutation, because `source().is_some()` was
all it asked of the message.

### Supporting unit tests (no scenario)

| Task | Test file | Test name | Why it exists |
|---|---|---|---|
| T12 | `crates/mc-testkit/src/frame/layout_test.rs` | `a_golden_is_the_default_image_inside_a_directory_named_for_its_capture` | Pins `<root>/<capture>/default.png`, the shape an adapter variant later adds a file to |
| T12 | `crates/mc-testkit/src/frame/layout_test.rs` | `a_goldens_provenance_sidecar_sits_beside_it` | Pins `default.provenance.json` next to its golden |
| T12 | `crates/mc-testkit/src/frame/layout_test.rs` | `a_captures_four_artifacts_share_a_directory_named_for_it` | Pins the artifact directory and its four filenames — and that nothing run-specific is in the path, which FR-4.1-S2 depends on |
| T12 | `crates/mc-testkit/tests/capture_identity.rs` | `a_lowercase_name_with_digits_and_separators_is_accepted` | The accepted alphabet |
| T12 | `crates/mc-testkit/tests/capture_identity.rs` | `a_nameless_capture_is_rejected` | An empty segment would put a capture's files in the root |
| T12 | `crates/mc-testkit/tests/capture_identity.rs` | `a_name_carrying_a_path_separator_is_rejected_naming_the_character` | Path-traversal guard on a path-forming public input |
| T12 | `crates/mc-testkit/tests/capture_identity.rs` | `a_parent_directory_reference_is_rejected` | `..` must never reach a path the harness writes to |
| T12 | `crates/mc-testkit/tests/capture_identity.rs` | `an_uppercase_name_is_rejected` | Two captures must not collide on a case-insensitive filesystem |
| T13 | `crates/mc-testkit/src/frame/selection_test.rs` | `an_adapter_of_unknown_kind_outranks_a_software_rasteriser` | `Cpu` ranks below `Other` — D2 calls this ordering the tested contract |
| T13 | `crates/mc-testkit/src/frame/selection_test.rs` | `an_empty_candidate_list_selects_nothing` | The no-adapter case feeding `AcquireError::NoAdapter` |
| T13 | `crates/mc-testkit/src/frame/selection_test.rs` | `a_limit_the_adapter_exactly_meets_is_not_reported` | The boundary: a limit met exactly is satisfied |
| T13 | `crates/mc-testkit/src/frame/selection_test.rs` | `a_limit_beyond_the_adapter_is_reported_with_both_numbers` | The detail FR-1.1-S4's real wgpu rejection carries in phase 3 |
| T14 | `crates/mc-testkit/tests/opt_ins.rs` | `an_opt_in_set_to_a_falsy_value_is_still_enabled` | Assumption 5: presence, not value |
| T14 | `crates/mc-testkit/tests/opt_ins.rs` | `each_variable_enables_only_its_own_opt_in` | Permission to rewrite goldens is not permission to skip |
| T22 | `crates/mc-testkit/tests/committed_golden.rs` | `the_golden_committed_to_the_repository_matches_the_frame_that_produced_it` | The git round trip — committed bytes read from their real repo path, compared, judged |
| T22 | `crates/mc-testkit/tests/committed_golden.rs` | `regenerating_the_committed_golden_leaves_it_matching_the_generator` | `#[ignore]`d. Mints the committed golden through the harness's own update path, so the bytes in the repo are a product of the real code |
| T20 (lead ruling, 2026-08-11) | `crates/mc-testkit/tests/golden_update.rs` | `an_update_that_cannot_write_the_golden_reports_that_it_was_not_updated` | The condition attached to approving the "reuse FR-4.2-S3's channel" design: a failed golden write must **say so**. It did not — the write failure sat in `GoldenFailure.artifacts` and never reached `Display`, so a caller who set the opt-in was shown only "no golden exists", read it as the state the update had just fixed, and re-ran into the same wall. Verified non-vacuous against the pre-fix code, where it fails on exactly that message |
| T20 (lead ruling, 2026-08-11) | `crates/mc-testkit/tests/golden_update.rs` | `a_failed_golden_update_still_reports_the_verdict_that_stands` | The other half of the same report: naming the failed write must not cost the verdict that still holds. Passes against the pre-fix code too — a regression guard on the standing `reason`, not a test of the fix |
| T20 (validation pass 1, Minor 4) | `crates/mc-testkit/tests/golden_update.rs` | `a_golden_written_without_its_sidecar_still_reports_the_path_it_wrote` | Ruling 5's defect with the sign reversed: the image write **succeeded** and only the sidecar failed, so collapsing to the standing verdict reported "the capture differs from its golden" and **zero** written paths about a file replaced a moment earlier — an FR-4.4-S3 violation, not just a misleading message. Provoked by pre-creating `default.provenance.json` as a *directory*, so the golden beside it writes normally and only the JSON fails; the test guards that precondition by asserting the golden on disk is the captured frame. Verified pre-fix in a throwaway worktree at `ca19469`: same provocation, `Failed(GoldenFailure { reason: Mismatch(..) })`, no paths |

### Shared fixtures

`crates/mc-testkit/tests/support/mod.rs` — the golden-lifecycle fixtures
(`reference_frame`, `drifted_frame`, the path helpers, `golden_settings`,
`synthetic_provenance`, the `UPDATING` opt-in value) and this suite's
`TestResult`.

It is deliberately **separate from and independent of `common`**: phase 1's
suite keeps compiling while phase 2 is still being built, so it stays a live
regression signal instead of collateral damage. Verified — phase 1's 29 tests
pass against a working tree in which every phase-2 binary fails to compile.

Both fixture frames are **split across rows**. The lifecycle writes captured
frames to disk and reads goldens back, so a row-order inversion is a realistic
bug in exactly this path, and a fixture symmetric down the rows cannot witness
one (`spec.md` § Structural constraints). The path helpers spell the D8 layout
out literally rather than calling `GoldenPaths`/`ArtifactPaths`, so a relocated
file is a failing test and not a silent move.

The committed golden's fixture is generated by `synthetic_frame` in
`committed_golden.rs` — every channel a function of the pixel's own
coordinates, so the file is byte-reproducible from the committed generator
alone. Green varies down the rows and red across the columns: neither a row
inversion nor a transposition survives it.

## Phase 3 — the wgpu adapter (12 scenarios)

Test command — the default build, where `wgpu` is in the graph:

```
cargo nextest run -p mc-testkit
```

Phases 1 and 2 keep their `--no-default-features` command. The five binaries
below name `mc_testkit::frame::gpu`, so each must be declared in
`crates/mc-testkit/Cargo.toml` with `required-features = ["gpu"]`; without that
they would break the GPU-free configuration the seam is only real in.

Every test here starts by acquiring a real device and **fails when none
answers**. No test in this phase skips itself, and none sets an environment
variable: a green run that verified nothing is the one outcome this harness may
not have.

| Scenario | Test file | Test name |
|---|---|---|
| FR-1.1-S1 | `crates/mc-testkit/tests/offscreen_capture.rs` | `a_headless_context_captures_a_single_pixel_frame` |
| FR-1.1-S2 | `crates/mc-testkit/tests/adapter_acquisition.rs` | `an_acquired_device_reports_the_adapter_it_selected` |
| FR-1.1-S3 | `crates/mc-testkit/tests/adapter_acquisition.rs` | `a_backend_with_no_adapters_fails_naming_every_backend_it_tried` |
| FR-1.1-S4 | `crates/mc-testkit/tests/adapter_acquisition.rs` | `a_device_request_past_the_adapters_limits_names_the_adapter_and_the_requirement` |
| FR-2.1-S1 | `crates/mc-testkit/tests/capture_color.rs` | `a_frame_cleared_to_opaque_red_comes_back_opaque_red` |
| FR-2.1-S2 | `crates/mc-testkit/tests/capture_color.rs` | `a_fill_of_the_top_half_comes_back_at_the_top_of_the_frame` |
| FR-2.1-S3 | `crates/mc-testkit/tests/capture_color.rs` | `a_quarter_alpha_clear_leaves_the_colour_channels_unscaled` |
| FR-2.1-S4 | `crates/mc-testkit/tests/capture_color.rs` | `a_mid_tone_clear_comes_back_srgb_encoded` |
| FR-2.1-S5 | `crates/mc-testkit/tests/capture_failure.rs` | `a_capture_whose_draw_work_fails_returns_that_error_and_no_image` |
| FR-2.1-S6 | `crates/mc-testkit/tests/capture_failure.rs` | `a_context_still_captures_after_a_failed_capture` |
| FR-2.2-S1 | `crates/mc-testkit/tests/offscreen_capture.rs` | `a_frame_whose_rows_defeat_the_copy_alignment_comes_back_unpadded` |
| FR-2.3-S1 | `crates/mc-testkit/tests/offscreen_capture.rs` | `a_completed_capture_reports_how_long_its_readback_took` |

### Supporting tests (no scenario)

| Task | Test file | Test name | Why it exists |
|---|---|---|---|
| T28 | `crates/mc-testkit/tests/capture_and_verify.rs` | `a_captured_frame_written_as_a_golden_matches_the_next_capture_of_that_scene` | The composition root end to end, against a **temporary** golden root with the update opt-in passed as a value. Nothing GPU-produced is committed; without this test `capture_and_verify` is uncovered code inside the coverage denominator |

### Shared fixtures

`crates/mc-testkit/tests/scene/mod.rs` — the device fixture (`device_context`,
`request`), the self-verification scene (`clear`,
`top_half_white_over_black`), the four clear colours and the two pixel-counting
helpers, plus this suite's `TestResult`.

**The scene lives in the tests, not in the library** — the library ships no
shaders. The harness hands out a canvas and never a scene, which is what keeps
it ignorant of the renderer it exists to verify.

`device_context` has no skip arm. `OptIns::default()` leaves
`MYCRAFT_ALLOW_NO_GPU` unset, so a machine with no adapter reaches the error
and reddens the run; a returned `Acquisition::Skipped` is itself a failure,
because nobody asked for one.

`top_half_white_over_black` is the only fixture in this phase that can witness a
row inversion in the capture path — every other assertion is uniform
(FR-2.1-S1/S3/S4, which assert colour *format* and say so), a count (FR-1.1-S1,
FR-2.2-S1) or a duration (FR-2.3-S1). Its WGSL spans clip space `y = 0.0 ..=
1.0`, because **clip-space y is up while framebuffer row 0 is the top**: the
two point in opposite directions, and getting that backwards is where the
ecosystem's flipped frames come from. Do not simplify the fixture to a column
split or a uniform fill — a column split is invariant under precisely the
inversion the scenario exists to detect.

The four FR-2.1 expectations were verified against this machine's adapter
before the tests were handed over (NVIDIA GeForce RTX 4090, Vulkan): the
top-half fill returns white at (32, 0) and black at (32, 63); the linear clear
of 0.215 858 4 encodes to exactly 128; and the 25%-alpha white clear returns
(255, 255, 255, 64), colour channels unscaled. The ±1 in FR-2.1-S4 is therefore
headroom for another backend's rounding, not slack this adapter needs.
