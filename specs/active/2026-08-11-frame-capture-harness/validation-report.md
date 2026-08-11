# Validation Report: Headless Frame-Capture Harness

**Spec**: [spec.md](spec.md) · **Issue**: PRO-849 · **Rigor**: `high`
**Branch**: `feature/PRO-849-frame-capture-harness` @ `09cbcca`
**Passes**: 1 (`91a7c2f`) and 2 (`09cbcca`) · **Date**: 2026-08-11
**Validator**: validate-PRO-849

## Verdict

**PASS at pass 2 — 0 Blocker, 0 Major, 0 new findings.**

Pass 1 returned 5 Minors and no Blockers or Majors. Four were fixed
(`c4ea0c9`, `d25d81c`, `f717606`, `09cbcca`); M3 was adjudicated and accepted
deliberately by the lead. Pass 2 re-ran the gate and reviewed the whole
changed surface: every fix does what it claims, and no new finding of any
severity appeared. Nothing blocks `/sdd-complete`.

Pass-1 detail is preserved below as the record of what was found and why;
resolutions are marked in place.

## Summary

| Pass | Blocker | Major | Minor | Info |
|---|---|---|---|---|
| 1 — correctness | 0 | 0 | 0 | 0 |
| 1 — coverage | 0 | 0 | 0 | 0 |
| 1 — quality | 0 | 0 | 3 | 0 |
| 1 — validator (independent) | 0 | 0 | 2 | 1 |
| **1 — total** | **0** | **0** | **5** | **1** |
| **2 — new findings** | **0** | **0** | **0** | **0** |

## Gate — pass 2 (`09cbcca`)

`pwsh scripts/sdd-gate.ps1` — **exit 0, GATE PASSED**. Run and observed
directly, as in pass 1.

| Stage | Result |
|---|---|
| format · lint · size · deps · sast · secrets | ok |
| gpu-free (`--no-default-features`) | ok — **67 passed**, 1 skipped |
| tests (`llvm-cov nextest`) | ok — **80 passed**, 1 skipped |
| coverage | ok — **91.08% lines, 90.7% regions, 1121 tracked lines** |

Both counts rose by exactly one against pass 1 (79→80, 66→67), matching the
one test added by `09cbcca`, and coverage rose with the denominator.

## Gate — pass 1 (`91a7c2f`)

`pwsh scripts/sdd-gate.ps1` — **exit 0, GATE PASSED**. Observed directly, not
taken on report.

| Stage | Result |
|---|---|
| format (`cargo fmt --check`) | ok |
| lint + complexity (clippy, zero warnings) | ok |
| gpu-free (mc-testkit, `--no-default-features`) | ok — 66 passed, 1 skipped |
| size (file limits) | ok |
| deps (`cargo machete`) | ok |
| sast (advisories, licenses, bans, sources) | ok |
| secrets (gitleaks) | ok — no leaks |
| tests (`llvm-cov nextest`) | ok — 79 passed, 1 skipped |
| coverage | ok — **90.36% lines, 90.06% regions, 1110 tracked lines** |

The 1110-line denominator is honest: `$CoverageExclude` at
`scripts/sdd-gate.ps1:69` now carries `_test\.rs$`, so the `#[path]`-included
`*_test.rs` siblings no longer inflate the figure. That was the T11 deferred
observation, and it was acted on rather than left open.

## Scenario coverage

**53 of 53 implemented and asserted.** Three independent reviewers each walked
the full set against `test-map.md`, the tests, and the implementation. Every
scenario maps to exactly one test, each test exists under the name and in the
file `test-map.md` records, no scenario is missing or double-mapped, and no
test name carries a spec or scenario ID.

Non-vacuity checks that mattered most, verified independently by the validator:

- **The capture path's row order (FR-2.1-S2)** is genuinely guarded.
  `tests/capture_color.rs:52` drives a real device through
  `tests/scene/mod.rs:173` `top_half_white_over_black`, whose WGSL quad spans
  clip-space `y = 0.0 ..= 1.0`, and asserts white at `(32, 0)` and black at
  `(32, 63)`. `unpad_rows` (`src/frame/readback.rs`) walks rows in order via
  `chunks_exact` and `png.rs` flips nothing on encode or decode, so an
  inversion introduced anywhere in the readback chain reddens this test. This
  is the defect class the harness exists to catch and it is caught.
