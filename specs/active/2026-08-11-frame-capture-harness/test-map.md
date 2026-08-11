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

`crates/mc-testkit/tests/comparison_without_adapter.rs` deliberately builds its
own fixtures instead of using `common`, so the file names nothing outside the
core.

## Phase 2 — GPU-free policy, lifecycle and reporting (20 scenarios)

Not yet authored.

## Phase 3 — the wgpu adapter (12 scenarios)

Not yet authored.
