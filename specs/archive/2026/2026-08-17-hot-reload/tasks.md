# Tasks: Hot reload — edit a block definition on a running server and see it change

**Spec**: [spec.md](spec.md) (SPEC-017, rigor `high`, **91 scenarios**) ·
**Architecture**: [architecture.md](architecture.md) (binding, 15 decisions) ·
**Requirements record**: [requirements.md](requirements.md) ·
**Branch**: `feature/PRO-918-hot-reload` · **Issue**: PRO-918 ·
**Created**: 2026-08-17

One task = one coherent scenario group in one area. Phases are the
architecture's: **26 · 15 · 26 · 12 · 7 · 5 = 91**. `[P]` = independent of other
`[P]` tasks in the same phase. **26 tasks.**

Every `Scenarios:` line below carries full scenario IDs, comma-separated, with no
ranges — so a mechanical check can expand the whole breakdown without parsing
prose. The phase table's ranges are a reader's summary; the task lines are the
record.

| Phase | Tasks | Scenarios | What it delivers |
|---|---|---|---|
| **1** — A candidate is applied at a tick boundary, or nothing is | T01–T08 | 26 | `World::adopt`, the `world::reload` child module, admission, `Simulation::adopt`, the drive through `Session` |
| **2** — Layers are appended, never renumbered, and the content is published | T09–T12 | 15 | `LayerAssignment`, the budget constant, `load`'s new parameter, `PublishedContent` and `ContentSerial` |
| **3** — A saved edit becomes one attempt, built off the tick thread, and a refusal is stated once | T13–T16 | 26 | the `ContentWatch` port and adapter, `ContentReload`, `ReloadRefusal` and its dedup |
| **4** — What a reload re-meshes, and against which content | T17–T19 | 12 | the geometry predicate, whole-world marking, the batch's registry, `Superseded`, the layer upload |
| **5** — A player inside a cell that became solid is moved clear | T20–T21 | 7 | `cleared`, wired into the accepted path and reported |
| **6** — The seam stays cut, the window is declared once, and the pages are true | T22–T26 | 5 | two needles and their fixtures, the capture-pipeline scan, the window scan, the documentation deliverable |

---

## Read this before implementing anything

### The test author is fresh per phase, and owns that phase's tests

At rigor `high` a phase's tests are authored by a test author that has not seen
any implementation and owns them for the whole phase. **The implementation
context never edits a test file, and there is no mechanical-change exemption —
not for a rename, not for a doc comment, not for a formatting fix.** Disputed
failures go to the test author with exactly one verdict:

- `test-correct` — the implementation conforms; the implementer fixes it.
- `test-wrong` — the author fixes and commits it.
- `scenario-ambiguous` — **this goes to the team lead, not back to the
  implementer.** It is a spec defect and the ruling is theirs.

### Staging, and the two rules that have already cost this project work

- **Explicit paths only.** `git add -A`, `git add .` and `git commit -a` are
  banned, with no exception for "the tree is clean, I checked" — a sweep once
  pulled a test author's in-flight file into an implementation commit. Never run
  `cargo fmt`.
- **Revert a mutation check by hand** — re-edit the line you broke. Never
  `git checkout -- <file>`: it once wiped an uncommitted implementation. Confirm
  with `git diff --exit-code` before continuing.

### Every FR-3, FR-4 and FR-5 test drives through `Session`, never through `Simulation::adopt`

This is the architecture's own Risks entry and it is the shape of the two
precedents `testing.md` §2 records — a client submitting a default intent every
tick left 406 of 406 green, and deleting the free-cursor guard left the same.
**A test that calls `Simulation::adopt` itself is agreement between two callers
of one function**: the `Session` drive can stop calling `at_tick_boundary`
entirely and every such test stays green. Driving through `Session` is what makes
26 scenarios redden if it does.

### A green suite is no evidence about a lint, and phase 2 opens with a window where nothing compiles

`load` gains a parameter (D9) and `Simulation::new` gains a third (D12). Both
touch every construction site in the workspace, so phase 2 opens with an
adaptation commit and **no compilable tree for the gate to run on** until the
implementation lands. Anything only the gate can see accumulates silently across
that window. Whoever authors tests inside it runs
`cargo clippy --workspace --all-targets --all-features -- -D warnings` directly.
Checking at a lower severity asks a different question, and without `-D warnings`
cargo marks repeats `(1 duplicate)` — which means *this same diagnostic, again*,
not *a pre-existing one lives elsewhere*.

---

## The sequencing traps, named out loud

The last two specs each found a phase whose scenarios would have arrived green if
an earlier phase took a shortcut, and both were caught because the task file said
so in advance. These are this spec's.

### Trap 1 — Phase 1 is barred from publishing, from layers, and from marking anything dirty

**Binding, and it is the architecture's own sequencing constraint.** Phase 1 must
**not** touch the layer assignment, must **not** publish content, and must
**not** mark any section for re-meshing. Three specific moves spring it, and each
is three lines away from code phase 1 is legitimately writing:

| The tempting move | Where it sits | What it hands away |
|---|---|---|
| Adding a `content: ArcSwap<PublishedContent>` field and a `Simulation::content()` "while the seam is open" | `crates/mc-sim/src/simulation.rs`, beside the `ArcSwap<SimSnapshot>` that is already there | phase 2's **FR-8.1-S1, FR-8.1-S2, FR-8.1-S4, FR-8.1-S5** — four of fifteen |
| Giving `load` its `spent` parameter, or touching `resolved_from`'s derivation | `crates/mc-sim/src/content.rs:151-155` | phase 2's **FR-5.1-S1, FR-5.1-S2, FR-5.1-S3** |
| Marking sections in `adopt` "so the picture is right" | `World::mark_dirty` at `crates/mc-sim/src/world/mod.rs:249`, one file away from `adopt` | phase 4's **FR-6.1-S2, FR-6.1-S3**, and **FR-4.1-S3** |

Phase 1 calls `mc_sim::content::load(root)` with its **current** signature and
uses the `LoadedContent` it gets back for its registry and nothing else. This is
`testing.md` §1's "implement deliberately less first". It is not sloppiness
deferred; it is the only way phases 2 and 4 can ever be red.

**How phase 2 detects the trap was sprung:** FR-8.1-S1 or FR-8.1-S4 green when
the phase opens. **How phase 4 detects it:** FR-6.1-S2 or FR-6.1-S3 green when
the phase opens.

> **FR-6.1-S1 is not a signal and must not be read as one.** It is green on
> arrival *legitimately* — phases 1–3 mark no section on a reload, so "a
> candidate changing nothing geometric meshes no section" is satisfied by an
> implementation that never meshes. SPEC-016's breakdown made exactly this
> mistake in the other direction: it named FR-1.2-S4 as a trap signal, and
> FR-1.2-S4 could not discriminate between the two implementations, so reading
> its greenness would have sent a reader to reopen a sound phase. **Verify phase
> 1 directly instead** — read `Simulation` for a second `ArcSwap` and
> `world::reload` for a call to `mark_dirty` — which is the order of evidence
> this check should ask for.

### Trap 2 — Phase 2 is where `PublishedContent` is born, and its `hud` field has exactly one scenario holding it up

Nothing else in phase 2 needs the HUD. `FR-4.4-S2`, `FR-5.*` and `FR-8.1-*` are
all satisfied by a `PublishedContent { serial, resolved }`, and the field feels
like phase 4's because phase 4 is where `App` assigns the layout. **Deferring it
is a Blocker, not a scheduling choice**: `mc_sim::content::load` refuses blocks
and HUD together (FR-2.1-S7), so applying the blocks while leaving the HUD behind
*is* the partial application `crates/mc-script/CLAUDE.md` invariant 7 forbids.

FR-4.5-S1 is the scenario the architecture stage promoted out of an assumption
for exactly this reason, and **it is what stops a later reader deleting the field
as unused.** If phase 2 closes at 14 of 15 with FR-4.5-S1 pushed forward, the
phase is reopened rather than accepted.

### Trap 3 — Phase 2 must not upload anything

The layers are right there once `LayerAssignment` exists, and `TextureLayers` is
one call away. Wiring the upload in phase 2 hands phase 4 **FR-4.3-S2** and
**FR-4.3-S3**. Phase 2 publishes a value; phase 4 is what makes a device see it.

### Trap 4 — Phase 3's build must go through `mc_sim::content::load` and nothing else

The build stage needs **no new fault vocabulary at all** (D4) — every refusal in
its row already arrives as `ContentError` carrying
`DefinitionFault { origin, block, field, cause }` underneath. So: **if T16 finds
itself writing a new fault type for a compile error, a misspelled field, two
files claiming one name or an emptied directory, that is the signal the reload is
reaching content through something other than `load`** — not a licence to write
the type.

FR-2.1-S5, FR-2.1-S6, FR-2.1-S7, FR-2.3-S1 and FR-2.3-S2 are inherited behaviour
and are **red on arrival** only because no reload path exists. Their value is as
controls: they redden exactly if the build goes round `load`.

### Trap 5 — FR-1.1-S4 needs two instruments, and neither covers the other

D10, and it must be carried here because the first architecture draft got it
wrong. One tick per rendered frame, so tick boundaries are roughly 16 ms apart;
the settling window is 150 ms, about nine boundaries. **Five writes genuinely
spread across one window reach five different boundaries and begin five attempts
unless the debouncer absorbed them first.** The domain's coalescing only collapses
reports arriving between two ticks.

1. **The coalescing test**, driven through the in-memory double: reports
   delivered between two boundaries begin one attempt. This is a real assertion
   about `ContentReload` and not agreement with the double, because the double
   holds no policy of its own.
2. **The window's value at the boundary it crosses**: a test asserting that the
   `Duration` handed to the debouncer builder *is* `SETTLING_WINDOW`. Without it,
   passing `Duration::ZERO` leaves the constant declared once (FR-9.1-S2 green),
   the coalescing test green, and the shipped client beginning one attempt per
   filesystem event. No filesystem, no timer.

**Neither covers the other.** Both are owed in phase 3.

### Trap 6 — the count a test author will derive from FR-6.1-S2's wording, and why it is wrong

This paragraph is lifted verbatim from architecture D7, which marks it as
required here.

> **This paragraph must appear in `tasks.md`, and it names a victim and a wrong
> fix.** FR-6.1-S2's wording — "mesh again every section whose own or whose
> neighbours' blocks include stone" — is a **lower bound**, not a count. A test
> author who derives an expected number from it writes `assert_eq!(keys, 82)`,
> and that assertion **reddens against a conforming implementation**. It is the
> over-tight assertion `testing.md` §2 names — *red that should have been green*
> — and its cheapest repair is to narrow the marking rule, which breaks D7 and
> silently fails FR-6.1-S3 in the same commit. **The expected value for S2 on
> the shared instrument is 256.** What S2 grades is that the set *contains* the
> stone-bearing sections and their neighbours; what tells S2 from S1 is that one
> meshes something and the other nothing, which is FR-6.1-S4's paired control.

The measurement behind it, verified in the tree at this branch's head:
`LOWEST_SURFACE = 32` and `HIGHEST_SURFACE = 48`
(`crates/mc-sim/src/replay/height.rs:21-22`), sea to 34, `LANDMARK_TOP = 64`
(`crates/mc-sim/src/replay/world.rs:40`), `SECTIONS_PER_COLUMN = 16`
(`crates/mc-world/src/column.rs:17`), `FOOTPRINT_COLUMNS = 4`
(`crates/mc-sim/src/replay/world.rs:27`). **The highest occupied section is 3 in
fifteen of the sixteen columns and 4 in one**, and everything above holds no
block at all. That is the measurement forcing the binary rule, and narrowing it
later is a change to FR-6.1-S3 and to the spec's Technical Considerations, not an
optimisation somebody may take while passing.

### Trap 7 — phase 4's staleness comparison must not live on the worker

`Retained` is *moved into* the worker thread at `Remesher::spawn`
(`crates/mc-client/src/remesh.rs`), so a serial held beside the retained layers
is only updated when the worker dequeues a retirement message. On one ordered
channel **the mismatch branch is then unreachable in production** — every batch
the worker dequeues carries the serial it currently holds — so the `Superseded`
variant would exist only for an artificial test to construct, which is
`testing.md` §2's definition of a test that cannot fail for a real reason. The
comparison runs on the client at collect time, where "serving now" is known.

### Trap 8 — two gate limits are already within reach

Measured at this branch's head with the gate's own counter (non-blank lines, 500
limit): `crates/mc-client/src/session.rs` is at **492** — eight lines of
headroom — and `crates/mc-client/src/app.rs` at **456**. D1's residue in
`mc-client` is 60–120 lines at this codebase's doc-comment density. **D14
pre-commits `crates/mc-client/src/reload.rs` as a new sibling of `remesh.rs`**
rather than letting someone discover 501 lines mid-phase and restructure to
satisfy a count. Whoever adds the drive to `Session` (T15) re-measures **before**
writing, not after.

### Trap 9 — the golden frames must be run after the assignment change alone

`prepare_scene` is what every golden is shot through, and D9 changes how its
layers are produced. `crates/mc-client/tests/launch_and_capture_agree.rs` and
both golden suites run **after T10 and before anything else in phase 2**.
Running them once at the end of the phase cannot tell a wiring defect from an
assignment defect.

### Trap 10 — one inherited test expires, and greening it early destroys its signal

SPEC-016's pin on the name-for-texture substitution turns red when PRO-902 closes
the gap, and **that red is its success signal.** Nothing in this spec may green
it early. FR-4.3-S2 declares `texture` equal to `name` for exactly that reason,
and an implementer who "fixes" `layer_for` to resolve through the registry while
passing has destroyed the only instrument that will announce PRO-902 landed.

---

## Phase 1 — A candidate is applied at a tick boundary, or nothing is

**26 scenarios.** Done means: a `LoadedContent` handed to `Simulation::adopt` at a
tick boundary either replaces the registry and the solidity view together and
re-derives the held block, or is refused having changed nothing at all — and the
world, the player, the tick counter and the save cross it untouched.

**Red on arrival because nothing swaps.** The skeleton that reddens this phase is
an **accepting no-op** — `adopt` returns `Ok` and changes nothing. It leaves the
eight controls green (see the table below) and reddens the other eighteen. A
refuse-everything skeleton is the wrong one here: it would pass FR-2.2-S1,
FR-3.3-S3 and FR-4.1-S5 for the wrong reason.

- [x] **T01** The names the world actually holds, and the refusal that lists them
      — `crates/mc-world/src/section/mod.rs`,
      `crates/mc-sim/src/world/mod.rs`,
      `crates/mc-sim/src/reload/mod.rs` (new)
      Scenarios: FR-2.2-S1, FR-2.2-S2, FR-2.2-S3
      - D13: `Section::names_in_use` is built on
        `Palette::surviving_entries` (`crates/mc-world/src/section/palette.rs:105`,
        currently `pub(super)` and returning palette **indices**), which answers
        from the reference counts the write path maintains in O(palette length).
        `SectionData::compacted()` is deliberately not reused — it allocates
        4 096 indices per section to answer the same question.
      - **Binding fixture constraint, and no assertion can enforce it.**
        FR-2.2-S2 needs a world in which no cell holds `base:water`. The shipped
        world fills sea to level 34, so water *is* placed. **The fixture must
        reach its no-water world by breaking the water out of a world that held
        it**, not by generating a world that never had any. A world that never
        had water leaves `surviving_entries` and `palette()` indistinguishable,
        and the scenario stops being the thing that catches a reload refused over
        a block the player broke ten minutes ago. This is `testing.md` §2's "a
        count cannot see shape": it is held by the code that builds the fixture
        and by a reviewer reading it.
      - `World::names_held(&self) -> BTreeSet<&BlockName>` over the world's
        columns. FR-2.2-S3 wants **every** such block, ascending — `base:grass`
        before `base:stone`, rather than whichever was found first.
        `SolidVoxels::resolve` reports only the first and is not the instrument.
      - `ReloadRefusal` is created here with **two variants only**:
        `BlocksTheWorldHolds` and `NothingToPlace` (T03). `Content`,
        `LayerBudget` and `BuilderLost` belong to phases 2 and 3 and adding them
        early would need `LAYERS_A_SESSION_MAY_ASSIGN`, which is phase 2's.
      - `NothingToPlace`'s sentence is `PreparationError::NothingToPlace`'s,
        unchanged to the byte. Two wordings are two places to disagree.

- [x] **T02** `World::adopt` — the second write door, with solidity settled first
      — `crates/mc-sim/src/world/mod.rs` (and its module header),
      `crates/mc-sim/src/world/reload.rs` (new)
      Scenarios: FR-4.1-S1, FR-4.1-S2, FR-4.1-S4, FR-4.1-S5
      Depends on: T01
      - D3 is binding and has three properties, each mirroring `World::write`.
        **Solidity is settled before either write** — `SolidVoxels::resolve` runs
        first and its `?` is what makes a refusal leave the world untouched
        *by construction* rather than by care. **`adopt` carries no `pub`**: the
        admission lives in `pub(crate) mod reload;`, a **child** of `world`,
        exactly as `pub(crate) mod action;` already is at `world/mod.rs:29`.
        Module visibility and item visibility are independent — `simulation` can
        reach the module while `adopt` stays invisible to it. **Nothing
        recomputes solidity separately.**
      - **A `pub(crate) fn adopt` would open a crate-wide write door and every
        test would stay green.** The instrument is the compiler, and a reviewer
        reading the header. There is no behavioural test for it.
      - **FR-4.1-S4 is the only instrument in the tree for "the two views state
        one answer."** The replay's overlap oracle cannot see a registry swapped
        without its bitset, because that oracle re-reads the world through the
        registry and would be agreeing with itself. S4 asks the physics view and
        a placement's occupancy check the same question and compares the answers.
      - **The module header's claim that "exactly one function writes anything"
        becomes false and must be rewritten, not left standing.** Two functions
        write; each settles solidity before either write; both keep no `pub`.
      - **Measured, not reasoned** (architecture Risks, phase 1):
        `SolidVoxels::resolve` walks 1 048 576 voxels on the tick thread at every
        accepted reload. It is the same call `World::new` already makes at
        launch, so the figure exists — **take it before deciding anything** and
        send it to the FR-9 benchmark either way. If it is a material term in the
        one-second budget, D3's deferred section-skip is the named answer; it is
        DEFERRED and is not taken here.

- [x] **T03** The block in the player's hand is re-derived, and a candidate with
      nothing to place is refused — `crates/mc-sim/src/world/reload.rs`,
      `crates/mc-client/src/session.rs`
      Scenarios: FR-3.3-S1, FR-3.3-S2, FR-3.3-S3, FR-3.3-S4
      Depends on: T02
      - `default_held_block` (`crates/mc-sim/src/world/action/mod.rs:324`) is
        reused, never reimplemented. Decision 6: the held block is a **policy
        over the registry**, not something the player accumulated, so it is
        re-derived. Preserving it would make a newly declared block unreachable
        until the next launch, which is the opposite of what hot reload is for.
      - **`Session` writes the re-derived `holding` back into its own field.**
        Nothing else does, and **FR-3.3-S2 and FR-3.3-S4 both fail without it.**
        This is "policy is not wiring" in miniature: the re-derivation can be
        perfectly correct inside `mc-sim` and never reach the player.
      - FR-3.3-S4 is the second witness on that path and it earns its place: it
        drives a *placement* of `base:amber` through the action path rather than
        re-reading `held_block()`, so a `Session` that displays the new block but
        hands the old one to the placement reddens here and nowhere else.
      - FR-3.3-S1 is a control (see the table): a swap that does nothing leaves
        the player holding the first solid block.

