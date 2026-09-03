---
id: SPEC-033
title: Nothing fails silently — five shipped defects where the person who can act is not told
status: implemented
completed: 2026-09-03
work-type: fix
rigor: low
branch: bugfix/PRO-961-nothing-fails-silently
issue: PRO-961
created: 2026-09-03
updated: 2026-09-03
approved: 2026-09-03
author: spec-PRO-961
---

# Fix: Nothing fails silently

Five shipped defects with one shape: **when something goes wrong, the person who
can act on it is not told what or where.** Four are silent or unhelpful failures.
One — PRO-961 — is the product doing the wrong thing outright, and it is the
reason this spec exists.

Each defect keeps its own section below because each has its own mechanism and
its own regression scenarios. They are batched because they share a definition
of done, not because they share code.

## Rigor

`low`, at the owner's direction and confirmed against the work. Every one of
the five is a narrow, reversible behaviour correction inside an existing design:
no persisted-format change, no wire-protocol change, no new Luau declaration
field, no crate boundary crossed, and nothing a later spec must not break.
`SCENE_REVISION` does not move — the aiming walk is not the golden frames'
`March` (`crates/mc-sim/src/world/action/trace.rs:19-25`), so no committed image
is affected.

TDD binds unchanged at `low`: tests are written first and their failing output
is displayed before any implementation. What `low` drops is the reviewer
workflow; the gate carries it.

**PRO-938 was split out of this batch** and is not specified here. It alters the
published mod-author fault vocabulary, which is a tier this spec's other work
should not pay for.

## Stakeholders and what they can do

| Stakeholder | What they can do that they cannot do today |
|---|---|
| Player | Aim, break and place while swimming, instead of being unable to interact with anything |
| Player | Read which texture keys drew a stand-in, instead of a sentence that names none |
| Player | Learn that edits have stopped being drawn, instead of watching a stale world |
| Mod author | Read the name of the key their first block declared but nobody baked |
| Harness author (in Rust) | Capture, redirect or silence every non-fatal client notice, through the sink a caller supplies |

**The last row is library-only and is stated that way deliberately.** A caller
embedding `mc_client` supplies a `Notices` and reads back or discards every line;
the shipped binary takes no argument for it, so **a server operator cannot
exercise this from a command line** — and a server operator does not run the
client binary in the first place, so the row named the wrong person as well as
promising more than landed. A flag is filed as PRO-1009 rather than written,
because a flag is a published surface: whether it silences refusals and endings
too, whether a silenced run still says why it refused to start, and whether
silence and redirection are one switch or two are decisions worth taking on
purpose. Principle 9 binds at the spec level and this spec clears it several times
over — a swimming player can aim, and a launch names the textures nobody baked.

---

## Defect 1 — A submerged player can aim at nothing (PRO-961)

- **Observed**: A player whose eye is inside a block that a ray may stop at
  targets that block, at distance 0 and with no entry face. With the shipped
  water, a swimming player's every swing hits the water their head is in and is
  refused (water declares `breakable = false`), and every placement is refused
  for want of a face. They can interact with nothing.
- **Expected**: A block you can see through does not stop your ray at the cell
  your own eye is in. Aiming while submerged reaches the lakebed, as it did
  before water became targetable.
