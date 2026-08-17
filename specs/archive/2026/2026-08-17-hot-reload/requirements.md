# Requirements record — PRO-918

What was established from the tree rather than assumed, and the decisions taken
where the issue's framing and the repository disagreed.

## Source material

- Linear issue PRO-918, "Hot reload: edit a block definition on a running server
  and see it change", project *MyCraft MVP 2: Scriptable Content*. Two of its
  statements are marked **USER RULING** and are binding: changing a script may
  reset state, and a player inside a newly solid cell is shoved to the nearest
  clear cell, sideways or up.
- `product/roadmap.md`, MVP 2 — the binding scope constraint, including the
  feature table that sequences PRO-902/PRO-914 **after** this spec.
- `CLAUDE.md` (read from disk), `crates/mc-script/CLAUDE.md` invariants 6 and 7,
  `crates/mc-sim/CLAUDE.md`, `crates/mc-render/CLAUDE.md`, `content/CLAUDE.md`.
- `docs/planning/client-server-split.md` — settled, binding, not re-derived.
- `specs/archive/2026/2026-08-16-blocks-in-luau/` (SPEC-016), which this builds
  directly on.
- The tree itself, at `main` = `4bc210a`.

## Ground truth, verified against the tree

Read directly. These are observations, not inferences.

### There is no registry swap today, and three things hold the registry

`BlockRegistry` is handed out as `Arc<BlockRegistry>` and never swapped.
`mc_sim::content::load` builds one (`crates/mc-sim/src/content.rs:112`), and
`prepare_launch` wraps it in an `Arc` shared three ways
(`crates/mc-client/src/launch.rs:197`):

| Holder | Where | What it uses it for |
|---|---|---|
| `mc_sim::world::World` | `crates/mc-sim/src/world/mod.rs:70` | resolving a name at every write, and `SolidVoxels` at construction |
| `mc_client::remesh::Retained` | `crates/mc-client/src/remesh.rs:135` | meshing a batch, **moved into the worker thread** |
| `PreparedLaunch::registry` | `crates/mc-client/src/launch.rs:84` | handed to both of the above |

`Simulation` publishes `SimSnapshot` through an `ArcSwap`
(`crates/mc-sim/src/simulation.rs:61`). **Content is not published at all** —
it is handed over once at collection and then held. Invariant 6's `ArcSwap`
swap of a replacement registry does not exist yet; this spec is what introduces
it.

`World` owns `blocks`, `solid`, `registry` and `dirty`, all private, and its
module header states that **nothing outside the module can write any of the
three views and exactly one function writes anything**. A reload needs a second
write door into that type, and it is the one place this spec weakens a
structural claim the tree currently makes. Recorded here so the architecture
answers it deliberately.

### The scratch VM is already how a content root is read

