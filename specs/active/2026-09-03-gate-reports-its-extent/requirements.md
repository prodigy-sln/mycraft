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

- [open] Q: Does this fix also make stage 2b's `&&` chain non-cancelling, so a
  failure at `220` no longer skips the clippy at `221` and the run at `222`? →
  Put to the owner during this phase with a recommendation. The spec is drafted
  under **scope it to the flag and the label, and name the chain cancellation as
  a residual**, because repairing the chain means changing `Invoke-Stage`'s
  contract or accumulating exit status across four commands — new gate surface,
  overturning the design decision recorded at `:215-217`, and a `decision`
  work-type rather than a `low` fix. The flag still buys real ground at
  `220`/`222`: a failure *inside* mc-testkit's suite reports that suite's full
  extent instead of one test. If the owner rules the other way, spec §"Defect 1
  root cause", Out of Scope and Notes item 1 all change together.

- [resolved] Q: Is `--max-fail=N` an alternative? nextest's own cancellation
  warning suggests it beside `--no-fail-fast`. → A: No. It still cancels, just
  later. Any bound below the suite size leaves "how much is broken" unanswered,
  which is the only question this fix exists to answer. Recorded in the spec's
  rejected alternatives.

## Open questions

One, above: whether stage 2b's `&&` chain is in scope. It does not block the
scenarios — the spec is drafted under the recommended reading and the affected
sections are named — but it must close before implementation starts.
