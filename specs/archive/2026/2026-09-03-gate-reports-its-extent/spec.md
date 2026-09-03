---
id: SPEC-034
title: A red gate reports its own extent
status: implemented
work-type: fix
rigor: low
branch: bugfix/PRO-994-gate-reports-its-extent
issue: PRO-994
created: 2026-09-03
updated: 2026-09-03
approved: 2026-09-03
completed: 2026-09-03
author: spec-PRO-994
---

# Fix: A red gate reports its own extent

Two defects in `scripts/sdd-gate.ps1` with one shape: **the gate's own output
says less than the reader takes it to say.**

A red gate reports a cancelled test count as though it were a complete one, so
nobody can tell one broken test from three hundred — and one stage name stands
for four `&&`-chained commands, so a failure in the first hides three. And
`-Quick` announces itself as running no tests while running two suites, in three
separate places, so the same hole sits in the hot edit loop under a label that
says it cannot.

Both are defects in an instrument, not in the game. That is the whole reason
this spec is worth writing: every other spec in this project trusts what the
gate prints.

## Rigor

`low`, set by the owner and confirmed against the work. The change is one flag on
four command invocations, one stage split into two calls of an unchanged helper,
and one label corrected in the three places it is written. Two files change
(`scripts/sdd-gate.ps1`, `docs/technical/testing.md`). No crate is touched, no
published surface moves, no persisted or wire format changes, no helper contract
moves, and nothing here is something a later spec must not break.

**What would have forced rigor up is deliberately not here.** Changing how the
gate *detects* failure — `Invoke-Stage`'s single inspection of `$LASTEXITCODE` —
cannot be validated by a green gate, and that circularity is what puts PRO-982 at
`high`. This spec leaves that mechanism untouched and files the part that needs
it as PRO-1011.

TDD binds unchanged at `low`: tests are written first and their failing output is
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
| Anyone reading a red GPU-free stage | See `mc-render`'s result even when `mc-testkit` failed, instead of one stage name standing for four commands |
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

**Stage 2b amplifies this.** Lines 219-222 are **four** `&&`-chained commands —
clippy(mc-testkit), nextest(mc-testkit), clippy(mc-render), nextest(mc-render) —
inside one `Invoke-Stage` (`:218-223`). A failure in the **first hides three**,
while the summary records only the stage name `gpu-free (mc-testkit + mc-render,
no default features)`.

The chain is deliberate, and its reason is real: `Invoke-Stage` (`:147-157`) runs
`& $Action` and then inspects `$LASTEXITCODE` **exactly once**, after the whole
scriptblock. The comment at `:215-217` records that on separate lines a clippy
failure would be silently overwritten by a passing test run that followed it.

**Two of the four are recovered here; the rest is a named residual.**

- `--no-fail-fast` on 220 and 222 makes each nextest invocation run its own suite
  to completion, so a failure *inside* mc-testkit's tests reports mc-testkit's
  full extent instead of one test.
- **Stage 2b is split into two `Invoke-Stage` calls** — mc-testkit, then
  mc-render — each still `&&`-chaining its own clippy+nextest pair. A failing
  mc-testkit suite then stops hiding all of mc-render. This changes no contract:
  it is two calls where there was one, with no new gate surface and no change to
  `Invoke-Stage`. Nothing in the recorded rationale forbids it — the comment's
  reason for naming both crates explicitly is about `--workspace` feature
  unification, not about the two sharing a single stage.
- **What remains** is the two-command residual *within* each new stage: a clippy
  failure still hides its own crate's test run. Removing that needs either a
  contract change to `Invoke-Stage` — which affects failure detection for every
  stage, i.e. the gate's core mechanism — or a `cmd /c exit N` hack to force an
  exit code, because a PowerShell accumulator variable does not set
  `$LASTEXITCODE`. Filed as **PRO-1011**, recommended there as `work-type:
  decision` above `low` rigor, because **a change to how the gate detects failure
  cannot be validated by a green gate.** That circularity is the same one that
  put PRO-982 at `high`.

