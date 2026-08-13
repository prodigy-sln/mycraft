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
- **No wall clock.** The replay advances one tick per rendered frame, once the world is ready —
  the spawn is derived from the world, so there is no simulation to tick while the preparation
  worker is still generating one, and the frames before it lands draw the clear colour and advance
  nothing. Nothing here reads a clock,
  and nothing may start to without a recorded decision — a wall clock is the easiest way to make a
  golden frame unreproducible.

## The scripted scene names blocks in Rust

`src/replay/world.rs` names `base:grass`, `base:dirt`, `base:stone`, `base:water` and `base:air`. It
is the **only** file listed in `EXEMPT_FILES` in `crates/mc-world/tests/no_hardcoded_block_names.rs`.

**This does not weaken invariant 1**, which forbids hardcoded block *definitions*. `world.rs` names
blocks to *place* them; texture and solidity still come only from `content/base/blocks/*.toml` via
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
