# content/ — the game, in Luau

Everything here is written against the public scripting API with no privileged engine access.
`content/base/` is the "vanilla" game and is a mod like any other (ADR-005).

API reference: `docs/modding/`. Host-side rules: `crates/mc-script/CLAUDE.md`.

## The rule that makes this work

**If the base game needs something the API cannot express, fix the API — never special-case the
base game in Rust.** That temptation will come up repeatedly and giving in to it once is how
modding APIs become second-class. The base game's job is to prove the API is complete.

Concretely: zero hardcoded block, item, recipe, NPC, biome, or quest definitions in Rust. If you
find yourself adding one, the change belongs in `mc-script` as a new binding.

## Authoring rules

- **String IDs, always namespaced**: `"base:stone"`, `"yourmod:thing"`. Never assume a numeric ID
  is stable across runs — it is assigned at registry build and remapped when the mod set changes.
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
  `quests/`, `dialogue/`). One concept per file.

## NPCs

Two styles, both fully scripted — pick by scale:

- **Coroutine brains** for characters with intent. They yield across ticks while Rust performs the
  movement. Readable, and the right default.
- **Declarative behaviour trees** for crowds. Compiled to a Rust-evaluated tree at registry build,
  so common NPCs cost no Lua call per tick.

Respect the LOD contract: an NPC's brain may run at 20 Hz, at 2 Hz, or not at all depending on
player proximity. Never assume a fixed tick rate, and never accumulate time by counting brain
invocations.

## Testing

Every mod carries a `tests/` directory. **These run on every hot-reload candidate before the swap**
— a mod whose tests fail never reaches the live world. That makes them a safety mechanism, not a
formality:

- Assert definitions exist with the expected properties after load
- Assert recipes resolve and produce what they claim
- Assert quest stage transitions fire on their trigger events
- Assert `on_reload` migrations preserve state across a version change

## Performance

You are running inside a 50 ms tick budget shared with 31 other players and the rest of the engine.

- Every callback runs under an instruction budget. Exceeding it aborts the callback and marks the
  mod degraded — it does not slow the server down, it stops your mod working.
- Allocate sparingly in per-tick paths; table churn is the usual culprit.
- Per-mod CPU time is measured and visible. Your mod's cost is attributable.
