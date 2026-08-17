# Validation report — SPEC-017, hot reload

Branch `feature/PRO-918-hot-reload` · issue PRO-918 · rigor `high` · conductor mode, so
user sign-off is not required. Two passes, both run by the conductor through the
`sdd-validate` workflow: three specialist reviewers in parallel, per-finding adversarial
verification, deterministic merge.

## Summary

| Pass | Reviewer | Blocker | Major | Minor | Info | Verdict |
|------|----------|---------|-------|-------|------|---------|
| 1 | correctness | 0 | 0 | 0 | 0 | PASS |
| 1 | coverage | 0 | 0 | 1 | 0 | PASS |
| 1 | quality | 0 | 0 | 0 | 0 | PASS |
| 1 | **merged** | **0** | **0** | **1** | **0** | **FAIL** |
| 2 | correctness | 0 | 0 | 0 | 0 | PASS |
| 2 | coverage | 0 | 0 | 0 | 0 | PASS |
| 2 | quality | 0 | 0 | 0 | 0 | PASS |
| 2 | **merged** | **0** | **0** | **0** | **0** | **FAIL** (scenario verdict, below) |

**Nothing was filtered out of either merge.** The per-reviewer counts sum exactly to the
merged counts in both passes, which is the check that matters: an absent reviewer and a
clean reviewer look identical in an aggregated verdict, and earlier in this MVP a real
defect was ranked out of an otherwise clean PASS.

## Pass 1 — one Minor, and it was the defect this MVP had already shipped once

**Scenario IDs embedded in code doc-comments**, against `CLAUDE.md`'s clause that *the
mapping lives in the spec folder's `test-map.md` — test names stay behavioral, and code
never carries spec or scenario IDs*. Fifteen occurrences across six files this branch adds,
confirmed by the workflow's verifier against the tree and again by the conductor.

It bites because of this project's own disposal setting: `spec-disposal: archive` with
`retention-days: 365` moves the spec folder on the way to merge and prunes it a year later,
so every one of those identifiers becomes a reference with no resolvable target — and one
site pointed a reader at "this spec's `test-map.md`" by name.

**Closed across three commits, none of which deleted a sentence.** Each comment now states
the substance the citation stood for, and where a pointer earned its place it names
something that outlives the archive — a sibling test file, a `docs/` page, a type:

- `2a518b9` — the fifteen scenario IDs, seven files.
- `bba3ad3` — six prose citations of spec-folder *documents*, five files. The same
  dangling-ness in a weaker form of the rule; taken deliberately as a first instalment
  rather than left whole, with the remainder filed.
- `591d096` — six bare requirement IDs (`FR-1.1`, `FR-3`, `FR-4.5`, `FR-6.2`, `FR-1.3`),
  five files.

## Pass 2 — zero findings, and a PARTIAL that was the conductor's own measurement error

Pass 2 returned **zero findings at every severity on all three dimensions**. Its FAIL comes
from a single scenario verdict, `scenario-id-closure-completeness = PARTIAL`: the reviewers
judged pass 1's closure incomplete, and they were right.

**The miscount was the conductor's.** The search used to size the class matched only the
*scenario* form, `FR-\d+\.\d+-S\d+`. The rule bans spec identifiers, and a bare `FR-1.1`
dangles on exactly the same schedule — so two thirds of the class were invisible to the
instrument that declared it closed, and the output of a narrow instrument was reported as a
measurement of the whole. That is the same failure this spec spent eight phases finding in
fixtures, committed by the person policing it.

Closure is now verified mechanically rather than by opinion:
`grep -rn "FR-[0-9]" crates/ --include=*.rs` returns **12 occurrences in 11 files, every one
outside this spec's manifest** — five of them in production sources left by earlier specs.
All are filed as PRO-950, deliberately untouched here because fixing them would widen this
spec's diff past its manifest and invalidate the gate reading taken against it.

**No pass 3 was commissioned, and that is a ruling rather than an omission.** The validate
skill forbids starting one unprompted, and the remaining item is *mechanically checkable*: a
`grep` settles whether the rule holds, and three reviewers cannot settle it better. A
mechanically checkable claim gets the mechanical check.

## A third form, found by applying the same lesson one level further

Before the closure was called complete, the test author swept for a *third* shape rather
than assuming two exhausted the class — decision identifiers, `Decision N`, `invariant N`,
`§`. No decision identifier remains in this manifest.

The sweep produced a distinction worth more than the count, and it constrains PRO-950:

> The rule is not "no document references in code". It is **no reference to something that
> dies.** The spec folder archives and prunes; `standards/global/testing.md` and `CLAUDE.md`'s
> invariants do not. Citations of the standards are exactly the durable pointer this repair
> asked for, and a regex widened to catch `architecture.md`'s `Decision 8` would destroy them.

## Scenario verdicts

92 of 92 scenarios have at least one test asserting their outcome; `test-map.md` carries the
mapping and an additional-coverage section. No scenario was reported as unmet by any
reviewer in either pass.

## Gate

Run by the conductor, unpiped, exit code read off the run itself, at the closure commit
`591d096`:

```
ok: format · lint + complexity · gpu-free · docs · size · deps · sast · secrets
Summary [ 103.413s] 1193 tests run: 1193 passed (2 slow), 1 skipped
ok: tests
ok: coverage 94.36%
GATE PASSED
```

**One transient failure is recorded rather than smoothed over.** An earlier instrumented run
at this same commit failed with one test of 242 before stopping; a plain
`cargo nextest run --workspace --all-features` immediately afterwards returned 1193 of 1193,
and the full gate then passed as above. **The failing test's name was not captured, because
the conductor's own output filter could not match nextest's indented `FAIL [` line** — an
instrument that destroyed the evidence it existed to collect, which is the same shape as the
miscount above. The suspect is the known load-sensitive `reload_remesh_blocks_no_tick`, whose
history is five recorded readings across this spec, but that is a derivation and is not
asserted as an observation. Added to PRO-949 as an uncharacterised data point.

## Manual acceptance

Performed by the conductor in the **shipped binary**, not in a test: a client launched from a
playground working directory with its own content root — a **relative** root, which is the
configuration that was inert before phase 7 — then edited while running. A misspelled field
printed the refusal naming the file, the block and the field within seconds; correcting it and
then breaking it differently printed the second, distinct refusal, so an accepted swap does not
wedge the loop.

**What this does not cover, stated plainly:** an accepted reload prints nothing by design, so
its player-visible effect has not been observed by a human at the time of writing. The
capability this spec exists to deliver is the accepted path, and a green suite is not evidence
about it — this spec's own history is that 86 of 91 scenarios were green over five phases while
the shipped client watched nothing at all, and then that 1 188 tests were green over a watcher
whose every event was discarded. The two figures belong to the two different defects and are
not interchangeable.

## Verdict

**PASS**, on the conductor's authority, at commit `591d096`:

- Pass 1's single Minor is closed, verified by `grep` rather than by report.
- Pass 2 returned zero findings at every severity on all three dimensions, and its PARTIAL
  scenario verdict is the closure of that same Minor, now mechanically confirmed.
- The gate exits 0 at the closure commit, run by the conductor.

Nine knowingly-accepted items were briefed to the reviewers with their reasoning and are
recorded in `tasks.md` and `docs/`; the one flagged for examination rather than acceptance —
that `App` actually *reaches* its report of a gone re-mesh worker, and that its wording is
right — drew no finding from any reviewer.
