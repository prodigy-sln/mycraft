# Test map — SPEC-014, Luau host, sandbox and the hostile-mod harness

Scenario → test file → test name. Scenario IDs live here and in commit
messages only; test names are behavioural and carry none.

Extra tests beyond the one-per-scenario floor are recorded under **Additional
coverage** with one line stating what each catches. A test whose purpose is not
written down is one nobody can later judge.

---

## Phase 1 — Vocabulary and the two structural guards

### Scenario coverage

| Scenario | Test file | Test name |
|---|---|---|
| FR-8.1-S1 | `crates/mc-script/tests/client_closure.rs` | `the_client_binary_resolves_nothing_that_reaches_the_scripting_host` |
| FR-8.1-S2 | `crates/mc-script/tests/client_closure.rs` | `a_walk_that_cannot_see_the_scripting_host_at_all_refuses_instead_of_reporting_the_client_clean` |
| FR-8.1-S3 | `crates/mc-script/tests/client_closure.rs` | `the_same_walk_reports_a_dependency_the_client_genuinely_has_as_present` |

**Note on FR-8.1-S1's first run.** All three pass on their first run, and this
is the state T02 predicted rather than a discipline failure: the walker is
entirely inside the test crate, and the invariant it reads is a property of
`mc-client`'s manifest, which already holds. Its RED state is a manifest that
violates FR-8, which is not a state anybody creates on purpose. Landing it in
phase 1 is the point — it guards the invariant for the eight phases that add
surface, rather than certifying it after the fact.

### Additional coverage

| Test file | Test name | What it catches |
|---|---|---|
| `crates/mc-script/tests/mlua_containment.rs` | `no_source_outside_the_adapter_directory_names_the_scripting_backend` | A backend type escaping `src/luau/` into the crate's own modules or into the harness, which would make the pre-1.0 dependency unswappable and put vendor error formatting on the public surface. The verdict is enumerated with a per-root file count, so a root the scan stopped reading reports as a refusal rather than as the clean answer. |
| `crates/mc-script/tests/mlua_containment.rs` | `the_same_scan_reports_a_leak_wherever_it_sits_and_passes_over_the_adapter` | The guard above going quiet: a broken walk, a needle matching nothing, or an exemption grown to swallow the tree. Six directions in one fixture — a plain `src/` module, a sibling `_test.rs` (which the other text guards in this repository skip and this one must not), a harness file wearing the adapter's own file name, a `tests/luau/` directory that an any-segment exemption would excuse, the adapter itself, and a copy of the guard file whose needle must not be its own hit. This is what makes the guard meaningful on its first run; the empty scan is not. |
| `crates/mc-script/tests/vocabulary.rs` | `a_chunk_level_fault_names_its_chunk_and_claims_no_attachment` | A rendering that invents a subject or component for a fault raised before any attachment exists, sending a mod author looking for a callback that was never invoked. |
| `crates/mc-script/tests/vocabulary.rs` | `a_compilation_fault_names_the_line_it_parsed_out_of_the_backend_message` | The typed `line` field being carried but never rendered, leaving an author with an error and no way to locate it. |
| `crates/mc-script/tests/vocabulary.rs` | `an_invocation_fault_names_the_defining_chunk_the_round_and_both_halves_of_its_attachment` | The most common fault in the system losing the defining chunk, the round, or either half of the attachment. Whole-string equality, so a lost field is a different string — a `contains` check sees none of them. |
| `crates/mc-script/tests/vocabulary.rs` | `a_refused_cascade_names_the_target_it_would_not_admit_as_well_as_the_requester` | The two attachments a refusal concerns — the requester and the target turned away — collapsing into one, leaving an operator unable to tell which mod to look at. |
| `crates/mc-script/tests/vocabulary.rs` | `a_fault_the_host_raised_on_its_own_behalf_names_a_round_and_no_script` | The chunk-less origin arm, which nothing else renders, and the host filing its own memory pressure against a script author who did nothing wrong. |
| `crates/mc-script/tests/vocabulary.rs` | `a_fault_that_can_attribute_itself_to_nothing_says_so_rather_than_rendering_a_gap` | The fourth origin arm. Both fields are public and optional, so the shape is constructible and its rendering has to be decided rather than left to whatever a format string does with two empty options. |
| `crates/mc-script/tests/vocabulary.rs` | `every_fault_kind_is_distinguishable_from_every_other_by_equality` | A fault kind that is not comparable by equality, or a `PartialEq` that answers the wrong way for some pair. The harness's evidence check (phase 8) compares an expected sequence of outcomes against an observed one by equality, which says nothing unless every kind equals itself and nothing else. This is also why no variant may carry data. |
| `crates/mc-script/tests/vocabulary.rs` | `the_default_memory_backstop_leaves_room_above_the_enforced_cap` | A shipped default pair that puts the host into permanent memory pressure from its first invocation, with every fault attributed to the host rather than to its cause. **A relation, deliberately not a value:** phase 9 (T27) owns fixing, documenting and asserting the six default values, and nothing here pins one. |

**Not covered in phase 1, and deliberately.** `IsolationUnit` has no observable
property before handles exist — a test on it here could only assert that it is
constructible and `Debug`, which is a test that cannot fail. It earns one in
phase 3 (T10), where a handle's unit is what makes hot reload's scratch-VM
substitution checkable.

**`HostLimits`'s other five defaults are unasserted in phase 1**, per T27's
ownership. Asserting a value on two sources is the drift FR-9 exists to catch.

---

## Phase 2 — The VM, the closed sandbox, and chunk evaluation

### Scenario coverage

| Scenario | Test file | Test name |
|---|---|---|
| FR-1.1-S1 | `crates/mc-script/tests/chunk_environment.rs` | `a_chunk_cannot_add_a_global_of_its_own` |
| FR-1.1-S2 | `crates/mc-script/tests/chunk_environment.rs` | `a_chunk_declares_a_local_and_returns_its_value_to_the_host` |
| FR-1.1-S3 | `crates/mc-script/tests/chunk_environment.rs` | `a_chunk_cannot_replace_a_shared_library_function_for_the_chunks_that_follow_it` |
| FR-1.1-S4 | `crates/mc-script/tests/chunk_environment.rs` | `a_chunk_cannot_smuggle_a_global_past_the_freeze_with_a_raw_write` |
| FR-1.2-S1 | `crates/mc-script/tests/sandbox_surface.rs` | `no_library_or_loader_the_host_denies_can_be_reached_from_a_chunk` |
| FR-1.2-S2 | `crates/mc-script/tests/sandbox_surface.rs` | `every_permitted_library_and_builtin_reaches_a_chunk_as_the_kind_of_value_it_should_be` |
| FR-1.2-S3 | `crates/mc-script/tests/chunk_environment.rs` | `a_chunk_cannot_reach_behind_a_shared_metatable_to_change_what_every_string_does` |
| FR-1.2-S4 | `crates/mc-script/tests/sandbox_surface.rs` | `no_environment_or_collector_hook_the_host_denies_can_be_reached_from_a_chunk` |
| FR-1.2-S5 | `crates/mc-script/tests/sandbox_surface.rs` | `the_globals_a_chunk_can_reach_are_exactly_the_ones_the_host_declares` |
| FR-1.2-S6 | `crates/mc-script/tests/sandbox_surface.rs` | `a_chunk_that_fails_to_replace_print_still_has_its_own_call_recorded_at_the_host` |
| FR-4.1-S3 | `crates/mc-script/tests/chunk_evaluation.rs` | `a_chunk_that_fails_to_compile_names_the_line_it_failed_on_and_leaves_the_host_usable` |

**Configured values.** The three tests in `chunk_evaluation.rs` run under a
call-and-loop budget of **2,000**; every other limit stays at its shipped
default. The budget counts calls and loop edges rather than instructions, so a
bare infinite loop spends one per iteration and reaches that in milliseconds.
Every other test in this phase runs at the shipped defaults, because nothing
they do approaches a limit.

