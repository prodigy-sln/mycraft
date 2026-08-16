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
  advance until it lands. `App::present` keeps a `session.tick()` call at the
  same statement position it always advanced a tick from; what moved is the
  call's *body* — the `Option<Simulation>` guard, the drain and the advance
  now live inside `Session::tick` (`crates/mc-client/src/session.rs`, §"The
  client input dispatch" below), not in the frame path, so the guard is
  reachable by a test that opens no window. Before the world lands the frame
  is the clear colour and no tick passes — a player advanced during
  preparation would spend the load falling. Pending input is drained inside
  that same guard and nowhere else, so a frame that draws nothing leaves the
  accumulated motion where it is rather than discarding it.
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

`BlockVolume::block_at` returns `Option<Contents<&BlockName>>`, and the two negative answers stay
separate all the way down: the `Option` says the volume reaches this position, and the `Contents`
says whether anything is in it (`technical/world-format.md`). `SolidVoxels::resolve` then answers
`false` for both — a position the volume does not reach and a cell holding nothing are both
not-solid — and this is the one place in the codebase where the two readings genuinely **coincide in
outcome**. That is worth naming rather than leaving implicit: no assertion on the resolved bitset,
and no independent overlap oracle, can tell a defect that confuses them apart from correct code
here. Everywhere else the split is observable; on this path it is held by the arms being written
separately and by review.

**The simulation cannot exist before the world does.**
`mc_sim::replay::simulation_for(world: &ReplayWorld, registry: &BlockRegistry) ->
Result<Simulation, SpawnError>` resolves `SolidVoxels`, derives the spawn and constructs the
`Simulation`; the client calls it and decides nothing. `PreparedScene` carries `world` and
`registry` alongside the mesh, so the golden, probe and determinism suites build their simulation
from the same preparation their frames are packed by; the composition root builds its own from
`PreparedLaunch`, which carries the `Simulation` itself rather than a world to derive one from,
because on a resume there is no `ReplayWorld` in the process at all. `PreparationError::Spawn` is
the variant that carries a refused `SpawnError` across the crate boundary. Because the world is generated on the preparation worker several frames after the window
opens, `collect_preparation` hands the constructed `Simulation` to `Session::attach_simulation` once
the scene reaches `ScenePhase::Ready` — the phase lives in `App`, the simulation lives in `Session`,
and the invariant that one is `Some` exactly when the other is `Ready` is held jointly by that one
function rather than structurally by either type alone. `App::present`'s tick advances only once the
world has landed — one tick per rendered frame, once the world is ready, not before.

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
losing focus is translated the same way any other event is, never decided in the adapter.
`crates/mc-client/src/events.rs` reduces a `winit::keyboard::KeyCode` to a `KeyKind` (`key_kind_of`)
and forwards it, but the five-row binding table itself — `bound_action(KeyKind) ->
Option<PlayerAction>` — lives in `Session` (`crates/mc-client/src/session.rs`) and is private to it,
callable from nowhere outside; `kind_of` lost its public visibility for the same reason. Every key
event still reaches `InputState::apply(action: Option<PlayerAction>, pressed: bool)` in `mc-sim`, but
the decision of which action a code maps to is now inside `Session`, in `mc-client`, driven directly
by the input harness described in `docs/technical/testing.md` rather than inferred from
`InputState`'s own contract (§"The client input dispatch" below).

**The camera the player's state implies is a pure derivation**,
`mc_sim::player::eye_pose(state) -> CameraPose`: the eye stands over the feet at `EYE_HEIGHT`, and
the target is the eye plus the unit look direction `(cos pitch × cos yaw, sin pitch, cos pitch ×
sin yaw)`. `mc_render::camera::waiting_view()` is what the frame reads before a `Simulation` exists —
the world origin, looking along +x — and it is a **declared** pose rather than an arbitrary one:
`ScenePhase::Preparing` never draws anything through it, but `frame_stats` builds a view-projection
matrix from every snapshot's camera regardless of phase, and a degenerate pose whose eye and target
coincide would put a NaN through that matrix rather than a harmless number.

## The client input dispatch: `Session`, the drivable core

Before this seam existed, a keystroke, a pointer motion and a cursor grab were each verified only as
*policy* — the pure functions above, and the binding table — never as product behaviour: a client
wired to none of its own input passed the whole suite, because the winit `ApplicationHandler` needed
a real `Window` and nothing in the suite constructed one. `crates/mc-client/src/session.rs`
(`Session`) is the drivable core that closes this: it holds the capture ladder walk, the
pointer-motion gate, the key binding table and the tick's guard-drain-advance sequence, reachable by
a test process that opens no window, builds no event loop and acquires no GPU adapter
(`crates/mc-client/tests/support/input/`, `docs/technical/testing.md`).

`Client` owns the `Session` beside the `App`, not inside it — the pointer platform is the window,
which `Client` owns, and a `Session` living inside `App` would need the window to travel into the
frame path. `Session::new(pointer)` walks the capture ladder before it returns, so a session always
starts having already asked the platform for a pointer; there is no separate `start()` a caller could
omit. `App::present`'s tick call sits at the same statement position it always occupied — only the
body moved into `Session::tick`, which drains the input accumulator and advances the simulation
inside the same `Option<Simulation>` guard the frame path held before.

The pointer capture crosses to the OS/compositor and answers nondeterministically, so it is a port
rather than a direct call:

```rust
pub trait PointerPlatform {
    fn grab(&mut self, capture: CaptureState) -> bool;
    fn release(&mut self);
    fn show_cursor(&mut self, visible: bool);
}
```

`grab` attempts exactly one capture mode and reports whether the platform granted it. The ladder walk
— which mode is attempted first, what follows a refusal, when the walk gives up — is `Session`'s own
decision, not the port's; folding the walk into a single `hold(wanted) -> CaptureState` method would
put it back on the window-facing side, where a client that never asked for a pointer at all would be
indistinguishable from one that walked the ladder correctly. `crates/mc-client/src/events.rs`'s
`WindowPointer`, over `Arc<Window>`, is the only production adapter; a recording double stands in for
it in tests.

**What stays permanently unreachable by any windowless test:**

1. The `KeyEvent → (PhysicalKey::Code, is_pressed)` reduction, inside `dispatch_window_event`'s
   `WindowEvent::KeyboardInput` arm. `winit::event::KeyEvent`'s `platform_specific` field is
   `pub(crate)`, with no constructor and no `Default`, so no downstream crate can build one — and a
   real window would not help either, because `winit` synthesizes no key events on any platform.
2. The `Client` → `dispatch_*` forwarding, the `Option<Session>`/`Option<App>` guards, window and
   surface creation, and `Client`'s match on the `LoopAction` a dispatch returns.
3. `App::present`'s `session.tick()` call itself, behind the `wgpu` surface acquire, and
   `App::redraw`'s early returns for an undrawable size or a failed acquire — two of the three ways a
   frame can draw nothing. The third — no simulation yet — is exactly what `Session::tick`'s own
   guard makes observable, which is why it is not in this list.
4. `WindowPointer`'s bodies — `set_cursor_grab`, `set_cursor_visible`, `grab_mode`. Whether the OS
   actually honoured a grab is a manual check (`docs/technical/testing.md`).

Not in this list, despite sitting beside window-facing code: the key binding table, the
`KeyCode → KeyKind` spelling, the `MouseMotion` destructure, `kind_of`, the pointer-motion gate, and
the whole capture ladder. Each lives inside `Session` or inside a `dispatch_*` entry a test calls
directly, and each is exercised by the suite.

**Drivable but unasserted by any scenario today, which is not the same thing as unreachable.**
`Session::on_mouse_pressed`, the `KeyKind::Escape` branch of `on_key`, and the `show_cursor`/`release`
calls on the pointer port can all be driven through the same dispatch and are driven by no test — the
requirement that would have asserted Escape releasing the pointer and a click re-capturing it was cut
before implementation. Closing that gap needs a later spec to write a test against a seam that
already exists, not a design change; the manual acceptance checks for it are in
`docs/technical/testing.md`.

**`mc-client`'s coverage exclusion (ADR-008, narrowed by ADR-013) is unchanged, and its stated
rationale for this file has expired.** The gate's own comment reads: "`mc-client` holds only the
`winit` event-loop adapter and composition wiring, every policy having moved into `mc-render`'s pure
layer. If logic ever accretes there, that is a new ADR and not a quiet edit to this line." `session.rs`
is that accretion. The narrowing is deferred rather than made: a single workspace-wide 80% line
threshold cannot be moved by admitting a ~150-line file to the denominator, so narrowing today buys a
true rationale and no working alarm. The scenario suite and the two source scans described in
`docs/technical/testing.md` are the working alarm, unaffected either way. No ADR is amended by this
deferral.

Three `pub` entries in `events.rs` — `dispatch_window_event`, `dispatch_device_event`, `dispatch_key`
— are what both the window adapter and a test cross to reach `Session`. Break-and-place (PRO-854)
adds a new event to this same dispatch; the seam it needs is already in place.

## The editable world: break and place

Raycast targeting and block break/place (PRO-854) gave the simulation its first `&mut` world.
Before this feature, `Simulation` held `Box<dyn Solidity + Send>` over a `SolidVoxels` bitset
resolved once at construction, and no world type in the workspace had a mutating method at all.
This section records the shape that replaced it, and the seam it deliberately does not cross.

**The block store lives in `mc-world`, addressed in world coordinates.** `mc_world::world::VoxelWorld`
holds columns and exposes `block_at`, `set_block`, `column`, `columns`, `extent` — this is chunk
storage's job by the crate map, and it is what a save writes and reads back. Its index type, `WorldPos { x, y,
z }`, is **unsigned**: the one place a sign needs checking is the place that refuses it, at the
`BlockPos → WorldPos` conversion, so the type itself carries the invariant rather than a runtime
check scattered across every reader. `Extent` — previously declared in `mc-sim`, where the replay
world's fixtures used it — moved to `mc_world::world` for the same reason `WorldPos` lives there, with
`mc-sim` re-exporting it from its old path so existing fixtures keep compiling.

**`Simulation` holds a concrete `mc_sim::world::World`, not a trait object.** `World` wraps a
`VoxelWorld`, a `SolidVoxels` bitset, a dirty-section set and an `Arc<BlockRegistry>` — and keeps all
four private. It carries **no** name for empty space: `World::new(VoxelWorld, Arc<BlockRegistry>)`
takes two parameters and there is no accessor handing one out, because a cell holds a block or
nothing and nothing has no name to hand out (`technical/world-format.md`). Physics is unaffected: `advance_player` and the `collide` module still take `&dyn
Solidity`, so the cheap `Chamber`/`Ground` collision fixtures are untouched and none of the exact-
position collision scenarios changed. What changed is only what *feeds* solidity — a `World` now,
where a bespoke test double used to stand in.

**One private function writes both views, and nothing else may.**

```rust
// crates/mc-sim/src/world/mod.rs — no `pub`, visible in this module and its
// descendants (mc_sim::world::action) and nowhere else in the crate.
fn write(&mut self, at: WorldPos, block: &BlockName) -> Result<(), WorldError> {
    let solid = self.registry.resolve(block)?.is_solid;   // resolved once, before either write
    self.blocks.set_block(at, block, &self.registry)?;    // the store
    self.solid.set(at, solid);                            // the collision view
    self.mark_dirty(at);                                  // remesh bookkeeping, see below
    Ok(())
}
```

The block store and the collision bitset cannot fall out of step because there is exactly one
function that can write either, and it writes both together or neither. `break_at` and `place_at` —
the two domain operations — live in the same module as `write` and call it; the action-resolution
code that calls *those* lives in a **child** module, `mc_sim::world::action`, specifically so it
inherits visibility into its parent's private items rather than needing `write` widened to
`pub(crate)`. Only the public vocabulary (`ActionIntent`, `TickIntent`, `Refusal`, `EditReport`,
`targeted`, `REACH`, `default_held_block`) is re-exported as `mc_sim::action`.

**A second option — deriving solidity from the store on every query, with no bitset at all — was
considered and rejected, and the trade-off is worth recording rather than losing.** It would have made
the store-and-collision-disagreement defect class **unspellable by construction**: with one source of
truth there is no second view to disagree with, which is the kind of structural elimination this
codebase generally prefers over a test. What rules it out is a specific committed test it would
silently gut: the replay's overlap oracle re-derives whether the player's box overlaps a solid voxel
by reading `ReplayWorld::block_at` and asking the registry directly, sharing no lookup chain with the
physics' own resolved `SolidVoxels` — deliberately, so that an adapter bug in the resolved bitset
cannot make both sides wrong the same way. Deriving solidity from the store on every query would make
that oracle's lookup chain *identical* to the code path it exists to check, so it would pass forever
regardless of what broke, and its own positive control (a box placed inside the world's landmark)
would keep passing too. The chosen shape keeps the oracle's independence at the cost of a private
invariant instead of a structural guarantee — recorded here because a future reviewer proposing the
derived form should know what it would cost, not just what it would buy.

**`World::mesh(&self) -> Result<Vec<SectionQuads>, PrepareError>` reads the whole world once and marks
nothing dirty.** It takes `&self`, and `mark_dirty` is reachable only through `write`'s `&mut self` —
so a call to `mesh` cannot also dirty what it just meshed, by the borrow checker rather than by
convention or review. That is what lets a launch mesh a resumed world exactly once, at preparation
time, with nothing left in the dirty set for the frame path to drain afterward.

**The raycast's reach bound is a single site.** `targeted(origin, direction, reach, world) -> Option<Hit>`
walks voxels in ascending entry distance and stops as soon as the next voxel's entry distance exceeds
`REACH` (5.0 blocks) — there is no second, separate `distance <= REACH` check anywhere else. This is
not a style preference: `Solidity` is **total** (it answers `false` for every position outside the
loaded world), so an unbounded traversal plus a separate distance check would never terminate for a
ray that hits nothing — the traversal has no other reason to stop. A reach bound spelled as a second,
independent comparison is therefore not just redundant, it is a latent hang waiting for the one ray
that never meets a solid voxel. The traversal additionally reports the entry face, which is what a
placement's target cell is computed from.

**The production traversal is deliberately not the goldens' oracle.** `crates/mc-client/tests/support/oracle.rs`'s
`March` is the independent DDA the golden frames are judged against; promoting it to production would
collapse oracle and subject into one implementation, which is exactly the failure mode
`docs/technical/testing.md`'s derived-oracle discipline exists to prevent. The two stay two separate
implementations on purpose, and neither may import the other.

**The action a client can request is an enum with no room for a position.** `ActionIntent { Break,
Place { block: BlockName } }`, carried alongside movement in `TickIntent { movement: MovementIntent,
action: Option<ActionIntent> }`. A break carrying a block name is unrepresentable, and neither variant
declares a position, a coordinate, a cell or an absolute orientation — invariant 4 ("the server is
authoritative") in structural form, the same shape `MovementIntent` already uses (ADR-014). A place
request names *what* to place; it is never asked, and never able to say, *where*.

**Ordering inside a tick: the action resolves after that tick's movement and look are applied.**
`Simulation::advance` advances the player first, then resolves the action against the state the tick
*ends* with — against the already-clamped, already ±89°-limited view, not the raw input deltas. A
click's target is therefore always consistent with the camera pose the same tick publishes. `Session`
holds `pending_action: Option<ActionIntent>`, set when a mouse press lands while the pointer is
captured; a tick's own `take()` clears it whether or not a simulation was attached to consume it, so a
click is spent by the tick it lands in — one press is one action, never a latch that keeps firing.

**The refusal vocabulary, and where each is decided.** A resolved action returns `EditReport`, a value
rather than an error — most call sites never inspect it, and it is deliberately not `#[must_use]`, but
a scripted test rig can assert *why* a request was refused, not only *that* it was.

