# Validation Report — the medium properties (PRO-957)

**Verdict: PASS** · pass 1 · rigor `high` · commit `5580a93`

## Counts

| Reviewer | Verdict | Blocker | Major | Minor | Info | Scenarios located |
|---|---|---|---|---|---|---|
| `sdd-review-correctness` | PASS | 0 | 0 | 0 | 0 | 44 / 44 |
| `sdd-review-coverage` | PASS | 0 | 0 | 0 | 0 | 44 / 44 |
| `sdd-review-quality` | PASS | 0 | 0 | 0 | 0 | n/a |

Confirmed findings: **0**. Plausible findings: **0**. Abstentions: **0**. Dead
dimensions: **0**. Minor overflow: **0**. Failed scenarios: **0**.

## Every dimension was read, not assumed

A merged "no findings" cannot distinguish a clean reviewer from an absent one,
so the per-agent payloads were inspected rather than the aggregate.

- **correctness** — 44 located scenario verdicts, each with `file:line`.
- **coverage** — 44 located scenario verdicts, each with `file:line`.
- **quality** — returned `scenarios: []`, which **looks** like the failure mode
  above. It is not: quality is not scenario-scoped, so an empty scenario list
  is correct for it. Verified by transcript rather than by argument — **123
  journal lines, 611 636 bytes, 40 tool calls** across the production diff
  (`water.luau`, `definition.rs`, `collide.rs`, `medium.rs`, `world/mod.rs`,
  `luau_declaration/mod.rs`, `spec.md`). It reviewed and found nothing.

**Refinement recorded for future passes:** a missing *located* judgement is a
prompt to read the transcript, not a conclusion.

## Gate

`scripts/sdd-gate.ps1` on `5580a93` — the commit itself, bracketed by
`git rev-parse HEAD` and `git status --short` before and after, both unchanged.

```
ok: format · lint + complexity (clippy, zero warnings) · gpu-free · docs ·
    size · deps · sast · secrets · art (generated set not committed) ·
    art (voxforge build) · tests · coverage
Summary [ 125.772s] 1502 tests run: 1502 passed (3 slow), 1 skipped
ok: coverage 93.71%
GATE PASSED   (exit 0, zero FAIL lines in 3081)
```

`1502 tests run` is a **bare** count — a complete run, not a cancelled one.
Baseline at branch point was 1435.

## Artifact lint — accepted, not closed

`sdd-artifact-lint.sh` reports nothing against this spec folder. Two findings
elsewhere are **accepted with reasons rather than closed**:

- **`requirements.md`, 177 lines against a 150 budget.** Written by the specify
  phase (`git log -1 -- requirements.md` → `8476cab`) and 177 lines before any
  implement phase began. The budget exists so knowledge migrates to `docs/` at
  consolidation; that is the consolidator's question, not a reason for an
  implement phase to trim a ledger later phases cite.
- **21 of 22 `specs/REGISTRY.md` entries exceed the 50-word cap.** All predate
  the rule. Rewriting an append-only historical record would be worse than the
  finding.

Neither blocks: the project gate never invokes the lint.

## Provenance limitation, stated rather than glossed

The review ran while the branch moved from `ba2b11d` to `5580a93`. Measured
delta: **2 files, 35 insertions, 13 deletions — prose only**
(`docs/technical/working-in-this-repo.md` attribution; one loose `spec.md`
Notes sentence corrected plus a checked negative recorded). No code, scenario,
test or fixture moved. No finding was raised against either changed region, so
nothing required re-verification against `5580a93`. **Cause: a `[DONE]` child
with unread messages from the conductor is not a frozen tree.**

## Scenario verdicts

All 44 PASS, located by both scenario-scoped reviewers. No scenario was
reported unimplemented, untested, or asserted only by execution.

## Metrics

Seven resolver-stamped phases, `2026-08-24T19:30Z` → `2026-08-25T03:24Z`:
specify · architect · tasks · implement ×4. Validation pass 1 cost **833 674
subagent tokens, 115 tool uses, 6 m 28 s** across three reviewers.

## Carried to completion

Not defects; recorded so completion does not rediscover them.

1. **A one-way-door fixture hazard**, owed to `docs/technical/testing.md`:
   FR-7.1-S5/S6/S7 need a save minted under the *previous* behaviour revision,
   and the revision is a compile-time constant — so those three scenarios
   became permanently untestable the moment T13 landed. The fixture was minted
   first, alone, before anything moved. **Nothing in `tasks.md` named it.**
2. **`docs/INDEX.md` sits outside `PAGES = ["docs", "modding"]`** and rotted
   twice on this branch alone → tracked issue.
3. **`[0, 59, 119]` exists as three uncompared mirrors** (`DECLARED_CAPTURE_TICKS`,
   `JUDGED_TICKS`, `SAMPLED_TICKS`); this spec touched two of them → tracked issue.
4. **Proposed standards amendment** at `spec.md`: `standards/global/git-workflow.md`
   bans `git add -A` but stops at staging — the index is shared, so the commit
   must name its paths (`git commit -- <paths>`). **Recorded, deliberately not
   applied; the owner approves standards changes.**
5. `SPEC-022` appears in no `docs/INDEX.md` row.

## Next

`/sdd-complete`. User sign-off is overridden in conductor mode.
