# Test map — PRO-962

**This file is over the artifact-lint budget and deliberately so.** The mapping
itself is one line per scenario, as the budget intends. What overflows it is the
evidence `testing.md` §2 requires to be *written down*: the fixture's provenance,
the tolerance derived from both directions, the RED reading with its invocation,
and the deletion check the spec's § "Where the decision lives" says the implement
phase owes — including whichever half of it did not bite. Deleting any of it to
reach a line count would delete the only record that these tests were checked
rather than merely written.

## Scenario → test

Paths relative to `crates/mc-client/tests/`.

- FR-1.1-S1 → `a_relative_pointer_stream_is_untouched.rs::two_samples_turn_the_camera_as_far_as_one_sample_of_their_sum`
- FR-1.1-S2 → `a_relative_pointer_stream_is_untouched.rs::a_sample_with_a_negative_component_turns_the_camera_the_way_its_sign_names`
- FR-2.1-S1 → `an_absolute_pointer_stream_is_spent_as_travel.rs::the_first_screen_position_of_a_session_turns_the_camera_by_nothing`
- FR-2.1-S2 → `an_absolute_pointer_stream_is_spent_as_travel.rs::a_second_screen_position_turns_the_camera_by_the_travel_between_them`
- FR-2.1-S3 → `an_absolute_pointer_stream_is_spent_as_travel.rs::each_further_screen_position_turns_the_camera_by_the_travel_since_the_last`
- FR-2.1-S4 → `an_absolute_pointer_stream_is_spent_as_travel.rs::the_recorded_remote_desktop_session_turns_the_camera_by_the_travel_it_recorded`
- FR-3.1-S1 → `an_absolute_pointer_stream_is_spent_as_travel.rs::the_same_travel_in_either_axis_turns_the_camera_by_the_same_angle`
- FR-4.1-S1 → `the_pointer_regime_changes_under_a_playing_client.rs::two_ordinary_deltas_hand_the_stream_back_to_the_relative_reading`
- FR-4.1-S2 → `the_pointer_regime_changes_under_a_playing_client.rs::one_ordinary_delta_between_screen_positions_is_spent_on_nothing`
- FR-5.1-S1 → `the_pointer_regime_changes_under_a_playing_client.rs::screen_positions_arriving_while_the_cursor_is_free_leave_the_camera_alone`
- FR-5.1-S2 → `the_pointer_regime_changes_under_a_playing_client.rs::taking_the_pointer_back_turns_only_by_the_travel_since_it_came_back`
- FR-5.1-S3 → `the_pointer_regime_changes_under_a_playing_client.rs::losing_the_window_turns_only_by_the_travel_since_it_came_back`

## Additional coverage

- `a_relative_pointer_stream_is_untouched.rs::a_lone_screen_position_does_not_stop_the_delta_after_it_being_spent`
  → the state machine's *Relative, pending / not position-shaped* row, which no
  scenario reaches. One freak position-shaped packet followed by an ordinary
  delta must leave the client relative and spend the delta whole; a client that
  flipped on one report would difference every later sample against a stale
  position for the rest of the run. It is the defect this fix exists to stop,
  arrived at from the other direction, and nothing else in the suite would
  redden.
- `the_pointer_regime_changes_under_a_playing_client.rs::losing_the_window_leaves_an_ordinary_delta_spent_whole`
  → the blast radius of FR-5.1-S3's repair, on the path FR-5.1-S3 cannot reach.
  Focus loss costs the position there was to measure from and nothing else, so a
  physical mouse's first sample after the window comes back is still spent whole.
  A repair that cleared the *regime* along with the anchor would leave the client
  waiting for two samples to corroborate a stream it never left, swallowing the
  first flick of every alt-tab back on a local console — and FR-5.1-S3 itself
  cannot see that, because on the absolute path "forget the anchor" and "reset
  everything" publish the same camera. **It is a guard rather than a falsifier at
  RED**: focus loss touches the pointer regime not at all today, so it passes
  before the repair exists, exactly as FR-1.1-S1 and FR-1.1-S2 do. Its job begins
  at GREEN.

## The fixture is copied, and phase B is not an arbitrary half

