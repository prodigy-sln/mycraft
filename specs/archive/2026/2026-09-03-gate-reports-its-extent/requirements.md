# Requirements ledger — a red gate reports its own extent

Source issue: PRO-994 (lead), with PRO-1000 closed into it as a duplicate.
The issue body and its two comments were read in full with
`linear-cli issues get PRO-994` and `linear-cli comments list PRO-994 -o json`
before a scenario was written, and every factual claim below was re-checked
against the tree on this branch rather than taken from the issue text.

## Clarifications

- [resolved] Q: How many nextest invocations does the gate actually have? The
  issue body names one (`:441`). → A: **Four**, and none carries
  `--no-fail-fast`. `grep -n "nextest" scripts/sdd-gate.ps1` on this branch
  returns `220` (`cargo nextest run -p mc-testkit --no-default-features &&`),
  `222` (`cargo nextest run -p mc-render --no-default-features`), `441`
  (`cargo nextest run --workspace --no-tests=pass`) and `504`
  (`cargo llvm-cov nextest`, whose flags continue on 505-508 via backtick
  continuation). All four are in scope. A fix that changes only `:441` leaves
  three cancelling invocations behind.

- [resolved] Q: Are the GPU-free invocations a *wider* hole than `:441`? → A:
  Yes. `220` and `222` are `&&`-chained inside one `Invoke-Stage`
  (`scripts/sdd-gate.ps1:218-223`) precisely so that `Invoke-Stage` inspects
  `$LASTEXITCODE` once, after the whole scriptblock — which the stage's own
  comment at `:215-217` states as deliberate. A failure at `220` therefore
  cancels the clippy at `221` *and* the run at `222`, and the summary records
  only `gpu-free (mc-testkit + mc-render, no default features)`.

- [resolved] Q: Is `-Quick` in scope? The gate's own label says it runs no
  tests. → A: **In scope, and the label is wrong.** The early exit is at
  `scripts/sdd-gate.ps1:294`; stage 2b is `198-223`. So `-Quick` runs stages 1,
  2, 2b, 2c and 3 — two real test suites among them — while `:173` prints
  `mode: QUICK (format + lint + size only)` and the `.PARAMETER Quick`
  docstring at `:40-42` says "Format, lint and size only". The fail-fast hole is
  in the hot edit loop, not only in the full gate. Correcting the label is the
  same file, the same lines, and the same read; it is part of this fix.