**The split was measured, not assumed.** No test holds a fixed stage list or
asserts a stage count. `crates/mc-client/tests/gate_stage_order.rs` uses
`Invoke-Stage` only inside its own synthetic control fixtures (`:41-157`), never
as a reading of the real script's stage list, and
`crates/mc-client/tests/gate_art_stages.rs` runs `-ArtOnly`, which selects stages
7 and 8 and never reaches 2b. `GateReport::StagesFailed(..)` lists the stages a
run named, in the order it named them, and asserts nothing about how many there
are. The only thing the split perturbs is the stage table in
`docs/technical/testing.md`, which this spec is already amending.

---

## Defect 2 — `-Quick` says it runs no tests, and it runs two suites

- **Observed**: The same false claim is made in **three** places, and a fourth
  document under-reports the gate by a whole stage:

  | where | what it says |
  |---|---|
  | `scripts/sdd-gate.ps1:173` | `mode: QUICK (format + lint + size only)` |
  | `scripts/sdd-gate.ps1:40-42` | `.PARAMETER Quick` — "Format, lint and size only. For tight edit loops" |
  | `docs/technical/testing.md:32` | "`-Quick` runs stages 1–3 only, for tight edit loops" |
  | `docs/technical/testing.md:19-30` | the canonical stage table has **no row for stage 2c** (`docs (rustdoc)`) at all |

  All three claims are false. The `-Quick` early exit is at `:294`; stage 2b (the
  GPU-free configuration, `:198-223`) and stage 2c (documentation links,
  `:225-241`) both sit above it. So `-Quick` runs stages 1, 2, **2b**, **2c** and
  3 — and stage 2b runs **four** commands, two of them real `cargo nextest`
  suites.

  The missing table row is folded in because it is load-bearing for the
  correction rather than an adjacent tidy-up: `-Quick`'s contents cannot be
  stated correctly in that document while a stage it reaches has no row to name.

- **Expected**: Every place that describes `-Quick` names what the mode actually
  runs, and the stage table names every stage the gate has. A reader deciding
  whether `-Quick` is safe in a tight loop is entitled to know it builds
  documentation and runs two test suites — whichever of the three they happen to
  read.

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

Label drift. All three statements were written when the claim was true and none
was revisited when stages 2b and 2c were inserted above the early exit. Nothing
ties a label to the stages that label describes, so it cannot go stale loudly —
it can only go stale silently, in as many places as it is written.

**This is in scope for this fix and not a separate cleanup.** The consequence of
the mislabel is precisely Defect 1's consequence: it places the fail-fast hole
in the hot edit loop while telling the reader that loop runs no tests.

**All three are corrected together, and that is the point.** A corrected banner
above a stale docstring is the same defect twice; correcting two of three and
leaving `docs/technical/testing.md:32` standing is the same defect a third time,
in the document a reader is most likely to consult *instead of* the script.

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

- **D1-S8**: THE SYSTEM SHALL run the `mc-testkit` and `mc-render` GPU-free
  checks as two separately-reported stages, so that a failure in the `mc-testkit`
  pair leaves the `mc-render` pair still run and still reported.

- **D1-S9**: THE SYSTEM SHALL state, in the gate script's own comment beside the
  `&&`-chained stage and in `docs/technical/testing.md`, that the chain still
  cancels the commands after a failing one and therefore still hides part of that
  stage's extent.

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

- **D2-S2**: THE SYSTEM SHALL describe `-Quick` consistently in all three places
  that describe it — the mode line at `:173`, the `.PARAMETER Quick` docstring at
  `:40-42`, and `docs/technical/testing.md` — so that no two of them can stand in
  disagreement.

- **D2-S3**: IF a gate script's `-Quick` mode line omits a stage that mode
  reaches THEN THE SYSTEM SHALL report the label as understating the mode rather
  than reporting it as accurate.

