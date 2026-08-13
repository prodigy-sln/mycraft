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

**Stage 2c runs `rustdoc`, because nothing else resolves intra-doc links.** A `[`Type`]` or
`[`module::func`]` naming an item that does not exist compiles, tests, lints and ships in silence —
SPEC-004 carried a dangling `startup::prepare_scene` reference through a green gate exactly that
way, and it surfaced only when a human tried to import the function the doc claimed existed.

The stage runs `cargo doc --workspace --no-deps` under
`RUSTDOCFLAGS="-D warnings -D rustdoc::broken_intra_doc_links"`. It was added with **zero backlog** —
the whole workspace already passed the day it went in — so it never needed a grace period, and a
failure here is always something introduced rather than something inherited. Verified to bite by
planting an unresolved link (rustdoc exits 101) and reverting by hand.

### Never read the gate's result through a pipe

`sdd-gate.ps1 | tail` reports **`tail`'s** exit status, not the gate's. A shell pipeline's status is
its last command's, so a red gate piped into anything that succeeds is indistinguishable from a green
one — and the transcript shows a status line that says so. This has produced a gate reported green
that had in fact failed, with the underlying failure (a test binary vanishing between compilation and
`--list`, from two coverage runs sharing one target directory) invisible until the run was repeated.

Redirect the output to a file, capture the exit code directly from the invocation, and read the file
afterwards. The same caution applies to any wrapper that summarises a command's output: the summary
is not the status, and only the status is the gate.

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

**The player physics repeats the same shape twice, at two different scopes.** Collision fixtures
(`crates/mc-sim/tests/support/solidity.rs`) are hand-built voxel worlds, chosen for what each one
would fail to catch if it were built differently: a flat floor cannot discriminate which column was
consulted, a ledge loses support at a position the fixture's own geometry predicts, and a step gives
adjacent columns different heights — the fixture's shape is a constraint no assertion enforces, so
it is held by the file's own doc comments and by review, the same lesson as the terrain heightmap
above. Over the declared replay, the same independence rule holds at world scale:
`crates/mc-sim/tests/support/overlap.rs` re-derives whether the player's box overlaps a solid voxel
from the world's own per-voxel accessor and the registry, sharing no lookup chain with the physics'
own resolved `SolidVoxels` bitset — an adapter that transposed an axis, saturated a coordinate or
resolved a name wrongly cannot make both sides wrong the same way, which is exactly what would
happen if the invariant borrowed `SolidVoxels` itself. A resting height is checked the same way:
against `surface_height(x, z) + 1`, read from the world's heightmap at the position the player
actually stopped, never against a coordinate committed from a run of the code under test.

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

**The announcement is currently lost in `mc-client`'s two golden suites, and that is a live gap.**
`crates/mc-client/tests/support/frames.rs` builds the skip notice and then discards it, returning
`None` for the device; both suites then early-return `Ok(())`, so under `MYCRAFT_ALLOW_NO_GPU` a run
that rendered not one pixel reports **PASS** rather than a skip — indistinguishable in the summary
from a run that rendered and matched, which is the exact failure this document is otherwise about.
It has not bitten yet, because failure is failure **by default**: a missing adapter is a hard error
unless the opt-in is deliberately set, and nothing in the repository or the gate sets it. That makes
this a trap laid for the first environment without a rendering device — CI on a headless runner, a
contributor's container — rather than a defect showing today, and it is tracked for repair before
any such environment runs these suites. Note while reading the table above that presence rather than
value enables the opt-in, so `MYCRAFT_ALLOW_NO_GPU=0` walks into it too.

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
seed, a declared sequence of inputs and a fixed tick count, which together are what make a real
frame's inputs byte-identical every run — belongs to the crate that owns a world and a player, and
lives in the terrain replay: a seeded 4×4-column world, a 120-tick **intent script** read by tick
index, a spawn derived from that world, and **one tick per rendered frame, once the world is ready,
with no wall clock anywhere in the path**. Advancing by frames rather than by elapsed time removes
the nondeterminism instead of isolating it; the cost is that the walk's speed varies with refresh
rate, which a scripted demo can afford.