- **The GPU-free seam holds in substance**, not just by name — see below.
- **`tests/dependency_graph.rs`** BFS-walks cargo's resolved graph and asserts
  it reached `image` before concluding anything about the three excluded
  crates, so an empty closure cannot pass it vacuously
  (`dependency_graph.rs:104-107`).
- **`tests/committed_golden.rs`** `synthetic_frame` varies green down rows and
  red across columns as claimed, so neither a row inversion nor a transposition
  survives it, and the non-ignored test reads the real committed bytes from
  `CARGO_MANIFEST_DIR/goldens`.
- **No phase-3 test skips itself.** `device_context` (`tests/scene/mod.rs:122`)
  turns `Acquisition::Skipped` into an `Err`, and no test in the suite sets an
  environment variable. A machine without an adapter reddens the run.

## The GPU-free seam (FR-6.1)

**Holds.** Verified structurally:

- `crates/mc-testkit/Cargo.toml` makes `wgpu` and `pollster` `optional = true`
  behind a default-on `gpu` feature, so `--no-default-features` removes them
  from the dependency graph rather than leaving them merely unused.
- `src/frame/mod.rs:40,58` cfg-gates `pub mod gpu` and the `wgpu` re-export.
- All five hardware test binaries carry `required-features = ["gpu"]`, so cargo
  declines to build them in the GPU-free configuration instead of compiling
  them into vacuous passes.
- The gate's `gpu-free` stage runs clippy **and** nextest under
  `--no-default-features`, chained with `&&`, and it is green.
- No `mc-*` dependency appears in any section of the manifest.

The comparison suite's "while the process holds no GPU adapter" therefore
asserts what it says, in the one configuration where no adapter *can* exist.

## Findings

### Minor 1 — `architecture.md` documents an `ArtifactError` variant that does not exist — **FIXED (`d25d81c`)**

`specs/active/2026-08-11-frame-capture-harness/architecture.md:902` gives
`ArtifactError` as `Directory` · `File`. The enum at
`crates/mc-testkit/src/frame/golden.rs:100-143` is `Directory`, `Image`,
`Report`, `GoldenNotUpdated`. `File` does not exist; `Image` and `Report` are
absent from the table. The phase-3 addendum two tables down covers
`GoldenNotUpdated`, but the base table was never corrected.

*Effect*: a reader consulting the architecture for `ArtifactError`'s shape is
told about a variant the code does not have and not told about two it does.

### Minor 2 — `architecture.md` still places `SkipNotice` behind the `gpu` feature — **FIXED (`d25d81c`)**

`architecture.md:828` declares `pub struct SkipNotice { ... }` inside the
`### GPU layer (feature = "gpu")` code block. `SkipNotice` is defined
core-side at `crates/mc-testkit/src/frame/selection.rs:83`, with no
`#[cfg(feature = "gpu")]`, per the D13 amendment. D13's own text
(`architecture.md:632`) acknowledges the contradiction and states the rule
overrides it, but the code block itself was never edited the way the
`CaptureContext` field list was, so the stale line is still there to be read.

### Minor 3 — unrelated conductor-tooling changes ride on this feature branch — **ADJUDICATED: accepted deliberately by the lead**

`.claude/agents/sdd-conductor.md` and `.claude/loops/conductor-loop.md` are
modified on this branch across eight commits interleaved with PRO-849 work
(`da7e44b`, `4fd8b0c`, `7dbaecf`, `6cdac86`, `d4dc3af`, `0180326`, `0922d48`,
`508a668`). Nothing in `spec.md` or `tasks.md` authorises them, and
`standards/global/git-workflow.md` §2 bars bundling unrelated changes.

*Effect*: merging PRO-849 carries process-documentation changes into `main` as
a side effect of a merge whose stated condition gates exactly one spec's work.
Not a defect in the harness.

### Minor 4 — a golden that writes but whose sidecar fails is reported as not written — **FIXED (`c4ea0c9`, `09cbcca`)**