`EditReport::Changed { cell, from: Contents, to: Contents }` — both ends carry `Contents`, because
both ends can be nothing: a break declaring no `breaks_into` leaves the cell empty, and a placement
into an empty cell replaces nothing. Neither is reported as a named block, so a report cannot claim a
block was removed where none was, or that one was left behind where the cell was emptied. The
residue is `named.map_or(Contents::Empty, Contents::Holds)` with **no fallback** — there is no name
for the engine to fall back to, and picking one would be a game rule content could not override.

| Refusal | Decided by |
|---|---|
| `NoTarget` | the raycast finds no solid voxel within reach — this also covers "something is there but too far away": the reach bound is one site, so a target beyond it and no target at all are indistinguishable without an unbounded search, and none is done |
| `NoFace` | the eye is inside the targeted block, so there is no entry face to place against |
| `Indestructible` | the targeted block's definition declares `breakable = false` |
| `Occupied` | a placement's target cell holds a block content does not declare `replaceable` — read from content, never derived from solidity. An **empty** cell is never occupied: it permits the placement because it is empty, not because content said so, and `replaceable` cannot speak for it |
| `InsidePlayer` | a placement's target cell overlaps the requesting player's own collision box |
| `UnknownBlock { name }` | a placement names a block the registry does not hold |
| `OutsideWorld { at }` | the target cell falls outside the world's storable range, in any direction — a negative coordinate and a past-the-edge one both collapse into this one variant rather than reporting separately, which keeps a fixture built for one edge from silently validating against the other |