The camera used to be a free function of the tick index — an orbit with nowhere to accumulate into,
so tick 60 could be asked for directly. It is now the camera the simulation *publishes*, derived
from an integrated player, so a frame's pose is reached only by advancing the script from the spawn.
The reproducibility that a golden depends on comes from the world and the script both being
declared, and is claimed within a run rather than across libm versions — which was already true of
the orbit, whose path went through `cos`/`sin` too.

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
  position.** Change the pose — even by exchanging a sine and a cosine, which merely turns it a
  quarter turn — and every threshold is silently invalidated while the probes still pass. That is
  why the probes' pose is **declared and written out** rather than taken from the simulation, and
  why the player's own camera is judged instead by a ray marched through the world's voxels, which
  needs no threshold to invalidate.
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

### The renderer has been visually accepted by a human — and what that does not cover

A human ran `cargo run -p mc-client` from the checkout and judged the live window against the three
committed goldens, reporting that the frame matches them. That is the one check no automated verdict
substitutes for: every check the renderer has — goldens, probes, the scene contract — is consistent
with a picture a person would call wrong, because the goldens are minted from the renderer they
verify. The acceptance is what closes that circle, and it is why it is recorded here rather than
left implicit in a spec folder that no longer exists.

**Three things the acceptance deliberately does not cover**, all known and owned elsewhere, so that
a later reader does not read "accepted" as "the picture is finished":

- **Texture minification aliasing.** `Nearest` sampling is load-bearing for the goldens, and the
  banding and blotching it produces are visible in live playback, not only in stills.
- **The placeholder palette's separation.** Stone and dirt land 47 RGB units apart while either sits
  roughly 190 from grass. Two independent observers have now misread dirt as stone — once costing a
  full ray-cast investigation into a mesher defect that did not exist. The cost of the thin
  separation is already being paid, not merely projected.
- **Pixel-scale correctness**, per the entry above. The acceptance is a judgement about the picture
  at a glance; a scattered handful of wrong pixels survives it exactly as it survives the probes.

The first two are owned by the texture-sampling and palette work; the acceptance narrows what is
unverified rather than clearing it.

### Manual acceptance — walking, looking, and the pointer

**A procedure to repeat, not a note about one afternoon.** Run it after any change to the client's
window, event loop, input adapter, or camera; run it again before any release.

Why it exists: every check below was, until SPEC-006, verified only as *policy* — the input
bindings, the pointer adapter, the cursor fallback policy — never as *product behaviour*. The winit
`ApplicationHandler` in `crates/mc-client/src/events.rs` needed a real window, and nothing in the
suite constructed one, so no automated test observed a keystroke or a mouse motion reaching the
simulation. That was measured, not feared: with `app.rs` submitting `MovementIntent::default()` every
tick, and separately with the adapter's `accepts_pointer_motion` gate deleted, the whole workspace
stayed green — a client wired to none of its own input passed.

**SPEC-006 closed three of the six checks — not by driving a real window, but by moving the dispatch
below the point a real window would have had to enter it anyway.** `winit::event::KeyEvent` cannot
be constructed outside `winit`: its `platform_specific` field is `pub(crate)`, with no constructor
and no `Default`. The keyboard seam therefore has to sit below the reduction of a `KeyEvent` to a
physical key and a pressed flag, and a windowed test would not have helped either way — `winit`
synthesizes no key events on any platform, so a window still could not deliver a keystroke. The
harness that replaces checks 1, 2 and 4 (§"The headless client-input harness" below) instead
constructs the real `winit::event::WindowEvent` and `DeviceEvent` values it can build, and enters
below the one reduction it cannot. It opens no window and acquires no GPU adapter — a sentence
elsewhere in this file once claimed it drove a real window, and that was wrong about the mechanism
before the mechanism was known.

Checks **3, 5 and 6** stay, for two different reasons. Escape releasing the pointer and a click
re-capturing it (checks 3 and 5) are driven by no scenario: the requirement that would have asserted
them (FR-5.1) was cut before implementation, being the only one of SPEC-006's requirement groups
behind none of its five exit-criterion mutations and the only one that would have changed what the
client does. The code path is untouched and still runs — `Session::on_mouse_pressed` and `on_key`'s
`KeyKind::Escape` branch are drivable through the same dispatch that replaced checks 1, 2 and 4 — but
nothing asserts it yet, so the manual check is what stands in until a later spec writes one. Whether
the operating system actually captured the pointer (check 6) is not something any harness can reach
at all: it is a fact about the compositor, not about the client's own dispatch. Six checks therefore
become **three**, not one — narrowing further would drop either an unasserted behaviour or a fact no
harness reaches, and neither stopped being true.

