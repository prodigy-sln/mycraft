# Testing & Verification

How MyCraft is checked, and — because much of this work is done by an agent that cannot look at a
screen — how results are verified without a human watching.

Governing standards: `standards/global/testing.md` (TDD, coverage, mocking) and
`standards/global/code-quality.md`. Coverage exclusions: ADR-008, as narrowed by
ADR-013, in `decisions.md`.

## The quality gate

`scripts/sdd-gate.ps1` must exit 0 at every phase end, before validation, and before completion.
PowerShell 7 is cross-platform, so this is the only gate script — there is no `.sh` twin to drift
out of sync.

Every stage runs even when an earlier one fails, so one invocation reports every problem.

| # | Stage | Tool | Fails on |
|---|-------|------|----------|
| 1 | format | `cargo fmt --check` | any deviation |
| 2 | lint + complexity | `cargo clippy -D warnings` | any lint, incl. complexity thresholds |
| 2b | gpu-free (`mc-testkit` + `mc-render`, no default features) | `cargo clippy` + `cargo nextest` with `--no-default-features` | any lint, or any test failure, in the configuration where `wgpu` is absent from the dependency graph |
| 3 | size | built-in | source > 500 lines, tests > 600 |
| 4 | deps | `cargo machete` | any unused dependency |
| 5 | sast | `cargo deny` | vulnerabilities, bad licenses, banned crates, untrusted sources |
| 6 | secrets | `gitleaks` | credentials in the working tree |
| 7 | tests + coverage | `cargo llvm-cov nextest` | test failure, or lines < 80% |

Flags: `-Quick` runs stages 1–3 only, for tight edit loops. `-SkipCoverage` runs tests without
instrumentation. Neither is valid at a phase boundary or in CI.

**No stage runs `rustdoc`, so a broken intra-doc link ships silently.** That is how a dangling
reference to a moved item survives a green gate. The tree is clean under
`RUSTDOCFLAGS="-D warnings -D rustdoc::broken_intra_doc_links" cargo doc --workspace --no-deps`
today, so the check is a standalone command rather than a stage, with no backlog behind it.

### The gate is not safe to run from two contexts at once

Two concurrent invocations contend over `target/llvm-cov-target` and fail in ways that look like
defects: a phantom `error spawning child process` in one run, a cancellation in another, while
`cargo nextest run --workspace` passed in between. **A red gate under concurrency is not evidence** —
and a false red invites "fixing" code that is correct, which is more expensive than the wait. Run it
on a quiet tree.

The GPU suites have a related sensitivity: at nextest's default parallelism, nine test processes each
holding its own device, buffer set and 256-layer array texture aborted a driver once
(`code 0xc0000005`). It has not recurred at `--test-threads 2`; a nextest test-group for the GPU
suites is the remedy if it does.

### Complexity thresholds

`clippy.toml` makes `code-quality.md` §2 machine-enforceable rather than reviewer-enforced:

| Limit | Value | Source |
|-------|-------|--------|
| Function length | 30 lines | code-quality.md §2 |
| Arguments | 4, **receiver included** | code-quality.md §2 |
| Nesting depth | 3 (= 2 nested blocks) | code-quality.md §2 |
| Cognitive complexity | 15 | firmer than clippy's default 25 |
| File length | 500 / 600 test | code-quality.md §2 |
| Banned names | `data`, `info`, `temp`, `result`, `item`, … | code-quality.md §3 |

Also lint-denied workspace-wide: `unwrap`, `expect`, `panic!`, `dbg!`, `todo!`, raw indexing,
`mem::forget`, float equality, swallowed errors. Raw indexing matters most in `mc-net`, where a
length prefix is attacker-controlled.

**`too_many_arguments` counts `self`, so a method gets three parameters and not four.** Clippy
measures the whole parameter list, receiver included. This is worth knowing before a signature is
designed rather than after: a four-parameter method reads as compliant, passes review, and fails the
gate. It cost PRO-852 one dispute and four restructurings.

Two things follow when a signature does hit it. **Prefer a named parameter group over splitting a
function that is doing one thing** — a group makes the values it carries part of the contract, where
a split invents a boundary the design did not have. And **`#[allow(clippy::too_many_arguments)]` is
not available to resolve it**: `code-quality.md` is constitutional, its violations need the user's
explicit approval, and neither an agent nor a reviewer can supply that on the user's behalf. If a
signature is fixed by an architecture document and still will not fit, the document is what changes.

### Supply chain

`deny.toml` governs stage 5. The license list is an **allowlist**, not a denylist, because the
client ships to players and a copyleft dependency reaching the binary is a distribution problem.
Advisory suppressions require a dated justification inline — never a silent skip.

