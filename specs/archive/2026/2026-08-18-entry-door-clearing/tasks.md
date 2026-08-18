# Tasks: A player entering a world is never left inside solid rock

**Spec**: [spec.md](spec.md) · **Architecture**: [architecture.md](architecture.md) ·
**Branch**: `feature/PRO-948-entry-door-clearing` · **Rigor**: `high` · **Created**: 2026-08-18

Three phases, 14 tasks, 18 scenarios. Every scenario is assigned to exactly one
task; the assignment is checked at the foot of this file.

The order is forced: the client cannot say what entry did before the verdict
reaches it, and neither can be documented before it exists.

---

## Counts: the rule that binds every scan in this spec

**Every expected count in a scan is re-derived from the tree at the moment the
task is written, and the command that produced it is stated beside it. No number
is copied from `architecture.md`.** A wrong expected count goes red on day one
for the wrong reason, and the cheapest green is editing the number until it
matches — at which point the scan has stopped reporting anything.

Everything in this section was re-derived on 2026-08-18 against the working tree
at `f260c64` (`git rev-parse --abbrev-ref HEAD` → `feature/PRO-948-entry-door-clearing`,
`git status --short` → clean).

### Two corrections carried from `architecture.md`, and the one rule behind both

The team lead checked two of the architecture's needle figures against the tree
and reported both as wrong. Both point at the same real fragility, and it is
load-bearing enough that the tasks below restate it rather than assume it.

**Correction 1 — `published:` matches three lines, not two.**

```
$ rg -n --no-heading "published:" crates/mc-sim/src
crates/mc-sim/src/reload/mod.rs:91:/// Call it after the tick it follows has been published: a tick answers every
crates/mc-sim/src/simulation.rs:118:    published: ArcSwap<SimSnapshot>,
crates/mc-sim/src/simulation.rs:145:            published: ArcSwap::from_pointee(SimSnapshot {
```

Raw total: **3**. `architecture.md:302` states 2 and names only the two in
`simulation.rs`. The third — `crates/mc-sim/src/reload/mod.rs:91`, the English
word "published" followed by a colon inside prose — is real, and the architecture
does not mention it at all.

**It is nevertheless not a scan site, and the reason is the whole point.** Line 91
begins with `///`, and `production_text` (`crates/mc-client/tests/reporting_seam.rs:307-316`,
read 2026-08-18) drops every line whose trimmed start is `///` or `//!` before any
`contains` runs. So the count the scan sees is 2 and the count `rg` sees is 3.

**What binds:** the scan's expected count is derived over `production_text`, never
over a bare `rg`. An implementer who derives `3` from the shell and writes it into
the table gets a scan that is red on arrival and whose cheapest green is a number
edit. An implementer who derives `2` from the shell without knowing why gets a
number they cannot defend. Both are avoided by deriving with the stripping applied
and stating that in the test file.

**And the exposure is one character wide.** Turn `crates/mc-sim/src/reload/mod.rs:91`
from `///` into `//` — a plausible edit, since it is prose — and the count silently
becomes 3 and the scan reddens with no seating door added. That is the residual
hole in this needle. It is recorded here rather than papered over; the needle stays
because a struct-literal second door is the shape it is paid to catch (D5), and a
reddening scan that names `reload/mod.rs` is a diagnosis a reader can act on in one
minute.

**Correction 2 — `Clearing` names three files today, not two, and eight lines, not seven.**

```
$ rg -n --no-heading "Clearing" crates/mc-client/src
crates/mc-client/src/app/reload.rs:12:use mc_sim::world::Clearing;
crates/mc-client/src/app/reload.rs:96:fn report_clearing(clearing: Clearing) {
crates/mc-client/src/app/reload.rs:98:        Clearing::Unneeded => {}
crates/mc-client/src/app/reload.rs:99:        Clearing::MovedTo(feet) => {
crates/mc-client/src/app/reload.rs:108:        Clearing::NoClearSpaceWithin { blocks } => {
crates/mc-client/src/session/reload.rs:15:use mc_sim::world::{Clearing, SectionKey};
crates/mc-client/src/session/reload.rs:79:        clearing: Clearing,
crates/mc-client/src/session/mod.rs:295:    /// Clearing it here makes "a click at a loading screen changes nothing" true
```

Raw: **8 lines across 3 files** — `app/reload.rs`, `session/reload.rs`,
`session/mod.rs`. `architecture.md:396` states "7 today" across two files, and
separately notes that `session/mod.rs:295` "is the English word in a `///` line,
not the type". So the architecture's 7 is the count of *type* occurrences after
stripping, and the raw figure it does not state is 8/3.


**This block's arithmetic is already stale, and that is the section working.** The
figures above were measured at `f260c64`. Phase 1 then put `pub clearing: Clearing`
on `PreparedLaunch` (`launch.rs:98`) with a `///` mention at `:75`, so phase 2
measured **11 raw across 4 files, 9 visible across 3**. Nothing about the *rule*
moved; only the numbers did, which is exactly why the rule is "re-derive at the
moment you write it" and never "inherit the count from this file". Any figure in
this document is a reading of a tree that no longer exists.

**The post-implementation figure is 3 files, not 5.** The lead's estimate of five
assumed `app/reload.rs` and `session/mod.rs` both survive. Verified by reading
both:

- `session/mod.rs:295` is a `///` line (`sed -n '295p'` — the line begins `    /// Clearing it here…`), so `production_text` drops it and it is not a site
  today and will not be one after.
- `app/reload.rs` loses all five: `report_clearing` is deleted whole
  (`:89-115`), and `use mc_sim::world::Clearing;` (`:12`) goes with it because
  `-D warnings` refuses an unused import. What remains at `:48` is the
  destructured binding `clearing`, lower-case, which the needle `Clearing` does
  not match.

So the file set the scan sees after implementation is `src/notice.rs`,
`src/launch.rs`, `src/session/reload.rs` — three — which is the rule
`architecture.md:396` states. **The rule is inherited; the figures behind it were
not, and neither raw figure appears in the architecture.**

**What binds, and it is the same rule as correction 1:** T08 derives this file set
by walking `production_text`, and its test file states in a comment that a bare
`rg -l` reports a different set and why. `session/mod.rs` is the standing
counter-example, and it is one `///` → `//` edit away from becoming a fourth file
in a scan that is looking at the right thing.

### The derived tables

**D5 — `crates/mc-sim/tests/one_way_seats_a_player.rs`, roots `crates/mc-sim/src`.**
Command for every row: `rg -n --no-heading -F "<needle>" crates/mc-sim/src`, then
the `///`/`//!` lines removed by hand, matching `production_text`.

| Needle | Raw today | Scan-visible today | Expected after implementation |
|---|---|---|---|
| `Simulation::new(` | 2 — `persistence.rs:153`, `replay/spawn.rs:103` | 2, neither in `simulation.rs` | **1**, only in `src/simulation.rs` |
| `Self::new(` | 0 | 0 | **0**, anywhere |
| `published:` | **3** — `simulation.rs:118`, `:145`, `reload/mod.rs:91` (`///`) | **2**, both in `simulation.rs` | **2**, only in `src/simulation.rs` |
| `self.player =` | 1 — `simulation.rs:183` | 1 | **1**, only in `src/simulation.rs` |
| `) -> Seated` | 0 | 0 | **1**, only in `src/simulation.rs` |

