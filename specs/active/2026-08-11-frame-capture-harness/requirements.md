# Requirements Gathering — Headless Frame-Capture Harness (PRO-849)

Date: 2026-08-11 · Rigor: `high` · Branch: `feature/PRO-849-frame-capture-harness`

## Rigor decision

`high`, the project default (`product/mission.md`, root `CLAUDE.md`). Not
downgraded. This spec is the load-bearing half of ADR-008: `mc-render`,
`mc-client` and `mc-server` are excluded from the coverage denominator on the
explicit bet that golden frames catch renderer regressions instead. A weak
harness turns that exclusion into a blindfold — the same failure shape
`docs/technical/testing.md` calls out for the gitleaks `.env` allowlist. That
is a `high`-tier risk, so the architecture stage is mandatory.

Escalation to `xhigh` was considered and rejected: the decisions here are
technical and uncontested, with a single stakeholder. Persona debate would add
ceremony, not information.

## Codebase discovery

No Explore subagent was spawned. The workspace is a bare skeleton — ten crates
under `crates/`, each holding an empty `src/lib.rs` and a dependency-free
`Cargo.toml`, plus `scripts/sdd-gate.ps1`. Everything relevant was read
directly:

| Source | What it fixes for this spec |
|--------|-----------------------------|
| `Cargo.toml` | `wgpu` 30, `image` 0.25.10, `bytemuck` 1.25 already pinned in `[workspace.dependencies]`; member crates must not version dependencies themselves |
| `crates/mc-testkit/` | Empty. Zero dependencies today. This is where the harness lands |
| `crates/mc-render/CLAUDE.md` | "Golden frames are compared perceptually, not byte-wise — GPU drivers differ." Keep logic out of the GPU-touching layer |
| `docs/technical/testing.md` | Golden-frame section: headless wgpu → offscreen texture → PNG, no window, no compositor; unexplained golden updates are a review stop |
| `docs/technical/decisions.md` ADR-008 | The coverage exclusion this harness exists to justify. `mc-testkit` is **not** excluded — it is inside the 80% denominator |
| `scripts/sdd-gate.ps1` | Coverage regex `crates[/\\](mc-render\|mc-client\|mc-server)[/\\]`; 500-line source / 600-line test size caps; `-D warnings` with `unwrap`/`expect`/`panic`/`indexing_slicing` denied |
| `PLAN.md` §"Real gaps in autonomy", M0 | The original rationale: the agent cannot see the screen, so it needs readable PNGs and perceptual-diff regression tests |
| `product/roadmap.md` MVP 1 | PRO-849 is first of eight; PRO-850..856 own everything this spec excludes |

Reuse candidates: none. This is the first functional code in the workspace.

## Decisions taken

These were made by the spec author rather than asked, because the spawning
context fixed the parameters (issue, branch, rigor, folder) and answering them
requires engine knowledge rather than product preference. Each is flagged in
the report for overrule.

### D1 — Tolerance model: CIELAB ΔE, two thresholds plus a hard ceiling

**Options considered.**

| Option | Verdict |
|--------|---------|
| Byte-exact comparison | Rejected. GPU rasterisation is not bit-identical across adapters or driver versions; `mc-render/CLAUDE.md` already forbids it |
| Per-channel RGB delta + pixel budget | Rejected. Equal RGB steps are not equally visible — a ±4 step in a dark blue is invisible, the same step in mid-grey is not. One RGB threshold must be set for the worst case, which then tolerates far too much everywhere else |
| Full perceptual metric (pdiff / butteraugli / SSIM) | Rejected for now. Contrast-sensitivity models carry tuning parameters, are hard to explain when they fire, and add a dependency for a precision this project cannot yet use |
| **CIELAB ΔE per pixel, with an area budget and a hard per-pixel ceiling** | **Chosen** |

**The model.** Both images are decoded to 8-bit RGBA, converted sRGB → linear →
XYZ → CIELAB, and compared per pixel by Euclidean distance in Lab (CIE76 ΔE).

