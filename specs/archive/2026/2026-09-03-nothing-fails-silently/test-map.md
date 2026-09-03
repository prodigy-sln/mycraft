# Test map — nothing fails silently

One row per scenario, 1:N. Test names are behavioural and carry no scenario ID;
this file is the only place the mapping lives.

## Defect 1 — a submerged player can aim at nothing (PRO-961)

| Scenario | Test | File |
|---|---|---|
| D1-S1 | `a_swing_from_inside_a_block_you_can_see_through_takes_the_block_four_cells_along` | `crates/mc-sim/tests/an_eye_inside_a_block_it_can_see_through.rs` |
| D1-S2 | `a_block_the_eye_is_inside_that_cannot_be_seen_through_is_the_target_at_no_distance` | `crates/mc-sim/tests/an_eye_inside_a_block_it_can_see_through.rs` |
| D1-S3 | `a_swing_from_inside_the_shipped_water_breaks_the_lakebed_rather_than_the_water` | `crates/mc-sim/tests/aiming_while_submerged_in_the_shipped_water.rs` |
| D1-S4 | `a_placement_from_inside_the_shipped_water_goes_against_the_lakebed` | `crates/mc-sim/tests/aiming_while_submerged_in_the_shipped_water.rs` |
| D1-S5 | `a_swing_from_inside_a_block_you_can_see_through_with_nothing_beyond_it_finds_no_target` | `crates/mc-sim/tests/an_eye_inside_a_block_it_can_see_through.rs` |

**D1-S2 is the control and is green before the fix**, which is what a control
is: it states the behaviour the fix must not take away. Its falsifier is the
over-eager shape — an implementation that skips the origin cell unconditionally
— against which it is the only one of the five that reddens. Recorded here
because a scenario that is green at RED time is otherwise indistinguishable
from one nobody ran.

## Defect 2 — the launch notice names no uncovered key (PRO-990)

| Scenario | Test | File |
|---|---|---|
| D2-S1 | `a_built_set_covering_every_declared_key_composes_no_line` | `crates/mc-client/src/notice_test.rs` |
| D2-S1 | `the_shipped_root_covers_every_key_it_declares_and_nothing_is_said` | `crates/mc-client/tests/uncovered_keys_are_named_at_launch.rs` |
| D2-S2 | `every_key_the_built_set_left_uncovered_is_named_in_ascending_order` | `crates/mc-client/src/notice_test.rs` |
| D2-S2 | `a_root_declaring_a_key_nothing_baked_names_that_key_and_no_other` | `crates/mc-client/tests/uncovered_keys_are_named_at_launch.rs` |
| D2-S2 | `the_shipped_binary_over_a_root_declaring_an_unbaked_key_names_it_on_its_error_stream` | `crates/mc-client/tests/shipped_binary.rs` |
| D2-S3 | `a_key_the_built_set_covers_is_not_named_among_the_stand_ins` | `crates/mc-client/src/notice_test.rs` |

**RED was taken against a deliberately over-eager skeleton** naming every
declared key, so all six of the readings that existed at that point failed on an
*assertion* rather than on the absent function. That skeleton is what the
controls exist for and it is the one that reddens them: an inert composer passes
D2-S1 vacuously. `cargo nextest run -p mc-client --lib --test
uncovered_keys_are_named_at_launch --no-fail-fast` — 15 tests run: 9 passed,
6 failed, 0 skipped.

### Additional coverage

| Test | What it catches |
|---|---|
| `a_covered_key_no_block_declares_is_not_named_either` (`src/notice_test.rs`) | `symmetric_difference` in place of `difference` — a key the set covers that nothing declares. All three scenario readings are green for both, so nothing else in the spec can see it. |
| `the_shipped_binary_over_a_root_declaring_an_unbaked_key_names_it_on_its_error_stream` | The wire. Every other reading reaches the composer, so a client that composes the line and never says it out loud left **639 of 639** green — measured. With this reading the same deletion reddens exactly one test and nothing else moves. |

Its fixture is the shipped content root copied whole with one block added
declaring `example:undrawn`, a key no manifest bakes, and **no save**, so the
only notice on the child's stream is the one it waits for.

## Defect 4 — refusal guidance travels with the call (PRO-940)