- **D2-S4**: THE SYSTEM SHALL give every stage the gate runs a row in the stage
  table in `docs/technical/testing.md`, including the `docs (rustdoc)` stage that
  has none today, and SHALL NOT fold into one row two stages the gate reports
  separately.

**D2-S4's second half was measured after this spec was stamped, by the owner
rather than by the spec author, and it is worse than one missing row.** The gate
reports **twelve** stages — format · lint + complexity · gpu-free · docs · size ·
deps · sast · secrets · art (generated set not committed) · art (voxforge build) ·
tests · coverage — and the table has **ten**. Row 9, "tests + coverage",
conflates two stages the gate prints apart: a run says `ok: tests` and then
`ok: coverage 93.85%`, and adds `tests (…)` or `coverage (…% < …%)` to the
failure list separately. So the table under-reports by two, not one. Both halves
are the same defect in the same table on the same read, and shipping a table
still short by a row would be the half-fix this spec exists to stop. The
implement phase counted the gate side independently — from the script's own
`Invoke-Stage` and `Write-Ok` calls rather than from the owner's run — and got
the same twelve.

**D2-S3 is the control.** A scan that graded every label as accurate would
satisfy D2-S1 and D2-S2 against the shipped script and against any other.
**D2-S2 is deliberately a three-way agreement rather than a two-way one**:
correcting the banner and the docstring while `docs/technical/testing.md:32`
still says "stages 1–3 only" is the same defect a third time, in the document a
reader is most likely to consult instead of the script.

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

