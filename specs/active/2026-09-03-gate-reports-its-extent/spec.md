---
id: SPEC-034
title: A red gate reports its own extent
status: active
work-type: fix
rigor: low
branch: bugfix/PRO-994-gate-reports-its-extent
issue: PRO-994
created: 2026-09-03
updated: 2026-09-03
author: spec-PRO-994
---

# Fix: A red gate reports its own extent

Two defects in `scripts/sdd-gate.ps1` with one shape: **the gate's own output
says less than the reader takes it to say.**

A red gate reports a cancelled test count as though it were a complete one, so
nobody can tell one broken test from three hundred. And `-Quick` announces
itself as running no tests while running two suites, so the same hole sits in
the hot edit loop under a label that says it cannot.

Both are defects in an instrument, not in the game. That is the whole reason
this spec is worth writing: every other spec in this project trusts what the
gate prints.

## Rigor

`low`, set by the owner and confirmed against the work. The change is one flag
on four command invocations and one corrected mode label, in one file. No crate
is touched, no published surface moves, no persisted format or wire format
changes, and nothing here is something a later spec must not break. TDD binds
unchanged at `low`: tests are written first and their failing output is
displayed before any implementation. What `low` drops is the reviewer workflow,
and the gate carries it.

## Why this spec may touch the gate at all

CLAUDE.md key principle 5 says gate amendments are "a project-level decision
with a runtime budget — never a spec deliverable." **That is not a prohibition
on this spec, and the reading is recorded here so nobody re-litigates it.**

What principle 5 forbids is a spec amending the gate *in passing, to suit
itself* — loosening a threshold that is failing its own work, so that the gate
stops reporting something true. This spec is the opposite case: the gate is its
declared **subject**, opened deliberately as PRO-994, and the amendment makes
the gate report *more* than it does today, never less.

**The runtime budget, stated:** `--no-fail-fast` costs **nothing on a green
run**. Every test executes anyway when none fails, so the flag changes no timing
on the path the gate spends almost all of its life on. It costs additional time
only on a *red* run — and that is time currently spent not knowing how much is
broken, which is the cost this fix exists to stop paying.

## Stakeholders, named honestly

CLAUDE.md principle 9 asks for a capability a **player**, **mod author** or
**server operator** can exercise. **This spec reaches none of those three, and
that is stated plainly rather than dressed up.** The subject is the project's
own verification instrument; it ships in no binary, appears in no content
declaration, and changes nothing a player can see or a mod author can write.

The stakeholder it does reach is **whoever reads a red gate** — the owner in
the `-Quick` edit loop, and every pipeline agent that treats a gate log as
evidence. What they can do afterwards that they cannot do today:

| Stakeholder | What they can do that they cannot do today |
|---|---|
| Anyone reading a red gate | Read the true number of failing tests off the gate's own output, instead of re-running the suite by hand to find out |
| Anyone running `-Quick` | Know that the mode runs two test suites and a documentation build, instead of a label claiming it runs neither |

This is the case principle 5 contemplates: a gate amendment taken as a
project-level decision. It is recorded as a knowing exception to principle 9's
three-stakeholder list rather than as a satisfied one — inventing a
player-facing claim here would be the more damaging outcome, because it would
teach the next reader that the list can be satisfied by assertion.

---

## Defect 1 — every test invocation in the gate cancels at the first failure

- **Observed**: `scripts/sdd-gate.ps1` runs tests through **four** invocations
  and none carries `--no-fail-fast`:

  | line | invocation |
  |---|---|
  | 220 | `cargo nextest run -p mc-testkit --no-default-features &&` |
  | 222 | `cargo nextest run -p mc-render --no-default-features` |
  | 441 | `cargo nextest run --workspace --no-tests=pass` |
  | 504 | `cargo llvm-cov nextest` (flags continue on 505-508 via backtick) |

  nextest's default is to cancel the run at the first failure. The summary then
  reads `N/M tests run`, which states nothing whatever about the M−N tests that
  never executed. PRO-994 records the gate reporting `1294/1591 tests run` with
  `297/1591 tests were not run due to test failure`; the phase that hit it
  re-ran the suite by hand with `--no-fail-fast` and got `1591 tests run: 1590
  passed, 1 failed, 1 skipped`. **It could not have known that from the gate.**