- **Reproduced**: 2026-09-03, `cargo test --package mc-sim`, throwaway
  integration test since removed. A player standing at `(8.0, 10.0, 8.5)` — eye
  at `y = 11.62`, so eye cell `(8, 11, 8)` — with `fixture:aimable`
  (`is_solid = false`, `targetable = true`, water's own shape for this question)
  in that eye cell and `base:dirt` four blocks along the row at `(12, 11, 8)`,
  asked for one `Break`. The edit report was
  `Changed { cell: (8, 11, 8), from: fixture:aimable, to: Empty }` — the swing
  took the block the player's head was in. The expected diff was the cell at
  `(12, 11, 8)`.

### Root Cause

`targeted()` in `crates/mc-sim/src/world/action/trace.rs:66-97` seeds its walk
with `met.cell = containing(origin)` and then tests `is_targetable(met.cell)` at
line 78 **before** taking any step. The origin cell is therefore judged by the
same rule as every stepped cell.

That predates PRO-904 and was harmless while only solid blocks were targetable,
because a player's eye is never inside a solid block. `content/base/blocks/water.luau:118`
declares `targetable = true`, and from that moment a submerged eye stops its own
ray at distance 0.

The declaration that draws the right line already exists and is already
per-block content: `occludes` (`crates/mc-core/src/block/definition.rs:97`,
declared at `content/base/blocks/water.luau:117` as `occludes = false`). A block
you can see through should not stop a ray at the cell you are standing in; a
block you cannot see through should. `mc-sim` does not carry occlusion today —
`ResolvedVoxels` packs solidity, targetability and medium — so the fix supplies
it, entirely inside `mc-sim`.

### Regression Scenarios

- **D1-S1**: WHEN a player whose eye is inside a block that does not occlude
  swings at a block four cells along their view THE SYSTEM SHALL break that
  fourth block and leave the block the eye is inside untouched.
- **D1-S2**: WHILE a player's eye is inside a block that occludes THE SYSTEM
  SHALL report that block as the target, at distance 0 and with no entry face.
- **D1-S3**: IF a player is swimming in the shipped water with the lakebed
  within reach and swings THEN THE SYSTEM SHALL break the lakebed block rather
  than refusing the swing as indestructible.
- **D1-S4**: IF a player is swimming in the shipped water with the lakebed
  within reach and asks to place a block THEN THE SYSTEM SHALL place it against
  the lakebed rather than refusing the placement for want of a face.
- **D1-S5**: IF a player's eye is inside a block that does not occlude and no
  block a ray may stop at lies within reach THEN THE SYSTEM SHALL report no
  target at all.

D1-S2 is the control. An implementation that skips the origin cell
unconditionally satisfies S1, S3, S4 and S5 and fails only this one.

---

## Defect 2 — The launch notice names no uncovered key (PRO-990)

- **Observed**: `crates/mc-client/src/main.rs:37` prints `PALETTE_NOTICE`
  unconditionally, before the content root is read and before any texture set is
  judged. It explains the rule that an unbaked key draws a generated stand-in
  and it names no key. It prints identically whether every declared key is
  covered or none is.
- **Expected**: The launch names the keys that had no image, and says nothing
  when every declared key is covered.
- **Reproduced**: 2026-09-03, by reading `crates/mc-client/src/main.rs:24-42`:
  the `println!` takes no argument derived from any set and precedes `run()`,
  which is the only thing that reads one. The stronger evidence is historical
  and is recorded in PRO-990: `base:water` was uncovered for three days, the sea
  rendered as a magenta checkerboard, this notice printed at every launch, and
  the defect was found by a person looking at the screen.

### Root Cause

The notice is a constant printed before the information it would need exists.
The information itself is already computed and already reachable: a launch
produces `PreparedLaunch` carrying both the declared keys
(`LayerAssignment`, `crates/mc-core/src/content.rs:237`) and the covered ones
(`SuppliedTexels::covering`, `crates/mc-render/src/texture/supplied.rs:54`), and
`crates/mc-client/tests/the_shipped_set_covers_every_key_it_declares.rs:227`
already performs exactly this comparison for the base game. What is missing is
that no shipped path performs it and tells anyone.

A test binds the base game only — an uncovered key is correct by design for a
third-party mod, so a gate stage would be wrong. The notice is the only channel
that reaches a mod author.

### Regression Scenarios

- **D2-S1**: WHEN a launch's built set covers every texture key the content
  declares THE SYSTEM SHALL print no stand-in notice at all.
- **D2-S2**: WHEN a launch's built set leaves declared texture keys uncovered
  THE SYSTEM SHALL name every uncovered key, in ascending key order, omitting
  none.
- **D2-S3**: IF a declared key is covered by the built set THEN THE SYSTEM SHALL
  NOT name it among the keys that drew a stand-in.

D2-S3 is the control. An implementation naming every declared key satisfies S2
and fails only this one.

---

## Defect 3 — Non-fatal notices write to the process error stream directly (PRO-941)

- **Observed**: Non-fatal client notices call `eprintln!` directly. Nothing can
  capture, redirect or silence them: a test cannot read them and a caller cannot
  route them elsewhere.
- **Expected**: Every non-fatal notice the client library emits goes through the
  caller-supplied sink the reported ending already uses, leaving exactly one
  place in the crate that names the process error stream.
- **Reproduced**: 2026-09-03, `grep -rn "eprintln!" crates/mc-client/src/`.

### Root Cause

The reported *ending* writes through a caller-supplied stream — `main.rs:41`
passes `&mut io::stderr()` to `report`, and that is the one place a stream is
chosen. Non-fatal notices never took that path, because the reporting guards are
about rendering rather than about streams, and a notice that does not end the
run never reaches them.

**The count has been wrong three times, and it was wrong in this spec too.**
PRO-941 names four. `docs/technical/architecture.md` said seven and narrated the
first correction itself: "The count was four and the list omitted the two
clearing notices". Its table then omitted the changed-blocks notice. This
section said **eight**, and was right about the tree it was written against —
and then **Defect 2 of this same spec added `notice::say_stand_ins`**. The tree
holds **nine**, counted with `grep -rn "eprintln!" crates/mc-client/src/`, whose
tenth hit (`session/reload.rs:49`) is a doc comment naming `eprintln!` rather
than a call site:

| Site | Notice | In PRO-941 | In the as-built table |
|---|---|---|---|
| `crates/mc-client/src/app/report.rs:32` | a frame was dropped | yes | yes |
| `crates/mc-client/src/app/report.rs:45` | a held block draws no indicator | yes | yes |
| `crates/mc-client/src/app/report.rs:62` | an edit could not be shown | yes | yes |
| `crates/mc-client/src/events.rs:403` | the cursor could not be released | yes | yes |
| `crates/mc-client/src/app/reload.rs:95` | content was not taken up | no | yes |
| `crates/mc-client/src/notice.rs` | what entry did about where you stand | no | yes |
| `crates/mc-client/src/notice.rs` | what a reload did about where you stand | no | yes |
| `crates/mc-client/src/notice.rs` | which blocks a save no longer agrees about | no | **no** |
| `crates/mc-client/src/notice.rs` | which declared texture keys nothing baked | no | **no** — it did not exist |

Every column has disagreed with the tree at some point, and the last row is this
spec disagreeing with itself one defect later. **That is the argument for the
seam rather than for counting more carefully**: a hand-maintained list of these
sites has drifted on every occasion anybody has counted, including this one.
Converting only some would also falsify `crates/mc-client/src/notice.rs`'s own
docstring, which states that writing straight to the error stream is "the
convention this crate's other non-fatal notices already follow" — a sentence that
stops being true the moment half of them stop following it. See *Decisions taken
in this spec*.

### Regression Scenarios

- **D3-S1**: WHEN the client emits any non-fatal notice THE SYSTEM SHALL write
  it through the caller-supplied sink rather than naming the process error
  stream. (Parameterised over all nine notices above; one test per notice where
  the site is reachable, and a source scan over the whole crate where it is not.)
- **D3-S2**: WHEN a caller supplies a sink THE SYSTEM SHALL let that caller read
  back the exact text of a non-fatal notice the run emitted.
- **D3-S3**: IF a run emits no non-fatal notice THEN the sink SHALL receive
  nothing.
- **D3-S4**: WHEN the same non-fatal notice recurs across frames THE SYSTEM
  SHALL write it to the sink once.

D3-S3 is the control against a sink written to unconditionally. D3-S4 guards the
per-reporter deduplication that `crates/mc-client/src/app/report.rs` holds
today and that this change is capable of dropping.

---

## Defect 4 — Refusal guidance travels with the call rather than with the failure (PRO-940)

- **Observed**: The sentence telling a player what to do about a refusal — such
  as offering `--load-changed-blocks` — is passed as an argument where the
  reported ending is constructed. A call site that supplies the wrong thing, or
  nothing, emits a refusal with no guidance, and nothing holds it to supplying
  it.
- **Expected**: The guidance is a property of the failure, so a site cannot fail
  to supply it.
- **Reproduced**: 2026-09-03, by reading the three construction sites —
  `crates/mc-client/src/main.rs:53`, `:71` and
  `crates/mc-client/src/app/mod.rs:201`, each spelling
  `Ending::failed(&failure, &failure.way_out())`. PRO-940 records the measured
  half: replacing the guidance argument on the one production line that can
  actually emit the sentence leaves the entire suite green.

### Root Cause

`Ending::failed(failure, guidance)` (`crates/mc-render/src/window.rs:205`) takes
guidance as a second parameter. What holds the property today is that the
parameter is not optional and that the ending cannot be constructed outside its
three doors. That is a shape argument, not a test — and the one site that can
emit the sentence runs inside a redraw needing a graphics device and a display
server, so no test reaches it.

### Regression Scenarios

- **D4-S1**: WHEN a launch is refused for a reason that has a way out THE SYSTEM
  SHALL print that way out beside the refusal.
- **D4-S2**: IF a refusal has no way out THEN THE SYSTEM SHALL print the refusal
  with no guidance sentence appended.
- **D4-S3**: WHEN a refusal is reported from within a running session THE SYSTEM
  SHALL print the same way out that the same failure prints at launch.

D4-S2 is the control against an implementation that appends a constant. D4-S3 is
the scenario the issue's measurement says nothing can currently reach; satisfying
it requires the guidance to be readable without opening a device, which is the
observable consequence of moving it onto the failure.

## Architecture Delta

Guidance becomes a property of the failure type rather than a parameter of the
call that reports it — a contract every reportable client failure must satisfy.
This is the only binding contract this spec introduces; the other four defects
extend existing structures only.

Note that the phase matrix declares no `architect` phase for `work-type: fix` at
any rigor (`.prospect/prompts/matrix.tsv`), so this delta is settled here and in
the implement phase rather than by an architect phase. See *Decisions taken in
this spec*.

---

## Defect 5 — A dead re-mesh worker (PRO-949) — verification only

- **Observed**: PRO-949 reports that `Remesher::collect` cannot distinguish a
  dead worker from an idle one, so the client says nothing when meshing has
  stopped.
- **Status**: **The production behaviour has already landed** and the issue is
  stale. `crates/mc-client/src/remesh.rs:102` declares
  `Collecting { NothingYet, WorkerGone, Finished(Remeshed) }` and `collect()` at
  `:204` maps `TryRecvError::Disconnected` to `WorkerGone`;
  `crates/mc-client/src/app/mod.rs:229` reports `WORKER_GONE` to the player.
  `git log -S "WorkerGone" -- crates/mc-client/src/remesh.rs` attributes it to
  `45b30b44 feat: hot reload — edit a block declaration and see it change (PRO-918)`.
- **What is not established**: that anything asserts through the shipped path.
  The `TheWorkerIsGone` arms are in test *support*
  (`crates/mc-client/tests/support/reload_remesh/collecting.rs:134,215`), which
  proves the arm exists, not that a test reaches it. `standards/global/testing.md`
  §2 is explicit that a second entry point onto a tested path is untested until
  something asserts through it.

### Regression Scenario

- **D5-S1**: WHEN the worker that re-meshes edits has stopped THE SYSTEM SHALL
  tell the player that their edits are no longer being drawn.

One scenario, at the owner's direction. It either confirms the fix or finds the
hole, and it is written about what the player is told rather than about the shape
of the verdict.

---

## Decisions taken in this spec

1. **PRO-941 widened from four sites to nine.** The issue names four, the
   as-built record said seven, this spec said eight, and Defect 2 of this spec
   then added a ninth. Converting only the four named leaves the crate
   half-routed and makes `notice.rs`'s stated convention false. Every count
   anybody has taken of these sites has been superseded, which is the case for
   the seam rather than for a better list. Recorded for the owner rather than
   assumed silently.
2. **The stand-in notice lists every uncovered key rather than a truncated
   summary.** PRO-990 illustrates the notice as "`base:water` and 2 others",
   explicitly as an example. `crates/mc-client/src/notice.rs:102-110` already
   sets the opposite convention for the changed-blocks notice, in terms that
   apply here word for word: "Every name, ascending, complete, and never
   truncated ... a bounded list is one they cannot act on." A mod author has to
   go and bake each missing key, so each has to be named.
3. **No architect phase will run.** `.prospect/prompts/matrix.tsv` declares
   `architect` rows for `work-type: feature` at `high|xhigh|max` only — there is
   no `fix` architect row at any rigor, and no architect row at `low` for any
   work type. Defect 4's Architecture Delta is therefore settled in this spec and
   in the implement phase.
4. **Branch named `bugfix/`, not `feature/`.** `/sdd-start` and
   `standards/global/git-workflow.md` both derive `bugfix/` for defect work.
   The `PRO-961` prefix is kept as instructed.

## Out of Scope

Recorded, not built.

- **PRO-938** — a malformed `follow_up` entry is skipped with no fault. Split
  into its own spec: it alters the published mod-author fault vocabulary
  (`FaultKind`, documented to authors in `docs/modding/script-faults.md`). The
  owner has ruled its design separately and posted that ruling to Linear. Write
  no scenarios for it here.
- **PRO-974** — the gate's blank-line size counter. Explicitly excluded by the
  owner; it has its own spec.
- **`SCENE_REVISION`** does not move. If any fix here would invalidate the
  golden set, stop and escalate rather than bumping it.
- **`content/base/`** is not touched. `occludes` is already declared there;
  Defect 1 needs `mc-sim` to carry it, not content to change.
- **Shortening PRO-972's stand-in documentation.** PRO-990 notes that the
  worked example documenting what a stand-in looks like on sight exists
  *because* nothing tells a mod author, and can be shortened once the notice
  lands. Deferred: the documentation obligation of this spec is to describe the
  new notice, not to retire prose written for a different audience.

## Existing Code to Leverage

| What | Location | Reuse |
|---|---|---|
| `occludes` on a block definition | `crates/mc-core/src/block/definition.rs:97` | Already declared per block; Defect 1 carries it into `mc-sim` |
| Packed voxel views | `crates/mc-sim/src/replay/resolved.rs:355-378` | The pattern a fourth view follows |
| Aimable/unaimable fixtures | `crates/mc-sim/tests/support/chamber.rs:212-213` | Blocks where targetability and solidity disagree, in both directions |
| Declared-vs-covered key comparison | `crates/mc-client/tests/the_shipped_set_covers_every_key_it_declares.rs:227` | The comparison Defect 2 moves onto a shipped path |
| The caller-supplied reporting stream | `crates/mc-client/src/main.rs:41` | The sink Defect 3 routes the eight notices through |
| Per-reporter deduplication | `crates/mc-client/src/app/report.rs` | Behaviour D3-S4 must preserve |
| `way_out()` on a startup failure | `crates/mc-client/src/startup.rs:261` | The guidance Defect 4 moves onto the failure |

## Documentation owed (Principle 4)

Part of done, not a follow-up.

- **Player** — that aiming works while swimming; that the launch names textures
  nobody baked; that the client says so when edits stop being drawn.
  Routed through `docs/user/gameplay.md`, which already describes swimming.
- **Mod author** — how to read the uncovered-key notice and what to do about it
  (`docs/modding/`), and the note that a stand-in is expected for a key they
  have not baked.
- **Engine reader** — the origin-cell rule and why it reads `occludes` rather
  than solidity; the reporting seam now that the client names one stream in one
  place (`docs/technical/architecture.md:2440-2465` records the deviation as
  seven notices and its table is one short — the whole section is replaced, not
  amended); guidance as a property of the failure.

## Notes

Recorded, not built.

- **The client has no `--quiet`** — **PRO-1009**. Defect 3's sink is reachable
  from Rust and only from Rust. The owner ruled against writing the flag here: it
  is two lines of code and not two lines of consequence, since a flag is a
  published surface and settling it by writing the lines is how a surface gets
  decided by accident. The stakeholder row above was rewritten to say what is true
  rather than the flag being added to make the row true.
- **Two counts, both measured, and they are not the same number** — **PRO-1008**.
  Of the nine notices, **five** have a test that reddens when their *wording*
  changes and **four** have one that reddens when the *call* disappears. Defect 5
  moved the first from four to five and left the second where it was, so
  `report_remesh` is half-closed rather than closed. The four device-bound sites
  with neither are the three other `App` reporters and the cursor release. The
  source scan is their whole evidence and it is blind in one direction — it catches
  a site going back to the process stream, never one that stops saying anything —
  and it is file-granular, so seven of the nine sites share two outcomes. Making it
  per-site is cheap and strictly better whether or not a windowed harness ever
  exists. Full table in `test-map.md` and `docs/technical/testing.md`.
- **A dead re-mesh worker is still fixture-unreachable.** Defect 5 closed the
  wording and not the state. The seam that would close the state is an injectable
  worker, which `docs/technical/testing.md` had already ruled belongs with a
  reading of a real occurrence rather than beside the arm it would test. Unchanged.
- **Three sentences this spec's own changes falsified were corrected in the complete
  phase, not deferred.** `crates/mc-client/tests/support/reload_remesh/collecting.rs`
  said the `TheWorkerIsGone` arms have "no witness at all", which Defect 5 made half
  wrong; `crates/mc-client/src/notice.rs`'s `say_entering` docstring still described
  writing "straight to the error stream ... the convention this crate's other
  non-fatal notices already follow", two lines above a body that now calls
  `notices.say`; and `crates/mc-client/src/session/reload.rs` declared its constant
  "rather than at the `eprintln!` that writes it" when Defect 3 had left the crate
  with no `eprintln!` at all. A sentence a change makes untrue is that change's
  defect, not the next editor's inheritance.

## Validation

**2026-09-03 — PASS.** `scripts/sdd-gate.ps1` exits 0: `1747 tests run: 1747
passed (14 slow), 1 skipped`, lines 93.85%, regions 92.4%.

All five defects implemented, each with its failing output displayed before any
implementation and recorded in the commit that carried the tests. Rigor `low`, so
the gate carries the review; what stands in for a reviewer on Defect 3 — where
`crates/mc-client/` is excluded from the coverage denominator wholesale — is the
per-site mutation table in `test-map.md`, taken at the owner's direction.

Three findings are reported rather than closed, and they are in `## Notes` above.
