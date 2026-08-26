---
id: PRO-971
title: Movement speed is the display's frame rate
status: implemented
work-type: fix
rigor: high
branch: feature/PRO-971-framerate-independent-speed
issue: PRO-971
created: 2026-08-26
approved: 2026-08-26
completed: 2026-08-26
author: Sebastian Grunow
---

# Fix: Movement speed is the display's frame rate

## Defect

- **Observed**: a human played a shipped build and reported *"Something
  completely broke movement — I just warp around with super speed."* The
  simulation advances exactly one tick per rendered frame, and a tick is a
  declared 1/60 s of simulated time, so the world runs at
  `frames_per_second / 60` times real speed. A walk is 4.5 blocks a second on a
  60 Hz display, 10.8 on a 144 Hz one, and unbounded where nothing paces the
  frame at all. Gravity, jump arcs, and every other quantity derived from
  `TICK_DURATION` scale by the same factor.
- **Expected**: the owner's words, which are the acceptance bar and were written
  down nowhere in this repository before this spec:

  > "A test could easily even without attaching to real time and using ticks
  > measure the speed at a certain framerate. Also: The speed should be the same
  > regardless of framerate."

  Movement speed is identical at every frame rate — not approximately — and that
  property is testable by driving frames at a *simulated* frame rate, with no
  wall clock and no sleep.
- **Reproduced**: not by an automated run, and the reason is the second half of
  this defect. `App::redraw` (`app/mod.rs:188`) has exactly one caller —
  `crates/mc-client/src/events.rs:205`, inside the `winit` loop — and **no
  in-process test can construct an `App`**, because doing so needs a real window
  and a real surface; every `App::` occurrence under `crates/mc-client/tests/` is
  a doc comment.

  **One subprocess harness does execute that path**, and it must not be
  overlooked: `crates/mc-client/tests/shipped_binary.rs` spawns
  `CARGO_BIN_EXE_mc-client` with a real window and device, and its fourth reading
  reaches `App::collect_preparation` inside a redraw. **It cannot be pointed at
  this defect**, for a reason that is about falsifiability rather than cost — see
  § Detection gap. So the property has no instrument, but the accurate statement
  is that it has no *observable*, not that the path is unreachable.

  The evidence is therefore the player report plus a mechanism traced end to end
  below, and the arithmetic that follows from it with no free parameters: a walk
  is `WALK_SPEED × TICK_DURATION` = 0.075 blocks per tick, `bounded` never binds
  at that size, one tick is spent per presented frame, so distance per second of
  wall time is `0.075 × frames_per_second`. At 144 frames a second that is 10.8
  blocks — 2.4×, which is what "warp around" describes.

## Root Cause

Three facts compose, each correct on its own:

1. **The frame is unpaced.** `crates/mc-client/src/events.rs:150-154` —
   `about_to_wait` calls `window.request_redraw()` on every loop iteration with
   nothing between it and the next frame. The rate is whatever the surface
   permits; `crates/mc-client/src/surface_setup.rs:87` takes
   `get_default_config(..)` and never names `present_mode`, so the cap is the
   display's refresh where the platform gives one and absent where it does not.
2. **One frame advances exactly one tick.** `crates/mc-client/src/app/mod.rs:280`
   — `session.tick()` in `App::present`, once per presented frame. It is the
   **only** production tick call site in the workspace; nothing else advances the
   simulation on the client, and `mc-server` has no tick loop yet.
3. **A tick is a declared quantum of simulated time, not a measured one.**
   `crates/mc-sim/src/player/physics.rs:26` —
   `const TICK_DURATION: f32 = 1.0 / 60.0;` — consumed at `:94`
   (`bounded(velocity * TICK_DURATION)`) and `:244`
   (`vertical - GRAVITY * TICK_DURATION`).

Fact 3 is the assumption; fact 2 is where it is violated. Simulated time is
declared at 60 ticks a second and delivered at the display's rate, and nothing
converts between the two. **The missing element is a conversion, not a bug in any
of the three.**

None of this is accidental or undocumented. `crates/mc-sim/CLAUDE.md` records it
as a decision — "The replay advances one tick per rendered frame" — and
`physics.rs:22-25` states the cost in as many words:

> Declared, never measured. The frame loop still drives ticks one to one, so a
> faster machine runs the world faster — a stated cost rather than a hidden one,
> and the day a pacing accumulator arrives it feeds this same fixed step.

**So the origin is not a defective change. It is a deferral whose bill came
due**, and the thing that turned a stated cost into a player-facing defect is
that a human ran the build on a display that is not 60 Hz. The decision was made
when the client drew a *scripted replay* with no player input, where "the world
runs faster" is a cosmetic property of a demo. It became a movement defect the
moment the player got the controls, and no one revisited it then.

## Regression Scenarios

### FR-1 — Movement speed does not depend on the frame rate