> **Correction to this finding's wording, recorded at pass 2.** The text below
> quotes the standing reason as `MissingGolden` ("no golden exists at X"). The
> defect has **two** shapes, not one: when a golden was installed first, the
> standing reason is `Mismatch` ("the capture differs from its golden"), which
> is what the pre-fix measurement in a detached worktree at `ca19469` actually
> returned. Both collapse identically and the fix covers both. The finding was
> correct; its illustration was narrower than the defect.


`crates/mc-testkit/src/frame/golden.rs:267-283` writes the golden image first,
then the provenance sidecar. A sidecar failure returns
`Err(ArtifactError::Report { .. })`, and `on_update` (`golden.rs:246-252`) maps
any `Err` to `self.fail(standing, Err(cause))`.

*Failure scenario*: `MYCRAFT_UPDATE_GOLDENS` is set, the capture mismatches,
the image write succeeds, the sidecar write fails. The golden on disk **has**
been replaced, but the outcome is `GoldenOutcome::Failed` carrying the standing
reason — "no golden exists at X" or "the capture differs from its golden" — and
reports **no** written golden path. FR-4.4-S3 requires the update path to
"report every golden path it wrote". The rendered message reads
`no golden exists at <path>; could not write the report <sidecar>`, whose
leading clause is false at the moment it is printed.

This is recorded in `tasks.md:972` as "Deferred observation, not fixed". My
judgement: **this is a defect filed rather than fixed**, not a legitimate
deferral. It is the same class the lead's ruling 5 already forced fixed in the
sibling case — a caller who set the opt-in is shown a state that is not true
and re-runs into it — surviving here with the sign reversed. It is narrow (it
needs the directory creatable, the image written, and only the JSON to fail),
which is why it is Minor and not Major.

### Minor 5 — FR-2.4-S2's test remains invariant under the transform its rationale names — **FIXED (`f717606`)**

`spec.md` § Technical Considerations states FR-2.4-S2's sole reason to exist:
FR-2.4-S1 is defeated by "a row flip on write plus a row flip on read", which
cancels in a round trip while leaving the PNG on disk upside-down, and "the
asymmetric on-disk assertion is what catches it".

`crates/mc-testkit/tests/png_io.rs:56` writes with `write_png` and decodes with
`read_png` — the harness's own reader — then asserts `(32, 0)` white and
`(32, 63)` black. Under a compensating flip pair in `encode_png` and
`read_png`, the decode returns the original and both assertions still hold,
while the file on disk is inverted. The test is still a round trip.

The 2026-08-11 amendment corrected a real defect: the previous column-split
fixture could not witness row order at all. The corrected row-split fixture
does catch a *single* flip — but `FR-2.4-S1` already caught that, by byte
equality on a fixture asymmetric in both axes. Against the compensating pair,
which is the marginal property S2 was written for, S2 adds nothing. Catching
it requires decoding the file bytes independently of `read_png`.

**Mitigation, and why this is Minor rather than Major**: the property is not
unguarded. `tests/committed_golden.rs:64` compares the golden's bytes *frozen
in git* — minted by unflipped code — against `synthetic_frame()`, whose green
varies down rows. A compensating pair makes `read_png` return
`flip(committed)`, the comparison mismatches, and the test fails. So the
harness would not ship a silent inversion; the gap is that the scenario the
spec designates for this property is not the test that holds it.

## Info

### Info 1 — one `fix:` commit precedes the `test:` commit that locks it in

`10922ea fix: say plainly that a failed update left the golden unwritten`
lands before `50f6510 test: lock in that a failed golden update says so`.
`standards/global/git-workflow.md` §2 specifies `test:` → `feat:`, and
`testing.md` §1 requires failing output before implementation.

Substance is intact — `test-map.md:118` records the test was verified
non-vacuous against the pre-fix code, where it fails on exactly that message —
so this is an ordering artefact of a late lead ruling, not a discipline
failure. Recorded, not counted.

## TDD discipline

**Holds.** Three phases, each `test:` strictly before `feat:`, with clean
ownership separation — no `feat:` commit touches a test file and no `test:`
commit touches implementation:

