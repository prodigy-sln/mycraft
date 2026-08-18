# Test map: SPEC-018 — A player entering a world is never left inside solid rock

Scenario → test file → test name. **Test names carry no scenario ID and never
will**; this file is the whole of the mapping, and it is where a reader goes to
find out which test grades which scenario.

**18 scenarios, 18 scenario tests, plus 10 additional-coverage tests.** Each
phase's test author appends below the previous phase's section rather than
editing it, and fills in that phase's mutation outcomes before the phase closes.

The names below are the mapping's floor, written from the scenarios before any
test exists. **A test author who lands on a better behavioural name updates this
file in the same commit** — the mapping is what must not drift, not the wording.

---

## Phase 1 — one door, and the whole of FR-1

**12 scenarios, 12 tests, plus 4 additional-coverage tests.**

Test command — the binaries this phase's scenarios live in:

```
cargo nextest run -p mc-sim -p mc-client -E 'binary(/^(one_way_seats_a_player|entry_)/)'
```

And what a phase boundary actually runs:

```
scripts/sdd-gate.ps1
```

**The blind window.** Commit B (T04) leaves the tree uncompilable, so nothing in
this table can run between commit B and commit C. The tests in commit A run
against today's signatures and are red *behaviourally* — the player is not
cleared — which is the RED this phase is opened on. Anything only the gate can
see accumulates silently across that window, which is why T04's procedure runs
`cargo clippy --workspace --all-targets --all-features -- -D warnings` by hand
on both sides of it.

### The mapping

| Scenario | File | Test |
|---|---|---|
| FR-1.1-S1 | `crates/mc-client/tests/entry_clears_a_resumed_player.rs` | `a_resumed_player_inside_a_solid_cell_starts_centred_on_the_clear_cell_one_step_sideways` |
| FR-1.1-S2 | `crates/mc-client/tests/entry_clears_a_resumed_player.rs` | `a_resumed_player_whose_box_abuts_a_solid_cell_without_overlapping_it_starts_exactly_where_the_save_recorded` |
| FR-1.1-S3 | `crates/mc-client/tests/entry_clears_a_resumed_player.rs` | `a_player_moved_at_entry_still_faces_the_yaw_and_pitch_the_save_recorded` |
| FR-1.1-S4 | `crates/mc-sim/tests/entry_leaves_a_generated_spawn_alone.rs` | `a_launch_with_no_save_to_resume_starts_the_player_at_the_spawn_the_generation_derives` |
| FR-1.1-S5 | `crates/mc-client/tests/entry_clears_a_resumed_player.rs` | `a_resumed_player_with_nothing_clear_within_eight_blocks_starts_where_the_save_recorded` |
| FR-1.1-S6 | `crates/mc-client/tests/entry_will_not_clear_a_player_off_the_map.rs` | `a_resumed_player_whose_only_clear_position_lies_partly_outside_the_world_starts_where_the_save_recorded` |
| FR-1.1-S7 | `crates/mc-client/tests/entry_clears_a_resumed_player.rs` | `the_first_snapshot_a_simulation_publishes_reports_the_position_entry_moved_the_player_to` |
| FR-1.2-S1 | `crates/mc-client/tests/entry_clears_whatever_the_load_reported.rs` | `water_made_solid_while_the_game_was_off_does_not_resume_the_player_inside_it` |
| FR-1.2-S2 | `crates/mc-client/tests/entry_clears_whatever_the_load_reported.rs` | `a_launch_that_accepted_no_changed_blocks_still_moves_a_player_saved_inside_stone` |
| FR-1.2-S3 | `crates/mc-client/tests/entry_clears_whatever_the_load_reported.rs` | `a_player_moved_at_entry_starts_within_eight_blocks_of_the_save_in_a_world_still_holding_its_blocks` |
| FR-1.3-S1 | `crates/mc-sim/tests/one_way_seats_a_player.rs` | `the_simulation_crates_sources_state_one_way_to_seat_a_player_and_it_reports_its_clearing` |
| FR-1.3-S2 | `crates/mc-sim/tests/one_way_seats_a_player.rs` | `a_second_source_that_seats_a_player_is_named_by_the_verdict` |

**12 scenarios, 12 tests, each scenario exactly once.**

### Which of the twelve are red in commit A, and which skeleton reddens the rest