**Run these at the physical machine. Never over remote desktop, and never through any session that
virtualises the pointer** — RDP, VNC, Parsec, a VM console, a screen-sharing tool with input control.
All three checks are about pointer *capture*, and a remote session sits between the compositor and
the client precisely where the thing under test lives. It changes the answer in both directions: a
correct build can fail these because the remote layer never delivered a relative motion, and a broken
one can pass because the remote layer supplied a confinement the client never asked for.

That matters more here than it would elsewhere, because these three are the **only** evidence pointer
capture works at all. `winit::event::KeyEvent` is unconstructible outside `winit`, so nothing
automated reaches them and no test would contradict a wrong result — a false failure recorded here
has nothing standing against it, and would send someone changing working code. This is not
hypothetical: runaway look was observed over RDP on a build that behaves correctly at the machine,
filed as **PRO-882**. If the only access available is remote, record the check as *not run* rather
than as failed.

Run `cargo run -p mc-client` from the checkout at the machine itself, then:

| # | Check | Observed |
|---|-------|----------|
| 3 | **Escape returns the cursor.** One press frees the pointer and it becomes visible again. | not since SPEC-006 |
| 5 | **A click re-captures it.** Clicking in the window takes the cursor back and turning resumes. | not since SPEC-006 |
| 6 | **The operating system actually captured the pointer.** While captured, the cursor is hidden and does not leave the window or reach anything behind it — push it hard against one edge and keep turning. | not since SPEC-006 |

**All three were observed by a human on the build before SPEC-006, and none has been re-run since.**
That is what the column is for. SPEC-006 is precisely the kind of change the instruction at the top
of this section names — the Escape branch, the re-capturing click and the whole capture ladder moved
out of the adapter and into `Session` — so the three yeses it inherited became stale on the same
commit that retired checks 1, 2 and 4, and carrying them forward unchanged would have been the
assumption this column exists to refuse. The three checks are the only evidence these facts hold, and
right now that evidence is one refactor old. Reset the column when the procedure is re-run, and
record what was seen rather than what was expected.

Checks 1 (walk with WASD), 2 (turn with the pointer, up is up and right is right) and 4 (the view
does not turn while the cursor is free) are retired from this table: they are now
`every_declared_binding_moves_the_player_along_its_own_axis_and_no_other`,
`pointer_motion_to_the_right_turns_the_camera_toward_the_players_right` /
`pointer_motion_downward_puts_the_cameras_target_below_its_eye`, and
`pointer_motion_arriving_while_the_cursor_is_free_leaves_the_camera_alone` in
`crates/mc-client/tests/{input_dispatch,pointer_dispatch}.rs`.

### The headless client-input harness

`crates/mc-client/tests/support/input/` drives the client's real input dispatch —
`dispatch_window_event`, `dispatch_device_event`, `dispatch_key` in
`crates/mc-client/src/events.rs` — with no event loop, no window and no GPU adapter constructed
anywhere in the process. It is not part of `mc-testkit`: that crate may name no `mc-*` dependency in
any section of its own manifest (invariant 5, asserted by its own `tests/dependency_graph.rs`), and
this harness needs `mc-client` and `mc-sim` types directly. Each scenario binary includes it by path
(`#[path = "support/input/mod.rs"] mod input;`) rather than through `tests/support/mod.rs`, because
that file also pulls in `frames.rs`'s `wgpu` stack — a graphics dependency in a binary whose entire
premise is that none is acquired.

The fixture is a `GroundPlane` — `is_solid(at) => at.y <= 63`, a floor and no wall — with the player
spawned standing on it at `(32.0, 64.0, 32.0)`. `GRAVITY` is `30.0 blocks/s²` and acts every tick, so
a `Solidity` answering `false` everywhere would put the player in free fall and turn every "unchanged"
assertion red against a correct client; a wall would stop `FR-2.1-S1`'s 20-tick walk in four
directions the way `camera_lens.rs`'s `WalledFloor` does after 10.