**Removing a crate from the graph beats suppressing an advisory about it.** `winit`'s
`wayland-csd-adwaita` feature is off workspace-wide because it pulls
`sctk-adwaita → ab_glyph → owned_ttf_parser → ttf-parser`, which is unmaintained with no safe
upgrade and fails the advisories stage. The feature draws Wayland client-side decorations on a
platform this project does not build for, and `winit` falls back to its own — so the dependency was
dropped rather than ignored. The ignore list is for things that cannot be removed.

### Secrets — a coupling that must not be broken

Stage 6 scans the **working tree**, not git history, because a secret is cheapest to remove before
it is committed. That means the scanner also sees `.env`, which legitimately holds live API keys
for asset generation (ADR-009). `.gitleaks.toml` therefore allowlists `.env` and `.env.local`.

**That allowlist is only safe because the gate separately asserts those paths are still
git-ignored.** If someone removes the `.gitignore` entry, the allowlist would silently hide a
genuinely committed secret — the allowlist turns from a convenience into a blindfold. The gate
runs `git check-ignore` on each allowlisted path and fails if any is no longer ignored.

Do not remove one half without the other. The check is verified by a negative test: un-ignoring
`.env` must turn the gate red.

`.env.example` is deliberately **not** allowlisted. It is tracked, so it must never contain a real
value, and the scanner should catch it if it ever does.

## Test placement

**Unit tests live in a sibling `foo_test.rs`**, wired into the module it
tests with

```rust
#[cfg(test)]
#[path = "foo_test.rs"]
mod tests;
```

which compiles the sibling as an inner module and so gives it the same
private access an inline module would. Integration tests are separate
crates under `tests/`, reaching the public API and nothing else. Doc
tests are `///` examples on the public item.

**When each — the rule that was practised for two specs before anyone
wrote it down.** *Test through the public API in `tests/` by default;
write a private-access unit test only where the property genuinely has no
public surface.* Both halves have a precedent. PRO-850 phase 4 needed
one: `Palette`'s reference-count transitions are observable through no
public API, and only a test asserting them directly catches a broken
release path (see the mutation result below). PRO-850 phase 3
deliberately wrote none: everything it needed was reachable from
`tests/`, and **a unit test whose only justification is a private item
the public surface already exercises asserts the same fact twice.**

### Siblings: a considered departure from Rust's default

Rust's default is an inline `#[cfg(test)] mod tests { ... }` at the
bottom of the file under test — what the Rust Book recommends and what
rustc, tokio and serde do. **This project deliberately does not follow
it**, and the reason is that three separate tools here are file-granular
and none of them can reach inside a file:

- **Coverage.** `cargo llvm-cov` excludes files by
  `--ignore-filename-regex` and has no region-level opt-out on stable
  (`#[coverage(off)]` is unstable). A sibling is excluded by
  `_test\.rs$`; an inline module is not, and lands in the denominator.
  Measured: the inline layout moved the tracked total from 2262 to 2587
  lines and the headline from 92.48% to 92.89% — 325 lines of test code
  that is near-fully covered by construction, inflating the one number
  ADR-008 leans on.
- **Size limits.** The gate measures a `src/` file against 500 lines. A
  sibling spends that budget on production code alone, which is the code
  somebody has to read to understand the module.
- **The source-literal scan** — the check that no hardcoded
  `base:`-namespaced block name appears outside test code
  (`technical/architecture.md`). Against siblings it is a file-name
  filter. Against inline modules it has to find where each `#[cfg(test)]`
  item *ends*, which means counting braces while tracking strings and
  comments, because a `{` in either does not open a block: an eighty-line
  state machine that is not quite a Rust parser, with a failure mode
  (losing a closing brace, swallowing the rest of the file) that looks
  exactly like a clean repository.

**This is a trade, not a universal best practice.** What it costs is
idiom: a Rust developer arriving here expects the inline module and finds
a `#[path]` line instead, and every new module pays a three-line wiring
tax. What it buys is three measurements that mean what they say. The
project has been through the alternative — the inline layout was adopted
and reverted — and the deciding evidence was that the costs above are
measured while the cost of the departure is stylistic.

**What is *not* a reason for it:** the rustdoc ban. A project-wide ban on
`base:` in doc examples was once justified by this layout; it was
misattributed. The ban existed because the scan read *comments*, which is
independent of where tests live. The scan drops doc comments now — a doc
example is a doc test, so it is test code that genuinely does live in a
production file — and the ban is withdrawn for good:
`/// let name = BlockName::parse("base:stone")?;` is fine.

