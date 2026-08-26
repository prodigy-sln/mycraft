# Validation — PRO-971 (fix, rigor high)

**Pass 1: FAIL** — 1 Blocker (confirmed by mutation), 0 Major, 1 Minor, 2 Info.
The Blocker and the Minor are fixed here, the Blocker with its own scenarios
(FR-2.2) and its own regression tests.

**Pass 2 is not recorded in this report, and the reason is a framework collision
rather than anybody's deviation.** `.prospect/prompts/fix/validate.md` step 3 has
the validating context fix findings and re-verify them; the conductor ruled that
pass 2 be run by a fresh agent instead. The two disagree, **the stricter was
taken**, and pass 2 belongs to that fresh agent's record and not to this file. A
reviewer briefed by the author of a fix is supporting evidence and cannot be the
record — which is why the one commissioned from this context is named in the spec's
Notes and kept out of the verdict here.

## Pass 1 — the Blocker

`session/reload.rs` assigned `self.reload_report` unconditionally, mapping
`ReloadStep::Nothing` to `None`. Its only caller `Session::tick`
(`session/mod.rs:327`) went from once a frame to up to fifteen times
(`session/mod.rs:446-448`), while the only consumer runs once a frame after the
whole loop (`app/mod.rs:290-292` → `app/reload.rs:39`, a `take()`).
`ContentReload::collected` takes `in_flight` (`crates/mc-sim/src/reload/mod.rs:198`),
so every tick after the collecting one answered `Nothing` and cleared the report.

Below 30 fps — the class this fix exists to serve — an accepted candidate swapped
the simulation's content while `serve` (`app/reload.rs:61-75`) never ran: layers
never uploaded, `remesher.retire(uploaded, content.serial)` never called, `self.hud`
stale, `notice::say_reloading` silent. That is the state `app/reload.rs:23-27`
declares must end the run, reached in silence. A refusal was lost the same way, and
the unwatchable-root refusal doubly so: the shipped watch reports it once
(`crates/mc-world/src/content/watch/notify_watch.rs:106`).

**Measured.** Mutation window 13:03 local, 2026-08-26, announced first: one line at
`tests/support/reload_watch/runs.rs:120`, `client.tick()` →
`client.frame(TICK_QUANTUM * 2)`. Five reload binaries gave
`11 tests run: 0 passed, 11 failed`, `left: [] / right: [TakenUp]`. Reverted by
hand; `git diff --exit-code -- crates/` clean; the same invocation then gave
`11 tests run: 11 passed`.

**This is not the residual hole the spec accepts.** That one is unobservable
because it needs a real window; this one was observable by eleven tests that
already existed, with a hand-driven clock and no hardware dependence.

## The fix, and the audit that shaped it

`ReloadStep::Nothing` now leaves the field standing — the narrowest repair, and
one that covers both producer arms. It cannot go stale: `App::present`'s early
returns skip the advance and the read *together*, so the field is drained in every
frame that advanced any tick.

