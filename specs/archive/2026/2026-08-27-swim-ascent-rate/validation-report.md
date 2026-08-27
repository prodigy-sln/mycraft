# Validation Report — SPEC-030 (PRO-992)

**Verdict: PASS — pass 1.** Rigor `high`, three specialist reviewers in parallel
with adversarial per-finding verification and a deterministic merge.

| Reviewer | Verdict | Findings |
|---|---|---|
| correctness | PASS | 0 |
| coverage | PASS | 0 |
| quality | PASS | 0 |
| **merged** | **PASS** | **B0 · M0 · m0 · plausible 0 · Info 0** |

No dead dimensions, no abstentions, no failed scenarios, no Minor overflow.
`agents_empty_result: 0` — every reviewer returned a payload, so this is not the
absent-reviewer case an aggregated "no findings" cannot otherwise distinguish.

## Gate

`Summary [137.090s] 1591 tests run: 1591 passed (3 slow), 1 skipped` — a **bare**
count, so a complete run rather than a cancelled one — `lines 93.73% regions
92.26% (11002 lines tracked)`, all 12 stages `ok:`, `GATE PASSED`. Taken at
`3d200c9` with `HEAD` read immediately before and after, unmoved.

## Provenance of this reading, stated rather than assumed

**The review spanned two commits.** It was launched against `0295b7d`; `3d200c9`
landed while the reviewers were running. The delta is **one line of
`specs/active/2026-08-27-swim-ascent-rate/tasks.md`** — T15 gaining the
behaviour-side obligation and naming `save_declarations.rs` — and **zero code
files**. The direction is one-way: the newer line carries strictly more
information, so it could only have produced a false positive about T15's routing,
never masked a defect. No such finding was raised. The gate condition above was
taken at the later commit.

## Per-reviewer payloads were read, not just the merged verdict

Every scenario carries a `file:line` and a reason. Reviewer 2 hand-traced the
physics arithmetic for every FR-4 scenario (S1 `2.0`, S2 `0.0`, S3
`−0.3333`/`−1.0`, S4 `−0.5`, S5 greatest-of-two, S6 sink independent of ascent,
S7 the displacement clamp, S8 the empty-cell identity, S9 the masked
non-swimmable ascent) and reports each matching its declared value exactly.
Reviewer 3 additionally confirmed the manifest was complete — *"git diff
--name-only against merge base contains no source file outside the manifest"* —
and checked the recognised-field list across all four documentation quotations
and both test mirrors.

**One imprecision in a reviewer's prose, caught by reading the file and recorded
because it propagated nowhere.** Reviewer 3 described `save_declarations.rs` as
*"version 4/25 bytes"*. The file states **twenty-nine** bytes for version 4
(`:102`); twenty-five is the superseded **version 3** length it also records
(`:138`). The committed constant `0xb808_ebfd_74e6_f12a` is the twenty-nine-byte
value. Reviewer summary only; no artifact carries the wrong figure.

## Knowingly accepted, briefed to the reviewers, and not refiled

1. `architecture.md:790-798` — rotted citation **and** a wrong stated consequence
   (`medium_table_width.rs`'s control is `assert!(many > shipped)`, a strict
   inequality that survives). Prose only, reported not renumbered.
2. `crates/mc-sim/tests/replay_solidity.rs:131`'s stale `FR-6.1-S6` — a different
   file's owner.
3. Four of six `sea.rs` line citations drifted `+9/+10`. **Aged measurements taken
   at `4814afe`, not rotted references** — following them today finds nothing
   *because the retarget landed*.
4. `chamber.rs` deliberately unchanged; a medium under its air was measured to
   redden 46 tests across twelve files.
5. Three FR-8.1 scenarios were **green on arrival**, accepted on measured
   falsifiers (M1 +1, M2 +8, M3 +23, M4 +3) recorded in `test-map.md`.
6. Six spec-prose repairs made during implementation are repairs, not defects.
7. `test-map.md`'s artifact-lint PASS is not evidence of concision (PRO-991).
8. The gate cannot report its own extent — no `--no-fail-fast` (PRO-994).

## Findings carried out of this spec

- **The golden frames cannot see `swim_ascent` at all.** Mutating it `3.5 → 4.0`
  leaves `terrain_goldens` green; `move_resistance 0.5 → 0.6` reddens them. The
  scripted walk never holds jump under water at any captured tick, so before
  FR-6.1-S2/S3 an ascent could have shipped guarded by nothing.
- **A fixture at rest cannot tell "keep the velocity" from "take this zero"
  apart.** Bit three times; `tasks.md` T08's originally-named falsifier pair does
  not falsify, and FR-5.1-S3 does.
- **A fixture that separates lists cannot see a held-back revision byte** — even
  the revision-3 fixture minted for this move.
- **A needle spelled after a constant misses the file that never names it.**
  `world-format.md:803-823` predicted this and it happened anyway, because
  `tasks.md` T15 and `spec.md` FR-7.1 named only two of three byte-stating files.
  Repaired at `3d200c9`. **A warning is only as good as the path that reaches it.**

## Next

`/sdd-complete`. User sign-off does not apply — conductor mode.