| Phase | RED | GREEN |
|---|---|---|
| 1 | `a874a01` (9 test files) | `61d221b` (7 src files) |
| 2 | `17dfbd3` (12 test files) | `55eeecc` (10 src files) |
| 3 | `fa5bca9` (6 test files) | `c3dfdcd` (8 src files) |

Test-file corrections (`06fefa2`, `9ba6333`, `50f6510`) are `test:`-typed and
touch only test files, consistent with test-author ownership. The single
ordering exception is Info 1 above.

## Deferred observations in `tasks.md` — validator judgement

| Observation | Judgement |
|---|---|
| T11 coverage denominator measurement (`tasks.md:720`) | **Legitimate, and since resolved.** A measurement handed to the lead as a gate decision, subsequently acted on in `735cfef`. The exclusion is live at `sdd-gate.ps1:69`. |
| Per-file coverage at the phase-1 boundary (`tasks.md:748`) | **Legitimate.** Low files were error paths with no phase-1 caller; the gaps closed in phase 2 by design, and the final figure is 90.36%. |
| T09 PNG determinism held (`tasks.md:756`) | **Not a deferral.** A positive result: the pre-authorised fallback was not needed. |
| ΔE 0.40 is arithmetically 0.3917 (`tasks.md:764`) | **Legitimate, and since resolved.** Raised rather than silently edited, then corrected in `497405a`. `spec.md` now carries full precision. |
| FR-2.4-S2's fixture cannot catch its failure mode (`tasks.md:773`) | **Correctly escalated, and partly resolved.** The test author was right that a fixture change was a spec change and not theirs to make. The amendment landed. But it closed the column-split defect, not the compensating-pair one — see Minor 5. |
| Exactly one golden committed, CPU-generated (`tasks.md:784`) | **Legitimate.** A deliberate design decision that protects the per-adapter deferral, not a deferred defect. |
| Golden writes, sidecar fails, still reported `Failed` (`tasks.md:972`) | **Not a legitimate deferral — see Minor 4.** A defect filed rather than fixed. |

Six of seven are sound. The seventh is Minor 4.

## Scope compliance

**Clean.** `spec.md`'s Out of Scope list holds: no `mc-render` pipeline work,
no chunk/mesh/block/world types, no `winit`/surface/swapchain, no
depth/stencil/MRT, no `criterion` benchmarks, no per-adapter golden variants
(the `default` stem is a constant at `layout.rs:24`), no multi-frame capture.
The self-verification scene and its WGSL live only in `tests/scene/mod.rs`;
the library ships no shaders. The only out-of-scope material on the branch is
the conductor tooling of Minor 3, which is process documentation rather than
product code.

## Documentation consistency

Two surviving document-vs-code contradictions, both in `architecture.md`, both
Minor (findings 1 and 2). Everything else reconciles: the amended
`CaptureContext` field list matches `gpu/acquire.rs`, `GoldenPaths` and
`ArtifactPaths` are `pub(crate)` as documented, the four phase-3 error variants
are present and core-side per D13, the golden-lifecycle state table matches
`golden.rs`, and the JSON report and sidecar shapes match `report.rs` and the
committed `default.provenance.json`.

## What should block `/sdd-complete`

Nothing behavioural. The harness does what the spec says, the gate is green,
and the seam that makes invariant 5 executable holds.

Recommended before completion, in priority order:

1. **Minor 4** — the clearest defect. Either fix it (report the written golden
   path alongside the sidecar failure) or convert the `tasks.md` note into an
   explicit, tracked deferral with a ruling attached.
2. **Minor 3** — decide deliberately whether the conductor-tooling changes
   should reach `main` through this merge.
3. **Minors 1 and 2** — two-line corrections to `architecture.md`, cheapest to
   fix now given `/sdd-complete` consolidates from these documents.
4. **Minor 5** — a judgement call. The property is guarded by
   `committed_golden.rs`, so the options are to give S2 an independent decode,
   or to amend `spec.md` to say the compensating pair is held by the committed
   golden and record what S2 actually covers.

