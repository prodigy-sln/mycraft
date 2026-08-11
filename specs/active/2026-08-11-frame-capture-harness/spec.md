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
    the **top** half of the target with opaque white over an opaque black clear
    THE SYSTEM SHALL return an image whose pixel at (32, 0) is
    (255, 255, 255, 255) and whose pixel at (32, 63) is (0, 0, 0, 255).
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
  - FR-2.4-S2: WHEN a captured 64×64 image whose **top** half is opaque white
    and **bottom** half is opaque black is written to a PNG THE SYSTEM SHALL
    produce a file whose pixel at (32, 0) decodes to (255, 255, 255, 255) and
    whose pixel at (32, 63) decodes to (0, 0, 0, 255).
  - FR-2.4-S3: IF the target path's directory does not exist and cannot be
    created THEN THE SYSTEM SHALL return an error naming the path and the
    underlying cause, and SHALL NOT report the image as written.

### Image comparison

Distances below are CIE76 ΔE in CIELAB. All three thresholds compare **strictly
greater than**: a pixel fails when its distance is greater than the per-pixel
tolerance, a pair fails the budget when the failing share is greater than the
budget, and fails the ceiling when any distance is greater than the ceiling.
The neutral greys used in these scenarios reduce ΔE to the lightness
difference: (128,128,128) vs (129,129,129) is ΔE ≈ 0.39, vs (140,140,140) is
ΔE ≈ 4.67, vs (180,180,180) is ΔE ≈ 19.72. To full precision these are 0.39168,
4.66505 and 19.72703 (see Clarifications).

- **FR-3.1**: Two images are compared per pixel by CIELAB distance against a
  per-pixel tolerance.
  - FR-3.1-S1: WHEN two 64×64 images are identical except for 3 pixels that are
    (128, 128, 128) in one and (129, 129, 129) in the other, and the per-pixel
    tolerance is ΔE 2.0, THE SYSTEM SHALL report a match.
  - FR-3.1-S2: IF two 64×64 images are identical except for 3 pixels that are
    (128, 128, 128) in one and (140, 140, 140) in the other, and the per-pixel
    tolerance is ΔE 2.0, THEN THE SYSTEM SHALL report those 3 pixels as failing.

- **FR-3.2**: A comparison fails when the share of failing pixels exceeds the
  area budget. These scenarios use a 320×180 image (57 600 pixels), because
  0.01% of a 64×64 image is 0.41 pixels — too small to express a boundary at
  all, since any single failing pixel would breach it.
  - FR-3.2-S1: WHEN 5 pixels of a 320×180 image (0.0087%) exceed the per-pixel
    tolerance and the area budget is 0.01% THE SYSTEM SHALL report a match and
    state the failing-pixel count as 5.
  - FR-3.2-S2: IF 6 pixels of a 320×180 image (0.0104%) exceed the per-pixel
    tolerance and the area budget is 0.01% THEN THE SYSTEM SHALL report a
    mismatch and state both the failing-pixel count and the budget.

- **FR-3.3**: A comparison fails when any single pixel exceeds the hard ceiling,
  regardless of the area budget.
  - FR-3.3-S1: WHEN the largest per-pixel distance in a pair is (128, 128, 128)
    against (140, 140, 140) and the hard ceiling is ΔE 10.0 THE SYSTEM SHALL
    report a match and state ΔE ≈ 4.67 as the maximum per-pixel distance.
  - FR-3.3-S2: IF exactly 1 pixel of a 320×180 image is (128, 128, 128) against
    (180, 180, 180) — a failing share of 0.0017%, inside the 0.01% area budget —
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

- **FR-3.5**: Thresholds default to ΔE 2.0 per pixel, a 0.01% area budget and a
  ΔE 10.0 hard ceiling, and each is overridable per comparison. A comparison
  that loosens a default records the reason.
  - FR-3.5-S1: WHEN a comparison is requested without explicit thresholds THE
    SYSTEM SHALL apply a per-pixel tolerance of ΔE 2.0, an area budget of 0.01%
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
| area budget | 0.01% of pixels | ~92 px at 1280×720. Absorbs isolated rounding and dithering, nowhere near a block-sized artifact — see below |
| hard ceiling | ΔE 10.0 | A single pixel this far off is a defect, not sampling noise. Without it, a small-area but severe error hides inside the area budget |

The area budget is deliberately an order of magnitude tighter than the 0.1%
that would suit an anti-aliasing-heavy renderer, because MVP 1's frames are not
that. PRO-851 is a **binary greedy mesher**, whose entire purpose is merging
coplanar faces into large quads — it maximises flat area and minimises edge
count. PRO-852 is flat shading with procedural placeholder textures and one
block family, and no MSAA or anti-aliasing appears anywhere in `docs/`,
`PLAN.md` or the crate `CLAUDE.md` files for MVP 1. The soft-edge drift a 0.1%
budget exists to absorb is largely absent; the hard silhouettes that do exist
(sky against terrain) flip whole pixels at very high ΔE and are caught by the
ceiling, not the budget.

