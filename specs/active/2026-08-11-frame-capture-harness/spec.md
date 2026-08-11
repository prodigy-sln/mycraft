---
id: SPEC-001
title: Headless Frame-Capture Harness
status: active
rigor: high
branch: feature/PRO-849-frame-capture-harness
jira: PRO-849
created: 2026-08-11
updated: 2026-08-11
author: Claude (sdd-start)
---

# Specification: Headless Frame-Capture Harness

## Goal

Give `mc-testkit` the ability to render a caller-supplied scene to an offscreen
target with no window and no display server, read the pixels back, and compare
them against a committed golden image under an explicit, deterministic
tolerance — writing a readable diff artifact whenever they disagree. This is
the mechanism that makes the renderer verifiable by an agent that cannot look
at a screen, and it is the standing justification for ADR-008's coverage
exclusion of `mc-render`.

Per MyCraft invariant 5, it lands **before** the renderer it verifies. It
therefore proves itself on a trivial scene of its own and has no knowledge of
any game pipeline.

## User Stories

- As the agent building the renderer, I want a captured frame written to disk as
  a PNG I can read, so that I can see my own output without a human at the
  screen.
- As the agent building the renderer, I want a changed frame to fail a test with
  an expected/actual/diff artifact, so that I can tell what changed and where
  without re-running anything.
- As the agent building the renderer, I want the comparison to tolerate driver
  rasterisation noise but not real regressions, so that golden tests are neither
  flaky nor toothless.
- As the maintainer, I want a missing or absent GPU to fail loudly rather than
  skip quietly, so that a green gate never means "verified nothing".
- As the maintainer, I want goldens to change only when someone explicitly asks
  them to, so that a regression can never mint its own ground truth.

## Functional Requirements

Two environment opt-ins are referenced throughout and are part of the contract:

| Opt-in | Effect |
|--------|--------|
| `MYCRAFT_ALLOW_NO_GPU` | Downgrades "no usable adapter" from a failure to an announced skip |
| `MYCRAFT_UPDATE_GOLDENS` | Permits writing golden files, which never happens otherwise |

### Headless GPU acquisition

- **FR-1.1**: The harness acquires a GPU adapter and device without creating a
  window, a surface, or any connection to a display server. Selection prefers a
  hardware adapter and falls back to a software adapter.
  - FR-1.1-S1: WHEN the harness is initialised in a process with no window and
    no display server THE SYSTEM SHALL return a context that completes a 1×1
    capture returning exactly 1 pixel.
  - FR-1.1-S2: WHEN the harness acquires a device THE SYSTEM SHALL report the
    selected adapter's name and backend.
  - FR-1.1-S3: IF no adapter can be acquired on any enabled backend THEN THE
    SYSTEM SHALL return an error naming every backend it tried, without
    panicking and without creating a window.
  - FR-1.1-S4: IF an adapter is acquired but the device request is rejected
    THEN THE SYSTEM SHALL return an error naming the adapter and the
    unsatisfied requirement, without panicking.
  - FR-1.1-S5: WHERE both a hardware and a software adapter are available THE
    SYSTEM SHALL select the hardware adapter.

- **FR-1.2**: A machine without a usable adapter fails capture by default;
  turning that into a skip requires `MYCRAFT_ALLOW_NO_GPU`, which announces
  itself.
  - FR-1.2-S1: IF adapter acquisition fails and `MYCRAFT_ALLOW_NO_GPU` is unset
    THEN THE SYSTEM SHALL surface the failure to the caller as an error rather
    than as a skipped capture.
  - FR-1.2-S2: WHERE `MYCRAFT_ALLOW_NO_GPU` is set and adapter acquisition
    fails THE SYSTEM SHALL report the capture as skipped with a warning
    containing the literal text `MYCRAFT_ALLOW_NO_GPU`.
  - FR-1.2-S3: WHERE `MYCRAFT_ALLOW_NO_GPU` is set and adapter acquisition
    succeeds THE SYSTEM SHALL run the capture and SHALL NOT report a skip.

### Offscreen capture