- [resolved] Q: `cargo-llvm-cov` documents `--ignore-run-fail` as "Run all
  tests regardless of failure and generate report", which reads more like this
  issue's ask than `--no-fail-fast` does. Which one? → A: **`--no-fail-fast`.
  `--ignore-run-fail` is forbidden by name in the spec.** Measured by the owner
  on 2026-09-03 against a 5-test fixture with 3 deliberate failures, run `-j1`
  (default parallelism starts every test before the first failure registers, so
  a small fixture cannot show fail-fast at all):

  | flags | summary line | exit |
  |---|---|---|
  | none | `2/5 tests run: 1 passed, 1 failed` | 100 — cancelled |
  | `--no-fail-fast` | `5 tests run: 2 passed, 3 failed` | 100 — correct |
  | `--ignore-run-fail` | `5 tests run: 2 passed, 3 failed` | **0 — catastrophic** |

  The last two summary lines are byte-identical and differ only in exit code.
  At `scripts/sdd-gate.ps1:510` an exit 0 takes the `else` branch, `Write-Ok
  'tests'` runs, the coverage read proceeds, and the gate passes with three red
  tests. Also measured: `cargo-llvm-cov` forwards `--no-fail-fast` verbatim to
  nextest rather than consuming it, and preserves nextest's exit code.

  **Re-measured independently during this phase**, on this branch, against a
  fresh five-test crate outside the repository (deleted afterwards). All four
  readings reproduced, including the byte-identical summary lines and the exit-0
  from `--ignore-run-fail`. The help text was read directly rather than quoted:
  `--no-fail-fast` at `cargo llvm-cov --help:169-170` ("Run all tests regardless
  of failure") and `--ignore-run-fail` at `:172-175`, adjacent, the latter's
  description a strict superset of the former's. That adjacency plus the superset
  wording is why the trap is live rather than theoretical.

- [resolved] Q: What does that trap force on the scenarios? → A: At least one
  scenario must pin the gate's **verdict** alongside the count. A scenario
  asserting only "the summary is a bare count naming every failure" goes green
  on the catastrophic implementation, because that implementation produces the
  identical summary line. This is `standards/global/testing.md` §1's "a stronger
  observable alongside a scenario's own": keep the scenario's own observable and
  add, in the same scenario, whatever makes it falsifiable.

- [resolved] Q: May a scan over the script filter a list of known invocations,
  the way `crates/mc-client/tests/gate/reading.rs` already does? → A: **No.**
  That file is the instantiation of the hazard. `reading.rs:55`'s
  `SKIPPING_TEST_COMMAND` is `cargo nextest run --workspace`, deliberately
  narrower than `cargo nextest run` so it does not match `220`/`222` — the
  doc comment says so. `test_command_lines` (`:271-276`) then
  `filter_map`s a hand-maintained two-element list through `sole_line_of`, and
  `sole_line_of` (`:294-303`) returns `None` whenever the needle appears on
  more than one line. A list compared by filtering cannot see a fifth
  invocation, and a needle that starts matching twice silently drops to zero
  hits. Any scan this spec adds must **enumerate** every nextest invocation it
  finds and compare the whole set in order, so a missing one, an extra one and
  a reordering are three distinct failures.
  (`standards/global/testing.md` §2, "a hand-maintained list compared by
  filtering cannot see an extra member".)

- [resolved] Q: Can the assertion be "the line carrying the command also
  carries the flag"? → A: **No, and it is actively harmful.** `:504`'s flags
  live on `505-508` behind backtick continuations, so that assertion is *false
  for a correct multi-line fix*. The cheapest way to green it is to let the
  test dictate the script's formatting, which is backwards. The flag must be
  read against the whole continued invocation.

- [resolved] Q: Should this spec add a `-TestsOnly` stage selector so a
  falsifier can make the real gate go red? → A: **No, ruled out by the owner
  before the spec opened.** `grep -c TestsOnly scripts/sdd-gate.ps1` returns 0
  today, and that absence is deliberate: the test stage is `--workspace`, so a
  gate that invoked itself would run the suite inside the suite.
  `docs/technical/testing.md:155-171` records the doctrine and names what is
  left to a person. A new selector is new gate surface and a design decision
  outside a `low`-rigor fix. The instruments available are the text scan
  (`crates/mc-client/tests/gate/reading.rs`) with synthetic control scripts
  (`crates/mc-client/tests/gate_stage_order.rs:200-261`), the run harness
  (`crates/mc-client/tests/gate/running.rs:97`, which spawns
  `pwsh -NoProfile -File sdd-gate.ps1` with arbitrary args and reads an
  enumerated `GateReport`), and a throwaway fixture crate if a live nextest
  reading is needed.

- [resolved] Q: CLAUDE.md key principle 5 says "Gate amendments are a
  project-level decision with a runtime budget — never a spec deliverable."
  Does that forbid this spec? → A: **No, and the spec states the reading so
  nobody re-litigates it.** What principle 5 forbids is a spec amending the
  gate *in passing* to suit itself — loosening a threshold that is failing its
  own work. It does not forbid a spec whose declared **subject** is the gate,
  opened deliberately, with the runtime budget weighed and recorded. The budget
  here: `--no-fail-fast` costs **nothing on a green run**, because every test
  executes anyway; it costs only additional *red*-run time, which is time
  currently spent not knowing how much is broken. The owner opened PRO-994 as
  that project-level decision and set `work-type: fix` / `rigor: low`.

- [resolved] Q: work-type and rigor. → A: Set by the owner, not derived here:
  `work-type: fix`, `rigor: low`, issue `PRO-994`, branch
  `bugfix/PRO-994-gate-reports-its-extent`. `low` is carried because the change
  is one flag on four invocations plus one corrected label in one file, with no
  crate touched, no published surface moved and nothing a later spec must not
  break. TDD binds unchanged at `low`; what `low` drops is the reviewer
  workflow, and the gate carries it.

- [resolved] Q: Does the architect phase run? → A: No.
  `.prospect/prompts/matrix.tsv` declares `architect` rows only for
  `work-type: feature`. The spec declares no Architecture Delta.

- [resolved] Q: Is the concurrent-run collision on `target/llvm-cov-target` in
  scope? → A: **No, binding.** That is PRO-982, being specced separately as a
  `decision` at `high` rigor because it amends ADR-031. Any temptation is
  recorded as a deferred observation and not built.

- [resolved] Q: Are the other two open gate/lint defects in scope? → A: No.
  PRO-1010 (`sdd-artifact-lint`'s test-map and registry detectors) and PRO-974
  (the size stage's `Measure-Object -Line` dropping blank lines) are separate
  issues against separate stages. Both are listed in Out of Scope.

- [resolved] Q: Does this fix also make stage 2b's `&&` chain non-cancelling? →
  A: **No — ruled by the owner, who checked `Invoke-Stage` before answering.**
  `Invoke-Stage` (`:147-157`) runs `& $Action` and inspects `$LASTEXITCODE`
  exactly once. A PowerShell accumulator variable does not set `$LASTEXITCODE`,
  so the alternatives are a contract change to `Invoke-Stage` — which affects
  failure detection for all twelve stages, i.e. the gate's core mechanism — or a
  `cmd /c exit N` hack. The first carries a circularity that decides the rigor:
  **a change to how the gate detects failure cannot be validated by a green
  gate**, which is the same reasoning that put PRO-982 at `high`. Filed as
  **PRO-1011**, recommended there as `work-type: decision` above `low`.

  Correction the owner made to my draft: the chain is **four** commands
  (clippy mc-testkit, nextest mc-testkit, clippy mc-render, nextest mc-render),
  so a failure in the first hides **three**. My draft said "the clippy at 221 and
  the run at 222", which undercounts it. Fixed throughout.

- [resolved] Q: Can stage 2b be split into two `Invoke-Stage` calls — mc-testkit,
  then mc-render — each still `&&`-chaining its own clippy+nextest pair? Raised
  by the owner, with the explicit instruction to measure it and decide rather
  than take it on their say-so. → A: **Yes, and it is in scope (D1-S8).**
  Measured on this branch:

  - **No test holds a fixed stage list or asserts a stage count.**
    `crates/mc-client/tests/gate_stage_order.rs` contains `Invoke-Stage` only
    inside its own synthetic control fixtures (`:41-157`) — those are scripts the
    test writes out to grade its own reading, never a reading of the real
    script's stage list.
  - `crates/mc-client/tests/gate_art_stages.rs` runs `-ArtOnly`, which selects
    stages 7 and 8 and never reaches 2b.
  - `GateReport::StagesFailed(..)` (`gate/running.rs:56`) carries the stages a run
    listed, in the order listed, and asserts nothing about how many exist.
  - Feature unification is per-invocation, so `-p mc-testkit
    --no-default-features` behaves identically in one scriptblock or two. The
    comment's reason for naming both crates explicitly is about avoiding
    `--workspace`, not about the two sharing a stage — nothing in the recorded
    rationale forbids the split.

  The only thing it perturbs is the stage table in `docs/technical/testing.md`,
  which this spec is amending anyway (D2-S4). So it is two calls plus one table
  row: no contract change, no new surface. Taken.

- [resolved] Q: How many places state the false `-Quick` claim? → A: **Three**,
  not two. Found during this phase: `scripts/sdd-gate.ps1:173` (banner),
  `:40-42` (`.PARAMETER Quick`), and **`docs/technical/testing.md:32`** — "`-Quick`
  runs stages 1–3 only, for tight edit loops". Correcting two and leaving the
  third is the owner's own "a corrected banner above a stale docstring is the
  same defect twice", one step further on, in the document a reader is most
  likely to consult *instead of* the script. All three are corrected (D2-S2).

- [resolved] Q: Anything else wrong in that stage table? → A: Yes.
  `docs/technical/testing.md:19-30` has rows for stages 1, 2, 2b, 3, 4, 5, 6, 7,
  8, 9 and **no row for stage 2c** (`docs (rustdoc, no broken intra-doc links)`,
  `scripts/sdd-gate.ps1:238`). The canonical stage table under-reports the gate by
  a whole stage. Folded in as D2-S4 rather than deferred, because it is
  load-bearing for the correction: `-Quick`'s contents cannot be stated correctly
  in that document while a stage it reaches has no row to name.

- [resolved] Q: Is `--max-fail=N` an alternative? nextest's own cancellation
  warning suggests it beside `--no-fail-fast`. → A: No. It still cancels, just
  later. Any bound below the suite size leaves "how much is broken" unanswered,
  which is the only question this fix exists to answer. Recorded in the spec's
  rejected alternatives.

- [resolved] Q: Is documenting the residual a deferral or a deliverable? → A: **A
  deliverable, asserted as D1-S9.** Ruled by the owner. What is out of scope is
  *restructuring* the chain; what is in scope is the remaining hole being stated
  where a reader meets it — the gate script's own comment beside the chain, and
  `docs/technical/testing.md`. The reason is concrete: the comment at `:215-217`
  justifies the `&&` on the masking argument alone, and the moment
  `--no-fail-fast` lands beside it that justification becomes true-but-incomplete.
  A half-true justification standing beside a changed line is the defect family
  this project keeps filing. The spec folder is archived at completion, so the
  script and `docs/` are what survive for the reader.

- [resolved] Q: Which issues are filed, and when? → A: **Both filed during this
  phase, not deferred to completion**, at the owner's direction, because a reader
  reaches for the tracker and `docs/` rather than a folder of superseded specs.
  Both verified after creation as team `prodigy-solutions`, project
  `MyCraft MVP 2: Scriptable Content`.

  | issue | subject |
  |---|---|
  | **PRO-1011** | The gpu-free stage `&&`-chains four commands, so a failure in the first hides three — carries the four-command count, the `Invoke-Stage:147-157` single-inspection constraint, both escape routes, and the `decision`-not-`fix` recommendation |
  | **PRO-1012** | Nothing ties a gate mode label to the stages that mode reaches — carries the three false `-Quick` statements, the missing stage 2c table row, and why D2-S2's three-way agreement is strictly weaker than agreeing with the stage list |

## Open questions

None. The one open question — the scope of stage 2b's `&&` chain — was ruled by
the owner during this phase: split the stage (in scope), document the residual
(in scope, D1-S9), leave the chain mechanism alone (out of scope, PRO-1011).