`LuauFileDefinitionSource` builds a `ScriptHost` **inside** `definitions()`,
uses it for every file, and drops it before the call returns
(`crates/mc-world/src/content/luau_source.rs`, §"One host per read, and no
handle outlives its file"). So invariant 7's "candidate registries are built in
a scratch VM off the tick thread" needs no new mechanism at all: it needs the
existing `mc_sim::content::load` called on a thread that is not the tick's,
which is exactly what `spawn_preparation` already does at launch
(`crates/mc-client/src/launch.rs:143`).

### The tick is the frame path, and the re-mesh already runs off it

One tick per rendered frame (`crates/mc-client/src/app.rs:245`,
`session.tick()`). A re-mesh runs on its own worker, one batch at a time, and
edits made meanwhile accumulate in the world's per-section dirty set
(`crates/mc-client/src/remesh.rs`, `crates/mc-sim/src/world/remesh.rs`). So "a
reload must not stall the tick" is a property the existing transport already
has; what this spec adds is getting the *new* registry and layers into that
worker before it meshes against them.

### The world is 256 sections

`FOOTPRINT_COLUMNS = 4` (`crates/mc-sim/src/replay/world.rs:27`) and
`SECTIONS_PER_COLUMN = 16` (`crates/mc-world/src/column.rs:17`): sixteen columns
of sixteen sections. `product/roadmap.md` records the mesher benchmark at
~136 µs/section for terrain, and meshing runs on rayon workers. A whole-world
re-mesh is therefore a bounded, already-parallel operation at this world size —
which is why the bound this spec states is on *work* rather than on cleverness.

### The array texture is pre-allocated to 256 layers, and never grows

`LAYER_BITS = 8` in the packed vertex, so `MAX_LAYER = 255`
(`crates/mc-render/src/geometry/vertex.rs:37,62`), and the array texture is
created at `depth_or_array_layers: TEXTURE_LAYERS = MAX_LAYER + 1`
(`crates/mc-render/src/gpu/buffers.rs:48,244`). `write_layer` refuses a layer at
or past that and names the capacity.

**So appending a layer does not recreate the array texture.** It is one
`queue.write_texture` of one 16×16 layer. The brief's caveat that "the array
texture may still need recreating when a layer is added" is not true of this
tree, and the real constraint is the opposite one: the 256 layers are a fixed
budget that append-never-renumber spends monotonically within a session.

### A texture edit is not observable in this increment

`layer_for` resolves a quad's layer by **parsing the block's own name as a
texture key** (`crates/mc-render/src/geometry/mod.rs:180-192`), and
`held_swatch` does the same for the indicator (`crates/mc-render/src/hud/held.rs:105`).
`crates/mc-render/CLAUDE.md` records both as one known gap and says MVP 2 must
close it. `registry.texture_keys()` reads each definition's `texture` field, so
the layer table is keyed on `texture` and looked up by `name`.

Consequences, stated because they decide what this spec's demonstration can be:

- Editing a placed block's `texture` field removes its old key from the layer
  table and adds a new one; the mesher then looks up the block's **name**, finds
  no layer, and the whole re-mesh batch fails with `UnresolvedTexture`.
- `docs/modding/blocks-items.md` §"Texture keys today" already documents the
  load-time half of this: "A declaration whose `texture` differs from its `name`
  will load and then not draw."

`product/roadmap.md`'s MVP 2 table lists **"Texture resolution through the
registry, and per-face keys — PRO-902, PRO-914"** as a separate P0 item after
this one. See Decision 3.

### The save format already classifies a definition change, in two halves

`crates/mc-world/src/persistence/format.rs` folds every block into two 64-bit
FNV-1a values: `behaviour_of` over `name`, `is_solid`, `replaceable`,
`breakable`, `breaks_into`; `appearance_of` over `name` and `texture`. Both are
`pub(crate)`. `RegistryVerdict { missing, changed, retextured }` and
`resolve(&SaveRequirements, &BlockRegistry)` are public
(`crates/mc-world/src/persistence/table.rs`), and `resolve` takes a save's
requirements rather than a second registry, so it is the right *shape* and not
directly the right *signature* for a candidate-versus-live diff.

The split is exactly the classification a reload needs and it is already
argued in the tree: "a block whose texture changed is the same block to stand
on, and a block whose solidity or drop changed is not."

### The player gets stuck, not shoved

`Refusal::InsidePlayer` (`crates/mc-sim/src/world/action/mod.rs:228`) refuses a
*placement* into the player's box, so nothing in the tree ever moves a player
out of anything. Collision resolves per axis and refuses a displacement that
would overlap (`crates/mc-sim/src/player/collide.rs:125`), so a box that
*already* overlaps a solid voxel has every displacement refused in every
direction, gravity included. A reload that makes an occupied cell solid wedges
the player permanently.

`base:water` is the only non-solid block the base game ships, so this is
reachable today: swim into water, declare water solid, save.

### The unknown-block path does not exist

`World::new` resolves solidity for every voxel through the registry and returns
`RegistryError::UnknownName` if a placed name is not registered
(`crates/mc-sim/src/replay/solid.rs`, `SolidVoxels::resolve`). The save path
refuses a `missing` name outright and states why: "nothing can go in the cell,
and that is not a judgement a player is in a position to make"
(`crates/mc-world/src/persistence/table.rs`, `RegistryVerdict::refusal`).

### Per-cell state does not exist, and neither do mod tests

`docs/modding/blocks-items.md` §"What is not here yet" lists per-cell state,
callbacks and components. `content/CLAUDE.md` states that block declarations
carry no `tests/` directory and cannot, "because there is no `mycraft.*` binding
for a declaration to call, so there is nothing for a mod-authored test to assert
against beyond what the loader already refuses."

A section's palette stores **names**, and the two-value save encoding stores
names plus the two hashes. There is nothing else per cell.

### `notify` is pinned and unused

`notify = "8.2.0"` and `notify-debouncer-full = "0.7.0"` are in
`[workspace.dependencies]` (`Cargo.toml`). No member crate names either
(`grep -rn notify crates/*/Cargo.toml tools/*/Cargo.toml` → no match). They are
available and pinned, and nothing has yet decided how they are reached.

### Nothing in the workspace runs `App`

`crates/mc-client/src/app.rs`'s own header records that this crate "holds no
coverage of its own", and `docs/technical/testing.md` records why nothing runs
`App` and that coverage cannot say so. The lesson the tree already paid for —
a client submitting a default intent every tick left 406 of 406 tests green —
binds here: a reload policy written in the frame path is a policy nothing
grades.

## Where the issue did not survive contact with the tree

1. **"Driven by the schema hash from the per-cell state issue"** — per-cell state
   is deferred (PRO-911) and not built, so there is no per-cell state to
   preserve or drop. The framing is not vacuous, though: the *definition* hash
   the issue reaches for already exists in the save format, in two halves, and
   is the right classifier for what a reload changed. See Decision 1.
2. **"the unknown-block path: solid, occluding, placeholder texture, state
   preserved opaquely"** — three of those four words name things that do not
   exist. `occludes` is PRO-904's field, "state preserved opaquely" is per-cell
   state, and there is no unknown-block concept anywhere in the tree. See
   Decision 2.
3. **"A mod's `tests/` run on every reload candidate before the swap"** (from
   `crates/mc-script/CLAUDE.md` and `content/CLAUDE.md`) — mod tests cannot
   exist until a `mycraft.*` binding does, and `content/CLAUDE.md` already says
   so in as many words. What gates a candidate here is the loader's own
   all-or-nothing validation, which is the same gate that runs at launch. Stated
   rather than left as an unmet promise.
4. **The array texture does not need recreating on append.** It is allocated at
   256 layers up front. The real constraint is the fixed 256-layer budget. See
   the ground-truth section above and FR-5.2.

## Decisions taken

### Decision 1 — what survives a reload is the world and the player, and the definition hashes are what classify a change

There is no per-cell state, so "texture change → state survives / shape change →
state dropped" has nothing to bite on in this increment. Saying so plainly is
the spec's job; specifying migration machinery for state that does not exist
would be building against a shape nobody has met.

What does exist and must survive is not small: every block the player has broken
or placed, the player's position, orientation and velocity, the tick counter,
the block in their hand, and the save that is written at quit. FR-3 is about
those.

What the issue's "schema hash" reaches for *does* exist, in the save format's
`behaviour_of`/`appearance_of` split, and it is reused rather than reinvented:
it is what decides whether a reload has to re-mesh anything at all (FR-6.1).

The USER RULING that "changing a script may reset state" is honoured and costs
nothing here, because there is no script-held state to reset. It is recorded so
that whichever spec builds per-cell state inherits the ruling rather than
re-asking.

### Decision 2 — a candidate that drops a block the world holds is refused, and the unknown-block path is not built

The issue's edge case wants a placeholder block: solid, occluding, placeholder
texture, state preserved opaquely. Two of those four properties name fields that
do not exist (`occludes` is PRO-904's; per-cell state is PRO-911's), and the
tree has no concept of a block that is in the world but not in the registry —
`SolidVoxels::resolve` refuses one and the save path refuses one.

So the reload refuses the candidate instead, naming every block the world holds
that the candidate does not declare, and the previous content keeps serving.
That is the same rule the save path already applies to a `missing` name, with
the same reasoning: nothing can go in the cell, and it is not a judgement to
make on the author's behalf.

**What it costs the author, stated rather than hidden:** renaming a block that
is already placed refuses the reload until they either restore the name or
remove the block from the world. That is a legible refusal naming the block,
which is better than silently swapping their world's blocks for a placeholder,
and it is exactly the diagnostic the unknown-block path would have made quiet.

Recorded in Out of Scope with its reasoning, not dropped.

### Decision 3 — a texture edit is not this spec's demonstration, and that is the roadmap's ruling rather than a preference

The mesher resolves a layer by parsing a block's **name**, so editing a
declaration's `texture` field changes nothing visible and breaks the re-mesh
batch for that block. `crates/mc-render/CLAUDE.md` records this as one known gap
with two sites, and `product/roadmap.md` gives it to **PRO-902 and PRO-914** as
a separate P0 item after this one.

`product/roadmap.md` is binding on scope, exactly as it was for SPEC-016's
`extends`. So this spec does not close it, and the demonstration it ships is
built from what *is* observable today:

| Edit | What the author sees |
|---|---|
| `solid` flipped | faces appear or vanish across the world; the block stops or starts stopping the player |
| `breakable = false` | breaking that block is refused as indestructible |
| `breaks_into` added | breaking leaves the named block behind |
| `replaceable` flipped | a placement may or may not overwrite it |
| a new declaration added, sorting first | it appears in the player's hand and can be placed, wearing its own texture on an appended layer |
| any typo | the game keeps running and the terminal names the file, the block and the field |

The spec states the texture limitation to the mod author in their own page
rather than leaving them to discover a block that stops drawing.

### Decision 4 — this spec owns the routine that moves a player clear

The issue says the shove is "the same routine needed when a piston pushes a
player and when a block is placed into occupied space — one routine, three
callers, written and tested once. Whichever spec lands first owns it."

Checked: there is no piston and no spec that adds one in MVP 2, and placement
into an occupied box is **refused** (`Refusal::InsidePlayer`) rather than
resolved by moving anybody. So hot reload is not merely the first caller — it is
the only thing in the tree that can create the situation at all, and it can
create it today via `base:water`.

Leaving it out means the headline feature can wedge a player permanently with no
way out but quitting. It is a pure function of a position and a solidity view,
drivable with no world and no window, and it is four scenarios. This spec owns
it (FR-7).

### Decision 5 — the whole content root reloads, blocks and HUD together

`mc_sim::content::load` reads blocks and HUD declarations from one root and
refuses them together, because "a crosshair the content declares is content
exactly as a block is, and a root that is good for one and bad for the other is
a root that failed."

Splitting that apart for reload would mean building a second, partial door into
content — which is the partial application invariant 7 calls a Blocker. So a
reload attempt reads the whole root through the existing call, and a refused HUD
declaration refuses the blocks with it.

### Decision 6 — the block in the player's hand is re-derived, not preserved

`default_held_block` is "the first solid block in registration order", which
`docs/modding/README.md` describes as the placeholder that makes a new block
reachable at all until an inventory exists. It is a policy derived from the
registry, not something the player accumulated.

Re-deriving it on reload is what lets a mod author declare a block and then go
and place it — the vertical slice this spec has to deliver under Key Principle
7. Preserving it instead would make a newly declared block unreachable until the
next launch, which is the *opposite* of what hot reload is for.

The cost is that a reload changing which block sorts first changes what the
player is holding. That is legible, it is what the documented rule says, and it
disappears the moment PRO-929 gives a player a real hand.

### Decision 7 — the change is noticed, not asked for

The roadmap's exit criterion is "edit a block definition in a text editor while
the game is running, **save**, and see it change" — a key the player presses
would not satisfy it. So the content root is watched.

The watcher is an external boundary in the `code-quality.md` §5 sense and gets a
port named for the capability, with the `notify`/`notify-debouncer-full`
adapter behind it and an in-memory double in front of it. That is what keeps
every scenario in this spec drivable without filesystem timing, and it is why
those two dependencies are pinned in the workspace and named by nothing yet.

**The capture path does not watch.** A golden frame is a claim about one content
set, and a run that could re-read content mid-capture is a run whose committed
image depends on what happened to be on disk. `prepare_scene` gains no watcher.

## The stakeholder capability

**Stakeholder:** the mod author (who, in singleplayer, is also the player).

**What they can now do:** leave the client running, open
`content/base/blocks/stone.luau` in a text editor, change `solid = true` to
`solid = false`, save, and walk through stone — without restarting, without
losing the hole they just dug or where they were standing. Get the edit wrong
and the game keeps running while the terminal names the file, the block and the
field; fix it and save again and the next attempt lands.

**Player:** the world can now change under them while they are in it, and they
are moved to the nearest clear space rather than trapped when something they are
standing in becomes solid. Their world, their position and their save are
unchanged by a reload.

**Engine reader:** the first `ArcSwap` of a registry replacement in this project,
the tick-boundary swap invariant 6 has always described, and the layer policy
that keeps every vertex already on the GPU valid.

## Open questions

None. The three places the issue and the tree disagreed are resolved above
against the tree and against `product/roadmap.md`, which `CLAUDE.md` and the
team brief both make binding.