Two controls keep the scan honest, and they are separate test functions
for the reason given under structural-invariant tests below: a name in a
sibling `*_test.rs` is skipped while one in the module beside it is still
found, so a filter that drifted into skipping too much shows up; and a
name in a doc example is not reported, so the comment-skipping is asserted
rather than assumed. Both were confirmed to bite by hand mutation, each
applied and reverted separately — keeping doc-comment text turns the
doc-example control red and *only* it; dropping the `_test.rs` term from
the file filter turns the sibling control red and *only* it. That each
mutation hits exactly one control is the evidence that the two are
independent rather than one check written twice.

## Structural-invariant tests

**A structural-invariant test — one that walks a dependency graph or
scans source for an absence — needs a positive control, not just the
negative assertion.** A test that only asserts something is *not* present
(no `toml` in `mc-core`'s resolved graph; no `base:` literal in production
source) goes green forever the day the thing it was guarding against is
quietly removed — a deleted TOML loader replaced with hardcoded
definitions would leave "no `toml` dependency" trivially true. Both
structural-invariant tests in `mc-world` pair the negative assertion with
a positive control in a separate test function over the same walk: the
dependency-graph test additionally asserts that `mc-world`'s own resolved
graph *does* reach `toml`, and the source-scan test additionally asserts
that the same scanning function, pointed at a fixture directory
containing one of the guarded names, *does* report a hit. Splitting each
into two functions rather than one is deliberate — as a single test, "the
control fails while the real assertion still passes" is not something a
test run can show you happening; as two, it is.

**Two mutation-testing results are worth keeping as illustrations of what
"looks correct but isn't" can survive a full green suite:**

- Allocating a fixed 8192 bytes at *every* index-width tier (rather than
  the size each tier actually needs) fails five separate size assertions
  — but only as long as the reported storage size reads the backing
  buffer's real length. Recomputing that figure from the width instead of
  reading the buffer makes every one of those assertions pass again,
  against an allocation that is wrong at five of six tiers. The lesson
  generalizes: a "storage size" accessor that recomputes rather than
  reports is not actually testing the allocation.
- Making a palette's reference-count release a no-op fails a handful of
  direct tests — but that same broken write path, combined with a
  compaction step that recomputes surviving entries from the voxel array
  instead of trusting the maintained counts, passes the overwhelming
  majority of the suite. Refcounting is wholly broken in that scenario,
  and only a test that asserts refcount transitions directly — independent
  of anything compaction-visible — catches it. The general lesson:
  compaction (or any similar reconciliation step) must steer on the state
  the write path maintains, not recompute it from scratch, or a "safer"
  recomputation silently absorbs the exact defect the reconciliation step
  exists to expose.

A failing `proptest` run writes a `*.proptest-regressions` file recording
the failing seed. When the failure came from a deliberately mutated
implementation (mutation testing, not a real regression), delete that file
rather than committing it — a committed seed should mean "this once broke
the real implementation," not "this once broke a scratch mutant."

### The derived-oracle rule: what coverage does not vouch for

`benches/` code that a test reaches through `#[path]` is **outside the
coverage denominator** — confirmed three separate times, most recently by
coverage staying byte-identical at 92.48% over 2262 tracked lines across
roughly 330 newly added bench lines. The gate's coverage percentage
therefore vouches for the mesher's benchmark fixtures, its independent
oracle, and its budget verdict **not at all**.

Sibling `*_test.rs` files are outside it as well, by the `_test\.rs$`
term in the gate's exclusion regex — see the placement section above for
why that term is only reachable because unit tests keep their own file.
What remains in the denominator is library code, which is the property
that lets mc-testkit's number stand behind the exclusion of `mc-render`'s
GPU-resident subtree. When a specific crate's figure is load-bearing,
read the per-file table rather than the total anyway.

The answer is derivation, never a snapshot: **no expected quantity may be
copied from a run of the code under test**, because a count committed from
the first green run commits whatever that run happened to produce. An
emit-nothing mesher gets `0` committed as its expected quad count and passes
forever — this project caught exactly that, before any mesher code existed.
As built: `solid` = 6 quads, by inspection; `checkerboard` = 12 288 = 2048
solid voxels × 6 facings, derived by arithmetic; `terrain` is pinned by **no
committed number at all** — the summed area of the emitted quads must equal
what an independent per-voxel visible-face oracle counts for the same
fixture.

The oracle shares no code with the mesher: it goes through the public
per-voxel API and re-derives adjacency from its own six explicit signed
offsets, and it was written *before* the mesher existed, so there was
nothing to borrow from (invariant 5). An oracle that borrowed the mesher's
own adjacency table would agree with a sign inversion or a swapped neighbour
slot instead of catching it.