- **FR-2.1**: The harness renders caller-supplied draw work into an offscreen
  colour target and returns the resulting pixels as 8-bit sRGB-encoded RGBA with
  straight (non-premultiplied) alpha.
  - FR-2.1-S1: WHEN a caller captures a 64×64 frame whose draw work clears the
    target to opaque red THE SYSTEM SHALL return an image in which all 4096
    pixels are (255, 0, 0, 255).
  - FR-2.1-S2: WHEN a caller captures a 64×64 frame whose draw work fills only
    the left half of the target with opaque white over an opaque black clear THE
    SYSTEM SHALL return an image whose pixel at (0, 32) is (255, 255, 255, 255)
    and whose pixel at (63, 32) is (0, 0, 0, 255).
  - FR-2.1-S3: WHEN a caller captures a 64×64 frame whose draw work clears the
    target to white at 25% alpha THE SYSTEM SHALL return an image whose red,
    green and blue channels are 255 in every pixel, unscaled by the alpha value.
  - FR-2.1-S4: WHEN a caller captures a 64×64 frame whose draw work clears the
    target to the colour that encodes to sRGB (128, 128, 128) THE SYSTEM SHALL
    return an image whose every pixel channel is within 1 of 128.
  - FR-2.1-S5: IF the caller's draw work returns an error THEN THE SYSTEM SHALL
    return that error to the caller and SHALL NOT return an image.
  - FR-2.1-S6: WHILE a context is reused after a previous capture returned an
    error THE SYSTEM SHALL return a valid image for the next successful capture.

- **FR-2.2**: A captured image has exactly the requested dimensions and carries
  no GPU row padding.
  - FR-2.2-S1: WHEN a caller captures a 257×129 frame THE SYSTEM SHALL return
    exactly 33 153 pixels, with each row containing exactly 257 pixels.
  - FR-2.2-S2: IF a caller requests a capture whose width or height is 0 THEN
    THE SYSTEM SHALL return an error naming the offending dimension and SHALL
    NOT submit any GPU work.
  - FR-2.2-S3: IF a caller requests a capture whose width exceeds the adapter's
    maximum 2D texture dimension THEN THE SYSTEM SHALL return an error naming
    the requested width and that maximum, and SHALL NOT submit any GPU work.

- **FR-2.3**: Readback from the GPU is bounded by a deadline, defaulting to 30
  seconds.
  - FR-2.3-S1: WHEN a capture completes on a working device THE SYSTEM SHALL
    return the image and report the elapsed readback time.
  - FR-2.3-S2: IF a capture's readback has not completed when its deadline
    elapses THEN THE SYSTEM SHALL return a timeout error naming the capture and
    the deadline, and SHALL NOT return an image.

- **FR-2.4**: A captured image can be written to disk as a PNG that decodes back
  to the same pixels in the same orientation.
  - FR-2.4-S1: WHEN a captured 64×64 image is written to a PNG and decoded again
    THE SYSTEM SHALL yield pixel data identical to the captured image.
  - FR-2.4-S2: WHEN a captured 64×64 image whose left half is opaque white and
    right half is opaque black is written to a PNG THE SYSTEM SHALL produce a
    file whose pixel at (0, 32) decodes to (255, 255, 255, 255) and whose pixel
    at (63, 32) decodes to (0, 0, 0, 255).
  - FR-2.4-S3: IF the target path's directory does not exist and cannot be
    created THEN THE SYSTEM SHALL return an error naming the path and the
    underlying cause, and SHALL NOT report the image as written.

### Image comparison

Distances below are CIE76 ΔE in CIELAB. All three thresholds compare **strictly
greater than**: a pixel fails when its distance is greater than the per-pixel
tolerance, a pair fails the budget when the failing share is greater than the
budget, and fails the ceiling when any distance is greater than the ceiling.
The neutral greys used in these scenarios reduce ΔE to the lightness
difference: (128,128,128) vs (129,129,129) is ΔE ≈ 0.40, vs (140,140,140) is
ΔE ≈ 4.67, vs (180,180,180) is ΔE ≈ 19.72.

- **FR-3.1**: Two images are compared per pixel by CIELAB distance against a
  per-pixel tolerance.
  - FR-3.1-S1: WHEN two 64×64 images are identical except for 3 pixels that are
    (128, 128, 128) in one and (129, 129, 129) in the other, and the per-pixel
    tolerance is ΔE 2.0, THE SYSTEM SHALL report a match.
  - FR-3.1-S2: IF two 64×64 images are identical except for 3 pixels that are
    (128, 128, 128) in one and (140, 140, 140) in the other, and the per-pixel
    tolerance is ΔE 2.0, THEN THE SYSTEM SHALL report those 3 pixels as failing.

