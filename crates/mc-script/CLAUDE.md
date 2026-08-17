# mc-script — Luau scripting host

This crate is the product. "Fully scriptable" is decided here, and so is whether one bad mod can
take down a 32-player server.

Full rationale: `docs/technical/decisions.md` ADR-003 and ADR-004. API surface: `docs/modding/`.

## Threat model

**Mod code is untrusted.** Not "mostly trusted", not "trusted because the server owner installed
it" — untrusted. A server operator running 40 community mods cannot audit them, and mod scripts are
also delivered to *clients*. Every binding is written as if the author is hostile or, far more
commonly, careless.

## Non-negotiable invariants

1. **Every VM is sandboxed, and `sandbox(true)` is not what does it.** Measured: closing the
   sandbox removes five of the denied globals and leaves the rest standing — `os`, `require`,
   `loadstring`, `debug`, `getfenv`, `setfenv`, `collectgarbage`, `newproxy` and `gcinfo` all
   survive it. The host removes every denied name **itself, before** closing the sandbox, because
   afterwards a write to a global lands in a child table and removes nothing. Do not read this
   invariant as "the backend handles `require`/`os`/`debug`"; it does not.
   `require` is currently **absent rather than confined**. There is no mod-directory concept yet,
   so absence over-satisfies the rule; confinement arrives with multi-file mods, and path traversal
   out of a mod's own directory is a Blocker then.
2. **Every script entry point has a call-and-loop budget.** Set via `Lua::set_interrupt`. A callback
   that exceeds it is aborted and its mod marked degraded. There is no unbudgeted path from engine
   into script — **including chunk evaluation**, which is such a path.
   **It does not count instructions**, and the name matters: the interrupt fires at seven opcodes,
   so a loop body of any size is free, a thousand straight-line statements cost one, a call within
   script costs two and a call into the host costs one. Size it against how many calls a workload
   makes, never against how much code it is.
3. **Every VM has a memory limit, and the allocator's own limit is not enough on its own.**
   `Lua::set_memory_limit` is the absolute backstop; what enforces the per-invocation cap is the
   interrupt, reading usage each tick against the baseline the invocation started from.
   Allocation failure does surface to script as an ordinary Lua error — which is exactly the
   problem. **Measured: a `pcall`-wrapped allocation bomb caught it, dropped the table, let the
   collector reclaim it, and looped ten times before returning normally.** A limit script can catch
   bounds nothing, so the abort **latches**: once tripped, no further script frame runs, including
   the handler that would have caught it. Reclaiming after an allocation fault takes **two**
   explicit collections, not one — the first ends the cycle in progress and the second sweeps.
4. **No script error propagates as a panic.** Every callback is invoked through the catch-all
   wrapper. Errors are logged with mod attribution; after N consecutive failures the callback is
   disabled and the server keeps running. `unwrap`/`expect`/`panic!` are lint-denied here for this
   reason — do not `#[allow]` them.
5. **Scripts never hold live Rust references.** Bindings pass handles (ids), not borrowed state.
   This is what makes discarding a VM on reload safe. See ADR-004.
6. **The registry is immutable once built.** Mutation happens by building a replacement and
   swapping via `ArcSwap` at a tick boundary — never by mutating a live registry.
7. **A failed reload changes nothing.** Candidate registries are built in a scratch VM off the tick
   thread and validated before the swap. If validation or the mod's own tests fail, the previous
   registry keeps serving and the failure is reported. Partial application is a Blocker.
8. **Numeric IDs are never persisted directly.** Persist the string↔numeric mapping alongside the
   world so IDs can be reassigned when the mod set changes. Unknown blocks round-trip losslessly.

## Freezing a table does not freeze what its metatable points at

A standing constraint on any change to how a chunk's environment is built, recorded because it
was got wrong once and because the scenario nearest to it stayed green throughout.

A chunk is evaluated against a fresh table of its own, frozen, whose metatable reads through to
the sandboxed globals. **Three tables have to be frozen and it is easy to see only two.** Freezing
the environment stops a chunk writing its own globals. Freezing the metatable stops it repointing
what the environment reads through. Neither stops it doing this:

```lua
local above = getmetatable(_G).__index
rawset(above, 'smuggled', 1)          -- and plain assignment works too
```

`above` is the table **every chunk on the server reads through**. Measured, by both routes: the
write succeeds and every later chunk reads the planted name. That is one mod adding to the global
environment exactly as if nothing were frozen.

The scenario that looks like it covers this writes `rawset(_G, 'smuggled', 1)` — same verb, same
name, one table apart — which lands on the frozen environment and is correctly refused. **It stays
green against a host with the hole.** The test that does catch it plants one level up and asserts a
*later* chunk cannot see the name.

Whoever refactors the environment construction re-introduces this by freezing the obvious two.

## Threading

The Lua VM is pinned to the tick thread. `mlua` is `!Send` by default and we keep it that way —
it is both simpler and better for determinism. Do not enable mlua's `send` feature to "make it
easier"; if you need work off-thread, move the *data* off-thread, not the VM.

## Performance rules

- **Never call into Lua per-event in a hot loop.** Subscriptions carry declarative predicates
  matched in Rust; script runs only on a match. Calling a script for every block break by 32 players
  does not survive contact with reality.
- Per-mod CPU time is accounted and exposed. "Which mod is eating the tick" must always have an
  answer.