Measured on the commit-A tree: **8 of the 16 tests fail, 8 pass.** The eight that
pass do so for two different reasons, and telling them apart is what says the
suite is doing something.

| Test | Commit A | Reddened by |
|---|---|---|
| FR-1.1-S1, S3, S7 · FR-1.2-S1, S2, S3 | **red** — the player is not cleared | the do-nothing skeleton turns these from red assertions into different red assertions; the real door greens them |
| `a_resumed_player_near_the_worlds_edge_is_moved_inward_rather_than_over_it` | **red** | same |
| FR-1.3-S1 | **red** as `AnotherSourceSeatsAPlayer([src/persistence.rs, src/replay/spawn.rs])` | the door landing |
| FR-1.1-S2, **S5**, S6 | green | **the over-eager skeleton only.** Each asserts the player was *not* moved, which a do-nothing entry satisfies for the wrong reason. `tasks.md` T05 names S2, S4 and S6 here and **omits S5** — S5 is in exactly the same position and the over-eager skeleton must be run against it too. |
| FR-1.1-S4 | green | **neither skeleton as `tasks.md` writes them.** Measured by the implementer: the over-eager skeleton re-centres the player on the cell they are already in, and the derived spawn is *already* horizontally cell-centred with a whole-number `y` — so the move is a no-op and S4 stays green. A second variant that **displaces** was needed. Without it S4 would have entered the phase unfalsified, which is the one thing it exists to prevent. |
| The three scan controls, and `the_derived_spawn_is_three_blocks_above_its_own_columns_surface_height` | green | nothing in this phase — a positive control over a `tempfile` fixture cannot be red on the tree it controls, which is the point of it |

### Additional coverage — what each one catches

The scenario floor above leaves four things that would go unreported, and each of
these is the kind `standards/global/testing.md` §1 names as earning its keep.

| File | Test | What it catches |
|---|---|---|
| `crates/mc-sim/tests/one_way_seats_a_player.rs` | `a_door_that_no_longer_hands_a_seating_back_is_named_by_the_verdict` | **A scan that can no longer look.** Second positive control, feeding `TheDoorNoLongerSeatsAPlayer`. Without it a renamed or moved door reads as a clean crate forever — the good verdict's own interior hole. |
| `crates/mc-sim/tests/one_way_seats_a_player.rs` | `a_tree_with_no_production_source_is_named_by_the_verdict` | **The same hole one level up.** Third positive control, feeding `NoSourceWasRead`. A vanished source root, a changed crate layout, or a walker that stopped recursing all land here rather than in a green pass. |
| `crates/mc-client/tests/entry_will_not_clear_a_player_off_the_map.rs` | `a_resumed_player_near_the_worlds_edge_is_moved_inward_rather_than_over_it` | **A second witness on the extent argument, by a different route from FR-1.1-S6.** S6 grades a *refusal* (the player stays put); this grades a *move* that the extent constrained. An extent that is too large fails one and not the other. A saved position near the world's edge is an input no reload fixture supplies. |
| `crates/mc-sim/tests/entry_leaves_a_generated_spawn_alone.rs` | `the_derived_spawn_is_three_blocks_above_its_own_columns_surface_height` | **A direct assertion on the shared derivation.** Every committed golden frame and FR-1.1-S4 both rest on `SPAWN_COLUMN` and `SPAWN_ABOVE_SURFACE` (`crates/mc-sim/src/replay/spawn.rs:38,45`). Asserting the formula localises a defect that the golden suites would otherwise report as dozens of image diffs at once. |

**Why FR-1.1-S1 is not additional coverage but load-bearing for FR-1.1-S6.** S1
asserts the exact destination — one cell sideways, horizontally centred, feet on
that cell's floor — and is therefore S6's positive control *through the extent
argument*: a shrunken extent rejects S1's destination while an over-large one
passes S1 and fails S6. Weakened to "covers no solid cell", S1 goes green under
both and S6 loses its control. This is recorded here because it is the thing most
likely to be softened by somebody tidying the suite.

### How the FR-1.3 scan can be fooled — two exposures, both measured

The scan matches **text** where it means to match **structure**, and that is a
property of how it is built rather than of either needle. Both exposures below are
one character wide, both are recorded rather than papered over, and **both belong in
`docs/technical/testing.md` before this spec closes** — `tasks.md` puts that in T13,
not T09, and it is the only home that survives the archive being pruned.

