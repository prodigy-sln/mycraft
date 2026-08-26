# Test map — PRO-971

**This file is over the artifact-lint budget and deliberately so.** The mapping
itself is one line per scenario, as the budget intends. What overflows it is the
evidence `testing.md` §2 requires to be *written down*: the fixture derivations
and the correction they force on the spec's tolerance argument, the RED reading
with its invocation, and the mutation results — including the one that did not
bite, which is the measurement of this fix's residual hole. Deleting any of it to
reach a line count would delete the only record that these tests were checked
rather than merely written.

## Scenario → test

Paths relative to `crates/mc-client/tests/` unless stated.

- FR-1.1-S1 → `frame_paced_movement.rs::the_same_elapsed_time_walks_the_same_distance_at_144_frames_a_second_and_at_60`
- FR-1.1-S2 → `frame_paced_movement.rs::the_same_elapsed_time_walks_the_same_distance_at_1000_frames_a_second_and_at_60`
- FR-1.1-S3 → `frame_paced_movement.rs::a_frame_delivering_three_tick_quanta_advances_the_player_by_three_ticks_walk`
- FR-1.1-S4 → `frame_paced_movement.rs::a_frame_delivering_less_than_one_tick_quantum_leaves_the_player_where_it_was`
- FR-1.1-S5 → `frame_paced_movement.rs::two_frames_of_half_a_quantum_advance_the_player_by_one_ticks_walk`
- FR-2.1-S1 → `frame_spends_each_input_once.rs::a_frame_that_spends_three_ticks_turns_the_view_by_one_pointer_motion_once`
- FR-2.1-S2 → `frame_spends_each_input_once.rs::a_frame_that_spends_three_ticks_carrying_one_break_request_breaks_one_voxel`
- FR-2.1-S3 → `frame_spends_each_input_once.rs::a_held_walk_key_applies_to_every_tick_a_frame_spends`
- FR-2.2-S1 → `reload_survives_a_multi_tick_frame.rs::a_candidate_a_multi_tick_frame_accepts_reaches_the_frame_path`
- FR-2.2-S2 → `reload_survives_a_multi_tick_frame.rs::a_candidate_a_multi_tick_frame_refuses_reaches_the_frame_path`
- FR-3.1-S1 → `frame_gap_is_bounded.rs::a_frame_reporting_ten_seconds_advances_no_further_than_the_catch_up_bound`
- FR-3.1-S2 → `frame_gap_is_bounded.rs::the_surplus_a_bounded_frame_discarded_does_not_reach_the_frames_after_it`
- FR-3.1-S3 → `frame_gap_is_bounded.rs::frames_a_tenth_of_a_second_apart_walk_the_same_distance_as_the_same_time_at_60`
- FR-4.1-S1 → `frame_path_pacing.rs::no_source_under_the_clients_frame_path_names_a_per_tick_advance`
- FR-4.1-S2 → `frame_path_pacing.rs::the_same_scan_reports_a_frame_path_that_advances_a_tick_and_passes_the_core_over`
- FR-4.1-S3 → `frame_paced_movement.rs::the_rate_the_overlay_reports_and_the_ticks_the_world_spent_come_from_one_reading`
- FR-5.1-S1 → `frame_paced_movement.rs::the_same_sequence_of_frame_times_leaves_the_player_in_the_same_place_twice`
- FR-5.1-S2 → `wall_clock_confinement.rs::no_production_source_of_the_client_or_the_renderer_reads_a_wall_clock_outside_the_ports_own` (existing; `EXEMPT_FILE` repathed with the port)

## Additional coverage

- `crates/mc-sim/src/player/physics_test.rs::the_quantum_a_frame_path_spends_is_the_same_length_as_the_tick_it_buys` → the two spellings of the quantum drifting apart. The physics multiplies by `TICK_DURATION` seconds and the frame path subtracts `TICK_QUANTUM` from an interval; a client spending quanta of one length into ticks of another runs slow or fast by the ratio, with every scenario above still green because both runs share the same wrong pair.
- `frame_path_pacing.rs::a_pacing_scan_that_read_no_production_source_refuses_rather_than_reporting_no_occurrences` → the scan's refusal arm becoming unreachable. FR-4.1-S1's exact-verdict assertion rejects "I could not look" only while that verdict can still be produced.

`wall_clock_confinement.rs`'s own two controls are unchanged and unlisted: they
were already the pattern FR-4.1-S2 follows.

No unit test of `session/pacing.rs` was written. Its arithmetic reaches the
world through exactly one caller, and a test calling `spend` directly would be a
second route to the same function rather than a second witness — `testing.md`
§1's "re-proves what another test already proves through the same code path".
The thirteen behavioural scenarios above drive it through the door the product
uses; the other three are scans.

## Fixture derivations

