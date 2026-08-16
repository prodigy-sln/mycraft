# Tasks: Luau host, sandbox and the hostile-mod harness

**Spec**: [spec.md](spec.md) · **Architecture**: [architecture.md](architecture.md) ·
**Requirements**: [requirements.md](requirements.md) ·
**Branch**: `feature/PRO-916-luau-host-sandbox` · **Issue**: PRO-916 · **Spec**: SPEC-014 ·
**Rigor**: `high` · **Created**: 2026-08-16

One task = one coherent scenario group in one area. `[P]` = independent of other `[P]`
tasks in the same phase. Scenario IDs appear **here and in commit messages only** —
never in code, test names or file names. The scenario↔test mapping is recorded in
`test-map.md` by the test author during `/sdd-implement`.

56 scenarios across 11 FRs, all assigned exactly once. See
[Scenario assignment](#scenario-assignment) for the proof.

`architecture.md`'s `## Discussion Findings` (DF1–DF20) is as binding as its body.
Where a task cites a DF, that finding is the authority and this file is the summary.

---

## The property this breakdown is built against

**Seven mechanisms in this design cannot currently be shown working.** That is not a
list of tasks; it is the thing the phasing exists to fix. Each has a home below, and
**every task that adds surface adds an eighth unless it lands with a test.**

| # | Mechanism | Where its test lives |
|---|---|---|
| 1 | `HostMemoryPressure` and its interaction with quarantine | **T20** — FR-3.1-S5, FR-4.2-S7 |
| 2 | `pending_bound`'s refusal path | **T23** — FR-5.1-S6 |
| 3 | The `Err`-with-`Clear`-guard branch of fault classification | **T12** — no scenario; an added test |
| 4 | The "the trampoline cannot return" inference | **T13** — witnessed by FR-2.1-S5 / FR-3.1-S4, falsified by a scheduled mutation |
| 5 | The permitted global set, historically defined by subtraction | **T07** — FR-1.2-S5 |
| 6 | `attach()`'s replace semantics | **T19** — FR-4.2-S9, FR-4.2-S10 |
| 7 | `refused_target` | **T23** — arrives with FR-5.1-S6, never before it |

This project has twenty-two recorded instances of something passing for the wrong
reason, and four more were caught during this spec's own review — including a
reviewer's proposed acceptance test that would have gone green while the exact harm it
was written to prevent occurred. **Assume the next one is in this breakdown.**

### Five things no scenario forces, placed here because nothing else places them

1. **A runaway top-level chunk** — T09. Nothing in the mapping produces it.
2. **FR-3's memory tests need a generous call-and-loop budget** — T14, binding. The two
   limits mask each other and the masking is measured.
3. **Three structural guards each need a positive control** (`testing.md` §2) — the FR-8
   closure walk (T02; its controls are FR-8.1-S2/S3 and already exist), the
   `mlua`-containment guard (T03; needs its own), and the FR-7 harness guard (T25;
   needs its own).
4. **The `mlua`-containment guard must scan `tests/` as well as `src/`** — T03. The
   harness is the code most likely to reach for `mlua` directly.
5. **The harness-agreement trap** — T25, and it is the worst of the five. A harness that
   defines its own budget, its own latch or its own deny list **agrees with the host by
   construction**: it would report all six hostile cases contained while the host's
   enforcement had been deleted. All five FR-7 scenarios run through the harness, so
   nothing else in the suite can see it.

---

## Phasing rationale

**Nine phases, and the count is deliberate.** 56 scenarios is more than this project's
usual breakdown carries, and the boundaries below are dependency boundaries rather than
convenience: each phase consumes the previous one's mechanism as a finished primitive.

- **Phase 1 — vocabulary and the two structural guards** (T01–T03). 3 scenarios. Pure
  Rust; **no `mlua`**. The guards land before the code they guard (invariant 5).
- **Phase 2 — the VM, the closed sandbox, chunk evaluation** (T04–T09). 11 scenarios.
  `mlua` arrives here, with the interrupt and the latch, because `evaluate` is a guarded
  entry.
- **Phase 3 — registration, dispatch, the trampoline, the budget** (T10–T13). 9
  scenarios.
- **Phase 4 — the memory cap and its cause** (T14–T17). 5 scenarios.
- **Phase 5 — quarantine, its lifting, and the pressure exclusion** (T18–T20). 11
  scenarios.
- **Phase 6 — raw reads** (T21). 4 scenarios.
- **Phase 7 — the bounded cascade** (T22–T23). 6 scenarios.
- **Phase 8 — the hostile-mod harness** (T24–T26). 5 scenarios.
- **Phase 9 — the shipped defaults** (T27). 2 scenarios.

**Why `mlua` is not in phase 1.** `cargo machete` (gate stage) fails a declared
dependency with no `use`, so `mlua` cannot land before the module that uses it. Phase 1
is therefore fully gate-green on its own, and the `mlua`-containment guard exists
*before* the first `mlua` reference rather than after it.

**Why FR-4.1-S3 sits in phase 2 and the rest of FR-4.1 in phase 3.** S3 is a
*compilation* fault — an `evaluate` fault, in the phase that builds `evaluate`. FR-4.1's
other four need the `pcall` trampoline, which is a callback mechanism.

**Why FR-3.1-S5 sits in phase 5 and the rest of FR-3.1 in phase 4.** S5 asserts an
attachment is **not quarantined**. Against a phase-4 host with no quarantine machinery
that assertion is true by construction and the scenario passes for the wrong reason. It
belongs beside FR-4.2-S7, which makes the same claim from the other side.

**Why raw reads (phase 6) precede the cascade (phase 7).** A9's callback return
convention has the host read the follow-up list from a script-supplied table with raw
reads, so a hostile `__index` on the returned table cannot run on the host's schedule.
The cascade consumes `read_field`'s discipline as a finished primitive.

**Why quarantine (phase 5) precedes the cascade (phase 7).** FR-5.1-S5 requires a
quarantined target found in the queue to be skipped.

27 tasks in total. Nine carry no scenario — T01, T03, T04, T09, T10, T12, T17, T25,
T26 — and are enabling, supporting, structural or documentation work, each with its
reason stated in the task.

---

## Test author / implementation split (rigor `high`)

- The test author owns **every file under `crates/mc-script/tests/` and every
  `*_test.rs` sibling** for the phase, writes them before any implementation, and keeps
  ownership for the whole phase. The implementation context never edits them; a disputed
  failure goes to arbitration with exactly one verdict: `test-correct`, `test-wrong` or
  `scenario-ambiguous`.
- Tests bind to the signatures in `architecture.md` § Interfaces **exactly as written**.
  A signature that turns out to be wrong is a dispute, not a unilateral edit on either
  side.
- Test placement follows `docs/technical/testing.md`: behavioural and integration tests
  in `tests/` (600-line budget); private plumbing through a
  `#[path = "x_test.rs"] mod tests;` sibling (also 600, and it sees private items). An
  inline `#[cfg(test)] mod tests` counts against the 500-line source limit and is not
  used. Sibling `_test.rs` files are outside the coverage denominator by the gate's
  `_test\.rs$` exclusion.
- **A test author who cannot run the full gate must still run**
  `cargo clippy --workspace --all-targets --all-features -- -D warnings`.
  A green suite is no evidence about a lint, and a phase opening with an adaptation
  commit (T04) has no compilable tree for the gate to run on until the implementation
  lands — anything only the gate can see accumulates silently across that whole window.
  Checking at a lower severity asks a different question, and without `-D warnings`
  cargo marks repeats `(1 duplicate)`, which means *this same diagnostic, repeated*, not
  *a pre-existing one lives elsewhere*.
- Tasks marked `Scenarios: none` are enabling, supporting or structural work. They still
  carry tests — they simply own no acceptance scenario.

### Definition of done, every phase

1. `cargo nextest run -p mc-script` green (and `--workspace` at a phase boundary)
2. `cargo clippy --workspace --all-targets --all-features -- -D warnings` clean
3. `pwsh scripts/sdd-gate.ps1` exits 0
4. No `mlua` identifier outside `crates/mc-script/src/luau/` — T03's guard, which must
   be green at every boundary from phase 1 on

`mc-script` has **no gate exemption and no GPU-style escape**: its library code counts
against the 80 % line threshold from the first line written. Push logic into testable
Rust and keep `src/` files under the 500-line ceiling with sibling `_test.rs`.

---

## Binding inputs (measured facts — rely on these, do not re-derive them)

Every one was measured against `mlua 0.12.0` + `luau0-src 0.20.7+luau728` on this
toolchain. Re-deriving them costs a probe crate and risks disagreeing with the design.

**Ordering — physical, not stylistic.**

- The host `print` is installed **before** `sandbox(true)`. Installing or removing it
  afterwards is a **silent no-op that still returns `Ok`**, and content can then recover
  the original and write to raw fd 1 outside all host control.
- Denied globals are removed **before** `sandbox(true)`, for the same reason: after the
  sandbox closes, writes land in a child table that reads through `__index` to an
  untouched parent.
- **Per-chunk environments are load-bearing.** With a shared environment one mod's
  `print = function() end` silences every other mod's `print` by plain assignment.

**The budget.**

- It counts **calls and loop edges**, not instructions — seven opcodes only. A loop body
  of any size is free; 1,000 straight-line statements cost 1. A Lua call costs 2, a host
  call costs 1.
- A bare 16³ pass costs **4,369**; with one Lua helper call per cell **12,561**, which
  **aborts at 10,000**; with one host call per cell **8,465**.
- Cost is reduced by **batching calls, never by shortening code.**

**Faults and limits.**

- `Lua::inspect_stack` works from inside the interrupt callback (~155 ns/tick), so a
  budget abort can name file and line. **Inside a coroutine only level 0 is visible.**
- An allocation abort's underlying error is `MemoryError("<nil>")` — **no line and no
  message** — so the host must synthesise the cause.
- `buffer` allocations are fully visible to the memory accounting.
- Some builtins are **not interruptible** (`string.rep`, `table.concat`) and are bounded
  by the memory cap, not the budget. `string.find` and `table.sort` **are** interruptible.
  A slow *non-allocating* builtin is an **unswept gap** (R3) — recorded, not guessed at.
- **`coroutine` is permitted.** The interrupt fires inside `resume` and `wrap`, and the
  latch defeats the catch. **Execution is settled; retention across invocations is not.**
  No task text may read as though both were.

**Staging.** Explicit paths only. Never `git add -A`, never `git commit -a`, never
`cargo fmt` (the gate runs `cargo fmt --all -- --check`; fix what it names).

---

## Phase 1 — Vocabulary and the two structural guards

3 scenarios. Pure Rust. **`mlua` is deliberately absent** — see the phasing rationale.

- [ ] **T01** Crate skeleton, the lint attribute, and the identity vocabulary —
      `crates/mc-script/src/{lib,limits,fault}.rs` + `_test.rs` siblings
      Scenarios: none (enabling)
      - Identity newtypes over `String`, **stored and compared, never parsed**:
        `SubjectName`, `ComponentName`, `Attachment`, `ChunkName`, `RoundIndex`,
        `IsolationUnit`.
      - `HostLimits` with all six fields — `call_and_loop_budget`, `memory_cap`,
        `memory_backstop`, `fault_threshold`, `round_bound`, `pending_bound` — plus
        `Default`. **The values are provisional here; T27 owns fixing and documenting
        them.** The name is `call_and_loop_budget`; "instruction" must not appear.
      - `FaultKind` — a **plain, comparable enum carrying no data**: `BudgetExhausted`,
        `Allocation`, `ScriptError`, `Compilation`, `CascadeRefused`, `CascadeDeferred`,
        `HostMemoryPressure`. Data on a variant would degrade FR-7.1-S4's evidence check
        from an equality comparison to a discriminant match (DF5).
      - `ScriptOrigin { chunk, round }` and `ScriptFault { origin, subject, component,
        kind, line, refused_target, cause }` with a **hand-written `Display`**, reading
        like `DefinitionFault` (`crates/mc-core/src/block/source.rs`) and `HudFault`
        (`crates/mc-core/src/hud/source.rs`).
      - Three deliberate divergences from those precedents, stated so a reviewer does not
        "fix" them back: `ScriptFault` is **flat** (no `Unreadable`/`Malformed` split —
        every script fault is about one entry into script); `line: Option<u32>` is
        **typed**; `refused_target: Option<Attachment>` is **typed**. A structured fact
        buried in a string formatted by a pre-1.0 dependency leaves substring matching as
        the only available assertion (A10, DF5).
      - `cause` is **unbounded, script-controlled text** (DF15). Mark it at the field:
        raw is correct, but raw and safe-to-splice are different properties, and every
        consumer that logs, formats or displays it inherits whatever a mod put there at
        whatever length a mod chose.
      - **The lint mechanism is a crate-root attribute, not D7's manifest table.**
        `crates/mc-script/src/lib.rs` gains
        `#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]` with a doc
        comment naming the invariant it enforces. `[lints] workspace = true` in
        `Cargo.toml` stays **exactly as-is**.
        **Do not attempt D7's `[lints.clippy]` table** — cargo hard-errors:
        `cannot override 'workspace.lints' in 'lints'`. Measured. The attribute composes
        with the inherited table rather than replacing it (`unwrap_used` becomes a hard
        error while the inherited `dbg_macro` still warns).
        **Scope difference, stated so nobody assumes parity:** the attribute covers the
        lib target and its `_test.rs` siblings, **not** `crates/mc-script/tests/*`, since
        each integration test is its own crate root. The gate's `-D warnings` still
        covers those.
      - `#[derive(Debug)]` on every public type — `missing_debug_implementations` is a
        workspace lint.

- [ ] **T02** `[P]` FR-8: `mc-client`'s resolved closure excludes `mc-script` —
      `crates/mc-script/tests/client_closure.rs`
      Scenarios: FR-8.1-S1, FR-8.1-S2, FR-8.1-S3
      - A **third** `cargo metadata` walker, deliberately.
        `crates/mc-testkit/tests/workspace_layering.rs` could be extended and is not, on
        that file's own recorded reasoning: *"an integration test is its own crate, the
        two invariants are independent, and a `tests/` module carried by both files would
        be the same amount of code in a less obvious place."*
      - Invoke `cargo metadata --format-version 1 --locked` through the `CARGO`
        environment variable cargo sets for test binaries (**never** a hardcoded
        `cargo`), cwd `CARGO_MANIFEST_DIR`; BFS from the `mc-client` node over
        `resolve.nodes[].deps`, following **every** dependency kind including dev.
      - Report an **enumerated verdict**, not an absence. `assert!(found.is_empty())`
        cannot tell an empty answer from a scan that can no longer look; an exact verdict
        rejects every other verdict *including* the ones meaning "I could not look", so a
        vanished package reddens for free.
      - S2 is the vacuity guard: `mc-script` absent from the resolved metadata altogether
        must **fail** rather than report `mc-client` clean. S3 asserts a dependency
        `mc-client` genuinely has is present in the closure — the walk inserts its own
        root unconditionally, so "the closure contains the crate" proves nothing.
      - Parsing `Cargo.toml` (direct deps only) or `Cargo.lock` (every workspace member,
        so the assertion is vacuously false) are both wrong answers.
      - **This test is expected to pass on its first run.** Its RED state is a manifest
        that violates FR-8, which is not a state we create on purpose. Landing it early is
        the point: it guards the invariant for the rest of the spec rather than certifying
        it at the end. FR-8.1-S2 and S3 are the tests that can genuinely go red first.

- [ ] **T03** `[P]` The `mlua`-containment guard — `crates/mc-script/tests/mlua_containment.rs`
      (+ its fixture)
      Scenarios: none (structural guard; `architecture.md` § Boundaries)
      - **No `mlua` type appears in `mc-script`'s public API.** R4's entire mitigation
        rests on this guard, and an unenforced litmus is a claim.
      - **Scan two roots: `crates/mc-script/src` AND `crates/mc-script/tests`.** The
        harness is the code most likely to reach for `mlua` directly, and a guard that
        watched only `src/` would miss exactly that.
      - **Enumerated verdict**, in the shape `no_hardcoded_block_names.rs` and
        `license_declaration_consumers.rs` already use:
        `EveryMluaReferenceIsUnderLuauDir` / `LeakedOutside(paths)` / `ScanFoundNoFiles`,
        with a **per-root non-zero file count**.
      - **Its own positive control, in a separate test function.** A test asserting only
        an absence goes green forever the day the thing it guarded against is quietly
        removed. The same scan function pointed at a fixture that *does* leak must report
        `LeakedOutside` naming it, and a fixture file under a `luau/` path segment must be
        passed over.
      - **Compare the exemption by path segment, never by bare file name.** The
        `seam_boundaries.rs` precedent records the trap: a name-only exemption silently
        excuses a `tests/support/hostile/vm.rs` that is precisely what a leak would be
        called.
      - Landing this before `mlua` exists is invariant 5, not sequencing convenience. The
        verdict is meaningful from the first run because the control is what carries it,
        not the empty scan.

---

## Phase 2 — The VM, the closed sandbox, and chunk evaluation

11 scenarios. `mlua` arrives. **T04 is an adaptation commit** — the tree does not compile
until T05 lands, so the gate cannot run in that window and the test author runs clippy
directly (see the split above).

- [ ] **T04** `mlua` opt-in, the `src/luau/` seam, and the layering control update —
      `crates/mc-script/Cargo.toml`, `crates/mc-script/src/luau/mod.rs`,
      `crates/mc-testkit/tests/workspace_layering.rs:43`
      Scenarios: none (enabling)
      - `mlua = { workspace = true }`. **Never a version literal in a member crate** —
        that is a review-stop (root `CLAUDE.md`). It is already pinned at `0.12.0` with
        `features = ["luau", "vendored"]`, so no version decision is taken here.
      - `crates/mc-testkit/tests/workspace_layering.rs:43`: `("mc-script", None)` →
        `("mc-script", Some("mlua"))`. **That pair is that test's positive control and it
        goes red when `mlua` lands — the guard working, not a break.** Update it in this
        diff as part of the change, not as an unexplained repair. Do not weaken
        `ClosureIsTheCrateAlone` for any other crate.
        *(`spec.md` § Existing Code says "line 40"; the pair is at line 43. Verified.)*
      - `src/luau/mod.rs` states the port contract in module docs: `Lua`, `Function`,
        `Table`, `Value` and `Error` stay behind it, and the crate's public surface is
        `ScriptHost`, `ScriptValue`, `ScriptTable`, `ScriptFunction`, `ScriptFault`,
        `DispatchReport`. The port is shaped around what the host needs — *invoke this
        attachment under a budget; tell me how it ended* — not around `Lua`'s surface.
      - `mlua`'s `send` feature stays **off**. Everything here is `!Send`; the host is not
        `Sync` and does not pretend to be.

- [ ] **T05** VM construction, the deny list, and the host `print` —
      `crates/mc-script/src/luau/vm.rs`, `src/host.rs`
      Scenarios: FR-1.2-S1, FR-1.2-S4, FR-1.2-S6
      Depends on: T04
      - `ScriptHost::new()`, `with_limits(HostLimits)`, `limits()`. `HostError` covers
        host construction and is **not** a script fault.
      - **BINDING ORDERING.** Remove the denied globals **and** install the host
        `print` **BEFORE** `sandbox(true)`. Afterwards, `luaL_sandboxthread` gives the
        thread a *child* environment reading through `__index`: assigning `nil` to a
        global returns `Ok` and **removes nothing**, and a host `print` installed late is
        bypassed by a fall-through to Luau's own `print`, which `fwrite`s to C `stdout` —
        a different buffer from Rust's, flushed at process exit, outside all host control.
        Measured, with a positive control. That is content reaching a capability the
        sandbox was supposed to have removed: **an escape, not a logging inconvenience.**
      - Thirteen denied: `io`, `os`, `package`, `require`, `loadstring`, `load`, `dofile`,
        `loadfile`, `debug`, `getfenv`, `setfenv`, `collectgarbage`, `newproxy`.
      - **Do not assume `sandbox(true)` removes them.** Measured, it removes five —
        `io`, `package`, `load`, `dofile`, `loadfile` — and **eight survive**: `os`,
        `require`, `loadstring`, `debug`, `getfenv`, `setfenv`, `collectgarbage`,
        `newproxy`. `crates/mc-script/CLAUDE.md` invariant 1 reads as though the sandbox
        handles `require`/`os`/`debug`. It does not. Four of the eight were not the four
        `spec.md` expected.
      - FR-1.2-S6 has two halves and the second is the load-bearing one: a chunk that
        **first attempts to replace `print` and then calls it** must still have that call
        recorded at the host, rather than reaching a `print` the host does not observe.
      - `require` is **absent, not confined** (D1). There is no mod-directory concept yet;
        absence over-satisfies the invariant and confinement lands with PRO-917.

- [ ] **T06** The per-chunk frozen environment — `crates/mc-script/src/luau/env.rs`
      Scenarios: FR-1.1-S1, FR-1.1-S2, FR-1.1-S3, FR-1.1-S4, FR-1.2-S3
      Depends on: T05
      - **`sandbox(true)` alone does not satisfy FR-1.1-S1.** Measured: with the sandbox
        on, `newname = 1` from a content chunk is **allowed** — the thread's child globals
        table is writable, which is the whole point of Luau's sandbox. The spec requires
        rejection.
      - Mechanism: `Chunk::set_environment(env)`, where `env` is a fresh table whose
        metatable's `__index` is the sandboxed globals, whose `_G` is itself, and on which
        `Table::set_readonly(true)` has been called.
      - **Set the metatable readonly too.** `set_readonly` freezes the table, not the
        metatable behind it — without this, `getmetatable(_G).__index = {}` succeeds
        inside a chunk. It grants no capability, but "frozen" should mean frozen and the
        fix is one line.
      - **A readonly table, not `__newindex`.** FR-1.1-S4 is what decides this: a frozen
        environment built on `__newindex` is defeated by `rawset` and one built on a
        readonly table is not. Settled by measurement.
      - **Per chunk, not shared, and this is containment rather than tidiness** (DF11).
        With a shared environment one mod's `print = function() end` silences every other
        mod's `print` — measured at zero host calls from a second chunk. `rawset`,
        `_G.print` and `setmetatable` are correctly refused by the readonly table; **plain
        assignment and `getfenv().print` are not**, because those write to the chunk's own
        environment, which is what an environment is for.
      - Each of FR-1.1-S3, S4 and FR-1.2-S3 has a **second chunk** clause — the assignment
        is rejected *and* a later chunk observes the original behaviour. Both halves are
        the test; the rejection alone would pass against a host that froze nothing and
        merely errored.
      - The rejection message names the chunk: `[string "content-chunk"]:1: attempt to
        modify a readonly table`.

- [ ] **T07** The permitted set, enumerated exactly — `crates/mc-script/src/luau/vm.rs`
      Scenarios: FR-1.2-S2, FR-1.2-S5
      Depends on: T06
      - **Mechanism 5 of the seven, and this is its test.** The deny list was derived by
        asking *"which globals should be removed?"* and never *"what does
        `StdLib::ALL_SAFE` actually leave standing?"* — a list nobody had enumerated.
        That is how `print` sat on the escape axis and was missed, and how four of the
        eight measured survivors turned out not to be the four the spec expected.
      - S2: eleven names available — `math`, `string`, `table`, `pairs`, `tostring`,
        `pcall`, `select`, `type`, `coroutine`, `buffer`, `print`.
      - S5: the set of names **reachable from a content chunk** is asserted **equal** to
        a set the host declares as a constant, **naming any name reachable but
        undeclared**. Set inequality alone is not enough — the failure message is the
        product.
      - **Enumerate through the chunk's own environment**, not through `Lua::globals()`
        from Rust. Under T06 those differ, and the question is what *content* can reach.
      - FR-1.2-S1/S4 catch a denied global being **reintroduced**; only the exact set
        catches one being **added** by a future Luau or `mlua` release, which is the
        accident case rather than the attack case. A judgement-derived denylist keeps
        missing whatever nobody thought to name.
      - `coroutine` is permitted and enumerated (DF10): the interrupt fires inside
        `resume` and `wrap`, the abort is catchable by three constructions, and the latch
        defeats all three including respawning a fresh coroutine after every abort.
        **That settles execution and nothing else** — see T26.

- [ ] **T08** The interrupt, the latch, and `evaluate` as a guarded entry —
      `crates/mc-script/src/luau/guard.rs`, `src/host.rs`
      Scenarios: FR-4.1-S3
      Depends on: T06
      - Guard state `Clear | Budget | Memory`. On exhaustion the interrupt transitions out
        of `Clear` and returns `Err`; **while not `Clear`, every subsequent interrupt
        returns `Err` immediately**, so no script frame can make progress after the trip.
        The host **clears the latch at the start of each guarded entry**, which is what
        makes budgets per-invocation rather than a one-way ratchet.
      - **Why the latch is not optional.** Measured: with a non-latching interrupt raising
        exactly one error, `pcall(function() while true do end end)` **caught the abort and
        the chunk returned normally**. Without it, `while true do pcall(f) end` re-enters
        the budget indefinitely and invariant 2 is decorative.
      - **`evaluate` is a guarded entry, on the same terms as callback invocation.**
        `crates/mc-script/CLAUDE.md` invariant 2 is categorical: *"There is no unbudgeted
        path from engine into script"*, and `evaluate` is a path from engine into script.
      - `evaluate(&mut self, name, source) -> Result<ScriptValue, ScriptFault>`.
        Chunk-level faults carry `subject: None`, `component: None` and the chunk name in
        `origin` — which is why those two fields are `Option`.
      - FR-4.1-S3: a compilation fault naming the chunk **and line 3**, and the host
        remains able to evaluate a subsequent valid chunk. The adapter parses Luau's
        measured `[string "name"]:N:` prefix **inside `src/luau/`**, where vendor error
        translation belongs (`code-quality.md` §4). **Assert `fault.line == Some(3)` —
        never `cause.contains(":3:")`.**
      - **Trap:** `mlua`'s interrupt dispatch carries a recursion guard that silently
        *continues* on a re-entrant interrupt. Latch logic must never call back into Lua.

- [ ] **T09** A runaway top-level chunk is aborted — `crates/mc-script/tests/`
      Scenarios: **none** — no scenario reaches this, and A3 requires the test
      Depends on: T08
      - Every chunk in FR-1.x terminates and FR-7.1-S5's failure is a compile error, so
        the mapping produces nothing that exercises a chunk whose **top level** runs away.
        Without this, a hostile script evaluated by FR-7's harness would hang the host —
        and hang the test binary.
      - **Two cases, not one.** A bare `while true do end` at chunk top level; and
        `while true do pcall(function() while true do end end) end` at chunk top level.
        The second is the latch at the `evaluate` entry, which **no scenario reaches at
        all** — FR-2.1-S5 covers that construction only for callbacks. One entry point
        onto a tested path is not evidence about the other.
      - Assert the fault carries the chunk name, `subject: None`, `component: None`, and
        `FaultKind::BudgetExhausted`; and that the host evaluates a subsequent valid chunk
        afterwards.

---

## Phase 3 — Registration, dispatch, the trampoline, and the call-and-loop budget

9 scenarios.

- [ ] **T10** The registry, `attach`, `dispatch` and `DispatchReport` —
      `crates/mc-script/src/{host,dispatch}.rs`
      Scenarios: none (enabling)
      - `attach(Attachment, ScriptFunction)`, `dispatch(&[Attachment]) -> DispatchReport`,
        `invocation_count(&Attachment) -> u64`. Replace semantics and quarantine lifting
        are **T19's**; `is_quarantined` and `release` land in phase 5.
      - `attach` takes a **`ScriptFunction`, not a `ScriptValue`**, so "you passed a
        number" is unrepresentable rather than an untested runtime branch. Tests obtain
        one from `evaluate` returning `ScriptValue::Function`.
      - `ScriptFunction { handle, unit: IsolationUnit, origin_chunk: ChunkName }` and
        `ScriptTable { handle, unit }`. Both fields are all host-known metadata, set at
        the one moment a handle is created, **opaque and never parsed**.
      - **Record the right reason for `IsolationUnit` in the doc comment (DF3), or the
        first person to read the standard correctly will delete the field.** It is
        justified by **hot reload**, which builds candidate registries in a scratch VM and
        whose entire job is substituting a scratch-VM `ScriptFunction` for a live one; a
        handle that cannot say which state it came from makes that unverifiable in the one
        path whose partial-failure mode the crate calls a Blocker. It is **explicitly not**
        justified by `code-quality.md`'s published-extension-API exemption, which is
        scoped to the published scripting surface and does not reach engine-internal Rust
        handles consumed by sibling crates. Exactly one unit value exists today, and that
        is the point.
      - `DispatchReport { order, invocations, faults, quarantined, pending }`.
      - Maps are `BTreeMap` keyed by `Attachment` — ordering is deterministic, the counts
        are small, and nothing in this spec calls dispatch from a tick.

- [ ] **T11** The `pcall` trampoline and fault classification —
      `crates/mc-script/src/luau/trampoline.rs`
      Scenarios: FR-4.1-S1, FR-4.1-S2, FR-4.1-S4, FR-4.1-S5
      Depends on: T10
      - **`Function::call` runs `__tostring`.** Measured: mlua installs `error_traceback`
        as the message handler for every protected call, and it calls `luaL_tolstring`,
        which honours `__tostring` — one invocation, counter reads `1`. FR-4.1-S2 requires
        `0`.
      - Mechanism: the host holds one Lua function, `function(f, ...) return pcall(f, ...) end`,
        **created before the sandbox is closed**, and invokes every callback through it. A
        script-raised error then arrives as an ordinary **return value**, never as a
        propagating Lua error, so no traceback handler touches it. The host renders it with
        a raw formatter that matches on `ScriptValue` and **never invokes a metamethod**.
      - Two structural outcomes, distinguished by shape rather than by inspecting error
        text: the trampoline **returns** `(false, value)` → the script raised → `ScriptError`,
        text rendered raw; the trampoline **cannot return** (`Function::call` yields `Err`)
        → a latched abort unwound past it → `BudgetExhausted` or `Allocation`, read from
        the guard.
      - FR-4.1-S4 is S2's **positive control**: a content chunk that itself calls
        `tostring(e)` on the same table leaves the counter at `1`. Without it, S2 could be
        green because the probe never fired. FR-4.1-S2's original construction — a
        `__tostring` that loops forever — was itself wrong-reason-passable, since a host
        that *did* invoke it would have the metamethod aborted by the budget and still
        report a fault. **The counter probe is the construction; do not substitute a
        looping one.**
      - FR-4.1-S5: the fault's origin names the **defining chunk** (`furnace.luau`), and a
        callback defined by a second chunk faulting in the same round names that second
        chunk. The name reaches the fault from `ScriptFunction::origin_chunk`, **not** from
        the dispatch round. As drafted, an invocation fault's origin was the round — which
        would mean the most common fault in the whole system names no file, leaving a mod
        author with an error and no way to locate it (DF4).

- [ ] **T12** The `Err`-with-`Clear` third outcome — `crates/mc-script/src/luau/`
      Scenarios: **none** — DF9; no scenario reaches it
      Depends on: T11
      - **Mechanism 3 of the seven.** T11's scheme enumerates two outcomes and the guard
        admits a third: `Function::call` yields `Err` while the guard is still `Clear` —
        no limit tripped, so the error came from **`mlua` itself** rather than from this
        host's enforcement.
      - Classified from the **`mlua` error discriminant**: an allocation error →
        `FaultKind::Allocation`, everything else → `FaultKind::ScriptError`. **Not from a
        default arm**, and **never from the error's text** — a pre-1.0 dependency's message
        formatting is exactly what the typed `line` field exists to avoid depending on.
      - **Try the end-to-end construction first.** Suggested: deeply nested Lua calls under
        a *generous* budget until Luau raises a C-stack overflow that escapes the
        trampoline's own `pcall`. Verify the guard is still `Clear` at classification.
      - **If no in-process construction reaches it**, cover the classification function
        directly with real `mlua::Error` values **and record in `test-map.md` that the
        wiring is unwitnessed**, naming what would go red if the adapter stopped calling
        it. `testing.md` §2: *policy is not wiring* — a test that calls the same pure
        function the adapter calls is agreement between two copies of one decision, and
        the adapter can stop calling it entirely with both staying green. Do not let a
        unit test stand silently as evidence of the call site.
      - Either way, deleting the `Err` + `Clear` arm is a **scheduled mutation** at
        `/sdd-validate`, and its outcome is recorded whether or not it bites.

- [ ] **T13** The call-and-loop budget, per attachment —
      `crates/mc-script/src/luau/guard.rs`, `src/limits.rs`
      Scenarios: FR-2.1-S1, FR-2.1-S2, FR-2.1-S3, FR-2.1-S4, FR-2.1-S5
      Depends on: T11
      - **It counts calls and loop edges, not instructions** — see Binding inputs. The
        field is `call_and_loop_budget`. A name like "instruction budget" guarantees
        somebody sizes it against VM instructions and is wrong by the size of the loop
        body.
      - S1: abort at 10,000 and report a budget-exhausted fault **naming the subject and
        the component**. S2: 1,000 iterations under 10,000 returns its result and reports
        **no fault**. S3: two attachments in one round — the first exhausts, the second
        gets its own full 10,000 and its result is returned. S4: an attachment aborted last
        time gets a **full** budget, not the remainder — the latch is cleared at each
        guarded entry.
      - **S5 is the headline latch test:** a non-terminating loop wrapped in a protected
        call still aborts and still reports budget exhaustion, rather than returning
        control to the callback. Measured with the latch installed: `spin_pcall` and
        `bomb_pcall` both end as `BudgetExhausted`, and a following ordinary invocation
        returns `1000` from its full fresh budget.
      - **Mechanism 4 of the seven, and it is an inference rather than a measurement.**
        *"A latched abort cannot be mistaken for a caught script error"* is reasoned from
        the latch's own construction and **never observed**. What was measured is that the
        latch makes every subsequent interrupt return `Err` and that the affected
        invocations end as `Err`. S5 and FR-3.1-S4 are its only witnesses. **Its falsifier
        is a mutation** — break the latch's re-arm and confirm `spin_pcall` reddens rather
        than reporting a caught `ScriptError` — scheduled at `/sdd-validate` as R3's
        companion. **The phase must not close claiming the inference is measured.**
      - `Lua::inspect_stack` is available from inside the interrupt callback and can name
        file and line for a budget abort (~155 ns/tick, about twice the bare interrupt
        overhead). **What ships from this phase is the chunk name**; the line on runtime
        aborts is recorded as available and costed, not committed (A10). Inside a coroutine
        only level 0 is visible.

---

## Phase 4 — The memory cap, its reclamation, and its cause

5 scenarios. FR-3.1-S5 is **not** here — see the phasing rationale.

- [ ] **T14** The enforced cap and the backstop —
      `crates/mc-script/src/luau/{guard,vm}.rs`, `src/limits.rs`
      Scenarios: FR-3.1-S1, FR-3.1-S2, FR-3.1-S4
      Depends on: T13
      - **`Lua::set_memory_limit` alone does not satisfy FR-3.1-S4.** Measured: with a
        1 MiB limit and no other mechanism, a `pcall`-wrapped allocation bomb looped **ten
        times and returned normally** — each caught `MemoryError` dropped the table, the
        collector reclaimed it, and the next round started again. The allocator-raised
        error is an ordinary catchable Lua error, defeated by exactly the construction
        FR-2.1-S5 covers for the budget. **ADR-003's Consequences say otherwise and are
        measured false** — see Carried to `/sdd-complete`.
      - Mechanism: the interrupt reads `Lua::used_memory()` each tick against the
        configured cap and **latches via the guard**; `set_memory_limit` is set **above**
        the enforced cap, as the configured `HostLimits::memory_backstop`, so a single
        allocation large enough to jump the gap between two interrupt ticks still fails.
      - The cap is a **delta above the entry baseline**. `used_memory()` is whole-VM under
        one VM, so the host snapshots at entry and caps the delta — which is what makes
        the fault attributable to the running attachment.
      - **Construction check, enforced at `ScriptHost` construction:** `memory_backstop`
        must exceed the measured VM baseline plus `memory_cap`, or the host is configured
        into permanent memory pressure from the first invocation. That is a `HostError`,
        not a script fault.
      - **BINDING TEST-CONSTRUCTION RULE (measured, Assumption 3).** These tests need a
        **generous** `call_and_loop_budget`. Filling 1 MiB costs more than 10,000 ticks, so
        under the scenarios' nominal 10,000 the bomb aborts as `BudgetExhausted` and every
        FR-3 memory scenario passes for the wrong reason. Configure a budget that does not
        trip first, and **state the chosen value in `test-map.md`**. The two limits mask
        each other, and this is not a hypothetical: it is what the probe measured.
      - **The backstop bounds peak allocation; it does not itself contain.** If one
        allocation jumps the inter-tick gap, `set_memory_limit` raises `MemoryError`, a
        script `pcall` catches it, and because the *failed* allocation never landed
        `used_memory()` is back **below** the enforced cap — so the next tick sees nothing
        and does not latch. The script retries in a loop. It is still contained, but by the
        **budget**, so the fault reports `BudgetExhausted`. This path is **not**
        unreachable: the moment the VM sits near the backstop, an *innocent* invocation's
        allocation jumps the gap and terminates as `BudgetExhausted`, which FR-4.2-S5
        counts toward quarantine. It is the second door to misattribution, and it is why
        T20's trigger is stated over the **entry** condition rather than over the terminal
        fault kind.
      - `buffer` allocations are fully visible to this accounting.

- [ ] **T15** Reclamation after an allocation fault — `crates/mc-script/src/luau/`
      Scenarios: FR-3.1-S3
      Depends on: T14
      - **Measured: without an explicit collection it does not work.** After the abort,
        `used_memory()` stayed at **1,434,679 B** and the following 512 KiB allocation
        failed with `MemoryError("not enough memory")`.
      - **Two `Lua::gc_collect()` calls, not one**: the first ends the incremental cycle in
        progress, the second sweeps. After both, usage returned to **exactly** the
        385,952 B baseline and the 512 KiB allocation returned.
      - The collection runs **only on the allocation-fault path**, so ordinary dispatch
        pays nothing. R5 records the latency spike as a real cost once `mc-sim` calls
        dispatch — record it, do not optimise it now.

- [ ] **T16** An allocation fault's cause is composed by the host —
      `crates/mc-script/src/luau/`
      Scenarios: FR-3.1-S6
      Depends on: T14
      - **Measured: the underlying error is literally `MemoryError("<nil>")` — no line and
        no message** — and a traceback taken afterwards is empty because the stack has
        already unwound. T11's raw formatter is correct for a script-raised error and,
        applied to this one, renders **nothing**: the fault would name its subject and its
        component and then say literally nothing about why.
      - The adapter in `src/luau/` therefore **synthesises** the cause from what the host
        knows — the configured cap, the observed usage, and the last-known `(source, line)`
        the interrupt records each tick. This is the one place where host-authored cause
        text is right rather than a leak of vendor formatting.
      - **Assert more than non-emptiness.** *"Non-empty"* is nearly true by construction of
        any format string, and an empty `cause` and a `cause` that was never populated read
        identically — the failure family this project has twenty-two recorded instances of.
        Keep the scenario's assertion **and** assert the configured limit appears in the
        rendered text, so a formatter emitting a constant string reddens when the cap
        changes (`testing.md` §1, *a stronger observable alongside a scenario's own*).
      - This is the strongest form of the typed-field argument: `line` and `refused_target`
        are typed because a string is a poor home for a structured fact; `Allocation` is
        the kind whose cause string is **guaranteed empty at the source**, so for it the
        structured fields are not the better diagnosis, they are the only one.

- [ ] **T17** `crates/mc-script/CLAUDE.md` invariants 1 and 3 — `crates/mc-script/CLAUDE.md`
      Scenarios: none (documentation; `architecture.md` § Integration requires it **in this
      spec's diff**, not at `/sdd-complete`)
      Depends on: T05, T14
      - Invariant 1 reads as though `sandbox(true)` removes `require`/`os`/`debug`. It does
        not — eight globals survive it and the host removes them explicitly (T05).
      - Invariant 3's *"allocation failure surfaces as a Lua error, never an abort"* is true
        but **incomplete**: a `pcall` then defeats it, which is what T14 measures and why
        the latch exists.
      - Correct both here rather than deferring. A future spec author reads the invariant
        list first, and `docs/modding/sandbox.md` does not fix a stale invariant.
      - Its own commit, `docs:` type, separate from code.

---

## Phase 5 — Quarantine, its lifting, and the pressure exclusion

11 scenarios — the largest phase, and the one where two of the seven mechanisms land.

- [ ] **T18** Consecutive-fault counting and quarantine —
      `crates/mc-script/src/quarantine.rs`, `src/dispatch.rs`
      Scenarios: FR-4.2-S1, FR-4.2-S2, FR-4.2-S3, FR-4.2-S4, FR-4.2-S5, FR-4.2-S6
      Depends on: T14
      - Threshold **3**, **consecutive**, count **reset by a success** (D3). A callback
        alternating fail/succeed is never quarantined and that is accepted: its cost is
        already bounded by the budget. The budget bounds cost; quarantine bounds
        repetition.
      - Quarantine is per **attachment**, the `(subject, component)` pair (D2) — never per
        component workspace-wide. S3 and S4 are the two halves of that: another component
        on the same subject keeps being invoked, and the same component on a different
        subject keeps being invoked.
      - **S5 is the enumeration that settles which kinds count**: budget exhaustion, then
        an allocation abort, then a script error, on three consecutive invocations, still
        quarantines. Three different kinds, one count.
      - S6: a quarantined attachment is left **uninvoked across three subsequent rounds**
        and its invocation count reads **unchanged from the count at quarantine**.
        Asserting quarantine *happened* once is weaker than asserting it *persists*.
      - `is_quarantined(&Attachment) -> bool` lands here.
      - **Neither cascade fault kind counts toward this total** (A9). The counting unit is
        the outcome of an **invocation**, and a cascade fault is not one: the invocation
        completed and returned, and the fault is a property of the round's admission
        control. Phase 7 must not amend this rule. *(Note the recorded correction: an
        earlier draft derived it from FR-5.1-S3 by arguing the cascade would self-quarantine
        by round 3. That derivation is wrong — D3 resets on success and the blamed requester
        succeeds dozens of times per round. The rule stands; only its reasoning changed.)*

- [ ] **T19** Release and replace lift quarantine — `crates/mc-script/src/host.rs`
      Scenarios: FR-4.2-S8, FR-4.2-S9, FR-4.2-S10
      Depends on: T18
      - **Mechanism 6 of the seven, and these three are its tests.**
      - `attach` over an existing attachment **replaces** the callback **and lifts
        quarantine**. That is not a convenience: **hot reload is exactly this operation**,
        and a replace that left quarantine standing would silently fail at the one thing
        reloading a broken mod exists to do (DF6).
      - `release(&Attachment) -> bool` lifts quarantine, leaves the callback in place, and
        returns whether the attachment was quarantined. **Not `detach`** — nothing unloads
        mods yet.
      - `invocation_count` **resumes from its frozen value** rather than resetting. It is
        cumulative telemetry about the **attachment** and a *different counter* from the
        consecutive-fault count, which already resets on a success (D3). Two counters
        answering two questions. S8 asserts **exactly one greater** than the frozen count.
      - **S9's real assertion is the returned result**, not the fact of invocation: the
        **newly attached** callback's result must come back. A host that kept the old
        callback and merely reported success would pass a weaker assertion.
      - S10 is S9 applied to a **quarantined** attachment: reported as no longer
        quarantined **and** the new callback invoked in the next round. Both halves.

- [ ] **T20** Faults under host memory pressure do not count toward quarantine —
      `crates/mc-script/src/luau/guard.rs`, `src/quarantine.rs`
      Scenarios: FR-3.1-S5, FR-4.2-S7
      Depends on: T18
      - **Mechanism 1 of the seven — both halves of it, and the reconciliation with
        FR-4.2-S5 that would otherwise ship unverifiable.** *"Does not count toward the
        consecutive-fault total"* sitting silently beside a rule that says it does is how
        an unverifiable mechanism ships.
      - **The trigger is derived, not tuned.** `HostMemoryPressure` is classified exactly
        when, at entry, `entry_baseline + memory_cap > memory_backstop` — *this invocation
        could fail for a reason that is not its own*. No constant to choose, defend or
        re-choose. It is justified on **correctness** (it removes a parameter) and never
        on security.
      - **The baseline read is the collected one** (A5, DF2). 1,434,679 B of garbage was
        measured surviving until an explicit `gc_collect()`, so a raw reading would report
        pressure caused by memory nothing is holding — condemning the host to permanent
        "pressure". A raw reading may pre-filter; **only a collected reading may
        conclude**.
      - FR-3.1-S5: an attachment whose callback allocates only 64 KiB, invoked three times
        under pressure — each fault is `HostMemoryPressure` carrying **no subject and no
        component**, and the attachment is **not quarantined**.
      - FR-4.2-S7: a callback running a **non-terminating loop** on three consecutive
        invocations under pressure — **not quarantined**. This is the scenario that
        reconciles with S5's "three kinds still quarantine", and it is where the accepted
        cost is visible.
      - **The cost is named, not hidden.** While pressure holds, an attachment whose own
        retention raised the baseline is immune to quarantine, and under sustained pressure
        quarantine is inactive for everyone. Accepted, because the excused failure is
        **loud** — a slow server an operator notices and acts on — where the alternative is
        **silent and misdirected**: an innocent mod permanently disabled with the blame
        filed against the wrong author, ending with the operator removing the wrong mod.
        Quarantine functioning would not help regardless: retention lives in closure
        upvalues that survive it.
      - **Fixture construction is the constraint no assertion can enforce here**
        (`testing.md` §2 — *a count cannot see shape*). The test must genuinely raise the
        collected baseline until the condition holds. **Assert the condition holds** in
        these two tests, and assert it **does not** hold in at least one of T18's ordinary
        quarantine tests — a fixture that established pressure permanently would make
        T18's scenarios pass vacuously, and nothing in T18 could see it.

---

## Phase 6 — The engine reads script values raw

4 scenarios, one task: they are one interface. The cascade consumes this discipline.

- [ ] **T21** `read_field` reads without invoking script —
      `crates/mc-script/src/host.rs`, `src/luau/`
      Scenarios: FR-6.1-S1, FR-6.1-S2, FR-6.1-S3, FR-6.1-S4
      Depends on: T13
      - `read_field(&self, table: &ScriptTable, field: &str) -> Option<ScriptValue>`, via
        raw reads. A metatable can then neither run on the host's schedule nor observe
        which fields the host reads.
      - S1: a field **absent** from a table whose metatable defines an `__index` function
        → reported absent, metamethod **not invoked**. S4 is its **positive control**: a
        content chunk indexing the same absent field on the same table reports the
        metamethod invoked **exactly once**. Without S4, S1 could be green because the
        probe never fired — one of the audit's five real holes.
      - S2 is the control against a host that reports everything absent: a field genuinely
        present returns its value.
      - S3: an `__index` that **would loop forever** — the host's read completes, **and**
        the calling attachment afterwards executes 9,000 iterations under its 10,000
        budget without being aborted. **The second half is the real observable**: it proves
        the host's read consumed no budget and left the attachment usable. Assert both.
      - This discipline is what T22 applies to the host's own read of the callback return
        convention.

---

## Phase 7 — The bounded cascade

6 scenarios.

- [ ] **T22** The pending queue, the round bound, and deferral —
      `crates/mc-script/src/dispatch.rs`
      Scenarios: FR-5.1-S1, FR-5.1-S2, FR-5.1-S3, FR-5.1-S4, FR-5.1-S5
      Depends on: T18, T21
      - **Dispatch is never re-entrant** (D4) — the load-bearing decision. Synchronous
        re-entry converts an unbounded script cascade into **Rust stack growth**, and a
        stack overflow is an abort, which is exactly the outcome invariant 3 forbids,
        reached by the mechanism a depth counter catches last. Queueing converts recursion
        depth into queue length, which is countable and boundable.
      - `PendingEntry { target, requested_by: Option<Attachment> }` — **a struct, not a
        bare tuple**, so a payload field stays additive (DF13). The payload itself is
        **declined**, with a tripwire written as a condition on an action rather than a
        note, because a note ages into permission: *the first spec requiring
        cross-invocation continuation must evaluate a queue payload before extending the
        callback return convention.*
      - **The queue survives the end of a round.** A `dispatch` seed is **appended** to
        pending rather than replacing it, and `dispatch(&[])` drains the residual.
      - S3's 200-over-4-rounds (64+64+64+8, reporting 200 total) is a **derived oracle** —
        all literals, never a number the host reports back to itself.
      - S4: exactly 64 at a bound of 64 completes **within** the round with **no cascade
        fault of either kind**. Reaching the bound cannot itself be the trigger.
      - S2 forces **eager** reporting: a non-terminating cascade and S3's terminating
        200-chain are locally indistinguishable at the end of round 1 — both hit the bound
        with work pending. The blamed attachment is the **requester** of the first entry
        that could not run, not the entry itself.
      - **Recorded so it is not later read as a defect:** a perfectly well-behaved
        terminating cascade emits three `CascadeDeferred` faults naming three different
        attachments before completing. That is operator-facing noise the spec permits, and
        it is noise precisely because deferral loses nothing.
      - S5: a quarantined target found in the queue is **skipped silently, with no fault**,
        and **skipping does not consume an invocation against the round bound**. Assert the
        second half — a host that consumed the invocation would pass a fault-count
        assertion alone. The queue drain is a **second entry point onto dispatch that no
        FR-4.2 scenario reaches**.
      - S1: the follow-up's entry is recorded **strictly after** the requesting callback's
        return in `DispatchReport::order`.
      - The host reads the returned follow-up list with **raw reads** (T21), so a hostile
        `__index` on the returned table cannot run on the host's schedule.
      - Do **not** add a branch counting cascade faults toward quarantine (T18).

- [ ] **T23** `pending_bound` and the refusal path — `crates/mc-script/src/dispatch.rs`
      Scenarios: FR-5.1-S6
      Depends on: T22
      - **Mechanisms 2 and 7 of the seven, and this is the only place either gets a test.**
        `refused_target` must not arrive before this scenario.
      - **Why a second bound at all:** `round_bound` limits *invocations per round* and
        says nothing about *queue length*. A callback returning a fan-out of N identities
        grows the queue by N−1 per invocation, 64 invocations per round, across unbounded
        rounds — and every entry is a **Rust-side allocation**, outside the Luau memory cap,
        outside `set_memory_limit`, outside the budget, and outside quarantine, since the
        requester succeeds every time. D4's claim that queueing "converts recursion depth
        into queue length, which is countable and boundable" is only true once something
        actually bounds it.
      - S6 lowers `pending_bound` to **8** — the only scenario that needs the queue to
        fill. 12 requested, **8 admitted**, 4 refused, **4 `CascadeRefused` faults**, each
        naming the requesting attachment **and** the attachment whose work was refused.
      - **Assert the 8 admitted actually run.** A host that refused all 12 satisfies a
        fault-count assertion alone.
      - `refused_target: Option<Attachment>` is a **typed field on `ScriptFault`**, never
        substring text inside `cause`. And `FaultKind` stays a plain data-free enum so
        `ContainmentEvidence::FaultReported(FaultKind)` keeps comparing by **equality** —
        had the target been carried inside the variant, FR-7.1-S4's evidence check would
        degrade to a discriminant match.
      - **`CascadeRefused` is not `CascadeDeferred`.** Deferral means the work is
        progressing normally and runs next round; refusal means the work was **dropped and
        will never run**. The consumer that makes it concrete is PRO-919's neighbour
        notification: a full queue silently drops one, a furnace never learns its neighbour
        changed, and the failure is content that quietly does nothing rather than a server
        that is slow. An operator reading one fault kind cannot tell "wait" from "something
        is gone". `CascadeDeferred` stays requester-only; the asymmetry is the point.

---

## Phase 8 — The hostile-mod harness

5 scenarios. **The spec's named deliverable.**

- [ ] **T24** The six hostile cases and their declared evidence —
      `crates/mc-script/tests/support/hostile/` + a sibling integration test
      Scenarios: FR-7.1-S1, FR-7.1-S2, FR-7.1-S3, FR-7.1-S4, FR-7.1-S5
      Depends on: T23
      - **Placement: `crates/mc-script/tests/support/hostile/`**, driven by a sibling
        integration test, following `crates/mc-client/tests/support/input/` — **not**
        `mc-testkit`, which is structurally forbidden from reaching what it verifies.
        Wire it as `#[path = "support/hostile/mod.rs"] mod hostile;`, per that precedent.
      - `hostile_cases() -> [HostileCase; 6]`, `HostileCase { name, requires }`.
      - **The verdict is three-valued:** `CaseOutcome { Contained, Uncontained,
        NotExercised }`. Conflating "uncontained" with "produced no fault" was one of the
        audit's five real holes, because `sandbox-escape` and `hostile-index` are contained
        **precisely by producing no fault**.
      - `ContainmentEvidence { FaultReported(FaultKind), EveryDeniedGlobalUnavailable,
        MetamethodNotInvoked }`.
      - S1: exactly six, named `infinite-loop`, `memory-bomb`, `sandbox-escape`,
        `faulting-callback`, `runaway-cascade`, `hostile-index`.
      - S4: the declared evidence per case — a reported fault for `infinite-loop`,
        `memory-bomb`, `faulting-callback` and `runaway-cascade`; every denied global
        reported unavailable for `sandbox-escape`; an un-invoked metamethod for
        `hostile-index`.
      - S2: all six contained in sequence, **and a benign chunk evaluated afterwards
        returns its value** — the host is left usable, which is the whole claim.
      - S3: a case running to completion **without** producing its declared evidence is
        `Uncontained`, **named**. S5: a case whose script fails to **compile** is
        `NotExercised`, **named** — never `Contained`. An enumerated outcome is what stops
        a harness that stopped running from reading identically to one that ran clean.
      - `memory-bomb`'s configuration inherits T14's binding rule: give it a budget
        generous enough that the memory cap trips first, or it is an `infinite-loop`
        duplicate wearing another name.

- [ ] **T25** The harness-agreement guard —
      `crates/mc-script/tests/harness_boundaries.rs` (+ its fixture)
      Scenarios: none (structural guard)
      Depends on: T24
      - **The worst trap in this breakdown.** All five FR-7 scenarios run **through** the
        harness, so nothing else in the suite can see this. A harness that defines its own
        budget, its own latch or its own deny list **agrees with the host by
        construction**: it would report all six hostile cases contained while the host's
        enforcement had been deleted.
      - **The harness must assert against the host's own enforcement, never against a
        reimplementation of the rule.** The deny list it checks for `sandbox-escape` is the
        host's declared list; the fault it requires is the host's `ScriptFault`; the budget
        that aborts `infinite-loop` is the host's `HostLimits`. Not a literal copy of the
        fourteen names, not a second numeric limit, not a second interrupt.
      - Shape follows `crates/mc-client/tests/seam_boundaries.rs`, whose own module docs
        record the identical failure: *"A harness that gated its own pointer motion on the
        capture policy would agree with the client by construction and pass a three-needle
        scan while the client's own gate was deleted."*
      - Enumerated verdict, per-root non-zero file count, **and its own positive control**:
        the same scan pointed at a fixture that *does* re-implement a policy must report it,
        while the file allowed to name the policy is passed over. **Compare exemptions by
        path segment, not bare file name.**
      - Suggested needles under `tests/support/hostile/`: any of the denied-global names as a
        literal; a numeric budget or memory-limit literal; a second
        `set_interrupt` or latch construction.

- [ ] **T26** The retention path and its misattribution — `crates/mc-script/tests/`
      Scenarios: none (R1; `architecture.md` requires a plain test, **not** a seventh
      hostile case — FR-7.1-S1 pins the harness at exactly six by name)
      Depends on: T20
      - `local kept = {} return function() kept[#kept+1] = string.rep('x', 1000) end`
        retains through a **closure upvalue with no state API at all**, trips no
        per-invocation delta cap, and as the backstop is approached every attachment's
        allocations begin failing with each fault charged to whoever happens to be running.
      - Demonstrate the path **and the misattribution it causes**. This is the residual A7
        accepts and the input to the per-mod-VM revisit at PRO-917, where per-mod VMs are to
        be treated as a **correctness requirement** rather than an optimisation.
      - Stated precisely, because both halves are easy to get wrong: **aggregate retention
        is bounded** by the backstop. What is unbounded is retention **per attachment**, and
        the damage is **misattribution**.
      - **A suspended coroutine is the second retention vector** (DF10, R1): it holds its
        own stack, its locals and everything they reference, for as long as a reference to
        it survives — and a reference to it is itself just an upvalue. What the probe
        measured about `coroutine` is that the interrupt fires inside `resume` and `wrap`
        and that the latch is not void there, which settles **execution only**. *"The latch
        contains it"* and *"it cannot retain across invocations"* are two claims and **only
        the first has evidence.** Neither this test nor its notes may read as though both
        were settled.
      - The framing carried forward is the **accidental** one — careless retention misfiles
        blame — not the hostile one. A mod weaponising containment is not the population
        this project has.

---

## Phase 9 — The shipped defaults

2 scenarios. Physically last because every scenario above configures its own limits, so
nothing there constrains what actually ships.

- [ ] **T27** `HostLimits::default()`, documented on one source and enforced —
      `crates/mc-script/src/limits.rs`, `crates/mc-script/tests/`
      Scenarios: FR-9.1-S1, FR-9.1-S2
      Depends on: T13, T14, T22
      - Without FR-9, a host whose shipped defaults are absent, zero or unlimited passes
        **every** scenario above — and it is the default, not the test value, that runs on
        a real server.
      - **S1 asserts over the set of limits the host reports**, not over four named ones
        (DF8). Two limits — `pending_bound` and `memory_backstop` — were added during the
        architecture's own lifetime, and a scenario naming four would have silently stopped
        covering the ones most likely to be wrong.
      - **FALSIFIABILITY TRAP, binding.** A test asserting only *"finite and non-zero"*
        **cannot fail** — the `NonZero*` types make that a compile-time guarantee. The
        documented values and the constants must live on **one source**, so changing a
        default reddens. The same trap applies to the derived product
        `round_bound × call_and_loop_budget`: it is non-zero by construction. **Assert
        against a stated ceiling or do not assert.**
      - **S2 is the wiring test** and S1 alone cannot substitute for it: with no budget
        configured, a non-terminating callback must abort under the documented default and
        report a budget-exhausted fault. S1 proves only that a number is reported.
      - **Sizing** (DF7): the budget default is sized for the **largest plausible
        unsliceable chunk**. The reason is structural and needs no estimate — `evaluate()`
        cannot be sliced and `dispatch` can: a callback over budget has a mechanism (the
        queue, across rounds), a chunk over budget has none. **No claim about relative
        magnitude may appear, in any form.** The earlier *"the load workload is larger, by
        an order of magnitude"* is **retracted** — redone honestly the two land in the same
        order of magnitude, and in the worked example the callback path was the larger one.
      - The two constraints on the one value **do not covary**: large enough for the biggest
        unsliceable chunk, small enough that `round_bound ×` it fits a tick. The first
        tracks the largest content pack anyone ships and the second tracks the tick budget.
      - **The split trigger is a condition, not a schedule:** the single value is safe until
        something calls `dispatch` from a tick, and whichever spec does that first owns the
        split and the joint re-derivation of **three** quantities — chunk budget, callback
        budget, round bound. Joint derivation is impossible today and that impossibility is
        the finding: there is no tick to derive against.
      - `memory_cap` is a **delta above the entry baseline**, so its floor is set by how
        much a callback plausibly needs and 256 KiB is perfectly expressible. It is
        `memory_backstop` that must exceed the measured **385,952 B** baseline plus that
        delta.
      - Sizing the budget against the measured costs in Binding inputs — 4,369 / 8,465 /
        12,561 for a 16³ pass — is the intended use of those numbers.

---

## Carried to `/sdd-complete`

Recorded here so they survive the spec folder's deletion. Four obligations, and the
first three are corrections to documents this spec has measured wrong.

1. **`spec.md`'s D7 lint mechanism is unbuildable.** Both `spec.md` § Technical
   Considerations and `requirements.md` D7 say `crates/mc-script/Cargo.toml` gains a
   per-crate `[lints.clippy]` table. Cargo **hard-errors** on a manifest carrying both
   `[lints] workspace = true` and a local `[lints.clippy]` table:
   `cannot override 'workspace.lints' in 'lints'`. The mechanism that ships is a
   crate-root `#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]` (T01),
   which **does not cover `crates/mc-script/tests/*`** — each integration test is its own
   crate root, and the gate's `-D warnings` covers those instead. One-line correction.
2. **ADR-003's Consequences claim `set_memory_limit` caps allocation. Measured false**
   (T14) — a `pcall`-wrapped bomb defeats it, looping ten times and returning normally.
   ADR-003's *decision* (Luau via `mlua`) is binding and not reopened; this is a factual
   correction to an existing ADR.
3. **`docs/INDEX.md`'s routing row says "instruction budgets"** (`docs/INDEX.md:83`) and
   is wrong for the same measured reason: the interrupt fires on calls and loop edges, not
   per instruction. The row routes to `docs/modding/sandbox.md`. This spec renamed the
   limit throughout its own documents; the routing table lives in `docs/`, so it is
   corrected when the consolidation lands.
4. **ADR-022 is required.** VM granularity is **its own decision, not an ADR-003
   amendment**, and the tree is at ADR-021. It records: one VM; the loader-versus-content
   argument that rejects per-component VMs (component count is controlled by *content*,
   mod count by an operator's loader, and making the VM count a function of a
   content-controlled quantity turns 389 KiB of unreclaimable fixed overhead into a
   per-registration multiplier spent *before* any invocation, invisible to every mechanism
   this spec builds); R1 and its accepted residual; the derived pressure trigger under the
   **accident** framing rather than the hostile one; and the retention ledger as a **priced
   refusal**. Conflating obligations 2 and 4 is the error to avoid.

**`docs/modding/sandbox.md`** is the pre-committed consolidation destination and does not
exist yet; `/sdd-complete` creates it. Two things must go in it and **neither is
softened** (DF19, DF17):

- It must answer *"how do I do a job bigger than one budget?"*, and today the only honest
  answer is the **retention construction** — a closure upvalue carrying a cursor between
  invocations, which is R1's own mechanism. That loop closes knowingly.
- Directly beside it, the authoring rule: *an abort is not recoverable in script; never
  rely on post-`pcall` cleanup; build then swap, never mutate in place.* After the latch no
  script frame progresses, so a `pcall` handler cannot run — **that is the latch working,
  not a defect**, and any on-abort hook re-opens the `pcall` hole the latch exists to
  close.

Whoever writes the page will find the tension between those two. It is deliberate. Do not
resolve it by omitting one.

**Also for the mutation table at `/sdd-validate`.** MVP 2's exit criterion is that each of
the six hostile cases is *"contained, named, and proven by a mutation that reddens a
test"*. The table covers FR-2 through FR-7 and **includes the mutations that did not
bite** — those are evidence about the code's structure, not a free pass. Three are already
named: R3's headline (remove the latch; confirm `spin_pcall` and `bomb_pcall` both pass
when they should fail), R3's companion (break the latch's **re-arm**; confirm `spin_pcall`
reddens rather than reporting a caught `ScriptError` — T13's marked inference), and T12's
`Err` + `Clear` arm.

---

## Scenario assignment

All 56 scenarios, each in exactly one task.

| Scenario | Task | Ph | | Scenario | Task | Ph | | Scenario | Task | Ph |
|---|---|---|---|---|---|---|---|---|---|---|
| FR-1.1-S1 | T06 | 2 | | FR-3.1-S6 | T16 | 4 | | FR-4.2-S10 | T19 | 5 |
| FR-1.1-S2 | T06 | 2 | | FR-4.1-S1 | T11 | 3 | | FR-5.1-S1 | T22 | 7 |
| FR-1.1-S3 | T06 | 2 | | FR-4.1-S2 | T11 | 3 | | FR-5.1-S2 | T22 | 7 |
| FR-1.1-S4 | T06 | 2 | | FR-4.1-S3 | T08 | 2 | | FR-5.1-S3 | T22 | 7 |
| FR-1.2-S1 | T05 | 2 | | FR-4.1-S4 | T11 | 3 | | FR-5.1-S4 | T22 | 7 |
| FR-1.2-S2 | T07 | 2 | | FR-4.1-S5 | T11 | 3 | | FR-5.1-S5 | T22 | 7 |
| FR-1.2-S3 | T06 | 2 | | FR-4.2-S1 | T18 | 5 | | FR-5.1-S6 | T23 | 7 |
| FR-1.2-S4 | T05 | 2 | | FR-4.2-S2 | T18 | 5 | | FR-6.1-S1 | T21 | 6 |
| FR-1.2-S5 | T07 | 2 | | FR-4.2-S3 | T18 | 5 | | FR-6.1-S2 | T21 | 6 |
| FR-1.2-S6 | T05 | 2 | | FR-4.2-S4 | T18 | 5 | | FR-6.1-S3 | T21 | 6 |
| FR-2.1-S1 | T13 | 3 | | FR-4.2-S5 | T18 | 5 | | FR-6.1-S4 | T21 | 6 |
| FR-2.1-S2 | T13 | 3 | | FR-4.2-S6 | T18 | 5 | | FR-7.1-S1 | T24 | 8 |
| FR-2.1-S3 | T13 | 3 | | FR-4.2-S7 | T20 | 5 | | FR-7.1-S2 | T24 | 8 |
| FR-2.1-S4 | T13 | 3 | | FR-4.2-S8 | T19 | 5 | | FR-7.1-S3 | T24 | 8 |
| FR-2.1-S5 | T13 | 3 | | FR-4.2-S9 | T19 | 5 | | FR-7.1-S4 | T24 | 8 |
| FR-3.1-S1 | T14 | 4 | | | | | | FR-7.1-S5 | T24 | 8 |
| FR-3.1-S2 | T14 | 4 | | | | | | FR-8.1-S1 | T02 | 1 |
| FR-3.1-S3 | T15 | 4 | | | | | | FR-8.1-S2 | T02 | 1 |
| FR-3.1-S4 | T14 | 4 | | | | | | FR-8.1-S3 | T02 | 1 |
| FR-3.1-S5 | T20 | 5 | | | | | | FR-9.1-S1 | T27 | 9 |
| | | | | | | | | FR-9.1-S2 | T27 | 9 |

**Counts by task.** T02 ×3 · T05 ×3 · T06 ×5 · T07 ×2 · T08 ×1 · T11 ×4 · T13 ×5 ·
T14 ×3 · T15 ×1 · T16 ×1 · T18 ×6 · T19 ×3 · T20 ×2 · T21 ×4 · T22 ×5 · T23 ×1 ·
T24 ×5 · T27 ×2. Total **56**.

**Counts by phase.** 1: 3 · 2: 11 · 3: 9 · 4: 5 · 5: 11 · 6: 4 · 7: 6 · 8: 5 · 9: 2.
Total **56**.

**Counts by requirement**, against `spec.md`'s own figures: FR-1.1 4 · FR-1.2 6 ·
FR-2.1 5 · FR-3.1 6 · FR-4.1 5 · FR-4.2 10 · FR-5.1 6 · FR-6.1 4 · FR-7.1 5 ·
FR-8.1 3 · FR-9.1 2. **All eleven match.**

---

## Notes

Deferred observations and follow-ups. Never delete task text; append status markers only.

- **`spec.md` § Existing Code cites `crates/mc-testkit/tests/workspace_layering.rs`
  line 40; the `("mc-script", None)` pair is at line 43**, which is what
  `architecture.md` § Integration says. Verified at `9c9ab8e`. Not corrected in the spec —
  a line number in prose ages by itself, and T04 names the pair rather than the line.
- **Two documents disagree about how many limits FR-9 covers, and both are right in
  their own frame.** `requirements.md` § Scenario audit describes FR-9 as "one enumerated
  verdict covering all four defaults" — that is the audit's outcome, recorded at 46
  scenarios. DF8 supersedes it: the assertion is over *the set of limits the host
  reports*, and `HostLimits` now carries six. The set formulation is binding, and the
  reason is exactly this drift. Resolved in T27; no edit to `requirements.md`, whose
  section is a dated record of what the audit said.
- **`requirements.md` D5 lists four limits** (call-and-loop budget, memory cap, fault
  threshold, round invocation bound) where `HostLimits` carries six. `pending_bound` and
  `memory_backstop` were added by A9 and DF2 after D5 was written. Same drift, same
  resolution; D5's *principle* — configurable, with a documented default — is unaffected
  and binding.
- **`requirements.md` D7 and `spec.md` still describe the manifest lint table**, which is
  unbuildable. Left standing deliberately: `architecture.md` A8 supersedes the mechanism
  and the correction is item 1 under Carried to `/sdd-complete`. Do not fix it in the
  implementation phases — a spec edit mid-implementation is a separate commit from code and
  would land the correction twice.
- **`crates/mc-testkit/src/lib.rs` advertises "scripted bot clients, determinism and load
  fixtures" and only `pub mod frame;` exists.** Not this spec's to fix; noted because a
  reader looking for a harness base will find the claim before finding T24's placement
  decision.
- **Root `CLAUDE.md`'s tech-stack table still lists `redb`/`bincode` for storage** after
  ADR-016 made the save a `postcard` plain file. Cosmetic and unrelated — noted only
  because the same table carries the Luau row this spec implements, so it should not be
  read as current.