`-> Result<Seated,` does not contain `) -> Seated`, so the two launch arms are not
sites — verified by inspection of the needle, not by a run.

**D6 — `crates/mc-client/tests/the_entry_sentence_is_said_once.rs`, roots
`crates/mc-client/src`.** Same command with `crates/mc-client/src`.

| Needle | Raw today | Scan-visible today | Expected after implementation |
|---|---|---|---|
| `entered the world inside solid blocks` | 0 | 0 | **1**, only in `src/notice.rs` |
| `so you were left inside them` | 0 | 0 | **1**, only in `src/notice.rs` |
| `fn say_entering` | 0 | 0 | **1**, only in `src/notice.rs` |
| `notice::say_entering(` | 0 | 0 | **1**, only in `src/app/mod.rs` |
| `Clearing` | **8 lines / 3 files** (above) | 7 / 2 files | **file set** `src/notice.rs`, `src/launch.rs`, `src/session/reload.rs` — no count |

`src/notice.rs` does not exist today (`ls crates/mc-client/src` — `app/`,
`bindings.rs`, `content.rs`, `events.rs`, `gpu_startup.rs`, `launch.rs`, `lib.rs`,
`main.rs`, `remesh.rs`, `session/`, `startup.rs`, `surface_setup.rs`, `upload.rs`),
which is why four of the five needles read 0.

**The adaptation surface**, re-derived rather than inherited:

```
$ rg -n --no-heading "Simulation::new\(" crates/ | wc -l     → 24 sites
$ rg -l "Simulation::new\(" crates/ | wc -l                  → 20 files (2 src, 18 tests)
$ rg -l "Simulation|simulation_for|simulation_at_launch|simulation_to_play|PreparedLaunch" \
      crates/*/tests | wc -l                                 → 43 test files to inspect
$ { rg -l "Simulation::new\(|Result<Simulation|Result<\(Simulation|simulation_for\(|\
      simulation_at_launch\(|simulation_to_play\(" crates/*/tests crates/*/src; \
    rg -l "playing_client\(simulation," crates/mc-client/tests; \
    rg -l "    simulation: Simulation," crates/*/tests; } | sort -u | wc -l
                                                             → 37 files (34 tests, 3 src)
  … same needle set counted rather than listed                → 82 sites
```

`architecture.md:626-635`'s floor of "≥37 files and ≥46 sites" is confirmed: 37
files and 82 needle sites. **How many actually need an edit is not knowable by
grep** — a `&Simulation` parameter is unaffected while a `Result<Simulation, _>`
is not — so the inspection set is the 43 and the edit set is discovered by
compiling. `crates/mc-client/tests/seam_boundaries.rs:148` names `Simulation`
only as a guard needle inside `WINDOW_FACING_GUARD`, whose `exempt` is
`|path| path != "src/events.rs"` (`:143-155`, read); it should be unaffected, and
it is read before commit B is staged because a reddening there inside the window
has nothing reporting it.

---

## Phase 1 — one door, and the whole of FR-1

Scenarios: FR-1.3-S1, S2 · FR-1.1-S1..S7 · FR-1.2-S1..S3.

The door and the search land together. Splitting them makes a later phase's
scenarios green on arrival, which is worse than a large phase.

**Three commits: A (test author), B (test author, the blind window), C
(implementer).** Commit B leaves the tree uncompilable, so the gate cannot run
inside it. T04 carries the procedure that verifies it anyway.

**Fresh test author and fresh implementer for this phase.** The test author owns
every test file for the whole phase; the implementation context never edits one,
and a clippy finding inside a test file is not an exception to that — it goes back
to the test author. Each task's closing report is all the next task inherits.

- [ ] **T01 [P] The seating scan and its three positive controls** — new
      `crates/mc-sim/tests/one_way_seats_a_player.rs`
      Scenarios: FR-1.3-S1, FR-1.3-S2
      Commit A · test author

      Shape it on `crates/mc-client/tests/reporting_seam.rs`: `production_text`
      stripping `///` and `//!` only, sibling `*_test.rs` files skipped,
      `/`-separated relative paths, `tempfile` trees for the controls. Root
      `crates/mc-sim/src`, **no exemption list**.

      The verdict is the total enumerated `Seating` of `architecture.md:276-286`:
      `OneWaySeatsAPlayerAndItReportsItsClearing`,
      `AnotherSourceSeatsAPlayer(Vec<Site>)`,
      `TheDoorNoLongerSeatsAPlayer(Vec<String>)`, `NoSourceWasRead`. `Site` carries
      `{ file, names, times }` — the `times` count is what closes the hole
      `reporting_seam.rs` leaves open, where one site per (file, needle) pair makes
      a second offence in an already-named file invisible.

      **Assert the enumerated verdict, never an absence.** `assert!(found.is_empty())`
      cannot tell an empty answer from a scan that can no longer look;
      `assert_eq!(verdict, OneWaySeatsAPlayerAndItReportsItsClearing)` rejects
      every other variant *including* the two that mean "I could not look", so a
      vanished source root reddens for free.

      **Needles and expected counts: use the D5 table above, and re-derive each
      one at the moment you write it** with `rg -n --no-heading -F "<needle>"
      crates/mc-sim/src`, dropping `///`/`//!` lines by hand. Write the derivation
      command into the test file beside the table. `published:` is the one that
      bites — see Correction 1; the third match is a `///` line in
      `crates/mc-sim/src/reload/mod.rs:91` and the scan does not see it. Derive the
      expected `Vec<Site>` from the needle table *in code*, the way
      `every_needle_named_in` (`reporting_seam.rs:188-197`) does, so a needle added
      without an expectation fails rather than standing unwatched.

      **Three controls, each a separate `#[test]` function feeding a different
      non-good variant.** One test per variant, not one test walking three
      fixtures — a shared fixture loop that stops early leaves two variants
      unproven and reads as three passes.

      1. tempdir whose `src/simulation.rs` is well-formed **plus** a `src/join.rs`
         that constructs a simulation → `AnotherSourceSeatsAPlayer` naming
         `src/join.rs`. This is FR-1.3-S2, and it is the MVP 3 shape.
      2. tempdir whose `src/simulation.rs` omits `) -> Seated` →
         `TheDoorNoLongerSeatsAPlayer`. Without it a renamed door reads as a clean
         crate forever.
      3. empty tempdir → `NoSourceWasRead`.

      **RED for the right reason, displayed before anything else in the phase:**
      against today's tree the verdict is
      `AnotherSourceSeatsAPlayer([persistence.rs, replay/spawn.rs])`. If it is red
      as `TheDoorNoLongerSeatsAPlayer` or `NoSourceWasRead`, the scan is broken,
      not the tree.

      **No ordinary `//` comment in `crates/mc-sim/src` may name a needle.**
      `production_text` strips `///` and `//!` and nothing else, so
      `// moved off Simulation::new` left behind by commit C *is* a site. Say so
      in the test file; commit C is where it would be written.