- [x] **T04** `Simulation::adopt` — the swap runs after the tick was published
      — `crates/mc-sim/src/simulation.rs`, `crates/mc-client/src/session.rs`
      Scenarios: FR-1.3-S1, FR-3.2-S2
      Depends on: T02
      - The seam splits where the borrow does (D3): `mc_sim::world::reload` takes
        only a `&mut World`; `Simulation::adopt` calls it and then does what needs
        the player and the publication. `Simulation`'s fields are private to
        `mc_sim::simulation` and `world::reload` is a descendant of `world`, so
        the other shape does not compile — recorded because the first
        architecture draft tried it.
      - **The swap runs after `Simulation::advance` has published its tick**, so
        the change is in force from the next tick. FR-1.3-S1 is the sharpest
        instrument this phase has on "between two ticks, never during one": a
        break of stone submitted on the earlier tick succeeds and one submitted
        on the later is refused as indestructible. A swap taken mid-tick makes
        one of the two wrong.
      - FR-3.2-S2 wants the later tick to be **exactly** the earlier one plus
        one, with the candidate's change in force on that later tick. The second
        half is what stops a skipped or doubled tick satisfying it.
      - **No `content` field, no serial, no publication here** — see Trap 1.

- [x] **T05** [P] The world and the player cross the swap unchanged, and the
      control that proves the swap happened —
      `crates/mc-client/tests/`, `crates/mc-sim/tests/`
      Scenarios: FR-3.1-S1, FR-3.1-S2, FR-3.1-S3, FR-3.2-S1, FR-3.2-S3
      Depends on: T04
      - **FR-3.1-S3 is the paired control the spec's own scenario audit added**,
        and without it FR-3.1-S1 and FR-3.1-S2 are satisfied by a reload that
        never happened. Throughout FR-3 the accepted candidate declares
        `base:stone` **non-solid**, and that effect is observed **in the same
        test**. A test that asserts only "everything is as it was" is evidence of
        nothing here.
      - **FR-3.2-S1 and FR-3.2-S3 are judged against an independent run of the
        same ticks, never against a value copied from a green run.** The oracle
        is a second simulation advanced the same number of ticks with the same
        inputs and no reload in flight. The spec's audit already repaired S1 once
        for demanding a position "exactly as it was" across a tick that applies
        gravity — that wording would have failed against a *correct*
        implementation and its cheapest repair was to freeze the tick.
      - FR-3.2-S3 is about a player who was **not** cleared. Phase 5 zeroes
        velocity on a clearing move; this test must not overlap that path, and
        its mutation (below) is what proves the two do not collide.

- [x] **T06** [P] The mutation rules cross the swap, and nothing the author did
      not edit moves — `crates/mc-client/tests/`, `crates/mc-sim/tests/`
      Scenarios: FR-4.2-S1, FR-4.2-S2, FR-4.2-S3, FR-4.2-S4, FR-4.4-S1
      Depends on: T04
      - The three mutation rules are what is observable in this increment
        (requirements §Decision 3): `breakable = false` refuses a break as
        indestructible; a `breaks_into` leaves the named block behind;
        `replaceable` lets a placement overwrite.
      - **FR-4.2-S4 pins an existing contract and must not be tightened.** A
        `breaks_into` naming `base:mithril`, which no declaration declares, is
        **accepted**. `docs/modding/blocks-items.md` states the late-resolution
        rule in as many words and gives its reason — it is what lets two mods
        name each other's blocks. The scenario audit proposed refusing it and the
        proposal was **rejected**; adding a cross-reference pass here breaks a
        documented promise inside a spec that is not about it.
      - FR-4.4-S1 is a control: a candidate changing only `blocks/stone.luau`
        leaves the other three blocks' declared fields exactly as they were.

- [x] **T07** [P] A save written after a reload records what the blocks are now
      — `crates/mc-client/tests/`
      Scenarios: FR-3.4-S1, FR-3.4-S2
      Depends on: T04
      - **The save format is untouched and that is the point.** A save written
        after a reload records `behaviour_of`/`appearance_of` of whatever
        registry the world then holds, so FR-3.4-S1's silent resume is a property
        of the swap having reached the world rather than of anything new on disk.
      - **FR-3.4-S1 is the discriminating end-to-end oracle this spec has** and
        FR-3.4-S2 is its control. S2 is green on arrival — it is the existing
        `Acceptance`/`RegistryVerdict` path and needs no reload machinery at all.
        Without S2 running and reddening under mutation, S1's silent resume is
        evidence that nothing is ever compared.

- [x] **T08** [P] A block declared for the first time is registered and answers
      for its fields — `crates/mc-sim/tests/`, `crates/mc-client/tests/`
      Scenarios: FR-4.3-S1
      Depends on: T04
      - Registration and field answers **only**. The appended layer is
        FR-4.3-S2's and the held-block indicator is FR-4.3-S3's, both phase 4 —
        see Trap 3.
      - `blocks/amber.luau` declares `texture = "base:amber"`, equal to its name.
        See Trap 10 for why that is not an oversight.

---

## Phase 2 — Layers are appended, never renumbered, and the content is published

**15 scenarios.** Done means: a texture key holds its layer for the session, the
budget is spent only by an accepted candidate and only monotonically, and an
accepted reload publishes a resolved content set, its HUD and a serial that a
reader observes by asking.

**Red on arrival** because phase 1 leaves the layer assignment exactly as a
launch produced it and publishes nothing — so a reload gives a new key no layer,
moves no serial and carries no HUD. If FR-8.1-S1 or FR-8.1-S4 is green when this
phase opens, phase 1 took Trap 1 and the phase is reopened rather than accepted.

- [x] **T09** The session's layer budget and the assignment that spends it —
      `crates/mc-core/src/content.rs`,
      `crates/mc-render/src/geometry/vertex.rs`
      Scenarios: FR-5.1-S1, FR-5.1-S2, FR-5.1-S3, FR-5.2-S2, FR-5.2-S3
      - **D6's failed reasoning, recorded so it is not re-derived.** The first
        draft made the assignment monotone by *keeping retired keys in it*, so
        the next free layer would be `assignment.len()`. That fails **FR-5.1-S3**
        outright: a key removed and reintroduced is still "already held", so it
        gets back the layer it had — the one outcome the scenario forbids. So
        `live: Vec<(TextureKey, u16)>` and `spent: u16`, with **`spent` a primary
        field rather than a derived one** — `live.len()` is what would be wrong,
        because a retired layer is spent and is not live.
      - `appending(keys)` is **all-or-nothing**: FR-5.2-S3 has 255 assigned and
        two keys needed, and requires that **neither** is appended rather than
        the one that fits. An implementation that appends greedily and refuses at
        the end passes FR-5.2-S1 and fails this.
      - **D5, and it is where the 256 comes from.** `LAYERS_A_SESSION_MAY_ASSIGN`
        is declared in `mc-core::content` — beside the value whose layers it
        bounds — and `mc-render` asserts equality at compile time
        (`const _: () = assert!(MAX_LAYER as usize + 1 == LAYERS_A_SESSION_MAY_ASSIGN);`).
        **Never restate 256 in `mc-sim`**: `mc-sim` may not name `mc-render`
        (`crates/mc-render/tests/dependency_graph.rs`), and a second spelling is
        two places for one decision, which the spec's Declared Quantities section
        forbids by name. That assertion is also what makes D15's claim true.
      - FR-5.1-S2 (the only block naming a key is removed) leaves every remaining
        key on the layer it already held. **A retired layer keeps its texels and
        is not rewritten**, because `live` is what reaches `TextureLayers::stated`
        and `write_textures` iterates only what it was given.

- [x] **T10** `load` takes the layers already spent, and the golden frames must
      not move — `crates/mc-sim/src/content.rs`,
      `crates/mc-core/src/content.rs`,
      `crates/mc-client/src/{launch,startup}.rs`
      Scenarios: FR-5.1-S4
      Depends on: T09
      - D9: `load(root: &Path, spent: &LayerAssignment)`. `prepare_scene` and
        `prepare_launch` pass `LayerAssignment::none()` — a launch has spent
        nothing, which is a fact rather than a decision, and passing it at the
        call is what makes the property visible there. `LoadedContent` keeps all
        three fields. **A second entry point for the reload was rejected**: two
        doors are two answers, and `layers_of` was deleted in SPEC-016 precisely
        so the golden path and the launch path could not derive an assignment
        separately.
      - **D9's enforcement point.** `ResolvedContent::stating`
        (`crates/mc-core/src/content.rs:81`) is public, infallible and takes
        arbitrary pairs; a sparse assignment entering there would make `spent()`
        silently lie. So `stating` takes a `LayerAssignment`, and
        `LayerAssignment` is constructed **only** by `none()` and `appending()` —
        density and ascending order become properties of the type's constructors
        rather than of a comment.
      - **Verified rather than assumed, and it is why the goldens should not
        move:** `BlockRegistry::texture_keys` returns a `BTreeSet<TextureKey>`
        (`crates/mc-core/src/block/registry.rs:55`) and `resolved_from` does
        `.into_iter().zip(0..)`; owned iteration of a `BTreeSet` is ascending, so
        today's numbering is lexicographic and dense from 0, and a fresh
        `LayerAssignment` appending over the same set in the same order produces
        the identical pair list byte for byte.
      - **Trap 9 discharges here.** Run
        `crates/mc-client/tests/launch_and_capture_agree.rs` and both golden
        suites **immediately after this task and before T11**.
      - FR-5.1-S4 is what the whole policy buys: a section not meshed again after
        layer 4 was appended packs its quads with the indices it carried before —
        `base:stone` still 2 — so every vertex already on the GPU stays valid.

- [x] **T11** `PublishedContent`, its HUD, its serial, and readers that observe
      by asking — `crates/mc-sim/src/simulation.rs`,
      `crates/mc-core/src/content.rs`, `crates/mc-client/src/launch.rs`
      Scenarios: FR-4.4-S2, FR-4.5-S1, FR-8.1-S1, FR-8.1-S2, FR-8.1-S3, FR-8.1-S4, FR-8.1-S5
      Depends on: T10
      - D12: a **second** `ArcSwap` beside the snapshot. The serial is **not** put
        inside `SimSnapshot` — that type is `Copy` and holds plain values, and
        `crates/mc-sim/CLAUDE.md` warns that changing its shape reopens the
        publication hole silently. Nothing needs the correlation: a batch carries
        its own serial (D8) and a reader that wants the content asks for it.
        `ContentSerial` is a saturating `u32`, mirroring the tick counter, so the
        two counters a reader sees share one convention.
      - **Trap 2 discharges here. `PublishedContent` carries the HUD on the day
        it is written.** FR-4.5-S1 widens `hud/crosshair-horizontal.toml`'s
        `size` from `[9, 1]` to `[21, 1]` and requires the element published at
        `[21, 1]` *and* the frame composed from it.
      - **The HUD's two halves are graded by two instruments and the split is the
        layer upload's exactly.** That the *published* content carries the
        widened element is assertable with no device — assert the value where it
        crosses the boundary. That it reaches a *drawn* frame goes through
        `crates/mc-client/tests/support/hud_frames.rs`, which composes a HUD over
        a frame on a device with no window. **Neither covers the other's half**,
        and the residue — `App` assigning the new layout to its own `hud`
        field — is held by review, as `App`'s share of an edit already is.
      - **FR-8.1-S3 must go through an accepted reload.** It is nearly SPEC-016's
        own assertion with a reload in front of it, and a test that reaches the
        packing path any other way re-proves what another test already proves
        through the same code path — `testing.md` §1's definition of a bogus
        test.
      - FR-8.1-S2 and FR-8.1-S4 close both directions on the serial: a reader
        that has not looked goes on seeing what it last observed, and two accepted
        candidates publish two distinct serials so a reader can tell a reload that
        happened from one that did not. FR-8.1-S5 closes the third: a refused
        candidate leaves the published content **and** its serial exactly as they
        were.
      - FR-4.4-S2's second half is what makes it non-vacuous: a byte-identical
        candidate leaves the assignment identical **and** publishes a **later**
        serial, so a skipped attempt cannot satisfy it.
      - **`Simulation::new` grows a third parameter** — the launch's published
        content — so a simulation is never in a state where it has a world and no
        content. **This touches every construction site in the workspace**:
        `crates/mc-client/src/launch.rs`, the replay suites and `mc-sim`'s own
        tests. It is a real cost, it is the adaptation window the preamble warns
        about, and the compiler is its whole instrument.

- [x] **T12** Running out of layers is a refusal, and a refused candidate spends
      nothing — `crates/mc-sim/src/reload/mod.rs`,
      `crates/mc-client/tests/documented_refusals.rs`
      Scenarios: FR-5.1-S5, FR-5.2-S1
      Depends on: T11
      - `ReloadRefusal::LayerBudget { needed, spent }` names the layers it would
        need, the 256 available, and that relaunching reclaims every layer retired
        since the client started. That last clause is **literally true** and its
        arithmetic is `spent - live.len()`.
      - **The refusal is quoted on `docs/modding/hot-reload.md` and held to a
        real run by `crates/mc-client/tests/documented_refusals.rs`, extended
        here** — in the phase that adds the refusal, not in a documentation phase
        at the end.
      - FR-5.1-S5 is a **weak instrument** and is recorded as one below: under D6
        the assignment is a value carried by the publication, so a refused
        candidate structurally cannot spend the budget — there is no ledger for
        it to write to. It is kept because it would catch a drift back to a
        mutable ledger, and it is not evidence this task did work.
      - **DONE, and `documented_refusals.rs` turned out to do more than this task
        asked of it.** Under the greedy-append mutation — `appending` handing out
        what fits and succeeding instead of refusing — all four of its tests
        reddened **alongside FR-5.2-S1 and FR-5.2-S3**. So the page/print guard is
        also a witness that the budget refusal is *reachable at all*, which is
        strictly more than "a quoted refusal matches a printed one". Recorded here
        because a future reader trimming that file would have no way to know it.
      - **The sentence landed on `LayerBudget` in `mc-core`, not on
        `ReloadRefusal`** — D4 as amended, see the phase-2 notes. The bullet above
        naming `ReloadRefusal::LayerBudget { needed, spent }` is the superseded
        shape and is left standing rather than rewritten.

---

## Phase 3 — A saved edit becomes one attempt, built off the tick thread, and a refusal is stated once

**26 scenarios.** Done means: a change under the content root is noticed and a
burst of them becomes one attempt; a candidate is built from the whole root on a
thread that is not the tick's; and every refusal the loader can produce reaches a
person once, by name, with the previous content still serving.

**Red on arrival** because nothing watches, nothing builds off-thread and nothing
reports.

