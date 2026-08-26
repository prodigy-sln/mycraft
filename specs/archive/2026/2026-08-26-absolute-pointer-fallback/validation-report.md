# Validation — SPEC-024 (PRO-962), rigor high

**Verdict: FAIL.** Gate green; **Blocker 0 · Major 1 · Minor 2 · Info 4**. A Major is
what fails it, not the gate.

## Gate

`pwsh -NoProfile -File scripts/sdd-gate.ps1`, no args, full gate. Tree: HEAD `3afc94d`
plus one unstaged `metrics.md` line (the resolver's). 15:48:59 → 15:56:01 +02:00.
**Exit 0, `GATE PASSED`.** `grep -n "^\s*FAIL"` on the log: no hits.

- `log:3105` — `     Summary [ 131.384s] 1533 tests run: 1533 passed (4 slow), 1 skipped`
  A **bare** `N tests run`: complete, not cancelled.
- `log:3110` — `ok: coverage 93.72%`
- 12 `ok:` stages at 5, 9, 201, 204, 209, 215, 1536, 1541, 1544, 1548, 3108, 3110.

Log kept outside the repository. Agrees with the implement phase's reading on
`6dc9391`; only wall-clock and the slow-test count differ.

## Deletion check, re-run rather than accepted

Window 15:58:40 → 16:04 +02:00. `session/mod.rs:225-227` replaced by hand with the
pre-fix raw forward, `cargo nextest run -p mc-client --no-fail-fast`:

`Summary [  41.116s] 397 tests run: 387 passed, 10 failed, 0 skipped`

Bare count, so the green 387 is an observation. The 10 are named: the five absolute-travel
tests, the three regime-change tests, the two capture tests. Hand-reverted (no
`git checkout --`); `git diff --exit-code` on the file exits 0. **This wiring is genuinely
not PRO-971's** — that one deleted clean at 383/383.

## Major — focus loss leaves the absolute anchor stale

`session/mod.rs:220-227` forgets the anchor **only** on the pointer-motion path, and only
when capture is refused. Focus loss takes a different route entirely — `events.rs:355` →
`window.rs:158` → `session/mod.rs:279-281` — which clears held keys and touches neither
`capture` nor `regime`. Regaining focus maps to `Other`/`Ignore`; the click that may raise
the window reaches `hold()`, which also forgets nothing.

Shipped-caller fact, not hypothesis: winit's device-event filter defaults to
`DeviceEvents::WhenFocused` (`winit-0.30.13/src/event_loop.rs:596-604`), registering raw
input with `RIDEV_DEVNOTIFY` and **without** `RIDEV_INPUTSINK`
(`.../windows/raw_input.rs:140-147`). `grep -rn listen_device_events crates/` returns
nothing, so the default stands and **no `MouseMotion` is delivered while unfocused**.

Inputs → observable, with fixture values: absolute reading engaged, anchor `x = 34475`.
Alt+Tab away (or a notification steals focus); the pointer crosses the remote desktop
unseen; Alt+Tab back. The next packet reports `x = 65477`; `regime.rs:212` takes the
`(_, true)` arm with the stale anchor and emits `31002 × 1920/65536 = 908.26` counts →
**1.998 rad of yaw from one event**, up to 2.376 rad of pitch for a full-height move.

The finding survives the one uncertainty in it: if raw input *did* arrive while unfocused,
the camera would instead turn while the player is in another application, capture still
believed held. Both branches are defects.

Not ruled item 5 or 6: it is a new hole in the fix's own state machine, and it is the
enormous turn FR-5.1 exists to forbid. FR-5.1-S2 misses it because Escape keeps focus, so
motion keeps arriving and *does* clear the anchor. `InputHarness::lose_focus()` already
exists (`tests/support/input/mod.rs:309`) and no pointer test calls it, so the falsifier is
cheap. Repair is one line on the `ClearInput` branch — but it needs a scenario and a
test-author's test, not an edit here.

## Minor

1. `events.rs:142` and `events.rs:271` still tell the reader "Raw pointer motion, which is
   relative". Commit `3527bb0` removed exactly this claim from `session/mod.rs` and left
   both copies in the adapter the defect came through. A maintainer who believes
   `events.rs:271` and re-simplifies `on_pointer_motion` is doing what it says.
2. `an_absolute_pointer_stream_is_spent_as_travel.rs:97-118` derives `SAME_TURN` for the
   **chord** comparison, then reuses it at :264 and :269 to bound a difference of two
   `atan2`/`asin` angles — a different quantity with a different floor. Adequate in fact
   (130–230× margin), but nothing written down establishes that, and it is the shape of
   instrument defect this spec already caught once.

## Info

Reviewer-reported, not independently re-run: `forget_position`'s "regime kept" half has no
falsifier (`regime.rs:157-167`); the state table has no row for `Absolute` with no anchor
meeting a position-shaped sample, which is the state every recapture lands in;
`architecture.md` says `winit_boundary.rs` fails on any `src/` file naming the library,
while `winit_boundary.rs:57-68` strips doc comments first; `testing.md`'s check 7 reads
"not since PRO-962" for a check nobody has run.

A fast high-DPI flick can produce two all-positive packets with a component ≥ 1000 and
briefly flip the reading. `requirements.md` reasoned about this class explicitly — a
considered trade, recorded rather than filed.

---

# Disposition of pass 1 — facts only, verdict withheld

**Everything above this line is the pass-1 record and is left exactly as it was written.**
It was true of the tree it named. It is no longer true of the tree, and the difference is
below.

**The verdict on the repair is not mine to give, and this section does not give one.**
I found the Major and I also wrote the repair. A report saying "the Major I found is
fixed" over the signature of the agent who fixed it is worth nothing as evidence — it is
the agreement-between-two-copies problem from `standards/global/testing.md` §2 with the
copies being me twice. What follows is what I did and what I measured: observations
anybody can re-take, and which need no trust in my independence. Whether the repair is
good is a pass-2 question and belongs to an agent that did not write it.

## What changed, and in which commits

- `7257bdf` — **FR-5.1-S3** added to the approved spec under FR-5, in the validation
  context, on the conductor's explicit written authorisation.
- `c7b40ce` — the failing test, authored by a test author that had not seen the
  implementation. Not mine.
- `26e9ba8` — the repair: `Session::on_input_cleared` now calls
  `regime.forget_position()`. One line of code.
- `b663ab6` — the lost-focus row in `architecture.md`'s state table, and three sentences
  that had become false: `events.rs:142`/`:271` calling raw pointer motion "relative",
  `architecture.md` on `winit_boundary.rs`, and the `f32`-step derivation in `testing.md`.
- `700dc0f` — a 145-character line my own insertion left unwrapped.
- `f82b6f5` — the same `f32`-step correction in the test file. Not mine; a test file is
  not the validation context's to edit.

## Readings, with their invocations

RED, before `26e9ba8`, `cargo nextest run -p mc-client --no-fail-fast -E
'test(losing_the_window)'`:
`3 tests run: 2 passed, 1 failed, 396 skipped`, the panic naming
`the_pointer_regime_changes_under_a_playing_client.rs:234` — an assertion failure, not a
compile error.

GREEN, after it, `cargo nextest run -p mc-client --no-fail-fast`:
`399 tests run: 399 passed, 0 skipped`.

Both are bare `N tests run`, so complete rather than cancelled.

## Disposition of the three pass-1 findings

- **Major** — repaired in `26e9ba8`; the falsifier is FR-5.1-S3, and
  `losing_the_window_leaves_an_ordinary_delta_spent_whole` guards the repair's blast
  radius on the path FR-5.1-S3 cannot see.
- **Minor 1** (`events.rs` calling the stream relative) — folded into `b663ab6`.
- **Minor 2** (`SAME_TURN` reused for a quantity its derivation does not cover) — **open**,
  filed at completion. The bound is adequate in fact; what is missing is the writing-down.
- **Info** — the `architecture.md` guard description is corrected in `b663ab6`; the
  `f32`-step reason in `testing.md` in `b663ab6` and in the test file in `f82b6f5`. The
  remaining Infos are filed at completion, including the state-table row for `Absolute`
  with no anchor, which is the state every recapture lands in.

## One thing the gate cannot see, recorded because it has stopped being theoretical

`session/mod.rs` was 499 non-blank lines and the repair's single line puts it at **exactly
500 against a limit of 500** (`sdd-gate.ps1:264` compares with `-gt`, so it passes). The
cost was paid during this repair: a thirteen-line explanation of why the pointer is
forgotten there measured 501 and had to move into `regime.rs::forget_position`. That is a
better home for it, which is luck rather than design — **the next line anybody adds to
that file fails the gate.** PRO-974, and a project-level gate decision rather than
anything this spec may fix.

---

# Validation pass 2 — the repair, by an agent that did not write it

**Verdict: FAIL.** New findings only, per `validation-calibration.md` § Re-review:
**Blocker 0 · Major 2 · Minor 1 · Info 1.** Pass 1's Major **is** repaired, and the repair is
now measured from the outside rather than attested by its author — every mutation reading below
stands. What fails this pass is two false statements in `docs/`, both of them statements this
spec's own work falsified.

**Both were first written up here as Minor, and the ranking was mine to get wrong.** What moved
them is not the conductor's ruling — that ruling declined to pre-rank — but this repository's own
precedent, which is more authoritative than my reading of the calibration prose.
`specs/archive/2026/2026-08-18-grass-block-art/validation-report.md:88-115` ranks four false
statements in `docs/` as **Major** (two of them found in *pass 2*, against the same calibration
file), and at `:96-97` ranks a false claim in a **source doc comment** as **Minor**, noting "Doc
comment only". That is a coherent line and it is the line drawn here: `docs/` is the as-built
record, read by people who were not present; a comment beside the code is read by someone who has
the code. Its named pattern — "statements this spec's own work falsified" — is exactly both
findings below. Its finding 5 is also the same *shape* as Minor→Major 1: correct sentences in
front of a wrong one, "which is how it survived being read."

## Gate — independent reading, agrees with both earlier ones

`pwsh -NoProfile -File scripts/sdd-gate.ps1`, no args, full gate. HEAD `a33f1a3`,
`git status --short` empty, `git stash list` empty, no `cargo`/`rustc` process alive at launch.
Log outside the repository and not committed. **Exit 0** — `GATE PASSED` is printed only on
`sdd-gate.ps1:504-507`'s zero-failure branch.

- 12 `ok:` stages, at stdout lines 5, 8, 11, 14, 19, 23, 27, 30, 33, 37, 40, 42.
- `ok: coverage 93.72%`.
- `     Summary [ 134.496s] 1535 tests run: 1535 passed (7 slow), 1 skipped`
  A **bare** `N tests run`: complete, not cancelled.
- `^\s*FAIL` = **0** in both streams. `Blocking waiting for file lock` = **0**.

Every figure matches the conductor's and pass 1's except wall clock and the slow count.
**Provenance caveat**: this run redirected stdout and stderr to separate files, so its line
numbers are not comparable to the 3116/3108 of the earlier readings. Content agrees; numbering
is this invocation's doing.

## The repair — read, then broken three ways

`Session::on_input_cleared` (`session/mod.rs:279-282`) calls `regime.forget_position()` beside
`input.clear_held()`. `PointerRegime` holds one field, `reading` (`regime.rs:120-122`), so
`forget_position` (`regime.rs:171-181`) cannot reach `capture` — assigned in exactly one place,
`session/mod.rs:507` — and cannot change which `Reading` variant is current. It loses the anchor
and the pending corroboration, and nothing else. **Correct.**

**One premise in the pass-2 brief is wrong and the code is right.** `forget_position` is *not* a
no-op on the relative path: `Reading::Relative { corroborating: Some(_) }` is reachable — one lone
position-shaped sample seen while still reading movements (`regime.rs:206`) — and the arm at
`regime.rs:173-175` clears it. That is load-bearing, not incidental. Left uncleared, a player whose
window went away between the first position-shaped sample and its corroborator would have the two
differenced across the gap on return, which is pass 1's Major by a second door. The no-op holds
only from `corroborating: None`, which is where a local console always sits.

## Three mutation windows, each announced, each hand-reverted

`git diff --exit-code` = 0 after every one; HEAD never moved from `a33f1a3`. All three runs report
a **bare** `399 tests run`, so every green below is an observation and not a cancellation.

| window | mutation | result |
|---|---|---|
| 15:07Z | `session/mod.rs:281` deleted | `399 tests run: 398 passed, 1 failed` |
| 15:14Z | `forget_position` body → unconditional `Absolute { anchor: None, corroborating: None }` | `399 tests run: 398 passed, 1 failed` |
| 15:21Z | `session/mod.rs:225-227` → pre-fix raw forward | `399 tests run: 388 passed, 11 failed` |

Window 1 — the single named FAIL is `losing_the_window_turns_only_by_the_travel_since_it_came_back`,
an **assertion** panic at `the_pointer_regime_changes_under_a_playing_client.rs:234:5`, not a compile
error. `losing_the_window_leaves_an_ordinary_delta_spent_whole` is named **PASS** in the same run.
FR-5.1-S3 would have been red. Pass 1's filtered `3 tests run: 2 passed, 1 failed, 396 skipped`
agrees, and this reading additionally shows nothing else moved.

Window 2 — the single named FAIL is `losing_the_window_leaves_an_ordinary_delta_spent_whole`
(`:267:5`), with FR-5.1-S3's own test named **PASS**. So the guard can redden, it reddens on the
exact repair shape `test-map.md` says it exists to catch, and the two tests fail on **disjoint**
mutations. The spec's claim that FR-5.1-S3 cannot see the reset-everything shape is now measured.

Window 3 — 11 named FAILs. The three pointer tests that do **not** redden are
`a_relative_pointer_stream_is_untouched::two_samples_turn_the_camera_as_far_as_one_sample_of_their_sum`,
`::a_sample_with_a_negative_component_turns_the_camera_the_way_its_sign_names`, and
`the_pointer_regime_changes_under_a_playing_client::losing_the_window_leaves_an_ordinary_delta_spent_whole`
— all three assertions that the relative path is unchanged, which is exactly what deleting the
consultation restores.

## The anchor sweep — checked, and it does not fully collapse

The collapse in `spec.md` § Notes is right about its own premises: `forget_position` and
`on_pointer_motion` each have one caller (`session/mod.rs:222`/`:281`, `events.rs:290`), `capture`
is assigned once (`session/mod.rs:507`), and `LoopAction::ClearInput` has exactly one producer,
`WindowEventKind::FocusLost` (`mc-render/src/window.rs:158`, reached only from `events.rs:362`).
Within the delivery model, foreground really is the only thing that stops a sample.

What the collapse does not cover is **Info 1** below: staleness by a changed frame of reference
rather than by a gap in the stream. It is not a Major and I say why there.

## Major — new

1. **`docs/technical/testing.md:1811` states a number this branch's own test file contradicts.**
   It reads "a fixture spawning one block higher puts a component past 16 and doubles the floor".
   The eye's largest component is `11.62`; one block higher is `12.62`, which is not past 16.
   `crates/mc-client/tests/an_absolute_pointer_stream_is_spent_as_travel.rs:119-122` — corrected in
   `f82b6f5`, the last commit before HEAD — says "**4.38 blocks of headroom** — five blocks of extra
   spawn height, not one". The repair landed in the test file and not in the document, so the tree
   ships two contradictory statements about one quantity, in the section whose subject is a number
   that is right while describing the wrong one. **Major, not Minor**: it is in `docs/`, it is
   objectively false, and it is a statement this spec's own work falsified. The reader it fails is
   the one maintaining `SAME_TURN` — "one block of headroom" reads as a knife edge and invites
   changing the tolerance, where 4.38 (see Minor 1: really ≥ 3.38) reads as room. Repair: correct
   the clause to match `an_absolute_pointer_stream_is_spent_as_travel.rs:119-122`.

2. **`docs/technical/architecture.md:754-757` states a measurement that no longer reproduces.**
   "**Measured, not argued**: `cargo nextest run -p mc-client --no-fail-fast` reports
   `397 tests run: 397 passed` intact and `397 tests run: 387 passed, 10 failed` with the
   consultation deleted." FR-5.1-S3's two tests were added after that reading was taken, and the
   document does not name the tree it was taken on. Re-measured at `a33f1a3` in window 3 above:
   **`399 tests run: 399 passed` intact and `399 tests run: 388 passed, 11 failed` deleted**, with
   three non-reddening tests rather than two. The prose around it stays true; the figures do not.

   **Major, not Minor, and not because the number is stale.** Both figures were true when taken.
   What fails is that a reader who re-runs the stated invocation on the tree the document ships
   with gets a different answer and **cannot tell whether the discrepancy is their tree, their
   invocation, or a defect** — while the passage's whole rhetorical weight is "**Measured, not
   argued**". A measurement claim that cannot be reproduced is the one kind of sentence a reader
   is invited to act on and cannot.

   **A bare update to 399 is not the repair**, because it rots again on the next spec that adds a
   test to `mc-client`. Either name the commit the reading was taken on, or state it in a form
   that does not rot. The second is available and costs nothing, because window 3 named its
   failures: *deleting the consultation reddens exactly these eleven and nothing else* —
   `a_relative_pointer_stream_is_untouched::a_lone_screen_position_does_not_stop_the_delta_after_it_being_spent`;
   `an_absolute_pointer_stream_is_spent_as_travel::{the_first_screen_position_of_a_session_turns_the_camera_by_nothing,
   a_second_screen_position_turns_the_camera_by_the_travel_between_them,
   each_further_screen_position_turns_the_camera_by_the_travel_since_the_last,
   the_recorded_remote_desktop_session_turns_the_camera_by_the_travel_it_recorded,
   the_same_travel_in_either_axis_turns_the_camera_by_the_same_angle}`;
   `the_pointer_regime_changes_under_a_playing_client::{two_ordinary_deltas_hand_the_stream_back_to_the_relative_reading,
   one_ordinary_delta_between_screen_positions_is_spent_on_nothing,
   screen_positions_arriving_while_the_cursor_is_free_leave_the_camera_alone,
   taking_the_pointer_back_turns_only_by_the_travel_since_it_came_back,
   losing_the_window_turns_only_by_the_travel_since_it_came_back}`.
   The three pointer tests that stay green are the two FR-1.1 relative-path scenarios and
   `losing_the_window_leaves_an_ordinary_delta_spent_whole` — all three assertions that the
   relative path is unchanged, which is what deleting the consultation restores. A named list is
   reproducible on any tree; a total count is not.

   **This repair is deliberately not written here.** Pass 2 exists because the validation context
   wrote pass 1's repair, and a pass-2 context that repairs its own Major hands pass 3 the same
   problem. The replacement text above is handed over rather than committed.

## Minor — new

1. **`an_absolute_pointer_stream_is_spent_as_travel.rs:120` names the wrong largest component.**
   "the largest is the eye's `11.62`" — but the compared observable is built from `eye` *and*
   `target`, and the target is the eye plus a unit direction, so its `y` reaches up to `12.62`.
   Headroom is at least `3.38` blocks, not `4.38`. The conclusion is unaffected: the floor is
   `9.5e-7` either way and `SAME_TURN` still sits some 500× above it. **Minor and not Info**: it is
   an objectively wrong number, and `validation-calibration.md` reserves Info for what is
   subjective. It is a source doc comment rather than `docs/`, which is the line
   `2026-08-18-grass-block-art/validation-report.md:96-97` draws — so Minor and not Major.

## Info — new

1. **The anchor carries no record of the frame it was taken in, and one class of change moves that
   frame without stopping a sample.** The absolute range is normalised over the display, so a
   display-mode change, a monitor hotplug or a Remote Desktop dynamic-resolution resize alters what
   a given screen position normalises to — with the window still foreground, motion still arriving,
   and `capture` unchanged. Neither route that forgets the anchor is reached: `WindowEvent::Resized`
   maps to `LoopAction::Resize` (`mc-render/src/window.rs:156`) and reaches the regime not at all.
   `spec.md` § Notes asserts that "a display mode change, a monitor hotplug, a window move and a
   session disconnect all collapse into that one route"; a window move and a disconnect do, the
   other two do not — they are a different mechanism, not a slower version of the same one.
   **Deliberately not filed as Major.** The magnitude depends on what the normalisation actually
   does across a mode change on this platform, and nobody has measured it — staking a FAIL on that
   is the exact mistake `spec.md` § Technical Considerations declines to make about
   `primary_monitor()`. It wants the same instrument the first defect needed: a human on Remote
   Desktop. Recorded here so the sentence in § Notes does not stop the next person looking, and
   recommended as a tracked issue beside PRO-983.

## What was looked at and found sound

`looking_apart` (`:424-425`) is a chord and is the right instrument; `yaw` (`atan2`) and `pitch`
(`asin` near zero) are both well-conditioned and carry no `acos`-shaped floor. `lose_focus()`
enters through the shipped `dispatch_window_event` with a real `WindowEvent::Focused(false)`
(`tests/support/input/mod.rs:309-311`), so a broken harness would redden FR-5.1-S3 rather than hide
in it. `docs/user/gameplay.md`'s two thirds of a revolution checks out (1920 × 0.0022 = 4.224 rad
against 2π), as does its 15-20%. `docs/modding/script-surface.md`'s absence is stated, which is
what Key Principle 4 asks for. `winit_boundary.rs:58-68` does strip `///` and `//!` before scanning,
so `architecture.md`'s corrected description of it is true.