**A recorded weakness in FR-1.1-S4's second clause.** The scenario's later chunk
reads `smuggled` through its own environment, which resolves up the `__index`
chain — so it does catch a write that reached the shared globals. What it cannot
catch is a write confined to the first chunk's own environment, because a
per-chunk environment makes that invisible to any later chunk by construction.
The refusal is what carries this test; the second clause narrows rather than
proves. The additional-coverage entry below is what closes the gap the scenario
leaves.

### Additional coverage

| Test file | Test name | What it catches |
|---|---|---|
| `crates/mc-script/tests/chunk_environment.rs` | `a_chunk_cannot_plant_a_global_in_the_table_every_other_chunk_reads_through` | A chunk planting a global one level above its own environment, which the scenario's own `rawset(_G, …)` cannot reach. Freezing the environment and its metatable leaves the table they read through writable, and every later chunk then sees the planted name. FR-1.1-S4 stays green against that host — same verb, same name, one hop apart — which is the single-witness shape `testing.md` §2 names. Both write routes are tried, and the later chunk reading the name as absent is the half that matters. |
| `crates/mc-script/tests/chunk_evaluation.rs` | `a_chunk_whose_top_level_never_returns_is_aborted_and_leaves_the_host_usable` | A chunk whose top level runs away. No scenario reaches one — every chunk in FR-1.x terminates and the harness's own failure case is a compile error — so without this the first hostile script anybody evaluates hangs the host and the test binary holding it. |
| `crates/mc-script/tests/chunk_evaluation.rs` | `a_chunk_that_catches_its_own_abort_in_a_loop_is_still_stopped` | An abort that is raised once instead of latching. The protected call swallows it, the outer loop starts again, and the budget bounds nothing — measured, with the chunk returning normally. The same construction is covered for callback invocation and nowhere for chunk evaluation, and coverage of one entry point says the code ran, not that anything was checking the other. |

---

## Phase 3 — Registration, dispatch, the trampoline, and the budget

### Scenario coverage

| Scenario | Test file | Test name |
|---|---|---|
| FR-2.1-S1 | `crates/mc-script/tests/callback_budget.rs` | `a_callback_whose_loop_never_terminates_is_aborted_and_the_fault_names_its_attachment` |
| FR-2.1-S2 | `crates/mc-script/tests/callback_budget.rs` | `a_callback_that_finishes_its_loop_inside_the_budget_returns_its_result_and_reports_no_fault` |
| FR-2.1-S3 | `crates/mc-script/tests/callback_budget.rs` | `the_second_attachment_of_a_round_gets_a_whole_budget_after_the_first_exhausts_one` |
| FR-2.1-S4 | `crates/mc-script/tests/callback_budget.rs` | `an_attachment_aborted_in_one_round_is_granted_a_whole_budget_in_the_next` |
| FR-2.1-S5 | `crates/mc-script/tests/callback_budget.rs` | `a_callback_that_catches_its_own_abort_is_still_aborted_and_never_regains_control` |
| FR-4.1-S1 | `crates/mc-script/tests/callback_faults.rs` | `a_callback_that_raises_is_reported_against_its_attachment_with_the_text_it_raised` |
| FR-4.1-S2 | `crates/mc-script/tests/callback_faults.rs` | `a_raised_table_is_reported_without_its_string_metamethod_ever_running` |
| FR-4.1-S4 | `crates/mc-script/tests/callback_faults.rs` | `a_chunk_that_converts_the_same_table_itself_leaves_the_counter_reading_one` |
| FR-4.1-S5 | `crates/mc-script/tests/callback_faults.rs` | `two_callbacks_faulting_in_one_round_each_name_the_chunk_that_defined_them` |

**Configured values.** `callback_budget.rs` runs at a call-and-loop budget of
**10,000** — the figure every FR-2.1 scenario names — with every other limit at
its shipped default. Every other file in this phase runs at the shipped
defaults, and `backend_errors.rs` needs them: its call stack has to run out
before any limit the host enforces does.

**The work that follows an abort is sized from a measurement, not from
taste.** A loop of 9,000 iterations costs 9,001 ticks, measured by bisecting the
budget on this toolchain (9,001 completes, 9,000 aborts) — so it fits inside
10,000 with about 990 ticks to spare and fits inside no remainder of an
exhausted budget at all. That is what makes FR-2.1-S3 and FR-2.1-S4 distinguish
a host handing out a whole budget from one handing out what is left; a smaller
load would pass against both. Every expected total is arithmetic performed in
the test (`(1..=n).sum()`), never a number read back from the host.

**How FR-4.1-S2 and FR-4.1-S4 are constructed.** One chunk builder, one
statement apart: the callback either raises the table or converts it, and
afterwards returns what the metamethod's counter reads. Everything lives in one
chunk because a per-chunk frozen environment means a second chunk can see none
of it, and the counter reaches the host as a **returned value** while the
conversion's own output reaches it through `printed()` — two host-side
observables that need no way to read a field of a script table, which does not
exist until phase 6. S4 is what makes S2's zero mean something: a probe nothing
can fire reads zero forever.

### Additional coverage

| Test file | Test name | What it catches |
|---|---|---|
| `crates/mc-script/tests/dispatch_registry.rs` | `every_handle_taken_from_one_host_reports_the_same_isolation_unit` | An isolation tag derived from the handle or from the chunk rather than from the script state. Both look correct while one state exists, and both are wrong the first time hot reload substitutes a scratch-state function for a live one — the path whose partial application this crate calls a Blocker. **Stated plainly: this cannot observe disagreement, because there is nothing to disagree with yet.** Three handles, two chunks, two handle kinds, so both wrong derivations are visible; a second state is what turns this into a real comparison. |
| `crates/mc-script/tests/dispatch_registry.rs` | `an_attachments_invocation_count_rises_once_for_each_round_it_is_invoked_in` | A counter that does not count, counts rounds rather than invocations, or is written only at some later moment. Read after every round rather than once at the end. It is the counter FR-4.2-S6 later asserts *resumes* from its frozen value, so a phase that ships it uncounted leaves that scenario asserting a number nothing established. An attachment nobody registered reads zero rather than being missing. |
| `crates/mc-script/tests/backend_errors.rs` | `a_chunk_that_exhausts_the_call_stack_is_reported_whole_and_leaves_the_host_usable` | The third outcome the guard admits — the backend refusing while the guard is still clear — producing a shrug, a panic, or an invented limit fault instead of a whole one. **Expected to pass on its first run**; see the note below. |
| `crates/mc-script/tests/backend_errors.rs` | `a_callback_that_exhausts_the_call_stack_is_reported_against_its_attachment` | A refusal escaping the host's protected call on the invocation path, where it would arrive with no attachment to name and tell an operator that a mod failed without telling them which. Measured: the protected call catches a call-stack refusal, so this is an ordinary raised value and must be attributed like one. |
| `crates/mc-script/src/luau/trampoline_test.rs` | `an_allocation_refusal_is_classified_as_one_however_little_it_says` | A classification read from the error's **text**. An allocation refusal was measured to arrive carrying no message at all, so anything matching on text calls it a script error and files a condition of the whole state against whichever mod happened to be running. |
| `crates/mc-script/src/luau/trampoline_test.rs` | `every_other_refusal_is_a_script_error_however_much_its_text_sounds_like_memory` | The same misattribution pointing the other way: a mod author who writes `not enough memory` into an error of their own having it reported as the host running out of memory, which quarantine and the pressure rule both treat differently. Neither direction is visible to a test whose inputs all have text agreeing with their kind. |
| `crates/mc-script/tests/script_values.rs` | `a_number_leaves_script_as_a_whole_number_only_when_it_is_one_and_fits` | A host that classifies a number by **casting** rather than by asking the backend. `as i64` saturates, so a magnitude past the range becomes the largest whole number there is and a value that is not a number becomes zero — both arriving as an ordinary quantity a mod appears to have asked for, with nothing downstream re-checking a number already classified. Guards a phase-2 derivation with a great many dependents and, until now, no assertion of its own. Cases are at the edges deliberately: a test using only small numbers agrees with a saturating cast on every one of them. **Passes on its first run** — it is a guard on a correct implementation, not a red-to-green step. |

