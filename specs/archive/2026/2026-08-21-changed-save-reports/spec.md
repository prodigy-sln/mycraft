---
id: SPEC-020
title: A save whose blocks changed behaviour loads and reports, rather than being refused
status: implemented
rigor: low
branch: feature/PRO-956-changed-save-reports
issue: PRO-956
created: 2026-08-21
completed: 2026-08-22
---

# Mini-Spec: A save whose blocks changed behaviour loads and reports

## Goal

Loading a save is refused when any block in it changed *behaviour*, while live
hot reload accepts the same edit and, where it is genuinely dangerous, moves the
player rather than refusing. That is inverted relative to risk. Make accepting the
default, name the changed blocks on the error stream, and keep the strictness
behind `--refuse-changed-blocks`.

## Why this is `low`

The change is a default in `acceptance_from`, a flag rename, one stderr line, two
test expectations and a documentation sweep. No format change, no new subsystem,
and the behaviour/appearance fold is untouched. **Reverting it is one line.**

**The one risk worth checking was the trapped player** — accepting a save whose
blocks changed could put somebody inside a cell that became solid. It is already
handled, and verified here rather than taken on report:
`simulation_at_launch` calls `seat` on the loaded-save arm
(`crates/mc-sim/src/persistence.rs:153`), and `seat` calls
`clearing::clear_the_player` unconditionally
(`crates/mc-sim/src/simulation.rs:149`) — there is no branch on what the load
reported. `crates/mc-client/tests/entry_clears_whatever_the_load_reported.rs`
exists for exactly this, and its own header states that its falsifier is a launch
over a save whose blocks **all still match**, whose recorded feet are inside solid
anyway. So the data-safety argument is carried by work that shipped on
2026-08-18, and this change adds no new exposure to it.

## What a stakeholder can do

A **mod author** can edit a block's declared behaviour offline and relaunch into
their existing world, reading which blocks the save disagrees with them about. A
**player** can no longer break water, and gets their world back after a content
update instead of a refused launch.

## Scenarios

Flat list; this is also the task list. Each becomes at least one test, mapped in
this folder's `test-map.md`.

- S1: WHEN the client is launched with no acceptance argument over a save
  recording a block whose declared behaviour has since changed THE SYSTEM SHALL
  load that world cell for cell and place the player at the position, yaw and
  pitch that save recorded.
- S2: IF the save names a block the running content does not declare at all THEN
  THE SYSTEM SHALL refuse the launch and name that block, whatever acceptance
  argument was passed.
- S3: WHEN a save recording `base:omega` and `base:alpha`, both of whose declared
  behaviour has since changed, is loaded THE SYSTEM SHALL write one line to the
  error stream naming `base:alpha` before `base:omega` and stating that the world
  was loaded anyway.
- S4: WHEN a save whose blocks' declared behaviour all still match the running
  content is loaded THE SYSTEM SHALL write no such line.
- S5: WHEN a save whose blocks differ from the running content in declared
  appearance only is loaded THE SYSTEM SHALL write no such line.
- S6: WHEN the client is launched with no save at the path it reads THE SYSTEM
  SHALL generate a world and write no such line.
- S7: WHEN the built client binary is run over a save recording one changed block
  THE SYSTEM SHALL write that line to the process's own error stream.
- S8: WHEN the client is launched with `--refuse-changed-blocks` over a save
  recording a block whose declared behaviour has since changed THE SYSTEM SHALL
  refuse the launch and name that block.
- S9: WHEN the client is launched with `--refuse-changed-blocks` over a save whose
  blocks all still match the running content THE SYSTEM SHALL load that world and
  place the player where the save recorded them.
- S10: IF the client is launched with `--refuse-changed-block` — the same argument
  less its final letter — over a save recording a changed block THEN THE SYSTEM
  SHALL load that world and name that block, because that is not the argument.
- S11: WHEN a launch is refused only because `--refuse-changed-blocks` was passed
  over changed blocks THE SYSTEM SHALL name that argument as the thing to drop.
- S12: IF a player breaks at a cell holding `base:water` THEN THE SYSTEM SHALL
  refuse the break and leave the water in the cell.
- S13: WHEN a player places a block into a cell holding `base:water` THE SYSTEM
  SHALL replace the water with that block.
- S14: WHEN the shipped content is compared against the committed save written
  before those blocks were Luau THE SYSTEM SHALL classify `base:water` as having
  changed behaviour and `base:dirt`, `base:grass` and `base:stone` as having
  changed appearance only.