- [ ] **T02 [P] Entry clears a resumed player, and moves nobody who needs nothing**
      — new fixtures under `crates/mc-sim/tests/` and `crates/mc-client/tests/`,
      reusing `tests/support/launch.rs` and `tests/support/persistence.rs`
      Scenarios: FR-1.1-S1, S2, S3, S4, S5, S6, S7
      Commit A · test author

      **Written against today's signatures** — `Result<Simulation, LaunchError>`
      and `Result<Simulation, SpawnError>` — and adapted in T04 with everything
      else. Their RED here is *behavioural*: the player is not cleared. Written
      against `Seated` they would not compile, and the blind window would be two
      commits rather than one.

      Four traps, each of which reddens a **correct** implementation and whose
      cheapest green is forbidden by Out of Scope:

      1. **FR-1.1-S7 is read at tick 0, not tick 1.** A cleared player arrives
         with `on_ground: false` — `resuming` sets it
         (`crates/mc-sim/src/persistence.rs:199`, read: `on_ground: false`) and the
         clearing move touches position and velocity only
         (`crates/mc-sim/src/simulation.rs:245-248`). So a player put on a cell
         floor at entry is standing on it while claiming no contact, and **tick 1
         settles that by falling a fraction and landing**. Read the player at tick
         1 and the `y` differs, the test goes red against a correct
         implementation, and the cheapest green is to ground the player or set
         `on_ground` in the search — both Out of Scope. The scenario says *the
         first snapshot the simulation publishes*, and `Simulation::new` publishes
         it at construction (`simulation.rs:144-150`), so tick 0 is what there is
         to read.
      2. **`centre_of` returns `(x + 0.5, y, z + 0.5)`** — horizontally centred,
         feet on the cell *floor*, no `+ 0.5` on `y`. Read at
         `crates/mc-sim/src/world/clearing.rs:120-127`. A test author deriving
         `y + 0.5` from "at that cell's centre" reddens a correct implementation,
         and the cheapest green is editing the search.
      3. **FR-1.1-S1 must assert the exact destination** — one cell sideways,
         horizontally centred, feet on that cell's floor — and must never be
         weakened to "covers no solid cell". S1 is designed as S6's positive
         control *through the extent argument*: a shrunken extent rejects S1's
         one-cell-sideways destination, and a bare "covers no solid cell" takes S1
         off the only path where it can fail.
      4. **FR-1.1-S6 must assert the exact stayed-put position, and its fixture
         must be a real save file read through the shipped launch.** An in-memory
         fixture takes it off the shipped path, which is where an over-large
         extent would actually be supplied.

      **FR-1.1-S4 asserts the derived spawn exactly** and that nobody is moved —
      horizontally centred over column `(32, 32)`, feet three blocks above that
      column's own surface height. Verified in the tree: `SPAWN_COLUMN: (u32, u32) = (32, 32)`
      (`crates/mc-sim/src/replay/spawn.rs:38`), `SPAWN_ABOVE_SURFACE: u32 = 3`
      (`:45`), `spawn()` reading `surface_height(column_x, column_z)` (`:64-67`).
      "Covers no solid cell" is green before a line is written — the derived spawn
      is three blocks above its surface and no shipped content can trap it — so it
      would be a test that cannot fail. What S4 is really for is the opposite
      failure: an entry check that moves, cell-centres or grounds a player who
      needed nothing, which changes **every committed golden frame**
      (`crates/mc-client/tests/support/frames.rs:119`,
      `crates/mc-sim/tests/replay_player.rs:87` shoot every committed frame
      through `simulation_for`).

      **Do not re-assert the search's own semantics.** Reach, ring order, cell
      centres, absence of downward candidates and eligibility are PRO-918's, pinned
      by five integration tests in `crates/mc-client/tests/reload_*.rs`, and
      re-proving them through the same code path is a bogus test
      (`standards/global/testing.md` §1). Only the two new-wiring facts are in
      scope: the extent argument at the new call site (S1/S6) and a saved position
      near the world's edge (S6), which no reload fixture supplies.

- [ ] **T03 [P] Entry clearing runs whatever the load reported about its blocks**
      — new fixtures under `crates/mc-client/tests/`, reusing
      `tests/support/persistence.rs` and `content/base/blocks/water.luau`
      Scenarios: FR-1.2-S1, S2, S3
      Commit A · test author

      Also written against today's signatures, adapted in T04.

      **FR-1.2-S2 is the falsifier for D3.** An implementation that gates on
      `RegistryVerdict` or on the acceptance mode passes S1 and fails S2: S2's
      launch is made *without* `--load-changed-blocks`, against a save whose blocks
      all match their current declarations, and whose recorded feet lie inside a
      cell holding `base:stone`. Nothing about that launch reports a change, and
      the player is still cleared. Do not let this fixture drift into one where a
      block changed.

      FR-1.2-S1 is the player's own journey and must run it: quit standing in
      `base:water`, redeclare `base:water` `solid = true` while the game is not
      running, resume with `--load-changed-blocks`.
      `content/base/blocks/water.luau` is the one shipped `solid = false` block and
      the sea the replay world fills.

      FR-1.2-S3 pins that the world still holds the blocks the save recorded — the
      clearing moved the player, not the world — and that the move is within 8
      blocks.

