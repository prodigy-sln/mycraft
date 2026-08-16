---
id: SPEC-014
title: Luau host, sandbox and the hostile-mod harness
status: implemented
rigor: high
branch: feature/PRO-916-luau-host-sandbox
issue: PRO-916
created: 2026-08-15
updated: 2026-08-16
completed: 2026-08-16
author: Sebastian Grunow
---

# Specification: Luau host, sandbox and the hostile-mod harness

## Goal

Embed Luau in `crates/mc-script` as a sandboxed, budgeted, memory-capped host whose
callback dispatch contains a hostile or careless mod, and ship the harness that runs
six named hostile scripts and asserts the host survives each. Invariant 3 — *a bad
mod never takes down the server* — stops being a claim and becomes a measurement,
before any scripting API exists to harden afterwards.

## User Stories

- As a server operator, I want a hostile or badly-written mod to be contained so that
  my server keeps running for the other 31 players.
- As a mod author, I want a fault in my callback attributed to my component by name so
  that I can find it without reading engine logs.
- As a mod author, I want another mod's broken attachment on a block I also extend to
  stop acting without stopping mine.
- As an engine contributor, I want a named hostile-script harness so that every
  containment claim is measured before the scripting API that would need hardening.
- As an engine contributor, I want the client to be structurally unable to reach the
  scripting host, so that mod behaviour cannot run inside the untrusted process.

## Functional Requirements

Scenario rules: `standards/global/scenario-guidelines.md`. Each scenario becomes
exactly one test, mapped in this folder's `test-map.md`.

**Vocabulary.** A **subject** is an opaque namespaced identity the host stores and
never interprets (a block name, later; an arbitrary string here). A **component** is
an opaque namespaced identity under which a callback is registered against a subject.
The `(subject, component)` pair is an **attachment**, and it is the unit of budget,
fault counting and quarantine. Attaching components to real block definitions is
PRO-919's; the host's notion that a callback belongs to an attachment is this spec's.

**Enumerated verdicts, not absences.** Several scenarios below assert that something
is unreachable. Per `standards/global/testing.md` §2, each is paired with a scenario
asserting the same mechanism *does* report the positive case, and the verdicts are
exact rather than "no findings".

**Configured values used below.** The scenarios configure small limits so they run
inside the integration-test budget: a call-and-loop budget of **10,000**, a memory cap
of **1 MiB**, a fault threshold of **3**, a dispatch round bound of **64**
invocations, and a pending-work bound of **256** entries — lowered to **8** by
FR-5.1-S6, which is the only scenario that needs the queue to fill. The absolute memory
backstop sits above the memory cap and is configured with it. These are test
configuration, not the shipped defaults — FR-9 is what holds the defaults to account.

**What the call-and-loop budget counts.** Not instructions. The Luau interrupt fires on
calls, on returns and on loop edges, and on nothing else, so **the body of a loop is
free**: a loop of ten statements costs exactly what an empty loop costs, and a thousand
straight-line statements cost one. A call into Rust costs one; a call within Luau costs
two. The consequence for whoever sizes this, and for whoever writes content against it:
**cost is reduced by batching calls, not by shortening code.**

### FR-1 — The sandbox is closed before content runs

- **FR-1.1**: The global environment is frozen before any content chunk is evaluated,
  so no chunk can add to it or overwrite what another chunk relies on.
  - FR-1.1-S1: WHEN a content chunk assigns to a previously undefined global name THE
    SYSTEM SHALL reject the assignment with a script error naming the chunk.
  - FR-1.1-S2: WHEN a content chunk declares a local and returns it THE SYSTEM SHALL
    return the value 42 to the host.
  - FR-1.1-S3: IF a content chunk assigns to `string.format` THEN THE SYSTEM SHALL
    reject the assignment, and a second chunk evaluated afterwards SHALL observe
    `string.format` unchanged.
  - FR-1.1-S4: IF a content chunk calls `rawset` on the global table with the
    previously undefined name `smuggled` THEN THE SYSTEM SHALL reject the write with a
    script error, and a second chunk evaluated afterwards SHALL report `smuggled` as
    unavailable.

