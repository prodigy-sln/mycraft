# Architecture: Hot reload — the registry becomes a published value, swapped between ticks

**Spec**: `spec.md` (SPEC-017, PRO-918, rigor `high`) · **91 scenarios** —
FR-1: 17, FR-2: 13, FR-3: 12, FR-4: 15, FR-5: 8, FR-6: 9, FR-7: 7, FR-8: 8,
FR-9: 2.

The architecture stage opened against **89** and closed against **91**. Two
assumptions this document had to make — that an accepted reload also applies the
HUD, and that a change the loader would not read begins no attempt — were
promoted to scenarios (**FR-4.5-S1** and **FR-1.1-S9**) on the owner's ruling,
because each stood in for a property the design answers and nothing asserted.
The first is Blocker-class: applying the blocks and not the HUD *is* the partial
application `mc-script`'s invariant 7 forbids.

Everything below was read from the tree at `877af4a` rather than inferred. Where
a number is a measurement rather than a derivation it says so.

**Reviewed once by `persona-architect` (Mode B) before being marked binding.** It
returned three Blockers, five Majors and six Minors; all are folded in, and the
three decisions it overturned — D6, D8 and D3's siting — are rewritten below with
the reasoning that failed recorded beside them, because the failed reasoning is
what a later reader would otherwise re-derive.

---

## Drivers

### Quality attributes that matter here, with their evidence

| Attribute | Why it matters for *this* feature | Evidence in the tree |
|---|---|---|
| **All-or-nothing application** | `crates/mc-script/CLAUDE.md` invariant 7 calls partial application a Blocker. A reload touches the registry, the solidity bitset, the layer assignment, the HUD, the held block, the dirty set and the array texture — seven places that can each be half-done. | FR-2.1-S5, FR-4.1-S5, FR-5.1-S5, FR-7.1-S6 |
| **Two views that must not disagree** | `crates/mc-sim/src/world/mod.rs` exists to make a block store and a solidity bitset unable to fall out of step, and its own header records why deriving one from the other was rejected: the replay's overlap oracle would then be judging itself. A reload is the first thing that writes both outside `World::write`. | FR-4.1-S4 |
| **Vertices already on the GPU stay valid** | A layer index rides inside every packed vertex (`geometry/vertex.rs:38,62`). Under a derived assignment a reload renumbers every index after an inserted block — silently, world-wide, not localised. | FR-5.1-S1..S4, FR-8.1-S3 |
| **No decision in the frame path** | `crates/mc-client/src/app.rs` needs a real window, nothing in the workspace constructs one, and `mc-client` is excluded from the coverage denominator **wholesale** (`scripts/sdd-gate.ps1:93`, ADR-013). Two measured precedents: a client submitting a default intent every tick left 406/406 green; deleting the free-cursor guard left the same. | FR-9.1-S1; the whole of the placement rule in D1 |
| **Determinism of the gate** | A wall-clock assertion is a flake generator and `testing.md` §8 quarantines flakes on sight. A filesystem watcher is the most obvious source of one this project has met. | FR-9.1-S1; D2's port |
| **Refusal text a person can act on** | `crates/mc-client/tests/documented_refusals.rs` compares a quoted refusal against a real run line for line. Every refusal this spec adds is quoted on a page. | FR-2.1-S1..S4, FR-2.2-S3, FR-5.2-S1 |
| **The tick never waits** | One tick per rendered frame (`app.rs:245`). A build that blocked the tick would stutter the game on every save. | FR-1.2-S2, FR-3.2-S1, FR-9.1-S1 |

### Constraints

- **`product/roadmap.md`** sequences PRO-902/PRO-914 (texture resolution through
  the registry) **after** this spec. Binding: a `texture` edit is not this
  spec's demonstration, and `crates/mc-render/src/geometry/mod.rs` `layer_for`
  and `hud/held.rs` `held_swatch` keep resolving by block *name*.
- **`mc-sim` may not name `mc-render`** (`crates/mc-render/tests/dependency_graph.rs`
  asserts neither resolves the other). So `TextureLayers` cannot travel through
  `mc-sim`, and the 256-layer bound cannot be read from `mc-render::MAX_LAYER`
  by the code that refuses a candidate for exceeding it. D5 answers this.
- **`mc-sim` reads no wall clock** (`crates/mc-sim/CLAUDE.md` §Boundaries), and
  "nothing may start to without a recorded decision". A debouncer reads one.
  D2 places it accordingly and records the decision.
- **`mc-client` is excluded from coverage wholesale.** `docs/technical/architecture.md`
  §"Saving and loading a world" already argues from this: putting policy there
  "would have put a real decision in the one place nothing measures it". This
  spec is bound by the same argument, and it is the single strongest constraint
  on placement.