| Exposure | What fools it | Status |
|---|---|---|
| `published:` counts prose | Turning `crates/mc-sim/src/reload/mod.rs:91` from `///` into `//` makes the count 3 and reddens the scan with no second seating door anywhere. `production_text` strips `///` and `//!` and nothing else. | **Open, and accepted.** A struct-literal second door inside `simulation.rs` is the shape the needle is paid to catch, and a scan that reddens naming `reload/mod.rs` is a diagnosis a reader can act on in a minute. |
| `) -> Seated` was a prefix test | `) -> SeatedPlayer {` contains `) -> Seated`, so renaming the return type to anything beginning with `Seated` left the scan green over a door that no longer hands back the type the rule names. A rename to `Admitted` bit either way. | **Closed.** The needle is now `) -> Seated {`; the brace rejects a renamed type outright, because a rename carries its own characters between the name and the body. |

**Neither was reasoned out — both were measured, and the second only because M7 was
run.** That is the argument for running a mutation whose row already says what it is
expected to do: M7's expectation was "red as `TheDoorNoLongerSeatsAPlayer`", it was
red under one rename and green under another, and only running both said which.

The same warning applies to the phase-2 scan, whose `notice::say_entering(` needle is
module-qualified for a related reason and whose `Clearing` row is a **file set**
rather than a count. Nothing there is a prefix test today; whoever adds a sixth
needle should ask whether it is one.

### The traps these tests are written around

Three assertions redden a **correct** implementation if written the obvious way,
and each has a cheapest green that Out of Scope forbids.

1. **FR-1.1-S7 is read at tick 0.** A cleared player arrives with
   `on_ground: false` (`crates/mc-sim/src/persistence.rs:199`) and the clearing
   move touches position and velocity only
   (`crates/mc-sim/src/simulation.rs:245-248`), so tick 1 settles them by falling
   a fraction and landing. Read at tick 1 the `y` differs; the cheapest green is
   to ground the player or set `on_ground` in the search, both forbidden.
2. **`centre_of` returns `(x + 0.5, y, z + 0.5)`** —
   `crates/mc-sim/src/world/clearing.rs:120-127`, no `+ 0.5` on `y`. Deriving
   `y + 0.5` from "at that cell's centre" reddens a correct search; the cheapest
   green is editing the search.
3. **FR-1.1-S6's fixture is a real save file read through the shipped launch.**
   An in-memory fixture takes it off the only path where an over-large extent
   would actually be supplied.

### Mutations — outcomes recorded here before the phase closes

Break by hand, observe, revert **by hand** (never `git checkout -- <file>`),
confirm `git diff --exit-code` clean. **Every outcome is recorded, including the
ones that do not bite** — a mutation that fails to bite is evidence about the
code's structure, not automatically a test gap.