Two refusal variants were designed and then struck before shipping. `OutOfReach`, distinct from
`NoTarget`, would have needed the unbounded traversal the reach-bound design above rules out — so
"nothing is there" and "something is there but out of reach" are deliberately the same answer.
`NotSolid`, refusing a placement whose *named* block was not itself solid, was struck because
`replaceable` already forbids overwriting anything content has not opened — a client naming a
non-solid block still cannot delete stone, so the extra check bought no additional safety while
costing `base:water` its own placeability.

**Three arms and never two, wherever both wrappers survive.** `broken`, `placed` and `overwritable`
each read the world through `Option<Contents<&BlockName>>`, and each writes `None`,
`Some(Contents::Empty)` and `Some(Contents::Holds(..))` as separate arms — a `let
Some(Contents::Holds(..)) = .. else` would answer "there is no such cell" and "this cell holds
nothing" with one refusal. Both readings reach the same permitting or refusing outcome at every one
of those sites today, which is precisely what would make the collapse invisible: the arms are
separate because the facts are separate, not because the outcomes differ. `broken`'s empty arm is
unreachable in practice — the raycast stops only at a solid cell — and it is written out anyway, so
that a break which ever did reach an empty cell refuses rather than being read as a cell the world
does not have.

One collapse is worth flagging as a known, accepted gap rather than a silent one: the world's own
bounds check discards *which* internal bound was crossed before it reaches `EditReport` — a target
past the positive-octant edge and a target that fails a lower-level section-array bound both arrive as
the same `OutsideWorld` refusal. Nothing in MVP 1 needs the distinction; a future consumer wanting a
refusal to name which bound was crossed changes that one collapse point.

## Making an edit visible: the remesh transport

Editing the block store and editing what the renderer draws are two different problems, and
break/place (PRO-854) keeps them on their existing sides of the simulation/renderer seam described
above rather than opening a new one.

**Marking is unconditional and generous.** `World::write` marks its own section **and all six
face-adjacent sections** dirty on every write, with no "only if the voxel sits on a section boundary"
test — the correct thing rather than the fast thing; an over-marked section costs an extra remesh, an
under-marked one leaves a stale face on screen. A section the loaded footprint does not contain is
silently skipped rather than reported — the edge of the world is not an error. The dirty set is a
`BTreeSet` keyed by section, which bounds it at the number of sections the footprint holds however
many edits accumulate between drains, and drains in a deterministic order.

