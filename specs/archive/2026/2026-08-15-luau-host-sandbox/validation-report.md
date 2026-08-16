# Validation Report — Luau host, sandbox and the hostile-mod harness

- **Verdict: PASS** (pass 1)
- Rigor: `high` · Branch `feature/PRO-916-luau-host-sandbox` at `f0b9a0b`
- Date: 2026-08-16

## Summary

| Reviewer | Blocker | Major | Minor | Info | Verdict |
|---|---|---|---|---|---|
| correctness | 0 | 0 | 0 | 0 | PASS |
| coverage | 0 | 0 | 0 | 0 | PASS |
| quality | 0 | 0 | 0 | 0 | PASS |

Three specialist reviewers ran in parallel over a 51-file manifest. No candidate
finding was raised, so no adversarial verification pass was required and nothing
was filtered out of the merge.

Each reviewer's payload was inspected individually rather than only the merged
verdict, because an absent reviewer and a clean reviewer produce the same
summary line. The quality reviewer returned an empty findings list after reading
every production source in the crate plus the specification documents and the
harness support files; its empty `scenarios` array is expected, since that
dimension issues no per-scenario verdicts.

## Gate

Run independently of the implementation, on the committed tree:

```
GATE_EXIT=0 — GATE PASSED
977 tests run: 977 passed, 1 skipped
lines 93.82%  regions 91.8%  (8999 lines tracked)
```

The gate body was scanned for `error:`, `FAILED` and `failed to remove`; the only
matches are test names containing those words.

## Scenarios

All 56 acceptance scenarios are implemented and covered: FR-1.1 (4), FR-1.2 (6),
FR-2.1 (5), FR-3.1 (6), FR-4.1 (5), FR-4.2 (10), FR-5.1 (6), FR-6.1 (4),
FR-7.1 (5), FR-8.1 (3), FR-9.1 (2). Every scenario received a PASS verdict from
both the correctness and coverage reviewers, each citing a distinct location.

## Stated limits of this verification

These are recorded because a validation that claims more than it established is
worse than one that admits its edges.

1. **The chunk label on a table handle is not witnessed by any test.** Replacing
   the label at the point where a field read is translated, with a deliberately
   wrong value, reddens nothing: every callback the suite attaches is labelled
   where its chunk is named, and the follow-up entries read out of a returned
   table are identities rather than callables, so no fault is ever attributed
   through the labelled value. The construction that would witness it is named in
   `test-map.md`. It ships as a reasoned argument, not as covered.

2. **Five of the six shipped limits are reported but not witnessed enforcing at
   their shipped values.** One scenario covers the call-and-loop budget at its
   default. The fault threshold and round bound are exercised at shipped values
   by the hostile harness, which configures nothing; a 16 MiB backstop is not
   reachable inside a test's time budget. Recorded deliberately rather than
   assumed away.

3. **A malformed follow-up entry is passed over silently.** Never fabricating a
   target the mod did not store is the deliberate half; the absence of any signal
   to the mod author is a known diagnosability gap, recorded as a deferred
   observation for a later increment.

## Notes carried into review

The reviewers were briefed that one task's claim about a raw-read scenario is
false — an attachment completing its remaining iterations afterwards proves
nothing about budget consumption, because every invocation begins on a whole
fresh budget — so the additional witness in that test is not duplicate coverage
and must not be filed as such. They were also briefed that an unbitten mutation
is evidence only when confirmed to have landed: this feature encountered one
mutation absorbed by a second enforcement site and one that silently failed to
apply.

## Next

`/sdd-complete`. User sign-off is not required for this increment.