- S15: WHEN that same save is loaded with no acceptance argument THE SYSTEM SHALL
  load it and write a line naming `base:water` and no other block.

Counted with:

```
grep -c -E "^- S[0-9]+:" specs/active/2026-08-21-changed-save-reports/spec.md
```

## Out of Scope

- **Regenerating `crates/mc-world/tests/fixtures/world_saved_against_the_toml_declarations.mcw`.**
  Nothing here requires it, and the day it happens the test stops being evidence
  about anything.
- **Touching the behaviour/appearance fold**, its field lists, or its two revision
  bytes.
- **Reporting retextured blocks.** `RegistryVerdict.retextured`'s doc comment says
  it is "reported so that a caller can say so", and no caller has ever said
  anything — `refuses` does not read the field and `resolve` has one production
  call site, which drops the verdict. The comment is **retracted** rather than the
  promise kept: a doc comment describing a caller that has never existed is the
  same defect class as the 31 prompt lines below, a record of a program nobody
  wrote.

  What the field is actually for, and what replaces the retracted sentence: it is
  the third arm of a **total** classification. Every block a save names lands on
  exactly one of `missing`, `changed` or `retextured`, which is what lets a test
  compare the whole verdict at once instead of asserting an absence — and that is
  precisely what S14 does. Drop `retextured` and a block whose appearance alone
  moved would fall into no list, so `resolve` could no longer be graded as a whole.
  The field is load-bearing for the evidence, not for a report.
- **Recording which field changed.** A save stores one 64-bit fold over the five
  behaviour fields, so the line can say "water no longer behaves as it did" and
  can never say "water stopped being breakable". Reporting the field is a
  save-format change.
- **Any prompt, dialog or HUD surface.** There is no prompt today and none is
  added.
- **Strict argument parsing.** An unrecognised argument stays ignored, which is
  also why the retired `--load-changed-blocks` needs no handling: it stops
  matching and the load it asked for is now the default.
- **Changing what a content reload does**, and any automatic backup or
  copy-on-load.

## Notes

### The two decisions worth recording

**What the line says.** One line, every changed block, ascending, complete. Not
truncated: `LoadError::Unresolvable`'s own `Display` already prints both lists
complete, so bounding the accepting path would report *less* than the refusal it
replaces. Behaviour only — the reasoning that keeps a retexture out of the refusal
transfers unchanged to a line, and a line after every art edit is noise the one
that matters hides in. Nothing at all when nothing changed, which is `notice.rs`'s
own rule: "composing anything for it would put a line on every player's terminal
on every run".

**Why `--refuse-changed-blocks` earns its keep**, rather than being a flag nobody
passes. A save is rewritten against the current declarations on quit —
`NameTable::record` (`crates/mc-world/src/persistence/table.rs:226-238`) writes
`behaviour_of`/`appearance_of` from the registry the process is running. So *load a
changed save → play → quit* replaces the recorded hashes and the mismatch is gone
from the file. The report is a one-shot. Somebody restoring a backup they are
unsure about wants the world left shut, because opening it and playing on is what
destroys the evidence. This is existing behaviour that no test asserts and no
document records; **it is owed documentation by this spec, not a test**, since
nothing here creates or changes it.

### Where the line is emitted — ruled, not open

`notice::say_entering` sits at `crates/mc-client/src/app/mod.rs:446`, below
`upload_textures` and `upload_scene` — both device operations. `shipped_binary.rs`
is cheap precisely because it needs no device. **Emit this line where the load
completes, before a device opens**, which is what makes S7 reachable without a
display server. It is a statement about the save, already true when `load_world`
returns, and the reason the clearing notice waits for a picture ("you were moved"
needs a world to be moved in) does not transfer to "which blocks changed".

**This is decided, and the asymmetry with `say_entering` is deliberate.** Moving
the line below the uploads for consistency with its sibling would be a regression
in what can be checked, not a tidy-up: it takes S7 — the only scenario that can see
a client which composes the line and never prints it — out of reach of the gate.
Anything proposing that move needs an explicit ruling rather than a refactor.

### S7 is the one scenario this change would be worth least without