A fixture lesson worth keeping alongside this: a **spatially coherent**
`terrain` heightmap is a binding constraint no test can enforce. Per-column
white noise satisfies every assertion written against that fixture while
measuring the opposite workload from the one the budget exists to bound. It
is held in place by the construction in `fixtures.rs` and by a reviewer
reading the hash, and by nothing else.

The replay world the renderer captures answers that same problem by
**derivation instead of assertion**. Its heightmap is interpolated value
noise on a lattice period of 16 with classic smoothstep (`3t² − 2t³`,
maximum derivative 1.5) over the height range `32..=48`, which gives a
maximum field slope of `1.5 × 16 / 16 = 1.5` per block — so adjacent
integer heights differ by at most 2, and the step bound the scene's
assertions rely on is a consequence of the construction rather than a
measurement of it. **The amplitude may be lowered; the period may not**
without redoing that derivation.

### Assertions that are unfalsifiable alone and load-bearing together

`crates/mc-world/tests/mesh_properties.rs` asserts three properties over
generated sections: no emitted quad adjoins a solid voxel; every visible
(solid voxel, facing) pair is covered; no two quads cover the same pair. The
middle two are **satisfied by an over-covering mesher** and read as vacuous
in isolation — they are falsifiable only as a trio with the first. Recorded
here so a future reviewer meeting one of them alone does not delete it as
dead weight; the unit of judgement is the set, not the test function.

Those property tests run at ~32 `proptest` cases rather than the default
256. Each case meshes a section against up to six generated neighbours and
runs the oracle over all of them, under coverage instrumentation, at every
gate run, and the generated sections are built through `Section::import` in
one shot rather than through 4096 individual `set_block` calls.

### A thread spawned inside a rayon pool is not in that pool

**`std::thread::spawn` called from inside `ThreadPool::install` does not
inherit the pool.** Measured: code running directly inside `install` on a
one-worker pool sees one worker; a `std::thread` spawned from within that
same closure sees the machine's full count — 16. The spawned thread runs
against the global pool, and the pool the test carefully constructed is
simply not in the picture.

This silently vacates any test whose subject is *whether the worker count
decides the result* — determinism under parallelism, most obviously, where
the whole point is that a one-worker run and a sixteen-worker run produce
identical bytes. Such a test passes for the wrong reason: both halves ran
on the same global pool. Keep the work inside `install`, or hand the
`ThreadPool` to the thread and install on the other side.

## Verification without a human at the screen

Most of this project is built by an agent that cannot see the game window, cannot feel input lag,
and cannot invite 32 people to a server. Each of those gaps has a specific mechanism, and **the
mechanism is always built before the thing it verifies.**

### Rendering — golden frames

`mc-render`'s GPU-resident subtree is excluded from coverage (ADR-008 as narrowed by ADR-013), so
golden frames are the only automated check on the part of the renderer that touches a device — the
pure layer beside it is counted and unit-tested like any other library code. That makes the harness
load-bearing, and it exists ahead of the renderer it will verify: the
headless frame-capture harness lives in `crates/mc-testkit`, module `frame`, and lands before a
single line of `mc-render` is written (invariant 5). It renders caller-supplied draw work to an
offscreen colour target, reads the pixels back, and compares them against a committed golden under
an explicit tolerance — no window, no compositor, no display server.

The harness deliberately does not depend on the code it verifies. `mc-testkit` never names
`mc-render`, `mc-client` or `mc-server` in any dependency of any kind, and this is not a convention
someone has to remember: a test walks `cargo metadata`'s *resolved* dependency graph breadth-first
from the `mc-testkit` node and fails if any of the three ever appears in it. A direct-dependency
check would miss a renderer reached through an intermediary; the resolved graph cannot.

**The orientation and pixel-format contract** — row 0 is the top of the image at every stage, and
the capture format is 8-bit sRGB-encoded RGBA with straight alpha — is asserted by this harness but
belongs to the renderer as much as to the harness that checks it, so it is recorded once, in
`technical/rendering.md`, not repeated here.

#### Tolerance model

Two images never compare byte-for-byte, because GPU drivers legitimately differ in rounding and
dithering. Comparison is per-pixel perceptual distance — CIE76 ΔE in CIELAB, computed sRGB → linear
→ XYZ (D65) → Lab — judged against three thresholds, each a **strictly greater-than** comparison so
a value sitting exactly on a threshold passes:

