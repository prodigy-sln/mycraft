# Requirements — Luau host, sandbox and the hostile-mod harness (PRO-916)

Source: Linear PRO-916, MyCraft MVP 2: Scriptable Content. The issue description is
the brief and carries the substance; this file records what the codebase answered,
what had to be decided, and why.

## What the codebase answered

**`crates/mc-script` is genuinely from zero.** `src/lib.rs` is a three-line doc
comment, `Cargo.toml` has an empty `[dependencies]`, and `Cargo.lock` contains no
`mlua` and no `luau`. Verified, not assumed.

**`mlua` is already pinned in `[workspace.dependencies]`** at `0.12.0` with
`features = ["luau", "vendored"]`. There is no version decision to make here; the
member opts in with `mlua = { workspace = true }` (root `CLAUDE.md` forbids
versioning a dependency in a member crate).

**The registry seam already exists and this spec does not touch it.**
`mc_core::block::DefinitionSource` is the port, `mc_world::content::TomlFileDefinitionSource`
the adapter, `BlockRegistry::apply` the only door in. ADR-012 put the registry
contract in `mc-core` precisely so `mc-script` can populate it later. PRO-917 adds
a second adapter; PRO-916 adds none.

**The established port shape is trait + `Fault` struct + `Unreadable`/`Malformed`
error split**, with the fault carrying `origin` + which-thing + which-field +
cause. `DefinitionFault` and `HudFault` are the two instances. A script fault type
should read like them.

**There is no degradation vocabulary in Rust today.** "Degraded", "quarantine",
"budget", "component" appear only in prose (`crates/mc-script/CLAUDE.md`,
`content/CLAUDE.md`, `product/roadmap.md`). This spec invents it.

**The harness precedent is the client-input harness, not `mc-testkit`.**
`mc-testkit` is structurally forbidden from reaching the crates it verifies
(`crates/mc-testkit/tests/dependency_graph.rs`), and the input harness lives at
`crates/mc-client/tests/support/input/` for exactly that reason, with a text guard
(`crates/mc-client/tests/seam_boundaries.rs`) asserting it re-implements no policy.
A hostile-mod harness needs `mc-script` types, so it follows that precedent.

**Every structural scan in the tree is now per-root** — `no_hardcoded_block_names.rs`,
`no_hardcoded_hud_names.rs` and `license_declaration_consumers.rs` all carry
`MEMBER_ROOTS = ["crates", "tools"]` with a per-root non-zero file count, and the
gate's size stage carries `$SizeRoots = @('crates','tools')` and fails a root that
measured zero. This spec adds no tree-walking scan, so it inherits no root scope.
Its one new guard walks *cargo's resolved metadata*, where root scope does not
apply — but vacuity still does, hence FR-8.1-S2 and FR-8.1-S3.

## Decisions taken, with the alternative recorded

Each of these was decision-shaped and none of them blocks: proceeding under the
recorded assumption is safe, and the alternative is written down so it is not
re-derived.

### D1 — `require` is absent, not confined

`crates/mc-script/CLAUDE.md` invariant 1 says `require` "resolves only inside the
mod's own directory". The issue says "No io, os, require or loadstring".

**Absent.** There is no mod-directory concept at this spec — no mod loading, no
multi-file content — so there is nothing to confine `require` to. Absence is
strictly stronger than confinement and does not contradict the invariant; it
over-satisfies it. Confinement becomes real when PRO-917 makes a mod a directory,
and the invariant is waiting there for it.

### D2 — Quarantine granularity is the attachment, not the component

The issue says "quarantine the offending component"; it also says "mod B's broken
**attachment** stops acting". These differ when a component is attached to many
subjects.

**Per attachment — the `(subject, component)` pair.** Rationale: per-component
state is already namespaced by component id *within* a subject
(`crates/mc-script/CLAUDE.md`), so the pair is the unit that already exists; and a
component broken on one subject is not proven broken on another, so quarantining
globally punishes more than was caused.