**Note on `backend_errors.rs`'s first test, which passes on arrival.** A chunk
whose top level exhausts the call stack was measured to reach the `Err`-with-a-
clear-guard state end to end in about three milliseconds, and the host already
answers `ScriptError` there — so the test is green the day it lands, in the same
way FR-8.1-S1 was in phase 1. It is kept because it is the **only** end-to-end
evidence that the arm is reached at all, and because it reddens against a host
that panics there, invents a limit fault, or loses the chunk and the line.

**What is unwitnessed, named rather than assumed away.** The classification
itself is covered beside the code that performs it, which is agreement between
two copies of one decision: the adapter can stop consulting
`classify_backend_error` entirely and both sibling tests stay green. **What
would go red if it did: nothing, in this phase.** The reason no end-to-end
witness exists for the distinction is that the two tests above reach the arm
only through a call-stack refusal, which is "everything else" and answers the
same as a text-matching host would. The allocation half becomes reachable in
**phase 4**, when `memory_backstop` makes a single allocation too large for the
interrupt to see fail while the guard is still clear — that is the phase that
can close this, and it should.

**Closed in phase 4.** `one_allocation_larger_than_the_state_may_hold_is_
reported_as_an_allocation_refusal`, below, is the end-to-end witness that was
missing: it reaches the arm with the guard clear and demands the allocation
answer, so an adapter that stopped consulting the classification reddens.

**Falsification recorded for `script_values.rs`, including the part that did not
bite.** Two mutations, both in a scratch copy of the crate, neither touching the
tree. (1) *Classify integral numbers by casting* — the two range cases reddened
(`2^63` and `1e30` both became `integer 9223372036854775807`) and the two
not-a-number cases did **not**, because `NaN.fract()` and `inf.fract()` are both
NaN and the guard therefore declined them. That is a fact about that mutation's
shape rather than a gap. (2) *Cast unconditionally* — the fraction became
`integer 3`, not-a-number became `integer 0`, and infinity became
`integer 9223372036854775807`, which is what those three cases are for. Between
them every case in the table is load-bearing against at least one plausible
wrong implementation.

**Not asserted in phase 3, deliberately.** `DispatchReport::quarantined` and
`DispatchReport::pending` both land this phase and are both empty by
construction until phases 5 and 7. Asserting they are empty here would be an
absence assertion that is true because the machinery does not exist, which is
the same reasoning that moved FR-3.1-S5 out of phase 4. `DispatchReport::order`
and `invocations` are asserted, in FR-2.1-S3, where a two-attachment round makes
both say something.

---

## Phase 4 — The memory cap, its reclamation, and its cause

### Scenario coverage

| Scenario | Test file | Test name |
|---|---|---|
| FR-3.1-S1 | `crates/mc-script/tests/callback_memory.rs` | `a_callback_that_allocates_past_the_cap_is_stopped_and_the_fault_names_its_attachment` |
| FR-3.1-S2 | `crates/mc-script/tests/callback_memory.rs` | `a_callback_that_allocates_well_inside_the_cap_completes_and_returns_its_result` |
| FR-3.1-S3 | `crates/mc-script/tests/callback_memory.rs` | `the_memory_a_stopped_callback_held_is_back_in_time_for_the_next_one` |
| FR-3.1-S4 | `crates/mc-script/tests/callback_memory.rs` | `a_callback_that_catches_its_own_allocation_failure_is_still_stopped` |
| FR-3.1-S6 | `crates/mc-script/tests/callback_memory.rs` | `an_allocation_fault_states_the_cap_it_exceeded_rather_than_saying_nothing` |

**FR-3.1-S5 is not in this phase and is not covered here.** It asserts an
attachment is *not quarantined*, which against a host with no quarantine
machinery is true by construction. It lands in phase 5 beside FR-4.2-S7, which
makes the same claim from the other side.

**The call-and-loop budget is 1,000,000 in every test in this file, and the
number is load-bearing rather than cautious.** The two limits mask each other:
filling a megabyte costs far more interrupt ticks than the 10,000 the FR-2.1
scenarios name, so under that budget the bomb is stopped for **budget
exhaustion**, the fault kind reads `BudgetExhausted`, and every FR-3 scenario
here goes green having measured the wrong mechanism. A million is a figure the
work below cannot approach — the largest of these callbacks spends on the order
of fifteen thousand ticks — so the only limit left that can stop a bomb is the
one this phase is about.

**The other configured values.** A memory cap of **1 MiB** (the figure every
FR-3.1 scenario names) and a memory backstop of **1.75 MiB**. The backstop is
chosen against a measurement rather than by taste, and it is the reclamation
test that pins it: after an abort at a 1 MiB cap the state holds about 1.43 MB
until something collects, and the following 512 KiB lands at about 1.96 MB. A
backstop between those two figures is what makes an uncollected host *fail* that
allocation, so FR-3.1-S3 can tell a host that gave the memory back from one that
did not. Higher and it could not; lower and the bomb would die at the allocator
instead of latching on the enforced cap. The same pair also keeps the host clear
of the memory-pressure condition throughout — an empty state's baseline plus the
cap is comfortably under the backstop — so nothing here is measuring phase 5's
mechanism by accident.

**Every bomb is bounded, and that is a falsifiability decision.** A bomb written
as `while true do` allocates until *something* stops it, and against a host that
enforces nothing that something is the machine: the test would take the run down
rather than failing, reporting nothing about which mechanism is missing. Each
bomb here asks for four times the cap in 4 KiB appends and then stops, so a host
that enforces nothing *returns* — visibly and in milliseconds — where a fault
was demanded. **No test in this phase can hang**, so none needs a
`.config/nextest.toml` override.

**FR-3.1-S2 passes on its first run**, and the reason is structural rather than
a discipline failure: it is the scenario asserting that the cap does *not* fire,
so a host with no cap at all satisfies it. There is no construction that makes
it red before the mechanism exists. It is kept because it is the control that
stops the other four passing against a host that refuses every allocation, and
because it reddens against a cap read as an absolute rather than as a delta
above the entry baseline — which is the ordinary condition of a server that has
been running for an hour.

**How FR-3.1-S6 is made falsifiable.** The scenario asks for a non-empty cause
stating the limit, and *non-empty* is nearly true by construction of any format
string. So the test keeps that assertion and adds two things. The cap must
appear in the cause **as its byte count in decimal** — an interface decision
taken here, since the limit is a byte count and there is no clean unit rendering
for an arbitrary one. And the same bomb is run at **two** caps, 1 MiB and
512 KiB, because a single cap cannot tell a composed cause from a constant
string that happens to contain the right number.

### Additional coverage

| Test file | Test name | What it catches |
|---|---|---|
| `crates/mc-script/tests/callback_memory.rs` | `a_host_whose_backstop_leaves_no_room_above_its_cap_refuses_to_start` | A host accepting a backstop that does not clear its own empty baseline plus the enforced cap. Such a host is in memory pressure from its first invocation, every fault it reports is about its configuration rather than about the mod that was running, and every later test of that condition measures nothing. A configuration error rather than a script fault — no mod caused it and no mod can fix it — so the refusal is a `HostError` at construction. The second half is the positive control: a host that refused every pair would satisfy the first half and be useless. |
| `crates/mc-script/tests/backend_errors.rs` | `one_allocation_larger_than_the_state_may_hold_is_reported_as_an_allocation_refusal` | The `Err`-with-a-clear-guard arm answering "the script failed" for an allocation refusal. **This is the end-to-end wiring witness phase 3 recorded as missing** and the reason that note said phase 4 should close it. One request larger than the whole state may hold never lands, so the usage the interrupt watches never moves and no limit of the host's can trip — which leaves the identity of the backend's own error as the only thing that can tell the two apart. It arrives through chunk evaluation rather than through a callback because the host's protected call catches this refusal exactly as it catches a call-stack one. |
| `crates/mc-script/tests/callback_memory.rs` | `a_bomb_built_from_binary_buffers_is_stopped_by_the_same_cap` | An allocation route the cap cannot see. Every other test here reaches the cap by allocating strings, so the cap had one witness and one allocator behind it. A buffer is binary, never interned and never shared, and the design records it as fully visible to this accounting on the strength of a measurement rather than of a mechanism — if it were not, a mod would have an unmetered heap sitting in plain sight in the permitted set, and every string-based test here would keep passing while it used it. |