**The batch that crosses to the client is an owned, `Send` value, not a borrow.** `Simulation::take_remesh_work`
returns a map of every section the batch needs — each dirty section plus its neighbours, cloned once —
together with the list of keys to mesh. Nothing about it holds a reference into the session the
simulation lives in; a `Section` is at most ~8 KB, and a typical batch is well under 100 KB. Meshing
and splicing the result back into scene geometry are both pure functions living in `mc-sim`, run on a
dedicated worker thread rather than the tick or the frame thread, so a remesh never competes with
either. `splice` replaces sections **positionally**, matching each remeshed section against its
existing slot by `(column, section index)` — it never appends and never sorts — which is exactly what
keeps `mesh_all`'s section ordering, a golden-frame dependency, from moving under an edit.

**Texture layer assignment is not re-resolved on a remesh, and that no longer rests on which blocks a
world happens to contain.** Layers are resolved once, at launch, from `layers_of(&registry)` — every
block the content registers, never the meshed world's quads (`technical/rendering.md` §"Textures are
array layers, never an atlas") — so an edit or a resumed save can add a block a world had none of
without shifting anything already assigned. Re-resolving on every remesh is unnecessary rather than
deferred: nothing about which blocks are registered changes mid-session in MVP 1. Hot-reloadable
content reopens the question, and the failure mode if a quad ever named an unresolved key is a loud
error at packing time (`GeometryError::UnresolvedTexture`) rather than a wrong texture — recorded
against MVP 2 in `crates/mc-render/CLAUDE.md`, which is a separate gap: a quad is still matched to a
key by the block's *name*, not through the registry.

**A failed remesh batch is dropped and reported once — the opposite rule from scene preparation,
deliberately.** Preparation fails the whole run if it cannot build a scene at all; a remesh that cannot
be applied mid-session should not take a running game down over one edit, so it is logged and the
batch is discarded instead.

## Saving and loading a world

Persistence adds one crate-boundary shape to the map above: a new leaf module in `mc-world`, a thin
policy layer in `mc-sim`, and wiring with no policy of its own in `mc-client` — the same
inward-dependency direction every other seam in this document follows.

**`mc_world::persistence` is a new module, and its public surface is the only place a save's byte
layout is named.** `save_world`, `write_save`, `replace_atomically`, `load_world`, `requirements`
and `stored_world_data` are its public functions; `SaveError` and `LoadError` are its public error
types (`technical/world-format.md` has the on-disk format itself). `postcard::` is nameable only
inside this module, and every one of its decode failures collapses to `LoadError::Malformed { path
}` at this module's edge — nothing about *how* the encoder declined a file crosses the boundary.
That confinement is a design decision with a measured payoff: when `bincode` was found permanently
unmaintained and had to be replaced with `postcard` (`technical/decisions.md`, ADR-016), the swap
touched four files and five call sites inside this one module and changed no test anywhere in the
workspace, because nothing outside the module had ever named the encoder or its error types.

**The save path reaches the world's blocks through `mc-sim`, not directly from `mc-client`.**
`mc_sim::world::World` gains `pub(crate) fn blocks(&self) -> &VoxelWorld` — a shared borrow, so the
crate's single private write path (`World::write`, see "The editable world" above) is untouched by
it — and a new `mc_sim::persistence` module owns both `save(simulation, path)` and
`simulation_at_launch(save, seed, registry, accepting)`. It takes the **seed** rather than a world
built from it, and generates one only in the arm where there is no save to resume — a caller handing
over a `ReplayWorld` has already generated one, so "a resume derives no world from the seed" would
otherwise be true of the function and false of the process running it. Deciding which world a launch plays,
what a refusal does, and what happens on quit is policy, and `mc-sim` is where it lives: it is the
crate the coverage gate actually measures (ADR-013), and a save is server state (invariant 4)
regardless of which process happens to host the authoritative simulation in MVP 1. Putting that
policy in `mc-client` instead — a crate ADR-013 excludes from coverage on the express ground that it
"holds only wiring", with its own Rejected section warning that narrowing the exclusion is "a new
record, not a quiet edit" — would have put a real decision in the one place nothing measures it.

**The resume decision sits inside the launch preparation, and `prepare_scene` is save-blind because
it is a different function.** `prepare_scene` is the public entry point the golden-frame, probe and
determinism suites all shoot through — the same pipeline a player launches, not a copy of it. A
"load the save if one exists" branch inside it would let a save file sitting in a capture's working
directory change what a golden frame shows, silently, for a reason that has nothing to do with the
renderer. That is the invariant, and it is unchanged.

What changed is where the other path lives. There are two preparation entry points rather than one
with a branch above it: `startup::prepare_scene(root)` turns a content root into a drawable
*generated* scene and reads no save, and `launch::prepare_launch(root, save, accepting)` asks
`mc-sim` which world this launch plays and prepares **that** world. The launch path establishes
which world is needed before doing any work for it — `launch::simulation_to_play` calls
`mc_sim::persistence::simulation_at_launch` first, and only then meshes, resolves layers and packs —
so a resume generates nothing at all, rather than generating a world, meshing it and discarding both
once a save turns up.

Two of anything on the golden path is how images drift, so the split is answered structurally rather
than by care: both doors share one mesher (`mesh_world`), one definition of the texture key set
(`layers_of`, over the *registry* and never over a world) and one packer (`scene_of`), so the only
thing that can differ between them is which world's blocks went in — and that equality is asserted
byte for byte, over both the section table and the packed vertices, by
`crates/mc-client/tests/launch_and_capture_agree.rs`.

**`mc-client` gains wiring and one ending translation, and no policy.** `launch::save_path()` names
where the save lives (relative to the working directory, mirroring `CONTENT_ROOT`'s own convention,
and deliberately not checked for existence — a missing save is the no-save case, not a failure);
`startup::acceptance_from(args)` parses the one command-line flag MVP 1's human channel needs
(`--load-changed-blocks`, `docs/user/gameplay.md`) and does nothing else. `Session::save` is a
three-line forward to `mc_sim::persistence::save`. The one piece of client-side logic that is more
than wiring is `ending_after_saving`, in `session.rs` and not in `mc-sim` because it returns
`mc_render::window::Ending` and `mc-sim` may not name `mc-render` — it decides that only a run that
ended by closing normally (`Ending::Closed`) saves at all, so that a device-lost or otherwise broken
run can never overwrite a good save with a half-built one, and that a failed save on a clean close
becomes a failed ending naming the path and the reason rather than a silent one. Nothing about
*which* world to play, *whether* a changed-block save should load, or *how* a refusal is worded
lives in `mc-client` — those answers all come from `mc-sim` and `mc-world`, unchanged by which crate
happens to call them.

## The HUD: content in three crates, composed through one entry point

