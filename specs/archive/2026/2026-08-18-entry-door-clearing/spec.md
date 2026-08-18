---
id: SPEC-018
title: A player entering a world is never left inside solid rock
status: implemented
rigor: high
branch: feature/PRO-948-entry-door-clearing
issue: PRO-948
created: 2026-08-18
updated: 2026-08-18
completed: 2026-08-18
author: spec-PRO-948
---

# Specification: A player entering a world is never left inside solid rock

## Goal

PRO-918 built the search that moves a trapped player clear, and wired it to one
door: the content-reload swap. Every other way into a world — resuming a save,
launching into a generated one — restores a position and asks nothing. So a
block declared `solid = true` while the game was off puts a resuming player
inside solid rock, with no move and no message. This spec makes being placed
somewhere unblocked a property of **entering a world** rather than of reloading
content, at the one door every entry already passes through.

## Stakeholder capability

**Stakeholder: the player.** After this spec they can quit a game, change what a
block's `solid` field says while the game is not running, relaunch with
`--load-changed-blocks`, and find themselves standing somewhere they can move —
with a line on the terminal saying where they were put and why. Today the same
sequence leaves them inside solid rock, silently, and the only way out is to undo
the edit.

**Secondary stakeholder: the mod author.** They can change a block's solidity
offline without having to reason about where every player was standing when the
server stopped, and `docs/modding/hot-reload.md` states the entry-time rule and
its exact refusal beside the reload-time one they already have.

## User Stories

- As a **player**, I want to resume my save after a block's solidity changed
  while the game was off, so that I keep playing instead of being stuck inside a
  block that used to be water.
- As a **player**, I want to be told where I was put and why, so that appearing
  somewhere I did not save reads as the game doing its job rather than as a bug.
- As a **player**, I want to be told when nothing nearby was clear, so that being
  stuck is an explained state rather than a broken launch.
- As a **mod author**, I want a solidity change to be safe to make offline, so
  that "edit the content root while the game is off" is not a more dangerous
  operation than editing it while the game runs.

## Functional Requirements

### FR-1 — Entry places the player clear of solid blocks

- **FR-1.1**: A player entering a world starts at a position no cell of whose
  box is solid, whenever the entry search finds one.
  - FR-1.1-S1: WHEN a launch resumes a save recording a position whose player box
    covers a solid cell, and the nearest position that is both inside the loaded
    world and clear is one cell sideways, THE SYSTEM SHALL start the player
    horizontally at that cell's centre with their feet on its floor.
  - FR-1.1-S2: WHEN a launch resumes a save recording feet at
    `(12.7, 10.0, 12.4)`, whose player box abuts the solid cell `(13, 10, 12)`
    without overlapping it, THE SYSTEM SHALL start the player at exactly that
    position.
  - FR-1.1-S3: WHEN entry moves a resumed player THE SYSTEM SHALL start them
    facing the yaw and pitch the save records.
  - FR-1.1-S4: WHEN a launch generates a world because there is no save to resume
    THE SYSTEM SHALL start the player at exactly the spawn the generation derives
    — horizontally centred over column `(32, 32)`, feet three blocks above that
    column's own surface height — moving them not at all.
  - FR-1.1-S5: IF every position within 8 blocks of a resumed player covers at
    least one solid cell THEN THE SYSTEM SHALL start the player at the position
    the save records.
  - FR-1.1-S6: IF the only position within 8 blocks of a resumed player that
    covers no solid cell would put part of their box outside the loaded world
    THEN THE SYSTEM SHALL start the player at the position the save records
    rather than at that position.
  - FR-1.1-S7: WHEN entry moves the player THE SYSTEM SHALL report the moved
    position in the first snapshot the simulation publishes, so the first frame
    drawn shows the player where they were put.

- **FR-1.2**: Entry clearing runs on every entry, whatever the save reports about
  its blocks.
  - FR-1.2-S1: WHEN a player quits while standing in `base:water`, `base:water`
    is redeclared `solid = true` while the game is not running, and the save is
    resumed with `--load-changed-blocks`, THE SYSTEM SHALL start the player at a
    position whose player box covers no solid cell.
  - FR-1.2-S2: IF a launch made **without** `--load-changed-blocks` resumes a
    save file whose blocks all match their current declarations and whose
    recorded feet lie inside a cell holding `base:stone` THEN THE SYSTEM SHALL
    start the player at a position whose player box covers no solid cell.
  - FR-1.2-S3: WHEN entry moves a resumed player THE SYSTEM SHALL start them
    within 8 blocks of the position the save records, in a world still holding
    the blocks that save recorded.