- [ ] **T04 The adaptation window: every fixture onto the new door** — the 37 files
      derived above, across `crates/mc-sim/tests/`, `crates/mc-client/tests/` and
      their `support/` modules
      Scenarios: none (this task adds no behaviour)
      Commit B · test author · Depends on: T01, T02, T03

      **From this commit until T05 lands, the tree does not compile and the gate
      cannot run.** This is `standards/global/testing.md` §2's blind window, and it
      is one commit wide. The procedure below is what makes it acceptable;
      **skipping it turns a bounded risk into an unbounded one.**

      **Nothing here requires judgement.** The adaptation is mechanical in four
      shapes:

      1. `.simulation` appended at a `Simulation::new` call — 24 sites
         (`rg -n "Simulation::new\(" crates/`), of which 22 are in test code across
         18 files (14 in `mc-sim/tests`, 4 in `mc-client/tests`; `rg -l`).
      2. A helper signature — `crates/mc-sim/tests/support/launch.rs` carries nine
         `Result<Simulation, LaunchError>` lines (`:183, 198, 212, 227, 253, 286,
         302, 318, 323` — `rg -n "Result<Simulation, LaunchError>"
         crates/mc-sim/tests/support/launch.rs`), and none of them is a call site.
         `:318` additionally returns `Result<&Simulation, String>`, whose type is
         unchanged and whose body is not.
      3. A type alias — `crates/mc-client/tests/support/persistence.rs:59` and
         `crates/mc-client/tests/support/reload_save.rs:186`, both
         `pub type Launched = Result<(Simulation, BlockName), PreparationError>`.
      4. A borrow into the result — `support/launch.rs:318` becomes
         `&seated.simulation`, a body change rather than a call-shape change.

      Four `mc-client` reload tests destructure the `Launched` alias and pass the
      simulation on to a local `playing_client(simulation: Simulation, …)`: the
      helper's signature is unaffected and the **call** changes —
      `reload_keeps_the_player.rs:198` and `:206`, `reload_keeps_the_world.rs:184`
      and `:190`, `reload_mutation_rules.rs:141` and `:150`,
      `reload_solidity.rs:142` and `:151` (`rg -n "playing_client\(simulation,"
      crates/mc-client/tests`). A fifth file,
      `crates/mc-client/tests/saved_changes_need_no_edit.rs:265`, holds a
      `simulation: Simulation` field.

      **`PreparedLaunch`'s new field costs the tests nothing.** No test file
      constructs one — the only literal in the workspace is
      `crates/mc-client/src/launch.rs:222` (read). The five test files naming the
      type do so in type positions only.
      `crates/mc-client/tests/edit_geometry.rs:301` looks like a counter-example
      and is not: it builds a local `Handed` struct with a `simulation` field, and
      it changes only because it also calls `Simulation::new(`.

      **Stop and raise rather than adapt, if a fixture turns out to place a
      trapped player deliberately.** Every adapted fixture is assumed to place a
      clear player, so each receives `Clearing::Unneeded` and no fixture's
      behaviour changes. One that does not would change behaviour, and that is a
      decision, not an adaptation — it does not get made inside the window.

      **The commit-B procedure, in order. Run it; do not summarise it.**

      1. **Before touching anything**, run
         `cargo clippy --workspace --all-targets --all-features -- -D warnings`
         on the pre-window tree, so the phase does not open carrying an unrelated
         lint that is later blamed on the adaptation. `-D warnings` and not a lower
         severity: without it cargo attributes the diagnostic to the first binary
         and marks the rest `(1 duplicate)`, which means *this same diagnostic
         repeated*, not *a pre-existing one lives elsewhere*.
      2. Read `crates/mc-client/tests/seam_boundaries.rs:143-155` before going
         further. Its `WINDOW_FACING_GUARD` names `Simulation` as a needle and is
         scoped `|path| path != "src/events.rs"`, so it should be unaffected — but
         it is exactly the kind of thing that reddens inside a window where
         nothing reports.
      3. Adapt the files. Compile-and-fix is the discovery mechanism; the 43-file
         inspection set is the bound.
      4. **Apply a throwaway local `seat`/`Seated` stub** to
         `crates/mc-sim/src/simulation.rs`, `crates/mc-sim/src/persistence.rs` and
         `crates/mc-sim/src/replay/spawn.rs` — enough for the tree to compile, with
         no clearing in it.
      5. Run `cargo clippy --workspace --all-targets --all-features -- -D warnings`
         and the full suite against the adapted tests. This is what buys the
         ≥37 adapted files being compiled and linted before they enter the one
         commit nobody can check.
      6. **Revert the stub by hand** — re-edit the lines you added.
         **Never `git checkout -- <file>`**: that discards everything uncommitted
         in the file, and it has already wiped a whole uncommitted implementation
         once in this project (`standards/global/git-workflow.md` §2).
      7. Confirm `git diff --exit-code -- crates/*/src` is clean. Nothing of the
         stub is committed, so the test-ownership rule is intact.
      8. **Stage explicit test paths only.** `git add -A` and `git add .` are
         banned with no "the tree is clean, I checked" exception — a sweep has
         already pulled a test author's in-flight file into an implementation
         commit in this project. Other sessions share this working tree: never
         stage a path you did not write, and re-read `git status --short`
         immediately before concluding anything.

      **Why the window is accepted rather than avoided.** The zero-window route —
      add `seat` beside a still-public `Simulation::new`, migrate, then demote —
      needs `simulation_for`, `simulation_at_launch` and `simulation_to_play` to
      exist in **both** return shapes at once, since a return type cannot be
      migrated in place. That is three duplicated public functions with duplicated
      documentation, and the migration commit still touches every affected file:
      the window is traded for a larger diff and a temporarily doubled public
      surface, not for less work. And T01's scan then sits red across three commits
      instead of one, which is the state `testing.md` §2 names as the one in which
      a test stops reporting anything new. "Two public doors would end up in
      `main`" is **not** an argument available here — `git-workflow` §3 mandates a
      squash merge, so `main` receives one commit either way.