The HUD (`docs/modding/hud.md` is the authoring contract) is the first thing
that is *purely presentation* and is nonetheless content: what is drawn, and
where on the screen, is declared under `content/` with no HUD definition
anywhere in Rust — invariant 1 applied to something that is not a block. The
debug overlay beside it is deliberately the opposite: engine-owned tooling
content cannot reach. Both halves land on the crate map above without adding
an edge between workspace crates.

| Piece | Location | Why there |
|---|---|---|
| Element model, field checking, namespaced name, the published draw-kind and readable-value sets | `mc-core/src/hud/` | Pure, no I/O, and the only place both the loader (`mc-world`) and the composer (`mc-render`) can reach. `mc_core::id::namespaced` is already here. |
| TOML directory reader | `mc-world/src/content/hud_toml_source.rs` | `toml` is confined to `src/content/` by `mc-world`'s own dependency-graph invariant, and this is the one file MVP 2's swap deletes. |
| Rectangle derivation, two-pass ordering, clipping, colour decode, held-block resolution | `mc-render/src/hud/` — **outside** `src/gpu/` | Pure functions, so ADR-013 counts them. |
| Screen-space pipeline and `hud.wgsl` | `mc-render/src/gpu/hud.rs` | Needs a device. |
| Overlay state, readout, clock port | `mc-render/src/overlay/` — **outside** `src/gpu/` | Counted by the gate. In `mc-client` it would be invisible to coverage entirely. |
| egui painting | `mc-render/src/gpu/overlay.rs` | Needs a device, and is the egui adapter (ADR-017). |
| Toggle binding, held-block reader, launch refusal | `mc-client/src/{bindings.rs, session.rs, startup.rs, app.rs}` | Composition root. |

**`mc-render` may not name `mc-sim`, so the HUD model lives in `mc-core`.**
Two alternatives were weighed and rejected: the whole HUD in `mc-client`,
where ADR-013 excludes the crate from coverage wholesale, so forty-odd pure
scenarios would be measured by nothing; and the whole HUD in `mc-world`
beside blocks, which would make `mc-render` reach composition maths through
`mc-world` and file screen-space geometry as world data. The cost of the
split chosen is stated rather than hidden: **a reader chasing one
declaration crosses two crate boundaries to see it drawn.** That is the
shape blocks already have (`mc-core` definition → `mc-world` loader →
`mc-render` texture), so it is a topology a contributor already knows.

**`HudElementSource` is its own port, not a reuse of `DefinitionSource`.**
`DefinitionSource` yields `BlockDefinition`; a parallel port yielding
`HudElement` is the only shape that keeps the two loaders swappable
independently at MVP 2. It is mandatory at first use as an I/O boundary,
which is the exemption `code-quality.md` §1 grants ports from the
three-uses rule. Two implementations exist immediately, as for blocks:
`TomlFileHudSource` in `mc-world`, and `InMemoryHudSource` beside the port.

`InMemoryHudSource` deliberately parts company with
`InMemoryDefinitionSource`: it holds **raw declarations**, not
already-checked elements. A source of accepted elements cannot express "a
declaration stating `size = [0, 4]`" at all — a test would have to
hand-build the fault it claims the model produces, and would then pass
against a model that produced no faults. Holding declarations is also the
shape a Luau table arrives in. The stated cost is that it cannot express an
unreadable source, which nothing needs it to.

**A separate `HudFault { origin, element, field, cause }`, not a renamed
`DefinitionFault`.** The block fault's field is literally named `block` and
its `Display` writes ``block `…` `` — reused as-is, every HUD failure would
read "block `base:crosshair-horizontal`", which is the wrong vocabulary in
the one message a content author reads. Renaming the field in `mc-core`
would change a public field, its `Display` output, and every block-loading
error message and test. So roughly 25 lines are duplicated, which
`code-quality.md` §1's "tolerate harmless duplication" covers. The argument
against is real — two fault types will drift in message shape — and what
holds it is that both are asserted per field by scenarios, which is exactly
where drift would show.

**The raw declaration form is format-agnostic, not TOML-shaped, and that was
forced rather than chosen.** `toml` and `serde` may not reach `mc-core` (the
resolved-graph invariant below, which follows dev-dependency edges too), so
the `#[serde(deny_unknown_fields)]`-over-`Option<toml::Value>` shape
`RawBlockDefinition` uses in `mc-world` was unavailable one crate in.
`mc_core::hud::RawHudElement` is a list of `(String, DeclaredValue)` pairs
where `DeclaredValue` is a small closed enum with an `Opaque(kind)` arm that
makes the conversion from any format **total**; unknown-field rejection is
`mc-core`'s own job rather than serde's, checked against an `ACCEPTED_FIELDS`
list assembled from the same field-name constants the checking reads. The
loader converts `toml::Value → DeclaredValue` and hands over a key/value
list; it deserializes into no struct. This is what the evolvability driver
actually bought: at MVP 2 a Luau table is not a `toml::Value` either, so the
checking survives the loader swap instead of being deleted with it.

### One composition entry point, and `record_terrain` preserved beside it

```rust
pub struct FrameSnapshot<'a> {
    pub terrain: &'a TerrainSnapshot,
    pub hud: &'a HudFrame,                    // may hold zero elements
    pub overlay: Option<&'a OverlayReadout>,  // None when hidden
}

impl FrameRenderer {
    /// The client's only frame call: terrain, then `compose_hud` exactly
    /// once, then the overlay when one is supplied.
    pub fn record_frame(&mut self, …) -> Result<FrameStats, FrameError>;
    /// The single HUD composition entry point, public so a test can compose
    /// onto an arbitrary cleared target.
    pub fn compose_hud(&mut self, …) -> Result<(), FrameError>;
    /// How many times `compose_hud` has run.
    pub fn hud_compositions(&self) -> u64;
}
```

`App::draw` calls `record_frame` and nothing else, and `App` owns a
`FrameRenderer` where it used to own a `TerrainRenderer`. **A frame test
that composed the HUD through a path the windowed client never takes would
verify a composition the product does not perform** — the "second entry
point onto a tested path" failure this project has now met five times
(`docs/technical/testing.md`). Two joined facts hold it, and neither is
sufficient alone: a source scan asserts `mc-client/src` names no HUD-drawing
spelling other than the one `record_frame` call site, and an offscreen test
drives `record_frame` once and watches `hud_compositions()` go 0 → 1. A test
cannot open a window in CI, so "the windowed client" is reached through the
object the windowed client owns.

**`TerrainRenderer::record_terrain` stays public and unchanged.** It *is*
"the HUD stage not run at all" — the zero-HUD baseline that every
"pixels outside the footprint are untouched" assertion compares against, and
what the three frozen terrain goldens are shot through. Its cost is recorded
in `technical/rendering.md`: the terrain goldens no longer traverse the
client's exact frame call, and it is the *pair* of golden sets that covers
the product path.