| # | Mutation | Expected | Outcome |
|---|---|---|---|
| M1 | `seat` computes the clearing and discards it | FR-1.1-S1, S7, FR-1.2-S1..S3 red | **bit wider than predicted — 7 of 18.** Red: FR-1.1-S1, S3, S7, S6's positive control (`a_resumed_player_near_the_worlds_edge_is_moved_inward_rather_than_over_it`), FR-1.2-S1, S2, S3 — the whole do-nothing set. **It also turned the one baseline red green**, and that is the row's real value: `reload_leaves_the_player_alone::a_candidate_that_would_have_trapped_the_player_and_was_refused_moves_them_nowhere` **passed** under M1. That is what established the fixture was failing because entry clearing moves the player it seats, and not because of anything else in the tuple it asserts — the same instrument run backwards. The `test-correct` verdict rested on this rather than on the position alone. |
| M2 | `seat` passes a whole-coordinate-space extent | FR-1.1-S6 red | **bit wider than predicted — 2 of 18.** FR-1.1-S6 and *also* its positive control, `a_resumed_player_near_the_worlds_edge_is_moved_inward_rather_than_over_it`. The prediction of "S6 only" reads the extent argument as one-directional: an over-large extent does not merely let a player be moved who should not be, it changes **where** a near-edge player who *should* move lands. Both directions of the argument are live at once. |
| M3 | `seat` passes an extent one cell smaller on each axis | FR-1.1-S1 red | **bit wider than predicted — 2 of 18.** FR-1.1-S1 and FR-1.1-S7. They share a fixture position: the shrunken extent rejects S1's one-cell-sideways destination, and the snapshot S7 reads is that same player, so one defect surfaces twice. |
| M4 | `clear_the_player` called after the first snapshot is published | FR-1.1-S7 red | **bit far wider than predicted — 8 of 18**, and the prediction could not have been right. Red: FR-1.1-S1, S3, S7, S6's control, FR-1.2-S1..S3, **plus `one_way_seats_a_player`**. `Simulation` exposes the player through `latest()` alone, so every entry scenario reads the same accessor and a defect upstream of the first publish surfaces in all of them together — "S7 only" is unreachable by construction. The scan bite is the natural spelling of this mutation needing a second `self.player =`, which is exactly the shape that needle is documented to catch. |
| M5 | `clear_the_player` sets position but not velocity | **no bite expected** — entering velocity is already `Vec3::ZERO` (`persistence.rs:196`), so zeroing it is inherited rather than newly observable | **did not bite at entry, as predicted — and bit once elsewhere. 1 of 1209.** Measured over the **full workspace suite** rather than the 18, because the function it mutates serves the reload caller too. No entry test moved. The one new red was `reload_clears_a_trapped_player::a_reload_that_moves_a_rising_player_upward_takes_their_climb_away`. **That is the finding: the velocity rule is witnessed through the reload caller and nowhere else.** Entry cannot see it because the entering velocity is already `Vec3::ZERO`, so a future change that drops the reload path's clearing takes away this rule's only witness and no entry test will report it. |
| M6 | a second `pub fn` in `simulation.rs` building the struct by literal | `one_way_seats_a_player` red — `published:` 2 → 3, `times` 1 → 2 | **bit exactly as predicted — 1 of 4** (measured over `one_way_seats_a_player` alone). Verdict `AnotherSourceSeatsAPlayer([Site { file: "src/simulation.rs", names: "published:", times: 3 }])`. The other three tests stayed green, so the bite was not bought by breaking the controls. |
| M7 | the door's return type renamed | `one_way_seats_a_player` red as `TheDoorNoLongerSeatsAPlayer` | **bit only after the needle was tightened.** `Seated` → `Admitted` bit. `Seated` → `SeatedPlayer` did **not**: the needle was `) -> Seated`, and `) -> SeatedPlayer {` contains it. With the needle now `) -> Seated {` the same rename reddens as `TheDoorNoLongerSeatsAPlayer([") -> Seated {"])`. See below. |

**The instrument, and the baseline every count above is against.** M1–M4 were run over
the five phase-1 binaries plus `reload_leaves_the_player_alone` — 18 tests; M5 over the
full workspace suite — 1209; M6 over `one_way_seats_a_player` alone — 4.
**All of them were measured before `382de2b`**, so
the baseline carried exactly one red,
`a_candidate_that_would_have_trapped_the_player_and_was_refused_moves_them_nowhere`,
which is why M1's row can report it going green. Counts above are reds *new* to the
mutation except where the baseline red is named explicitly.

**Four of the six bit wider than the plan predicted, and they share one cause.** Every
FR-1.1 and FR-1.2 scenario reads the player through `Simulation::latest()` — there is
no other accessor — so any defect upstream of the first published snapshot reddens all
of them at once. A prediction written per-scenario against one shared accessor will
keep coming out narrow, and M4's "FR-1.1-S7 only" is that error in its purest form.
Read the wider bites as evidence about the shape of the observable, not as tests
overreaching.

M6 and M7 grade whether the FR-1.3 scan is doing anything at all. If either fails
to bite, the scan is the defect and not the code.

---

## Phase 2 — the two sentences

**6 scenarios, 6 tests, plus 6 additional-coverage tests.**

```
cargo nextest run -p mc-client -E 'binary(/^the_entry_sentence_is_said_once$/) + test(/notice/)'
```

No blind window — nothing existing changes signature.

### The mapping

