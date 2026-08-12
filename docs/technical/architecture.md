# Architecture: Crate Boundaries and Dependency Direction

Crate topology as established by the block registry and chunk storage work,
and by the terrain renderer that first put a simulation, a renderer and a
windowed binary on the same graph. This records only what those built — it
is not a survey of all ten crates in the workspace (see `CLAUDE.md`'s crate
map for the full list and the general inward-dependency rule).

## Where the registry contract lives, and why

The block registry **contract** — `BlockName`, `TextureKey`, `BlockDefinition`,
`BlockId`, `BlockRegistry`, and the `DefinitionSource` port — lives in
`mc-core` (`mc_core::block`, `mc_core::id`). `mc-core` performs no I/O and
depends on nothing else in the workspace.

Chunk and column storage, and the file-backed content-root loader, live in
`mc-world` (`mc_world::section`, `mc_world::column`, `mc_world::content`).
`mc-world → mc-core` is the only edge between the two; `mc-world` may
perform I/O, `mc-core` may not.

The driver for this placement is MVP 2, not MVP 1: the Luau scripting host
(`mc-script`) must populate the *same* registry that chunk storage reads
from, and `mc-script` depends inward on `mc-core` like everything else. A
registry owned by `mc-world` would force `mc-script → mc-world`, dragging
chunk storage, worldgen, and eventually `redb`-backed persistence into the
scripting host's dependency graph for no reason connected to scripting.
Putting the contract in `mc-core` — whose whole purpose is primitives other
crates share — avoids that inversion without adding an eleventh crate to
the workspace's fixed ten-crate map.

## The registry/loader seam

`BlockRegistry::apply` is the **only** way to populate a registry:

```rust
impl BlockRegistry {
    pub fn apply(&mut self, source: &dyn DefinitionSource) -> Result<(), RegistryError>;
}
```