- [x] **T13** The port, the relevance rule and the settling window's one
      declaration — `crates/mc-world/src/content/watch/mod.rs` (new),
      `crates/mc-world/src/content/mod.rs`
      Scenarios: FR-1.1-S5, FR-1.1-S7, FR-1.1-S9
      - **D2, and the fact that decides it: the relevance rule must be built from
        the loader's own constants or it silently narrows.**
        `LuauFileDefinitionSource` declares `BLOCKS_DIRECTORY` and
        `DECLARATION_EXTENSION` (`crates/mc-world/src/content/luau_source.rs:50,53`);
        `TomlFileHudSource` declares `HUD_DIRECTORY` and **its own**
        `DECLARATION_EXTENSION` (`crates/mc-world/src/content/hud_toml_source.rs:25,28`)
        — two constants of the same name in different modules, which the rule
        imports and must disambiguate. Those four values decide FR-1.1-S5 and
        FR-1.1-S7 outright. A rule written in `mc-sim` would be a *second* list,
        and the day a third declaration kind arrives it would go on answering for
        two.
      - **The port speaks in paths that changed**, not in `notify`'s
        create/modify/remove taxonomy. The loader reads the whole root on any
        change (FR-1.2-S1), so *which kind* of change happened is information the
        domain has no use for, and a port mirroring `EventKind` would be a defect
        by architecture-principles' own test.
      - **FR-1.1-S9 carries its own discriminating half and the test must carry
        both.** `content/base/materials/dirt.toml` begins no attempt **while the
        same instrument begins one for `blocks/stone.luau`.** An absence alone is
        satisfied by a watcher that never fires and by a relevance rule that has
        come to refuse everything.
      - `SETTLING_WINDOW: Duration = Duration::from_millis(150)` is declared here
        and nowhere else (FR-9.1-S2, whose scan is phase 6's).

- [x] **T14** The `notify` adapter, and the window at the boundary it crosses —
      `crates/mc-world/src/content/watch/notify_watch.rs` (new),
      `crates/mc-world/Cargo.toml`
      Scenarios: FR-1.1-S1, FR-1.1-S2, FR-1.1-S3, FR-1.1-S4, FR-1.1-S6, FR-1.1-S8
      Depends on: T13
      - **Trap 5 discharges here and FR-1.1-S4 needs both instruments.** The
        coalescing test through the in-memory double, **and** a test asserting the
        `Duration` handed to the debouncer builder *is* `SETTLING_WINDOW`. Neither
        covers the other. The second needs no filesystem and no timer.
      - `mc-world` gains `notify` and `notify-debouncer-full` in
        `[dependencies]` — both already pinned in `[workspace.dependencies]` and
        named by no crate — and **must never let either escape this one module**,
        the same structural claim `mc-world` already makes about `mlua`.
        `ContentWatch` and `ContentChanges` name no `notify` type. **Re-read
        `crates/mc-world/tests/dependency_graph.rs` in this task.**
      - **Litmus test the engine-reader record must state: if `notify`
        disappeared tomorrow, one file changes.** That must remain true.
      - **The adapter is the least coverable code in a covered crate.** Keep it to
        construction, drain and map, with every decision on the `mc-sim` side of
        the port or in a pure function beside it. FR-1.1-S8 (an absent root) is
        deterministic against a `tempfile` path that does not exist. **One
        integration test with a generous timeout is the whole of what touches a
        real filesystem**; everything else goes through the double. If coverage
        dips, the answer is a thinner adapter, not an exclusion.
      - FR-1.1-S6 is a weak instrument on its own — a watcher that never fires
        satisfies it. FR-1.1-S1 and FR-1.1-S9 are its discriminating partners.

- [x] **T15** `ContentReload` — the pending flag, the off-thread build, and the
      drive — `crates/mc-sim/src/reload/mod.rs`,
      `crates/mc-client/src/reload.rs` (new, D14),
      `crates/mc-client/src/session.rs`
      Scenarios: FR-1.2-S1, FR-1.2-S2, FR-1.2-S4, FR-1.2-S5, FR-1.3-S2, FR-1.3-S3
      Depends on: T14
      - **D10's coalescing, and the order is the whole of it.** A single
        `pending: bool`. On a relevant change, set it. At a tick boundary with
        nothing in flight and `pending` set: **clear it, then start the build.** A
        change arriving during the build sets the flag again, and the boundary
        after the build ends starts exactly one further attempt (FR-1.2-S4). A
        queue would run N builds for N saves and publish N serials for one edit;
        refusing would drop an edit silently, which is the worst outcome
        available.
      - The build is `mc_sim::content::load` on a `std::thread::spawn`, polled
        with `is_finished()` then `join()`, exactly as `App::collect_preparation`
        polls the preparation. `join`'s `Err` becomes `ReloadRefusal::BuilderLost`
        (FR-1.2-S5), mirroring `PreparationError::WorkerLost`.
      - FR-1.2-S1: a build after `blocks/stone.luau` alone changed produces a
        candidate registering **all four** blocks. The whole root is read; there
        is no incremental door.
      - FR-1.2-S2 and FR-1.3-S2: the player advances over the build's ticks to
        where **an independent run of the same ticks** would have put them, and
        every tick during the build is answered from the content in force when the
        build began. Independent-run oracle again — never a snapshotted value.
      - FR-1.3-S3: a change reported before any tick has been advanced is held by
        the same `pending` flag until a tick boundary exists. Architecture
        Assumption 3 reads this as also covering a reload attempt arriving while
        the launch preparation is still in flight — deferred, not run.
      - **`watching_shipped_content(root)` is the one door a client goes
        through**, which is what makes phase 6's FR-8.2-S1 true of the client's
        sources. Do not let `Session` name the adapter.
      - **Trap 8 discharges here.** `session.rs` measured **492 of 500** non-blank
        lines at this branch's head. Re-measure before writing, and put the
        report type, the refusal reporting and its dedup, the `TextureLayers`
        rebuild and what an accepted reload hands the renderer into
        `crates/mc-client/src/reload.rs`. `Session` keeps only the `ContentReload`
        field, the drive after `advance`, and `take_reload_report`. The boundary
        is by responsibility, not by count: `session.rs` is what a keystroke
        decides, `remesh.rs` is what an edit becomes, this is what a content
        change becomes.
      - `Session::tick` keeps its `Option<EditReport>` signature; the reload's
        outcome is stashed and taken by `take_reload_report()`, the same take-once
        shape `pending_action` already uses. **`Session` is the only driver** — a
        second caller of `at_tick_boundary` would swap twice per tick, and nothing
        structural prevents it (architecture Assumption 5).

- [x] **T16** The refusal vocabulary, and a recurring refusal reported once —
      `crates/mc-sim/src/reload/mod.rs`,
      `crates/mc-client/src/reload.rs`,
      `crates/mc-client/tests/documented_refusals.rs`
      Scenarios: FR-1.2-S3, FR-2.1-S1, FR-2.1-S2, FR-2.1-S3, FR-2.1-S4, FR-2.1-S5, FR-2.1-S6, FR-2.1-S7, FR-2.1-S8, FR-2.3-S1, FR-2.3-S2
      Depends on: T15
      - **Trap 4 discharges here. The build stage needs no new fault vocabulary
        at all**, and that is the finding rather than a convenience. FR-2.1-S1
        (compile), S2 (`slid`), S3 (two files), S4 (emptied directory), S7 (HUD)
        and FR-2.3-S1/S2 (memory cap, raised error) are the **same refusals a
        launch already produces**, reached through the same call, arriving as
        `ContentError` over `DefinitionFault { origin, block, field, cause }`.
        `ReloadRefusal::Content` is `#[error(...)]` over it and **adds no wrapper
        that restates its cause**.
      - FR-1.2-S3 belongs with them: a declaration that loops is aborted at the
        shipped call-and-loop budget of 1 000 000, the candidate is refused naming
        the declaration file, and **the simulation goes on advancing**. That last
        clause is the half a refusal test forgets.
      - **FR-2.1-S8's dedup compares rendered text**, the exact shape of
        `App::report_remesh` (`crates/mc-client/src/app.rs:465`), which the spec's
        Existing Code table names as the contract. Comparing values instead needs
        `PartialEq` on an error chain that does not have it. Two structurally
        different refusals rendering identically are reported once — accepted
        (architecture Assumption 6), because the rendering is what a person reads.
        S8's second half is what makes it non-vacuous: **the next refusal that
        differs is reported**.
      - FR-2.1-S5 (the four blocks, the layer assignment and the held block
        exactly as they were) and FR-2.1-S6 (a corrected file's next candidate is
        accepted — a refusal ends one attempt, not the watching) are the two
        halves that stop a refusal path from being a dead end.
      - **The refusals this phase adds are quoted on `docs/modding/hot-reload.md`
        and held to a real run by `documented_refusals.rs`, extended here.**

---

## Phase 4 — What a reload re-meshes, and against which content

**12 scenarios.** Done means: a reload that changed geometry marks every section
and one that did not marks none, measured on one instrument; every batch is
meshed against the content it was drawn under; a batch meshed against superseded
content is discarded with its sections put back; and an appended layer reaches the
device.

**Red on arrival** because phases 1–3 mark no section on a reload and the worker
keeps the layers it was spawned with. **FR-6.1-S1 is green on arrival and is not
a trap signal** — see Trap 1.

- [x] **T17** The geometry-change predicate, and whole-world marking —
      `crates/mc-sim/src/world/reload.rs`, `crates/mc-sim/src/world/mod.rs`
      Scenarios: FR-4.1-S3, FR-6.1-S1, FR-6.1-S2, FR-6.1-S3, FR-6.1-S4
      - **Trap 6 is binding on this task. Read it before writing any expected
        count.** The expected value for FR-6.1-S2 on the shared instrument is
        **256**, not 82.
      - D7's rule, binary: a candidate whose accepted content changes some
        block's declared `is_solid` or declared `texture`, or adds or removes a
        block, marks **every section of the world**. One that changes neither
        marks **none**.
      - **The predicate is stated as fields, not as a hash.** Compare the serving
        and candidate registries by name: a name in one and not the other, or a
        name whose `is_solid` or `texture` differs. `behaviour_of`/`appearance_of`
        (`crates/mc-world/src/persistence/format.rs:307,319`) are **not** reused —
        `behaviour_of` folds `replaceable`, `breakable` and `breaks_into`, which
        change no geometry and which FR-6.1-S4 pins exactly, and both are
        `pub(crate)` to `mc-world` anyway. The save format's split is the right
        *idea* and the wrong *set*.
      - **The instrument is `RemeshWork::keys()`** — an `ExactSizeIterator`
        (`crates/mc-sim/src/world/remesh.rs:38`) reachable through
        `Session::take_remesh_work()` — and the dirty set is a `BTreeSet`, so a
        whole-world mark yields exactly 256 distinct keys, each once.
      - **FR-6.1-S4 is FR-6.1-S1's paired control and the two must run on one
        instrument.** Two candidates in one session: one changing only
        `base:stone`'s `breakable` meshes nothing, one changing its declared
        solidity meshes something. Without S4, an implementation that meshes
        nothing on any reload satisfies S1 and S3 jointly — which is what the
        spec's own scenario audit found and added S4 for.
      - FR-4.1-S3 is the drawn half of solidity: a face culled against solid stone
        appears once stone is not solid. It rides here because it needs the
        marking.
      - The over-marking is nearly free — the sections it adds beyond a selective
        rule are exactly the empty ones, which mesh to zero quads — and it is the
        rule `World::mark_dirty` already runs on, in its own words: "the correct
        thing rather than the fast thing". **The selective rule is DEFERRED, and
        taking it is a spec change** to FR-6.1-S3 and to the spec's Technical
        Considerations, not an optimisation somebody may take while passing.

- [x] **T18** The batch carries its registry; staleness is decided where "serving
      now" is known — `crates/mc-client/src/remesh.rs`,
      `crates/mc-sim/src/world/remesh.rs`, `crates/mc-client/src/session.rs`
      Scenarios: FR-6.2-S1, FR-6.2-S2, FR-6.2-S3, FR-6.2-S4, FR-6.2-S5
      Depends on: T17
      - **The registry travels with the batch — structural.** `RemeshWork` gains
        `Arc<BlockRegistry>`, taken from the `World` that produced it; `Retained`
        **loses** its `registry` field. A batch therefore cannot be meshed against
        a registry other than the one its world was resolved against, so
        FR-6.2-S2 and FR-6.2-S5 become **unspellable rather than checked**. The
        batch grows by one pointer-sized clone.
      - **Trap 7 is binding: the staleness comparison runs on the client at
        collect time, not on the worker.** `Remesher` remembers the in-flight
        batch's keys and the serial it was drained under, and holds the serial now
        serving, updated by `retire(layers, serial)`. `retire` goes on the **same
        ordered channel** the batches use, which is what makes "told before it
        meshes anything with them" true of the next batch for free.
      - `Remeshed` becomes `Scene | Superseded { keys } | Failed`. **FR-6.2-S4 is
        the half that is easy to drop**: the discarded batch's keys go back into
        the world's dirty set through `Session::mark_for_remesh`, or those
        sections stay stale for the rest of the run — a wrong picture with no
        error anywhere. FR-6.2-S3 is the existing `Failed` path, unchanged.
      - **Correcting the architecture's Integration table, which sends the reader
        to the wrong file.** The "a batch is seven sections per edited section at
        worst, so the copy is well under 100 KB" claim is at
        **`crates/mc-sim/src/world/remesh.rs:10`**, not
        `crates/mc-client/src/remesh.rs`. It becomes false — a whole-world batch
        is 256 sections, roughly 1.3 MB — and must be updated **with its new
        derivation** rather than deleted. The clone runs on the tick thread;
        confirm the figure rather than restating this one.
      - **Consequence, accepted and stated:** a reload that changes nothing
        geometric still supersedes a batch that happened to be in flight, so those
        sections are re-meshed although they were already correct. It costs one
        batch and can only re-mesh sections that were dirty anyway. A second
        "geometry serial" is **DEFERRED**.

- [x] **T19** The appended layer reaches the device, and the hand that holds the
      new block — `crates/mc-client/src/reload.rs`,
      `crates/mc-client/src/app.rs`, `crates/mc-render/tests/`
      Scenarios: FR-4.3-S2, FR-4.3-S3
      Depends on: T17
      - **Both halves, and neither covers the other.** Assert the *value* where it
        crosses the boundary — that the report hands over `TextureLayers` holding
        `base:amber` at layer 4 — **and** drive that same value through a real
        device with no window
        (`crates/mc-render/tests/terrain_offscreen.rs`,
        `crates/mc-render/tests/hud_offscreen.rs`). The assignment inside `App`
        stays held by review, as `App`'s share of an edit already is.
      - **D15: an upload that fails after an accepted swap ends the run**, on the
        same grounds `PreparationError::Upload` already does — a world serving
        content the device never received is a wrong picture with no error. The
        `report_remesh` trade is right for a *batch*, because a stale section is a
        stale picture of the same content; it is wrong here because the content
        itself has moved. **The one content-caused failure is unreachable**:
        `write_layer` refuses a layer at or past capacity
        (`crates/mc-render/src/gpu/buffers.rs:176`) and T09's budget refuses such
        a candidate first, with T09's compile-time assertion stopping the two
        bounds disagreeing.
      - Read `crates/mc-render/src/gpu/buffers.rs` before sizing anything:
        `write_textures` iterates **every** entry per call and `write_layer` is
        private with no single-layer path, so "append one layer" is a rewrite of
        every live layer. That is acceptable and it is why the per-reload upload
        cost is what it is.
      - **Trap 10 applies to this task above all others.** FR-4.3-S2 declares
        `texture` equal to `name`. Do not close the name-for-texture gap here.

---

## Phase 5 — A player inside a cell that became solid is moved clear

**7 scenarios.** Done means: a player whose box overlaps a cell the reload made
solid is moved to the nearest clear position, sideways before upward and never
downward, within a bounded search — and when there is nowhere to go, a person is
told.

**Red on arrival** because the swap from phase 1 moves nobody. **FR-7.1-S3 and
FR-7.1-S6 are green on arrival** for exactly that reason and are named below.

- [x] **T20** The clearing search — its candidates, its order, its bound and its
      verdict — `crates/mc-sim/src/world/clearing.rs` (new),
      `crates/mc-sim/src/player/collide.rs`, `crates/mc-sim/src/simulation.rs`
      Scenarios: FR-7.1-S1, FR-7.1-S2, FR-7.1-S3, FR-7.1-S4, FR-7.1-S5, FR-7.1-S7
      - D11, fully specified because a wrong tie-break is invisible.
        **Candidates are cell centres** — feet at `(cx + 0.5, cy, cz + 0.5)` — so
        the 0.6-wide box spans `[x+0.2, x+0.8]` and lies strictly inside one cell
        column, making clearance a question about the two cells the 1.8-tall box
        occupies rather than four. **Cost, and a player will notice it:** being
        cleared loses their sub-block position. Not being cleared leaves it
        exactly, which is why FR-7.1-S3 is a *no move at all* rather than a move
        to where they already were.
      - **The cube is `dx, dz ∈ [-8, 8]` and `dy ∈ [0, 8]`.** Downward is not
        ranked last, it is **absent** — "never downward" as a property of the
        candidate set rather than of an ordering, which is what makes FR-7.1-S4
        hold under any future reordering.
      - **Order: `(dy, max(|dx|, |dz|), dz, dx)` ascending.** `dy` first is what
        makes FR-7.1-S2 come out sideways when a sideways and an upward cell are
        both one away. Chebyshev horizontal distance matches the cube the bound
        describes. The last two are a declared tie-break, so two runs agree.
      - **Assert reachability, never a position count.** A clear space at exactly
        8 sideways is found and one at 9 is not. The spec's declared cost ceiling
        is 17³ = 4 913 positions; this search spends 9 × 17 × 17 = 2 601 of it,
        which is a deliberate narrowing **under** the stated bound and not a
        different bound — so a test asserting 4 913 positions tested reddens
        against a conforming implementation. That is Trap 6's shape again, in a
        second place.
      - **The predicate is `collide::overlaps`** (`crates/mc-sim/src/player/collide.rs:240`,
        currently private) promoted to `pub(crate)`, so `HALF_WIDTH`, `HEIGHT` and
        the half-open `[v, v+1)` rule are stated once and read here. Rewriting the
        overlap test in `clearing.rs` would be agreement between two copies of one
        decision.
      - **Velocity is zeroed on any clearing move, not only an upward one.**
        FR-7.1-S7 demands it for upward; doing it for every move is one rule
        instead of one per direction, and a cleared player has been teleported. It
        does not touch FR-3.2-S3, which is about a player who was **not** cleared.
      - **The verdict is enumerated** — `Clearing::{ Unneeded, MovedTo(Vec3),
        NoClearSpaceWithin { blocks } }` — so "could not be cleared" is a value
        rather than an absence. **And it must reach a person**: FR-7.1-S5 requires
        the system to *report* it, so `Clearing` travels out through
        `ReloadStep::Accepted` and `ReloadReport::Accepted` to the one place that
        prints. **A verdict computed and dropped satisfies nothing**, and a test
        that reads `Clearing` from a pure call rather than from the report does
        not grade that.
      - It runs **after** `adopt`, against the solidity the candidate produced.

- [x] **T21** A refused candidate moves nobody —
      `crates/mc-sim/src/simulation.rs`, `crates/mc-client/tests/`
      Scenarios: FR-7.1-S6
      Depends on: T20
      - Structural: `cleared` is reached only from the accepted path, so a refused
        candidate moves nobody **because the code never runs**.
      - **This is green on arrival and its mutation is what decides whether it is
        a control or filler.** Call `cleared` on the refusal path and confirm the
        test reddens. If it does not, the test is not driving a *refused*
        candidate that would have made the player's cell solid, and it is
        FR-7.1-S3 again through the same code path — a second witness on a path
        that already has one, which `testing.md` §1 calls a bogus test.

---

## Phase 6 — The seam stays cut, the window is declared once, and the pages are true

**5 scenarios.** Done means: neither the client's own sources nor the capture
pipeline's name a door that reads a content root or watches one; the settling
window is declared in exactly one place, reported by a scan that can tell one
declaration from several and from having read nothing; a reload blocks no tick;
and all three audiences can act on what this spec built without reading Rust.

**Every scenario in this phase reports on a property the earlier phases already
established.** What phase 6 builds is the *instrument*, and an instrument written
against an already-true property is green the moment it compiles. **The positive
controls are therefore the whole of this phase's falsifiability** and none of them
is optional.

- [x] **T22** The client-source scan gains the reload path and the watcher —
      `crates/mc-client/tests/client_names_no_content_door.rs`
      Scenarios: FR-8.2-S1
      - Two needles: **the watcher adapter's constructor and the vendor's own
        spelling**. They are chokepoints rather than type names, following the
        four already there — renaming a source does not rename the door.
      - **The fixture derives its expected report from the needle list, and a
        needle added without a fixture entry is a needle nobody has watched match
        anything.** The fixture must name **every** door, old and new.
      - **The scan wants no exemption at all.** If one turns out to be needed,
        that is a door left behind rather than a licence to write the exemption —
        the guard's own doc comment says so and it is why `watching_shipped_content`
        exists (T15).
      - Its known residual is already recorded in the guard's doc comment and is
        unchanged: a *second* door bypasses every needle, and the instrument that
        would close it is the dependency-closure guard, which belongs to the
        composition-root spec.

- [x] **T23** The capture pipeline watches nothing, and the scan can tell that
      from having read nothing — `crates/mc-client/tests/` (new guard)
      Scenarios: FR-8.2-S2, FR-8.2-S3
      Depends on: T22
      - **This is a different root from T22's scan, not an extension of it.** It
        reads the capture pipeline's own sources — `crates/mc-client/src/startup.rs`,
        the golden suites and `mc-testkit`. A golden frame is a claim about one
        content set, and a run that could re-read content mid-capture is a run
        whose committed image depends on what happened to be on disk.
        `prepare_scene` gains no watcher (Decision 7).
      - **The verdict is enumerated**, following
        `client_names_no_content_door.rs`'s three-way shape, so a scan that read
        nothing cannot pass for a clean one.
      - **FR-8.2-S3 is its positive control and is the only thing standing between
        this scan and a test that cannot fail.** Run the same scan over a source
        that *does* name the watcher door and require it to report that file and
        the spelling it named.

- [x] **T24** The settling window is declared exactly once —
      `crates/mc-world/tests/` or `crates/mc-client/tests/` (new scan)
      Scenarios: FR-9.1-S2
      - The verdict is `DeclaredExactlyOnce` / `DeclaredIn(Vec<String>)` /
        `NoSourceWasRead`. **An enumerated verdict rejects every other answer
        including the ones meaning "I could not look"**, so a vanished source
        directory reddens for free — which `assert!(found.len() == 1)` cannot do.
      - Its two controls, both owed: a second declaration in a second module
        yields `DeclaredIn([..])`, and an empty root yields `NoSourceWasRead`.
      - **This scan does not cover Trap 5's second instrument and must not be read
        as covering it.** A window declared once and handed to the debouncer as
        `Duration::ZERO` leaves this scan green. The boundary assertion lives in
        T14.

- [x] **T25** A reload's re-mesh blocks no tick —
      `crates/mc-client/tests/`, `crates/mc-sim/tests/`
      Scenarios: FR-9.1-S1
      - The ticks advanced while a whole-world re-mesh runs are **the ticks the
        same inputs would have advanced with no reload in flight** — an
        independent-run oracle, not a threshold.
      - **Green the moment phase 4 landed**, because the re-mesh transport already
        runs off the tick thread. It is the deterministic half of a latency target
        whose other half is a benchmark, and it reddens if anything ever moves the
        re-mesh onto the tick.
      - **Why the one-second target is measured and not gated:** a wall-clock
        assertion on shared hardware is a flake generator, and one that fails
        intermittently teaches everybody to re-run it — which is the state in which
        it reports nothing. The latency is carried by a `criterion` benchmark run
        as a standalone command, exactly as the mesher's < 200 µs/section budget
        already is. **The measurement T02 takes goes into it.**

- [x] **T26** The documentation deliverable, all three audiences, and every
      falsified statement — `docs/modding/hot-reload.md` (new),
      `docs/modding/{blocks-items,README}.md`, `docs/user/gameplay.md`,
      `docs/technical/{architecture,rendering,testing,decisions}.md`,
      `docs/INDEX.md`, `content/CLAUDE.md`, `crates/mc-script/CLAUDE.md`,
      `crates/mc-sim/CLAUDE.md`, `crates/mc-client/src/startup.rs`,
      `crates/mc-sim/src/world/mod.rs`
      Scenarios: **none of its own** — see "Mechanisms no scenario covers"
      Depends on: T22, T23, T24, T25
      - **Mod author — `docs/modding/hot-reload.md`, new.** `docs/INDEX.md`
        already routes "Hot-reload semantics and state migration" there. It states:
        what triggers a reload and what does not, including the 150 ms settling
        window; that the whole root is read, blocks and HUD together; **every
        refusal an author can meet, quoted**, held to a live run; what survives; that
        a newly declared block arrives in their hand and why that is a placeholder
        rule; the 256-layer session budget, its refusal and that relaunching
        reclaims it; **the standing limitation that editing `texture` is not yet
        visible and what it does instead**; that a `breaks_into` naming nothing is
        still accepted and still fails at the break; **that being moved clear costs
        a player their sub-block position** (D11); **that a reload also applies the
        HUD the same root declares** (D12); and a **complete worked example — edit
        one file, save, see the change — that runs.** A reference listing names
        without a working example is not documentation.
      - **The worked example must not declare a `texture` that differs from its
        `name`**, for the reason SPEC-016's own documentation task recorded: such a
        declaration loads and then does not draw, the batch fails with
        `UnresolvedTexture`, and a failed batch is logged and dropped rather than
        failing the run — so every guard this spec has would stay green over a
        walkthrough that does not work.
      - **Player — `docs/user/gameplay.md`.** Content edited while you are playing
        now reaches the world without a restart; your world, your position and your
        save are untouched by it; if something you are standing in becomes solid you
        are moved to the nearest clear space rather than trapped, and told when you
        could not be. "Not applicable to that audience" is refused.
      - **Engine reader.** `docs/technical/architecture.md`: the reload seam end to
        end — the watcher port and its adapter, the candidate built off the tick
        thread through the existing content door, the tick-boundary swap, the second
        write door into `World` **as a child module and why** (D3), the content
        publication and its serial, **that a batch carries its registry while
        staleness is decided on the client** (D8), **that a retired layer is spent
        but not live** (D6), why a discarded batch's sections go back into the dirty
        set, and **that `mc-world` names the watcher vendor in exactly one file**
        (the Boundaries litmus test) — which is the thing a future change may not
        break. `docs/technical/rendering.md`: appended never renumbered within a
        session, what that buys, the 256-layer budget, that appending writes one
        layer rather than re-creating the array, **and that a retired key keeps its
        layer and its texels for the session and what that costs**.
        `docs/technical/testing.md`: how a reload is driven with no filesystem and no
        window, this spec's mutation table, why the latency budget is a benchmark
        rather than a gate stage, and **the one manual acceptance check no harness
        can drive** — a real editor, a real save, a real window.
        `docs/technical/decisions.md`: an ADR for append-never-renumber and one for
        refusing a candidate that drops a placed block, each with its rejected
        alternative.
      - **Two in-tree comments are falsified by this spec and must be rewritten
        rather than left standing.** `crates/mc-client/src/startup.rs`'s `scene_of`
        states that "the registry does not change mid-session" — **its conclusion
        survives intact and is FR-5.1-S4; its premise does not**, and a reader who
        finds the old premise will draw the wrong inference. And
        `crates/mc-sim/src/world/mod.rs`'s header claim that exactly one function
        writes anything (T02). Confirm both were done; neither is mechanically
        checked.
      - **`crates/mc-sim/CLAUDE.md`** — its "No wall clock" rule demands a recorded
        decision before anything reads one. The decision is D2: the clock is the
        debouncer's, it lives behind a port in `mc-world`, and `mc-sim` still reads
        none. **Record it there, not only in the archived architecture.**
      - **`crates/mc-script/CLAUDE.md` and `content/CLAUDE.md`** — invariant 7 gains
        the sentence saying where it is now realised, and the mod-`tests/` clause is
        corrected in both: what gates a reload candidate today is the loader's own
        all-or-nothing validation, the same gate that runs at launch. Both files
        currently promise something the tree does not keep.
      - **`docs/modding/blocks-items.md`** — "What is not here yet" opens with "Hot
        reload of declarations: a declaration is read once, at load." **That becomes
        the present tense**, with a pointer to the new page.
        **`docs/modding/README.md`** — the first-block walkthrough gains its last
        step: keep the client running and edit the file you just wrote.
        **`docs/INDEX.md`** — register `modding/hot-reload.md` and add SPEC-017 to
        the Sources column of every file above.
      - **Author- and player-facing pages name no issue tracker.** A reader of the
        modding guide cannot open a Linear issue, the reference means nothing to
        them, and it dangles permanently once the issue closes — the same rule as
        code and test names never carrying scenario IDs. **State the decision, drop
        the pointer.** Engine-facing docs may name one, provided the reference stays
        supplementary to substance that stands without it.

---

## Scenarios green on arrival inside their own phase

Named here rather than buried, because **a phase whose scenarios were already
green did not do work, and a breakdown that lets these read as progress
overstates phases 1, 4, 5 and 6.**

**Every one of these is mutated before it is called a control.** That is the
measurement separating *green on arrival* from *inert*, and the last spec found
seven such scenarios and proved every one load-bearing. Break the implementation
by hand, observe the suite, revert by hand, confirm `git diff --exit-code` is
clean. **Record the outcome either way, including mutations that did not bite** —
a mutation that does not bite is evidence about the code's structure, not
automatically a test gap.

| Scenario | Phase / task | Green because | The mutation that must redden it |
|---|---|---|---|
| FR-3.1-S1 | 1 / T05 | a swap that does nothing leaves the broken and placed cells alone | have `adopt` clear the section holding the player's edits |
| FR-3.1-S2 | 1 / T05 | same, cell for cell | write `base:dirt` into one world position inside `adopt` — if it does not redden, the cell-for-cell comparison is not total |
| FR-3.2-S1 | 1 / T05 | a swap that does nothing leaves the player where the ticks put them | (a) `adopt` sets the position to the origin; (b) **run the oracle one tick short** — the test must redden, which is what proves the comparison is against a real independent run rather than against itself |
| FR-3.2-S3 | 1 / T05 | same, for velocity | `adopt` zeroes velocity. This mutation also proves phase 5's blanket velocity-zeroing does not reach a player who was not cleared |
| FR-3.3-S1 | 1 / T03 | the first solid block is still the first solid block | `adopt` sets `holding` to `base:water`, the one non-solid shipped block. If it does not redden, the test is not reading `holding` through `Session` |
| FR-3.4-S2 | 1 / T07 | it is the existing `Acceptance`/`RegistryVerdict` path and needs **no reload machinery at all** | make `RegistryVerdict::resolve` report an empty `changed`. **If this does not redden, FR-3.4-S1's silent resume is evidence that nothing is ever compared** |
| FR-4.1-S5 | 1 / T02 | a swap that does nothing leaves stone stopping the player | **write the registry before the names-held check** inside `adopt_candidate` — i.e. break D3's "settle everything fallible first". This is the sharpest mutation in phase 1 |
| FR-4.4-S1 | 1 / T06 | nothing moves, so nothing the author did not edit moves | apply stone's fields to every block in the candidate build |
| FR-6.1-S1 | 4 / T17 | phases 1–3 mark no section on a reload, so an implementation that never meshes satisfies it | make the geometry predicate return `true` unconditionally. **Its paired control is FR-6.1-S4 and the two run on one instrument** |
| FR-7.1-S3 | 5 / T20 | phases 1–4 move nobody | make `cleared` return `MovedTo(feet + (1,0,0))` where it returns `Unneeded` |
| FR-7.1-S6 | 5 / T21 | `cleared` is reached only from the accepted path | call `cleared` on the refusal path. **If it does not redden, the test is not driving a refused candidate that would have made the player's cell solid** — see T21 |
| FR-9.1-S1 | 6 / T25 | green the moment phase 4 landed; the re-mesh transport already runs off the tick thread | make `Session::tick` join the re-mesh worker before returning |

**Twelve.** Counted scenario by scenario against the phase each opens in.

**Four more are green as soon as their instrument is written**, which is a
different thing and is recorded separately rather than merged into the twelve.
Phase 6's structural scans report on properties phases 1–5 already established, so
none of them can ever have a red step in the ordinary sense — **their positive
controls are the whole of their falsifiability:**

| Scenario | Task | Its control |
|---|---|---|
| FR-8.2-S1 | T22 | add the watcher's spelling to a client source and require the scan to name it; revert by hand |
| FR-8.2-S2 | T23 | FR-8.2-S3 **is** its control, and is the reason the scan exists in this shape |
| FR-8.2-S3 | T23 | point the scan at an empty directory and require `NoSourceWasRead` rather than a clean report |
| FR-9.1-S2 | T24 | declare a second `SETTLING_WINDOW` in a second module and require `DeclaredIn([..])`; and an empty root yielding `NoSourceWasRead` |

> **Where this list came from, and the architecture now carries the same one.**
> `architecture.md`'s Phasing section originally headed its list "Scenarios green
> on arrival inside their own phase" and then included a bullet — phase 3's
> FR-2.1-S5, S6, S7 and FR-2.3-S1, S2 — whose own text says they are **red on
> arrival**. Read by the bullets' text it was nine; read by its heading, fourteen.
> Neither matched a scenario-by-scenario count, and the count found three it did
> not name: **FR-6.1-S1, FR-7.1-S3 and FR-7.1-S6**. Both failures came from
> assembling the list per *decision* — which scenarios does each phase's design
> leave untouched — rather than per *phase*, which is what the tree does when that
> phase opens. **Only the second question can be right**, because a scenario is
> green or red against a tree and not against a design. `architecture.md` was
> corrected at this stage and now carries these twelve and the separate table
> below, so the two documents do not have to be reconciled by whoever reads them
> next.

## Inherited behaviour that is red on arrival, and what it controls

Not green, and named separately so nobody merges the two ideas. These are
refusals the loader already produces at launch; they are red only because no
reload path exists yet, and **they redden again if the reload ever reaches content
through anything other than `mc_sim::content::load`** (Trap 4).

| Scenario | Phase / task | Already implemented by |
|---|---|---|
| FR-2.1-S5 | 3 / T16 | the loader is all-or-nothing; `load` hands back a `LoadedContent` or a refusal and there is no third answer |
| FR-2.1-S6 | 3 / T16 | a fresh read per attempt; a refusal ends one attempt, not the watching |
| FR-2.1-S7 | 3 / T16 | `mc_sim::content::load` refuses blocks and HUD together, "a root that is good for one and bad for the other is a root that failed" |
| FR-2.3-S1 | 3 / T16 | `HostLimits`' per-entry memory cap of 256 KiB |
| FR-2.3-S2 | 3 / T16 | `ScriptFault` over an error a declaration raised |

## Weak instruments, named

Kept because they cost nothing and would catch a drift, but **not evidence their
task did work**, and `test-map.md` must say so.

- **FR-5.1-S5** (a refused candidate appends no layer) — structurally true under
  D6 once the assignment is a value carried by the publication: a refused
  candidate never reaches the publication, so there is no ledger for it to write
  to. It would catch a drift back to a mutable ledger object.

  **VINDICATED in phase 2, and the prediction is why this entry earns its place.**
  Moving the publication *before* admission is that drift's exact shape, and it is
  the mutation that reddens FR-5.1-S5 — alongside FR-8.1-S5 and nothing else. So a
  weak instrument is not a useless one; what makes it worth keeping is that the
  prediction above was **specific enough to be checked**, and was then checked. An
  entry saying only "kept in case it catches something" could not have been.
- **FR-1.1-S6** (nothing changed → no attempt, however many ticks) — satisfied by
  a watcher that never fires. Its discriminating partners are FR-1.1-S1 and
  FR-1.1-S9, and it must not be read as evidence on its own.
- **FR-6.2-S2 and FR-6.2-S5** (a section is meshed against the content now
  serving) — become **unspellable** once `RemeshWork` carries its own registry
  (D8). They are structural confirmations rather than defences, and the
  behavioural instrument for the hazard is FR-6.2-S1.

## Mechanisms no scenario covers, and where each gets its test

This project ships these labelled rather than silently.

| Mechanism | Why no scenario reaches it | Where it gets its test |
|---|---|---|
| `adopt` staying module-private rather than `pub(crate)` | structural; every behavioural test stays green either way | the compiler (a `pub(crate)` `adopt` compiles, so the check is a reviewer reading `world/mod.rs`'s rewritten header) — T02 |
| `App` assigning the published `HudLayout` to its own `hud` field | `crates/mc-client/src/app.rs` needs a real window and nothing in the workspace constructs one; `mc-client` is excluded from the coverage denominator **wholesale** | held by review, as `App`'s share of an edit already is. The two halves either side of it are covered: the value at the boundary (T11) and a drawn frame through `hud_frames.rs` (T11) |
| `App` uploading the appended layers | same | same shape — the boundary assertion and the offscreen render in T19; the assignment itself is review |
| **D15** — an upload failure after an accepted swap ends the run | nothing in the spec covers it, and the default is whatever the implementer types in the one file nothing grades | no test. Its safety rests on the one content-caused failure being **unreachable**, which is T09's compile-time assertion. Recorded in the engine-reader record (T26) |
| The whole-world solidity resolve's cost on the tick thread | it is a latency question and the gate asserts no wall clock | **measured in T02** and sent to the FR-9 `criterion` benchmark. The number is taken before anything is decided, not after |
| The one-second end-to-end target | needs a real editor, a real filesystem and a real window | a **named manual acceptance check**, documented in `docs/technical/testing.md` (T26) and run once by hand |
| `Simulation::new`'s third parameter reaching every construction site | mechanical | the compiler — T11, and it is the adaptation window the preamble warns about |
| `notify` never escaping `notify_watch.rs` | a scan for an absence goes green forever the day the thing it guarded is removed | Rust's extern-crate rules make the manifest half structural; `crates/mc-world/tests/dependency_graph.rs` is re-read in T14 and the litmus test is stated in the engine-reader record |
| The `remesh.rs` batch-size comment's new derivation | prose | T18 confirms the figure against the tree; review reads the rewrite |
| The documentation deliverable (T26) | nothing mechanically checks that a falsified statement was rewritten or that the player paragraph exists | `/sdd-validate`'s reviewer against the enumerated list in T26 — which is why that list is file by file rather than described |

## Notes

*Deferred observations and follow-ups. Never delete task text; append status
markers only.*

### Phase 1 outcome, 2026-08-17 — T01–T08 done, 26 of 26 scenarios green

**RED was 23 failing and 3 green, not 18 and 8.** The accepting-no-op skeleton
left only FR-3.3-S1, FR-3.4-S2 and FR-4.4-S1 green. Five of the eight scenarios
this file predicted green on arrival were red, because the test author routed
their observation through a save written *after* the swap (FR-3.1-S1, FR-3.1-S2)
or through an in-test control the no-op fails (FR-3.2-S1, FR-3.2-S3, FR-4.1-S5).
Stronger than predicted; the prediction was assembled from the design, and the
tree is what answers.

**Every named mutation was run. Five bit, three did not.**

| Scenario | Mutation | Outcome |
|---|---|---|
| FR-3.1-S1 | `adopt` empties the 4 096 cells of the section holding both edits | **red** |
| FR-3.1-S2 | `adopt` writes the first registered block into world `(1, 200, 1)` | **red** — the cell-for-cell walk is total |
| FR-3.2-S1 (a) | `adopt` sets the player's position to the origin | **green at first**, **red after the repair below** |
| FR-3.2-S1 (b) | the oracle advanced one tick short | **red** — run by the test author, verified from their decoded output |
| FR-3.2-S3 | `adopt` zeroes the player's velocity | **green at first**, **red after the repair below** |
| FR-3.3-S1 | `adopt_candidate` hands back registration id 3 (`base:water`) | **red** |
| FR-3.4-S2 | `judge` stops pushing onto `RegistryVerdict::changed` | **red** |
| FR-4.1-S5 | the registry written before the names-held check | **green** — see below |
| FR-4.4-S1 | every block wearing registration id 2's `is_solid`/`replaceable`/`breakable` | **red** |

**The two that mattered: FR-3.2-S1 and FR-3.2-S3 were structurally vacuous, and
are fixed.** Both read their headline value from the published snapshot, and a
swap publishes no tick of its own — so nothing the swap did to the player could
change what either assertion read. Phase 5 is why it mattered: the clearing
search writes the player at the swap, and FR-3.2-S3's whole job is to hold for a
player who was *not* cleared. The test author's repair reads both a tick **after**
the swap, over a grass floor under a stone ceiling, so the candidate cannot
legitimately move either player — a floor of the changed block would have made
the comparison red against a *correct* client, which is this file's Trap 6 shape
in a third place.

**Re-verified independently after the repair, each mutation biting only its own
scenario:** `self.player.position = Vec3::ZERO` in `Simulation::adopt` reddens
FR-3.2-S1 and leaves FR-3.2-S3 green; `self.player.velocity = Vec3::ZERO`
reddens FR-3.2-S3 and leaves FR-3.2-S1 green. So neither test is quietly grading
the other's value.

**What carries the signal in FR-3.2-S1, measured and worth holding.** Across one
walking tick the position pair differs by 0.072 blocks in x while **yaw, pitch
and height are bit-identical** — look is applied on the tick that drains the
accumulator and does not accumulate afterwards, and a walking tick does not
change height. So only the two horizontal axes can discriminate a tick-count
error. Dropping position from that comparison, or comparing orientation alone,
retires the mutation-(b) defence entirely, and `require_moved` refusing a run
that never left the spawn is what keeps the walk load-bearing rather than
scene-setting.

**FR-4.1-S5's mutation cannot bite, and that is the code's shape rather than a
test gap.** Its candidate drops `grass.luau` while the world holds grass, so
`World::adopt`'s own `SolidVoxels::resolve` refuses before either write and the
refusal arrives as the same `BlocksTheWorldHolds` the test expects. Solidity
being settled first is what neutralises it.

**A mechanism no scenario covered, found by mutation — now instrumented on the
team lead's ruling.** `World::adopt` assigning `self.registry` *before* resolving
solidity — D3's property 1, exactly — left **1110 of 1110 tests green**, because
`adopt_candidate`'s names-held check makes `adopt`'s error path unreachable
through the reload flow. **RULING: a backstop with no witness stops backstopping
silently, and this path had zero witnesses rather than one.** `World::adopt`'s
contract is that a refusal leaves the world untouched; one caller pre-checking
does not make the contract unreal, it makes it unwitnessed — and "a failed reload
changes nothing, partial application is a Blocker" is the project rule the
ordering locally expresses. So it is a rule with no instrument, not dead code.

The instrument is `crates/mc-sim/src/world/mod_test.rs`, authored by the test
author: `World::adopt` called **directly**, bypassing `adopt_candidate`, with a
candidate dropping a block the world holds — asserting the refusal **and** that
the registry is the one it had before. Paired with a positive control in a
separate test function, because "the registry is still the one it had" is
satisfied just as well by an `adopt` that never replaces it at all. Two
mutations pin the pair, and **both were run and both bit, each on exactly one
guard**: assigning `self.registry` before the resolve reddens
`a_candidate_missing_a_block_the_world_holds_is_refused_and_leaves_the_registry_it_had`
and leaves the control green; deleting the assignment entirely reddens
`a_candidate_answering_for_everything_the_world_holds_replaces_the_registry`
and leaves the refusal guard green. So neither is standing in for the other, and
the sharper mutation that survived 1110 tests now has a witness.

**Phase 1 closed with the gate at exit 0.** format · lint + complexity ·
gpu-free · rustdoc · size (433 files under `crates`, 60 under `tools`, all within
limits) · deps · sast · secrets · **1112 tests passed, 1 skipped** · **coverage
94.03%, regions 92.13%**, 9 381 lines tracked. The 1112 is 1110 plus the two D3
guards.

### The shipped-content boundary this spec depends on, checked at every phase end

**Two directories under `content/base/` are load-bearing for this spec and the
rest are not.** The loader reads `blocks/` and `hud/` and nothing else;
`materials/` and `models/` are read only by `tools/voxforge`, and no block
declaration names a material. So content work anywhere but those two directories
cannot reach a scenario here — which is the same fact FR-1.1-S9 exists to pin,
and it is measured rather than assumed.

What `blocks/` and `hud/` being quiet buys, and therefore what moving them would
cost:

- `blocks/` holding exactly four declarations, each with `texture` equal to
  `name`, is what makes `base:dirt` 0, `base:grass` 1, `base:stone` 2 and
  `base:water` 3 — so a fifth declaration or a changed `texture` field turns
  FR-5.1-S1's "`base:amber` gets layer **4**" into layer 5, breaks FR-4.4-S2's
  stated assignment, and falsifies the "shipped content" convention FR-5.1-S1..S4
  and FR-8.1-S3 all rest on.
- Every committed golden frame is shot through the shipped content, so the same
  edit moves all of them. That is Trap 9's whole subject.
- Phase 3's watcher is under test *against* these directories, so a concurrent
  writer inside one is a race rather than an attribution nuisance: it would flake
  intermittently and nobody would suspect the cause.

**So the condition is the narrow one — `content/base/blocks/` and
`content/base/hud/` unchanged — checked at every phase boundary and reported as a
fact rather than a reassurance. If either moves, the phase stops and it is
reported.** Concurrent content work in `materials/` or `models/` is fine and needs
no coordination.

**A concurrent writer under `materials/` is not a control.** It would exercise
FR-1.1-S9's negative case for free, and a control nobody can schedule is not a
control. FR-1.1-S9 keeps its own fixture at full strength.

### Phase 3 outcome, 2026-08-17 — T13–T16 done, 26 of 26 scenarios green

**Gate exit 0.** 682 tests across `mc-sim`, `mc-client` and `mc-world`, 453 files
measured under `crates`, **coverage 94.19%** (regions 92.27%, 9 608 lines).

**THE CLOSING CONDITION IS DISCHARGED, and it took a test that no phase-3 scenario
could express.** Passing `LayerAssignment::none()` where the build reads the
serving assignment reddens **two** tests: the one written for it, and FR-2.1-S6's
corrected candidate, which expects a reintroduced key on the next unused layer.
Two independent witnesses. Why no existing scenario could see it: appending the
shipped four to an empty assignment produces the identical pair list, so the two
arguments agree *unless* a key the session has never assigned is introduced.

**Five mutations, all measured, each biting only what it should.**

| Mutation | Reddened |
|---|---|
| the build reads `LayerAssignment::none()` | the serving-assignment test **and** FR-2.1-S6 |
| the dedup compares the top sentence rather than the whole chain | FR-2.1-S8 |
| the relevance rule keys on the extension alone | the shipped-root listing oracle **and** FR-1.1-S9 |
| `pending` cleared when a build is collected rather than when it starts | FR-1.2-S4 |
| the shipped door hands the debouncer `Duration::ZERO` | **only** the boundary assertion |

The last one is Trap 5 confirmed by measurement: `Duration::ZERO` leaves the
coalescing test and every other scenario green, so the boundary assertion is the
sole instrument and both halves really are owed.

**A test-side defect I diagnosed and the author fixed, and the half that mattered
was not the half that failed.** 21 of 27 failed because `until_settled` counted
boundaries as a stand-in for "long enough for a build to land" — a boundary costs
~100 ns in a test loop and one rendered frame in the shipped client, so the
conversion factor differs by five orders of magnitude. Measured: a candidate build
is **0.7–1.7 ms**, observable through `is_finished` after ~1 ms, against ~0.2 ms of
patience; instrumented at 1 800+ polls with the build still in flight. Confirmed by
replacing the poll with a blocking `join`, which turned 64 of 64 green.

**The same defect ran the other way and nobody had asked about it: the runs
expecting *no* attempt were vacuous.** Three hundred tight boundaries span 30 µs,
so an implementation that wrongly began an attempt on `stone.luau.swp` or on a
material file would not have finished the build before the run ended — FR-1.1-S6,
S7, S8, S9's first half and FR-1.3-S3's first half were all right answers for the
wrong reason. Both bounds are now durations denominated in `SETTLING_WINDOW`, so
no new magic number entered.

**A gap the blocking experiment exposed, and it is worth more than the 21.**
**FR-1.2-S2 passes with the build blocking the tick.** Its observable is where the
ticks put the player; its property is *when* the ticks happened. A blocking collect
changes only the second, so the scenario does not grade "off the tick thread"
despite its wording. **FR-9.1-S1 has the same shape and the same hole**, and phase
6 inherits a fixture that no longer even makes the fragile discriminator available
— the one candidate being "more than one boundary with a build in flight", which is
a timing assertion wearing a count's clothing and was deliberately not taken.
Carried to phase 6's brief.

**Two things of mine.** I introduced a regression: my first `Session::tick` drained
the input accumulator before checking for a simulation, breaking the property
`tick`'s own doc comment states, and `pointer_dispatch` caught it. And
`session/mod.rs` reached **518 of 500**; rather than shave comments I split the
keyboard and mouse vocabulary into `session/vocabulary.rs` — a vocabulary is not a
decision — leaving `mod.rs` at **485**.

### CLOSING CONDITION ON PHASE 2 — `documented_refusals.rs`, all four by name

**Four of its tests went red on the shared fixture rather than on their own
subject**, because `printed_refusals` could not produce a layer-budget refusal
while a skeleton `appending` never refused. That is *red for a known reason hiding
red for an unknown one* — the family with the worst record in this project,
because a known-red invites deferral and a deferred red stops reporting anything
new. The rule is explicit: **fixed before the phase closes, never annotated.**

**The asymmetry that earns this its own heading:** four tests sharing one cause
look like one problem, so if three clear and one does not, **the one that remains
is exactly the interesting case** — a real disagreement between a page and a
printed refusal. A partial clear must not read as success, which is why the
condition is discharged per test and not in aggregate.

**Discharged, named individually, measured after the implementation landed:**

| Test | Colour |
|---|---|
| `every_refusal_the_modding_pages_quote_is_a_refusal_the_client_prints` | PASS |
| `a_page_that_quotes_no_refusal_at_all_is_reported_as_quoting_none` | PASS |
| `a_quoted_refusal_altered_to_text_the_client_never_prints_is_reported_with_both_sides` | PASS |
| `pages_quoting_the_printed_refusals_verbatim_agree_and_their_other_blocks_are_passed_over` | PASS |

No page disagrees with a printed refusal.

**What T26 owes the page is the TEMPLATE, not an instantiation of it**, and the
first draft of this note got that wrong by quoting one fixture's numbers as though
they were the sentence. The sentence is at `crates/mc-core/src/content.rs`:

```
this content needs {needed} texture layers and a session has
{LAYERS_A_SESSION_MAY_ASSIGN}; {spent} are already assigned, and relaunching
reclaims every layer retired since the client started
```

**The direction of the dependency is what makes this load-bearing, and the numbers
hide it.** `documented_refusals.rs` asserts that every refusal a **page quotes** is
one the **run prints** — page to run, not run to page. So a page committing a bare
`257 … 256 … 256` would require, forever, a run producing that exact string: change
the fixture's budget or its content and a *documentation* test reddens for a reason
that has nothing to do with the documentation being wrong.

So the page states each placeholder and what it counts — `needed`, `spent`, that
the bound is the session's and where it comes from — and **any worked example is
marked as an example and taken from a refusal the run demonstrably prints.** Never
a bare instantiation presented as the refusal. It is the derived-oracle rule applied
to prose: a number copied out of one run reads as authoritative and is only true of
that run.

### Phase 2 outcome, 2026-08-17 — T09–T12 done, 15 of 15 scenarios green

**Gate exit 0.** 343 tests in `mc-sim` + `mc-client`, 440 files measured under
`crates`, **coverage 94.14%** (regions 92.22%, 9 458 lines tracked).

**RED measured at 23 of 343**, not the 16 the phase owns. The other seven all had
one cause — a skeleton `appending` that numbered from zero and never refused — and
they are worth naming because each is a second witness the phase did not plan:
four in `documented_refusals.rs` (the layer-budget sentence a page quotes could
not be produced at all) and three staged-append falsifiers inherited from
SPEC-016. **A known-red hiding an unknown one was the risk**, and it is
discharged: all seven went green with the sixteen, so no page disagrees with a
printed refusal.

**Trap 9 was nearly missed and then recovered.** I implemented T10 and T11 in one
step and ran the goldens once, which is precisely what the trap forbids — running
them together cannot tell a wiring defect from an assignment defect. Recovered by
reverting the publication **by hand** and running the three instruments against
the assignment change alone: `launch_and_capture_agree` and both golden suites,
**5 of 5 green**. So D9 moves no committed frame, which is what its
`BTreeSet`-ascending derivation predicted, and the evidence is now isolated rather
than confounded.

**Five mutations, all measured, each biting only what it should.** Phase 2 owns
none of the twelve named green-on-arrival scenarios, so these target the phase's
load-bearing decisions instead.

| Mutation | Reddened | Left green |
|---|---|---|
| Retired keys stay live — **D6's failed design, verbatim** | FR-5.1-S2, FR-5.1-S3 | everything else, incl. FR-5.1-S5 |
| `appending` appends greedily and **succeeds** instead of refusing | FR-5.2-S1, FR-5.2-S3, all four `documented_refusals` | FR-5.2-S2, which legitimately fits |
| The serial never moves | FR-4.4-S2, FR-8.1-S1, FR-8.1-S2, FR-8.1-S4, FR-8.1-S5 | FR-4.5-S1's halves |
| The **serving** HUD republished in place of the candidate's | FR-4.5-S1's boundary half **and** its drawn half | all five serial scenarios |
| Publication moved **before** admission | FR-8.1-S5, **FR-5.1-S5** | everything else |

Three findings out of that table:

- **Trap 2 is discharged with evidence rather than argument.** The HUD mutation
  reddens exactly FR-4.5-S1's two halves and nothing else, so
  `PublishedContent.hud` is measurably load-bearing and a later reader deleting it
  as unused is caught by two independent instruments.
- **FR-5.1-S5 earned its keep, and this file predicted how.** It is recorded above
  as a weak instrument that "would catch a drift back to a mutable ledger" —
  publishing before admission *is* that drift's shape, and it is the mutation that
  reddens it. A weak instrument is not a useless one.
- **FR-8.1-S2 is not satisfiable by a publisher whose serial never moves.** That
  was the published-snapshot trap in its polarity-reversed form — "a reader that
  has not looked goes on seeing what it last observed" is the *desired* behaviour,
  so a test for it can pass because nothing ever publishes. The serial mutation
  reddens it, so the direction is closed.

**One mutation could not bite and the reason is structural, not a gap.** Appending
greedily *and still refusing* leaves the caller with an `Err` either way, because
`appending` returns a new value rather than mutating `self` — so "neither key is
appended" is unspellable rather than checked. The reachable defect is the greedy
success above, which is what was run.

### Phase 4 outcome, 2026-08-17 — T17–T19 done, 12 of 12 scenarios green

13 tests over the seven named binaries; 1 170 of 1 170 workspace-wide, `cargo fmt
--all --check` clean, `cargo clippy --workspace --all-targets --all-features -- -D
warnings` at exit 0.

**Six mutations, every one measured. Five bit; one did not, and the one that did
not is the phase's most important result.**

| Mutation | Reddened | Left green |
|---|---|---|
| `changes_geometry` → `true` unconditionally | FR-6.1-S1, FR-6.1-S4, **FR-6.2-S4** | the five whose candidate does change geometry |
| `changes_geometry` → `false` unconditionally | FR-6.1-S2, FR-6.1-S3, FR-6.1-S4, FR-6.2-S2, FR-4.1-S3 | FR-6.1-S1, FR-6.2-S4 — correctly, they expect no marking |
| `mark_every_section` marks only the lowest four sections of each column (64 of 256) | FR-6.1-S2, FR-6.1-S3, FR-6.1-S4, FR-6.2-S2 | **FR-4.1-S3** |
| `Remesher::retire` records the layers and not the serial | FR-6.2-S1, FR-6.2-S3, FR-6.2-S4 | everything outside the staleness binary |
| `judged`'s staleness comparison inverted (`==` for `!=`) | FR-6.2-S1, FR-6.2-S3, FR-6.2-S4 | same |
| **`App::serve` does not upload the reloaded texture layers at all** | **nothing — 234 of 234 in `mc-client`** | everything |

**The third row is Trap 6's subject arriving as a measurement.** Marking 64
sections instead of 256 covers every *occupied* section of the shipped world, so
**the drawn observable cannot see it**: FR-4.1-S3 — a face culled against solid
stone appearing once stone is not solid — passes against a selective rule. Only the
counting scenarios discriminate. The bound in FR-6.1-S3 is therefore held by
FR-6.1-S2, S3, S4 and FR-6.2-S2 and by nothing that looks at pixels, which is
exactly why the expected value had to be derived rather than observed. **Had 82 been
written there, all four would have agreed with it and the spec's stated bound would
have been silently abandoned.**

**Row 6 is not a gap this phase can close, and it is a real hole rather than a
formality.** With the reload's texture upload deleted outright, the whole of
`mc-client` stays green — so in the shipped client an appended layer would never
reach the device and nothing would say so. This is `testing.md`'s "nothing in this
workspace runs `App`" in its most consequential instance yet: the mutation deletes
a *user-visible* outcome, not an internal ordering. FR-4.3-S2's two halves are both
honest — one asserts the value the report hands over, one drives that value through
a real device — and **neither of them is `App`**. The mechanism was already on the
reviewer-held list before it was measured; it now has evidence attached rather than
an argument, and **it is escalated rather than filed**, because a source-level scan
of the reload path is the kind of instrument phase 6 already builds. Owner: the team
lead, with the test author.

**FR-6.2-S1 survives both geometry-predicate mutations, and that is correct.**
Supersession keys off the content serial, not off what was marked, so a reload that
changes no geometry still discards a batch in flight — the consequence T18 records
as accepted. Rows 4 and 5 are what grade it.

**The `drop(stale)` result is banked separately** and belongs to FR-6.2-S4's
repair: replacing the hand-back inside `Session::collect_remesh` with
`drop(stale.into_keys())` reddens exactly
`a_client_that_discards_a_batch_leaves_the_sections_it_would_have_meshed_waiting`
and nothing else, where the same defect one shape earlier left 3 of 3 and then 77 of
77 green. The full account is in the second §2 addition above.

**`Stale::keys()` is gone.** It had one caller, the fixture's verdict, and no lint
could report it — `pub` on a `pub` type. Removing it puts "assert *which* sections
were discarded" out of reach structurally, which is FR-6.2-S4's original defect one
level along wearing the new type. `into_keys` is `pub(crate)` with exactly one
caller, which is what makes omitting the hand-back fail the build twice over
(`unused variable: stale`, and `method into_keys is never used`). **Widening
`into_keys` to `pub` would silently lose half that guard.**

**Two files crossed the 500-line limit in this phase and both split by the seam
their own header had already named** — `session/mod.rs` at 485 plus
`session/reload.rs`, `app/mod.rs` at 482 plus `app/reload.rs` at 71, each the
crossing responsibility in a **child** module because it writes the parent type's
own fields. Recorded as as-built practice above.

**The batch-size figure was measured before it was changed.** T18 instructed that
the ~100 KB claim "becomes false" and must be updated to roughly 1.3 MB. It does
not: a whole-world batch is **44.5 KB** of packed indices across the 54 sections
that carry any, under 70 KB with palettes and map nodes, because 202 sections have
one-entry palettes needing zero bits per voxel. The original claim holds for a
reason it did not state — index-width tiering rather than section count — and
obeying the instruction literally would have written a false figure into the tree.

### Phase 5 outcome, 2026-08-17 — T20–T21 done, 7 of 7 scenarios green

7 tests over three binaries; 1 177 of 1 177 workspace-wide, `cargo fmt --all
--check` clean, `cargo clippy --workspace --all-targets --all-features -- -D
warnings` at exit 0. The test author displayed 5 of 7 failing on their own
assertions against a skeleton before any of this was written, and measured both
green-on-arrival controls themselves.

**Eight mutations, every one measured, every one bit.**

| Mutation | Reddened |
|---|---|
| `cleared` answers `MovedTo(feet + (1,0,0))` where it answers `Unneeded` | FR-7.1-S3, and only it |
| the search **also** runs on the refusal path, the accepted path untouched | **FR-7.1-S6, and only it** |
| the search runs *before* the swap, so both paths read the old solidity | FR-7.1-S1, S2, S4, S6, S5, S7 — six of seven |
| horizontal distance ranked ahead of `dy` | FR-7.1-S1, S2, S4 |
| `dy ∈ [-8, 8]` — downward added to the candidate set | FR-7.1-S1, S2, S4, S5, S7 |
| the velocity survives a clearing move | **FR-7.1-S7 only, with FR-3.2-S3 green** |
| the reach widened to 9 | FR-7.1-S5, FR-7.1-S7 |
| the reach narrowed to 7 | FR-7.1-S5, on the `blocks` the verdict carries |

**Rows 2 and 3 are the same defect at two sharpnesses, and the sharp one is the
control.** Moving the call before the swap reddens six scenarios, which tells you
only that something is wrong; adding a second call on the refusal path and leaving
the accepted path alone reddens **exactly FR-7.1-S6**. That is what T21 asked for,
and it is only possible because the test author's fixture gives the refusal-path
call work to do — see the amendment to T21 below.

**Row 6 is the pair the mutation table asked for, in one run.** Dropping the
velocity zeroing reddens FR-7.1-S7 and leaves FR-3.2-S3 green, so the blanket rule
demonstrably does not reach a player who was *not* cleared.

**Rows 7 and 8 bound the reach from both sides**, which is what makes 8 a measured
value rather than a copied one: too wide and the wedge finds a cell, too narrow and
the verdict carries the wrong number. Nothing asserts a count of positions tested,
per T20's second statement of Trap 6.

**T20's promotion could not be taken literally, and the substitute is stronger.**
`collide::overlaps` takes an `Aabb`, which is private to `collide.rs`, so promoting
that function alone would have meant promoting the box type and its constructor as
well — three items, and the box's shape leaving the physics. What was promoted
instead is **`collide::overlaps_solid(feet, world)`**, which is the whole question
in one call: the `0.6 × 1.8 × 0.6` shape, the half-open `[v, v + 1)` rule and the
solidity lookup stay stated once, `Aabb` stays private, and `clearing.rs` restates
none of it. `collide::cell_of` came with it so the floor rule is read rather than
rewritten too.

### Amendments phase 5's test author earned, each with a measurement

**T21's diagnosis of FR-7.1-S6 was necessary and not sufficient.** T21 said the
mutation fails to bite only if the test is "not driving a refused candidate that
would have made the player's cell solid". There is a second way: **on the refusal
path `World::adopt` never ran**, so a search called there reads the solidity the
world *still has*, and the candidate's solidity exists nowhere for it to read. A
player whom only the candidate would have trapped overlaps nothing there, the
search answers `Unneeded`, and the mutation leaves the test green. Measured: with
the refusal-path mutation live and the naive fixture, 2 of 2 passed. The fixture
therefore stands the player with their head **already** inside stone and their feet
in the water the candidate would make solid — the stone gives the mutation work to
do, the water makes the candidate the one the scenario describes — and
`require_a_refusal_could_have_moved_them` asserts both halves.

**FR-7.1-S7's stated rationale is untestable and its requirement is.** The
scenario's reason — "the next tick does not carry them straight back into the cell
they were moved out of" — reads as a *downward* velocity, and a downward velocity
cannot carry the signal: every upward destination the search takes is a cell whose
floor is the thing that blocked the candidate one `dy` lower, so a cleared player
always lands supported and that tick's own collision spends a fall either way. Only
a rise survives. The fixture's player is therefore mid-jump. A note about the
spec's rationale, not a change to what it demands.

### OPEN QUESTION — a cleared player may be put outside the loaded world

**CLOSED in phase 8** — ruled in phase 7, implemented as an eligibility rule in the
clearing search alone. See *"Phase 8 outcome"* below. Kept as it was written, because
what the workaround was and why it was invisible is the reasoning the archive exists
to preserve.

**Found by phase 5's test author, unspecified in `spec.md` and in D11, and not
gradeable by any scenario this spec has.** `SolidVoxels::is_solid` answers `false`
for every position past the footprint by construction, so a position outside the
world is **clear**, and it is a position the search will take. The search reaches 8
blocks horizontally and the shipped footprint is 64 blocks square, so **every player
in it has candidates outside it**; in a wedge those are the first clear ones a
search finds.

The author worked around it rather than papering over it — FR-7.1-S5 uses a
two-column, 32-block world so the whole cube lies inside — and said so. **No
scenario grades it and this phase cannot invent one**, because what should happen is
a product decision: refuse such candidates, treat outside as solid, or accept that a
reload can put a player off the map. Owner: the team lead. It is recorded here
rather than left in a message because the workaround is invisible in the test that
carries it.

### The upload obligation, landed in phase 4 rather than deferred to a scan

**RULING (team lead): the gap phase 4's sixth mutation found may not stay
reviewer-held, and a compiler-held obligation is preferred to a source-level scan
wherever one is expressible.** The distinction the lead drew, so it is not lost: a
scan was refused for the `Superseded` gap because a *superior* instrument existed
there; it is not refused as such, and it beats no instrument at all for a silent,
player-visible failure.

One was expressible. `crates/mc-client/src/upload.rs` holds `Unuploaded`:
`stated()` borrows the layers for the assertion on what crosses the boundary, and
**`uploaded_to` consumes and is the only route to an owned `TextureLayers`** — which
is what `Remesher::retire` needs. Both realistic spellings of the omission fail the
build: passing the wrapper is `E0308`, and deleting the upload line leaves `E0425`
on a name that no longer exists.

**What decided the siting, recorded because the obvious place is wrong.** Putting
the obligation on `retire` — a witness type only an upload can produce — would have
broken `reload_supersedes_a_batch_in_flight.rs`, which drives `retire` legitimately
and has **no device**. Giving that type a device-free constructor destroys the
obligation. So the wrapper had to sit at the report boundary, which cost exactly one
line in one fixture.

**The residual hole is written into `upload.rs`'s header, not left in a report.**
`TextureLayers` is `Clone`, so cloning the borrow and retiring that compiles. The
polarity is what makes it acceptable: for an upload the realistic defect is an
omission or a reorder during a refactor, which the compiler covers — the reverse of
`Stale`, where a deliberate discard was plausible and the test half was therefore
owed.

**Still not covered, and it is a smaller sibling rather than the same hole:** `App`
forgetting `retire` altogether, and `App` printing the clearing verdict. Both are
diagnostics or a stale pack rather than a wrong picture of moved content.

### Phase 6 outcome, 2026-08-17 — T22–T25 done, 5 of 5 scenarios green; T26 all but its quoted refusals

16 tests over six binaries, all green. Told apart as this phase requires: **green once
their instrument compiled** — FR-8.2-S1, FR-8.2-S2, FR-8.2-S3, FR-9.1-S2. **Green on
arrival in the ordinary sense** — FR-9.1-S1 only. Every positive control was run by
breaking the scan by hand and reverted by hand.

**This phase found the spec's largest defect, with an instrument no task asked for.**

`Session::attach_reload` had **no caller outside test fixtures.**
`App::collect_preparation` attached a simulation and spawned the re-mesh worker and
attached no reload; `main::run` moved its content root into the preparation and kept no
copy. So 86 of 91 scenarios were green, five phases of machinery were correct and
tested, and **`cargo run -p mc-client` watched nothing.** The capability the spec opens
with — leave the client running, edit `stone.luau`, save, walk through stone — was
unreachable by the person it is for.

Fixed rather than ruled out of scope, because Key Principle 7 does not leave that open:
`PreparedLaunch` hands back the root it was prepared from, which is the only place "the
root being played" and "the root being watched" are the same value by construction, and
`collect_preparation` puts it under watch once the simulation is attached.

> **An absence guard over a seam is satisfied by a product that never reaches the seam
> at all, so it owes a presence guard over the wiring beside it.**

Two notes from writing the second guard, both in `docs/technical/testing.md`: spell a
needle as a **call** rather than a name, or it is satisfied by the method's own
definition in the crate being scanned; and a presence assertion needs no positive
control and does need the could-not-look arm.

**And the wiring was still inert after that fix, for a second and worse reason — found
by the owner playing the game.** `notify` reports the paths the platform gives it, which
are absolute. `mc_sim::content::shipped_directory` is relative, and it travelled through
`PreparedLaunch::root` unchanged. The relevance rule compared the two **as written**, so
every save was classified as not content, no attempt began, **and no refusal was printed
either** — from the domain's point of view nothing had changed.

**Every fixture over that path was correct and that is what hid it.** All of them built
their root with `tempfile`, which is absolute, so the two spellings always agreed and
1 188 tests were green over a no-op.

> **A fixture that supplies an argument in a form no caller uses tests a contract nobody
> has.** A port whose contract holds for one spelling of its argument has no contract; it
> has a habit.

**And this is the sharper reading of the presence guard above.** *Policy is not wiring*
one layer down: the wiring landed and the thing it wires was a no-op, so a guard that
says the call exists does not say the call does anything. Recorded in
`docs/technical/testing.md`, which is also where the third consequence goes — **the
repair belongs in the rule, not at the call site.** Canonicalising the root where the
client builds its watcher would have fixed the client and left the port exactly as
narrow, and the author's test rejects that repair by construction by handing the root to
the *adapter* in three spellings.

The rule now asks whether the saved file's own parent is the declared directory,
comparing as written first and asking the filesystem only when that fails, with both
sides canonicalised or neither. The **parent** is canonicalised rather than the file,
because a removal reports a path that no longer exists and its directory still does.

**The closing condition is instrumented, not escalated, and by identity and ordering.**
A build injected through `ContentReload`'s own door announces the `ThreadId` it began
on; the test crosses a second tick boundary while the build is still deliberately
blocked. Two mutations, each reddening that test alone:

| Mutation | Verdict read |
|---|---|
| the build runs **inline** | `OnTheTickingThread` |
| the collect **joins** the builder instead of polling | `SomewhereElseButATickWaitedForIt` |

**The inline mutation is the result worth keeping.** All four `reload_builds_off_the_tick`
tests stayed green under it, `require_a_build_in_flight` included — **an in-flight guard
cannot see a build that ran inline on the tick thread**, provided its outcome is
reported at the following boundary. That is what the closing condition was for.

**T25 was checked off on half of itself, and finding 6 is exactly why.** Its first half
— the deterministic guard that the re-mesh runs off the tick thread — was delivered by
the test author and reported to me, and I checked the task on that report. Its second
half is *"the latency is carried by a `criterion` benchmark run as a standalone
command"*, and **no such benchmark existed** — while `docs/technical/testing.md`, which
I had just written, claimed one did. A report of a change and the change are separate
artifacts, and I wrote the literal without looking at `crates/*/benches/`.

`crates/mc-sim/benches/reload.rs` now exists, following the meshing benchmark's three
steps. **Measured: candidate build 0.8 ms, whole-world re-mesh of 256 sections 9.1 ms,
9.9 ms against an 850 ms engine share** — the one-second target less the settling
window, computed from the two constants rather than written as a third. Criterion's own
estimate for the build is 1.79 ms over the same work; as next door, two numbers exist
for one workload and only the run's own mean decides anything.

**One deviation from T24's stated verdict shape, wider rather than narrower:**
`RootsThatContributedNothing([..])` in place of `NoSourceWasRead`, because a total
cannot see one member root going quiet beside hundreds of files in another.

### T26 — what landed, and the one thing still outstanding

Landed: `docs/modding/hot-reload.md` (new, all of the mod-author contract), the player
section in `docs/user/gameplay.md`, the reload seam in `docs/technical/architecture.md`,
the layer policy and the marking rule with its measurement in
`docs/technical/rendering.md`, ADR-024 and ADR-025, the seven findings and four new
entries in `docs/technical/testing.md`, the new
**`docs/technical/working-in-this-repo.md`** carrying the operational items, `INDEX.md`
registration and Sources, and the three `CLAUDE.md` corrections. Both in-tree comments
this spec falsified were rewritten rather than left standing.

**Landed after the fact, and now complete: the four reload refusals quoted verbatim.** `documented_refusals.rs`
recognises any fenced block in `docs/modding/` whose first line begins `mycraft: ` and
matches it against `printed_refusals()`, which produces four *launch and budget*
refusals. **So a reload refusal quoted on the page fails the guard until it is
produced** — the page-follows-run dependency working exactly as its own header says it
should. The four requested are the reload's outer sentence over: a broken declaration, a
candidate that stops declaring a block the world holds, one registering no solid block,
and the layer budget as a *session* meets it. `RootUnwatchable` and `BuilderLost` are
deliberately **not** requested: neither is an authoring mistake, so the page states them
in prose, which keeps them out of the recogniser on purpose.

**How the four texts were obtained is worth recording, because the guard is the
oracle.** No function hands them over — they are produced inside the test binary — so
each was found by quoting a deliberately-truncated prefix on the page and reading the
`produced` half of `Verdict::Mismatch` back, which reports the printed refusal the
quotation agrees with for longest. Four iterations, one per refusal, each converging in
a single step once the prefix diverged from its neighbour later than from any other. The
page is now held to a live run line for line.

**T26 is complete.** With these four the modding pages quote **seven** of the roughly
fourteen refusals an author can trip, against three before this spec.

### Phase 7 outcome and the owed upload mutation, 2026-08-17

Three mutations, hand-reverted one at a time with `git diff --exit-code` confirmed
between each. **Two bit. One did not, and the non-bite is a real coverage hole.**

| Mutation | Reddened | Notes |
|---|---|---|
| `uploaded_to` consumes the wrapper and returns the layers **without writing to the device** | `reload_hand_shows_the_new_block`, and only it | `strayed == 576`, `shown == 0`, against `strayed == 0`, `shown == 2` |
| `is_the_same_directory` loses its filesystem fallback (the written comparison alone) | `content_watch_root_forms` **and** `reload_takes_up_a_save_under_a_relative_root` | the absolute form still passes; both other forms fail |
| the rule accepts a declaration nested **one directory deeper** | **nothing** — 316 of 316 in `mc-world`, 87 of 87 in the reload family | see below |

**The upload mutation matched its prediction to the number**, which is the measurement
that had never been run: `strayed == 576`, `shown == 0`. It reddens **only** the held-block
scenario, and the reason the other two are untouched is the test author's own analysis —
the packer wants a borrow, and the drawn half uploads to a `TerrainRenderer`, which
`uploaded_to` does not take. So FR-4.3-S3 grades the *upload* and not merely obtaining
the value, which is what the obligation was added to make true.

**The relativisation mutation's output is the vendor evidence, read rather than
argued.** The two spellings that failed are the two relative ones, and the shape of each
is the point: the crate's own directory,
then the caller's spelling of the root **embedded verbatim mid-path** — a
backslash-separated parent-directory pair in one, and a `./` with forward slashes in the
other — then the platform's separators again for the trailing components. That is why
neither joining onto the working directory nor `fs::canonicalize` is the repair: the
first goes green on the plain relative form and stays red on the dotted one, and the
second produces a verbatim form the vendor never reports. The `blocks` **directory**
appears in the refused list beside the file, which is the removed-directory asymmetry
visible in raw output rather than reasoned about.

### A NON-BITE THAT IS A COVERAGE HOLE — nested-deeper is stated and unwitnessed

**Nothing in the workspace refuses a declaration nested one directory deeper.** The rule
says so — `declares_content`'s own doc names "a declaration nested one directory deeper"
among what is not content — and with the rule changed to accept an ancestor rather than
the parent, **403 tests across the two crates that could see it all pass.**

Why each near neighbour misses it:

- `every_declaration_the_shipped_root_holds_is_content_and_nothing_else_under_it_is`
  walks the *shipped* root, which holds no nested declaration, so the property is true of
  its fixture by accident rather than asserted.
- `an_editors_scratch_file_beside_a_declaration_begins_no_attempt` varies the
  **extension**, not the depth.
- `content_watch_root_forms` varies the root's **spelling**, not the depth.

This is `testing.md`'s own rule — a missing test that would catch something real is added
rather than deferred — and it is a test file, so it is the test author's to write. The
fixture is cheap: write `blocks/experiments/draft.luau` under a watched root and require
no attempt. **Reported rather than left in this table**, because a non-bite recorded and
not acted on is how a stated rule becomes an unstated one.

### Phase 8 outcome, 2026-08-17 — FR-7.1-S8 green, and the OPEN QUESTION above is closed

The question recorded under *"a cleared player may be put outside the loaded world"*
was ruled in phase 7 and implemented here: **a candidate is eligible only if every
cell the player's box would cover is known and clear.** The extent the search may
consider is passed to `cleared` beside the solidity; `is_solid` is untouched; no
method was added to `Solidity`, because the fixture doubles that answer solidity for
a hand-written set of cells have no extent and would have had to invent one — a
trait is defined by all of its implementors. No new vocabulary: the boundary wedge
takes `NoClearSpaceWithin { blocks: 8 }`.

RED was observed before anything changed, as an assertion failure rather than a
compile error: `MovedOffTheMap([-0.5, 10.0, -0.5])` against
`ToldNothingWasClear { blocks: 8 }`, with the control already green.

**Three mutations, hand-reverted one at a time by re-editing the line, with
`git diff --exit-code -- crates/` confirmed clean between each. Two bit; the third
did not, and the non-bite is a coverage hole.**

| Mutation | Reddened | Notes |
|---|---|---|
| `eligible` drops `known` — the eligibility check deleted | the scenario, and only it | the control stays green, so the check is what separates the two |
| `eligible` answers `false` for every candidate | the **control**, and only it | the scenario passes vacuously, which is exactly what the control exists to catch — "outside is ineligible" implemented as "nothing is eligible" is invisible to the scenario alone |
| `holds` keeps the sign refusal but drops `ground.contains` — the world's **far** edge unguarded | **nothing** — 1 192 of 1 192 across the workspace | see below |

### A NON-BITE, PHASE 8 — only the near edge of the world is witnessed

**Every out-of-world candidate any fixture in the suite reaches has a *negative*
coordinate.** The trapped player stands at `(2.5, 10.0, 2.5)` in a world 16 blocks
square and the reach is 8, so the cube spans `[-6, 10]` on both horizontal axes: it
leaves the world at `x < 0` and `z < 0` and never at `x >= 16`. `inside_the_world`'s
sign refusal alone therefore carries the whole scenario, and `Extent::contains`, which
is the half that will still matter when a world is not indexed from the origin, is
stated and unwitnessed. With `ground.contains` short-circuited to `true` the entire
workspace passes.

It is reachable and the fixture is cheap: the same wedge at `(13.5, 10.0, 13.5)` in a
one-column world puts the cube's far side at `x = 21` against an edge at 16, with every
out-of-world candidate positive. **It is a test file, so it is the test author's to
write** — reported rather than left in this table, on phase 7's rule that a non-bite
recorded and not acted on is how a stated rule becomes an unstated one.

**Deferred observation, outside this diff.** `mc_sim::world::inside_the_world` reads as
a question about the world and answers only a question about the sign — it says so in
its own doc, but the name is what a caller sees. Renaming it touches every caller and
is not this phase's diff.

### Phase 2 — the architecture contradicted itself, and where the resolution landed

**D4 and D9 could not both hold.** D4 listed `ReloadRefusal::LayerBudget` in
`mc-sim` carrying the sentence that names the count, the 256 and the way out;
D9 puts the appending **inside `load`**, whose error is `ContentError`, so the
refusal originates a level below that enum. A `ReloadRefusal::LayerBudget` would
therefore have spelled that sentence twice — once where it is raised and once
where it is re-wrapped — which the spec's Declared Quantities section forbids by
name. Phase 2's test author found it as the feature's first consumer.

**Resolution, and `architecture.md`'s D4 is amended rather than only this file:**
the sentence lives on `LayerBudget` in `mc-core`, beside the budget it names;
`ContentError` gains `Layers(#[from] LayerBudget)` with no wording of its own,
following `Blocks` and `Hud`; `ReloadRefusal::Content` reaches it through that.
**`mc-sim` then never names the count at all, which is D5's whole point.**
`mc-client`'s `PreparationError` gains a transparent `Layers` variant because
`From<ContentError>` flattens and the match must be total — unreachable from a
launch, which has spent nothing, and it says so.

**A second signature change D12 forced and nothing had costed.**
`simulation_at_launch` was already **at** the four-argument limit, which is
constitutional and has no `#[allow]`, and `content` is the fifth thing a launch
takes. So the four arguments beyond the save become `Launching { seed, registry,
content, accepting }`; `simulation_for` grows a third argument; and
`mc_client::launch::simulation_to_play`, also already at four, becomes
`(save, Launching)`. Fourteen test call sites, all the test author's.

**A constraint on the SPEC-016 falsifier that D9 creates and nobody noted.**
`LayerAssignment` is constructible only by `none()` and `appending()`, so
**an arbitrary pair list is no longer expressible** — and
`stated_layers_are_honoured.rs` exists to hand a reader an assignment that is
deliberately *not* the positional order. It is still expressible, by **staged
appends**: append one key, then append that key together with a
lexicographically-earlier one, and the first keeps the lower layer. That is how a
non-lexicographic assignment arises in production, so the falsifier now asserts a
state a session can actually be in, which a hand-written pair list never did.

### CLOSING CONDITION ON PHASE 3 — not an inherited observation

**Nothing in phase 2 grades that the *product* reads the serving assignment
rather than `LayerAssignment::none()`.** In phase 2 the test hands it in, because
the worker that will do it is phase 3's; everything downstream of that argument
is graded and the argument itself is not. That is `testing.md`'s "policy is not
wiring" in its purest form, and it is the exact shape that left 406 tests green
over a client submitting a default intent every tick.

**RULING: phase 3 does not close until something goes red when the product reads
`LayerAssignment::none()` in place of the serving assignment.** A closing
condition gets checked; an observation gets read. The mutation is named: have the
reload's build stage pass `LayerAssignment::none()` to `load` and require a phase-3
scenario to redden. The fixture door phase 2 leaves behind is
`crates/mc-client/tests/support/reload_content.rs::candidate_against`.

**If phase 3's own scenarios cannot express it, that is reported and a test is
added** — `testing.md` is explicit that a missing test which would catch
something real gets added rather than deferred to a phase that happens to revisit
the path. Two phases of this project have shipped a known hole for exactly that
reason.

### Four unreachability arguments, and only two of them earn an instrument

Recorded because otherwise a later phase concludes that every unreachable branch
wants a test, and that conclusion is wrong.

| Branch | What it guards | Verdict |
|---|---|---|
| **D3** — `World::adopt` writing the registry before resolving solidity | **State.** A wrong ordering leaves the world named against a registry that cannot answer for a block it holds, and it becomes reachable the day `names_held` regresses. There is a behaviour to witness and a consequence to prevent. | **Witnessed** — `crates/mc-sim/src/world/mod_test.rs`, two guards, both mutations measured |
| **D15** — an upload failure after an accepted swap ends the run | **A classification.** Its safety rests on sorting failure modes into content-caused and not, which is a claim about the world rather than about the type system — and claims about the world can be wrong. | **Interrogated in phase 4**, see below |
| **`PreparationError::Layers`** — a launch meeting the layer budget | **Nothing.** The variant exists because `From<ContentError>` flattens and a match must be total. There is no behaviour there to go wrong silently. | **No test, and a later phase offering one is refused** |
| **`Collecting::WorkerGone`** — the re-mesh worker's channel gone, and `App`'s report of it | **A silent stop.** A worker that dies makes the client stop drawing edits for the rest of the run, and before this arm existed nothing said so. Defect-reachable: a panic inside the worker disconnects the channel. | **Kept, unwitnessed, and reviewer-held** — added in phase 7, see below |

**`Collecting::WorkerGone` is defect-reachable and fixture-unreachable, and those are
different facts.** `rebuild_batches` ends on exactly two conditions — `work.recv()`
failing, which needs the `Remesher`'s sender dropped, and `finished.send(...)` failing,
which needs its receiver dropped — and **both are the `Remesher` itself being dropped.**
So no test can hold a live `Remesher` and ask it about a dead worker. The only remaining
route is a panic inside the worker, and crate-wide `indexing_slicing` denial plus
`Result`-returning steps in `rebuilt` make one un-inducible from a fixture.

- **What holds it:** `App`'s match over `Remeshing` is exhaustive with no wildcard, so
  the arm cannot be silently deleted — removing it fails the build.
- **What is unheld, and is a reviewer's:** that the report is *reached*, and that its
  wording is right. Named here so a reviewer knows it is theirs rather than assuming a
  test has it.
- **Why it was still worth adding in the last phase:** it converts a silent, permanent,
  player-visible stop into a reported one, and it makes the test author's
  `TheWorkerIsGone` a millisecond answer from the channel rather than a fifteen-second
  inference from `is_free()`.
- **The seam that would make it witnessable is on PRO-949**, not here: an injectable
  worker in the shape of `ContentReload::building` — changing *what* the worker does and
  never *where*. A production door added to be observed is a real technique with
  precedent in this spec, and it belongs beside a reading of a real occurrence rather
  than rushed in alongside the arm it would test.

**Its kinship with `PreparationError::Layers` is instructive and they are not the same
case.** That variant guards *nothing* — there is no behaviour to go wrong. This one
guards a behaviour, and is unreachable only by the instruments available. The
discriminator that separates them is the one this table exists for: **what would go
wrong silently, not how hard it is to reach.**

**Reaching `PreparationError::Layers` would need a launch fixture in a state a
launch cannot be in, which tests the fixture.** Note the symmetry with phase 1,
where that same objection was raised about D3 and was *wrong* because the state
there was genuinely reachable. The objection is sound here and unsound there, and
the difference is whether a real defect can put the program in that state.

### Mechanisms that remain reviewer-held, and therefore owe prose in `docs/` (T26)

**RULING 3: `tasks.md` is archived at completion and is history, not
documentation.** `docs/` is the only as-built record, so every mechanism still
held by review lands in `docs/technical/` or a crate `CLAUDE.md` before this spec
closes, each with the reason it matters and what a future change must not break.
The D3 ordering has left this list. What remains on it:

| Mechanism | Phase | Where its prose goes |
|---|---|---|
| `World::adopt` staying module-private rather than `pub(crate)` — the compiler plus a reviewer reading the rewritten header | 1, done | `docs/technical/architecture.md`, the reload seam |
| `App` assigning the published `HudLayout` to its own `hud` field | 2, **done** | `docs/technical/architecture.md`, in the reload seam's three-assignments paragraph — grouped with the clearing verdict and the layer retirement, because all three are assignments of arrived values and share one reason |
| ~~`App` uploading the appended layers~~ | 4 | **OFF THIS LIST.** Measured, not argued: deleting the upload left 234 of 234 green, so it stopped being a mechanism a reviewer can hold. Now compiler-held by `Unuploaded` — see the ruling below. What remains reviewer-held in its place is `App` forgetting `retire`, which costs a stale pack rather than content the device never received |
| `App` turning `ReloadReport::Accepted`'s `Clearing` into a line a person reads | 5 | `docs/technical/architecture.md`. Phase 4's smaller sibling: what is lost is a diagnostic, not a user-visible outcome, and the halves either side are covered — the verdict at the boundary by FR-7.1-S5, the rendering of a chain by the existing assertions |
| **D15** — an upload failure after an accepted swap ends the run, its safety resting on the one content-caused failure being unreachable (T09's compile-time assertion) | 4 | `docs/technical/architecture.md` and `docs/technical/rendering.md` |
| The one-second end-to-end target — a real editor, a real save, a real window | 6 | `docs/technical/testing.md`, as a **named manual acceptance check** |
| `notify` never escaping `notify_watch.rs` — the manifest half is structural, the litmus test is prose | 3 | `docs/technical/architecture.md` (the Boundaries litmus: if `notify` disappeared tomorrow, one file changes) |
| The `remesh.rs` batch-size figure's new derivation | 4 | the comment itself, confirmed against the tree in T18 |
| **The all-or-nothing marking rule and the measurement behind it** — a selective rule marks about 82 of 256 in the shipped world and fails the spec's bound, and what binary marking adds is the empty sections | 4 | `docs/technical/rendering.md`. The predicate's own comment states the rule and the figure; the *terrain measurement it rests on* (highest occupied section 3 in fifteen columns, 4 in one) belongs beside the world's shape rather than in a `mc-sim` doc comment |
| **`LayerAssignment::appending` returns a new value rather than mutating `self`**, so a partial append is not merely unchecked — it **cannot be written**. Measured: the greedy-but-still-refusing mutation leaves the caller an `Err` either way and reddens nothing. | 2, done | `docs/technical/architecture.md`, beside the layer policy. **A future refactor to `&mut self` would destroy this silently while every test stayed green**, which is exactly why it needs prose rather than a line in a mutation table |

**Discharged rather than carried:** the whole-world solidity resolve's cost
(measured in T02, 5.0 ms) and `Simulation::new`'s third parameter reaching every
construction site (the compiler, in T11).

### EIGHT findings owed to `docs/technical/testing.md` at T26

**None of these is a fact about hot reload, which is why none of them may stay in
this file.** `tasks.md` is archived; `docs/` is the only as-built record. Each is
landed in **general terms with its worked example subordinate**, never as an
anecdote about the scenario that exposed it.

**1 — A channel that does not update cannot report what happened through it.**

> A swap that publishes no tick of its own means every assertion reading the
> published snapshot is blind to what the swap did — an observation taken through
> a channel the operation does not update cannot see the operation, and a control
> half can keep passing under both mutations while the headline claim is
> unfalsifiable.

The actionable half: this project's publication seam is deliberately a pointer
swap that **only `advance` performs**, so every future test of something that
mutates state *between* ticks has this trap under it — **phase 5's clearing search
first.** Found by mutation on FR-3.2-S1 and FR-3.2-S3, both of which were green
and vacuous.

**2 — When an unreachable branch earns a witness, and when it does not.** The
three-way table above. The load-bearing sentence is the discriminator —
**whether a real defect can put the program in that state** — and the three cases
are the worked examples rather than the subject. Keep the symmetry note: the same
"that tests a fixture" objection is *sound* for one of them and *unsound* for
another, and which it is depends on reachability rather than on how the test would
look. **It took three cases side by side to become visible**, which is exactly the
kind of rule that is expensive to re-derive and cheap to write down once — a
future reader with one unreachable branch in front of them has no way to reach it.

**3 — A multi-crate `-D warnings` run names only the crate that failed first.**
Not every crate that would fail. `mc-sim`'s test support tripped an unreachable
pattern and the build aborted before `mc-client`'s twin of the same fixture was
checked, so fixing what clippy named would have met the identical diagnostic in a
second crate on the next gate run, reading as a new defect. **When you fix a lint,
look for the same defect in the sibling crate's copy.** Same shape as the existing
"read a multi-stage gate failure for one cause before reading it as several", with
the sign flipped: one cause, reported in one of the several places it lives.

**4 — A count is not a membership, and the lesson arrived four times in one
spec.** Once as scenario counts, once as construction-site counts, once as a
refusal template quoted as one fixture's instantiation, and once as a batch-size
figure. Whatever number reaches `docs/` is the **measured** one, never an estimate
that happened to survive review: a reader cannot tell an estimate from a
measurement, so an estimate in as-built documentation is worse than no number at
all.

**The fourth surface has a twist worth stating separately, because obeying the
instruction would have *created* the defect.** `architecture.md` told T18 to
replace `remesh.rs`'s "well under 100 KB" with ~1.3 MB, on the premise that a
whole-world batch makes the claim false. Measured, the batch is **44.5 KB** — 54 of
256 sections carry any packed indices, because a one-entry palette needs zero bits
per voxel and the shipped world is uniform above and below the terrain. **The claim
does not become false, and the replacement figure was wrong by an order of
magnitude.**

A wrong count reports something false. **A wrong instruction premised on a
falsehood makes a faithful implementer write the falsehood into the code — and the
more disciplined they are, the more certainly it lands.** So:

> **An instruction to update a figure is itself a claim, and it is measured before
> it is obeyed.**

**5 — An observation is annotated, never amended.** A measurement carries the
commit it was taken at, and a dated measurement that turns out to be superseded is
**still evidence** — rewriting the number destroys the only thing that made it
evidence, and makes the document claim an observation nobody took. Annotate with
the current fact and leave the original standing. (This one was not in this file
when it was first reported as recorded; added on the second pass, which is item 6
below and is left visible rather than backfilled — the file demonstrating its own
rule.)

**6 — A report of a change and the change are separate artifacts, and only one of
them a future reader can act on.** *"I wrote it in a message" and "it is in the
tree" are different facts.* The check that closes it: **read the file before
reporting anything as recorded** — not memory, and not the message you wrote about
it.

This belongs in §2 beside *an absent reviewer and a clean reviewer look
identical*, because it is the same family: verification about verification rather
than about code.

**It has occurred in both directions in this MVP, which is why it is a project
rule and not a remark about one agent's diligence.** Upward, as a report claiming
a note had landed in `tasks.md` when it had only been written in a message.
Downward into briefs, three times and worse, because a brief is acted on: a file
path relayed without checking it existed, a scenario count whose *membership* was
wrong, and a preference stated as settled when it had already been ruled the other
way. **The tell was identical every time — somebody was writing a literal they had
not looked at.**

**Its kinship with item 4 is the part that makes the pattern visible**, and
neither one alone shows it: a **count** survives being wrong about every element;
a **report** survives being wrong about whether the write landed. Both are
summaries that read as evidence and are not, and the defence in both cases is to
go and look at the thing rather than at the summary of it.

**7 — An instruction that names an action without naming the discrimination it
serves will be satisfied by the action alone.**

Trap 9 told phase 2 to run the goldens *and why*: that running them after the
assignment change **and** the publication together cannot tell a wiring defect
from an assignment defect. The implementer collapsed T10 and T11 into one step and
ran them once. What made the shortfall visible afterwards was the stated purpose,
not the named instrument — in the implementer's own words, **had the trap only
said "run the goldens", I would have run them and reported a pass.**

So every trap, task and closing condition **states the discrimination, not just the
command.** It is cheap to comply with and this project writes a great many of them.

**Its kinship with item 6 is the sharpest one on this list, because it is the same
error turned inward.** Item 6 is that a report of a change and the change are
separate artifacts. Here the implementer had **briefed the phase's test author on
that very ordering** and then collapsed it personally. Explaining a rule is not
applying it — and that is worse than merely forgetting it, because having just
explained it feels like having done it. Same defence as item 6: go and look at the
thing rather than at your account of it.

**9 — A report about one half of a two-part task is not a report about the task, and it
passed two people who were both watching for exactly that.**

**RULING (team lead): this goes on the numbered list**, which therefore closes at nine
rather than eight. Not because the omission was serious — it was one half of one task —
but because of what it demonstrates.

T25 has two halves: a deterministic guard that the re-mesh runs off the tick thread, and
*"the latency is carried by a `criterion` benchmark run as a standalone command"*. The
test author delivered the first and reported it. **The implementer checked the task on
that report, and then wrote into `docs/technical/testing.md` that the benchmark existed.
It did not.** The team lead accepted the same checkmark on the same report and did not
open `benches/` either.

- **This is item 6 turned on its own author.** The implementer wrote item 6 — *a report
  of a change and the change are separate artifacts; read the file before reporting
  anything as recorded* — and then wrote a literal about `benches/` without opening it,
  within the hour. Same tell as every previous instance.
- **And it is item 7's mechanism inward, for the second time**: an instruction naming two
  deliverables was satisfied by a report about one of them, because the report was about
  the half that had a test attached.
- **The defence is the one already stated for mutation results: per item, never in
  aggregate.** A two-part task takes two answers. That rule was being applied to
  mutations in the same messages in which it was not being applied to task completion.

The benchmark now exists — `crates/mc-sim/benches/reload.rs`, build 0.8 ms, whole-world
re-mesh 9.1 ms, 9.9 ms against an 850 ms derived budget — and `docs/` carries the
machine as well as the date, because a benchmark that gates nothing leaves its
documented figures as the only baseline anybody will have.

### The eighth finding, owed to `docs/technical/` and deliberately NOT to `testing.md`

**Kept separate on purpose.** The six above are rules about *evidence*; this one
is as-built reality about how this repository is worked, which is normally by
several agents at once. Folding an operational rule into that page blurs what the
page is for.

> **In a shared tree, an unexpected failure is a question about who else is
> working before it is a question about what is broken.**
>
> And the operational half, which is what somebody needs at 3am: **never remove a
> live `.git/index.lock`. Wait, then retry.**

Three measured instances in this spec, each a foreign action wearing the costume
of a local defect:

| What was seen | What it actually was |
|---|---|
| A test failing, then passing on re-run — a flake | Another agent's mutation live in the tree during the run |
| The gate red on a stage that had never fired | Another agent's untracked files inside a directory a guard scans |
| `fatal: Unable to create '.git/index.lock': File exists`, whose own text invites removing it | Another agent mid-commit; the lock cleared in one second |
| Shipped content in the tree with `base:stone` declared non-solid — a content regression | Somebody's hand-run of this spec's own manual acceptance check, left in the tree |

**A fifth, and it is the fourth's sharper form: an acceptance run *in flight during a
gate run* rather than left behind.** The gate reported `hud_goldens` mismatching by
**706 743 of 921 600 pixels, worst distance 66**, plus `edit_geometry`'s own control
failing — both of which read as a catastrophic rendering regression with total
confidence, because the content the gate read genuinely was different. What diagnosed
it: both tests passing in isolation, `git diff -- content/base/` clean at both ends,
and **mtimes on three block declarations inside the window the gate ran in.** A file
edited and reverted leaves no diff and does leave a timestamp.

> **A manual acceptance check of hot reload is, by construction, a mutation of shipped
> content in a shared tree — and the gate cannot tell it from a regression.**

So that check needs an announced window exactly as a mutation window does. `testing.md`'s
manual-acceptance entry said "put the file back afterwards", which is necessary and, as
this shows, nowhere near sufficient. Both pages now carry the rule.

**The fourth wears a new costume and that is why it is listed.** The first three are a
foreign *action* mistaken for a local defect; this is a foreign **acceptance run**
mistaken for a content regression — and the edit is one the spec's own walkthrough
asks a person to make, so it looks exactly like documentation being followed
correctly. Reverted by hand, `git diff --exit-code -- content/base/` confirmed clean,
and the owner told rather than the revert left silent. Worth knowing: the gate would
have caught it, because golden frames cull against solidity.

The third is the dangerous one, because **the error message recommends the
destructive move**: removing a live lock corrupts somebody else's in-flight commit
rather than recovering your own.

`standards/global/git-workflow.md` §2 is arguably the truer home — its staging
rules exist for this exact hazard — but that file is constitutional and amending
it needs the owner's approval. **It is not touched here**; the team lead puts it to
the owner at completion.

**A fourth instance, and it is the same hazard one step past the rule that was
obeyed.** §2 bans `git add -A` and `git add .`; both agents obeyed it and a file
still crossed the boundary. A rename of `crates/mc-client/src/app.rs` to
`src/app/mod.rs`, staged by the implementation, rode along in the test author's
commit `7dfaed1` — recorded there with zero content change.

> **`git add <paths>` is not explicit-path staging when the index is shared.**
> `git commit` commits *the index*, so anything a concurrent agent staged rides
> along. The safe form is **`git commit -- <paths>`** (implied `--only`), which
> commits only those paths and leaves another agent's staged file in the index
> untouched.

Measured in a scratch repository rather than assumed: with `theirs.txt` staged by
someone else, `git commit -- mine.txt` committed one file and left `theirs.txt`
staged. This has the same disposition as the note above — `git-workflow.md` §2 is
the truer home, it is constitutional, and the team lead puts it to the owner with
the rest.

**The recovery matters as much as the rule**, because the instinct is to unpick.
The rename was left where it landed: the branch squash-merges, so attribution to a
commit on the branch is immaterial to `main`, and unpicking means reaching into
another agent's index for no durable gain. **Confirm the content survived, then
commit on top** — `git show --stat` reading `| 0` for the moved path is what
established that nothing of the implementation's was lost.

### A green clippy is no evidence about rustdoc — a fourth `docs/technical/` item

The same shape as `testing.md` §2's "a green suite is no evidence about a lint", one
instrument along, and found the hard way at phase 5's boundary: `cargo fmt --check`
clean, `cargo clippy --workspace --all-targets --all-features -- -D warnings` at
exit 0, 1 177 of 1 177 tests passing, and **the gate's rustdoc stage red.**

> **A module carrying both an outer `///` doc on its `mod` declaration and an inner
> `//!` header has the whole block's links resolved in the *parent's* scope.** So a
> bare local name in the header — `` [`Unuploaded::uploaded_to`] `` — is
> unresolvable, while the identical link inside an item's own doc resolves fine. Use
> a fully-qualified path or plain backticks in a module header.

Two things make it worth a page rather than a shrug. **The failure is invisible to
every instrument an edit loop runs** — only `cargo doc --workspace --no-deps` under
`-D rustdoc::broken_intra_doc_links` reports it. And **the error message points at
the header, not at the `mod` line**, so the natural reading is that the link is
wrong rather than that the scope is somewhere else.

### A `pub` item on a `pub` type is invisible to dead-code lints — the third `docs/technical/` item

Sited with the eighth finding for the same reason as the pattern below: it is how
this repository is worked rather than a rule about evidence. **RULING (team lead):
the most broadly useful of the three.**

> **At a crate's public boundary, "does anything actually call this?" is a question
> only a reader can answer.** No dead-code lint reports a `pub` item on a `pub`
> type, however few callers it has — so "a test finds it handy" survives
> unchallenged by construction.

Measured on `Stale::keys()`: clippy `-D warnings` is clean both with it and without,
and its only caller was a fixture. It went, because the value it handed a test was
the value the scenario was about — an assertion on *which* sections were discarded
in place of one on their being meshed again, which is FR-6.2-S4's original defect
one level along wearing the new type.

**A second instance outside this spec is what makes it a pattern rather than an
anecdote.** `ContentView::is_solid` in the same crate is a `pub` accessor with no
production caller and two test files asserting against it, and neither a lint nor
coverage flagged it — **coverage saw it exercised.** That one belongs to the
composition-root issue; what is recorded here is the shape, not the issue.

### The header-names-its-own-seam pattern, as-built practice — also NOT a ninth item

Sited with the eighth finding rather than on the numbered list: it is how this
repository is worked, not a rule about evidence. `code-quality.md` §2 already says
to split by responsibility when a size limit is exceeded, so this needs no
constitutional change — it records only how this project *finds* the
responsibility.

> **When a file approaches its size limit, read its own header for the seam before
> inventing one.** A header written to explain the file to a reader has usually
> already drawn the line the limit is about to force.

Twice in this spec — `session.rs` and `app.rs` — the boundary was named in the
file's header before anyone went looking, which is why both splits came out clean
rather than arbitrary. Each became a directory with the crossing responsibility in
a **child** module, because the extracted code writes the parent type's own
fields.

### An addition to `docs/technical/testing.md` §2's existing material — NOT a ninth item

**RULING: this belongs in §2's absence-assertion material rather than on the
numbered list**, which stays closed at eight. §2 already carries two faces of the
same subject — a structural-invariant test needing a positive control, and an
enumerated verdict beating an absence assertion. This is a third face of it, so it
joins them.

> **An absence assertion needs a window long enough for the presence it denies.**

A bound expressed as a count of loop iterations is a proxy for elapsed time whose
conversion factor differs by orders of magnitude between a tight test loop and a
rendered frame — and it fails in **both** directions. The patience case is loud: a
run waiting for something that has not happened yet reports a failure. The
no-attempt case is silent: **three hundred tight boundaries span 30 µs against a
0.7–1.7 ms build, so FR-1.1-S6, S7, S8, S9's first half and FR-1.3-S3's first half
all reported "no attempt began" in a window too short for an attempt to have
completed** — five scenarios passing against a correct implementation for the wrong
reason.

**The fix is the model and is described as such:** both bounds denominated in
`SETTLING_WINDOW` so neither carries a number of its own, on the argument that a
build outlasting the declared window would have blown the one-second budget long
before a fixture noticed. **That is a derived bound that actually bounds
something**, which is rarer than it sounds — most derived bounds are arithmetic on
a number somebody still chose.

### A second addition to §2, beside "policy is not wiring" — the mechanism behind it

**RULING: this joins "policy is not wiring" as its mechanism, not the numbered
list.** That entry says what the failure looks like; this says how it is
manufactured, which is the half a reader can act on before writing the fixture.

> **A fixture that offers a decision as a call of its own is how a scenario about
> that decision comes to make it.**

Phase 4 measured both halves rather than arguing them. FR-6.2-S4 — a discarded
batch's sections go back among those waiting — was first built with the hand-back
reachable as `Stale::back_into`, driven by the test through a harness forward. Then
the frame path's arm was replaced with `drop(stale)`: **3 of 3 still passed.** The
scenario was grading the fixture's own call.

- **Renaming the reachable call changes nothing.** `back_into` and a public
  `mark_for_remesh` are the same mistake under two names, because neither runs the
  product's path. The repair had to move *which function the product calls*, not
  what it is called.
- **The verdict alone cannot see it, which is why the observable is downstream.**
  Under the mutation the collect still answered `Discarded` — a discard that lost
  its keys is still a discard — so what reddens is the sections found waiting
  afterwards.
- **The durable half is that the mistake stopped being writable.** With the
  hand-back inside the client's own collect and `Session::mark_for_remesh` private,
  there is no harness forward and no arm in the frame path to write either way. The
  `pub` came off with **no exception needed anywhere**, which is the tell that the
  seam was real rather than convenient.
- **A `#[must_use]` value handed to a caller is a weaker guard than it looks.**
  `Superseded(stale) => {}` fails the build under the denied `unused_variables`
  lint, but `Superseded(_) => {}` would not — `#[must_use]` fires on unused
  expressions, not on discarded bindings. Moot under the landed shape, and recorded
  for the next design tempted to hand one over.

### CLOSING CONDITION ON PHASE 6 — the build runs off the tick thread, instrumented or escalated

**RULING: this may not be filed as a mechanism a reviewer holds.** The standing
rule is that **a requirement for a performant solution binds even when nothing in
the increment measures it, and the absence of a measurement is not licence to drop
the property.** Building the candidate off the tick thread is the whole reason a
reload does not stall the game; if nothing grades it, a later change moves the
build onto the tick thread, every test stays green, and the one-second budget is
blown silently. **That is strictly worse than D3, because D3's violation at least
corrupts an observable value.**

**What is established, measured in phase 3:** FR-1.2-S2 **passes with the build
blocking the tick.** Its observable is *where* the ticks put the player; its
property is *when* they happened, and a blocking collect changes only the second.
**FR-9.1-S1 has the same shape and the same hole.**

**The discrimination, stated rather than only the instruction:** what must redden
is **a build that runs on the tick thread**. What must **not** be used is elapsed
time, or any count that proxies it — a polled collect spanning many boundaries
against a blocking one spanning exactly one is a timing assertion wearing a
count's clothing, holds here by four orders of magnitude, and would go red on a
machine where a tick costs a millisecond. Phase 3's test author found that
discriminator and refused it, correctly.

**Direction to look first: identity rather than duration.** A `ThreadId`
comparison is deterministic, carries no window, and discriminates exactly — the
build either ran somewhere other than the ticking thread or it did not. Not
prescribed. **A compiler-held version would be better still**, and phase 1 already
used that shape when `World::adopt` stayed module-private: a build entry point
requiring a handle only the worker has would put this beyond a test's reach
entirely. Against that, a production field added solely to be observed is its own
hazard.

**If phase 6 finds no honest instrument, escalate before closing. Do not close on
prose.** That is the difference between carrying a constraint knowingly and
dropping it, and only one of those is available.

**Phase 6 also inherits a fixture where even the fragile version is unavailable**,
because pacing the boundaries removed the count the weak discriminator would have
rested on. That is a net improvement and it also closes the cheap escape.

**AND THE BENCHMARK DOES NOT COVER THIS. Do not let the pair's existence stand in
for coverage the pair does not have.** FR-9.1-S1 is the deterministic half of a
latency target whose other half is T25's `criterion` benchmark, and the natural
inference — that the benchmark carries what the scenario cannot — is **false by
construction rather than merely unmeasured**:

> The benchmark measures end-to-end reload latency, and **a tick blocked by the
> build does not lengthen it.** If anything a blocking collect is *faster* than a
> polled one, because it drops the polling overhead.

**So the property being violated makes the number look better**, which is the
worst arrangement available: not an instrument that fails to see the defect, but
one that rewards it. If neither half can see a blocked tick, **the property has no
instrument at all** — which is why this is a closing condition rather than a note.

### The findings list is CLOSED at eight — seven to `testing.md`, one to `docs/technical/`

**Closed at seven, then reopened once for item 7 and closed again.** The reopening
is recorded rather than smoothed over, because the reason is the rule: **a bar that
never admits anything is not a bar, it is a wall.** Item 7 cleared the stated bar
more cleanly than some of the six that were already on the list, so refusing it
would have been enforcing the number rather than the standard.

**The bar for a ninth is unchanged: a rule a future reader can act on *before* they
have the bug** — not an account of something that happened. Whoever proposes one
says why it clears the bar rather than merely proposing it.

And the honourable alternative, which is what actually keeps the list short: if it
reads "we also noticed", it is a note in this file and it archives with the spec —
**the correct fate for most observations, and not a demotion.** A bar with no
alternative gets argued around.

**T02's measurement, taken before anything was decided.** One whole-world
`SolidVoxels::resolve` over 1 048 576 voxels is **5.0 ms** (release, seven
consecutive rounds within 4.99–5.17 ms). Against the one-second target whose
dominant term is the 150 ms settling window that is not a material term, so D3's
deferred section-skip **stays deferred**. The figure goes to T25's benchmark.

**DEVIATION from D14's siting — accepted, and `architecture.md` is amended
rather than only this file.** `architecture.md` carried the superseded siting in
three places (the decision text, the code-comment illustration and the module
map) plus a stale count in its Risks table; all four are corrected there, marked
"amended in phase 1", with the compile reason beside the decision. This file is
archived at completion and `docs/` consolidates from `architecture.md`, so a
correction living only here would let `docs/` inherit the wrong siting.
`crates/mc-client/src/session.rs` is now `session/mod.rs`, with the reload
surface in `crates/mc-client/src/session/reload.rs`. D14 sited that file as a
*sibling* of `remesh.rs`, and a sibling module cannot reach `Session`'s private
`simulation` and `holding` fields — so D14's own fallback clause, "a second
`impl Session` block in that file", could not have compiled. `session.rs` was at
492 of 500 non-blank lines and `adopt_content` does not fit, so the choice was a
child module or a crate-visible accessor for the field `session.rs`'s header
exists to keep private. **`session/mod.rs` is now at 496 of 500**, so T15 must
put the `ContentReload` field, the drive and `take_reload_report` in
`session/reload.rs` rather than in `mod.rs`. It can: a child module sees those
fields. This also breaks nothing in `OUTSIDE_THE_CORE_GUARD`, whose exemption
followed the path.

**A lint-reading rule this phase paid for.** A `--all-targets --all-features
-D warnings` run over the workspace names the crate that failed **first**, not
every crate that would fail: `mc-sim`'s test support tripped an unreachable
pattern and the build aborted before `mc-client`'s twin of the same fixture was
checked. Fixing only what clippy named would have met the identical diagnostic in
a second crate on the next gate run, reading as a new defect. Same shape as
`docs/technical/testing.md`'s "read a multi-stage gate failure for one cause
before reading it as several", with the sign flipped — one cause, reported in one
of the several places it lives.

**For T11 — my figure was wrong and phase 2's author measured it.** I reported
`Simulation::new` at 24 sites with 20 in test files and 4 in production. Measured:
**24 sites, 22 in test files across 18 files, and 2 in production** —
`crates/mc-sim/src/persistence.rs` and `crates/mc-sim/src/replay/spawn.rs`.
Separately, **`mc_sim::content::load` has 9 call sites, 7 of them in tests**,
which nobody had estimated. The conclusion stands and is stronger than I put it:
the third parameter is almost entirely a test-file adaptation and the window
belongs to the phase's test author.

**This is the count-versus-membership lesson arriving a second time in this
spec**, and it binds T26: whatever reaches `docs/` carries the **measured**
numbers, never these estimates. An estimate that survives into as-built
documentation is worse than no number, because a reader has no way to tell one
from the other.

**`documented_refusals.rs` is at 562 of 600 non-blank lines** with T12 still
adding to it. If it crosses, split by responsibility and **do not compress** —
same ruling as `reload_world.rs`, and the same reason: the prose is why that file
is trustworthy.

**Deferred observation, outside phase 1's diff.** `adopt_candidate`'s
`refused_over` has an unreachable arm returning an empty block list, whose
rendering would read "the world holds no block that this content does not
declare". It cannot be reached — a name resolve refuses nothing but an
unregistered name — and narrowing `World::adopt`'s error type to make it
unspellable is a change to `SolidVoxels::resolve`'s signature and out of scope
here.

- **Scenario totals reconcile, checked mechanically in both directions.** The
  spec holds **91** scenarios — FR-1: 17, FR-2: 13, FR-3: 12, FR-4: 15, FR-5: 8,
  FR-6: 9, FR-7: 7, FR-8: 8, FR-9: 2. The phases hold
  **26 + 15 + 26 + 12 + 7 + 5 = 91**. Expanded per requirement: FR-1 = 1 (p1) +
  16 (p3); FR-2 = 3 (p1) + 10 (p3); FR-3 = 12 (p1); FR-4 = 10 (p1) + 2 (p2) +
  3 (p4); FR-5 = 8 (p2); FR-6 = 9 (p4); FR-7 = 7 (p5); FR-8 = 5 (p2) + 3 (p6);
  FR-9 = 2 (p6). **Every scenario appears in exactly one task and no scenario
  appears twice.**
- **Two facts measured against this branch's head rather than reasoned.**
  `crates/mc-client/src/session.rs` is at **492** non-blank lines against the
  gate's 500 limit and `app.rs` at **456** — which is why D14 pre-commits
  `reload.rs` rather than letting someone discover it mid-phase. And the terrain
  measurement forcing the binary re-mesh: the highest occupied section is **3 in
  fifteen columns and 4 in one**, everything above holding no block at all.
- **Two corrections made to `architecture.md` at this stage**, rather than
  recorded here and left standing there — it is archived beside this file and a
  future reader meets the two as peers, not as a draft and its successor.
  (a) Its Integration table put the "well under 100 KB" batch-size claim in
  `crates/mc-client/src/remesh.rs`; it is at
  **`crates/mc-sim/src/world/remesh.rs:10`**, and the wrong path would have sent
  phase 4 to the wrong crate to update a figure that becomes false. Nothing else
  about that row changes. (b) Its green-on-arrival list has been replaced with the
  twelve above, with phase 3's five moved to their own "red on arrival" heading
  and one line recording why the earlier reading was wrong. Neither correction
  changes what any phase must do.
- **DEFERRED, each with its revisit condition, and taking any of them while
  passing is out of scope:**
  - D3's section-skip in the solidity resolve — revisit when the FR-9 benchmark
    shows the swap is a material term in the one-second budget, or when the world
    outgrows a fixed footprint.
  - D7's selective marking rule — **taking it is a spec change** to FR-6.1-S3 and
    to the spec's Technical Considerations, not an optimisation.
  - D8's second "geometry serial" — revisit if a profile shows reload-coincident
    batches matter.
- **Out of Scope is binding** (spec): the unknown-block path; texture resolution
  through the registry and per-face keys (PRO-902, PRO-914); cross-checking
  `breaks_into` at load or reload; reclaiming layers retired during a session;
  per-cell state and its migration; mod-authored `tests/` gating a candidate;
  reloading the rule expression graph and reloading worldgen for an
  already-generated world; a content-set identity or hash; moving the composition
  root and restoring the dependency-closure guard; an operator switch for whether
  a running server watches its content; and reloading anything but the content
  root. Recorded, not built.
- **The USER RULING that changing a script may reset state** costs nothing in this
  increment — there is no script-held per-cell state to reset — and is recorded in
  the spec so that whichever spec builds per-cell state inherits the ruling rather
  than re-asking.