4. **Agreement is not enough: the three descriptions must enumerate.** Added
   after approval, by the owner, and it is the `--ignore-run-fail` shape one
   level up — *the cheapest way to satisfy the assertion degrades the artifact*.
   D2-S2 as written asks the three descriptions to agree **with each other**, and
   the cheapest way to satisfy that is to make all three vague: write "stages 1–3
   only" in the banner, in the docstring and in `docs/technical/testing.md` and
   they agree perfectly, while the banner has become *less* informative than it is
   today. `docs/technical/testing.md:32` is what makes the trap live — unlike the
   other two it is not flatly false but **ambiguous**, because that document's own
   table numbers stages 1, 2, 2b and 3, so "stages 1–3" arguably already includes
   the stage running the suites. The test is therefore written so mutual vagueness
   cannot satisfy it: each of the three is graded against a **closed enumeration**
   of what `-Quick` runs — format, lint, gpu-free **tests**, **docs**, size — and
   `"stages 1-3 only"` is one of the control's inputs, graded as naming nothing.
   The remedy is enumeration, never paraphrase. (`testing.md` §2, "an over-tight
   assertion invites a real defect", read in its mirror image.)

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
- **Making the `&&` chain itself non-cancelling**, so a clippy failure no longer
  hides its own crate's test run. **PRO-1011.** Two neighbouring things *are* in
  scope and must not be read out of it: the stage **split** (D1-S8), which halves
  the residual, and **stating the residual** in the script's comment and in
  `docs/technical/testing.md` (D1-S9). What is excluded is the restructure alone.
- **A check tying a mode label to the stages that mode reaches** — **PRO-1012**.
  Correcting the three labels (D2-S2) and adding the missing stage row (D2-S4)
  are in scope; the instrument that would stop them drifting again is not.
- **Any change to `Invoke-Stage`'s contract** or to the summary format. Splitting
  stage 2b adds a second call to the existing `Invoke-Stage`; it changes nothing
  about what `Invoke-Stage` does.
- **Changing which stages `-Quick` runs.** This spec corrects what `-Quick`
  *says*, never what it *does*. Whether stage 2b or 2c belongs above the early
  exit at all is a separate question and is not asked here.

## Notes

Observations recorded during the specify phase. **Each says explicitly which part
this spec builds and which part it does not** — "recorded, not built" applies to
the mechanism in each case, never to writing the residual down.

1. **The `&&` chain still cancels within each of the two new stages** — filed as
   **PRO-1011**. After D1-S8's split and `--no-fail-fast`, a failure in one
   crate's clippy still hides that crate's test run.

   **In scope, and asserted: D1-S9.** The residual is stated in the gate script's
   own comment beside the chain, and in `docs/technical/testing.md`. This is not
   optional polish. The comment at `:215-217` today justifies the `&&` on the
   masking argument alone; the moment `--no-fail-fast` lands beside it that
   justification becomes **true but incomplete** — it will not say the chain
   still cancels and still hides part of the stage's extent. A half-true
   justification standing beside a changed line is the defect family this project
   keeps filing, and the reader who needs the warning meets it at that comment,
   not in an archived spec folder.

   **Out of scope: restructuring the chain.** That needs either a contract change
   to `Invoke-Stage` (`:147-157`), which affects failure detection for every
   stage in the gate, or a `cmd /c exit N` hack to force an exit code, since a
   PowerShell accumulator variable does not set `$LASTEXITCODE`. PRO-1011
   recommends `work-type: decision` above `low`, because **a change to how the
   gate detects failure cannot be validated by a green gate** — the same
   circularity that put PRO-982 at `high`.

2. **Nothing ties a mode label to the stages that mode reaches** — filed as
   **PRO-1012**. Defect 2 is label drift that went unnoticed because it cannot go
   stale loudly, and being written three times in three files is the evidence.

   **In scope: correcting all three statements (D2-S2) and adding the missing
   stage row (D2-S4).** **Out of scope: the check that would stop it recurring.**
   D2-S2 asks the three descriptions to agree *with each other*, which is
   strictly weaker than asking any of them to agree with the stage list — three
   mirrors can be made consistent and all three still wrong. A check deriving the
   label from the stages, or grading it against them, is new gate surface and
   larger than this fix should introduce. PRO-1012 carries the sizing, and notes
   that `crates/mc-client/tests/gate/reading.rs` already reads the script's
   stages and their nesting, so the instrument largely exists.

3. **The stage table in `docs/technical/testing.md` had no row for stage 2c** —
   the canonical table under-reported the gate by a whole stage and nothing
   noticed. D2-S4 adds the row; nothing prevents the next stage from landing
   unrowed. Same root as Note 2, folded into **PRO-1012** rather than filed
   separately.

4. **The script's own `.DESCRIPTION` stage list was a fourth mirror, and the
   implement phase corrected it.** Defect 2's table names three `-Quick` claims
   and the document's stage table; it does not name the header block at
   `scripts/sdd-gate.ps1:7-16`, which lists the gate's stages as 1 through 9 with
   no 2b, no 2c, and `tests+cov` as one entry. **In scope, and the reasoning is
   recorded rather than assumed**: it is the same defect — a list of the gate's
   stages that under-reports the gate — in the file this spec is already
   amending; it is *output*, since `Get-Help` prints it; and the stage split would
   otherwise have left it describing a gate that no longer exists. Correcting
   three mirrors and leaving a fourth is Defect 2's own root cause, repeated. No
   behaviour changes and no gate surface is added. The recurrence check is still
   **PRO-1012**.

5. **`--ignore-run-fail` appears nowhere in the script, including in the warning
   against it.** The spec forbids it "anywhere in the gate script" and the test
   reads the raw text, so the `.DESCRIPTION` block describes the flag and points
   at `docs/technical/testing.md` for its name and the measurement, rather than
   carrying either. The document is where a reader looks anyway, and it now
   carries the three-row table.

6. **What the D1-S1/D1-S2 test does and does not catch, stated.** The fixture is
   run through `cargo nextest run`, so an implementation carrying
   `--ignore-run-fail` reddens it as `TheRunnerRefusedTheFlags` — nextest does
   not accept an llvm-cov flag — rather than as the exit-0 verdict the spec
   measured. It reddens either way, which is what the spec asks of a test
   observing count and verdict together, and the exit-0 distinction is carried by
   the reading's enumeration instead (D1-S7 tells `HidesTheFailureFromTheGate`
   from `RunsEveryTestItWasGiven`). Running the fixture under
   `cargo llvm-cov nextest` would close the last of the gap and was rejected: it
   nests a coverage run inside the gate's own coverage run, on Windows, for no
   falsifier the pair does not already provide.

