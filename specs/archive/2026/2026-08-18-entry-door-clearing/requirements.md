# Requirements — PRO-948, entry-door clearing

Gathered 2026-08-18. Sources: Linear PRO-948, the owner ruling quoted in it,
`specs/archive/2026/2026-08-17-hot-reload/architecture.md` (decision D11),
`docs/technical/architecture.md`, and the tree as it stands at `d63d5e5`.

## The hole, verified in the tree rather than taken from the ticket

`clearing::cleared(feet, world, ground)` lives at
`crates/mc-sim/src/world/clearing.rs:63`. It is `pub(crate)`, and its one caller
is `Simulation::clear_the_player` (`crates/mc-sim/src/simulation.rs:243`), which
is itself reached only from `Simulation::adopt` (`:209`) — the content-reload
swap. Nothing else in the workspace calls it.

The resume path does not: `mc_sim::persistence::simulation_at_launch`
(`crates/mc-sim/src/persistence.rs:145`) restores the saved position through
`fn resuming` (`:192`) — position, yaw and pitch verbatim, `velocity: Vec3::ZERO`,
`on_ground: false` — and hands it straight to `Simulation::new`. No collision
question is asked anywhere on that path.

**The concrete journey, entirely within shipped content.** `base:water` is the
one shipped block declared `solid = false`
(`content/base/blocks/water.luau:13`), the generated world fills a sea
(`crates/mc-sim/src/replay/world.rs:32`), so: walk into the water, quit, set
`solid = true` in `water.luau` while the game is off, relaunch with
`--load-changed-blocks`. The load succeeds (a solidity change is classified
`changed`, and `Acceptance::ChangedBlocksToo` admits it), the player is restored
inside solid rock, and nothing moves them or says a word. That is the defect.

## The entry doors that exist today

Explored across `mc-sim`, `mc-client`, `mc-server`.

| Door | Where | Clearing today |
|------|-------|----------------|
| Resume from save | `crates/mc-sim/src/persistence.rs:153` | none |
| First launch, generated world | `crates/mc-sim/src/replay/spawn.rs:98` (`simulation_for`), reached from `persistence.rs:158` | none — the spawn is *derived* from the column's surface height, not checked |
| Content reload swap | `crates/mc-sim/src/simulation.rs:209` | yes, this is PRO-918's |
| ~20 test/harness fixtures | mc-sim and mc-client `tests/` | none |

All four go through exactly one constructor: `Simulation::new`
(`crates/mc-sim/src/simulation.rs:143`), which is `pub`. There is **no** existing
"admit a player to a world" concept anywhere — `admission` in this tree means
*content* admission (whether a registry may replace the current one), and `spawn`
means either a thread or the generated start position. PRO-948 introduces the
concept; it does not reuse one.

`mc-server` is a six-line stub with an empty `[dependencies]`. The composition
root is still `mc-client`. PRO-944 moves it and has not started.

## Discussion findings — the four decisions the ticket left open

### 1. The wording

The reload path prints, from `report_clearing`
(`crates/mc-client/src/app/reload.rs:96`):

```
mycraft: the reload made your cell solid, so you were moved to (x, y, z)
mycraft: the reload made your cell solid and nothing within 8 blocks is clear, so you were left where you were
```

Both sentences describe an event the player *witnessed*. At entry they are false:
the player was not there when the content changed, and on a first launch there was
no change at all. **Decided wording:**

```
mycraft: you would have entered the world inside solid blocks, so you were moved to (x, y, z)
mycraft: you would have entered the world inside solid blocks and nothing within 8 blocks is clear, so you were left inside them
```

Why this shape:

- It describes **the state found**, not an event witnessed, so it is true of a
  resume after an offline edit, of a hand-written save, and of a generated world
  alike. No second sentence per door.
- "would have entered" is what makes the move comprehensible to somebody with no
  prior frame to compare against — the reload's reader saw where they were a
  moment ago and this reader did not, so the sentence has to supply the
  counterfactual the reload's reader supplies themselves.
- The refusal ends "left inside them" rather than the reload's "left where you
  were", because at entry there is no *where you were*.
- The reach is named in the refusal, exactly as the reload's is, so a player
  reads how far was looked at rather than guessing.

Same channel (stderr, from the client), same once-per-event rule. Once is easier
here than at a reload: there is exactly one entry per process run, so the verdict
is produced once and printed where the launch is collected — no dedup field, and
no repetition risk from the frame path.

### 2. The siting — decided, with the alternative recorded