- **FR-1.1**: The distance a player walks depends on elapsed time and not on how
  many frames that time was delivered in.
  - FR-1.1-S1: WHEN a player walking at full deflection is advanced through the
    same total elapsed time once at a simulated 144 frames a second and once at a
    simulated 60 frames a second THE SYSTEM SHALL leave the player the same
    distance from where the walk began in both runs.
  - FR-1.1-S2: WHEN the same total elapsed time is delivered at a simulated 1000
    frames a second THE SYSTEM SHALL leave the player the same distance from
    where the walk began as at a simulated 60 frames a second.
  - FR-1.1-S3: WHEN a frame delivers exactly three tick quanta of elapsed time
    THE SYSTEM SHALL advance the player by exactly three ticks' walk.
  - FR-1.1-S4: IF a frame delivers less elapsed time than one tick quantum THEN
    THE SYSTEM SHALL leave the player where it was.
  - FR-1.1-S5: WHEN two consecutive frames each deliver half a tick quantum THE
    SYSTEM SHALL advance the player by exactly one tick's walk.

### FR-2 — A frame that spends several ticks spends each input once

- **FR-2.1**: Input the player is *still making* applies to every tick a frame
  spends; input they made *once* is spent once, whatever the frame cost.
  - FR-2.1-S1: WHEN a frame that spends three ticks carries a pointer motion of
    10 device counts THE SYSTEM SHALL turn the view by that motion once and not
    three times.
  - FR-2.1-S2: WHEN a frame that spends three ticks carries one break request THE
    SYSTEM SHALL break one voxel.
  - FR-2.1-S3: WHILE the walk key is held THE SYSTEM SHALL apply the walk to every
    tick a frame spends.
- **FR-2.2**: A frame that spends several ticks loses nothing a tick produced.
  Added during validate, from a confirmed Blocker: the frame path reads what a
  tick answered about the content root **once**, after every tick of the frame,
  so a later tick answering "nothing changed" must not read as "the answer is
  withdrawn".
  - FR-2.2-S1: WHEN a frame that spends three ticks carries a content candidate
    one of its ticks accepted THE SYSTEM SHALL report that acceptance to the
    frame path.
  - FR-2.2-S2: WHEN a frame that spends three ticks carries a content candidate
    one of its ticks refused THE SYSTEM SHALL report that refusal to the frame
    path.

### FR-3 — A pathological frame gap is bounded

- **FR-3.1**: Catch-up after a stall is bounded, and the bound is above every
  frame interval a working machine produces.
  - FR-3.1-S1: IF a frame reports 10 seconds of elapsed time THEN THE SYSTEM
    SHALL advance the player by at most the catch-up bound's worth of ticks.
  - FR-3.1-S2: IF a frame reports 10 seconds of elapsed time THEN THE SYSTEM
    SHALL carry none of the discarded surplus into the frames that follow, so a
    frame delivering one tick quantum after it advances by exactly one tick.
  - FR-3.1-S3: WHILE frames arrive 100 milliseconds apart THE SYSTEM SHALL leave
    the player the same distance from where the walk began as the same total
    elapsed time delivered at a simulated 60 frames a second.

### FR-4 — The frame path is what consults the pacing

- **FR-4.1**: The client's frame path advances the simulation by an elapsed
  duration and has no way to advance it by a fixed number of ticks.
  - FR-4.1-S1: THE SYSTEM SHALL report that no source under the client's frame
    path names a per-tick advance, as an enumerated verdict rather than an empty
    list of offences.
  - FR-4.1-S2: WHEN the same scan is pointed at a fixture frame path that
    advances a fixed number of ticks per frame THE SYSTEM SHALL report that
    fixture.
  - FR-4.1-S3: WHEN a frame is timed THE SYSTEM SHALL pace the simulation from
    the same elapsed reading the debug overlay's frame rate is computed from, so
    the rate the overlay reports and the time the simulation spent cannot
    disagree.

### FR-5 — Pacing introduces no nondeterminism

- **FR-5.1**: The pacing carries state, and state is a new way to break replay.
  - FR-5.1-S1: WHEN the same sequence of frame durations is delivered twice from
    the same starting state THE SYSTEM SHALL leave the player in the same place
    both times.
  - FR-5.1-S2: THE SYSTEM SHALL name no wall clock in any client or renderer
    source outside the one exempt file.

## RCA

### Causal chain

| Link | Where |
|---|---|
| Symptom: the player warps at high speed | Player report, shipped build |
| Every simulated quantity advances `fps/60` times too fast | `physics.rs:94`, `:244` consume a declared `TICK_DURATION` |
| Simulated time is delivered at the display's rate | `app/mod.rs:280` — one `session.tick()` per presented frame |
| The frame rate is the display's, unpaced | `events.rs:150-154` unconditional redraw request; `surface_setup.rs:87` never names `present_mode` |
| Origin: a recorded deferral, not a regression | `crates/mc-sim/CLAUDE.md` ("advances one tick per rendered frame"); `physics.rs:22-25` ("the day a pacing accumulator arrives") |

The origin link is worth stating precisely because it is unusual: **no commit
introduced this.** It was correct for a scripted replay with no player, was
written down as a known cost, and became a defect when player control landed
without anyone re-reading the note that predicted it.

### Detection gap

Three holes, and they are nested — each one would have hidden the defect on its
own.