- **FR-1.3**: There is exactly one way to seat a player in a world, and it says
  what it did.
  - FR-1.3-S1: WHEN the simulation crate's own sources are examined THE SYSTEM
    SHALL report the verdict *one way seats a player, and it reports its
    clearing*.
  - FR-1.3-S2: IF a source offers a second way to seat a player in a world THEN
    THE SYSTEM SHALL report a verdict naming that source, and never the verdict
    above.

### FR-2 — The player is told what entry did to their position

- **FR-2.1**: Entry's verdict reaches the person playing, in words that describe
  the world they arrived in rather than a change they did not witness.
  - FR-2.1-S1: WHEN entry moves the player to feet at `(12.5, 10, 12.5)` THE
    SYSTEM SHALL compose exactly `mycraft: you would have entered the world
    inside solid blocks, so you were moved to (12.5, 10, 12.5)`.
  - FR-2.1-S2: WHEN a resumed save's player box covers no solid cell THE SYSTEM
    SHALL compose nothing about where the player was placed.
  - FR-2.1-S3: IF nothing within 8 blocks of the entering player is both inside
    the loaded world and clear THEN THE SYSTEM SHALL compose exactly `mycraft:
    you would have entered the world inside solid blocks and nothing within 8
    blocks is clear, so you were left inside them`.
  - FR-2.1-S4: WHEN a content reload moves a player who was already playing THE
    SYSTEM SHALL write `mycraft: the reload made your cell solid, so you were
    moved to (x, y, z)`, and never the entry sentence.
  - FR-2.1-S5: WHEN the client collects a launch whose entry moved the player THE
    SYSTEM SHALL write the composed sentence to its error stream, rather than
    composing a sentence of its own or writing none.
  - FR-2.1-S6: WHILE a run continues past its first drawn frame THE SYSTEM SHALL
    write no further entry sentence, however many frames are drawn.

## Technical Considerations

### The seam: one door, and it is closed by the compiler

Entry clearing is asked at **the single point where a player and a world become a
simulation**, and that point becomes `mc-sim`'s only public way to make one.
`Simulation::new` (`crates/mc-sim/src/simulation.rs:143`) is today the sole
constructor and is `pub`; it is demoted to crate-private and a named admission
function takes its place in the public surface, asking the clearing question and
carrying the answer out with the simulation.

This is binding, and the reasoning is in `requirements.md` §2. In short: every
door — resume, first launch, golden capture, every fixture — already passes
through that constructor, so closing it makes "no player enters a world
unchecked" a property the compiler holds rather than one a scan looks for. A call
added to `simulation_at_launch` instead can be deleted with the whole suite green
and the generated-world door was never covered by it at all.

**The compiler holds it only against callers outside the crate**, which is why
FR-1.3 exists: a second seating path added *inside* `mc-sim` — and MVP 3's join is
exactly that — reopens the hole with everything green. FR-1.3's verdict is total
and enumerated so that a scan which can no longer look is distinguishable from
one that looked and found one door, and FR-1.3-S2 is its positive control.

**What is deliberately not committed to.** MVP 3's networked join adds a player
to an already-running simulation and will not call a constructor, so it inherits
the *rule* — the same search, the same eligibility, the same verdict type — and
not the function. No join API, trait or `Admission` abstraction is introduced
here; the architect names how the rule is factored so a later join calls it
rather than restating it, and FR-1.3 is what makes forgetting visible.

**Known cost.** Demoting the constructor moves roughly twenty integration-test
fixtures across `crates/mc-sim/tests`, `crates/mc-client/tests` and their
`support/` modules onto the new door. They all place clear players, so their
behaviour is unchanged, but the adaptation is a commit whose tree does not
compile until it lands — `standards/global/testing.md` §2's blind window. Clippy
is run directly by whoever authors tests inside it.

### The search is PRO-918's and is not re-specified

`clearing::cleared(feet, world, ground)`
(`crates/mc-sim/src/world/clearing.rs:63`) is reused unchanged. Binding rulings
carried forward from that spec's decision D11:

- A candidate is eligible only if **every cell the player's box would cover is
  known and clear**. Outside the loaded world is unknown, not clear.
- `is_solid` does not change. Eligibility is sited in the clearing search alone.
- `Solidity` is not widened — the world's extent is passed to `cleared`, because
  a trait is defined by all of its implementors and the fixture doubles have no
  extent.
- The ring order `(dy, max(|dx|, |dz|), dz, dx)`, the cell-centre candidates, the
  absence of downward offsets and the 8-block reach all stand.
- Velocity is zeroed on any clearing move. At entry the entering velocity is
  already zero, so this is inherited rather than newly observable.