| Scenario | File | Test |
|---|---|---|
| FR-2.1-S1 | `crates/mc-client/src/notice_test.rs` | `a_player_moved_at_entry_is_told_they_would_have_entered_inside_solid_blocks_and_where_they_were_put` |
| FR-2.1-S2 | `crates/mc-client/src/notice_test.rs` | `a_player_who_needed_no_moving_at_entry_is_told_nothing_about_where_they_were_placed` |
| FR-2.1-S3 | `crates/mc-client/src/notice_test.rs` | `a_player_with_nothing_clear_within_reach_is_told_they_were_left_inside_the_solid_blocks` |
| FR-2.1-S4 | `crates/mc-client/src/notice_test.rs` | `a_player_moved_by_a_reload_is_told_the_reload_made_their_cell_solid_and_never_the_entry_sentence` |
| FR-2.1-S5 | `crates/mc-client/tests/the_entry_sentence_is_said_once.rs` | `the_client_composes_the_entry_sentence_once_and_says_it_where_the_launch_is_collected` |
| FR-2.1-S6 | `crates/mc-client/tests/the_entry_sentence_is_said_once.rs` | `a_second_source_that_composes_or_says_the_entry_sentence_is_named_by_the_verdict` |

**6 scenarios, 6 tests, each scenario exactly once.**

### Additional coverage — what each one catches

| File | Test | What it catches |
|---|---|---|
| `crates/mc-client/tests/the_entry_sentence_is_said_once.rs` | `a_composition_nothing_asks_for_is_named_by_the_verdict` | **The `ComposedButNeverSaid` control.** This is FR-2.1-S5's actual failure mode — the *policy-is-not-wiring* case — and without a control feeding it, a scan that stopped being able to find the call site reads as clean. It is also the phase's opening RED. |
| `crates/mc-client/tests/the_entry_sentence_is_said_once.rs` | `a_tree_with_no_production_source_is_named_by_the_verdict` | **A scan that can no longer look**, feeding `NoSourceWasRead`. Same hole as phase 1's, in the other crate. |
| `crates/mc-client/src/notice_test.rs` | `both_refusals_name_the_reach_the_verdict_carries_rather_than_a_literal_eight` | **A needle that cannot match, from the behavioural side.** The refusal's `8` is `NoClearSpaceWithin { blocks }` interpolated (`crates/mc-sim/src/world/clearing.rs:53`), so a `blocks` of anything other than 8 must appear in the sentence. A hardcoded `8` passes FR-2.1-S3 and fails this. **Renamed and widened** from `the_reload_refusal_names_…`: both refusals carry the same field and the entry one is the new code, so asking only the reload's would leave the new refusal's interpolation ungraded. Asked with a reach of 3. |
| `crates/mc-client/src/notice_test.rs` | `an_entry_sentence_renders_a_whole_number_coordinate_without_a_trailing_zero` | **The formatting trap from the other side.** FR-2.1-S1's expected text depends on `Display` on `f32` rendering `10.0` as `10`; adding `{:.1}` reddens S1 with no other signal about *why*. This says why. |
| `crates/mc-client/src/notice_test.rs` | `no_clearing_verdict_is_told_in_the_same_words_at_entry_and_at_reload` | **The two sentence pairs being unified**, in the observable half. FR-2.1-S4 catches `reloading` returning the entry wording; this holds that no verdict is told in the same words at the two moments. **Renamed, because the name it was given asserts something false**: after their opening clauses the two *move* sentences are character-identical (`, so you were moved to (x, y, z)`), so "share no wording after their opening clause" would redden a correct implementation — `testing.md` §2's over-tight assertion, whose cheapest green is editing the sentences. `Unneeded` is outside the claim: both moments compose `None`, and that is agreement rather than unification. |
| `crates/mc-client/tests/the_entry_sentence_is_said_once.rs` | `a_tree_that_states_every_rule_is_named_by_the_good_verdict` | **A rule set nothing can satisfy**, and the only test that exercises the good verdict before `notice.rs` exists. **Repurposed from `every_needle_the_scan_names_carries_an_expectation`**, which the design made redundant: the offending fixture in the FR-2.1-S6 control is generated *from* `NEEDLES` and its expectation derived from `NEEDLES`, so a sixth needle is watched by construction and a separate test asserting it would be a tautology. What is **not** free, because this scan's rules are heterogeneous where phase 1's are uniform, is that the rules can all be met at once — a needle whose home contradicts another's would leave the implementer fighting the instrument, and the fight would look exactly like a defect in their own code. Measured non-vacuous: emitting one extra copy of every spelling reddens it as `AnotherSourceComposesOrSaysIt`. |

### Why the write needed a scan and not a subprocess

