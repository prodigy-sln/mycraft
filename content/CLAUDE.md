# content/ — the game, as content

Everything here is content, not engine code, read through a well-defined loading
contract with no privileged engine access. `content/base/` is the "vanilla" game
and is a mod like any other (ADR-005).

**Blocks are declared in Luau; the HUD and the tooling formats are data files.**
A block declaration is a chunk that returns a table, evaluated inside the
sandboxed host under its call-and-loop budget and memory cap — so a declaration
may compute what it declares. What is *not* authored in Luau is **behaviour**:
there is no `mycraft.*` binding of any kind, so a declaration returns a value and
calls nothing the engine provides.

Authoring starts at `docs/modding/README.md`, which routes to the contract for each
kind of file; block authoring itself is `docs/modding/blocks-items.md`. The scripting
host exists now — its author-facing side is the four `docs/modding/script-*.md` pages,
and its host-side rules are `crates/mc-script/CLAUDE.md`.

## The rule that makes this work

**If the base game needs something the loading contract cannot express, fix
the contract — never special-case the base game in Rust.** That temptation
will come up repeatedly and giving in to it once is how modding APIs become
second-class. The base game's job is to prove the contract is complete.

Concretely: zero hardcoded block, item, recipe, NPC, biome, or quest
definitions in Rust. If you find yourself adding one, the change belongs in
the loader's public contract (`DefinitionSource` for blocks; `mc-script`
bindings once behaviour lands), never inline in engine code.

## What is declared here today

**Block declarations are Luau**, one block per file, under
`content/<mod>/blocks/<block>.luau`. The full contract — the six fields, the
namespaced-id rule, all-or-nothing loading, how a refusal reads, and the four
bounds a content root carries — is `docs/modding/blocks-items.md` and is not
repeated here. HUD elements remain TOML under `content/<mod>/hud/`; voxel models
and materials are read by `voxforge` alone, but they **do** reach a player now —
`voxforge build` bakes a model's faces into the images a texture key draws from,
and the base game's grass, dirt and stone are drawn from that bake
(`docs/modding/voxel-models.md`). Items, recipes, NPCs, biomes, quests and
dialogue have no format at all yet.

Blocks are read through `DefinitionSource`, a Rust trait (`mc_core::block::source`)
that the engine consumes without ever learning whether a definition came from a
file, a script, or anything else. **The swap from a TOML reader to a Luau one
went through that trait and did not touch the registry the definitions end up
in** — which is why the contract a block author learned did not change with the
language.

The guidance below — declarative definitions, NPC styles, mod `tests/`, the
performance budget — describes the **behaviour** layer, which does not exist
yet. It is kept here because it is the intended shape of authoring once
components land, and there is no better place to record it meanwhile. What it
says about **string IDs is already true** of every declaration in this tree.

## Authoring rules (behaviour, not yet reachable)