**A fixture defect found by the implementation and corrected here, recorded
because it is the exact failure `testing.md` §2 names.** The bombs in
`callback_memory.rs` first appended `string.rep('x', 4096)` a thousand times and
documented themselves as reaching four times the cap. **The backend interns
strings**, so those thousand appends were a thousand references to *one* string:
measured, the loop grew the state by **120,629 bytes** against a cap of
1,048,576 and returned normally. Nothing can stop an invocation that never
allocates, so FR-3.1-S1 and FR-3.1-S4 were unpassable by any implementation, and
FR-3.1-S4's protected bomb had never retried anything — it reported zero catches
because nothing had failed. *A count cannot see shape*: fixture construction is
the one constraint no assertion can enforce, and a count-based check is
satisfied by a fixture measuring the wrong workload.

The fix concatenates the loop index, which makes every string distinct and
therefore separately allocated — measured at **1,445,562 bytes** for the same
loop. The in-tree evidence that it is load-bearing is that these tests now trip
a 1 MiB cap at all, which a 120 KB workload cannot do by arithmetic. The doc
comment on the fixture says so at the line somebody would "simplify".

**A constraint on where the `Err`-with-a-clear-guard arm is testable, which the
memory cap created.** Unbounded recursion is not free — each frame that stays is
script memory — and measured, the backend refuses it only after the state has
grown by **1,911,289 bytes**. So under the shipped 256 KiB cap the host stops
deep recursion for *memory* long before the C stack runs out, and reports
`Allocation`. That is the correct answer to a different question, and it is why
the two call-stack tests in `backend_errors.rs`, green through phase 3, went red
the moment a cap existed: their host now configures a **4 MiB** cap, twice what
the recursion costs.

**The arm is therefore reachable only under a cap of roughly two megabytes**,
which is above what this host ships. Tidying those numbers down looks harmless
and silently deletes the only end-to-end witness the arm has, in both its halves
— the call-stack half here and the allocation half closed above. Written down
because the next person to read those constants will see a 4 MiB cap in a file
about stack overflows and assume it is arbitrary.

---

## Phase 5 — Quarantine, its lifting, and the pressure exclusion

### Scenario coverage

| Scenario | Test file | Test name |
|---|---|---|
| FR-4.2-S1 | `crates/mc-script/tests/callback_quarantine.rs` | `an_attachment_whose_callback_fails_three_rounds_running_stops_being_invoked` |
| FR-4.2-S2 | `crates/mc-script/tests/callback_quarantine.rs` | `an_attachment_that_recovers_before_the_threshold_keeps_being_invoked` |
| FR-4.2-S3 | `crates/mc-script/tests/quarantine_isolation.rs` | `another_component_on_a_subject_keeps_running_after_one_of_them_is_quarantined` |
| FR-4.2-S4 | `crates/mc-script/tests/quarantine_isolation.rs` | `the_same_component_on_another_subject_keeps_running_after_one_of_them_is_quarantined` |
| FR-4.2-S5 | `crates/mc-script/tests/callback_quarantine.rs` | `an_attachment_that_fails_three_different_ways_running_is_stopped_the_same_way` |
| FR-4.2-S6 | `crates/mc-script/tests/callback_quarantine.rs` | `a_quarantined_attachment_is_left_alone_and_its_count_stays_where_it_stopped` |
| FR-4.2-S7 | `crates/mc-script/tests/host_memory_pressure.rs` | `an_attachment_looping_under_a_full_state_is_still_not_quarantined_for_it` |
| FR-4.2-S8 | `crates/mc-script/tests/quarantine_lifting.rs` | `releasing_a_quarantined_attachment_puts_it_back_in_the_next_round` |
| FR-4.2-S9 | `crates/mc-script/tests/quarantine_lifting.rs` | `a_callback_attached_over_another_is_the_one_that_answers_next_round` |
| FR-4.2-S10 | `crates/mc-script/tests/quarantine_lifting.rs` | `a_callback_attached_over_a_quarantined_one_lifts_the_quarantine_and_answers` |
| FR-3.1-S5 | `crates/mc-script/tests/host_memory_pressure.rs` | `a_modest_allocation_that_fails_under_a_full_state_is_blamed_on_nobody` |

**An interface decision this phase takes, because the condition it turns on is
otherwise unobservable.** `ScriptHost::collected_memory_in_use() -> usize`
reports what the script state holds **after a collection**. Without it the only
window onto the memory-pressure condition is the very classification the
condition decides, so a test could neither establish that its fixture was valid
nor — more importantly — establish that an *ordinary* quarantine test's fixture
was not accidentally under pressure. The tests never ask the host for a pressure
verdict: each one computes the condition itself, from the reading plus
`limits().memory_cap` against `limits().memory_backstop`, so a host whose
classification is wrong does not also get to decide whether the fixture was
sound. The collection is what makes the figure mean retention rather than
garbage: 1,434,679 B was measured surviving until an explicit collection, and a
raw reading would report pressure caused by memory nobody is holding.

**Configured values.** All four files configure `fault_threshold` explicitly at
**3** — the figure every scenario names — rather than inheriting the shipped
default, so nothing here asserts anything about what the host ships (T27 owns
that). `callback_quarantine.rs`, `quarantine_isolation.rs` and
`quarantine_lifting.rs` run at a call-and-loop budget of **50,000** and at the
shipped memory limits; the budget is sized from both sides at once, large enough
that the mixed-kind test's allocation bomb is stopped by the memory cap rather
than by its ticks — the two limits mask each other and the masking is measured —
and small enough that a runaway loop ends in single-digit milliseconds.
`host_memory_pressure.rs` runs at a cap of **1 MiB** (the figure both its
scenarios name), a backstop of **2 MiB** and the same 50,000 budget.

**How the pressure fixture raises the baseline, and why it converges.** A ballast
attachment retains 16 KiB per invocation in a table its **closure** holds — the
retention mechanism the design names as the one no per-invocation limit can
reach — and the fixture invokes it until a further 64 KiB will not fit below the
backstop. Two constructions are load-bearing. **The retention is buffers, not
strings**: the backend shares identical strings, so a loop retaining the same one
allocates it once and the fixture would raise nothing while every count in it
still read plausibly — the same defect corrected in phase 4's bombs. And **the
reading taken before each ballast round is a collected one**, which is what makes
the loop converge rather than stall: after a collection, what is free below the
backstop is exactly what the reading says, so the ballast can allocate precisely
while the stop condition has not fired. The retained increment is deliberately
smaller than the modest allocation under test, so the room left at the end is
between one increment short of 64 KiB and none — never enough for the invocation
under test, always enough for the host's own working room. The fixture **fails
loudly** if it cannot reach that state in 256 rounds; a fixture that quietly
established nothing would leave both tests green and measuring a host under no
pressure at all.

**The anti-vacuity assertion, in both directions.** `pressure_before` and
`pressure_after` are asserted **true** in both pressure tests, on both sides of
the rounds they measure. The two tests in `callback_quarantine.rs` that
quarantine an attachment assert the same condition is **false** while they do it.
That pairing is the point: faults raised under pressure do not count at all, so a
fixture that had put an ordinary quarantine test into that condition would leave
it measuring nothing, and nothing else in those tests could see it.

**Two skeletons, because one cannot redden this phase.** Every test was run
against two deliberately-less implementations in a scratch copy of the crate, and
the pair is what shows each one falsifiable:

- **Nothing is ever quarantined.** Eight of the eleven go red on an assertion.
  The three that do not are the three asserting an attachment is *not*
  quarantined — FR-4.2-S2, FR-4.2-S7 and FR-3.1-S5's quarantine half — which a
  host with no quarantine satisfies by construction, the same structural case as
  FR-3.1-S2 in phase 4.