---

# Validation pass 3 — the four repairs, by an agent that wrote none of them

**Verdict: PASS.** New findings only: **Blocker 0 · Major 0 · Minor 0 · Info 0.** No blocking
issues. All four repairs are true, not merely different; every number in the new text traces to
a reading somebody took or to arithmetic anybody can redo; and nothing at Major or above is new.

## Gate — fourth independent reading, on the tree under review

`pwsh -NoProfile -File scripts/sdd-gate.ps1`, no args, full gate. HEAD `aef46ef`,
`git status --short` empty and `git stash list` empty at launch, no `cargo`/`rustc`/`nextest`
process alive. 15:38:02Z → 15:43:34Z. Log outside the repository, not committed.

- **Exit 0**, `GATE PASSED`.
- `log:3112` — `     Summary [ 139.338s] 1535 tests run: 1535 passed (8 slow), 1 skipped`.
  A **bare** `N tests run`: complete, not cancelled.
- 12 `ok:` stages at 6, 10, 202, 205, 210, 216, 1537, 1542, 1545, 1549, 3115, 3117.
- `ok: coverage 93.72%`.
- `grep -nE "^\s*FAIL"` = **0** hits. `Blocking waiting for file lock` = **0**.

Agrees with pass 1, pass 2 and the repairing agent's reading on every figure except wall clock
and the slow count. This is the first full-gate reading taken **on `aef46ef` by an agent that
committed nothing to it**.