1. **The frame path is unreachable from any *in-process* test, and the one
   harness that does reach it cannot observe pacing.** `App::redraw` has one
   caller, `events.rs:205`, and constructing an `App` needs a window and a
   `wgpu::Surface`; no in-process test constructs one.
   `crates/mc-client/tests/shipped_binary.rs` reaches it through a real
   subprocess, and that route was considered and rejected on three grounds, the
   first of which is decisive:

   - **At 60 frames a second the defect and the fix are indistinguishable by any
     wall-clock observation of the child.** Broken, ticks = frames = `f · T`;
     fixed, ticks = `60 · T`. These are equal exactly when `f = 60`. The surface
     takes wgpu's default `Fifo` present mode (`surface_setup.rs:87`), so `f` *is*
     the display refresh — meaning such a test is blind on a 60 Hz panel, fails
     for unrelated reasons on a headless machine (the harness's own
     `NeverGotAsFarAs…` failing verdict), and discriminates only on a panel that
     happens to be faster. **Green on the most likely configurations and red on
     another is flaky by hardware**, and it reads as evidence while being none —
     `testing.md` §2. Adapting or skipping on the observed rate is worse still:
     that is *an absent reviewer and a clean reviewer look identical*.
   - **The child emits no observable for this.** The existing readings work
     because they watch lines the product already says for a player's benefit; a
     pacing reading has none, and the client's whole argument surface is one flag
     (`REFUSE_CHANGED_BLOCKS`, `startup.rs:51`). Adding output or a test-only
     argument is a product change whose only consumer is a test. There is also no
     back door through the save: the child receives no input, so its player never
     moves and there is nothing time-dependent to write down.
   - It would cost seconds of wall time per reading, against a `PATIENCE` budget
     already set at 20 s.

   The entire ratio "one tick per frame" therefore lives in code that is
   *executable* but whose pacing is *unobservable*, which is a sharper hole than
   an unreachable one and a good deal easier to miss.
2. **`mc-client` is wholly excluded from the coverage denominator** (ADR-013,
   `docs/technical/decisions.md:427`, `$CoverageExclude` in
   `scripts/sdd-gate.ps1`). The exclusion is honest — the crate is meant to hold
   no policy — but "one tick per frame" *is* policy, and it was sitting in the
   excluded crate. This is the exact shape `testing.md` §2 names: *a handler
   needing a real window, socket or device that nothing constructs, sitting in a
   layer coverage is configured not to count.*
3. **Every test drives the tick directly.** All 20-odd tick call sites under
   `crates/mc-client/tests/` call `session.tick()` or `simulation.advance(..)`.
   A test driving the tick cannot observe how often the *frame* drives it — this
   is `testing.md` §2's *policy is not wiring* with the roles reversed: here it is
   not that the adapter stopped consulting the policy, it is that the adapter's
   own conversion was never a policy anything could consult.

**So the property was invisible by construction, not by oversight, and the suite
size is no comfort: every test in the workspace passes over this defect.** The
gate could not see it either — a lint has nothing to say about a ratio.

### Sibling sweep

- **Other per-frame work in `App::present`** — `collect_preparation`,
  `exchange_remesh`, `submit_remesh`, `take_up_reloaded_content`. Each is
  "consume whatever is ready", which is correctly frame-paced rather than
  time-paced. **No siblings.** At a higher frame rate the re-mesh worker is
  merely asked more often, which is benign.
- **`mc-server`** holds only `main.rs` and has no tick loop, so the defect cannot
  exist there yet. When it grows one it will be time-paced from its own clock and
  this client-side bound does not transfer — recorded in Notes.
- **The wall clock** is already confined to one file
  (`crates/mc-render/src/overlay/clock.rs`), guarded by
  `crates/mc-client/tests/wall_clock_confinement.rs` over roots
  `crates/mc-client/src` and `crates/mc-render/src`. There is no second clock to
  find.

**One finding from the sweep changes the risk profile of the fix**, and it runs
in the fix's favour. The obvious hazard in spending several ticks in one frame is
that input gets spent several times — a look delta applied three times is a 3×
sensitivity spike, a click applied three times breaks three voxels. Both drains
are *already* written for this: `crates/mc-sim/src/player/input.rs:134-145`
drains `yaw_delta`/`pitch_delta` and keeps the held keys, with a doc comment
naming that distinction as its whole purpose, and `Session::tick`
(`session/mod.rs:326`) `take()`s the pending action. A loop over ticks inherits
the right semantics without changing either. FR-2 exists to *hold* that, because
it is currently true by accident of a design made for one tick per frame.

### Prevention

The class is **a conversion living in a layer nothing can execute**. Three checks,
in decreasing strength:

1. **Make the defect unspellable.** The per-tick step stops being the frame path's
   door; the only advance the client's frame path can name takes an elapsed
   duration. A reversion to one-tick-per-frame then fails to compile rather than
   failing a test. This is the project's own idiom — the re-mesh serial is held
   the same way (`mc-sim/CLAUDE.md`: *"unspellable rather than checked"*).
2. **A structural scan with a positive control** (FR-4.1-S1, FR-4.1-S2) for the
   reversion unspellability cannot catch: a frame path that calls the duration
   door with a *constant* duration every frame. Asserted as an enumerated verdict
   per `testing.md` §2, so a scan that could no longer look cannot answer under
   the clean verdict's name.

   **This is the weakest of the three and it is the residual hole**, stated
   plainly rather than dressed up: a scan reads text, and text is not execution.
   The subprocess harness is the only instrument that could execute the wiring
   instead of reading it, and § Detection gap records why it cannot be pointed
   here. A documented residual hole is worth more than a test that appears to
   close it and is blind on a 60 Hz panel.