**Nothing is asserted against a committed coordinate.** Every FR-2 and FR-3 scenario compares two
runs of the same harness that differ only in what was dispatched — a no-input control against the
dispatched run — so the oracle is independent of `WALK_SPEED`, `GRAVITY`, `JUMP_SPEED` and
`LOOK_SENSITIVITY`, none of which this seam touches. Assertions are about direction and sign, never
magnitude: a tick walks 0.075 blocks and 20 ticks walk 1.5, comfortably above float noise.

**Every scenario asserting something is unchanged carries its own control in the same test** — the
same quantity, under the same input undenied, must move — so a client that did nothing whatever
satisfies none of them. The exit-criterion measurement (§"The mutation-count discipline" below) is
what confirmed that working rather than assumed: `FR-3.1-S3` and `FR-2.1-S2/S3/S4` all go red under
mutations that are not theirs, and in every case the assertion that broke is the control, not the
absence claim.

### The mutation-count discipline

A scenario that is green when written has told you nothing yet; the evidence is a mutation applied
by hand to the shipped production code, observed, and reverted (`standards/global/testing.md` §2).
The client-input seam's exit criterion (SPEC-006) is stated as a property of five named mutations,
each required to turn at least one test red, and the finished count — measured at validation, every
run compiled and reporting a test count, every mutation reverted with `git diff --exit-code`
confirmed clean — was:

| Mutation | Site | Scenarios red |
|---|---|---|
| A — advance with `MovementIntent::default()` | `Session::tick` | 11 |
| A′ — drain hoisted out of the simulation guard | `Session::tick` | 1 — FR-3.1-S6 alone |
| B — `accepts_pointer_motion` guard deleted | `Session::on_pointer_motion` | 1 — FR-3.1-S3 |
| C — ladder never walked | `Session::new` | 6 |
| D — transition never reaches the accumulator | `Session::on_key` | 6 |
| E swapped — `on_pointer_motion(*y, *x)` | `dispatch_device_event` | 5 |
| E negated — `on_pointer_motion(*x, -*y)` | `dispatch_device_event` | 1 — FR-3.1-S2 alone |

Three mutations have exactly one falsifier, and that is a durable fact rather than a validation
footnote: it tells a future reader which scenarios cannot be trimmed without silently reopening the
mutation they alone kill.

**A mutation with a plural name has plural spellings.** Mutation E's swap (the two raw pointer axes
exchanged) has four falsifiers; its negation (the vertical axis alone flipped) has exactly one. A
suite that had dropped `FR-3.1-S2` would still have reported "E is killed" had E only ever been
spelled as the swap — the axis and the sign are two different claims wearing one name, and both
spellings must be run.

**A non-biting mutation needs a biting one beside it, or the result is ambiguous.** Phase 2 hoisted
`Session::tick`'s drain outside its simulation guard and watched all 36 tests then in the suite stay
green — not because the scenario meant to catch it (`FR-2.1-S7`) was wrong to exist, but because
`InputState::take_intent` drains the look delta and keeps the held keys, so a key held across an
early drain loses nothing observable. `FR-2.1-S7`'s soundness was established, not assumed, by adding
a second, stronger mutation to the same guard — `else { self.input.clear_held(); }` — and watching
`FR-2.1-S7` alone go red. Without that second mutation, "the first mutation did not bite" cannot
distinguish *the mutation could not reach this scenario* from *this scenario does not watch this* —
the same shape the positive-control rule above states for a structural-invariant test, applied one
level up to a mutation's result.

**The near miss.** Mutation A′ kills exactly one test out of 41 — `FR-3.1-S6`, which asserts that
pointer motion dispatched before a simulation exists is still spent at the first tick after one is
attached. That scenario had been cut from the spec on the claim that `FR-2.1-S7` covered the same
drain guard; phase 2 measured that claim false. Had the cut stood, the finished suite — every other
scenario green, the gate green, validation passing — would have shipped with a mutation carrying
**zero** falsifiers, at a site `docs/technical/architecture.md` §"The client input dispatch" names
explicitly as a decision.

### The break/place mutation table