**Decided: entry clearing is sited at the single door through which a player and
a world become a simulation, and that door becomes the only public one.**
`Simulation::new` is demoted to crate-private and a named admission function
takes its place as `mc-sim`'s public entry. The clearing question is asked
inside it and its answer travels out with the simulation.

Why this seam and not the launch path:

- **It is unbypassable by the compiler rather than by a scan.** Every simulation
  holding a player — resume, first launch, every golden capture, every test
  fixture — already passes through `Simulation::new`, and there is no second
  constructor. Closing it makes "no player enters a world unchecked" a property
  the compiler holds. That is the strongest available answer to `testing.md`'s
  *policy is not wiring*: a call bolted onto `simulation_at_launch` can be
  deleted and the generated-world door still misses, with the whole suite green.
- **It covers both shipped doors at once**, including the generated one, whose
  safety today is a derivation (`spawn.rs` computes a height and checks nothing)
  rather than a check.
- **Everything it needs is already in scope at the call.** `World::extent()` is
  public and `World` implements `Solidity`, so both arms of
  `simulation_at_launch` have the feet, the solidity and the extent in the same
  expression. No new plumbing, and in particular no widening of `mc-world`'s
  persistence types.

**On designing for MVP 3's join, which does not exist.** The risk is real and it
is why the commitment is deliberately narrow. What is being committed to now is
one sentence — *there is exactly one place that seats a player in a world, and it
returns what it did* — which is true with zero imagined callers and is worth
having today for the two doors that ship. What is **not** being committed to is a
join's shape: a networked join adds a player to an already-running simulation and
will not call a constructor at all, so it inherits the *rule* (the same search,
the same eligibility, the same verdict type) rather than the *function*. Naming
that seam is the architect's task; inventing an `Admission` trait or a join API
now would be a type with one implementor and one caller, guessed from a protocol
nobody has designed. The three-copies risk the ticket names is a risk about the
**rule**, not about the number of call sites, and one factored rule with two
callers answers it.

Cost, stated because it is not free: demoting `Simulation::new` forces every
integration-test fixture that constructs one (~20 sites across `mc-sim/tests`,
`mc-client/tests` and their `support/` modules) to move to the new door. Those
fixtures place clear players, so the verdict they receive is "unneeded" and
nothing about their behaviour changes — but the adaptation is a commit whose tree
does not compile until it lands, which `testing.md` §2 warns leaves the gate
blind for that window. Whoever authors tests in it runs clippy directly.

**Alternative refused:** calling `cleared` from `simulation_at_launch`'s two arms
directly. Cheaper by exactly the adaptation commit, and it leaves the rule
reachable from a place any future door can forget. It is also the shape the
ticket calls "a third copy by the time MVP 3 lands".

### 3. Unconditional, not gated on the registry verdict

**Decided: the search runs at every entry, unconditionally.**

- **The verdict is not there to gate on.** `RegistryVerdict`
  (`crates/mc-world/src/persistence/table.rs:52`) is computed and dropped inside
  `load_world` — `crates/mc-world/src/persistence/read/world.rs:145` calls
  `resolve(...).refusal(accepting)` and keeps nothing, and `LoadedWorld` (`:68`)
  carries only `{ world, player }`. Gating would mean widening `mc-world`'s
  persistence return so that `mc-sim` can ask persistence whether to ask the
  physics a question. That is the coupling, and it buys nothing below.
- **The cost of not gating is two block reads.** `cleared` opens with
  `if !collide::overlaps_solid(feet, world) { return Unneeded; }` — the 0.6-wide
  box lies inside one cell column, so a clear player costs the two cells their
  1.8-tall box occupies and returns. The 2 601-candidate ring is walked **only**
  by a player who is actually trapped, which is the case the feature exists for.
  There is no launch cost worth coupling two subsystems to avoid.
- **`changed` is not the only way to be trapped.** A hand-edited save, a save
  written by an older build, a generated spawn whose derivation was wrong, and
  (MVP 3) a join into a world somebody has since built rock into all produce a
  trapped player with a clean verdict. Conditioning the invariant on a diagnosis
  makes it an invariant about the diagnosis.

The decision is made falsifiable by FR-1.2-S2: a save whose blocks all match
their declarations, recording a player inside solid rock, must still be cleared.
An implementation that gates on the verdict fails it.

### 4. What a player who cannot be cleared is told at launch