`tests/fixtures/rdp_pointer/phase_b_cursor_grabbed.txt` holds the 312 raw-motion
lines of phase B of `E:\Temp\rdp-probe\rdp-mouse-probe-output.txt`, copied
verbatim, with the probe's own header preserved as provenance. Nothing in this
workspace can produce a packet of that shape.

**Phase A would have been the wrong half and would have looked fine.** Phase A
does not grab the cursor; its `CursorMoved` data is healthy (max 674×490, real
window pixels), and phase B delivered **no `CursorMoved` at all**. The shipped
client holds the cursor for the whole time a player is playing, so a fixture
drawn from phase A is a fixture of a world the product does not inhabit — which
is the same reading that killed the ticket's candidate repair 3.

## Two derivations, stated because no assertion can enforce them

- **The recording's expectation telescopes.** Differencing consecutive positions
  and summing the differences leaves the last row minus the first, so the whole
  312-sample replay is worth exactly the travel between row 1 and row 312:
  `(34671 − 55313, 26005 − 22552)` units, `(−604.746, +56.904)` counts. The
  oracle therefore never performs the differencing it is checking. It is valid
  only while *every* sample is position-shaped, which the test asserts of the
  fixture rather than assuming.
- **The tolerance is derived from both directions, never loosened until green.**
  The replay reaches the accumulator as 311 separate `f32` additions where the
  control makes one; the smallest difference the comparison must still catch is
  one dropped sample, whose mean step is 419 units — 12.3 counts, 0.027 rad.
  **The analytic bound written here first (`311 × 2⁻²⁴ × 1.5 ≈ 3e−5` rad) was
  the wrong floor**, and measuring is what found that out: see § Measurements
  for the instrument it replaced and the floor that actually binds.

## RED

`cargo test -p mc-client --no-fail-fast --test a_relative_pointer_stream_is_untouched --test an_absolute_pointer_stream_is_spent_as_travel --test the_pointer_regime_changes_under_a_playing_client`,
run on the tree with the tests present and no implementation.

**10 of the 12 named tests FAILED**, and every failure is the defect rather than
merely a failure: the published camera reads
`eye: [8.5, 11.62, 8.5], target: [8.49, 10.62, 8.49]` — a target a full unit
*below* the eye, which is the pitch driven into its own clamp by a single screen
position spent as a device count. The recording's replay reported the travel
comparison off by **1.447 radians**.

**The two that passed are FR-1.1-S1 and FR-1.1-S2, and they cannot be red.**
Both state that the relative path is *unchanged*, so before the fix they are
statements about behaviour that is already correct. `testing.md`'s note that a
requirement that nothing changes yields only guards applies to them exactly:
their job begins at GREEN, where they are what reddens if the fix damages a
physical mouse. FR-5.1-S1 failed at its own positive control rather than at its
absence assertion — the held-pointer control could not turn right because the
defect had already pitched that camera into the clamp.

### RED — FR-5.1-S3, added at validation

`cargo test -p mc-client --no-fail-fast --test the_pointer_regime_changes_under_a_playing_client`,
on the tree carrying the fix with the focus route unrepaired (`a7394d8`):
`test result: FAILED. 5 passed; 1 failed; 0 ignored`. Widened to the package,
`cargo nextest run -p mc-client --no-fail-fast` reported a **bare**
`399 tests run: 398 passed, 1 failed, 0 skipped` — a complete run, and the one
failure is the new test.

**That failure is the defect rather than merely a failure.** The published camera
faces `(−0.612957, 0, 0.790116)` where its control faces
`(0.828709, 0, 0.559680)`: yaw 2.23059 rad against 0.59400, and 0.59400 is
`2 × 135 × LOOK_SENSITIVITY`, the two travels the run actually made. The excess is
**1.636595 rad**, and `25392 × 1920/65536 × 0.0022` is **1.636594** — the gap
between the position the client last saw before the window went away and the
first one it saw after it came back, spent whole as a turn. The two agree to
`9e−7`, which is the published pose's own `f32` resolution at a target 8.5 units
from the origin.

