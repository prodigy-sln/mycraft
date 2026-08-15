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

1. **Every VM is sandboxed.** `Lua::sandbox(true)` before any user code is loaded. No `io`, no
   `os.execute`, no `package.loadlib`. `require` resolves only inside the mod's own directory —
   path traversal out of it is a Blocker.
2. **Every script entry point has an instruction budget.** Set via `Lua::set_interrupt`. A callback
   that exceeds it is aborted and its mod marked degraded. There is no unbudgeted path from engine
   into script.
3. **Every VM has a memory limit.** `Lua::set_memory_limit`. Allocation failure surfaces as a Lua
   error, never an abort.
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
- Every limit (instruction budget, memory cap, failure-disable threshold) has a test that actually
  trips it.
- Hot reload is tested for state preservation, not just for "the new code ran".
- Reload failure paths are tested: syntax error, failed validation, failing mod test, error thrown
  inside `on_reload`. Each must leave the previous registry serving.

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
2. Is it bounded — in allocation, in instruction count, in the radius or volume it can read or
   write? An unbounded binding is a Blocker however useful it is.
3. Is it named for the capability rather than the implementation?
4. Is it documented in `docs/modding/api-reference.md` in the same change?

**"The base game does not need it yet" is not a reason to refuse it** — see the surface policy
above. ADR-005's claim that the base game is the API's proof of completeness still holds and is not
this: it says the base game must *exercise* the API, never that the API may hold only what the base
game happens to use.