- **Clippy thresholds** (`docs/technical/testing.md` §"Complexity thresholds"):
  30-line functions, **4 arguments including the receiver**, nesting depth 3,
  **500 non-blank lines per source file**. `#[allow]` is not available. Measured
  at `877af4a`: `crates/mc-client/src/session.rs` is at **492** non-blank and
  `app.rs` at **456** — eight and forty-four lines of headroom. D14 answers it.
  (After phase 1 that file is `crates/mc-client/src/session/mod.rs` at **496**;
  see D14's amendment.)
- **The client's own sources name no content door**
  (`crates/mc-client/tests/client_names_no_content_door.rs`), and FR-8.2-S1
  extends that set to the reload path and the watcher.
- **`World::write` carries no `pub` at all** and the action resolution reaches
  it only because it is a *child* module. Preserving that is D3's whole shape.

### Volatile, and expensive to reverse

- **Volatile**: the refusal wording (documentation quotes it verbatim); the
  three declared quantities (150 ms, 8 blocks, 256); which fields count as
  geometry-relevant (PRO-904 adds `occludes`, PRO-902 changes how a texture is
  resolved — both change D7's predicate).
- **Expensive to reverse**: the monotone layer budget (it is what every packed
  vertex already on the GPU depends on); the shape of the published content
  value and its serial (a second participant will read it); the second write
  door into `World`; `mc-world` gaining a filesystem-watching vendor.

---

## Boundaries

| External dependency | Volatility (V/R/S/Sub) | Port | Adapter location | Direct-use justification |
|---|---|---|---|---|
| **Filesystem change notification** (`notify` 8.2.0, `notify-debouncer-full` 0.7.0 — inotify/ReadDirectoryChangesW/FSEvents underneath) | V: low (OSS) · R: none · **S: high** (pre-1.0 debouncer, platform backends differ in what they report for one save) · **Sub: moderate** (`watchexec`, polling). Also a **source of nondeterminism and of wall-clock time** — both mandate a port under architecture-principles §3. | `ContentWatch` (**new**, `mc-world`), answering *"what under this root has changed since I last asked?"* | `crates/mc-world/src/content/watch/notify_watch.rs` (**new**) | — |
| **Luau VM via `mlua` 0.12**, and every value a declaration produces | unchanged by this spec | `ScriptHost`, `ScriptValue`, `ScriptFault`, `HostLimits` (existing, `mc-script`) | `crates/mc-script/src/luau/` (existing) | — |
| **Filesystem** (`std::fs` reads and listings) | std library, in-process | none | — | architecture-principles §3 exclusion: standard library. Consistent with `luau_source.rs` and `hud_toml_source.rs` today. |
| **`wgpu` array texture** (one layer written per key) | unchanged by this spec | `FrameRenderer::upload_textures` (existing) | `crates/mc-render/src/gpu/` (existing) | — |

**The port is shaped around the domain's question, not the vendor's.** `notify`
speaks in `Event`s carrying `EventKind`, `Vec<PathBuf>` and an `AccessMode`
taxonomy that differs per platform. The port speaks in **paths that changed**,
and knows nothing of create/modify/remove — because the loader reads the whole
root on any change (FR-1.2-S1), so *which kind* of change happened is
information the domain has no use for. A port that mirrored `EventKind` would be
a defect by architecture-principles' own test.

**Litmus test — if `notify` disappeared tomorrow, how many files change?** One:
`watch/notify_watch.rs`. `mc-sim` never names it, `mc-world` names it only in
that file's own module, and the relevance rule, the coalescing and every
scenario in FR-1 except the settling window itself are reachable through the
in-memory double. That must remain true and is what the engine-reader record has
to state as the thing a future change may not break.

---

## Decisions

### D1 — The reload decision lives in `mc-sim`; `mc-client` gains reporting, one upload and one staleness comparison. **BINDING**

**Options.**

1. **In `Session` (`mc-client`).** It is the tick boundary a windowed client
   crosses, it already owns `holding` and the simulation, and ~40 test binaries
   drive it.
2. **In `App` (`mc-client`).** Where the frame, the renderer and the re-mesh
   worker already are.
3. **In `mc-sim`**, driven from `Session::tick`, with the client handling the
   report, the upload and the batch it can no longer use.

**Evaluation.** Option 2 is ruled out outright: nothing in this workspace runs
`App`, and `docs/technical/testing.md` §"Nothing in this workspace runs `App`"
records that coverage will not say so. Option 1 is the tempting one — `Session`
*is* drivable — but `scripts/sdd-gate.ps1:93` excludes `crates/mc-client/`
**wholesale** from the coverage denominator, and the architecture record already
settled this exact trade for persistence in as many words. A reload policy in
`mc-client` would be graded only by the tests somebody remembered to write, with
no floor under it.

**Recommendation: option 3.** `mc_sim::reload` owns the pending flag, the
in-flight build, admission, the swap, publication and refusal deduplication.

**The residue in `mc-client` is larger than one line, and this draft first
under-counted it.** Enumerated, because a residue nobody counted is a residue
nobody sizes:

1. `Retained` loses `registry` and the worker gains a control message (D8).
2. `Remesher` remembers the in-flight batch's keys and serial, and compares at
   collect time — **a decision, not an `!=`** (D8).
3. `Remeshed` becomes a three-variant enum; `exchange_remesh` grows an arm that
   hands keys back to the world. **AMENDED IN PHASE 4: that arm may not hold the
   decision.** A mutation replacing it with `drop(keys)` left 77 of 77 green,
   because nothing in this workspace constructs `App` — `App::new` has one call
   site, in `events.rs` — so the arm is unreachable by any test and the omission is
   silent and permanent: those sections stay stale for the rest of the run, a
   wrong picture with no error anywhere. Unlike residue items 4 and 6, which are
   **assignments of values that arrived** with covered halves either side, this one
   is **a decision to call at all, guarding state** — D3's category, which earned
   two guards.
   **AMENDED AGAIN, and this is the shape that landed.** The first repair —
   `Superseded` carrying a `Stale` whose only consuming method is
   `back_into(&mut Session)`, with a test driving that method — was built and
   measured, and `drop(stale)` in the frame path still left **3 of 3 green**.
   Renaming a call the fixture makes does not move it onto the product's path.
   So the *collect* moved instead: `Session::collect_remesh(&mut Remesher) ->
   Remeshing` performs the hand-back itself, `Session::mark_for_remesh` is
   **private**, and the frame path has no arm to write either way — `App` matches
   on a total verdict whose `Discarded` carries nothing. `drop` inside
   `collect_remesh` now reddens exactly one test, and omitting the call fails the
   build twice over (`unused variable: stale`, `into_keys is never used`).
   **Collecting moved and submitting deliberately did not**: an exchange doing both
   would put handed-back sections straight into flight and leave the discard with no
   observable at all.
4. Rebuilding `TextureLayers` from the published content and uploading them,
   including what an upload failure does now that the swap has already happened
   (D15).
5. Refusal-text reporting — a fourth `Option<String>` beside `reported`,
   `reported_remesh` and `reported_swatch`.
6. `Session` grows four methods and the drive after `advance`.

That is a real cost of option 3 and it is the strongest argument against it. D14
answers where it goes; the tests that grade it are named in Risks.

### D2 — The watcher port and its adapter live in `mc-world`; the policy that consumes it lives in `mc-sim`. **BINDING**

**Options.** (a) Port and adapter both in `mc-sim`, beside the policy.
(b) Port and adapter in `mc-world`, beside the other two content readers.
(c) Adapter in `mc-client`, which is where a window and a runtime already are.

**Evaluation.** (c) is refused by FR-8.2-S1. Between (a) and (b), one fact
decides it: **the relevance rule must be built from the loader's own constants
or it silently narrows.** `LuauFileDefinitionSource` declares `BLOCKS_DIRECTORY`
and `DECLARATION_EXTENSION` (`luau_source.rs:50,53`); `TomlFileHudSource`
declares `HUD_DIRECTORY` and its own `DECLARATION_EXTENSION`
(`hud_toml_source.rs:25,28`) — **two constants of the same name in different
modules**, which the relevance rule imports and must disambiguate. FR-1.1-S5 (a
HUD file begins an attempt) and FR-1.1-S7 (`stone.luau.swp` begins none) are
decided by exactly those four values. A relevance rule in `mc-sim` would be a
*second* list of directories and extensions, and the day a third declaration
kind arrives it would go on answering for two — which is precisely the failure
mode `docs/technical/testing.md` §"A guard that names a specific dependency
silently narrows as the set it guards grows" is about.

`mc-world` is also where every `fs` call on the content path already lives, and
`mc-sim`'s own CLAUDE.md forbids it a wall clock without a recorded decision —
which a debouncer needs.

**Recommendation: (b).** `mc_world::content::watch` holds the port, the
relevance rule and the `notify` adapter. `mc-world` gains `notify` and
`notify-debouncer-full` in `[dependencies]` and **must never let either escape
that one module** — the same structural claim `mc-world` already makes about
`mlua`.

**Strongest argument against.** `mc-world` is in the coverage denominator and an
adapter over a platform watcher is the hardest thing in this spec to cover. The
mitigation is that the adapter is thin by construction — build a debouncer,
drain a channel, hand over paths — with every decision except the window's
delivery on the `mc-sim` side of the port or in a pure function beside it.
Recorded in Risks.

**The relevance rule, concretely.** A path begins a reload attempt iff it lies
directly under `<root>/blocks/` with the block declaration extension, or
directly under `<root>/hud/` with the HUD declaration extension. Both
directories and both extensions are read from the sources that declare them,
promoted to `pub(crate)` within `mc_world::content`.
`content/base/materials/*.toml` therefore begins no attempt —
`mc_sim::content::load` does not read it (the only reader is `tools/voxforge`),
and a rule derived from the loader is the rule that says so. **FR-1.1-S9 pins
it**, and it carries its own discriminating half: the same instrument must begin
an attempt for `blocks/stone.luau`, because an absence alone is satisfied by a
watcher that never fires and by a relevance rule that has come to refuse
everything.

### D3 — The second write door into `World` is a child module; the orchestration that also needs the player lives with the player. **BINDING**

`crates/mc-sim/src/world/mod.rs` claims that nothing outside the module can
write any of its three views and that **exactly one function writes anything**.
That claim must change, and the spec is explicit that it must stay a door and
not a hole.

```rust
// crates/mc-sim/src/world/mod.rs — private to this module and its descendants,
// exactly as `write` is.
impl World {
    fn adopt(&mut self, registry: Arc<BlockRegistry>) -> Result<(), RegistryError> {
        let solid = SolidVoxels::resolve(&self.blocks, &registry)?;  // settled first
        self.solid = solid;
        self.registry = registry;
        Ok(())
    }
}
```

Three properties, each mirroring `World::write`:

1. **Solidity is settled before either write.** A candidate naming a block the
   world holds and the candidate does not refuses **without having changed
   anything** — which is what FR-2.2-S1 and FR-4.1-S5 assert, and it is the same
   sentence `write`'s doc comment already carries.
2. **`adopt` carries no `pub`.** The admission and the swap live in a **child**
   module, `mc_sim::world::reload`, declared `pub(crate) mod reload;` exactly as
   `pub(crate) mod action;` already is at `world/mod.rs:29`. Module visibility
   and item visibility are independent: `simulation` can reach the *module* while
   `adopt` stays invisible to it. A `pub(crate) fn adopt` would have opened a
   crate-wide write door, which is a different and much weaker claim.
3. **Nothing recomputes solidity separately.** A caller that swapped the
   registry and left the bitset to be refreshed would re-open the disagreement
   the type exists to make unspellable — **and the replay's overlap oracle
   cannot see it**, because that oracle re-reads the world through the registry
   and would be agreeing with itself. FR-4.1-S4 is the only instrument.

**The seam splits where the borrow does, and this draft first got it wrong.**
The original shape was `world::reload::apply(&mut Simulation, candidate)`.
That cannot compile: `Simulation`'s fields are private to `mc_sim::simulation`
(`simulation.rs:59-70`), and `mc_sim::world::reload` is a descendant of `world`,
not of `simulation`, so it sees none of them — forcing `Simulation` to grow
crate-visible mutable accessors for the world, the player *and* the content
swap, which widens exactly the type D3 exists to keep narrow. So:

- **`mc_sim::world::reload`** (child of `world`, holds the `adopt` privilege)
  takes only a `&mut World`: the names-held check, the held-block
  re-derivation, `adopt`, the geometry predicate and the dirty marking.
- **`Simulation::adopt`** (in `simulation`, where the player and the publication
  are) calls it, then clears the player, then bumps the serial and publishes.

Same structural claim, and no new door into `Simulation`.

**Why the whole-world solidity resolve, on the tick thread.** `SolidVoxels::resolve`
walks every voxel (16 × 256 × 16 per column, 1 048 576 in all) with
`LastResolved` run-coherence — the same call `World::new` already makes at
launch. It cannot move to the worker: the world changes while the build runs
(FR-1.2-S2), so a bitset resolved off-thread would be resolved against a world
that no longer exists. It cannot move off the tick boundary either, because a
swap *is* a tick boundary.

A **DEFERRED** refinement, recorded so it is not re-derived: only voxels whose
block's declared solidity changed can change a bit, so a section whose palette
holds no such name could be skipped wholesale. `code-quality.md` §8 rules that
out for now — make it work first. **Revisit when** the FR-9 benchmark shows the
swap is a material term in the one-second budget, or when the world outgrows a
fixed footprint.

### D4 — Two stages of refusal, one vocabulary, and the existing fault types carry nearly all of it. **BINDING**

The four failure kinds are **one enum reached from two stages**, and the split is
not stylistic — it is forced by what each stage can see.

| Stage | Where it runs | What it can see | What it refuses |
|---|---|---|---|
| **Build** | worker thread, `mc_sim::content::load` | the content root, and the layers already spent this session | a chunk that will not compile · a misspelled field · two files claiming one name · an emptied `blocks/` · a refused HUD declaration · a declaration that looped past the budget, allocated past the memory cap, or raised · a candidate needing a layer past 256 |
| **Admission** | tick thread, `mc_sim::world::reload` | the running world and the player | a candidate that declares no block the world holds · a candidate declaring no solid block at all |

**The build stage needs no new fault vocabulary at all**, and that is the
finding rather than a convenience. Everything in its row already arrives as
`ContentError` — `Blocks(RegistryError)` or `Hud(HudLoadError)` — carrying
`DefinitionFault { origin, block, field, cause }` underneath. FR-2.1-S1
(compile), S2 (`slid`), S3 (two files), S4 (emptied directory), S7 (HUD) and
FR-2.3-S1/S2 (memory cap, raised error) are the *same refusals a launch already
produces*, reached through the same call. That is why FR-2.3 is inherited
behaviour rather than new work.

Only three variants are new — **amended in phase 2 from four; this draft listed a
`ReloadRefusal::LayerBudget` and it cannot exist here.** The contradiction is
between this decision and D9: D9 puts the appending **inside `load`**, whose error
is `ContentError`, so the budget refusal originates one level below this enum. A
`ReloadRefusal::LayerBudget` carrying the sentence below would therefore be the
sentence spelled **twice** — once where the refusal is raised and once where it is
re-wrapped — which the spec's Declared Quantities section forbids by name.

**Resolution: the sentence lives on `LayerBudget` itself, in `mc-core` beside the
budget it names**, `ContentError` gains `Layers(#[from] LayerBudget)` with no
wording of its own — the pattern `Blocks` and `Hud` already follow — and
`ReloadRefusal::Content` reaches it through that. `mc-sim` then never names the
count at all, which is D5 working rather than D5 being worked around, and the
refusal a mod author reads is unchanged to the byte.

**The deciding argument is not "one decision in one place" but something
narrower: a refusal's wording that quotes a bound belongs beside the bound.** The
256 is a property of the layer budget, and the width it comes from
(`LAYER_BITS` → `MAX_LAYER` → `TEXTURE_LAYERS`) is what D5 already anchors to
`mc-core`. Wording one crate away from the number it quotes is how a message
comes to say 256 after somebody changes `LAYER_BITS`: the two go stale together
or not at all, and only co-location makes that true. `mc-client`'s `PreparationError` gains a transparent
`Layers` variant for the same reason its `Blocks`/`Hud` ones exist, unreachable
from a launch because a launch has spent nothing.

```rust
// crates/mc-core/src/content.rs — where the sentence lives
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error(
    "this content needs {needed} texture layers and a session has \
     {LAYERS_A_SESSION_MAY_ASSIGN}; {spent} are already assigned, and relaunching \
     reclaims every layer retired since the client started"
)]
pub struct LayerBudget { pub needed: usize, pub spent: usize }

// crates/mc-sim/src/reload/mod.rs
#[derive(Debug, Error)]
pub enum ReloadRefusal {
    /// Everything the content root itself was refused for, the layer budget
    /// included — it arrives underneath as `ContentError::Layers`.
    #[error("the content root could not be read")]
    Content(#[from] ContentError),

    /// The world holds blocks the candidate does not declare, ascending (FR-2.2-S3).
    #[error("the world holds {} that this content does not declare", named(blocks))]
    BlocksTheWorldHolds { blocks: Vec<BlockName> },

    /// No solid block, so a player would have nothing to place.
    #[error(
        "this content registers no solid block, so a player would have nothing to place; \
         the block a client holds is the first solid one in registration order"
    )]
    NothingToPlace,

    /// The thread building the candidate ended without producing one.
    #[error("the thread building the candidate ended without producing one or a refusal")]
    BuilderLost,
}
```

`NothingToPlace`'s sentence is `PreparationError::NothingToPlace`'s, unchanged
to the byte — the two say the same thing to the same person and a second wording
is a second place to disagree. `BuilderLost` mirrors `PreparationError::WorkerLost`.

**Each refusal leaves the running world untouched by construction, not by care.**
The build stage never touches the simulation; the admission stage settles
everything fallible before `adopt`, which itself settles solidity before either
write. There is no point at which a refusal can arrive half-applied.

**Reported once, however many attempts meet it.** `ContentReload` holds the last
refusal's rendered text and emits a `ReloadStep::Refused` only when it differs —
the exact shape of `App::report_remesh` (`app.rs:465`), which the spec's own
Existing Code table names as the contract. Comparing rendered text rather than
values is deliberate: it is what the tree already does, and the alternative
needs `PartialEq` on an error chain that does not have it.

### D5 — The 256-layer bound is declared in `mc-core` and asserted equal in `mc-render` at compile time. **BINDING**

The refusal in FR-5.2 has to be made in `mc-sim`, and the number comes from
`mc-render`'s packed vertex (`LAYER_BITS = 8`, so `MAX_LAYER = 255` and the
array texture is created at `depth_or_array_layers: MAX_LAYER + 1`). `mc-sim`
may not name `mc-render`.

**Options.** (a) Restate `256` in `mc-sim`. (b) Move the count to `mc-core`
and derive `LAYER_BITS` from it. (c) Declare the count in `mc-core::content`,
beside the value whose layers it bounds, and have `mc-render` assert agreement
at compile time.

(a) is two places for one decision, which the spec's Declared Quantities section
forbids by name. (b) inverts the causality — the field is 8 bits wide, which is
*why* the count is 256 — and would leave a reader of `vertex.rs` unable to see
where its own bound came from.

**Recommendation: (c).**

```rust
// crates/mc-core/src/content.rs
/// How many array-texture layers one session may assign. Eight bits of the
/// packed vertex carry a layer index (`mc_render::geometry::vertex`), so this
/// is a property of the content-to-renderer contract and not of either side.
pub const LAYERS_A_SESSION_MAY_ASSIGN: usize = 256;
```
```rust
// crates/mc-render/src/geometry/vertex.rs
const _: () = assert!(MAX_LAYER as usize + 1 == LAYERS_A_SESSION_MAY_ASSIGN);
```

`mc-render` already depends on `mc-core` (`TextureKey`, `ResolvedContent`). The
compile-time assertion is the same device `height.rs` already uses for the
lattice shift. **It is also what makes D15's claim true** — that the one
content-caused upload failure is unreachable, because the sim-side refusal and
the render-side bound cannot disagree.

### D6 — A layer is spent for the session; the assignment carries the live keys and the count spent. **BINDING**

**The reasoning that failed, recorded so it is not re-derived.** The first draft
made the assignment monotone by *keeping retired keys in it*, so the next free
layer would be `assignment.len()` and no separate counter would exist. That
fails **FR-5.1-S3** outright: a key removed and reintroduced is still "already
held", so it gets back the layer it had — the one outcome the scenario names and
forbids. The property it was reaching for is right; the representation was
wrong.

```rust
pub struct LayerAssignment {
    /// The layer each key the *serving* content names holds, ascending by layer.
    live: Vec<(TextureKey, u16)>,
    /// How many layers have ever been handed out this session. The high-water
    /// mark, and a primary field rather than a derived one — `live.len()` is
    /// what would be wrong, because a retired layer is spent and is not live.
    spent: u16,
}
```

`appending(keys)` gives each key of `keys`, ascending, the layer it already holds
in `live` or the next value of `spent`; the result's `live` is exactly `keys`.

| Scenario | Outcome |
|---|---|
| FR-5.1-S1 — `base:amber` introduced, sorting before `base:dirt` | `amber` takes `spent` = 4; dirt/grass/stone/water reuse 0/1/2/3 ✔ |
| FR-5.1-S2 — the only block naming a key is removed | that key leaves `live`; every remaining key reuses its layer ✔ |
| FR-5.1-S3 — the key is then reintroduced | it is not in `live`, so it takes `spent` — **the next unused layer, not the one it held** ✔ |
| FR-5.1-S4 — a section not re-meshed keeps `base:stone` at 2 | `stone` stayed live and reused 2, so re-packing resolves it to 2 again ✔ |
| FR-5.1-S5 — a refused candidate appends nothing | the value is never published, so `spent` never moves ✔ |
| FR-5.2-S2 — exactly the 256th layer | `spent` 255 + one key → the key takes 255, `spent` becomes 256 ≤ 256 ✔ accept |
| FR-5.2-S3 — 255 assigned, two keys needed | `spent` would reach 257 → refuse, and **neither** is appended, because `appending` is all-or-nothing ✔ |

**A retired layer keeps its texels and is not rewritten**, because `live` is what
reaches `TextureLayers::stated` and `write_textures` iterates only what it was
given. That is exactly what FR-5.1-S4 wants: vertices already on the GPU
referencing a retired layer go on sampling what was uploaded there. And
FR-5.2-S1's sentence — "relaunching reclaims every layer retired since the client
started" — is literally true: `spent - live.len()` of them.

**Still no ledger object.** The assignment is a value carried by the published
content, so **only an accepted candidate can spend the budget** structurally: a
refused candidate never reaches the publication and the next attempt reads the
same prior.

### D7 — What a reload re-meshes is binary: everything, or nothing. **BINDING**

The spec fixes the *rule*: "a reload meshes again only if some block's declared
solidity or declared texture key differs". It leaves the granularity to the
architecture, and one scenario decides it.

**FR-6.1-S3 asks for exactly 256 sections when all four declarations change.**
Verified against the tree: `SECTIONS_PER_COLUMN = 16` and `COLUMN_HEIGHT = 256`
(`mc-world/src/column.rs:17,23`), `FOOTPRINT_COLUMNS = 4` → 16 columns × 16 =
**256 sections**. Terrain runs `LOWEST_SURFACE = 32` to `HIGHEST_SURFACE = 48`,
sea to 34, and one landmark pillar at world position `(12, 12)` — local `(12,12)`
of column `(0,0)` — reaches `LANDMARK_TOP = 64`. **So the highest occupied
section is 3 in fifteen columns and 4 in column `(0,0)`, and everything above is
empty.** A selective rule — mark a section iff its palette holds a name whose
geometry-relevant declaration changed, plus its six neighbours — marks roughly
**82 of 256** and therefore **fails FR-6.1-S3**.

So the rule is binary:

> A candidate whose accepted content changes some block's declared `is_solid`
> or declared `texture`, or adds or removes a block, marks **every section of
> the world** for re-meshing. One that changes neither marks **none**.

**The over-marking is nearly free, and that is why this is not a compromise.**
The sections it adds beyond the selective rule are exactly the empty ones, which
mesh to zero quads. It is also the rule `World::mark_dirty` already runs on, in
its own words: "the correct thing rather than the fast thing; an over-marked
section costs an extra remesh, an under-marked one leaves a stale face on
screen."

**Predicate, stated as fields and not as a hash.** Compare the serving and
candidate registries by name: a name in one and not the other, or a name whose
`is_solid` or `texture` differs. `behaviour_of`/`appearance_of`
(`persistence/format.rs:307,319`) are *not* reused for it — `behaviour_of` folds
`replaceable`, `breakable` and `breaks_into`, which change no geometry
(FR-6.1-S4 pins exactly that), and both are `pub(crate)` to `mc-world`. The
save format's split is the right *idea* and the wrong *set*.

The spec's Technical Considerations now say **exactly** 256 rather than *at
most*, with this measurement beside them, so the loose reading can no longer
invite the selective design from the other side of the document.

**The instrument, and the number FR-6.1-S2 reports on it.**
`RemeshWork::keys()` is an `ExactSizeIterator` (`world/remesh.rs:38`) reachable
through `Session::take_remesh_work()`, and the dirty set is a `BTreeSet`, so a
whole-world mark yields exactly 256 distinct keys, each once. **FR-6.1-S2 is
satisfied as a superset and its value on that instrument is 256, not ~82.**

> **This paragraph must appear in `tasks.md`, and it names a victim and a wrong
> fix.** FR-6.1-S2's wording — "mesh again every section whose own or whose
> neighbours' blocks include stone" — is a **lower bound**, not a count. A test
> author who derives an expected number from it writes `assert_eq!(keys, 82)`,
> and that assertion **reddens against a conforming implementation**. It is the
> over-tight assertion `testing.md` §2 names — *red that should have been green*
> — and its cheapest repair is to narrow the marking rule, which breaks D7 and
> silently fails FR-6.1-S3 in the same commit. **The expected value for S2 on
> the shared instrument is 256.** What S2 grades is that the set *contains* the
> stone-bearing sections and their neighbours; what tells S2 from S1 is that one
> meshes something and the other nothing, which is FR-6.1-S4's paired control.

**DEFERRED:** the selective rule, with a revisit condition — a world that
outgrows a fixed footprint (the spec's own Assumptions already flag that it
reopens FR-6.1-S3's bound), or a benchmark showing whole-world marking dominates
the reload's latency. **Taking it is a spec change** — to FR-6.1-S3 and to the
Technical Considerations paragraph that now records the measurement — and not an
optimisation somebody may take while passing.

### D8 — The registry travels with the batch; staleness is decided where "serving now" is known. **BINDING**

FR-6.2 names two hazards the existing transport does not handle: a batch in
flight when a reload lands was meshed against superseded content, and the worker
must be told the new content *before* it meshes anything with it. The spec warns
that an ordering rule is the weak form of both. They get different answers, and
the first draft got the second one wrong.

**The registry travels with the batch — structural, and unchanged.**
`RemeshWork` gains `Arc<BlockRegistry>`, taken from the `World` that produced it;
`Retained` **loses** its `registry` field. A batch therefore cannot be meshed
against a registry other than the one its world was resolved against —
FR-6.2-S2 and FR-6.2-S5 become unspellable rather than checked. The batch grows
by one pointer-sized clone.

**The staleness comparison cannot live on the worker, and the first draft put it
there.** `Retained` is *moved into the worker thread* at `Remesher::spawn`
(`remesh.rs:75-86`), so a serial held beside the retained layers is only updated
when the worker dequeues a retirement message. Trace FR-6.2-S1: batch `B` is
dequeued at serial `S0`; the reload lands and a retirement is queued behind it;
the worker finishes `B` while still holding `S0`, compares `S0 == S0`, and packs
a scene against layers that have been retired. Worse, on one ordered channel the
mismatch branch is **unreachable in production** — every batch the worker
dequeues carries the serial it currently holds — so the variant would exist only
for an artificial test to construct, which is `testing.md` §2's definition of a
test that cannot fail for a real reason.

**So the comparison runs on the client, at collect time, where "serving now" is
actually known.** `Remesher` remembers the in-flight batch's keys and the serial
it was drained under, and holds the serial now serving, updated by `retire`:

```rust
pub enum Remeshed {
    Scene(Arc<SceneGeometry>),
    /// The batch was meshed against content that is no longer serving. The keys
    /// go back into the world's dirty set, or those sections stay stale for the
    /// rest of the run.
    Superseded { keys: Vec<SectionKey> },
    Failed(RemeshError),
}
```

`retire(layers, serial)` sends the new layers to the worker on the **same
ordered channel** the batches use — which is what makes "told before it meshes
anything with them" true of the *next* batch for free — and updates the serial
this side. FR-6.2-S1 and FR-6.2-S4 are then the same mechanism, and FR-6.2-S3 (a
batch that could not be meshed at all) is the existing `Failed` path, unchanged.

**Consequence, accepted and stated.** A reload that changes nothing geometric
still supersedes a batch that happened to be in flight, so those sections are
re-meshed although they were already correct. It costs one batch, it can only
re-mesh sections that were dirty anyway, and the alternative — a second
"geometry serial" tracking only reloads that changed geometry — is a second
counter to keep in step for a saving nobody can see. **DEFERRED**, revisit if a
profile shows reload-coincident batches matter.

### D9 — The content root is read through one door, which takes the layers already spent. **BINDING**

`mc_sim::content::load(root)` derives the layer assignment inside itself,
lexicographically, from scratch (`content.rs:151-155`). Under D6 the assignment
is session state, so it can no longer be a function of the root alone.

**Options.** (a) Take `resolved` out of `LoadedContent` and let each caller
assign layers. (b) Add a second entry point for the reload. (c) Give `load` the
assignment already spent.

(a) is refused by the tree's own history: `layers_of` was a shared resolver and
was **deleted** in SPEC-016 precisely so that the golden path and the launch path
could not derive an assignment separately. (b) is two doors and therefore two
answers.

**Recommendation: (c).**

```rust
pub fn load(root: &Path, spent: &LayerAssignment) -> Result<LoadedContent, ContentError>;
```

`prepare_scene` and `prepare_launch` pass `LayerAssignment::none()` — a launch
has spent nothing, which is a fact rather than a decision, and it makes the
property visible at the call. `LoadedContent` keeps all three fields.

**Verified, not assumed: the golden frames do not move.**
`BlockRegistry::texture_keys` returns a `BTreeSet<TextureKey>`
(`block/registry.rs:55`) and `resolved_from` does `.into_iter().zip(0..)`; owned
iteration of a `BTreeSet` is ascending, so today's numbering is lexicographic and
dense from 0. A fresh `LayerAssignment` appending over the same set in the same
order produces the identical pair list, byte for byte.

**One invariant needs an enforcement point.** `ResolvedContent::stating` is
public and infallible and takes arbitrary pairs; if its field becomes a
`LayerAssignment` documented as dense and ascending, `stating` is where a sparse
one would enter and `spent()` would silently lie. Every fixture in the tree is
dense (`stated_layers_are_honoured.rs:61,68`), so nothing breaks today.
**Decision: `LayerAssignment` is constructed only by `none()` and `appending()`,
and `ResolvedContent::stating` takes a `LayerAssignment` rather than pairs** — so
density is a property of the type's constructors rather than a comment.

### D10 — The adapter debounces and the domain coalesces; **neither test alone grades FR-1.1-S4**. **BINDING**

An editor's save is several filesystem events; a fast typist's saves are several
*changes* arriving while a build is in flight. These are different problems and
they get different mechanisms.

**The adapter debounces** at the declared 150 ms settling window, absorbing one
save's event storm into one report. That is `notify-debouncer-full`'s job and it
is the only place a clock is read.

**The domain coalesces** with a single `pending: bool`. On a relevant change,
`pending = true`. At a tick boundary with nothing in flight and `pending` set:
clear it, then start the build. The order is the whole of it — a change arriving
during the build sets the flag again, and the boundary after the build ends
starts exactly one further attempt.

**Queue, coalesce or refuse — the answer is coalesce, and the reason is not
taste.** A candidate is built from the **whole root** (FR-1.2-S1), so any build
started after the last change observes every change before it. N pending changes
therefore collapse to at most one further build with no information lost. A
queue would run N builds for N saves and publish N serials for one edit;
refusing would drop an edit silently, which is the worst outcome available. And
a change landing in the window between clearing the flag and the worker's
`read_dir` costs one extra build that reads identical content — accepted, with a
new serial, which FR-4.4-S2 already blesses.

**The first draft claimed the domain's coalescing grades FR-1.1-S4. It does
not, and the arithmetic says why.** One tick per rendered frame, so tick
boundaries are roughly 16 ms apart; the settling window is 150 ms, about nine
boundaries. Five writes genuinely spread across one window therefore reach five
*different* boundaries and begin five attempts unless the debouncer absorbed
them first. Coalescing only collapses reports that arrive between two ticks.

So FR-1.1-S4 is graded by **two instruments and needs both**:

1. **The domain's coalescing**, driven through the in-memory double — reports
   delivered between two boundaries begin one attempt. This is a real assertion
   about `ContentReload`, not agreement with the double, because the double
   holds no policy of its own.
2. **The window's value at the boundary it crosses** — a test asserting that the
   `Duration` handed to the debouncer builder *is* `SETTLING_WINDOW`. Without
   it, passing `Duration::ZERO` leaves the constant declared once (FR-9.1-S2
   green), the coalescing test green, and the shipped client beginning five
   attempts per save. That is `testing.md` §2's "when a decision is about what
   crosses a boundary, assert it at the boundary", and it needs no filesystem
   and no timer.

`tasks.md` must carry both, and say that neither covers the other.

**The window is declared once**, as a `Duration` beside the port in
`mc_world::content::watch`, read by the adapter. FR-9.1-S2's scan reports an
enumerated verdict — `DeclaredExactlyOnce` / `DeclaredIn(Vec<String>)` /
`NoSourceWasRead` — following `client_names_no_content_door.rs`'s three-way
shape, so a scan that read nothing cannot pass for a clean one.

### D11 — A cleared player is placed at a cell's centre, searched sideways-first over a bounded cube, never downward. **BINDING**

FR-7's search, fully specified because a wrong tie-break is invisible.

- **Candidates are cell centres**, not whole-block offsets from where the player
  stood: feet at `(cx + 0.5, cy, cz + 0.5)` for a cell `(cx, cy, cz)`. The box
  is 0.6 wide (`HALF_WIDTH = 0.3`), so a centred box spans `[x+0.2, x+0.8]` and
  lies strictly inside one cell column — which makes clearance a question about
  the two cells the 1.8-tall box occupies rather than about four. **Cost, stated
  because a player will notice it:** being cleared loses their sub-block
  position. Not being cleared leaves it exactly (FR-7.1-S3, which is a *no move
  at all* rather than a move to where they already were).
- **The cube is `dx, dz ∈ [-8, 8]` and `dy ∈ [0, 8]`.** Downward is not ranked
  last, it is **absent** — "never downward" (FR-7.1-S4) as a property of the
  candidate set rather than of an ordering. The spec's declared cost ceiling is
  17³ = 4 913 positions; this spends 9 × 17 × 17 = 2 601 of it, which is a
  deliberate narrowing under the stated bound and not a different bound. The
  test author should assert **reachability** — a clear space at exactly 8
  sideways is found and one at 9 is not — never a position count.
- **Order: `(dy, max(|dx|, |dz|), dz, dx)` ascending.** `dy` first is what makes
  FR-7.1-S2 come out sideways when a sideways and an upward cell are both one
  away. Chebyshev horizontal distance matches the cube the bound describes. The
  last two are a declared tie-break, so two runs agree.
- **The predicate is `collide::overlaps`**, promoted to `pub(crate)`, so
  `HALF_WIDTH`, `HEIGHT` and the half-open `[v, v+1)` rule are stated once and
  read here — the spec's Existing Code table names `collide.rs` for exactly
  this.
  **AMENDED IN PHASE 5 — the named function could not be promoted and what
  replaced it is stronger.** `overlaps` takes an `Aabb`, which is private to
  `collide.rs`, so promoting it alone would have meant promoting the box type and
  its constructor too — three items, and the box's shape leaving the physics.
  Promoted instead: **`collide::overlaps_solid(feet, world)`**, the whole question
  in one call, plus `collide::cell_of` for the floor rule. `Aabb` stays private and
  `clearing.rs` restates nothing.
- **Velocity is zeroed on any clearing move**, not only an upward one. FR-7.1-S7
  demands it for upward; doing it for every move is one rule instead of one per
  direction, and a cleared player has been teleported. It does not touch
  FR-3.2-S3, which is about a player who was *not* cleared.
- **The answer is an enumerated verdict**, so "could not be cleared" is a value
  a caller reports (FR-7.1-S5) rather than an absence:
  `Clearing::{ Unneeded, MovedTo(Vec3), NoClearSpaceWithin { blocks: u32 } }`.
- **It runs after `adopt`**, against the solidity the candidate produced, and it
  is reached only from the accepted path — so a refused candidate moves nobody
  (FR-7.1-S6) because the code never runs.
- **The verdict reaches a person.** FR-7.1-S5 requires the system to *report*
  that a player could not be cleared, so `Clearing` travels out through
  `ReloadStep::Accepted` and `ReloadReport::Accepted` to the one place that
  prints (D14). A verdict computed and dropped satisfies nothing.
- **DECIDED, and it is FR-7.1-S8's subject. A candidate is eligible only if every
  cell the player's box would cover is *known and clear*.** Outside the loaded
  world is **unknown, not clear**, and a search over unknown ground is not a
  search. Found in phase 5, ruled in phase 7, and **implemented in phase 8** —
  which is why this bullet is a decision rather than a note: the reach is 8 blocks
  and the shipped footprint is 64 square, so any player trapped within 8 blocks of
  an edge has candidates outside the world, in a wedge those are the nearest
  "clear" ones the ring search meets, and the player is put where nothing is solid
  and falls out of the world.

  **Two alternatives refused, with the reasons, because both look cheaper.**

  - *Treating outside-the-world as solid* is a lie told in the world model.
    `is_solid` is read by collision, meshing and the physics, so the change is not
    local, and it asserts a fact the model does not have. It also **inverts the
    moment the world streams**: an unloaded neighbour is unknown, not solid, and
    code that learned to read `true` there will refuse legitimate moves.
  - *Accepting that a reload can put a player off the map* is refused outright. It
    is a silent, player-visible failure of exactly the kind this spec exists to
    prevent — a reload reported as succeeding, after which the player is falling
    through nothing.

  **`is_solid` does not change, and eligibility is sited in the clearing search
  alone.** The predicate the search asks becomes *known clear* rather than *not
  solid*, which is the only one of the three answers that stays true when the world
  becomes streamed and effectively unbounded. That is what the eligibility check
  is worth paying for.

  **No new vocabulary.** A boundary wedge with no eligible candidate takes the
  refusal path this decision's enumerated verdict already carries and FR-7.1-S6
  already grades. And FR-7.1-S5's two-column, 32-block world **stops being a
  fixture workaround and becomes the rule** — recorded as such, because "the author
  worked around a hole" and "the fixture satisfies the eligibility rule" are
  different statements and only the second is now true.

  The general form of the unsoundness — `is_solid` answering `false` past the
  footprint wherever it is consulted, not only here — is a deferred observation in
  `docs/technical/architecture.md`, and belongs to whichever spec makes the world
  streamed.

### D12 — The content is published through a second `ArcSwap` beside the snapshot; `SimSnapshot` stays `Copy`. **BINDING**

```rust
// crates/mc-core/src/content.rs
pub struct ContentSerial(u32);   // saturating, exactly as the tick counter is

// crates/mc-sim/src/simulation.rs
pub struct Simulation {
    published: ArcSwap<SimSnapshot>,
    content: ArcSwap<PublishedContent>,   // new
    player: PlayerState,
    world: World,
}

pub struct PublishedContent {
    pub serial: ContentSerial,
    pub resolved: ResolvedContent,
    /// The HUD the same root declared. It travels because it is refused with the
    /// blocks (FR-2.1-S7), so applying one and not the other is the partial
    /// application invariant 7 calls a Blocker. **FR-4.5-S1 is what stops a
    /// later reader deleting this field as unused.**
    pub hud: Arc<HudLayout>,
}
```

**The HUD's two halves are graded by two instruments, and the split is the
layer upload's exactly.** That the *published* content carries the widened
element is assertable with no device — assert the value where it crosses the
boundary. That the widened element reaches a *drawn* frame goes through
`crates/mc-client/tests/support/hud_frames.rs`, which composes a HUD over a
frame through the client's own frame call on a device with no window. Neither
covers the other's half, and the residue — `App` assigning the new layout to its
own `hud` field — is held by review, as `App`'s share of an edit already is.

The serial is **not** put inside `SimSnapshot`: that type is `Copy` and holds
plain values, and `mc-sim`'s CLAUDE.md warns that changing its shape reopens the
publication hole silently. Nothing needs the correlation — a batch carries its
own serial (D8), and a reader that wants the content asks for it.

`u32` with `saturating_add`, mirroring the tick counter's own choice, rather
than `u64`: the two counters answer to the same reader and a second width would
be a second convention.

**Readers observe by asking, not by being told** (FR-8.1-S2). `Simulation::content()`
returns `Arc<PublishedContent>`; a reader that has not looked since the last
accept goes on seeing what it last observed. That is what keeps FR-8.1 an
arrangement rather than a callback, and it is the same shape as `latest()`.

`Simulation::new` grows a third parameter — the launch's published content — so
that a simulation is never in a state where it has a world and no content. That
touches `launch.rs` and every test that constructs a simulation; it is recorded
in Integration.

### D13 — `mc_world::section::Section` gains `names_in_use`. **BINDING**

FR-2.2-S3 needs **every** block the world holds that the candidate does not
declare, ascending. `SolidVoxels::resolve` reports only the first, and reporting
one at a time makes an author fix a rename in as many saves as they have blocks.

A palette holds entries no voxel names any more, so reading `Section::palette()`
directly would refuse a reload over a block the player broke ten minutes ago —
a defect, not a cost. `Palette::surviving_entries` already answers exactly this
from the reference counts the write path maintains, in **O(palette length)**
rather than O(4 096), so:

```rust
// crates/mc-world/src/section/mod.rs
pub fn names_in_use(&self) -> impl Iterator<Item = &BlockName>;
```

and in `mc-sim`, `World::names_held(&self) -> BTreeSet<&BlockName>` over the
world's columns. The whole check is 256 sections × a handful of palette entries.
`SectionData::compacted()` is deliberately not reused: it allocates 4 096
indices per section to answer the same question.

### D14 — The client's reload wiring is a new file, named now rather than discovered at the gate. **BINDING**

Measured at `877af4a` with the gate's own counter (non-blank lines, 500 limit):
`crates/mc-client/src/session.rs` is at **492** — eight lines of headroom — and
`app.rs` at **456**. D1's residue is 60–120 lines at this codebase's doc-comment
density. Discovering that at 501 lines mid-phase costs a restructuring made to
satisfy a count, which `docs/technical/testing.md` warns invents a boundary the
design did not have.

**`crates/mc-client/src/session/reload.rs`, a child of `session`** — *amended in
phase 1; this draft said `crates/mc-client/src/reload.rs`, a sibling of
`remesh.rs`, and that siting could not compile.* It holds: the `ReloadReport`
type, the refusal reporting and its dedup, rebuilding `TextureLayers` from the
published content, and what an accepted reload hands the renderer and the re-mesh
worker. `Session` keeps only the `ContentReload` field, the drive after
`advance`, and `take_reload_report` — and its reload surface lives in a second
`impl Session` block in that file, which is what the sibling siting made
impossible.

**Why the sibling siting was wrong, recorded because the reasoning is what a
later reader would otherwise re-derive.** Rust privacy is module-scoped: a
sibling of `session` cannot see `Session`'s private `simulation` and `holding`
fields, so the fallback clause this decision originally offered — a second
`impl Session` block in that file — would not have compiled. The drive has to
write the re-derived held block back into `holding`, so the only alternatives
were a child module or a crate-visible accessor for the field `session.rs`'s
header exists to keep private. The child module keeps the size limit, the
no-borrow property and this decision's responsibility split; the accessor would
have given up a structural claim to buy a file layout.

The boundary is by responsibility and not by count: `session/mod.rs` is what a
keystroke decides, `remesh.rs` is what an edit becomes, and this is what a
content change becomes. It would have been the right split at 400 lines too.

**Measured after phase 1: `session/mod.rs` is at 496 of 500 non-blank lines.**
Four lines of headroom. A later phase needing a line in `mod.rs` splits
deliberately and says so; shaving a file header to buy four lines deletes the
reason a file exists in order to satisfy a size limit, and the limit is not what
that header protects.

**AMENDED IN PHASE 4 — `app.rs` crossed the limit too, and this decision's rule
was applied to it unchanged.** At 540 non-blank lines it became
`app/mod.rs` (482) plus **`app/reload.rs`** (71), holding what a content change
becomes on the device: the report, the texture upload and its run-ending policy,
the layer retirement, and the HUD assignment. A **child** module for the reason
above — it writes `App`'s own fields — and by the same responsibility boundary
rather than by count. `session/mod.rs` ended the phase at 485.

**Both splits found their seam already written in the file's own header**, which is
recorded as as-built practice in `tasks.md` for `docs/technical/`. That is why
neither boundary had to be invented to satisfy a count.

### D15 — An upload of the **texture layers** that fails after an accepted swap ends the run. **BINDING, amended in phase 4**

Nothing in the spec covers it, and the default is whatever the implementer types
in the one file nothing grades. Today `upload_textures` `?`-propagates into a
run-ending `PreparationError::Upload` (`app.rs:397`, reached from `redraw` at
`:168`). After this spec the swap has *already happened* when the upload runs —
the simulation is serving new content and the array texture is not, which is the
one state the all-or-nothing driver exists to forbid.

**Decision: end the run, on the same grounds `PreparationError::Upload` already
does.** A world serving content the device never received is a wrong picture
with no error, which is the outcome this project ranks lowest. The alternative —
report and draw on — is the `report_remesh` trade, and it is right for a *batch*
because a stale section is a stale picture of the same content; it is wrong here
because the content itself has moved.

**AMENDED IN PHASE 4 — WHICH UPLOAD. This decision governs the *texture* upload
only.** "An upload that fails after an accepted swap" silently covered two uploads
with two policies, and an unqualified sentence that happens to be applied
correctly today is a defect waiting for the next reader. The **scene** upload after
a reload goes through `App::show`, which reports and drops — and `SceneTooLarge` is
**genuinely reachable** there, since a whole-world re-mesh can exceed `MAX_QUADS`.
Both policies are deliberate; only the texture one is this decision.

**Recorded for validation, because it is the first of its kind in this spec:** on
that reachable `SceneTooLarge` path, a reload the player was *told succeeded*
leaves them looking at a stale or partial world plus a log line. At this world size
that is accepted and no scenario covers it.

**AMENDED IN PHASE 4 — THE REASON BELOW WAS WRONG AND IS STRUCK THROUGH RATHER
THAN DELETED.** Not the annotate-an-observation case: a dated measurement that
turns out to be superseded is still evidence, but **a design justification resting
on a false premise is not evidence of anything**, so it is preserved only so
nobody reconstructs it.

> ~~**The one content-caused failure is unreachable**, and that is what makes this
> a device decision rather than a mod-author one: `write_layer` refuses a layer at
> or past capacity (`gpu/buffers.rs:176`), and D6's budget refuses such a candidate
> first. D5's compile-time assertion is what stops the two bounds disagreeing.~~

**Why it failed:** it rests on a *classification of causes* into content-caused and
not, which is a claim about the world, and it invited exactly the question "which
upload failures are genuinely unreachable, and which are merely not
content-caused?" A taxonomy is only as good as its completeness, and nothing
established that this one was complete.

**The true reason is structural, and checkable by reading two functions:**
`upload_textures` **has no reachable failure after a swap at all.**

- `RendererError` has exactly two variants and **both are CPU-side bound checks
  taken before anything reaches the queue** — `TextureLayerOutOfRange`
  (`u32::from(layer) >= TEXTURE_LAYERS`) and `SceneTooLarge`. `queue.write_texture`
  and `queue.write_buffer` return `()`.
- **Device loss is reachable and does not travel through here.** It arrives via
  `set_device_lost_callback` → `Gpu::is_device_lost` → `SurfaceErrorKind::DeviceLost`
  → `FrameAction::Fatal` → `Ending::Frame`, and already ends the run by its own
  path with its own message.
- **Out-of-memory on a large append is unreachable because an append allocates
  nothing.** The array texture is created once at `MAX_LAYER + 1` depth in
  `SceneBuffers::new`; a reload writes texels into layers that already exist.

That claim is strictly stronger than the struck-through one, because it does not
depend on a cause taxonomy being complete. **D5's compile-time assertion keeps its
own job** — stopping the sim-side bound and the render-side capacity disagreeing —
rather than being what makes this decision safe.

### Trivial decisions, one line each

- The build worker is `std::thread::spawn`, polled with `is_finished()` then
  `join()`, exactly as `App::collect_preparation` polls the preparation — no new
  mechanism, and `join`'s `Err` becomes `ReloadRefusal::BuilderLost`.
- No attempt begins before a simulation exists; a change reported first is
  remembered by the same `pending` flag (FR-1.3-S3).
- The swap runs **after** `Simulation::advance` has published its tick, so the
  change is in force from the next tick (FR-1.3-S1, FR-3.2-S2).
- An unwatchable root (FR-1.1-S8) is reported once and yields no changes
  thereafter; nothing retries, because nothing in the spec asks it to.
- `Session::tick` keeps its `Option<EditReport>` signature; the reload's outcome
  is stashed and taken by `Session::take_reload_report()`, the same take-once
  shape `pending_action` already uses.
- **`Session` writes the re-derived `holding` back into its own field** when a
  reload is accepted. Nothing else does, and FR-3.3-S2 and FR-3.3-S4 both fail
  without it.
- `content/base/` gains no file. The demonstration is an author's edit.

---

## Interfaces

### `mc-core` (new)

```rust
// crates/mc-core/src/content.rs
pub const LAYERS_A_SESSION_MAY_ASSIGN: usize = 256;

/// Which array-texture layer each key the serving content names holds, and how
/// many layers this session has spent. Constructed only by `none` and
/// `appending`, so density and ascending order are properties of the type.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LayerAssignment { live: Vec<(TextureKey, u16)>, spent: u16 }

impl LayerAssignment {
    pub fn none() -> Self;
    /// This assignment with each key of `keys` on the layer it already holds, or
    /// on the next unspent one. **All or nothing**: a candidate introducing two
    /// keys with one layer free appends neither (FR-5.2-S3).
    ///
    /// # Errors
    /// Returns the count needed and the count already spent when the result
    /// would not fit `LAYERS_A_SESSION_MAY_ASSIGN`.
    pub fn appending(&self, keys: &BTreeSet<TextureKey>) -> Result<Self, LayerBudget>;
    pub fn layer_of(&self, key: &TextureKey) -> Option<u16>;
    pub fn spent(&self) -> u16;
    pub fn entries(&self) -> impl Iterator<Item = (&TextureKey, u16)>;
}

pub struct ContentSerial(u32);
impl ContentSerial { pub const FIRST: Self; pub fn next(self) -> Self; pub fn get(self) -> u32; }
```

`ResolvedContent` holds a `LayerAssignment` and `stating` takes one;
`layer_assignment()` keeps its signature, so `ContentView` is untouched.

### `mc-world` (new)

```rust
// crates/mc-world/src/content/watch/mod.rs
/// How long an editor's save is allowed to settle before a change is reported.
/// **Declared here and nowhere else** (FR-9.1-S2).
pub const SETTLING_WINDOW: Duration = Duration::from_millis(150);

/// What has changed under a content root since this was last asked.
pub enum ContentChanges {
    Nothing,
    /// Paths under the root, in whatever order the platform reported them.
    Changed(Vec<PathBuf>),
    /// The root could not be watched at all.
    Unwatchable { directory: PathBuf, cause: String },
}

pub trait ContentWatch { fn changes(&mut self) -> ContentChanges; }

/// Whether a path is one `mc_sim::content::load` would read: directly under
/// `blocks/` with the block declaration extension, or directly under `hud/`
/// with the HUD declaration extension. Built from the two sources' own
/// constants, which share a name and are disambiguated at the import.
pub fn declares_content(root: &Path, path: &Path) -> bool;

// crates/mc-world/src/content/watch/notify_watch.rs
pub struct NotifyContentWatch { /* debouncer + receiver */ }
impl NotifyContentWatch { pub fn watching(root: &Path) -> Self; }
impl ContentWatch for NotifyContentWatch { .. }

// crates/mc-world/src/section/mod.rs
impl Section { pub fn names_in_use(&self) -> impl Iterator<Item = &BlockName>; }
```

### `mc-sim` (new and changed)

```rust
// crates/mc-sim/src/content.rs — CHANGED signature (D9)
pub fn load(root: &Path, spent: &LayerAssignment) -> Result<LoadedContent, ContentError>;
// CHANGED in phase 2: a third variant, carrying `LayerBudget`'s sentence rather
// than restating it. See D4's amendment.
pub enum ContentError { Blocks(RegistryError), Hud(HudLoadError), Layers(LayerBudget) }

// crates/mc-sim/src/persistence.rs — CHANGED in phase 2
// `simulation_at_launch` was already at the four-argument limit and `content` is
// the fifth thing a launch takes, so the four beyond the save become a group.
pub struct Launching { pub seed: u64, pub registry: Arc<BlockRegistry>,
                       pub content: PublishedContent, pub accepting: Acceptance }
pub fn simulation_at_launch(save: &Path, launching: Launching) -> Result<Simulation, LaunchError>;
// and, for the same reason, `mc_sim::replay::simulation_for` grows a third
// argument and `mc_client::launch::simulation_to_play` becomes (save, Launching).

// crates/mc-sim/src/world/reload.rs — child of `world`; takes only a `&mut World`
// AMENDED in phase 1: it takes the candidate's registry rather than the whole
// `LoadedContent`, because `BlockRegistry` is not `Clone` and the HUD and the
// resolved content belong to the publication, which is `Simulation::adopt`'s.
pub(crate) struct Adopted { pub holding: BlockName }
pub(crate) fn adopt_candidate(world: &mut World, registry: Arc<BlockRegistry>)
    -> Result<Adopted, ReloadRefusal>;

// crates/mc-sim/src/world/clearing.rs
pub enum Clearing { Unneeded, MovedTo(Vec3), NoClearSpaceWithin { blocks: u32 } }
pub(crate) fn cleared(feet: Vec3, world: &dyn Solidity) -> Clearing;

// crates/mc-sim/src/simulation.rs — where the player and the publication are
pub struct Accepted { pub serial: ContentSerial, pub holding: BlockName, pub clearing: Clearing }
impl Simulation {
    pub fn content(&self) -> Arc<PublishedContent>;
    pub(crate) fn adopt(&mut self, candidate: LoadedContent) -> Result<Accepted, ReloadRefusal>;
}

// crates/mc-sim/src/reload/mod.rs — the policy
pub struct ContentReload { /* root, watch, pending, in_flight, reported */ }

impl ContentReload {
    pub fn watching(root: PathBuf, watch: Box<dyn ContentWatch>) -> Self;
    /// Called once per tick, after the tick has been advanced.
    #[must_use]
    pub fn at_tick_boundary(&mut self, simulation: &mut Simulation) -> ReloadStep;
}

#[must_use]
pub enum ReloadStep {
    Nothing,
    /// Emitted only when it differs from the last one reported (FR-2.1-S8).
    Refused(ReloadRefusal),
    Accepted(Accepted),
}

/// The one door a client goes through, so the client itself names no watcher.
pub fn watching_shipped_content(root: PathBuf) -> ContentReload;
```

Every signature is inside the four-argument limit, receiver included.

### `mc-client` (changed)

```rust
// session/reload.rs — NEW FILE (D14, amended in phase 1)
pub enum ReloadReport {
    Refused(String),
    Accepted { content: Arc<PublishedContent>, clearing: Clearing },
}

// session.rs
impl Session {
    pub fn attach_reload(&mut self, reload: ContentReload);
    pub fn take_reload_report(&mut self) -> Option<ReloadReport>;
    pub fn mark_for_remesh(&mut self, keys: Vec<SectionKey>);
}

// remesh.rs
pub struct Retained { pub meshed: Vec<SectionQuads>, pub layers: TextureLayers }
impl Remesher { pub fn retire(&mut self, layers: TextureLayers, serial: ContentSerial); }
pub enum Remeshed { Scene(Arc<SceneGeometry>), Superseded { keys: Vec<SectionKey> }, Failed(RemeshError) }
```

### Error contract

Every refusal a reload produces renders through the existing chain and adds no
wrapper that restates its cause. `ReloadRefusal::Content` is `#[error(...)]`
over `ContentError`, which is already `#[from]` over `RegistryError` and
`HudLoadError`, which already carry `DefinitionFault { origin, block, field,
cause }`. A refusal quoted on `docs/modding/hot-reload.md` is held to a real run
by `crates/mc-client/tests/documented_refusals.rs`, extended in the phase that
adds each refusal rather than in a documentation phase at the end.

---

## Data

No persisted format changes. Two runtime values are new and one changes shape:

| Value | Where it lives | Lifetime |
|---|---|---|
| `LayerAssignment` | inside the published `ResolvedContent` | the session; `spent` is monotone (D6) |
| `ContentSerial` | inside `PublishedContent`; copied into every `RemeshWork` and remembered by `Remesher` | the session; saturating `u32` |
| `RemeshWork` | produced by `World::take_remesh_work` | one batch; **gains** `Arc<BlockRegistry>` and `ContentSerial` |

**The save format is untouched and that is the point.** A save written after a
reload records `behaviour_of`/`appearance_of` of whatever registry the world
then holds, so FR-3.4-S1's silent resume is a property of the swap having
reached the world, not of anything new on disk.

**Retention:** none. Nothing sensitive, nothing persisted, nothing logged beyond
refusal text that content authors wrote themselves — which is already
script-controlled text at whatever length a mod chose, exactly as
`LuauFileDefinitionSource::printed` is.

---

## Integration

| File | What connects | What must not break |
|---|---|---|
| `crates/mc-sim/src/world/mod.rs` | `adopt`, private; `names_held`; `mark_for_remesh`; `pub(crate) mod reload` | The module header's claim that exactly one function writes anything. **It must be rewritten, not left standing**: two functions write, and each settles solidity before either write. `write` and `adopt` both keep no `pub`. |
| `crates/mc-sim/src/simulation.rs` | second `ArcSwap`, `PublishedContent`, `content()`, `adopt()`; **`Simulation::new` grows a third parameter** | Every construction site — `launch.rs`, the replay suites, `mc-sim`'s own tests — must be updated. `SimSnapshot` stays `Copy` and free of interior mutability. |
| `crates/mc-sim/src/content.rs` | `load` gains a parameter; `resolved_from` appends instead of deriving | A fresh assignment must reproduce today's lexicographic numbering exactly, or every committed golden frame moves. |
| `crates/mc-client/src/{startup,launch}.rs` | both pass `LayerAssignment::none()`; `PreparedLaunch` carries the launch `PublishedContent` | `prepare_scene` and `prepare_launch` must go on packing byte-identical scenes — `crates/mc-client/tests/launch_and_capture_agree.rs` is the instrument. |
| `crates/mc-client/src/startup.rs` `scene_of` | doc comment | Its premise — "the registry does not change mid-session" — becomes **false**; its conclusion is FR-5.1-S4 and survives. Rewrite it, do not leave it to mislead. |
| `crates/mc-client/src/remesh.rs` | `Retained` loses `registry`; `retire` as a channel message; `Remeshed` becomes an enum; in-flight `(keys, serial)` | **The batch-size claim this row is about lives in `crates/mc-sim/src/world/remesh.rs:10`, not here** — corrected at `/sdd-tasks`, because the wrong path sends phase 4 to the wrong crate to update a figure that becomes false. ~~Its "a batch is seven of them per edited section at worst, so the copy is well under 100 KB" becomes false — a whole-world batch is 256 sections, roughly 1.3 MB.~~ **AMENDED IN PHASE 4: THE CLAIM DOES NOT BECOME FALSE, AND ~1.3 MB WAS WRONG BY AN ORDER OF MAGNITUDE.** Measured over the generated world: 256 sections, **54 carrying indices, 45 568 bytes — 44.5 KB**, widest 4 bits per index; under 70 KB with palettes and map nodes. A section whose palette holds one entry needs **zero** bits per voxel, and the shipped world is uniform above the terrain and uniformly stone below it, so only the sections straddling a boundary pay anything. **The bound holds for a reason that has nothing to do with section count: index-width tiering.** T18 keeps the claim and records why, with the figure marked measured. Following this instruction literally would have written a false number into a code comment. |
| `crates/mc-client/src/session/reload.rs` | **new file** (D14, **amended in phase 1** — a sibling of `remesh.rs` cannot see `Session`'s private fields, so it is a child of `session`) | It is the client's whole share of a reload apart from the drive in `Session` and the upload in `App`. |
| `crates/mc-client/src/app.rs` | takes the report; `upload_textures`; **assigns the published `HudLayout` to its own `hud` field**, the same assignment `collect_preparation` already makes at launch; forwards `Superseded` keys; ends the run on a failed upload (D15) | `App`'s share must stay two assignments, an upload and a report. Ask of every function this spec adds: what calls this, and what goes red if it stops? |
| `crates/mc-client/src/session/mod.rs` | owns `ContentReload`; drives it after `advance`; writes `holding` back | **496 of 500 non-blank lines already used after phase 1** (492 as `session.rs` before the split). `Session` hands out no borrow of what it owns. The `ContentReload` field, the drive and `take_reload_report` go in `session/reload.rs`, which sees the private fields; four lines is not room for them here. |
| `crates/mc-render/src/gpu/buffers.rs` | none, but read it before sizing anything | `write_textures` iterates **every** entry per call and `write_layer` is private with no single-layer path, so "append one layer" is a rewrite of every live layer. That is acceptable and it is why the per-reload upload cost is what it is. |
| `crates/mc-render/src/geometry/vertex.rs` | compile-time assertion against `mc-core`'s constant | `MAX_LAYER` stays derived from `LAYER_BITS`. The assertion is a check, not a redefinition. |
| `crates/mc-world/Cargo.toml` | `notify`, `notify-debouncer-full` | Neither may be named outside `src/content/watch/notify_watch.rs`. `crates/mc-world/tests/dependency_graph.rs` should be re-read in the watcher's phase. |
| `crates/mc-client/tests/client_names_no_content_door.rs` | two needles added — the watcher adapter's constructor and the vendor's own spelling — with the fixture naming **every** door | The fixture derives its expected report from the needle list. A needle added without a fixture entry is a needle nobody has watched match anything. |
| **FR-8.2-S2's scan** | the capture pipeline's own sources — `crates/mc-client/src/startup.rs`, the golden suites and `mc-testkit` — scanned for the watcher door with the same three-way verdict | This is a **different root** from the client-source scan, not an extension of it. FR-8.2-S3 is its positive control. Whichever way it is built, it must be able to tell a clean pipeline from sources it could not read. |

### Documentation (Key Principle 3, part of done)

The spec's Documentation deliverable is adopted unchanged and is not restated
here. Four additions the architecture forces:

- **`docs/technical/architecture.md`** must record, beyond the spec's list: that
  the second write door is a *child module* and why (D3), that a batch carries
  its registry while staleness is decided on the client (D8), that a retired
  layer is spent but not live (D6), and that `mc-world` names the watcher vendor
  in exactly one file (the Boundaries litmus test).
- **`crates/mc-sim/CLAUDE.md`** — its "No wall clock" rule demands a recorded
  decision before anything reads one. The decision is D2: the clock is the
  debouncer's, it lives behind a port in `mc-world`, and `mc-sim` still reads
  none. Record it there rather than only here.
- **`docs/technical/rendering.md`** — beside the spec's list, that a retired key
  keeps its layer and its texels for the session, and what that costs.
- **`docs/modding/hot-reload.md`** — beside the spec's list, that being moved
  clear costs a player their sub-block position (D11), and that a reload also
  applies the HUD the same root declares (D12).

---

## Assumptions

A reviewer may veto any of these; each stands in for something no scenario
states.

**Two of this draft's assumptions are no longer here, and that is the outcome
they were for.** "An accepted reload applies the HUD the same root declared" and
"a change under `content/base/materials/` begins no attempt" were assumptions
until the owner ruled they were holes; they are now **FR-4.5-S1** and
**FR-1.1-S9**. An assumption that names a property the design answers and
nothing asserts is a scenario that has not been written yet, and the honest end
of it is a scenario rather than a longer list here.

1. **A candidate is admitted against the world as it is at the swap, not as it
   was when the build began.** A player can place `base:stone` during a build,
   so FR-2.2-S1 evaluated at build time would accept a candidate that then fails
   `SolidVoxels::resolve` at the swap. This is why admission is a second stage
   (D4) and it is load-bearing.
2. **A byte-identical candidate is accepted and publishes a new serial.**
   FR-4.4-S2 says so directly; this assumption is that it also holds for the
   *extra* build D10's race can produce, which nothing observes as different.
3. **A reload attempt while the launch preparation is still in flight is
   deferred, not run.** Two readers of one root is harmless, but accepting a
   candidate before the launch's own content is in place is not, and nothing
   would grade it. FR-1.3-S3's "hold the attempt" is read as covering this.
4. **A cleared player loses their sub-block position** (D11). No scenario states
   where inside a cell they land, and cell centres are the only choice that
   keeps the box inside one cell column.
5. **`Session` is the only driver of `ContentReload`.** A second caller of
   `at_tick_boundary` would swap twice per tick. Nothing structural prevents it;
   this is held by there being one call site.
6. **The refusal deduplication compares rendered text**, following
   `App::report_remesh`. Two structurally different refusals rendering
   identically would be reported once — accepted, because the rendering is what
   a person reads.

---

## Risks

| Risk | What to verify, and when |
|---|---|
| **Two things a reload hands the device are gradeable only in halves.** If `App` never uploads the appended layer, the new block draws from stale texels; if `App` never assigns the new `HudLayout`, the widened crosshair never appears. Both live in the crate with no coverage that nothing runs. | **Phases 2 and 4.** For each, assert the *value* where it crosses the boundary (`testing.md` §2) — that the report hands over `TextureLayers` holding `base:amber` at layer 4, and a layout whose crosshair is `[21, 1]` — **and** drive that same value through a real device with no window: `crates/mc-render/tests/*_offscreen.rs` for the layer, `crates/mc-client/tests/support/hud_frames.rs` for the HUD. Neither half covers the other. The two assignments in `App` stay held by review, as its share of an edit already is. |
| **The settling window never reaches the debouncer.** `Duration::ZERO` leaves FR-9.1-S2 green, the coalescing test green, and the shipped client beginning one attempt per filesystem event. | **Phase 3**, and it is the second half of FR-1.1-S4 (D10). A test asserting the `Duration` handed to the builder. No filesystem, no timer. |
| **The whole-world solidity resolve hitches the tick.** 1 048 576 voxels on the tick thread at every accepted reload. | **Phase 1, measured not reasoned.** It is the same call `World::new` already makes at launch, so the figure exists — take it before deciding anything. If it is a material term in the one-second budget, D3's deferred section-skip is the named answer. Send the number to the FR-9 benchmark either way. |
| **The golden frames move.** `prepare_scene` is what every golden is shot through, and D9 changes how its layers are produced. | **Phase 2, and before the documentation work.** `launch_and_capture_agree.rs` and both golden suites are the instruments, and they must be run *after the assignment change alone* — running them once at the end of the phase cannot tell a wiring defect from an assignment defect. |
| **The adapter is the least coverable code in a covered crate.** `mc-world` is in the coverage denominator. | **Phase 3.** Keep the adapter to construction, drain and map. FR-1.1-S8 (an absent root) is deterministic and testable against a `tempfile` path that does not exist. A single "a real write is reported" integration test with a generous timeout is the whole of what touches a real filesystem; everything else goes through the double. If coverage dips, the answer is a thinner adapter, not an exclusion. |
| **`RemeshWork` grows from ~100 KB to ~1.3 MB.** D7 marks all 256 sections, and `take_remesh_work` clones each. | **Phase 4.** Confirm the copy against `remesh.rs`'s own stated bound and update that comment with the new derivation rather than leaving a false figure. The clone runs on the tick thread. |
| **The client stops reacting to a published reload and nothing reddens.** The exact shape of the two measured precedents in `testing.md` — a pure policy nobody calls. | Every FR-3, FR-4 and FR-5 test drives through `Session`, not through `Simulation::adopt` directly, so a `Session` that stopped calling `at_tick_boundary` reddens 26 scenarios. **Say this out loud in `tasks.md`**: a test that calls `adopt` itself is agreement between two callers of one function. |
| **Two gate-limit collisions are already within reach.** `session.rs` at 492/500; **after phase 1's split it is `session/mod.rs` at 496/500**. | **Phase 1 and phase 4.** D14 pre-commits the split. Whoever adds the drive checks the count before, not after — and puts it in `session/reload.rs`, because four lines is not room for it in `mod.rs` and shaving the header to fit is not the answer. |
| **`mc-world` gains a vendor that could leak.** `notify` types escaping `notify_watch.rs` would put a platform watcher in the loader's vocabulary. | Structural: `ContentWatch` and `ContentChanges` name no `notify` type. Re-read `crates/mc-world/tests/dependency_graph.rs` in phase 3 and state the litmus test in the engine-reader record. |
| **The `World` module header's structural claim is weakened in the wrong direction.** Making `adopt` `pub(crate)` instead of module-private would open a crate-wide write door and every test would stay green. | **Phase 1.** The instrument is the compiler: the admission and swap live in `world::reload`, a child module, and `Simulation::adopt` reaches the module rather than the item. A reviewer reads the header's rewrite. FR-4.1-S4 is the only behavioural instrument. |

---

## Phasing

The load-bearing constraint: no phase may open with its scenarios already green.
Two forces decide the shape here.

**Force one — the swap is the spine, and almost nothing is observable before it
exists.** Solidity, the mutation rules, the held block, the world's survival and
the save all become observable the moment a candidate can be applied at a tick
boundary, and none of them can be made red by a *later* phase — only by there
being no swap yet.

**Force two — the swap must be reachable without a watcher, or phase 1 cannot
exist.** `Simulation::adopt(candidate)` is that seam, and it is the same entry
point the worker's output feeds in phase 3. Phase 1's tests reach it through
`Session`, never directly (see Risks).

| Phase | Scenarios | What it delivers | Why it is red on arrival |
|---|---|---|---|
| **1 — A candidate is applied at a tick boundary, or nothing is** (26) | FR-1.3-S1 · FR-2.2-S1..S3 · FR-3.1-S1..S3 · FR-3.2-S1..S3 · FR-3.3-S1..S4 · FR-3.4-S1..S2 · FR-4.1-S1 · FR-4.1-S2 · FR-4.1-S4 · FR-4.1-S5 · FR-4.2-S1..S4 · FR-4.3-S1 · FR-4.4-S1 | `World::adopt` and the `world::reload` child module (D3); admission against the world (D13) and against the held-block policy; `Simulation::adopt`; the drive through `Session`, including writing `holding` back | nothing swaps at all |
| **2 — Layers are appended, never renumbered, and the content is published** (15) | FR-4.4-S2 · **FR-4.5-S1** · FR-5.1-S1..S5 · FR-5.2-S1..S3 · FR-8.1-S1..S5 | `LayerAssignment` and the budget constant (D5, D6); `load`'s new parameter (D9); `PublishedContent`, its HUD and `ContentSerial` (D12) | phase 1 leaves the layer assignment exactly as a launch produced it and publishes nothing, so a reload gives a new key no layer, moves no serial and carries no HUD |
| **3 — A saved edit becomes one attempt, built off the tick thread, and a refusal is stated once** (26) | FR-1.1-S1..**S9** · FR-1.2-S1..S5 · FR-1.3-S2..S3 · FR-2.1-S1..S8 · FR-2.3-S1..S2 | the `ContentWatch` port, the relevance rule and the `notify` adapter (D2); `ContentReload`'s coalescing and in-flight build (D10); `ReloadRefusal` and its deduplication (D4) | nothing watches, nothing builds off-thread, and nothing reports |
| **4 — What a reload re-meshes, and against which content** (12) | FR-4.1-S3 · FR-4.3-S2..S3 · FR-6.1-S1..S4 · FR-6.2-S1..S5 | the geometry-change predicate and whole-world marking (D7); `RemeshWork` carrying the registry, `Remesher`'s in-flight comparison, `retire` and `Superseded` (D8); `App`'s upload and D15 | phases 1–3 mark no section on a reload and the worker keeps the layers it was spawned with |
| **5 — A player inside a cell that became solid is moved clear** (7) | FR-7.1-S1..S7 | `cleared` (D11), wired into the accepted path and reported in the same phase | the swap from phase 1 moves nobody |
| **6 — The seam stays cut and the window is declared once** (5) | FR-8.2-S1..S3 · FR-9.1-S1..S2 | two needles and their fixture entries in the client-source scan; the capture-pipeline scan; the settling-window declaration scan with its enumerated verdict | the scan's needle set does not yet name the reload path or the watcher, no capture-pipeline scan exists, and no scan of the window exists |

**Phase totals: 26 + 15 + 26 + 12 + 7 + 5 = 91**, which is the whole spec.

**Where the two added scenarios land, and why neither is in phase 1.**
FR-4.5-S1 needs a published content set to carry a HUD, and phase 1 is barred
from publishing anything — so it goes to phase 2, where `PublishedContent` is
born, and it is what makes that type's `hud` field load-bearing on the day it is
written rather than a field a later reader deletes as unused. FR-1.1-S9 needs a
watcher and a relevance rule, which is phase 3 by construction.

**Why FR-1.3-S2 and S3 sit in phase 3 rather than beside FR-1.3-S1.** Both name
machinery phase 3 delivers — S2 is *"while a candidate is being built"* and there
is no build in phase 1; S3 is *"when a change is reported"* and there is no
change reporting until `pending` exists. Only S1 — a break on the earlier tick
succeeding and one on the later refusing — is the swap-at-a-boundary property
itself, and it is drivable in phase 1 through `Session`.

**Binding sequencing constraint on phase 1.** Phase 1 must **not** touch layers,
must **not** publish content, and must **not** mark any section dirty. Whoever
implements it will be tempted to do all three while the swap is open —
`resolved_from` is three lines away and `mark_dirty` is in the same file — and
taking any of them hands phase 2 or phase 4 scenarios that are green on arrival.
This is the same "implement deliberately less first" that SPEC-016's phase 1/2
split ran on, and the tasks breakdown must say it out loud.

**Scenarios green on arrival inside their own phase, named rather than hidden.**

> **Corrected at `/sdd-tasks`, 2026-08-17, and the correction is not cosmetic.**
> This list was wrong in two independent ways. It **headed a "green on arrival"
> list with a bullet whose own text says its scenarios are red on arrival** —
> phase 3's inherited five, which are correctly red and are moved to their own
> heading below. And its **membership did not match the tree**: it missed three
> scenarios that are green when their phase opens, one of them the emit-nothing
> case the spec's own scenario audit had already caught once (FR-6.1-S1). Both
> failures came from the same method — the list was assembled per *decision*, by
> asking which scenarios each phase's design leaves untouched, rather than per
> *phase*, by asking what the tree does when that phase opens. **Only the second
> question can be right**, because a scenario is green or red against a tree and
> not against a design. Read by its bullets' text the old list was nine; read by
> its heading it was fourteen; counted scenario by scenario against the tree it
> is **twelve**, below. `tasks.md` carries the same twelve with the mutation owed
> for each.

| Scenario | Phase | Green because |
|---|---|---|
| FR-3.1-S1 | 1 | a swap that does nothing leaves the broken and placed cells alone |
| FR-3.1-S2 | 1 | same, cell for cell |
| FR-3.2-S1 | 1 | a swap that does nothing leaves the player where the ticks put them |
| FR-3.2-S3 | 1 | same, for velocity |
| FR-3.3-S1 | 1 | the first solid block is still the first solid block |
| FR-3.4-S2 | 1 | it is the existing `Acceptance`/`RegistryVerdict` path and needs **no reload machinery at all** — green the moment the fixture exists |
| FR-4.1-S5 | 1 | a swap that does nothing leaves stone stopping the player |
| FR-4.4-S1 | 1 | nothing moves, so nothing the author did not edit moves |
| **FR-6.1-S1** | 4 | **phases 1–3 mark no section on a reload, so an implementation that never meshes satisfies "a candidate changing nothing geometric meshes no section"** — the emit-nothing case, and FR-6.1-S4 is its paired control |
| **FR-7.1-S3** | 5 | **phases 1–4 move nobody**, so "a cell the box does not overlap becomes solid → the player stays put" is satisfied by doing nothing |
| **FR-7.1-S6** | 5 | **same** — `cleared` is reached only from the accepted path, so a refused candidate moves nobody because the code never runs |
| FR-9.1-S1 | 6 | green the moment phase 4 lands, because the re-mesh transport already runs off the tick thread; it reddens if anything ever moves the re-mesh onto the tick |

Phase 1's eight are controls, not evidence — FR-3.1-S3 is the paired control the
spec's own scenario audit added for exactly this, and FR-4.1-S1, FR-4.2-S1..S4,
FR-3.3-S2/S4, FR-3.4-S1 and FR-2.2-S1..S3 are what actually redden. FR-3.4-S2 is
the control for FR-3.4-S1, which is the discriminating end-to-end oracle this
spec has.

**Four more of phase 6's scenarios are green as soon as their instrument is
written**, which is a different thing and is not merged into the twelve.
FR-8.2-S1, FR-8.2-S2, FR-8.2-S3 and FR-9.1-S2 are structural scans reporting on
properties phases 1–5 already established, so none can have a red step in the
ordinary sense — **their positive controls are the whole of their
falsifiability**, and `tasks.md` names one for each.

**Inherited behaviour that is red on arrival, and what it controls.** Not green,
and separated from the list above because merging the two ideas is what made the
earlier version wrong. FR-2.1-S5, S6 and S7 and FR-2.3-S1/S2 are refusals the
loader already produces at launch — it is already all-or-nothing, already refuses
blocks and HUD together, and already enforces the budget and the memory cap.
**They are red on arrival because no reload path exists**, and their value is as
controls: they redden again if the reload reaches content through anything other
than `mc_sim::content::load`.

**One scenario is strengthened rather than new.** FR-8.1-S3 (a reader honours a
deliberately non-lexicographic assignment) is nearly SPEC-016's own assertion
with a reload in front of it. Its test must go through an accepted reload, or it
is re-proving what another test already proves through the same code path —
`testing.md` §1's definition of a bogus test.

**One expiring test is inherited and must not be repaired.** SPEC-016's pin on
the name-for-texture substitution turns red when PRO-902 closes the gap, and
that red is its success signal. Nothing in this spec may green it early;
FR-4.3-S2 declares `texture` equal to `name` for exactly that reason.