- **FR-3.2**: A comparison fails when the share of failing pixels exceeds the
  area budget.
  - FR-3.2-S1: WHEN 4 pixels of a 64×64 image (0.098%) exceed the per-pixel
    tolerance and the area budget is 0.1% THE SYSTEM SHALL report a match and
    state the failing-pixel count as 4.
  - FR-3.2-S2: IF 5 pixels of a 64×64 image (0.122%) exceed the per-pixel
    tolerance and the area budget is 0.1% THEN THE SYSTEM SHALL report a
    mismatch and state both the failing-pixel count and the budget.

- **FR-3.3**: A comparison fails when any single pixel exceeds the hard ceiling,
  regardless of the area budget.
  - FR-3.3-S1: WHEN the largest per-pixel distance in a pair is (128, 128, 128)
    against (140, 140, 140) and the hard ceiling is ΔE 10.0 THE SYSTEM SHALL
    report a match and state ΔE ≈ 4.67 as the maximum per-pixel distance.
  - FR-3.3-S2: IF exactly 1 pixel of a 64×64 image is (128, 128, 128) against
    (180, 180, 180) — a failing share of 0.024%, inside the 0.1% area budget —
    and the hard ceiling is ΔE 10.0 THEN THE SYSTEM SHALL report a mismatch
    naming the ceiling as the reason.

- **FR-3.4**: Comparison is a deterministic pure function of the two images and
  the thresholds, and needs no GPU.
  - FR-3.4-S1: WHEN the same image pair is compared with the two arguments
    swapped THE SYSTEM SHALL report the same verdict, the same failing-pixel
    count and the same maximum distance.
  - FR-3.4-S2: IF the two images have different dimensions THEN THE SYSTEM SHALL
    report a mismatch whose stated reason is the dimension difference, naming
    both sizes, rather than comparing any pixels.
  - FR-3.4-S3: WHILE the process holds no GPU adapter THE SYSTEM SHALL report a
    comparison verdict for two caller-supplied 64×64 images.

- **FR-3.5**: Thresholds default to ΔE 2.0 per pixel, a 0.1% area budget and a
  ΔE 10.0 hard ceiling, and each is overridable per comparison.
  - FR-3.5-S1: WHEN a comparison is requested without explicit thresholds THE
    SYSTEM SHALL apply a per-pixel tolerance of ΔE 2.0, an area budget of 0.1%
    and a hard ceiling of ΔE 10.0.
  - FR-3.5-S2: WHERE the caller sets the per-pixel tolerance to ΔE 0.2 and two
    64×64 images are identical except for 3 pixels that are (128, 128, 128) in
    one and (129, 129, 129) in the other THE SYSTEM SHALL report those 3 pixels
    as failing.
  - FR-3.5-S3: IF a caller supplies a negative per-pixel tolerance THEN THE
    SYSTEM SHALL return an error naming the rejected value.

### Golden-frame lifecycle

FR-4.2 refers to **the artifact set**: the expected image, the actual image, a
diff image and a machine-readable report, written together into the capture's
own artifact directory.

- **FR-4.1**: A capture matching its golden passes and leaves no artifacts
  behind.
  - FR-4.1-S1: WHEN a capture matches its golden within tolerance THE SYSTEM
    SHALL report a pass and SHALL NOT write any file into that capture's
    artifact directory.
  - FR-4.1-S2: WHEN a capture matches its golden and its artifact directory
    still holds files from an earlier mismatch THE SYSTEM SHALL remove those
    stale files.
  - FR-4.1-S3: IF another capture's artifact directory holds files THEN THE
    SYSTEM SHALL leave them untouched while clearing its own.

- **FR-4.2**: A mismatching capture writes the artifact set and points at it.
  - FR-4.2-S1: WHEN a capture mismatches its golden and `MYCRAFT_UPDATE_GOLDENS`
    is unset THE SYSTEM SHALL write the artifact set into that capture's
    artifact directory.
  - FR-4.2-S2: WHEN a capture mismatches its golden THE SYSTEM SHALL name the
    artifact directory in the reported failure.
  - FR-4.2-S3: IF the artifact directory cannot be created or written THEN THE
    SYSTEM SHALL still report the mismatch, with the artifact failure and its
    underlying cause named alongside it.