## No mutation window was opened, and it was not needed

Pass 2's three windows were taken at `a33f1a3`. `git diff a33f1a3..aef46ef -- '*.rs'`, filtered
to lines that are not `///` doc comments, is **empty** — the only Rust change in the three
intervening commits is the doc comment at
`an_absolute_pointer_stream_is_spent_as_travel.rs:119-126`. No test function was added, removed
or renamed and no product line moved, so pass 2's readings transfer to `aef46ef` **by
construction** rather than by re-measurement. That is a stronger transfer than a re-run: a re-run
would only show the same numbers, where the diff shows there was nothing that could have changed
them.

## Repair 1 — `docs/technical/testing.md:1810-1815`

The two quantities are both stated and neither is stated as the other:
"**3.38 blocks of headroom**, and **four** blocks of extra spawn height is the smallest whole
number that crosses — `12.62 + 3 = 15.62` does not, `12.62 + 4 = 16.62` does, and past 16 the
floor becomes `1.9e-6`."

Chain re-derived from source, not from the brief: `tests/support/input/world.rs:52` `FLOOR = 9`
and `:60` `SPAWN = Vec3::new(8.5, (FLOOR + 1) as f32, 8.5)` → spawn `y = 10.0`;
`mc-sim/src/player/mod.rs:30` `EYE_HEIGHT = 1.62` → eye `y = 11.62`; target is eye plus a unit
direction → `y ≤ 12.62`. `16 − 12.62 = 3.38` ✓. `12.62 + 3 = 15.62 < 16` ✓, `12.62 + 4 = 16.62
≥ 16` ✓, so 4 is the smallest whole crossing ✓. The `f32` step over `[8,16)` is `2^3 × 2^-23 =
2^-20 = 9.5367e-7` and over `[16,32)` is `2^-19 = 1.907e-6` ✓ — the stated `1.9e-6`. Raising
spawn by 3 leaves `x`/`z` at 8.5/9.5 and `y` at 14.62, all still in `[8,16)`, so the floor
genuinely does not move until 4 ✓.