- **String IDs, always namespaced**: `"base:stone"`, `"yourmod:thing"`. Never assume a numeric ID
  is stable across runs — it is assigned at registry build and remapped when the mod set changes.
  (Already true: a block declaration's `name` field follows the identical rule.)
- **Definitions are declarative; behaviour is functions inside them.** Do not perform side effects
  at load time. Load-time code runs in a scratch VM during every hot-reload candidate build, so
  side effects there run at unpredictable moments.
- **No state in Lua globals.** Runtime state belongs in the ECS via handles; mod-owned persistent
  state is intended to be declared through `mycraft.state(...)` so it survives reload (ADR-004).
  A global counter will silently reset and you will lose an afternoon to it.
  **`mycraft.state(...)` does not exist yet — nor does any other `mycraft.*` binding.** A block
  declaration returns a table and calls nothing the engine provides. The rule above is the shape
  to design toward, not an API you can call.
- **Prefer declarative subscriptions to polling.** Register an event predicate and let Rust match
  it; do not scan the world every tick. This is the difference between a server that holds 32
  players and one that does not.
- **Use Luau types.** Gradual typing is why Luau was chosen over LuaJIT — annotate public functions
  and definition tables. Type errors caught at reload beat crashes at 2am.
- Files stay small and grouped by domain (`blocks/`, `items/`, `recipes/`, `npcs/`, `biomes/`,
  `quests/`, `dialogue/`). One concept per file. (`blocks/` already follows this — one declaration
  per `.luau` file; the other domain directories arrive with their respective definition kinds.)

## NPCs (behaviour, not yet reachable)

Two styles, both fully scripted — pick by scale:

- **Coroutine brains** for characters with intent. They yield across ticks while Rust performs the
  movement. Readable, and the right default.
- **Declarative behaviour trees** for crowds. Compiled to a Rust-evaluated tree at registry build,
  so common NPCs cost no Lua call per tick.

Respect the LOD contract: an NPC's brain may run at 20 Hz, at 2 Hz, or not at all depending on
player proximity. Never assume a fixed tick rate, and never accumulate time by counting brain
invocations.

## Testing (behaviour, not yet reachable)

Every mod is intended to carry a `tests/` directory, running on every hot-reload
candidate before the swap so that a mod whose tests fail never reaches the live
world. **That does not exist. What gates a reload candidate today is the loader's
own all-or-nothing validation — the same gate that runs at launch** — plus the
reload's own two checks: that every block the running world holds is still
declared, and that the content still registers a solid block. A candidate failing
any of them is refused whole and the running content goes on serving. See
`docs/modding/hot-reload.md`.

The intended shape, once there is a `mycraft.*` binding for a test to assert
against:

- Assert definitions exist with the expected properties after load
- Assert recipes resolve and produce what they claim
- Assert quest stage transitions fire on their trigger events
- Assert `on_reload` migrations preserve state across a version change

Block declarations have no `tests/` directory of their own — the engine's
content-root loader is what enforces their correctness (refusing missing fields,
unrecognised fields, bad namespacing, duplicate names, and every content-supplied
quantity past its bound; see `docs/modding/blocks-items.md`), and **that same
loader is now what gates a reload candidate** — a declaration saved while the game
is running goes through it before anything swaps.

A declaration is Luau and so it *could* carry a test of its own one day. It does
not yet, and the reason is the same one the rest of this section is about: there
is no `mycraft.*` binding for a declaration to call, so there is nothing for a
mod-authored test to assert against beyond what the loader already refuses.

## Performance (behaviour, not yet reachable)

You are running inside a 50 ms tick budget shared with 31 other players and the rest of the engine.

- Every callback runs under a **call-and-loop budget** — **not** an instruction budget, and the
  difference decides how you make something fit. It charges calls and loop edges, so a loop body of
  any size is free, a thousand straight-line statements cost one, a call within script costs two and
  a call into the host costs one. **Cost comes down by batching calls, never by shortening code.**
  Exceeding it aborts the callback and the abort latches, so a protected call cannot swallow it — it
  does not slow the server down, it stops your mod working. Measured on a 16³ pass — 4,096 cells —
  4,369 bare, 8,465 with one host call per cell, 12,561 with one call within script per cell: so
  roughly one, two and three ticks a cell, and the code is the same length in all three. **The
  shipped budget is 1,000,000**, which all three of those fit inside comfortably; what it is sized
  against is the largest chunk evaluation that cannot be sliced, since a callback over budget has
  the queue to go to and a chunk has nowhere. See `docs/modding/script-limits.md` for the
  arithmetic and for how to size a slice against it.
- Allocate sparingly in per-tick paths; table churn is the usual culprit. Memory is capped per
  invocation as a delta above what the state already held, and exceeding it aborts the same way.
- **Not built yet:** per-mod CPU accounting. Cost is not attributable to a mod today — a fault
  raised while the host itself is short of memory names nobody, deliberately, rather than blaming
  whoever happened to be running.