`crates/mc-client/tests/shipped_binary.rs` runs the real binary and cannot reach
this: it works only because a missing content root refuses *before* a device is
opened, and a successful launch opens one. And a unit test of `entering` alone is
agreement between two copies of one decision — `collect_preparation` can stop
calling it entirely with every such test green. The scan is what makes deleting
the call visible, which is mutation M8's whole job.

### Which of the twelve are red at commit D, and what reddens the rest

**12 tests: 1 red at commit D, 4 green, 7 that cannot run yet.** Telling those
three groups apart is what says the suite is doing something.

| Test | Commit D | Reddened by |
|---|---|---|
| FR-2.1-S5 | **red as `ComposedButNeverSaid`**, naming all five unstated spellings | `notice.rs` landing and `collect_preparation` calling it |
| The four scan controls | green | nothing in this phase — a positive control over a `tempfile` fixture cannot be red on the tree it controls. Each was separately measured non-vacuous; see below |
| FR-2.1-S1, S2, S3, S4 and the three `notice_test.rs` extras | **cannot run** — the sibling is an orphan until `notice.rs` declares `#[cfg(test)] #[path = "notice_test.rs"] mod tests;` | T09's two deliberately wrong bodies |

**The seven were run anyway, and none of them is a test that has never
compiled.** The test author wired a temporary `notice.rs` and one `pub mod`
line, ran both wrong bodies, then reverted by hand and confirmed
`git diff --exit-code` clean; nothing but the two test files is in commit D.
Measured, not predicted:

| Skeleton | Result |
|---|---|
| `entering` and `reloading` both always `None` | **6 of 7 red on assertions** — S1, S3, S4 and all three extras. S2 passes **vacuously**: it expects `None`, and a skeleton that composes nothing satisfies it for the wrong reason |
| Both always `Some(<a wrong sentence>)` | **S2 red.** This is the phase-1 lesson applied: one skeleton could not falsify this phase's scenarios, and which one is needed depends on the scenario |

**A lint was found in a test file during that window and fixed by its author**,
which is the only reason it was found at all: an orphan sibling is invisible to
`cargo fmt`, to `cargo clippy` and to the gate, so between commit D and T09 there
is a window in which nothing can see `notice_test.rs`. `manual_contains` on the
file-set rule would otherwise have surfaced at T09 inside a file the implementer
may not edit.

### What re-derivation corrected, and what the controls actually measure

**Three corrections, all measured on the commit-D tree at `9a3f0eb`.**

1. **`tasks.md`'s Correction 2 is out of date and its figures are superseded.**
   It states `Clearing` at 8 raw lines across 3 files, 7 visible across 2 — true
   at `f260c64`, before phase 1 put `pub clearing: Clearing` on `PreparedLaunch`
   (`crates/mc-client/src/launch.rs:98`) with a `///` mention at `:75`. Today:
   **11 raw across 4 files, 9 visible across 3** (`app/reload.rs`, `launch.rs`,
   `session/reload.rs`). The rule the correction states is unchanged; only its
   arithmetic moved, which is the argument for re-deriving rather than inheriting.
2. **`fn say_entering` was a prefix test and is now `fn say_entering(`.** Phase 1
   measured this class of hole with `) -> Seated` matching `) -> SeatedPlayer {`.
   Renaming the function to `say_entering_now` leaves the bare spelling matching;
   the parenthesis rejects it. 0 either way today, so nothing is asked of the
   implementation by the change.
3. **A third standing counter-example to "a bare `rg` is the wrong instrument"
   now exists, and it is this phase's own test file.** `rg -l "Clearing"` reports
   **five** files where the scan sees three: the two it drops are `///` prose and
   `src/notice_test.rs`, which names all four sentences and is not a place any of
   them is said. The sibling skip is load-bearing rather than tidy — dropping the
   `_test.rs` term from the file filter reddens the FR-2.1-S6 control and **only**
   it, while the real-tree scenario test stays red for its own reason and reports
   nothing about it.

**The `ComposedButNeverSaid` control was deliberately made stronger than
`tasks.md` prescribes.** The task asks for a fixture holding `src/notice.rs`
alone; that tree is missing four spellings at once, so it cannot say whether
deleting *the call* is what reaches this verdict — which is the only thing M8
needs to know. The fixture used instead states every rule and deletes the one
call line, so the verdict is attributable to one line. The weaker tree is not
left ungraded: today's real client is a superset of it, and FR-2.1-S5's own test
reads it.