- **Expected**: A red gate reports a complete count — the bare `N tests run`
  form — so its output bounds the damage, and still fails.

- **Reproduced**: 2026-09-03, on this branch, against a throwaway five-test
  crate with three deliberate failures, run `-j1` (at default parallelism every
  test starts before the first failure registers, so a small fixture cannot
  exhibit fail-fast at all). Fixture deleted after measurement.

  | flags | summary line | exit |
  |---|---|---|
  | none | `2/5 tests run: 1 passed, 1 failed` | 100 |
  | `--no-fail-fast` | `5 tests run: 2 passed, 3 failed` | 100 |
  | `cargo llvm-cov nextest --no-fail-fast` | `5 tests run: 2 passed, 3 failed` | 100 |

  The third row is the one that matters for line 504: **`cargo-llvm-cov`
  forwards `--no-fail-fast` verbatim to nextest rather than consuming it, and
  preserves nextest's exit code.**

### Root cause

nextest cancels on first failure unless told otherwise, and no invocation in
`scripts/sdd-gate.ps1` tells it otherwise. There is no defect in the gate's
*reporting* code — `Invoke-Stage` (`:147-157`) and the summary block (`:545+`)
faithfully report a stage as failed. The gate under-reports because the tool it
calls stopped early, and the gate never asked it not to.

**Line 220 amplifies this.** Lines 219-222 are `&&`-chained inside one
`Invoke-Stage` (`:218-223`), deliberately: the stage's own comment at `:215-217`
records that `Invoke-Stage` inspects `$LASTEXITCODE` once, after the whole
scriptblock, so on separate lines a clippy failure would be silently overwritten
by a passing test run. A failure at 220 therefore also cancels the clippy at 221
and the run at 222, while the summary records only the stage name `gpu-free
(mc-testkit + mc-render, no default features)`.

**That chain cancellation is a named residual of this fix, not a thing it
repairs.** After `--no-fail-fast` lands on 220 and 222, each invocation runs its
own suite to completion, so a failure *inside* mc-testkit's tests reports
mc-testkit's full extent instead of one test. What still goes unreported is the
two commands *after* the failing one. Removing that would mean changing
`Invoke-Stage`'s contract or accumulating status across four commands — new gate
surface, overturning a recorded design decision, and outside `low`. It is filed
rather than built (see Notes).

---

## Defect 2 — `-Quick` says it runs no tests, and it runs two suites

- **Observed**: `scripts/sdd-gate.ps1:173` prints `mode: QUICK (format + lint +
  size only)` and the `.PARAMETER Quick` docstring at `:40-42` says "Format,
  lint and size only. For tight edit loops". Both are false. The `-Quick` early
  exit is at `:294`; stage 2b (the GPU-free configuration, `:198-223`) and stage
  2c (documentation links, `:225-241`) both sit above it. So `-Quick` runs
  stages 1, 2, **2b**, **2c** and 3 — and stage 2b runs two real `cargo nextest`
  suites.

- **Expected**: The mode line and the parameter documentation name what the mode
  actually runs. A reader deciding whether `-Quick` is safe in a tight loop is
  entitled to know it builds documentation and runs two test suites.

- **Reproduced**: 2026-09-03, by reading, and the reading is named as a reading.
  `:187` opens `if (-not $ArtOnly) {`; stage 2b's `Invoke-Stage` at `:218` and
  the `if ($Quick)` exit at `:294` are at the same indent inside that one block,
  with no `$Quick` guard between them. The script is a straight run of top-level
  statements with no loop and no function containing a stage, which
  `crates/mc-client/tests/gate/reading.rs` already relies on for exactly this
  kind of question. A live `-Quick` run would additionally cost a full
  `clippy --workspace --all-targets --all-features` pass, which buys nothing the
  read does not already settle.

### Root cause

Label drift. The mode line and the docstring were written when the claim was
true and were not revisited when stages 2b and 2c were inserted above the early
exit. Nothing in the gate ties the two together, so the label cannot go stale
loudly — it can only go stale silently.

**This is in scope for this fix and not a separate cleanup.** The consequence of
the mislabel is precisely Defect 1's consequence: it places the fail-fast hole
in the hot edit loop while telling the reader that loop runs no tests. Same
file, same lines, same read.

---