Its semantics are pinned by five integration tests in
`crates/mc-client/tests/reload_*.rs`. Re-asserting them at entry would re-prove
them through the same code path, which `standards/global/testing.md` §1 calls a
bogus test. **Two things are new wiring rather than PRO-918 semantics**, and both
are scenarios here:

- **The extent argument at the new call site.** `cleared` takes the ground it may
  consider, and the entry caller must pass the played world's own extent.
  FR-1.1-S6 catches an extent that is too large and FR-1.1-S1 catches one that is
  too small — a shrunken extent rejects S1's one-cell-sideways destination. **S1
  is therefore doing duty as S6's positive control**: the two belong in the same
  suite, and S1 must not be weakened to a bare "covers no solid cell".
- **A saved position near the world's edge**, which is an input no reload fixture
  supplies. That is PRO-918's own lesson applied — ask what the shipped caller
  supplies and which shipped path reaches the behaviour.

**The generated door's scenario asserts the derived spawn exactly** (FR-1.1-S4)
rather than "covers no solid cell", because the latter is green before a line is
written: the derived spawn is three blocks above its column's surface, the sea
fills only to a fixed height below every surface, and no shipped content can trap
it. What that scenario is really for is the opposite failure — an entry check
that moves, cell-centres or grounds a player who needed nothing.

### Unconditional, and why the alternative was refused

The search runs at every entry rather than only where the load reported changed
blocks. `RegistryVerdict` (`crates/mc-world/src/persistence/table.rs:52`) is
computed and dropped inside `load_world`
(`crates/mc-world/src/persistence/read/world.rs:145`); `LoadedWorld` carries only
`{ world, player }`. Gating would mean widening `mc-world`'s persistence return so
`mc-sim` can ask persistence whether to ask the physics a question — and it would
buy the two block reads that a clear player's early return costs, since the
2 601-candidate ring is walked only by a player who is genuinely trapped.

FR-1.2-S2 is what makes the decision falsifiable: an implementation that gates on
the registry verdict, or on the acceptance mode, fails it.

### Reporting: composing and writing are two obligations

The two entry sentences are new; the reload's two are untouched.

**The composition must be reachable with no device.** `App`'s existing
`report_clearing` (`crates/mc-client/src/app/reload.rs:96`) sits behind a real
window that nothing in the workspace constructs, which is why the reload's own
two sentences are today asserted by nothing at all. Repeating that shape would
give FR-2's whole stakeholder promise no observable, so the entry sentences are
composed by something a test can call directly (FR-2.1-S1, S2, S3), and
FR-2.1-S4 closes the reload's existing hole as a by-product while guarding
against unifying the two sentences.

**The write is a separate obligation with its own scenario.** FR-2.1-S5 grades
that the client actually writes what was composed — the *policy is not wiring*
case `crates/mc-client/tests/shipped_binary.rs` was written for. A subprocess run
cannot reach it: that test works only because a missing content root refuses
before a device is opened, and a successful launch opens one. So the instrument
is a scan reporting a total, enumerated verdict with a positive control, in the
shape of `reporting_seam.rs` and `one_content_path_to_the_registry.rs`. The
architect names it.

"Once for the launch" needs no dedup field — there is exactly one entry per
process run — but FR-2.1-S6 pins it anyway, because the verdict has to be parked
somewhere between the preparation worker and the collection, and a parked value
read by the frame path repeats every frame.

The client's non-fatal notices are written straight to stderr rather than through
the reporting sink, a gap `docs/technical/architecture.md` already records as a
deferred observation. This spec follows the existing convention rather than
fixing it; routing them through a sink is Out of Scope, because doing it for one
notice alone would create a second mechanism.

### Documentation owed (Key Principle 3)

- **Player** — `docs/user/gameplay.md`: that resuming a save now places you
  somewhere you can move even when a block became solid while the game was off,
  what you read on the terminal when it happens, and what happens when nothing
  nearby is clear.
- **Mod author** — `docs/modding/hot-reload.md`: that a solidity change made
  offline is answered at the next entry exactly as one made live is answered at
  the swap, with the entry wording quoted, and that the same "only ground the
  world actually holds counts as clear" rule applies at entry.
- **Engine reader** — `docs/technical/architecture.md`: the admission seam, why
  it is the constructor rather than the launch path, what the compiler holds
  there and what it does not, why the search is unconditional, and what a future
  change must not break. `docs/technical/testing.md`: the entry-door fixtures,
  the compose/write split and its two instruments, and the mutations recorded
  against them.

## Existing Code to Leverage