They are told, in the words above, and the launch proceeds. Refusing to start is
worse: a player who cannot be cleared can still quit, edit the content root and
relaunch, and a refused launch takes that away along with their save. Leaving
them where they entered is also what the reload path already does, so the two
doors agree.

Two things the entry message carries that the reload's does not have to:

- **The position they were moved to matters more here**, because there is no
  prior frame. It is already named in the move sentence.
- **The reach is named in the refusal** so "nothing within 8 blocks" reads as a
  bounded search rather than a shrug.

No damage, suffocation or forced respawn follows from being left inside solid
blocks — none of that exists in this game and none of it is in scope.

## Rulings carried forward, not re-opened

From PRO-918's D11, binding here:

- A candidate is eligible only if **every cell the player's box would cover is
  known and clear**. Outside the loaded world is unknown, not clear.
- `is_solid` does not change; eligibility lives in the clearing search alone.
- `Solidity` is not widened; the world's extent is passed to `cleared`.
- The ring order `(dy, max(|dx|,|dz|), dz, dx)` and the 8-block reach stand.

## What is not re-specified, and why

The search's own semantics — cell centres, never downward, the ring order, the
reach, known-and-clear eligibility — are PRO-918's and are proved by five
integration tests in `crates/mc-client/tests/reload_*.rs`. Re-asserting them at
entry would re-prove them **through the same code path**, which `testing.md` §1
calls a bogus test. Two exceptions are kept, both new *wiring* rather than
PRO-918 semantics: the extent argument at the new call site (FR-1.1-S1 and S6,
which are each other's control), and a saved position near the world's edge,
which is an input no reload fixture supplies. That is the shape of the PRO-918
lesson applied here — ask what the shipped caller supplies.

## Scenario audit, 2026-08-18

Ten findings and two contradictions, all accepted. The five material ones:

1. **FR-2 had no observable through a shipped path.** `App::report_clearing`
   sits behind a real window that nothing in the workspace constructs, so the
   *reload's* own two sentences are asserted by nothing today. Repeating that
   shape would have left this spec's whole stakeholder promise ungraded.
   Answered by splitting the obligation: composition is reachable with no device
   (FR-2.1-S1..S3), the write is graded separately (FR-2.1-S5), and FR-2.1-S4
   closes the reload's existing hole as a by-product.
2. **The generated-door scenario could not fail.** "Starts at a position covering
   no solid cell" is true of the derived spawn before a line is written — it is
   three blocks above its column's surface and the sea never reaches it. Rewritten
   to assert the derived spawn exactly and that nobody is moved, which makes it a
   test of the opposite failure: an entry check that moves someone who needed
   nothing.
3. **A wrong implementation passed FR-1.2-S1.** Answering a trapped resume by
   discarding the save and generating a world satisfies every "covers no solid
   cell" assertion while the player loses their world. FR-1.2-S3 added.
4. **The spec's central binding decision had no test.** The compiler holds the
   demoted constructor only against callers *outside* `mc-sim`; a second seating
   path added inside it — MVP 3's join is named as exactly that — reopens the
   hole with everything green. FR-1.3 added, as a total enumerated verdict with a
   positive control.
5. **FR-1.2-S2's fixture would have drifted off the shipped path.** A player
   trapped with unchanged blocks is unreachable in-game, so that fixture is
   necessarily hand-authored — which is precisely when it becomes an in-memory
   `PlayerState` that never touches persistence. Now pinned to a save file read
   through the shipped launch, and it names the flag's *absent* side, which
   nothing else did.

Two contradictions, both fixed:

- **FR-2.1-S2 demanded silence in a case FR-2.1-S3 demanded a sentence for** —
  both were phrased on "entry moved nobody", which is true of the refusal too. S2
  now names the state (the box covers no solid cell) rather than the outcome.
- **"At that cell's centre" is not what the search does.** `centre_of` returns
  `(x + 0.5, y, z + 0.5)` — horizontally centred, feet on the cell *floor*. A test
  author deriving `y + 0.5` from the spec text would have gone red against a
  conforming implementation, and the cheapest green is to edit the search, which
  Out of Scope forbids. Exactly `testing.md` §2's "an over-tight assertion invites
  a real defect", and reachable from the spec text alone.

Two suggestions were considered and deliberately **not** turned into scenarios:
re-asserting the near-corner/far-edge off-map split (both run through the same
extent code from the entry door, so the second is a re-proof), and the
outside-the-loaded-world saved position, which is now stated in Out of Scope
rather than left silent.

## Open questions

None.