| Scenario | Test | File |
|---|---|---|
| D4-S1 | `a_refusal_the_player_asked_for_is_reported_with_the_way_out_the_failure_carries` | `crates/mc-client/tests/a_refusal_carries_its_own_way_out.rs` |
| D4-S1 | `the_way_out_a_failure_carries_is_said_after_the_whole_chain_it_answers` | `crates/mc-render/src/window_test.rs` |
| D4-S2 | `a_refusal_with_no_way_out_is_reported_with_no_guidance_appended` | `crates/mc-client/tests/a_refusal_carries_its_own_way_out.rs` |
| D4-S2 | `a_failure_carrying_no_way_out_is_reported_with_nothing_appended` | `crates/mc-render/src/window_test.rs` |
| D4-S3 | `the_refusal_a_running_session_reports_carries_the_same_way_out_it_carries_at_launch` | `crates/mc-client/tests/a_refusal_carries_its_own_way_out.rs` |

**RED was taken against a skeleton appending a fixed sentence**, so all five
failed on an assertion. That is the shape the controls exist for: a constructor
appending nothing passes both D4-S2 readings vacuously. `cargo nextest run -p
mc-client -p mc-render --test a_refusal_carries_its_own_way_out --lib
--no-fail-fast` — 109 tests run: 104 passed, 5 failed, 0 skipped.

**Every reading here writes the sentence out.** Interpolating `way_out()` into
the expected text is the defect one level up — a reading that agrees with a
failure which has stopped saying anything. Measured: with
`PreparationError::way_out` returning the empty string, **four** of 644 redden
(the two above that write it out, `refusals_state_a_cause_once`'s occurrence
count, and `launch_acceptance`), and `refusals_state_a_cause_once`'s whole-text
comparison and `shipped_binary`'s expected refusal do **not**, because both
build their expectation out of `way_out()`. The prediction written before that
run said only the two new readings would bite; the occurrence count and
`launch_acceptance` were not predicted and are recorded here because a
prediction that was wrong in the safe direction is still a prediction that was
wrong.

The mc-render pair carries D4-S1 and D4-S2 over a fixture failure that states
its way out **per value** rather than deriving it from its message, which is what
separates a constructor consulting the failure from one appending a constant.

## Defect 3 — non-fatal notices name the process error stream (PRO-941)

| Scenario | Test | File |
|---|---|---|
| D3-S1 | `no_production_source_in_this_crate_writes_to_a_process_stream` | `crates/mc-client/tests/no_notice_names_the_error_stream.rs` |
| D3-S2 | `every_notice_a_caller_asks_for_is_read_back_off_the_sink_they_supplied` | `crates/mc-client/src/notice_test.rs` |
| D3-S3 | `a_notice_with_nothing_to_say_puts_nothing_on_the_sink` | `crates/mc-client/src/notice_test.rs` |
| D3-S4 | `a_fault_that_recurs_every_frame_is_said_once` | `crates/mc-client/src/app/report_test.rs` |

**Nine sites, not eight** — Defect 2 of this spec added `say_stand_ins`. D3-S1 is
carried by a scan rather than by nine readings because five of the nine sit behind
a window nothing in this workspace constructs; the four that do not are asserted
directly by D3-S2.

**RED against an inert `say_*` and a `SaidOnce` that says everything**, so all
four failed on an assertion: `cargo nextest run -p mc-client --lib --test
no_notice_names_the_error_stream --no-fail-fast` — 21 tests run: 17 passed,
4 failed, 0 skipped.

### Additional coverage

| Test | What it catches |
|---|---|
| `a_fault_that_changes_is_said_again` (`app/report_test.rs`) | A reporter that goes quiet after its first line. It satisfies D3-S4 perfectly and is useless; the pair is what pins the dedup to the *last* line rather than to everything ever said. |
| `a_notice_is_still_said_after_a_panic_poisoned_the_sink` (`notice_test.rs`) | A propagated poison, which would let one unrelated panic silently disable every later notice — this spec's own defect, reintroduced by its fix. Its first asserted element is the premise that the lock really was poisoned. |
| `the_same_scan_reports_a_source_that_writes_to_a_stream` | A recogniser that stopped matching. It earned its keep on arrival: `eprintln!` *contains* `println!`, so the first version reported every site twice and called the error stream the standard one. |
| `a_scan_that_read_no_source_says_so_rather_than_reporting_a_clean_crate` | A moved source root reading as a clean crate. |

### Mutation evidence (the owner's condition 3)

`mc-client` is excluded from coverage wholesale, so this is the verification.
Each site mutated alone, tree restored between, `cargo nextest run -p mc-client
--no-fail-fast` — 506 tests run each time.

**Three mutations per site, not one**, because "covered" is three questions:
does anything notice the site going back to the process stream, does anything
notice its *wording* change, does anything notice the *call* disappear.