S4, S5 and S6 are absence assertions. If the presence-side witnesses all reach the
line by calling its composer directly — which is how
`crates/mc-client/tests/support/printed_refusals.rs` reaches `notice::entering`,
and `launch_acceptance.rs` runs `simulation_to_play` rather than a process — then a
composer returning the right `Option<String>` while the client never calls its
`say_*` sibling leaves every one of them green. That is `testing.md` §2 "Policy is
not wiring", and its measured instance lives in this same mechanism: 191 tests
stayed green against a `RegistryVerdict::refuses` that ignored its argument.
`crates/mc-client/tests/shipped_binary.rs` already runs the built binary and reads
its real streams; its header names this exact failure.

At `low` the displayed failing output is the whole discipline gate, so S7's red
must be shown from a real process before anything is implemented.

### The red this starts from, and the trap in fixing it

Measured on this branch — `cargo nextest run --workspace --all-features
--no-fail-fast`: **1370 run, 1368 passed, 2 failed, 1 skipped.** Both failures are
in `shipped_declarations_and_an_older_save.rs` and both are caused by the owner's
`water.luau` edit. Water is reported `changed`, `dirt`/`grass`/`stone`
`retextured` — the two-hash separation firing correctly on a real content edit for
the first time, rather than on a fixture. The mechanism is right; the expectations
were wrong.

**The second expectation must not simply have its argument flipped.**
`refusal(Acceptance::OnlyUnchangedBlocks) == None` existed to catch a shared
revision byte: share it and every save reports every block as changed. Under the
new default, `refusal(ChangedBlocksToo)` returns `None` for *any* changed list, so
flipping the argument yields a test that passes however badly the behaviour fold
breaks. **S14 is the replacement oracle** — a shared byte gives `changed: [all
four], retextured: []` against its `changed: [base:water]`. S15 asserts through the
line instead, which is a different consumer, not a second witness of the fold.

### S8 and S10 are each other's controls

`acceptance_from` is an `any()` over one string, so the likeliest defect in this
diff is the constant renamed while its value stays `"--load-changed-blocks"` —
that string is in six `.rs` files today. The exact spelling must refuse; one letter
short must not.

### The transport, and one recorded decision that needs rewording

`load_world` (`crates/mc-world/src/persistence/read/world.rs:132-158`) computes the
verdict, asks it for a refusal, and drops it; `LoadedWorld` carries
`{ world, player }`. `docs/technical/architecture.md:1029-1040` records that
widening this was deliberately refused — but for a different purpose, gating the
entry clearing search on the verdict, which would have coupled persistence to
physics. Carrying a report out adds no physics dependency, so the decision is not
reversed on its own terms; the sentence "`LoadedWorld` carries only
`{ world, player }`" simply becomes false and needs rewriting.
`docs/technical/architecture.md:961-962` says `acceptance_from` "does nothing
else", which stays true only if the line is composed elsewhere.

The rail already in the tree for this shape is `Clearing`, which rides out of
`mc-sim` on `Seated`, through `simulation_to_play`, onto `PreparedLaunch`.

### Existing code to touch

| What | Where |
|------|-------|
| The flag constant, the default, the way-out sentence | `crates/mc-client/src/startup.rs` |
| `Acceptance`, `RegistryVerdict`, `refusal`/`refuses`/`judge` docs | `crates/mc-world/src/persistence/table.rs` |
| The verdict stops being dropped | `crates/mc-world/src/persistence/read/world.rs` |
| The line's composer, `notice.rs`'s shape | `crates/mc-client/src/notice.rs` |
| The two reddened expectations | `crates/mc-world/tests/shipped_declarations_and_an_older_save.rs` |
| The owner's `breakable = false` edit | `content/base/blocks/water.luau` |

`Refusal::Indestructible` already exists (`crates/mc-sim/src/world/action/mod.rs:190`),
so S12 needs no engine work — only the declaration and a witness on the shipped
path. Breaking an ordinary block is already covered, so S12's positive control
exists.

A quoted line under `docs/modding/` is compared **line for line** against a real
run by `crates/mc-client/tests/documented_refusals.rs`, which already sources two
of its texts from `mc_client::notice::entering`. So quoting the new line on a
modding page obliges extending that harness's produced set — or the build goes red,
which is the right outcome. `docs/user/gameplay.md` is not scanned.

`entry_clears_whatever_the_load_reported.rs`'s header describes its falsifier as
"a launch made **without** `--load-changed-blocks`". The fixture does not need to
change — an unchanged save with feet inside solid is still the right falsifier —
but that sentence goes stale with the rename.

## Definition of done — the limitation sweep