- **FR-1.2**: The denied capability surface is unreachable from script, the permitted
  surface is reachable, and the reachable set is exactly the set the host declares — so
  a global a future release *adds* is reported rather than silently arriving.
  - FR-1.2-S1: WHEN a content chunk evaluates each of the global names `io`, `os`,
    `package`, `require`, `loadstring`, `load`, `dofile`, `loadfile` and `debug` THE
    SYSTEM SHALL report every one of the nine as unavailable.
  - FR-1.2-S2: WHEN a content chunk evaluates each of the global names `math`,
    `string`, `table`, `pairs`, `tostring`, `pcall`, `select`, `type`, `coroutine`,
    `buffer` and `print` THE SYSTEM SHALL report every one of the eleven as available.
  - FR-1.2-S3: IF a content chunk assigns a new `__index` to the shared string
    metatable THEN THE SYSTEM SHALL reject the assignment with a script error, and a
    second chunk evaluated afterwards SHALL observe the original metatable behaviour.
  - FR-1.2-S4: WHEN a content chunk evaluates each of the global names `getfenv`,
    `setfenv`, `collectgarbage`, `newproxy` and `gcinfo` THE SYSTEM SHALL report
    every one of the five as unavailable.
  - FR-1.2-S5: WHEN the set of global names reachable from a content chunk is
    enumerated THE SYSTEM SHALL report it as exactly equal to the permitted set the
    host declares, naming any name reachable but undeclared.
  - FR-1.2-S6: WHEN a content chunk calls `print` THE SYSTEM SHALL record the call at
    the host, and a chunk that attempts to replace `print` inside a protected call and
    then calls it SHALL report the replacement as refused and SHALL have that later
    call recorded at the host too, rather than reaching a `print` the host does not
    observe. *(The protected call is load-bearing rather than incidental: under a
    frozen environment an unprotected replacement aborts the chunk at the assignment,
    so it never reaches the call this scenario is about.)*

### FR-2 — Every entry into script is budgeted, per attachment

- **FR-2.1**: Each callback invocation runs under a call-and-loop budget charged to its
  attachment, and exhausting it aborts that invocation and nothing else.
  - FR-2.1-S1: WHEN a callback executes a loop that never terminates THE SYSTEM SHALL
    abort the invocation once its call-and-loop budget of 10,000 is exhausted and report
    a budget-exhausted fault naming the subject and the component.
  - FR-2.1-S2: WHEN a callback completes a loop of 1,000 iterations under a
    call-and-loop budget of 10,000 THE SYSTEM SHALL return its result and report no
    fault.
  - FR-2.1-S3: WHEN two attachments are invoked in the same dispatch round and the
    first exhausts its call-and-loop budget of 10,000 THE SYSTEM SHALL invoke the second
    with its own full 10,000 and return the second's result.
  - FR-2.1-S4: WHEN an attachment whose previous invocation was aborted for budget
    exhaustion is invoked again THE SYSTEM SHALL grant it a full call-and-loop budget of
    10,000 rather than the remainder of the exhausted one.
  - FR-2.1-S5: IF a callback wraps its non-terminating loop in a protected call THEN
    THE SYSTEM SHALL still abort the invocation once its call-and-loop budget of 10,000
    is exhausted and report a budget-exhausted fault, rather than returning control to
    the callback.

### FR-3 — Allocation is capped and the cap is recoverable

- **FR-3.1**: A script that allocates past the host's memory limit is stopped by a
  script-level error, never by an allocation abort, and the memory is reclaimed — and a
  failure the callback's own demand did not cause is not attributed to it.
  - FR-3.1-S1: WHEN a callback appends to a table until the configured 1 MiB memory
    limit is reached THE SYSTEM SHALL abort the invocation and report an allocation
    fault naming the subject and the component.
  - FR-3.1-S2: WHEN a callback allocates a 64 KiB structure under a 1 MiB limit THE
    SYSTEM SHALL complete the invocation and return its result.
  - FR-3.1-S3: WHEN a callback has been aborted for exceeding the 1 MiB memory limit
    THE SYSTEM SHALL reclaim its allocation such that a subsequent callback allocating
    512 KiB completes successfully.
  - FR-3.1-S4: IF a callback wraps its allocation past the 1 MiB limit in a protected
    call THEN THE SYSTEM SHALL still abort the invocation and report an allocation
    fault, rather than returning control to the callback.
  - FR-3.1-S5: WHILE the host's collected memory baseline is high enough that not even
    a further 64 KiB fits below the absolute backstop, WHEN an attachment whose callback
    allocates only 64 KiB is invoked three times THE SYSTEM SHALL report each fault as a
    host-memory-pressure fault carrying no subject and no component, and SHALL report
    that attachment as not quarantined. *(The tighter precondition is what the fixture
    builds to. "A further 1 MiB no longer fits" is the classification condition and it
    still admits a 64 KiB allocation succeeding, which would leave nothing faulting and
    so no fault to reclassify.)*
  - FR-3.1-S6: WHEN a callback is aborted for exceeding the 1 MiB memory limit THE
    SYSTEM SHALL report a fault whose rendered cause is non-empty and states the limit
    that was exceeded.

