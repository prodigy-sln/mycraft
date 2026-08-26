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

## The stable byte fold lives in `mc-core`, and only the fold does

`mc_core::hash::fnv_1a_64` is FNV-1a-64 over a byte slice, hand-written and
deliberately not the standard library's default hasher — that algorithm is
documented as unspecified and may change between compiler releases, and a hash
that moves with the toolchain invalidates every save on an upgrade.

That same property is why it sits in `mc-core` rather than beside its first
caller. Two programs have to fold identically and forever: the save format in
`mc-world`, which records a behaviour hash and an appearance hash per block
declaration (`docs/technical/world-format.md`), and `voxforge`, the art build
under `tools/`, whose texture-set index must fold to the value the client
computes when it reads the same bytes. `tools/` may depend inward on `crates/`
and the reverse never holds — an invariant a test enforces (see "Mechanically
enforced invariants"), so `tools/voxforge` cannot host the shared copy. `mc-core`
is the one place both sides already reach.

**The failure this arrangement exists to make unspellable is not a hash that is
wrong. It is two hashes that were each computed correctly and do not match** —
an index a build declares current and a client declares stale, with no error
anywhere and nothing to read. A second implementation on either side of the
`crates/`–`tools/` line reintroduces exactly that, which is why the rule is that
there is one, in `mc-core`, and everything else calls it.

**Only the fold moved, and two things that look like it deliberately stayed
put.** `folded()` and `DefinitionHash` remain in `mc_world::persistence::format`,
because `folded` names `postcard` and that crate's manifest confines `postcard`
to `mc-world/src/persistence/`. And `mc_render::texture::placeholder` keeps its
own inlined copy of the same two constants: it hashes a block name to a debug
colour, which is not a value two programs must agree on, and folding it in would
change golden frames for no correctness gain. A future change may consolidate it,
but it is not the same obligation and nothing breaks if it never does.

## The texture-set index, and the byte sequence its fold runs over

`mc_core::art` is the whole of the agreement between the art build and the client:
a `parse`/`rendered` pair over `&str`/`String`, plus `folded_sources`, plus the
rule for what a set's image may be named. **It opens no file.** The writer renders
a `String` and writes it; the reader reads bytes and parses them; both hand the
bytes they read to the fold. That is what keeps `mc-core` free of I/O while still
being the only place the agreement is written down, and it is why `toml` is not
reachable from here — the index is a line format, not a document.

An index is text, one record to a line:

```text
mycraft-texture-set 1
fold 00008f14e45fceea
source models/grass-block.mcvox
source materials/dirt.toml
key base__grass_top.png base:grass_top
```

Three things about that shape are load-bearing and a future change must not break
any of them.

- **The fold is padded to sixteen hex digits.** A value with a high zero byte
  written unpadded is a shorter line a strict reader refuses, and the value it
  would parse as is a different one.
- **A `key` record is written image first, key last.** A `TextureKey` may contain
  whitespace and an image file name may not (`is_an_ordinary_image_name`), so only
  one of the two can be the rest of the line and it has to be the one with no
  character set imposed on it.
- **Any ASCII control character in a key or a source path is refused on both
  sides, render and parse.** A key has no character set, so
  `base:a\nfold 0000000000000000` is a spellable manifest entry whose rendered
  index a reader would otherwise accept with a fold nobody folded. Refusing only
  on parse leaves a writer that can emit the forgery; refusing only on render
  leaves a reader that believes one.

The fold's byte sequence is stated rather than inherited, so that a test can build
an oracle sharing no code with either side: for each source in order, the recorded
path as UTF-8 bytes preceded by its length as a little-endian `u64`, then the
file's bytes preceded by theirs; FNV-1a-64 over the concatenation. **Length
prefixes rather than separators**, so a file holding whatever byte a separator
used cannot forge a boundary — the source `ab` holding nothing and the source `a`
holding `b` would otherwise fold over the same two bytes.

**Recorded paths are relative to the manifest's own directory and `/`-separated.**
That is what lets a copied content root re-fold to the same value somewhere else
on disk, which every fixture in the build's test suite depends on; absolute paths
would make a copied root permanently stale and would put developer home
directories into a file the gate builds.

## `voxforge build` in the topology

`voxforge` gained a fourth subcommand: `build <manifest>` reads a texture manifest
(`content/base/textures.toml`), bakes the faces it names, and writes the images and
the index into the manifest's `output` directory. It is the first thing on the
non-committed side of ADR-026 — deterministic and free to reproduce, so the tree
carries the models and the manifest and regenerates the images.

Three shape decisions the as-built record needs to keep:

- **Entries are grouped by model, and each model is emitted whole.** Not an
  optimisation. The cubic precondition — a face set is a block's six faces — lives
  in `emit`'s whole-set arm only, so a per-entry emission of one face would never
  ask it and a model that is not a cube would bake a "block texture set" that is
  not one. Six faces from one load and one render pass is the side benefit.
- **The seam verdict binds only on the faces some entry selected.** Every face is
  rendered and judged, and a failing verdict on a face no entry asked for is passed
  over: refusing on one would refuse a set for a picture nobody is going to draw,
  and no positive scenario would notice.
- **The cache key is the fold, and it is whole-set.** Matching value plus every
  image the index names present means nothing is opened; anything else rebuilds
  the whole set. Per-entry caching would need a second, finer-grained record that
  every reader of the index would then also have to understand, for seven 16x16
  images. The images are checked for presence and not content, so a hand-edited
  image survives a build — the stated consequence of a whole-set key.

Everything a refusal can be about is settled before the first file is opened, which
is what makes a build refused on its fourth entry leave the previous set intact.
That property is what P6's gate stage will lean on: a refused art build leaves a
stale set on disk, so the gate must not then test against it.

## The client's verdict on a built set

`mc_client::textures` is the reader half of the agreement above, and the whole of
what a launch does about art. Both launch doors ask it one question before a world
is generated, and either start or turn the run away with one sentence.

```rust
pub fn built_set(root: &Path) -> Result<(SetVerdict, SuppliedTexels), TextureSetError>;
pub fn refusal_for(verdict: &SetVerdict) -> Option<PreparationError>;
```

**The verdict is a total enumeration returned in `Ok`, never an error and never an
absence check.** `SetVerdict` has six arms — `NoArtDeclared`, `Absent`,
`StaleAgainstSources`, `SourceMissing { source }`, `ImageMissing { key, image }`,
`Current` — and every test that reads one compares the whole of it. That is the
property a future change must not give up, and the reasoning is the one this
project has now paid for twice: `assert!(nothing_was_refused)` cannot tell a
healthy set from a client that has lost the ability to check, because both say
nothing. `assert_eq!(verdict, Current)` rejects every other arm *including* the
ones that mean "I could not look", so a check that stops checking reddens for
free.

Returning it only as an error would undo that. Three of the six arms let a launch
continue, so they would be unconstructible in `Ok` and the totality the suite
holds would not be the one the reasoning claims.

**`refusal_for` lives beside the enum it is total over**, in `textures/mod.rs`
rather than in `startup.rs`. Adding an arm and forgetting to say what it means is
then a non-exhaustive match in the file that declared it, rather than a silent
`None` in another one.

### Two arms let a launch through, and they are not the same thing

`NoArtDeclared` is separated from `Absent` by the presence of `textures.toml` and
by nothing else. Applied literally to four verdicts, a content root with no
manifest at all has an absent index and would be told to run the art build before
content that declares no art will load — which blames the wrong party, and every
mod author's first root is exactly that shape.

`Current` while covering no key a block declares is the other one. A key the set
does not cover is not a refusal at any point: it costs a generated texture, not a
launch. The two messages — *the build step was not run* and *this key was never
authored* — must never collapse into one, and a test holds them apart.

### The sources are re-folded as the index recorded them

The client reads the index's recorded source list, in the recorded order, resolves
each relative path against the content root it was given, and folds those bytes
again. **It never reads the manifest.** Two independent derivations of one source
list agree on the shipped tree and part company the first time a build changes
what it reaches — the drift `mc_core::art` exists to make unspellable, one level
up. The witness is a test that drops a `materials/*.toml` the index never
recorded: a client re-deriving its list from the manifest folds it and calls the
set stale; one re-folding the recorded list cannot see it.

Resolving against the root the client was given, rather than against anything
absolute, is what keeps every `copy_tree`'d fixture in this workspace current.

### The order of the questions is the order of the answers

A root that declares no art is never stale. A set that was never built has no
sources to check. A set whose sources have moved is not judged on images it may
not have baked. And an image *name* is checked against
`mc_core::art::is_an_ordinary_image_name` **before** anything looks for the file,
because looking means joining that name onto a path first — this is the reader
half of the rule the build applies when it derives the name, and this client is
that function's first caller inside `crates/`.

### `<root>/textures/` is hard-coded, and that is a known dead end

A manifest states its own `output`; the client does not read the manifest, so it
looks under `textures/` and nothing tells it otherwise. A root whose manifest says
`output = "art"` therefore builds cleanly and is judged `Absent` at every launch,
told to run the build it has just run, with nothing in either message naming
`output`.

That is stated plainly in `modding/voxel-models.md` rather than left to be
discovered. Closing it means recording the output directory *in the index*, which
changes a format two programs share and belongs to the spec that makes that
change.

### `TextureSetError` is the other axis, and one of its arms was added late

`SetVerdict` answers "what is the state of this set". `TextureSetError` is a set
that admits no answer: `Unreadable { path, cause }`, `Index { path, cause }`, and
`UnusableImageName { key, image }`.

The third was not in the architecture plan and is not a verdict, and it is here
because the client takes an image name **out of a file** and joins it onto a path.
`TextureSetIndex::parse` accepts `elsewhere/base__stone.png` — relative,
`/`-separated, naming no parent — and it is still not an image name. Without this
arm, a set built by an older or a patched tool is a set whose index the client
believes.

`Unreadable` carries what the filesystem said as a `#[source]` and never inside
its own message. A message interpolating its cause reports one flattened sentence
and drops whatever sits under it, which is the defect `tests/reporting_seam.rs`
exists over — and that guard caught this variant's first spelling.

### Five `PreparationError` variants, and the field that could not be called `source`

`PreparationError` gains `TextureSetAbsent`, `TextureSetStale`,
`TextureSetSourceMissing { missing }`, `TextureSetImageMissing { key, image }` and
`TextureSetUnreadable(#[from] TextureSetError)`.

Five and not four: an `Option<PathBuf>` cannot be rendered conditionally inside
one `thiserror` format string, so the stale case and the missing-source case are
separate variants rather than one with a hole in its sentence.

The field is `missing` and not `source`. `thiserror` reads any field named
`source` as the error's cause, and the variant does not compile with it — *the
method `as_dyn_error` exists for reference `&PathBuf`, but its trait bounds were
not satisfied*. The `Display` text is unaffected.

`BUILD_THE_TEXTURE_SET` is a `pub const` beside `REFUSE_CHANGED_BLOCKS` in
`startup.rs`, spelled once, for the reason that constant carries: a message
quoting a command nothing accepts reads as a way out and is not one. It is `pub`
so that the four places which must agree — the constant, `README.md`,
`modding/voxel-models.md`, and the test that compares them — can be compared
against each other rather than against a second copy of the string.

### The texels are decoded once, at the composition root, and held for the run

`built_set` hands back a `SuppliedTexels` alongside the verdict, and it is filled:
one entry per key the index names, holding that image's level-zero texels in
`[R, G, B, A]` stored bytes. **Only a `Current` set is decoded** — every other
verdict either refuses the launch or says there is no art, so decoding first
would report a broken image out of a set the client was about to refuse for a
reason its author can act on.

**The decode is the client's and the renderer may not acquire one.** `mc-render`
names no `std::fs`, no `PathBuf` and no image decoder anywhere in `src/`; it is
*handed* level-zero texels and computes the mip chain from them.
`crates/mc-client/src/textures/decode.rs` is the one file of this workspace's
composition root that may name `image::`, and
`crates/mc-client/tests/the_decode_stays_at_the_composition_root.rs` holds both
halves with a positive control on each scan. Pixels are the client's side of the
snapshot split: a server that needed them would break the asymmetry that makes a
texture pack a legal client modification and a block declaration not.

**Two doors read a set and there are exactly two, one per launch shape.**
`startup::prepare_scene` — the capture path every golden is shot through — asks
for the root it is given. `launch::start` — the player path — asks before the
window opens and before the preparation worker is spawned, so a contributor who
has not run the art build reads one sentence instead of waiting out a world
nobody will show them. Each shape reads once; neither reads twice.

**The supply is given to the renderer at construction and is held for the whole
run.** `launch::Starting` carries it from the composition root into
`App::new`, which hands it to `FrameRenderer::new` inside a `gpu::TerrainTextures`
alongside the sampler request. It is deliberately **not** carried by `Unuploaded`
or by the re-mesh worker's retirement: the built set is a pre-build artefact that
does not change while the client runs, so a reload appending a key finds either
art that was already read or no art at all — and the second is the ordinary
per-key fallback reached by a second road. Threading a supply through the reload
path would create a value that can arrive *empty*, and a world drawing its baked
art would go back to hash-derived colours the moment somebody saved a block file.
`crates/mc-client/tests/a_reload_keeps_the_supply_the_renderer_was_built_with.rs`
is the only guard on that, and it needs one renderer living through two uploads
to be one at all.

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

- `mc_world::content::LuauFileDefinitionSource` — reads a content root's
  `blocks/*.luau` files, evaluating each through `ScriptHost`. The production
  loader.
- `mc_core::block::source::InMemoryDefinitionSource` — definitions held
  directly, in the order given. This is **production code**, not a test
  fixture: because no public `register` exists, it is the only
  programmatic way to build a registry at all, and its existence is what
  makes the port a real seam rather than an asserted one.

**The loader implementation was swapped from TOML to Luau through this port and
the registry was untouched** — which is the claim the port existed to make good,
now measured rather than asserted. The port is shaped around the domain need
("hand me the definitions this source declares, and tell me where each one came
from"), not around any format's API: no `Path`, `File`, `PathBuf`, `toml::` or
`mlua::` type appears anywhere in `mc-core`. An origin (`DefinitionOrigin`) is an
opaque, human-readable label — it wraps a plain `String` — so a file path and a
chunk name are equally expressible, and `mc-core` never learns what kind of thing
produced either.

**The strongest evidence the swap changed nothing it should not: a world saved
against the TOML declarations loads against the Luau ones reporting no block as
missing, changed or retextured.** `persistence/format.rs` folds a definition into
two hashes, `DeclaredBehaviour` and `DeclaredAppearance` — the field lists and
their revision bytes are stated once in `technical/world-format.md` and have both
grown since — and **deliberately excludes `origin`** from both, so a save does not
depend on the path a definition was read from. That exclusion is what makes the
comparison possible,
and it is the one instrument in the swap that compares a whole resolved
definition against an oracle computed before the swap existed.

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
- **A frame advances the world by the time it took, once the world is ready.**
  The spawn is derived from the world, and the world is generated on the
  preparation worker several frames after the window opens, so there is no
  simulation to advance until it lands. `App::present` calls
  `Session::advance_frame(&clock)` once per presented frame; the
  `Option<Simulation>` guard, the drain and the advance live inside
  `Session`'s private per-tick step (`crates/mc-client/src/session/mod.rs`,
  §"The client input dispatch" below), not in the frame path, so the guard is
  reachable by a test that opens no window. Before the world lands the frame
  is the clear colour and no tick changes anything — a player advanced during
  preparation would spend the load falling. Pending input is drained inside
  that same guard and nowhere else, so a frame that draws nothing leaves the
  accumulated motion where it is rather than discarding it.

  **This used to be one tick per rendered frame, and that was the defect
  PRO-971 fixed.** A tick is a declared sixtieth of a second, so delivering one
  per presented frame ran the world at `frames_per_second / 60` times real
  speed: right by coincidence on a 60 Hz display and 2.4× fast on a 144 Hz one,
  which a player reported as warping around with super speed. The ratio was not
  a regression — `mc-sim` recorded it as a stated cost of a scripted replay with
  no player in it, and it became a movement defect the moment player control
  landed. The conversion that was missing is §"Pacing the frame" below.
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

**Physics reads the world through `Solidity`, resolved once — and the walk a swing travels reads
`Targetable`, resolved beside it.** `mc_sim::player::Solidity::is_solid(BlockPos) -> bool` and
`mc_sim::player::Targetable::is_targetable(BlockPos) -> bool` are both total — outside the loaded
world, below `y = 0` and every negative coordinate all answer `false` — so a caller has nothing to
handle and nothing to swallow.

**Narrow traits where the consumers differ.** `Solidity` and `Targetable` are two traits because
collision reads one at nine sites and the walk a swing travels reads the other, and neither set may
reach the other's question — a single trait carrying both would hand every one of those sites a
question it must never ask, and a collision test could then exercise aiming by accident. Where one
site reads two properties of one question, they travel in one value: `Medium` returns
`VoxelMedium { swimmable, resistance }`, because splitting it would segregate nothing —
`advance_player` reads both, one line apart, from one fold over one box — while admitting a fixture
that states one and inherits the other. The composite `Traversal: Solidity + Medium` is where the
physics' exclusion of `Targetable` is stated.

`ResolvedVoxels::resolve(volume: &dyn BlockVolume, registry: &BlockRegistry) ->
Result<Self, RegistryError>` builds **three** per-voxel arrays once, over the volume's declared
extent, resolving every name it finds through the registry — each answer read from its own declared
field, never derived from another. Two are one bit wide, and each costs
1 048 576 voxels × 1 bit = **+128 KiB** once, at world scale. The third is an **index** into a table
of the distinct `(swimmable, move_resistance)` answers the *registry* declares, and its width is
chosen once at resolve from `{1, 2, 4, 8, 16, 32}` — a power of two that divides 64, so a read stays
one shift and one mask, with a floor of one bit. The shipped registry declares at most two distinct
media between all its blocks, so that index is one bit and the same 128 KiB again, against a table of
a handful of entries. The width is a property of **content**: any number of blocks sharing one answer
costs nothing, a third distinct answer takes it to 2 bits, and a fifth to 4. It takes a `BlockVolume` rather than
`&ReplayWorld` concretely, because the only way to state "an invented block whose definition
disagrees with its name" is a volume the replay world is merely one implementor of — `ReplayWorld`
implements it, and a test fixture can too. This keeps solidity a bounds test and a bit test with no
failure mode to swallow, and keeps the physics untied to the replay fixture a real chunk store
replaces.

`BlockVolume::block_at` returns `Option<Contents<&BlockName>>`, and the two negative answers stay
separate all the way down: the `Option` says the volume reaches this position, and the `Contents`
says whether anything is in it (`technical/world-format.md`). `ResolvedVoxels::resolve` then answers
"nothing" for both, in all three views — a position the volume does not reach and a cell holding
nothing are alike neither solid nor targetable, and both are `VoxelMedium::NOTHING` — and this is the
one place in the codebase where those readings genuinely **coincide in outcome**. The coincidence is
now three-way. That is worth naming rather than leaving implicit: no assertion on the resolved view,
and no independent overlap oracle, can tell a defect that confuses them apart from correct code
here. Everywhere else the split is observable; on this path it is held by the arms being written
separately and by review — **and that applies to the medium arm too**, which is worth saying out loud
because it is the newest of the three and the least exercised.

**The simulation cannot exist before the world does.**
`mc_sim::replay::simulation_for(world: &ReplayWorld, registry: Arc<BlockRegistry>, content:
PublishedContent) -> Result<Seated, SpawnError>` resolves `ResolvedVoxels`, derives the spawn and
seats the player; the client calls it and decides nothing. It hands back a `Seated` rather than a
`Simulation` because every door into a world now passes through one seating function and carries
that function's verdict out with it — see "The seating door" below. `PreparedScene` carries `world` and
`registry` alongside the mesh, so the golden, probe and determinism suites build their simulation
from the same preparation their frames are packed by; the composition root builds its own from
`PreparedLaunch`, which carries the `Simulation` itself rather than a world to derive one from,
because on a resume there is no `ReplayWorld` in the process at all. `PreparationError::Spawn` is
the variant that carries a refused `SpawnError` across the crate boundary. Because the world is generated on the preparation worker several frames after the window
opens, `collect_preparation` hands the constructed `Simulation` to `Session::attach_simulation` once
the scene reaches `ScenePhase::Ready` — the phase lives in `App`, the simulation lives in `Session`,
and the invariant that one is `Some` exactly when the other is `Ready` is held jointly by that one
function rather than structurally by either type alone. `App::present`'s advance does something only once the
world has landed — however many ticks the frame's elapsed time buys, once the world is ready, not
before (§"Pacing the frame").

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
Before this feature, `Simulation` held `Box<dyn Solidity + Send>` over a resolved solidity bitset
built once at construction, and no world type in the workspace had a mutating method at all.
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
`VoxelWorld`, a `ResolvedVoxels` — itself two bitsets, one for what stops the player and one for what
a ray may stop at — a dirty-section set and an `Arc<BlockRegistry>`, and keeps all four fields
private. It carries **no** name for empty space: `World::new(VoxelWorld, Arc<BlockRegistry>)`
takes two parameters and there is no accessor handing one out, because a cell holds a block or
nothing and nothing has no name to hand out (`technical/world-format.md`). Physics is unaffected: `advance_player` and the `collide` module still take `&dyn
Solidity`, so the cheap `Chamber`/`Ground` collision fixtures are untouched and none of the exact-
position collision scenarios changed. What changed is only what *feeds* solidity — a `World` now,
where a bespoke test double used to stand in.

**One private function writes all three views, and nothing else may.**

```rust
// crates/mc-sim/src/world/mod.rs — no `pub`, visible in this module and its
// descendants (mc_sim::world::action) and nowhere else in the crate.
fn write(&mut self, at: WorldPos, contents: Contents<&BlockName>) -> Result<(), WorldError> {
    let answers = match contents {                        // settled once, before any write
        Contents::Empty => VoxelAnswers::NOTHING,         // emptying needs no registry at all
        Contents::Holds(block) => {
            let declared = self.registry.resolve(block)?;
            VoxelAnswers {
                solid: declared.is_solid,
                targetable: declared.targetable,
                medium: self.resolved.medium_index_of(declared),  // minted by the table that holds it
            }
        }
    };
    // ... the store, from the same `contents` ...
    self.resolved.set(at, answers);                       // all three resolved views, together
    self.mark_dirty(at);                                  // remesh bookkeeping, see below
    Ok(())
}
```

The block store and the three resolved views cannot fall out of step because there is exactly one
function that can write any of them, and it writes them together or not at all. **`set` takes every
answer in one call for that reason**: a caller able to write one without the others is the
disagreement the type exists to make unspellable. It takes them as one `VoxelAnswers` whose fields
are named at the call site, so a call still says which answer is which.

**The medium is the one of the three that is a minted index rather than a value, and that is what
keeps `set` total.** Every `bool` is writable, but a medium *value* is not: writing one means finding
it in a table built at resolve time, and `set` is `pub`. Handed a value no registry produced, an
implementation could only fall back silently, panic on a write path, or widen the packing under an
edit — so `MediumIndex` has no public constructor and the table's owner mints every legal value.
Unspellable rather than checked, which is the same standard a re-mesh batch is held to.

**This is what makes what a swing can find follow an edit rather than only a load.** A targetability
view built when the world was loaded and never written again answers every question about a
*declared* world correctly and every question about an edited one wrongly — and nothing that writes a
cell and then reads that same cell back can tell the two apart. `break_at` and `place_at` —
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
physics' own resolved `ResolvedVoxels` — deliberately, so that an adapter bug in the resolved bitset
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

**The raycast asks `Targetable` and never `Solidity`.** `targeted(origin, direction, reach, world) ->
Option<Hit>` stops at the first voxel whose block declares a ray may stop there. What stops a player
and what a swing can find are separate declarations — `base:water` stops nobody and a swing finds it
— so reading solidity here would put back a game rule content cannot override, at the one site every
action's target comes from. The nine call sites that read `Solidity` all mean collision, and none of
them is this one.

**The reach bound is a single site.** The walk goes in ascending entry distance and stops as soon as
the next voxel's entry distance exceeds `REACH` (5.0 blocks) — there is no second, separate
`distance <= REACH` check anywhere else. This is not a style preference: `Targetable` is **total**
(it answers `false` for every position outside the loaded world), so an unbounded traversal plus a
separate distance check would never terminate for a ray that hits nothing — the traversal has no
other reason to stop. A reach bound spelled as a second, independent comparison is therefore not just
redundant, it is a latent hang waiting for the one ray that never meets a targetable voxel. The
traversal additionally reports the entry face, which is what a placement's target cell is computed
from.

**A placement lands one step back, except into a cell content declares `replaceable`.** Ordinarily
the cell is the one the ray occupied immediately before the hit, so a block goes on the near side of
what you are looking at. A hit cell holding something `replaceable` is built *into* instead — and it
has to be, because otherwise no ray could land a block in such a cell at all: had the cell one step
back held something replaceable, the walk would have stopped *there*. The rule is stated by the
declaration and knows no block name; `base:water` is the only shipped block that reaches it, and it
could not before water declared `targetable`. **Choosing a different cell changes which cell the
later checks read and nothing about whether they run** — a placement aimed at a replaceable cell the
player is standing in is still `InsidePlayer`.

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
| `NoTarget` | the raycast finds no voxel within reach that a block declares a ray may stop at — this also covers "something is there but too far away": the reach bound is one site, so a target beyond it and no target at all are indistinguishable without an unbounded search, and none is done |
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

**`Indestructible` is reachable for the shipped content, and aiming being its own declaration is
why.** It is decided against the *targeted* block, and `targeted` returns a hit where
`is_targetable` answers true. `base:water` declares `targetable = true` beside `breakable = false`,
so a swing arrives at a water cell, `broken` is called on it, and the player is refused with the
water left where it was.

**It was unreachable until `solid` was split, and the reason is worth keeping.** While the walk
stopped at the first *solid* cell, water's `breakable = false` refused nothing — a swing went
straight through the water and emptied whatever stood behind it. Water was not broken because it
could not be aimed at, which is a different fact from being protected, and the declaration said what
water was without ever stopping anything. **Aiming and yielding are two claims and the first is what
makes the second reachable**: a block declaring `targetable = false` puts `breakable` back in that
inert state whatever it says, and a mod can do exactly that.

`crates/mc-sim/tests/shipped_water_is_not_broken_and_is_built_through.rs` holds both halves — the
swing stopping at the water rather than at the stone behind it, and the refusal naming
`Indestructible` with the cell read back by name. It carried a fuse recording the debt while the
variant was unreachable; the fuse has been blown and the scenario it asked for is what replaced it.

**Three arms and never two, wherever both wrappers survive.** `broken`, `placed` and `overwritable`
each read the world through `Option<Contents<&BlockName>>`, and each writes `None`,
`Some(Contents::Empty)` and `Some(Contents::Holds(..))` as separate arms — a `let
Some(Contents::Holds(..)) = .. else` would answer "there is no such cell" and "this cell holds
nothing" with one refusal. Both readings reach the same permitting or refusing outcome at every one
of those sites today, which is precisely what would make the collapse invisible: the arms are
separate because the facts are separate, not because the outcomes differ. `broken`'s empty arm is
unreachable in practice — the raycast stops only where a block declares a ray may, and an empty cell
holds no block to declare anything — and it is written out anyway, so
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

**Texture layer assignment is not re-resolved on a remesh, and it is no longer derived by whoever
draws at all.** The assignment is **stated** by the simulation when it reads the content root and
**honoured** by the client — see §"The layer assignment is stated, not derived" below for why that
direction is the whole point. It covers every block the content registers, never the meshed world's
quads (`technical/rendering.md` §"Textures are array layers, never an atlas"), so an edit or a
resumed save can add a block a world had none of without shifting anything already assigned.
Re-resolving the *assignment* on every remesh is unnecessary rather than deferred: nothing about
which blocks are registered changes mid-session. The failure mode if a quad ever names a key the
assignment does not cover is a loud error at packing time (`GeometryError::UnresolvedTexture`,
naming the block, the facing and the key) rather than a wrong texture.

**Which key a face draws is resolved at packing time, out of the block's declaration.** `layer_for`
(`crates/mc-render/src/geometry/mod.rs`) and `hud::held::held_swatch`
(`crates/mc-render/src/hud/held.rs`) both take a `TextureResolution` — every registered block's six
declared keys beside the assignment — and neither parses a block's name. A declaration whose
`texture` differs from its `name`, and a table naming six different keys, therefore draw what they
declare. **The layer index remains the only content-derived value inside a packed vertex**: a vertex
is three section-local coordinates, a facing, the layer, and a scene section index assigned at
assembly. A `Quad` carries no resolved key and must not gain one —
`technical/rendering.md` §"A face draws what its block declared, and a `Quad` carries no key" says
why.

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
`simulation_at_launch(save: &Path, launching: Launching) -> Result<Seated, LaunchError>`, where
`Launching` bundles the seed, the registry, the published content and the acceptance. It takes the
**seed** rather than a world built from it, and generates one only in the arm where there is no save to resume — a caller handing
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
which world is needed before doing any work for it — `launch::simulation_to_play(save, launching)
-> Result<(Seated, BlockName), PreparationError>` calls `mc_sim::persistence::simulation_at_launch`
first, and only then meshes, resolves layers and packs — so a resume generates nothing at all,
rather than generating a world, meshing it and discarding both once a save turns up. The `Seated`
it returns is what carries the entry clearing's verdict up to the client; `simulation_to_play`
itself decides only which block a player holds.

Two of anything on the golden path is how images drift, so the split is answered structurally rather
than by care: both doors share one mesher (`mesh_world`) and one packer (`scene_of`), and both build
their `ContentView` from the same stated assignment rather than each calling one shared resolver — so
the only thing that can differ between them is which world's blocks went in. `layers_of` used to be
that shared resolver and is gone: the property its doc comment recorded, that the geometry a player is
handed and the geometry a golden is shot from cannot differ, is now true by construction instead of by
both paths calling one function. That equality is asserted byte for byte, over both the section table
and the packed vertices, by `crates/mc-client/tests/launch_and_capture_agree.rs`.

**`mc-client` gains wiring and one ending translation, and no policy.** `launch::save_path()` names
where the save lives (relative to the working directory, mirroring `CONTENT_ROOT`'s own convention,
and deliberately not checked for existence — a missing save is the no-save case, not a failure);
`startup::acceptance_from(args)` parses the one command-line flag MVP 1's human channel needs
(`--refuse-changed-blocks`, `docs/user/gameplay.md`) and does nothing else. **The argument asks for
the stricter answer and its absence is the accepting one**, which is the reverse of the flag it
replaced: a save whose blocks merely behave differently is loadable data, and refusing it turned a
content update into a world nobody could open. Which blocks moved is *said* by
`notice::say_changed_blocks`, called from `launch::played_and_reported` — so `acceptance_from` still
does nothing but parse, and the sentence lives beside the other notices rather than inside the
parse. `Session::save` is a
three-line forward to `mc_sim::persistence::save`. The one piece of client-side logic that is more
than wiring is `ending_after_saving`, in `session.rs` and not in `mc-sim` because it returns
`mc_render::window::Ending` and `mc-sim` may not name `mc-render` — it decides that only a run that
ended by closing normally (`Ending::Closed`) saves at all, so that a device-lost or otherwise broken
run can never overwrite a good save with a half-built one, and that a failed save on a clean close
becomes a failed ending naming the path and the reason rather than a silent one. Nothing about
*which* world to play, *whether* a changed-block save should load, or *how* a refusal is worded
lives in `mc-client` — those answers all come from `mc-sim` and `mc-world`, unchanged by which crate
happens to call them.

## The seating door: one way a player enters a world, and it says what it did

`mc_sim::simulation::seat(spawn, world, content) -> Seated` is the only way to put a player into a
world. It runs the clearing search first, moves the player if their box covers a solid cell, builds
the `Simulation`, and hands back `Seated { simulation, clearing }`.

**The door is the constructor, not the launch path, and that is the whole design.**
`Simulation::new` is module-private — no `pub`, no `pub(crate)` — so `seat` is the only thing in the
workspace that can construct a `Simulation` at all. Every way into a world already passed through
that constructor: a resume, a first launch into a generated world, a golden capture, every fixture
in `crates/mc-sim/tests` and `crates/mc-client/tests`. Closing it makes "no player enters a world
unchecked" a property the compiler holds. The alternative considered and refused was a call added
to `simulation_at_launch`: that can be deleted with the whole suite green, and it would never have
covered the generated-world door in the first place.

**What the compiler holds, and what it does not.** It holds against *every* caller, `mc-sim`'s own
modules included, because `new` is private to the `simulation` module rather than to the crate.
What it cannot hold is a second seating path added **inside that module** — a `pub fn` next to
`seat` building the struct by literal. That is why `crates/mc-sim/tests/one_way_seats_a_player.rs`
exists: it reads the crate's own sources and answers an enumerated verdict —
`OneWaySeatsAPlayerAndItReportsItsClearing`, `AnotherSourceSeatsAPlayer(sites)`,
`TheDoorNoLongerSeatsAPlayer(spellings)` or `NoSourceWasRead` — so a scan that has stopped being
able to look never reads as a clean crate.

**The residual hole is narrower than the scan and is worth naming exactly**: a second `pub fn` that
*calls* `seat` and discards the `Clearing`. The rule still runs — the player is still moved — and
only the reporting is lost. Nothing detects that, and the reason it is tolerable is that the thing
which must not be skippable is the search rather than the sentence.

### The rule is factored so a join inherits it, and one mistake stays invisible

MVP 3's networked join adds a player to an already-running simulation, so it will not call a
constructor. It inherits the *rule* rather than the function: it calls
`clearing::clear_the_player(&mut joining, &self.world, self.world.extent())` and gets the same
`Clearing` back that entry and the reload swap get. No join API, trait or admission abstraction is
built for it here.

**The omission a join cannot make is forgetting the rule; the mistake it can still make is
supplying the wrong ground.** `clear_the_player` takes the extent the search may consider as an
argument, because a cell past it is *unknown* rather than clear and `is_solid` cannot say so — and
an extent that is too large lets a player be moved off the map, while one that is too small rejects
destinations that were fine. Nothing in this design makes a wrong extent visible. **Whichever spec
adds the join owes it a scenario**; it is not buildable here, because there is no join to grade.

**The ground is an argument rather than something the rule reads for itself, and there is a stated
condition for changing that.** Reading `world.extent()` inside `clear_the_player` would make the
wrong-ground mistake unspellable — the expression would appear on one line in the whole crate and no
future caller could pass a different one. It was refused for a reason that expires: the two
scenarios that control the argument grade it *at the call site*, one from each direction — a
shrunken extent rejects the exact destination the one-cell-sideways scenario asserts, and an
over-large one lets a near-edge player be moved off the map — so reading the extent inside would
leave both of them with almost nothing left to catch. That reasoning holds while there are two
callers, and it stops holding at a third: three call sites each spelling `world.extent()` is a rule
with three copies, which is the condition the rejected option was right for. **Revisit the factoring
when a third caller appears** — MVP 3's join is the candidate.

### The search runs at every entry, and the alternative was refused on coupling

Entry does not ask whether the save reported changed blocks. Gating on it would mean letting
`mc-sim` ask persistence whether to ask the physics a question — persistence coupled to physics for
a saving of almost nothing, since `cleared` opens with an `overlaps_solid` early return and a clear
player therefore costs only the cells their own box covers (two, where the box lies inside one cell
column). The 2 601-candidate ring is walked only by a player who is genuinely trapped. `changed` is
also not the only way to be trapped: a hand-written save, or a launch over a save whose blocks all
still match, reaches the same state.

**The coupling refusal stands; the premise it rested on does not.** `LoadedWorld` no longer carries
only `{ world, player }` — it carries `changed` as well, the ascending list of names whose declared
behaviour moved, so the verdict `load_world` computes is reported instead of dropped. That widening
adds no physics dependency: a list of names travelling out to be printed is not persistence asking
about solidity, which is the thing the decision above refused. **What is forbidden is unchanged and
is what `crates/mc-client/tests/entry_clears_whatever_the_load_reported.rs` reads for**: nothing may
branch the clearing search on that list. `seat` asks the physics its question unconditionally, and
the cheap implementation is now *spellable* where it previously was not — which is why that file
matters more than it did, not less.

### Where the verdict is parked, and why once-ness is structural

The verdict rides out of `mc-sim` on `Seated`, through `simulation_to_play`, onto
`PreparedLaunch::clearing`, and is read exactly once in `App::collect_preparation`, which calls
`notice::say_entering(prepared.clearing)`. There is no dedup field and none is wanted: the
preparation handle is `take`n, the `PreparedLaunch` is consumed, and there is one entry per process
run — so "said once" is a property of the construction rather than of a flag. The shape that would
break it is a `Clearing` parked on long-lived client state and read by the frame path, which is
what `crates/mc-client/tests/the_entry_sentence_is_said_once.rs` watches for; `src/session/reload.rs`
already parks one for the reload, so the natural place to park a second is three files from where a
scan over `src/app` alone could see it.

**Why the entry notice is said only after the uploads succeed.** `say_entering` sits below
`upload_textures` and `upload_scene` in `collect_preparation`, deliberately:

> A launch that fails to reach the device must not tell a player where they were put in a world
> they will never see.

That sentence exists in no source file, and how it came not to is the part worth recording. It was
written as a comment in `crates/mc-client/src/app/mod.rs` and dropped to keep that file inside its
500-line cap — **the cap did not reject code, it evicted an explanation**, silently, with nothing
going red and no deletion for a reviewer to see. This document is its only remaining home. Note
also that `app/mod.rs` now measures exactly 500 non-blank lines: **it has zero margin**, so the next
line added to it forces a split that should be planned rather than discovered by a red gate.

### A cleared player arrives with `on_ground: false`, and that is deliberate

This is the single thing about entry clearing most likely to be re-derived wrongly. `resuming`
gives a resumed player `velocity: Vec3::ZERO` and `on_ground: false`, and the clearing move touches
position and velocity only. So a player moved at entry is standing at the centre of a cell, on that
cell's floor, *not yet marked as being in contact* — and tick 1 settles them by falling a fraction
and landing. Read at tick 0 the position is exactly where the search put them; read at tick 1 the
`y` differs slightly.

`resuming` does not set `on_ground` because that would be a claim about contact nothing checked,
and grounding a cleared player is Out of Scope for the spec that built this. The cheapest way to
make a tick-1 assertion pass is to set the flag in the search or ground the player there; both are
forbidden, and a future change that does either is changing behaviour rather than fixing a test.

### Entry clearing is self-limiting through the save, not through a flag

Nothing records that a player was moved. It does not need to: the moved position is what the next
save writes, so a player cleared once resumes from where they were put rather than from where they
were trapped. A stored "already cleared" flag would be a second source of truth about a fact the
save already carries.

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
| Overlay state and readout | `mc-render/src/overlay/` — **outside** `src/gpu/` | Counted by the gate. In `mc-client` it would be invisible to coverage entirely. |
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

**The wall clock is confined, not admitted.** `mc-render/src/time/clock.rs`
holds the `FrameClock` port and its one `SystemFrameClock` adapter, and is
the single file exempted from a scan asserting that no other production
source under `crates/mc-client/src` or `crates/mc-render/src` names `Instant`
or `SystemTime`. The injectable port is what makes "ten frames 20 ms apart" a
test at all — a frame rate no test can drive is a readout no test can grade,
and a pacing no test can drive is a movement speed no test can grade.

**The port was named and placed for the overlay and is neither any more.** It
was `OverlayClock` in `overlay/clock.rs` while the debug overlay was its only
consumer; what it answers is how long a frame took, which is what paces the
simulation, and `code-quality.md` §3 names a port for its capability. A reader
asking "what paces the simulation?" would not have looked under an overlay, and
`mc-render`'s own boundary rule says that crate does not simulate. The number of
exemptions in the confinement scan is unchanged at one — that count, not the
path, is what the guard is about — and the port and its adapter stay in one file
because splitting them would need a second.

`app/mod.rs`'s header used to claim "no wall clock is read anywhere in this
client". One now is, so the claim was **narrowed to the true one**: exactly one
reading is taken per frame, and everything derived from it agrees. A stale
absolute in the very file a scan is about would be the worst place in the
workspace to leave one.

### Pacing the frame

**A frame does not buy a tick; a sixtieth of a second does.** `App::present`
hands `Session::advance_frame` the `SystemFrameClock` it has held since the
window opened. The session reads it **once**, derives this frame's interval,
and gives that one interval to two consumers: the overlay's ring, which shows
it as a frame rate, and the pacing, which spends whole tick quanta out of it.

**One reading, not two, is the whole argument for reusing the port.** Timing a
frame and pacing a frame want the same interval. Taking it twice would let the
overlay report 144 fps while the simulation spent some other amount of time —
both readings individually right, and no assertion about either alone able to
say so. Taking it once makes that unspellable, and it *removes* a public door
rather than adding one: `Session::record_frame_time` is gone, and
`DebugOverlay::record_frame` now takes a `Duration` instead of a clock.

**The pacing state is not called an accumulator.** `session/mod.rs` already uses
that word for the *input* accumulator, and two meanings four lines apart is a
file nobody can read. What `session/pacing.rs` holds is the previous clock
reading and the **unspent** frame time; the operation is *spending whole
quanta*. `mc_sim::player::TICK_QUANTUM` is the quantum as a `Duration`, declared
beside the `TICK_DURATION` seconds the physics multiplies by, with a sibling
unit test holding the two within a nanosecond of each other — neither
`Duration::from_secs_f32` nor `as_secs_f32` is `const`, so they cannot be
derived from one another at compile time.

**Nanoseconds, not seconds, and that is what makes the equalities exact.** The
unspent time is a `Duration`, so nothing is lost at a frame boundary: the ticks
a stretch of elapsed time buys are `floor(total / quantum)` however the stretch
was cut into frames. Two runs delivering the same total therefore leave the
player at bit-identical coordinates, which is why the regression suite asserts
equality rather than a tolerance.

**Whole quanta are spent by subtraction, never by division, and that is the one
line here a rewrite is most likely to "simplify".** `unspent / TICK_QUANTUM`
looks like the same answer and is not: `Duration` division goes through floating
point, and exactly three quanta comes back as `2.999 999 999 999 999 6`, which
floors to **two**. A frame that had bought three ticks would spend two and carry
the third, so a tick is lost every frame — this defect at a smaller amplitude and
far harder to notice, because the world would merely run a little slow rather
than absurdly fast. `session/pacing.rs` loops on `checked_sub` instead, which is
exact by construction and saturates at the bottom rather than being able to go
negative.

**A pathological gap is clamped to 15 quanta — 250 ms — before the time is
carried, and the surplus is discarded.** The bound is derived from the floor
rather than the ceiling: ten frames a second is the slowest rate at which a game
is arguably being played rather than hung, that is a 100 ms interval, and a bound
below it would make a slow machine lose simulated time systematically — this
defect with the sign flipped and harder to notice. 2.5× headroom over that floor.
Discarding rather than carrying is what stops a lid closed for an hour from being
replayed on resume. **When `mc-server` grows a tick loop this does not transfer**:
a server paces from its own clock and a client's stall must not stall the world
(invariant 4).

**The per-tick step is private, and that is the prevention.** `Session::tick` is
private to `session/mod.rs`, so a frame path spending one tick per frame no
longer compiles. `crates/mc-client/tests/frame_path_pacing.rs` adds a structural
scan over `crates/mc-client/src` — enumerated verdict, positive control, vacuity
refusal — for the day somebody makes the step public again. **That scan is the
weakest of the guards and it is a knowingly accepted residual hole**: a scan
reads text, and text is not execution. The one harness that runs the real frame
path, `tests/shipped_binary.rs`, cannot be pointed here — the surface takes
wgpu's default `Fifo` present mode, so the child's frame rate *is* the display
refresh, and a broken client's `f · T` ticks equal a fixed one's `60 · T` exactly
when `f` is 60. Such a test would be green on the commonest configuration there
is, red headless for an unrelated reason, and discriminating only above 60 Hz;
flaky by hardware reads as evidence while being none. If the client ever grows a
player-facing reason of its own to report simulated time or position on a stream,
that reading becomes available and this hole is worth revisiting. **The size of the
hole is measured rather than estimated: deleting the wiring outright —
`session.advance_frame(&self.frame_clock)` out of `App::present` — leaves 383 of
383 tests green.** What stands in its place is three things and not one of them
is an assertion: the private per-tick step, so the one-tick-per-frame shape does
not compile; the scan; and the single clock reading, which makes an overlay
disagreeing with the pacing unspellable rather than untested.

**The harness drives the same door.** `InputHarness::frame(took)` moves a
`DrivenClock` the suite owns and calls `advance_frame`; `tick()` is one frame of
exactly one quantum. So every scenario in `crates/mc-client/tests/` now reaches
the simulation through the door the product uses, rather than through a per-tick
call no shipped path takes — which was the third of this defect's three detection
holes.

**A per-tick effect a frame reads once must survive the ticks after it — this is
the rule a future change here must not break.** A tick and a frame stopped being
the same cadence, so anything a tick *writes* and the frame path *reads once*
now has up to fourteen further ticks between the two. Every such effect has to be
idempotent under repetition, and the ways they are differ:

| Effect | How it survives |
|---|---|
| The pending action | `take()`n by the first tick; the rest carry none |
| Pointer look delta | drained by `take_intent`; the rest see zero |
| Held keys | kept deliberately — a held key *is* asking for every tick |
| The edit report | at most one is non-`None`, so the frame collapses with `.last()` |
| The world edit | gated on an action, so at most one a frame |
| The reload's `pending` / `in_flight` / `reported` | a flag set-and-cleared, a handle taken once, a dedupe that latest-wins |
| The watch's change queue | drained per call, and `pending` accumulates what it drained |
| The world's sections needing a re-mesh | accumulates rather than latest-wins |
| The block the client holds | latest-wins, and a swap is the only thing that writes it |
| The reload report | a boundary with nothing to say **leaves the last answer standing** |

The last row was a Blocker found in validation, and it is the one worth reading
twice. `cross_reload_boundary` wrote the field unconditionally, mapping "nothing
changed" to `None` — correct while a frame was one tick and destructive the
moment it was two. Below thirty frames a second an accepted candidate swapped the
simulation's content while `App::take_up_reloaded_content` saw nothing: layers
never uploaded, `Remesher::retire` never called, the HUD stale, the author never
told — the state `app/reload.rs`'s own header declares must end the run, reached
in silence. A refusal was lost the same way, and the unwatchable-root refusal
doubly so, because the shipped watch reports that one *once*.

**"No news" is not "the news was withdrawn."** Keeping the last answer costs
nothing, because the frame path takes the report in every frame that advanced a
tick at all — `App::present`'s earlier returns skip the advance and the read
together — so what stands in the field was always produced by the frame reading
it. `crates/mc-client/tests/reload_survives_a_multi_tick_frame.rs` holds both
producer arms through a three-tick frame.

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
everything that bounds what content may do inside it (`modding/script-writing.md`
and the three `script-*` pages beside it are the author-facing side of this).
Its position on the crate map is the first thing worth
recording, because it is unusual: **`mc-script` resolves `mlua` and no workspace
crate at all**, and **nothing in `crates/` depends on `mc-script`**. The host is
exercised solely by its own tests and by the hostile-mod harness. **`mc-world` now resolves it**, for the block loader, and that is the only
workspace edge into this crate. The dependency is confined to
`crates/mc-world/src/content/` exactly as `toml` is, and the direction is what
keeps `mc-script` ignorant of what a block is: putting the loader *inside*
`mc-script` would have required it to depend on `mc-core` and to learn the
domain, which is precisely what its opaque `SubjectName`/`ComponentName` design
exists to refuse. **`mlua` must never reach `mc-world`.** The vendor's blast
radius is `crates/mc-script/src/luau/` plus `HostLimits`, and that is what a
future change may not break.

One absence remains and is deliberate rather than pending: there is no per-mod
CPU accounting, because accounting needs a tick to attribute against and no tick
calls into this crate.

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

## Block declarations in Luau: the loader, its bounds, and the seam it sits behind

`mc_world::content::LuauFileDefinitionSource` is the second implementation of
`DefinitionSource` and the one the game runs on. It splits in two, along a line
where the halves change for different reasons: `luau_source.rs` decides **which
entries under a content root are declarations and how they are read**, and
`luau_declaration.rs` decides **what a declaration must say**. A new field or a
changed default is a change to the second; a change to which files count is a
change to the first. The second is the Luau counterpart of `raw.rs`, which does
the same job for the HUD's TOML.

### Evaluation is an entry into script and is guarded like one

Every declaration goes through `ScriptHost::evaluate` **at the shipped limits** —
the call-and-loop budget, the per-entry memory cap, the sandbox and the frozen
environment. None of that machinery is written in the loader, and that is the
point: a loader that read a file and ran it round the side of the host would
satisfy every requirement about fields and would hang the server on the first
declaration that looped.

Because the host's guard is all-unwanted by nature — every one of its scenarios
asserts a refusal — **the thing that stops a host configured with an absurdly
small budget passing all of them is the requirement that a well-formed
declaration registers**, which must therefore run against the shipped limits and
never a test-sized host. Measured: replacing the loader's host with a
100,000-tick one left all nine guard tests green and reddened only that one.

**One host per read, and no handle outlives its file.** `definitions` takes
`&self` while `evaluate` takes `&mut self`, so the source cannot hold a host.
Holding one behind interior mutability would make two overlapping streams a
re-entrant borrow, which panics — and a panic on a path content reaches is what
`mc-script`'s own invariants forbid. So a host is built inside the call, used for
every file, and dropped before the call returns: the failure is unexpressible
rather than avoided.

**4,096 declarations through one script state was verified before anything was
built on it.** Peak memory, holding every returned table handle and forcing no
collection, was 2,105,960 bytes against the 16 MiB backstop — about 12.5%. The
production path completing at all is the decisive evidence, because the backstop
is enforced by the allocator on raw usage.

### The raw key enumeration, and the metamethod it actually meets

`ScriptHost::field_names(&self, table, most) -> FieldNames` answers *which keys
does this table hold*, raw. It exists because `deny_unknown_fields` was TOML's:
**a host that can read a named field but cannot ask what fields exist can never
tell a typo from an absence**, so a misspelled `replacable` would become the
silently-lost declaration the documentation promises it is not.

Four properties are load-bearing.

**Raw — and the metamethod at risk is not the obvious one.** `read_field` is
already defended against `__index`. An *enumeration* meets `__iter`, `__pairs`
and `__len`. Measured on this toolchain: `mlua`'s `Table::pairs` is **already
raw** against `__iter` and `__pairs` — a script-side `for k in t do` sees the
metamethod's list while `Table::pairs` sees the table's own keys — but
**`Table::len` honours `__len`**, answering 0 for a table holding three keys
behind a `__len` that returns zero.

**That is the reachable defect and it is a silent total failure.** A host that
sized its enumeration with `len()` would report *every* declaration as carrying
no fields at all: it would refuse nothing, lose every typo, and be
indistinguishable from a table that genuinely was empty. The enumeration is
written the way it is for that reason, not because a metamethod exists in the
abstract. **Anyone simplifying this by reaching for a length reintroduces it, and
the suite stays green** — the only test that can see it is the one asserting that
a `__len` reporting nothing does not hide what a table holds.

**Bounded inside the walk, with the bound as a parameter.** The walk stops one
key past the allowance rather than filling a vector and measuring it afterwards,
because measuring afterwards has already made the allocation the bound exists to
refuse. The bound is a parameter because `mc-script` may not learn a
block-specific number.

**Total over key types, and sorted here rather than at the caller.** A key that
is not a string is rendered by the same rendering `print` uses, never skipped — a
skipped key would make "an unrecognised field is refused" a promise holding only
for the key types somebody thought of. And Lua leaves hash-part order
unspecified, so the state's order carries **no information at all**; returning it
would hand a caller noise to render into a refusal nobody can quote, and
`documented_refusals.rs` compares a quoted refusal against a real run **line for
line**.

### The four content-root bounds, and why their order is fixed

None of these existed while the format was TOML, because a parser and a
filesystem supplied the practical limits.

| Bound | Value | Why it exists |
|---|---|---|
| declarations per root | 4,096 | a directory listing is a content-controlled allocation |
| declaration file size | 256 KiB | read into memory in full before evaluation |
| declared text length | 256 **characters** | bounds what a `BlockDefinition` retains — three strings across a whole root |
| field names per declaration | 64 | the enumeration copies every key name out of the script state |

**The check order is asserted, not incidental.** The count is taken **before any
entry is asked anything**, so a root of 4,097 files is refused from the length of
the listing rather than four thousand filesystem calls later. A file's size is
taken from its directory entry **before it is opened**, so an oversized file is
refused on its size rather than on whatever its text turned out to say — a true
statement about the wrong problem sends an author to edit a file that was never
going to be read.

**Characters, not bytes**, because the documentation says characters and bytes
would refuse a non-ASCII id at a different length than the page states. That
distinction is only catchable from the *accepting* side: 257 ASCII characters are
257 bytes, so both measures agree wherever the value is refused.

**One bound cannot state its observed quantity, and that is the design working
rather than an omission.** The field-count refusal names the bound alone, because
the enumeration stops one key past the allowance and so never learns how many
keys the declaration really held — learning it would mean performing the
allocation the bound exists to prevent. Every other bound states both quantities,
so a reader can tell "slightly over" from "far over".

**Script output the host retains is bounded too, and it truncates rather than
refuses** — a chunk that printed too much is not a malformed declaration. The
allowance is a `HostLimits` field so an operator can read and set it; it covers
one host's whole life rather than one entry, because a content root is read
through a single host; reaching it stops recording rather than dropping the
oldest, since the first line a chunk printed is what locates a failed load; and
**what was not kept is counted**, because "the mod printed nothing" and "the host
stopped keeping it" are different facts. That count travels with the lines rather
than beside them, so nothing can read the record without meeting it.

### The chunk name and the origin are deliberately different

`ScriptHost::evaluate(name, source)` is handed the file's **name alone**
(`amber.luau`), and the `DefinitionOrigin` is built by the loader from the whole
path. That split is what makes the requirement falsifiable at all: a loader
passing the full path as the chunk name would make the two coincide, and every
assertion that a refusal points at a path a person can open would become two
copies of one decision agreeing with each other. The host is handed a label and
never opens anything, so what it can report back is the label it was given.

### The simulation loads content; the client receives it resolved

`docs/planning/client-server-split.md` is the binding reasoning and is not
re-derived here. The rule: **the client never evaluates anything any other
participant, the server included, must agree with.** A content set is the
sharpest case there is, because a texture layer index rides inside every packed
vertex, so one block a participant does not share shifts every index after it and
the world is textured wrong with no error anywhere.

**What moved is the construction, not the loader.** `mc_sim::content` is where a
content root becomes a registry and a HUD. `LuauFileDefinitionSource` and
`TomlFileHudSource` stay in `mc-world`, `mc-script` keeps its
no-workspace-crate-depends-on-it property in the direction that matters, no crate
moved, and `mc-client` gained no dependency. `prepare_launch`, `prepare_scene`
and `scene_of` keep their names and their place, because the golden frames are
shot through them and the exit criterion is decided there. What left them is the
construction: the definition source, the HUD source, the registry they were
applied to, and the resolution of the content directory itself.

**Four chokepoints are watched in the client's own sources, and they are
chokepoints rather than type names** — renaming a source does not rename the
door. `registry.apply(` is the only way to populate a registry at all;
`HudLayout::load` is the only door into a layout; `BlockRegistry::new` catches a
client that builds an empty registry to fill by some other route; and
`content_root` catches a client that resolves the content directory for itself
even if it never reads it. The last of those is why the simulation's resolver is
named `shipped_directory`: a needle that admits no exemption is worth its cost,
and the alternative is a needle with a carve-out, which is where the next breach
lives.

**Two residues, stated rather than hidden.** `PreparedScene` and `PreparedLaunch`
still hand the client a whole `Arc<BlockRegistry>`, carrying the rules by which a
world is mutated as well as the fields a client draws with, because in this
arrangement the client binary *is* the server. And the source scan is the weaker
instrument: somebody adding a *second* door — a new public registration call —
bypasses it, and no text scan closes that. The instrument that would is a
dependency-closure guard, which **cannot pass while one binary hosts both
halves** — a binary's closure is the union of everything inside it — and is
therefore the composition-root spec's exit criterion rather than something this
arrangement can assert. A guard green exactly when the rule is broken is inverted
rather than weak.

### What crosses the seam, and what deliberately does not

`mc_core::content::ResolvedContent` is the value. It lives in `mc-core` and not
in `mc-sim` because `mc-render` has to be able to accept a stated assignment while
never naming the simulation — putting it in `mc-sim` would have made the renderer
reach for a crate the dependency rules forbid it — and because it is a content
primitive with no I/O, which is what `mc-core` is for.

It carries each block's **name**, the **key each of its six faces draws from** and
its **solidity**, in registration order, plus the **layer assignment**. It carries none of
`replaceable`, `breakable` or `breaks_into`: those are the rules by which a world
is *mutated*, the simulation recomputes every one of them, and a client holding
them would be holding rules it may not apply. **That absence is asserted by
discrimination rather than by inspection** — two content roots differing in
nothing but those three resolve to values that compare equal, while two differing
in a `texture` and a `solid` resolve to values that differ in both — because a
type that simply has no such field cannot fail a test about not having one, and
neither direction alone is sufficient.

There is **no identity, digest or hash of the content set**, and its absence is a
decision. With one process nothing can disagree, so nothing could falsify such a
field, and a test that cannot fail reads as evidence and is not. It becomes
falsifiable the moment a second participant exists. That is the exact opposite of
the layer assignment below, which is here precisely because its consumer exists
today.

### The six facing words are `mc-core`'s; the one mapping to axes is `mc-world`'s

A block declares a texture key per face, and the two crates that have to agree on
what a face *is* cannot both own the word. `mc_core::content::Face` is the
vocabulary content writes — `up`, `down`, `north`, `south`, `east`, `west` — with
`FaceTextures` carrying one key per face. `mc_world::mesh::Facing` is the
vocabulary a mesher writes: an axis and a sign. `Facing::face` is the single
total mapping between them and is **the one place in the workspace where a
compass word meets an axis**: `up` is +Y, `down` −Y, `north` −Z, `south` +Z,
`east` +X, `west` −X.

**The split is forced rather than chosen.** `mc-core` cannot see `Facing`,
because `Facing` is defined in terms of a section's coordinate system and
`mc-core` performs no I/O and knows nothing of chunks; and `Facing` cannot move
into `mc-core`, because its declaration order is simultaneously the face emission
order, the neighbour slot order and its `Ord`. So the published vocabulary lives
where every crate can reach it and the exhaustive `match` lives in the only crate
that can see both types.

**Two enums for six directions reads as duplication on sight**, and the drift is
closed mechanically rather than by anybody keeping two lists in step: a round trip
over both `ALL` arrays asserts that the six facings name six distinct faces and
leave none unnamed. That guard is a completeness one and says so — swapping
`north` and `south` is still a bijection, and the only witness for such a swap is
a block placed in a world with its faces read back by axis.

`FaceTextures::at` is **total**: a face always has a key, so there is no `None` to
handle at any call site and no index to get wrong. `FaceTextures::stating` takes
its six keys positionally in `Face::ALL` order rather than paired with their
faces, because a list of pairs could name one face twice and leave another
unnamed, which would put the missing case back.

### `TEXTURE_EDGE` is a contract constant, on `LAYERS_A_SESSION_MAY_ASSIGN`'s terms

`mc_core::content::TEXTURE_EDGE` is the edge of one block texture in texels. Like
the layer bound beside it, it is a property of the content-to-renderer contract
and not of either side: `mc-render` allocates its array texture to it and fills
every layer to it, and an art build has to bake to it. **It is never restated
elsewhere** — an allocation and a fill that disagreed about the size would be a
copy that either overruns or leaves a band unwritten, which is a defect no test of
either side alone can see.

`voxforge build` enforces it from the other end: a model's declared `scale` times
the manifest's `pixels_per_voxel` must equal `TEXTURE_EDGE`, refused naming the
model, the product and the edge. Three numbers with nothing connecting them is how
a 32x32 set builds cleanly, commits cleanly, passes the gate, and refuses a launch
with a message about an *image* — pointing a mod author at a file they never
authored. The build tool depends on `mc-core` already, so this costs one constant
and no new edge.

### The layer assignment is stated, not derived

**A layer index rides inside every packed vertex.** Derived as a key's position in
a sorted key set — which is what the client used to do — inserting one block
renumbers every index after it and the whole world is textured wrong: silently,
with no error anywhere, and not localised to the block that caused it. That is not
a networking concern. It is a live defect on hot reload, in one process, today.

So `mc_sim::content::resolved_from` states key-to-layer pairs when it reads the
root, and `mc_client::content::ContentView` honours them through
`TextureLayers::stated`. **Nothing on the receiving side checks the assignment
against a sort**, because checking would be the same derivation written a second
time and would refuse exactly the assignments the mechanism exists to accept. The
order the simulation happens to assign is lexicographic today, and that is an
implementation detail rather than a contract — nothing downstream may derive it,
which is what shipping the assignment buys.

`ContentView` is built from the resolved value and reads nothing else: no
registry, no path, no scripting host. That is the single property distinguishing
this seam from a rename, and the two failures it rules out are worth naming
because each would leave every scenario about content green while nothing had been
cut — a resolved value that is a newtype over the registry, and a view that reaches
back through one. `ContentView::is_solid` has **no production caller yet**: the
mesher still culls against the `BlockRegistry` that is still travelling, which is
the residue above, and this is what it reads once the registry stops travelling.

**The wiring is not behaviourally falsifiable in this arrangement, and that is
measured rather than suspected.** Restoring the production derivation left every
behavioural test green — both readings of what the value carries, the view's own,
all four of the assignment's, and both golden suites — because a client that
honours and one that derives answer identically for every content root that can be
built today, and permuting the assignment permutes the array texture's fill in the
same breath so the pictures are unchanged. **Only the source scan reddens.** It
becomes falsifiable the moment an assignment is appended rather than renumbered,
which is hot reload's, and the scan can be retired then.

## Hot reload: the seam from a saved file to a swapped registry

A content root edited while the game is running reaches the world without a
restart. Eight things happen and each of them is somewhere different on purpose.

**1. The watcher, behind a port, in `mc-world`.** `ContentWatch` is a trait in
`mc_world::content::watch`; `NotifyContentWatch` is its only adapter and the only
place in the workspace that names `notify` or `notify-debouncer-full`. The port
lives beside the other two content readers rather than beside the policy that
consumes it, which is what keeps `mc-sim` free of a filesystem dependency —
**and it is the Boundaries litmus test for this seam: if `notify` disappeared
tomorrow, exactly one file changes.** `crates/mc-world/tests/` holds the manifest
half structurally; the litmus sentence is prose because no scan states it.

**2. Which saves count, derived rather than restated.** `declares_content(root,
path)` answers yes for a file sitting *directly* inside `blocks/` with the block
extension or directly inside `hud/` with the HUD extension — and it is built from
the two loaders' own four constants rather than from a copy of them, so a loader
that changes its directory or extension changes what is watched in the same edit.
Editors write scratch files beside the file being edited; a rule that watched the
directory would try to load them.

**3. The settling window, declared once.** `SETTLING_WINDOW` is 150 ms and lives
in `mc_world::content::watch`. A scan holds it to exactly one declaration —
`crates/mc-world/tests/settling_window_declared_once.rs`, whose verdict is total so
a vanished source directory reddens rather than reading as one declaration. **That
scan does not cover the second instrument**: a window declared once and then handed
to the debouncer as `Duration::ZERO` leaves it green, and the boundary assertion
for that is separate.

**4. The clock, and why `mc-sim`'s no-wall-clock rule survives.** A settling window
is a duration, so something must read a clock. That clock is the debouncer's, it is
behind the port, and `mc-sim` is handed *changes* — a tick boundary asks whether any
arrived, never how long ago. Recorded in `crates/mc-sim/CLAUDE.md`, which is where
that rule demands its exceptions be written down.

**5. The candidate is built off the tick thread, through the existing content
door.** `ContentReload` spawns a build that calls the same `mc_sim::content::load`
a launch calls, with the serving layer assignment, and the tick boundary polls it.
Nothing new reads a content root. The build is the expensive part and the whole
reason a reload does not stall the game — instrumented by identity rather than
duration, because every timing discriminator available here is a flake generator
and a *blocking* collect is faster than a polling one.

**6. The swap, at a tick boundary, all or nothing.** `Simulation::adopt` runs
between two published ticks. Admission comes first, so a refusal returns before
anything is published and the content a reader holds is untouched by construction:
`World::adopt` resolves the new solidity *before* writing either view, and refuses a
candidate that does not declare every block the world holds. **`adopt` is the
world's second write door and it carries no `pub` at all** — the reload admission
that reaches it is a *child* module of `world` for exactly that reason, because a
sibling would have forced `pub(crate)`, which is a much weaker claim.

**7. Content is published through a second `ArcSwap` beside the snapshot**, holding
the resolved content, its HUD, and a `ContentSerial` that increments per accepted
candidate. `SimSnapshot` stays `Copy` and holds no content; nothing needs the
correlation, because a re-mesh batch carries its own serial.

**8. What the client does with it.** The report the session hands the frame path
carries the content now serving, the array-texture layers it states, and what the
swap did about the player. Three properties are held by shape rather than by
convention:

- **A batch carries its registry, and staleness is decided on the client.**
  `RemeshWork` holds the `Arc<BlockRegistry>` of the world that produced it, and
  `Retained` has no registry at all — so meshing a batch against a registry other
  than the one its world was resolved against is *unspellable* rather than checked.
  Whether a finished batch is stale is decided at collect time against the serial
  now serving, because that is the only place "serving now" is known; the worker is
  told which layers are serving on the **same ordered channel** its batches travel,
  which is what makes "told before it meshed anything with them" true of the next
  batch without a handshake.
- **A discarded batch's sections go back into the dirty set, inside the collect.**
  `Session::collect_remesh` performs the hand-back itself and `mark_for_remesh` is
  private, so there is no arm in the frame path where forgetting it is writable.
  Dropping those keys would leave those sections stale for the rest of the run — a
  wrong picture with no error anywhere. **Do not merge collecting with submitting**:
  handed-back sections would go straight into flight and the property would vanish
  into the call meant to prove it.
- **A retired layer is spent but not live.** Layers are appended and never
  renumbered within a session, because a layer index already rides inside packed
  vertices the renderer holds. A key that stops being declared keeps its layer and
  its texels for the session; the budget is spent by distinct keys ever seen, and
  relaunching reclaims the difference.

**A texture upload that fails after an accepted swap ends the run.** On the same
grounds a launch's upload failure already does: the simulation would be serving
content the device never received, which is a wrong picture with no error. The
draw-on trade the re-mesh path takes for a *batch* is right there — a stale section
is a stale picture of the same content — and wrong here, because the content itself
has moved. **This governs the texture upload only:** a scene that will not pack
after a reload is reported and drawn over, and that path is genuinely reachable,
since a whole-world re-mesh can exceed the scene's quad bound.

**Three assignments the frame path makes that nothing asserts through**, recorded
because `App` needs a real window and nothing in this workspace constructs one:
the published HUD layout reaching `App`'s own `hud` field, the clearing verdict
becoming a line a person reads, and the layers being *retired* to the re-mesh worker
after they are uploaded. Each is an assignment of a value that arrived, with covered
halves either side, and each costs a diagnostic or one stale pack rather than a wrong
picture of moved content — which is what separates them from the upload above, whose
omission was measured at 234 of 234 green and is now compiler-held.

**What a future change must not break.** The registry replaced whole rather than
mutated in place; solidity resolved before either view is written; the reload
admission staying a child module of `world`; `notify` named in one file; the
settling window declared once; a batch carrying its own registry; and appended
layers never renumbered. Each of the last four has an instrument; the first three
are held by the module structure and by a header that says so.

**Two things this seam deliberately does not have.** A geometry serial, so a reload
that changes nothing geometric still supersedes a batch in flight — it costs one
batch and can only re-mesh sections that were dirty anyway. And selective marking:
a candidate that changes what is drawn marks *every* section of the world.

### Where a reload puts a player it trapped, and the ground the search may consider

`crates/mc-sim/src/world/clearing.rs` runs after `adopt`, against the solidity the
accepted candidate produced, and only from the accepted path — so a refused candidate
moves nobody because the code never runs. It answers an enumerated verdict rather than
an absence: `Unneeded`, `MovedTo(feet)`, or `NoClearSpaceWithin { blocks }`, which is
what lets "could not be cleared" travel out through `ReloadReport::Accepted` to the
line a person reads.

The search itself: **candidates are cell centres**, `(cx + 0.5, cy, cz + 0.5)`, so the
0.6-wide box lies strictly inside one cell column and clearance is a question about the
two cells the 1.8-tall box occupies rather than about four. The cube is
`dx, dz ∈ [-8, 8]` with `dy ∈ [0, 8]`; **downward is absent from the candidate set
rather than ranked last**, so "never downward" survives any future reordering. The
order is `(dy, max(|dx|, |dz|), dz, dx)` ascending — `dy` first is what makes a sideways
cell win over an upward one at the same distance, and the last two are a declared
tie-break so two runs agree. Velocity is zeroed on any move, not only an upward one: a
cleared player has been teleported. The cost, which a player notices, is their
sub-block position; a player who needed no clearing is therefore not moved at all
rather than moved to the middle of the cell they are already in.

**A candidate is eligible only if every cell the box would cover is *known* and
clear.** `is_solid` answers `false` past the edge of the loaded world — because nothing
is there, not because it is clear — so a predicate of "not solid" alone reads outside
the world as the nearest available ground. The reach is 8 blocks and the shipped
footprint is 64 square, so any player trapped within 8 blocks of an edge had candidates
outside the world, and in a wedge those were the nearest "clear" ones the ring order
met: the player was put where nothing is solid, and fell out of the world. Reachable by
walking to an edge and saving a solidity change.

The world's extent therefore travels to `cleared` beside its solidity, named as the
ground the search may consider, and eligibility is checked there and nowhere else. A
negative coordinate names nothing the world holds; `Extent::contains` decides the rest.
Knowing comes before clearance because "not solid" about a cell the world does not hold
is not an answer about ground at all.

**Both halves of that are load-bearing, and the helper's name overstates what it
answers.** `mc_sim::world::inside_the_world` reads as a question about the world and
decides only the *sign* — its doc says so, but the name is what a caller sees, and a
caller who trusts `inside_the_world(cell).is_some()` to mean the world holds that cell
has reintroduced this defect at the world's far edge, where out-of-world coordinates are
positive and the sign refusal passes them all. The extent check is what covers that
side. Renaming it touches every caller and is deferred; believing the name is what must
not happen in the meantime.

**Three shapes that were refused, with the reasons, because each looks cheaper.**

- *Reading outside as solid.* A claim the world model does not have, in a value
  collision, meshing and the physics all read — and it inverts the moment the world
  streams, where an unloaded neighbour is unknown rather than solid and code that
  learned to read `true` there refuses legitimate moves.
- *Accepting that a reload can put a player off the map.* A silent, player-visible
  failure of exactly the kind the seam exists to prevent.
- *A second method on `Solidity`.* "Do you know this cell?" is a question about the
  world's *shape*, and the fixture doubles that answer solidity for a hand-written set
  of cells have no extent — they would have to invent an answer, which is the first
  refusal one layer out. **A trait is defined by all of its implementors, not by its
  best one.**

**No new vocabulary.** A boundary wedge with no eligible candidate takes the same
`NoClearSpaceWithin { blocks: 8 }` path a wedge in the middle of a lake takes: the
reload stands, the player stays, and a person is told.

**What a future change must not break.** The candidate set must stay free of downward
offsets rather than merely ordering them last. The ring order is load-bearing and is
what the paired positive control measures — a control expecting a destination four
blocks along `+x` at Chebyshev distance 4 is reachable *only* under
`(dy, max(|dx|, |dz|), dz, dx)`, so that control reddening means the order changed, not
that the control is wrong. And the eligibility rule must never be satisfied by refusing
everything: "outside is ineligible" implemented as "nothing is eligible" leaves the
boundary scenario green, because a scenario asserting a refusal is vacuously satisfied
by a search that finds nothing ever. That pair — a refusal scenario and a mandatory
positive control in its own test function — is why both exist.

**A fixture that traps a player near an edge is about the footprint, not about the
search.** Any wedge fixture must hold the whole cube inside the world for its premise
to be about clearance at all; the 32-block, two-column world the earlier clearing
scenarios use follows that rule rather than working around it.

### Two deferred observations about this seam, recorded rather than fixed

**1. A removed `blocks/` directory is classified differently depending on how the root
was spelled, and that asymmetry is a consequence of the repair's shape rather than an
edge case.** The relevance rule compares the saved file's parent against the declared
directory as written, and asks the filesystem only when that fails. Under an *absolute*
root the written comparison succeeds without asking, so removing the directory itself
still reports as content. Under a *relative* root there is nothing left to canonicalise,
so it reports as nothing.

Three clauses, because the first alone reads as a note about an edge case:

- It exists **only** because the rule absorbs the root's spelling. Under the shape where
  the adapter reports root-relative paths instead, both forms classify identically and
  the question never arises.
- **The shipped behaviour today is silence** — the worse of the two candidate
  behaviours, since an emptied declaration directory is a refusal this spec grades and
  a removed one now says nothing at all.
- The issue that moves this watcher into `mc-sim` **removes the asymmetry and makes the
  product decision**, and carries the lexical route: `Path::strip_prefix` compares
  components and `Components` already drops `.` while keeping `..`, so both sides
  normalise without touching the filesystem.

An *emptied* directory works under either spelling, because its parent still exists.
That is what the scenarios grade, and it is why this is deferred rather than a defect.

**2. `ResolvedVoxels::is_solid` answers `false` for every position past the world's
footprint, and that is unsound wherever it is consulted — not only in the clearing
search.** Outside the loaded world is *unknown*, not empty. Collision, meshing and the
physics all read the same answer, and `is_targetable` answers the same way for the same
reason — so a ray leaving the footprint meets nothing rather than meeting the unknown.

**In the clearing search it was a live defect and it is now closed**, because there
the answer was acted on. The eligibility rule above is the repair: a candidate counts
only if every cell the box would cover is known and clear, sited in the search alone,
with `is_solid` unchanged.

What is deferred is everywhere *else*. The footprint is fixed today and nothing else
asks past it, so the unsoundness is unreachable outside the search; it becomes reachable
the moment the world is streamed, and belongs to whichever spec does that. **The fix
there is the same shape and not a change to `is_solid`:** teaching the world model to
answer `true` off the edge would assert a fact it does not have, is read by three
subsystems rather than one, and inverts under streaming — an unloaded neighbour is
unknown, not solid, and code that learned to read `true` there would refuse legitimate
moves.

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

## The reporting seam: how a failure becomes text a person reads

**One renderer, one sink, and three doors.** `mc-render/src/window.rs` holds
`rendered(&dyn Error) -> String`, which is a failure's own message followed by
`": "` and the message of every failure beneath it in the `source()` walk,
outermost first; and `report(&Ending, &mut dyn Write)`, which writes
`"mycraft: "`, that text unmodified, and a line break. `Ending::Failed` is
`#[non_exhaustive]`, so no crate outside `mc-render` can write the struct
literal — the only ways to build a reported failure are `Ending::failed`,
`Ending::failed_under` and `Ending::stated`, and each renders the whole chain.

**It lives beside the endings rather than in `main`, and the reason is
coverage.** ADR-013 excludes `mc-client` from the coverage denominator
wholesale. Reporting written there is reporting nothing measures, which is
exactly how the defect this seam replaced survived: the client flattened a
typed failure with `.to_string()`, printed the outermost sentence, and every
test that asked the *value* stayed green while a mod author read one generic
line. `exit_code` already sat in `window.rs` for the same argument; rendering is
the other half of it.

**A message never states its own cause.** Under a full chain walk, a variant
whose own `Display` interpolates its source has that source read out twice — so
`LaunchError::Load` names the save and no longer the reason, `LaunchError::WorldGen`
names the stage and no longer the block, and `PreparationError::Launch` is
transparent. What a player reads is unchanged to the byte on those paths: the
`": "` joiner moved out of the format strings and into the renderer. A refusal
that gains a layer, like a generated world reaching a malformed id, gains it
because that layer was being dropped before.

**A way out is not a cause.** Dropping `--refuse-changed-blocks` says what to
do rather than what happened, so `PreparationError::way_out()` supplies it
separately and `Ending::failed` appends it *after* the whole chain. Wrapping it
around the front, which is what a `Display` suffix does, strands the advice ahead
of the refusal it answers. The advice now names an argument to **stop** passing:
a load refused for changed blocks alone can only have been refused because that
argument was on the command line, since nothing else produces the decision.

**What the guards forbid, and what they do not.** A source scan over
`crates/mc-client/src` reports any production file naming `Ending::Failed`,
`.to_string()`, or an error interpolated under the bindings `{failure}`,
`{cause}` or `{refused}`, with **no exemption list** — an exemption list is how
the original defect survived. The first two needles plus `#[non_exhaustive]`
carry the real invariant: a reported failure cannot be composed in `mc-client`
at all. The last three are a naming-convention guard over a narrow residual
hole, since a site interpolating an error under some other binding name escapes
them. That hole is real and it is written down rather than papered over.

The scan cannot see a report that is never *reached*, so a second test runs the
shipped binary as a subprocess and asserts what it actually writes. The two are
halves of one claim and neither is sufficient alone.

**One deviation, recorded rather than smoothed over.** The reported ending goes
through a caller-supplied `&mut dyn Write`, and `main.rs` is the only place that
names `std::io::stderr()`. But **seven non-fatal notices** in the library still
write to the process error stream directly with `eprintln!`:

| Where | Notice |
|---|---|
| `app/mod.rs` | the dropped-frame notice |
| `app/mod.rs` | the swatch notice |
| `app/mod.rs` | the unshowable-edit notice |
| `events.rs` | the cursor-release notice |
| `app/reload.rs` | the refused-content-root notice |
| `notice.rs` | what an **entry** did about a player it found inside solid blocks |
| `notice.rs` | what a **reload** did about a player its swap trapped |

**The count was four and the list omitted the two clearing notices**, which were
inside `App::report_clearing` when it was written. Both now live in `notice.rs`
beside the entry notice this seam gained — the composing half of each is a total
function of a `Copy` verdict and the `eprintln!` is all that needs a running
client, which is what made the exact words assertable at all. None of the seven
ends a run and none goes through `report`, which is why the reporting guards do
not cover them — they are a stream question rather than a rendering one. It does mean nothing can capture,
redirect or silence them, and a library naming a stream is not the shape the
sink parameter exists to establish. The spec that next touches client output
should route them through a sink too.

## Mechanically enforced invariants

Several facts about these designs are asserted by tests that walk real
structure, not by convention:

- **Neither `toml` nor `mlua` is anywhere in `mc-core`'s resolved
  dependency graph**, and `mc-world`'s reaches **both**. A test walks
  `cargo metadata`'s resolved graph breadth-first from each node and
  fails if a needle appears where it must not, including transitively.
  The two assertions are each other's positive control: the day somebody
  deletes the loader and hardcodes block definitions into `mc-core`
  directly — exactly the regression this structure exists to prevent —
  `mc-core` would still be parser-free and an absence-only check would
  pass cheerfully forever.

  **Both needles are named because one stopped being sufficient, and this
  is the general lesson rather than a detail of this guard.** The check
  originally named `toml` alone, on the reasoning that a declaration
  format's parser belongs to the loader and nowhere else. The day block
  declarations became Luau chunks, `toml` stopped being how block
  declarations arrive — so the guard went on passing while the property
  it protects had gone unguarded for the new way in, and its companion
  assertion could no longer tell a loader that reads block declarations
  from one that has stopped reading them at all. **A guard that
  enumerates specific dependencies silently narrows whenever the set of
  things it guards against grows, and nothing about it goes red to say
  so.** Whenever a format, a backend or a vendor is *added* beside an
  existing one rather than replacing it, every absence guard naming the
  old one is already out of date. The constant is named for the **HUD
  format's** parser for the same reason: a constant that keeps an
  outdated name is how a guard drifts from its purpose without anyone
  editing a line of it.
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
  same positive control as the `mc-core` needle walk above: it also
  asserts that a dependency each inspected crate genuinely has *is*
  present, so a walk that silently resolved nothing does not pass
  vacuously. `tools/` — home to `voxforge` and, per ADR-009, developer
  tooling generally — may depend inward on `crates/`; the reverse never
  holds.
- **`mc-client` reaches `mc-script` today, and that is a residue of one binary
  hosting both halves rather than a property anybody wants.** The guard that once
  forbade it — the client's resolved closure excluding the scripting host in
  every dependency kind, with positive controls proving the walk could see — has
  been retired, and the reasoning printed here for its retirement was wrong. It
  said the client "has to know what blocks exist, and it learns that by
  evaluating the same block declarations the server evaluates", and that a client
  which cannot reach the host cannot draw the world. **That is a non-sequitur.**
  The client needs *resolved definitions*. It has never needed the evaluator, and
  nothing about drawing a world requires a VM in the binary that draws it.

  **The rule, which is binding and is recorded with its reasoning in
  `docs/planning/client-server-split.md`: the client never evaluates anything any
  other participant, the server included, must agree with.** A content set is the
  sharpest case there is. A texture layer index rides inside every packed vertex,
  so one block a participant does not share shifts every index after it and the
  entire world is textured wrong — silently, with no error anywhere, and not
  localised to the disputed block. Passing that test makes client evaluation
  *permissible*, never obligatory: performance and isolation rules bind
  independently, which is why neighbour-dependent block appearance is still
  refused despite passing it.

  **Why the guard was nonetheless unassertable, which is the part worth keeping.**
  A binary's dependency closure is the union of the closures of everything inside
  it, and in singleplayer the client binary *is* the server. Whichever crate loads
  content sits inside it. The only arrangement in which that test passes today is
  the one where the client sources content itself — so the guard was green exactly
  when the rule was broken, which is inverted rather than weak. The binding
  constraint is not crate topology but the fact that `mc-client` is the
  composition root, and that is a choice rather than a law. **Restoring the guard
  is the exit criterion of the spec that moves the root**, which is a better job
  for it than guarding, and it is not something an intermediate spec can assert.

  **What still carries the security weight is Invariant 4, unchanged: the server
  is authoritative, and anything a client claims is recomputed server-side.** Nor
  does any of this bear on where a *player* aimed — re-deriving that on the
  server is what a competitive shooter needs, and MyCraft is not one. The
  `mlua`-containment guard below is a different guard about a different property
  and stands exactly as it was.

  A reader arriving with the question *"should the client evaluate content?"*
  should find this paragraph rather than silence. **No.** It receives content
  already resolved, and the boundary that matters is agreement — what every
  participant must hold the same value for — as much as it is authority over
  world state.
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
  `SystemTime`** except `mc-render/src/time/clock.rs`. Both scans carry a
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
