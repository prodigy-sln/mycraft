# Validation Report: underwater tint (PRO-998)

**Verdict: PASS** — pass 1, rigor `high`, `review-mode: solo`.
Tree: `feature/PRO-998-underwater-tint` at `b167b27d90c08506b287e10880a56bc82ed9a8cd`.

## Gate

Taken by the conductor, not relayed. `pwsh -NoProfile -File scripts/sdd-gate.ps1`,
with the SHA and `git status --short` captured inside the same invocation on both
sides of the run:

- before: `b167b27d90c08506b287e10880a56bc82ed9a8cd`, status empty, no stashes
- after: `b167b27d90c08506b287e10880a56bc82ed9a8cd`, status empty
- exit code 0

Twelve stages ok: format · lint + complexity (clippy, zero warnings) · gpu-free ·
docs (rustdoc, no broken intra-doc links) · size · deps · sast · secrets · art
(both) · tests · coverage.

`Summary [180.111s] 1721 tests run: 1721 passed (13 slow), 1 skipped` — a **bare**
count, so the run was complete rather than cancelled. Coverage **93.84 % lines**,
92.38 % regions.

Two earlier attempts failed for reasons that were not the code, and both are worth
recording because each produced a log that reads like a broken build:

1. `error: creating test list failed … --list --format terse … The system cannot
   find the file specified (os error 2)`, nextest exit 104. The coverage target
   directory had filled the disk; the build printed `Finished` and never wrote the
   binary. Cleared 2 441 stray `.profraw` files plus 31 GB of *unnamespaced* build
   output left over from before the per-worktree slots — the gigabytes PRO-997
   already predicts a `target/`-only cleaner will miss.
2. A run killed at a session boundary, which left `FAIL … (test failed with exit
   code 1073807364)` in the log. `1073807364` is `DBG_TERMINATE_PROCESS`; the count
   was also slashed (`354/1721`), so it was a cancelled run and worthless either
   way.

## Review

`Workflow({name: "sdd-validate"})`, run `wf_12df19d3-c14`: three specialist
reviewers in parallel over a 101-file manifest, per-finding adversarial
verification, deterministic merge. 3 agents, 0 errors, 0 skipped, **0 empty
results**, 205 tool uses, 791 118 subagent tokens, 604 s.

| reviewer | verdict | Blocker | Major | Minor | Info |
|---|---|---|---|---|---|
| correctness | PASS | 0 | 0 | 0 | 0 |
| coverage | PASS | 0 | 0 | 0 | 0 |
| quality | PASS | 0 | 0 | 0 | 0 |

**The per-reviewer payloads were read, not just the merged verdict** — an absent
reviewer and a clean reviewer are indistinguishable in a merge. All three returned
substantive payloads (3 779 / 3 960 / 6 603 characters) carrying per-scenario
verdicts with `file:line` citations. **All 42 scenarios were verdicted and every
one is PASS.** No dead dimensions, no abstentions, no minor overflow.

**The filtered buckets were checked too** — `findings`, `plausibleFindings`,
`info` and `abstentions` are empty in every reviewer's payload, not merely in the
merge. This matters because a real defect was ranked out of an otherwise clean
PASS earlier in this MVP.

The quality reviewer additionally confirmed scope: every changed file is in the
manifest or is spec-folder bookkeeping, and each item named Out of Scope in
`spec.md` is confirmed absent from the diff.

## Knowingly accepted

Twelve conductor rulings were passed to the reviewers so they would not be refiled
as findings. They remain accepted and are recorded here because the spec folder is
archived rather than kept:

1. **Three scenarios cannot witness the wiring** — FR-2.3-S1, FR-2.4-S1 and
   FR-4.1-S3 all pass on a build that never writes the tint uniform, because a
   zeroed buffer *is* `tint_reach = 0`. FR-4.1-S3 is nonetheless a **correct
   reading**: it is about a refused reload leaving the loaded tint in force, and it
   says so. The correction was to the accounting, not to the test. Measured by
   forcing `reach = NO_TINT` in production: nine of ten readings reddened and that
   one did not.
2. **Five scenarios are red only on a control, and they are one control observed
   five times** — not five witnesses. They go green together and stay red together.
3. **FR-2.2-S1 must not be weakened.** The five controls assert only
   `differing(...) > 0`, which accepts a draw path writing garbage; FR-2.2-S1 is
   what constrains the tinted picture. Weaken it and five controls hollow out with
   nothing going red.
4. **`TELLS_THEM_APART = 3.0` is measured, not borrowed** — band 1.19 < T < 7.45 by
   three independent instruments. A correct frame is off by up to ΔE 1.19 through
   the 8-bit sRGB attachment, so any tolerance below that goes red on correct code.
5. **`tint_reach` is the literal `0.0` when dry, with no branch in the fragment.**
   That is what makes FR-3.1-S1 a byte-for-byte claim; an `if` there leaves every
   other reading green while the claim degrades to a tolerance claim.
6. **`SCENE_REVISION` deliberately did not move**, and no capture moved.
7. **`Tinting` is structural**: one `srgb8_to_linear` call feeds both the uniform
   and the clear, and a second decode parts the sky from the far terrain by a
   transfer function.
8. `upload_frame` takes `&RecordTarget` to stay inside the arity cap of 4.
9. **FR-6.1-S1 was green on arrival and RED was unavailable for it**, because it
   reads over code an earlier phase had already landed. Reported rather than
   contrived; a mutation stood in, and the sibling scan staying green is the
   measurement showing the new scan is additive rather than a second copy.
10. `terrain.wgsl` is the weak member of the Invariant 1 source set, kept with the
    reason written down. A shader special-casing a block by **layer index** carries
    no name and still slips past.
11. `seam_boundaries.rs` stands at 608 lines and was over the cap before this spec
    touched it.
12. **The 0.38-block submersion margin is known debt** — a later change to
    `EYE_HEIGHT` or to the sea's depth kills submersion silently, and FR-2.6-S2 is
    the only thing that would say so.

## Evidence produced during implementation

Recorded here because it is the reason the verdict can be trusted, and because the
spec folder does not survive as documentation:

- **The radial law is proved by mutation.** Carrying the tint by depth instead of
  radial distance moved 6 of 1710 tests. Four were the readings the guard names;
  two — FR-2.2-S1 and FR-2.4-S2 — were not predicted and stand over the **shipped
  sea** at ΔE 2.0, tighter than the fixture's 3.0. So the property has six
  witnesses, two of them on the world a player stands in.
- **The re-mesh guard was blind and is not any more.** `reload.rs` promised that
  `reload_marks_sections.rs` reddens when a new property joins the geometry key.
  Adding `tint` to that key moved **zero** of 1710 tests. With the reading added it
  moves **exactly one**, and that one is the new reading — which is what shows
  nothing else in the workspace rested on that key by accident.
- **Two defects were found and repaired inside the spec**: a grid predicting every
  sample at the centre's distance (worst ΔE 9.751 against a 3.0 tolerance, whose
  cheapest green was to tint by depth — the very defect another scenario exists to
  catch), and two goldens minted through a path hardcoding `tint: None`, which is a
  golden minted from the thing it verifies.

## Verdict

```
Validation PASS — pass 1
Gate: green · Findings: B0 M0 m0 Info0
```

User sign-off does not apply: this spec ran in conductor mode, where the conductor
is the approving authority.