Raycast targeting, block break and place (PRO-854) is a second data point for the same discipline
SPEC-006 established above, at a different site: the deliverable is the mutation table, not the
scenario count. **15 named mutations, 21 biting spellings, every one measured dead** against the
*finished* suite — `cargo nextest run --workspace --no-fail-fast`, with **467 tests confirmed executed
on every single run**, which is what tells a mutant that failed to compile apart from one the suite
actually killed. Every revert was by re-editing the line by hand, never `git checkout --`, with
`git diff --exit-code` confirmed clean before the next spelling; the production tree finished the
sweep byte-identical to where it started. No `*.proptest-regressions` files were written, because none
of the sweep's failures came from a genuine regression.

Three results did not bite, and are recorded beside their biting neighbours rather than dropped —
because "did not bite" and "the scenario is vacuous" are otherwise indistinguishable (the same rule
`standards/global/testing.md` §2 states for a non-biting mutation generally): a fold of both of
`VoxelWorld`'s extent guards into one at the same site as mutation M, a plainer deletion of both extent
guards at that site (derived from the call chain, deliberately *not* measured — mutating production
code outside a real sweep buys nothing a reading of the code does not already show), and a mis-spelling
of mutation F2 from the tick's already-limited state rather than its pre-tick one.

**A mutation with a plural name has plural falsifier counts, confirmed a second time at a different
site.** SPEC-006's mutation E (above) already established that a swap and a negation wearing one name
are two different claims. Break/place adds a second shape of the same lesson: mutation L ("only the
edited section is dirtied") has two spellings that are *not* the same mutation. Spelled narrowly — at
the marking site itself, replacing the six-neighbour marking loop with marking only the edited section
— it turns exactly one scenario red. Spelled by deleting the helper that supplies a section's meshing
neighbours instead, it additionally strips the neighbours a section is meshed *against*, which is a
**meshing defect, not a marking one**, and turns three tests red rather than one. A sweep that reported
the wide spelling's three reds as mutation L's count would be reporting the spelling rather than the
mutation — exactly the trap the plural-name rule exists to name.

**A mutation whose falsifier would be a hang is not a falsifier.** The raycast's reach bound is a
single site (`docs/technical/architecture.md` §"The editable world"): the traversal stops once the
next voxel's entry distance exceeds the reach, with no second, independent distance check anywhere
else. The mutation that would have removed the bound entirely ("the traversal's distance limit
removed") was struck before it was ever spelled, because `Solidity` is total — it answers `false`
everywhere outside the loaded world — so an unbounded traversal never terminates against a ray that
meets nothing: its "falsifier" would be a hang, not a red assertion, which cannot be measured the way
this discipline requires. The reach comparison's own deletion (a different mutation, at the same
practical site) survived as a real, measurable spelling — but even it had to be designed around: the
10 000-block exit criterion's rig (below) schedules its one deliberately-refused, out-of-reach
operation as a **placement**, never a break. Spelled as a break, the reach-comparison deletion makes
the far target's block disappear on round one; every later round then walks a ray toward a cell that
is now empty, which never terminates against a total `Solidity` — measured at 116 seconds before being
killed, rather than failing outright. A placement leaves the far block standing every round, so the
same mutant terminates and reports red in the usual few seconds. A schedule that quietly used a break
for that operation would trade a clean kill for a timeout, and a timeout is what an automated sweep
records as inconclusive, not as a defect found.

**Fixture geometry decides whether a scenario can grade anything, illustrated twice.** A one-press
scenario aimed straight down at the fixture world's floor would find only the single cell under the
player's feet and nothing beneath it — so a press that latched and re-fired every tick would still
change exactly one block, and the scenario guarding against a latching press would report a kill it
never made. The click-dispatch fixture's aim is derived to avoid exactly that: 280 raw counts of
downward pointer motion, 0.616 rad, 35.29° below level, chosen so a latching press has somewhere
further to go and its own control (the same aim clicked once per tick, which must change more than one
block) can catch the difference at runtime rather than assume it. Separately, the edit-visibility
scenarios pick their fixture section for the same reason: the world's landmark pillar is the one place
in the replay where a whole section holds exactly one solid voxel, showing five faces that cannot
merge with anything. A fixture surrounded by other solid blocks would leave most of an edit's faces
already merged into a neighbour's quad, leaving a quad *count* blind to the very change the scenario
exists to catch — a count cannot see shape, the same lesson the mesher and the scene contract both
turn on. Both cases are the same principle from two directions: an assertion is only as strong as the
geometry it is asked to distinguish, and that geometry is a constraint no assertion can enforce —
it is held by the fixture's construction and by review.

**When two independent refusal checks discard which one fired, no assertion about the outcome can
grade either — mutation O's lesson, one layer lower.** Deleting the block-registration check earlier
in this same feature (mutation O) left the store's own write refusing an unknown name anyway, so both
worlds of that scenario's two-run comparison came back byte-identical and only the refusal's *name*
differed — which is why every placement-refusal scenario asserts the refusal by name
(`Refused(UnknownBlock { .. })`, never `Refused(Storage(..))`), not merely that nothing changed. The
world's own bounds check repeats the same shape one level down, and this time the name is lost before
any scenario can reach it at all: `World::block_at` ends in `.ok()`, which erases *which* `WorldError`
fired, collapsing a section-array bound and a world-extent bound to the same `None`. An out-of-range
index that **wraps** instead of refusing is caught by a scenario, because the wrap actually changes a
different cell — but the plainer bypass, deleting the extent guards outright, is refused just as surely
by a lower-level bound the `.ok()` has already discarded the identity of by the time any scenario-level
assertion runs. No assertion about the *outcome* can tell that bypass apart from correct code, however
the scenario is written. What catches it is a **unit test** that calls the write path directly and
matches the error variant by name — graded outside the scenario↔mutation mapping the rest of the table
uses, and recorded as a weaker guarantee than the rest of the table for exactly that reason, not netted
out against it.

### Persistence: a second entry point onto a tested path is untested until something asserts through it

World persistence produced the same general lesson four times, at four different layers, and it is
worth stating as its own rule rather than as four coincidences: **a property held at one layer, for
one caller of a function, does not transfer to a second caller of that same function until a test
reaches it through that second caller specifically.** Coverage reports that the code ran; only a
mutation reports whether anything was checking, and in every one of these four cases something *was*
running the line and *nothing* was checking it.

| Surface | Held by nothing while | Closed by |
|---|---|---|
| `RegistryVerdict::refuses` ignoring its `accepting` argument | every scenario naming a changed block was independently refused by a missing name too, so a mutation dropping `Acceptance` entirely left the whole suite green | the one scenario where acceptance is the *only* thing standing between a save and a world |
| `load_world` handing back a zeroed player | no test read `LoadedWorld.player` — every scenario about the player read it through a narrower function one layer down | a scenario reading the player's resumed position, yaw and pitch from a real launch, none of the three numbers zero and none matching the generated spawn either |
| the launch ignoring `Acceptance` entirely | the wire from a parsed command-line flag through to the refusal was never asserted end to end, one layer further up | one scenario per direction — always-accept and always-refuse each caught by exactly one test, alone |
| `mc_sim::persistence::save` writing a zeroed player, or an empty world | nothing exercised a save produced by an actual quit and read back by an actual resume, only saves built directly by test fixtures | a scenario for each half, each reddened by exactly the mutation matching it and nothing else |

Each was found by deliberately mutating the shipped code and watching the existing suite stay green
— not by reasoning about the call graph, which is the discipline `standards/global/testing.md` §2
asks for and which the general rule above generalises: a function correct for its first caller can
still be silently wrong for its second, and the only way to know is a test that goes through the
second caller, not the first.

### A verification no test can perform: `sync_all()`

**Removing the `sync_all()` call between writing a save's sibling file and renaming it over the
target reddens nothing, and no test in this codebase can make it redden.** What that flush buys is
survival of one specific failure — the machine losing power, or the process being killed, between
the write and the rename — and that failure is defined by the machine stopping. A test process that
observes its own assertions afterward is, by construction, a process that did not stop; there is no
vantage point inside a running test from which "the bytes reached the disk before the rename" and
"the bytes were still only in a page cache when the rename happened" look different, because both
look identical to everything that runs afterward in the same process on the same live machine.

This is recorded here rather than left implicit so that a reviewer removing the call to simplify the
write path, or a future change restructuring `crates/mc-world/src/persistence/write.rs`'s `filled`
function, does not read a green gate as permission. The call is held by review alone, deliberately,
and that is the correct place for it to be held — not a gap to be closed by inventing a scenario for
a failure mode that only a machine which has actually stopped can distinguish.

### A falsifier list derived from call paths is a hypothesis, not a measurement

Retiring the base game's empty block (`modding/blocks-items.md`) was the third feature to make the
mutation table its deliverable, and it produced a failure mode the first two did not: **falsifier
lists that were written from reading the call paths and turned out to name scenarios the mutation
cannot structurally reach.** Three of them, the same defect three times, each caught only by running
the mutation and watching what actually went red.

- A mutation of the *replay generator* was credited with falsifying a scenario whose test builds its
  own section inside `mc-world` and meshes it directly. That test never runs the generator, so no
  mutation of the generator can turn it red — however plainly the two appear to be about the same
  behaviour.
- A mutation making the section write a no-op was credited with two scenarios it cannot reach: one
  asserts only the edit report, whose `to` field is derived from the block's definition and never
  re-read from the store, so a write that did not happen cannot change it; the other reads the
  collision bitset, which the write path sets whether or not the store write took effect — so the
  mutation makes store and bitset disagree in the one direction that scenario cannot see.
- A row reading "at minimum, every scenario that reads a cell" gave its own sweep no completion
  condition: it could be reported as killed by whatever happened to run, never as *killed*. Its
  falsifier set was rebuilt by following the site's actual consumers — five of them, four able to
  falsify — and the fifth was recorded as **unable to**, which is a property of the code rather than
  a gap in the tests.

Two rules fall out. **Bound a falsifier list by the site's consumers**, enumerated from the code, and
treat a consumer that cannot falsify as a finding to record rather than a scenario to add. And when a
mutation is spelled against a *specific value*, check that the value can bite: refilling empty space
with a **non-solid** block emits no quad and leaves every rendering scenario green, so the same
mutation spelled against a solid block is the one that grades four scenarios instead of two. A single
row claiming both spellings would claim four falsifiers and deliver two.

**Mutual controls are demonstrated, not asserted.** Two scenarios of that feature — every cell above
the surface holds nothing, every cell at or below it holds a block — are each other's control, and
that was *shown*: refilling the sky turns the first red while the second stays green, and deleting
the ground fill turns the second red while the first stays green, because an entirely empty world
satisfies the first vacuously. The second was reported by the test author as unable to be red, which
was true of the RED against the unchanged tree and false of the mutation table. A scenario green on
first compile is not ungraded; it is graded somewhere other than its own RED, and the table is where
that is written down.

### The 10 000-block exit criterion, and what makes it honest

**MVP 1 exit criterion** — read as *this spec's* criterion, not as MVP 1 finished: a scripted replay
drives at least 10 000 successful placements and at least 10 000 successful breaks through the same
request, targeting and edit path a click uses, in one continuous run, and the resulting world is
asserted against the world the schedule says it should produce. Four conditions are what keep that
assertion from grading nothing:

- **The expected world is derived by arithmetic before any `Simulation` exists**, folded from the
  schedule itself rather than read back from a run — the same derived-oracle discipline as everywhere
  else in this document, applied at world scale. The schedule's own operation counts are pinned
  against the criterion **at compile time** — `const _: () = assert!(PLACES >= CRITERION);` and the
  same for breaks — stating the criterion where it cannot drift away from the schedule that has to
  meet it. A schedule edited down below 10 000 fails to build; it is not a shortfall a green run
  could quietly absorb.
- **Aim is computed from the simulation's own published eye, never from the state the test built the
  fixture with**, so the raycast under test is what resolves a chosen cell into a hit, and the schedule
  cannot silently script around it — the aim reuses the server's own state, never its targeting.
- **The schedule deliberately contains operations that must be refused** — a target beyond the 5.0
  block reach, a target whose definition names no block it breaks into, a placement into an occupied
  cell — and `EditReport` is asserted to name the intended refusal for each, so "succeeded at
  everything" and "did the correct thing" are answers a passing run can tell apart.
- **The assertion is over block identities at coordinates across every one of the world's 65 536
  cells, never over an operation count.** The comparison walks the whole extent — a one-column
  footprint, 16 × 256 × 16 — not merely the cells the schedule meant to touch, so a cell changed
  that no operation named is caught alongside a cell that failed to change. A count cannot see
  shape: a rig that changed the wrong cells the right number of times satisfies a bare success
  count and fails this one.

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

**Hand-mutation is only sound with a single writer in the tree.** During SPEC-006's validation a
reviewer observed a concurrent writer touching `crates/mc-client/src/session.rs` while running its
own hand-mutation checks: a reverted mutation reappeared in a later diff, then the file returned to
its original text with no edit of the reviewer's own in between. Nothing was lost — the tree was
confirmed pristine afterward and the gate re-ran green — but the specialist reviewers use
hand-mutation as their primary evidence, and a second writer in the tree invalidates that evidence
rather than merely slowing it down. Worktree isolation for review agents is filed as an open issue
rather than fixed here.

**Isolation is the fix, and an instruction is not a substitute for it — the reason is about
ordering, not about compliance.** A reviewer that is told mid-flight not to hand-mutate may already
have mutated and reverted before the message arrives: an instruction cannot reach work that has
already happened, so a perfectly obedient reviewer and a disobedient one produce the same tree. Any
scheme that depends on reviewers being told in time is therefore unfalsifiable in exactly the way
this document keeps warning about — it cannot be distinguished from one that works. Give each
reviewer its own tree and the question stops being asked.

**Text that was recently reviewed reads authoritative, and that is not the same as safe.** SPEC-006
produced two verification failures worth reading as a pair — either alone reads as bad luck, together
they are a property of how verification behaves here. First, a review nearly vanished:
`persona-product-owner`'s first turn ended without reporting anything, and only a ping recovered it —
the recovered review was the one that cut the spec from 36 scenarios to 17. A lost review and a clean
review look identical from outside, which is the point made above; this is a second instance of it.
Second, a review arrived, was acted on, and stayed half false: a Major finding correctly split an
architecture decision on `App::redraw`'s early returns, and incorrectly on the drain guard inside
`Session::tick` — the false half then propagated through four documents and was the stated reason
`FR-3.1-S6` was cut, which turned out to be mutation A′'s only falsifier (§"The mutation-count
discipline" above). It was restored once the false half was measured against `InputState::take_intent`'s
actual contract, and the finished suite confirmed A′ kills that scenario and nothing else.

The harness build produced the same shape once more, worth recording alongside these two:
`move_pointer` landed as a no-op skeleton first, and all five phase-3 scenarios failed on their
assertions rather than at compile time — but `FR-3.1-S3`'s own claim ("a free cursor's motion changes
nothing") is *true* under a no-op and would have passed on its own. What went red was its control —
the same control §"The headless client-input harness" above describes, without which a no-op adapter
would have satisfied the scenario it exists to catch.

### A review manifest built from the diff omits exactly the riskiest files

The set of files a reviewer is handed is usually derived from `git diff --name-only main...HEAD`.
That set reconciles perfectly against the change and is still the wrong set, because a review's
subject is the union of the diff and the files the task breakdown and the scenario↔test map name —
and the difference between the two is precisely the tests that are **carried**: pre-existing,
deliberately unmodified, and claimed to verify new behaviour while running unchanged.

That claim is the one most worth checking and the least visible. A file absent from a diff is
exactly a file whose test may have quietly stopped discriminating, so a diff-derived manifest drops
the highest-risk cases while appearing complete. It happened on a 61-file change: three carried test
files were omitted, and the two reviewers responded differently in a way worth keeping. One returned
"gap — I cannot see these files" rather than guessing. The other returned **PASS on all three
without ever opening them** — a verdict carrying no evidence, and from the outside indistinguishable
from a real one. Same lesson as the section above, one level up: the absence was in the *input* to
the review rather than in its output.

Two working rules follow. Build the manifest as the union, not the diff. And when files are added
after a review has run, re-decide them in a **supplementary** pass rather than a second pass — a
second pass accepts only new findings of Major or higher, which would give never-reviewed files a
weaker review than the ones already read. A supplementary result is added to the first, and the
combination is marginally weaker than one review over the whole set, since no single reviewer held
all of it. Say so rather than presenting the two as equivalent.