- **Quarantined on the first fault, with no memory-pressure exclusion.** Those
  three go red — FR-4.2-S7 reading `(faults 1, invocations 1, quarantined true)`
  against `(3, 3, false)` — and so does everything else that expects the third
  fault to be the one that stops it.

No test is green against both.

**FR-4.2-S9 is already satisfied by the tree and its test passes on arrival.**
`attach` has replaced the registered callback since phase 3, and the invocation
count has been cumulative since then too, so the scenario's own claim holds
before this phase starts. What was untested is the half that is *not* a
replacement — lifting a quarantine — and that is FR-4.2-S10, which reddens. The
test is kept because it is the only witness that a replacement is what answers,
and because it is genuinely falsifiable: a mutation making `attach` reset the
invocation count turns its third element from 2 into 1 and it goes red.

**Strengthened assertions, recorded against the scenarios they belong to.**
FR-4.2-S8's test also releases an attachment that was **never** quarantined and
asserts `release` answers `false` — without that control a `release` that always
answered `true` would tell an operator it had undone something it had not.
FR-4.2-S9's and FR-4.2-S10's tests also assert the invocation count **across**
the lift, which is the rule that the count resumes rather than resets applied to
replacement; DF6 states it for both paths and only the release path has a
scenario. FR-4.2-S7 asserts the fault count and the invocation count beside the
quarantine, because "not quarantined" is equally true of an attachment that was
never invoked and of one that never failed.

**What FR-4.2-S7 deliberately does not assert.** The *kind* of fault a looping
callback produces under pressure. The scenario is about what is counted, and
pinning what such a fault is called would settle a question it does not ask —
the design says a fault raised under pressure is reported as host memory
pressure, and FR-3.1-S5 is where that is asserted.

**One test can hang rather than fail**, and it is the only one in this phase:
`an_attachment_looping_under_a_full_state_is_still_not_quarantined_for_it`
invokes a non-terminating loop three times and relies on the budget abort that
phase 3 built. Its cost at the configured budget is single-digit milliseconds, so
a `.config/nextest.toml` override on that exact name can only fire if the abort
is genuinely absent — the same reasoning, and the same load-bearing exact-name
filter, as the two runaway-chunk tests already listed there.

---

## Phase 6 — The engine reads script values raw

### Scenario coverage

| Scenario | Test file | Test name |
|---|---|---|
| FR-6.1-S1 | `crates/mc-script/tests/raw_field_reads.rs` | `a_field_the_table_lacks_is_reported_absent_without_its_index_metamethod_running` |
| FR-6.1-S2 | `crates/mc-script/tests/raw_field_reads.rs` | `a_field_the_table_genuinely_has_comes_back_with_its_value` |
| FR-6.1-S3 | `crates/mc-script/tests/raw_field_reads.rs` | `a_table_whose_index_never_returns_is_read_and_leaves_its_attachment_a_whole_budget` |
| FR-6.1-S4 | `crates/mc-script/tests/raw_field_reads.rs` | `a_chunk_that_indexes_the_same_absent_field_itself_leaves_the_counter_reading_one` |

**An interface decision this phase takes, because the return type has nowhere
else to put the answer.** A field the table does not hold reads back as `None`,
never as `Some(ScriptValue::Nil)`. In script the two are one state, so the host
has exactly one honest answer for both; spending it on `Some(Nil)` would make
every read `Some` and leave the `Option` saying nothing, with the caller that
has to branch on "the mod did not supply this" reading a value to find out. The
tests render absence as `absent` and nil as `nil`, so a host that chose the
other spelling reddens rather than passing quietly.

**Configured values.** A call-and-loop budget of **10,000** — the figure
FR-6.1-S3 names — with every other limit at its shipped default. Nothing else in
the file approaches a limit.

**How the probe is constructed, and why the two halves cannot be separated.**
One chunk builder, one statement apart, on the model `callback_faults.rs`
established: a counter table, a supplied table whose `__index` increments it,
and a callback that hands the table back the first time it is invoked and
afterwards performs one action and returns what the counter reads. The action is
nothing at all (S1, S2) or `print(supplied.ash)` (S4). Everything lives in one
chunk because a per-chunk frozen environment means a second chunk can see none
of it, and the two observables — the counter as a **returned value**, the
metamethod's own answer through `printed()` — are host-side facts that do not
lean on the method under test. An assertion that read `read_field` to check
`read_field` would prove nothing. S4 is what makes S1's zero mean anything: an
increment nothing can trigger reads zero forever. S2 is the control in the other
direction, since a host answering `absent` to everything satisfies S1 perfectly.

**FR-6.1-S3 is satisfied, as the scenario words it, by the implementation it
exists to forbid — measured, not supposed.** An ordinary indexed read runs the
looping metamethod, the interrupt stops it on whatever the previous entry left
behind, and the read swallows the refusal: the read *completes*, the attachment
afterwards runs its 9,000 iterations, and the scenario passes against a host
reading exactly the way FR-6.1 forbids. So the looping metamethod **says one
line before it loops**, and the test asserts that line was never printed
alongside the scenario's own two halves. That is `testing.md` §2's *stronger
observable beside a scenario's own*: the scenario's assertion is kept intact and
what makes it falsifiable is added in the same test. The 9,000 iterations are
sized from phase 3's measurement — 9,001 ticks against a 10,000 budget, under a
thousand to spare — so a read that charged the attachment, latched its guard or
left the state part-way through something aborts it.

**Two skeletons, because neither alone can redden this phase.** Every test was
run against both in a scratch copy of the crate, nothing in the tree touched:

- **The ordinary indexed read** (`Table::get`). Four of the five go red. The one
  that does not is FR-6.1-S2, which such a host answers correctly — it is the
  control, and its being green here is what it is for.
- **Every field is absent** (`None` unconditionally). FR-6.1-S2 goes red, and so
  does the metatable-chain test below, whose present-field half is the same
  control read through the same handle. The other three are green, which is the
  point of running this skeleton at all.

No test is green against both. A third run against a correct raw read
(`Table::raw_get`) puts all five green, so nothing here is unpassable — the
failure mode phase 4's interned-string fixture defect is on record for.

**One test can hang rather than fail, and it is a conditional rather than a
certainty.** `a_table_whose_index_never_returns_is_read_and_leaves_its_
attachment_a_whole_budget` was measured *not* to hang against the ordinary
indexed read, because the interrupt is armed for the life of the state and the
runaway metamethod is stopped on the previous entry's leftover budget in
milliseconds. What hangs is a host that disarms or neutralises the interrupt
around the read in order to honour "consumed none of the attachment's budget"
literally — a plausible misreading of the scenario rather than a fanciful one.
The exact-name override in `.config/nextest.toml` costs one filter entry and
converts that wedged run into a red one; the same load-bearing-name caveat
applies as for the three tests already listed there.

### Additional coverage

| Test file | Test name | What it catches |
|---|---|---|
| `crates/mc-script/tests/raw_field_reads.rs` | `a_field_that_exists_only_behind_the_metatable_is_reported_absent_as_well` | A metatable whose `__index` is a **table** rather than a function. The counter probe cannot be built that way — a table cannot count — so every scenario in this phase reaches the raw read through the function case only, and a host special-casing that one while still reading through a table would satisfy all four. What it hands the engine is a field no mod stored, arriving as though one had, which is exactly what the follow-up list the cascade reads out of a returned table (T22) must never contain. The present field is read through the same handle in the same test, so `absent` cannot be a handle that quietly stopped working. Red against both skeletons. |

**Unwitnessed, and measured to be so rather than assumed either way: the chunk
a value read out of a table inherits.** `read_field` labels whatever it hands
back with the chunk name carried by the table it read from, so a callback
obtained through a raw read reports the chunk that defined it and a fault
raised by that callback names a file its author can open — the property
FR-4.1-S5 buys for callbacks obtained from `evaluate`. **Replacing that read
with a constant wrong chunk name reddened nothing**: all 86 tests standing at
the time passed, and no crate in the workspace depends on `mc-script`, so those
86 were the whole population that could have seen it. Reverted by re-editing the
line, `git diff --exit-code` clean.