## The trap: `--ignore-run-fail` is forbidden by name

`cargo-llvm-cov` carries a second flag whose help text reads **more** like this
issue's ask than the correct flag does. The two sit adjacent in
`cargo llvm-cov --help`:

```
--no-fail-fast
    Run all tests regardless of failure

--ignore-run-fail
    Run all tests regardless of failure and generate report

    If tests failed but report generation succeeded, exit with a status of 0.
```

`--ignore-run-fail`'s description is a strict superset of `--no-fail-fast`'s.
Measured on the same fixture, 2026-09-03:

| flags | summary line | exit |
|---|---|---|
| `--no-fail-fast` | `5 tests run: 2 passed, 3 failed` | 100 — correct |
| `--ignore-run-fail` | `5 tests run: 2 passed, 3 failed` | **0 — catastrophic** |

**The two summary lines are byte-identical. They differ only in exit code.** At
`scripts/sdd-gate.ps1:510` an exit 0 takes the `else` branch: `Write-Ok 'tests'`
runs, the coverage read proceeds, and **the gate passes with three red tests.**

`--ignore-run-fail` MUST NOT appear anywhere in the gate script, and the reason
is recorded here so a future reader cannot re-introduce it as an apparent
improvement over the flag this spec chose.

---

## Regression Scenarios

### D1 — a red run reports its full extent, and still fails

- **D1-S1**: WHEN a test invocation the gate runs has three failing tests among
  five THE SYSTEM SHALL run all five and report the complete bare form
  `5 tests run: 2 passed, 3 failed`, never the cancelled form `2/5 tests run`.

- **D1-S2**: WHEN a test invocation the gate runs has three failing tests among
  five THE SYSTEM SHALL exit non-zero and name the stage in its failure summary,
  reporting the same complete count it reports on any red run.

- **D1-S3**: THE SYSTEM SHALL carry `--no-fail-fast` on every `cargo nextest`
  and `cargo llvm-cov nextest` invocation in `scripts/sdd-gate.ps1`, read across
  each invocation's whole continued command rather than its first line.

- **D1-S4**: THE SYSTEM SHALL carry no `--ignore-run-fail` anywhere in
  `scripts/sdd-gate.ps1`.

- **D1-S5**: WHEN the gate script is scanned for its test invocations THE SYSTEM
  SHALL report every invocation it finds as an ordered enumeration, so that a
  missing invocation, an added invocation and a reordering are three distinct
  failures.

- **D1-S6**: IF a gate script carries a test invocation without `--no-fail-fast`
  THEN THE SYSTEM SHALL name that invocation rather than reporting the script as
  complete.

- **D1-S7**: IF a gate script carries `--ignore-run-fail` THEN THE SYSTEM SHALL
  report it as forbidden rather than as satisfying D1-S3.

**D1-S2 is the load-bearing control, and it is what makes D1-S1 falsifiable.**
An implementation using `--ignore-run-fail` satisfies D1-S1 exactly — same
count, same words, same bytes — and fails only D1-S2. **D1-S1 and D1-S2 must
therefore be asserted together, over one reading, in at least one test**: a test
that observes the count without observing the verdict is green on the
catastrophic implementation and is not evidence. D1-S6 and D1-S7 are the
positive controls for the scan: without them, a scan that silently stopped
finding invocations would report the shipped script as complete forever.

### D2 — the mode label describes the mode

- **D2-S1**: WHEN the gate is run with `-Quick` THE SYSTEM SHALL print a mode
  line naming test execution and documentation among what that mode runs.

- **D2-S2**: THE SYSTEM SHALL describe `-Quick` consistently in the mode line it
  prints and in its `.PARAMETER Quick` documentation, so the two cannot drift
  apart unnoticed.

- **D2-S3**: IF a gate script's `-Quick` mode line omits a stage that mode
  reaches THEN THE SYSTEM SHALL report the label as understating the mode rather
  than reporting it as accurate.

**D2-S3 is the control.** A scan that graded every label as accurate would
satisfy D2-S1 and D2-S2 against the shipped script and against any other.

---

## Binding on the tests

Three constraints hold over every test written for this spec. Each is an
instance of a rule already in `standards/global/testing.md`, instantiated at the
exact defect at hand.