**The scan's reading order is inverted from phase 1's, and deliberately.**
`one_way_seats_a_player.rs` reports a second source before a missing door; here
what is *unstated* outranks what is said in the wrong place, because on the
commit-D tree `Clearing` is named in `src/app/reload.rs` — a file the
implementation deletes — and an extras-first reading would open the phase with a
diagnosis about a file that is about to disappear. The masking that buys is real
and recorded in the test file's header: while anything is unstated, a second
source saying it is invisible.

**What each control was measured to catch, by hand:**

| Control | Falsifier run | Outcome |
|---|---|---|
| FR-2.1-S6 (`a_second_source…`) | drop `_test.rs` from the file filter | **red, and only it** — the sibling fixture's sites appear |
| `a_tree_that_states_every_rule…` | emit one extra copy of every spelling | **red as `AnotherSourceComposesOrSaysIt`**, naming the three `notice.rs` spellings and the call. `Clearing` correctly absent: a file-set rule sets no ceiling, which is why `notice.rs` naming the type once per match arm never churns |
| `a_composition_nothing_asks_for…` | is itself M8's shape | **`ComposedButNeverSaid`** — so M8's expected bite is measured before T10 runs it |

### Mutations

Every row below was run against `d6ce7d6` with
`cargo nextest run -p mc-client --no-fail-fast`, so each count is the whole
suite and not the prefix a fail-fast run stopped at. Reverted by hand each time,
`git diff --exit-code` clean between them; no proptest seeds were written.

| # | Mutation | Expected | Outcome |
|---|---|---|---|
| M8 | delete `notice::say_entering(prepared.clearing);` | `the_entry_sentence_is_said_once` red as `ComposedButNeverSaid` | **bit exactly as predicted, and alone — 1 of 275.** `left: ComposedButNeverSaid`. FR-2.1-S5 is proven: nothing else in the workspace reddens when the client stops saying what entry did, which is the whole reason this scan exists. |
| M9 | call `say_entering` a second time in the frame path | red as `AnotherSourceComposesOrSaysIt` | **bit as predicted — 1 of 275.** The second call went into `take_up_reloaded_content`, where a `Clearing` is already in scope and which `present` calls on every frame (`app/mod.rs:274`), so this is a genuine frame-path repetition rather than a contrived one. `AnotherSourceComposesOrSaysIt([Site { file: "src/app/reload.rs", names: "notice::say_entering(", times: 1 }])` — the site is named, not merely counted. |
| M10 | `entering` returns the reload wording | FR-2.1-S1 and S4 red together | **bit wider than predicted, and S4 was not among them — 5 of 275.** Red: FR-2.1-S1, FR-2.1-S3, and all three of `both_refusals_name_the_reach…`, `an_entry_sentence_renders_a_whole_number…`, `no_clearing_verdict_is_told_in_the_same_words…`. **FR-2.1-S4's own test stayed green, correctly**: `reloading` was untouched and still answers the reload wording, so S4 is blind to this direction by construction. What sees it is `no_clearing_verdict…`, which is the additional-coverage test written for exactly this. The prediction was wrong about S4 and the coverage was there anyway. |
| M11 | add `{:.1}` to the `y` coordinate | FR-2.1-S1 red | **bit as predicted, with its own diagnosis attached — 2 of 275.** FR-2.1-S1 plus `an_entry_sentence_renders_a_whole_number_coordinate_without_a_trailing_zero`. The second is what makes the first readable: S1's failure message is about the wording, and a reader seeing only it looks for a reworded sentence. The additional test names the number instead, which is the difference between a five-minute diagnosis and an afternoon. |
| M12 | make `entering` and `reloading` one function | FR-2.1-S4 red | **bit as predicted, and narrower than M10 — 3 of 275.** `reloading` delegating to `entering`: FR-2.1-S4, `both_refusals_name_the_reach…`, `no_clearing_verdict…`. The asymmetry with M10 is the finding: **each direction of the unification is seen by a different scenario test, and only `no_clearing_verdict…` sees both.** Neither scenario test alone would have caught a unification performed in the direction it cannot observe. |
| M13 | inline the two consts at their use sites | **no behavioural bite expected** — the composed text is identical. What this measures is whether D6's needles survive rustfmt wrapping the literals into a `\` continuation. A miss means the needles rest on formatting luck rather than on the const idiom, and that is a finding. | **bit — 1 of 275 — but not for the predicted reason, and the predicted hazard is still open.** Inlining both entry clauses and running `cargo fmt --all` left both literals **unwrapped**, on lines of 115 and 148 characters: no `rustfmt.toml` exists, `format_strings` is off, and rustfmt never reflows a string literal. **So the needles do not rest on formatting luck.** What reddened instead is duplication: `entered the world inside solid blocks` then appears **twice** in `notice.rs`, once per entry sentence, and `OnlyIn { times: 1 }` reports `AnotherSourceComposesOrSaysIt([Site { file: "src/notice.rs", names: "entered the world inside solid blocks", times: 2 }])`. The const's real work is keeping the clause single, not keeping it unwrapped. |
| M13b | from M13's state, hand-wrap the inlined refusal clause across a `\` continuation | none — run to answer the question M13 was posed to answer and did not | **did not bite: 275 of 275 green.** One inlined clause unwrapped, one inlined and hand-wrapped so it matches nothing: the count falls back to 1, and the scan calls the client well wired while the const idiom has been abandoned in `entering` altogether. **The hazard the test file's header records is real, and rustfmt cannot reach it — a human writing the continuation can.** Against that the scan is blind, and only review sees it. Recorded rather than papered over; it belongs beside the file's existing residual-hole paragraph. |