`grep -rn "one block higher\|4\.38\|five blocks"` over `docs/` and `crates/` returns **no hit
about this quantity** — the remaining hits are `mc-sim` collision and movement comments on
unrelated subjects. The superseded wording survives only inside this report's pass-2 section,
where it is quoted history, and in the conductor's `.claude/pm/` working notes, not `docs/`.

## Repair 2 — `an_absolute_pointer_stream_is_spent_as_travel.rs:119-126`

`git show c0a56d3 -- <the test file>` is **one hunk**, and filtering its `+`/`-` lines to
anything not matching `^[+-]/// ` returns **nothing**. Doc comment only: no assertion, no
constant, `SAME_TURN` still `1e-3`.

## Repair 3 — `docs/technical/architecture.md:754-781`

Both bare counts are gone from `docs/` — `grep -rn "397 tests run\|399 tests run" docs/` returns
nothing. The passage now names the invocation *with* `--no-fail-fast` (which is what makes
"nothing else" an observation rather than a cancellation), lists the eleven, names the three that
stay green with the reason, and dates itself: "This list was last read at `a33f1a3`."

**Verified mechanically, not by eye.** Extracting every `#[test]` function name from the three
pointer test files and comparing against the names the passage lists: **14 test functions, 11 in
the red list, 3 in the green list, no name in both, none missing, and no listed name that is not
a test function.** The two lists are an exact partition of the pointer suite — which also makes
the word "eleven" a redundant check on its own list. Every name matches pass 2's window-3 report
character for character, and the three green ones do all assert the relative path is unchanged
(`a_relative_pointer_stream_is_untouched`'s two FR-1.1 scenarios and
`losing_the_window_leaves_an_ordinary_delta_spent_whole`), which is what deleting the
consultation restores.

