# Scenario audit — SPEC-031

Run by `sdd-scenario-auditor` against the 33-scenario draft, at rigor `high`.
**13 gaps, 6 merge/vacuity findings, 4 contradictions.** Every finding is
dispositioned below; the auditor's three load-bearing citations were re-run
before any was acted on.

## Citations re-verified before acting

| Claim | Command | Reading |
|---|---|---|
| `mean_of_stored` rounds half up | `sed -n '178,186p' crates/mc-render/src/texture/mip.rs` | `((total + 2) >> 2)` — so 0,0,255,255 gives **128**, not the 127.5 arithmetic mean |
| A face is culled when what lies beyond occludes | `sed -n '305,320p' crates/mc-world/src/mesh/sweep.rs` | `if occludes(resolved, beyond)? \|\| beyond == key` at `:311` |
| `golden_inventory` already holds the set to `declared_capture_ids` | `sed -n '14,40p' crates/mc-client/tests/no_committed_golden_shows_the_stand_in.rs` | stated at `:16-17`, and the blend tail measured at 7 036–10 293 px per capture |

## Accepted

**Gaps.** G1 inclusive bounds accepted (FR-1.1-S4) · G2 refusals name file and
block (FR-1.2-S2..S5) · G3 concrete non-finite values (FR-1.2-S4, S5) · G4
`opacity < 1.0` with `occludes = true` is refused (FR-1.3-S1) · G5 alpha-less
source and the constant-255 control (FR-3.2-S1, FR-3.1-S2) · G6 save format
(FR-5.3-S1, S2) · G7 reload in the *removal* direction (FR-6.1-S2) · G8
Invariant 1 name-scan with its positive control (FR-7.1-S1, S2) · G9 the model
itself documented, folded into FR-4.1-S2 · G10 the submerged camera (FR-4.3-S1)
· G12 the gameplay page enumerated with a control (FR-7.3-S1, S2) · G13 oracle
independence (FR-5.1-S2).

**Merges and vacuity.** O1 FR-3.1-S1/S2 merged, and two precision corrections
taken: the expected value is the stored mean **rounded half up** (128, not the
127.5 a truncating author would derive) and the rival is named as the
linear-light average rather than as "differs" · O2 the old FR-5.2-S1 **deleted**
— `golden_inventory` already proves it, and the measured hole was the sidecars ·
O3 FR-5.1-S4 re-aimed at the *new* class (blend of two) rather than restating a
property `oracle.rs:60` already holds · O4 FR-2.2-S2 rewritten to an absolute
colour with a hundred-pixel floor · O5, O6 examined and both kept, deliberately.

**Falsifiability.** FR-2.3-S1 replaced by an enumerated verdict plus FR-2.3-S2 as
its rival, because the original passed against an implementation drawing nothing
at all *and* was already green on today's tree · FR-2.1-S4's colour space fixed
(linear light, re-encoded) and bound in the FR-2 preamble · FR-3.2-S1 conditioned
with `WHERE` so it no longer forecloses candidate A1 · FR-4.1-S2 made two-armed
so it cannot pass by having nothing to record · FR-4.2-S1 given an examined-pixel
floor · FR-4.2-S2 given its positive control · FR-5.1-S3 made to name the tree
its expected values were captured on · FR-6.1-S1 given an absolute colour.

## Rejected, with reasons

- **G11 — a scenario asserting the three stale PRO-952 citations now read
  PRO-993.** The correction is kept as a deliverable in Technical
  Considerations, but not as a scenario. A comment's issue key is not a
  behaviour, and the reading would be a grep, which cannot date itself: it
  answers "is it there now" and passes silently for "was it there then". The
  complete phase's doc consolidation is where it is checked.
- **The auditor's choice of `-math.huge` for the ordering discriminator.**
  Replaced with `math.huge`. Negative infinity is below the floor *and*
  non-finite, so it discriminates the same ordering `NaN` already does — the one
  `number.rs` records as settled. Positive infinity is above the **new** ceiling
  and non-finite, so it is the value that discriminates the ordering *this spec
  introduces*. Both `0/0` and `math.huge` are kept (FR-1.2-S4, S5).

## Contradictions, all four closed

1. FR-2 preamble vs FR-2.2-S2 — S2 rewritten; it no longer compares two renderings.
2. FR-3.2-S1 vs Architecture Delta A — conditioned with `WHERE`; A1 is no longer foreclosed.
3. Out of Scope vs FR-4.2-S1 over the submerged ticks — Out of Scope reworded to
   distinguish a whole-frame tint from per-surface blending, and FR-4.3-S1 pins
   the behaviour. It had to be stated **before** the goldens were re-shot.
4. FR-1.2's bounds vs a `0.0` fixture — closed by FR-1.1-S4.

## Budget — over, and confirmed at 44

**Ruled 2026-08-27: keep all 44, cut list not applied.** The standing rule
recorded with the ruling is that **a falsifier removed to satisfy a count is
refused** — the `~40` line is a prompt to justify an addition, not a cap that
retires one. The justification is below and was accepted as the binding part.


**44 scenarios**, against the `~40` at which `scenario-guidelines.md` requires
the user to confirm every addition. The count is not padding: 15 of the additions
close a named gap and 2 scenarios were deleted or merged away. The four
strongest candidates for a cut, if one is wanted, and what each would give up:

| Cut | Gives up |
|---|---|
| FR-2.1-S2 (`opacity = 1.0` draws unblended) | a regression guard already green today; FR-1.1-S4 keeps the loader half |
| FR-4.2-S2 (no seam between adjacent cells) | a regression guard on `beyond == key`, already green |
| FR-3.1-S2 (constant 255 reduces to 255) | the control distinguishing "carried unchanged" from "never read" — **the weakest cut**, it is what makes FR-3.1-S3 mean anything |
| FR-7.3-S2 (gameplay page control) | the positive control on a docs reading; FR-7.2-S2 keeps the pattern elsewhere |

**Recommendation: keep all 44.** Three of the four are the controls and
regression guards that `testing.md` §2 exists to insist on, and the fourth
(FR-2.1-S2) costs almost nothing to keep. 44 is 10 % over a soft threshold whose
purpose is to stop scenario sprawl, and this set is the opposite of sprawl — it
is what an audit found missing.
