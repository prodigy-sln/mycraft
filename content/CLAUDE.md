# content/ — the game, as content

Everything here is content, not engine code: today that means data files read
through a well-defined loading contract, with no privileged engine access.
`content/base/` is the "vanilla" game and is a mod like any other (ADR-005).
**Luau arrives with MVP 2** and will progressively take over authoring — see
"MVP 1 today vs. MVP 2" below for exactly what that changes and what it does
not.

API reference: `docs/modding/` (block authoring: `docs/modding/blocks-items.md`).
Host-side rules, once the scripting host exists: `crates/mc-script/CLAUDE.md`.

## The rule that makes this work

**If the base game needs something the loading contract cannot express, fix
the contract — never special-case the base game in Rust.** That temptation
will come up repeatedly and giving in to it once is how modding APIs become
second-class. The base game's job is to prove the contract is complete.

Concretely: zero hardcoded block, item, recipe, NPC, biome, or quest
definitions in Rust. If you find yourself adding one, the change belongs in
the loader's public contract (`DefinitionSource` for blocks today;
`mc-script` bindings once Luau lands), never inline in engine code.

## MVP 1 today vs. MVP 2

**Today, block definitions are TOML data files**, one block per file, under
`content/<mod>/blocks/<block>.toml`. The full authoring contract — required
fields, the namespaced-id rule, all-or-nothing loading, how failures are
reported — is documented in `docs/modding/blocks-items.md`; it is not repeated
here. Items, recipes, NPCs, biomes, quests, and dialogue have no defined
format yet — MVP 1 ships blocks only.

Blocks are read through `DefinitionSource`, a Rust trait (`mc_core::block::source`)
that the engine consumes without ever learning whether a definition came from
a file, a script, or anything else. **MVP 2 replaces the TOML file reader with
a Luau-backed implementation of the same trait — it does not touch the
registry the definitions end up in.** A block author who learns the contract
in `docs/modding/blocks-items.md` today does not need to relearn it once Luau
arrives; what changes is only the file extension and the authoring language,
not what a block *is* or how it gets validated.

The guidance below — string IDs, declarative definitions, NPC styles, mod
`tests/`, the performance budget — describes **MVP 2's Luau-scripted world**.
None of it applies to MVP 1's TOML block files, which have no behaviour,
no load-time code, and no tests of their own beyond what the engine's loader
enforces. It is kept here, not deleted, because it is still the intended
shape of authoring once Luau lands and there is no better place to record it
in the meantime.

## Authoring rules (MVP 2, Luau)

- **String IDs, always namespaced**: `"base:stone"`, `"yourmod:thing"`. Never assume a numeric ID
  is stable across runs — it is assigned at registry build and remapped when the mod set changes.
  (This part is already true today: MVP 1's TOML `name` field follows the identical rule.)
- **Definitions are declarative; behaviour is functions inside them.** Do not perform side effects
  at load time. Load-time code runs in a scratch VM during every hot-reload candidate build, so
  side effects there run at unpredictable moments.
- **No state in Lua globals.** Runtime state belongs in the ECS via handles; mod-owned persistent
  state is declared through `mycraft.state(...)` so it survives reload (ADR-004). A global counter
  will silently reset and you will lose an afternoon to it.
- **Prefer declarative subscriptions to polling.** Register an event predicate and let Rust match
  it; do not scan the world every tick. This is the difference between a server that holds 32
  players and one that does not.
- **Use Luau types.** Gradual typing is why Luau was chosen over LuaJIT — annotate public functions
  and definition tables. Type errors caught at reload beat crashes at 2am.
- Files stay small and grouped by domain (`blocks/`, `items/`, `recipes/`, `npcs/`, `biomes/`,
  `quests/`, `dialogue/`). One concept per file. (MVP 1's `blocks/` directory already follows this;
  the other domain directories arrive with their respective definition kinds.)

## NPCs (MVP 2, Luau)

Two styles, both fully scripted — pick by scale:

- **Coroutine brains** for characters with intent. They yield across ticks while Rust performs the
  movement. Readable, and the right default.
- **Declarative behaviour trees** for crowds. Compiled to a Rust-evaluated tree at registry build,
  so common NPCs cost no Lua call per tick.

Respect the LOD contract: an NPC's brain may run at 20 Hz, at 2 Hz, or not at all depending on
player proximity. Never assume a fixed tick rate, and never accumulate time by counting brain
invocations.

## Testing (MVP 2, Luau)

Every mod carries a `tests/` directory. **These run on every hot-reload candidate before the swap**
— a mod whose tests fail never reaches the live world. That makes them a safety mechanism, not a
formality:

- Assert definitions exist with the expected properties after load
- Assert recipes resolve and produce what they claim
- Assert quest stage transitions fire on their trigger events
- Assert `on_reload` migrations preserve state across a version change

MVP 1's TOML block files have no `tests/` directory of their own — the
engine's content-root loader is what enforces their correctness (rejecting
missing fields, unknown fields, bad namespacing, and duplicate names; see
`docs/modding/blocks-items.md`), and it runs on every load, not on a reload
candidate specifically, because there is no reload yet to gate.

## Performance (MVP 2, Luau)

You are running inside a 50 ms tick budget shared with 31 other players and the rest of the engine.

- Every callback runs under an instruction budget. Exceeding it aborts the callback and marks the
  mod degraded — it does not slow the server down, it stops your mod working.
- Allocate sparingly in per-tick paths; table churn is the usual culprit.
- Per-mod CPU time is measured and visible. Your mod's cost is attributable.