3. **Move the pacing decision into the drivable core**, where a fake clock the
   test owns can drive N frames at a simulated rate. That is what makes FR-1
   writable at all, and it is precisely what the owner asked for.

A project-wide check — "no ratio between two rates may live in an
coverage-excluded crate" — is not proposable as a mechanical rule, so it is not
proposed. What generalizes instead is a question for review, recorded in Notes:
**when a spec puts a decision in `mc-client`, ask what executes it.**

## Technical Considerations

### Proposed shape

A **fixed-timestep accumulator in the drivable core, fed an injected elapsed
duration.** Concretely, and as a proposal the implement phase may revise with
evidence:

- The client's drivable core (`crates/mc-client/src/session/`) gains one frame
  door taking the elapsed-time port. It reads the clock **once**, derives this
  frame's interval, gives that interval to the overlay's ring (which already
  computes exactly this quantity, `overlay/state.rs:119`) and to the pacing
  state, and spends whole tick quanta out of it by calling the existing per-tick
  step once per quantum.
- `App::present` replaces its two calls — `session.record_frame_time(&clock)` and
  `session.tick()` — with that one. The clock it passes is the adapter **the
  `App` already holds** (`app/mod.rs:102`, `:162`).
- The per-tick step stops being the frame path's public door.

### The port is renamed for its capability

Once this trait paces the simulation it is a frame clock, not an overlay clock.
`code-quality.md` §3 requires ports be named for the capability, and
`crates/mc-render/CLAUDE.md` says that crate "does not simulate" — a port
carrying pacing should not be owned by the renderer's *overlay* subsystem by name
or by module path. A reader asking "what paces the simulation?" will not look
under `mc_render::overlay::clock`.

**`OverlayClock` → `FrameClock`, `SystemOverlayClock` → `SystemFrameClock`, moved
out of `overlay/` to `crates/mc-render/src/time/clock.rs`.** Thirteen references
across five files. The port and its one adapter stay in **one** file, which that
file's own header argues for: splitting the adapter out would need a second
exemption in the confinement scan, and needing a second exemption is what the
guard exists to avoid. The scan's `EXEMPT_FILE` constant changes path; the number
of exemptions does not change.

This is not scope sprawl: the fix creates a new public door on `Session`, and
naming the port that door takes is part of building the door. **The relocation
does not go further than the rename, and the reason is recorded** — moving the
port to `mc-core` would be a better end state, since `mc-core` lies under neither
of the confinement scan's roots and the exemption could then be *deleted* rather
than repathed, which is strictly stronger than any one-file exemption. It is also
a crate-boundary change that rewrites the scan's roots, its positive control and
its vacuity guard. That is architecture, not this fix. Recorded in Notes.

**A correction worth keeping, because it inverts an easy misreading**:
`crates/mc-client/src/clock.rs` is **not** an exempt path awaiting a file. It is
the *offending* file in the scan's positive control
(`wall_clock_confinement.rs:164-172`), chosen because — in that file's own words
at `:39` — "the client grew a clock of its own is exactly the change this guard
exists to report". Putting the port there would be the precise offence the guard
was built to catch.

### Vocabulary: the pacing state is not called an accumulator

`session/mod.rs` already uses "accumulator" for the **input** accumulator, in the
doc comments at `:337` and `:342`. A time accumulator in the same module would
make the word mean two things four lines apart. The pacing state is the
**carried** or **unspent** frame time, and the operation on it is *spending whole
quanta*. "Accumulator" in that module continues to mean the input one.

### Why this shape and not a clock inside the tick

- **The goldens must stay reproducible.** `mc-sim/CLAUDE.md` is categorical: no
  wall clock in the simulation, because a wall clock is the easiest way to make a
  golden frame unreproducible. The capture harness drives ticks by hand and must
  go on doing so. Injecting an elapsed duration into the *client* and leaving the
  tick a declared quantum satisfies both: `TICK_DURATION` keeps its meaning
  unchanged, and this is the "pacing accumulator" `physics.rs:22-25` explicitly
  anticipated feeding it.
- **One reading, not two.** Timing the frame and pacing the frame want the same
  interval. Taking it twice would let the overlay report 144 fps while the
  simulation spent a different amount of time; taking it once makes that
  unspellable (FR-4.1-S3), and it removes a public door rather than adding one.
  **This argument is the whole case for reusing the existing port**, and it stands
  alone.
- **No new wall clock is named, and that is a consequence rather than a reason.**
  `crates/mc-client/tests/wall_clock_confinement.rs` refuses `Instant` and
  `SystemTime` under `crates/mc-client/src` and `crates/mc-render/src` outside one
  exempt file, and reusing an existing adapter means the exemption count stays at
  one (FR-5.1-S2 pins it). Recorded because it is true and worth knowing — **not
  offered as a design argument.** Keeping a structural test's exemption untouched
  is a convenience, and a convenience must not decide where a port lives; that is
  why the rename above proceeds despite costing an edit to that constant.
