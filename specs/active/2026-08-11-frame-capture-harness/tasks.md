# Tasks: Headless Frame-Capture Harness

**Spec**: [spec.md](spec.md) · **Architecture**: [architecture.md](architecture.md) ·
**Branch**: `feature/PRO-849-frame-capture-harness` · **Issue**: PRO-849 ·
**Rigor**: `high` · **Created**: 2026-08-11

One task = one coherent scenario group in one area. `[P]` = independent of other
`[P]` tasks in the same phase. Scenario IDs appear **here and in commit messages
only** — never in code, test names or file names. The scenario↔test mapping is
recorded in `test-map.md` by the test author during `/sdd-implement`.

53 scenarios across 17 FRs, all assigned exactly once. See
[Scenario assignment](#scenario-assignment) for the proof.

## Phasing rationale

The phase boundary is a design decision, not a convenience (architecture.md,
"Build order"). Phases 1 and 2 contain **only the 41 GPU-free scenarios**; the 12
that observe real hardware are quarantined in phase 3. Two consequences that are
binding:

- **No GPU-free scenario is blocked behind a GPU-requiring one.** The entire
  deterministic CPU body — comparison, diffing, PNG, golden lifecycle, reporting,
  the pure adapter decisions — lands and goes green before any adapter behaviour
  enters the process.
- **Phases 1 and 2 must be green under `--no-default-features`.** `mc-testkit`
  carries no `wgpu` dependency at all until phase 3, which is the strongest
  possible form of the seam. From phase 3 on, the `gpu` feature is what keeps it
  true, and the gate stage added in T02 is what keeps *that* honest.

The GPU-free work is split across two phases because 41 scenarios is more than
one test author can hold as a coherent interface design, and because there is a
real dependency boundary between them: the golden lifecycle (phase 2) consumes
comparison, diffing, PNG and reporting (phase 1) as finished primitives.

- **Phase 1 — the pixel pipeline** (T01–T11). Image, colour, comparison, diff,
  PNG, row unpadding, the structural invariant. 21 scenarios.
- **Phase 2 — policy, lifecycle and reporting** (T12–T22). Opt-ins, adapter
  decisions, deadline, paths, report, golden lifecycle, and the one committed
  golden. 20 scenarios.
- **Phase 3 — the wgpu adapter** (T23–T28). 12 scenarios, every one an
  observation of real hardware.

28 tasks in total. Ten carry no scenario — T01, T02, T04, T05, T11, T12, T21,
T22, T23, T28 — and are enabling or supporting work, each with its reason stated
in the task.

### Definition of done, phases 1 and 2

All four, at each phase boundary:

1. `cargo nextest run -p mc-testkit --no-default-features` green
2. `cargo clippy -p mc-testkit --no-default-features --all-targets -- -D warnings` clean
3. `pwsh scripts/sdd-gate.ps1` exits 0
4. `grep -rn 'wgpu::\|std::env' crates/mc-testkit/src` returns nothing outside
   `frame/gpu/**` and `frame/optins.rs` — in phase 1 it must return nothing at all

Item 1 is the load-bearing one: it is the only process in which no adapter *can*
exist, which is what makes FR-3.4-S3 assert what it says. `cargo check` is not a
substitute — the configuration must be **run**.

### Definition of done, phase 3

The four above, plus:

5. `cargo nextest run -p mc-testkit` (default features, `gpu` on) green
6. `cargo clippy -p mc-testkit --all-targets --all-features -- -D warnings` clean

Item 1 stays in the list. Once `wgpu` is linked in the default build, item 1 is
the only run in which FR-3.4-S3 still means what it says.

## Rulings applied (from the lead — binding inputs, not open questions)

1. **The gate gains the `--no-default-features` run** (T02), in its own commit,
   touching only the lint/test stage.
2. **Alpha sits outside the comparison metric.** No task; the declaration stays in
   the docs with its revisit trigger (first golden containing non-opaque pixels).
3. **`CaptureId` validation is kept** (T12) — path-traversal guard on a public
   input, `code-quality.md` §7. No scenario, and that is fine.
4. **The coverage-denominator check is measurement only** (T11). Report the
   number; do **not** touch `$CoverageExclude`.
5. **`.gitattributes` is added and exactly one golden is committed** (T21, T22).
   The golden is CPU-generated, never a capture. Rationale in the tasks.
6. **The T09 PNG-determinism fallback is pre-authorised** — byte-identity on the
   diff buffer before encoding *plus* decoded-pixel identity after the round
   trip, never a decoded-pixel assertion alone. Recorded as a Clarification in
   `spec.md` if taken, and reported to the lead either way.

Rulings 5 and 6, and the core-side placement of `AcquireError`, `CaptureError`
and the provenance normalisation, are folded into `architecture.md` (D13, the
Integration table and the Data section) so the two documents cannot disagree.

Approved dependencies: `pollster = "0.4"`, `serde_json = "1.0"`,
`tempfile = "3"` (dev-only), and narrowing `image` to
`default-features = false, features = ["png"]`. `bytemuck` is excluded and must
not be added. **All pinned in `[workspace.dependencies]` only**; member crates
opt in with `dep = { workspace = true }`. A version literal in
`crates/mc-testkit/Cargo.toml` is a review-stop.

## Test author / implementation split (rigor `high`)

- The test author owns **every file under `crates/mc-testkit/tests/` and every
  `*_test.rs` sibling** for the phase, writes them before any implementation, and
  keeps ownership for the whole phase. The implementation context never edits
  them; a disputed failure goes to arbitration.
- Tests bind to the signatures in architecture.md § Interfaces **exactly as
  written**. A signature that turns out to be wrong is a dispute, not a
  unilateral edit on either side.
- Test placement follows D10: behavioural and integration tests in `tests/`
  (600-line budget); private plumbing through a
  `#[path = "x_test.rs"] mod tests;` sibling (also 600, and it sees private
  items). An inline `#[cfg(test)] mod tests` counts against the 500-line source
  limit and is not used.
- Tasks marked `Scenarios: none` are enabling or supporting work. They still
  carry tests (unit tests for the plumbing they add) — they simply own no
  acceptance scenario.

---

## Phase 1 — GPU-free pixel pipeline

21 scenarios. Nothing here links `wgpu`.

- [x] **T01** Crate skeleton and dependency wiring — root `Cargo.toml`,
      `crates/mc-testkit/Cargo.toml`, `src/lib.rs`, `src/frame/mod.rs`
      Scenarios: none (enabling)
      - Root `[workspace.dependencies]`: add `pollster = "0.4"`,
        `serde_json = "1.0"`, `tempfile = "3"`; narrow `image` to
        `{ version = "0.25.10", default-features = false, features = ["png"] }`.
      - `crates/mc-testkit/Cargo.toml`: `image`, `serde_json`, `thiserror` as
        `{ workspace = true }`; `tempfile` under `[dev-dependencies]`. No version
        literals. No `mc-*` dependency in any section, ever (FR-6.1).
      - `src/lib.rs` gains `pub mod frame;`. `frame/mod.rs` states the seam
        contract in module docs: the core never invokes a GPU, and `wgpu::` may
        appear only under `frame/gpu/`.
      - **`wgpu`, `pollster`, `serde` and the `[features]` stanza are deliberately
        NOT added here.** `cargo machete` (gate stage 4) fails a declared
        dependency with no `use`, including an optional one. `serde` arrives with
        T16, `wgpu`/`pollster` and the `gpu` feature with T21. This is sequencing,
        not a departure from D1 — the seam is stronger in phases 1–2 than the
        feature makes it, because the dependency is absent entirely.

- [x] **T02** `[P]` Gate: run `mc-testkit` in the `--no-default-features`
      configuration — `scripts/sdd-gate.ps1` (lint/test stage only)
      Scenarios: none (approved ruling 1)
      - Its own commit, touching nothing else in the gate. Suggested shape: a new
        stage immediately after `lint + complexity`, invoking
        `cargo clippy -p mc-testkit --no-default-features --all-targets -- -D warnings`
        and `cargo nextest run -p mc-testkit --no-default-features`.
      - **Trap:** `Invoke-Stage` inspects `$LASTEXITCODE` after the scriptblock, so
        two statements on separate lines only check the second. Chain them with
        `&&` (PowerShell 7 pipeline chain operator) so a first-command failure
        still surfaces.
      - Do not pass `--all-features` here — it would re-enable `gpu` and make the
        stage meaningless. `-p mc-testkit` keeps the cost to one small crate.
      - If adding it breaks the gate for a reason visible only at the keyboard,
        **stop and ask the lead** rather than adjusting the gate further.

- [x] **T03** `[P]` Image container and frame-size validation — `frame/image.rs`,
      `tests/`
      Scenarios: FR-2.2-S2, FR-2.2-S3
      - `Rgba8Image` (`from_rgba`, `width`, `height`, `pixel`, `as_bytes`) with
        the format contract in module docs: 8-bit RGBA, sRGB-encoded, straight
        (non-premultiplied) alpha, **row 0 = top, no stage of this crate flips
        rows or touches alpha** (D5).
      - `validate_frame_size(width, height, max_dimension) -> Result<FrameSize, _>`
        and `FrameSizeError::{ZeroDimension, TooLarge}`. Pure — it exists so
        FR-2.2-S2/S3's "SHALL NOT submit any GPU work" is structural in phase 3,
        and because wgpu's own oversized-texture validation is panic-shaped.
      - `ImageShapeError` for a pixel buffer whose length is not `w * h * 4`.

- [x] **T04** `[P]` sRGB → CIELAB conversion and ΔE — `frame/color.rs`,
      `frame/color_test.rs`
      Scenarios: none (supporting; the distances it produces are asserted by T07)
      - `srgb8_to_lab([u8; 3]) -> Lab` via a 256-entry `u8 → linear f64` LUT
        computed once, then XYZ (D65) → Lab. `delta_e(Lab, Lab) -> f64`, CIE76.
        `f64` throughout.
      - **`delta_e` is the only place a distance is computed** and the only
        CIEDE2000 swap point (D9). No `Metric` trait. `compare.rs` must never
        inspect `L*`, `a*`, `b*` or a raw channel.
      - Unit tests pin the three reference distances the spec's comparison
        scenarios are built on: (128,128,128) vs (129,129,129) ≈ 0.40, vs
        (140,140,140) ≈ 4.67, vs (180,180,180) ≈ 19.72. If the implementation
        cannot reproduce them, the spec's scenarios are unsatisfiable — escalate,
        do not adjust the expected numbers.
      - The LUT also makes conversion bit-reproducible run to run, which
        FR-3.3-S1's stated maximum and FR-4.3-S2's byte-identity both lean on.

- [x] **T05** `[P]` Row unpadding and 256-byte alignment — `frame/readback.rs`,
      `frame/readback_test.rs`
      Scenarios: none (FR-2.2-S1 is owned end-to-end by T25)
      - `padded_row_bytes(width) -> Result<u32, ReadbackError>` (private) and
        `unpad_rows(padded, row_bytes, padded_row_bytes, height) -> Result<Vec<u8>, _>`
        (public).
      - `integer_division` is lint-denied: use `(width * 4).next_multiple_of(256)`,
        never `(x + 255) / 256 * 256`. `indexing_slicing` is denied: use
        `chunks_exact` / `get`.
      - Unit tests must include the 257×129 shape (`row_bytes = 1028`,
        `padded = 1280`) so the padding arithmetic is proven before a device is
        ever involved. The **scenario** FR-2.2-S1 is a real capture and belongs to
        phase 3; this task proves the maths it depends on.

- [x] **T06** `[P]` Structural independence from the code under test —
      `tests/dependency_graph.rs`
      Scenarios: FR-6.1-S1
      - Invoke `cargo metadata --format-version 1 --locked` through the `CARGO`
        environment variable cargo sets for test binaries (never a hardcoded
        `cargo`), with cwd `CARGO_MANIFEST_DIR`; parse with `serde_json`; BFS from
        the `mc-testkit` node over `resolve.nodes[].deps`, following **all**
        dependency kinds; assert `mc-render`, `mc-client` and `mc-server` appear
        nowhere in the closure.
      - Parsing `Cargo.toml` (direct deps only) or `Cargo.lock` (every workspace
        member, so the assertion is vacuously false) are both wrong answers — D11.
      - **This test is expected to pass on its first run.** Its RED state is a
        manifest that violates FR-6.1, which is not a state we create on purpose.
        Landing it early is the point: it guards the invariant for the rest of the
        spec rather than certifying it at the end.

- [x] **T07** Comparison: thresholds, verdict, area budget, hard ceiling —
      `frame/compare.rs`, `tests/`
      Scenarios: FR-3.1-S1, FR-3.1-S2, FR-3.2-S1, FR-3.2-S2, FR-3.3-S1,
      FR-3.3-S2, FR-3.4-S1, FR-3.4-S2, FR-3.5-S1, FR-3.5-S2, FR-3.5-S3
      Depends on: T03, T04
      - `Thresholds` (private fields, validating constructor rejecting negative,
        NaN and infinite values by field name — FR-3.5-S3), `Default` = 2.0 /
        0.0001 / 10.0, `Verdict`, `MismatchReason`, `FailingMask`, `Comparison`,
        and `compare(expected, actual, thresholds) -> Comparison`.
      - **`compare` never fails.** A dimension difference is a *mismatch* carrying
        both sizes as its reason, not an error (FR-3.4-S2, requirements D3), and
        no pixels are compared in that case.
      - All three thresholds are **strictly greater than** comparisons. This also
        keeps `float_cmp` quiet — nothing is compared with `==`.
      - `failing_mask` is `None` iff dimensions differ; T10 depends on that.
      - Order symmetry (FR-3.4-S1) is asserted over verdict, failing-pixel count
        and maximum distance, per Assumption 6.
      - Test images: the 320×180 pair (57 600 px) for FR-3.2 and FR-3.3-S2, where
        5 px = 0.0087% passes and 6 px = 0.0104% fails; 64×64 elsewhere.

- [x] **T08** Comparison in a process that holds no adapter — `tests/`
      Scenarios: FR-3.4-S3
      Depends on: T07
      - One behavioural test over two caller-supplied 64×64 images, in a test file
        that references nothing feature-gated, so it compiles and runs under
        `--no-default-features`.
      - Its worth is entirely in the configuration it runs in. Keep it separate
        from T07's file so that dependency is visible rather than incidental.

- [x] **T09** `[P]` PNG encode, write and read — `frame/png.rs`, `tests/`
      Scenarios: FR-2.4-S1, FR-2.4-S2, FR-2.4-S3
      Depends on: T03
      - `encode_png`, `write_png` (creates the parent directory; failure to create
        it is `ImageIoError::Directory` — FR-2.4-S3), `read_png`
        (`ImageIoError::Decode`, which FR-4.4-S5 later leans on).
      - Pin `CompressionType` and `FilterType` explicitly rather than relying on
        `image`'s defaults, so byte-identity does not depend on an upstream
        default.
      - **Risk check, first thing in this task:** encode the same `Rgba8Image`
        twice and compare the bytes. If `image`'s PNG encoder is not
        byte-deterministic across identical inputs, FR-4.3-S2 is unsatisfiable as
        written on encoded bytes.
      - **Pre-authorised fallback** (lead ruling, 2026-08-11) so implementation
        does not stall: assert byte-identity on the **diff image buffer before
        encoding**, *plus* decoded-pixel identity after the PNG round trip.
        Do **not** fall back to a decoded-pixel assertion alone — that discards
        the determinism the scenario exists to prove. If this branch is taken,
        record it as a Clarification in `spec.md` and report it to the lead, who
        wants to know which branch we are on but does not want the work blocked.
      - FR-2.4-S1/S2 say "a captured 64×64 image". No capture is involved: the
        test constructs the image by hand. That is the data seam (D1) working.
      - FR-2.4-S2 is the asymmetric on-disk assertion — a row flip on write plus a
        row flip on read passes FR-2.4-S1 while the file is upside-down for the
        agent this harness exists for. Assert named coordinates on the decoded
        file, and do not "simplify" it into a round trip.
      - `tempfile::TempDir` for the filesystem cases; FR-2.4-S3 is provoked
        portably by a path whose parent is an existing *file*.

- [x] **T10** Diff image rendering — `frame/diff.rs`, `tests/`
      Scenarios: FR-4.3-S1, FR-4.3-S2, FR-4.3-S3
      Depends on: T07, T09
      - `render_diff(expected, comparison) -> Option<Rgba8Image>`: failing
        positions become opaque magenta (255, 0, 255, 255); every other position
        carries the **expected** image's pixel; `None` when dimensions differ
        (FR-4.3-S3, whose "record that no diff image could be produced" half lands
        in T18's report).
      - FR-4.3-S2's byte-identity is asserted on `encode_png` output, not on the
        in-memory buffer.

- [x] **T11** Coverage-denominator measurement — no production change
      Scenarios: none (approved ruling 4, measurement only)
      Depends on: T03–T10 (needs real `src/` and `tests/` to measure)
      - Run the gate's coverage command and inspect the JSON summary's per-file
        list for `crates/*/tests/` entries. Record: total tracked lines, the line
        percentage, and the same two numbers with `tests/**` excluded.
      - **Do not extend `$CoverageExclude`.** Write the numbers into
        [Notes](#notes) and report them to the lead, who decides. Measure first,
        change second.

---

## Phase 2 — GPU-free policy, lifecycle and reporting

20 scenarios. Still nothing links `wgpu`; the phase-1 DoD applies unchanged.

- [ ] **T12** Capture identity and on-disk layout — `frame/layout.rs`,
      `frame/layout_test.rs`
      Scenarios: none (approved ruling 3)
      - `CaptureId` validated newtype: one path segment, non-empty, `[a-z0-9_-]`
        only, never `.` or `..`. Kept on `code-quality.md` §7 grounds — it is
        input validation on a path-forming public input, and it names the capture
        in three roles (timeout error, golden directory, artifact directory).
      - `GoldenPaths` → `<golden-root>/<capture-id>/default.png` and
        `default.provenance.json`; `ArtifactPaths` → `<artifact-root>/<capture-id>/`
        with `expected.png`, `actual.png`, `diff.png`, `report.json` (D8).
      - `default` is a written constant. **No variant selection logic** — the
        headroom for an adapter discriminator is in the path shape only, and
        variants are Out of Scope.
      - Paths are deliberately **stable across runs** — no PID, no timestamp —
        because FR-4.1-S2 needs today's pass to find yesterday's stale files.
      - `GoldenSettings { golden_root, artifact_root, capture, thresholds, opt_ins }`.
        Roots are always caller-supplied, never guessed.

- [ ] **T13** `[P]` Adapter description DTOs and preference ranking —
      `frame/selection.rs`, `frame/selection_test.rs`
      Scenarios: FR-1.1-S5
      - `Backend`, `AdapterKind`, `AdapterDescription`, `AdapterLimits`,
        `UnsatisfiedLimit`, `AcquireError`. **`AcquireError` lives in the core**,
        not in `gpu/` — it carries only `Backend` and `String`, and T14's
        `classify_acquisition` takes it as input, so a feature-gated definition
        would make FR-1.2-S1..S3 untestable without a GPU.
      - `select_preferred(&[AdapterDescription]) -> Option<usize>`, ranking
        `Discrete > Integrated > Virtual > Other > Cpu`, ties broken by
        enumeration order. **`Cpu` ranks last, below `Other`** — `Cpu` is the only
        variant that definitively means a software rasteriser, while `Other` is
        reported by real hardware on GL/ANGLE and some Vulkan drivers. Ranking
        `Other` below `Cpu` would silently mint goldens from a software adapter.
        This ordering is the tested contract (D2).
      - `unsatisfied_limit(required, available) -> Option<UnsatisfiedLimit>`
        (private, unit-tested here). Its scenario, FR-1.1-S4, is a real wgpu
        rejection in phase 3.

- [ ] **T14** Environment opt-ins and acquisition classification —
      `frame/optins.rs`, `frame/selection.rs`, `tests/`
      Scenarios: FR-1.2-S1, FR-1.2-S2, FR-1.2-S3
      Depends on: T13
      - `OptIns { allow_no_gpu, update_goldens }` with
        `from_lookup(impl Fn(&str) -> Option<OsString>)` and
        `from_environment()`. **`from_environment` is the only function in the
        crate that names `std::env`**, and the two variable names exist in exactly
        one place.
      - **Tests must not set environment variables.** `std::env::set_var` is
        `unsafe` in edition 2024 and `unsafe_code` is warn + `-D warnings`; the
        opt-ins are injected as values through `from_lookup` instead (D3). An
        `#[allow(unsafe_code)]` in a test is exactly the escape hatch the gate
        exists to make visible.
      - Presence, not value, enables an opt-in: `MYCRAFT_ALLOW_NO_GPU=0` still
        enables the skip (Assumption 5). Assert it through a fake lookup.
      - `classify_acquisition(Result<AdapterDescription, AcquireError>, &OptIns)
        -> AcquisitionVerdict`. FR-1.2-S2 requires the **literal string**
        `MYCRAFT_ALLOW_NO_GPU` in the skip notice — assert the literal, not a
        paraphrase.

- [ ] **T15** Deadline-bounded readback wait — `frame/clock.rs`, `tests/`
      Scenarios: FR-2.3-S2
      Depends on: T12
      - `trait Clock`, `SystemClock::started_now()`, `Progress<T>`, `Elapsed<T>`,
        `poll_until_deadline(clock, deadline, step) -> Result<Elapsed<T>, DeadlineExpired>`.
      - The deadline must elapse **during** the wait, not before it (audit fix
        #15). A fake clock advancing per call produces a 30-second scenario in
        microseconds — that is the whole reason this is testable.
      - **`poll_until_deadline` must not sleep.** Sleeping belongs in the caller's
        `step` closure, so the fake-clock test does not sleep either.
      - `CaptureError` is defined **in the core**, not under `gpu/`: its variants
        are `Size(FrameSizeError)`, `DrawWork(Box<dyn Error + Send + Sync>)`,
        `ReadbackTimeout { capture: CaptureId, deadline: Duration }` and
        `Readback(ReadbackError)` — no wgpu type among them. FR-2.3-S2 asserts
        that the timeout names both the capture and the deadline, and it must do
        so under `--no-default-features`.

- [ ] **T16** Report, provenance and JSON shape — `frame/report.rs`, `tests/`,
      `crates/mc-testkit/Cargo.toml`
      Scenarios: FR-5.1-S1, FR-5.1-S2, FR-5.1-S3
      Depends on: T12, T13
      - Add `serde = { workspace = true }` to the manifest here — this is its
        first use, and `cargo machete` would have failed it in T01.
      - `AdapterProvenance { name, backend, driver_description }` and
        `FrameReport`, serialised with `serde_json` to the two shapes fixed in
        architecture.md § Data. `failing_pixels` is always a JSON **number**,
        never null. On a dimension mismatch the three pixel statistics are `0`
        (Assumption 9) and `diff_omitted_reason` is populated.
      - The provenance block is six fields — adapter name, backend, driver
        description, the three thresholds, the failing-pixel count, the maximum
        per-pixel distance — all present in every report.
      - FR-5.1-S3's `"unknown"` normalisation must be a **pure core function**
        (e.g. a constructor taking `Option<&str>`), called by the phase-3 adapter.
        Inlining it in `gpu/acquire.rs` would make FR-5.1-S3 need hardware, and it
        is one of the 41 that must not.

- [ ] **T17** Golden lifecycle: the pass path and stale-artifact clearing —
      `frame/golden.rs`, `tests/`
      Scenarios: FR-4.1-S1, FR-4.1-S2, FR-4.1-S3
      Depends on: T07, T09, T12
      - `verify_against_golden(captured, provenance, settings) -> GoldenOutcome`
        with `GoldenOutcome`/`GoldenFailure`/`GoldenFailureReason` as specified.
        Fully GPU-free: it takes an image and provenance **as values**.
      - **Deletion is by explicit filename allowlist, never recursive.** Remove
        `expected.png`, `actual.png`, `diff.png`, `report.json` if present, and
        nothing else. The artifact root is caller-supplied, so `remove_dir_all` on
        it is a foot-gun aimed at whatever the caller passed (§7).
      - The clear runs on **every non-mismatch path**, not only on a pass (D7).
      - FR-4.1-S3: clearing one capture's directory leaves another capture's
        files untouched. Real temp directories (`tempfile::TempDir`), per
        testing.md §4 — no filesystem mock.
      - The lifecycle is the branchiest code in the spec. The state table in
        architecture.md § "Golden lifecycle behaviour" **is** the decomposition:
        one small named function per row, dispatched by a match, to stay inside
        `too_many_lines = 30` and `cognitive_complexity = 15`.

- [ ] **T18** Golden lifecycle: the mismatch artifact set —
      `frame/golden.rs`, `tests/`
      Scenarios: FR-4.2-S1, FR-4.2-S2, FR-4.2-S3
      Depends on: T10, T16, T17
      - On a mismatch with `MYCRAFT_UPDATE_GOLDENS` unset: write `expected.png`,
        `actual.png`, `diff.png` (omitted on a dimension mismatch, with the reason
        recorded in the report — FR-4.3-S3's other half) and `report.json`.
      - The artifact directory is named in the reported failure's `Display`
        (FR-4.2-S2).
      - FR-4.2-S3: an artifact-write failure does **not** swallow the mismatch —
        `GoldenFailure.artifacts` is a `Result`, and an `Err` still fails with the
        artifact failure and its underlying cause named alongside. Provoke it
        portably by pointing the artifact root at a path whose parent is an
        existing file.
      - `let_underscore_must_use` and `map_err_ignore` are denied: best-effort
        deletion cannot be written `let _ = fs::remove_file(p);` or
        `.map_err(|_| ..)`. Sanctioned shape:
        `if fs::remove_file(&path).is_err() { /* audit #22: recovery is Out of Scope */ }`.

- [ ] **T19** Golden lifecycle: missing and undecodable goldens —
      `frame/golden.rs`, `tests/`
      Scenarios: FR-4.4-S1, FR-4.4-S2, FR-4.4-S5
      Depends on: T17
      - Missing golden with the opt-in unset: fail naming the missing path, and
        write `actual.png` **only** into the artifact directory after clearing it
        — no report, no diff (Assumption 8). The file is never created.
      - Undecodable golden: fail naming the path and the decode error, and **do
        not overwrite the file** — a corrupt golden is never silently replaced.
      - This is D4 made structural: a broken renderer must never mint its own
        ground truth.

- [ ] **T20** Golden lifecycle: the update path — `frame/golden.rs`, `tests/`
      Scenarios: FR-4.4-S3, FR-4.4-S4, FR-5.1-S4
      Depends on: T14, T16, T17
      - With `MYCRAFT_UPDATE_GOLDENS` set (injected as an `OptIns` value, never as
        a real environment variable) and a mismatch: overwrite the golden and its
        provenance sidecar, report **every** golden path written, and do **not**
        write the mismatch artifact set (audit contradiction 1 — recording a
        failure against ground truth that has just been replaced).
      - With the opt-in set and a match: golden bytes **and** sidecar unchanged,
        no paths reported.
      - FR-5.1-S4: the sidecar records the adapter name and backend alongside each
        golden written. Its shape comes from T16's `AdapterProvenance`. This is
        what makes the per-adapter-golden deferral end by adding files rather than
        migrating them.

- [ ] **T21** `[P]` `.gitattributes` for byte-sensitive files — `.gitattributes`
      (repo root)
      Scenarios: none (lead ruling, 2026-08-11)
      - The repo has **no `.gitattributes` at all** today, so a committed PNG
        relies entirely on git's content auto-detection — on Windows that is the
        one thing standing between a golden and a line-ending rewrite.
      - `*.png binary`, plus `*.provenance.json text eol=lf` so the committed
        sidecar is stable across platforms. Own commit, small, before T22 commits
        the first PNG.
      - A golden is byte-sensitive by definition. Discovering this against a real
        frame in PRO-852 is the failure mode being pre-empted.

- [ ] **T22** One committed golden, CPU-generated, read from its real repo path —
      `crates/mc-testkit/goldens/<capture-id>/`, `tests/`
      Scenarios: none (lead ruling, 2026-08-11 — exercises the committed-golden
      workflow, which no scenario covers)
      Depends on: T09, T16, T17, T20, T21
      - **The fixture image is produced on the CPU**, by a deterministic function
        in the test code — never by a capture. A GPU-generated golden would couple
        this check to the development machine's adapter and pre-empt the
        per-adapter-golden deferral, which is precisely what FR-5.1-S4 exists to
        keep clean.
      - Generate it **through the harness's own update path** —
        `verify_against_golden` with `OptIns { update_goldens: true }` passed as a
        value — so the committed PNG and its sidecar are written by the real code,
        not by hand. The sidecar must record the golden as synthetic (a name that
        cannot be mistaken for a real adapter), never a fabricated adapter string.
      - One test then loads that golden **from its real repo path**
        (`env!("CARGO_MANIFEST_DIR")/goldens`, per D8) with the artifact root
        pointed at a `TempDir`, and asserts a match. The temp artifact root is
        what keeps a passing run from writing anything into the working tree.
      - This is the git round trip — committed bytes, read back, compared, judged
        — that no temp-root test can exercise. Running it for the first time in
        PRO-852 would make a failure ambiguous between a wrong renderer and a
        wrong golden workflow, which is the exact ambiguity this harness exists to
        remove (invariant 5: the verifier is proven before the thing it verifies).
      - Regenerating the fixture must be byte-reproducible from the committed
        generator function, so a future contributor can rebuild it rather than
        trusting it.

---

## Phase 3 — the wgpu adapter and hardware observations

12 scenarios, every one an observation of real hardware. This is the first phase
in which `wgpu` is in the dependency graph, and therefore the first in which the
`gpu` feature and the T02 gate stage carry weight.

- [ ] **T23** Feature gate and GPU module wiring —
      `crates/mc-testkit/Cargo.toml`, `frame/mod.rs`, `frame/gpu/mod.rs`
      Scenarios: none (enabling)
      - `[features] default = ["gpu"]`, `gpu = ["dep:wgpu", "dep:pollster"]`;
        `wgpu` and `pollster` as `{ workspace = true, optional = true }`. Still no
        version literals.
      - `#[cfg(feature = "gpu")] pub mod gpu;` and `#[cfg(feature = "gpu")] pub use wgpu;`
        — consumers link the same wgpu, and the coupling is explicit rather than
        claimed away.
      - `frame/gpu/**` is the **only** place `wgpu::` may appear in this crate.
        DoD item 4 becomes a real check from here on.
      - Feature unification note: once any workspace member enables `gpu`, a
        consumer's `default-features = false` is unified back on. The standalone
        `-p mc-testkit --no-default-features` invocation is the only place the
        seam is real, which is why T02 exists.

- [ ] **T24** Device acquisition, adapter selection wiring and provenance —
      `frame/gpu/acquire.rs`, `tests/`
      Scenarios: FR-1.1-S2, FR-1.1-S3, FR-1.1-S4
      Depends on: T13, T14, T16, T23
      - `AcquireOptions { backends, required_limits }`, defaulting to
        `Backends::PRIMARY` and `Limits::downlevel_defaults()`;
        `CaptureContext::acquire(&OptIns, &AcquireOptions) -> Result<Acquisition, AcquireError>`;
        `Acquisition::{Ready(Box<CaptureContext>), Skipped(SkipNotice)}`.
      - Map `wgpu::AdapterInfo`/`wgpu::Limits` into T13's DTOs, call
        `select_preferred`, and normalise the driver description through T16's
        pure function. No window, no surface, no display-server connection at any
        point.
      - **Both failure scenarios are provoked for real**, not with hand-built
        errors (D2): FR-1.1-S3 with `backends: Backends::BROWSER_WEBGPU`, which
        enumerates zero adapters on native and yields a non-empty `tried` list
        (`Backends::empty()` would make the assertion degenerate); FR-1.1-S4 with
        `required_limits.max_texture_dimension_2d = u32::MAX`, which wgpu's own
        limit validation rejects. Both must return an error **without panicking
        and without creating a window**.
      - These two tests are also the cheapest available cover for
        `gpu/acquire.rs`'s error branches, which matters because `mc-testkit` is
        inside the coverage denominator.
      - Known and accepted gap: FR-1.1-S5's *wiring* is observed only indirectly,
        through FR-1.1-S2's reported name and backend. A two-adapter machine is
        not available.

- [ ] **T25** Offscreen target, readback and frame geometry —
      `frame/gpu/target.rs`, `frame/gpu/mod.rs`, `tests/`
      Scenarios: FR-1.1-S1, FR-2.2-S1, FR-2.3-S1
      Depends on: T03, T05, T15, T24
      - `CaptureRequest { capture, size, deadline }` (default 30 s), `Capture
        { image, readback }`, `CaptureContext::capture(...)`.
      - Sequence, in this order, so FR-2.2-S2/S3's "SHALL NOT submit any GPU work"
        is structural: `validate_frame_size` (pure) → create the
        `Rgba8UnormSrgb`, `RENDER_ATTACHMENT | COPY_SRC` texture and a
        `COPY_DST | MAP_READ` buffer of `padded_row_bytes(w) * h` → record the
        caller's draw work → `copy_texture_to_buffer` with
        `bytes_per_row = padded_row_bytes(w)` and submit → `map_async` +
        `poll_until_deadline` with a non-blocking polling `step` → `unpad_rows` →
        unmap.
      - Per-capture texture and buffer drop at the end; the context keeps only
        instance/adapter/device/queue. That is what makes FR-2.1-S6 true.
      - FR-1.1-S1 is the 1×1 smoke capture in a process with no window and no
        display server: exactly 1 pixel back. FR-2.2-S1 is 257×129 — both
        dimensions defeat the 256-byte alignment — yielding exactly 33 153 pixels
        with 257 per row. FR-2.3-S1 asserts an elapsed readback time is reported
        at all; it is **not** a timing assertion and must not become one
        (performance is Out of Scope).
      - wgpu 30's exact poll/map call names are the implementer's to confirm
        against the docs; the sequence above is behavioural.
        `COPY_BYTES_PER_ROW_ALIGNMENT = 256` is confirmed.

- [ ] **T26** Caller draw work: colour, alpha and the sRGB contract — `tests/`
      (self-verification scene), `frame/gpu/mod.rs`
      Scenarios: FR-2.1-S1, FR-2.1-S2, FR-2.1-S3, FR-2.1-S4
      Depends on: T25
      - `trait DrawWork` over `(&mut wgpu::CommandEncoder, &wgpu::TextureView)`
        plus the `draw_fn` coercion helper — passing a bare closure makes rustc
        infer a higher-ranked bound over two independent lifetimes and fail with
        an opaque error, so the helper fixes the shape at the call site (D6).
      - **The caller owns the render pass, including its load op.** The harness
        supplies a canvas, never a scene — that is FR-6.1 and invariant 5.
      - The solid-colour pipeline and its WGSL live in `crates/mc-testkit/tests/`.
        **The library ships no shaders.** FR-2.1-S2's left-half fill needs a real
        pipeline and draw; the other three are caller-chosen clears.
      - These four are D5's computed ground truth: values derived from first
        principles, never compared against a committed image. A systematically
        wrong capture path — swapped channels, flipped rows, premultiplied alpha —
        must fail here.
      - No alpha arithmetic anywhere: a clear of `(1,1,1,0.25)` stores
        `(255,255,255,64)`, nothing premultiplies, readback never divides.
        FR-2.1-S4's mid-tone is a **linear** clear of ≈0.2159 landing on 128±1;
        the ±1 absorbs backend rounding. If it lands outside ±1 on this adapter,
        that is information about cross-adapter drift feeding the
        per-adapter-golden trigger — **not** grounds for widening a tolerance.
      - **AMENDED 2026-08-11 (lead ruling): FR-2.1-S2 is now a _top_-half fill,
        not a left-half fill**, asserting (32, 0) and (32, 63). The task text
        above predates the change and is kept intact per this file's own rule;
        `spec.md` and `architecture.md` D6 are authoritative. The half-fill still
        needs a real pipeline and draw, so nothing about this task's shape
        changes — but the fixture is now the only thing in phase 3 that can
        witness a row inversion in the readback chain, and the WGSL must get
        clip-space y right (y is up; the top half is y > 0). Do not "simplify" it
        back to a column split.

- [ ] **T27** Draw-work failure and context reuse — `tests/`
      Scenarios: FR-2.1-S5, FR-2.1-S6
      Depends on: T25
      - FR-2.1-S5: the caller's error is returned unchanged and no image is
        produced. `CaptureError::DrawWork(#[source] Box<dyn Error + Send + Sync>)`
        preserves it in the `source()` chain, downcastable. No submission has
        happened at that point.
      - FR-2.1-S6: the same context produces a valid image on the next capture
        after a failed one. A test binary captures many frames from one context,
        so a poisoned context would be a silent cliff.

- [ ] **T28** Composition root: capture then verify, end to end — `frame/gpu/mod.rs`,
      `tests/`
      Scenarios: none (no scenario exercises the composition root end to end)
      Depends on: T17–T20, T25
      - `capture_and_verify(context, request, draw, settings) -> Result<GoldenOutcome, CaptureError>`.
        It lives in the GPU layer, not the core, so the core never needs to invoke
        a device.
      - One round-trip test against a **temporary** golden root: capture a known
        clear, write the golden with `OptIns { update_goldens: true }` passed as a
        value, capture again, assert a pass. No environment variable is set, and
        nothing GPU-produced is ever committed — the harness proves its capture
        path against computed ground truth (D5). The one committed golden is
        T22's CPU-generated fixture and stays out of this test.
      - Without this test `capture_and_verify` is uncovered code inside the
        coverage denominator.

---

## Scenario assignment

All 53 scenarios, each in exactly one task.

| Scenario | Task | Phase | | Scenario | Task | Phase |
|---|---|---|---|---|---|---|
| FR-1.1-S1 | T25 | 3 | | FR-3.5-S1 | T07 | 1 |
| FR-1.1-S2 | T24 | 3 | | FR-3.5-S2 | T07 | 1 |
| FR-1.1-S3 | T24 | 3 | | FR-3.5-S3 | T07 | 1 |
| FR-1.1-S4 | T24 | 3 | | FR-4.1-S1 | T17 | 2 |
| FR-1.1-S5 | T13 | 2 | | FR-4.1-S2 | T17 | 2 |
| FR-1.2-S1 | T14 | 2 | | FR-4.1-S3 | T17 | 2 |
| FR-1.2-S2 | T14 | 2 | | FR-4.2-S1 | T18 | 2 |
| FR-1.2-S3 | T14 | 2 | | FR-4.2-S2 | T18 | 2 |
| FR-2.1-S1 | T26 | 3 | | FR-4.2-S3 | T18 | 2 |
| FR-2.1-S2 | T26 | 3 | | FR-4.3-S1 | T10 | 1 |
| FR-2.1-S3 | T26 | 3 | | FR-4.3-S2 | T10 | 1 |
| FR-2.1-S4 | T26 | 3 | | FR-4.3-S3 | T10 | 1 |
| FR-2.1-S5 | T27 | 3 | | FR-4.4-S1 | T19 | 2 |
| FR-2.1-S6 | T27 | 3 | | FR-4.4-S2 | T19 | 2 |
| FR-2.2-S1 | T25 | 3 | | FR-4.4-S3 | T20 | 2 |
| FR-2.2-S2 | T03 | 1 | | FR-4.4-S4 | T20 | 2 |
| FR-2.2-S3 | T03 | 1 | | FR-4.4-S5 | T19 | 2 |
| FR-2.3-S1 | T25 | 3 | | FR-5.1-S1 | T16 | 2 |
| FR-2.3-S2 | T15 | 2 | | FR-5.1-S2 | T16 | 2 |
| FR-2.4-S1 | T09 | 1 | | FR-5.1-S3 | T16 | 2 |
| FR-2.4-S2 | T09 | 1 | | FR-5.1-S4 | T20 | 2 |
| FR-2.4-S3 | T09 | 1 | | FR-6.1-S1 | T06 | 1 |
| FR-3.1-S1 | T07 | 1 | | | | |
| FR-3.1-S2 | T07 | 1 | | | | |
| FR-3.2-S1 | T07 | 1 | | | | |
| FR-3.2-S2 | T07 | 1 | | | | |
| FR-3.3-S1 | T07 | 1 | | | | |
| FR-3.3-S2 | T07 | 1 | | | | |
| FR-3.4-S1 | T07 | 1 | | | | |
| FR-3.4-S2 | T07 | 1 | | | | |
| FR-3.4-S3 | T08 | 1 | | | | |

**Counts.** Phase 1: 21 (T03 ×2, T06 ×1, T07 ×11, T08 ×1, T09 ×3, T10 ×3).
Phase 2: 20 (T13 ×1, T14 ×3, T15 ×1, T16 ×3, T17 ×3, T18 ×3, T19 ×3, T20 ×3).
Phase 3: 12 (T24 ×3, T25 ×3, T26 ×4, T27 ×2). Total 53.

GPU-free: 41 (phases 1–2). GPU-requiring: 12 (phase 3) — FR-1.1-S1/S2/S3/S4, all
six of FR-2.1, FR-2.2-S1, FR-2.3-S1, exactly as the architecture identifies.

## Notes

Deferred observations and follow-ups. Never delete task text; append status
markers only.

- **T11 measurement result** — measured 2026-08-11 at the phase-1 boundary,
  against the gate's own coverage command. `$CoverageExclude` was **not**
  changed; the lead decides.

  | Denominator | Tracked lines | Line % |
  |---|---|---|
  | A. As the gate measures today | 440 | **87.95%** |
  | B. Excluding `crates/*/tests/` | 440 | **87.95%** |
  | C. Also excluding `*_test.rs` siblings | 378 | **85.98%** |

  **The risk the architecture predicted does not exist.** A and B are identical
  because `cargo llvm-cov` never counted `crates/*/tests/` in the first place —
  integration tests are separate crates and are excluded by default. Zero files
  under `crates/*/tests/` appear in the report. Extending `$CoverageExclude`
  with `\|crates[/\\][^/\\]+[/\\]tests[/\\]` would be a no-op.

  **A smaller effect of the opposite sign does exist**, from a source the risk
  register did not name: the `*_test.rs` siblings of D10 are `#[path]`-included
  into the **lib** target, so llvm-cov cannot tell them from library code and
  counts them. They are 100%-covered by construction, so they *inflate* the
  figure rather than diluting it — 62 of 440 tracked lines (14%) are test code,
  worth about 2 percentage points today. Both numbers clear the 80% bar, so
  nothing is forced now, but the inflation grows with every sibling phase 2 adds
  (`layout_test.rs`, `selection_test.rs`), softening the bar for exactly the
  crate ADR-008 leans on. If the lead wants the bar honest, the change is
  `$CoverageExclude` gaining `\|_test\.rs$` — a gate change, deliberately not
  made here.

- **Per-file coverage at the phase-1 boundary**, for context on the number
  above: `color.rs` 100%, `compare.rs` 97.79%, `readback.rs` 90.70%,
  `diff.rs` 90.00%, `image.rs` 76.92%, `png.rs` 53.57%. The two low files are
  error paths with no phase-1 caller — `read_png`'s decode and shape failures
  are first exercised by FR-4.4-S5 in phase 2 (T19), and `Rgba8Image::pixel`'s
  bounds arithmetic by the golden lifecycle. Deferred observation, not a defect:
  the aggregate is above threshold and the gaps close in phase 2 by design.

- **T09 PNG determinism: the primary branch held.** The pre-authorised fallback
  was **not** taken. With `CompressionType::Best` and `FilterType::Adaptive`
  pinned explicitly, `encode_png` is byte-identical for identical input both
  within one process and **across separate processes** — the stronger property,
  and the one a committed golden actually depends on. FR-4.3-S2 is asserted on
  encoded PNG bytes exactly as written, so no Clarification was added to
  `spec.md`.

- **The spec's ΔE ≈ 0.40 is arithmetically 0.3917.** Computed against CIE76 /
  D65 / the sRGB transfer function, the three reference distances are 0.39168,
  4.66505 and 19.72703. The 4.67 and 19.72 figures in `spec.md` are accurate;
  0.40 rounds high, and 0.39 is the correct two-decimal value. Nothing is
  blocked — the figure is quoted with `≈` and the tests assert a ±0.02 window —
  but a correct implementation lands at 0.392, and a reviewer should not read
  that as a defect. Raised to the lead; `spec.md` deliberately left unedited
  pending their call.

- **FR-2.4-S2's fixture cannot catch the failure mode it was written for.**
  Raised to the lead at the phase-1 boundary, unresolved at time of writing.
  The scenario's left-half-white / right-half-black frame is *invariant under a
  vertical row flip*, so asserting (0,32) and (63,32) catches a horizontal
  mirror rather than the compensating write-flip/read-flip pair that `spec.md`
  § Structural constraints and architecture D5 both cite as its reason to exist.
  The test is written exactly as specified; correcting it means changing the
  fixture to vertical asymmetry, which is a spec change and not the test
  author's or the implementation's to make. Partial mitigation already in place:
  FR-2.4-S1's fixture is asymmetric in both axes, so a single flip on write *or*
  read is caught there — only a symmetric pair survives.
- **Exactly one golden is committed** (T22), CPU-generated and marked synthetic
  in its sidecar. Every FR-4.x *scenario* still runs against a temporary golden
  root; the committed one exists solely to exercise the git round trip — read a
  golden from its real repo path, compare, judge — which no temp-root test
  reaches and which would otherwise run for the first time in PRO-852, where a
  failure would be ambiguous between a wrong renderer and a wrong workflow. It is
  deliberately **not** GPU-produced: an adapter-specific committed golden would
  pre-empt the per-adapter deferral this spec keeps open.
- **Out of Scope is binding.** No `mc-render` pipeline work, no chunk/mesh/block
  types, no `winit` or surface, no benchmarks, no per-adapter golden variants, no
  CI integration, no multi-frame capture, no depth/stencil/MRT. Anything
  discovered along the way is recorded here, not built.
