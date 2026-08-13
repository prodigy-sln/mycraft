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

- `mc-sim` owns the world, the tick counter and the player whose state the
  camera is derived from, and publishes `Arc<SimSnapshot>` — tick, camera
  pose and player state — through
  `arc_swap::ArcSwap`. `Simulation::advance(&mut self, intent)` advances the
  player and stores a pointer; `Simulation::latest(&self)` loads one. A
  publish therefore never waits on a
  renderer holding the previous snapshot, which is the whole reason
  `ArcSwap` is here rather than a plain value: `store` swaps a pointer
  instead of waiting on readers, where an `RwLock<Arc<…>>` would satisfy
  the same property only by the discipline of cloning and dropping the
  guard immediately — a discipline nothing enforces.
- **`advance` takes `&mut self` because of where the player's state lives,
  not because `ArcSwap` is missing anything.** A tick assigns `self.player`,
  a plain `PlayerState` field sitting *beside* the `ArcSwap` rather than
  inside it, and an `&self` method cannot assign to it at all. The signature
  is a consequence of that placement, not a choice between two workable
  ones. An earlier version of this paragraph justified it by claiming
  `ArcSwap` offers no compare-and-swap; **that was false** — the pinned
  1.9.2 has both `compare_and_swap` and `rcu`, each taking `&self`. What
  actually rules `rcu` out is its retry semantics, which its own docs are
  explicit about: it re-runs the closure until the swap lands uncontended,
  and a tick's effect is not confined to the cell being swapped, so a
  retried closure would step the player more than once for one tick number.
  **Nor is the exclusive borrow currently guarding a reachable race.**
  `Simulation` holds a `Box<dyn Solidity + Send>`, which leaves the struct
  `Send` but *not* `Sync`, so no two threads can hold `&Simulation` to race
  through in the first place. Read the `&mut` as recording that a tick
  mutates simulation state outside the published cell — not as a defence
  against a lost update. `latest` stays `&self`, so no
  reader is affected, and `SimSnapshot` stays a `Copy` value with no
  interior mutability. The cost is that "a publish never waits on a reader"
  is now a property of publication that nothing can *observe*: no reader can
  hold `&Simulation` while an advance is in flight. Restoring an observable
  form of it is a cloneable `Arc<ArcSwap<SimSnapshot>>` handle, deliberately
  not built while the only reader is the owner, and a two-way door precisely
  because `latest` and `SimSnapshot` did not change shape.
- **One tick per rendered frame, once the world is ready.** The spawn is
  derived from the world, and the world is generated on the preparation
  worker several frames after the window opens, so there is no simulation to
  advance until it lands: `App` holds `Option<Simulation>` and the frame path
  advances a tick only after the scene has reached `ScenePhase::Ready`.
  Before that the frame is the clear colour and no tick passes — a player
  advanced during preparation would spend the load falling. Pending input is
  drained where the tick is advanced and nowhere else, so a frame that draws
  nothing leaves the accumulated motion where it is rather than discarding
  it.
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

## The player: intent, physics and collision

Player state — `crates/mc-sim/src/player/`, placed by `crates/mc-sim/src/replay/` — is what the
previous section's `SimSnapshot.player` field carries, and what `eye_pose` derives the published
camera from.

**The client submits an intent, never a position.** `MovementIntent` carries exactly `forward,
strafe, yaw_delta, pitch_delta, jump` — five fields, and `Default` is "asks for nothing" — with no
field that could carry a position, a velocity or an absolute orientation, so a client cannot state
where it is even by mistake. `InputState` (held keys, pending look delta, `take_intent()`) lives
beside it in `mc-sim` and is client-side behaviour that happens to live in the authority's crate: in
MVP 3 the client still accumulates key state and pointer motion and sends the resulting
`MovementIntent` over the wire, and the server still clamps it on receipt. The clamp lives inside
`advance_player`, on the **receiving** side: magnitude capped at 1, non-finite rejected, per-axis
displacement capped at 1.0 block — the same answer for a well-behaved client and one sending
`1000.0` or a `NaN`. That is invariant 4 made structural rather than promised.

**`advance_player(state, intent, world) -> state` is a pure function**, called by
`Simulation::advance(&mut self, intent)` once per tick. It takes a `PlayerState` by value and
returns a new one, so it cannot mutate the simulation's own state; only `Simulation` owns the state
a snapshot is published from. Its steps run in a fixed order, and every derived quantity below —
the jump apex, the stopping distance of a walk, which axis a diagonal collision resolves against —
is a consequence of that order: changing it changes the numbers.

