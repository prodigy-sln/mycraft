---
spec: SPEC-001
title: Architecture — Headless Frame-Capture Harness
status: binding
rigor: high
jira: PRO-849
created: 2026-08-11
updated: 2026-08-11
author: Claude (sdd-architect)
reviewed-by: persona-architect (Mode B, 2026-08-11) — 1 blocker, 6 major, 8 minor; all folded
amended: 2026-08-11 (sdd-tasks, lead ruling) — D13 added; Integration table gains
  .gitattributes and one committed CPU-generated golden; Data section records it
amended: 2026-08-11 (sdd-implement phase 3, lead ruling) — D13 generalised from a
  list of types into a placement rule with precedence over the module map and
  § Interfaces; § GPU layer's CaptureContext field list corrected to what the
  implementation holds
amended: 2026-08-11 (sdd-implement phase 3, lead ruling) — remaining stale lines
  brought in line with the approved design: CaptureContext::device/limits and
  CaptureRequest::new added to § GPU layer, the capture sequence's repeat of the
  old field list corrected, and § Error contracts gains the four scenario-less
  variants phase 3 added
---

# Architecture: Headless Frame-Capture Harness

The organising idea of this design, from which almost everything else follows:

> **The GPU layer gathers facts and executes. Every branch that decides
> something lives in a pure function that never sees a device.**

Adapter preference, limit checking, size validation, row unpadding, deadline
expiry, comparison, diffing, golden lifecycle, report shape and path
construction are all decisions. None of them needs a GPU, and none of them is
allowed to have one. What is left in the wgpu layer is: create an instance,
enumerate, request, allocate, encode, submit, map, copy bytes out.

The rule has one deliberate limit, added after review: a decision lifted into a
pure function is only as good as the wiring that calls it, so where wgpu itself
can be provoked into the real failure cheaply, the real failure is tested
instead of the stand-in. See D2.

## Drivers

### Quality attributes

| Attribute | Why it matters here | Evidence |
|-----------|---------------------|----------|
| **Testability without a GPU** | `mc-testkit` is **inside** the coverage denominator (ADR-008 excludes only `mc-render`/`mc-client`/`mc-server`). Every line of harness logic counts against the 80% line threshold, and GPU-resident code is the expensive kind to cover | `scripts/sdd-gate.ps1` `$CoverageExclude`; `docs/technical/decisions.md` ADR-008; spec "Structural constraints" |
| **Verifiability of the harness itself** | This spec is the load-bearing half of ADR-008. A harness that cannot be tested is an exclusion with nothing behind it | spec Goal; `requirements.md` rigor decision |
| **Determinism** | FR-3.4-S1 (order symmetry), FR-4.3-S2 (byte-identical diffs), FR-4.4-S4 (unchanged golden bytes). A flaky verdict makes the whole suite a rubber stamp | FR-3.4, FR-4.3-S2, FR-4.4-S4 |
| **Evolvability at two named points** | (a) CIE76 → CIEDE2000 behind the same three-threshold contract; (b) per-adapter golden variants at a named trigger | spec Technical Considerations; `requirements.md` D1, D9 |
| **Operability / loudness** | A missing GPU must be a red gate, not a quiet skip. A missing golden must not mint itself | FR-1.2, FR-4.4; `requirements.md` D2, D4 |
| **Liveness** | A lost device must not hang a test run indefinitely | FR-2.3; `requirements.md` D7 |

Performance is explicitly **not** a driver — capture speed is Out of Scope, and
the FR-2.3 deadline is a liveness bound, not a budget.

### Constraints

| Constraint | Consequence for this design |
|------------|-----------------------------|
| `unwrap`/`expect`/`panic!`/`indexing_slicing` lint-denied, `-D warnings` | Every path is fallible-by-return. Slice access via `get`/`chunks_exact`. wgpu's own panic-shaped validation (oversized texture) must be pre-empted by a pure check (FR-2.2-S3) |
| `unsafe_code = "warn"` + edition 2024 | `std::env::set_var` is `unsafe` in edition 2024 (verified: `rustc 1.97.1 --edition 2024` rejects it, `--edition 2021` accepts), so **tests must not set environment variables** — doing so needs an explicit `#[allow(unsafe_code)]`, which is exactly the kind of escape hatch this gate exists to make visible. Environment state is injected as a value instead (D3) |
| `integer_division` denied | Row-padding math uses `(width * 4).next_multiple_of(256)`, never `(x + 255) / 256 * 256` |
| `let_underscore_must_use` and `map_err_ignore` denied | Best-effort stale-artifact deletion (D7) cannot be written `let _ = fs::remove_file(p);` or `.map_err(\|_\| ..)`. Sanctioned shape: `if fs::remove_file(&path).is_err() { /* audit #22: recovery is Out of Scope */ }` |
| `missing_debug_implementations` warn | Every public type declared in Interfaces needs `Debug`, including `CaptureContext`, `SkipNotice`, `GoldenSettings`, `Progress<T>`, `Elapsed<T>` |
| `missing_errors_doc` / `missing_panics_doc` warn | Fire only on publicly-visible items. Every function made `pub` gains a mandatory `/// # Errors` section — a real cost that shapes D10 |
| `too_many_lines = 30`, `cognitive_complexity = 15`, `excessive_nesting = 3`, `too_many_arguments = 4` | The golden lifecycle (the most branch-heavy piece) must be decomposed into named small functions, and multi-parameter calls take a settings struct |
| Gate size stage (`sdd-gate.ps1:122`): 600 lines for `tests/`, `benches/`, **and any file matching `*_test.rs` in any directory**; 500 otherwise | An inline `#[cfg(test)] mod tests` counts against 500, but a `#[path = "x_test.rs"] mod tests;` sibling gets 600 and still sees private items (D10) |
| Gate lint stage (`sdd-gate.ps1:111`) runs clippy `--workspace --all-targets --all-features`; coverage (`:235`) runs default features | **No gate configuration currently builds without the `gpu` feature.** This is why B1 below is a build-order requirement rather than a nicety |
| Dependencies pinned only in `[workspace.dependencies]` | New pins go in the root `Cargo.toml`; `crates/mc-testkit/Cargo.toml` uses `{ workspace = true }` |
| No hosted CI; one machine, one adapter, for MVP 1 | Justifies the 0.01% area budget and the single-golden deferral |

### External dependencies, classified (architecture-principles §2)

| Dependency | Vendor risk | Regulatory | API stability | Substitutability | Verdict |
|------------|-------------|------------|---------------|------------------|---------|
| `wgpu` 30 | Low — open source, gfx-rs/Rust ecosystem, no pricing or terms | None | **High volatility.** Breaking majors roughly quarterly (22 → 30 inside two years) | Poor — the alternative is raw `ash`/`vulkano`, a rewrite | **Contain, do not wrap** (D1). Deliberately part of the public contract (D6), so the blast radius is stated honestly rather than claimed away |
| GPU driver / adapter availability | n/a | None | n/a | n/a | **Nondeterministic**: availability, name, backend and driver string vary per machine. All decisions over them are pure functions over DTOs (D2) |
| Environment variables | n/a | None | n/a | n/a | **Nondeterministic**: isolated to one lookup function (D3) |
| System clock | n/a | None | n/a | n/a | **Nondeterministic**: behind a `Clock` trait (D4) |
| Filesystem | n/a | None | Stable (`std`) | n/a | `std` — exempt per architecture-principles §3. Real temp directories in tests per testing.md §4 |
| `image` 0.25 | Low | None | Stable within 0.25 | Good (`png` crate directly) | In-process pure library — exempt |
| `serde` / `serde_json` / `thiserror` | Low | None | 1.x stable | Good | In-process pure libraries — exempt |
| `pollster` | Low | None | Trivial surface | Trivially replaceable | In-process pure library — exempt; used only inside the wgpu module |

### Volatile vs. expensive-to-reverse

**Volatile:** the wgpu API; the ΔE metric; the set of adapters on the machine.
**Expensive to reverse:** the on-disk golden and artifact layout (every future
golden depends on it), the capture pixel-format contract (every future golden
is bytes in that format), the public API shape including its wgpu types (seven
downstream specs consume it), and the three-threshold comparison contract.
**Cheap to reverse:** module split within the crate, error enum variants, JSON
field names before any golden exists.

## Boundaries