- **FR-4.3**: The diff image marks exactly the failing pixels and is
  reproducible.
  - FR-4.3-S1: WHEN a diff image is produced for a pair whose 12 pixels exceed
    the per-pixel tolerance THE SYSTEM SHALL set exactly those 12 positions to
    opaque magenta (255, 0, 255, 255) and leave every other position carrying
    the expected image's pixel.
  - FR-4.3-S2: WHEN a diff image is produced twice for the same image pair and
    thresholds THE SYSTEM SHALL produce byte-identical output both times.
  - FR-4.3-S3: IF the two images differ in dimensions THEN THE SYSTEM SHALL omit
    the diff image and record in the report that no diff image could be
    produced.

- **FR-4.4**: A golden is never created or overwritten without
  `MYCRAFT_UPDATE_GOLDENS`.
  - FR-4.4-S1: IF the golden file for a capture does not exist and
    `MYCRAFT_UPDATE_GOLDENS` is unset THEN THE SYSTEM SHALL report a failure
    naming the missing golden path and SHALL NOT create that file.
  - FR-4.4-S2: WHEN the golden file for a capture does not exist and
    `MYCRAFT_UPDATE_GOLDENS` is unset THE SYSTEM SHALL write the captured image
    into the capture's artifact directory.
  - FR-4.4-S3: WHERE `MYCRAFT_UPDATE_GOLDENS` is set and the capture mismatches
    an existing golden THE SYSTEM SHALL overwrite that golden with the captured
    image and report every golden path it wrote.
  - FR-4.4-S4: WHERE `MYCRAFT_UPDATE_GOLDENS` is set and the capture matches its
    existing golden THE SYSTEM SHALL leave that golden's bytes unchanged and
    report no written golden paths.
  - FR-4.4-S5: IF the golden file for a capture exists but cannot be decoded as
    a PNG THEN THE SYSTEM SHALL report a failure naming the path and the decode
    error, and SHALL NOT overwrite that file.

### Reporting and provenance

FR-5.1 refers to **the provenance block**: the adapter name, the backend, the
driver description, the three thresholds applied, the failing-pixel count and
the maximum per-pixel distance.

- **FR-5.1**: Every mismatch report is machine-readable and records the
  environment and thresholds that produced the verdict; every written golden
  records the adapter that produced it.
  - FR-5.1-S1: WHEN a mismatch report is written THE SYSTEM SHALL produce a file
    that parses as JSON whose failing-pixel count is a number.
  - FR-5.1-S2: WHEN a mismatch report is written THE SYSTEM SHALL record the
    provenance block in it.
  - FR-5.1-S3: IF the adapter reports no driver description THEN THE SYSTEM
    SHALL record that field as `unknown` rather than omitting the field.
  - FR-5.1-S4: WHERE `MYCRAFT_UPDATE_GOLDENS` is set THE SYSTEM SHALL record the
    adapter name and backend alongside each golden it writes.

### Structural independence

- **FR-6.1**: The harness does not depend on the code it verifies. This is
  MyCraft invariant 5 made checkable; a structural invariant with no test is a
  comment.
  - FR-6.1-S1: THE SYSTEM SHALL resolve `mc-testkit`'s full dependency graph
    without `mc-render`, `mc-client` or `mc-server` appearing anywhere in it.

## Technical Considerations

### Tolerance model (binding)

Per-pixel CIELAB distance (CIE76 ΔE) after sRGB → linear → XYZ → Lab
conversion, evaluated against three thresholds, each a strictly-greater-than
comparison:

| Threshold | Default | Rationale |
|-----------|---------|-----------|
| per-pixel tolerance | ΔE 2.0 | ΔE ≈ 1.0 is a just-noticeable difference; 2.0 absorbs rounding and dithering differences between adapters while staying invisible |
| area budget | 0.1% of pixels | ~921 px at 1280×720 — enough for anti-aliased edges to drift on a differently-rasterising adapter, far too few to hide a lighting, texture or geometry regression |
| hard ceiling | ΔE 10.0 | A single pixel this far off is a defect, not sampling noise. Without it, a small-area but severe error hides inside the area budget |