**The stretch.** `THE_STRETCH` is 500 004 000 ns, and every rate in
`frame_paced_movement.rs` and `frame_gap_is_bounded.rs` partitions exactly that
integer: 500 frames at a simulated 1000 a second, 72 at 144, 30 at 60, 5 a tenth
of a second apart. That is what makes the equalities exact rather than
approximate, and it is why the nominal rates are off by a hundredth of a percent
— no integer nanosecond count is divisible by an exact 144th *and* an exact 60th
of a second, because a quantum is not divisible by twelve.

**A correction to the spec's tolerance derivation, and it replaces a
constraint.** § "The tolerance, derived from both directions" reasoned that two
partitions of one total spend tick counts "differing only by nanosecond rounding
across the partition boundaries — at most ~1 µs of simulated time", and asked the
fixture to choose a total clear of a quantum boundary for that reason. The
residue is not ~1 µs; with a `Duration` accumulator it is **exactly zero**,
because the carried remainder is whole nanoseconds and nothing is lost at a
boundary. The ticks a stretch buys are therefore `floor(total / quantum)`
*however it is cut up*, so two runs delivering the same integer nanosecond count
cannot straddle a boundary differently and the margin cannot bind. The
conclusion — exact equality is the right assertion — stands, for a stronger
reason than the one given.

The margin is recorded anyway so a reader can check rather than take it:
`THE_STRETCH` sits **3 990 ns above the 30-quantum boundary**, 0.02 % of a
quantum. An implementation accumulating `f64` seconds would err by ~10⁻⁷ ns at
this magnitude, ten orders of magnitude inside that margin, so the fixture is not
tuned to the implementation it happens to have.

**Why the drive is half a second.** The harness floor is one chunk column, 16
blocks across, spawn centred at x 8.5 facing +x. Half a second of walking is 2.25
blocks (5.25 to the edge), and the *broken* client — 2.4× the ticks over the same
stretch — walks 5.4 and still lands inside the world. A red produced by two runs
piling into the same edge would be evidence of nothing; the red recorded below
reads as 2.4×.

**Half a quantum is rounded up.** 16 666 667 ns has no exact half. Two frames of
the rounded-*down* half come to one nanosecond short of a quantum and correctly
buy no tick, which is FR-1.1-S4's claim rather than FR-1.1-S5's.

**The catch-up bound is stated twice, on purpose.** `frame_gap_is_bounded.rs`
carries `AT_THE_BOUND = 15` and derives its oracle from fifteen whole-quantum
frames; the implementation carries `CATCH_UP_TICKS` separately. A test that
imported the constant would agree with the implementation by construction and
would go green the day the bound became one tick.

## RED evidence

Run before the fix, over the adaptation commit — a frame door that took an
elapsed reading and spent exactly one tick regardless, which is the shipped
defect expressed through the new interface. Invocation:

```
cargo nextest run -p mc-client --no-fail-fast --test frame_paced_movement \
  --test frame_spends_each_input_once --test frame_gap_is_bounded --test frame_path_pacing
```

`16 tests run: 6 passed, 10 failed, 0 skipped` — a complete run, not a cancelled
one. Every one of the ten is an **assertion** failure reproducing the defect, not
a compile error and not a missing function:

- FR-1.1-S1 — 72 frames at 144 Hz and 30 at 60 Hz over the same 500.004 ms left
  the player in different places: the fast run spent 72 ticks where the slow one
  spent 30. That ratio is 2.4, which is the player's report in numbers.
- FR-1.1-S3 — one frame of three quanta left the player at x 8.575; three frames
  of one quantum left them at x 8.725. One tick paid instead of three.
- FR-1.1-S4 — a frame one nanosecond short of a quantum moved the player anyway.
- FR-3.1-S1 — a ten-second frame advanced one tick; fifteen whole-quantum frames
  advance fifteen.
- FR-3.1-S2 — after the gap, x 8.65 against the 9.699997 sixteen ticks reach.
- FR-4.1-S3 — the overlay reported 143.9988 frames a second (right) while the
  simulation had spent **72** ticks over 500.004 ms, where 60 a second is 30.
  Both readings individually plausible; only the pair says which is wrong.

The six that passed are the three scan tests in `frame_path_pacing.rs`, FR-5.1-S1
and two of the three FR-2 scenarios. **FR-2 is a hold rather than a repair** and
the spec says so (§ Sibling sweep): the drains that make a multi-tick frame spend
each input once were already written that way, and a frame that is one tick
cannot exercise them. They were vacuous under the skeleton and are meaningful
after the fix. FR-2.1-S3 is the exception and did redden, because "the walk
applies to every tick" is the same claim FR-1.1-S3 makes.

**FR-4.1-S1 was green on arrival** and could not have been otherwise: the
adaptation commit had already made the per-tick step private, so no source under
`crates/mc-client/src` could name it. Its non-vacuity rests on FR-4.1-S2's
positive control, on the `ReadNoProductionSource` refusal, and on the mutation
recorded below.

## Mutation evidence