The deciding number: 0.1% at 1280×720 is 921 pixels, and a single block face at
mid distance is comfortably 900+ pixels. That budget could have forgiven an
entire wrong block face so long as each pixel stayed under ΔE 10 — which is
exactly the shape of a same-family texture regression. At 0.01% it cannot.
Since MVP 1 runs on one machine and one adapter, and same-adapter rendering is
deterministic for identical command streams, the tighter budget should not cost
flakiness. FR-3.5's per-comparison override is the release valve: a later spec
may loosen it **with a recorded reason**. Start tight and loosen on evidence,
never the reverse — the reverse is how a golden suite rots into a rubber stamp.

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
- **Row-axis asymmetry is required of any content assertion.** Any scenario
  asserting image *content* must use a fixture that is asymmetric on the **row**
  axis, unless it states why row order is irrelevant to what it asserts. This is
  a structural rule, not a style note: three separate scenarios in this spec were
  written with fixtures invariant under the exact row-order inversion they
  existed to detect (FR-2.4-S1 self-cancelling, caught by the scenario audit;
  FR-2.4-S2 and FR-2.1-S2 column-split, caught during phase-1 implementation).
  Three occurrences is a pattern, so the requirement moves from something a
  reviewer must remember to something this document demands. Uniform fixtures
  satisfy it by stating their exemption: FR-2.1-S1, FR-2.1-S3 and FR-2.1-S4
  assert colour *format* — channel values, alpha handling, sRGB encoding — and
  not layout, so row order cannot affect their outcome.

- **The y-orientation convention is a caller contract, not an internal detail.**
  The harness takes caller-supplied draw work, so which way is up is part of its
  public interface. Stated plainly, and binding on every caller:
  - **Framebuffer row 0 is the top** of the image, and stays the top through
    readback, comparison, PNG encode and PNG decode. No stage flips rows.
  - **Clip-space y is up.** wgpu's framebuffer origin and clip-space y point in
    opposite directions, which is one of the most common sources of flipped
    output in the ecosystem.
  - Consequently **a caller filling the top half of the target writes y > 0**.

  FR-2.1-S2 is what holds this honest, and it is the only scenario that does:
  the readback chain (`copy_texture_to_buffer` → row unpadding) is simultaneously
  the only place a row inversion plausibly originates and — before this scenario
  was given a row-split fixture — the only place nothing asserted against one.
  A capture path that inverts rows would make **every golden this project ever
  commits wrong in the same direction**, consistently and therefore invisibly.
  That is worse than having no harness, because it is confidently wrong: the
  agent building the renderer would chase phantom geometry bugs against ground
  truth that is itself upside-down. Settling the convention here, against
  computed ground truth on a 64×64 image, is far cheaper than settling it
  against terrain, where a vertically mirrored world looks entirely plausible
  and would be blamed on worldgen, the camera or the mesher first.

  **This convention must be consolidated into `docs/` at `/sdd-complete`** so
  the renderer spec inherits it rather than rediscovering it.

- The harness fixes one capture format — 8-bit RGBA, sRGB-encoded, straight
  alpha (FR-2.1-S3, FR-2.1-S4) — so captures are comparable across runs, and
  unpads the 256-byte-aligned buffer rows wgpu requires on texture-to-buffer
  copies. FR-2.2-S1 uses 257×129 precisely because both dimensions defeat that
  alignment.
- FR-2.4-S2 exists because a round trip through the harness's own writer and
  reader cancels symmetric errors: a row flip on write plus a row flip on read
  passes FR-2.4-S1 while the PNG on disk is upside-down for the agent who is
  the whole point of user story 1. The asymmetric on-disk assertion is what
  catches it. **The fixture must therefore be split across rows, not columns.**
  PNG is a row-ordered format, so row-order inversion is the realistic bug; a
  column mirror is not a failure mode any encoder or unpadding step plausibly
  produces. A left/right split asserted on one row is invariant under precisely
  the transform the scenario exists to detect — flip the rows and the left half
  is still white at every row — which is why the fixture is top/bottom and the
  assertion names two rows of the same column.