`spec.md`'s y-orientation convention must reach `docs/` at `/sdd-complete`, as
§ Technical Considerations requires.

---

# Pass 2 (`09cbcca`)

Pass 2 accepts only **new** findings of severity Major or higher. **None were
found — at any severity.** The changed surface is six files
(`golden.rs`, `golden_update.rs`, `png_io.rs`, `architecture.md`, `tasks.md`,
`test-map.md`), all reviewed in full.

## Fix verification

### M4 — the new `GoldenOutcome` variant

`GoldenWrittenWithoutProvenance { paths, failure }` was scrutinised as a new
public variant, not accepted on the strength of the finding that prompted it.

- **Genuinely reachable.** `write_golden` (`golden.rs:302-331`) now returns a
  private `GoldenWriteFailure` that records how far it got, and `on_update`
  (`golden.rs:277-292`) maps `Provenance { written, cause }` onto the new
  variant. `golden_update.rs:155` reaches it through a real provocation — the
  sidecar path pre-created as a *directory*, so the image beside it writes
  normally and only the JSON fails. Not a stub, not an injected error.
- **The test cannot silently degrade.** It asserts its own precondition first —
  the golden on disk must equal the captured frame — so it cannot quietly
  become a second copy of the both-writes-failed test the file already carries.
- **FR-4.4-S3 served**: `paths` carries the golden path that was written, and
  the test asserts `paths.contains(&golden)`. The old behaviour reported zero.
- **FR-5.1-S4 served**: the requirement cannot be met when the disk refuses the
  sidecar, so the variant names that failure explicitly rather than reporting
  `GoldenWritten` and implying a provenance record that does not exist.
- **No silent-pass risk.** No consumer maps `GoldenOutcome` to a boolean.
  Every call site (`capture_and_verify.rs:44,54`, `committed_golden.rs:78,107`,
  and the `golden_*` suites) matches explicit variants, so the new one fails
  those assertions rather than slipping through them.
- **The image-write branch is unchanged**, so ruling 5's
  `GoldenNotUpdated` behaviour and its two locking tests still hold — confirmed
  green in the pass-2 gate.

### M5 — the independent decode

`png_io.rs:105` `decode_without_the_harness` reads the file and decodes it
through `image::load_from_memory_with_format` directly, so no `frame::png` code
sits on the read side of the orientation assertion.

Confirmed independently, as asked: `read_png` still appears in that file, but
only at line 46, inside
`an_image_written_to_a_png_decodes_back_to_the_same_pixels` — FR-2.4-S1, which
*is* the round-trip scenario and correctly remains one. It is absent from
FR-2.4-S2's body.

The injection evidence is stronger than the reasoning that prompted the
finding: with a compensating flip pair in `encode_png` and `read_png`,
FR-2.4-S1 passed and the **previous** FR-2.4-S2 passed, while the new one
failed on `[0, 0, 0, 255]` at the top of the file. That settles it as
measurement rather than argument — the old test demonstrably survived exactly
the transform its rationale names, so `spec.md` was right and the test was
underdelivering. No spec amendment was needed, and none was made.

### M1 and M2 — the document corrections

`ArtifactError`'s table now reads `Directory` · `Image` · `Report`, matching
`golden.rs:100-143` with `GoldenNotUpdated` covered by the phase-3 addendum
table. `SkipNotice` moves to the core-types block, and the line where it used
to sit now carries an explicit note that it is *not* part of the feature-gated
block — a correction that survives being read out of context, which the
original ambiguity did not.

`architecture.md`'s frontmatter gains a fourth `amended:` entry, and the
lifecycle section gains a two-failure-point table distinguishing a failed image
write from a failed sidecar. The `tasks.md` deferral is marked resolved with
the original note preserved verbatim beneath it rather than rewritten, which is
the correct handling for append-only task text.

## Verdict

**PASS.** Gate green at `09cbcca`, 53 of 53 scenarios asserted, all four fixes
verified against the code, zero new findings.

Nothing blocks `/sdd-complete`. Carried forward into it: `spec.md`'s
y-orientation convention must reach `docs/`, so PRO-850 and PRO-852 inherit it
rather than rediscovering it.