Every effect reachable from the per-tick step was classified idempotent-under-
repetition or not — twelve checked, eleven idempotent, the reload report the one
that was not. The full list with citations is in the spec's `### Recorded during
validate`; the rule it produces is in `docs/technical/architecture.md` §"Pacing the
frame" as what a future change must not break. **§ Sibling sweep missed this by
checking a consumer and not its producer**, which is the generalizable lesson and
is recorded as one.

## RED, then green

`cargo nextest run -p mc-client --no-fail-fast --test reload_survives_a_multi_tick_frame`
over `fbf9c0d`, before the fix: `2 tests run: 0 passed, 2 failed`, both
`left: NobodyTheAnswerWasOverwritten` against `right: TheMultiTickFrame` —
assertion failures on an enumerated verdict, not a compile error. After the fix,
`cargo nextest run -p mc-client --no-fail-fast`: `385 tests run: 385 passed`.

## The gate found what the suite could not

The first gate run over the fix failed: exit 1,
`GATE FAILED - 1 stage(s): format (cargo fmt --check)`, on the new test file's
import block — in the same run that reported `1521 tests run: 1521 passed` and a
clean clippy. `testing.md` §2's *a green suite is no evidence about a lint*, on
this diff. Fixed at `ae743d5`.

## Disposition of pass 1's findings

- **Blocker** — fixed (`770d42e`), scenarios FR-2.2-S1/S2, tests at `fbf9c0d`.
- **Minor** — fixed (`4ae428d`). `docs/technical/architecture.md` named the
  wall-clock exemption at `mc-render/src/overlay/clock.rs`, which the port's move
  deleted, while the same document's own section named the new path; repathed to
  match `EXEMPT_FILE` (`tests/wall_clock_confinement.rs:71`).
- **Info ×2** — recorded in the spec's Notes, no code. The overlay ring's first
  entry is now the interval since the clock started, a deliberate reversal reasoned
  about pacing rather than about the ring; and `record_frame` names two different
  calls in the client, so `hud_entry_point.rs:23` reads ambiguously while its guard
  is unaffected.

## Otherwise sound (pass 1, unchanged)

Root cause addressed at `session/pacing.rs:95-103`, the per-tick door closed by
privacy (`session/mod.rs:322`). Quantum arithmetic verified at every boundary; the
`unspent < Q` invariant bounds the loop at 15. `TICK_QUANTUM`/`TICK_DURATION` pinned
within 2 ns by `mc-sim/src/player/physics_test.rs:32-49`, tolerance derived from
both directions. `Simulation::advance` returns `Some` only when `intent.action` is
`Some` (`mc-sim/src/simulation.rs:236-245`), so `advance_frame`'s `.last()` is
lossless. Scope clean; the updated documents are true of the code as built.

## Pass 2 — independent, on the frozen tree

**Verdict: PASS.** 0 Blocker, 0 Major, 1 Minor, 1 Info — **no new finding at
Major or above**, which is the only bar this pass accepts. Run by an agent that
wrote none of this code and read pass 1's findings only as a disposition list.

### Gate reading

`pwsh -NoProfile -File scripts/sdd-gate.ps1` at `887d0f0`, tree clean
(`git status --short` empty, `git stash list` empty) and taken **before** any
write of this pass. Log outside the repository at
`…/scratchpad/gate-887d0f0.log`; exit 0, final line `GATE PASSED`.

Stages read, all `ok:` — format, lint + complexity, gpu-free, docs, size, deps,
sast, secrets, art ×2, tests, coverage 93.72%. Test line verbatim:

```
     Summary [ 126.680s] 1521 tests run: 1521 passed (4 slow), 1 skipped