Recorded per `testing.md` §2, including the mutation that did **not** bite. Each
was applied by hand, observed, and reverted by hand — never with
`git checkout --` — and `git diff --exit-code` afterwards reported only
`metrics.md`, which the resolver wrote and which is not mine.

The tree is shared with other agents, so the window is stated: **12:04:43 to
12:07:54 local on 2026-08-26**. A failing-test count seen by anybody else inside
it belongs to a mutation and not to a flake.

Invocation for M1, M2 and M4 (a complete run — bare `N tests run`, not `N/M`):

```
cargo nextest run -p mc-client --no-fail-fast --test frame_paced_movement \
  --test frame_spends_each_input_once --test frame_gap_is_bounded --test frame_path_pacing
```

M3's claim is an absence, so it was run over the whole crate instead:
`cargo nextest run -p mc-client --no-fail-fast`.

| # | Mutation | Result |
|---|---|---|
| M1 | `session/pacing.rs`: drop the clamp — `took.min(CATCH_UP_TICKS * TICK_QUANTUM)` → `took` | `16 tests run: 14 passed, 2 failed`. FR-3.1-S1 and FR-3.1-S2 reddened; nothing else moved. |
| M2 | `session/pacing.rs`: discard the carry — `self.unspent -= ticks * TICK_QUANTUM` → `self.unspent = Duration::ZERO` | `16 tests run: 12 passed, 4 failed`. FR-1.1-S1, FR-1.1-S2, FR-1.1-S5 and FR-4.1-S3 reddened. FR-1.1-S5 is the direct one; the other three catch it as accumulated drift. |
| M3 | `app/mod.rs`: delete `session.advance_frame(&self.frame_clock)` — the client's only call, i.e. the whole wiring | **`383 tests run: 383 passed`. Nothing reddened.** |
| M4 | Make `Session::tick` `pub` and add `session.tick()` back to `App::present` | `10 tests run: 9 passed, 1 failed`. FR-4.1-S1 reddened, which is the only instrument that can see it. |

**M3 is the important one and it is a measurement of the residual hole rather
than a surprise.** The spec predicted it (§ Detection gap): `App` needs a real
window and a real `wgpu::Surface`, so no in-process test constructs one, and the
one subprocess harness that executes the frame path cannot observe pacing. The
whole client suite passes with the frame path advancing nothing at all. What
stands in that gap is the compile-time privacy of `Session::tick` and the
structural scan M4 exercises — both of which read the source rather than run it.
`testing.md` §2's *policy is not wiring*, measured on this diff, and recorded
rather than dressed up.

M4 is what keeps FR-4.1-S1 from being a test that could not fail: the scan was
green on arrival because the adaptation commit had already removed the offence,
and M4 is the demonstration that it reddens on the real tree when the offence
comes back.

## RED evidence — FR-2.2, added during validate

Run over the tree at `fbf9c0d`, before the fix. A complete run — bare
`N tests run`, not `N/M`:

```
cargo nextest run -p mc-client --no-fail-fast --test reload_survives_a_multi_tick_frame
```

`2 tests run: 0 passed, 2 failed`. Both are **assertion** failures, and both read
`left: NobodyTheAnswerWasOverwritten` against `right: TheMultiTickFrame` — the
verdict naming the defect rather than a missing function or a fixture that could
not look.

**The verdict is enumerated for a reason that is specific to this fixture.** A
candidate is built on a thread, and nothing can ask whether that build has
finished without collecting it — the ask *is* the collect. So the run waits the
bound an attempt may not outlast, takes its reading with one three-tick frame,
then keeps crossing ordinary boundaries. `ALaterBoundarySoTheBuildWasStillRunning`
is its own arm so that a machine too slow for the wait produces a *distinguishable*
red rather than being recorded as the defect, and `SomethingElse` catches a
fixture that stopped provoking what it says it provokes. An
`assert!(report.is_some())` would have conflated all four.

Three ticks rather than two: a defect dropping only the last tick's answer would
survive a two-tick frame, and three is unambiguously the multi-tick regime.

## Mutation evidence — the whole wiring, measured during validate

Recorded per `testing.md` §2. Applied by hand, observed, reverted by hand;
`git diff --exit-code -- crates/` clean afterwards. Window **13:03 local on
2026-08-26**, announced before the tree was touched.

| # | Mutation | Result |
|---|---|---|
| M5 | `tests/support/reload_watch/runs.rs:120`: make every reload boundary a frame of two quanta — `client.tick()` → `client.frame(TICK_QUANTUM * 2)` | `11 tests run: 0 passed, 11 failed` across five reload binaries, `left: [] / right: [TakenUp]`. Reverted: `11 tests run: 11 passed`. |

**M5 is what turned the Blocker from an argument into a reading**, and it is also
the measurement of how narrow the multi-tick coverage was: eleven existing
scenarios all pass today only because `InputHarness::tick()` means a frame of
exactly one quantum. It is not a mutation of the product and no product line was
touched — what it perturbs is the *regime* the suite runs in, which is the thing
the harness's name hides.