The HUD pass is a second render pass with `LoadOp::Load` on colour and **no
depth attachment**; the overlay is a third, likewise `Load`. Order is
terrain → HUD → overlay, which is what makes "content cannot obscure the
overlay" true by construction rather than by a check.

**Error contract.** A HUD load failure is fatal to the launch:
`PreparationError` gains one variant carrying it. `FrameError` gains **no**
variant — an unresolvable swatch texture draws nothing and is reported once
through `App`'s existing "state it once" shape.

### The debug overlay: engine-owned, clock-confined, unreachable from content

**The overlay is deliberately not content, and that is a rule rather than a
convention.** A mod must not be able to disable the instrument used to
diagnose that mod. Three facts hold it: no field of a declaration can refer
to the overlay, the published readable-value set is a constant asserted to
be exactly `{"held-block"}`, and the overlay's pass is composed after all
content. An element named `base:debug-overlay` is an ordinary element and
changes nothing — the name is not recognised anywhere.

**The guarantee is about *loaded, running* content, and the narrowing is
correct only because content loads once, at startup.** A declaration that
fails to load never runs: the launch is refused, and a fault naming file,
element and field is a better diagnostic than an overlay showing a position
and a frame time — which could not have diagnosed a malformed declaration
anyway. **Forward constraint, named as an event so it is triggered rather
than rediscovered: the commit that makes HUD or block declarations
hot-reloadable (MVP 2) reopens this.** At that moment a HUD fault can arrive
mid-session, refusing is no longer available because there is nothing left to
refuse *to*, and invariant 3's reasoning — a bad mod never takes down the
server — binds the client identically. The overlay guarantee then becomes
load-bearing in the way the engine-contributor story originally intended, and
the "no content-load-failure screen" position expires with it.

**The wall clock is confined, not admitted.** `mc-render/src/overlay/clock.rs`
holds the `OverlayClock` port and its one `SystemOverlayClock` adapter, and is
the single file exempted from a scan asserting that no other production
source under `crates/mc-client/src` or `crates/mc-render/src` names `Instant`
or `SystemTime`. That is what keeps a replay identical at 30 and 300 fps while
the overlay still shows a frame rate, and the injectable port is what makes
"ten frames 20 ms apart" a test at all — a frame rate no test can drive is a
readout no test can grade.

`app.rs`'s header used to claim "no wall clock is read anywhere in this
client". One now is, so the claim was **narrowed to the true one**: no clock
reaches the tick, the snapshot or the capture path. A stale absolute in the
very file a scan is about would be the worst place in the workspace to leave
one.

Four smaller shapes, each load-bearing:

- **`Session::held_block()` returns an owned `Option<BlockName>`, never a
  borrow.** `BlockName` is an `Arc<str>` newtype, so this is a refcount bump,
  and it preserves the property `session.rs` protects: the session hands out
  no borrow of what it owns. `overlay_visible()` is the same shape over a
  `Copy` value.
- **`Session::overlay_readout` derives the reading and `App` merely forwards
  it.** `App` is the one object nothing in this workspace executes
  (`docs/technical/testing.md`), so every piece of a readout assembled there
  could be wrong with the whole suite green. `App` gained one call and no
  arithmetic. For the same reason `DebugOverlay::readout(Option<Vec3>)`
  derives the column from a position it is *handed*, rather than the client
  assembling the readout field by field — otherwise "no position before the
  world lands" would be a test setting `None` and asserting `None`, the
  subject agreeing with the test.
- **The toggle is a client-side action that never reaches `mc-sim`.**
  `PlayerAction` is untouched, so "a replay reaches the same world state with
  the overlay shown and hidden" holds by type rather than by discipline.
  Visibility flips on press only. `Bindings` lives in its own
  `mc-client/src/bindings.rs` rather than inside `session.rs` — with it
  inside, that file measured 532 non-blank lines against the 500 limit — and
  `bound_action` is `pub(crate)` there. The property the placement protects
  is unchanged: an integration-test binary is a separate crate, so a test
  asking the table what a key means is still a compile error, and a test
  constructs bindings instead.
- **`mc_world::column::column_containing(x: f32, z: f32) -> ColumnCoordinate`**
  takes two scalars rather than a `Vec3` because **`mc-world` has no `glam`
  dependency and this must not add one**; the caller destructures. It
  **floors** rather than truncating, so x = −0.5 is column −1, and it lives
  beside `SECTION_SIZE` and `ColumnCoordinate`, which is where its two facts
  already are.

## The scripting host: what is Rust-side, what is behind the backend adapter

`mc-script` holds the sandboxed Luau state, the callbacks content registers, and
everything that bounds what content may do inside it (`modding/sandbox.md` is
the authoring contract). Its position on the crate map is the first thing worth
recording, because it is unusual: **`mc-script` resolves `mlua` and no workspace
crate at all**, and **nothing in `crates/` depends on `mc-script`**. The host is
exercised solely by its own tests and by the hostile-mod harness. Two absences
follow from that and are deliberate rather than pending: there is no
`DefinitionSource` adapter backed by script — the registry seam above is
untouched — and there is no per-mod CPU accounting, because accounting needs a
tick to attribute against and no tick calls into this crate.

**The backend is nameable in `crates/mc-script/src/luau/` and nowhere else.**
Everything outside that directory speaks in the crate's own vocabulary —
`ScriptHost`, `ScriptValue`, `ScriptTable`, `ScriptFunction`, `ScriptFault`,
`HostLimits`, `DispatchReport` — and `mlua`'s `Lua`, `Function`, `Table`,
`Value` and `Error` appear on no public signature. This is the same
boundary-isolation shape as `mc_world::persistence`'s encoder confinement, and
for the same measured reason: the backend is pre-1.0, breaking minor releases
are routine, and a vendor type on a public signature is a migration the crate
could not perform without breaking every consumer. The port is shaped around
what the host needs — *evaluate this chunk under a budget; invoke this
attachment; tell me how it ended* — not around the backend's surface, which is
what a replacement would otherwise have to reproduce.

Five backend behaviours the design rests on carry no stability promise: which
globals closing the sandbox leaves standing, the writable child environment it
hands the running thread, the interrupt's error propagation, how allocation
failure is reported, and the `[string "name"]:N:` prefix a message carries. All
five are observed only inside that directory, which is what makes an `mlua`
upgrade a re-measurement rather than a rewrite.