- **The seam guard already constrains where the pacing may live.**
  `crates/mc-client/tests/seam_boundaries.rs:184-196` forbids `.advance(`,
  `take_intent`, `TickIntent` and `pending_action` anywhere under
  `crates/mc-client/src` except `src/session/mod.rs`. The pacing must therefore
  live in the core, which is also the only place a test can drive it. The
  constraint and the requirement point the same way.

### The tolerance, derived from both directions

`testing.md` §2 forbids loosening until green and equally forbids an assertion so
tight it reddens a correct implementation. Both bounds are computable here:

- **The floor.** With a `Duration` accumulator the residue below one quantum is
  carried, so two partitions of the *same* total elapsed time spend tick counts
  differing only by nanosecond rounding across the partition boundaries — at most
  ~1 µs of simulated time over a one-second drive, against a quantum of 16.67 ms.
  The tick counts are therefore **equal**, and equal tick counts give a
  **bit-identical** position. Exact equality is the correct assertion.
- **The ceiling.** The smallest difference the test must still catch is one tick
  of walk, 0.075 blocks; the defect's own difference is 6.3 blocks. Exact equality
  catches both.
- **The one way exact equality could red a correct implementation** is a total
  elapsed time landing within nanoseconds of a quantum boundary, where the two
  partitions could straddle it. **The test must therefore choose a total that sits
  away from a boundary**, and this is a constraint on fixture construction that no
  assertion can enforce — `testing.md` §2's *a count cannot see shape*. A total of
  100 frames at a simulated 144 Hz is 41.67 quanta, a third of a quantum (5.5 ms)
  clear of the nearest boundary. The test author records the chosen total and its
  margin in `test-map.md`.

### The frame-gap bound

**Proposal: clamp a frame's elapsed time to 250 ms — 15 ticks — before
accumulating, and discard the surplus rather than carrying it.**

- **Floor.** The bound must exceed every frame interval a *working* machine
  produces, or a slow machine systematically loses simulated time and runs the
  world slow — the original defect with the sign flipped. Ten frames a second is
  the lowest rate at which the game is arguably being played rather than hung, and
  that is a 100 ms interval. 250 ms carries 2.5× headroom over it. FR-3.1-S3 is
  what makes this half falsifiable: without it, a bound of one tick passes
  FR-3.1-S1 and FR-3.1-S2 and the world would crawl on any machine below 60 fps.
- **Ceiling.** 15 ticks is bounded work — the tick is arithmetic over a small
  voxel neighbourhood — so a frame that hits the cap costs a fraction of a frame
  budget and cannot spiral.
- **Discard, not carry.** A debugger pause, a laptop resuming from sleep, or a
  breakpoint accumulates unbounded elapsed time. Carrying it would replay it: a
  hang bounded only by how long the machine was asleep, which is the failure this
  bound exists to prevent. A single-player client losing that time is the correct
  answer — the player was not playing. **When `mc-server` grows a tick loop this
  does not transfer**: a server's own clock is authoritative and a client's stall
  must not stall the world (invariant 4).
- 250 ms is also the conventional value in the fixed-timestep literature. That is
  corroboration, not the derivation — the derivation is the 10 fps floor.

### Invariants

- **Invariant 4 (the server is authoritative)** is untouched. This changes *how
  often* the client advances its own simulation, not what it is permitted to
  claim. `requested_walk`'s receiving-side clamp (`physics.rs:150-160`) is unchanged,
  and nothing here lets a client ask for a longer tick — the elapsed duration is
  read from the client's own clock and bounded before use.
- **Invariant 2 (state in Rust, behaviour in script)** is untouched: the
  accumulator is engine state in the ECS-side core, and no game rule moves into
  or out of Luau.
- **Invariant 5 (verification precedes the thing it verifies)** is what FR-4
  serves: the instrument that can see the frame path's pacing lands with the fix.

## Existing Code to Leverage

| What | Location | Reuse |
|------|----------|-------|
| An injected monotonic clock port, already held by `App` | `crates/mc-render/src/overlay/clock.rs`; `app/mod.rs:102` | The elapsed source, renamed for its capability and moved out of `overlay/`. |
| The subprocess harness that executes `App::redraw` | `crates/mc-client/tests/shipped_binary.rs` | Considered for the wiring and rejected; the reason is in § Detection gap. Not reused. |
| Per-frame interval derivation, saturating and monotonic | `crates/mc-render/src/overlay/state.rs:119` | The same reading feeds pacing; one door instead of two. |
| Drain-vs-hold input semantics | `crates/mc-sim/src/player/input.rs:134-145` | Already correct for a multi-tick frame; FR-2 holds it. |
| One-shot action drain | `crates/mc-client/src/session/mod.rs:326` | Same. |
| Structural scan with a positive control and an enumerated verdict | `crates/mc-client/tests/wall_clock_confinement.rs`, `tests/seam_boundaries.rs` | The pattern FR-4.1-S1/S2 follow. |
| The declared quantum | `crates/mc-sim/src/player/physics.rs:26` | Unchanged in value and meaning; the accumulator feeds it. |

## Out of Scope

Binding. Recorded, not built.