7. **`sdd-artifact-lint.sh` reports this folder red for two reasons that are not
   this phase's to repair**, recorded so the complete phase is not surprised.
   `test-map.md: 36 lines for 0 mappings` — the detector counts `→`/`->` and every
   test map in `specs/archive/` uses a markdown table instead, so it counts zero
   for all of them; that is **PRO-1010**, already Out of Scope above. And
   `requirements.md: 236 lines (budget 150)`, which is the specify phase's
   artifact and predates this phase.

## Open Questions

None.

## Validation

**2026-09-03 — PASS.** `scripts/sdd-gate.ps1` exits 0 at commit
`08b9ef20579148bec23c2926c350dcc0ca45ace9` with an empty dirty list and an empty
stash, which together identify the measured tree (`e8ed93e…`): 13 stages all
`ok:`, `1757 tests run: 1757 passed (14 slow), 1 skipped`, lines 93.85%, regions
92.4%. The count is bare rather than slashed, which is this spec's own subject —
a complete run, not a cancelled one — and it is 1747 + 10, the ten regression
tests this spec added. `gpu-free` appears twice, for `mc-testkit` and
`mc-render`, which is D1-S8's stage split visible in the gate's own output.

Rigor `low`, so the gate carries the review. Every regression scenario's failing
output was displayed before implementation and recorded in the commit that
carried the tests: 10 tests, 6 assertion failures.

### The reading this spec nearly shipped on

**A gate reading is a statement about a tree, and this spec nearly settled itself
by reasoning about which files changed instead of measuring them.** A full gate
was in flight when two comment-only edits landed. The proposal was to report that
in-flight reading, on the argument that the delta was provably confined to files
only three test binaries read. The argument was *probably correct* — the size
stage filters `*.rs` (`sdd-gate.ps1:295`), so neither changed file was measured
there, and no Rust changed, so coverage could not move. It was overruled and the
gate re-run on a committed quiet tree, and the prediction held.

**Being right about the outcome and having measured it are different things, and
only one of them is evidence.** That is `standards/global/testing.md` §2's named
failure mode reached verbatim — arguing about which files changed in order to
predict what a re-run would say.

The sharper reason it could not stand is specific to this file. **PowerShell
parses the whole script at launch**, so the in-flight run was executing the
*pre-edit* `sdd-gate.ps1`. Its verdict described the old gate — and this spec's
subject *is* that script. The instrument under test was the instrument taking the
reading.

**The irony is the lesson.** The spec written to stop the gate under-reporting
its own extent nearly shipped on a gate reading whose extent was reasoned about
rather than measured: the same defect, one level up, in the artefact that was
supposed to prove the defect fixed.

### What the re-run bought beyond the verdict

The retaken log stamps `TREE=`, `DIRTY=[]` and `STASH=[]` into its own header
before the gate's first stage. (`TREE=` holds a commit rather than a tree object
— a field naming something other than what it holds, which is Defect 2's own
shape surviving into this spec's evidence. Recorded in
`docs/technical/testing.md` with the correction for the next stamp.) **That makes the reading date itself**, which is
strictly stronger than an external `git status` taken afterwards: an equality
check performed later is a true statement about a *later* instant and cannot
recover the tree the reading was taken on. Evidence carried inside the reading
cannot be taken at the wrong moment. The technique generalises past this gate and
is consolidated into `docs/technical/testing.md`.

Both halves are carried into `docs/technical/testing.md` rather than left here —
the archive is history, not documentation.