### FR-4 — A fault is reported, attributed, and never a panic

- **FR-4.1**: A fault raised anywhere inside script execution is reported to the host
  as a value, names the chunk that defined the failing callback, and leaves the host
  usable.
  - FR-4.1-S1: WHEN a callback raises a script error THE SYSTEM SHALL report a fault
    carrying the subject, the component and the error's text, and SHALL return control
    to the caller.
  - FR-4.1-S2: IF a callback raises a table whose `__tostring` metamethod increments a
    counter field on a second script table THEN THE SYSTEM SHALL report a fault and
    that counter SHALL still read 0.
  - FR-4.1-S3: IF a content chunk fails to compile THEN THE SYSTEM SHALL report a
    compilation fault naming the chunk and line 3, and SHALL remain able to evaluate a
    subsequent valid chunk.
  - FR-4.1-S4: WHEN a content chunk itself converts that same table to a string THE
    SYSTEM SHALL leave the counter reading 1.
  - FR-4.1-S5: WHEN a callback defined by a chunk named `furnace.luau` raises a script
    error during a dispatch round THE SYSTEM SHALL report a fault whose origin names
    `furnace.luau`, and a callback defined by a second chunk faulting in the same round
    SHALL report that second chunk's name.

- **FR-4.2**: An attachment that faults on three consecutive invocations is
  quarantined; nothing else is; faults raised while the host is under memory pressure do
  not count toward that total; and quarantine is lifted only by releasing the attachment
  or by attaching a new callback to it.
  - FR-4.2-S1: WHEN an attachment's callback faults on three consecutive invocations
    THE SYSTEM SHALL stop invoking it and report the quarantine naming the subject and
    the component.
  - FR-4.2-S2: WHEN an attachment's callback faults on two consecutive invocations and
    succeeds on the third THE SYSTEM SHALL invoke it again on the fourth and report it
    as not quarantined.
  - FR-4.2-S3: WHILE one attachment on a subject is quarantined THE SYSTEM SHALL
    continue to invoke every other component attached to that same subject and return
    their results.
  - FR-4.2-S4: WHILE a component is quarantined on one subject THE SYSTEM SHALL
    continue to invoke that same component on a different subject.
  - FR-4.2-S5: WHEN an attachment's callback is aborted for budget exhaustion, then
    aborted for exceeding the memory limit, then raises a script error, on three
    consecutive invocations THE SYSTEM SHALL quarantine it and report the quarantine
    naming the subject and the component.
  - FR-4.2-S6: WHILE an attachment is quarantined THE SYSTEM SHALL leave it uninvoked
    across three subsequent dispatch rounds and report its invocation count as
    unchanged from the count at quarantine.
  - FR-4.2-S7: WHILE the host's collected memory baseline is high enough that a further
    1 MiB no longer fits below the absolute backstop, WHEN an attachment's callback runs
    a loop that never terminates on three consecutive invocations THE SYSTEM SHALL
    report that attachment as not quarantined.
  - FR-4.2-S8: WHEN a quarantined attachment is released THE SYSTEM SHALL invoke it in
    the next dispatch round and report its invocation count as one greater than the
    count frozen at quarantine.
  - FR-4.2-S9: WHEN a callback is attached to an attachment that already carries one
    THE SYSTEM SHALL invoke the newly attached callback in the next dispatch round and
    return the newly attached callback's result.
  - FR-4.2-S10: WHEN a callback is attached to a quarantined attachment THE SYSTEM SHALL
    report that attachment as no longer quarantined and invoke the newly attached
    callback in the next dispatch round.

### FR-5 — A cascade settles slowly; it never hangs and never recurses