This change **lifts a stated limitation**, so
`docs/technical/working-in-this-repo.md` §"Lifting a limitation costs one grep per
place it was stated" applies from the start. PRO-947 paid six validation findings
for exactly this omission, every one stale documentation and not one a code defect.
The sweep is definition-of-done, not a follow-up, and closes by re-running the
greps rather than trusting the table below.

Measured 2026-08-21: `grep -rln "load-changed-blocks\|LOAD_CHANGED_BLOCKS"` reports
6 `.rs` files, 5 `docs/` pages and `specs/REGISTRY.md`; `grep -rn` reports 13 prose
lines under `docs/`.

| File | What goes false |
|------|-----------------|
| `docs/user/gameplay.md` | The whole "A save from before this build opens" section and the save-refusal paragraphs |
| `docs/modding/hot-reload.md` | The offline-solidity passage and worked-example step 6, which walks an author through the refusal and the flag |
| `docs/technical/world-format.md` | The three-outcome load decision, the two-constants argument, the "loads without asking" arms, Known limitations |
| `docs/technical/architecture.md` | The `acceptance_from` sentence, the `LoadedWorld` coupling refusal, the way-out-is-not-a-cause passage |
| `docs/technical/testing.md` | The way-out coverage note naming the flag |
| `docs/INDEX.md` | The `hot-reload.md` and `gameplay.md` digest rows, which restate the limitation in their own words — one of the two hiding places that section names |
| `specs/REGISTRY.md` | Three lines naming the flag or the prompt |

No ADR states this limitation — checked across `docs/technical/decisions.md`. That
is the other hiding place, and its absence here is worth recording.

**There is no prompt anywhere in this system and there never was.** The old
behaviour is a refused launch naming a flag. Measured: of 36 grep lines for
prompt-or-ask language, 5 are coincidental and **31 lines in 14 files** describe
this mechanism as prompting or asking. Five are shipped `mc-world` doc comments
arguing *from* a prompt that does not exist. The worst is player-facing and false
about the shipped program today — `docs/user/gameplay.md:81`, "The **game asks**
before opening a save whose blocks *behave* differently" — on a page corrected for
this same class two commits ago. All 31 are inside this sweep.

The three audiences: the **mod author** gets the write-edit-relaunch loop and the
line they will read; the **player** gets that their world opens, what the terminal
says, and that water can no longer be broken but can still be built through; the
**engine reader** gets the new default, the verdict's transport, and the
laundering-on-quit property that `--refuse-changed-blocks` exists for.

## Assumptions

- The committed pre-Luau fixture stays as committed and keeps holding all four of
  `base:dirt`, `base:grass`, `base:stone` and `base:water`. The control for this
  already exists and passes:
  `the_committed_save_really_does_need_all_four_of_the_blocks_the_base_game_ships`.
- `content/base/` ships four blocks; S14 names all four, so a fifth would need it
  reworded rather than silently passing.
- There is no HUD or dialog in this build, so one line on the error stream is the
  whole user interface this decision has.

## Validation — 2026-08-22

**At `low` the green gate is the whole validation.** There is no reviewer
workflow, no `validation-report.md` and no sign-off (`CLAUDE.md`'s rigor table;
the `sdd-validate` skill's low-tier rule, which says to record the reading here).
Rigor `low` was the project owner's explicit ruling on this spec, and nothing
heavier was manufactured for it.

Read on the completion tree, with all documentation consolidated and the registry
and roadmap entries in place:

```
scripts/sdd-gate.ps1        exit 0
11 stages, GATE PASSED
1386 tests run: 1386 passed, 1 skipped
lines 93,63%  regions 92,09%  (10730 lines tracked)
```

Two things about how that reading was taken, because both are rules this repo
paid for. The process table was confirmed quiet first — no `cargo`, `rustc`,
`nextest` or `voxforge` process — since a red gate under concurrency is not
evidence (`docs/technical/testing.md` §"The gate is not safe to run from two
contexts at once"). And the output was redirected to a file with the exit code
taken from the invocation, **never read through a pipe**: a first attempt here
piped the gate into `tee`, which reports `tee`'s status rather than the gate's,
and was killed rather than read (§"Never read the gate's result through a pipe").

The suite now spawns the built client twice and one of those readings must reach
a window, so a window may flash during the run. That is expected and recorded in
`docs/technical/testing.md` rather than left for the next person to diagnose.

**Verdict: PASS.**