`losing_the_window_leaves_an_ordinary_delta_spent_whole` passed here, as
§ Additional coverage says it must.

## Measurements

Both were taken on the tree at commit `ad8c882`, the last commit before the
documentation. The mutation window was announced to the conductor at
2026-08-26T13:14Z and closed at 13:23Z; each break was reverted by re-editing the
line, never by `git checkout --`, and `git diff --exit-code` on the touched path
was clean afterwards.

### The deletion check — it bites, and PRO-971's hole is not reproduced

The spec's § "Where the decision lives" calls the claim a measurement this phase
owes rather than an argument. Invocation both times:
`cargo nextest run -p mc-client --no-fail-fast --no-tests=pass`.

- **Intact**: `397 tests run: 397 passed, 0 skipped`. A bare `397 tests run`, so
  the run was complete rather than cancelled.
- **Consultation deleted** from `Session::on_pointer_motion` by hand, restoring
  the two-line raw forward: `397 tests run: 387 passed, 10 failed, 0 skipped`.
  Also a complete run.

**Which ten reddened**, named rather than counted:

`a_lone_screen_position_does_not_stop_the_delta_after_it_being_spent` ·
`the_first_screen_position_of_a_session_turns_the_camera_by_nothing` ·
`a_second_screen_position_turns_the_camera_by_the_travel_between_them` ·
`each_further_screen_position_turns_the_camera_by_the_travel_since_the_last` ·
`the_recorded_remote_desktop_session_turns_the_camera_by_the_travel_it_recorded` ·
`the_same_travel_in_either_axis_turns_the_camera_by_the_same_angle` ·
`two_ordinary_deltas_hand_the_stream_back_to_the_relative_reading` ·
`one_ordinary_delta_between_screen_positions_is_spent_on_nothing` ·
`screen_positions_arriving_while_the_cursor_is_free_leave_the_camera_alone` ·
`taking_the_pointer_back_turns_only_by_the_travel_since_it_came_back`

**Which two did not, and why that is the right answer rather than a hole.**
FR-1.1-S1 and FR-1.1-S2 stayed green. Both assert that a relative stream is
*untouched*, and deleting the consultation restores exactly the untouched
relative path — a mutation that cannot redden them is evidence about what they
say, not a gap. The 10 that reddened are precisely the 10 that were red at RED.

This is the contrast the spec drew: PRO-971 shipped a fix whose entire wiring
could be deleted with 383 of 383 green
(`docs/technical/architecture.md`, "Pacing the frame"). **That hole is not
reproduced here.** The difference is structural — the decision sits *between*
the dispatch and the accumulator rather than beside the path — and it is now a
measured difference rather than a claimed one.

### The tolerance, and an oracle that had to be replaced to measure it

Measuring the arithmetic before choosing the assertion, as `testing.md` §2
requires, **found a defect in the test's own instrument**. The first version
compared the recorded replay against the travel control with
`Vec3::angle_between`, and with the threshold temporarily set to `0.0` it
reported the deviation as **exactly `0`** — not because the runs agreed, but
because `acos` of an `f32` dot product near 1 cannot resolve anything below about
`5e-4` rad. The tolerance of `1e-3` was sitting a factor of two above the
*instrument's* noise floor while its doc comment claimed a factor of thirty above
the *arithmetic's*. Replaced with a chord between the two unit facings, which is
linear in the angle and readable to the pose's own precision (`ad8c882`).

Re-measured with the chord, the observable deviation is still exactly `0`. That
is now a fact about the camera rather than about the instrument: simulating the
two accumulations offline over the fixture gives a real difference of **3.6e-7
rad of yaw and 3.0e-8 of pitch** — the 311 `f32` additions do *not* agree with
the single one — and an `f32` step at a target 8.5 units from the origin is about
`1e-6`, so the disagreement falls below what the published pose can represent.

`SAME_TURN` stays `1e-3`: about 500x above that resolution floor and 27x below
one dropped sample of the recording (0.027 rad). It is deliberately **not**
tightened to bit equality even though this fixture would pass it — an over-tight
assertion fails against a correct camera the day the fixture spawns further out,
and the cheapest way to green that is to round something in the product.