- **FR-5.1**: Follow-up work requested by a callback is queued rather than executed
  inline; a dispatch round is bounded by a total invocation count; and the queue itself
  is bounded, so work that cannot be admitted is refused and named rather than silently
  dropped. Work deferred to a later round and work refused outright are reported as
  different faults, because only one of them loses the work.
  - FR-5.1-S1: IF a callback requests follow-up work THEN THE SYSTEM SHALL record the
    follow-up's entry strictly after the requesting callback's return in the round's
    invocation order.
  - FR-5.1-S2: WHEN a callback requests follow-up work that in turn requests further
    follow-up work without ever terminating THE SYSTEM SHALL end the round once its
    bound of 64 invocations is reached, return control to the caller, and report a
    cascade-deferred fault naming the subject and the component that produced the
    overflow.
  - FR-5.1-S3: WHEN a terminating cascade of exactly 200 invocations runs with a round
    bound of 64 THE SYSTEM SHALL complete it over 4 rounds and report 200 invocations
    performed in total.
  - FR-5.1-S4: WHEN a terminating cascade of exactly 64 invocations runs with a round
    bound of 64 THE SYSTEM SHALL complete all 64 within that round and report no
    cascade fault of either kind.
  - FR-5.1-S5: IF a callback requests follow-up work naming an attachment that is
    already quarantined THEN THE SYSTEM SHALL complete the round without invoking that
    attachment and report no fault for it.
  - FR-5.1-S6: IF a callback requests 12 follow-up attachments while the pending-work
    bound is 8 and the queue is empty THEN THE SYSTEM SHALL admit 8 of them, refuse the
    remaining 4, and report 4 cascade-refused faults, each naming the requesting
    attachment and the attachment whose work was refused.

### FR-6 — The engine reads script values raw

- **FR-6.1**: A field the host reads from a script-supplied table is read without
  invoking script code, so a metatable can neither run on the host's schedule nor
  observe which fields the host reads.
  - FR-6.1-S1: WHEN the host reads a field that is absent from a script-supplied table
    whose metatable defines an `__index` function THE SYSTEM SHALL report the field as
    absent and the metamethod SHALL NOT have been invoked.
  - FR-6.1-S2: WHEN the host reads a field that is genuinely present on a
    script-supplied table THE SYSTEM SHALL return that field's value.
  - FR-6.1-S3: IF a script-supplied table's `__index` metamethod would loop forever
    when invoked THEN THE SYSTEM SHALL complete the host's read of that table, and the
    calling attachment SHALL afterwards execute a loop of 9,000 iterations under its
    call-and-loop budget of 10,000 without being aborted.
  - FR-6.1-S4: WHEN a content chunk itself indexes the absent field on that same
    script-supplied table THE SYSTEM SHALL report the metamethod as having been
    invoked exactly once.

### FR-7 — The hostile-mod harness is the deliverable

- **FR-7.1**: The harness enumerates its hostile cases by name, declares the
  containment evidence each requires, and distinguishes a contained case from one that
  was never exercised.
  - FR-7.1-S1: WHEN the harness is asked for its hostile cases THE SYSTEM SHALL report
    exactly six, named `infinite-loop`, `memory-bomb`, `sandbox-escape`,
    `faulting-callback`, `runaway-cascade` and `hostile-index`.
  - FR-7.1-S2: WHEN the harness runs all six hostile cases in sequence THE SYSTEM
    SHALL report each as contained, and a benign chunk evaluated afterwards SHALL
    return its value.
  - FR-7.1-S3: IF a hostile case runs to completion without producing the containment
    evidence its case declares THEN THE SYSTEM SHALL report that case as uncontained,
    naming it, rather than reporting the run as clean.
  - FR-7.1-S4: WHEN the harness is asked for its hostile cases THE SYSTEM SHALL report
    for each of the six the containment evidence it requires — a reported fault for
    `infinite-loop`, `memory-bomb`, `faulting-callback` and `runaway-cascade`; every
    denied global reported unavailable for `sandbox-escape`; an un-invoked metamethod
    for `hostile-index`.
  - FR-7.1-S5: IF a hostile case's script fails to compile THEN THE SYSTEM SHALL
    report that case as not exercised, naming it, rather than as contained.

### FR-8 — The client cannot reach the scripting host

