# Phase 4 test-author brief — PRO-904, "One bit answers four questions"

**Spec**: `spec.md` (SPEC-021) · **Architecture**: `architecture.md`, binding ·
**Tasks**: `tasks.md`, section "Phase 4: What can be aimed at, and what a new
player holds" · **Branch**: `feature/PRO-904-solid-split-properties` ·
**Tree this brief was written against**: `cbe49c0`, clean.

Every measurement below was taken on that tree with the command printed beside
it. Nothing here is relayed from a conversation; where a figure disagrees with
`tasks.md` or `architecture.md`, the disagreement is stated rather than
smoothed over.

---

## 0. What you are and what you are not

You are the test author for Phase 4. Rigor is `high`, so:

- **You write the phase's failing tests before any implementation exists, and
  you own every test file for the whole phase.** The implementer never edits a
  test file. If the implementer disputes a failure, it comes to you with a
  verdict request and you answer exactly one of `test-correct` (the
  implementation must change), `test-wrong` (you fix and commit), or
  `scenario-ambiguous` (the user decides).
- **You also own the test-side adaptation** this phase forces: a rename and a
  signature change reach test files, and §7 lists every one of them.
- You do **not** write production code. Not a stub, not a trait definition, not
  a `todo!()`.

Keep your own prose output minimal. What you produce is test files, `test-map.md`
entries, and one report.

---

## 1. What Phase 4 is

Twelve scenarios: **FR-3.1-S1, FR-3.1-S2, FR-3.2-S1, FR-3.2-S2, FR-3.3-S1,
FR-3.3-S2, FR-3.4-S1, FR-3.4-S2, FR-3.5-S1, FR-4.1-S1, FR-4.1-S2, FR-4.1-S3.**

In one sentence: **what a ray stops at stops being "the first solid cell" and
becomes "the first cell whose block is declared `targetable`", the shipped water
becomes aimable, `breakable = false` on water acquires a player-visible
consequence for the first time, and the block a new player holds keeps reading
`solid` — deliberately, and stated as a ruling rather than left to fall out.**

One rule moves with it that `tasks.md` does not name and §10 rules on: **a
placement whose hit cell is itself declared `replaceable` lands in that cell
rather than one step back**, which is what keeps "you can build straight into
water" true once water can be aimed at.

Two things you will meet that are **not** yours and should not distract you: the
documentation this phase owes (T18) is the implementer's, and one paragraph of
it — `docs/modding/blocks-items.md:561-568`, *"until it does, the mesher still
decides what to draw from `solid`, and a trace still decides what you are aiming
at from `solid`"* — has been half false since Phase 2. It is logged and will be
rewritten with the rest. Do not write a test against a documentation sentence.

**Phase 4 depends on Phase 1 only.** It is placed after Phase 3 so that Phase 3's
whole-suite run over `mc-sim` met a tree in which nothing else had changed.

**Unlike Phase 3 it is not gate-atomic.** The gate must be green at phase end and
there is no licensed red window.

**Nothing in this phase may move a golden.** Verified in `tasks.md` and not
re-derived here: the capture path composites no targeting outline, the only
overlay in the client's draw path is the frame-time debug reading, and FR-4.1-S1
keeps the held block `base:dirt`, which is what the HUD golden draws. **If a
golden moves, stop and report it** — that is a finding, not something to absorb.

---

## 2. Read these from disk before you write a line

Not from this brief, and not from anybody's summary.

1. `CLAUDE.md` (repository root) — Key Principles, MyCraft Invariants.
2. `standards/global/testing.md` — §1 (TDD, what counts as RED), §2
   (Falsifiability), §3 (Test Quality).
3. `standards/global/code-quality.md`, `standards/global/git-workflow.md`.
4. `spec.md` — **FR-3 and FR-4 in full**. That is your boundary. No camera, no
   mesh, no save.