- [ ] **T05 The door: `seat`, `Seated`, the factored rule, `new` demoted** —
      `crates/mc-sim/src/simulation.rs`, `crates/mc-sim/src/world/clearing.rs`,
      `crates/mc-sim/src/persistence.rs`, `crates/mc-sim/src/replay/spawn.rs`,
      `crates/mc-client/src/launch.rs`
      Scenarios: turns T01, T02, T03 green — owns none of its own
      Commit C · implementer · Depends on: T04

      Per D1, D2 and the Interfaces block (`architecture.md:504-540`):

      - `pub struct Seated { pub simulation: Simulation, pub clearing: Clearing }`,
        deriving `Debug` only — `Simulation` has a hand-written `Debug`
        (`simulation.rs:292`) and is neither `Clone` nor `PartialEq`.
      - `#[must_use] pub fn seat(spawn, world, content) -> Seated`, a **free
        function in `simulation.rs`**.
      - `Simulation::new` loses `pub` **entirely** — not `pub(crate)`. Module
        privacy is what closes the hole against `mc-sim`'s own callers, and it
        costs nothing: `Simulation::new` has exactly two callers in `mc-sim/src`
        (`persistence.rs:153`, `replay/spawn.rs:103`), both moving onto `seat`, and
        `mc-sim` has no sibling unit-test file for `simulation.rs`
        (`crates/mc-sim/src/world/mod_test.rs` is the crate's only `*_test.rs`).
      - `pub(crate) fn clear_the_player(player: &mut PlayerState, world: &dyn Solidity, ground: Extent) -> Clearing`
        moves into `crates/mc-sim/src/world/clearing.rs` with its doc comment. It
        is `Simulation::clear_the_player` (`simulation.rs:243`) with `&mut self`
        replaced by what it actually touches. **The middle parameter stays
        `&dyn Solidity`** so D11's ruling — `Solidity` is not widened, the extent
        is passed — is carried forward to the letter. The method is deleted.
      - Two callers: `seat`, as
        `clearing::clear_the_player(&mut spawn, &world, world.extent())` on the
        owned `spawn` **before `Simulation::new` publishes the first snapshot** —
        which is what makes FR-1.1-S7 structural rather than a second publish; and
        `Simulation::adopt` (`simulation.rs:217`), as
        `clearing::clear_the_player(&mut self.player, &self.world, self.world.extent())`,
        two disjoint field borrows the borrow checker accepts inside the method
        body.
      - `cleared` (`clearing.rs:63`) is **untouched**. Out of Scope forbids
        changing the search.
      - Return types change: `simulation_for -> Result<Seated, SpawnError>`,
        `simulation_at_launch -> Result<Seated, LaunchError>`,
        `simulation_to_play -> Result<(Seated, BlockName), PreparationError>`,
        `PreparedLaunch` gains `pub clearing: Clearing`. `simulation_for` carries
        it too: FR-2's sentence is deliberately true of a generated world as well
        as a resume, and discarding the verdict there would make FR-1.3's own
        verdict name false.
      - `use glam::Vec3;` (`simulation.rs:41`) becomes unused when the method
        moves out — its only use is `Vec3::ZERO` at `:247`. Remove it in the same
        commit; `-D warnings` will insist.
      - `PreparedLaunch`'s header (`launch.rs:56-61`) argues the type carries
        nothing the frame path could pick the wrong one of. Add one sentence
        saying why a `Clearing` — a plain `Copy` verdict with no second candidate —
        does not weaken that, or a reader sees the precedent rather than the
        distinction.
      - No error contract is added. `seat` is infallible: `cleared` is total over
        its inputs and `NoClearSpaceWithin` is a verdict, not a refusal.
        `LaunchError` and `SpawnError` are unchanged — a player who cannot be
        cleared still launches.

      **Two deliberately wrong bodies, in sequence, both RED outputs displayed,
      before the right one** (`testing.md` §2 — one skeleton is not enough, and
      which one depends on the phase):

      1. **Do-nothing** (`Clearing::Unneeded` always, no search). This is the first
         tree that compiles, so it is what turns T02 and T03's failures from
         compile errors into *assertion* failures: FR-1.1-S1, S3, S5, S7 and
         FR-1.2-S1..S3 go red on their assertions.
      2. **Over-eager** (always `MovedTo(centre_of(…))`). The do-nothing skeleton
         passes FR-1.1-S2, S4 and S6 **vacuously** — S2 and S6 both expect the
         player not to be moved, and S4 expects the derived spawn untouched — and
         only the over-eager one reddens them. FR-1.1-S4 is the first assertion to
         run against it, and the golden suites are the second witness.

      **Write no ordinary `//` comment in `crates/mc-sim/src` naming a T01
      needle** — `// moved off Simulation::new` is a scan site, because
      `production_text` strips `///` and `//!` and nothing else.

      **At the first moment this tree compiles, before committing**, run
      `cargo clippy --workspace --all-targets --all-features -- -D warnings` in
      full, over the real door rather than T04's stub. A finding **inside a test
      file** goes back to the test author; the implementation context does not edit
      test files, and a lint is not an exception. Then the full suite and
      `scripts/sdd-gate.ps1` green.

- [ ] **T06 Phase 1 mutations, every outcome recorded** — no files changed at rest
      Scenarios: none · implementer · Depends on: T05

      Break the implementation by hand, observe the suite, revert **by hand**, and
      confirm `git diff --exit-code` is clean before continuing. **Record the
      outcome of every one, including the ones that do not bite** — a mutation that
      fails to bite is evidence about the code's structure, not automatically a
      test gap.

      | # | Mutation | Expected to bite |
      |---|---|---|
      | M1 | `seat` computes the clearing and discards it (`Simulation::new(spawn, …)` with the un-cleared `spawn`) | FR-1.1-S1, S7, FR-1.2-S1..S3 |
      | M2 | `seat` passes `Extent` of the whole coordinate space instead of `world.extent()` | FR-1.1-S6 only — S6's whole purpose |
      | M3 | `seat` passes a shrunk extent (one cell smaller on each axis) | FR-1.1-S1 only — the other direction of the same argument |
      | M4 | `seat` calls `clear_the_player` **after** `Simulation::new` publishes | FR-1.1-S7 only, and only if it is read at tick 0 |
      | M5 | `clear_the_player` sets position but not velocity | expected **not** to bite — entering velocity is already zero (`resuming` sets `Vec3::ZERO`, `persistence.rs:196`), so this is inherited rather than newly observable. Record the miss and why. |
      | M6 | add a second `pub fn` to `simulation.rs` that builds the struct by literal and seats a player | T01's scan, via `published:` moving 2 → 3 and `times` 1 → 2 |
      | M7 | delete `) -> Seated` by renaming the door's return type | T01's scan → `TheDoorNoLongerSeatsAPlayer` |

      M6 and M7 are the ones that grade whether T01 is doing anything at all. If
      either fails to bite, the scan is the defect, not the code.

---

## Phase 2 — the two sentences

Scenarios: FR-2.1-S1..S6.

No adaptation window — nothing existing changes signature. **A fresh test author
and a fresh implementer**, switched at this boundary and never mid-phase; T06's
closing report is all they inherit.

- [ ] **T07 [P] What each verdict composes, with no device in reach** — new
      `crates/mc-client/src/notice_test.rs` (sibling unit test, per
      `docs/technical/testing.md`)
      Scenarios: FR-2.1-S1, S2, S3, S4
      Commit D · test author

      `entering` and `reloading` are total functions of a `Copy` enum: no device,
      no window, no `App`, no session. That is the whole point — `report_clearing`
      (`crates/mc-client/src/app/reload.rs:96`, read) sits behind a `wgpu::Surface`
      and a `winit::Window` that nothing in the workspace constructs, which is why
      **the reload's two sentences are today asserted by nothing at all**.

      Assert the exact strings:

      ```
      mycraft: you would have entered the world inside solid blocks, so you were moved to (12.5, 10, 12.5)
      mycraft: you would have entered the world inside solid blocks and nothing within 8 blocks is clear, so you were left inside them
      mycraft: the reload made your cell solid, so you were moved to (x, y, z)
      mycraft: the reload made your cell solid and nothing within 8 blocks is clear, so you were left where you were
      ```

      - FR-2.1-S1: `entering(MovedTo(vec3(12.5, 10.0, 12.5)))` → the first line.
        **`Display` on `f32` is what the expected text was written against**:
        `10.0` renders `10`, inherited from `app/reload.rs:100-106`, not chosen
        here. **No width or precision specifier may be added** — `{:.1}` renders
        `10.0` and reddens the scenario.
      - FR-2.1-S2: `entering(Unneeded)` → `None`.
      - FR-2.1-S3: `entering(NoClearSpaceWithin { blocks: 8 })` → the second line.
        The `8` is the field interpolated, never a literal in the source.
      - FR-2.1-S4: `reloading(…)` → the reload's two, **character-identical to what
        `app/reload.rs:100-112` writes today**. This closes the reload's existing
        hole as a by-product and simultaneously guards against the two sentence
        pairs being unified.

      The reload's two must be read out of `app/reload.rs` before it is edited, so
      "character-identical" is measured against the shipped text rather than
      against a memory of it.

- [ ] **T08 [P] One place composes it, one place says it, and it is said once** —
      new `crates/mc-client/tests/the_entry_sentence_is_said_once.rs`
      Scenarios: FR-2.1-S5, FR-2.1-S6
      Commit D · test author

      Same shape as T01, over `crates/mc-client/src`. A file walk, so it compiles
      and runs before `notice.rs` exists and is **red as `ComposedButNeverSaid`**.

      **Why a scan and not a subprocess.** FR-2.1-S5 is the *policy-is-not-wiring*
      case: a test that calls the same pure function the adapter calls is agreement
      between two copies of one decision, and the adapter can stop calling it
      entirely with both green. A subprocess run cannot reach it either —
      `crates/mc-client/tests/shipped_binary.rs` works only because a missing
      content root refuses *before* a device is opened, and a successful launch
      opens one.

      Verdict, total and enumerated (`architecture.md:358-368`):
      `ComposedOnceAndSaidWhereTheLaunchIsCollected`,
      `AnotherSourceComposesOrSaysIt(Vec<Site>)`, `ComposedButNeverSaid`,
      `NoSourceWasRead`.

      **A needle may not be a whole sentence**, in two independent ways verified in
      the tree: the refusal interpolates its reach — `Clearing::NoClearSpaceWithin`
      carries `blocks` (`crates/mc-sim/src/world/clearing.rs:53`) and the inherited
      print spells `nothing within {blocks} blocks is clear`
      (`crates/mc-client/src/app/reload.rs:110`), so the source never contains the
      literal `8`; and the refusal is ~133 characters and must wrap across a `\`
      continuation, which `production_text` joins with `\n` before the `contains`
      check — exactly as `app/reload.rs:101` and `:110` already do.

      Needles and expectations: **the D6 table above, each re-derived at the moment
      you write it** with `rg -n --no-heading -F "<needle>" crates/mc-client/src`
      and the `///`/`//!` lines dropped by hand. The `Clearing` row is the one that
      bites — see Correction 2. Write into the test file that a bare `rg -l`
      reports **three** files today (`app/reload.rs`, `session/reload.rs`,
      `session/mod.rs`) while the scan sees **two**, because `session/mod.rs:295`
      is a `///` line; and that the expected post-state is the three-file set
      `src/notice.rs`, `src/launch.rs`, `src/session/reload.rs`.

      **The `Clearing` needle is scoped to all of `src`, and `src/app` alone would
      be the wrong scope.** The frame path's *state* does not live on `App`:
      `redraw` takes `&mut Session` (`crates/mc-client/src/app/mod.rs:179`), and
      `crates/mc-client/src/session/reload.rs:79` already parks a `Clearing` on
      `ReloadReport::Accepted`. A verdict parked for re-reading would most
      naturally be parked right beside it — D4's forbidden shape, with a working
      precedent three files away — where a scan scoped to `src/app` could never see
      it. A **file set** is the rule rather than per-file counts, because
      `notice.rs` and `launch.rs` will legitimately name the type several times and
      a count there would churn.

      **`notice::say_entering(` is module-qualified, and that spelling is
      BINDING** — the count is then the number of calls and cannot be moved by
      import style. Splitting `fn say_entering` from `notice::say_entering(` is
      what makes "composed but never said" and "said twice" different answers:
      zero calls with the definition present is `ComposedButNeverSaid`; two calls
      is `AnotherSourceComposesOrSaysIt`, which is FR-2.1-S6's shape.

      **Three controls, each a separate `#[test]` function feeding a different
      non-good variant:** a fixture whose `src/app/frame.rs` spells the clause
      itself → `AnotherSourceComposesOrSaysIt`; a fixture holding `src/notice.rs`
      alone → `ComposedButNeverSaid`; an empty tempdir → `NoSourceWasRead`.

      **Residual hole, recorded rather than papered over:** a site that imports
      `notice`'s consts and composes a third sentence out of them (caught by review
      only), or a `say_entering` whose body stopped writing (caught by T07 plus the
      fact that `say_entering`'s whole body is the `eprintln!`). What backs the
      needles structurally is `-D warnings`: a name imported and not called is an
      unused import and fails the gate, so "named but never called" is not a state
      this tree can be in.

- [ ] **T09 `notice.rs`, and the two call sites** —
      new `crates/mc-client/src/notice.rs`; `crates/mc-client/src/lib.rs`,
      `crates/mc-client/src/app/mod.rs`, `crates/mc-client/src/app/reload.rs`
      Scenarios: turns T07, T08 green — owns none of its own
      Commit E · implementer · Depends on: T07, T08

      ```rust
      const ENTERED_INSIDE_SOLID_BLOCKS: &str = "you would have entered the world inside solid blocks";
      pub fn entering(clearing: Clearing) -> Option<String>;
      pub fn reloading(clearing: Clearing) -> Option<String>;
      pub fn say_entering(clearing: Clearing);
      pub fn say_reloading(clearing: Clearing);
      ```

      - **Each distinguishing clause is a `&str` const**, the idiom
        `CONTENT_NOT_TAKEN_UP` (`crates/mc-client/src/session/reload.rs:57`, used at
        `app/reload.rs:83`) already sets. rustfmt then puts each literal on a line
        of its own and never inside a continuation, which is what T08's needles
        depend on. No `rustfmt.toml` exists, so `format_strings` is off and a
        string literal is never reflowed.
      - `pub mod notice;` in `crates/mc-client/src/lib.rs`.
        `lib.rs:20` claims "This crate holds no policy" — that claim already has
        this exception (the reload's two sentences have been composed here since
        PRO-918). **The header owes a sentence saying so**: this spec makes the
        exception visible and tested rather than widening it.
      - `crates/mc-client/src/app/mod.rs`: one line,
        `notice::say_entering(prepared.clearing);`, inside `collect_preparation`
        (`:417`). **Placed after the uploads succeed**, so a launch that fails to
        reach the device says nothing about where the player was put. It still
        decides nothing.
      - `crates/mc-client/src/app/reload.rs`: `report_clearing` (`:89-115`) deleted
        whole, `use mc_sim::world::Clearing;` (`:12`) with it,
        `take_up_reloaded_content` calling `notice::say_reloading(clearing)` at
        `:50`. `report_reload`'s dedup field is untouched, and the reload's two
        sentences stay character-identical.
      - **`notice.rs` is inside an existing guard and must stay inside it.**
        `crates/mc-client/tests/reporting_seam.rs` walks all of `src` for
        `.to_string()`, `Ending::Failed`, `{failure}`, `{cause}` and `{refused}`.
        Compose with `format!`, bind the coordinates as `{x}`, `{y}`, `{z}` and the
        composed line as `{said}`. A `Vec3` flattened with `.to_string()` reddens
        that guard, and the failure will read as being about error rendering.

      **The once-ness is structural, not a dedup field (D4).** The verdict rides
      `PreparedLaunch` from the preparation worker (`launch.rs:186`) and is moved
      out once by `collect_preparation`, whose guard at `app/mod.rs:418-424`
      returns early unless the handle is finished and whose `:425`
      `self.preparation.take()` leaves `None` behind — so no second frame reaches
      the body. **Do not add a `reported_entry: bool` beside
      `reported`/`reported_remesh`/`reported_swatch`/`reported_reload`
      (`app/mod.rs:106-116`).** Those four exist because their events *recur*; an
      entry happens once per process run, and a dedup field would guard a
      repetition the design need never permit. After collection there is no entry
      `Clearing` anywhere in the client to read.

      **Two deliberately wrong bodies, both RED displayed, before the right one:**
      `None` always reddens FR-2.1-S1, S3 and S4; `Some(…)` always reddens
      FR-2.1-S2. Neither alone is enough.

      Then `cargo clippy --workspace --all-targets --all-features -- -D warnings`,
      the full suite, and `scripts/sdd-gate.ps1` green.

- [ ] **T10 Phase 2 mutations, every outcome recorded** — no files changed at rest
      Scenarios: none · implementer · Depends on: T09

      | # | Mutation | Expected to bite |
      |---|---|---|
      | M8 | delete `notice::say_entering(prepared.clearing);` from `collect_preparation` | T08's scan → `ComposedButNeverSaid`. **This is the whole reason T08 exists**; if it does not bite, FR-2.1-S5 is unproven. |
      | M9 | call `say_entering` a second time in the frame path | T08's scan → `AnotherSourceComposesOrSaysIt` (FR-2.1-S6) |
      | M10 | `entering` returns the reload wording | FR-2.1-S1 and S4 together |
      | M11 | add `{:.1}` to the `y` coordinate | FR-2.1-S1 — the over-tight-vs-inherited-formatting trap, from the other side |
      | M12 | make `entering` and `reloading` one function | FR-2.1-S4 |
      | M13 | inline the two consts at their use sites (no `const`) | expected **not** to bite behaviourally — the composed text is identical. Whether T08's needles survive rustfmt's wrapping is exactly what this measures. Record the outcome either way; a miss here means the needles rest on formatting luck rather than on the const idiom. |

---

## Phase 3 — documentation (Key Principle 3, part of done)

Not a follow-up and not a separate issue. The spec folder is archived to
`specs/archive/2026/` and pruned at 365 days — **the archive is history, not
documentation**, and nothing a future reader needs may be left to it.

Every task here depends on T09.

- [ ] **T11 [P] Player: what is different when they play** — `docs/user/gameplay.md`
      Scenarios: none · Depends on: T09

      The page already carries the reload's story at `:156-172` ("If something you
      are standing in becomes solid, you are moved clear", the eight-block reach,
      the never-outside-the-world rule). The entry story goes **beside** it, not
      inside it, because the two are different events for the player:

      - Resuming a save now places you somewhere you can move **even when a block
        became solid while the game was off** — you quit, you edit, you relaunch,
        and you are standing somewhere rather than inside rock.
      - **The exact line on the terminal**, quoted:
        `mycraft: you would have entered the world inside solid blocks, so you were moved to (12.5, 10, 12.5)`.
      - What happens when nothing within 8 blocks is clear: you are left inside
        them, told so in the second exact line, and **the launch proceeds** — a
        refused launch would take away the edit-and-relaunch escape along with the
        save.
      - That a first launch into a generated world moves you not at all.