- **FR-8.1**: `mc-client`'s resolved dependency closure — across every dependency
  kind, dev-dependencies included — excludes `mc-script`, and the walk that says so is
  proven non-vacuous in both directions.
  - FR-8.1-S1: WHEN the workspace's resolved dependency graph is walked from
    `mc-client` THE SYSTEM SHALL report `mc-script` as absent from its closure.
  - FR-8.1-S2: IF `mc-script` is absent from the resolved workspace metadata
    altogether THEN THE SYSTEM SHALL fail rather than report `mc-client` clean.
  - FR-8.1-S3: WHEN the same walk is performed THE SYSTEM SHALL report a dependency
    `mc-client` genuinely has as present in its closure.

### FR-9 — Every limit has a documented default that is itself enforced

Physically last because it was added by the scenario audit; every scenario above
configures its own limits, so nothing there constrains what actually ships.

- **FR-9.1**: A host constructed without configuration applies documented defaults,
  and those defaults enforce.
  - FR-9.1-S1: WHEN a host constructed without configuration is asked for its
    effective limits THE SYSTEM SHALL report every limit it defines, and each reported
    limit SHALL equal the value documented for it — so that a limit added later is
    covered without this scenario being amended.
  - FR-9.1-S2: IF no call-and-loop budget has been configured THEN THE SYSTEM SHALL
    abort a non-terminating callback under the documented default and report a
    budget-exhausted fault.

## Technical Considerations

Rationale for each decision, with the alternative considered, is in
`requirements.md` (D1–D8). The binding outcomes:

- **`require` is absent rather than confined** (D1). There is no mod-directory concept
  yet to confine it to; absence over-satisfies `crates/mc-script/CLAUDE.md`
  invariant 1 and confinement lands with PRO-917.
- **Quarantine is per attachment, `(subject, component)`** (D2), not per component
  workspace-wide. Containment costs `3 × subjects` faults for a component broken
  everywhere; each of those faults is itself bounded by FR-2 and FR-3.
- **Three *consecutive* faults, count reset by a success** (D3). The budget bounds
  cost; quarantine bounds repetition. A callback alternating fail/succeed is never
  quarantined and that is accepted, because its cost is already bounded.
- **Dispatch is never re-entrant** (D4) — this is the load-bearing decision behind
  FR-5. Synchronous re-entry would turn an unbounded cascade into Rust stack growth,
  and a stack overflow is an abort, which is exactly the outcome invariant 3 forbids.
  Queueing converts recursion depth into queue length, which is countable and
  boundable. The bounded work queue is also what PRO-919's neighbour notification
  needs, so nothing here is built to be deleted.
- **Every limit is configurable with a documented default** (D5). Not flexibility for
  its own sake: a production-sized memory cap cannot be tripped inside the
  one-second integration-test budget. The scenarios configure small values; the
  defaults ship. Confirming the default *values* against measurement is the
  architect's.
- **How many VMs exist is left to `/sdd-architect`** (D6) and named as a driver: one
  shared VM makes interop trivial, one per mod isolates the memory cap the way FR-2
  isolates the call-and-loop budget. Every FR-3 scenario is stated at the boundary a
  script observes and reads identically under either answer.
- **`crates/mc-script/src/lib.rs` carries a crate-root `#![deny(...)]`** for
  `unwrap_used`, `expect_used` and `panic` (D7). The per-crate `[lints.clippy]`
  table this originally specified is *unbuildable*: cargo hard-errors with
  `cannot override 'workspace.lints' in 'lints'` on a manifest carrying both
  `[lints] workspace = true` and a local table. The attribute composes with the
  inherited workspace lints instead of displacing them, so
  `crates/mc-script/CLAUDE.md` invariant 4 becomes true at plain `cargo check`.
  **Its scope is narrower than the manifest table would have been:** it covers
  the lib target and its sibling `_test.rs` modules but **not**
  `crates/mc-script/tests/*`, where each file is its own crate root and only the
  gate's `-D warnings` reaches.
- **Fault attribution is in scope; per-mod CPU accounting is not** (D8). Accounting
  needs a tick to attribute against, and `mc-sim` does not call `mc-script` here.
- **Fault type shape follows the two ports already in the tree.** `DefinitionFault`
  and `HudFault` both carry `origin` + which-thing + which-field + cause with a
  hand-written `Display`; a script fault should read like them rather than invent a
  third convention.