- Binding-call overhead matters more than script-body speed. Prefer one call passing a batch over
  N calls passing one item.

## Testing

- Sandbox escapes get an explicit test each. A test that asserts `io` is absent is worth more than
  ten tests of happy-path binding behaviour.
- Every limit (call-and-loop budget, memory cap, failure-disable threshold) has a test that
  actually trips it. **Watch for one limit masking another**: filling a megabyte costs far more
  interrupt ticks than a test-sized budget allows, so a memory test under a small budget dies of
  ticks and reports the wrong limit while passing.
- Hot reload is tested for state preservation, not just for "the new code ran".
  **Realised for declarations** (`docs/modding/hot-reload.md`): the world, the
  player's position and velocity, the block in their hand and every edit they made
  all survive a swap, and each has a scenario of its own.
- Reload failure paths are tested: syntax error, failed validation, failing mod test, error thrown
  inside `on_reload`. Each must leave the previous registry serving. **Realised for
  the paths that exist today** — a chunk that will not compile, a misspelled field,
  a candidate that stops declaring a block the world holds, one that registers no
  solid block, and one needing more array-texture layers than the session has left.
  There is no mod `tests/` directory and no `on_reload`; what gates a candidate is
  the loader's own all-or-nothing validation, the same gate that runs at launch.

## Composition is the extension model

**Content extends the game by attaching components, never by subclassing.** This matches the
runtime: the engine is already an ECS, so the content model and the execution model are one idea
rather than two.

**Blocks are flyweight; entities are not.** A block cannot have an entity — a section is 16³ and a
definition is shared by millions of placements. So components attach to the **definition** for
blocks and to the **instance** for entities. Same model, different granularity. **State this
wherever it could be misread**, because purifying it into an entity-per-voxel kills the mesher.

One question — *which components does this definition have?* — replaces four mechanisms: **tags**
are marker components with no data, **capabilities** are components carrying a contract,
**behaviour** is components carrying callbacks, and inheritance is not needed at all.

**Why not inheritance**, since it is the obvious alternative: it is single-parent and exclusive.
One mod wanting "furnace, but faster" and another wanting "furnace, but bigger" both subclass, and
a player installing both gets two incompatible furnaces. Composition lets both attach. It also
keeps the property inheritance structurally cannot have — **a third party can attach to a block
whose author never heard of them**, which is what interop actually needs, since the integrating mod
controls neither party.

**Ordering, when two components carry the same callback.** Derived from the fault split above
rather than invented:

- **Queries** (may a block be placed here) — **conjunction.** Every attached component must agree;
  any refusal refuses. Order-independent, so the question does not arise.
- **Notifications** — **all run, none cancels**, ordered by *declared constraints*
  (`after = "base:furnace"`), never by a priority number. A dependency graph is checkable; a
  priority integer is a race to the top that every modding ecosystem has lost.

**Budgets, quarantine and per-cell state are per component, not per block.** A broken attachment
from one mod stops acting while another mod's behaviour on the same block keeps working — strictly
finer isolation than disabling the block, and it follows from invariant 4 rather than replacing it.
Namespace per-component state by the component's id, or two mods declaring a `fuel` field collide.

**Attach yes, remove no.** Removing another mod's component silently breaks its assumptions;
removal needs the owner's permission or does not exist.

## API surface policy — breadth of capability, narrowness of commitment

**A published modding API is exempt from "no abstraction before three concrete uses."** Waiting for
three uses means the modder who needed a hook already worked around its absence, and the workaround
becomes the thing that cannot be broken. Provide capability for uses you cannot enumerate.

**But the cost model is not wasted effort, it is permanent commitment.** A speculative method that
turns out wrong is not work you can delete — it is a wart you cannot withdraw, and mods will build
on it *because* it exists. Four disciplines replace the rule that no longer applies:

1. **Ship the capability, version the surface.** An API version in the mod manifest is what makes
   deprecation possible. Without one, v1 is forever.
2. **Mark provisional things mechanically.** An `unstable` namespace content must opt into, so "this
   may change" is enforced by the loader rather than written in a document nobody read.
3. **Expose intent, not mechanism.** The exposure that hurts is a signature encoding how the mesher
   happens to work today.
4. **Invariant 3 above becomes the filter** — not *"do we need this yet?"* but *"can this be abused,
   and is it bounded?"* A method nobody uses costs nothing; one that lets a mod allocate without
   limit costs the server.

**This exemption is scoped to the published scripting surface and nowhere else.** Inside the engine
— ports, internal abstractions, generic wrappers over our own code — three-uses still binds. Cite
this section to justify a speculative *internal* trait and you have misread it.

## When adding a binding

Ask in order:
1. Could a hostile mod use this to hang the server, exhaust memory, escape the sandbox, or read
   another player's private state? **This is the gate**, and a hook with no consumer yet passes it
   as easily as one with three.
2. Is it bounded — in allocation, in calls made, in the radius or volume it can read or write?
   An unbounded binding is a Blocker however useful it is.
3. Is it named for the capability rather than the implementation?
4. Is it documented in `docs/modding/api-reference.md` in the same change?

**"The base game does not need it yet" is not a reason to refuse it** — see the surface policy
above. ADR-005's claim that the base game is the API's proof of completeness still holds and is not
this: it says the base game must *exercise* the API, never that the API may hold only what the base
game happens to use.