- [ ] **T12 [P] Mod author: how to write it, and what a refusal reads like** —
      `docs/modding/hot-reload.md`
      Scenarios: none · Depends on: T09

      The page carries the reload-time story at `:202-246` and a worked
      water-solidity example at `:386`. What it does not carry is the offline case
      at all.

      - A solidity change made **offline** is answered at the next entry exactly as
        one made live is answered at the swap. **Quote the entry wording beside the
        reload's**, so the two are on the page together and the difference is
        visible rather than inferred:

        **One fence per sentence, carrying the real coordinates — never a fence
        holding two, and never an `(x, y, z)` placeholder.** T12a measured why:
        `quoted_refusals_in` joins *every line of a fence into one string* and
        compares it whole, so a fence holding both sentences can never match any
        single produced text, and a placeholder matches nothing at all. The two
        entry lines a run actually produces, verified 2026-08-18:

        ```
        mycraft: you would have entered the world inside solid blocks, so you were moved to (12.5, 10, 12.5)
        ```

        ```
        mycraft: you would have entered the world inside solid blocks and nothing within 8 blocks is clear, so you were left inside them
        ```

        The reload's own two go in their own fences beside them, as the page
        already quotes them. **The harness is strict in a way documentation prose
        is not** — a fence grouping two related sentences and a coordinate
        placeholder are both what a good technical writer produces, and both are
        invisible failures until something runs. Write to the harness's grammar.
      - The same "only ground the world actually holds counts as clear" rule
        applies at entry: near an edge you are moved inward or upward and never out
        over it, and where the eligible ground is all solid you get the
        nothing-within-eight-blocks answer.
      - **A complete worked example that runs**, in the shape the page already
        uses at `:386`: quit while standing in water, edit
        `content/base/blocks/water.luau` from `solid = false` to `solid = true`,
        relaunch with `--load-changed-blocks`, and read the line you get. A
        reference that lists names without showing a working example is not
        documentation.

