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

## When adding a binding

Ask in order:
1. Could a hostile mod use this to hang the server, exhaust memory, escape the sandbox, or read
   another player's private state?
2. Does the base game need it? If not, why does it exist? (ADR-005: the base game is the API's
   proof of completeness.)
3. Is it named for the capability rather than the implementation?
4. Is it documented in `docs/modding/api-reference.md` in the same change?