Rejected alternatives, with reasons, are recorded in `requirements.md` (D1):
byte-exact comparison (drivers differ), per-channel RGB deltas (equal RGB steps
are not equally visible, so one threshold is simultaneously too tight in shadows
and too loose in midtones), and full contrast-sensitivity metrics (tuning
parameters that are hard to explain when they fire, for precision this project
cannot yet use). CIE76 is chosen over CIEDE2000 because it fits the gate's
30-line/complexity-15 lint budget as straight arithmetic and is accurate enough
at the just-noticeable-difference scale; it can be replaced later behind the
same three-threshold contract.

Determinism holds because comparison never touches the GPU (FR-3.4-S3), and its
only reductions — a count and a maximum — are order-independent.

### Structural constraints

- `mc-testkit` must **not** depend on `mc-render` (FR-6.1). The harness owns
  device acquisition, the offscreen target, readback, comparison and artifacts;
  callers supply draw work. If this spec ever needs the terrain renderer, the
  build order has been inverted and the spec is wrong (invariant 5).
- `mc-testkit` is inside the coverage denominator — ADR-008 excludes only
  `mc-render`, `mc-client` and `mc-server`. The CPU half (comparison, diff
  rendering, report writing, path handling) must therefore be reachable in
  tests without a GPU, which FR-3.4-S3 asserts and which matches
  `crates/mc-render/CLAUDE.md`'s rule that logic stays out of the GPU-touching
  layer.
- The harness fixes one capture format — 8-bit RGBA, sRGB-encoded, straight
  alpha (FR-2.1-S3, FR-2.1-S4) — so captures are comparable across runs, and
  unpads the 256-byte-aligned buffer rows wgpu requires on texture-to-buffer
  copies. FR-2.2-S1 uses 257×129 precisely because both dimensions defeat that
  alignment.
- FR-2.4-S2 exists because a round trip through the harness's own writer and
  reader cancels symmetric errors: a row flip on write plus a row flip on read
  passes FR-2.4-S1 while the PNG on disk is upside-down for the agent who is
  the whole point of user story 1. The asymmetric on-disk assertion is what
  catches it.
- Adapter selection prefers hardware and may fall back to software (FR-1.1-S5);
  whichever is used is recorded (FR-5.1). Making selection a pure function over
  an enumerated candidate list keeps FR-1.1-S5 testable without two physical
  GPUs. Whether hardware and software adapters can share one golden set is left
  to observation — per-adapter golden variants are Out of Scope until drift is
  demonstrated.
- Dependencies: `wgpu`, `image` and `bytemuck` are already pinned in
  `[workspace.dependencies]`. Blocking on `map_async` needs an executor there
  (e.g. `pollster`), and FR-5.1-S1 fixes the report format as JSON, so
  `serde_json` is needed too. Member crates never carry versions.
- Lint budget: `unwrap`, `expect`, `panic!` and raw indexing are denied
  workspace-wide, so every readback and validation path is fallible-by-return —
  FR-2.2-S3 exists because wgpu's own validation failure for an oversized
  texture is panic-shaped. Source files cap at 500 lines, tests at 600.

## Existing Code to Leverage

| What | Location | Reuse |
|------|----------|-------|
| Central dependency pins | `Cargo.toml` `[workspace.dependencies]` | `wgpu` 30, `image` 0.25.10, `bytemuck` 1.25 are already pinned — opt in with `{ workspace = true }` |
| Empty target crate | `crates/mc-testkit/` | The harness's home; currently a dependency-free skeleton |
| Quality gate | `scripts/sdd-gate.ps1` | Coverage exclusion regex and size limits this spec must satisfy |
| Golden-frame contract | `docs/technical/testing.md`, `crates/mc-render/CLAUDE.md` | Perceptual comparison and "unexplained golden update is a review stop" already stated as project policy |

## Out of Scope

Binding. None of the following is built by this spec.

- **Any `mc-render` pipeline work** — no terrain shader, no vertex format, no
  indirect draw, no camera. The harness renders only its own trivial
  self-verification scene. This is the ordering constraint of invariant 5 and
  the single most important exclusion here (PRO-852).