| Threshold | Default | Why |
|-----------|---------|-----|
| `per_pixel_delta_e` | 2.0 | ΔE 1.0 is roughly a just-noticeable difference under ideal viewing. 2.0 leaves headroom for rounding and dithering differences between adapters while staying below anything a human would call a visual change |
| `max_failing_fraction` | 0.001 (0.1%) | At 1280×720 that is ~921 pixels — enough for anti-aliased edges to drift on a differently-rasterising adapter, nowhere near enough to hide a lighting, texture or geometry regression |
| `hard_ceiling_delta_e` | 10.0 | A single pixel this far off is a defect, not sampling noise. Without it, a small-area but catastrophic error (one wrong sprite, a black hole in a texture) hides inside the area budget |

The ceiling is the part that makes the model honest: an area budget alone is a
percentage, and percentages forgive exactly the small severe errors that matter.

**Why CIE76 rather than CIEDE2000.** CIE76 is about ten lines of arithmetic,
which fits inside the gate's 30-line function and cognitive-complexity-15 lint
budget without a helper cascade, and it is accurate enough at the
just-noticeable-difference scale we threshold at. CIEDE2000 can replace it later
behind the same three-threshold contract without changing any caller.

**Determinism.** The comparison is a pure function of the two decoded buffers.
Its two reductions — a count and a maximum — are order-independent, so the
verdict does not depend on iteration order or on whether the work is
parallelised. No GPU is involved in comparison, which also means the differ is
unit-testable on any machine.

### D2 — Absent GPU fails by default; skipping requires an explicit opt-in

If capture tests skipped silently on a machine with no adapter, the gate would
go green while proving nothing, and ADR-008's exclusion would be hiding real
defects. Default is a hard failure. An explicit environment opt-in downgrades it
to a skip, and the skip announces itself and names the variable, so a green run
that verified nothing is never silent.

### D3 — Dimension mismatch is a mismatch, not an error

A golden frame that changed size is a genuine regression, so it must fail the
comparison rather than error out ambiguously. It is reported with its own
reason so it is never confused with pixel drift, and the diff image is omitted
(none can be produced) with the report saying so.

### D4 — A missing golden never auto-creates itself

Silently minting a golden on first run lets a broken renderer define its own
ground truth. A missing golden fails, writes the captured image as an artifact,
and names the path. Writing a golden requires an explicit opt-in, and the run
reports every golden file it wrote so the commit can justify it — which is what
`docs/technical/testing.md` requires of a changed golden.

### D5 — The harness proves itself against computed ground truth, not a golden

At least one capture asserts pixel values derived from first principles (a
known clear colour; a known half-frame fill), not against a committed image.
Otherwise the harness is only self-consistent, and a systematically wrong
capture path — swapped channels, flipped rows, premultiplied alpha — would
still pass forever.

### D6 — The harness knows nothing about `mc-render`

Callers supply the draw work; the harness owns device acquisition, the offscreen
target, readback and comparison. `mc-testkit` must not depend on `mc-render`.
This is invariant 5 made structural: if the harness needed the terrain renderer,
the build order would be inverted and this spec would be wrong.

### D7 — Bounded readback

`map_async` plus a poll loop can wait forever on a lost device. Every capture
carries a readback deadline (default 30 s) and returns a timeout error naming
the capture rather than hanging a test run.

## Constraints inherited

- `mc-testkit` is **inside** the coverage denominator (ADR-008 excludes only
  `mc-render`, `mc-client`, `mc-server`). Harness code must be structured so
  the CPU half — comparison, diff rendering, report writing, path handling —
  is testable without a GPU.
- Dependencies are pinned in `[workspace.dependencies]` only. `wgpu`, `image`
  and `bytemuck` already are; anything else this needs must be added there,
  never versioned in `crates/mc-testkit/Cargo.toml`.
- `unwrap`, `expect`, `panic!` and raw indexing are lint-denied workspace-wide.
- Source files cap at 500 lines, test files at 600.