The cost is honest and bounded: a component genuinely broken everywhere reaches
its threshold independently on each subject, so containment costs
`threshold × subjects` faults rather than `threshold`. That is finite, and each
fault is a caught error or a spent budget — both already bounded by FR-2 and FR-3.
The alternative (component-wide quarantine after the first attachment trips) is
cheaper to contain and worse to diagnose; if the fault volume ever matters,
promoting attachment-level quarantine to component-level is additive.

### D3 — Three *consecutive* faults, not three total

`crates/mc-script/CLAUDE.md` invariant 4 says "N consecutive failures". Concrete
value: **3**, and the count resets on a successful invocation.

Consecutive rather than cumulative because a callback that fails on a rare input
and succeeds otherwise is doing useful work, and cumulative counting eventually
quarantines every long-lived callback in the game. The residual — a callback
alternating fail/succeed forever, never quarantined — is accepted, because its
*cost* is already bounded by the per-invocation budget (FR-2) whether or not it is
quarantined. Quarantine bounds repetition and log noise; the budget bounds cost.
Those are two jobs and only one of them is load-bearing for invariant 3.

### D4 — Dispatch is never re-entrant; follow-up work is queued

The cascade case needs a callback to be able to cause further callbacks, and there
is no block API to cause them with.

**A callback returns follow-up work as opaque `(subject, component)` identities;
the host queues it and drains the queue in the same round, bounded by a total
invocation count.** A callback never synchronously re-enters dispatch.

This is the load-bearing half of the decision. If a callback could re-enter
dispatch synchronously, an unbounded cascade would grow the *Rust* stack, and a
stack overflow is an abort — precisely the outcome invariant 3 forbids, reached by
the mechanism a depth counter is least likely to catch first. Queueing converts
recursion depth into queue length, which is countable, boundable and observable.

Rejected alternative: a test-only re-entrant binding supplied by the harness. That
would test a fixture rather than production dispatch, and the containment claim
would be about code the server never runs. A bounded work queue is also exactly
what PRO-919's neighbour notification needs, so it satisfies the MVP 2 constraint
that nothing is built which a later spec deletes.

**The queue is bounded in two dimensions, not one — `round_bound` and `pending_bound`.**
Recorded here because the decision above is incomplete without it: bounding
*invocations per round* says nothing about *queue length*, and a callback returning a
fan-out of follow-up identities grows the queue by more than a round drains, across
unbounded rounds. Every entry is a host-side allocation, so it sits outside the script
memory cap, outside the budget and outside quarantine — the requester succeeds every
time. "Queueing converts recursion depth into queue length, which is countable and
boundable" is only true once something actually bounds it. `pending_bound` is that
bound, and an entry that would exceed it is **refused** rather than dropped quietly.

Refusal and deferral are reported as **two different faults**. They are not the same
event: deferred work runs next round, refused work never runs. The consumer named above
makes the difference concrete — a full queue drops a neighbour notification, so a block
never learns its neighbour changed, and the symptom is content that quietly does nothing
rather than a server that is visibly slow.

### D5 — Limits are configurable with documented defaults

Every limit (call-and-loop budget, memory cap, fault threshold, round invocation
bound) is configurable, with a default the host documents.

Not flexibility for its own sake: a memory cap sized for a real server cannot be
tripped inside the one-second integration-test budget, and a fault threshold that
cannot be lowered makes every quarantine test spend three real faults. The
scenarios below configure small values; the defaults are what ships. Confirming
the default *values* against measurement is the architect's, not the spec's.

### D6 — How many VMs is an architecture question, deliberately left open

FR-3 says "the VM's memory limit" without saying how many VMs exist. One VM shared
by all mods is simpler and makes cross-mod interop trivial; one VM per mod isolates
the memory cap so that one mod cannot exhaust the allowance every other mod is
sharing — which is the same argument that made budgets per component rather than
per block.

