# Architecture: Luau host, sandbox and the hostile-mod harness

Spec: `spec.md` (SPEC-014, PRO-916, rigor `high`, 56 scenarios).
Decisions D1–D8 in `requirements.md` are inputs; this document settles D6, replaces
D7's mechanism, and adds the decisions the spec deliberately left to design.

**Everything asserted below about `mlua`/Luau behaviour was measured**, against
`mlua 0.12.0` + `luau0-src 0.20.7+luau728` on this toolchain (`rustc 1.97.1`), in a
throwaway crate outside the workspace. Where a measurement contradicts what the spec or
the crate's own `CLAUDE.md` assumed, the measurement is recorded as the reason and the
assumption is named. No claim here is reasoned from first principles about how Luau
"should" behave. **What is measured is behaviour; the inferences drawn from it are
not measured** and are marked where they carry weight.

Reviewed once against the spec before it was circulated. Three blockers (A9's unbounded
pending queue, unbudgeted chunk evaluation, and a callback type that could not be
expressed) are folded in below, as are corrections to A9's derivation and to R1's
grounds. Nothing was overridden. A later round of challenge produced the findings recorded at the
end of this document; the sections it changed are marked *amended per discussion*.

---

## Drivers

| Driver | Why it matters here | Evidence |
|---|---|---|
| **Containment (invariant 3)** is the feature, not a quality of it | The spec exists to convert "a bad mod never takes down the server" from a claim into a measurement. Every other attribute loses to it. | `spec.md` Goal; `crates/mc-script/CLAUDE.md` invariants 1–4 |
| **Falsifiability of containment** | MVP 2's exit criterion is that each hostile case is "proven by a mutation that reddens a test". A containment mechanism that cannot be shown failing is not evidence. | `spec.md` Verification obligation; `standards/global/testing.md` §2 |
| **Attribution** | A fault must name `(subject, component)` or an operator cannot act on it, and quarantine has nothing to act on. | FR-2.1-S1, FR-3.1-S1, FR-4.1-S1, FR-4.2-S1, user story 2 |
| **Evolvability of the backend** | ADR-003 keeps WASM/`wasmtime` as a *deferred, not rejected* second backend. `mlua` is pre-1.0 (0.12) with routine breaking releases. | ADR-003; `PLAN.md` §4.6 |
| **Test-time operability** | Every scenario must run inside the integration-test budget (<1 s). A production-sized memory cap cannot be tripped there. | D5; `standards/global/testing.md` §3 |
| **Determinism / single-threading** | The VM is pinned to the tick thread; `mlua`'s `send` feature stays off. Constrains every type here to `!Send`. | ADR-003; `crates/mc-script/CLAUDE.md` Threading |
| **Coverage from the first line** | `mc-script` has no gate exemption, so library code counts against the 80 % line threshold immediately. Pushes logic into testable Rust and keeps `src/` files under the 500-line ceiling via sibling `_test.rs`. | `scripts/sdd-gate.ps1` `$LineThreshold = 80`; `docs/technical/testing.md` |

**Constraints.** No regulatory or data-sensitivity exposure: no personal data crosses
this boundary and nothing here is networked. Solo, agent-driven project — the gate
carries the weight review carries elsewhere. Nothing in `crates/` may depend on
`mc-script` when this spec completes.

**Volatile / expensive to reverse.** The public surface of `mc-script` (PRO-917/918/919
build on it) and the fault vocabulary (invented here, permanent once content reads it).
The *internals* of budget and quarantine enforcement are cheap to change and are not
treated as one-way doors.

---

## Boundaries

| External dependency | Volatility (V/R/S/Sub) | Port | Adapter location | Direct-use justification |
|---|---|---|---|---|
| `mlua` 0.12 (Luau, vendored) | V: low-mid (MIT, widely used, active) · R: none · **S: high — pre-1.0, breaking minors are routine** · **Sub: real — ADR-003 keeps `wasmtime` deferred-not-rejected** | `mc-script`'s own public surface: `ScriptHost`, `ScriptValue`, `ScriptTable`, `ScriptFunction`, `ScriptFault`, `DispatchReport` | `crates/mc-script/src/luau/` | — |

**No `mlua` type appears in `mc-script`'s public API.** Not isolation theatre and not
YAGNI-able: API stability and substitutability both score high on their own terms
(architecture-principles §2), and ADR-003 already names the second backend. The port is
shaped around what the host needs — *invoke this attachment under a budget; tell me how
it ended* — not around `Lua`'s surface, which is why `Lua`, `Function`, `Table`, `Value`
and `Error` stay behind it. It costs approximately nothing: it is the crate's public API
declining to leak `mlua`, not an added layer.

**The litmus is mechanised, not asserted.** A text guard walks `crates/mc-script/src`
**and** `crates/mc-script/tests` — the harness is the code most likely to reach for
`mlua` directly — and reports an enumerated verdict
(`EveryMluaReferenceIsUnderLuauDir` / `LeakedOutside(paths)` / `ScanFoundNoFiles`)
rather than an absence. It carries a positive control over a fixture that *does* leak,
per `standards/global/testing.md` §2, and a per-root non-zero file count in the shape
`no_hardcoded_block_names.rs` and `license_declaration_consumers.rs` already use. An
unenforced litmus is a claim, and R4's entire mitigation rests on this one.

The design touches no network, no clock, no RNG and no filesystem. Luau itself is
deterministic and `os`/`io` are removed before content runs (A1), so the host introduces
no nondeterminism requiring a further port.

---

## Decisions

### A1 — Denied globals are removed, and the host `print` installed, *before* `sandbox(true)` — BINDING *(amended per discussion — DF10, DF11)*

**Measured, and it inverts the obvious order.** `Lua::sandbox(true)` calls `luaL_sandbox`
+ `luaL_sandboxthread`, which makes the base global table readonly and gives the running
thread a *child* environment that reads through `__index`. After that point, assigning
`nil` to a global **cannot remove it**: the write lands in the child table and the read
falls straight through `__index` to the still-populated parent.

Measured after `sandbox(true)`, setting each denied global to `nil` returned `Ok`, and all
eight were still readable afterwards. Measured with removal moved *before* `sandbox(true)`:
all 13 gone, from Rust and from inside a chunk, with all 8 permitted globals intact.

**A second measured fact corrects the spec.** `spec.md` Technical Considerations names
`getfenv`/`setfenv`/`collectgarbage`/`newproxy` as the ones Luau's sandbox leaves standing.
It leaves **eight** standing:

| Removed by `sandbox(true)` alone | Survive it — the host must remove these |
|---|---|
| `io`, `package`, `load`, `dofile`, `loadfile` | `os`, `require`, `loadstring`, `debug`, `getfenv`, `setfenv`, `collectgarbage`, `newproxy` |

`os`, `require`, `loadstring` and `debug` surviving is the load-bearing part —
`crates/mc-script/CLAUDE.md` invariant 1 reads as though `sandbox(true)` handles them.
It does not. The deny list is explicit, ordered before the sandbox call, and its
completeness is what FR-1.2-S1 measures.

*Counter-argument:* an explicit deny list is a denylist, and denylists rot as Luau adds
globals. Accepted, with the mitigation that FR-1.2-S1 enumerates all fourteen as an exact
verdict, so a Luau upgrade that reintroduces one reddens rather than passing. **Amended per
discussion:** that mitigation covers *reintroduction* and not *addition*, so the permitted side is
now enumerated too — the set of names reachable from a chunk is asserted equal to a set the host
declares, and a global a future Luau or `mlua` release adds is reported rather than silently
reachable. See DF10.

**Amended per discussion — `print` is a sandbox escape, not an unrouted output channel, and it
belongs in the enumerated verdict on the same footing as the fourteen.** Luau's `print` writes with
`fwrite` to C `stdout`, a *different buffer* from Rust's, flushed at process exit. Installed *after*
`sandbox(true)`, a host `print` is defeated: `print = nil` in content lands in the child
environment, `__index` falls through to the original C `print` still sitting in the readonly
globals, and the script writes to **raw fd 1, outside all host control** — measured, with a positive
control distinguishing a clean detector from an absent one. That is content reaching a host
capability the sandbox was supposed to have taken away, which is the definition of an escape and
not a logging inconvenience. It is therefore enumerated and tested rather than noted as considered.

**The binding part is the ordering, and it is stated here so an implementer cannot miss it:
the host `print` is installed BEFORE `sandbox(true)`. Installing or removing it afterwards is a
silent no-op that still returns `Ok`.** One ordering rule governs the deny list and the `print`
install alike, for the same underlying reason — after the sandbox closes, writes to the globals
land in a child table that reads through `__index` to an untouched parent.

**The structural finding is the one to carry forward, because it says why this was missed.** The
deny list was derived on the escape axis, and `print` was on that axis the whole time. It survived
because the derivation asked *"which globals should be removed?"* and never asked *"what does
`StdLib::ALL_SAFE` actually leave standing?"* — a list nobody had enumerated. Two of A1's own
measured corrections have the same shape: four of the eight survivors were not the four the spec
expected. **That is the justification for the exact-set enumeration test** (DF10): a denylist
derived by judgement will keep missing whatever nobody thought to name, and the only defence that
does not depend on thinking of it is enumerating what is actually there and comparing.

**Amended per discussion — `coroutine` is permitted and enumerated, not denied.** The case for
denying it was that `coroutine.resume` mirrors the hostile `pcall` construction A3 exists to
defeat. Measured otherwise: the interrupt fires normally inside both `coroutine.resume` and
`coroutine.wrap`, so neither limit is void there. The abort *is* catchable — by `resume`, by a
`pcall` around `wrap`, and by a `pcall` inside the coroutine — and the latch defeats all three,
including the adversarial shape of respawning a fresh coroutine after every abort. One caveat for
the implementer: `mlua`'s interrupt dispatch carries a recursion guard that silently *continues* on
a re-entrant interrupt, which matters if latch logic ever calls back into Lua. It must not.
One measured limit on attribution comes with it: inside a coroutine only stack level 0 is visible,
because the resuming frames live on another `lua_State`.

### A2 — Every content chunk is evaluated in its own frozen environment — BINDING *(amended per discussion — DF11)*

**`sandbox(true)` alone does not satisfy FR-1.1-S1.** Measured: with the sandbox on,
`newname = 1` from a content chunk is **allowed** — `luaL_sandboxthread` hands the thread
a *writable* child globals table, which is the whole point of Luau's sandbox (protect the
host's globals, let the script have its own). The spec requires the assignment to be
*rejected*.