- **A protected call does not defeat a limit** (FR-2.1-S5, FR-3.1-S4). Both an
  interrupt-raised abort and an allocation failure surface to Lua as ordinary
  catchable errors, so `while true do pcall(f) end` would re-enter the budget
  indefinitely and invariant 2 would be decorative. The invocation must latch as
  aborted once a limit trips, so the abort re-arms rather than being swallowed.
  This is the audit's most consequential finding and is not optional.
- **Denied by default, including anything Luau's own sandbox leaves standing.**
  FR-1.2-S1 and FR-1.2-S4 name fourteen globals as unavailable. Where Luau's
  `sandbox(true)` does not itself remove one — `getfenv`/`setfenv` are the escape
  pair, `collectgarbage` a memory side channel, `newproxy` a `__gc` vector — the host
  removes it explicitly. This is the API surface policy's filter applied in the
  direction it cuts hardest: the question is *"can this be abused, and is it
  bounded?"*, and content has no business collecting garbage.
- **`gcinfo` is denied for determinism, and it was found by enumerating rather
  than by judgement.** It reports the heap size, so a script branching on it is a
  function of the collector's state rather than of its own inputs. World
  generation is the caller that cannot survive that: a mod-provided generator
  branching on heap size returns different terrain for one seed, which loses a
  world rather than leaking a number — the same objection that denies
  `collectgarbage`, from the other direction, since one reads the collector and
  one drives it. It is also a published surface the moment this ships, at which
  point removing it breaks third-party mods. Nobody had named it: it was found by
  FR-1.2-S5's enumeration of what the standard library actually leaves standing,
  which is precisely the accident case that scenario exists to catch.
- **FR-9 exists because every other scenario configures its own limits.** Without it,
  a host whose shipped defaults are absent, zero or unlimited passes every scenario
  above — and it is the default, not the test value, that runs on a real server.
  FR-9.1-S1 is stated over *the set of limits the host reports* rather than over four
  named ones, because a scenario naming four stops covering the fifth silently, and two
  limits were added to the design after that scenario was first written.
- **The pending work queue is bounded as well as the round.** A round bound limits
  invocations per round and says nothing about queue length; a callback returning a
  fan-out grows the queue faster than a round drains it, and every entry is a host-side
  allocation outside every limit above. Work that cannot be admitted is **refused**, and
  refusal is reported as a different fault from deferral because only refusal loses the
  work — a deferred follow-up runs next round, a refused one never runs at all, and an
  operator reading one fault kind cannot tell "wait" from "something is gone".
- **The permitted surface is enumerated exactly, not merely sampled.** FR-1.2-S5 asserts
  set equality against a list the host declares. FR-1.2-S1/S4 catch a denied global being
  *reintroduced*; only an exact set catches one being *added* by a future Luau or `mlua`
  release, which is the accident case rather than the attack case. The stronger
  justification is the deny list's own history: it was derived by asking which globals
  should be removed, never by enumerating what the standard library actually leaves
  standing — which is how `print` sat on the escape axis and was missed, and how four of
  the eight globals that survive Luau's own sandbox turned out not to be the four this
  spec expected. A judgement-derived denylist keeps missing whatever nobody thought to
  name; enumerating what is really there is the only defence that does not depend on
  thinking of it first.
- **`coroutine` is permitted.** The interrupt fires inside `coroutine.resume` and
  `coroutine.wrap`, so neither limit is void there, and the latch defeats every catch
  construction available inside a coroutine — including respawning a fresh one after each
  abort. Denying it would remove the only cooperative-yield primitive content has, in
  exchange for nothing measured.
- **`print` is a sandbox escape, and it is enumerated on the same footing as the fourteen
  denied globals.** Luau's own `print` writes to C `stdout`, a different buffer from the
  host's, which cannot be routed. Installed after the sandbox is closed, the host's
  `print` is bypassed by a fall-through to the original, and content writes to raw fd 1
  outside all host control — content reaching a capability the sandbox was supposed to
  have removed, which is an escape and not a logging inconvenience. **The binding rule is
  the ordering: the host's `print` is installed before the sandbox is closed; installing
  or removing it afterwards silently succeeds and changes nothing.** The deny list obeys
  the same rule for the same reason. FR-1.2-S6 is what holds it.
