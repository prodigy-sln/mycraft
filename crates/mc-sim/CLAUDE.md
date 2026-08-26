# mc-sim — the simulation

Headless-capable simulation core. This crate **is** the server: it owns world state, the tick, and
everything authoritative. `mc-render` reads what it publishes and never the other way round —
`crates/mc-render/tests/dependency_graph.rs` asserts neither crate resolves the other, with
`mc-client` as the positive control.

## Boundaries

- **The sim publishes world state; it never publishes pixels.** Quads are as far as it goes; the
  quad → vertex → packed-vertex conversion is `mc-render`'s pure layer, called by the composition
  root. Not a workaround for the dependency rule — it is the shape MVP 3 needs, where chunk data
  arrives over the wire and the *client* meshes it.
- **Publishing never waits on a reader.** Publication is a pointer swap (`ArcSwap`); `latest`
  returns an owned `Arc`, so a reader holds nothing a publisher could wait on. Keep the snapshot
  type free of interior mutability: a `Mutex` or `AtomicU32` field reopens that hole silently, with
  every existing test green. Why the guard lives in the *shape* of these types, and which test
  pins it by compiling, is documented at that test in `tests/publication.rs`.
- **No wall clock.** A tick is a declared quantum of simulated time and nothing here reads a clock;
  nothing may start to without a recorded decision, because a wall clock is the easiest way to make a
  golden frame unreproducible.

  **What drives the ticks is the driver's business, and it is no longer one per frame.** The client
  reads elapsed time once a frame and spends whole `TICK_QUANTUM`s out of it, bounded at fifteen
  (`docs/technical/architecture.md` §"Pacing the frame"); the capture harness advances by hand.
  Both feed this same fixed step, which is exactly what `physics.rs` anticipated when it recorded
  the ratio as a stated cost. The world is still only advanced once it is ready — the spawn is
  derived from the world, so there is no simulation to tick while the preparation worker is still
  generating one, and the frames before it lands draw the clear colour and change nothing.

  **Hot reload needed one and the rule held. This is that recorded decision.** A
  filesystem watcher has to wait out a settling window — an editor writes a file in
  several syscalls, and reading it mid-write refuses a candidate the author never
  saved. That clock is the debouncer's, it lives behind the `ContentWatch` port in
  `mc-world`, and **`mc-sim` still reads none**: this crate is handed *changes*, and
  a tick boundary asks the watcher whether any arrived rather than asking how long
  ago. The window is declared in exactly one place, `mc-world`'s
  `content/watch/mod.rs`, and a scan holds it there.

- **The world has a second write door, and it is a child module for a reason.**
  `World::write` is one edit; `World::adopt` is the whole registry replaced by
  content read while the game was running. Both settle **every** view's answer
  before writing any of them, both carry no `pub` at all, and the reload admission
  that reaches `adopt` is a *child* of `world` for exactly that reason — a sibling
  would have forced `pub(crate)`, which is a much weaker claim. The dirty set is not
  one of the views that claim protects.

- **There are three resolved views, not two, and they answer different questions.**
  `ResolvedVoxels` carries a bitset for what stops the player, a second for what a
  ray may stop at, and a packed index saying what medium each voxel's volume is,
  behind three narrow traits — `Solidity`, `Targetable` and `Medium`. Content
  declares them independently, so an engine that derived any from another would be
  writing a game rule content could not override. Collision reads `Solidity` at nine
  sites and means collision by it; the walk a swing travels reads `Targetable` and
  nothing else does. **Keep those two traits two**: one trait with both methods
  gives every collision site access to a question it must never ask, and a collision
  test could then exercise aiming by accident.

  **`Medium` is one trait returning one value** — `VoxelMedium { swimmable,
  resistance }` — because one site reads both of its properties, one line apart,
  from one fold over one box. Splitting it would separate nothing and would let a
  fixture state one property and inherit the other, which is the live hazard here
  rather than the one above. The composite `Traversal: Solidity + Medium` names what
  **one tick of motion** may ask: `Targetable` is deliberately not among it, and
  `advance_player` is its only production door. Coercion only ever narrows, so the
  eight other `&dyn Solidity` doors cannot reach a medium question.

  **A fixture that exists to state solidity implements `Medium` as
  `VoxelMedium::NOTHING` unconditionally, both halves, and never as a function of
  its own solidity.** Those fixtures compute solidity from a geometric rule whose
  negation *is* the air, so "the air is the medium" is a one-line change that reads
  as insight and would put a resistance under dozens of collision assertions whose
  whole content is where a box stops.

  **Which half the suite actually protects was measured, and it is the opposite of
  what it looks like.** Make the air resistant and **46** pre-existing tests redden
  across twelve files — every walk, every fall, every jump arc, the replay poses and
  a golden frame. Make the air buoyant and **one** reddens, and
  `player_collision.rs` does not move at all: nearly every jump in the suite is
  asked *from the ground*, where `on_ground || buoyant` is already true, so
  buoyancy has almost no reach into collision. Counting how often a file mentions
  `jump` predicts the wrong answer here; only running it gives the right one.
  **So the rule is unconditional over both halves precisely because the buoyant
  half is the thin one** — a single `player_ground` test is the whole reporter, and
  it is what a later refactor must not delete.

