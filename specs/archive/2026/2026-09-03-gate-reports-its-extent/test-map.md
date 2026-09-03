# Test map — SPEC-034, a red gate reports its own extent

Scenario ↔ test, one line per scenario. Test names are behavioural and carry no
scenario id; this file is where the two are joined.

Paths are relative to the repository root. Every test below lives in
`crates/mc-client/tests/gate_reports_its_extent.rs`.

| Scenario | Test name |
|----------|-----------|
| D1-S1 | `a_red_run_reports_every_test_it_was_given_and_still_fails` |
| D1-S2 | `a_red_run_reports_every_test_it_was_given_and_still_fails` |
| D1-S3 | `every_test_invocation_the_gate_runs_finishes_the_suite_it_was_given` |
| D1-S4 | `the_gate_carries_the_forbidden_flag_nowhere` |
| D1-S5 | `every_test_invocation_the_gate_runs_finishes_the_suite_it_was_given` |
| D1-S6 | `the_same_reading_tells_a_cancelling_invocation_from_a_finishing_one` |
| D1-S7 | `the_same_reading_tells_a_cancelling_invocation_from_a_finishing_one` |
| D1-S8 | `the_two_gpu_free_crates_are_reported_as_separate_stages` |
| D1-S8 | `the_same_reading_tells_two_staged_crates_from_one` |
| D1-S9 | `what_the_chain_still_hides_is_stated_where_the_chain_is` |
| D2-S1 | `every_description_of_quick_names_what_quick_runs` |
| D2-S2 | `every_description_of_quick_names_what_quick_runs` |
| D2-S3 | `the_same_reading_tells_a_complete_description_from_a_vague_one` |
| D2-S4 | `every_stage_the_gate_reports_has_a_row_in_the_stage_table` |

D1-S1 and D1-S2 share one test deliberately: `--ignore-run-fail` produces a
byte-identical count and exit 0, so a count observed without its verdict is
green on the implementation that makes the gate pass with red tests.

D1-S4's control is the same assertion's second half, which reports line 6 of a
synthetic script carrying the forbidden flag; without it an absence assertion
would go green the day the scan stopped looking.

D2-S3's control includes `stages 1-3 only` as a description, which is what
three mutually-agreeing but vague labels would look like: it is graded as
naming nothing, so the cheap way to satisfy D2-S2 cannot pass.