- **The budget is named for what it counts, and it does not count instructions.** The
  Luau interrupt fires on calls, on returns and on loop edges only, so **the body of a
  loop is free** and a thousand straight-line statements cost one. A name like
  "instruction budget" guarantees somebody sizes it against VM instructions and is wrong
  by the size of the loop body, which is why the name is corrected here rather than left
  for consolidation — the harm lands during implementation, not after it. The literal
  10,000 is unchanged; only the name and the prose describing what it counts.
- **The cost driver is the number of calls, which is a quantity a content author
  controls.** This is a measured cost rule, not a style preference, and
  `crates/mc-script/CLAUDE.md` already carries the same advice on other grounds — *"Prefer
  one call passing a batch over N calls passing one item"*, written about binding
  overhead. It turns out to govern the budget too: a call into Rust costs one, a call
  within Luau costs two, and the work between calls costs nothing. So the way to fit a
  budget is to batch calls, never to shorten code, and the sizing question is "how many
  calls does this workload make" rather than "how many instructions does it cost" —
  the second being a question nobody can answer.
- **A fault names the chunk that defined the failing callback** (FR-4.1-S5), not only the
  dispatch round it occurred in. Without it, the most common fault in the design names no
  file, which leaves a mod author with an error and no way to locate it.
- **An allocation abort's cause is composed by the host, not passed through** (FR-3.1-S6).
  The underlying error carries no message and no line at all, so rendering it faithfully
  produces an empty string — a fault that names its subject and component and then says
  nothing about why. An empty cause and a cause that was never populated read identically,
  so the scenario asserts a non-empty one that states the limit exceeded. This is also why
  the line and the refused target are typed fields rather than text: allocation is the one
  fault kind whose cause string is empty at the source, which makes the structured fields
  not the better diagnosis but the only one.
- **Quarantine is lifted by replacing or releasing the attachment, and by nothing else.**
  Replacing a callback lifts it because that is what reloading a fixed mod *is*; a replace
  that left the attachment quarantined would silently fail at the one thing reloading
  exists to do. The invocation count is cumulative telemetry about the attachment, so it
  resumes from its frozen value rather than resetting — it is a different counter from the
  consecutive-fault count, which already resets on a success (D3).
- **Faults raised while the host is short of memory do not count toward quarantine**
  (FR-3.1-S5, FR-4.2-S7). When the host's own baseline leaves no room for an invocation's
  cap, the invocation can fail for a reason that is not its own, and attributing that to
  the running attachment disables an innocent mod and files the blame against the wrong
  author. The cost of the exclusion is accepted and stated rather than hidden: while the
  condition holds, a genuinely looping mod is not quarantined either. That failure is
  loud — the server is slow and the operator acts — where the misattribution is silent and
  points at the wrong file.

**Verification obligation carried from the roadmap.** MVP 2's exit criterion is that
each of the six hostile cases is "contained, named, and **proven by a mutation that
reddens a test**". `/sdd-validate` must record a mutation table for FR-2 through FR-7,
following the discipline in `docs/technical/testing.md` — including the mutations that
did *not* bite, which are evidence about the code's structure rather than a free pass.

## Existing Code to Leverage

| What | Location | Reuse |
|------|----------|-------|
| Port + fault shape (`Fault` struct, `Unreadable`/`Malformed` split, hand-written `Display`) | `crates/mc-core/src/block/source.rs`, `crates/mc-core/src/hud/source.rs` | pattern for the script fault type |
| Resolved-graph walk with vacuity guards and exact enum verdicts | `crates/mc-testkit/tests/workspace_layering.rs`, `crates/mc-world/tests/dependency_graph.rs` | mechanism for FR-8 |
| Harness-as-test-support with a guard that it re-implements no policy | `crates/mc-client/tests/support/input/`, `crates/mc-client/tests/seam_boundaries.rs` | placement and shape for FR-7 — **not** `mc-testkit`, which is forbidden from reaching what it verifies |
| `mlua` pinned at 0.12.0 with `luau` + `vendored` | root `Cargo.toml` `[workspace.dependencies]` | opt in with `mlua = { workspace = true }`; never version it in the member |
| Sibling `#[path = "x_test.rs"] mod tests;` unit-test convention | `docs/technical/testing.md` | keeps `src/` files under the 500-line gate ceiling and out of the coverage denominator |

**One existing test must change in this spec's diff.**
`crates/mc-testkit/tests/workspace_layering.rs` line 40 pairs `("mc-script", None)`
and asserts the verdict `ClosureIsTheCrateAlone`. Adding `mlua` turns it red — that
is its positive control working. The pair becomes `Some("mlua")` as part of this
change, not as an unexplained repair.