| What | Location | Reuse |
|------|----------|-------|
| The clearing search and its verdict | `crates/mc-sim/src/world/clearing.rs` | called unchanged; `Clearing` is the verdict type at entry too |
| The sole simulation constructor | `crates/mc-sim/src/simulation.rs:143` | becomes crate-private behind the admission door |
| How a reload's verdict travels to a person | `Accepted` (`simulation.rs:99`) → `ReloadReport` → `crates/mc-client/src/app/reload.rs:96` | the pattern the entry verdict's path mirrors, with the composition pulled somewhere a test can reach |
| The launch decision | `crates/mc-sim/src/persistence.rs:145` | both arms route through the new door |
| The generated spawn | `crates/mc-sim/src/replay/spawn.rs:98` | routes through the new door |
| What a launch hands the client | `PreparedLaunch` (`crates/mc-client/src/launch.rs:79`) | the carrier the verdict rides to the client on |
| Where a launch is collected | `crates/mc-client/src/app/mod.rs:417` | the one place that writes for a person |
| A seam graded by a total verdict with a positive control | `crates/mc-client/tests/one_content_path_to_the_registry.rs`, `reporting_seam.rs` | the shape FR-1.3 and FR-2.1-S5 are graded in |
| The world's extent | `World::extent()` (`crates/mc-sim/src/world/mod.rs:169`) | already public, already in scope at both launch arms |
| The offline-edit journey's content | `content/base/blocks/water.luau` | the one shipped `solid = false` block, and the sea the replay world fills |

## Out of Scope

Binding.

- **Changing the search.** The reach, the ring order, the cell-centre rule, the
  absence of downward candidates, the eligibility rule and `Clearing`'s variants
  are PRO-918's and are not touched.
- **Changing `is_solid` or widening `Solidity`.** Explicitly refused in D11 and
  refused again here.
- **Making `RegistryVerdict` available at the launch seam**, or any other
  widening of `mc-world`'s persistence return.
- **A save recording a position outside the loaded world.** The search answers
  `Unneeded` there — nothing outside is solid — so such a player is not moved and
  falls. It is reachable only from a hand-authored save, it is not "inside solid
  rock", and it belongs to whichever spec makes the world streamed, alongside the
  general `is_solid`-past-the-footprint observation D11 already deferred.
- **Clearing a player at any moment other than an entry or the existing reload
  swap** — not per tick, not after another player's edit, not on demand.
- **MVP 3's networked join**, and any join API, trait or abstraction built for it.
- **Moving the composition root to `mc-server`** — that is PRO-944.
- **Routing the client's non-fatal notices through a reporting sink.**
- **Any consequence of being left inside solid blocks** — no damage, no
  suffocation, no forced respawn, no refused launch.
- **Snapping a resumed player to the ground**, or deriving their height from the
  loaded world's blocks. A resumed player is still placed from the save; entry
  clearing moves them only when their box is inside something solid.
- **Re-asserting the search's own semantics at entry**, beyond the two new-wiring
  scenarios named in Technical Considerations.

## Dependencies

- PRO-918 (hot reload, SPEC-017) — **done**. Supplies `cleared`, `Clearing` and
  the eligibility ruling this spec reuses.

## Assumptions

- One local player, and exactly one entry per process run. "Once for the launch"
  is therefore a property of there being one entry; FR-2.1-S6 grades that the
  construction actually has it.
- `mc-server` remains a stub for the duration of this spec; the composition root
  is `mc-client`.
- The player box is 0.6 wide and 1.8 tall, as `collide` states it, and a
  cell-centred box lies strictly inside one cell column.

## Open Questions

None.

## Clarifications

### Session 2026-08-18

- Q: What does an entry say, given the reload's sentence blames a reload the
  player did not witness? → A: `you would have entered the world inside solid
  blocks, so you were moved to (x, y, z)`, and for the refusal `… and nothing
  within 8 blocks is clear, so you were left inside them`. It names the state
  found rather than an event witnessed, so one sentence is true of a resume, a
  first launch and a hand-written save alike.
- Q: Where is it sited, given MVP 3's join does not exist yet? → A: at the sole
  simulation constructor, which becomes `mc-sim`'s only public door. The
  commitment is one sentence — there is exactly one place that seats a player in
  a world and it returns what it did — which is true with zero imagined callers.
  No join API is designed here; FR-1.3 is what stops a second door being added
  silently.
- Q: Every entry, or only when the save reported changed blocks? → A: every
  entry. The verdict is not available at the seam without coupling persistence to
  physics, a clear player costs two block reads, and `changed` is not the only
  way to be trapped.
- Q: What is a player who cannot be cleared told at launch? → A: the refusal
  sentence above, and the launch proceeds. A refused launch would take away the
  edit-and-relaunch escape along with the save.