This is named as an architecture driver rather than decided here because it turns
on facts a spec does not hold (per-VM fixed overhead in `mlua`'s Luau build, and
whether the reload path in ADR-004 wants to discard one VM or all of them). It
does not block: every FR-3 scenario is stated at the boundary a script observes,
and reads the same under either answer.

### D7 — The per-crate lint table is in scope

`crates/mc-script/CLAUDE.md` invariant 4 claims `unwrap`/`expect`/`panic!` "are
lint-denied here". They are not: the workspace lint table sets them to `warn` and
every member carries exactly `[lints] workspace = true`. The gate's `-D warnings`
promotes them, so the practical effect matches today — but the claim as written is
false and this is the spec that creates the code it governs.

`crates/mc-script/Cargo.toml` gains the workspace's first per-crate
`[lints.clippy]` table setting those three to `deny`, so the invariant is true at
`cargo check` and not only under the gate. Recorded here rather than as a
functional requirement because it has no observable behaviour of its own.

### D8 — Fault attribution is in scope; CPU accounting is not

Every fault names its component — that is required by the scenarios and is what
makes "which mod is eating the tick" answerable later. Actually *measuring and
exposing* per-mod CPU time (`crates/mc-script/CLAUDE.md`, performance rules) needs
a tick to attribute against, and `mc-sim` does not call `mc-script` at the end of
this spec. Out of scope, recorded.

## Contradictions between the tree and its own documents

Found during discovery, reported rather than fixed except where noted.

1. **`crates/mc-testkit/tests/workspace_layering.rs` asserts `mc-script` has no
   dependencies.** `INSPECTED[2] = ("mc-script", None)` expects the verdict
   `ClosureIsTheCrateAlone`. Adding `mlua` turns it red. This is the positive
   control working as designed, and updating the pair to `Some("mlua")` is part of
   this spec's change, not a surprise for the implementer.
2. **`crates/mc-script/CLAUDE.md` invariant 4's lint claim is false today** — see D7,
   which brings it into scope.
3. **`crates/mc-testkit/src/lib.rs` advertises "scripted bot clients, determinism
   and load fixtures".** Only `pub mod frame;` exists. Not this spec's to fix;
   noted because a reader looking for a harness base will find the claim first.
4. **`docs/INDEX.md` routes sandbox limits, instruction budgets and fault isolation
   to `docs/modding/sandbox.md`, which does not exist.** That is the pre-committed
   destination for this spec's consolidation, and `/sdd-complete` creates it. **The
   routing row's own wording is now wrong as well**, quoted above exactly as
   `docs/INDEX.md:83` has it: there is no instruction budget, because the Luau interrupt
   fires on calls and loop edges rather than per instruction. This spec renamed the limit
   throughout its own documents; the routing table is in `docs/`, so it is corrected when
   the consolidation lands rather than here.
5. **Root `CLAUDE.md`'s tech-stack table still lists `redb`/`bincode` for storage**
   after ADR-016 made the save a `postcard` plain file. Cosmetic, unrelated, and
   the same table carries the Luau row — noted so it is not read as current.

## Scenario audit — 2026-08-15

`sdd-scenario-auditor` ran against the 33-scenario draft and returned **15 gaps and 2
contradictions**. All 15 drafts and both resolutions were accepted, with two
adjustments, taking the spec to **46 scenarios**.

**The five that would have shipped a real hole:**

1. **A protected call defeats every limit.** `while true do pcall(f) end` — an
   interrupt-raised abort and an allocation failure both surface to Lua as catchable
   errors, so a hostile mod could burn its budget, catch the abort and re-enter,
   forever. Every one of the 33 draft scenarios stayed green through that. Now
   FR-2.1-S5 and FR-3.1-S4, and the latching requirement is written into Technical
   Considerations.
2. **The harness's own verdict contradicted itself for two of the six cases.**
   FR-7.1-S3 defined uncontained as "no fault or quarantine reported", but
   `sandbox-escape` and `hostile-index` are contained precisely by producing neither.
   Each case now declares the evidence its containment requires (FR-7.1-S4).