Mechanism: each chunk is loaded with `Chunk::set_environment(env)`, where `env` is a fresh
table whose metatable's `__index` is the sandboxed globals, whose `_G` is itself, and on
which `Table::set_readonly(true)` has been called. **The metatable is set readonly too** —
`set_readonly` freezes the table, not the metatable behind it, so without this
`getmetatable(_G).__index = {}` succeeds inside a chunk. It grants no capability and
cannot leak across chunks, but "frozen" should mean frozen and the fix is one line.

Measured against this environment, one case per scenario:

| Chunk | Result | Scenario |
|---|---|---|
| `newname = 1` | rejected — `[string "content-chunk"]:1: attempt to modify a readonly table` | FR-1.1-S1 (names the chunk) |
| `local x = 42 return x` | `42` | FR-1.1-S2 |
| `string.format = function() end` | rejected; a later chunk still gets `7` from `string.format('%d',7)` | FR-1.1-S3 |
| `rawset(_G, 'smuggled', 1)` | rejected; a later chunk reads `smuggled` as `nil` | FR-1.1-S4 |
| `getmetatable('').__index = {}` | rejected | FR-1.2-S3 |
| `math.floor(3.7)` | `3` | FR-1.2-S2 |

FR-1.1-S4's audit note — "a frozen environment built on `__newindex` is defeated by
`rawset` and one built on a readonly table is not" — is settled in favour of the readonly
table, by measurement.

*Counter-argument:* a per-chunk environment means chunks cannot share state through
globals, so multi-file mods will need an explicit sharing mechanism rather than getting one free.
That is the correct default for untrusted content and the sharing should be designed deliberately
when multi-file mods land; it is a cost, not a defect. **Amended per discussion — that answer used
to rest on principle alone, and it no longer has to.** The measured `print` case below is a concrete
attack the shared alternative admits and this one does not, so the trade is a named benefit against
a named cost rather than a preference for strictness.

**Amended per discussion — the per-chunk environment is load-bearing for something nobody had
connected to it, and that raises its status from tidiness to containment.** With a *shared*
environment, one mod's `print = function() end` silences every other mod's `print` — measured, zero
host calls from a second chunk after the first shadows it. Per-chunk `Chunk::set_environment` with
`__index → globals` fixes it (measured: one host call from the second chunk, with the first still
shadowed for itself). A2 is therefore not merely how FR-1.1-S1 is satisfied; it is what stops one
mod removing another mod's only diagnostic channel.

**Read the sentence above as a property of the alternative, not of what ships**, because two of its
clauses stop holding once A2's environment is in place and phases 3–9 will read this paragraph.
Under a *shared* environment, `rawset`, `_G.print` and `setmetatable` are refused by the readonly
table while plain assignment is not — that is the defect. Under the **per-chunk frozen** environment
this decision takes, plain assignment is refused too, with the message FR-1.1-S1 asserts. And
`getfenv().print` is not a live route at all under either: `getfenv` is one of the denied globals
and is removed before the sandbox closes, so it names nothing from a content chunk.

### A3 — Limit aborts latch, and **every** entry into script is guarded — BINDING *(amended per discussion — DF7, DF12)*

The spec calls the latch its most consequential audit finding and marks it not optional.
**Measured, and it is required.** With a non-latching interrupt that raises exactly one
error, `pcall(function() while true do end end)` **caught the abort and the script carried
on** — the chunk returned normally. Without a latch, `while true do pcall(f) end` re-enters
the budget indefinitely and invariant 2 is decorative, exactly as the audit predicted.

Mechanism: the interrupt callback consults a Rust-side guard holding
`Clear | Budget | Memory`. On exhaustion it transitions out of `Clear` and returns `Err`;
**while not `Clear`, every subsequent interrupt returns `Err` immediately**, so no script
frame can make progress after the trip. The host clears the latch at the start of each
guarded entry, which is what makes budgets per-invocation (FR-2.1-S4) rather than a
one-way ratchet.

**Chunk evaluation is a guarded entry, on the same terms as callback invocation.**
`crates/mc-script/CLAUDE.md` invariant 2 is categorical — *"There is no unbudgeted path
from engine into script"* — and `evaluate` is a path from engine into script. A chunk
whose **top level** is `while true do end` would otherwise hang the host, and hang the
test binary, since FR-7's harness evaluates hostile scripts. Faults from chunk evaluation
carry `subject: None, component: None` and the chunk name in `origin`, which is why those
two fields are `Option`. **No scenario reaches a runaway top-level chunk** — every chunk in
FR-1.x terminates and FR-7.1-S5's failure is a compile error — so `/sdd-tasks` must add a
test for it rather than expecting the mapping to produce one.

Measured with the latch installed: `spin_pcall` and `bomb_pcall` — both wrapping their
hostile body in `pcall` inside an outer infinite loop — end as `BudgetExhausted`, and a
following ordinary invocation returns `1000` from its full fresh budget (FR-2.1-S3,
FR-2.1-S4).

**Amended per discussion — the limit is misnamed and the field is renamed to
`call_and_loop_budget`.** It is not an instruction budget. The interrupt is emitted at exactly
seven opcodes — call, fastcall, return, numeric-for iteration, generic-for iteration, backward jump
and long jump — so what it counts is **calls and loop edges**. A thousand straight-line statements
cost one tick, and a loop whose body is ten statements costs exactly what an empty loop costs,
because the body is free. A Lua call costs two ticks and a host call costs one. Measured, and the
consequence is not academic: a bare 16³ pass costs 4,369 ticks, the same pass with one **Lua**
helper call per cell costs 12,561, and the same pass with one **host** call per cell costs 8,465 —
so at a budget of 10,000 the ordinary shape of content code, one helper call per cell, does not
fit. Leaving the field named `instruction_budget` guarantees somebody sizes it against VM
instructions and is wrong by the size of the loop body. Sizing and the eventual split are DF7.