1. Sanitise: a non-finite `forward` or `strafe` zeroes both; a non-finite `yaw_delta` or
   `pitch_delta` leaves both look accumulators untouched.
2. Look: accumulate, wrap yaw into `[0, 2π)`, clamp pitch to ±89°.
3. Horizontal velocity is **set**, never accumulated, from the intent: magnitude
   `min(1, ‖(forward, strafe)‖) × 4.5 blocks/s` in the basis `forward = (cos yaw, 0, sin yaw)`,
   `right = (−sin yaw, 0, cos yaw)` — horizontal whatever the pitch is.
4. Jump: requested and `on_ground` sets vertical velocity to 9.0 blocks/s.
5. Gravity: vertical velocity becomes `max(v − 0.5, −48.0)` blocks/s. Integration is
   **semi-implicit Euler** — gravity is taken from the velocity before the velocity is applied to
   the position — which is why the jump apex is the discrete 1.275 blocks and not the continuous
   `v²/2g` = 1.35.
6. Displacement = velocity × 1/60 s, each axis clamped to ±1.0 block. The clamp is on the
   displacement, not the velocity — the velocity is what a scenario reports, and clamping it would
   misreport it.
7. Resolve **x, then z, then y**, each axis applied and resolved before the next begins.
8. Ground contact is a query, not a memory: the resolved box lowered by 1 × 10⁻⁴ blocks and tested
   for overlap.
9. A tick ending in ground contact zeroes a negative vertical velocity; a vertical axis stopped by a
   collision zeroes it in either direction.

**Collision is per-axis and exact.** A solid voxel occupies `[v, v+1)` on every axis; overlap is
strict on both sides, so a resolved face sits exactly on the blocking face and is not re-detected the
next tick, and no skin distance is needed. Resolving x, then z, then y — rather than all three
together — is what gives a walk pressed into an inside corner a different answer from one merely
brushing past a diagonal neighbour, and resolving the vertical axis last is what makes ground contact
describe the *end* of the tick, the value the next tick's jump reads. Per-axis resolution is exact
only while a tick displaces the box less than one block on an axis, since only then can the box newly
overlap the adjacent voxel layer; the largest per-tick displacement under the declared constants is
0.8 blocks (terminal fall), so the property holds by derivation, and step 6's runtime clamp is what
keeps it holding if a constant changes, rather than letting tunnelling appear silently. The player's
box is a single AABB, ±0.3 blocks in x and z from the feet centre, 1.8 blocks tall, no rotation and
no sub-boxes.

**Declared constants** (`crates/mc-sim/src/player/{physics,collide,look,input,mod}.rs`):

| Constant | Value | Derives |
|---|---|---|
| Tick duration | 1/60 s | declared, never measured — no wall clock in `mc-sim` |
| Walk speed | 4.5 blocks/s | 0.075 blocks/tick |
| Gravity | 30.0 blocks/s² | 0.5 blocks/s per tick |
| Jump speed | 9.0 blocks/s | apex 1.275 blocks at tick 17 under the semi-implicit integrator; floor again at tick 35 |
| Terminal fall speed | 48.0 blocks/s | 0.8 blocks/tick — under one block, which is what keeps per-axis resolution exact |
| Player box | 0.6 × 1.8 × 0.6 blocks | ±0.3 in x/z from the feet centre |
| Eye height | 1.62 blocks above the feet | — |
| Pitch limit | ±89° | keeps the look direction off `Vec3::Y`, so `look_at` never degenerates |
| Look sensitivity | 0.0022 rad per raw pointer count | applied client-side in `InputState::look`; the intent carries radians |

**Physics reads the world through `Solidity`, resolved once.**
`mc_sim::player::Solidity::is_solid(BlockPos) -> bool` is total — outside the loaded world, below
`y = 0` and every negative coordinate all answer `false` — so a caller has nothing to handle and
nothing to swallow. `SolidVoxels::resolve(volume: &dyn BlockVolume, registry: &BlockRegistry) ->
Result<Self, RegistryError>` builds a one-bit-per-voxel grid once, over the volume's declared
extent, resolving every name it finds through the registry. It takes a `BlockVolume` rather than
`&ReplayWorld` concretely, because the only way to state "an invented block whose definition
disagrees with its name" is a volume the replay world is merely one implementor of — `ReplayWorld`
implements it, and a test fixture can too. This keeps solidity a bounds test and a bit test with no
failure mode to swallow, and keeps the physics untied to the replay fixture a real chunk store
replaces.

