# mc-sim — the simulation

Headless-capable simulation core. This crate **is** the server: it owns world state, the tick, and
everything authoritative. `mc-render` reads what it publishes and never the other way round —
`crates/mc-render/tests/dependency_graph.rs` asserts that neither crate resolves the other, with
`mc-client` as the positive control.

## Boundaries

- **The sim publishes world state; it never publishes pixels.** Quads are as far as it goes. The
  quad → vertex → packed-vertex conversion is `mc-render`'s pure layer, and the composition root
  calls it. That is not a workaround for the dependency rule: it is the shape MVP 3 needs, where
  chunk data arrives over the wire and the *client* meshes and packs it.
- **Publishing never waits on a reader.** Publication is a pointer swap (`ArcSwap`), and the
  snapshot type stays free of interior mutability so that "a later publish changed the snapshot I am
  holding" is not expressible. A field carrying a `Mutex` or an `AtomicU32` would reopen that hole
  silently, with every existing test still green.
- **No wall clock.** The replay advances one tick per rendered frame. Nothing here reads a clock,
  and nothing here may start to without a decision recorded first — a wall clock is the single
  easiest way to make a golden frame unreproducible.

## The scripted scene names blocks in Rust — where invariant 1's proxy over-approximates

`src/replay/world.rs` names `base:grass`, `base:dirt`, `base:stone`, `base:water` and `base:air`,
and it is the **only** file in the workspace listed in `EXEMPT_FILES` in
`crates/mc-world/tests/no_hardcoded_block_names.rs`.

**This is not a weakening of invariant 1.** The invariant forbids hardcoded block *definitions*.
`world.rs` names blocks in order to *place* them; texture and solidity still come only from
`content/base/blocks/*.toml`, through the registry the generator is handed. The scan forbids every
*mention*, which was a free over-approximation of the invariant right up until something
legitimately needed to reference a block — and that has now happened. What is recorded here is where
the proxy is stricter than the rule it enforces, not a relaxation of the rule.

Two mechanisms keep it from spreading. The entry is matched on the file's full trailing path, so it
cannot widen to `mc-sim`, to `replay/`, or to any other `world.rs`. And the exemption is pinned by
`the_exemption_skips_exactly_one_file_of_the_production_tree`, which walks the real tree and asserts
which files the filter actually skipped — so a second exemption fails the suite and has to be argued
for in a commit message rather than appear in a diff nobody reads.

That pin deliberately measures the scan's *behaviour* rather than the contents of `EXEMPT_FILES`. An
equality assertion on the constant catches only an exemption spelled as an entry in it; one spelled
as a second constant and a second clause in the filter leaves it untouched and green. That was
measured, not assumed: under a mutation exempting a second real file through the filter, the
constant-equality version passed while this one failed.

**Closing this deletes both the entry and its pinning test.** The missing hook is content-authored
worldgen — a scripted demo scene is exactly what should be Luau content, and invariant 1's own
remedy is to fix the missing hook in the API rather than to special-case it. That is MVP 2/3 work.
It was deferred for MVP 1 because the binding signature `ReplayWorld::generate(seed, &BlockRegistry)`
carries no content root the choice could be read out of, and changing it mid-spec would have moved a
signature five phases of tasks were broken down against. Until the hook exists, that one file is
held by review rather than by the scan.

## Determinism is the product here

The replay exists so that a golden frame means something, so every part of it is a pure function of
a seed or of a tick index:

- `pose` is a free function with no state to accumulate into, so "the pose at tick 60" is the same
  value whether it is asked for directly or reached by advancing sixty times. An accumulated path
  drifts, and does so only on the machine that ran it.
- The heightmap's spatial coherence is **construction, not luck**: value noise on a lattice period
  of 16 with smoothstep bounds the field's slope at 1.5 blocks per block, which is where the
  "adjacent columns differ by at most 2" bound comes from. Lowering the amplitude is safe;
  shortening the period is not, and invalidates that derivation.
- Meshing runs on rayon workers and may only ever use an **indexed** collect into a `Vec`.
  `for_each` into a shared sink, and collecting into a set or a map, are forbidden on that path —
  they let the worker count decide the order quads reach the packer in, and it breaks invisibly,
  because the machine the goldens were captured on never changed its worker count.