- **Chunk, mesh, block or world types** of any kind (PRO-850, PRO-851).
- **Windowed presentation** — no `winit`, no surface, no swapchain, no
  present-mode handling. Offscreen only.
- **Deterministic replay** — fixed world seed, scripted camera paths and fixed
  tick counts are the scene-side half of golden testing and belong to the specs
  that own a world and a camera (PRO-852, PRO-853).
- **Bot clients, network harnesses, load rigs and the determinism harness** —
  other `mc-testkit` capabilities, specced separately.
- **HUD, `egui` overlays and any game content** (PRO-856).
- **`criterion` benchmarks and performance budgets** for capture. Capture speed
  is not a requirement of this spec; the FR-2.3 deadline is a liveness bound,
  not a performance budget.
- **Per-adapter or per-platform golden variants**, and any golden-set migration
  tooling. One golden per capture until adapter drift is actually observed.
- **CI runner integration.** The gate is run locally today; making capture tests
  run on a hosted runner is a separate concern.
- **Video, animation or multi-frame sequence capture.** Single frames only.
- **Depth, stencil or multi-render-target capture.** One colour target.
- **Perceptual metrics beyond CIELAB ΔE** (SSIM, butteraugli, contrast-
  sensitivity models), and automatic threshold tuning.
- **Recovery from a failed stale-artifact deletion** (audit gap #22, rejected in
  `audit.md`): provoking it needs filesystem-lock injection disproportionate to
  a benign consequence.

## Dependencies

- A GPU adapter reachable through wgpu on the development machine (DX12 or
  Vulkan on Windows 11), or an installed software adapter.
- New `[workspace.dependencies]` entries for a blocking executor and JSON
  serialisation (see Technical Considerations).
- No blocking dependency on any other spec: PRO-849 is the first spec of MVP 1
  and everything downstream depends on it, not the reverse.

## Assumptions

- The development machine exposes a hardware adapter; a software adapter is a
  fallback, not the primary path.
- Goldens committed by this spec are captured on that adapter class, and
  FR-5.1-S4 records which one. Cross-adapter reproducibility is expected to hold
  within the tolerance model, and if it does not, the response is a spec for
  per-adapter goldens, not a widened tolerance.
- Golden PNGs are small enough to live in git without LFS at the sizes this
  harness will use for its self-verification scenes.
- There is no hosted CI; "headless" here means "no window and no display
  server", which is what an SSH session or a service context provides.

## Open Questions

None.

## Clarifications

### Session 2026-08-11

- Q: What tolerance model does golden comparison use? → A: Per-pixel CIELAB
  CIE76 ΔE with three thresholds — per-pixel tolerance 2.0, area budget 0.1%,
  hard per-pixel ceiling 10.0, each a strictly-greater-than comparison. The
  ceiling exists because an area budget alone forgives small severe errors.
- Q: What happens on a machine with no usable GPU adapter? → A: Hard failure by
  default; `MYCRAFT_ALLOW_NO_GPU` downgrades it to a skip that names itself in
  the warning. A silent skip would let the gate go green while verifying
  nothing, which is exactly the risk ADR-008 accepts on the harness's behalf.
- Q: Is a dimension difference an error or a mismatch? → A: A mismatch, with the
  dimension difference as its stated reason. A resized frame is a real
  regression; erroring would blur that with "could not compare".
- Q: May a missing golden be created automatically on first run? → A: No. It
  fails, writes the captured image as an artifact, and names the missing path.
  Writing a golden requires `MYCRAFT_UPDATE_GOLDENS`, and the run reports every
  golden path it wrote.
- Q: What happens when `MYCRAFT_UPDATE_GOLDENS` is set and a capture mismatches?
  → A: The update path owns that case — the golden is overwritten and reported,
  and the mismatch artifact set is not written. Doing both would record a
  failure against ground truth that had already been replaced.
- Q: How does the harness avoid depending on the renderer it verifies? → A:
  Callers supply the draw work; the harness owns everything around it and
  self-verifies on a trivial scene with analytically known pixels, not against a
  committed image. FR-6.1 asserts the dependency graph directly.
- Q: How does a lost device avoid hanging a test run? → A: Every capture carries
  a readback deadline (default 30 s) and returns a timeout error naming the
  capture.