## Out of Scope

Binding. Recorded, not built.

- **Any block API or `mycraft.*` binding of any kind.** No block, world, entity or
  registry access from script. The host is exercised only through callbacks the
  harness registers.
- **Component attachment itself** — declaring that a component attaches to a block
  definition (PRO-919). This spec holds attachments as opaque identity pairs.
- **Hot reload**, the scratch-VM candidate build and the `ArcSwap` registry swap
  (PRO-918). `crates/mc-script/CLAUDE.md` invariants 6 and 7 belong to that spec.
- **The rule expression graph** (PRO-920).
- **The Luau-backed `DefinitionSource` adapter and retiring TOML** (PRO-917). The
  registry seam is untouched here.
- **Integration into `mc-sim`'s tick loop.** Nothing in `crates/` depends on
  `mc-script` at the end of this spec.
- **Per-mod CPU time accounting and exposure.**
- **Multi-file mods and `require` confinement** — see D1.
- **Raising these three lints to `deny` anywhere other than `crates/mc-script`**,
  whether per crate or in `[workspace.lints.clippy]`.
- **Any content authored in Luau under `content/base/`.** The four blocks stay TOML
  until PRO-917.
- **Preventing content from installing `__index` on tables the *scripting API*
  supplies.** There are none at this spec; the rule lands with the API that supplies
  them. Standard-library metatables are a different question and are covered — see
  FR-1.2-S3.

## Dependencies

- **ADR-003** (mlua 0.12, `luau` + `vendored`, `sandbox(true)`, `set_interrupt`,
  `set_memory_limit`, VM on the tick thread) and **ADR-004** (state in Rust,
  behaviour in script) are binding and not reopened here.
- **`crates/mc-script/CLAUDE.md`** invariants 1–4 are this spec's contract;
  invariants 5–8 belong to PRO-917 and PRO-918.
- **The composition model and API surface policy** landed in `c10fc99` and govern
  this spec: budgets, quarantine and per-cell state are per component, and the
  published scripting surface is exempt from "no abstraction before three concrete
  uses" — the filter is *"can this be abused, and is it bounded?"*.
- **`docs/modding/sandbox.md`** is the pre-committed consolidation destination per
  `docs/INDEX.md`'s routing guide; it does not exist yet and `/sdd-complete`
  creates it.

## Assumptions

- `crates/mc-script` is a three-line stub with an empty `[dependencies]` and `mlua`
  is absent from `Cargo.lock`. **Verified against the tree**, not inherited from the
  brief.
- `mlua = { version = "0.12.0", features = ["luau", "vendored"] }` is already pinned
  in `[workspace.dependencies]`, so no version decision is taken here.
- The Lua VM is `!Send` and stays that way; nothing in this spec moves it off a
  single thread, and mlua's `send` feature is not enabled.
- `mc-script` is not excluded from the gate's coverage denominator and has no
  GPU-style exemption available to it, so its library code counts against the 80 %
  line threshold from the first line written.
- No `mc-*` crate depends on `mc-script` when this spec completes; the host is
  exercised solely by its own tests and the harness.

## Open Questions

None. D6 (how many VMs) is named as an architecture driver for `/sdd-architect`
rather than an open question: it does not block, and every scenario above reads
identically under either answer.

## Clarifications

### Session 2026-08-15

- Q: `require` confined to the mod's directory (`crates/mc-script/CLAUDE.md`
  invariant 1) or absent entirely (PRO-916's description)? → A: Absent. No
  mod-directory concept exists yet; absence over-satisfies the invariant and
  confinement lands with PRO-917. (D1)
- Q: Does quarantine act on the component workspace-wide or on the
  `(subject, component)` attachment? → A: The attachment — the issue's own phrase is
  "mod B's broken attachment stops acting", and per-component state is already
  namespaced within a subject. (D2)
- Q: Is the fault threshold consecutive or cumulative, and what is N? → A: Three
  consecutive, reset by a success. (D3)
- Q: With no block API, what does a cascading callback cascade *through*? → A:
  Follow-up work returned as opaque attachment identities, queued and drained by a
  bounded round — never re-entered inline, because synchronous re-entry makes an
  unbounded cascade a Rust stack overflow. (D4)