| External dependency | Volatility (V/R/S/Sub) | Port | Adapter location | Direct-use justification |
|---|---|---|---|---|
| `wgpu` (device, queue, textures, buffers) | Low / None / **High** / Poor | none — see justification | `crates/mc-testkit/src/frame/gpu/**` (only place `wgpu::` may appear inside this crate) | Two independent grounds. **(1)** architecture-principles §3's vendor bullet targets "a third-party vendor that can change pricing, terms, behavior, or existence"; wgpu has no vendor, no pricing and no terms, and §2's sole-provider amplifier addresses commercial lock-in. code-quality §1 puts it on the framework side of the line it draws against "third-party service SDKs (payment, LLM, transcription, mail)", and forbids wrapping a framework "for flexibility". **(2)** More decisively, §3 requires redesign when a port "mirrors the vendor SDK one-to-one": a capture port whose draw-work type *is* `&mut wgpu::CommandEncoder` cannot be implemented by anything that is not wgpu, so it would be theater by construction. The real justification is that **the core never invokes the GPU** — the direction is already `gpu → core`, so there is no call to isolate. **Blast radius, stated honestly:** D6 puts wgpu types in this crate's *public* API, so a wgpu major touches `frame/gpu/**` **plus every consumer's draw-work closure** — not one directory. Accepted, because the only way to shrink it is a scene abstraction that would invert invariant 5. `pub use wgpu;` (under the feature) makes the coupling explicit and guarantees consumers link the same wgpu |
| GPU adapter facts (name, backend, kind, driver, limits) | nondeterministic | `AdapterDescription`, `AdapterProvenance`, `Backend`, `AdapterKind`, `AdapterLimits` — DTOs owned by the core | mapped from `wgpu::AdapterInfo`/`wgpu::Limits` in `frame/gpu/acquire.rs` | — |
| Environment variables | nondeterministic | `OptIns` value type owned by the core, constructed through an injectable lookup | `OptIns::from_environment()` — the only function in the crate that names `std::env` | — |
| System clock | nondeterministic | `trait Clock { fn elapsed(&self) -> Duration; }` | `SystemClock` in `frame/clock.rs` | — |
| Caller-supplied draw work | caller-owned | `DrawWork` trait over `&mut wgpu::CommandEncoder` + `&wgpu::TextureView` | defined in `frame/gpu/mod.rs`, i.e. inside the adapter, re-exported as public contract | The draw work *is* wgpu by nature; a domain-shaped scene type would require the harness to know about renderable content, which invariant 5 forbids (D6) |
| Filesystem | stable | none | `std::fs`, used by `frame/png.rs`, `frame/report.rs`, `frame/golden.rs` | architecture-principles §3 exclusion: the standard library |

## Decisions

### D1 — The GPU-free seam: a data seam inside one crate, plus an optional `gpu` feature — **BINDING**