There is no public `register` method, and no other way to insert a
definition. This is MyCraft invariant 1 ("the base game holds no privilege
a mod lacks") made **structural** rather than caught by a test someone has
to remember to run: seeding a registry with a block definition from Rust
source does not compile.

The port itself:

```rust
pub trait DefinitionSource {
    fn origin(&self) -> DefinitionOrigin;
    fn definitions(&self) -> DefinitionStream<'_>;
}
pub type DefinitionStream<'a> =
    Box<dyn Iterator<Item = Result<BlockDefinition, DefinitionSourceError>> + 'a>;
```

Two implementations exist:

- `mc_world::content::TomlFileDefinitionSource` — reads a content root's
  `blocks/*.toml` files. The production loader for MVP 1.
- `mc_core::block::source::InMemoryDefinitionSource` — definitions held
  directly, in the order given. This is **production code**, not a test
  fixture: because no public `register` exists, it is the only
  programmatic way to build a registry at all, and its existence is what
  makes the port a real seam rather than an asserted one.

**MVP 2 swaps the loader implementation for a Luau-backed
`DefinitionSource`; the registry itself is untouched.** The port is shaped
around the domain need ("hand me the definitions this source declares, and
tell me where each one came from"), not around TOML's API: no `Path`,
`File`, `PathBuf`, or `toml::` type appears anywhere in `mc-core`. An
origin (`DefinitionOrigin`) is an opaque, human-readable label — it wraps a
plain `String` — so a Luau chunk name is exactly as expressible as a file
path, and `mc-core` never learns what kind of thing produced either one.

`apply` is **atomic by construction, not by discipline**. It runs a
fallible validation pass that drains the source's stream into a staging
`Vec<BlockDefinition>` — checking every incoming name against both the
registry's existing contents and the batch validated so far — followed by
a private commit step whose signature returns `()`:

```rust
fn commit(&mut self, validated: Vec<BlockDefinition>);   // infallible
```

Because `commit` cannot fail or return early, a partial application (some
definitions registered, others not, from one `apply` call) is not an
outcome the code can produce. Either the whole batch lands, or none of it
does.

## The simulation/renderer seam

`mc-sim` publishes; `mc-render` consumes; **`mc-client` is the composition
root and the only crate that resolves both.**

- `mc-sim` owns the world, the tick counter and the camera path, and
  publishes `Arc<SimSnapshot>` — tick plus camera pose — through
  `arc_swap::ArcSwap`. `Simulation::advance` stores a pointer;
  `Simulation::latest` loads one. A publish therefore never waits on a
  renderer holding the previous snapshot, which is the whole reason
  `ArcSwap` is here rather than a plain value: `store` swaps a pointer
  instead of waiting on readers, where an `RwLock<Arc<…>>` would satisfy
  the same property only by the discipline of cloning and dropping the
  guard immediately — a discipline nothing enforces.
- `mc-render` defines the type it consumes, `TerrainSnapshot { tick,
  camera, scene }`, and **never names `mc-sim` in any dependency of any
  kind**. It depends on `mc-world` for the mesher's `Quad` and on
  `mc-core`.
- `mc-client` builds a `TerrainSnapshot` per tick from the published
  `SimSnapshot` plus an `Arc::clone` of the scene. Each crate keeps its own
  camera-pose type — `mc_sim`'s `CameraPose`, `mc_render`'s `CameraView` —
  and the client owns no conversion type of its own: it passes plain
  arrays to `mc_render::camera::camera_view`, so even the construction of
  a view is a counted pure function rather than client code.

**Immutability across the seam is an obligation on the type, not a
compiler error.** A consumer holding `Arc<SimSnapshot>` cannot observe a
later publish only for as long as `SimSnapshot` and everything reachable
from it stay free of interior mutability and expose no `&mut` accessor —
`Arc::get_mut` exists, and a field carrying a `Mutex` or an `AtomicU32`
would silently reopen the hole. That constraint is binding on the type and
is what a test asserts; it is not enforced by the borrow checker the way
the mesher's purity is.

**The renderer stores no last-rendered tick at all.** Frame statistics are
produced by a free function, specifically so there is nowhere to put one.
"Renders whatever tick it is handed, never refuses a stale one" is then
structural rather than promised — with no stored tick there is no
comparison that could refuse.

**Where geometry is built, and why the current shape is not a
destination.** The quad → vertex → packing conversion is `mc-render`'s
pure layer, so `mc-client` calls it: `mc-sim` meshes and hands back quads,
and a `mc-sim → mc-render` edge would break the dependency direction.
**This is a third shape, not where the project is going.** Once chunk data
arrives over the wire, the *client* owns a chunk store and a mesher and
`mc-sim` meshes nothing. What survives from today's arrangement is
`SimSnapshot` and the direction of the arrows — nothing about `mc-sim`
meshing is endorsed by this document, and an earlier claim that it was
already the networked shape was withdrawn as false.

A reader of "the simulation publishes, the renderer consumes" will look
for a geometry field in `SimSnapshot` and not find one. That is the cost
of the split, stated here rather than left to be rediscovered: only tick
and camera travel through the snapshot; geometry reaches the renderer via
the client.

## The pure/GPU seam inside `mc-render`

`mc-render` has a default-on `gpu` Cargo feature. **`wgpu::` may be named
only under `crates/mc-render/src/gpu/` and inside `[[test]]` targets
carrying `required-features = ["gpu"]`.** Under `--no-default-features`
wgpu is absent from the *resolved* graph, not merely unused, so a stray
`use wgpu::` in the pure layer does not compile. `mc-testkit` carries the
identical seam, and the quality gate runs clippy and the test suite in
that configuration for both crates.

**It is a Cargo feature and not a port, deliberately.** A `RendererPort`
trait fails the boundary-isolation test in reverse: the only conceivable
second implementation of a renderer port is another renderer, so the port
would carry every rendering decision to the wrong side of an interface
that will never have a second implementor — and `crates/mc-render/CLAUDE.md`
forbids exactly that growth. The feature seam buys the same testability
with a violation that is a build error rather than a review finding.

**Every client decision is a pure function in `mc-render`, and that is
what keeps `mc-client`'s coverage exclusion honest** (ADR-013). Surface
format selection, resize and depth-reallocation policy, surface-error →
frame-action policy, window-event → action policy and the device-request
description all live in the pure layer, so `mc-client` is left holding the
`winit` event-loop adapter, composition wiring, and the per-frame
mechanics that have no pure form — acquire the surface texture, create an
encoder, submit, present — and nothing that *decides* anything. The
GPU-touching layer keeps only the mechanical part: allocate, upload,
encode, submit, present.

**`mc-client` is a library plus a binary, not a binary alone.**
`crates/mc-client/src/lib.rs` publishes the startup path — generate → mesh
→ resolve texture layers → build geometry → assemble — as a single public
statement, imported by the binary *and* by the golden, probe and replay
suites. The goldens are therefore shot through the pipeline the product
runs, rather than through a second assembly written for tests that could
drift from it. Because `mc-render` and `mc-sim` may not resolve each
other, every test needing both lives in `crates/mc-client/tests/`; the
committed goldens still live under `crates/mc-render/goldens/`.

## Mechanically enforced invariants

Several facts about these designs are asserted by tests that walk real
structure, not by convention:

- **`toml` is absent from `mc-core`'s entire resolved dependency graph.**
  A test walks `cargo metadata`'s resolved graph breadth-first from the
  `mc-core` node and fails if `toml` appears anywhere in it — including
  transitively. The same test also asserts the **positive control** that
  `mc-world`'s resolved graph *does* reach `toml`: without that second
  assertion, the check would still pass, vacuously, the day someone
  deleted the loader and hardcoded block definitions into `mc-core`
  directly — exactly the regression this structure exists to prevent.
- **No Rust source outside test code contains a `base:`-namespaced block
  name literal.** A source scan reads every `crates/*/src/**/*.rs` file
  except the sibling `*_test.rs` unit files, and fails if any of the five
  shipped block names appears in a file's *production text* — the file
  minus its doc comments, since a doc example is a doc test that does
  live in a production file (`technical/testing.md`). This scan also
  carries a positive control: pointed at a fixture directory containing
  one of the five names, it must report a hit, or a broken path glob or
  matcher could pass by never actually looking at anything. It carries
  two further controls for the filters themselves — a name in a sibling
  `*_test.rs` is skipped while one in the module beside it is still
  found, and a name in a doc example is not reported — because a filter
  that skipped too much would leave the first control green while
  scanning nothing.
- **`mc-render` and `mc-sim` do not resolve each other, in any dependency
  kind.** The walk follows dev-dependency edges too, because those are
  edges in the resolved graph like any other. This is why `mc-render`'s
  dev-dependency on `mc-testkit` is declared `default-features = false`:
  a default-featured one would drag wgpu into `mc-render`'s
  `--no-default-features` graph and fail the seam check in a way that
  reads as a feature bug in `mc-render` itself. The `default-features =
  false` belongs on the workspace dependency entry, not the consumer —
  Cargo refuses a member overriding a workspace default.
- **`winit` is nameable in `crates/mc-client/src/events.rs` alone**,
  pinned by a source scan whose filter was mutation-checked rather than
  assumed.

**The instrument matters, and the two questions are not the same
question.** Feature questions go to `cargo tree --package`, which resolves
features for the one package named and honours `--no-default-features`.
Reachability questions go to `cargo metadata`'s resolved graph.
`cargo metadata`'s `resolve` is **workspace-unified whatever manifest it
is pointed at**, so a `--no-default-features` walk of it re-enables `gpu`
through `mc-client` and answers the opposite of the question asked —
unfalsifiable in exactly the direction the check guards. Unification
changes which features are on; it cannot invent a non-optional path edge
between two workspace members, which is why reachability is safe on it.
Measured: `cargo tree -p mc-render --no-default-features -e all` reaches
`wgpu` zero times, and with defaults, 93.

## Internal-crate version pinning

`deny.toml`'s `wildcards = "deny"` rejects any `[workspace.dependencies]`
entry that carries a path but no version — and `mc-world → mc-core` was
the workspace's first internal crate edge to exist, which is what
surfaced this. The fix generalizes past the one edge that triggered it:
**all eight internal crate entries in the root `Cargo.toml` carry
`version = "0.1.0"` alongside their path**, kept in step with
`[workspace.package] version`. Pinning only the one edge that happened to
exist would have left the wildcard check silently disabled for every
crate the workspace adds afterward; pinning all eight applies
`CLAUDE.md`'s central-pinning rule to internal crates rather than treating
them as an exception to it.