## Repair 4 — `spec.md:497-506`

The false collapse is gone and the replacement is checked against source rather than accepted:
`crates/mc-render/src/window.rs:156` is `WindowEventKind::Resized(size) => LoopAction::Resize(*size)`
✓, and `LoopAction::Resize` has exactly one consumer, `events.rs:134 → on_resize`
(`events.rs:198-202`), whose body touches `self.app` and never `self.session` — so it reaches the
regime **not at all** ✓. `grep -n "Resize" crates/mc-client/src/session/mod.rs` returns nothing.
The bound is now correctly qualified as holding over the delivery model it was drawn across, and
the second mechanism is tracked as PRO-984.

## Filtered out — looked at, deliberately not filed

- **The brief's claim that the two headroom locations are "worded identically" is not quite
  right, and it does not matter.** Their trailing clauses run in opposite polarity: the test file
  says "`15.62` stays under, and `16.62` does not", `testing.md` says "`15.62` does not, `16.62`
  does". Both are true and both name the same two quantities. Reported because the brief credits
  a safeguard that is not actually in place, not because either sentence is wrong.
- **`an_absolute_pointer_stream_is_spent_as_travel.rs:108-109` says "every component lies in
  `[8, 16)`"**, but target `x`/`z` reach `8.5 ± 1`, so a component can sit in `[4, 8)`. Not filed:
  it is pre-existing text from `f82b6f5` rather than anything this repair introduced, it errs
  toward a *finer* step (`2^-21 = 4.8e-7`, still above the `3.6e-7` disagreement the sentence
  exists to call unrepresentable), and the headroom claim uses the largest component and is
  untouched by it. Sub-Minor, and confirming it would need the fixture's facings measured.
- **`spec.md:497` "A window move and a session disconnect collapse into that one route."** A
  window move arguably stales nothing at all rather than collapsing into the focus-loss route,
  since absolute coordinates normalise over the display and not the window. Surviving text, not
  the repair's subject, and sub-Major either way.
- Everything on the conductor's ruled list: PRO-979, PRO-974, the artifact-lint overflows,
  PRO-981, PRO-984, PRO-978/980/983, pass 1's Minor 2, PRO-972, PRO-971's pacing, and PRO-985's
  four sibling bare counts — including `architecture.md:781`'s "383 of 383 green", which is
  the same defect one paragraph away and correctly left for a mutation re-run rather than an edit.