| Threshold | Default | What it absorbs |
|-----------|---------|------------------|
| per-pixel tolerance | ΔE 2.0 | ΔE ≈ 1.0 is a just-noticeable difference; 2.0 absorbs rounding and dithering differences between adapters while staying invisible |
| area budget | 0.01% of pixels | Isolated rounding and dithering — nowhere near a block-sized artifact |
| hard ceiling | ΔE 10.0 | A single pixel this far off is a defect, not sampling noise. Without it, the area budget alone could forgive a small but severe error |

Every threshold is overridable per comparison, and loosening a default requires a recorded reason —
never the reverse. The area budget's default is the number worth explaining: a renderer with
anti-aliasing usually wants something closer to 0.1%, but at 0.1% the budget at 1280×720 is 921
pixels, comfortably more than one block face at mid distance. That budget could forgive an entire
wrong block face so long as every pixel on it stayed under the ΔE 10 ceiling — which is exactly the
shape of a same-family texture regression, the kind of bug this harness exists to catch rather than
wave through. At 0.01% it cannot. The project starts tight and loosens only on evidence, because
loosening first is how a golden suite quietly rots into a rubber stamp.

`delta_e` is the sole place a distance is computed, and the sole swap point if CIE76 is later
replaced with CIEDE2000: the three-threshold contract above stays the same regardless of which
metric produces the scalar.

#### The GPU-free seam

`mc-testkit`'s `gpu` Cargo feature is default-on and is the only place `wgpu::` may appear in the
crate. `mc-render` carries the same seam, with `src/gpu/` as its gated subtree. `cargo build
--no-default-features` removes wgpu from the dependency graph entirely — not merely leaves it
unused — so a stray `use wgpu::` outside the gated module is a build error rather than something a
reviewer has to catch. The quality gate runs both clippy and `nextest` in that configuration for
both crates (stage 2b above), which is what makes the seam a compile-time fact rather than a
convention that can quietly rot: it is the only process in which no GPU adapter can exist at all,
which is what makes assertions like "the process holds no adapter" mean what they say rather than
merely describing a test that declined to acquire one.

For `mc-render` the seam also decides the coverage denominator (ADR-013): everything outside
`src/gpu/` is counted, so the pure layer's tests are the figure the gate reads rather than a
promise the exclusion made on their behalf.

Every decision the harness makes — adapter preference, frame-size validation, row unpadding,
readback-deadline expiry, image comparison, diff rendering, the golden lifecycle, and golden/artifact
path construction — is a pure function over plain values that never sees a device. The wgpu-touching
module is left with only the mechanical part: create an instance, enumerate, request, allocate,
encode, submit, map, copy bytes out. Anything expressible as a pure function — meshing, vertex
packing, culling maths, atlas layout, light propagation — is unit-tested normally and gets no
exemption; only GPU-resident work does. Keeping logic out of the GPU-touching layer is therefore a
testability requirement here, and the same discipline is expected of `mc-render` once it exists.

#### Golden and artifact layout

```
crates/mc-testkit/goldens/<capture-id>/default.png                  # committed
crates/mc-testkit/goldens/<capture-id>/default.provenance.json      # committed

<artifact-root>/<capture-id>/expected.png     # transient, git-ignored under target/
<artifact-root>/<capture-id>/actual.png
<artifact-root>/<capture-id>/diff.png
<artifact-root>/<capture-id>/report.json
```

A directory per capture, with `default.<ext>` inside it, is deliberate headroom: a future
per-adapter golden variant is a **new file in an existing directory** (e.g.
`goldens/<capture-id>/intel-uhd-770.png`), never a rename of the committed set. No variant-selection
logic exists today — the headroom is in the path shape only, waiting for the day a second adapter
runs the gate or the first discrete-only feature needs a fallback path.

Artifact paths are deliberately stable across runs — no process id, no timestamp — so that a passing
run can find and remove the previous run's stale mismatch files. Clearing is by explicit filename
allowlist, never a recursive directory delete, because the artifact root is caller-supplied and
`remove_dir_all` on caller-supplied input is a foot-gun aimed at whatever they passed.

The diff image sets exactly the failing positions to opaque magenta (255, 0, 255, 255) and carries
the expected image's pixel everywhere else, so the marks read as an overlay on the frame that was
supposed to be produced. It is omitted when the two images differ in dimensions — there is no
position-by-position diff between frames that share no positions — and the omission's reason is
recorded in the mismatch report instead.

#### The environment opt-ins

| Opt-in | Effect |
|--------|--------|
| `MYCRAFT_ALLOW_NO_GPU` | Downgrades "no usable adapter" from a hard failure to an announced skip whose warning contains that literal string |
| `MYCRAFT_UPDATE_GOLDENS` | The only way a golden is ever created or overwritten |
| `MYCRAFT_SKIP_PERF_BUDGET` | Waives the mesher benchmark's timing comparison only, never its work assertions; see "The mesher budget" under Performance below |