- **Content is published through a second `ArcSwap` beside the snapshot**, carrying
  a serial that increments on every accepted candidate. A re-mesh batch carries its
  own serial and its own registry, so a batch cannot be meshed against a registry
  other than the one its world was resolved against — that is unspellable rather
  than checked — and whether a finished batch is stale is decided by the client at
  collect time, where "serving now" is known.

## The scripted scene names blocks in Rust

`src/replay/world.rs` names `base:grass`, `base:dirt`, `base:stone` and `base:water` — every block
the scene places, and no fifth one for the space above them, because it places nothing there. It is
the **only** file listed in `EXEMPT_FILES` in `crates/mc-world/tests/no_hardcoded_block_names.rs`.

**This does not weaken invariant 1**, which forbids hardcoded block *definitions*. `world.rs` names
blocks to *place* them; texture and solidity still come only from `content/base/blocks/*.luau` via
the registry the generator is handed. The scan forbids every *mention* — a free over-approximation
until something legitimately needed to reference a block. This records where the proxy is stricter
than the rule, not a relaxation of the rule.

It cannot spread: the entry matches the full trailing path (so it cannot widen to `mc-sim`, to
`replay/`, or to another `world.rs`), and `the_exemption_skips_exactly_one_file_of_the_production_tree`
walks the real tree asserting which files the filter *actually* skipped, so a second exemption fails
the suite. That pin measures behaviour rather than the contents of `EXEMPT_FILES`, because an
exemption spelled as a second constant plus a second clause leaves the constant untouched.

**Closing this deletes both the entry and its pin.** The missing hook is content-authored worldgen —
a scripted demo scene is exactly what should be Luau content — which is MVP 2/3 work. Deferred for
MVP 1 because `ReplayWorld::generate(seed, &BlockRegistry)` carries no content root to read the
choice from. Until the hook exists, that one file is held by review rather than by the scan.

## Determinism is the product here

The replay exists so that a golden frame means something, so its world is a pure function of a seed
and its inputs a pure function of a tick index:

- **The camera is not among them, and asking for it to be would be the wrong fix.** There was a
  `pose` here — an orbit, a free function of the tick index, so "the pose at tick 60" was the same
  value whether asked for directly or reached by advancing sixty times. The camera a frame is now
  shot through is the one `Simulation` publishes, which is an *integrated* player: it can only be
  reached by advancing `scripted_intent` from the spawn, and tick 59 cannot be asked for out of
  order. What replaces the orbit's property is that the world and the script are both declared, so
  the same advance produces the same camera — within a run, which is what a golden depends on.
  Reproducibility across libm versions is not claimed and never was; `cos`/`sin` were already in
  the orbit's path.
- The heightmap's spatial coherence is **construction, not luck**: value noise on a lattice period
  of 16 with smoothstep bounds the slope at 1.5 blocks per block, which is where "adjacent columns
  differ by at most 2" comes from. Lowering the amplitude is safe; shortening the period is not, and
  invalidates that derivation.
- Meshing runs on rayon workers and may only ever use an **indexed** collect into a `Vec`.
  `for_each` into a shared sink, and collecting into a set or a map, are forbidden there — they let
  the worker count decide the order quads reach the packer in, and it breaks invisibly, because the
  machine the goldens were captured on never changed its worker count.
