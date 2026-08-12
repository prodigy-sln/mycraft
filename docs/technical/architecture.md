# Architecture: Crate Boundaries and Dependency Direction

Crate topology as established by the block registry and chunk storage work.
This records only what that work actually built — it is not a survey of
all ten crates in the workspace (see `CLAUDE.md`'s crate map for the full
list and the general inward-dependency rule).

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

## Mechanically enforced invariants

Two facts about this design are asserted by tests that walk real
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
  and fails if any of the five shipped block names appears in its
  *production text* — the file minus every `#[cfg(test)]` item and minus
  every doc comment, since a unit test and a doc example are both test
  code that lives in a production file (`technical/testing.md`). This
  scan also carries a positive control: pointed at a fixture directory
  containing one of the five names, it must report a hit, or a broken
  path glob or matcher could pass by never actually looking at anything.
  It carries a second control for the skip itself — a name inside a test
  module is skipped while one after that module is still found — because
  a walk that lost a closing brace would swallow the rest of every file
  with the first control still green.

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