```

A **bare** `N tests run`, so a complete run and the count is a count.
`grep -c "^\s*FAIL"` over the log returns **0** — the case-insensitive hits are
test names inside `PASS` rows, as briefed.

### The Blocker fix, and its regression test

`session/reload.rs:145` — `ReloadStep::Nothing => {}`. Both producer arms are
closed: `:147` writes `Refused`, `:152` writes `Accepted`, and the third arm now
writes nothing at all, so no later tick can withdraw either.

**The staleness argument is sound, derived rather than checked.**
`App::present` (`app/mod.rs:266-297`) returns early on `Reconfigure` (`:271`),
`Fatal` (`:274`) and `Skip | Render` (`:275`) — all above `advance_frame`
(`:290`) and `take_up_reloaded_content` (`:292`), with no return between those
two. `advance_frame` has exactly one production call site and
`take_up_reloaded_content` exactly one, both there. So every frame that advanced
a tick also drained the field, and a kept report cannot outlive the frame that
produced it.

**The regression test would have been red, for this reason.** Confirmed by an
independent mutation rather than by reading the claim — window announced,
applied by hand, reverted by hand, `git diff --exit-code` clean, `HEAD` unmoved.

| # | Mutation | Result |
|---|---|---|
| M6 | `session/reload.rs:145`: restore the pre-fix `ReloadStep::Nothing => { self.reload_report = None; }` | `cargo nextest run -p mc-client --no-fail-fast` → `385 tests run: 383 passed, 2 failed, 0 skipped` (bare `N`, complete). Exactly the two FR-2.2 tests reddened, `left: NobodyTheAnswerWasOverwritten` / `right: TheMultiTickFrame`; nothing else moved. Reverted: gate green. |

A **one**-quantum frame could not have caught it: the accepting tick is the
frame's only tick, so `take_reload_report` finds the report whether or not
`Nothing` clears it. The defect needs a tick *after* the answering one inside the
same frame, which is what `TICKS_IN_THE_FRAME = 3` buys.

### The per-tick enumeration is complete — walked independently

Every mutable reach of `Session::tick` (`session/mod.rs:322-329`) was followed
without consulting the table: `pending_action`, `input` (`take_intent` drains the
look delta, keeps held keys), `Simulation::advance` (`player`, `world` via
`resolve`, `published`), and `cross_reload_boundary` → `ContentReload::at_tick_boundary`
(`watch.changes()`, `pending`, `in_flight`, `reported`, plus `content` and
`holding` through `adopt`). `Session`'s other fields — `capture`, `bindings`,
`overlay`, `pacing`, `pointer` — are untouched per tick; `overlay.record_frame`
and `pacing.spend` sit outside the loop at `session/mod.rs:441-446`, which is
what keeps the frame-rate ring at one entry per frame. The only take-once state
below the tick is `World::take_dirty` (`mc-sim/src/world/mod.rs:128`), reached
solely from `take_remesh_work`, which the frame path calls once
(`app/mod.rs:246`) and which accumulates.

**No thirteenth effect. I found no missing row.** The table's classifications
each hold on the code as read.

### Findings

**Minor — a slow build is misattributed to the Blocker's own verdict.**
`reload_survives_a_multi_tick_frame.rs:73` sets
`A_STRAGGLER_MAY_NOT_OUTLAST = AN_ATTEMPT_MAY_NOT_OUTLAST` (1 500 ms,
`support/reload_watch/runs.rs:66`). The run gives the build 1.5 s of sleep
(`:165`), then crosses ordinary boundaries for a further 1.5 s (`:192`). A build
finishing in (1.5 s, 3.0 s] lands in `ALaterBoundarySoTheBuildWasStillRunning` —
the distinguishable red the design intends. A build slower than **3.0 s total**
lands in `NobodyTheAnswerWasOverwritten` and reports the Blocker as regressed for
a reason about the machine. The module's own constant for "how long before a run
may conclude no attempt" is `A_RUN_EXPECTING_ONE_MAY_NOT_OUTLAST` = 6 s, and
`runs.rs:74-75` gives the reason verbatim — *"a slow attempt would be reported as
no attempt. Four times over."* This fixture concludes at 1×. In the same gate
run, `reload_takes_up_a_save_under_a_relative_root` took **6.461 s** end to end,
so the 6 s budget is not idle generosity on this hardware. One-constant repair:
`A_STRAGGLER_MAY_NOT_OUTLAST = A_RUN_EXPECTING_ONE_MAY_NOT_OUTLAST`. Not
blocking — the tests are green and the arm structure is already right; what is
short is the window that keeps the arms distinguishable.

**Info — the enumeration's own checksum is wrong, and two of its citations point
into prose.** `spec.md:620` reads *"the zeros are the result, and they are eleven
of the twelve"*; the table at `:624-636` has **thirteen** data rows, twelve `yes`
and one `no`. It has had thirteen since it was introduced at `130c295`, so this
is a miscount rather than a stale count. The number is doing real work — the
sentence around it exists so *"a reader can tell a complete audit from a lucky
one"* — and a reader who counts will have to redo the walk to find that nothing
is missing. Separately, `session/reload.rs:135` and `:139` (rows 13 and 12) name
lines inside the doc comment; the assignments are at `:147`/`:152` and `:150`.
Every other citation in the table is accurate. The condensed mirror at
`docs/technical/architecture.md:1445-1456` carries no count and is not affected.

### Nothing was filtered out

No finding was ranked out of this pass. The Minor above was considered for Major
and rejected against `validation-calibration.md`: it produces no wrong result in
shipped code and violates no scenario, and *"a misleading error message"* is
named Minor there. It is recorded here rather than waved through the accepted
list, which it is not on. Accepted items 1-6 were re-read and none of them covers
it; none of the six looks wrong to this pass.