**The simulation cannot exist before the world does.**
`mc_sim::replay::simulation_for(world: &ReplayWorld, registry: &BlockRegistry) ->
Result<Simulation, SpawnError>` resolves `SolidVoxels`, derives the spawn and constructs the
`Simulation`; the client calls it and decides nothing. `PreparedScene` carries `world` and
`registry` alongside the mesh, so the composition root — and the golden, probe and determinism
suites — build the simulation from the same preparation the product runs, and
`PreparationError::Spawn` is the variant that carries a refused `SpawnError` across the crate
boundary. Because the world is generated on the preparation worker several frames after the window
opens, `App` holds `Option<Simulation>` and advances a tick only once the scene has reached
`ScenePhase::Ready` — one tick per rendered frame, once the world is ready, not before.

**The spawn is derived, not declared.** The player starts three blocks above column (32, 32)'s own
surface height, facing 225° toward the scene's landmark pillar. The shipped generator answers
`surface_height(32, 32) = 37`, so the feet start at y = 40 and the eye — `EYE_HEIGHT` above the feet
— stands at y = 41.62 at tick 0. (A design-time re-implementation of the heightmap, done independently
while drafting this feature's architecture, answered 40 and placed the eye at 44.62; that was a
second implementation of the generator, not a reading of it, and the figures above are the measured
ones. What the player's camera actually shows from that eye is recorded in
`docs/technical/rendering.md` §"What golden-frame verification cannot see".)

**Cursor capture and focus are pure policy, all in `mc_render::window`, all `winit`-free.** Five
functions, not four: `first_capture_attempt()` is the head of the ladder `next_capture_attempt(refused)`
walks down one rung at a time, and it exists because `next_capture_attempt` can only answer about a
*refusal* — asking what is attempted first has nothing else to call. `capture_after_escape(state)`
always releases; `capture_after_click(state)` re-enters the ladder at `first_capture_attempt` from an
uncaptured state and leaves a captured one alone; `accepts_pointer_motion(state)` is false only when
uncaptured. `WindowEventKind::FocusLost` maps to `LoopAction::ClearInput` in `window_event_action`, so
losing focus is translated the same way any other event is, never decided in the adapter. The adapter
(`crates/mc-client/src/events.rs`) holds the one thing that cannot be pure — the five-row
`winit::keyboard::KeyCode` binding table — behind two public functions, `bound_action(key) ->
Option<PlayerAction>` and `kind_of(event) -> WindowEventKind`, so the table and the focus chain can be
asserted against the key codes and events the operating system actually delivers. Every key event
reaches `InputState::apply(action: Option<PlayerAction>, pressed: bool)` in `mc-sim`, so the one
decision in the key path — which action a code maps to — is inside the crate the coverage denominator
counts, and the adapter is left translating rather than deciding.

**The camera the player's state implies is a pure derivation**,
`mc_sim::player::eye_pose(state) -> CameraPose`: the eye stands over the feet at `EYE_HEIGHT`, and
the target is the eye plus the unit look direction `(cos pitch × cos yaw, sin pitch, cos pitch ×
sin yaw)`. `mc_render::camera::waiting_view()` is what the frame reads before a `Simulation` exists —
the world origin, looking along +x — and it is a **declared** pose rather than an arbitrary one:
`ScenePhase::Preparing` never draws anything through it, but `frame_stats` builds a view-projection
matrix from every snapshot's camera regardless of phase, and a degenerate pose whose eye and target
coincide would put a NaN through that matrix rather than a harmless number.

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
  and not because Cargo refuses the consumer's version. Cargo 1.97.1
  **silently ignores** it: a member writing `default-features = false`
  beside `workspace = true` gets a warning (`` `default-features` is
  ignored for X … this could become a hard error in the future ``) and
  builds with defaults on regardless, whether or not the workspace entry
  states `default-features` itself. A refusal would be a build error
  nobody could miss; what actually happens leaves the manifest reading as
  though the feature were off while the seam check fails.
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