All three are a contract, not a convenience. The default with no GPU present is hard failure,
deliberately: a silent skip would let the gate go green while verifying nothing, which is exactly
the risk ADR-008 accepts on this harness's behalf, and a skip that does not announce itself would
make that risk invisible on top of accepted. A missing golden fails the same way — it never mints
itself, it fails and writes the captured image as an artifact, naming the missing path. Presence of
any of the three variables enables it, not its value: `MYCRAFT_ALLOW_NO_GPU=0` still enables the
skip and `MYCRAFT_SKIP_PERF_BUDGET=0` still skips the timing comparison, because a variable someone
bothered to set is a request.

The mechanism is only half of it. A changed golden must be justified in the commit that changes it;
an unexplained golden update is a review stop (`validation-calibration.md`). `MYCRAFT_UPDATE_GOLDENS`
makes minting a golden a deliberate act rather than an accident — it cannot make it a *justified*
one.

#### Reporting and provenance

Every mismatch report is JSON, carrying the adapter name, backend, driver description (normalised to
the literal `"unknown"` when the adapter reports none, so a reader can tell "the adapter did not say"
from "nobody looked"), the three thresholds the verdict was judged against, the failing-pixel count,
and the maximum per-pixel distance found. Every golden the harness writes records the adapter that
produced it in its `.provenance.json` sidecar, which is what lets the per-adapter-golden deferral end
by adding files rather than by migrating an already-committed set whose provenance nobody recorded.

One golden is committed today, and it is **CPU-generated**, deliberately not captured from real
hardware — baking this machine's adapter into the repository as the one committed golden would
pre-empt the per-adapter-golden deferral before it has a reason to end. Its purpose is narrower than
verifying a frame: it exercises the read-a-committed-golden-from-git path here, on a synthetic fixture
with a sidecar that honestly records it as synthetic, rather than for the first time against a real
rendered frame in PRO-852, where a failure would be ambiguous between a wrong renderer and a wrong
golden workflow.

The harness supplies the capture path only. The scene-side half of golden testing — a fixed world
seed, a scripted camera path and a fixed tick count, which together are what make a real frame's
inputs byte-identical every run — belongs to the crate that owns a world and a camera, and lives in
the terrain replay: a seeded 4×4-column world, a 120-tick camera orbit computed from the tick index
by a free function with nowhere to accumulate into, and **one tick per rendered frame with no wall
clock anywhere in the path**. Advancing by frames rather than by elapsed time removes the
nondeterminism instead of isolating it; the cost is that orbit speed varies with refresh rate, which
a scripted demo can afford.

#### Assertions that do not come from a golden

A golden is minted from the renderer it verifies, so it cannot tell a correct renderer from a broken
one. The **derived probes** can, and they are what makes re-shooting a golden set safe: every
assertion they make about a captured frame is computed from the declared camera, world and colours —
the projected pixel a known landmark must occupy, the row the horizon must fall on, the fraction of
the frame that must lie more than ΔE 10 from the clear colour, the share of pixels clustering within
ΔE 10 of each **declared** placeholder mean colour. **No probe reads an expected value out of any
committed image**, which is exactly the property a golden lacks.

Probes reuse the harness's metric through the public `compare` API rather than reimplementing CIE76:
build a uniform image of the declared colour at the frame's size and compare, then read the failing
mask or the failing fraction; for a bare distance between two colours, compare two 1×1 images and
read `max_delta_e`. Comparing 1×1 images to obtain a scalar reads as a workaround and is deliberate —
`delta_e` is the sole place a distance is computed and the sole swap point if CIE76 is later replaced,
and a second copy would let goldens and probes silently judge by different metrics after such a swap.

**Each probe carries its own negative control**, because a probe suite that cannot fail is
indistinguishable from one that passes: a blank frame must fail the coverage probe, a vertically
flipped frame must fail the horizon probe, and a horizontally mirrored frame must fail the landmark
probe. The controls are asserted, not merely defined — the mirrored control in particular only works
because the landmark sits far enough off centre that its mirror lands on sky; an earlier placement
put the mirror inside the landmark's own width, where the control **could not fail**.

Two standing consequences:

- **Every probe threshold is derived from a screen-space budget computed from the declared camera
  positions.** Change the camera path — even by exchanging its sine and cosine, which merely rotates
  the orbit a quarter turn — and every threshold is silently invalidated while the probes still pass.
  The camera path is pinned by its own scenario for that reason.
- **When a measured figure lands under its threshold, the model is wrong and the work escalates.**
  The threshold does not quietly move to accommodate the measurement; that is how a derived bound
  decays into a snapshot.