The reason is structural rather than an oversight in this phase: every callback
the suite attaches comes from `evaluate`, which labels it at the moment the
chunk producing it is named, and nothing yet reads a **function** out of a table
and attaches it. The cascade does read a returned table (T22), but the follow-up
entries it reads are identities rather than callables, so no fault is ever
attributed through a value this line labelled. **What would go red if the label
stopped being propagated: a test that evaluates a chunk returning a table of
callbacks, reads one out with `read_field`, attaches it, and asserts the fault
it raises names the chunk that defined it.** That test is not written, because
nothing in this spec attaches a callback that way and writing the consumer to
justify the test would be building surface no scenario asks for. It ships as an
argument honestly labelled: the propagation is correct by inspection and is the
only sensible thing for the line to do, and it is held by no assertion until a
spec adds the caller that needs it.

---

## Phase 7 — The bounded cascade

### Scenario coverage

| Scenario | Test file | Test name |
|---|---|---|
| FR-5.1-S1 | `crates/mc-script/tests/cascade_queue.rs` | `requested_follow_up_work_is_entered_only_after_its_requester_has_returned` |
| FR-5.1-S2 | `crates/mc-script/tests/cascade_rounds.rs` | `a_cascade_that_never_terminates_ends_its_round_and_blames_whoever_asked_for_what_is_left` |
| FR-5.1-S3 | `crates/mc-script/tests/cascade_rounds.rs` | `a_cascade_of_two_hundred_invocations_finishes_across_four_bounded_rounds` |
| FR-5.1-S4 | `crates/mc-script/tests/cascade_queue.rs` | `a_cascade_that_exactly_fills_a_round_completes_inside_it_without_a_cascade_fault` |
| FR-5.1-S5 | `crates/mc-script/tests/cascade_queue.rs` | `follow_up_work_naming_a_quarantined_attachment_is_skipped_without_a_fault_or_an_invocation` |
| FR-5.1-S6 | `crates/mc-script/tests/cascade_refusal.rs` | `follow_up_work_past_the_pending_bound_is_refused_and_each_refusal_names_what_was_dropped` |

**The return convention these tests bind, stated here because the architecture
names it without spelling it.** A callback requests follow-up work by returning
a table carrying a `follow_up` field holding an array of `{ subject, component }`
entries; a callback returning anything else is returning a result and requesting
nothing, which is what every earlier phase's callback does and stays true. The
host reads the field, each array slot and each `subject`/`component` **raw**, so
a metatable a mod hung on the returned table can neither run on the host's
schedule nor observe which parts the host looked at.

**Three files rather than one**, split by question rather than by task: what the
queue does with one request (`cascade_queue.rs`), what happens when a cascade
outlives its round (`cascade_rounds.rs`), and what happens when the queue has no
room (`cascade_refusal.rs`). One file carrying all six would have run past the
600-line limit with the prose each discriminator needs.

**Every expected quantity is written from the configured limits.** S3's
`64 + 64 + 64 + 8 = 200` is built in the test from `ROUND_BOUND` and
`CASCADE_LENGTH` and never read back off a run; S6's nine invocations are
`1 + PENDING_BOUND` and its four refusals are the tail of the requested list
past what the queue holds; S2's blamed attachment is *derived* from the parity of
the round bound by a helper rather than transcribed, because the pair alternates
and an odd bound would move the answer. Where a count could be satisfied by a
host that ran fewer invocations than it reported, the callback's own tally comes
back as the round's result — a script-side oracle sharing no code with the host's
counter — and `invocation_count` is read as a third witness.

**Falsification: one correct skeleton and six deliberately-less ones**, all run
in a scratch copy of the crate with nothing in the tree touched. Against a
correct queueing host all eight tests pass, so none of them is unpassable. Each
deliberately-less host is listed with exactly what it reddens:

| The host does this instead | What goes red |
|---|---|
| Enters follow-up work inline, recursing rather than queueing | S1, S3, S6 and both quarantine witnesses; **and the non-terminating test overflows the stack and aborts the process**, taking its whole test binary with it — which is the outcome the decision exists to prevent, observed rather than argued |
| Charges a skipped quarantined entry against the round bound | S5, and nothing else |
| Enforces no round bound | S3 goes red and **S2 wedges** (see the hang note below); S1, S4, S5 and both refusal tests stay green, which is what S4 being the control means |
| Enforces no pending bound | S6 and the refusal witness, and nothing else |
| Blames the entry that could not run rather than its requester | S2, and nothing else |
| Counts cascade faults toward quarantine | both witnesses below, and nothing else |
| Reads `follow_up` with an ordinary indexed read | four of phase 6's five tests in `raw_field_reads.rs`, whose probe hands back a table with a counting `__index` — which is why this phase adds no test of its own for that read |

**S1 was rewritten after the first falsification run.** Its original form asserted
that the requester's last printed line preceded the follow-up's first. That
passed against the re-entrant host, because re-entry here happens after the Lua
call has returned, so no print order can see it — an assertion that could not
fail for the reason it was written. It now runs a fan-out (`smelt` asks for
`vent` and `flue`; `vent` asks for `ash`), where a queue drain gives
`smelt, vent, flue, ash` and a depth-first walk gives `smelt, vent, ash, flue`.

**One test can hang rather than fail**, and it is the same shape as the four
already listed in `.config/nextest.toml`:
`a_cascade_that_never_terminates_ends_its_round_and_blames_whoever_asked_for_what_is_left`.
Its cascade is genuinely endless and its round bound is the only thing that ends
it; against a host that queues without bounding the round, the run wedges rather
than going red — measured, not supposed. Note what the bound has to be: **bounded
rounds, not a bounded loop**. The entry it needs is an exact-name filter, with
the same load-bearing-name caveat as the four already there.

### Additional coverage

| Test file | Test name | What it catches |
|---|---|---|
| `crates/mc-script/tests/cascade_rounds.rs` | `work_deferred_to_a_later_round_does_not_count_against_the_attachment_that_asked_for_it` | A host routing a cascade fault through the same bookkeeping as a raised error. The rule that neither cascade kind counts toward quarantine has been implemented since phase 5 and asserted nowhere. It cannot be witnessed at the threshold the scenarios use — the blamed requester succeeds dozens of times a round, so its faults are never consecutive and the reset hides the defect — so this runs at a threshold of one over two rounds, where a host that counted the deferral invokes nothing in the second. |
| `crates/mc-script/tests/cascade_refusal.rs` | `work_refused_for_want_of_room_does_not_count_against_the_attachment_that_asked_for_it` | The same defect through the other fault kind, which reaches quarantine's bookkeeping by a different path in every plausible implementation. Same construction: a pending bound of one, a threshold of one, and a second round that a host counting the refusal never runs. |

---

## Phase 8 — The hostile-mod harness

### Scenario coverage

| Scenario | Test file | Test name |
|---|---|---|
| FR-7.1-S1 | `crates/mc-script/tests/hostile_containment.rs` | `the_harness_reports_exactly_the_six_hostile_cases_it_is_named_for` |
| FR-7.1-S2 | `crates/mc-script/tests/hostile_containment.rs` | `all_six_hostile_cases_are_contained_in_sequence_and_the_host_still_evaluates_afterwards` |
| FR-7.1-S3 | `crates/mc-script/tests/hostile_containment.rs` | `a_case_that_runs_to_completion_without_its_declared_evidence_is_reported_uncontained_by_name` |
| FR-7.1-S4 | `crates/mc-script/tests/hostile_containment.rs` | `every_hostile_case_declares_the_containment_evidence_it_requires` |
| FR-7.1-S5 | `crates/mc-script/tests/hostile_containment.rs` | `a_case_whose_script_does_not_compile_is_reported_not_exercised_by_name` |