The central decision. The seam must make the CPU half reachable with no adapter
in the process (FR-3.4-S3, the phase-1 constraint, ADR-008's coverage arithmetic).

| Option | Evaluation against drivers |
|--------|----------------------------|
| **A. `trait FrameCapture` port with an associated `DrawWork` type**, golden pipeline generic over it | Achieves containment. But nothing in the golden pipeline needs to *call* capture — it needs an image. The trait would exist to type one composition function, could not be used as `dyn`, and its associated type would be wgpu-only: isolation theater under §3's own redesign rule |
| **B. Data seam: the core consumes `Rgba8Image` + `AdapterProvenance` values; the wgpu module produces them and owns composition. Reinforced by a default-on `gpu` Cargo feature that gates the wgpu module and its dependencies** | Every CPU-half function takes plain values, so tests construct inputs by hand — no fake, no trait, no GPU. `--no-default-features` makes the seam a **compile-time fact**: wgpu is not in the dependency graph, so an accidental `use wgpu::` in `compare.rs` is a build error, not a review finding. Phase-1 TDD also builds in seconds instead of compiling wgpu |
| **C. Split into two crates (`mc-testkit` + `mc-testkit-gpu`)** | Strongest enforcement, but adds a workspace member the spec does not authorise, and FR-6.1 is written about `mc-testkit`'s graph. Disproportionate |

**Recommendation: B.**

```toml
# crates/mc-testkit/Cargo.toml
[features]
default = ["gpu"]
gpu = ["dep:wgpu", "dep:pollster"]
```

```rust
// crates/mc-testkit/src/frame/mod.rs
#[cfg(feature = "gpu")]
pub mod gpu;
#[cfg(feature = "gpu")]
pub use wgpu;    // consumers link the same wgpu; the coupling is explicit
```

**The feature must be exercised, or FR-3.4-S3 is tested vacuously.** FR-3.4-S3
reads "WHILE the process holds no GPU adapter…". In the default build wgpu is
linked and any test process *could* acquire an adapter, so a test that simply
declines to call `acquire` demonstrates "we did not call it", not the state the
scenario names. The only process in which no adapter *can* exist is the
`--no-default-features` build, and today no gate configuration produces one:
`sdd-gate.ps1:111` runs clippy `--all-features`, `:235` runs coverage with
default features.

Binding consequence — **phase 1 is not done until this is green**:

```
cargo clippy   -p mc-testkit --no-default-features --all-targets -- -D warnings
cargo nextest run -p mc-testkit --no-default-features
```

A `cargo check` is not sufficient: it compiles the configuration without running
FR-3.4-S3 in it.

**Recommended and deliberately not done unilaterally:** add those two lines to
the gate's lint stage, so the guarantee survives past phase 1. That is a change
to shared tooling for one crate's benefit, so it is flagged for the lead rather
than made here. If it is declined, the phase-1 command above still holds, but
the seam's enforcement decays to convention after this spec ships — which is the
same failure shape ADR-008 exists to avoid, and should be recorded as accepted
risk rather than forgotten.

Note on feature unification: once any workspace member enables `gpu`, a
consumer's `default-features = false` is unified back on. The standalone
`-p mc-testkit --no-default-features` invocation is therefore the *only* place
the seam is real, which is another reason it belongs in the gate.

**Strongest argument against B:** a Cargo feature only enforces the seam when
someone builds that configuration, and a second build configuration can rot
silently. Both are mitigated by the same gate line, and by nothing else.

### D2 — Decisions are pure functions; real wgpu failures are provoked where that is cheap — **BINDING**

Most of FR-1.1 and all of FR-1.2 would otherwise be **untestable on the
development machine**, which does have a GPU, and whose environment variables a
test must not set (edition 2024). The decisions are therefore lifted out of the
I/O:

```rust
// frame/selection.rs — no wgpu, no env, no clock
pub fn select_preferred(candidates: &[AdapterDescription]) -> Option<usize>;      // FR-1.1-S5
pub fn unsatisfied_limit(required: &AdapterLimits, available: &AdapterLimits)
    -> Option<UnsatisfiedLimit>;                                                  // FR-1.1-S4
pub fn classify_acquisition(outcome: Result<AdapterDescription, AcquireError>,
                            opt_ins: &OptIns) -> AcquisitionVerdict;              // FR-1.2-S1..S3

// frame/image.rs
pub fn validate_frame_size(width: u32, height: u32, max_dimension: u32)
    -> Result<FrameSize, FrameSizeError>;                                         // FR-2.2-S2/S3

// frame/readback.rs
fn padded_row_bytes(width: u32) -> Result<u32, ReadbackError>;           // private — 256-byte align
pub fn unpad_rows(padded: &[u8], row_bytes: usize, padded_row_bytes: usize,
                  height: u32) -> Result<Vec<u8>, ReadbackError>;                 // FR-2.2-S1
```

**Visibility in this block is illustrative; D10 is authoritative.** The point
here is that each of these is a *pure function over plain values*, not that each
is public. `padded_row_bytes` is **private** (amended 2026-08-11, lead ruling —
it previously read `pub` here and `private` in D10 and tasks T05, and a
contradiction left on disk gets resolved differently by whoever reads it next).
It is tested through its `readback_test.rs` sibling and, until the phase-3
capture path calls it, carries `#[cfg_attr(not(test), expect(dead_code))]` naming
that caller — `expect` rather than `allow`, so the annotation becomes a warning
the moment T25 wires it and cannot rot silently.

`unsatisfied_limit` and `classify_acquisition` are **also private** (settled
2026-08-11, lead ruling), as D10 always said and as this block's `pub` markers
wrongly suggested otherwise. Neither is a caller-facing capability: the harness's
public surface is capture, compare and verify; `classify_acquisition` is internal
policy and `unsatisfied_limit` an error-detail helper. Widening a public API so a
test can reach it is the API leaking to serve its tests.

The consequence lands on **tasks T14, which changes**: FR-1.2-S1/S2/S3 move from
`tests/` into `frame/selection_test.rs`, which is exactly what the D10 sibling
pattern exists for. `AcquisitionVerdict` is `pub(crate)` for the same reason —
phase 3's `gpu/acquire.rs` calls `classify_acquisition` from inside the crate, so
nothing needs it public. T13 already complied and is unchanged.

Note the timing, because it changed the answer: until the coverage denominator
excluded `*_test.rs` siblings, pushing tests into them carried a real cost —
they inflated the figure. With that removed the choice stands on its merits, and
on the merits internals get sibling tests. **This does not weaken the phase-2
`--no-default-features` DoD:** every type involved is core-side (D13 places
`AcquireError` there for precisely this reason), so no sibling names a `wgpu`
type. Confirmed empirically by phase 1, whose `color_test.rs` and
`readback_test.rs` siblings run in that configuration today.

**Adapter ranking (FR-1.1-S5):** `Discrete > Integrated > Virtual > Other > Cpu`,
ties broken by enumeration order. `Cpu` ranks **last**, below `Other`, because
`Cpu` is the only variant that definitively means a software rasteriser
(lavapipe, WARP) while `Other` means "unknown" and is reported by real hardware
on GL/ANGLE and some Vulkan drivers. Ranking `Other` below `Cpu` would silently
select a software adapter on such a machine and mint goldens from it — an
FR-1.1-S5 violation nobody would notice until cross-adapter drift appeared. This
ordering is the tested contract.

**Where a pure function is not enough.** `classify_acquisition` fed a
hand-constructed `AcquireError` proves the formatting, not the trigger, and
leaves FR-1.1-S3's and FR-1.1-S4's "without panicking / without creating a
window" clauses unasserted. wgpu provides both real triggers at no
infrastructure cost, so both scenarios are **real GPU tests**:

```rust
pub struct AcquireOptions { pub backends: wgpu::Backends, pub required_limits: wgpu::Limits }
impl Default for AcquireOptions { /* Backends::PRIMARY, Limits::downlevel_defaults() */ }

pub fn acquire(opt_ins: &OptIns, options: &AcquireOptions)
    -> Result<Acquisition, AcquireError>;
```

- **FR-1.1-S3:** `backends: Backends::BROWSER_WEBGPU` enumerates zero adapters on
  native, so `request_adapter` fails for real on a machine that has a GPU, with a
  non-empty `tried` list. (`Backends::empty()` would make the assertion
  degenerate.)
- **FR-1.1-S4:** `required_limits.max_texture_dimension_2d = u32::MAX` is rejected
  by wgpu's own limit validation, so a real `RequestDeviceError` flows through
  the real mapping into `DeviceRejected { adapter, requirement }`.

`AcquireOptions` also answers what `required` *is* in production — without it,
`unsatisfied_limit` would be a function with no production caller and FR-1.1-S4
would have no trigger path at all. Default is `Limits::downlevel_defaults()`,
which covers everything this harness needs (one 2D colour target).

Secondary benefit worth naming: these two tests cover `gpu/acquire.rs`'s error
branches, which is the cheapest available insurance against the coverage risk
listed in Risks, since `mc-testkit` is inside the denominator.

**Remaining counter-argument:** FR-1.1-S5's wiring is still only observed
indirectly, through FR-1.1-S2 (the selected adapter's name and backend are
reported). A bug where `acquire` ignores `select_preferred` and takes
`adapters[0]` passes every unit test. Accepted: forcing a two-adapter machine is
not available, and the spec itself anticipates selection being "a pure function
over an enumerated candidate list".

### D3 — Environment opt-ins are a value built through an injectable lookup — **BINDING**

```rust
// frame/optins.rs
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OptIns { pub allow_no_gpu: bool, pub update_goldens: bool }

impl OptIns {
    /// Presence, not value, enables an opt-in.
    pub fn from_lookup(lookup: impl Fn(&str) -> Option<OsString>) -> Self;
    /// The only function in this crate that names `std::env`.
    pub fn from_environment() -> Self { Self::from_lookup(std::env::var_os) }
}
```

| Option | Evaluation |
|--------|------------|
| `trait Environment { fn var(&self, key) -> Option<String> }` | Mirrors `std::env` one-for-one — the shape architecture-principles §3 calls a defect and orders redesigned |
| `trait OptIns { fn allow_no_gpu(&self) -> bool; ... }` | Domain-shaped, but two booleans need no laziness and no dynamic dispatch |
| **Plain value + injectable lookup + one `from_environment()`** | `OptIns` *is* the port, as a data port rather than a behavioural one, and it isolates more strongly than the trait §3 literally asks for: behind a trait any module could still call `env.var("MYCRAFT_…")` with a typo, whereas here the two variable names exist in exactly one function. `grep -rn 'std::env' crates/mc-testkit/src` returns one file |

**Recommendation: the value struct.** `from_lookup` is what keeps the contract
itself tested: FR-1.2-S2 requires the literal string `MYCRAFT_ALLOW_NO_GPU`, and
Assumption 5's presence-not-value semantics (`=0` still enables the skip) are
asserted by passing a fake lookup. `from_environment` shrinks to a single
uncoverable line.

**Strongest argument against:** a literal reading of architecture-principles §3
mandates a *port* for environment, and a struct is not an interface. I hold that
the rule's purpose — the domain never reaches into ambient state — is fully met
with fewer moving parts, and the deviation is declared here rather than hidden.

### D4 — The deadline wait is a pure loop over a `Clock` and a step closure — **BINDING**

```rust
// frame/clock.rs
pub trait Clock { fn elapsed(&self) -> Duration; }
pub struct SystemClock(Instant);          // SystemClock::started_now()

pub enum Progress<T> { Ready(T), Pending }
pub struct Elapsed<T> { pub value: T, pub elapsed: Duration }

pub fn poll_until_deadline<T>(
    clock: &dyn Clock,
    deadline: Duration,
    step: impl FnMut() -> Result<Progress<T>, ReadbackError>,
) -> Result<Elapsed<T>, DeadlineExpired>;
```

The wgpu layer supplies a `step` that polls the device non-blockingly and reports
whether the `map_async` callback has fired. `DeadlineExpired` maps to
`CaptureError::ReadbackTimeout { capture: CaptureId, deadline: Duration }` so
FR-2.3-S2 names both. FR-2.3-S1's "elapsed readback time" is `Elapsed::elapsed`.

A trait is right here, unlike D3, because time is a *stream of observations*, not
a value read once: FR-2.3-S2 requires the deadline to elapse **during** the wait
(audit fix #15), which a fake clock advancing per call produces in microseconds.
This is the whole reason a 30-second scenario is testable.

**Counter-argument:** the real wait must also avoid spinning a core. Sleeping
belongs in the caller's `step` closure and is easy to forget. Recorded as an
implementation note rather than a design change — `poll_until_deadline` must stay
free of sleeping, so the fake-clock test does not sleep either.

### D5 — Capture format: `Rgba8UnormSrgb` render target, bytes copied verbatim — **BINDING**

| Option | Evaluation |
|--------|------------|
| `Rgba8Unorm` (linear) target + CPU sRGB encode on readback | An extra CPU stage that can be wrong, on the exact axis FR-2.1-S4 tests. Rejected |
| **`Rgba8UnormSrgb` target; readback copies texels byte-for-byte** | The hardware performs the sRGB encode, which is the standard path and the one the renderer will use. FR-2.1-S4's "encodes to sRGB (128,128,128)" is a **linear** clear of ≈0.2159 landing on 128±1 — the ±1 absorbs backend rounding |

Binding consequences:

- **No alpha arithmetic anywhere.** A clear of `(1,1,1,0.25)` stores
  `(255,255,255,64)`; nothing premultiplies, and readback never divides
  (FR-2.1-S3). Straight alpha is achieved by *not writing code*, which is why the
  scenario asserts the RGB channels are unscaled rather than an exact alpha byte.
- **No vertical flip at any stage.** wgpu texture row 0 is the top of the render
  target; PNG row 0 is the top. FR-2.4-S2 exists precisely to catch a
  compensating pair of flips, so the contract is "zero flips", stated once and
  asserted asymmetrically.
- Texture usage is `RENDER_ATTACHMENT | COPY_SRC`; one colour target, no depth,
  no MSAA (Out of Scope).

### D6 — The caller owns the render pass, including its load op — **BINDING**

```rust
// frame/gpu/mod.rs
pub type DrawResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

pub trait DrawWork {
    fn record(&mut self, encoder: &mut wgpu::CommandEncoder,
              target: &wgpu::TextureView) -> DrawResult;
}

/// Coercion helper. Passing a bare closure to `capture` makes rustc infer a
/// higher-ranked bound over two independent lifetimes, which it routinely fails
/// at with an opaque error; this fixes the shape at the call site.
pub fn draw_fn<F>(f: F) -> impl DrawWork
where F: FnMut(&mut wgpu::CommandEncoder, &wgpu::TextureView) -> DrawResult;
```

The harness creates the texture and hands out an encoder and a view; the caller
begins its own passes. Rationale: FR-2.1-S1/S3/S4 are clears with *caller-chosen*
colours ("draw work clears the target to opaque red"), and FR-2.1-S2 needs a real
pipeline and draw for the top-half fill, which a harness-owned single render
pass could not express. This is also what keeps the harness ignorant of
`mc-render` (FR-6.1, invariant 5): it supplies a canvas, never a scene.

**Orientation is part of this contract** (amended 2026-08-11, lead ruling).
Because the caller owns the render pass, which way is up is public interface,
not an internal detail:

- **Framebuffer row 0 is the top**, and stays the top through readback,
  comparison, PNG encode and PNG decode. No stage flips rows (D5).
- **Clip-space y is up.** wgpu's framebuffer origin and clip-space y point in
  opposite directions; that mismatch is one of the most common sources of
  flipped output in the ecosystem, and it is the caller's to get right.
- Therefore **a caller filling the top half of the target writes y > 0**.

FR-2.1-S2 is the only scenario that holds this honest, which is why its fixture
is split by row rather than by column. Every other phase-3 assertion is uniform
(FR-2.1-S1/S3/S4), a count (FR-1.1-S1, FR-2.2-S1) or a duration (FR-2.3-S1), and
none is sensitive to row order; FR-2.4-S2 does not close the gap either, since it
runs a hand-built image through the PNG writer and reader with no capture in the
path. The readback chain is thus the only place a row inversion plausibly
originates, and was the only place nothing asserted against one. Consolidate this
convention into `docs/` at `/sdd-complete` so PRO-852 inherits it.

FR-2.1-S5 ("return *that* error") is satisfied by
`CaptureError::DrawWork(#[source] Box<dyn Error + Send + Sync>)` — the caller's
error is preserved in the `source()` chain and is downcastable. A generic `E`
parameter was rejected: it infects every signature and trips `type_complexity`.

The self-verification scene (a solid-colour pipeline and its WGSL) lives in
`crates/mc-testkit/tests/`, not in the library. The library ships no shaders.

### D7 — The golden lifecycle performs its own filesystem I/O; it is not a pure planner — **BINDING**

| Option | Evaluation |
|--------|------------|
| Pure planner returning `Vec<FileEffect>` + a thin executor | Maximally unit-testable, but FR-4.2-S3 (artifact write fails, mismatch must still be reported with the cause named) requires executor errors to fold back into the verdict, which re-couples the halves through a second error channel |
| **Direct `std::fs` inside `verify_against_golden`, tested against real temp directories** | testing.md §4 explicitly prefers a real temp filesystem to mocks. Still 100% GPU-free. FR-4.2-S3 becomes a natural `Result` field |

**Recommendation: direct I/O.** FR-4.2-S3 is provoked portably by pointing the
artifact root at a path whose parent is an existing *file* — creation fails on
both Windows and Linux with no lock-injection machinery.

**Deletion is by explicit filename allowlist, never recursive.** The clear
removes `expected.png`, `actual.png`, `diff.png`, `report.json` if present, and
nothing else. The artifact root is caller-supplied, so `remove_dir_all` on it is
a foot-gun aimed at whatever the caller passed (code-quality §7). The clear runs
on **every non-mismatch path**, not only on a pass: a missing-golden run that
writes a fresh `actual.png` beside last run's `expected.png`/`diff.png`/
`report.json` would present a stale artifact set as current, misleading exactly
the agent this harness exists for. FR-4.1-S2 mandates it only for a pass; doing
it everywhere is free and strictly less misleading.

**Counter-argument:** filesystem tests are slower and can leak directories on a
panicking test. Accepted: `tempfile::TempDir` cleans up on drop, and these stay
inside testing.md's <1s integration budget.

### D8 — Golden and artifact layout, with adapter-discriminator headroom — **BINDING**

```
crates/mc-testkit/goldens/<capture-id>/default.png                  # committed
crates/mc-testkit/goldens/<capture-id>/default.provenance.json      # committed (FR-5.1-S4)

<artifact-root>/<capture-id>/expected.png     # git-ignored (under target/ by default)
<artifact-root>/<capture-id>/actual.png
<artifact-root>/<capture-id>/diff.png
<artifact-root>/<capture-id>/report.json
```

| Option | Evaluation |
|--------|------------|
| Flat `goldens/<capture-id>.png`, later `goldens/<capture-id>@<variant>.png` | Adding variants renames nothing, but the sidecar and every future per-variant file clutter one flat directory, and `@` is a new grammar to parse |
| **Directory per capture, `default.<ext>` inside** | A variant is a **new file in an existing directory** (`goldens/<capture-id>/intel-uhd-770.png`), with no rename of anything committed. The sidecar sits next to its golden. Satisfies the spec's Assumption that the deferral ends by adding files, not migrating them |

The constant `default` is written and read today; **no variant selection logic is
implemented** — that is Out of Scope ("one golden per capture"). The headroom is
in the path shape only.

`CaptureId` is a validated newtype: one path segment, non-empty, `[a-z0-9_-]`,
no separators, not `.` or `..`. It names the capture in
`CaptureError::ReadbackTimeout` (FR-2.3-S2), the golden directory and the
artifact directory — **one identifier, three roles**. Validation has no scenario;
it is input validation on a path-forming public input (code-quality §7) and is
declared in Assumptions for veto.

Roots are **caller-supplied**, never guessed:

```rust
pub struct GoldenSettings {
    pub golden_root: PathBuf,     // typically Path::new(env!("CARGO_MANIFEST_DIR")).join("goldens")
    pub artifact_root: PathBuf,   // typically <target-dir>/mycraft-frames
    pub capture: CaptureId,
    pub thresholds: Thresholds,
    pub opt_ins: OptIns,
}
```

`env!("CARGO_MANIFEST_DIR")` is a compile-time constant expanded in the *calling*
crate, so a future `mc-render` golden test resolves to its own directory — no
runtime environment access, and no assumption about the process working
directory.

**Parallelism (FR-4.1-S3).** `nextest` runs each test in its own process, and
each capture owns a distinct `<capture-id>` directory, so clearing one capture's
artifacts can never touch another's. Paths are deliberately **stable across
runs** — no PID, no timestamp — because FR-4.1-S2 requires today's pass to find
and remove yesterday's stale mismatch files, which a per-run unique directory
would make impossible.

### D9 — Comparison: one scalar distance function is the only CIEDE2000 swap point — **BINDING**

```rust
// frame/color.rs
pub struct Lab { pub l: f64, pub a: f64, pub b: f64 }
pub fn srgb8_to_lab(rgb: [u8; 3]) -> Lab;   // sRGB → linear (256-entry LUT) → XYZ (D65) → Lab
pub fn delta_e(a: Lab, b: Lab) -> f64;      // CIE76 today. THE swap point.
```

The swap stays cheap because of three contracts, all binding:

1. **`delta_e` is the only place a distance is computed.** `compare.rs` receives a
   scalar and never inspects `L*`, `a*`, `b*` or any RGB channel. CIEDE2000
   consumes the same two `Lab` values and returns the same scalar, so replacing
   it is a one-function-body edit in one file.
2. **All three thresholds are expressed in ΔE units**, and every comparison is
   `>` (strictly greater than — which also sidesteps the `float_cmp` lint, since
   nothing is compared with `==`). The verdict logic is metric-agnostic.
3. **No `Metric` trait.** One implementation, and the spec puts other perceptual
   metrics Out of Scope; a trait would be speculative abstraction and would put a
   dynamic call in a per-pixel loop. The cost of the swap is a function body, not
   an architecture change.

`f64` throughout, with a 256-entry `sRGB u8 → linear f64` lookup table computed
once. The LUT removes `powf` from the per-pixel loop and makes conversion
bit-reproducible run to run, which FR-3.3-S1's "state ΔE ≈ 4.67" and FR-4.3-S2's
byte-identity both lean on.

**Counter-argument:** a free function is a weaker seam than a trait — nothing
stops a future contributor inlining a channel comparison into `compare.rs` as a
fast path. Accepted: the contract is stated here and in the module docs, and the
alternative buys dynamic dispatch in a hot loop for a swap that happens once.

**Alpha is not compared.** ΔE is defined over RGB; every spec scenario uses
opaque pixels or asserts alpha behaviour on the *capture* side (FR-2.1-S3), never
on the comparison side. Two pixels differing only in alpha therefore compare
equal. Declared in Assumptions with its revisit trigger.

### D10 — Test placement: `tests/` for behaviour, `*_test.rs` siblings for internals — **BINDING**

The gate (`sdd-gate.ps1:122`) grants 600 lines to `tests/`, `benches/`, **and any
file matching `*_test.rs` in any directory**; an inline `#[cfg(test)] mod tests`
counts against the 500-line source limit. That gives three options, not two:

| Option | Evaluation |
|--------|------------|
| Inline `#[cfg(test)] mod tests` in `src/*.rs` | Counts against 500 and pushes production code out of its own file. Rejected |
| Everything in `crates/mc-testkit/tests/` | 600-line budget, but forces every unit under test to be **public**, and `missing_errors_doc`/`missing_panics_doc` are gate errors on public items — a mandatory `/// # Errors` section for every one |
| **Split**: behavioural and integration tests in `tests/`; internal plumbing unit-tested through a `#[path = "x_test.rs"] mod tests;` sibling | The sibling gets the 600-line budget *and* sees private items, so plumbing stays private |

Applied:

- **`tests/`** — FR-2.x, FR-3.x, FR-4.x, FR-5.x, FR-6.1, and the GPU scenarios.
  These are behavioural and belong there on merit.
- **Public API** — `Rgba8Image`, `Thresholds`, `Comparison`, `compare`,
  `render_diff`, the PNG functions, `FrameReport`, `verify_against_golden`,
  `CaptureId`, `GoldenSettings`, `OptIns`, `Clock`, `select_preferred`,
  `validate_frame_size`, `unpad_rows`. The last three are genuine product
  surface: downstream specs will want the same primitives.
- **Private, tested via `*_test.rs` siblings** — `padded_row_bytes`,
  `unsatisfied_limit`, `classify_acquisition`, and the `srgb8_to_lab` LUT
  internals. Plumbing, not product; keeping them private avoids the doc-lint tax
  and a public surface nobody asked for.

### D11 — FR-6.1 is checked by walking `cargo metadata`'s resolved graph — **BINDING**

```rust
// crates/mc-testkit/tests/dependency_graph.rs
// $CARGO metadata --format-version 1 --locked  (cwd = CARGO_MANIFEST_DIR)
// → parse with serde_json → BFS from the mc-testkit node over resolve.nodes[].deps
// → assert mc-render / mc-client / mc-server appear nowhere in the closure
```

| Option | Evaluation |
|--------|------------|
| Parse `crates/mc-testkit/Cargo.toml` | Only direct dependencies. FR-6.1-S1 says "anywhere in it" |
| Parse `Cargo.lock` | Lists every workspace member regardless of reachability — the assertion would be vacuously false |
| **`cargo metadata` + BFS from the `mc-testkit` node** | The only option that answers the question actually asked |

Details that matter: use the `CARGO` environment variable cargo sets for test
binaries (not a hardcoded `cargo`); run with cwd `CARGO_MANIFEST_DIR`; follow
**all** dependency kinds (normal, build, dev) from `mc-testkit`'s node — the
strictest reading, and safe because cargo's resolve graph does not include
dev-dependencies of non-workspace packages. A future `mc-render` dev-dependency
*on* `mc-testkit` is a legal cargo dev-cycle and creates no edge in this
direction, so the assertion keeps working.

**How the design keeps FR-6.1 true**, beyond the test: `mc-testkit` gains
`image`, `serde`, `serde_json`, `thiserror`, and optionally `wgpu` + `pollster`.
None is an `mc-*` crate. The harness supplies a canvas and consumes images (D6);
it has no reason to name a renderer type, and the moment it does, invariant 5 has
been inverted and the test goes red.

### D12 — Dependency picks

| Pin | Verdict |
|-----|---------|
| `pollster = "0.4"` — new `[workspace.dependencies]` entry | **Confirmed.** `request_adapter`/`request_device` return futures that resolve immediately on native, and blocking on them is the standard wgpu idiom. code-quality §5's "could 20 lines replace it?" fails here: a hand-rolled `block_on` needs a `RawWaker`, and `unsafe_code = "warn"` + `-D warnings` makes that a gate failure. Zero dependencies, MIT/Apache. Gated behind the `gpu` feature |
| `serde_json = "1.0"` — new entry | **Confirmed.** FR-5.1-S1 fixes the report format as JSON, and D11's graph walk parses `cargo metadata`'s JSON. Pairs with `serde` (already pinned, `derive` enabled) for a typed `FrameReport` rather than stringly-built JSON |
| `tempfile = "3"` — new entry, **dev-dependency only** | **I would add a third.** D7 and every FR-4.x test need real temp directories, and testing.md §4 prefers a real temp filesystem to mocks. Hand-rolling unique directories under `std::env::temp_dir()` means hand-rolling cleanup-on-panic too. 3.27 is already in the local cargo registry |
| `image` — **change the existing pin** to `{ version = "0.25.10", default-features = false, features = ["png"] }` | **Recommended, flagged for overrule.** Default features pull jpeg/gif/webp/tiff/bmp codecs this spec never touches — build time, `cargo deny` license surface and audit surface for nothing. No crate uses `image` yet, so the change is free today and expensive once `mc-render` depends on it |
| `bytemuck` | **Not needed and must not be added.** Readback is byte-oriented: the mapped range is `&[u8]` and the output is `Vec<u8>`. Adding it would fail `cargo machete` |
| `wgpu` 30, `serde`, `thiserror` | Already pinned; opt in with `{ workspace = true }` |

### D13 — Types that pure functions consume live in the core, not behind the feature — **BINDING**

Added at task-breakdown time (lead ruling, 2026-08-11); **generalised from a list
into a rule at the phase-3 boundary** (lead ruling, 2026-08-11), after a fourth
type was pulled across the seam case by case. Four occurrences of the same
correction is a class, and fixing the class beats fixing instances — the same
move `spec.md` § Structural constraints made when it replaced fixture-by-fixture
row-order corrections with a standing requirement.

**The rule.** A type belongs on the **core** side of the seam — declared outside
`frame/gpu/**` and **not** feature-gated — when all three hold:

1. a **pure function consumes or produces it** (as input, return type, or a field
   of either), and
2. it **names no `wgpu` type** anywhere in its definition, and
3. it is **not** feature-gated, so it compiles under `--no-default-features`.

The GPU layer may still be its only production *caller*; that is placement of the
call, not of the declaration. Where the two documents appear to disagree — the
module map or § Interfaces listing a type under `gpu/` — **this rule wins**, and
the disagreement is a documentation defect to record, not a design question to
re-open.

Why the rule rather than the list: the "41 GPU-free" figure in FR traceability is
only reachable if every such type sits core-side. A type placed behind the
feature drags its scenarios into phase 3 with it, silently — the scenario still
passes, it simply now needs hardware, and nothing announces that the count moved.

**The four instances found so far**, each an application of the rule and none of
them new design:

| Type | Placement | Which pure function consumes it, and what would break |
|---|---|---|
| `AcquireError` — `NoAdapter { tried: Vec<Backend> }`, `DeviceRejected { adapter, requirement }` | `frame/selection.rs` | `classify_acquisition` takes it as *input* (D2). Behind `gpu` it would drag FR-1.2-S1/S2/S3 into phase 3 with it |
| `CaptureError` — `Size`, `DrawWork(Box<dyn Error + Send + Sync>)`, `ReadbackTimeout { capture, deadline }`, `Readback` | core, beside `clock.rs` | FR-2.3-S2 asserts the timeout names both the capture and the deadline, and must do so under `--no-default-features` |
| The `"unknown"` normalisation of `AdapterProvenance::driver_description` | `frame/report.rs`, as a constructor over `Option<&str>` | FR-5.1-S3 is one of the 41. "Normalised in the adapter" below means the adapter *calls* it, not that it is *implemented* there |
| `SkipNotice` — the announced skip carrying the literal `MYCRAFT_ALLOW_NO_GPU` (found in phase 2; § Interfaces lists it under the GPU layer, and the rule overrides that) | `frame/selection.rs`, public | `classify_acquisition` *returns* it inside `AcquisitionVerdict::Skip`. FR-1.2-S2 asserts its literal text under `--no-default-features` |

None of the four names a `wgpu` type, so all four compile with the feature off.
Together they are what make FR-1.2-S1/S2/S3, FR-2.3-S2 and FR-5.1-S3 reachable
without hardware.

### Trivial decisions (one line each)

- `thiserror` for every error enum; one enum per boundary, no `anyhow` in library code.
- `FailingMask` is a `Vec<bool>` (≈900 KB at 720p); a bitset is a later optimisation with no caller impact.
- PNG encoding pins `CompressionType` and `FilterType` explicitly rather than relying on `image`'s defaults, so FR-4.3-S2's byte-identity does not depend on an upstream default.
- Provenance `driver_description` is normalised to the literal `"unknown"` by a pure core constructor over `Option<&str>` that **the adapter calls**, so the core type is `String` and never `Option` (FR-5.1-S3 requires the field present, never omitted) and the scenario stays testable without hardware — D13.
- `Acquisition::Ready` boxes the context to keep the enum's variants comparable in size (`clippy::large_enum_variant`).
- The skip notice (FR-1.2-S2) is a returned value carrying the literal text `MYCRAFT_ALLOW_NO_GPU`, additionally printed with `eprintln!`; no logging dependency is added for one line.
- Comparison is single-threaded. Its reductions are order-independent (spec, "Determinism"), so `rayon` remains available later without changing any verdict.

## Build order

The phase boundary is part of the design, because phase 1's premise (no GPU in
the process) is only real if something builds and runs that configuration.

**Phase 1 — no GPU in the process.** `image.rs`, `color.rs`, `compare.rs`,
`diff.rs`, `png.rs`, `report.rs`, `layout.rs`, `golden.rs`, `optins.rs`,
`clock.rs`, `selection.rs`, `readback.rs`, and their tests. 41 of 53 scenarios.

Definition of done — all four:
1. `cargo nextest run -p mc-testkit --no-default-features` green
2. `cargo clippy -p mc-testkit --no-default-features --all-targets -- -D warnings` clean
3. `scripts/sdd-gate.ps1` exit 0
4. `grep -rn 'wgpu::\|std::env' crates/mc-testkit/src` returns nothing outside
   `frame/gpu/**` and `frame/optins.rs`

**Phase 2 — the wgpu adapter.** `gpu/mod.rs`, `gpu/acquire.rs`, `gpu/target.rs`,
the self-verification scene in `tests/`, and the 12 hardware scenarios:
FR-1.1-S1/S2/S3/S4, all six of FR-2.1, FR-2.2-S1 end-to-end, FR-2.3-S1.

## Interfaces

Module map — the seam is the `gpu/` directory boundary:

```
crates/mc-testkit/src/
  lib.rs                  pub mod frame;
  frame/
    mod.rs                re-exports + the seam contract in module docs
    image.rs              Rgba8Image, FrameSize, validate_frame_size        FR-2.2-S2/S3
    color.rs              srgb8_to_lab, delta_e            ← CIEDE2000 swap point
    compare.rs            Thresholds, Comparison, Verdict, compare          FR-3.1..3.5
    diff.rs               render_diff                                       FR-4.3
    png.rs                encode_png / write_png / read_png                 FR-2.4, FR-4.4-S5
    report.rs             FrameReport, AdapterProvenance, Backend           FR-5.1
    layout.rs             CaptureId, GoldenPaths, ArtifactPaths             FR-4.1..4.4 paths
    golden.rs             verify_against_golden                             FR-4.1, 4.2, 4.4
    optins.rs             OptIns, from_lookup, from_environment             FR-1.2, FR-4.4
    clock.rs              Clock, SystemClock, poll_until_deadline,
                          CaptureError (core-side — D13)                    FR-2.3
    selection.rs          AdapterDescription, select_preferred,
                          unsatisfied_limit, classify_acquisition,
                          AcquireError (core-side — D13)                    FR-1.1, FR-1.2
    readback.rs           padded_row_bytes, unpad_rows                      FR-2.2-S1
    gpu/                  #[cfg(feature = "gpu")] — the ONLY `wgpu::` in the crate
      mod.rs              CaptureContext, CaptureRequest, DrawWork, draw_fn,
                          AcquireOptions, capture_and_verify
      acquire.rs          instance, enumeration, device request → core DTOs
      target.rs           offscreen texture, copy-to-buffer, map, unpad
```

### Core types (no `wgpu`, no feature gate)

```rust
pub struct Rgba8Image { /* width, height, pixels: Vec<u8>, len == w*h*4 */ }
impl Rgba8Image {
    pub fn from_rgba(width: u32, height: u32, pixels: Vec<u8>) -> Result<Self, ImageShapeError>;
    pub fn width(&self) -> u32;
    pub fn height(&self) -> u32;
    pub fn pixel(&self, x: u32, y: u32) -> Option<[u8; 4]>;
    pub fn as_bytes(&self) -> &[u8];
}
// Contract: 8-bit RGBA, sRGB-encoded, straight (non-premultiplied) alpha,
// row 0 = top. No stage of this crate flips rows or touches alpha.

pub struct CaptureId(String);
impl CaptureId {
    /// One path segment: non-empty, `[a-z0-9_-]` only, never `.` or `..`.
    pub fn new(name: &str) -> Result<Self, CaptureIdError>;
    pub fn as_str(&self) -> &str;
}

pub struct Thresholds { /* private fields */ }
impl Thresholds {
    /// Rejects negative, NaN or infinite values, naming the field and the value.
    pub fn new(per_pixel_delta_e: f64, max_failing_fraction: f64,
               hard_ceiling_delta_e: f64) -> Result<Self, ThresholdError>;   // FR-3.5-S3
    pub fn per_pixel_delta_e(&self) -> f64;
    pub fn max_failing_fraction(&self) -> f64;
    pub fn hard_ceiling_delta_e(&self) -> f64;
}
impl Default for Thresholds { /* 2.0, 0.0001, 10.0 */ }                      // FR-3.5-S1

pub enum MismatchReason {
    AreaBudget,                                                              // FR-3.2-S2
    HardCeiling,                                                             // FR-3.3-S2
    Dimensions { expected: (u32, u32), actual: (u32, u32) },                 // FR-3.4-S2
}
pub enum Verdict { Match, Mismatch(MismatchReason) }

/// Positions that exceeded the per-pixel tolerance. Row-major, `w * h` entries.
pub struct FailingMask { /* width, height, failing: Vec<bool> */ }
impl FailingMask { pub fn is_failing(&self, x: u32, y: u32) -> bool; }

pub struct Comparison {
    pub verdict: Verdict,
    pub failing_pixels: u64,
    pub total_pixels: u64,
    pub failing_fraction: f64,
    pub max_delta_e: f64,
    pub thresholds: Thresholds,
    pub failing_mask: Option<FailingMask>,   // None iff dimensions differ
}

/// Never fails: a dimension difference is a mismatch, not an error
/// (`requirements.md` D3).
pub fn compare(expected: &Rgba8Image, actual: &Rgba8Image,
               thresholds: &Thresholds) -> Comparison;                        // FR-3.1..3.5

/// Failing positions → opaque magenta (255,0,255,255); every other position
/// carries the EXPECTED image's pixel. None when dimensions differ.
pub fn render_diff(expected: &Rgba8Image, comparison: &Comparison) -> Option<Rgba8Image>;  // FR-4.3

pub fn encode_png(image: &Rgba8Image) -> Result<Vec<u8>, ImageIoError>;       // FR-4.3-S2
/// Creates the parent directory if it does not exist; a failure to create it is
/// `ImageIoError::Directory` (FR-2.4-S3's "does not exist and cannot be created").
pub fn write_png(image: &Rgba8Image, path: &Path) -> Result<(), ImageIoError>;
pub fn read_png(path: &Path) -> Result<Rgba8Image, ImageIoError>;             // FR-4.4-S5

pub struct AdapterProvenance {
    pub name: String,
    pub backend: Backend,
    pub driver_description: String,   // "unknown" when the adapter reports none — FR-5.1-S3
}
pub enum Backend { Vulkan, Dx12, Metal, Gl, BrowserWebGpu, Other }
pub enum AdapterKind { Discrete, Integrated, Virtual, Cpu, Other }
pub struct AdapterDescription { pub name: String, pub backend: Backend,
                                pub kind: AdapterKind, pub driver_description: String }

pub enum GoldenOutcome {
    Pass,                                                                    // FR-4.1-S1/S2
    GoldenUnchanged,                                                         // FR-4.4-S4
    GoldenWritten { paths: Vec<PathBuf> },                                   // FR-4.4-S3
    Failed(GoldenFailure),
}
pub enum GoldenFailureReason {
    MissingGolden { path: PathBuf },                                         // FR-4.4-S1
    UndecodableGolden { path: PathBuf, cause: String },                      // FR-4.4-S5
    Mismatch(Comparison),                                                    // FR-4.2
}
pub struct GoldenFailure {
    pub reason: GoldenFailureReason,
    pub artifact_dir: PathBuf,                            // named in Display — FR-4.2-S2
    pub artifacts: Result<Vec<PathBuf>, ArtifactError>,   // Err still fails — FR-4.2-S3
}

/// Fully GPU-free: takes an image and provenance as values.
pub fn verify_against_golden(captured: &Rgba8Image,
                             provenance: &AdapterProvenance,
                             settings: &GoldenSettings) -> GoldenOutcome;
```

### Golden lifecycle behaviour (FR-4.1 / FR-4.2 / FR-4.4)

Decomposed into named helpers to stay inside the 30-line / complexity-15 budget —
this is the file most likely to trip those lints, and the table below *is* the
decomposition: one small function per row, dispatched by a match.

| Golden state | `update_goldens` unset | `update_goldens` set |
|---|---|---|
| Missing | `Failed(MissingGolden)`; artifact dir cleared by allowlist, then `actual.png` **only** written (FR-4.4-S1/S2). No report, no diff — the spec asks for the captured image and nothing more | Create golden + provenance sidecar, `GoldenWritten` |
| Undecodable PNG | `Failed(UndecodableGolden{path, cause})`, file **not** overwritten (FR-4.4-S5) | Same — a corrupt file is never silently replaced |
| Present, matches | `Pass`; artifact dir left with no file, stale files from an earlier mismatch removed by allowlist (FR-4.1-S1/S2/S3) | `GoldenUnchanged`; golden bytes **and** sidecar untouched, no paths reported (FR-4.4-S4) |
| Present, mismatches | `Failed(Mismatch)` + the artifact set: `expected.png`, `actual.png`, `diff.png` (omitted on a dimension mismatch, with the reason recorded in the report — FR-4.3-S3), `report.json` (FR-4.2-S1) | Overwrite golden + sidecar, `GoldenWritten{paths}`; the mismatch artifact set is **not** written (audit contradiction 1) |

### GPU layer (`feature = "gpu"`)

```rust
pub struct AcquireOptions { pub backends: wgpu::Backends, pub required_limits: wgpu::Limits }
impl Default for AcquireOptions { /* Backends::PRIMARY, Limits::downlevel_defaults() */ }

// Amended 2026-08-11 (lead ruling, phase-3 boundary): the field list previously
// read `instance, adapter, device, queue, provenance, limits`. The instance and
// adapter handles are NOT retained — wgpu's `Device` and `Queue` hold their own
// reference to the instance internals, so keeping them would be two fields
// nothing reads, which `dead_code` makes a gate failure. The property this list
// was written to guarantee is unaffected: "a failed capture leaves nothing
// poisoned" is about the per-capture texture and buffer dropping at the end of
// each capture, which is what FR-2.1-S6 asserts and passes.
pub struct CaptureContext { /* device, queue, provenance, limits */ }
pub struct SkipNotice { /* message contains the literal "MYCRAFT_ALLOW_NO_GPU" */ }
pub enum Acquisition { Ready(Box<CaptureContext>), Skipped(SkipNotice) }

impl CaptureContext {
    pub fn acquire(opt_ins: &OptIns, options: &AcquireOptions)
        -> Result<Acquisition, AcquireError>;                                // FR-1.1, FR-1.2
    pub fn provenance(&self) -> &AdapterProvenance;                          // FR-1.1-S2
    pub fn capture(&self, request: &CaptureRequest,
                   draw: &mut dyn DrawWork) -> Result<Capture, CaptureError>;

    // Added 2026-08-11 (lead ruling, phase-3 boundary). Both return *facts the
    // layer already holds*, never decisions, so the seam is unchanged.
    //
    // `device` because `DrawWork::record` receives an encoder and a view, and
    // neither can create a pipeline — without it FR-2.1-S2's real draw is
    // unsatisfiable, since the caller owns the render pass (D6).
    //
    // `limits` because a `FrameSize` is obtainable only from
    // `validate_frame_size(w, h, maximum)` and the maximum is a device fact;
    // without an accessor every caller hard-codes a guess, which is the thing
    // the pure check exists to avoid.
    pub fn device(&self) -> &wgpu::Device;
    pub fn limits(&self) -> AdapterLimits;
}

pub struct CaptureRequest { pub capture: CaptureId, pub size: FrameSize, pub deadline: Duration }
impl CaptureRequest {
    // Added 2026-08-11 (lead ruling, phase-3 boundary): the constructor for the
    // common case. The three fields stay public, so a caller with a reason to
    // bound a readback differently needs no second constructor.
    pub fn new(capture: CaptureId, size: FrameSize) -> Self;  // deadline 30s — FR-2.3
}

pub struct Capture { pub image: Rgba8Image, pub readback: Duration }         // FR-2.3-S1

/// Composition root of the harness: capture, then verify. Lives here, not in the
/// core, so the core never needs to invoke a GPU.
pub fn capture_and_verify(context: &CaptureContext, request: &CaptureRequest,
                          draw: &mut dyn DrawWork,
                          settings: &GoldenSettings) -> Result<GoldenOutcome, CaptureError>;
```

Capture sequence (`target.rs`), in order — validation strictly before any GPU
work, so FR-2.2-S2/S3's "SHALL NOT submit any GPU work" is structural:

1. `validate_frame_size(w, h, limits.max_texture_dimension_2d)?` — pure.
2. Create texture (`Rgba8UnormSrgb`, `RENDER_ATTACHMENT | COPY_SRC`) and a
   `COPY_DST | MAP_READ` buffer of `padded_row_bytes(w) * h`.
3. `draw.record(&mut encoder, &view)?` — the caller's error propagates unchanged
   (FR-2.1-S5), and no submission has happened.
4. `copy_texture_to_buffer` with `bytes_per_row = padded_row_bytes(w)`
   (`wgpu::COPY_BYTES_PER_ROW_ALIGNMENT` = 256, confirmed against wgpu docs),
   then submit.
5. `map_async` + `poll_until_deadline(&SystemClock::started_now(),
   request.deadline, step)` where `step` polls non-blocking and reports
   `Ready`/`Pending` (FR-2.3).
6. `unpad_rows(mapped, w*4, padded_row_bytes(w), h)?` — pure (FR-2.2-S1: 257×129
   gives `row_bytes = 1028`, `padded = 1280`).
7. Unmap; per-capture texture and buffer drop. The context keeps only
   device/queue (plus the provenance and limits it reports), so a failed capture
   leaves nothing poisoned and the next capture succeeds (FR-2.1-S6). Amended
   2026-08-11 alongside the field list above — this sentence restated the same
   stale claim.

### Error contracts

| Error | Variants that carry a scenario | Scenario |
|---|---|---|
| `AcquireError` | `NoAdapter { tried: Vec<Backend> }` · `DeviceRejected { adapter: String, requirement: UnsatisfiedLimit }` | FR-1.1-S3, FR-1.1-S4 |
| `FrameSizeError` | `ZeroDimension { dimension: &'static str }` · `TooLarge { dimension, requested: u32, maximum: u32 }` | FR-2.2-S2, FR-2.2-S3 |
| `CaptureError` | `Size(FrameSizeError)` · `DrawWork(#[source] Box<dyn Error + Send + Sync>)` · `ReadbackTimeout { capture: CaptureId, deadline: Duration }` · `Readback(ReadbackError)` | FR-2.1-S5, FR-2.3-S2 |
| `ImageIoError` | `Directory { path: PathBuf, #[source] cause }` · `Write { path: PathBuf, #[source] cause }` · `Decode { path: PathBuf, #[source] cause }` | FR-2.4-S3, FR-4.4-S5 |
| `ThresholdError` | `Invalid { field: &'static str, value: f64 }` | FR-3.5-S3 |
| `CaptureIdError` | `Empty` · `IllegalCharacter { name: String, character: char }` | — (guard, see Assumptions) |
| `ArtifactError` | `Directory { path: PathBuf, #[source] cause }` · `File { path: PathBuf, #[source] cause }` | FR-4.2-S3 |

Every `Display` names the offending value (path, dimension, backend list,
deadline), per code-quality §4's "specific and actionable".

**Variants added during phase 3 that carry no scenario** (lead ruling,
2026-08-11). Each is a reachable failure the table above left with nowhere to go,
so the alternatives were swallowing it — barred by code-quality §4 — or filling a
field with something untrue. None names a `wgpu` type, so all four sit on the
core side of the seam under D13's rule.

| Error | Variant | Why it exists |
|---|---|---|
| `AcquireError` | `DeviceUnavailable { adapter: String, cause: String }` | An adapter refusing a device for anything other than the one limit the harness models. `UnsatisfiedLimit` can only name a modelled capability, so `DeviceRejected` would have had to synthesise one and print "it offers 16384, and 8192 was required" — confidently wrong |
| `ReadbackError` | `DeviceLost { cause: String }` | `map_async` reporting failure, `poll` failing, or the mapping being dropped unreported. This is the payload `DeadlineExpired::Step` was declared for and never had. Cost `ReadbackError` and `DeadlineExpired` their `Copy` |
| `CaptureError` | `Shape(#[source] ImageShapeError)` | The unpadded buffer not describing the frame requested. Unreachable by arithmetic; present because the alternative is an `expect` on the one path that would reveal the arithmetic is wrong, and `expect` is lint-denied workspace-wide |
| `ArtifactError` | `GoldenNotUpdated { path: PathBuf, #[source] cause: Box<ArtifactError> }` | The update path failing to write the golden. Added by the ruling-5 correction, and listed here for the same reason as the other three. It wraps **only** the image write: past that line the golden is on disk, so a sidecar failure stays `Report` rather than claiming a golden that *was* updated was not |

`GoldenFailure`'s `Display` is written by hand rather than derived, so that a
failed write reaches the reader at all — the derived form printed the standing
verdict and an artifact directory and never rendered `artifacts` when it was
`Err`.

## Data

No database, no migrations, no personal data. Two on-disk artefacts.

**Golden provenance sidecar** — `goldens/<capture-id>/default.provenance.json`,
committed alongside its PNG (FR-5.1-S4, audit gap #19). Rewritten only when its
golden is (FR-4.4-S4).

```json
{ "capture": "clear-red-64", "adapter": "NVIDIA GeForce RTX 4090",
  "backend": "vulkan", "driver_description": "566.36" }
```

**Mismatch report** — `<artifact-root>/<capture-id>/report.json` (FR-5.1-S1/S2).
`failing_pixels` is always a JSON **number**, never null. The provenance block is
adapter name, backend, driver description, the three thresholds, the
failing-pixel count and the maximum per-pixel distance — all six present in
every report.

Pixel mismatch (area budget or hard ceiling), diff present:

```json
{
  "capture": "clear-red-64",
  "verdict": "mismatch",
  "reason": "area_budget",
  "thresholds": { "per_pixel_delta_e": 2.0, "max_failing_fraction": 0.0001,
                  "hard_ceiling_delta_e": 10.0 },
  "failing_pixels": 6,
  "total_pixels": 57600,
  "failing_fraction": 0.000104,
  "max_delta_e": 4.67,
  "adapter": { "name": "NVIDIA GeForce RTX 4090", "backend": "vulkan",
               "driver_description": "unknown" },
  "artifacts": { "expected": "expected.png", "actual": "actual.png",
                 "diff": "diff.png", "diff_omitted_reason": null }
}
```

Dimension mismatch — no pixels were compared, so the three pixel statistics are
`0` (FR-3.4-S2 says the reason *is* the dimension difference), and the diff is
omitted with its reason recorded (FR-4.3-S3):

```json
{
  "capture": "half-fill-64",
  "verdict": "mismatch",
  "reason": "dimensions",
  "expected_size": [64, 64],
  "actual_size": [65, 64],
  "thresholds": { "per_pixel_delta_e": 2.0, "max_failing_fraction": 0.0001,
                  "hard_ceiling_delta_e": 10.0 },
  "failing_pixels": 0,
  "total_pixels": 0,
  "failing_fraction": 0.0,
  "max_delta_e": 0.0,
  "adapter": { "name": "NVIDIA GeForce RTX 4090", "backend": "vulkan",
               "driver_description": "unknown" },
  "artifacts": { "expected": "expected.png", "actual": "actual.png",
                 "diff": null, "diff_omitted_reason": "dimension mismatch" }
}
```

Retention: goldens and their sidecars are permanent, versioned in git (spec
Assumptions: small enough without LFS). Artifacts are transient under `target/`,
git-ignored, and cleared by the next non-mismatch run of the same capture.

This spec commits exactly one golden: a synthetic, CPU-generated fixture whose
sidecar records it as such rather than naming a real adapter. Its purpose is the
git round trip — a golden read from its real committed path, compared, and
judged — which no temp-root test can exercise and which would otherwise run for
the first time against a real frame in PRO-852. Every other golden-lifecycle
test uses a temporary golden root, and the harness still proves its *capture*
path against computed ground truth, never against a committed image (D5).

## Integration

| File | What changes | What must not break |
|---|---|---|
| `Cargo.toml` (root) | Add `pollster`, `serde_json`, `tempfile` to `[workspace.dependencies]`; narrow the `image` pin to `default-features = false, features = ["png"]` | No member crate gains a version literal. Existing pins keep their versions |
| `crates/mc-testkit/Cargo.toml` | `image`, `serde`, `serde_json`, `thiserror` as `{ workspace = true }`; `wgpu`, `pollster` `optional = true`; `tempfile` under `[dev-dependencies]`; `[features] default = ["gpu"]` | Must **not** gain `mc-render`, `mc-client` or `mc-server` in any section (FR-6.1). `cargo machete` must stay clean — do not add `bytemuck` |
| `crates/mc-testkit/src/lib.rs` | Currently empty; gains `pub mod frame;` | — |
| `crates/mc-testkit/goldens/` | New committed directory holding exactly **one** golden: a deterministic CPU-generated fixture and its provenance sidecar (lead ruling, 2026-08-11). It exercises the read-a-committed-golden path here rather than first in PRO-852, where a failure would be ambiguous between a wrong renderer and a wrong golden workflow | The fixture must **not** be GPU-generated — that would bake this machine's adapter into the repo and pre-empt the per-adapter-golden deferral. Its sidecar records it as synthetic |
| `.gitattributes` | **New file**: `*.png binary` (plus `*.provenance.json text eol=lf`). The repo has none today, so a byte-sensitive golden currently relies entirely on git's content auto-detection on Windows | — |
| `.gitignore` | Verify `target/` is ignored so artifacts are never committed | The gate's gitleaks allowlist `git check-ignore` stage must stay untouched |
| `scripts/sdd-gate.ps1` | **Recommended, not done here** (D1): the two `--no-default-features` lines in the lint stage. Possibly also extend `$CoverageExclude` — see Risks | Shared tooling; the lead's call |

Nothing else in the workspace is touched. No `mc-render` code exists yet, which
is the point (invariant 5).

## FR traceability

| FR | Component | GPU needed? |
|---|---|---|
| FR-1.1 | `gpu/acquire.rs` + `selection.rs` | S1, S2 yes · **S3, S4 yes** (real wgpu triggers, D2) · S5 **no** |
| FR-1.2 | `optins.rs` + `selection.rs::classify_acquisition` | **No** — env cannot be set in tests, so all three are pure (D2/D3) |
| FR-2.1 | `gpu/target.rs` + caller draw work (D6) | Yes (all six) |
| FR-2.2 | `image.rs::validate_frame_size`, `readback.rs::unpad_rows` | S1 **no** as a unit + yes end-to-end · S2, S3 **no** |
| FR-2.3 | `clock.rs::poll_until_deadline` + `gpu/target.rs` | S1 yes · S2 **no** (fake clock) |
| FR-2.4 | `png.rs` | **No** |
| FR-3.1 … FR-3.5 | `color.rs`, `compare.rs` | **No** |
| FR-4.1, FR-4.2, FR-4.4 | `golden.rs`, `layout.rs` | **No** |
| FR-4.3 | `diff.rs` | **No** |
| FR-5.1 | `report.rs` | **No** — S4's provenance is a value type, constructible in a test |
| FR-6.1 | `tests/dependency_graph.rs` | **No** |

**GPU-free: 41 of 53 scenarios.** The phase-1 floor asked for 17; the design
reaches 41 by making every *decision* pure (D2) and by having the golden pipeline
consume images and provenance as values rather than a capture backend (D1). The
12 that need an adapter are FR-1.1-S1/S2/S3/S4, all six of FR-2.1, FR-2.2-S1
end-to-end, and FR-2.3-S1 — every one an observation of real hardware.

## Assumptions

Each is a declared substitute for a driver the spec does not fix. A reviewer can
veto any of them.

1. **Alpha is outside the comparison metric** (D9). ΔE is RGB-only, so two pixels
   differing only in alpha compare equal. Every spec scenario uses opaque pixels,
   and FR-2.1-S3 asserts alpha behaviour on the capture side instead. Revisit
   trigger: the first golden whose frame contains non-opaque pixels.
2. **`CaptureId` validation** (`[a-z0-9_-]`, one path segment) has no scenario. It
   is input validation on a path-forming public input (code-quality §7). If this
   counts as unspecced scope, delete the newtype and pass `&str`.
3. **Narrowing the `image` pin** to `default-features = false, features = ["png"]`
   changes a shared workspace file for a crate no one else uses yet.
4. **`tempfile` as a third new dependency**, dev-only.
5. **Presence, not value, of the two environment variables** is what "set" means —
   `MYCRAFT_ALLOW_NO_GPU=0` still enables the skip. The spec says "set"/"unset"
   and never mentions a value. Asserted through `OptIns::from_lookup` (D3).
6. **FR-3.4-S1's symmetry** is asserted over verdict, failing-pixel count and
   maximum distance, as the scenario states. `MismatchReason::Dimensions` labels
   its two sizes by argument role, so that payload does swap under a swapped
   call — irrelevant to S1, whose images are the same size.
7. **The `default` golden filename is a constant.** No variant *resolution* is
   implemented; only the path shape leaves room (D8), per "one golden per
   capture" in Out of Scope.
8. **A missing golden writes `actual.png` and nothing else** (FR-4.4-S2 asks for
   the captured image; the full artifact set belongs to FR-4.2's mismatch case),
   after clearing any stale artifacts (D7).
9. **On a dimension mismatch the report's pixel statistics are `0`**, not null —
   FR-5.1-S1 requires `failing_pixels` to parse as a number, and no pixels were
   compared.
10. **`cargo metadata` is available and fast in the test environment** (D11). It
    resolves rather than builds, and tests run after cargo has released its build
    lock.

## Risks

| Risk | Blast radius | Verify early |
|---|---|---|
| **The `--no-default-features` configuration is never run**, so D1's seam and FR-3.4-S3 are both theoretical | The whole GPU-free premise | Phase-1 DoD items 1 and 2 cover this spec. Beyond it, only the gate line does — decide it before phase 1 starts |
| **`image`'s PNG encoder is not byte-deterministic** across identical inputs, breaking FR-4.3-S2 | One scenario | Phase 1, first task: encode the same `Rgba8Image` twice and compare bytes, with explicit `CompressionType`/`FilterType`. If it fails, escalate — the fallback (asserting decoded pixels) weakens the scenario and needs a ruling, not a quiet change |
| **`cargo llvm-cov` instruments `tests/**` by default**, moving a large body of ~100%-covered lines into the denominator and diluting the 80% bar for the one crate ADR-008 leans on | The gate's coverage threshold, workspace-wide | One-line check in phase 1: inspect the JSON summary's file list. If confirmed, extend `$CoverageExclude` with `\|crates[/\\][^/\\]+[/\\]tests[/\\]` — a gate change to raise, not to make silently |
| **`mc-testkit` is inside the coverage denominator and the wgpu module is uncoverable without a GPU** | The 80% line threshold | Keep the wgpu module thin by construction (D2); the two real-failure tests from D2 cover its error branches. With no adapter the gate goes *red* rather than quietly under-covering, which is the intended behaviour (`requirements.md` D2) |
| **The golden lifecycle trips `cognitive_complexity = 15` / `too_many_lines = 30`** — eight state combinations in one function | One file, but the branchiest code in the spec | The state table is the decomposition: one small function per row, dispatched by a match |
| **wgpu 30's exact poll/map API differs from the sequence above** | `frame/gpu/target.rs` only | The sequence is behavioural (submit → map → poll non-blocking under a deadline → unpad → unmap); exact call names are the implementer's to confirm against wgpu 30's docs. `COPY_BYTES_PER_ROW_ALIGNMENT = 256` is confirmed |
| **Hardware sRGB rounding puts FR-2.1-S4 outside ±1** on some adapter | One scenario | A real hardware observation; if it fires, that is information about cross-adapter drift feeding the per-adapter-golden trigger, not grounds for widening a tolerance |
| **A 720p comparison converts ~1.8M pixels to Lab** (three `cbrt` each) | Test duration only | The sRGB→linear LUT removes the expensive half; if it ever matters, the reductions are order-independent and `rayon` is already pinned. Not a phase-1 concern |
| **wgpu 31 lands mid-MVP** | `frame/gpu/**` **plus every consumer's draw-work closure** (D6) | Deliberately accepted, not claimed away — see the Boundaries table. `pub use wgpu` keeps consumers on one version |