#### The scene contract, and the golden inventory

Goldens assert the *picture*. The scene contract asserts the **world that was captured**, and closes
the gap that an all-stone world, or a world at the wrong heights, satisfies every geometry assertion
and is then captured into its own goldens. It follows the mesher benchmark's ordering: work
assertions first, never waived; the snapshot only after they pass.

- **Correctness, with no committed number.** An oracle walks every voxel through the public
  per-voxel API with its own six explicit signed offsets and shares no code with the mesher or the
  geometry builder. The summed area of the emitted quads must equal the oracle's total visible face
  area, **and must equal it per block name** — that second equality is what an all-stone world fails,
  by name, in a message naming the block whose area is wrong. Area is the right invariant and quad
  count is not: greedy merging changes how faces are grouped but never which faces are visible.
- **Numbers derived by arithmetic**, on the same standing as the mesher's `12 288 = 2048 × 6`: total
  `+Y` visible area 4096 (one top face per column, a heightmap with no overhangs), split 4095 grass
  and 1 stone (every column's surface is grass except the landmark column, whose cap is stone), and
  total `−Y` area 4096 (the world floor, whose neighbour is absent). The 4095/1 split is what a
  missing landmark fails, and it costs nothing.
- **A labelled change-detection snapshot.** The scene's quad count is a committed snapshot and says
  so: it verifies nothing, and its only job is to **fail first** — before any image comparison — the
  moment the mesh contract moves. On the day ambient occlusion narrows the merge predicate, every
  area assertion above still holds while the quad count changes, so this is the test that speaks, and
  its message names the remedy: bump the scene revision, delete the previous revision's golden
  directories, re-shoot under the opt-in, justify it in the commit.
- **The inventory test** then asserts `crates/mc-render/goldens/` holds exactly the capture ids the
  current scene revision produces. A bumped revision *renames* the directories, so the orphaned
  previous set fails the gate until it is deleted and the commit shows added and removed PNGs rather
  than a modified binary blob.

This chain exists because documentation was losing to a one-command shortcut. `MYCRAFT_UPDATE_GOLDENS`
is one command, produces green, and is available by design; a procedure that competes with it is a
hope, not a policy. The quad-count failure is not fixable by that opt-in — only editing a constant
fixes it, and that edit lands in the diff beside the revision it should have bumped.

**The residual leak, stated.** A developer who edits the quad count *without* bumping the revision
reaches green. Closing that completely means deriving the revision from a hash of the contract so the
golden set renames itself, at the cost of opaque directory names and a change to the declared capture
ids. Not taken: what remains is a one-line edit beside an explanatory comment, which
`validation-calibration.md` already makes a review stop.

See `technical/rendering.md` for the orientation and pixel-format contract this harness asserts —
recorded there once, not repeated here, because it binds every future caller of draw work, not only
this harness. That file also carries a warning any future golden of rendered terrain depends on:
adding ambient occlusion narrows the mesher's merge predicate and changes quad counts, so every
golden captured against today's mesh is invalidated when AO arrives.

### Multiplayer — bot clients

`mc-testkit` runs headless bots against the **real** network stack. A test that bypasses the
transport is not a network test.

- Scripted behaviour: join, authenticate, move, edit, chat, disconnect.
- Machine-readable output: tick time distribution, p99 latency, bandwidth per class, desync count.
- Scale target: 32 concurrent bots, p99 tick < 25 ms, zero desyncs over 10 minutes.

### Security — adversarial bots

The M4.5 suite is a baseline, not a ceiling. Each attack is a test asserting rejection *and*
logging, with the server unaffected: speed hack, reach hack, edit flood, chat flood, inventory
dupe, malformed and oversized packets, replayed auth challenges.

`proptest` fuzzes packet structure; explicit cases cover known attack shapes.

### Scripting — reload and sandbox

- Sandbox escapes get one explicit test each. A test asserting `io` is absent is worth more than
  ten happy-path binding tests.
- Every limit — instruction budget, memory cap, failure-disable threshold — has a test that trips
  it.
- Hot reload is tested for **state preservation**, not merely that new code ran.
- Every reload failure path (syntax error, failed validation, failing mod test, error inside
  `on_reload`) must leave the previous registry serving.
- Mods' own `tests/` run on every reload candidate before the swap, making them a safety
  mechanism rather than a formality.

### Performance — benchmarks as gates

`criterion` benchmarks with committed baselines, so regressions are caught numerically rather than
felt. Key budgets: chunk meshing < 200 µs/section; server tick p99 < 25 ms at 32 players and 500
active NPCs.

#### The mesher budget: a standalone command, deliberately not a gate stage

The chunk-meshing budget is real and measured: `crates/mc-world/benches/meshing.rs`, a
`harness = false` bench target, run as

```
cargo bench -p mc-world --bench meshing
```

This runs as a **standalone command that the quality gate deliberately does not run.**
`CLAUDE.md` principle 4 requires gates to be deterministic, and a wall-clock threshold is not one —
a gate that goes red on a slower machine is a gate people learn to ignore. The command has exactly
two required run points: a spec's own `/sdd-validate`, and MVP exit verification. Both are
deliberate acts by a person who can account for a slow machine, which is exactly why the check does
not live where every gate run would hit it automatically. `cargo nextest` neither runs nor builds
the bench target — confirmed against the real tree with the `harness = false` target present (the
binary listing reports the lib test and the integration tests only, and no bench binary) — which is
what keeps the wall-clock threshold outside the gate without needing a `test = false` entry on the
`[[bench]]`.

Measured figures, recorded as observations with their spread rather than as a single number: the
`terrain` fixture measured **129.885 µs** on one run and **137.452 µs** on another, on the same
machine, against a **< 200 µs** budget — a ~6% spread, which is itself the argument for keeping the
check out of the deterministic gate. `checkerboard` measured **~376–383 µs** against a 1 ms
ceiling. `solid` is reported but unbudgeted by design.

The command enforces a strict ordering: **work assertions for all three fixtures run first, are
never waived, and no timing is measured or reported at all if the work does not check out.** A
mesher that emits the wrong number of quads for a fixture fails before criterion ever runs, so a
"passing" timing can never be measured against a mesher doing the wrong amount of work.

**Two numbers exist per fixture, and only one of them gates.** The command measures its own mean —
warm up, then run N timed iterations under `std::hint::black_box`, mean = total ÷ N as a `Duration`
— and that is the number the exit code is judged against. Criterion's own estimate is printed
alongside it purely as a report; no test reads it, and nothing in the repository compares the two
numbers to each other. This is a deliberate consequence of criterion's own documentation: its
`estimates.json` output is stated to be a private implementation detail whose structure may change,
so the verdict is not built on it.

`MYCRAFT_SKIP_PERF_BUDGET` waives the **timing** comparison only — the work assertions still run
and can still fail, so a waived run never means "verified nothing". As with `MYCRAFT_ALLOW_NO_GPU`,
presence enables it, not value (`MYCRAFT_SKIP_PERF_BUDGET=0` still skips), and it announces itself
in its own output by its own literal name. It is named generically, not after the mesher, so later
budgets — the 25 ms tick p99 above among them — can reuse the same switch.

## What automation cannot check

Stated plainly, because pretending otherwise is how these get missed:

- **Game feel** — jump arc, mining cadence, camera response, input weight. Needs a human. Mitigated
  by making every such value hot-reloadable, so tuning never requires a rebuild.
- **Whether a golden frame looks *good*** — the harness proves it did not change, not that it was
  ever right.
- **Art direction and audio quality.**
- **Whether a scripting API is pleasant to write against** — completeness is provable (the base
  game uses it), ergonomics is not.
- **Pixel-scale correctness in the renderer.** Goldens and share-based probes verify terrain at
  large scale; neither can see a scattered handful of pixels (`technical/rendering.md`). What is
  left is reading the code, and a human looking at the window.

### The renderer has not been visually accepted by a human

**Nobody has deliberately run `cargo run -p mc-client` and judged that the terrain looks right.** The
window has been opened by an agent and did appear, which establishes that the binary starts and
presents; it does not establish what is in the frame, because the thing being checked is precisely
the judgement an agent cannot make. Every automated check the renderer has — goldens, probes, the
scene contract — is consistent with a picture a person would call wrong, and no verdict recorded
anywhere substitutes for that look.

This is the standing verification state of the renderer, not a task that fell off a list: the check
costs one command, and until someone runs it and says so, "terrain renders correctly" rests on the
code review and on assertions that were all written by the same hands that wrote the renderer.

### An absent reviewer and a clean reviewer look identical

A verdict aggregated from several reviewers' structured output can be clean because a reviewer
returned **nothing**, not because it found nothing. This has happened: three specialist reviews over
an 84-file change merged to zero findings, and reading the transcripts showed one of them had
declared a pass with an empty payload in the middle of its own investigation, its final narration
still listing files it intended to open.

The shape is the same one this whole document is about — a green test and an assertion that never
ran are indistinguishable in a summary line. **Zero findings is a result that has to be corroborated,
not accepted**: check that each reviewer actually produced a payload, and treat an implausibly clean
aggregate as a reason to read the parts rather than the total.