**The harness lives at `crates/mc-script/tests/support/hostile/`** — `mod.rs`
(the six cases, the three-valued verdict, the judge), `scripts.rs` (the hostile
Luau, generated from the host's own declarations) and `exercise.rs` (how each
shape is driven and what is observed) — wired into the driving suite as
`#[path = "support/hostile/mod.rs"] mod hostile;`, following
`crates/mc-client/tests/support/input/`. It is deliberately not in `mc-testkit`,
which may name no `mc-*` crate and so could not reach what it verifies.

**Interface decisions taken here, beyond the four types the architecture
declares.** `run(&mut ScriptHost, &HostileCase) -> CaseReport`, where
`CaseReport { name, outcome }` carries the name because two scenarios require a
failed case to be *named*. `HostileCase::from_source(name, requires, source)`
builds a case whose script the caller supplies — the only way to reach
`Uncontained` and `NotExercised`, since against a working host the six cannot
produce either, and it is judged through exactly the path the six go through.
The script itself is a private field, so a case remains "a name and the evidence
it requires" from outside. `probe_denied_globals` exposes what the escape case
asked and what answered.

**Every case runs against `ScriptHost::new()` — the shipped defaults — and the
harness constructs no limits at all.** That is what makes the numbers doing the
containing the numbers a server runs, and it is what `harness_boundaries.rs`
enforces. Where a workload needs a size, it is derived from the host: the bomb
asks for four times `memory_cap` in 4 KiB pieces, and the cascade's fan-out is
`1 + 2 x ceil(pending_bound / round_bound)`, which is what guarantees the queue
reaches refusal inside one round with a factor of two in hand.

**Two fixture constructions are load-bearing and neither is visible in an
assertion.** The bomb allocates *buffers*, because the backend shares identical
strings and a string bomb grows the state by almost nothing. The cascade stops
requesting after a round's worth of invocations and the harness then drains the
queue, so the next case does not start behind somebody else's backlog — a
harness that skipped the drain would report `hostile-index` uncontained for a
reason that has nothing to do with metamethods.

**`DENIED_GLOBALS` is held by three independent things**, which is what its
one public-for-the-harness justification is worth: the escape probe's script is
generated from it; the script reports back the names it was *asked* about and
`the_escape_case_asks_the_running_script_about_every_global_the_host_declares_denied`
compares that to the constant; and the case's own verdict requires the reported
list to equal the host's whole declaration, so an empty `standing` from a probe
that asked about nothing is not containment. Measured: a harness rewritten to
probe a hardcoded three-name list reddens all three at once
(`harness_boundaries`, the probe test, and FR-7.1-S2 naming `sandbox-escape`).

**Falsification, part one: two skeletons, because one is not enough.** Both were
run in the real tree before the harness existed, and neither reddens what the
other does. An **over-eager** harness (six cases it cannot name, every verdict
`Contained`) reddens S1, S3, S4, S5 and both extra tests — and **passes S2**,
because "everything was contained" is exactly what it says. An **empty** harness
(the six declared correctly, every verdict `Uncontained`, no script run) reddens
S2 and S5 and passes S3. Every test in this phase has been observed red for the
reason it was written to catch.

**Falsification, part two: seven deliberately-less hosts**, each applied and
reverted in a scratch copy of the workspace with nothing in the tree touched
(`git diff --exit-code` clean throughout; only new test files are untracked).
Each reddens FR-7.1-S2 naming **exactly one** case, which is what says the six
are independent rather than six views of one mechanism:

| The host does this instead | What goes red |
|---|---|
| Leaves one denied global standing | `sandbox-escape` uncontained, **and** the probe test reporting `standing: ["os"]` |
| Reports a budget abort as a script error | `infinite-loop`, and nothing else |
| Enforces no per-invocation memory cap (`ceiling()` unbounded) | `memory-bomb`, and nothing else |
| Reads a supplied field with an ordinary indexed read | `hostile-index`, and nothing else |
| Admits every follow-up request (no pending bound) | `runaway-cascade`, and nothing else |
| Swallows a raised script error as a returned value | `faulting-callback`, and nothing else |
| The **harness** carries its own three-name deny list | `harness_boundaries`, the probe test, and S2 naming `sandbox-escape` |

**One test can hang rather than fail**, and it is the shape already listed five
times in `.config/nextest.toml`:
`all_six_hostile_cases_are_contained_in_sequence_and_the_host_still_evaluates_afterwards`.
Measured, not supposed: with the interrupt never armed, the run did not return
in 42 seconds and had to be killed. Its `infinite-loop` case is the one workload
here that cannot be bounded from the test side — that the host stops it *is* the
claim — where the bomb (four times the cap), the cascade (bounded drain) and the
retention fill (bounded rounds) all return against a host that enforces nothing.
It needs an exact-name entry, with the same load-bearing-name caveat as the five
already there. It costs 0.05 s against a correct host.

### Additional coverage

| Test file | Test name | What it catches |
|---|---|---|
| `crates/mc-script/tests/hostile_containment.rs` | `a_case_that_faults_for_a_reason_other_than_the_one_it_declares_is_uncontained` | A judge asking "did anything fault?" rather than comparing the kind — which would make the four fault cases interchangeable and let a memory bomb stopped by its tick budget count as containment, the exact masking this project has measured. The S3 test shows silence is not containment; this shows noise is not either, and the pair pins the judge from both sides. |
| `crates/mc-script/tests/hostile_containment.rs` | `the_escape_case_asks_the_running_script_about_every_global_the_host_declares_denied` | A probe that quietly narrows. An empty `standing` is what a probe asking about nothing reports, so this compares the names the *script* says it was asked about against `ScriptHost::DENIED_GLOBALS` in the host's own order — the only test in the suite that could see a harness which stopped consulting the declaration. |
| `crates/mc-script/tests/harness_boundaries.rs` | `the_hostile_harness_states_no_deny_list_no_limit_and_no_interrupt_of_its_own` | The harness-agreement failure: a harness carrying its own copy of the deny list, its own limits or a second interrupt reports all six contained on the day the host's enforcement is deleted, and every FR-7 scenario runs through the harness so nothing else can see it. Needles are generated from `ScriptHost::DENIED_GLOBALS`, so the guard cannot commit the offence it forbids. Enumerated verdict with a per-root file count: a harness directory that moved reports as a refusal, not as the clean answer. |
| `crates/mc-script/tests/harness_boundaries.rs` | `the_same_scan_reports_a_harness_file_that_states_a_policy_wherever_it_sits` | The guard going quiet — a broken walk, a needle that stopped matching, an exemption grown to cover the tree. The fixture commits **every** needle once, so the expected count is the needle list's own length and an unwatched needle reddens here; and it plants a harness file wearing the name of a suite file that is *allowed* to name these things, which a bare-name exemption would excuse. |
| `crates/mc-script/tests/retention_across_invocations.rs` | `a_mod_that_keeps_what_it_allocates_fills_the_state_and_the_failures_land_on_somebody_else` | R1's residual, in observable form: retention through a closure upvalue with no state API, tripping no per-invocation cap, growing past `memory_cap` while the aggregate stays under the backstop — and the misattribution that costs, with the innocent mod's failure blamed on nobody and **no fault anywhere naming the keeper, including its own**. The keeper's own failure is asserted so that "named in nothing" is not true by construction. Distinct from `host_memory_pressure.rs`, which asserts what pressure does to quarantine; this asserts who is *not* named, which is the price DF14 accepted. |
| `crates/mc-script/tests/retention_across_invocations.rs` | `a_suspended_coroutine_holds_what_it_allocated_across_invocations` | The second retention vector (DF10, R1). A value allocated during one invocation is reported from the coroutine's own stack during the next, and the state carried it in between. **Scoped deliberately:** what was measured about `coroutine` is that the interrupt fires inside `resume`/`wrap` and the latch is not void there, which settles *execution*. This settles nothing about containment in either direction — it records that the vector is real, so permitting `coroutine` is never later read as having settled both claims. If it ever reddens because retention has become bounded, R1 has been closed and the test should be retired rather than repaired. |

**Fixture-construction notes that no assertion can enforce.** The retention fill
fails loudly rather than returning a state it did not establish, and the values
it retains are made distinct — measured: with identical strings, 512 rounds
retain nothing (the state reaches 410,617 bytes, its own baseline) and the
fixture reports that instead of measuring an empty claim. The growth it asserts
is compared against the host's *own* `memory_cap` rather than against a number
written in the test.

