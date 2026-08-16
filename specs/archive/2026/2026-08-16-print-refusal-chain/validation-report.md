# Validation Report — a refusal a person can act on

- **Verdict: PASS** (pass 1)
- Rigor: `high` · Branch `feature/PRO-939-print-refusal-chain` at `e041e2e`
- Date: 2026-08-16

## Summary

| Reviewer | Blocker | Major | Minor | Info | Verdict |
|---|---|---|---|---|---|
| correctness | 0 | 0 | 0 | 0 | PASS |
| coverage | 0 | 0 | 0 | 0 | PASS |
| quality | 0 | 0 | 0 | 0 | PASS |

Three specialist reviewers ran in parallel over a 32-file manifest. No candidate
finding was raised, so no adversarial verification pass ran and nothing was
filtered out of the merge.

Each reviewer's payload was inspected individually rather than only the merged
verdict, because an absent reviewer and a clean reviewer produce the same summary
line. All three returned per-scenario verdicts with distinct citations.

## Gate

Run independently of the implementation, on the committed tree:

```
GATE_EXIT=0 — GATE PASSED
1007 tests run: 1007 passed, 1 skipped
lines 94.03%  regions 92.01%  (9033 lines tracked)
```

Coverage is unchanged across all four phases, so nothing landed that dilutes the
denominator. The gate body was scanned for `error:`, `FAILED` and
`failed to remove`; nothing matched.

## Scenarios

All 26 acceptance scenarios are implemented and covered: FR-1.1 (4), FR-2.1 (6),
FR-3.1 (3), FR-4.1 (3), FR-5.1 (3), FR-6.1 (4), FR-7.1 (3). The scenario ID sets
in the specification and in `test-map.md` were compared mechanically and are
identical in both directions.

## What this increment delivers

A mod author who mistypes one field in a content file runs the client and reads
the file, the declaration, and the parser's caret pointing at the word, with the
accepted spellings listed. One edit, no bisect, no reading Rust. That text is
quoted in two authoring pages, and a guard compares those quotes against a real
run so the page and the terminal cannot drift apart.

## Stated limits of this verification

Recorded because a verification that claims more than it established is worse
than one that admits its edges.

1. **The way-out guidance has exactly one uncovered production line, measured.**
   Replacing the guidance argument at the failing redraw site with an empty string
   leaves the entire suite green. That line runs inside a redraw needing a device
   and a display server. The other failing site is fed only by a refusal whose
   guidance is empty by construction, so it is incapable of emitting the sentence;
   mutating it would be a semantically identical program and was deliberately not
   run, because a green result there would look like evidence and be none. What
   holds it is that the guidance argument is not optional and the ending cannot be
   constructed outside its three doors — not a test.

2. **Four scenarios were green on arrival with no honest failing step available.**
   Each is a scenario about a scan's own behaviour, and those scans are test code
   end to end, so there was no production defect to stage. What keeps them
   non-vacuous is that each compares a whole enumerated verdict, which rejects
   "I could not look" as well as every wrong answer.

3. **One needle in the no-exemption scan has no production witness left.** The
   inventory it was written against went from four sites to zero as this increment
   converted them, so it rests entirely on its positive-control fixture.

4. **Two overlay sites and five windowing sites are structurally covered only.**
   Asserting the overlay's text needs a device and a display server; asserting the
   windowing sites' text would be asserting a dependency's internals.

## Deferred, raised as their own issues

- **Guidance travels with the call rather than with the error.** A trait carrying
  it on the failure would remove the possibility of a site not supplying it,
  rather than watching for it.
- **Four non-fatal notices write to the process error stream directly**, outside
  the renderer and outside any sink, so nothing can capture, redirect or silence
  them. The as-built record was corrected to name all four rather than quietly
  stop being false.

## Next

`/sdd-complete`. User sign-off is not required for this increment.