- [ ] **T12a The harness must produce the entry sentences before T12 quotes them**
      — `crates/mc-client/tests/support/printed_refusals.rs`
      Scenarios: none · Depends on: T09 · **Blocks: T12**

      Found by phase 2's test author and confirmed against the harness. **T12 as
      written above reddens the suite**, and this task is what makes it landable.

      `crates/mc-client/tests/documented_refusals.rs` treats **any fenced block in
      `docs/modding/` whose first line begins `mycraft: `** as a quoted refusal
      (`REFUSAL_PREFIX`, `:135`) and matches it line-for-line against text produced
      by a real run; anything unmatched is `Verdict::Mismatch`. The recogniser is
      derived from the artefact rather than agreed with an author — that is the
      point of it, and it is why quoting the entry sentences in a fence is caught.
      No run in that harness produces an entry sentence today.

      **Ruled: extend the producer rather than dodge the recogniser.** The two
      candidate escapes were (a) quote the sentences inline as prose instead of
      fenced, and (b) teach `printed_refusals.rs` to produce them. (a) buys a green
      suite by making the documentation unguarded — the page would promise text
      nothing checks, which is the exact failure `documented_refusals.rs` exists to
      catch and which its own header records happening once already. Take (b).

      The four texts already produced for `docs/modding/hot-reload.md` show the
      shape (`documented_refusals.rs:58`); the entry pair is a fifth and sixth of
      the same kind. **Produce them through `notice::entering` — production's own
      composer** — never by spelling the expected sentence out here. A guard that
      restates the text is comparing the page against a third copy of somebody's
      belief about the program, which the harness header names as the thing it
      refuses to be.

      `docs/user/gameplay.md` is outside the scanned roots and needs nothing.