3. **Budget and allocation aborts were not stated to count toward quarantine.** If
   they do not, the `infinite-loop` mod is never quarantined and burns a full budget
   on every attachment every round forever — bounded per invocation, unbounded in
   aggregate, and the harness would still report it contained. FR-4.2-S5 settles it:
   they count, and three faults of three different kinds still quarantine.
4. **Two absence assertions the spec's own preamble promised to pair, and had not.**
   FR-6.1-S1 ("the metamethod was not invoked") and FR-4.1-S2 could both have been
   green because the probe never fired. FR-6.1-S4 and FR-4.1-S4 are the controls.
   FR-4.1-S2's original construction — a `__tostring` that loops forever — was also
   wrong-reason-passable: with the interrupt in place, a host that *did* invoke it
   would have the metamethod aborted by the budget and still report a fault. A
   counter probe replaces it.
5. **No limit had a shipped default under test.** Every scenario configures its own
   small value, so a host whose defaults are absent, zero or unlimited passed all of
   them. FR-9 is new.

**Also accepted:** `rawset` on the global table (FR-1.1-S4) — a frozen environment
built on `__newindex` is defeated by it and one built on a readonly table is not, so
the scenario is what decides which claim the spec makes; `getfenv`/`setfenv`/
`collectgarbage`/`newproxy` added to the denied set (FR-1.2-S4); `pcall`/`select`/
`type` stated in the permitted set; quarantine asserted to *persist* rather than to
happen once (FR-4.2-S6); follow-up work naming a quarantined attachment (FR-5.1-S5,
the queue drain being a second entry point onto dispatch that no FR-4.2 scenario
reached); concrete literals for every limit; a derived oracle for the cascade size
(200 over 4 rounds with a bound of 64, all literals) rather than a number the host
reports back to itself; an observable for FR-6.1-S3; an ordered record for FR-5.1-S1;
and the four minor wording fixes.

**Two adjustments to what the audit proposed:**

- Its FR-5.1-S5 (a cascade of exactly 64 at a bound of 64) subsumes the draft's S4 (a
  cascade comfortably under the bound) — same assertion at the tighter point. Merged
  into one scenario rather than shipping two near-duplicate tests.
- Its gap 7 asked for a `WHERE`-form default scenario for each of the four limits.
  Reduced to FR-9's two: one enumerated verdict covering all four defaults as finite
  and non-zero, and one proving a default is actually wired to enforcement rather
  than merely reported. Four near-duplicates of existing scenarios would have cost
  more than the hole is worth.

**Rejected:** nothing.

**Later count.** 46 is the audit's outcome, not the current total. Challenging the spec
against the architecture added ten more, for mechanisms the design had acquired that
nothing could show working: host memory pressure and its reconciliation with FR-4.2-S5,
the pending queue's refusal path, `release` and the two `attach`-over-an-existing-
attachment cases, fault attribution to the defining chunk, the permitted set as an
exact-set assertion, `print`'s routing through the host, and an allocation fault's cause
being non-empty. **The spec stands at 56 scenarios** — FR-1.1:4 FR-1.2:6 FR-2.1:5
FR-3.1:6 FR-4.1:5 FR-4.2:10 FR-5.1:6 FR-6.1:4 FR-7.1:5 FR-8.1:3 FR-9.1:2.

## Out of scope, confirmed against the issue

Any block API or `mycraft.*` binding of any kind · component attachment itself
(PRO-919) · hot reload and the scratch-VM candidate build (PRO-918) · the rule
expression graph (PRO-920) · the Luau-backed `DefinitionSource` adapter and
retiring TOML (PRO-917) · per-mod CPU accounting · multi-file mods and `require`
confinement · integration into `mc-sim`'s tick · any content authored in Luau.

The issue names the shape of this spec as deliberate: "A Luau host with nothing
exposed is a layer — accepted anyway, because the alternative is shipping the API
and hardening afterwards, which is the sentence invariant 3 exists to forbid." If a
reviewer objects to the shape, the answer is the invariant, not a redesign.
