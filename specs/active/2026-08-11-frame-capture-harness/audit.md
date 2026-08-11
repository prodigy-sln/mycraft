# Scenario Audit Resolution — SPEC-001

Auditor: `sdd-scenario-auditor`, 2026-08-11. Reviewed 33 scenarios across 15
FRs; returned 23 gap drafts (5 high, 9 medium, 5 low), 9 guideline violations
in existing scenarios, and 2 contradictions.

Verdict: **21 of 23 gaps accepted**, 1 accepted in reduced form, 1 rejected.
All 9 guideline fixes accepted. Both contradictions resolved. The spec after
resolution carries 53 scenarios across 17 FRs.

## Accepted in full

| # | Gap | How it landed |
|---|-----|---------------|
| 1 | Straight vs premultiplied alpha unasserted (every scenario used alpha 255) | FR-2.1-S3. Asserts the colour channels are **not** scaled by alpha, rather than an exact alpha byte — 0.25 × 255 = 63.75 is rounding-ambiguous, and the relationship is the actual contract |
| 2 | sRGB half of the format contract unasserted | FR-2.1-S4, as a mid-tone round trip within ±1. Pins the colour-space decision without hanging a test on an 8-bit rounding boundary |
| 4 | Golden overwrite path missing, and its interaction with the mismatch artifact set undefined | FR-4.4-S3 and FR-4.4-S4 |
| 5 | Invariant 5 — "`mc-testkit` must not depend on `mc-render`" — asserted by nothing | New FR-6.1. The auditor is right: a structural invariant with no test is a comment |
| 6 | Opt-ins referred to only as "the skip opt-in" while a scenario requires naming them | Fixed as `MYCRAFT_ALLOW_NO_GPU` and `MYCRAFT_UPDATE_GOLDENS` throughout |
| 7 | ΔE distances stated without the pixel values that produce them | All comparison scenarios now use neutral greys, where ΔE reduces to \|ΔL*\|: (128,128,128) vs (129,129,129) ≈ 0.40, vs (140,140,140) ≈ 4.67, vs (180,180,180) ≈ 19.72. Verified by hand against the sRGB → linear → L* chain |
| 8 | Defaults and overrides described but never asserted | FR-3.5-S1 (defaults applied when unspecified) and FR-3.5-S2 (an override takes effect). The 30 s deadline default moved from Clarifications into FR-2.3's text |
| 9 | Oversized capture unbounded, in a workspace where `panic!` is lint-denied | FR-2.2-S3 |
| 10 | "The CPU half is testable without a GPU" — the ADR-008 coupling — unasserted | FR-3.4-S3 |
| 11 | FR-2.4-S1 round-trips through the harness's own writer and reader, cancelling symmetric errors (a row flip on write plus a row flip on read passes while the PNG is upside-down) | FR-2.4-S2, asserting asymmetric content at named coordinates on the decoded file. Strongest finding in the audit — this is exactly the systematic failure D5 exists to catch |
| 12 | Diff image "is reproducible" promised, never asserted | FR-4.3-S2 |
| 13 | "Machine-readable report" satisfied by a prose text file as written | FR-5.1-S1 fixes the format as JSON. This also settles the open implementation choice: `serde_json` |
| 14 | Corrupt/undecodable golden undefined | FR-4.4-S5 |
| 15 | FR-2.3-S2 fired a deadline that had already elapsed — a guard clause, not the wait loop D7 exists to bound | FR-2.3-S2 rewritten to elapse *during* the wait, and given the missing "SHALL NOT return an image" clause |
| 16 | Artifact deletion is a behavior, and `nextest` runs tests in parallel processes | FR-4.1-S3 scopes deletion to the capture's own golden directory |
| 17 | `MYCRAFT_ALLOW_NO_GPU` had no set-and-GPU-present scenario, so it could short-circuit a machine that does have one | FR-1.2-S3. This is the silent-green failure D2 exists to prevent, so its absence mattered |
| 18 | Adapter acquired but device request rejected | FR-1.1-S4 |
| 19 | Provenance recorded only on mismatch, never on golden write, leaving a golden's origin adapter unrecoverable | FR-5.1-S4, as a sidecar written next to each golden |
| 20 | "Prefers hardware, may fall back to software" was binding prose with no scenario | FR-1.1-S5, phrased over the available adapters so selection can be a pure function over an enumerated list and therefore testable without two physical GPUs |
| 21 | Nothing said whether a context survives a failed capture, though a test binary captures many frames from one | FR-2.1-S6 |
| 23 | Minimum valid capture (1×1) uncovered | Folded into FR-1.1-S1, which needed an observable outcome anyway — "a context that completes a 1×1 capture returning exactly one pixel" replaces the unobservable "a usable capture context" |