5. `architecture.md` — **Decision 4** ("Targetability is a second pre-resolved
   view, written at the single existing write site") and **Decision 11**
   (the one-liners, including FR-4.1's held block).
6. `tasks.md` — the Phase 4 section, T15 to T18.
7. `test-map.md` — the Phase 1 to 3 entries, so your Phase 4 entries match the
   house style: mutations recorded whether or not they bit, invocation quoted
   beside every count, additional coverage each with a line saying what it
   catches.

---

## 3. Verified facts about the tree

### 3.1 The shipped water already declares everything it needs

`content/base/blocks/water.luau` (landed in Phase 3, FR-1.3) declares:

```luau
solid = false, breakable = false, replaceable = true,
drawn = true, occludes = false, targetable = true,
```

`base:dirt`, `base:grass` and `base:stone` state `solid = true` and none of the
three new fields, so each defaults to `true`. **No content file changes in this
phase.**

### 3.2 The trait, and the population that must not move

```
grep -rn "&dyn Solidity" crates/
```

→ **17 occurrences**, and they split three ways:

- **Nine in `src/` that mean collision and do not change**:
  `player/collide.rs:101,125,222,250,259` · `player/physics.rs:74` ·
  `world/clearing.rs:68,98,114`.
- **One in `src/` that moves**: `world/action/trace.rs:56`, `targeted`.
- **Seven in `tests/`, all collision helpers, none of which change**:
  `player_collision.rs:224,232` · `player_ground.rs:211,219` ·
  `player_motion.rs:157` · `player_resolution.rs:170` ·
  `replay_solidity.rs:244`.

`tasks.md` says "nine sites that mean collision" and that is exactly right for
`src/`. **`Solidity` keeps its meaning at every one of them.** Widening
`is_solid` to mean targetable was rejected for the reason `architecture.md`
gives: silently changing what one method means at nine call sites is the shape
of a defect nothing can see.

`Solidity` itself is `crates/mc-sim/src/player/mod.rs:91`:

```rust
pub trait Solidity {
    fn is_solid(&self, at: BlockPos) -> bool;
}
```

`impl Solidity` exists at five places: `replay/solid.rs:202` (`SolidVoxels`),
`world/mod.rs:332` (`World`), and three test fixtures —
`mc-sim/tests/support/chamber.rs:151` (`Chamber`),
`mc-sim/tests/support/solidity.rs:70` (`Ground`),
`mc-client/tests/camera_lens.rs:60` (`WalledFloor`).

### 3.3 The two write sites, verbatim

`crates/mc-sim/src/world/mod.rs`:

- **`write`, line 248**, doc comment at 232: *"**The one place either view is
  written**, and there is no other."* Body:

  ```rust
  let solid = match contents {
      Contents::Empty => false,
      Contents::Holds(block) => self.registry.resolve(block)?.is_solid,
  };
  // … store write …
  self.solid.set(at, solid);
  self.mark_dirty(at);
  ```

- **`adopt`, line 275**, doc comment at 262: *"**The other place either view is
  written**, and there is no third."* Body re-resolves the whole view:
  `let solid = SolidVoxels::resolve(&self.blocks, &registry)?;`

`tasks.md` and `architecture.md` cite these as `:232`/`:262` and `:249`/`:274`
respectively; the doc-comment lines are 232 and 262 and the `fn` lines are 248
and 275 on this tree. The words are as quoted.

**"either" becomes "any"** in both, and **the property those two sentences name
is what FR-3.2 rests on.**

### 3.4 What a ray does today

`crates/mc-sim/src/world/action/trace.rs:56`:

```rust
pub fn targeted(origin: Vec3, direction: Vec3, reach: f32, world: &dyn Solidity) -> Option<Hit>
```

and the loop stops at `if world.is_solid(met.cell) { return Some(met); }`.

`Hit { cell, face: Option<Facing>, distance }`. `face` is the face of `cell` the
ray crossed **to enter it** — so for a ray travelling +x it is `West`.

The one production caller is `crates/mc-sim/src/world/action/mod.rs:167`, inside
`resolve`, which derives the ray from the player through `aim(player)` →
`eye_pose` → `look_direction(yaw, pitch)`:

```rust
fn look_direction(yaw: f32, pitch: f32) -> Vec3 {
    let horizontal = pitch.cos();
    Vec3::new(horizontal * yaw.cos(), pitch.sin(), horizontal * yaw.sin())
}
```

Yaw 0 faces +x, positive pitch looks up, `EYE_HEIGHT = 1.62`, `REACH = 5.0`.

**`mc_sim::action::targeted` is publicly re-exported and has no test caller
today** (`grep -rn "targeted(" crates/` finds only the production caller, the
definition, and a string literal inside `mc-client/tests/seam_boundaries.rs`,
which is a guard fixture rather than a call).

### 3.5 What a placement does with the hit

`placed` (`crates/mc-sim/src/world/action/mod.rs:233`):

```rust
let Some(face) = hit.face else { return EditReport::Refused(Refusal::NoFace) };
let cell = stepped(hit.cell, face);            // one step BACK toward the eye
… registry check … overwritable(cell) … occupies(player.position, cell) …
```

**The placement cell is always one step in front of the hit cell, on the side the
ray came from.** `overwritable` reads `replaceable` off whatever that cell holds.
There is **no** rule that places into the hit cell when the hit cell is itself
replaceable. This matters — see §4.9 and the open ruling in §10.

### 3.6 The held block

`default_held_block` (`crates/mc-sim/src/world/action/mod.rs:341`) walks
registration order and takes `.find(|definition| definition.is_solid)`. **It is
unchanged by this phase.** Its production callers are
`crates/mc-client/src/launch.rs:199` and `crates/mc-sim/src/world/reload.rs:52`;
its existing direct test is `crates/mc-sim/tests/held_block.rs`.

Content is read in file-name order, so a registry built from `content/base` is
`base:dirt`, `base:grass`, `base:stone`, `base:water` — the first block is also
the first solid one, which is why `held_block.rs` builds a registry of its own.

### 3.7 The fuse T16 blows

`crates/mc-sim/tests/shipped_water_is_not_broken_and_is_built_through.rs`. Three
tests. The file header states its own arithmetic and it is correct: feet at
`(8.5, 10.0, 8.5)`, eye at `(8.5, 11.62, 8.5)`, yaw 0, pitch −30°, so the
direction is `(0.866, −0.5, 0)`; the ray *"crosses x = 9 while still above
y = 11, so it enters the cell holding the water first and meets the upward face
of the stone standing at (9, 10, 8) after it."*

- `TARGET = (9, 10, 8)` holds stone; `THE_WATER_CELL = (9, 11, 8)` holds water.
- **`a_break_aimed_through_the_shipped_water_reaches_the_solid_block_behind_it`
  (line 138)** carries the doc comment *"It reddens when water becomes
  targetable, which is the point. Read the header before changing it: the repair
  is a new scenario, not a new expectation."* FR-3.4 is that new scenario.
- **`a_placement_into_the_shipped_water_replaces_it_with_the_block_being_placed`
  (line 109) also reddens, and `tasks.md` does not say so.** See §4.9.

### 3.8 The helper population, re-measured

`tasks.md` says this figure read 21 before Phase 1 and 24 after, and instructs
that it be re-run rather than carried. Re-run on `cbe49c0`:

```
grep -rn "fn registry_declaring\|solid: bool\|is_solid: bool\|&\[(&str, bool)\]" --include=*.rs crates/ | grep -v "/src/"
```

→ **26**. It grew again, because Phase 2 added `Declaration { solid, … }` and
`Declaration::like_solidity` to `crates/mc-world/tests/mesh_common/mod.rs`.
**That is the third different figure the same measurement has produced — 21, 24,
26 — and each time the growth came from the previous phase's own adaptation. Do
not carry 26 forward either.** Re-run it against the tree you are holding.

**Of those 26, exactly one is Phase 4's**: `registry_declaring` at
`crates/mc-sim/tests/support/volume.rs:113`, with **three call sites**
(`held_block.rs:37`, `replay_solidity.rs:263`, `solidity_updates.rs:50`).

**`mesh_common`'s copy is not Phase 4's.** Phase 2's `test-map.md` note says
"Phase 4 needs a `targetable` answer in the same module"; it does not. Nothing
in `mc-world` reads targetability — the mesher does not, and `mesh_common`'s
`registry_of_declarations` already sets `targetable: states.solid`, which is
what every meshing fixture means. **Leave `mesh_common/mod.rs` alone.** It stands
at **490 non-blank lines against the gate's 600** and has no business growing
here.

### 3.9 File ceilings

The gate counts **non-blank** lines: 500 for source, 600 for test files
(`scripts/sdd-gate.ps1:128-129`). Measured on `cbe49c0`:

| File | non-blank | ceiling |
|---|---|---|
| `crates/mc-sim/tests/support/chamber.rs` | 456 | 600 |
| `crates/mc-sim/tests/support/mod.rs` | 242 | 600 |
| `crates/mc-sim/tests/block_targeting.rs` | 215 | 600 |
| `crates/mc-sim/tests/shipped_water_is_not_broken_and_is_built_through.rs` | 191 | 600 |
| `crates/mc-sim/tests/support/volume.rs` | 132 | 600 |
| `crates/mc-sim/tests/support/roots.rs` | 308 | 600 |
| `crates/mc-sim/tests/held_block.rs` | 49 | 600 |
| `crates/mc-sim/src/replay/solid.rs` | 287 | 500 |
| `crates/mc-sim/src/world/mod.rs` | 341 | 500 |
| `crates/mc-sim/src/world/action/mod.rs` | 344 | 500 |

`chamber.rs` is the one with the least headroom of the files you will grow.
**Two files elsewhere in the tree are close to a ceiling and are not yours**:
`crates/mc-world/tests/mesh_common/mod.rs` at 490/600 and
`crates/mc-client/tests/support/oracle.rs` at 525/600. Say so if something you
do approaches one.

### 3.10 Baseline, on this tree

```
cargo nextest run -p mc-sim --no-fail-fast
```
→ `Summary [2.947s] 166 tests run: 166 passed, 0 skipped`

A **bare** `166 tests run` is a complete run. A slashed `N/M tests run` is a
cancelled one and says nothing about the rest. The workspace figure recorded for
`cbe49c0` is **1415**; use it as a free tree-state discriminator.

---

## 4. Scenario by scenario

### Where each family belongs

- **FR-3.1 and FR-3.2** are about *declared* fixtures where `targetable` and
  `solid` disagree. Build them over the **fixture registry** in
  `crates/mc-sim/tests/support/chamber.rs`, driven through `BlockChamber` →
  `World` → `Simulation::advance`, exactly as
  `crates/mc-sim/tests/block_targeting.rs` already does.
- **FR-3.3, FR-3.4 and FR-3.5** are about **the shipped water**, so they must be
  built over `support::content_registry()` — the real content root — as
  `shipped_water_is_not_broken_and_is_built_through.rs` already is. A registry
  assembled in Rust would be the engine answering on content's behalf.
- **FR-4.1** is about the registry alone (S1, S2) and about a reload (S3).

### The observable, and why not the obvious one

**Drive everything through `Simulation::advance` and judge by
`EditReport` plus `support::chamber::differences(&declared, after)`.** That is
the shipped path: `advance` → `resolve` → `aim(player)` → `targeted(eye,
direction, REACH, world)` where `world` is the real `World`.

**Do not write a test that calls `targeted` against a hand-built fixture
implementing the new trait.** `testing.md` §2, "policy is not wiring": a test
that calls the same pure function the adapter calls is agreement between two
copies of one decision, and the adapter can stop calling it while both stay
green. There is no such test today and this phase must not add the first one.

Correspondingly: **no test you write needs to name the new trait at all.** Where
it lives and whether it is re-exported is the implementer's decision; your
fixtures reach it through `World`.

### FR-3.1-S1 — a ray stops at a `targetable = true, solid = false` cell

*WHEN a ray crosses a cell holding a block declaring `targetable = true, solid =
false` THE SYSTEM SHALL report that cell as the hit.*

Fixture: a `BlockChamber` with a floor, a block declared `solid = false,
targetable = true` standing in the eye's row along the ray, and **a differently
named solid block standing further along the same ray**. Break.

Assert `differences(&declared, after)` is exactly the near, non-solid cell
changing — and the far block still holding what it was declared with. The two
blocks carry different names on purpose: an edit that took the cell one step
beyond the hit changes the same *number* of cells as the correct one, and only
the names say which went. That is `block_targeting.rs`'s own rule and it applies
unchanged.

The block must be **breakable** so a break produces a `Changed` report rather
than a refusal.

### FR-3.1-S2 — a ray passes through a `targetable = false, solid = true` cell

*IF a ray crosses a cell holding a block declaring `targetable = false, solid =
true` THEN THE SYSTEM SHALL pass through it and report whatever lies beyond.*

**This is the scenario `tasks.md` names as the phase's shape of RED.** Assert
that the cell reported is the one **beyond** the non-targetable block. A
`targeted` still reading solidity reports the block itself, so the assertion is
red against today's implementation with an *assertion* failure, not a compile
error — provided you build the fixture with a chamber that already compiles.

Watch the player's own box: a `solid = true` block in the eye's row a couple of
blocks away is fine, but do not stand the player inside it.

**Note the fixture constraint no assertion can enforce**: the block beyond must
be a *third* name, distinct from both the pass-through block and the floor, or a
diff cannot say which cell moved.

### FR-3.2-S1 — a placed block becomes aimable without a reload

*WHEN a block declaring `targetable = true, solid = false` is placed into a
previously empty cell THE SYSTEM SHALL report that cell as the hit for a ray
crossing it.*

Two ticks over one simulation: a placement that puts the non-solid targetable
block into a previously empty cell, then a break aimed along a ray that crosses
it. Assert the second edit landed in **that** cell.

**This is one of the two scenarios that must fail against a targetability view
built at load and never re-written.** If it passes against a build-once
implementation, the test is wrong and not the implementation. The way to keep it
honest is that the cell must hold **nothing** at load — a view resolved once at
construction answers `false` there forever.

### FR-3.2-S2 — breaking the hit block moves the hit along the ray

*WHEN the block in a cell that was reported as a hit is broken THE SYSTEM SHALL
report the next targetable cell along the same ray instead.*

Two breaks from the same unmoved player. The first takes the near cell; the
second must take the **next targetable cell along the same ray**, which requires
the near cell's bit to have been cleared by the write.

The strongest form asserts **both** edits in one comparison — the whole diff
after two ticks is exactly two named cells changing — so an implementation that
broke the same cell twice, or that broke nothing the second time, fails on the
enumerated result rather than on a count.

Make the second cell's block a **third** name again.

### FR-3.3-S1 — the shipped water is aimed at, not the block behind it

*WHEN a player aims at a cell holding `base:water` with a solid block behind it
THE SYSTEM SHALL report the water's cell rather than the block behind it.*

Shipped content. The existing water file's geometry is exactly this fixture: the
ray enters `(9, 11, 8)` (water) and would meet `(9, 10, 8)` (stone) after it.

The discriminator is a **break**: under the correct implementation the break is
refused and the stone is untouched; under today's it empties the stone. Assert
the whole `(EditReport, differences)` pair, not an absence — "nothing changed" is
also what a fixture nobody aimed at produces.

**Consequence to state in `test-map.md`:** FR-3.3-S1 and FR-3.4-S1/S2 share an
observable. That is acceptable — they are different claims about the same run —
but say so, so a later reader is not misled into thinking there are three
independent witnesses where there are three assertions over one.

### FR-3.3-S2 — water beyond reach reports no target

*IF the nearest cell holding `base:water` along a player's aim lies beyond their
reach THEN THE SYSTEM SHALL report no target at all.*

`REACH = 5.0`, measured from the **eye** to the point the ray meets the block.
Pair the refusing run with a minimally different accepting run — one field of the
player's state moved, the fixture untouched — and assert both answers at once, as
`block_targeting.rs:142` does with its 5.05/4.95 pair. A refusal alone is
satisfied by a simulation that edits nothing.

**Watch the trap `block_targeting.rs`'s header records**: on a horizontal ray a
feet-measured distance is *longer* than an eye-measured one, so a feet-measuring
implementation refuses more rather than less — only the **accepting** side of the
pair falsifies it. Keep the aimed-at cell in the eye's voxel row and leave the
feet's row empty along that ray.

### FR-3.4-S1 — a break swung at water is refused as indestructible

*WHEN a break is swung at a cell holding `base:water` THE SYSTEM SHALL refuse it
as indestructible and leave the water in the cell.*

`EditReport::Refused(Refusal::Indestructible)` **and** the water still in its
cell. Assert the refusal **by name**: `Refusal::NoTarget` and
`Refusal::Indestructible` both leave the world untouched, so a world comparison
alone cannot tell "the swing was refused because water cannot be broken" from
"the swing found nothing at all", and the second is what an implementation that
forgot to make water targetable produces.

This is the first time in the product's life that `broken` is called on a
non-solid cell.

### FR-3.4-S2 — the block behind the water is untouched

*WHEN a break is swung at a cell holding `base:water` THE SYSTEM SHALL leave the
solid block behind the water untouched.*

The other half of the same run: the full `differences` list is empty *and* the
refusal above is the reported answer. Written as its own test function so that
"the refusal was right but the world moved anyway" is a distinguishable failure.

### FR-3.5-S1 — a placement aimed at water builds through it

*WHEN a placement is aimed at a cell holding `base:water` THE SYSTEM SHALL
replace the water with the block being placed.*

**Ruled, and the ruling is binding — §10.** `placed` gains one rule: **when the
hit cell itself holds a block content declares `replaceable`, the placement
lands in that cell rather than one step back.** The scenario is then satisfiable
and the committed placement test in the water file stays green **verbatim** —
same cell, same `from`, same `to`, reached by a different route.

Three consequences for how you test it, each of them the point rather than an
aside:

- **FR-3.5-S1 is green before the change**, for the wrong reason: today the
  step-back rule happens to land in the water cell because water is not
  targetable. So it joins FR-4.1-S1 and FR-4.1-S3 in §6's "green before the
  change" list, **and it is the one whose non-vacuity cannot be argued** — it
  rests entirely on the mutation in §10's condition 3, which is yours to run.
- **The existing test is the natural home**, and you may keep its assertion
  unchanged. What you may not do is let "it still passes" stand as the evidence;
  a test that survives a mechanism change unchanged is sometimes a test that
  cannot see the mechanism.
- **The `InsidePlayer` guard must still fire** when the hit cell is replaceable
  and the player is standing in it. The new rule changes *which cell* is chosen,
  never whether the box check runs, and a branch that picks a cell and skips a
  guard is the obvious defect here. §10 condition 4 makes that a test you owe.

### 4.9 Why that rule is needed: FR-3.5-S1 and the second fuse

Take the existing fixture, which is the natural one for FR-3.5-S1 and is already
committed.

Ray from `(8.5, 11.62, 8.5)` along `(0.866, −0.5, 0)`. It crosses `x = 9` at
`t = 0.577`, where `y = 11.33`, so it **enters the water cell `(9, 11, 8)`
through that cell's West face**. Today water is not targetable, so the walk
continues and meets the stone at `(9, 10, 8)` through its **Up** face at
`t = 1.24`; `placed` steps back through `Up` and lands in `(9, 11, 8)` — the
water cell — which is replaceable, so the water is replaced. That is the
committed expectation and it passes today.

**After water becomes targetable, the hit is the water cell itself, entered
through `West`.** `placed` steps back through `West` and lands in `(8, 11, 8)`.
That is not the water cell, so nothing replaces the water. Worse, the player's
box at feet `(8.5, 10.0, 8.5)` with `HALF_WIDTH = 0.3` and `HEIGHT = 1.8` covers
`x = 8`, `y ∈ {10, 11}`, `z = 8` — so `(8, 11, 8)` is inside the player and the
placement is refused with `Refusal::InsidePlayer`.

So **`a_placement_into_the_shipped_water_replaces_it_with_the_block_being_placed`
goes red too**, and it is a second fuse nobody planted.

And the generalisation is the problem, not the fixture: **once water is
targetable, no ray geometry can make a placement land in a water cell.** The
placement cell is always the cell the ray occupied immediately before the hit; if
that cell held water, the walk would have stopped there instead. The only other
case is a ray originating inside the water, and that is `hit.face == None` →
`Refusal::NoFace`.

FR-3.5-S1 is therefore unsatisfiable against `placed` as written, and it and
FR-3.3-S1 pull in opposite directions on the same fixture. **That is what §10's
rule resolves**, and it is why the resolution had to be a production rule rather
than a re-shaped fixture: no geometry can fix a cell that is unreachable by
construction.

### FR-4.1-S1 — the shipped content still puts `base:dirt` in hand

*WHEN the shipped content is registered THE SYSTEM SHALL put `base:dirt` in a new
player's hand, unchanged by this split.*

`support::content_registry()` → `default_held_block` → `Some("base:dirt")`.
Content is read in file-name order, so dirt is both the first block and the first
solid one; **that is why this scenario alone cannot falsify the rule**, and it is
not meant to. Its job is to pin the shipped answer through the split, and the
existing `held_block.rs` test is what keeps the rule itself honest.

**This is the scenario that keeps the HUD golden still.** If it ever answers
anything but `base:dirt`, stop.

### FR-4.1-S2 — a registry of only non-solid blocks offers no held block

*IF a registry holds only blocks declaring `solid = false` THEN THE SYSTEM SHALL
offer no held block at all, even where some of them are drawn.*

`default_held_block(&registry)` → `None`.

**"even where some of them are drawn" is the whole point, and it is why this
scenario needs the split to exist.** At least one block in the fixture must
declare `solid = false, drawn = true` (and, to make it sharper, `targetable =
true`) — an implementation that had drifted into reading `drawn` or `targetable`
would answer that block instead of `None`. That fixture cannot be stated by
`registry_declaring(&[(&str, bool)])`, which is why §5 widens it.

### FR-4.1-S3 — a reload re-derives the held block

*WHEN a reload publishes a registry whose first colliding block in registration
order is `base:stone` THE SYSTEM SHALL report `base:stone` as the block the
player holds.*

Use the existing reload machinery in `crates/mc-sim/tests/support/roots.rs` and
the pattern in `crates/mc-sim/tests/reload_admission.rs`:

```rust
let candidate = shipped()?
    .restating(DIRT_FILE,  DIRT_THAT_IS_NOT_SOLID)?
    .restating(GRASS_FILE, GRASS_THAT_IS_NOT_SOLID)?;
let answered = adoption(mc_sim::reload::adopt_at_tick_boundary(&mut simulation, candidate.candidate()?));
assert_eq!(answered, accepted(STONE), "…");
```

`roots.rs` already has `restating`, `shipped`, `adoption`, `accepted`, and
`STONE_THAT_IS_NOT_SOLID` as the shape to copy. You add the two new declaration
constants.

**Restate, do not remove.** `not_declaring(&[DIRT_FILE, GRASS_FILE])` would also
leave stone first — but it would leave stone the *first block* as well as the
first solid one, so a rule reading plain registration order would answer stone
too and the scenario would be vacuous. Restating them as `solid = false` keeps
dirt first in registration order and stone first among the colliding blocks, and
those are two different answers.

Two fixture constraints, neither of which an assertion can enforce:

- The world the simulation is playing must **hold** dirt and grass, or something
  the candidate still declares — otherwise the admission's
  `BlocksTheWorldHolds` check fires first and you are testing that instead.
  `reload_admission.rs`'s `playing(blocks)` shows the shape.
- Put the player well clear of everything (`ABOVE_EVERYTHING` in
  `reload_admission.rs` is `(8.5, 40.0, 8.5)`). Making dirt and grass non-solid
  can otherwise put the reload's clearing search in the picture.

---

## 5. The two helper widenings, and the one that is a trap

### 5.1 `crates/mc-sim/tests/support/volume.rs` — owed, and yours

`registry_declaring(blocks: &[(&str, bool)])` takes one boolean per block and
sets `drawn: is_solid, occludes: is_solid, targetable: is_solid`. It cannot state
a fixture where the four differ, which is what FR-4.1-S2 needs.

Phase 2 left it alone **deliberately** — nothing before this task could state
such a fixture and observe anything, so widening it then would have been the
speculative generalisation `code-quality.md` §1 forbids. It is owed now.

**Widen by addition, exactly as Phase 2 widened `mesh_common`:** keep
`registry_declaring`'s signature and let it delegate, add a `Declaration`-shaped
struct and a `registry_of_declarations`, so the one-answer and four-answer routes
build their `BlockDefinition` in one place and cannot drift. All three existing
call sites then move by zero lines.

`crates/mc-world/tests/mesh_common/mod.rs:255-369` is the worked precedent; its
`Declaration` has three fields, yours needs four.

### 5.2 `crates/mc-sim/tests/support/chamber.rs` — needed for FR-3.1 and FR-3.2

`Declared { name, is_solid, replaceable, breakable, breaks_into }` (line 196) is
turned into a `BlockDefinition` at line 325 with
`drawn: block.is_solid, occludes: block.is_solid, targetable: block.is_solid`.
FR-3.1 and FR-3.2 need fixture blocks where `targetable` and `solid` disagree in
**both** directions, so `Declared` gains a targetability answer and `OVERLAY`
gains blocks for it.

Name them for what they are, in the register the module already uses
(`UNBREAKABLE`, `CRUMBLING`, `UNBUILDABLE`), and give each a doc paragraph saying
which reading it separates — that module documents every fixture block by the
defect it makes visible, and a block added without one is a block a later reader
cannot judge.

### 5.3 The trap: **do not "correct" the chamber's `base:water`**

`BASE_CONTENT` (chamber.rs:240) declares `base:water` through `open()`, which
sets `is_solid: false, replaceable: true, breakable: true` — and therefore
`targetable: false`. Its doc comment says *"The four blocks content ships,
spelled and declared as content declares them."*

**That comment is already inaccurate and has been since before this spec**:
content declares water `breakable = false` and this fixture declares it
`breakable = true`. The fixture registry borrows content's *names*; it is not a
mirror of content's declarations.

Making its water `targetable = true` to "match content" would break
`crates/mc-sim/tests/block_breaking.rs:242`, which builds
`filled_with(WATER).cell(EMPTIED, STONE)` — a chamber whose *background* is
water, used to tell "this cell was emptied" apart from "this cell holds the
background". A targetable background stops every ray in that fixture at the first
cell it crosses.

So: **leave the chamber's water as it is.** If you judge the doc comment worth
repairing, repair the sentence to say what the fixture actually is — **it borrows
content's *names*, not its *declarations*** — and say it in those words, because
the next person to "fix" the divergence will otherwise break a background block
in `block_breaking.rs`. Do not repair the declarations to match the sentence.

### 5.4 `mesh_common` is not yours

See §3.8. Leave it alone and say so in `test-map.md`, since Phase 2's own note
predicted otherwise.

---

## 6. What RED must look like

`testing.md` §1: a behaviour scenario needs an **assertion** failure, not a
compile error. Get there.

The shape of this phase makes that easy for most of it, because the fixtures
compile against today's tree: `BlockChamber`, `Simulation::advance`,
`EditReport` and `differences` all exist. The parts that do **not** compile until
the implementer lands T15 are the ones naming a renamed type or a changed
signature — §7.

- **FR-3.1-S2** must fail as *"the ray reported the `targetable = false, solid =
  true` block itself, and the scenario says it must report the cell beyond."*
- **FR-3.1-S1** fails because a non-solid block is not a hit today, so the break
  changes nothing and the enumerated diff is empty against an expectation of one
  named cell.
- **FR-3.2-S1 and S2 must fail against a targetability view that is built at load
  and never re-written.** A view built once satisfies all of FR-3.1. **If either
  is green against a build-once implementation, the test is wrong, not the
  implementation** — that is the ruling and it comes from `tasks.md`.
- **FR-3.3-S1, FR-3.4-S1, FR-3.4-S2** fail today because the break reaches the
  stone: the reported answer is `Changed` where the scenario wants `Refused`, and
  the diff names a cell where the scenario wants none.
- **FR-4.1-S2** fails today because `registry_of_declarations` does not exist and
  then, once it does, because the assertion is about a fixture nothing could
  previously state — make sure it reaches an assertion.
- **Three scenarios are green before the change: FR-3.5-S1, FR-4.1-S1 and
  FR-4.1-S3.** Say so in `test-map.md` and say why for each. **A scenario that is
  green before the change it is about is not evidence** — record what you did to
  establish each is not vacuous:
  - **FR-4.1-S1** pins a shipped answer through the split; what falsifies the
    *rule* is `held_block.rs`'s own registry, which registers a non-solid block
    first.
  - **FR-4.1-S3** pins a rule the split does not move; its non-vacuity is the
    restate-don't-remove argument in §4, which keeps "first block" and "first
    colliding block" two different answers.
  - **FR-3.5-S1** is green today through the *old* mechanism and must be green
    afterwards through the *new* one. Nothing about it can be argued — it rests
    on §10's condition 3 mutation, and if that mutation does not redden it, the
    test cannot see the mechanism and something else is needed before FR-3.5-S1
    may rest on it.

**Ask of the second bitset: what calls it, and what would go red if the write
site stopped setting it?** The answer must be **FR-3.2-S1 and FR-3.2-S2 and
nothing else in the suite.** If a third test reddens, one of the three is
measuring something other than what it claims; if none reddens, the wiring is
untested. Record the answer either way.

---

## 7. The adaptation this phase forces on test files — yours

T15 renames `SolidVoxels` to `ResolvedVoxels` and changes what `set` writes.
Those reach test files, and the implementation context may not touch them.

**Measured** (`grep -rn "SolidVoxels" crates/`):

| File | What it does | What it needs |
|---|---|---|
| `crates/mc-sim/tests/replay_sky.rs:39,174,393` | imports the type, resolves one, names it in a signature | rename |
| `crates/mc-sim/tests/replay_solidity.rs:36,255,252,261,264` | same, plus `slab_of` returning it | rename |
| `crates/mc-sim/tests/solidity_updates.rs:20,51` | resolves one **and calls `set(at, bool)`** | rename **and** the new `set` shape |
| `crates/mc-sim/tests/support/overlap.rs:9` | doc-comment mention | rename in prose |
| `crates/mc-client/tests/support/reload_trap.rs:18` | doc-comment mention | rename in prose |

`crates/mc-sim/src/world/mod_test.rs` needs **no** change: its
`BlockDefinition` literal already carries all four fields from Phase 1.

**`solidity_updates.rs` is the one to think about rather than mechanically
update.** Its scenario — *"setting one voxel's solidity changes that voxel and no
other"* — is what catches a write that transposed two axes or addressed the wrong
word. With two bitsets there are now two ways for that to be wrong and one more
way for them to disagree, so consider whether the scenario is still a single
witness or wants a second reading over the targetable bit. `testing.md` §1's
"a second witness on a path with only one" applies directly. Whatever you decide,
put it in `test-map.md` under additional coverage with a line saying what it
catches.

**Sequencing.** Anything naming `ResolvedVoxels` cannot compile until T15 lands,
so the adaptation commit and the new tests are two different moments. Say in your
report which tests you left uncompilable and why; `testing.md` §2's note about a
phase opening with an adaptation commit having no compilable tree for the gate to
run on is exactly this window, and **while it is open you run
`cargo clippy --workspace --all-targets --all-features -- -D warnings` directly**,
because a green suite is no evidence about a lint and the gate cannot run.

---

## 8. Coverage worth adding beyond the twelve

The mapping is a floor. Two candidates, both with a stated reason:

- **Targetability across a reload.** FR-3.2's two scenarios cover `World::write`
  and say nothing about `World::adopt`, the *other* place either view is written.
  `adopt` replaces the whole resolved view in one assignment, so the two bits
  cannot be written apart there structurally — which is an argument, not a
  witness. `crates/mc-sim/tests/reload_solidity_views.rs` is the existing
  instrument for exactly this question about solidity and is the file to model.
  If you add one, say in `test-map.md` that it is a second witness on `adopt` and
  that FR-3.2 covers `write` alone.
- **The targetable bit under `set`.** See §7 on `solidity_updates.rs`.

**No bogus tests.** A test that re-proves through the same code path what another
already proves is worse than none, because it reads as evidence.

---

## 9. Working rules

- **Never a spec or scenario ID in a test name, a comment, or any code.** They go
  in commit messages on this branch and in `test-map.md`, nowhere else.
- **Names describe behaviour.** One logical assertion per test.
- **Prefer a derived oracle to a committed number.** No expected quantity copied
  from a run of the code under test.
- **An enumerated verdict beats an absence assertion.** `differences(...)`
  compared whole against a named expectation, never `assert!(changes.is_empty())`.
- **A count is only a count with `--no-fail-fast`.** Quote the invocation beside
  every count. Bare `N tests run` complete, slashed `N/M` cancelled.
- **A green suite is no evidence about a lint.** Run clippy directly, as above.
- **Announce a mutation window with a baseline count before deliberately breaking
  the tree**, say how many failures to expect, revert **by hand** (never
  `git checkout -- <file>`), and confirm with `git diff --exit-code`. Two agents
  hold this working tree.
- **Never `git add -A` or `git add .`.** Explicit paths. `git branch
  --show-current` before every commit. Conventional Commits. Test code and
  implementation code never in one commit. Never commit a gate log.
- **Do not run `scripts/sdd-gate.ps1`** — that is the team lead's. While a gate
  is running the tree is frozen and nothing may be committed; the lead announces
  the start and the end.
- **Push at every task boundary.** Unpushed work is unbacked work.
- **Nothing you write may cite a location inside `specs/active/`.** That folder is
  archived at completion and the citation would dangle. State a rule by its
  failure, not by a path prefix.
- **Do not loosen a threshold, budget or bound to reach green**, and do not
  refresh a stale figure into a decisive-sounding one — if a reason has stopped
  discriminating, say so.
- **A contingency is not a property.** "It does not happen" and "it cannot
  happen" are different sentences.

---

## 10. The ruling on FR-3.5-S1, and one prediction to check

### Ruled: the placement rule changes, the requirement does not

§4.9 shows that once water is targetable, no ray geometry lets a placement land
in a water cell. The team lead's ruling, binding:

> **`placed` gains one rule: when the hit cell itself holds a block content
> declares `replaceable`, the placement lands in that cell rather than one step
> back.**

**Why that and not a re-worded scenario**, recorded because the reasoning
generalises past this phase: re-wording FR-3.5-S1 would be rewording a
requirement because the implementation got harder — *the same act as loosening a
threshold, one level up, and worse, because a threshold at least leaves the
capability intact.* Three things say the capability is meant to survive, none of
them anybody's judgement: FR-3.5's own heading is *"A placement aimed at water
still builds through it"*; `tasks.md` says *"`replaceable` is a separate
declaration and is unchanged, **which is why FR-3.5 still holds**"*; and
`docs/user/gameplay.md` states it to players as a promise. **Out of Scope binds
in both directions** — it forbids building what nobody specified and equally
forbids quietly dropping what somebody did.

**This is not unspecced work.** FR-3.5-S1 is a scenario of this phase, so making
it true *is* the work. What `tasks.md` failed to name is only which file the work
lands in. **Amendment, recorded: T16 gains
`crates/mc-sim/src/world/action/mod.rs`.**

### The four conditions attached to the ruling

Conditions 1 and 2 bind the implementer; conditions 3 and 4 are **yours**.

**1. The rule is stated in terms of the declaration, never the block.** *"When
the hit cell holds a block content declares `replaceable`"* — never *"when the
hit cell holds water"*. A block name in that branch would be a hardcoded content
decision in Rust, which is **invariant 1**, and it is the single easiest
invariant to break while writing something that works. If you review the diff and
see a name there, that is a `test-correct` dispute waiting to happen.

**2. The branch is reachable today only through the split, and the code says so.**
Water is the only `replaceable` block content ships and it is not targetable
before this phase, so the new branch cannot fire on the current tree and cannot
regress existing behaviour. The doc comment carries that reason, because a reader
meeting a branch that never fires will otherwise take it for dead code.

**3. Prove the verbatim survival by mutation — yours to run.** A test that
survives a mechanism change unchanged is sometimes a test that cannot see the
mechanism, and that distinction has cost this project twenty-two defects. So:
**after the split, with the old step-back rule restored by hand,
`a_placement_into_the_shipped_water_replaces_it_with_the_block_being_placed` must
go red.** §4.9 predicts `Refusal::InsidePlayer` — **confirm it, do not reason
it.** If it stays green, the test cannot distinguish the two rules and FR-3.5-S1
needs a different or additional witness before it may rest on that test. Record
the outcome either way, with the invocation.

**4. The `InsidePlayer` guard still fires, and you owe the test that says so.**
The new rule changes which cell is chosen, not whether the box check runs — so a
placement aimed at a replaceable cell the player is standing in must still be
`Refusal::InsidePlayer`. That is the obvious place for the new branch to skip a
guard it should not, it is not covered by any of the twelve scenarios, and it
goes in `test-map.md` under additional coverage with a line saying what it
catches. `crates/mc-sim/tests/placement_overlap.rs` is the existing home for
that question and is the file to model.

### One prediction, worth checking rather than assuming

Making the shipped water targetable changes what a ray meets in **any** fixture
built over the shipped replay world with a sea in it. Surveyed on `cbe49c0`:
every break in `crates/mc-client/tests/edit_geometry.rs` and
`crates/mc-client/tests/saved_changes_need_no_edit.rs` is aimed at the landmark
top (`y = 64`) or at the footprint's corner column (`y = 34`, at sea level and so
not submerged), and every reload fixture in `mc-client` uses small declared
worlds with no water. **The prediction is that no `mc-client` test moves.** It is
a prediction and not a measurement — the implementer measures it when the change
lands, and a surprise there is a finding, not something to absorb.

---

## 11. What you record, and where

In `specs/active/2026-08-22-solid-split-properties/test-map.md`, a `## Phase 4`
section matching the house style of Phases 1 to 3:

- One line per scenario: scenario ID → test name → file.
- **Additional coverage**, each with one line saying what it catches.
- **The RED output, with the invocation beside every count**, and the failing
  test named — a run reporting a named test as FAILED cannot have come from the
  tree where that test passes, which is the provenance that cannot be taken at
  the wrong moment.
- **Every scenario that was green before the change** — FR-3.5-S1, FR-4.1-S1,
  FR-4.1-S3 — with what you did to establish each is not vacuous (§6).
- **Mutations run and what each proved, including the ones that did not bite.**
  Two are mandatory:
  - **The wiring mutation.** Delete the targetable write in `World::write` and
    record which tests redden. The answer must be **FR-3.2-S1 and FR-3.2-S2 and
    nothing else** — a third reddening test means one of the three measures
    something other than what it claims; none reddening means the wiring is
    untested.
  - **The placement-rule mutation (§10, condition 3).** Restore the old
    step-back rule by hand after the split and confirm
    `a_placement_into_the_shipped_water_replaces_it_with_the_block_being_placed`
    goes red, and with which refusal. This is the whole of FR-3.5-S1's evidence.
- **The `InsidePlayer` test owed by §10 condition 4**, under additional coverage,
  with the line saying what it catches.
- The re-measured helper population (§3.8) with its command, the note that this
  is the **third** different figure the same measurement has produced, and the
  note that `mesh_common` was left alone and why — Phase 2's `test-map.md`
  predicted otherwise and was wrong.
- The `chamber.rs` water finding (§5.3), so nobody "corrects" it later, stated in
  those words: **the fixture registry borrows content's *names*, not its
  *declarations*.**

---

## 12. Reporting and arbitration

Report via **SendMessage to: main**.

Never end a turn silently — silence is indistinguishable from still working.
`[DONE]` only when nothing of yours is running and the tree holds none of your
uncommitted work; otherwise one line of prose.

Every claim carries the command that produced it.

If you hit a decision you cannot resolve from the spec, the standards, or the
repo, do not guess and do not fail the phase: send the question with the options
and your recommendation, and wait. If you depart from an instruction, say so
explicitly rather than presenting the result.

During implementation, disputes arrive from the implementer with a verdict
request. Answer exactly one of `test-correct`, `test-wrong`,
`scenario-ambiguous`, judged against the spec scenario and nothing else.