1. **A count without a verdict is not an observation.** At least one test
   asserts D1-S1 and D1-S2 over the same reading. (`testing.md` §1, "a stronger
   observable alongside a scenario's own".)

2. **`--ignore-run-fail` is forbidden by name**, with the measurement above as
   the recorded reason. A test asserts its absence, and D1-S7 is the positive
   control that keeps that absence assertion honest. (`testing.md` §2, "a
   structural-invariant test needs a positive control".)

3. **Any scan over the script enumerates and compares the whole set — it never
   filters a hand-maintained list of known invocations.** This project already
   ships the failure mode: `crates/mc-client/tests/gate/reading.rs:55`'s
   `SKIPPING_TEST_COMMAND` is `cargo nextest run --workspace`, deliberately
   narrow enough to miss lines 220 and 222; `test_command_lines` (`:271-276`)
   then `filter_map`s a two-element list through `sole_line_of` (`:294-303`),
   which returns `None` whenever its needle appears on more than one line. A
   fifth invocation is invisible to it, and a needle that starts matching twice
   drops silently to zero hits. **Read the invocations out of the script and
   compare the whole enumeration in order.** (`testing.md` §2, "a
   hand-maintained list compared by *filtering* cannot see an extra member".)

   A corollary, because it is the cheap wrong answer: **"the line carrying the
   command also carries the flag" is false for a correct fix.** Line 504's flags
   live on 505-508 behind backtick continuations. An assertion in that shape
   would go red against a correct multi-line implementation, and the cheapest
   way to green it is to let the test dictate the script's formatting — which is
   backwards. Read the flag against the whole continued invocation.

---

## Rejected alternatives

- **`--max-fail=N`.** nextest's own cancellation warning suggests it beside
  `--no-fail-fast`. Rejected: it still cancels, just later. Any bound below the
  suite size leaves the same question unanswered — how much is broken — which is
  the only question this fix exists to answer.

- **`--ignore-run-fail`.** Rejected and forbidden by name; see the trap above.
  It produces the correct summary line and the wrong verdict, which is worse
  than the defect it would replace.

- **A `-TestsOnly` stage selector**, so a falsifier could make the real gate go
  red on a real suite. Ruled out by the owner before this spec opened, and the
  reasoning stands on its own: the test stage is `--workspace`, so a gate
  invoking itself would run the suite inside the suite.
  `docs/technical/testing.md:155-171` records that doctrine and names what is
  left to a person. A new selector is new gate surface and a design decision
  outside a `low` fix. `grep -c TestsOnly scripts/sdd-gate.ps1` returns 0 today,
  and it still should afterwards.

## Out of Scope

Binding. Recorded, not built.

- **The concurrent-run collision on `target/llvm-cov-target`** — two gate runs
  sharing one directory produce a failure that reads as a test failure. That is
  **PRO-982**, being specced separately as a `decision` at `high` rigor because
  it amends ADR-031.
- **`sdd-artifact-lint`'s test-map and registry detectors** — PRO-1010.
- **The size stage counting with `Measure-Object -Line`, which drops blank
  lines** — PRO-974. A gate defect, and a different stage.
- **Restructuring the `&&` chain in stage 2b** so a failure at line 220 no
  longer cancels lines 221-222. See Notes.
- **Any change to `Invoke-Stage`'s contract**, to the summary format, or to
  which stages `-Quick` runs. This spec corrects what `-Quick` *says*, never
  what it *does*.

## Notes

Deferred observations, recorded during the specify phase and not built here.

1. **Stage 2b's `&&` chain still cancels.** With `--no-fail-fast` on 220 and
   222, each invocation runs its own suite to completion, but a failure at 220
   still short-circuits the clippy at 221 and the run at 222. Repairing that
   means changing `Invoke-Stage`'s contract or accumulating exit status across
   the chain — new gate surface, overturning the design decision recorded at
   `:215-217`, and a `decision` work-type rather than a `low` fix. To be filed
   as its own issue at completion.

2. **Nothing ties a mode label to the stages that mode reaches.** Defect 2 is
   label drift that went unnoticed because it cannot go stale loudly. D2-S2
   asks the two descriptions to agree with each other, which is weaker than
   asking either to agree with the stage list. A check deriving the label from
   the stages, or grading it against them, is a larger piece of gate surface
   than this fix should introduce.

## Open Questions

None.
