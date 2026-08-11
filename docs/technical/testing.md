# Testing & Verification

How MyCraft is checked, and — because much of this work is done by an agent that cannot look at a
screen — how results are verified without a human watching.

Governing standards: `standards/global/testing.md` (TDD, coverage, mocking) and
`standards/global/code-quality.md`. Coverage exclusions: ADR-008 in `decisions.md`.

## The quality gate

`scripts/sdd-gate.ps1` must exit 0 at every phase end, before validation, and before completion.
PowerShell 7 is cross-platform, so this is the only gate script — there is no `.sh` twin to drift
out of sync.

Every stage runs even when an earlier one fails, so one invocation reports every problem.

| # | Stage | Tool | Fails on |
|---|-------|------|----------|
| 1 | format | `cargo fmt --check` | any deviation |
| 2 | lint + complexity | `cargo clippy -D warnings` | any lint, incl. complexity thresholds |
| 2b | gpu-free (`mc-testkit`, no default features) | `cargo clippy` + `cargo nextest` with `--no-default-features` | any lint, or any test failure, in the configuration where `wgpu` is absent from the dependency graph |
| 3 | size | built-in | source > 500 lines, tests > 600 |
| 4 | deps | `cargo machete` | any unused dependency |
| 5 | sast | `cargo deny` | vulnerabilities, bad licenses, banned crates, untrusted sources |
| 6 | secrets | `gitleaks` | credentials in the working tree |
| 7 | tests + coverage | `cargo llvm-cov nextest` | test failure, or lines < 80% |

Flags: `-Quick` runs stages 1–3 only, for tight edit loops. `-SkipCoverage` runs tests without
instrumentation. Neither is valid at a phase boundary or in CI.

### Complexity thresholds

`clippy.toml` makes `code-quality.md` §2 machine-enforceable rather than reviewer-enforced:

| Limit | Value | Source |
|-------|-------|--------|
| Function length | 30 lines | code-quality.md §2 |
| Arguments | 4 | code-quality.md §2 |
| Nesting depth | 3 (= 2 nested blocks) | code-quality.md §2 |
| Cognitive complexity | 15 | firmer than clippy's default 25 |
| File length | 500 / 600 test | code-quality.md §2 |
| Banned names | `data`, `info`, `temp`, `result`, `item`, … | code-quality.md §3 |

Also lint-denied workspace-wide: `unwrap`, `expect`, `panic!`, `dbg!`, `todo!`, raw indexing,
`mem::forget`, float equality, swallowed errors. Raw indexing matters most in `mc-net`, where a
length prefix is attacker-controlled.

### Supply chain

`deny.toml` governs stage 5. The license list is an **allowlist**, not a denylist, because the
client ships to players and a copyleft dependency reaching the binary is a distribution problem.
Advisory suppressions require a dated justification inline — never a silent skip.

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

## Verification without a human at the screen

Most of this project is built by an agent that cannot see the game window, cannot feel input lag,
and cannot invite 32 people to a server. Each of those gaps has a specific mechanism, and **the
mechanism is always built before the thing it verifies.**

### Rendering — golden frames

`mc-render` is excluded from coverage (ADR-008), so golden frames are the only automated check on
it. That makes the harness load-bearing, and it exists ahead of the renderer it will verify: the
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
crate. `cargo build --no-default-features` removes wgpu from the dependency graph entirely — not
merely leaves it unused — so a stray `use wgpu::` outside the gated module is a build error rather
than something a reviewer has to catch. The quality gate runs both clippy and `nextest` in that
configuration (stage 2b above), which is what makes the seam a compile-time fact rather than a
convention that can quietly rot: it is the only process in which no GPU adapter can exist at all,
which is what makes assertions like "the process holds no adapter" mean what they say rather than
merely describing a test that declined to acquire one.

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

#### The two environment opt-ins

| Opt-in | Effect |
|--------|--------|
| `MYCRAFT_ALLOW_NO_GPU` | Downgrades "no usable adapter" from a hard failure to an announced skip whose warning contains that literal string |
| `MYCRAFT_UPDATE_GOLDENS` | The only way a golden is ever created or overwritten |

Both are a contract, not a convenience. The default with no GPU present is hard failure, deliberately:
a silent skip would let the gate go green while verifying nothing, which is exactly the risk ADR-008
accepts on this harness's behalf, and a skip that does not announce itself would make that risk
invisible on top of accepted. A missing golden fails the same way — it never mints itself, it fails
and writes the captured image as an artifact, naming the missing path. Presence of either variable
enables it, not its value: `MYCRAFT_ALLOW_NO_GPU=0` still enables the skip, because a variable
someone bothered to set is a request.

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

What the harness does **not** yet supply is the scene-side half of golden testing: a fixed world
seed, a scripted camera path and a fixed tick count, which together are what make a real frame's
inputs byte-identical every run. Those need a world and a camera to exist, so they belong to the
specs that own one. Until then the harness proves its own capture path against computed ground
truth — analytically known pixels on a trivial scene it renders itself — never against a committed
image of something it cannot generate.

See `technical/rendering.md` for the orientation and pixel-format contract this harness asserts —
recorded there once, not repeated here, because it binds every future caller of draw work, not only
this harness.

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

## What automation cannot check

Stated plainly, because pretending otherwise is how these get missed:

- **Game feel** — jump arc, mining cadence, camera response, input weight. Needs a human. Mitigated
  by making every such value hot-reloadable, so tuning never requires a rebuild.
- **Whether a golden frame looks *good*** — the harness proves it did not change, not that it was
  ever right.
- **Art direction and audio quality.**
- **Whether a scripting API is pleasant to write against** — completeness is provable (the base
  game uses it), ergonomics is not.