**The split, stated as the two halves it is.** Behind the adapter live the
things that are the backend's shape: the state's construction order, the frozen
per-chunk environment, the interrupt guard and its latch, the script-side
protected call every callback is invoked through, value translation and the
handles. On the Rust side, keyed by `Attachment` — the `(subject, component)`
pair — live the things that are the host's policy: which callback belongs to
which attachment, the cumulative invocation count, the consecutive-fault count
and the quarantined set, the pending-work queue, the round index, and the
construction and attribution of every fault.

| Concern | Where | Why there |
|---|---|---|
| Denied globals removed, host `print` installed, sandbox closed, globals frozen, interrupt armed | `src/luau/vm.rs` | The order is load-bearing (below) and every step names the backend. |
| Per-chunk frozen environment, and the three tables that have to be frozen | `src/luau/env.rs` | Chunk isolation is a property of how the environment is built. |
| The latch: budget and memory cap on one interrupt tick | `src/luau/guard.rs` | It is the interrupt callback's own state, shared by a refcounted handle. |
| `pcall` trampoline, backend-error classification | `src/luau/trampoline.rs` | A raised value must come back as a return value, not as a propagating error. |
| Message/line splitting, value translation, raw rendering | `src/luau/translate.rs` | Vendor error translation belongs at the adapter and this is the whole of it. |
| Callback registry, invocation counts, the pending queue, round bookkeeping | `src/dispatch.rs` | Policy about attachments, expressible without naming a VM. |
| Consecutive-fault counting, quarantine, which kinds count | `src/quarantine.rs` | Same. |
| The six limits and their shipped defaults | `src/limits.rs` | One source for the numbers, with the reason each holds beside it. |

**The construction order is a design decision, not a style.** Everything the
host removes or installs happens **before** the sandbox is closed, because
afterwards the running thread reads through a child table: setting a denied
global to `nil` then returns success and removes nothing, and a host `print`
installed late is bypassed by a fall-through to the backend's own, which writes
to raw file descriptor 1 outside every log and every limit the host controls.
That is a capability escaping the sandbox, not a logging inconvenience.
Closing the backend's sandbox is not sufficient in two separate ways — it
removes five of the fourteen denied names and leaves nine standing, and it
leaves the sandboxed globals table itself writable, so the host freezes that
too.

**One latch, two limits, and it is what makes a limit mean anything.** The
interrupt checks memory before charging the tick — with a budget generous enough
not to trip, an allocation bomb must be stopped by the cap, and charging first
would report whichever limit happened to be nearer. Once either trips, the guard
is sticky: every subsequent interrupt fails without looking at anything, so no
script frame runs, including the `pcall` handler that would have caught the
error. `Lua::set_memory_limit` alone does not cap allocation — the allocator's
refusal is an ordinary catchable Lua error — so it is set *above* the enforced
cap as an absolute backstop, and the enforced per-invocation cap is the
interrupt reading `Lua::used_memory()` each tick against the baseline the entry
started from. The host clears the latch at the start of each guarded entry,
which is what keeps the budget per-invocation rather than a one-way ratchet.

**Dispatch is never re-entrant, and that is the load-bearing decision behind the
round.** Follow-up work a callback asks for is appended to a queue and drained
by later invocations of the same round or by later rounds; entering it inline
would turn an unbounded cascade into Rust stack growth, and a stack overflow is
an abort — exactly the outcome invariant 3 forbids. Queueing converts recursion
depth into queue length, which is countable and therefore boundable, and two
bounds are needed rather than one: the round bound limits invocations per round
and says nothing about queue length, while a callback returning a fan-out grows
the queue faster than a round drains it, and every entry is a host-side
allocation outside every script-side limit. Work that cannot be admitted is
**refused and named**; work that merely did not fit this round is **deferred**.
They are different fault kinds because only refusal loses the work, and an
operator reading one kind cannot otherwise tell "wait" from "something is gone".
A seed is appended to whatever is still waiting rather than replacing it, so a
round with an empty seed drains the residue; and a quarantined entry is skipped
**without spending an invocation**, or a queue full of quarantined targets would
crowd out everything still running.

**Every value the host reads out of script is read raw.** An ordinary indexed
read consults `__index`, which is script the table's author chose — so a host
reading a field the ordinary way runs a mod's code on its own schedule,
unbudgeted, at a moment the mod picked. The follow-up list is the one value in
the design whose shape a mod chooses, and the field, each slot and both identity
strings are all read raw; an entry that is not two strings is passed over rather
than guessed at. The same rule governs rendering: a raised value is rendered by
matching on it and never through `tostring`, because the backend installs a
message handler for every protected call it makes and that handler would run a
`__tostring` before the host ever saw the error.

**Handles carry two facts the engine cannot reconstruct later.** A
`ScriptTable` or `ScriptFunction` is opaque — the engine may hold one and hand
it back, never reach through it — and each is stamped at creation with the
script state it came from and the chunk it came out of. The chunk is what lets a
fault name the file that *defined* the failing callback rather than only the
round it ran in; the state tag is what will make substituting a scratch-state
callback for a live one verifiable when reload builds candidate registries off
the tick thread.

**Nothing in the crate may panic, and that is a build error rather than a
convention:** `crates/mc-script/src/lib.rs` carries a crate-root
`#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]`, which
composes with the inherited workspace lints. Its scope stops at the library and
its sibling unit-test modules — `technical/testing.md` has what that leaves
uncovered and what reaches it instead.

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