| Site | Back to the stream | Wording changed | Call deleted |
|---|---|---|---|
| `notice.rs` — entry | scan (file-granular) | **7** | **1** |
| `notice.rs` — changed blocks | scan (file-granular) | **8** | **3**, incl. shipped subprocess |
| `notice.rs` — stand-ins | scan (file-granular) | **6** | **2**, incl. shipped subprocess |
| `notice.rs` — reload | scan (file-granular) | **3** | **1** |
| `app/report.rs` — unshowable edit | scan (file-granular) | **1** | nothing |
| `app/report.rs` — dropped frame | scan (file-granular) | nothing | nothing |
| `app/report.rs` — swatch | scan (file-granular) | nothing | nothing |
| `app/reload.rs` — content not taken up | scan | nothing | nothing |
| `events.rs` — cursor release | scan | nothing | nothing |

Counts of tests reddened, each mutation run alone with the tree restored between,
`cargo nextest run -p mc-client --no-fail-fast` — 506 tests before Defect 5 landed
and 508 after; every "nothing" row is a full green run of that size.

**Five of nine hold their wording. Four of nine hold the call.** The two numbers
are different and a single figure would hide which. The unshowable-edit row is the
whole of the difference: Defect 5 moved that decision out of the redraw, which
made its text assertable and left its wiring exactly as uninstrumented as the four
rows below it. **Half-closed, not closed** — reporting it as closed would be the
kind of false simplification this spec spent its day catching.

The scan is the only guard the last four rows have, and it is weak in two named
ways: it is blind to a site going *silent* while catching one going *back to the
stream*, and it is file-granular, so `notice.rs`'s four sites and
`app/report.rs`'s three share two outcomes. Per-site is cheap and strictly better
regardless of whether a windowed harness ever exists — **PRO-1008**.

Predictions written before the runs and wrong, all in the safe direction:
`say_reloading` was predicted to redden nothing and reddens the sink reading;
`say_entering` was predicted to redden `the_entry_sentence_is_said_once` and does
not; and `report_reload`'s wording was expected to be held by the documented-refusal
guard and is not — `printed_refusals.rs` composes that line from production pieces
rather than by calling `App::report_reload`, so the two are copies that agree.

**What the `say_entering` miss taught, which is the useful part.**
`the_entry_sentence_is_said_once` watches the sentence's *constant* and its *call
site*: reword the constant and it reddens, and it did, in the wording run above.
Empty the function body and it does not, because the body is neither of the two
things it reads. That is a fact about what that scan can observe, and it was only
visible because a prediction about it was written down first and then missed.

## Defect 5 — a dead re-mesh worker (PRO-949)

| Scenario | Test | File |
|---|---|---|
| D5-S1 | `a_worker_that_has_stopped_tells_the_player_their_edits_will_not_be_shown` | `crates/mc-client/src/app/report_test.rs` |

**RED against a skeleton answering one constant for every verdict**, so both
readings failed on an assertion: 20 tests run, 18 passed, 2 failed.

### Additional coverage

| Test | What it catches |
|---|---|
| `a_collect_that_is_not_a_fault_tells_the_player_nothing` | A frame path that reports every collect. Two of the three ordinary verdicts happen on most frames of a run, so that implementation puts a line on nearly every frame — and D5-S1 passes against it. |

### What this confirms, and what it leaves open

The issue is stale: the production behaviour landed with `45b30b44` (PRO-918).
What was never established is that anything asserted through it, and that splits
in two.

- **The wording is now held by a test.** It was unassertable only because
  `App::exchange_remesh` chose it inside a redraw. `report::said_about` is that
  choice as a function of the verdict. Measured: the `WorkerGone` arm answering
  `None` reddens exactly the D5 reading — 508 tests run, 507 passed, 1 failed.
- **Reaching the state is still unwitnessed, and that is unchanged.** No fixture
  can produce a live `Remesher` with a dead worker: the worker ends only when the
  `Remesher`'s own sender or receiver drops, and both are the `Remesher`. The one
  route is a panic inside it, which the crate's lints make un-inducible. A seam
  that would close it is an injectable worker — a production door added to be
  observed — and it is recorded rather than built, as `docs/technical/testing.md`
  already ruled.
- **Deleting the frame path's call to `said_about` reddens nothing** — 508 passed.
  So Defect 5 moved this site's *wording* into the held column and left its
  *wiring* where it was, beside the three other `App` reporters and the cursor
  release. Half-closed, and the Defect 3 table above carries both counts.