- **PRO-972** — water renders as a magenta/black checkerboard because
  `base:water` is not a baked texture key. Separately tracked. Found in the same
  play session; not touched here.
- **PRO-962** — mouse look unusable over Remote Desktop. Separately tracked.
- **Naming `present_mode` on the surface.** `surface_setup.rs:87` is cited above
  as context for *why* the frame rate is the display's, not as a thing this fix
  changes. Pinning a present mode is a rendering decision with its own trade-offs
  (latency against tearing) and it does not fix this defect — a vsynced 144 Hz
  display still runs 2.4× fast. It would only mask the defect on some machines,
  which is worse than leaving it visible.
- **Capping the frame rate.** Same reasoning: a frame cap is a power and latency
  decision, not a correctness one, and it does not make speed independent of the
  rate.
- **Interpolating the rendered position between ticks.** The natural companion to
  a fixed timestep, and a real improvement to how the motion *looks* at rates
  above 60 Hz. It is a rendering feature with its own scenarios and its own
  golden-frame consequences, and it is not needed for speed to be correct.
- **A tick loop for `mc-server`.** Does not exist yet.

## Notes

Deferred observations, recorded per the scope guard.

- The magenta water and the RDP look defect were both visible in the same play
  session and both are tempting one-line fixes. Neither is touched.
- `docs/technical/architecture.md` and `crates/mc-sim/CLAUDE.md` both state "one
  tick per rendered frame" as standing fact. Both become wrong the moment this
  lands and must be updated as part of this spec's definition of done
  (CLAUDE.md Key Principle 4), not as a follow-up.
- A generalizable review question, offered rather than proposed as a mechanical
  gate check: **when a spec puts a decision in `mc-client`, ask what executes
  it.** The crate is excluded from coverage on the premise that it holds no
  policy; that premise is load-bearing and nothing currently checks it.
- When `mc-server` grows a tick loop, the 250 ms client bound does not transfer.
  A server paces from its own clock and a client's stall must not stall the world.
- **Moving the frame-clock port to `mc-core` is the better end state and is not
  done here.** `mc-core` lies under neither of the confinement scan's roots, so
  the adapter would need no exemption at all and the scan's exemption list could
  become *empty* — strictly stronger than a one-file exemption, because an empty
  list has nothing to grow from. It is a crate-boundary change that rewrites the
  scan's roots, its positive control and its vacuity guard, which is architecture
  rather than this fix. Worth a decision spec.
- **The rename is part of the fix, not cleanup beyond it — read this before
  obeying the implement fragment literally.** `.prospect/prompts/fix/implement.md:8-9`
  instructs the implementer: *"Preserve unrelated behavior; no cleanup, renaming,
  or abstraction beyond the fix."* Taken alone that reads as forbidding the
  `OverlayClock` → `FrameClock` rename, and both wrong resolutions are plausible —
  refusing it as out-of-bounds, or performing it while believing it a violation
  and reporting it as one. **Neither is right.** The fix creates a new public door
  on `Session`, and naming the port that door takes is *building* the door. The
  fragment's prohibition is aimed at drive-by tidying of code the fix merely
  passes by; it does not reach the interface the fix is introducing. Perform the
  rename, and do not report it as a scope violation.

  The same fragment has no test-author/implementer split on the `fix` path — step
  1 writes the failing regression tests and step 2 the fix, one owner for both. So
  repathing `EXEMPT_FILE` in `wall_clock_confinement.rs` raises no arbitration
  question.
- **A residual hole is accepted knowingly**: FR-4.1-S1/S2 read text where
  execution would be better, and the one harness that executes the frame path
  cannot observe pacing (§ Detection gap). If the client ever grows a reason of
  its own — a player-facing one — to report simulated time or position on a
  stream, that reading becomes available and this hole should be revisited.

### Recorded during implement

Observations the implement phase made, none of them built beyond what the
scenarios asked for.

- **The residual hole is now measured rather than argued.** Deleting
  `session.advance_frame(&self.frame_clock)` from `App::present` — the client's
  only call, and so the entire wiring — leaves **383 of 383** `mc-client` tests
  green. Recorded in `test-map.md` as mutation M3, with the invocation.
- **The spec's tolerance derivation was too pessimistic, and the conclusion
  survives.** § "The tolerance, derived from both directions" reasoned to a
  residue of ~1 µs across a partition boundary and asked the fixture to choose a
  total clear of a quantum boundary. With a `Duration` accumulator the residue is
  **exactly zero**, so equal totals give equal tick counts however the time is cut
  into frames and the boundary cannot be straddled. Exact equality remains right,
  for a stronger reason. `test-map.md` records the margin anyway.
- **`docs/technical/testing.md` was a third document claiming one tick per
  rendered frame**, alongside the two this spec's Notes named. Updated with them.
- **Prevention 1 cost a harness change, and it paid for itself.** Making
  `Session::tick` private is what turns a reversion into a compile error, and the
  only other caller was the test harness. `InputHarness` now drives frames through
  `advance_frame` with a clock the suite owns, so all ~380 client scenarios reach
  the simulation through the door the product uses — which closes the third of the
  three detection holes (§ Detection gap 3) as a side effect rather than as extra
  scope.