- [ ] **T13 Engine reader: the as-built record** —
      `docs/technical/architecture.md`, `docs/technical/testing.md`
      Scenarios: none · Depends on: T09

      **`docs/technical/architecture.md`** — the new material:

      - The seating door: why it is the constructor and not the launch path, what
        the compiler holds (**against every caller including `mc-sim`'s own,
        because `new` is module-private**) and what it does **not** hold — a second
        seating path added inside `simulation.rs`, which is why FR-1.3's scan
        exists and what its residual hole is (a second `pub fn` that calls `seat`
        and discards the `Clearing`: the rule still runs, only the reporting is
        lost).
      - The factored rule and how a join inherits it: MVP 3's join calls
        `clearing::clear_the_player(&mut joining, &self.world, self.world.extent())`
        and gets the same `Clearing` back. **The omission it cannot make is
        forgetting the rule; the mistake it can still make is supplying the wrong
        ground**, which nothing here makes visible and which whichever spec adds
        the join owes a scenario for.
      - Why the search is unconditional: `RegistryVerdict`
        (`crates/mc-world/src/persistence/table.rs:52`) is computed and dropped
        inside `load_world`, `LoadedWorld` carries only `{ world, player }`, gating
        would mean coupling persistence to physics, and a clear player costs two
        block reads because `cleared` opens with an `overlaps_solid` early return
        (`clearing.rs:64`).
      - Where the verdict is parked and why once-ness is structural rather than a
        dedup field.
      - **Why the entry notice is said only after the uploads succeed.** This
        rationale exists in **no file** — it was written as a comment in
        `app/mod.rs` during T09 and dropped to keep that file inside its 500-line
        cap, which it is now exactly at. `docs/technical/architecture.md` is its
        only remaining home:

        > A launch that fails to reach the device must not tell a player where
        > they were put in a world they will never see.

        Record the mechanism with it — the cap did not reject code, it **evicted
        an explanation**, silently, with nothing going red and no deletion for a
        reviewer to see. Note also that `app/mod.rs` has **zero margin left**, so
        the next line added to it forces a split that should be planned rather
        than discovered.
      - **That a cleared player arrives with `on_ground: false` and settles on
        tick 1**, and that this is deliberate: `resuming` does not set it because
        it would be a claim about contact nothing checked, and grounding the player
        is Out of Scope. This is the single thing most likely to be re-derived
        wrongly by a future reader.
      - That the move is what the next save records, so entry clearing is
        self-limiting by that route rather than by a stored flag.

      **Four passages go stale and are corrected in the same pass** — line numbers
      re-derived 2026-08-18 with
      `rg -n "simulation_for\(world: &ReplayWorld|simulation_at_launch\(save, seed|launch::simulation_to_play|non-fatal notices" docs/technical/architecture.md`:

      | Line | What is stale |
      |---|---|
      | `:295` | the `simulation_for` signature — now returns `Result<Seated, SpawnError>` |
      | `:647` | the `simulation_at_launch` signature — now returns `Result<Seated, LaunchError>` |
      | `:669` | the launch ordering paragraph naming `launch::simulation_to_play` |
      | `:1607` | "four **non-fatal notices**" and their list. It names `app.rs`'s dropped-frame, unshowable-edit and swatch notices and `events.rs`'s cursor-release notice, and **does not include the reload's clearing notices at all**. This spec adds the entry notice and moves both clearing notices into `notice.rs`. |

      `architecture.md` cites the launch-ordering passage as `:670`; the string is
      at `:669`. The observation that these notices bypass the reporting sink stays
      a deferred observation — **the count and the list do not**.

      **`docs/technical/testing.md`** — the entry-door fixtures; the compose/write
      split and why it exists (a composition behind a device nothing constructs is
      a composition nothing asserts); both scans with their verdicts and their
      three controls each; **why a needle may not be a whole sentence** (the
      interpolated reach and the `\` continuation joined with `\n`); why
      `production_text`'s `///`/`//!` stripping makes a bare `rg` count the wrong
      instrument, with `crates/mc-sim/src/reload/mod.rs:91` and
      `crates/mc-client/src/session/mod.rs:295` named as the two standing
      counter-examples; and **the mutations of T06 and T10 with their recorded
      outcomes, including M5 and M13 if they did not bite**.

      **Three further scan exposures, measured in phase 2**, belong beside the two
      from phase 1 — a test file's header is capped at 600 lines and the spec
      folder is pruned, so `docs/technical/testing.md` is the only durable home:

      - **`fn say_entering` was a prefix test**, satisfied by `say_entering_now`;
        tightened to `fn say_entering(`. Same class as `) -> Seated` being
        satisfied by `) -> SeatedPlayer`. **A substring needle matches its own
        extension** — check every needle against that before trusting it.
      - **The file-set rule has no ceiling where the counted rules do**, which is
        why a fourth file naming `Clearing` is caught but a second offence inside
        an already-named file needs the `times` count.
      - **The reading order is missing-before-extras, and the masking it buys is
        deliberate**: while anything is unstated, a second source saying it is
        invisible. Never both true in a tree anyone would commit — but say so,
        because a reader who does not know it will mis-read a verdict once.

- [ ] **T14 Fill in the test-map's mutation column and close the phase** —
      `specs/active/2026-08-18-entry-door-clearing/test-map.md`
      Scenarios: none · Depends on: T06, T10, T13

      `test-map.md` is written now with its scenario↔test mapping and its
      "Additional coverage" section. What it cannot carry yet is the mutation
      outcomes. Fill them in from T06 and T10 before the phase closes — **including
      every mutation that did not bite, with the reason.**

      **A test red for a known reason is fixed before the phase closes, never
      annotated.** Red for a known reason hides red for an unknown one, and a
      known-red test invites deferral, which is precisely the state in which it
      stops reporting anything new.

---

## Scenario assignment — every scenario in exactly one task

| Scenario | Task |
|---|---|
| FR-1.1-S1, S2, S3, S4, S5, S6, S7 | T02 |
| FR-1.2-S1, S2, S3 | T03 |
| FR-1.3-S1, S2 | T01 |
| FR-2.1-S1, S2, S3, S4 | T07 |
| FR-2.1-S5, S6 | T08 |

18 scenarios, 5 scenario-owning tasks, no scenario assigned twice and none
unassigned. T04, T05, T06, T09, T10 and T11–T14 own no scenarios: they are the
adaptation, the two implementations, the two mutation passes and the
documentation, and each is named as such above.

## Notes

- **Out of Scope is binding**, and three of its entries are reachable by an
  implementer looking for a cheap green: grounding the player or setting
  `on_ground` in the search (the FR-1.1-S7 tick-0 trap), editing `centre_of` (the
  `y + 0.5` trap), and widening `mc-world`'s persistence return to gate the search
  (FR-1.2-S2). Any of these is a stop-and-raise, not a fix.
- **D8 stands deferred**: the walk-and-needle scaffolding is copied a fourth and
  fifth time rather than extracted into `mc-testkit`. Each guard is worth exactly
  as much as a reader's ability to see the whole of it in one file, and a shared
  engine makes every guard weakenable by an edit made for another guard's sake.
  Revisit when a sixth guard is written, or when the copies are found to have
  drifted in a way that mattered.
- **A future join supplying the wrong ground** is not made visible by anything in
  this spec. D2 records it; whichever spec adds the join owes it a scenario. Not
  buildable here — there is no join to grade.
- **A save recording a position outside the loaded world** is Out of Scope and
  belongs to whichever spec makes the world streamed. The search answers
  `Unneeded` there, so such a player is not moved and falls.
