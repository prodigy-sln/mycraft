---
id: SPEC-024
title: Mouse look is unusable over Remote Desktop
status: implemented
work-type: fix
rigor: high
branch: feature/PRO-962-absolute-pointer-fallback
issue: PRO-962
created: 2026-08-26
approved: 2026-08-26
completed: 2026-08-26
author: Sebastian Grunow
---

# Fix: Mouse look is unusable over Remote Desktop

## Defect

- **Observed**: over a Windows Remote Desktop session the camera snaps and spins
  instead of turning. Raw pointer motion reaches the client as *screen positions*
  normalised to `0..65535`, and the client spends every one of them as a look
  delta. One packet from the recorded run — `x = 22348` — is
  `22348 × LOOK_SENSITIVITY` = **49.2 radians**, or 7.8 full revolutions, from a
  single event. This blocks *playing* the game over RDP, which is the only way
  the owner reaches this machine when away from home, and so blocks manual
  acceptance of every MVP shipped so far.
- **Expected**: mouse look over RDP turns the camera by the distance the pointer
  moved, at a comparable rate to a local mouse, whether or not the cursor is
  grabbed; and a physical mouse on a local console behaves exactly as it does
  today.
- **Reproduced**: yes, on the real system, 2026-08-25. `E:\Temp\rdp-mouse-probe\`
  (a winit-0.30.13 harness pinned to the version the client ships) was run by the
  owner over a live RDP session; 809 samples are recorded in
  `E:\Temp\rdp-probe\rdp-mouse-probe-output.txt` and summarised in PRO-962's
  comment. **584 of 584 raw-motion samples exceed 1000**, peaking at 65477 — the
  top of the absolute range. Consecutive samples trace smooth monotone paths
  (`22348 → 22191 → 21917 → 21681 → 21407 → 21132`) and hold *exactly constant*
  while the mouse is still, where a relative stream emits nothing at all.

  Two further readings from that run shape this fix and are not derivable from
  the Win32 contract:

  - **Phase B (cursor grabbed, `Locked` granted) delivered no `CursorMoved` at
    all.** Deriving look from the window's cursor position — PRO-962's candidate
    repair 3 — is dead on the exact path that needs it, because the client holds
    the cursor for the whole time the player is playing. Phase A's cursor data is
    healthy (max 674×490, real window pixels), so **the phase-A data alone would
    have endorsed the wrong design.**
  - **The absolute range is normalised per axis over the display**, so equal
    pointer travel is *not* equal in the two axes' units. Paired
    `MouseMotion`/`CursorMoved` samples in phase A give **39.26 units per pixel
    horizontally and 71.94 vertically** (t = 1.864 s: motion 43775 ↔ cursor 310;
    t = 2.473 s: motion 44678 ↔ cursor 333 — and the same two constants recover
    every other pair in the run). Differencing the raw stream and feeding the
    existing sensitivity would leave the vertical axis 1.83× as fast as the
    horizontal.

## Root Cause

Three links, each individually defensible, and the defect is at the second.

1. **The client forwards raw motion unconditionally.**
   `crates/mc-client/src/events.rs:281-285` destructures
   `DeviceEvent::MouseMotion { delta }` and hands both components to
   `Session::on_pointer_motion`, which spends them as device counts
   (`crates/mc-client/src/session/mod.rs:212-217` →
   `crates/mc-sim/src/player/input.rs:124`, `yaw_delta += raw_x * 0.0022`). The
   adapter's own doc comment states that it decides nothing, deliberately —
   `seam_boundaries.rs` and `winit_boundary.rs` both enforce that no decision
   lives in that file. **The forward is correct; what is missing is a decision on
   the session's side of the seam.**

2. **`DeviceEvent::MouseMotion`'s delta is not always a delta on Windows.**
   `winit-0.30.13/src/platform_impl/windows/event_loop.rs:2569`:

   ```rust
   if util::has_flag(mouse.usFlags as u32, MOUSE_MOVE_RELATIVE) {
       let x = mouse.lLastX as f64;
       let y = mouse.lLastY as f64;
       ...
       if x != 0.0 || y != 0.0 { ... MouseMotion { delta: (x, y) } }
   ```

   with `has_flag(bitset, flag) -> bitset & flag == flag`
   (`.../windows/util.rs:41`) and
   `MOUSE_MOVE_RELATIVE: MOUSE_STATE = 0u16`
   (`windows-sys-0.59.0/src/Windows/Win32/UI/Input/mod.rs:34`). `usFlags & 0 == 0`
   is **true for every packet**, so the guard that was meant to admit only
   relative motion admits all of it, and `MOUSE_MOVE_ABSOLUTE` (`= 1`) is never
   consulted anywhere in the crate. A physical mouse sets the relative bit and
   the defect is invisible; RDP's virtual mouse sets `MOUSE_MOVE_ABSOLUTE`, where
   `lLastX/lLastY` are positions normalised to `0..65535` over the display.

3. **The client believes a library contract on a platform where the library
   breaks it.** winit documents `MouseMotion` as a change in physical position,
   and nothing in this workspace tests a third-party contract. Verified as ours
   alone: the same `has_flag(_, ZERO)` shape occurs once in winit — every other
   flag it tests (`RI_MOUSE_WHEEL`, `TOUCHEVENTF_*`, `POINTER_FLAG_*`,
   `RI_KEY_E0/E1`) is non-zero.

The same class reaches tablets, touchscreens and some VM guest drivers, which
also set `MOUSE_MOVE_ABSOLUTE`.

## Regression Scenarios

Every scenario below is dispatched through `dispatch_device_event` — the entry
the shipped event loop crosses — and observed as the `CameraPose` the renderer is
handed, as `tests/pointer_dispatch.rs` already does. Neither the trigger nor the
observable is an intermediate the product could stop consulting while still
drawing the same wrong picture.

Concrete values below use the calibration in § Technical Considerations: one
absolute unit is `1920/65536` device counts horizontally and `1080/65536`
vertically, so **4608 units of horizontal travel and 8192 units of vertical
travel are each exactly 135 counts.**

### FR-1 — A relative stream is untouched

- **FR-1.1**: The path a physical mouse takes is the path it took before this
  fix: every sample is spent as a delta, and nothing accumulates between samples.
  - FR-1.1-S1: WHEN two consecutive samples of 50 counts to the right arrive THE
    SYSTEM SHALL turn the camera by the same angle a single sample of 100 counts
    turns it.
  - FR-1.1-S2: WHEN a sample carrying a negative component arrives THE SYSTEM
    SHALL turn the camera by that sample as a delta, in the direction its sign
    names.

### FR-2 — An absolute stream is measured before it is believed

- **FR-2.1**: A sample is *position-shaped* when both components lie within
  `0..=65535` and at least one is at least 1000. One such sample is not evidence;
  two consecutive ones are.
  - FR-2.1-S1: WHEN the first position-shaped sample of a session arrives —
    `(30000, 20000)` — THE SYSTEM SHALL leave the camera exactly where a tick
    with no motion at all leaves it.
  - FR-2.1-S2: WHEN `(30000, 20000)` is followed by `(34608, 20000)` THE SYSTEM
    SHALL turn the camera by the 135 counts that travel means, and not by the
    thirty thousand counts either position names.
  - FR-2.1-S3: WHEN a further sample `(39216, 20000)` follows THE SYSTEM SHALL
    turn the camera by a further 135 counts, measured from the sample before it.
  - FR-2.1-S4: WHEN the raw samples the recorded RDP session delivered while the
    cursor was grabbed are dispatched in order THE SYSTEM SHALL turn the camera
    by the angle that recording's pointer *travel* implies, and not by the four
    orders of magnitude more that its raw values imply.

### FR-3 — Both axes mean the same travel

- **FR-3.1**: The absolute range is normalised per axis, so the two axes carry
  different units and the client converts them separately.
  - FR-3.1-S1: WHEN 4608 units of horizontal travel and 8192 units of vertical
    travel are each dispatched from the same starting camera THE SYSTEM SHALL
    change the yaw and the pitch by the same angle.

### FR-4 — The regime can change while the game is running

An RDP session resumed on the local console, and a local session reconnected
over RDP, both change the shape of the stream under a client that is already
playing.

- **FR-4.1**: Evidence in either direction is corroborated before it is acted
  on, and the sample that decides is never also spent as the wrong kind.
  - FR-4.1-S1: WHILE the absolute fallback is engaged, WHEN two consecutive
    samples that are not position-shaped arrive — `(3, 2)` then `(-4, 1)` — THE
    SYSTEM SHALL turn the camera by the second of them as a delta and by nothing
    else.
  - FR-4.1-S2: WHILE the absolute fallback is engaged, WHEN a single
    not-position-shaped sample `(800, 600)` arrives between position-shaped ones
    THE SYSTEM SHALL leave the camera where it was for that sample, and turn it
    by the travel measured from the last position for the one after it.

### FR-5 — The pointer the game does not hold teaches it nothing

- **FR-5.1**: A capture the platform did not grant, or gave back, leaves the
  fallback with no position to measure from.
  - FR-5.1-S1: WHILE the cursor belongs to the desktop, WHEN position-shaped
    samples arrive THE SYSTEM SHALL leave the camera exactly where a tick with no
    motion at all leaves it.
  - FR-5.1-S2: WHEN the pointer is held again after the cursor was free, THE
    SYSTEM SHALL turn the camera only by travel measured since it was held again,
    and never by the distance between where the pointer was released and where it
    came back.
  - FR-5.1-S3: WHILE the fallback is engaged, WHEN the window loses focus and
    position-shaped samples resume afterwards THE SYSTEM SHALL turn the camera
    only by travel measured since they resumed, and never by the distance the
    pointer covered while the window was not focused.

    **Added at validation, and it is a different route from S2 rather than a
    restatement of it.** S1 and S2 free the cursor by *capture* — Escape, then a
    click — and on that route the window keeps focus, so raw motion keeps
    arriving and the sample that arrives while uncaptured is what clears the
    anchor. Losing focus stops the samples instead: winit registers raw input
    without `RIDEV_INPUTSINK` (`DeviceEvents::WhenFocused`, its default, which
    nothing in this workspace overrides), so no `MouseMotion` is delivered at all
    while the window is not foreground — and the capture state, which is the only
    thing the anchor is forgotten against, never changes. The anchor therefore
    survives an Alt+Tab, and the first sample after the player returns is
    differenced against a position from before they left: measured on this
    spec's own fixture, an anchor at `x = 34475` meeting `x = 65477` emits
    `31002 × 1920/65536 = 908.26` counts, **1.998 radians of yaw from one
    event**, and up to 2.376 radians of pitch for a full-height move.

    The route was not enumerated because FR-5.1's prose says "a capture the
    platform did not grant, or gave back", which is the capture ladder — the
    thing that has an event — and the window's focus has no bearing on it. What
    made it invisible is recorded in § Notes.

## RCA

### Causal chain

- **Symptom** → the camera snaps and spins under RDP (owner's report; probe
  output, 584/584 samples over 1000).
- **Failing mechanism** → `crates/mc-client/src/session/mod.rs:216` spends
  `raw_x`/`raw_y` as device counts with no question asked about what they are.
- **Origin** → `winit-0.30.13/src/platform_impl/windows/event_loop.rs:2569`,
  guarding the relative path with a flag whose value is `0`. Inherited, not
  introduced here; this client's part is that it forwards unconditionally, which
  was a deliberate and *documented* choice (`events.rs:270-280`) made under the
  assumption that the library's contract holds.

### Detection gap

Three instruments could have reported this and none was pointed at it.

1. **Every fixture in the suite supplies the form a local mouse produces.**
   `tests/pointer_dispatch.rs` dispatches `RAW_COUNTS = 100.0`; no test in the
   workspace has ever handed the client a value above 1000, a value that never
   goes negative, or two consecutive identical values. This is `testing.md` §2's
   rule — *what does the shipped caller supply* — with the answer "one of the two
   callers, and not the broken one". The fixtures are correct; they describe a
   world half the product's users inhabit.
2. **A forward has nothing to assert.** `on_pointer_motion` held no
   input-validity decision, so there was no policy to test and no wiring to
   check. Coverage would have called the line covered on every run.
3. **The gate cannot see it.** No lint, scan or golden reads the *units* of a
   third-party event, and `mc-client` is out of the coverage denominator
   (ADR-008, narrowed by ADR-013) in any case.

**The instrument that did find it was a probe run by a human on the real system**,
and the probe's own history is the transferable part: its phase-A data was
*accurate* about the raw stream and would have endorsed a repair that ships a
dead camera, because phase A does not grab the cursor and the shipped client
always does. A screen validated on one output has not been validated on another.

### Sibling sweep

- **Inside the client**: `events.rs` reads exactly four things from the library —
  a key code, a mouse button, a surface size, and this delta. Key codes and
  buttons are discrete symbols with no units. `WindowEvent::Resized(size)` is
  physical pixels, and a wrong value would be visible as a wrong-sized image on
  the first frame. **`MouseMotion.delta` is the only continuous quantity the
  client consumes whose units it must take on trust, and it is the one that
  broke.** No in-scope sibling.
- **Inside winit**: one instance of `has_flag(_, ZERO)`, and it is this one
  (§ Root Cause 3). Out of scope here; the upstream repair belongs in a tracked
  issue, not in this fix.
- **The consumer/producer question** (`docs/technical/testing.md`, "A sweep that
  asks whether a consumer is correctly paced does not ask what writes what it
  reads"): the consumer is `InputState::look`, and it is correct — it turns
  counts into radians and always has. Asking *what writes the number it reads,
  and in what units* is the question this defect lives in, and it had never been
  asked.

### Prevention

- **A fixture in the broken regime**, which is what the suite lacked: FR-2, FR-3
  and FR-4 are all driven by position-shaped samples through the shipped
  dispatch. This is the analogue of
  `tests/reload_survives_a_multi_tick_frame.rs` — a regime change needs a fixture
  in the new regime.
- **A fixture made of real measurements** (FR-2.1-S4): an excerpt of the recorded
  RDP run is committed under `crates/mc-client/tests/fixtures/` and replayed. It
  is the one test in this spec whose input was produced by the system that
  actually breaks, rather than by someone reasoning about it.
- **No gate amendment is proposed.** There is no deterministic project-wide check
  for "a third-party contract is false on one platform"; the honest prevention is
  the regime fixture above.

## Technical Considerations

### Where the decision lives, and why the wiring cannot be deleted quietly

In `Session`, consulted by `on_pointer_motion` **on the path**, not beside it.
Two independent constraints force this: `winit_boundary.rs` fails the build if a
second file in `mc-client/src` names the library, and `seam_boundaries.rs` fails
it if a decision arrives in `events.rs`.

This is deliberately a different structural position from PRO-971's, whose entire
wiring could be deleted with 383 of 383 tests green
(`docs/technical/architecture.md` § "Pacing the frame"). Here the decision sits
*between* the dispatch and the accumulator, so deleting the consultation restores
the raw forward — and every FR-2, FR-3 and FR-4 scenario reddens, because each
one dispatches a position-shaped sample and asserts a bounded turn. **The
strength of that claim is a measurement the implement phase owes, not an
argument**: delete the consultation by hand, record which tests redden and which
do not, and revert by hand.

### The repair, stated as a state machine

A sample is **position-shaped** iff `0 ≤ x ≤ 65535`, `0 ≤ y ≤ 65535`, and
`x ≥ 1000 ∨ y ≥ 1000`. The threshold is the probe's own reading threshold: 584 of
584 recorded samples clear it, with a minimum `|x|` of 21093 — while `|y|` fell
as low as 35, which is why *one* component suffices and both are not required.

| state | sample | look emitted | next state |
|---|---|---|---|
| Relative, nothing pending | position-shaped | none | Relative, pending = sample |
| Relative, pending | position-shaped | `(sample − pending) × scale` | Absolute, anchor = sample |
| Relative, any pending | not position-shaped | sample unchanged | Relative, nothing pending |
| Absolute, anchor | position-shaped | `(sample − anchor) × scale` | Absolute, anchor = sample |
| Absolute, nothing pending | not position-shaped | none | Absolute, pending = sample |
| Absolute, pending | not position-shaped | sample unchanged | Relative, nothing pending |
| Absolute, pending | position-shaped | `(sample − anchor) × scale` | Absolute, anchor = sample |
| either, cursor not held | anything | none | anchor forgotten, regime kept |

Three properties are worth naming because they are what the scenarios buy:

- **No first-sample snap, in either direction.** A single position-shaped sample
  never turns the camera, so the very packet that produces today's 49-radian spin
  produces nothing at all.
- **Two-sample hysteresis both ways**, so one freak report cannot flip the
  regime, and the sample that decides a change is spent as the kind the new
  regime says it is — never differenced against a stale anchor.
- **Forgetting the anchor when the cursor is free** is what stops the pointer's
  journey across the desktop, between an Escape and the click that comes back,
  from arriving as one enormous turn.

### The scale is a declared calibration, not a measurement, and this is the trade

`counts_x = units_x × 1920/65536`, `counts_y = units_y × 1080/65536`: the
absolute range is declared to span a nominal 1920 × 1080 display. Crossing the
full width of the remote display is then 1920 counts, or 4.224 radians — about
two thirds of a revolution — which is what a 1920-pixel sweep of an ordinary
mouse means locally.

Checked against the run rather than asserted: at the measured 39.26 and 71.94
units per pixel, this calibration yields **1.150 counts per pixel horizontally
and 1.185 vertically** — the axes agree to 3.1%, and the overall rate runs 15-19%
faster than a one-count-per-pixel mouse. **A nominal is wrong by the ratio of the
real display to 1920 × 1080**: half speed on a 3840-wide display, half again as
fast on a 1280-wide one.

The alternative is one `PointerPlatform` method returning the extent the absolute
range actually spans, answered in production from `primary_monitor()`.
**Testability does not separate the two, and an earlier draft of this section
claimed it did.** The conversion is a pure function of the extent either way and
is testable with any extent, nominal or real; what no windowless test can execute
is only whether production passes the *real* extent rather than a constant — and
a nominal has exactly that same unreachable step, plus a known wrongness.

What separates them is a value nobody in this project has measured: **what
`primary_monitor()` returns inside an RDP session — the remote session's extent
or the host machine's.** The exact route is therefore staked on an unmeasured
platform behaviour, which is the specific mistake this defect already taught:
reading the Win32 contract was right about the packet shape and would have been
wrong about `CursorMoved`, and only the probe told them apart. A nominal's error
is bounded, deterministic, reproducible on any machine, and diagnosable from the
documentation. The exact route's error, if `primary_monitor()` answers with the
host's display, is silent and machine-dependent. **The nominal is taken for that
reason and not for testability.** Measuring what `primary_monitor()` reports over
RDP is what would reopen this, and it needs a human on RDP, exactly as the first
probe did.

### What the documentation owes, and it is part of this fix

The calibration above is a constant presented honestly or not at all — a constant
presented as *correct* is one nobody re-examines. All three audiences are owed
something here (CLAUDE.md Key Principle 4), and the mod-author surface is the one
that genuinely does not exist: no scripted content can see or influence pointer
input, and that absence is stated rather than left silent.

- **Player** — mouse look now works over Remote Desktop, and over any other
  pointer that reports absolute positions. It engages by itself; there is nothing
  to switch on. It may feel somewhat faster than a local mouse, and **there is no
  sensitivity setting yet** — the honest instruction when it feels wrong is to
  say so, because that is the only thing that changes it.
- **Engine reader** — the as-built record: the winit guard, the state machine and
  its hysteresis, the threshold's derivation from the recording, the rejected
  clamp with its numbers, and the calibration written down as **assumed
  1920 × 1080 against a session measured at ~1669 × 911, giving a rate 15-19%
  fast there**. A future change may not remove the two-sample corroboration in
  either direction, and may not spend a deciding sample as the regime it is
  leaving.
- **Mod author** — nothing. Pointer input reaches no scripted surface, and this
  fix adds none.

### A plausibility clamp was considered and rejected on the data

Bounding the differenced step would swallow the regime-change artefact for free.
The recording says not to: the largest consecutive steps in it are 32085 units
over a 604 ms gap and 16913 over 390 ms — legitimate movement across intervals
where no event was delivered — while the largest step at the stream's ordinary
15 ms cadence is 6868. A bound loose enough to admit the first is useless against
the artefact, and a bound tight enough to catch the artefact deletes real turns
on any machine that drops frames. The hysteresis above handles the regime change
instead, and does so without a threshold to tune.

### Invariants

Invariant 4 (the server is authoritative) is untouched: this changes how the
client turns *its own* camera, and the movement intent it sends is recomputed
server-side exactly as before. No `mc-sim`, `mc-proto` or content change.

## Existing Code to Leverage

- `crates/mc-client/tests/support/input/` — `InputHarness::move_pointer` already
  dispatches a real `DeviceEvent::MouseMotion` through `dispatch_device_event`,
  and `tests/pointer_dispatch.rs` already reads the published `CameraPose`,
  derives "right" from a control's own facing, and pairs every absence assertion
  with a positive control. Every scenario here is a new case in that shape.
- `crates/mc-client/src/session/` — the drivable core, reachable with no window,
  no event loop and no GPU adapter.
- `E:\Temp\rdp-probe\rdp-mouse-probe-output.txt` — the source of FR-2.1-S4's
  committed fixture.

## Out of Scope

- **PRO-972** — water rendering as a magenta/black checkerboard because
  `base:water` is not a baked texture key. Separate issue, untouched.
- **Frame pacing** — PRO-971 merged on 2026-08-26 and the client now advances the
  world by elapsed time. Nothing here touches it.
- **Patching or upstreaming the winit flag fix.** Correct at the source and the
  heaviest thing to carry; it belongs in its own tracked issue.
- **A player-facing look-sensitivity setting.** The calibration above makes the
  absolute regime 15-19% faster than a one-count-per-pixel mouse on the measured
  display, and proportionally off on others, and there is no setting to trim it
  with. Recorded, not built — and tracked, because it is what makes the choice of
  calibration stop mattering.
- **A pointer-regime line in the debug overlay.** It would give the player a way
  to see which regime the client is in, and it would change the HUD goldens.
- **Reading the real display extent through `PointerPlatform`.** See § Notes.
- **Tablet, touchscreen and VM-guest pointers.** The same absolute regime reaches
  them and this fix should serve them, but none was measured here and no scenario
  claims them.

## Notes

- **The nominal scale was ruled on rather than assumed.** The alternative — one
  `PointerPlatform` method returning the real extent — is weighed in
  § Technical Considerations and declined because it rests on an unmeasured
  platform behaviour, not because of testability. What would reopen it is a
  measurement of what `primary_monitor()` reports inside an RDP session, which
  needs a human on RDP.
- **What no test in this spec can reach**, stated plainly so validation does not
  have to discover it: (a) winit producing an absolute packet — nothing in the
  workspace can make one, which is why the recorded run is committed as a
  fixture; (b) `Client`'s forwarding of a device event into
  `dispatch_device_event`, item 2 on `architecture.md`'s permanently-unreachable
  list; (c) whether the calibration *feels* right, which is a manual acceptance
  check over RDP and belongs in `docs/technical/testing.md`.
- **The `id:` field, corrected against the tree rather than against this note.**
  This spec was drafted as `SPEC-023` on the reading that the previous spec's
  frontmatter said `id: PRO-971` and so left the run at `SPEC-022`. **The
  frontmatter and the documentation disagreed**: PRO-971's merge, commit
  `5eaf438`, wrote `SPEC-023` into `docs/INDEX.md` four times, on the
  `architecture.md`, `testing.md` and `user/gameplay.md` rows, attributing the
  frame-pacing content to it. `SPEC-023` was therefore already spent, and this
  spec is **`SPEC-024`**.

  Recorded rather than quietly fixed, because the *shape* of the mistake is what
  transfers. `docs/INDEX.md`'s Sources column exists so a reader can trace a
  documentation claim back to the spec that made it, and two specs sharing one id
  destroys exactly that property **while the column still looks well-formed** —
  an identifier that no longer identifies is a checksum that fails its own
  purpose. It was found by grepping the tree, not by reasoning from the run, which
  is the only method that could have found it: a note derived from frontmatter
  cannot see what a completion wrote into `docs/`. The frontmatter/docs
  inconsistency in PRO-971 itself is filed separately and is not repaired here.

- **FR-5.1-S3's repair was written in the validation context, and the commit history does
  not show that.** Recording it rather than letting a reader reconstruct it, because what
  a reconstruction would produce is wrong: `c7b40ce` (test) and `26e9ba8` (repair) carry
  different intents and read like the ordinary author/implementer split, and that split
  did not happen. The scenario and the repair were both written by the agent that found
  the defect, on the conductor's explicit written authorisation, and were **re-verified
  independently afterwards** by an implementer that hand-traced the repair through
  `regime.rs` against both tests rather than inferring correctness from green.

  **The separation is worth recording because of what it caught, not as bookkeeping.** The
  independent trace is what established that a repair which reset the whole *reading*
  rather than only the anchor would also pass FR-5.1-S3 — and would silently swallow the
  first flick after every alt-tab on a local console, because the client would sit waiting
  for two samples to corroborate a stream it never left. FR-5.1-S3 cannot see that
  distinction: on the absolute path, "forget the anchor" and "reset everything" publish
  the same camera. `losing_the_window_leaves_an_ordinary_delta_spent_whole` is the test
  that can, and it exists because somebody who had not written the repair went looking for
  what it might break.

- **The defect was found by asking what stops a sample arriving, not by reading the
  scenarios.** FR-5.1's prose says "a capture the platform did not grant, or gave back",
  which is the capture ladder — the thing that has an event — and a window's focus has no
  bearing on it. The route was invisible from inside the scenario set for the same reason
  the original defect was invisible from inside the test suite: **the guard and the thing
  it guards against were freed by different mechanisms, and only one of them was
  enumerated.** The generalisable question is the one that closed it: `forget_position`
  had one caller, `on_pointer_motion` had one caller, and `capture` is assigned in exactly
  one place, so the anchor can only go stale when raw motion stops while capture still
  reads as held — and the only thing that stops raw motion is losing foreground. A window
  move and a session disconnect collapse into that one route. **A display-mode change and a
  monitor hotplug do not**: they stale the anchor by changing the *frame of reference* it was
  taken in rather than by interrupting the stream, so the window stays foreground, motion
  keeps arriving, `capture` is unchanged, and `WindowEvent::Resized` maps to
  `LoopAction::Resize` (`crates/mc-render/src/window.rs:156`), which never reaches the regime
  at all. That second mechanism is tracked as **PRO-984**. **Bounding the space beats
  enumerating candidates** — but the bound holds only over the delivery model it was drawn
  across, and staleness by a moved frame of reference sits outside that model rather than
  inside the route.