---

## The counts every scan asserts, and where they came from

**No expected count in either scan is transcribed from a green run, and none is
copied from `architecture.md`.** Each was re-derived from the tree on 2026-08-18
at `f260c64`; the derivation tables and the commands that produced them are in
`tasks.md` under "Counts: the rule that binds every scan in this spec", together
with the two corrections that re-derivation turned up.

The one thing a reader of the test files must know, and which is written into
both of them: **a bare `rg` is the wrong instrument.** `production_text`
(`crates/mc-client/tests/reporting_seam.rs:307-316`) drops every `///` and `//!`
line before any `contains` runs, so the scan's count and the shell's count
differ. Two standing counter-examples in this tree —
`crates/mc-sim/src/reload/mod.rs:91` matches `published:` and
`crates/mc-client/src/session/mod.rs:295` matches `Clearing`, both inside `///`
prose — and each is one `///` → `//` edit away from becoming a site in a scan
that is looking at exactly the right thing.

---

## Where this file's measured findings went, and why that mattered

This folder is archived to `specs/archive/2026/` and pruned at 365 days, so
everything above that a future reader needs had to reach `docs/`. Checked at the
close of phase 3:

| Finding here | Home in `docs/` |
|---|---|
| Both mutation tables, with their instruments, baselines and every non-bite | `technical/testing.md`, "The entry door's mutation tables, including the ones that did not bite" |
| The two phase-1 scan exposures — `published:` counting prose, `) -> Seated` as a prefix test | `technical/testing.md`, "A bare `rg` is the wrong instrument…" and "A needle may not be a whole sentence…" |
| The three phase-2 exposures — `fn say_entering` as a prefix test, the file-set rule's absent ceiling, the inverted reading order and its masking | same two sections |
| Why the fixtures are real saves, and why the exact-destination and exact-spawn assertions must not be loosened | `technical/testing.md`, "The entry door: what the fixtures supply…" |
| The compose/write split and the device that made the reload's sentences unassertable | same section |
| Both scans' verdicts and their controls, and why one needs a fourth | `technical/testing.md`, "Two scans, four verdicts each…" |
| The entry pair becoming producible, and why the reload pair stays in prose | `technical/testing.md`, "A documentation guard needs its subject to be an artefact…" |

**Three holes are recorded in `docs/` as open rather than closed**, and none of
them was resolved by the documentation pass:

- The hand-wrapped `\` continuation that leaves 275/275 green with the const
  idiom abandoned. rustfmt cannot reach it; only review sees it.
- The `published:` prose exposure — one `///` → `//` edit reddens the seating scan
  with no door added. Open and accepted, because the diagnosis is a minute's work.
- The velocity rule's single witness through the reload caller. Dropping that
  path's clearing takes the rule's only witness with it and no entry test reports it.

**A fourth belongs to a future spec rather than to `docs/`**: a join can still
supply the wrong ground, and nothing here makes that visible. It is recorded in
`technical/architecture.md` under the seating door, with the sentence that whichever
spec adds the join owes it a scenario.