---

## Phase 9 — The shipped defaults

### Scenario coverage

| Scenario | Test file | Test name |
|---|---|---|
| FR-9.1-S1 | `crates/mc-script/tests/shipped_defaults.rs` | `a_host_given_no_configuration_reports_every_limit_at_the_value_documented_for_it` |
| FR-9.1-S2 | `crates/mc-script/tests/shipped_defaults.rs` | `a_callback_that_never_returns_is_stopped_under_the_shipped_budget_and_reported_as_exhausted` |

**The six values and what each is answerable to**, recorded here because this is
the phase that fixes them and nothing else in the spec constrains them:

| limit | value | derivation |
|---|---|---|
| `call_and_loop_budget` | 1,000,000 | The largest plausible **unsliceable** workload. The reason is structural: `evaluate()` cannot be sliced and `dispatch` can, so a callback over budget has a mechanism — the queue, across rounds — and a chunk over budget has none. Worked against the measured 16³ costs (4,369 bare, 8,465 at one host call per cell, 12,561 at one Lua call per cell): the largest workload admitted is a top level walking a 64³ volume at one host call per cell, about 540,000 ticks. A million admits that with room and refuses a workload an order of magnitude past it. |
| `memory_cap` | 256 KiB | A delta above the entry baseline, so its floor is what a callback plausibly needs rather than what a state weighs. |
| `memory_backstop` | 16 MiB | Must exceed the measured 385,952-byte baseline plus the cap, i.e. 648,096; about twenty-five times that leaves room for legitimately retained state across many attachments while staying a number an operator can reason about. It is the one value that decides when scripting stops working for everybody. |
| `fault_threshold` | 3 | D3 — three consecutive faults, the count reset by a success. |
| `round_bound` | 64 | Invocations one round may perform. |
| `pending_bound` | 256 | Four rounds' worth of queued work at that round bound. |

**How the scenario's quantifier is met.** FR-9.1-S1 is stated over *the set of
limits the host reports* (DF8), not over four named ones. The expectation is
built as a whole `HostLimits` value **with no `..` in it**, so a seventh limit
added later leaves this file unable to build, naming the field it is missing.
Measured: adding a `reload_bound` field produced
`error[E0063]: missing field 'reload_bound' in initializer of 'HostLimits'` and
the crate's test targets stopped compiling. A limit cannot leave coverage
quietly; it can only leave it loudly.

**Why the expected values are literals in the test rather than the constants
themselves.** Reading the constants back would compare the host to itself. The
documentation and the constants stay on one source in
`crates/mc-script/src/limits.rs`; the test is the independent oracle that makes
changing either of them redden.

**Both scenarios pass on their first run**, and the reason is structural rather
than a discipline failure: this task owns *choosing* the six values, and the
provisional constants placed in phase 1 already carried the numbers the
derivation above arrives at. Nothing about that is evidence, so both are
falsified by hand below.

**Falsification: five mutations, all of which bit.** Each was applied in the
tree, observed, and reverted by re-editing the line, with `git diff --exit-code`
clean before the next one.

| The host does this instead | What goes red |
|---|---|
| `pending_bound` 256 → 255 | S1, naming the field and both values. One of the two limits most at risk — added last and named by nothing. |
| `memory_backstop` 16 MiB → 8 MiB | S1. Phase 1's `the_default_memory_backstop_leaves_room_above_the_enforced_cap` **stays green**, which is the difference between asserting the relation and asserting the value. |
| `new()` builds itself a 10,000-tick budget instead of the default | S1 on the budget value, **and** S2 — but S2 only on its counting half: the runaway is still aborted and still reported `BudgetExhausted`, exactly as it would be under the documented budget. |
| `dispatch` enforces 10,000 ticks while `limits()` keeps reporting the documented million | **S2 alone; S1 stays green.** This is the mutation that says S1 cannot substitute for S2 — a number reported is not a number enforced. |
| `call_and_loop_budget` → `u64::MAX` | S1 on the value, and S2 **wedges rather than failing**: the run was terminated at the 10-second bound after the interrupt never tripped. |

**S2 is the one test in this phase that can hang instead of failing**, for the
reason already recorded against five others: its runaway is bounded by the host
and by nothing on the test side. It has an exact-name entry in
`.config/nextest.toml`, with the same load-bearing-name caveat as the rest —
renaming it silently removes the bound. Measured: 0.033 s against a correct
host, terminated at 10 s against a host with no budget.

**Why S2 asserts a completing workload as well as an abort.** "Aborts a
non-terminating callback" is satisfied by a budget of ten, so the abort alone
says nothing about *which* budget shipped. The second attachment's load is
900,000 loop edges — a `for` loop of N iterations costs N ticks and its
enclosing call one more, measured — which completes under the documented million
with about a tenth of it spare and is aborted under anything much smaller. Its
total, 405,000,450,000, is arithmetic performed in the test rather than a number
read off a run. The pair brackets the shipped budget's order of magnitude, which
is the strongest claim available: no observable reports a tick count.

### Additional coverage

| Test file | Test name | What it catches |
|---|---|---|
| `crates/mc-script/tests/shipped_defaults.rs` | `the_round_bound_and_the_budget_together_stay_within_the_ceiling_stated_for_the_pair` | A raise of either default past what a single round was ever stated to cost. One round may enter script `round_bound` times and each entry may spend a whole budget, so the pair bounds a round; the two constraints on the budget do not covary, and nothing today can derive the pair jointly because there is no tick calling `dispatch` to derive it against. **That impossibility is the finding**, so the ceiling is stated at the pair's present value in `PROVISIONAL_ROUND_BUDGET_CEILING` and any raise has to be deliberate. Asserting the product were finite and non-zero would be unfalsifiable — the product of two `NonZero`s is non-zero — so the assertion is against the stated number, and the stated number is itself compared to a literal in the test so it cannot be quietly moved either. **Honest limitation:** today the ceiling and the product are equal, so with the six values asserted next door this cannot redden on its own; what it adds is the only assertion anything makes on the ceiling constant, and the obligation it places on the next change. |

**The half of that limitation which is not a limitation, measured after the
constant landed.** Lowering `PROVISIONAL_ROUND_BUDGET_CEILING` by one — to
63,999,999, with the six defaults untouched — reddened the ceiling test **and
nothing else**, naming both numbers in its failure message. So the direction
that is genuinely covered is a ceiling quietly moved to accommodate a raise,
which is the direction the constant exists to make impossible; the direction it
cannot see alone is a default raised past a ceiling raised with it, and there S1
reddens first. Reverted by re-editing the line, `git diff --exit-code` clean.

**The one test in this phase that arrives red.** The ceiling test binds to
`mc_script::PROVISIONAL_ROUND_BUDGET_CEILING`, which does not exist yet:

```text
error[E0432]: unresolved import `mc_script::PROVISIONAL_ROUND_BUDGET_CEILING`
   no `PROVISIONAL_ROUND_BUDGET_CEILING` in the root
```

An unresolved import stops every test target in the crate from building, so this
is the first thing the implementation closes. It is a `u64`, defined in
`crates/mc-script/src/limits.rs` beside the six constants and re-exported from
the crate root the way `HostLimits` is, written as an independent literal rather
than computed from the pair — a value computed from the two fields would move
whenever they did, which is the one thing a ceiling must not do.

**Not asserted here, deliberately.** Nothing tests that the shipped
`memory_cap`, `memory_backstop`, `fault_threshold`, `round_bound` and
`pending_bound` *enforce* at their shipped values, the way S2 tests the budget.
Each is enforced under a configured value by its own phase and reported at its
shipped value by S1; what stays unwitnessed is the wiring between the two for
five of the six. It is not a gap this phase can close cheaply — `fault_threshold`
and `round_bound` are already exercised at their shipped values by the hostile
harness, which constructs no limits at all, and a 16 MiB backstop cannot be
reached inside a second. Named rather than assumed away.