- **`DebugOverlay` stopped holding the previous clock reading.** § Proposed shape
  says the session derives the interval and gives it to the overlay's ring; doing
  that literally means the ring can no longer read a clock itself, so
  `record_frame_time(&impl Clock)` became `record_frame(Duration)` and the
  previous reading moved into `session/pacing.rs`. A small `mc-render` API change
  the spec did not list, and the thing that makes "one reading, not two" true by
  construction rather than by discipline.
- **A frame yields at most one `EditReport` however many ticks it spends**, so the
  frame door returns `Option<EditReport>` losslessly: the first tick takes the
  pending action and the rest carry none. This is the same drain FR-2.1-S2 pins.
- **`overlay_over_content.rs` refuses to let a frame-recording suite name
  `OverlayReadout`**, and it correctly reported FR-4.1-S3's new test file. The
  exemption list stayed at four: `InputHarness` grew a forwarder answering with
  the frame rate as an `f64`. Not in § Existing Code to Leverage, and worth
  knowing before writing any further test that reads the overlay.
- **The gate reported three things a green suite could not**, all of them arriving
  after a clean `cargo clippy` run earlier in the phase: `clippy::integer_division`
  on the quantum arithmetic (this repository's standing answer is a shift, which
  needs a power of two — a sixtieth of a second is not one, so whole quanta are
  subtracted one at a time and a float division is refused because it rounds), a
  public doc comment linking the now-private `Session::tick`, and
  `session/mod.rs` 32 lines over the 500-line limit. `testing.md` §2's *a green
  suite is no evidence about a lint*, live.
- **`session/mod.rs` was split by responsibility, not by convenience.** The
  pointer port and its ask vocabulary to `session/pointer.rs` — a port is not a
  decision and needs nothing the session owns, which is why the keyboard
  vocabulary already had a file — and the ending a run reports after saving to
  `session/quit.rs`. Both re-exported; no path outside the module changed.
- **The harness's `tick()` and `ticks(n)` now mean "a frame of exactly one
  quantum".** The names still read well and were left alone. A later phase that
  wants them to say `frame`/`frames` should rename them together with the ~380
  call sites, which is not this fix's business.
- **Moving the port to `mc-core` is still the better end state and is still not
  done here**, unchanged from the Notes above: the exemption count stayed at one
  rather than becoming zero.

### Recorded during validate

Pass 1 found one Blocker. It is fixed here rather than deferred, with its own
scenarios (FR-2.2) and its own regression tests.

- **The defect: a multi-tick frame destroyed the reload report before the frame
  path read it.** `cross_reload_boundary` wrote `self.reload_report`
  unconditionally, mapping `ReloadStep::Nothing` to `None`. Its only caller is
  the per-tick step, which this fix changed from once a frame to up to fifteen
  times, while the only consumer runs once a frame after the whole loop. The tick
  that answered was followed by ticks answering `Nothing`, and each of those
  cleared the answer. Below thirty frames a second — the class this fix exists to
  serve — an accepted candidate swapped the simulation's content while the frame
  path never uploaded its layers, never retired the re-mesh worker's and never
  said a word to the author. **`ReloadStep::Nothing` now leaves the last answer
  standing**, which is the narrowest repair and covers both producer arms.
- **The audit that found it was of a class, not of a line.** Every effect
  reachable from the per-tick step was classified as idempotent under repetition
  or not. The complete list checked, so a reader can tell a complete audit from a
  lucky one — **the result is the third column, and it is twelve yeses of
  thirteen**:

  | Effect | Where | Idempotent under repetition, and why |
  |---|---|---|
  | The pending action | `session/mod.rs:323` | yes — `take()`n by the first tick, the rest carry none; nothing re-arms it inside the loop, because no event dispatch runs during a frame |
  | Pointer look delta | `mc-sim/src/player/input.rs:142-143` | yes — drained to zero, the rest see nothing |
  | Held keys | `input.rs:136-140` | yes, deliberately — a held key *is* asking for every tick |
  | Player advance | `mc-sim/src/simulation.rs:235` | yes — per-tick motion is the point |
  | The published tick counter | `simulation.rs:240-244` | yes — one step per tick is the point |
  | The world edit | `simulation.rs:236-239` | yes — gated on `intent.action.is_some()`, so at most once a frame |
  | The returned `EditReport` | `session/mod.rs:446-448` | yes — `Some` only when the action was, so `.last()` is lossless |
  | The watch's change queue | `mc-sim/src/reload/mod.rs:181` | yes — drained per call, and `pending` accumulates what it drained |
  | `ContentReload::pending` / `in_flight` | `reload/mod.rs:184`, `:221-228` | yes — cleared before the spawn, and the spawn is guarded on `in_flight.is_some()`, so fifteen ticks start at most one build |
  | `ContentReload::reported` | `reload/mod.rs:242` | yes — a refusal dedupe, latest-wins |
  | Sections needing a re-mesh | taken by `take_remesh_work` | yes — accumulates rather than latest-wins |
  | The block the client holds | `session/reload.rs:150` | yes — latest-wins, and only a swap writes it |
  | **The reload report** | **`session/reload.rs:147`, `:152`** | **no — overwritten with `None` by any later tick** |

  **The implement phase reasoned about this exact hazard correctly for one of the
  two things a tick emits and did not carry the reasoning to the other** —
  `advance_frame`'s own doc comment argues `.last()` is lossless and says nothing
  about the reload report four lines away. That is what an unsystematic audit
  looks like from the inside, which is why the list above is written down rather
  than the conclusion alone.