## Accepted in reduced form

**#3 — no threshold has a scenario at its boundary, so inclusive vs exclusive
is undefined for all three.** The ambiguity is real and had to be closed. Two of
the four suggested scenarios landed: FR-3.2-S1/S2 use 4/4096 (0.098%) and
5/4096 (0.122%) against a 0.1% budget, which straddles the boundary exactly.
The per-pixel tolerance and hard ceiling boundaries are closed in FR text
instead — both are **strictly greater than** comparisons — rather than by two
more tests. Reason: the comparison operator is one decision shared by all three
thresholds, and once stated it cannot be satisfied inconsistently while
FR-3.2's pair passes. Two tests for it, not four.

## Rejected

**#22 — "IF a stale artifact file cannot be removed THEN the system SHALL still
report the pass, naming the file it could not remove."** Rejected. Provoking a
removal failure needs filesystem-lock injection machinery disproportionate to
the risk, and the consequence is benign: a stale file lingers beside a passing
test, misleading nobody who reads the failure message that is not there. The
sibling case it was modelled on (FR-4.2-S3, artifact directory unwritable
during a *mismatch*) is kept, because there the artifact is the only record of
what went wrong.

## Guideline violations — all 9 fixed

| Scenario | Fix applied |
|----------|-------------|
| FR-1.1-S1 | Split into S1 (context completes a 1×1 capture) and S2 (adapter name and backend reported). "Usable" was not observable |
| FR-2.3-S1 | Was tautological — returning within the deadline guarantees an elapsed time below it. Now asserts that an elapsed readback time is reported at all |
| FR-3.3-S1 | "SHALL not fail the pair on the ceiling" had no observable surface. Now asserts a match and the stated maximum distance |
| FR-3.1/3.2/3.3 | Concrete source pixel values throughout (gap #7) |
| FR-4.3-S1 | The marking convention was undefined, so no value could be asserted. Fixed as opaque magenta (255, 0, 255, 255), with non-failing positions carrying the expected image's pixel |
| FR-4.2-S1 | Four artifacts in one SHALL. Resolved by defining "the artifact set" as one named contract in FR-4.2's text, so it is one assertion |
| FR-5.1-S1 | Six fields in one SHALL. Same remedy — "the provenance block" is a named contract in FR-5.1's text |
| FR-4.1-S2 | Used the unwanted `IF…THEN` pattern for a state precondition that is not an error. Rewritten as `WHEN` |
| FR-2.3-S2 | Missing the "SHALL NOT return an image" clause its sibling FR-2.1-S5 carries |

## Contradictions — both resolved

1. **FR-4.2-S1 vs FR-4.4-S3.** With `MYCRAFT_UPDATE_GOLDENS` set *and* a
   mismatch, both fired unconditionally: the run would write a failure artifact
   set and overwrite the ground truth it had just failed against, with the
   verdict unspecified. FR-4.2-S1 is now qualified with "and
   `MYCRAFT_UPDATE_GOLDENS` is unset", and FR-4.4-S3/S4 own the update path.
2. **FR-2.3-S1 vs Out of Scope (no performance budgets).** The deadline is a
   liveness bound, not a perf budget; the FR-2.3-S1 rewording removes the
   wording that read as a timing assertion.

## Note on FR-6.1

FR-6.1 (the dependency-graph invariant) carries a single ubiquitous scenario and
no separate unwanted-behavior scenario. This is deliberate rather than an
oversight: the scenario's assertion *is* the negative — the failure it guards
against is the appearance of a forbidden crate in the graph, which the same test
detects. Splitting it would produce two tests asserting one fact.