**Amended per discussion — some builtins are not interruptible, and the memory cap is what bounds
them.** `string.rep` and `table.concat` run to completion between two interrupt ticks regardless of
the budget: measured 80.9 ms and 157.8 ms at a budget of 10,000. They are bounded rather than
unbounded, because they must allocate, so A4's cap is the thing that stops them — at a 16 MiB cap
the same `string.rep` is refused in 0.1 ms. **The worst non-interruptible pause the host can
suffer is therefore "the time it takes to fill the memory cap"**, which is a real number an operator
can be told rather than an unbounded hang. Checked and found *interruptible*: `string.find` with
catastrophic backtracking (aborted at 1,001 ticks, and Luau's matcher carries its own depth guard)
and `table.sort` both with and without a Lua comparator. **A gap is recorded rather than guessed
at:** no exhaustive sweep was made for a slow *non-allocating* builtin, and if one exists the memory
cap does not bound it. This is R3's residual and is written there too.

### A4 — The memory cap is enforced by the interrupt; `set_memory_limit` is the backstop — BINDING *(amended per discussion — DF1, DF2)*

**`Lua::set_memory_limit` alone does not satisfy FR-3.1-S4.** Measured: with a 1 MiB limit
set and no other mechanism, a `pcall`-wrapped allocation bomb looped **ten times and
returned normally** — each caught `MemoryError` dropped the table, the collector reclaimed
it, and the next round started again. The allocator-raised error is an ordinary catchable
Lua error, so the cap is defeated by exactly the construction FR-2.1-S5 covers for the
budget.

Mechanism, measured to hold: the interrupt reads `Lua::used_memory()` each tick against the
configured cap and **latches via A3**; `set_memory_limit` is set *above* the enforced cap as
a hard backstop, so a single allocation large enough to jump the gap between two interrupt
ticks still fails rather than sailing past. Measured with cap = baseline + 1 MiB and
backstop = cap + 512 KiB: the same `pcall`-wrapped bomb ends `Err` (latched).

Options rejected: **allocator limit only** — measured to fail FR-3.1-S4 (above);
**interrupt only, no backstop** — a single `string.rep` between two ticks allocates
unboundedly and the interrupt observes it only after the fact.

**The backstop bounds peak allocation; it does not itself contain.** If one allocation
jumps the inter-tick gap, `set_memory_limit` raises `MemoryError`, a script `pcall` catches
it, and because the *failed* allocation never landed, `used_memory()` is back **below** the
enforced cap — so the next interrupt tick sees nothing and does not latch. The script can
retry in a loop. It is still contained, but by the **call-and-loop budget**, so the fault
reports `BudgetExhausted` rather than `Allocation`.

**Amended per discussion — that path was dismissed as unreachable and it is not.** The draft
recorded it as a corner no scenario reaches, on the grounds that FR-3.1-S4's bomb grows gradually
and latches correctly. That is true of FR-3.1-S4 and false in general: the moment the VM sits near
the backstop for any reason — R1's retention being the ordinary one — an *innocent* invocation's
allocation jumps the remaining gap, is caught, is retried, and terminates as `BudgetExhausted`,
which FR-4.2-S5 counts toward quarantine. So this is not a fault-kind cosmetic issue; it is the
second door through which an innocent attachment gets quarantined for the host's condition, and it
is why DF1's trigger is stated over the *entry* condition rather than over the terminal fault kind.

**Amended per discussion — the backstop is a configured limit, not prose.** It moves into
`HostLimits` as `memory_backstop`. It is the single number that decides when scripting wedges for
everybody, and while it lived only in this document an operator could neither read it nor set it.
Its relationship to the enforced cap is a constraint the host checks at construction: the backstop
must exceed the measured VM baseline plus the enforced per-invocation cap, or the host is
configured into permanent memory pressure from the first invocation.

*Counter-argument:* `used_memory()` is whole-VM, so under A7 (one VM) the enforced cap
measures *the VM's* usage, not the invocation's. The host therefore snapshots
`used_memory()` at entry and caps the **delta**, which is what makes the fault attributable
to the running attachment. This bounds allocation *per invocation* and not memory *retained
across* invocations — see Risk R1, the main input to A7.

### A5 — An allocation fault is followed by an explicit collection — BINDING *(amended per discussion — DF2)*

FR-3.1-S3 requires that after an abort for exceeding the cap, a subsequent 512 KiB
allocation succeeds. **Measured: without an explicit collection it does not.** After the
abort, `used_memory()` stayed at 1,434,679 B and the following 512 KiB allocation failed
with `MemoryError("not enough memory")`. After two `Lua::gc_collect()` calls usage returned
to **exactly** the 385,952 B baseline and the 512 KiB allocation returned `512`.

Two calls, not one: the first ends the incremental cycle in progress, the second sweeps.
The collection runs only on the allocation-fault path, so ordinary dispatch pays nothing.

**Amended per discussion — the same measurement governs DF1's pressure test: pressure is decided
against a *collected* reading, never a raw one.** The 1,434,679 B measured above is garbage that
survives until an explicit `gc_collect()`, and a pressure comparison made on the raw figure would
fire on garbage rather than on retention — condemning the host to permanent "pressure" for memory
nothing is holding. The classification therefore collects before it decides. A raw reading may
*suggest* pressure and is a cheap pre-filter; only the collected reading may conclude it.

### A6 — Callbacks are invoked through a Lua-side `pcall` trampoline — BINDING *(amended per discussion — DF9, DF20)*

FR-4.1-S2 requires that a script raising a table with a `__tostring` metamethod produces a
fault **without that metamethod running**. **Measured: `Function::call` runs it.** mlua
installs `error_traceback` as the message handler for every protected call, and it calls
`luaL_tolstring`, which honours `__tostring`. Measured: one invocation via `Function::call`,
`__tostring` counter reads `1`.

Mechanism: the host holds one Lua function, `function(f, ...) return pcall(f, ...) end`,
created before the sandbox is closed, and invokes every callback through it. A script-raised
error then arrives as an ordinary **return value**, never as a propagating Lua error, so no
traceback handler touches it; the host renders it with a raw formatter that matches on
`ScriptValue` and never invokes a metamethod.

Measured across a full dispatch sweep — nine callbacks including `raise_table`, `spin_pcall`
and `bomb_pcall` — **`__tostring` invocations = 0** (FR-4.1-S2), while a chunk that calls
`tostring(e)` itself leaves the counter at `1` (FR-4.1-S4, the control).

**The trampoline and the latch compose, and that is the point.** The two outcomes are
distinguished structurally rather than by inspecting error text:

- trampoline **returns** `(false, value)` → the script raised, the host caught it as a
  value → `ScriptError`, text rendered raw;
- trampoline **cannot return** (`Function::call` yields `Err`) → a latched abort unwound
  past it → `BudgetExhausted` or `Allocation`, read from the guard.

A latched abort cannot be mistaken for a caught script error, because the latch is precisely
what stops the trampoline's own `return` from executing.

**Amended per discussion — that last sentence is an INFERENCE, not a measurement, and it is marked
as one.** What was measured is that the latch causes every subsequent interrupt to return `Err` and
that the affected invocations end as `Err`. That no path exists on which the trampoline both
latches *and* returns normally is reasoned from the latch's own construction, not observed. It is
the load-bearing inference of A6's whole two-outcome scheme, so it is the companion mutation for
R3: break the latch's re-arm and confirm that `spin_pcall` reddens rather than reporting a caught
`ScriptError`.

**Amended per discussion — the raw formatter has nothing to render on the allocation path.** It is
the right formatter for a script-raised error and it produces an empty string for an allocation
abort, whose underlying error carries no message at all. That cause is composed by the host, not
passed through; see A10.

**Amended per discussion — the `Err` + `Clear` state is defined rather than left to a fallback.**
The scheme above enumerates two outcomes and the guard admits a third: `Function::call` yields
`Err` while the guard is still `Clear` — no limit tripped, so the error came from `mlua` itself
rather than from this host's enforcement. It is classified from the `mlua` error discriminant:
an allocation error becomes `FaultKind::Allocation`, everything else becomes
`FaultKind::ScriptError`. Not from a default arm, and never from the error's text — a
pre-1.0 dependency's message formatting is exactly what A10's `line` field exists to avoid
depending on.

### A7 — One VM for the host at this spec; per-mod VMs revisited at PRO-917 — BINDING (with named revisit) *(amended per discussion — DF1, DF3)*

This is D6, the decision the spec left open. Measured facts first:

| Measurement | Value |
|---|---|
| Per-VM footprint, `Lua::new()` + `sandbox(true)` | **385,952 B** Lua-reported, **389,169 B** process-side |
| Linearity at 1 / 8 / 64 / 256 VMs | **exactly 389,169 B/VM at every count** — no amortisation |
| Construction cost | ~95–130 µs/VM |
| Footprint with `StdLib::NONE` instead of `ALL_SAFE` | 364,617 B/VM — **trimming libraries saves 6 %** |
| `set_memory_limit` scope | **per `Lua` state** (attaches to that state's `MemoryState`) |
| `set_interrupt` scope | per `Lua` state (`lua_callbacks(main_state)->interrupt`) |

So per-VM isolation is genuinely cheap (~380 KiB, ~100 µs), and the memory cap is genuinely
per-VM — the brief's expectation that this question would decide it was right. It decides
it, but not in the direction the framing suggests.

**Options.** (1) one VM owned by the host, memory cap enforced per invocation by A4's delta;
(2) one VM per component; (3) one VM per mod, which is what D6 describes and what ADR-004's
reload story implies.

**Option 3 is not implementable at this spec.** There is no mod. `spec.md` puts mod loading,
multi-file mods and `require` confinement in Out of Scope (PRO-917), and its vocabulary
defines subject and component as *"an opaque namespaced identity the host stores and never
interprets"*. A host forbidden from interpreting the namespace cannot derive a mod from a
component. Option 3 is PRO-917's decision, not this one.

**Option 2 is the one to reject explicitly, because it looks like the safe choice and is
not.** The distinguishing fact is *who controls the count*: component count is controlled by
**content**, mod count by a **loader**, which is to say by the operator. Making the VM count
a function of a content-controlled quantity turns a measured 389 KiB of *unreclaimable fixed
overhead* into a per-registration multiplier, spent at registration and therefore invisible
to every mechanism this spec builds — the budget, the memory delta and quarantine all act
during an invocation, and this cost is incurred before one happens. That is an invariant-3
breach manufactured by the isolation mechanism itself, and it needs no hostile intent. The
unit that makes per-VM isolation safe is one a loader controls.

*(A hostile mod registering 10⁴ components would cost ~3.8 GB, but that figure is an
illustration of the shape, not a predicted workload — a realistic mod set is hundreds of
components, ~195 MB, which is a real cost and not a breach. The argument that carries is
loader-versus-content, not the arithmetic.)*

**Recommendation: Option 1.** No FR-3 scenario distinguishes the options — the spec says so
and it is verified: every FR-3 scenario is stated at the boundary a script observes, and all
four are satisfied by A4 + A5 under one VM (measured). The reversal is cheap and the seam is
named: budget, quarantine, fault counting and the cascade queue are all Rust-side and keyed
by `Attachment`, none of them touching the VM. Adding VMs later means `ScriptHost` holding a
map from isolation unit to VM and dispatch looking one up — `src/luau/` changes, dispatch
does not.

**Amended per discussion — `ScriptTable` and `ScriptFunction` carry an opaque isolation-unit tag
from the first design, and the reason recorded for it is hot reload, not the extension-API
exemption.** The reason matters more than the field. Hot reload builds its candidate registry in a
**scratch VM** — a second Lua state, wanted on lifecycle grounds that have nothing to do with R1 —
and its entire job is substituting a scratch-VM `ScriptFunction` for a live one. A handle that does
not say which state it came from makes that substitution unverifiable in the one path whose
partial-failure mode `crates/mc-script/CLAUDE.md` calls a Blocker. That is a correctness
requirement of a spec already scheduled, not speculation about a second VM that may never arrive.

**It is explicitly NOT justified by `code-quality.md`'s published-extension-API exemption**, and
recording that wrong reason would be worse than recording none: the exemption is scoped, in its own
words, to the *published scripting surface*, and these are Rust handles consumed by sibling crates.
Anyone reading the standard correctly would find the justification void and delete the field —
plausibly two weeks before hot reload needs it. The field ships as
`ScriptFunction { handle, unit, origin_chunk }` and `ScriptTable { handle, unit }`: all host-known
metadata, set at the one moment a handle is created, opaque and never parsed. Today exactly one
unit value exists, and that is the point — a tag whose domain has one member costs one field, and
makes the day it has two a change inside `src/luau/` rather than at every consumer.

**Strongest argument against.** Under one VM, memory *retained across* invocations is not
bounded per attachment. A callback is a closure and a closure holds upvalues, so
`local kept = {} return function() kept[#kept+1] = string.rep('x', 1000) end` retains
without any state API at all. It trips no per-invocation delta cap, and as the VM's absolute
backstop is approached, *every* attachment's allocations begin failing and each fault is
charged to whoever happens to be running. Option 3 would fix this exactly and cheaply. This
is Risk R1.

**Also taken — a named pressure fault, so the failure above is observable rather than
silent.** The host already reads `used_memory()` at entry (A4), so this is one comparison on a
value already in hand: while the host is under memory pressure, a fault is reported as
`FaultKind::HostMemoryPressure` with `subject`/`component` `None` instead of being attributed to
the running attachment. It does not fix R1 — nothing can without a loader-controlled isolation unit
— but it converts silent misattribution into a named, observable condition, which given that
attribution is a top-line driver is worth one enum variant and one branch.

**Amended per discussion — the trigger is *derived*, not tuned, and the earlier "a documented
fraction of the absolute backstop" is withdrawn rather than left standing.** A fraction is a
constant somebody has to choose, defend and re-choose whenever the backstop moves, and no
measurement was ever going to tell them what it should be. The condition that actually matters is
statable exactly:

> `entry_baseline + memory_cap > memory_backstop`

— *this invocation could fail for a reason that is not its own*. While it holds at entry, faults
are classified `HostMemoryPressure` and do **not** count toward the attachment's consecutive-fault
total. No constant, nothing to tune, and it is the literal statement of the thing being detected
rather than a proxy for it. It is also **cheaper** than what it replaces, which is why it is
justified on correctness grounds and not on security ones: it removes a parameter instead of
adding a mechanism. The baseline it reads is the collected one, per A5.

**The cost is named, because it is real and it is not small.** The condition is a property of the
whole VM, not of the attachment, so two things follow and both were weighed:

- an attachment whose *own* retention raised the baseline becomes immune to quarantine — the mod
  causing the pressure is the one the rule excuses;
- under *sustained* pressure, quarantine is inactive for every attachment and every fault kind,
  which re-opens the case FR-4.2-S5 exists to close: a looping mod that is never quarantined and
  burns a full budget on every attachment every round.

**Ruling: take the excuse, name the cost, and give it a scenario.** The deciding argument is which
failure is *visible*. Excusing means a looping mod is not quarantined: a CPU cost, bounded per
invocation, and **loud** — the server is slow, the operator notices, the operator acts. Not
excusing means an innocent mod is permanently disabled *and the engine attributes it to the wrong
author*: silent, misdirected, and it ends with the operator removing the wrong mod. With one
operator who chose every mod installed, the loud failure beats the silent misattribution.

Two further facts make it acceptable rather than merely chosen. The condition is **already
reported on every invocation** that faults under it, so "quarantine is off" is observable rather
than inferred; and sustained pressure is *already* the documented terminal state (DF16), so the
rule does not create a state the design otherwise avoided. Most decisively: **quarantine
functioning would not help anyway** — retention lives in closure upvalues that survive quarantine,
so disabling attachments reclaims nothing. The asymmetry that governs is time, not measurement: a
rare, loud condition is fine to excuse indefinitely; a steady state is not, which is what makes the
terminal state a state and not a mode.

The reconciliation with FR-4.2-S5 — which asserts the opposite for three fault kinds — is not left
to reading. It gets its own scenario, because "does not count toward the consecutive-fault total"
sitting silently beside a rule that says it does is how an unverifiable mechanism ships.

**Revisit when:** PRO-917 introduces a mod as a loader-controlled unit. Escalate sooner if
any spec gives content a way to retain state across invocations.

### A8 — The lint mechanism is a crate-root attribute; D7's manifest table is impossible — BINDING

**D7 as specified cannot be built.** `spec.md` (Technical Considerations) and
`requirements.md` D7 both say `crates/mc-script/Cargo.toml` gains a per-crate
`[lints.clippy]` table. Measured, against this exact toolchain:

```
error: failed to parse manifest at `.../member/Cargo.toml`
Caused by:
  cannot override `workspace.lints` in `lints`, either remove the overrides
  or `lints.workspace = true` and manually specify the lints
```

Cargo **hard-errors** on a manifest carrying both `[lints] workspace = true` and a local
`[lints.clippy]` table. This is better than the silent-drop the brief feared — the
regression is loud, not invisible — but the mechanism does not exist. Three things do:

1. **Drop `workspace = true` and restate every workspace lint locally.** Compiles, and is
   the trap: `mc-script` would carry a hand-copied duplicate of a 20-entry table that
   nothing keeps in step. A lint added to the workspace later silently would not apply to
   the one crate where it matters most, and no stage goes red.
2. **Raise the three lints to `deny` in `[workspace.lints.clippy]`.** Verified to cost
   nothing today — the tree contains **zero** `.unwrap()`/`.expect()` call sites and no
   `#[allow(clippy::unwrap_used)]` — and it would cover every target in every crate. Not
   taken because it changes nine crates this spec was not asked about, but it is the option
   to reach for if the scope difference in 3 ever bites.
3. **Keep `workspace = true`; put the denials in a crate-root inner attribute.** Taken.

`crates/mc-script/src/lib.rs` gains

```rust
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
```

Measured to compose rather than replace: with the workspace table inherited unchanged,
`unwrap_used` became a hard **error** while the inherited `dbg_macro` still reported as a
**warning** — the attribute raises the three named lints without displacing the other
twenty. It delivers what D7 wanted, which was for `crates/mc-script/CLAUDE.md` invariant 4
to be true at plain `cargo check` rather than only under the gate's `-D warnings`.

**One scope difference from D7's manifest table, stated so nobody assumes parity:** the
attribute covers the lib target and its sibling `_test.rs` modules, but **not**
`crates/mc-script/tests/*`, since each integration test is its own crate root. The gate's
`-D warnings` still promotes the workspace `warn` there, so nothing regresses relative to
today — the integration tests are exactly as protected as every other crate's are.

`spec.md` is **not edited** by this document; the architecture supersedes the mechanism.
Challenged and upheld; `spec.md` still takes a one-line correction at `/sdd-complete`.

*Counter-argument:* an attribute in `lib.rs` is easier to delete than a manifest table and
sits where a reader looking at build configuration will not find it. Mitigated by a
doc-comment naming the invariant it enforces.

### A9 — Dispatch is a bounded queue, bounded in **both** dimensions, drained across rounds — BINDING *(amended per discussion — DF5, DF13)*

D4, carried unchanged and load-bearing. A callback returns follow-up work as opaque
attachment identities; the host enqueues them and drains the queue in the same round. A
callback never synchronously re-enters dispatch, because synchronous re-entry converts an
unbounded script cascade into Rust stack growth and a stack overflow is an abort — the one
outcome invariant 3 forbids, reached by the mechanism a depth counter catches last.

**Two bounds, because the round bound alone bounds the wrong thing.** `round_bound` limits
*invocations per round*; it says nothing about *queue length*. A callback returning a
fan-out list of N identities grows the queue by N−1 per invocation, 64 invocations per
round, across unbounded rounds — and every one of those entries is a Rust-side allocation,
so it sits outside the Luau memory cap (A4 measures `used_memory()`, which is the Lua
state), outside `set_memory_limit`, outside the call-and-loop budget, and outside quarantine,
since the requester succeeds every time. D4's claim that queueing "converts recursion depth
into queue length, which is countable, boundable and observable" is only true once something
actually bounds it. `HostLimits` therefore carries **`pending_bound`**, and an entry that
would exceed it is **refused**.

A cascade fault is emitted on either of two conditions, both naming the **requester**:

- **Refusal** — the pending queue is full, so the follow-up is never admitted.
- **Deferral past the round's end** — the round reached `round_bound` with work still
  pending.

Queue entries therefore carry who asked for them, `(target, requested_by: Option<Attachment>)`.
That is what makes FR-5.1-S2's "naming the subject and the component that produced the
overflow" answerable: the blamed attachment is the *requester* of the first entry that could
not run, not the entry itself.

**Amended per discussion — the two conditions are two fault kinds, `CascadeRefused` and
`CascadeDeferred`, not one `CascadeBound`.** The draft named them together and priced only one of
them: deferral's operator-facing noise is costed two paragraphs below, and refusal was never
priced at all. They are not the same event. Deferral means the work is progressing normally and
will run next round; refusal means the work was **dropped and will never run**. D4 names the
consumer that makes the difference concrete — a full pending queue silently drops a neighbour
notification, so a furnace never learns its neighbour changed, and the failure is a piece of
content that quietly does nothing rather than a server that is slow. An operator reading one fault
kind cannot tell those apart, and it is the difference between "wait" and "something is lost".

**`CascadeRefused` names both the requester and the refused target; `CascadeDeferred` stays
requester-only.** The asymmetry is the point: for a dropped item you must know *what* was dropped,
and for a deferred one there is nothing to name because nothing is missing. Both facts are already
in the queue entry, so this costs a field and no bookkeeping.

**The refused target is a typed field, `refused_target: Option<Attachment>` on `ScriptFault` —
never text inside `cause`.** This is A10's own argument for `line: Option<u32>` applied unchanged:
a structured fact buried in a string formatted by a pre-1.0 dependency leaves substring matching as
the only available assertion. And `FaultKind` stays a plain, comparable enum carrying no data, so
`ContainmentEvidence::FaultReported(FaultKind)` keeps comparing by equality. Had the target been
carried *inside* the variant, that comparison would have had to degrade to matching on the
discriminant, which is exactly the hostile-case evidence check FR-7.1-S4 relies on being exact.

**Why deferral and not merely "reached the bound".** FR-5.1-S4 runs a cascade of exactly 64
at a bound of 64 and requires *no* cascade fault of either kind; the 64th invocation completes and
leaves the queue empty. Reaching the bound cannot itself be the trigger. Conversely,
FR-5.1-S2's non-terminating cascade and FR-5.1-S3's terminating 200-chain are locally
indistinguishable at the end of round 1 — both hit the bound with work pending — so eager
reporting is forced by S2. The consequence, recorded so it is not later read as a defect: a
perfectly well-behaved terminating cascade emits three `CascadeDeferred` faults naming three
different attachments before completing. That is operator-facing noise the spec permits — and it
is noise precisely because deferral loses nothing, which is why it must not wear the same name as
refusal.

**The pending queue survives the end of a round.** FR-5.1-S3 requires a terminating cascade
of exactly 200 at a bound of 64 to complete over 4 rounds (64+64+64+8) and report 200 total,
so the residual is host state carried forward. Calling convention: a `dispatch` seed is
**appended** to pending rather than replacing it, and `dispatch(&[])` drains what is left.

**Neither cascade fault kind counts toward an attachment's consecutive-fault total.**
FR-4.2-S1 counts "an attachment's callback [faulting] on three consecutive **invocations**" —
the counting unit is the outcome of an invocation, and a cascade fault is not one: the
invocation completed and returned, and the fault is a property of the round's admission
control. FR-4.2-S5 corroborates by deliberately enumerating the kinds that do count (budget,
memory, script error), an enumeration `requirements.md` records as a settled audit gap rather
than an oversight. *(An earlier draft derived this from FR-5.1-S3 instead, arguing the
cascade would self-quarantine by round 3. That derivation is wrong — D3 resets the count on
success, and the blamed requester succeeds dozens of times per round, so the faults are never
consecutive and no quarantine occurs. The rule stands; only its reasoning changed.)*

A quarantined target found in the queue is skipped silently, with no fault (FR-5.1-S5);
skipping does not consume an invocation against the round bound.

### A10 — Fault type follows the two ports already in the tree, and diverges from them once — BINDING *(amended per discussion — DF4, DF5, DF15, DF20)*

`DefinitionFault` and `HudFault` are both `origin` + which-thing + which-field + `cause` with
a hand-written `Display`. `ScriptFault` reads the same way, with `kind` replacing `field`
because *why* it failed is the discriminating axis for a script fault where *which field* is
for a definition.

**Two deliberate divergences, stated so a reviewer does not "fix" them back:**

- The precedents sit under a two-level `Unreadable`/`Malformed` split
  (`DefinitionSourceError`). `ScriptFault` is **flat**, with an enumerated `FaultKind`. The
  split exists to separate "the source could not be read at all" from "one item in it was
  wrong", and a script host has no such division — every fault is about one entry into
  script.
- `ScriptFault` carries a typed **`line: Option<u32>`**, which neither precedent has.
  FR-4.1-S3 requires a compilation fault "naming the chunk **and line 3**", and with the line
  buried in `cause` the only available assertion is `cause.contains(":3:")` — substring
  matching on a string formatted by a pre-1.0 dependency (R4). The adapter parses Luau's
  measured `[string "name"]:N:` prefix inside `src/luau/`, which is where vendor error
  translation belongs (`code-quality.md` §4), and the test asserts `fault.line == Some(3)`.

`FaultKind` is enumerated rather than stringly-typed for the same reason: FR-4.2-S5 (three
faults of three different kinds still quarantine) becomes expressible, and FR-7.1-S4's
per-case evidence declaration can be matched exactly.

**Amended per discussion — a third divergence: `refused_target: Option<Attachment>`**, for
`CascadeRefused` only, on the reasoning in A9. It is the same divergence as `line`, one step
further: a structured fact belongs in a typed field, and the enumerated `FaultKind` stays data-free
so evidence matching stays an equality comparison.

**Amended per discussion — `ScriptOrigin` carries the defining chunk from its first design.**
Today the design would give an *invocation* fault an origin of the dispatch round, which means the
most common fault in the whole system names no file at all — and a chunk-level fault carries
`subject: None`, `component: None` and no line, so a mod that fails to load reports
`BudgetExhausted` and nothing else. The fix costs nothing because **`ScriptOrigin` does not exist
yet**: this is a type being designed, not a type being changed, and shaping it to carry the
defining chunk alongside the round is one field decided at the only moment it is free. It is not
new fault *detail* either — it is an existing field populated with a locator instead of with host
bookkeeping. The chunk name reaches the fault by being stamped onto `ScriptFunction` at creation,
which is the same one-moment, all-host-known-metadata diff as the isolation-unit tag (A7).

**Measured, and recorded without a design commitment beyond the above:** a *line* is obtainable for
budget aborts. `Lua::inspect_stack` works from inside the interrupt callback and yields the full
frame chain — source, kind, definition line, current line, name — honouring `Chunk::set_name`, at
roughly 155 ns per tick, about twice the bare interrupt overhead. Allocation aborts carry
**nothing**: the error is literally `MemoryError("<nil>")`, with no line and no message, and a
traceback taken afterwards is empty because the stack has already unwound; the measured workaround
is for the interrupt to keep a last-known `(source, line)` each tick and let the allocation fault
report that. Inside a coroutine only level 0 is visible. What ships from this is the chunk name;
the line on runtime aborts is recorded as available and costed, not committed.

**Marked, because it is the one field content controls and nothing bounds it:** `cause` is
unbounded, script-supplied text rendered raw (A6). Raw is correct — rendering it any other way
means running script on the host's schedule — but "raw" and "safe to splice into anything" are
different properties, and every consumer that logs it, formats it or shows it to a human inherits
whatever a mod put there, at whatever length a mod chose.

**Amended per discussion — on the allocation path there is nothing to render, and the host
therefore SYNTHESISES the cause rather than passing one through.** Measured: the error mlua raises
is literally `MemoryError("<nil>")`, carrying no line and no message, and a traceback taken
afterwards is empty because the stack has already unwound. A6's raw formatter is right for a
script-raised error and, applied to this one, produces **nothing** — so the fault would name its
subject and its component and then say literally nothing about why. An empty `cause` and a `cause`
that was never populated read identically, which is the failure family this project keeps meeting.
The adapter in `src/luau/` therefore composes the cause on that path from what the host knows: the
configured cap, the observed usage, and the last-known source position the interrupt recorded. It is
the one place where host-authored text is correct rather than a leak of vendor formatting.

**This is also the strongest form of the typed-field argument above.** `line` and `refused_target`
are typed because a string is a bad place to keep a structured fact. `Allocation` goes further:
it is the one fault kind whose cause string is **guaranteed empty at the source**, so the structured
fields are not merely the better diagnosis, they are the only diagnosis there is.

### Trivial decisions

- Crate layout `src/{lib,host,dispatch,quarantine,fault,limits}.rs` + `src/luau/`; unit tests in sibling `_test.rs` per `docs/technical/testing.md`.
- `mlua` opts in as `mlua = { workspace = true }`. Never versioned in the member (root `CLAUDE.md`).
- Subject and component are newtypes over `String`, stored and compared, never parsed.
- `#[derive(Debug)]` on every public type — `missing_debug_implementations` is a workspace lint.

---

## Interfaces

Signatures the implementation must provide. `!Send` throughout; the host is not `Sync` and
does not pretend to be. **Amended per discussion** — the block below carries DF1 through DF8;
the changes are the renamed budget, `memory_backstop`, the isolation-unit tag and chunk stamp on
the two handles, the split cascade kinds with `refused_target`, `ScriptOrigin`'s defining chunk,
and `release`.

```rust
// ---- identity (opaque; never parsed) ----
pub struct SubjectName(String);
pub struct ComponentName(String);
pub struct Attachment { pub subject: SubjectName, pub component: ComponentName }

// ---- limits (D5: configurable, documented defaults; FR-9) ----
pub struct HostLimits {
    /// Ticks of the Luau interrupt, which fires on calls and loop edges — NOT on
    /// every VM instruction (A3). One value for chunk evaluation and callback
    /// invocation alike; the split is a condition, not a schedule (DF7).
    pub call_and_loop_budget: NonZeroU64,
    pub memory_cap: NonZeroUsize,      // bytes of script allocation above the entry baseline
    pub memory_backstop: NonZeroUsize, // absolute `set_memory_limit` ceiling for the VM (A4)
    pub fault_threshold: NonZeroU32,
    pub round_bound: NonZeroU32,
    pub pending_bound: NonZeroU32,     // max entries the pending queue may hold (A9)
}
impl Default for HostLimits { /* documented, finite, non-zero — FR-9.1-S1 */ }

// ---- faults ----
pub enum FaultKind {                   // plain and comparable: no variant carries data (A9)
    BudgetExhausted, Allocation, ScriptError, Compilation,
    CascadeRefused, CascadeDeferred, HostMemoryPressure,
}
pub struct ScriptOrigin {
    pub chunk: Option<ChunkName>,        // the defining chunk — populated from ScriptFunction
    pub round: Option<RoundIndex>,       // the dispatch round, for invocation faults
}
pub struct ScriptFault {
    pub origin: ScriptOrigin,            // the defining chunk and/or the dispatch round
    pub subject: Option<SubjectName>,    // None for chunk-level and host-pressure faults
    pub component: Option<ComponentName>,
    pub kind: FaultKind,
    pub line: Option<u32>,               // parsed in src/luau/ — FR-4.1-S3
    pub refused_target: Option<Attachment>, // Some only for CascadeRefused (A9)
    pub cause: String,                   // rendered raw — never via __tostring (A6);
                                         // unbounded script-controlled text (A10)
}
impl fmt::Display for ScriptFault { /* hand-written, per DefinitionFault */ }

// ---- values the host reads without running script (FR-6) ----
pub enum ScriptValue {
    Nil, Boolean(bool), Integer(i64), Number(f64), Text(String),
    Table(ScriptTable), Function(ScriptFunction), Opaque,
}
/// Opaque; one value exists today. Says which Lua state a handle came from, so
/// hot reload's scratch-VM substitution is checkable rather than assumed (A7).
pub struct IsolationUnit(/* opaque; never parsed */);
pub struct ScriptTable    { /* backend handle; no mlua type crosses this boundary */ unit: IsolationUnit }
pub struct ScriptFunction { /* ditto */ unit: IsolationUnit, origin_chunk: ChunkName }

// ---- the host ----
pub struct ScriptHost { /* owns the VM, the guard, the registry, the pending queue */ }

impl ScriptHost {
    pub fn new() -> Result<Self, HostError>;
    pub fn with_limits(limits: HostLimits) -> Result<Self, HostError>;
    pub fn limits(&self) -> &HostLimits;                                    // FR-9.1-S1

    /// Evaluates a content chunk in its own frozen environment (A2), under the
    /// same guard as an invocation (A3).
    pub fn evaluate(&mut self, name: &str, source: &str) -> Result<ScriptValue, ScriptFault>;

    /// Registers a callback against an attachment. Takes a `ScriptFunction`, not a
    /// `ScriptValue`, so "you passed a number" is unrepresentable rather than an
    /// untested runtime branch. Over an existing attachment it **replaces** the
    /// callback and **lifts quarantine** — hot reload is exactly this operation, and
    /// a replace that did not lift quarantine would silently fail at the one thing
    /// reloading a broken mod exists to do (DF6).
    pub fn attach(&mut self, attachment: Attachment, callback: ScriptFunction);

    /// Lifts quarantine, leaving the callback in place. Returns whether the
    /// attachment was quarantined. Not `detach`: nothing unloads mods yet (DF6).
    pub fn release(&mut self, attachment: &Attachment) -> bool;

    /// Drains one bounded round. Never re-entrant (A9). The seed is appended to
    /// pending work; `dispatch(&[])` drains the residual of a previous round.
    pub fn dispatch(&mut self, seed: &[Attachment]) -> DispatchReport;

    /// Reads a field from a script-supplied table without invoking script (FR-6).
    pub fn read_field(&self, table: &ScriptTable, field: &str) -> Option<ScriptValue>;

    pub fn is_quarantined(&self, attachment: &Attachment) -> bool;          // FR-4.2-S2

    /// Cumulative telemetry belonging to the **attachment**, not to the callback:
    /// it resumes from its frozen value after a release or a replace rather than
    /// resetting. Distinct from the consecutive-fault count, which resets on a
    /// success (D3) — two counters answering two questions (DF6).
    pub fn invocation_count(&self, attachment: &Attachment) -> u64;         // FR-4.2-S6
}

pub struct DispatchReport {
    pub order: Vec<Attachment>,       // entry order — FR-5.1-S1
    pub invocations: u32,             // this round
    pub faults: Vec<ScriptFault>,
    pub quarantined: Vec<Attachment>, // newly quarantined this round
    pub pending: u32,                 // carried into the next round — FR-5.1-S3
}
```

**The harness (FR-7) is the spec's named deliverable and gets designed here, not left to
the implementer.** Its verdict is three-valued, not two: conflating "uncontained" with
"produced no fault" was one of the audit's five real holes, because `sandbox-escape` and
`hostile-index` are contained precisely *by* producing no fault.

```rust
pub enum ContainmentEvidence {
    FaultReported(FaultKind),         // infinite-loop, memory-bomb, faulting-callback, runaway-cascade
    EveryDeniedGlobalUnavailable,     // sandbox-escape
    MetamethodNotInvoked,             // hostile-index
}
pub enum CaseOutcome { Contained, Uncontained, NotExercised }   // FR-7.1-S2 / S3 / S5
pub struct HostileCase { pub name: &'static str, pub requires: ContainmentEvidence }

pub fn hostile_cases() -> [HostileCase; 6];                      // FR-7.1-S1, FR-7.1-S4
```

An enumerated `CaseOutcome` is what `standards/global/testing.md` §2 asks for — it stops a
harness that stopped running from reading identically to one that ran clean, which is exactly
the failure `NotExercised` exists to name (FR-7.1-S5).

**Error contract.** `HostError` covers host construction and is not a script fault. Everything
a script can cause is a `ScriptFault` — a value, never a panic, never an `Err` carrying an
`mlua::Error`. `unwrap`/`expect`/`panic!` are denied crate-wide (A8).

**Callback return convention.** A callback returns either a value or a list of follow-up
attachment identities; the host reads that list with raw reads, so a hostile `__index` on the
returned table cannot run on the host's schedule (FR-6 discipline applied to the host's own
reads).

**Amended per discussion — a payload on the queue entry was proposed and is DECLINED, with a
tripwire rather than a note.** The motivating case is real: a job too large for one budget has no
supported continuation except a closure upvalue, which is R1's own retention mechanism, so the
design's only answer to "do a big job" is the construction its main risk is about. A payload does
**not** close that, because R1's mechanism is *any* closure upvalue and removing one motivating
case leaves it intact — which is why the ask was withdrawn rather than deferred. What is taken is
the **record shape only**: a follow-up entry is a struct, not a bare tuple, so a payload field is
additive whenever it is earned. The obligation is written as a condition on an action, because a
note ages into permission and a condition fires:

> **The first spec requiring cross-invocation continuation must evaluate a queue payload before
> extending the callback return convention.**

---

## Data

No persistence, no migration, no sensitive data. All state is in-process and dies with the
host:

| State | Shape | Lifetime |
|---|---|---|
| Registered callbacks | `Attachment → ScriptFunction` | until the host drops |
| Consecutive fault count | `Attachment → u32`, **reset on success** (D3) | per attachment |
| Quarantine set | `Set<Attachment>` — the pair, never the component (D2) | until released or replaced (DF6) |
| Invocation count | `Attachment → u64`, frozen at quarantine (FR-4.2-S6), **resumed** on release or replace | per attachment |
| Pending work | `VecDeque<PendingEntry { target, requested_by }>` — a struct, not a tuple, so a payload stays additive (DF13) — **bounded by `pending_bound`** (A9) | across rounds |
| Guard | `{ used, budget, memcap, latch }` | per guarded entry, cleared at entry (A3) |

Maps are `BTreeMap` keyed by `Attachment` — ordering is deterministic and the counts are
small; nothing in this spec calls dispatch from a tick.

**Amended per discussion — that all of it dies with the process is what makes the terminal state
survivable today, and it is not a property that lasts.** Sustained memory pressure (DF1) is a
recorded terminal state rather than a solved problem: the host does not recover from it on its own,
because retention lives in closure upvalues that no mechanism here can reach. Recovery is an
operator restart. Today that costs nothing — all host state dies with the process, and nothing in
`crates/` depends on `mc-script`, so there is no session to lose. **Once the simulation drives
dispatch, the same recovery is a live server restart with players on it.** The escalation is
recorded here, at the state table that explains why it is currently free, so that whoever connects
dispatch to the tick meets it rather than inherits it.

---

## Integration

| Touched | What connects | What must not break |
|---|---|---|
| `crates/mc-script/Cargo.toml` | `mlua = { workspace = true }` | `[lints] workspace = true` stays **exactly as-is** (A8) |
| `crates/mc-script/src/lib.rs` | replaces the 3-line stub; carries the `#![deny(...)]` attribute (A8) | — |
| `crates/mc-testkit/tests/workspace_layering.rs:43` | `("mc-script", None)` → `("mc-script", Some("mlua"))` | The pair is that test's **positive control**. It goes red when `mlua` lands; that is the guard working. Update it in this diff; do not weaken `ClosureIsTheCrateAlone` for other crates. |
| `crates/mc-script/tests/` | new: FR-8 closure guard, the `mlua` containment guard (Boundaries), the hostile harness | All walk `cargo metadata` or source text, creating no build edge |
| `crates/mc-script/CLAUDE.md` invariants 1 and 3 | invariant 1 reads as though `sandbox(true)` removes `require`/`os`/`debug` (A1 measured otherwise); invariant 3's "allocation failure surfaces as a Lua error, never an abort" is true but incomplete — A4 measured that a `pcall` then defeats it | Correct both in this spec's diff. A future spec author reads the invariant list first, and `docs/modding/sandbox.md` does not fix a stale invariant. |
| `docs/technical/decisions.md` ADR-003 | its Consequences say "`Lua::set_memory_limit` caps allocation", which A4 measured to be defeated by `pcall` | **Settled per discussion, both halves.** ADR-003's *decision* (Luau via mlua) is binding and not reopened; its Consequences carry a **factual correction** — the claim is measured false. Separately, **a new ADR is required at `/sdd-complete`** (the tree is at ADR-021, so ADR-022) for VM granularity, which is its own decision and **not** an ADR-003 amendment. It records one VM, the loader-versus-content argument that rejects per-component VMs, R1 and its accepted residual, the derived pressure trigger with the accident framing rather than the hostile one, and the retention ledger as a priced refusal. It is **not written here** — this is the requirement, not the ADR. |
| `docs/modding/sandbox.md` | pre-committed consolidation destination (`docs/INDEX.md:76`) | Created by `/sdd-complete`, not by implementation. **It must answer "how do I do a job bigger than one budget?"**, and today the only honest answer is the retention construction — a closure upvalue holding a cursor across invocations, which is R1's own mechanism. That loop is closed knowingly, and it closes directly beside the authoring rule below, which tells the same author their cleanup will not run on an abort. Both go in; neither is softened. |

**Harness placement.** `crates/mc-script/tests/support/hostile/`, driven by a sibling
integration test — following `crates/mc-client/tests/support/input/`, **not** `mc-testkit`,
which is structurally forbidden from reaching what it verifies. Per the
`crates/mc-client/tests/seam_boundaries.rs` precedent the harness needs a text guard that it
**re-implements no containment policy**: a harness defining its own budget, its own latch or
its own deny list would agree with the host by construction and report six contained cases
while the host's enforcement was deleted. That guard needs its own positive control.

**FR-8's guard is a third `cargo metadata` walker, deliberately.**
`crates/mc-testkit/tests/workspace_layering.rs` already inspects `mc-client` and could be
extended instead. Not taken, on that file's own recorded reasoning: *"an integration test is
its own crate, the two invariants are independent, and a `tests/` module carried by both
files would be the same amount of code in a less obvious place."* The `crates/` ↔ `tools/`
direction and "the client cannot reach the scripting host" are independent invariants.

**Nothing in `crates/` gains a dependency on `mc-script`.** `mc-client`'s closure excluding it
(FR-8) is currently true for the trivial reason that *nothing* depends on it; FR-8.1-S2 and
FR-8.1-S3 are the vacuity guards that make the verdict mean something.

---

## Assumptions

Each is a declared assumption a reviewer may veto, not a measured fact.

1. **`HostLimits::default()` values are chosen by the implementer against measurement, not
   fixed here.** FR-9 requires only that they are finite, non-zero, documented and enforced.
   Note the two quantities are different and must not be conflated: the enforced `memory_cap`
   is a **delta above the entry baseline**, so its floor is set by how much a callback
   plausibly needs and a 256 KiB cap is perfectly expressible; it is the `set_memory_limit`
   **backstop** that must exceed the measured **385,952 B** baseline plus that delta. **Amended
   per discussion:** the backstop is now `HostLimits::memory_backstop` rather than a value the
   adapter picks privately, so that relationship is a constraint the host can check and an
   operator can read (A4). The budget's default is **sized for the largest plausible unsliceable
   chunk**, on the reasoning in DF7.
2. ~~**Interrupt tick granularity is fine enough for the instruction budget to mean
   something.** Measured: a 10,000-instruction budget produced 10,001–10,002 interrupt calls, so a
   tick is approximately one VM instruction.~~ **WITHDRAWN — measured false (DF7, A3).** The
   observation was real and the inference from it was wrong: the loop that produced those 10,001
   calls was ticking once per *iteration*, not once per instruction. The interrupt is emitted at
   seven opcodes only, so a tick is one call or one loop edge and a loop body of any size is free.
   What replaces the assumption is a measurement, not a weaker assumption — see A3, where the
   field is renamed accordingly.
3. **The budget and the memory cap interact, and a test must not let one mask the other.**
   Measured: the allocation bomb aborts as `BudgetExhausted` under a 10,000-tick budget, because
   filling 1 MiB costs more ticks than that — it reports `Allocation` only when the budget is
   generous. FR-3's tests must configure a budget that does not trip first. An assumption about
   *test construction*, for `/sdd-tasks`.
4. ~~**FR-9.1-S1 must assert the documented default *values*, not their type.**~~ **Superseded by
   DF8**, which keeps this reasoning and widens the assertion. The `NonZero*` types make "finite
   and non-zero" a compile-time guarantee, so a test asserting only that **cannot fail**
   (`standards/global/testing.md` §2) — that part stands, and the documentation and the constants
   stay on one source so changing a default reddens. What changes is the quantifier: the scenario
   asserts over **the set of limits the host reports**, not over four named ones, so a limit added
   later is covered without amending it. Two limits were added in this document's own lifetime
   (`pending_bound`, `memory_backstop`) and a scenario naming four would have silently stopped
   covering the ones that matter most.
5. **`ScriptOrigin` mirrors `DefinitionOrigin`/`HudOrigin` closely enough to be worth a third
   instance** rather than being generalised over. Three uses is where an abstraction becomes
   arguable; the deliberate choice is still three parallel types, because they are in three
   crates with three different which-thing fields.

---

## Risks

**R1 — Retained memory is not bounded *per attachment* under one VM.** The residual of A7.
Stated precisely, because both halves are easy to get wrong: aggregate retention **is**
bounded, by A4's absolute `set_memory_limit` backstop. What is unbounded is retention per
attachment, and the damage is **misattribution** — as the backstop is approached every
attachment's allocations begin failing and each fault is charged to whoever happens to be
running. The retention *mechanism* is reachable today, inside this spec's own harness, via a
closure upvalue and no state API. What is absent is a **production caller**: nothing in
`crates/` depends on `mc-script`, there is no tick integration. Mitigated but not fixed by
`FaultKind::HostMemoryPressure` (A7), which names the condition instead of hiding it.
**Verify early at PRO-917**, and treat per-mod VMs as a correctness requirement there rather
than an optimisation. A plain test should demonstrate the closure-retention path and its
misattribution — **not** a seventh hostile case, since FR-7.1-S1 pins the harness at exactly
six by name.

**Amended per discussion — the retention vectors are two, not one, and permitting `coroutine`
adds the second.** A closure upvalue is the one A7 names. A **suspended coroutine** is the other:
it holds its own stack, its locals and everything they reference, for as long as a reference to it
survives — and a reference to it is itself just an upvalue. What the probe measured about
`coroutine` is that the interrupt fires inside `resume` and `wrap` and that the latch is not void
there, which settles **execution**. *"The latch contains it"* and *"it cannot retain across
invocations"* are different claims and **only the first was measured.** Recorded so that permitting
`coroutine` (DF10) is not later read as having settled both. It changes nothing about the decision —
retention was already unbounded per attachment by the upvalue route, and this is a second door into
the same room — but it widens what the verification at the per-mod-VM revisit has to cover.

**Amended per discussion — a per-attachment retention ledger was proposed and is REFUSED, not
deferred, and the refusal is priced rather than asserted.** A ledger attributing retained bytes to
the attachment that allocated them would name the offender directly. What it would actually be is a
*second unverified mechanism layered on a mitigation that already closes the gap by construction*:
the derived pressure trigger (DF1) makes the misattribution impossible rather than detectable,
which is a stronger property than a ledger can offer, and a ledger's own attribution is a heuristic
that would need its own evidence to be believed. The price of refusing is stated plainly: when the
host is under pressure, **nothing names which attachment caused it**. The operator sees that
scripting is degraded and not who degraded it, and finding out means removing mods until it stops.
That is accepted for a server whose operator chose every mod installed; it would not be accepted
for a host running mods it did not choose, and that is the condition under which to reopen it.
The framing carried forward is the accidental one — *careless retention misfiles blame* — not the
hostile one; a mod weaponising containment is not the population this project has.

**R2 — The deny list is a denylist (A1).** A Luau or `mlua` upgrade may reintroduce a global.
Mitigated by FR-1.2-S1/S4 enumerating all fourteen as exact verdicts. That four of the eight
survivors were *not* the four the spec expected is evidence this risk is live rather than
theoretical.

**R3 — The whole containment story rests on the interrupt.** Budget (A3) and memory (A4) both
route through one `set_interrupt` callback. If a Luau path exists that runs script without
firing the interrupt, both limits are void on that path simultaneously. **Verify early**:
this is the highest-value mutation in the FR-2/FR-3 table (`/sdd-validate`'s obligation) —
remove the latch and confirm `spin_pcall` and `bomb_pcall` both pass when they should fail,
which is the measured behaviour A3 and A4 record. **Its companion mutation, per discussion, is
A6's marked inference:** break the latch's re-arm and confirm `spin_pcall` reddens rather than
reporting a caught `ScriptError`, since "the trampoline cannot return once latched" is reasoned
from construction and never observed.

**Amended per discussion — R3 has a measured residual and one unswept gap.** Two paths that run
script *without* firing the interrupt are known and are not hypothetical: `string.rep` and
`table.concat` run to completion between ticks (A3). They are bounded, because they must allocate
and A4's cap therefore stops them — so the worst pause is the time it takes to fill the memory cap,
not an unbounded hang. `string.find` with catastrophic backtracking and `table.sort` with a Lua
comparator were both checked and are interruptible. **The gap is a slow *non-allocating* builtin:
no exhaustive sweep was made for one, and the memory cap would not bound it.** Recorded as an
unswept gap rather than filled with a guess, and it is the first thing to re-measure on an
`mlua`/Luau upgrade alongside R4's five behaviours.

**R4 — `mlua` is pre-1.0 and this design depends on five of its behaviours** that are not part
of any stability promise: `error_traceback` calling `luaL_tolstring` (A6), `sandbox(true)`'s
child-environment semantics (A1/A2), `used_memory()` accounting (A4), interrupt-error
propagation (A3), and the `[string "name"]:N:` error prefix (A10's `line`). Mitigated by the
port: all five are observed only inside `src/luau/`, enforced by the Boundaries guard. An
upgrade is a re-measurement, and the probe programme behind this document is what to re-run.

**R5 — `gc_collect()` on the allocation path is a latency spike.** Two full collections on
every allocation fault. Irrelevant here (no tick integration) and cheap at this heap size, but
a real cost once `mc-sim` calls dispatch. Record it; do not optimise it now.

---

## What went to discussion *(amended per discussion — both items are now closed)*

Two items where this document took a position the spec did not hold. Both were carried and both
are resolved; their outcomes are below and in the sections named.

1. **A8** — D7's mechanism is impossible; the architecture substitutes a crate-root attribute
   with a narrower target scope. **Closed, ruling for the attribute:** the manifest table is
   unbuildable, cargo hard-errors on `cannot override 'workspace.lints' in 'lints'`, and the
   crate-root `#![deny(...)]` is measured to compose with the inherited workspace lints. The scope
   difference stands as A8 states it — `tests/*` are covered by the gate's `-D warnings` and not by
   the attribute. `spec.md` still describes the manifest table and takes a one-line correction at
   `/sdd-complete`.
2. **A7's residual (R1)** — whether shipping a per-attachment retention hole is acceptable
   given invariant 3 is the spec's entire subject. **Closed, accepted with a mitigation that
   changed shape under challenge:** the pressure trigger is now derived rather than tuned (DF1),
   its cost to quarantine is named and given a scenario, a retention ledger is refused and priced
   (R1), and the terminal state is recorded with its escalation (Data). The honest grounds are
   unchanged: the *mechanism* is reachable now but no production caller exists, and the damage is
   misattribution rather than unboundedness — not that the hole is unreachable.

**Settled, listed only so it is not re-litigated:** cascade faults do not count toward
quarantine (A9), for either kind. It follows from FR-4.2-S1's "on three consecutive invocations"
read literally, corroborated by FR-4.2-S5's enumeration. An earlier draft escalated this as two FRs
silently constraining each other; that framing was wrong.

---

## Discussion Findings

The spec and this document were challenged together, from four stakeholder lenses, against a
second measurement probe run specifically to settle the questions the challenge raised. Every
finding below is **agreed** — there were no deadlocks — and every one is binding. Each names the
section it changes, and each of those sections has been amended rather than annotated: a
superseded decision does not stand next to the finding that overrode it.

Ten scenarios' worth of this landed in `spec.md` rather than here, because a mechanism nobody can
show working is not a design decision, it is a hope. That principle is the single thread through
the findings: **every ask that adds surface arrives with a test, or it does not arrive.**

### DF1 — The host-memory-pressure trigger is derived, not tuned — *amends A7, A4, A5*

`HostMemoryPressure` is classified exactly when `entry_baseline + memory_cap > memory_backstop`
holds at entry — *this invocation could fail for a reason that is not its own*. That replaces "a
documented fraction of the absolute backstop" with one comparison and no constant to choose,
defend or re-choose. It is justified on **correctness**, never on security: it removes a parameter
rather than adding a mechanism, and the failure it prevents is an innocent mod being disabled and
the blame filed against the wrong author.

The cost is stated in A7 and is not small: while pressure holds, faults do not count toward the
consecutive-fault total, so an attachment whose own retention raised the baseline is immune to
quarantine, and under sustained pressure quarantine is inactive for everyone. **Accepted, with the
reasoning recorded** — the excused failure is loud (a slow server an operator notices and acts on)
and the alternative is silent and misdirected (an innocent mod permanently disabled, the operator
removing the wrong one). Quarantine functioning would not help regardless: retention lives in
closure upvalues that survive it. Two scenarios exist for this and neither is optional.

### DF2 — The memory backstop is a configured limit; pressure is read after a collection — *amends A4, A5, Interfaces*

`memory_backstop` joins `HostLimits`. It was prose, and it is the one number that decides when
scripting wedges for everybody — an operator could neither read it nor set it. And the pressure
comparison is made against a **collected** reading: 1,434,679 B of garbage was measured surviving
until an explicit `gc_collect()`, so a raw reading would report pressure caused by memory nothing
is holding.

### DF3 — `ScriptTable` and `ScriptFunction` carry an opaque isolation-unit tag — *amends A7, Interfaces*

Justified by **hot reload**, which builds candidate registries in a scratch VM and whose entire job
is substituting a scratch-VM `ScriptFunction` for a live one; a handle that cannot say which state
it came from makes that unverifiable in the one path whose partial-failure mode the crate calls a
Blocker. **Explicitly not justified by the published-extension-API exemption**, which is scoped to
the published scripting surface and does not reach engine-internal Rust handles consumed by sibling
crates. The distinction is the finding: record the wrong reason and the first person to read the
standard correctly finds it void and deletes the field.

### DF4 — `ScriptOrigin` carries the defining chunk from first design — *amends A10, Interfaces*

Not a change: `ScriptOrigin` does not exist yet, so this is what the type is designed to be. As
drafted, an invocation fault's origin was the dispatch round, meaning the most common fault in the
system named no file. The chunk name reaches it by being stamped onto `ScriptFunction` at creation
— the same diff as DF3, all host-known metadata set at one moment. A line number for runtime aborts
is measured available for budget aborts and measured absent for allocation aborts; that measurement
is recorded in A10 and nothing beyond the chunk name is committed.

### DF5 — Cascade faults split into `CascadeRefused` and `CascadeDeferred` — *amends A9, A10, Interfaces*

One kind conflated two events with different consequences: deferral progresses normally, refusal
**drops work permanently**. The draft priced deferral's noise and never priced refusal at all.
`CascadeRefused` names both requester and refused target, `CascadeDeferred` names the requester
only, and the asymmetry is deliberate. The refused target is a **typed field**,
`refused_target: Option<Attachment>`, never substring text inside `cause` — and `FaultKind` stays a
plain data-free enum so the hostile-case evidence check keeps comparing by equality instead of
degrading to a discriminant match.

### DF6 — `attach` replaces and lifts quarantine; `release` lifts it; `invocation_count` belongs to the attachment — *amends Interfaces, Data*

One rule covers replace and release: quarantine lifts, and `invocation_count` **resumes from its
frozen value** rather than resetting, because it is cumulative telemetry about the attachment and a
*different counter* from the consecutive-fault count that already resets on success. Replace
lifting quarantine is not a convenience — it is hot reload's path in miniature, and a replace that
left quarantine standing would silently fail at the one thing reloading a broken mod exists to do.
`detach` is deferred; nothing unloads mods yet. Three scenarios, because "replace semantics" was
one of the mechanisms nothing could show working.

### DF7 — One budget, renamed; sized for the largest unsliceable chunk; the split is a condition — *amends A3, Assumptions 1–3, Interfaces*

**One value, not two.** A second field would be a second number chosen against nothing measurable
today, and a seventh mechanism nothing can show working; the later cost of splitting is a refactor,
not a break, since `evaluate`'s signature does not change and `HostLimits` has public fields and a
`Default`.

**Renamed to `call_and_loop_budget`**, because the probe measured that the interrupt fires at seven
opcodes — calls and loop edges — and not per instruction. The old name guarantees somebody sizes it
against VM instructions and is wrong by the size of the loop body.

**Sized for the largest plausible *unsliceable* chunk.** The reason is structural and needs no
estimate: `evaluate()` cannot be sliced and `dispatch` can. A callback over budget has a mechanism
— the queue, across rounds; a chunk over budget has none. **No claim about relative magnitude
appears anywhere, in any form.** An earlier "the load workload is larger, by an order of magnitude"
is **retracted**: redone honestly the two land in the same order of magnitude, and in the worked
example the callback path was the larger one. An unmeasured ratio is an unmeasured number wearing a
different hat.

The second half of the argument is that the two constraints on one value **do not covary**: it must
be large enough for the biggest single unsliceable chunk and small enough that `round_bound ×` it
fits a tick. The first tracks the largest content pack anyone ships and the second tracks the tick
budget; nothing ties them together, so a value satisfying both today is broken by either moving.

**The split trigger is a condition, not a schedule:** *the single value is safe until something
calls `dispatch` from a tick; whichever spec does that first owns the split.* Not "forced at the
next-but-one spec" — that is a bet on an ordering that has already been changed once.

Recorded alongside the constants: the product `round_bound × call_and_loop_budget`, as a derived
quantity, with both values provisional and joint re-derivation of **three** quantities — chunk
budget, callback budget, round bound — named as an obligation at the first caller that dispatches
from a tick. Joint derivation is impossible today and that impossibility *is* the finding: there is
no tick to derive against. **Falsifiability caution:** a test asserting that product is "finite and
non-zero" **cannot fail**, since the product of two `NonZero`s is non-zero. Assert against a stated
ceiling or do not assert.

**One more consequence, and it is a pleasant one: this measurement turns a style rule the crate
already asserted into a costed one.** `crates/mc-script/CLAUDE.md` says *"Binding-call overhead
matters more than script-body speed. Prefer one call passing a batch over N calls passing one
item."* That was written about the cost of crossing the Rust/Luau boundary, and it turns out to
govern the **budget** as well: a host call is one tick, a Lua call is two, and a loop body of any
size is free. The advice was right and its reason was only half the reason. The reframing that
follows is the useful part — the sizing question is not *"how many instructions does this workload
cost?"*, which nobody can answer, but **"how many calls does it make?"**, which a content author
controls directly and can be told about. Batching stops being a performance suggestion and becomes
the difference between a callback that fits its budget and one that aborts.

### DF8 — FR-9 asserts over the set of limits the host reports — *amends Assumption 4*

Not four named limits. Two limits were added during this document's own lifetime, and a scenario
naming four would have silently stopped covering the ones most likely to be wrong. The assertion is
over the reported set, against documented values kept on one source with the constants.

### DF9 — A6's `Err` + `Clear` state is defined; "the trampoline cannot return" is marked as an inference — *amends A6, R3*

The trampoline scheme enumerated two outcomes and the guard admits a third. `Err` with the guard
still `Clear` means the error came from `mlua` rather than from this host's enforcement, and it is
classified from the error discriminant — allocation errors to `Allocation`, everything else to
`ScriptError` — never from a default arm and never from the error's text. Separately, the claim
that a latched abort can never be mistaken for a caught script error is **reasoned, not observed**;
it is marked as an inference and made R3's companion mutation.

### DF10 — `coroutine` is permitted and enumerated — *amends A1, FR-1.2*

Probe-conditional and the probe answered: the interrupt fires inside both `coroutine.resume` and
`coroutine.wrap`, the abort is catchable by three different constructions, and the latch defeats all
three — including respawning a fresh coroutine after every abort. Permitted, and *enumerated*: the
permitted set becomes an exact-set assertion against a list the host declares, so a Luau or `mlua`
release that silently **adds** a global is reported. That is an accident case, not an attack case,
and the enumeration makes the decision visible whichever way it had gone.

**Scoped precisely, because it is easy to over-read:** what was measured is that the interrupt
fires inside `resume` and `wrap` and that the latch is not void there. That settles **execution**
and nothing else. A *suspended* coroutine still retains its stack and everything it references, so
permitting `coroutine` adds a second retention vector to R1 — where it is recorded. "The latch
contains it" and "it cannot retain across invocations" are two claims and only one of them has
evidence.

### DF11 — `print` is a sandbox escape, and the install ordering is binding — *amends A1, A2, FR-1.2*

**Its disposition changed under measurement.** `print` was going to be a decision recorded either
way. It is not that: installed *after* `sandbox(true)`, a host `print` is defeated by `print = nil`
falling through `__index` to the original C `print` still sitting in the readonly globals, and the
script then writes to **raw fd 1, bypassing Rust logging entirely** — measured, with a positive
control. Content reaching a host capability the sandbox was supposed to have removed is an escape,
not an unrouted output channel, so `print` is enumerated and tested on the same footing as the
denied globals rather than noted as considered.

**The binding part is the ordering:** the host `print` is installed **before** `sandbox(true)`;
installing or removing it afterwards is a silent no-op that still returns `Ok`. One rule, shared
with the deny list, for one underlying reason.

**The companion finding is the same defect from the interop side, and it is what A2 was missing.**
With a shared environment, one mod's `print = function() end` silences every other mod's `print` —
measured at zero host calls from a second chunk. A2's counter-argument for per-chunk frozen
environments previously rested on principle; it now has a concrete attack the alternative admits.

**And the structural lesson is the durable one.** The deny list was derived on the escape axis and
`print` sat on that axis the entire time. It was missed because the derivation asked which globals
should be removed and never asked what `StdLib::ALL_SAFE` actually leaves standing — a list nobody
had enumerated, and the same omission that made four of the eight measured survivors a surprise.
**That is the argument for the exact-set enumeration test**, not tidiness: a judgement-derived
denylist keeps missing exactly what nobody thought to name, and enumerating what is really there is
the only defence that does not depend on thinking of it first.

### DF12 — Some builtins are not interruptible; the memory cap is what bounds them — *amends A3, R3*

`string.rep` and `table.concat` run to completion between two ticks regardless of budget. Bounded
rather than unbounded, because they must allocate — so the worst non-interruptible pause is "the
time it takes to fill the memory cap", which is a number an operator can be given. `string.find`
with catastrophic backtracking and `table.sort` were checked and are interruptible. The unswept gap
— a slow *non-allocating* builtin — is recorded as a gap rather than filled with a guess.

### DF13 — The queue payload is declined, with a tripwire on the action — *amends A9, Interfaces*

A payload on the follow-up queue entry does not close the hole it was proposed for: the retention
mechanism is *any* closure upvalue, so removing one motivating case leaves it intact. The **record
shape** is taken — a follow-up entry is a struct, not a bare tuple — so a payload field stays
additive. The obligation is written as a condition on an action, not as a note, because a note ages
into permission: *the first spec requiring cross-invocation continuation must evaluate a queue
payload before extending the callback return convention.*

### DF14 — The per-attachment retention ledger is refused, and priced — *amends R1*

Refused, not deferred: it is a second unverified mechanism layered on a mitigation that already
closes the gap by construction (DF1 makes the misattribution impossible rather than detectable).
The price is stated rather than waved past — under pressure, **nothing names which attachment
caused it**, and the operator finds out by removing mods until it stops. Accepted for a server
whose operator chose every mod; the condition for reopening is a host running mods it did not
choose.

### DF15 — `cause` is unbounded, script-controlled text — *amends A10*

Rendering it raw is correct; A6 exists so that rendering it any other way cannot run script on the
host's schedule. But raw and safe-to-splice are different properties, and every consumer that logs,
formats or displays it inherits whatever a mod put there at whatever length a mod chose. Marked at
the field.

### DF16 — The terminal state is recorded with its escalation — *amends Data*

Sustained pressure is a terminal state the host does not recover from, because retention lives
where no mechanism here can reach. Recovery is an operator restart. **Today that is free** — all
host state dies with the process and nothing depends on `mc-script`. **Once the simulation drives
dispatch it is a live server restart with players on it.** Recorded where the reason it is
currently free is written down, so the next spec meets it rather than inherits it.

### DF17 — The authoring rule replaces an on-abort hook — *documentation, no design change*

After the latch no script frame progresses, so a `pcall` handler cannot run and post-abort cleanup
in script is impossible. **That is A3 working, not a defect**, and any on-abort hook re-opens the
`pcall` hole the latch exists to close. What ships instead is an authoring rule with an in-tree
precedent in the crate's registry-swap invariant: *an abort is not recoverable in script; never
rely on post-`pcall` cleanup; build then swap, never mutate in place.*

### DF18 — A new ADR is required at `/sdd-complete`, and ADR-003 needs a factual correction — *amends Integration*

**Two separate obligations, and conflating them is the error to avoid.** VM granularity is its own
decision and gets its own ADR — the tree is at ADR-021, so **ADR-022** — recording one VM, the
loader-versus-content argument that rejects per-component VMs, R1 and its accepted residual, the
derived pressure trigger under the accident framing rather than the hostile one, and the retention
ledger as a priced refusal. It is **not** an amendment to ADR-003. Separately, ADR-003's
Consequences claim `set_memory_limit` caps allocation, which is measured false, and that is a
factual correction to an existing ADR. **Recorded as a requirement; neither is written now.**

No tracking apparatus goes with it: no conditional triggers, no dependency chain across four specs,
no blocking completion conditions. The decision is recorded and the obligation is named. Building
governance around it would cost more than it protects on a project this size.

### DF19 — What `docs/modding/sandbox.md` must answer, and the loop it closes — *amends Integration*

The consolidated modding documentation has to answer *"how do I do a job bigger than one budget?"*,
and today the only honest answer is the retention construction — a closure upvalue carrying a
cursor between invocations, which is R1's own mechanism. That loop closes **knowingly**, and it
closes directly beside DF17's rule telling the same author their cleanup will not run on an abort.
Both go in the page, neither is softened, and the pairing is noted here so that whoever writes it
does not discover the tension and quietly resolve it by omitting one.

### DF20 — An allocation abort carries no cause, so the host composes one — *amends A10, A6*

Measured: the error is literally `MemoryError("<nil>")` — no line, no message — and a traceback
taken afterwards is empty because the stack has already unwound. A6's raw formatter is correct for
a script-raised error and, applied to this one, renders **nothing**. So the fault as designed would
name its subject and its component and then say nothing whatever about why. The host therefore
**synthesises** the cause on that path from what it knows — the configured cap, the observed usage,
and the last-known source position the interrupt recorded — rather than passing a vendor string
through. This is the one place where host-authored cause text is right rather than a leak.

It needs a scenario asserting the rendered cause is non-empty, and the reason is not thoroughness:
**an empty `cause` and a `cause` that was never populated are indistinguishable to a reader**, which
is the failure family this project has twenty-two recorded instances of. It also completes the
typed-field argument. `line` and `refused_target` are typed because a string is a poor home for a
structured fact; `Allocation` is the kind whose cause string is **guaranteed empty at the source**,
so for it the structured fields are not the better diagnosis, they are the only one.