- **§ Sibling sweep checked consumers and not producers, and that is the general
  lesson.** It surveyed `take_up_reloaded_content` and classified it as correctly
  frame-paced — true of the *consumer*, and it is the *producer* on the other end
  of the field that changed cadence. A sweep that asks "is this consumer paced
  right?" without asking "what writes what it reads, and how often now?" will miss
  this class every time.
- **One shape probed and found unreachable, recorded because a future change could
  open it.** Two non-`Nothing` steps inside one frame would silently drop the
  earlier one, since the field is latest-wins and read once — an `Accepted`
  followed by a `Refused` would reproduce the Blocker's symptom exactly. It cannot
  happen today: the second step needs a second build to *finish* between two ticks
  microseconds apart, while `begin_a_build` spawns the thread at the end of the
  first, and the unwatchable arm fires once and only before any build exists. A
  change to `at_tick_boundary` that made two answers per frame reachable would need
  to revisit this.
- **Two producer arms feed the one field, and the refusal arm is the quieter
  loss.** An acceptance is destroyed because `collected` takes `in_flight`, so
  the next tick answers `Nothing`. A refusal is destroyed the same way, and the
  unwatchable-root refusal doubly so: the shipped watch reports it **once**
  (`unwatchable.take()`), so the very next tick of the same frame answers
  `Nothing`. Both arms have a scenario.
- **The harness's `tick()` name is masking which regime a scenario tests, and
  that is why this slipped.** `tick()` means "a frame of exactly one quantum", so
  the whole client suite still exercises the one-tick-a-frame case and the
  multi-tick path is reached by four files. The implement phase deferred renaming
  `tick()`/`ticks(n)` across ~380 call sites as cosmetic. **It is not cosmetic** —
  it is the reason a reader cannot see, at a call site, which regime a scenario
  is in, and this Blocker is the first thing that hid behind it. Recorded for a
  follow-up issue; the rename is still not attempted inside this spec.
- **Two observations recorded and not built, both about names and neither worth
  code.**

  **The overlay's frame-rate ring now remembers its first reading, and the reason
  it used to skip one is no longer written anywhere near it.** The removed field
  doc argued the point exactly — "one reading is not a frame time … the interval
  it would have to be measured against is the moment the clock started" — and
  `record_frame(took)` records that interval now, so the ring's first entry is
  App-construction-to-first-present rather than an inter-frame gap. It is a
  deliberate reversal and it *is* reasoned, at `session/pacing.rs:8-16` — but that
  passage argues about *pacing*, where the first frame genuinely did take that
  long, and nothing where the ring lives says the same thing about a frame *rate*.
  The effect is bounded: one entry of sixty, evicted after `FRAMES_REMEMBERED`
  frames, on a debug overlay. Worth a sentence beside the ring the next time that
  file is opened, not a change now.

  **`record_frame` names two different calls in the client.** `app/mod.rs:343` is
  the renderer's HUD recording; `session/mod.rs:442` is the overlay's frame-rate
  ring, new here. `crates/mc-client/tests/hud_entry_point.rs:23` says "the one call
  the client is allowed to make is spelled `record_frame`", which now reads
  ambiguously. **The guard itself is unaffected** — `record_frame` is not among its
  needles (`hud_entry_point.rs:80-86`), so nothing was weakened; what changed is
  only how its prose reads. Renaming either call is a rename of a `mc-render`
  public method and belongs to whoever next touches that surface.
- **A third instruction-versus-fragment collision on this one spec, and it is a
  Prospect finding rather than a note about any agent.** The first was
  `.prospect/prompts/fix/implement.md:8-9` — "no cleanup, renaming, or abstraction
  beyond the fix" — against a port rename the fix required, resolved in the Notes
  above. The second and third are in validate: `fix/validate.md` step 3 has the
  validating context *fix findings and re-verify them*, while the phase was also
  directed to hand pass 2 to a fresh agent, and separately its ≤60-line report
  budget collides with `testing.md` §2's requirement that mutation invocations be
  written down. **Each time, the resolution was to take the stricter of the two and
  say which was taken** — a fresh reviewer is more than the fragment asks for, not
  less, and evidence was not deleted to reach a line count. Recording the pattern
  because three collisions on one spec is a fragment problem: an agent that
  resolved one of them the other way would be following the fragment literally and
  would look, from the record, like it had chosen to.
- **A reviewer the fixing context commissioned is supporting evidence and cannot be
  the record.** An `sdd-reviewer` was run from this context over the fix under the
  re-review rules and returned zero findings at Major or above, re-deriving the
  staleness argument from `app/mod.rs` rather than grading the claim, and finding
  the unreachable two-answers-in-one-frame variant recorded above. It is named here
  and deliberately kept out of `validation-report.md`'s verdict: the brief was
  written by the author of the fix, and "the fixer briefed its own reviewer" is not
  a sentence this spec's record should have to carry given that its whole history
  is defects that passed green suites.