- Adapter selection prefers hardware and may fall back to software (FR-1.1-S5);
  whichever is used is recorded (FR-5.1). Making selection a pure function over
  an enumerated candidate list keeps FR-1.1-S5 testable without two physical
  GPUs. Per-adapter golden variants are Out of Scope here but not hypothetical
  — `crates/mc-render/CLAUDE.md` already makes them project policy for any
  discrete-only feature's fallback path. The golden path and naming convention
  must therefore leave room for an adapter discriminator from the start, so
  that adding variants later is a new file rather than a migration of every
  existing golden.
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
  tooling. One golden per capture. This deferral is conditional, not open-ended:
  it ends at **a second adapter running the gate, or the first discrete-only
  feature with a fallback path, whichever comes first**. `mc-render`'s
  `CLAUDE.md` already requires a fallback path to carry its own golden frame, so
  the policy exists — MVP 1 simply ships no discrete-only feature, since
  flat-shaded placeholder terrain renders identically on an Intel UHD 770 and an
  RTX 4090. FR-5.1-S4 is what makes the trigger actionable: every written golden
  records the adapter that produced it, so the day a second adapter appears, the
  existing set's provenance is already known.
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
- The golden path and naming convention leave room for an adapter discriminator
  even though nothing uses one yet, so that the deferral above can end by adding
  files rather than by renaming the whole set.
- Golden PNGs are small enough to live in git without LFS at the sizes this
  harness will use for its self-verification scenes.
- There is no hosted CI; "headless" here means "no window and no display
  server", which is what an SSH session or a service context provides.

## Open Questions

None.

## Clarifications

### Session 2026-08-11

- Q: What tolerance model does golden comparison use? → A: Per-pixel CIELAB
  CIE76 ΔE with three thresholds — per-pixel tolerance 2.0, area budget 0.01%,
  hard per-pixel ceiling 10.0, each a strictly-greater-than comparison. The
  ceiling exists because an area budget alone forgives small severe errors.
- Q: Why is the area budget 0.01% rather than the 0.1% a renderer usually wants?
  → A: MVP 1 has no anti-aliasing and a greedy mesher that maximises flat area,
  so there is little soft-edge drift to absorb. At 0.1%, the 921-pixel budget at
  720p exceeds the size of one block face, so it could have hidden an entire
  wrong face. Loosening later requires a recorded reason (FR-3.5).
- Q: When does the per-adapter golden deferral end? → A: At a second adapter
  running the gate, or the first discrete-only feature with a fallback path,
  whichever comes first. Not "when drift is observed" — that has no owner.
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

### Session 2026-08-11 (phase-1 implementation)

- Q: FR-2.4-S2's fixture was left half white / right half black, asserted at
  (0, 32) and (63, 32). Does that catch the failure the scenario exists for? →
  A: **No, and the fixture has been corrected to top/bottom.** A row-order flip
  maps row 32 to row 31, where the left half is still white and the right still
  black, so the assertion was invariant under precisely the transform it was
  written to detect. It caught a column mirror, which no encoder or unpadding
  step plausibly produces. The corrected fixture splits across rows and asserts
  (32, 0) and (32, 63). FR-2.4-S1's fixture is asymmetric in both axes and
  already catches any *single* flip, so S2's remaining job is the compensating
  write-flip-plus-read-flip pair — a row-axis phenomenon.
- Q: Does anything assert that the **capture** path preserves row order? → A:
  **It does now — FR-2.1-S2, whose fixture changed from a left/right split to a
  top/bottom one for that reason.** It previously did not, and the gap was the
  most consequential defect found in this spec. Every other phase-3 assertion is
  uniform (FR-2.1-S1/S3/S4), a count (FR-1.1-S1, FR-2.2-S1) or a duration
  (FR-2.3-S1); none is sensitive to row order. FR-2.4-S2 does not close it
  either, since it runs a hand-built image through the PNG writer and reader
  with no capture in the path. So the readback chain
  (`copy_texture_to_buffer` → row unpadding) was simultaneously the only place a
  row inversion plausibly originates and the only place nothing asserted against
  one, while "no vertical flip at any stage" was already binding. The scenario's
  purpose is unchanged — a half-fill needs the same real pipeline and draw on
  either axis — and the change additionally forces the caller's WGSL to get
  clip-space y right, which is where the ecosystem's flipped-output bugs come
  from and is better settled here than against terrain.
- Q: Are the three reference ΔE values quoted under "Tolerance model" exact? →
  A: They are rounded, and one of them rounds the wrong way. To full precision,
  CIE76 over CIELAB (D65, sRGB transfer function) gives (128,128,128) vs
  (129,129,129) = **0.39168**, vs (140,140,140) = **4.66505**, vs (180,180,180)
  = **19.72703**. The 4.67 and 19.72 figures round correctly; the 0.40 did not
  and now reads 0.39. A conforming implementation lands on 0.392, which is
  correct rather than drift — recorded so a reviewer does not read it as one.
  No threshold, scenario or verdict changes: 0.39 and 0.40 sit on the same side
  of every threshold in this spec.