**Every rendering decision is a pure function in `mc-render`, and that is
what keeps the GPU-facing half of `mc-client`'s coverage exclusion honest**
(ADR-013). Surface format selection, resize and depth-reallocation policy,
surface-error → frame-action policy, window-event → action policy and the
device-request description all live in the pure layer, so the GPU-touching
side of `mc-client` is left holding the `winit` event-loop adapter,
composition wiring, and the per-frame mechanics that have no pure form —
acquire the surface texture, create an encoder, submit, present — and
nothing that *decides* anything there. **This no longer describes all of
`mc-client`**: `session.rs` decides the capture ladder and the pointer-motion
gate, and `bindings.rs` holds the key binding table (§"The client input
dispatch" above). The HUD and overlay work took the opposite direction
deliberately for exactly this reason — the rectangle derivation, the two-pass
composition and the whole overlay readout live in `mc-render`'s pure layer,
counted, while `mc-client` gained a field, a call and a forward (§"The HUD"
above). ADR-013's
exclusion is unchanged, and its stated rationale no longer covers that file
— see that section for why the narrowing is deferred rather than made. The
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
  except the sibling `*_test.rs` unit files, and fails if any of the four
  shipped block names appears in a file's *production text* — the file
  minus its doc comments, since a doc example is a doc test that does
  live in a production file (`technical/testing.md`). This scan also
  carries a positive control: pointed at a fixture directory containing
  one of the four names, it must report a hit, or a broken path glob or
  matcher could pass by never actually looking at anything. It carries
  two further controls for the filters themselves — a name in a sibling
  `*_test.rs` is skipped while one in the module beside it is still
  found, and a name in a doc example is not reported — because a filter
  that skipped too much would leave the first control green while
  scanning nothing.
- **The same scan carries a second list, of names the base game has
  *retired*.** A name that leaves the shipped list stops being watched,
  which is the one way the invariant above quietly loosens: the base
  game's former empty block was removed the day a cell could hold nothing
  (`modding/blocks-items.md`), and after that removal nothing mechanical
  stood between the engine and writing the name again. An entry on the
  retired list never leaves it. It is one walk answering both lists, so
  the exemption and the test-file skip mean the same thing for both, and
  the two results are kept in separate collections so the absence check
  and its control each assert on the retired result alone — a single list
  would let a control naming a retired block pass on a shipped-name match,
  which is the reading the control exists to rule out.

  **It buys less than "the name cannot come back", and the difference is
  worth knowing.** The scan's one exemption applies to retired names too,
  so the name may still reappear in that single exempt file; doc comments
  are stripped before matching, so it may reappear in any doc comment
  anywhere; and test sources are not scanned at all, which is what lets a
  test declare a *solid* block under that very name to prove the engine
  recognises no name at all. What it does buy is the rest of the
  production tree, where the name has no business being and where nothing
  else would notice its return.
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
- **Nothing in `crates/` resolves `tools/voxforge`, in any dependency
  kind.** A test walks every `crates/mc-*` package's resolved dependency
  closure and fails if `voxforge` appears anywhere in it, carrying the
  same positive control as the `mc-core`/`toml` walk above: it also
  asserts that a dependency each inspected crate genuinely has *is*
  present, so a walk that silently resolved nothing does not pass
  vacuously. `tools/` — home to `voxforge` and, per ADR-009, developer
  tooling generally — may depend inward on `crates/`; the reverse never
  holds.
- **`mc-client`'s resolved closure excludes `mc-script`, in every dependency
  kind.** The client is untrusted code running on a player's machine and the
  scripting host is the server's enforcement of what a mod may do; an edge
  between them — through any intermediary, dev-dependencies included — would put
  the enforcement inside the thing it enforces against. The walk reads `cargo
  metadata`'s `resolve.nodes[].deps`, which is cargo's own resolution with every
  kind already folded in, rather than a manifest (direct edges only) or
  `Cargo.lock` (every workspace member, which would make the assertion vacuously
  false). It returns an **enumerated verdict** rather than a boolean, so
  "the host is absent from the metadata altogether" and "the client does not
  reach it" cannot be reported the same way — a renamed or removed crate reddens
  for free. Two controls sit beside it: the same walk over doctored metadata
  with the host removed must *refuse*, and the same walk must report a
  dependency the client genuinely has (`winit`) as present, because the walk
  seeds itself with its own root and would otherwise report a closure having
  followed no edge at all.
- **`mlua` is nameable under `crates/mc-script/src/luau/` and nowhere else**,
  asserted by a text guard over both the crate's `src/` and its `tests/` roots.
  `tests/` is scanned because the hostile-mod harness is the code most likely to
  reach for the backend directly, and a harness that built its own VM would be
  verifying the host against a copy of the thing it watches. Each root's
  contribution is counted **on its own** — a root that contributes nothing
  leaves a total healthy and the absence check green over a tree nothing read.
  Its two exemptions (the adapter directory, and the guard file itself, whose
  needle would be its own hit) are compared **segment by segment against the
  whole path**, never against a bare file name: a name-only exemption for
  `vm.rs` would silently excuse a `tests/support/hostile/vm.rs`, which is
  precisely what a leak would be called. Unlike the other text guards here,
  sibling `*_test.rs` files are **not** exempt — those guards are about
  production behaviour, this one is about which code may hold a vendor type.
- **`winit` is nameable in `crates/mc-client/src/events.rs` alone**,
  pinned by a source scan whose filter was mutation-checked rather than
  assumed. This is one reason `egui-winit` is not taken (ADR-017): it would
  name `winit` in a second `mc-client` file and fail the build over it.
- **No shipped HUD element name or declared HUD colour appears in production
  Rust**, and **this scan's watch list is derived from
  `content/base/hud/` at test time** rather than hand-copied — so deleting a
  content file cannot silently stop the scan watching its name, which is the
  hole that made the hand-maintained retired list next door necessary. It
  carries the same two controls as its neighbour (a fixture source naming a
  shipped element is reported; a fixture naming a shipped colour as a byte
  quadruple is reported) plus two vacuity guards the derivation makes
  necessary: reading zero production sources fails, and an **empty derived
  watch list** fails rather than passing an assertion it can no longer
  falsify. The nine anchor names are deliberately **not** watched — they are
  the engine's own vocabulary and must appear in Rust, and adding them would
  force an exemption for the file that defines them, which is the thing this
  guard exists to avoid needing.

  **Deriving the list closes the deletion hole and cannot close the rename
  one — measured, not reasoned about.** Renaming a shipped element in content
  and writing the *old* name into a production Rust source leaves the whole
  workspace green, because the retired list is empty and empty is the honest
  state: an entry belongs there once a name has meant something to content and
  stopped. **Nothing mechanical prompts that entry**, so the day an element is
  renamed, adding its old name is part of that commit. One further limit,
  easy to misread as covered: the colour needles are derived by formatting the
  parsed colour, so the hex needle is upper case and a lower-case Rust literal
  would not be caught — which is why the control is written against the byte
  quadruple, the form a Rust constant far more readily takes.
- **Every element `content/base/hud/` ships states a contrast outline**,
  asserted over the **parsed** field rather than over the file's text: two of
  the three shipped files also mention *outline* in a prose comment, so a text
  scan would have stayed green under exactly the mutation this check exists to
  catch — delete the field, keep the sentence explaining why it matters. The
  element list is derived from the directory, so a fourth shipped element is
  covered with no Rust edit.
- **The client's frame path names no HUD drawing other than the one
  `record_frame` call site**, and **no production source under
  `crates/mc-client/src` or `crates/mc-render/src` names `Instant` or
  `SystemTime`** except `mc-render/src/overlay/clock.rs`. Both scans carry a
  positive control and a reads-zero-files refusal, and both were **verified
  non-vacuous on day one**: the needles appeared in zero production sources
  before the feature landed, so neither guard was born red and tuned around a
  hit. The wall-clock scan's exemption is the only hit it will ever have —
  which is exactly the shape of guard that goes green forever the day the
  thing it watched is quietly removed, hence the controls.

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
