---
id: SPEC-022
title: You can swim in water — the medium properties and one resistance field
status: implemented
work-type: feature
rigor: high
rigor-reason: >
  This adds a field to the published block-declaration surface mod authors
  write, and it moves a save-format fold revision. Both are things a later spec
  must not break. It is also the first numeric field a declaration may state, so
  it decides the numeric-validation vocabulary every later number inherits.
branch: feature/PRO-957-medium-properties
issue: PRO-957
created: 2026-08-24
updated: 2026-08-25
approved: 2026-08-24
completed: 2026-08-25
author: Sebastian Grunow
---

# Specification: You can swim in water

## Goal

Water is visible, aimable and unbreakable, and a player who walks into it falls
through it to the lakebed and stays there — a hole in the world with a surface
painted on. Give a block declaration the two properties that make a volume a
*medium* rather than an absence: `swimmable`, which says a player can hold
itself up in it, and `move_resistance`, which says how much the volume slows
movement through it. Water declares both, and a player can swim in the sea.

## Stakeholder capability delivered

Named, per Key Principle 7:

- **Player** — you can swim. Walk off the shore into the sea and you sink
  slowly instead of dropping, hold jump and you rise to the surface and float
  there, and swimming carries you more slowly than walking on land does.
- **Mod author** — you can declare a medium. `swimmable = true` makes a block
  one a player can hold itself up in; `move_resistance = 4.0` makes any block's
  volume slow what moves through it, whether or not it is swimmable. Both are
  fields you write in a `*.luau` file, both have a refusal that names the field
  and says what is wrong, and `move_resistance` is the first number a
  declaration may state.

## Documentation deliverables

Per Key Principle 4, owed as part of this spec's definition of done and not as a
follow-up. The engine and mod-author pages grow the two new fields in the usual
way; the **player** page has to be *repaired*, and that repair is the one nobody
would find later.

**`docs/user/gameplay.md` becomes false the moment `BEHAVIOUR_REVISION` moves to
3**, and it is a page PRO-904 rewrote and landed on `main`. Three passages:

- `:103` heads the section *"A save from before this build opens, and is told
  about once"*.
- `:117` *"The behaviour record moved with this build"* — "this build" now names
  two different builds, and a reader cannot tell which.
- `:124` **"It happens once."** *"Quit normally and the save is rewritten under
  the new record; the next launch says nothing."* This is the passage that
  breaks: a player who saved after PRO-904 and quit normally **is told again**,
  because this is the second consecutive behaviour-byte move. The page currently
  promises them silence.
- `:268` *"The game has twice changed how it records what a block is"* — three
  now, and this passage already anticipates the shape with *"before one of these
  builds"*, so it is the one that needs the least surgery.

The repair states that the behaviour record has moved more than once, that
crossing each move costs one report, and that "once" is once *per move* rather
than once ever. The person who knows why the byte moved twice is whoever writes
this spec, now; a reader reconstructing it from `format.rs` an increment later
gets the mechanism and not the promise that was broken.

## The revision rule, and the re-shoot that follows from it

**The deliverable is the corrected rule. The re-shoot is its consequence.**

`SCENE_REVISION` (`crates/mc-render/src/capture.rs:32`) is documented as:

> *"Bumped whenever a change to the **mesh contract** invalidates every
> committed frame. `crates/mc-sim`'s scene contract is the tripwire that fails
> first and names this constant as the remedy."*

**That sentence is now false, and this spec is what falsifies it.** The mesh
contract is untouched — the world is unchanged, so `SCENE_QUAD_COUNT`,
`total_face_area` and `area_by_block` all hold — and yet two committed frames
are invalidated. So a golden can now be invalidated by something the revision
does not name and **the tripwire cannot see**: `scene_contract.rs` compares quad
count and per-block area, every one of them unchanged, so it will not fire. The
first symptom anybody gets is two image comparisons failing, with none of the
guidance PRO-904 wrote into that failure message.

The sentence used to be true, and it is worth recording why, because nobody
wrote it carelessly: the camera was derived from the spawn and the spawn from
the world, so the trajectory *was* a function of the mesh contract. This spec
adds a third input — **content-declared physics** — and severs that chain. The
doc is stale, not permissive.

The deliverables:

- **Rewrite the constant's doc** to state what it actually covers: any change
  that makes a same-named capture incomparable — the mesh contract, *or* the
  declared camera path, which now includes the physics the script runs under.
  Say what the tripwire can and cannot see, so the next reader is not told a
  guard exists that does not.
- **Bump `SCENE_REVISION` to `r3` and re-shoot all four golden directories**,
  including `player-walk-t000-r2` and `player-walk-hud-t000-r2`, whose images
  are byte-identical under either physics. The revision's job is the single
  promise *"every image under this name describes one declared observation"*.
  Re-shooting only the two that move leaves `player-walk-t059-r2` meaning one
  thing before this spec and another after — a revision silently redefined,
  permanently, in a repository that archives specs and expects them to be
  re-readable. The asymmetry decides it rather than the principle: bumping costs
  two captures that reproduce identical images, not bumping costs a name that
  quietly means two things forever.
- **Update what is stated against the moved poses**: `JUDGED_TICKS`
  (`replay_oracle.rs`), `SAMPLED_TICKS` (`replay_determinism.rs`),
  `the_sea_the_camera_sees_is_the_water_layer.rs`, and the second and third of
  PRO-904's recorded water sample counts `56 / 200 / 111`, which are stated
  against poses that move. Re-derive them; never copy them from a run.

## User Stories

- As a **player**, I want to swim in the sea rather than sink through it, so
  that water is somewhere I can go rather than a place I fall into.
- As a **player**, I want moving through water to feel different from walking on
  land, so that the sea is a place with its own rules.
- As a **mod author**, I want to declare that my block is something a player can
  swim in, so that a pool of my own liquid behaves like water without the engine
  knowing its name.
- As a **mod author**, I want to declare how much my block slows movement
  through it, so that I can make a thicket or a drift without needing it to be
  swimmable too.
- As a **mod author**, I want a number I got wrong to be refused with the field
  named, so that a typo costs me a refusal I can read rather than a block that
  quietly behaves like air.

## Functional Requirements

**44 scenarios, presented at 44 and confirmed** rather than trimmed to the ~40
threshold. The budget is a confirmation gate against a spec too large to hold
whole, and the history behind it is PRO-904's own: 82 scenarios in one issue hid
eighteen wrong-reason passes. 44 across 11 FR groups is half that, and the split
that produced PRO-957 already did the work the gate asks for. Two of the four
cuts drafted for this decision would have made the spec weaker — one removes a
second entry point onto a tested path that `standards/global/testing.md` §2
argues for keeping, the other trades a mechanical check for a promise about a
later phase, which this project has shipped a known hole for twice.

Every fixture stating a `move_resistance` states `solid = false` alongside it
unless the scenario says otherwise: a solid fixture is never overlapped, because
collision stops the walk first, so a test over one would measure collision and
report a clean pass.

### FR-1 — `swimmable` on a declaration

- **FR-1.1**: A declaration may state `swimmable` as a boolean, and its absence
  means `false` for every declaration, whatever else that declaration says.
  - FR-1.1-S1: WHEN a declaration states `swimmable = true` THE SYSTEM SHALL
    register a block that, while the player's box overlaps it, honours a jump
    asked for off the ground by ending that tick with the player higher than it
    began it.
  - FR-1.1-S2: WHEN a declaration states no `swimmable` THE SYSTEM SHALL
    register a block that does not honour a jump asked for off the ground, for a
    declaration stating `solid = true` and for one stating `solid = false`
    alike.
  - FR-1.1-S3: IF a declaration states `swimmable = 1` THEN THE SYSTEM SHALL
    refuse that declaration, naming `swimmable`, saying it must be `true or
    false`, and saying it is `a number`.

### FR-2 — `move_resistance` on a declaration

- **FR-2.1**: A declaration may state `move_resistance` as a finite number no
  less than zero, written as a Luau integer or as a Luau number, and its absence
  means `0.0`.
  - FR-2.1-S1: WHEN a declaration states `move_resistance = 4` THE SYSTEM SHALL
    register a resistance of `4.0`.
  - FR-2.1-S2: WHEN a declaration states `move_resistance = 4.5` THE SYSTEM
    SHALL register a resistance of `4.5`.
  - FR-2.1-S3: WHEN a declaration states no `move_resistance` THE SYSTEM SHALL
    register a resistance of `0.0`, whatever that declaration says about
    `solid` or `swimmable`.
  - FR-2.1-S4: WHEN a declaration states `move_resistance = 0` or
    `move_resistance = 1e30` THE SYSTEM SHALL register that exact value rather
    than refusing it or clamping it.

- **FR-2.2**: A `move_resistance` the engine cannot use is refused, never
  clamped, coerced or parsed.
  - FR-2.2-S1: IF a declaration states `move_resistance = -1` THEN THE SYSTEM
    SHALL refuse that declaration, naming `move_resistance` and stating that a
    resistance may not be less than zero.
  - FR-2.2-S2: IF a declaration states `move_resistance = 0/0` or
    `move_resistance = 1/0` THEN THE SYSTEM SHALL refuse that declaration,
    naming `move_resistance` and stating that a resistance must be a finite
    number.
  - FR-2.2-S3: IF a declaration states `move_resistance = true` or
    `move_resistance = "1.0"` THEN THE SYSTEM SHALL refuse that declaration,
    naming `move_resistance`, saying it must be a number, and naming the kind
    it found — `a boolean` and `a string` respectively.

### FR-3 — What a declaration may state, as an author reads it

- **FR-3.1**: The recognised-field list a refusal quotes back holds both new
  names, and the modding documentation says the same thing the engine does.
  - FR-3.1-S1: WHEN a declaration states a field the loader has no meaning for
    THE SYSTEM SHALL name that field and quote back all eleven recognised
    names — `name`, `texture`, `solid`, `replaceable`, `breakable`,
    `breaks_into`, `drawn`, `occludes`, `targetable`, `swimmable`,
    `move_resistance` — read whole and in order out of the refusal rather than
    tested for membership one at a time.
  - FR-3.1-S2: WHERE a page under `docs/modding/` quotes that refusal THE
    SYSTEM SHALL produce that page's text exactly, line for line, from a real
    run.
  - FR-3.1-S3: WHERE a page under `docs/modding/` lists what a declaration may
    state THE SYSTEM SHALL find `swimmable` and `move_resistance` named there,
    each with the value its absence means.

### FR-4 — A volume that resists movement

- **FR-4.1**: While the player's box overlaps a block's volume, that block's
  `move_resistance` divides the velocity one tick uses and carries forward, as
  `1 / (1 + move_resistance)`, on every axis alike.
  - FR-4.1-S1: WHILE the player's box overlaps a block declaring
    `move_resistance = 1.0` THE SYSTEM SHALL carry a full-deflection walk half
    as far in one tick as the same walk carries the player through air.
  - FR-4.1-S2: WHILE the player's box overlaps only blocks declaring
    `move_resistance = 0.0` THE SYSTEM SHALL carry a full-deflection walk
    exactly as far in one tick as it does through air.
  - FR-4.1-S3: WHILE the player's box overlaps a block declaring
    `move_resistance = 1.0` THE SYSTEM SHALL carry an unpowered fall half as
    far in one tick as the same fall carries the player through air.
  - FR-4.1-S4: WHILE the player's box overlaps a block declaring
    `move_resistance = 1.0`, WHEN ten ticks of unpowered fall have passed THE
    SYSTEM SHALL report a downward speed strictly less than the speed ten ticks
    of the same fall through air report.

- **FR-4.2**: A block resists what moves through its volume, and nothing else.
  - FR-4.2-S1: WHILE the player stands on a block declaring `solid = true` and
    `move_resistance = 3.0`, its box overlapping only the empty cells above
    that block, THE SYSTEM SHALL carry a full-deflection walk exactly as far in
    one tick as the same walk carries it over a block declaring
    `move_resistance = 0.0`.
  - FR-4.2-S2: WHILE the player's box overlaps a block declaring
    `move_resistance = 1e30`, WHEN a tick's intent asks for a full-deflection
    walk THE SYSTEM SHALL end that tick at a finite position no further back
    along the walk's direction than it began.

- **FR-4.3**: Where the box overlaps more than one cell, the greatest resistance
  among them decides, and a cell holding no block contributes nothing.
  - FR-4.3-S1: WHILE the player's box overlaps one block declaring
    `move_resistance = 1.0` and one declaring `move_resistance = 3.0` THE
    SYSTEM SHALL carry a full-deflection walk a quarter as far in one tick as
    it does through air.
  - FR-4.3-S2: WHILE the player's box overlaps one cell holding no block and
    one block declaring `move_resistance = 3.0` THE SYSTEM SHALL carry a
    full-deflection walk a quarter as far in one tick as it does through air.
  - FR-4.3-S3: WHILE the player's box lies wholly outside the world's volume,
    its feet at `y = 300.0` over a world 256 blocks tall, THE SYSTEM SHALL
    carry a full-deflection walk exactly as far in one tick as it carries a
    player whose box overlaps only empty cells inside the volume.

### FR-5 — A medium a player can hold itself up in

- **FR-5.1**: While the player's box overlaps a swimmable block, a jump request
  is honoured on every tick regardless of ground contact; otherwise it is
  honoured only from the ground, exactly as it is today. Being swimmable resists
  nothing by itself.
  - FR-5.1-S1: WHILE the player's box overlaps a swimmable block and the player
    is not on the ground, WHEN a tick's intent asks to jump THE SYSTEM SHALL
    end that tick with the player higher than it began it.
  - FR-5.1-S2: WHILE the player's box overlaps a swimmable block and the player
    is not on the ground, WHEN a tick's intent asks for no jump THE SYSTEM
    SHALL end that tick with the player lower than it began it.
  - FR-5.1-S3: WHILE the player's box overlaps no swimmable block and the
    player is not on the ground, WHEN a tick's intent asks to jump THE SYSTEM
    SHALL end that tick with the player no higher than an identical tick whose
    intent asked for no jump.
  - FR-5.1-S4: WHILE the player's box overlaps a block declaring
    `move_resistance = 3.0` and `swimmable = false`, and the player is not on
    the ground, WHEN a tick's intent asks to jump THE SYSTEM SHALL end that
    tick with the player lower than it began it.
  - FR-5.1-S5: WHILE the player's box overlaps a block declaring
    `swimmable = true` and no `move_resistance` THE SYSTEM SHALL carry a
    full-deflection walk exactly as far in one tick as it does through air.

### FR-6 — The shipped sea

- **FR-6.1**: A player in the sea the shipped world generates can swim in it.
  - FR-6.1-S1: THE SYSTEM SHALL ship `base:water` declaring `swimmable = true`
    and a `move_resistance` greater than zero, and `base:dirt`, `base:grass` and
    `base:stone` declaring neither field and so registering `swimmable = false`
    and a resistance of `0.0`.
  - FR-6.1-S2: WHEN a player standing on the lakebed of the **deepest** column
    of the shipped sea holds jump THE SYSTEM SHALL raise its feet to `y ≥ 34.0`
    within 120 ticks.
  - FR-6.1-S3: WHILE a player on that column holds jump for 600 ticks THE
    SYSTEM SHALL end **every** tick of that hold — including the ticks its box
    spends clear of the water — with its feet below
    `35.0 + v / 60 + v² / 60` blocks, where
    `v = (JUMP_SPEED − GRAVITY × TICK_DURATION) / (1 + move_resistance)`.
  - FR-6.1-S4: WHEN a player floating at the surface of that column stops
    asking to jump THE SYSTEM SHALL lower its feet back onto the lakebed within
    `360 × move_resistance` ticks.
  - FR-6.1-S5: WHEN a player falls into the shipped sea from `y = 44` THE
    SYSTEM SHALL bring it to rest on the lakebed on a later tick than the same
    fall through air comes to rest on the same lakebed.
  - FR-6.1-S6: WHILE a player is submerged in the shipped sea, WHEN a tick's
    intent asks for a full-deflection walk THE SYSTEM SHALL carry it less far in
    that tick than the same walk carries a player standing on the shore at
    column `(63, 35)`.

### FR-7 — What a save records

- **FR-7.1**: Both new fields are folded into a block's declared *behaviour*,
  never its appearance, and the behaviour list moves to revision 3.
  - FR-7.1-S1: THE SYSTEM SHALL fold a block's declared behaviour over a byte
    sequence stated whole by hand, beginning with the byte `3` and ending with
    the bytes of `swimmable` and then `move_resistance`.
  - FR-7.1-S2: WHEN two definitions differ only in `swimmable`, or only in
    `move_resistance`, THE SYSTEM SHALL record different behaviour hashes for
    them.
  - FR-7.1-S3: WHEN two definitions state the same name, the same textures and
    the same value for every declared field THE SYSTEM SHALL record the same
    behaviour hash for them.
  - FR-7.1-S4: WHEN two definitions differ only in `swimmable`, or only in
    `move_resistance`, THE SYSTEM SHALL record the *same* appearance hash for
    them.
  - FR-7.1-S5: WHEN a world saved under behaviour revision 2 is loaded while
    changed blocks are accepted THE SYSTEM SHALL report every block it holds as
    behaving differently and load the world anyway.
  - FR-7.1-S6: IF a world saved under behaviour revision 2 is loaded while only
    unchanged blocks are accepted THEN THE SYSTEM SHALL refuse the load and
    name every block whose behaviour changed.
  - FR-7.1-S7: WHEN a world saved under appearance revision 3 is loaded THE
    SYSTEM SHALL judge no block it holds to have changed appearance, while
    judging every one of them to have changed behaviour.

### FR-8 — Editing a medium while the game runs

- **FR-8.1**: `swimmable` and `move_resistance` are hot-reloadable, take effect
  on the next tick, and re-mesh nothing.
  - FR-8.1-S1: WHEN a reload changes only `swimmable` or only
    `move_resistance` THE SYSTEM SHALL accept it and apply the new value to the
    next tick's movement.
  - FR-8.1-S2: WHEN a reload changes only `swimmable` or only
    `move_resistance` THE SYSTEM SHALL mark no section for re-meshing.
  - FR-8.1-S3: WHEN a reload removes a `swimmable = true` a block was serving
    THE SYSTEM SHALL apply `swimmable = false` from the next tick, so a jump
    asked for off the ground while overlapping that block no longer raises the
    player.
  - FR-8.1-S4: IF a reload's declaration states a `move_resistance` that would
    be refused THEN THE SYSTEM SHALL refuse the reload whole, name the field,
    and go on serving the content it was already serving.

## Architecture Delta

**Not `none`.** Three binding decisions:

1. **A per-voxel *value* on the tick path, within a stated budget.**
   `ResolvedVoxels` (`crates/mc-sim/src/replay/resolved.rs`) exists so that
   every tick query is a bounds test and a bit test, with no name to resolve and
   no failure to swallow. `swimmable` is a bit and fits that unchanged.
   `move_resistance` is a number, and one `f32` per voxel over the shipped
   1 048 576-voxel world is **4 MiB** against the 128 KiB a bit view costs —
   32×, and paid per world. **Everything a tick needs to answer both medium
   questions per voxel must fit in 256 KiB for that world**, twice what one
   existing bit view costs. That bound is what rules out the dense array and
   makes this a design decision rather than a field addition; the architect
   derives the mechanism and measures against it.
2. **A third narrow trait beside `Solidity` and `Targetable`.** The two existing
   traits are deliberately separate so that a collision site cannot ask an
   aiming question by accident. A medium is a third question, and whether it is
   one trait answering both medium properties or two is a boundary decision of
   the same kind.
3. **The behaviour fold gains two fields and moves to revision 3.** Derived in
   `requirements.md` §5 rather than inherited. This extends an existing
   structure rather than introducing one, and is recorded here because it is a
   binding public contract: every save in existence is re-read against it.

`mc-proto` and the wire format are **unchanged**. Movement is resolved
server-side from an intent (invariant 4), so nothing about a medium crosses the
network.

## Technical Considerations

- **The divisor acts on the velocity, not only on the displacement.** Both
  readings agree on the first tick, which is all a single-tick scenario can see,
  and they diverge without bound after it — FR-4.1-S4 is the scenario that
  distinguishes them. Dividing only the displacement leaves the stored velocity
  accumulating as if in air, so a deep fall reaches the air terminal speed while
  crawling downward and then leaps the instant the box clears the water. Dividing
  the velocity gives the medium a finite terminal speed of `GRAVITY ·
  TICK_DURATION / move_resistance` and no exit discontinuity.
- **`move_resistance` is a divisor, not a subtraction and not a multiplier in
  `[0, 1]`.** `1 / (1 + r)` is total over every value the loader admits: `0.0`
  is exactly "unaffected", which is what every declaration written before this
  field means by its silence, and no admissible value can reverse a movement or
  produce a NaN. A retained-fraction field bounded at 1 would put "stops you
  dead" inside the range and make the boundary a cliff; a subtraction would let
  a large value push a walk backwards.
- **The scale is unbounded above on purpose.** `move_resistance = 1e30` is a
  block that is effectively unwalkable, and it is still a finite number with a
  well-defined answer. Refusing it would need a ceiling nobody can derive.
  FR-2.1-S4 and FR-4.2-S2 are what hold that open.
- **"Unbounded above" is a statement about policy, and the retained width still
  imposes a ceiling.** No *rule* here decides that some resistance is too much to
  mean — that is what the bullet above holds open, and `1e30` registers. But the
  engine keeps the number as the `f32` the tick divides by, and a declaration
  past that width **is an infinity by the time it is held**. Admitting one would
  register neither the declared value FR-2.1-S4 promises nor the refusal FR-2.2
  promises, which is exactly the silent coercion FR-2.2 forbids — so the
  finiteness check is asked **after** the narrowing rather than before it, and
  the ceiling is the largest finite `f32`. Measured against the shipped loader:
  `1e30` and `3.4e38` register; `3.5e38` and `1e40` are refused, naming
  `move_resistance` and saying it must be a finite number. This is a bound a mod
  author meets, so `docs/modding/blocks-items.md` states it in the field table
  rather than leaving "unbounded" to be discovered as false. Covered as
  additional coverage on FR-2.2-S2's test-map line rather than as its own
  scenario.
- **FR-6.1's fixture is the deepest sea column, and the depth is load-bearing.**
  The sea spans 131 columns of differing depth — the shallowest is one voxel,
  the deepest two — and "the lakebed of the shipped sea" does not say which. The
  deepest is the worst case for both the rise and the sink, so a shallower
  column would pass a resistance the sea's own worst case refuses. Measured: on
  the shallowest column the admissible band is roughly `[2.1, 5]` and on the
  deepest roughly `[2.1, 2.5]`, so the choice decides whether a value passes.

- **FR-6.1-S4's budget scales with the declared resistance, and a fixed one is
  not equally sharp across the band.** A fixed `600` was tried and rejected for
  a reason that is easy to miss: at `r ≈ 0.15` it is roughly **36× slack** — an
  assertion that has stopped asking its question — and at `r = 2.5` it is
  **1.003×**, brittle enough to flip on arithmetic noise. The same number is
  useless at one end and a tripwire at the other.

  Sinking approaches a terminal `GRAVITY · TICK_DURATION / r`, so a fall of
  `depth` blocks takes about `120 · depth · r` ticks. Measured against the
  simulation at five values of `r`, the real sink sits at **0.979 to 0.997** of
  that estimate — just under it at every point, so the closed form is a tight
  upper bound rather than a guess. The scenario states **1.5×** it:
  `360 × move_resistance` ticks for the depth-2 fixture, **a constant 50%
  headroom everywhere in the band**, which is what an assertion should hold.

  The tolerance is derived from measurement in both directions — above the
  measured worst case, below the smallest failure worth catching — rather than
  widened until green.

- **Both scenarios now scale with `r` rather than bounding it, so what still
  bounds it is written down here.** This is the part a later reader will not
  reconstruct, and a repair that fixed two assertions while silently removing
  every constraint on the value they were about would leave an implementer free
  to declare anything.

  **This table was corrected at implementation. It named the wrong scenario at
  the lower end**, and the correction is recorded in place rather than beside it,
  because a note saying a table is wrong leaves the wrong table standing.

  | End | What holds it | Measured against the built simulation |
  |---|---|---|
  | Above | FR-6.1-S2, the 120-tick rise | `r ≤ 16.0`. The rise is **exactly 120** ticks at `16.00` and 121 at `16.05`, pinned at hundredths. *(Drafted as "121 ticks at `r = 16`" — off by one, conclusion unchanged.)* |
  | Below | **FR-6.1-S4, the sink inside `360 × r` ticks** | `r ≥ 0.16`, and this is the binding lower constraint. It breaches at `r = 0.10`, where the sink takes 42 ticks against a 24-tick estimate. |
  | Below | FR-6.1-S5, a fall coming to rest later than in air | Observable from `r = 0.05`. **Looser than S4 and does not bind.** |
  | Below | FR-6.1-S6, swimming slower than walking the shore | Observable from `r = 0.05`. **Looser than S4 and does not bind.** |

  **The lower end is ragged, not a clean threshold, and a reader who assumes it
  is monotone will draw the wrong conclusion from a single sample.** Below
  `0.16`, admissibility *alternates*: FR-6.1-S4 fails at `0.05`, `0.06`, `0.09`,
  `0.10` and `0.15`, and passes at `0.07`, `0.08` and `0.11`–`0.14`. The cause is
  discreteness rather than noise — **the phase of the bob at the instant jump is
  released** decides how far the feet still have to fall, and that phase is not a
  continuous function of `r`. `0.16` is the point above which no sampled value
  fails: 80 further samples at `0.2` steps across `(0.30, 16.10)` found one
  failure, at `16.1`, above the other endpoint.

  The draft credited the lower end to FR-6.1-S5 and FR-6.1-S6 alone, which is
  what measurement falsified: both are observable three times lower than S4
  tolerates, so they were the pair that could not bind.

  Neither remaining bound is tight, and that is the intended state: the
  scenarios constrain the *behaviour* and leave the value to be derived.
  **Derive it from the built simulation, record the derivation, and record the
  measurement showing the shipped `r` sits inside that window.** Never tune it
  until the scenarios go green; a disagreement with the closed forms above is a
  finding to escalate rather than a number to adjust.
- **FR-6.1-S3's ceiling is a formula in the declared resistance, and the two
  earlier constants — `35.0` and `35.15` — were both wrong, in opposite
  directions and for the same reason.** This paragraph replaces a derivation
  that was persuasive and false; whoever reads it next would otherwise
  re-derive it and reach the same wrong answer.

  The topmost water voxel occupies `[34, 35)`, so a player whose feet are at
  `34.99` still overlaps water and swims one more tick. **`35.0` was too low**:
  it reddens against correct code, and the cheapest green is a surface clamp
  nobody specced. The repair asserted `35.15`, derived as *"the voxel ceiling
  plus one unresisted tick's rise, `(9.0 − 30 · 1/60) · 1/60 = 0.1417`"*. **That
  is one tick's displacement, and it stops there.** It omits the velocity the
  player carries out of the water — the quantity FR-4.1 exists to create, since
  the divisor acts on the velocity and it carries forward. Once the box clears
  `y = 35` nothing resists it any more and it coasts ballistically.

  **Two figures, two different quantities.** `35.125` is the highest a tick
  *whose box still overlaps water* can end. The peak across the **whole hold**
  is higher: measured `35.5806` at `r = 0.5` and `36.2583` at `r = 0`. Less
  resistance cannot produce a lower peak, and that contradiction is the tell
  that the two numbers were never measuring the same thing. **FR-6.1-S3 means
  every tick of the hold**, because the property worth asserting is *holding
  jump floats you at the surface and does not launch you out of it* — and a
  ceiling that stops looking the moment the box clears the surface cannot see
  the failure the scenario is named for.

  **The ceiling follows from the declared resistance and does not bind it.**
  Every *constant* ceiling would: the peak is monotone decreasing in `r`, so a
  fixed number is a hidden lower bound on the value the implementation is
  supposed to derive. `35.15` forced `r ≳ 2.1`; `36.0` would have forced
  `r ≳ 0.15` — a larger cage, the same mistake. The escape is a closed form, and
  it is one line of arithmetic rather than the heavier scenario it looks like.

  A player leaves the water at `v = (JUMP_SPEED − GRAVITY · TICK_DURATION) /
  (1 + r)`, rises at most `v · TICK_DURATION` on that tick, then coasts at most
  `v² / (2 · GRAVITY)` further. With `GRAVITY = 30` the second term is `v² / 60`.

  **It is arithmetic over declared constants and `r`, sharing no code with the
  simulation**, so it is an oracle rather than a snapshot. Validated against the
  simulation at eleven values of `r` from `0.0` to `16.0`: **it bounds the
  measured peak at every one**, with slack falling from `0.0875` blocks at
  `r = 0` to `0.0042` at `r = 16`. Never violated, and tight enough to still
  catch a runaway rise. That validation belongs in the record: it is what turns
  a formula into an oracle a reader can trust, and it is the evidence someone
  needs the day `GRAVITY` or `JUMP_SPEED` moves.
- **Absence means a constant, never solidity.** `drawn`, `occludes` and
  `targetable` default to `solid` because one bit used to answer those
  questions, so an old declaration still states them. Nothing ever answered
  these two, so a derived default here would invent a claim no author made —
  and would make every solid block in existence swimmable.
- **The greatest resistance among the overlapped cells decides.** The box spans
  1.8 blocks and always overlaps two or three voxel layers, so *some* rule is
  required. The greatest is the conservative one and it is one sentence; an
  average would make the answer depend on the box's height, and "the voxel at
  the feet" would let a player wade at full speed with its head under water.
  FR-4.3-S1 uses `1.0` and `3.0` rather than `0.0` and `3.0` precisely so that
  the greatest, the sum, the average and the least each give a different answer.
- **A block resists what moves *through* its volume, not what stands on it.**
  Overlap is strict and half-open (`collide.rs`), so a player standing on grass
  overlaps only the air above it. FR-4.2-S1 is what stops an implementation
  sampling the voxel under the feet, which is the obvious shortcut because
  `on_ground` already lowers the box to ask a similar question.
- **A declared resistance is normalised on read, so `-0.0` registers as `0.0`.**
  The fold serialises the retained number by its bits, and `-0.0` and `0.0` have
  different ones — two declarations meaning the same thing would otherwise hash
  differently and tell every player their blocks changed. The retained width is
  the same `f32` the physics multiplies by, so no declared value is kept at a
  precision the tick cannot use. Covered as additional coverage on FR-2.1-S4's
  test-map line rather than as its own scenario.
- **`swimmable` and `move_resistance` are independent in both directions.**
  FR-5.1-S4 proves resistance does not imply buoyancy; FR-5.1-S5 proves buoyancy
  does not imply resistance. Water declares both, and if the implementation ever
  derives one from the other, one of those two reddens.
- **The unrecognised-field list is read whole and in order** (FR-3.1-S1),
  because a hand-maintained mirror compared by filtering cannot see an extra
  member — `standards/global/testing.md` §2 records two such mirrors that were
  each held at six while this same list grew to nine, and neither reddened.
- **FR-7.1-S1 states the byte sequence by hand.** `format.rs`'s own doc comment
  records that only a test which states the sequence can see a revision move;
  every other witness compares one fold to another and cannot see a leading byte
  that moved in both. FR-7.1-S7 is the same guard from the other side: it is
  what catches an appearance byte bumped along with the behaviour one, which
  FR-7.1-S4 alone cannot see because both hashes would have moved together.
- **FR-8.1-S2 asserts an absence and needs its control.** A reload path that
  came to mark nothing at all would satisfy it forever, so its test-map line
  carries a second test running the same harness over a reload that changes only
  `drawn` and asserting every section *is* marked.

## Existing Code to Leverage

| What | Location | Reuse |
|------|----------|-------|
| Declaration checking and the fault vocabulary | `crates/mc-world/src/content/luau_declaration/mod.rs` | `optional_boolean` takes `swimmable` unchanged; `FieldFault::wrong_kind` / `::invalid` and `kind_of` give the numeric refusals their wording |
| The recognised-field list and its refusal | same file, `RECOGNISED_FIELDS` | append two names in documented order |
| The block definition | `crates/mc-core/src/block/definition.rs` | append two fields |
| The two resolved views and their traits | `crates/mc-sim/src/replay/resolved.rs`, `crates/mc-sim/src/player/mod.rs` | the pattern a medium view follows; `Resolved::of` is where a definition becomes a per-voxel answer |
| One tick of motion | `crates/mc-sim/src/player/physics.rs` | `bounded`, `launched`, `fallen` and `settled` are where a medium enters |
| The box and its overlap rule | `crates/mc-sim/src/player/collide.rs` | `Aabb::around` already enumerates the voxels a box overlaps |
| The behaviour fold | `crates/mc-world/src/persistence/format.rs` | `DeclaredBehaviour` gains two fields; `BEHAVIOUR_REVISION` moves 2 → 3 |
| Load acceptance | `crates/mc-world/src/persistence/table.rs`, `Acceptance` | `OnlyUnchangedBlocks` and `ChangedBlocksToo` are what FR-7.1-S5 and S6 separate |
| Reload's geometry test | `crates/mc-sim/src/world/reload.rs`, `changes_geometry` | neither field belongs in it — the doc comment's list of non-geometry fields grows |
| Documented-refusal guards | `crates/mc-client/tests/documented_refusals.rs`, `documented_property_refusals.rs` | already sweep every page under `docs/modding/`; the four quoted refusals must be re-cut |

## Out of Scope

Binding.

- **Buoyancy in the physical sense**, and any field named `density`. Simulating
  it needs the player to carry a mass per volume too, which is a commitment well
  beyond swimming. The name stays reserved.
- **A second number on the declaration** of any kind — no separate vertical
  resistance, no swim speed, no directional or per-axis resistance.
- **Drowning, an oxygen meter, fall damage, or any health effect.** There is no
  damage or health system in the tree to attach one to.
- **A swimming animation, an underwater camera tint, fog, or any change to what
  water looks like.** PRO-904 settled how water is drawn, and nothing here
  changes a pixel of it.

  > **Amended 2026-08-24. This bullet used to end "and this spec re-shoots no
  > golden frame", and that clause rested on a factual premise measurement has
  > since falsified.** The premise was that the declared replay walk stays dry.
  > It does not: the scripted player's box overlaps `base:water` at **60 of the
  > 120 ticks** — 44–99 and 116–119 — including two of the three declared
  > capture ticks, 59 and 119 (architect phase, measured against the real
  > simulation; `open-questions.md` carries the per-tick box contents). FR-6.1-S1
  > forces water to declare `move_resistance > 0` and FR-4.1 makes that a divisor
  > on carried-forward velocity, so the pose at those ticks moves under every
  > design available to this spec — about 4.3 blocks at `r = 4`, which is blocks
  > rather than pixels. **Out of Scope binds against building what nobody asked
  > for; it has never bound a spec to ship a knowingly-red suite.** The re-shoot
  > is deductive from an approved scenario, and it is specified below.
  >
  > What stays out of scope, explicitly: **moving the declared spawn, moving
  > `SEA_LEVEL`, or adding a second declared scene.** Routing the walk around the
  > water to preserve two files would be shaping the instrument to avoid
  > measuring the change, and a golden walk that wades through the sea is better
  > evidence this spec shipped than one that does not.
- **New blocks.** Nothing that would use `move_resistance` without `swimmable`
  ships here — no lava, cobweb, thicket or deep snow. The field is exercised by
  test fixtures and by the documentation's worked example.
- **A medium acting on anything but the player.** No other entity exists.
- **Swimming affecting what a swing can reach**, placement rules, or block
  breaking. Aiming, reach and `replaceable` are unchanged.

## Dependencies

- PRO-904 (SPEC-021), merged. It supplies the split declaration surface this
  extends, the visible sea a player can now see themselves swimming in, and the
  spawn at `(63, 35)` on the shore.

## Assumptions

- **The shipped spawn is on the shore, so the sea is reachable on foot.**
  Derived from PRO-904's measured water footprint (`x ∈ [60, 63]`,
  `z ∈ [0, 34]`) against `SPAWN_COLUMN = (63, 35)`. FR-6.1-S6 asserts against
  that column directly, so the assumption is measured by a scenario rather than
  merely held.
- **The player is the only thing a medium acts on**, and it moves only through
  `advance_player`. Verified by reading `crates/mc-sim/src/player/`.

## Test-first exceptions

`standards/global/testing.md` §1 admits exploratory spikes and requires every
exception be documented here.

- **The architect phase spikes the physics integration point in a throwaway
  worktree**, to re-derive water's admissible `move_resistance` window against a
  built simulation rather than against the model in Technical Considerations.
  The instrument the binding demands does not exist until the divisor and the
  swim-jump are placed, which is what that phase decides — so without the spike
  the window can only be re-derived mid-implementation, where a disagreement
  reopens a closed spec. The spike is **discarded**: nothing reaches the branch,
  and `architecture.md` records the window and the method to obtain it and
  **never a value and never the spike's code**, so the implementer re-derives
  rather than copies. The architect is neither the test author nor the
  implementer, and both of those are fresh contexts that never see the spike, so
  the ownership rule the exception brushes against is not the one at risk.

## Notes

Out-of-scope observations, recorded and not built.

- **The documented-refusal guard sweeps `docs/modding/` and nothing else.**
  `crates/mc-client/tests/support/quoted_refusals.rs:54` holds
  `PAGES = ["docs", "modding"]`, so the guard walks that tree alone. But
  `docs/user/gameplay.md:140` quotes a printed refusal verbatim — *"`base:water`
  no longer behaves as it did when this world was saved…"* — and **nothing
  checks it.** A page the player reads carries an unguarded copy of engine text,
  which is precisely the drift the guard exists to catch on the mod-author side.
  Widening `PAGES` to include `docs/user/` looks like a two-line change and is
  not this spec's to make.

- **The bob peak is bounded from above and not from below.** This observation
  was first written against a fixed `36.0` ceiling, where it read *"nothing
  asserts the peak precisely"* — and **the closed form has since largely closed
  that gap**, since its slack is `0.0042` to `0.0875` blocks rather than most of
  a block. Recorded in its corrected, narrower form rather than deleted, because
  the residue is real.

  What survives: FR-6.1-S3 is an upper bound, so an error that makes the bob
  *weaker* than intended — a peak of `35.02` where the mechanism gives `35.12` —
  breaches nothing. The only witness left is FR-6.1-S2's 120-tick rise, which is
  loose by design (it admits `r ≲ 16`). Closing this means a two-sided assertion
  on the peak, and **an over-tight assertion on exactly this quantity is what
  caused this episode**: `35.15` reddened against correct code and its cheapest
  green was a surface clamp nobody specced. A lower bound needs a tolerance
  derived from the discrete undershoot, which is more than this spec should
  attempt while `r` is itself being derived. Tracked at completion.

- **Three pages now state counts that are true at merge and false in the
  interim, and Phase 4 must not read them as evidence.**
  `docs/modding/blocks-items.md:718` and `docs/INDEX.md:76` say the save folds
  **eight** fields, and `blocks-items.md:755` and `INDEX.md:76-77` say a
  declaration states **eleven**. The eleven is true as of Phase 1. **The eight is
  not true until T13 moves `DeclaredBehaviour`** — the fold still holds six.
  `tasks.md` assigns both counts to T03, and the branch squash-merges, so `main`
  never sees the gap. Recorded here because the safety of that is conditional on
  whoever implements Phase 4 knowing it: **a page saying eight is not evidence
  the fold has already grown**, and `BEHAVIOUR_REVISION` still has to move 2 → 3.
  One convention applied to all four sites, rather than two sites precise and two
  deferred, which would leave a reader working out which page follows which.

  **Only the counts moved.** `docs/INDEX.md:77` goes on to *enumerate* which
  fields re-mesh and which change behaviour only — "`drawn`, `occludes` and the
  six keys re-mesh, `solid`, `targetable`, `replaceable`, `breakable` and
  `breaks_into` do not" — and that sentence mirrors `docs/modding/hot-reload.md`'s
  "Visible after a reload?" table at `:143` and the paragraph under it. **Both are
  T14's**, which is where the reload contract for the two new fields is *written
  down* — `architecture.md` Decision 7 is where it was decided, and T14 introduces
  no part of it — and **they are updated together or they disagree.** Verified by word-level diff
  that Phase 1 changed three words in `INDEX.md` and nothing else: `nine` →
  `eleven`, `six` → `eight`, `nine` → `eleven`.

- **A `benches/support/` module is a test file, and nothing says so.**
  `crates/mc-world/benches/support/fixtures.rs` is `#[path]`-included by **four**
  `tests/` binaries — `mesh_budget.rs:36`, `mesh_fixtures.rs:17`,
  `mesh_fixture_scale.rs:26`, `mesh_properties.rs:36` — one of which carries the
  proptests holding the mesher to an exact partition of the visible faces. So an
  edit there changes what tests observe and belongs to the test author at rigor
  `high`, whatever the directory is called. **Nothing in the tree marks it**, and
  the implementer of this phase reached for it as a bench before being corrected.
  The convention belongs in `docs/technical/testing.md`, which is already where
  this project records test placement — not in a guard, which nobody specced.

- **Writing a *correct* refusal onto a modding page is what breaks the guard.**
  `quoted_refusals_in` (`crates/mc-client/tests/support/quoted_refusals.rs`)
  recognises a quoted refusal as **any fenced block whose first line begins
  `mycraft: `** — deliberately, so a newly pasted refusal comes under comparison
  with nothing to remember. The consequence nobody states: `documented_refusals.rs`
  then compares that block, line for line, against a run over **three** specific
  declarations, so a fenced refusal that is real, accurate and about anything else
  is reported as text the client never prints. This phase nearly shipped one —
  T03's ceiling refusal was drafted as a fenced block and caught before the gate
  ran, then rewritten as prose with inline backticks. The trap is that the safer-
  looking choice (quote the artefact) is the one that fails, and the mechanism is
  in a support module a documentation author has no reason to read. Widening the
  comparison's declaration set is the real repair and is twelve-fold work
  `documented_refusals.rs`'s own header already files as deferred; a line in the
  modding-guide contributor guidance is the cheap one. Neither is built here.

- **`move_resistance`'s documented ceiling is a number with no falsifier.**
  `docs/modding/blocks-items.md` now promises `at most 3.4e38`, and **nothing in
  the suite would redden if that boundary moved.** The `1e40` fixture is a decade
  past `f32::MAX`, so it would still pass against a loader that had come to refuse
  everything above `1e20` — it proves a ceiling exists, not where it is. The
  `3.4e38` / `3.5e38` reading that produced the number is a *measurement*, true of
  the tree the day it was taken, and a measurement is not an instrument. It is
  unguarded in a second sense too: the field-table test reads the `Field` and
  `Absent means` columns, so `Bound` is prose in a guarded file **one column away**
  from a test that could see it. Raised by the test author and outside its assigned
  scenarios — no scenario states a ceiling, and FR-3.1-S3 is scoped to "the value
  its absence means", which a bound is not. It named two repairs — a boundary pair
  at `3.4e38` / `3.5e38`, or extending the field-table reading to the `Bound`
  column — and filed both for completion.

  **Both were built, and this entry is corrected rather than deleted so that
  completion does not file an issue for work already in the branch.** The pair
  landed at `e4352f5` and `b1eb3fe` (the second completing the mutation record with
  the pair's own two mutations), and the `Bound` column reading landed at
  `599b01b`. The `1e40` fixture stays alongside, for the reason recorded above:
  a decade past the width proves a ceiling exists, not where it is, and the two
  witnesses sit at different resolutions — one good to a decade, one to three
  percent — with neither subsuming the other. Nothing here is outstanding.

- **`crates/mc-world/src/content/luau_declaration/mod.rs` is at 499 of 500
  lines.** Two fields and their reasoning took it from 488 to 518; the numeric
  vocabulary moving to a `number` child brought it back under, and the doc
  comment this phase owes `defaulting_to` spent most of what was left. **The next
  line added to that file fails the gate**, and the suite cannot report it —
  clippy and the size stage are the only instruments that can, which is
  `standards/global/testing.md` §2's "a green suite is no evidence about a lint".
  The split that is ready to make is the fault vocabulary — `FieldFault`,
  `listed` and `kind_of`, roughly 150 lines — into a child beside `texture` and
  `number`, which both already import it from the parent. Not made here: no
  scenario needs it and it is out of this phase's diff.

- **`docs/INDEX.md` is a field-list mirror that no guard covers.** It stated
  three counts — "all nine fields", "which of the nine fields re-mesh", and "six
  fields a save folds" — and it sits outside `PAGES = ["docs", "modding"]`. **All
  three were corrected by hand in Phase 1** (`:76-77`, to eleven, eleven and
  eight) rather than at completion, because a schedule whose only enforcement is
  somebody remembering a Notes entry is not a schedule. **The standing gap is
  that nothing would have told anyone.** That is the same hole recorded above for
  `docs/user/gameplay.md`, and a file which rots twice for one reason is worth
  one line saying so.

- **Nothing catches a moved camera path the way `scene_contract.rs` catches a
  moved mesh.** This spec proves the hole rather than hypothesising it: the
  declared walk's pose at ticks 59 and 119 moves, four goldens are invalidated,
  and the scene contract stays green throughout because quad count and per-block
  area are properties of a mesh nothing touched. A tripwire stated over the
  *declared camera path* — the per-tick player pose the script produces, which
  `support::frames::player_pose` already computes without a GPU — would fail
  first and name `SCENE_REVISION` the way the mesh tripwire does, instead of
  leaving two image comparisons to report it without guidance. **Correcting the
  constant's false doc sentence is repair and is in scope; adding this guard is
  a feature and is not.** Tracked at completion.

- **`ResolvedVoxels::set` takes a named triple, and `architecture.md` declares
  four loose arguments.** Decision 1 and the `## Interfaces` block both state
  `set(&mut self, at, solid: bool, targetable: bool, medium: MediumIndex)`
  verbatim and BINDING. **The gate refuses it**: `clippy::too_many_arguments`
  counts `&mut self`, so that is 5 of 4, and `clippy.toml:10` sets the threshold to
  4 quoting `code-quality.md` §2 — *"max 4 parameters (use an object beyond
  that)"*. Measured twice, independently: the test author hit it in a throwaway
  scaffold and the implementer reproduced the same error from the shipped tree.
  **Only clippy can see this** — no suite reports a lint, and the window in which
  the tree did not compile had no gate at all.

  What shipped is `set(&mut self, at: WorldPos, answers: VoxelAnswers)`, where
  `VoxelAnswers { solid, targetable, medium }` is the module's existing private
  `Resolved` struct made public. **Taking the object needs no exception and
  suppressing needed one**: `code-quality.md` names the remedy in the same
  sentence as the rule, while an `#[expect]` is a violation of a constitution rule
  and those "require explicit justification and user approval". The tree carries
  nine allow/expect attributes in total and one on a production lint, so
  suppressing was also against its own habit.

  **The sentence this displaces was a real argument, not boilerplate.** `set`'s
  doc comment said the arguments were separate *"so that a caller passing the same
  answer twice reads as a caller doing so on purpose"* — written when there were
  two `bool`s. A struct whose **fields are named at the call site** serves that
  reason rather than defeating it, and the doc comment was rewritten to say so
  rather than left standing beside a signature it no longer described. Recorded
  because a later reader comparing the code against `architecture.md` will find the
  two disagree and is owed the reason.

- **`BlockRegistry` gains a `definitions()` accessor, and no scenario names it.**
  The medium table is built from *every registered definition* (Decision 1), and
  `mc-core` had no way to ask for them — only `registered_count()` beside
  `definition(BlockId)`, which returns a `Result` that cannot fail here. Rather
  than propagate an impossible error along the resolve path, the registry answers
  what it holds. It is a public `mc-core` addition reaching a crate boundary
  without a scenario behind it, which is the same kind of thing Phase 1 recorded
  for `BlockDefinition` losing `Eq`.

- **`architecture.md`'s adaptation counts are low, and Phase 4 should not reuse
  them.** Decision 3 measured "three test-helper signatures" and "eight edits, six
  of them in test files". Re-measured with `grep -rn "dyn Solidity"
  --include=*.rs crates/*/tests/`: **seven** `&dyn Solidity` helper parameters in
  **five** test files — `player_collision.rs:224` and `:232`, `player_ground.rs:211`
  and `:219`, `player_motion.rs:157`, `player_resolution.rs:170`,
  `replay_solidity.rs:244`. Two files it never names carry a helper each. The
  widened `set` added an eighth site the task list did not carry either
  (`resolved_voxel_updates.rs`, which Decision 1 discusses at length while
  `tasks.md` omits from T05), and `tests/support/volume.rs`'s `Declaration` had to
  grow the two fields and drop `Eq`. Ten edits in nine files, all of them the test
  author's. **The direction of the error is what matters**: every count in that
  decision was a floor rather than a total, and it was reached by reading rather
  than by running the grep.

- **`crates/mc-sim/src/replay/resolved.rs` reached 490 of 500 and was split in
  this phase's refactor**, unlike Phase 1's ready split, which was named and
  deliberately not made. The difference is whose diff it is: `luau_declaration`
  was already at 499 before that phase touched it, while this file was taken to 490
  *by this phase*, so the split is refactoring of the phase's own diff rather than
  work outside it. `MediumIndex` and the table moved to `replay/medium.rs`; the
  view is at 407 and the table at 126.


- **`architecture.md` Decision 3 has the two halves of the fixture rule backwards,
  by a factor of forty-six, and the rule is right anyway.** It argues the
  `swimmable` half is what the existing suite guards — naming `player_collision.rs`
  (29 `jump` references) and `player_ground.rs` (37) as "the teeth" — and that the
  `move_resistance` half "is inert" and is "the half that rots". Measured at
  implementation by making the air itself buoyant, then resistant:

  - **buoyant → one** pre-existing test reddens, and **`player_collision.rs`, the
    file named first, does not move at all.** Nearly every jump in the suite is
    asked *from the ground*, where `on_ground || buoyant` is already true, so the
    buoyant half has almost no reach into the collision suite.
  - **resistant → 46** reddens, across twelve files: every walk, every fall, every
    jump arc, the replay poses and a golden frame.

  **The rule survives and its case is stronger.** Both halves stay
  `VoxelMedium::NOTHING` unconditionally — and the reason is now the measured one:
  *the buoyant half is the thin one*. A rule that exempted the half nothing watches
  would have exempted precisely the half that turns out to be unguarded, so what
  needs defending is that single `player_ground` test rather than the two files the
  document names.

  **The mechanism of the error is worth more than the correction.** The table it
  rests on counts how often those files *mention* `jump`. That is necessary and not
  sufficient, and the substitution of a count of mentions for an observation of
  behaviour is the same move as the `0.1417` derivation and the two-file reporter
  estimate: a quantity reached by reading, believed because it was carefully
  reached. Corrected in `architecture.md` Decision 3 and in
  `crates/mc-sim/CLAUDE.md`, both of which carried it. Full record in
  `test-map.md`'s Phase 2 mutation check.

- **A non-power-of-two resistance must stay in the suite, and Phase 3 is where this
  stops being theoretical.** `slowed` promises a division by `1 + r` and refuses a
  multiplication by its reciprocal — the two agree **bit for bit** wherever `1 + r`
  is a power of two. Every resistance Phase 2 declared was one (`0.0`, `1.0`,
  `3.0`, `1e30`), so the mutation swapping the two forms **survived a 560-of-560
  green run**. It is closed by one fixture declaring `2.5`, and the choice was
  measured before it was made: at the obvious first candidate `0.5` the two forms do
  not differ at all, so the naive fixture is once again the one that passes. The
  difference at `2.5` is one unit in the last place — three orders below the epsilon
  every other comparison in that file uses — so bit equality is *required* there
  rather than merely allowed.

  **T09 derives water's `move_resistance` from the built simulation and it will not
  be a power of two minus one.** Whatever it derives, the awkward fixture stays: if
  it is removed, or if every remaining resistance happens to make `1 + r` a power of
  two, that mutation goes straight back to surviving and the division/reciprocal
  distinction ships with nothing behind it again.

- **On a shared tree, a `reset` that correctly un-stages someone else's work leaves
  `HEAD` internally inconsistent, and only a clean checkout or a compile would show
  it.** Recorded as a mechanism rather than as blame, because no suite in this
  repository could have reported it. An implementation commit swept in a test file
  the test author had staged between a `git status` and a bare `git commit` — the
  index is shared, so explicit paths on `git add` do not protect it and only
  `git commit -- <paths>` does. The repair was right: `reset --soft HEAD~1`, unstage
  the test file without touching the worktree, re-commit path-limited. But the
  unstaged file then sat in the worktree across four commits, so from `beec49f` to
  `2404a0f` **`HEAD` carried a triple-taking `set` in `src` and a four-argument call
  in `tests`.** Nobody working in that tree could see it, because the worktree
  compiled; only a fresh clone would not have. Closed at `7e2e1f0`. **The gate
  cannot run on a tree that does not compile**, which is what makes this the same
  family as the uncompilable RED window `tasks.md` already plans for — except that
  this one was invisible and unplanned.


- **PROPOSED STANDARDS AMENDMENT — `standards/global/git-workflow.md` §2
  "Staging".** Not applied here: the standards are constitutional and an
  amendment needs the owner's approval. Recorded with its mechanism so the
  approval question can be asked from evidence rather than from a recollection.

  The section bans `git add -A` and `git add .` and stops there. **That is a rule
  about staging, and the hazard is at the commit.** On a shared working tree the
  *index* is shared too, so staging explicit paths guarantees nothing about what a
  bare `git commit` will take — anything another agent staged between your
  `git status` and your commit goes in with it. Measured here: an implementation
  commit swept in a test file its author had staged in that gap, which is exactly
  the separation the TDD sequence exists to keep, produced by a command that
  followed the existing rule to the letter.

  The amendment is one sentence: **stage explicit paths *and* commit explicit
  paths — `git commit -- <paths>` — because the ban on `git add -A` protects your
  staging and nothing protects your index.**

  The repair sequence, which is the other half worth keeping: catch it on the
  commit's own `--stat`; confirm `HEAD` has not moved; `git reset --soft HEAD~1`;
  `git reset -- <the file you did not write>` to unstage **without touching the
  worktree**; re-commit path-limited. The other agent's content survives
  byte-for-byte and it commits the file itself. Never `git checkout --` at any
  point — that discards the work rather than unstaging it, which is the failure
  `git-workflow.md` §2 already warns about in its own paragraph.

  **And it leaves a second defect behind that no suite can see**, recorded above
  under the shared-tree entry: the un-staged file sits in the worktree, so `HEAD`
  is internally inconsistent until its owner commits it, while every working tree
  in that window compiles.

- **Water's `move_resistance` was derived at `1.6`, and this is the record
  `architecture.md` Decision 5 binds T09 to leave.** The instrument was a
  throwaway crate outside the repository, path-depending on `mc-core`, `mc-sim`
  and `mc-world`: a real registry from a copy of `content/base` with water's
  declaration rewritten per sweep point, the real `ReplayWorld` from
  `REPLAY_SEED`, the real `World`, and the real `advance_player`. The player is
  dropped into open air and advanced under a no-op intent until the world stops
  it, never placed at a computed height. **It reproduces `spec.md`'s own recorded
  figures exactly** — peak `36.2583` at `r = 0`, `35.5806` at `r = 0.5`, ceiling
  slack `0.0875` at `r = 0` — and that agreement is what makes it an instrument
  rather than a second opinion.

  **The window, measured.** Above, `r ≤ 16.0`: FR-6.1-S2 surfaces in **exactly
  120** ticks at `16.00` and 121 at `16.05`, pinned at hundredths. Below,
  `r ≥ 0.16`: from there upward nothing fails at any sampled point — 80 samples
  at `0.2` steps across `(0.30, 16.10)`, one failure, at `16.1`, above the
  endpoint. **Below `0.16` admissibility is ragged rather than absent**, and the
  raggedness is the near-surface discreteness Decision 5 step 4 warns of: the
  phase of the bob at the instant jump is released decides how far the feet still
  have to fall, so FR-6.1-S4 fails at `0.05`, `0.06`, `0.09`, `0.10` and `0.15`
  while passing at `0.07`, `0.08` and `0.11`–`0.14`.

  **The value is the geometric centre of that window** — `√(0.16 × 16.0) = 1.6`
  exactly, a factor of ten from each end. The criterion is the one part of T09
  that is judgement rather than measurement, and it is recorded as such: the spec
  deliberately leaves the value under-determined, so *some* criterion was needed.
  The geometric rather than the arithmetic centre because the window spans two
  orders of magnitude and the parameter is a multiplicative one — the arithmetic
  centre is `8.08`, which stands `1.98×` from the upper bound and `50×` from the
  lower and is a centre in no sense that matters here. `1 + r = 2.6` is not a
  power of two, so it does not collide with the entry below.

  **What that criterion optimises for is not the obvious thing, and the
  distinction is the reason this entry exists.** It maximises the factor by which
  a *physics constant or a scenario threshold* can move before the shipped value
  stops satisfying a scenario. **It says nothing whatever about how the water
  feels to play.** That is the right criterion here for one reason and it is not a
  flattering one: **nobody has played this and nobody can, so a feel criterion was
  unavailable rather than declined.** There is no windowed client anyone has swum
  in, and no person has yet formed an opinion about the sea for a number to be
  fitted to.

  **So this is the first number to revisit once somebody actually swims in it.**
  The figures a person will form an opinion about, recorded here so they are
  findable next to the reason nobody had one yet: swimming carries you at
  **38.5%** of walking speed (`1.73` against `4.5` blocks per second), a released
  float sinks at a terminal **`0.3125`** blocks per second, and surfacing from the
  deepest lakebed takes **19 ticks**, about a third of a second. Every one of
  those is a consequence of `1.6` and none of them was chosen. If the sea feels
  wrong to the first person who swims in it, the window `[0.16, 16.0]` is what
  they have to move within, and re-deriving the centre is not the answer — their
  judgement is.

  **Measured at `1.6`, against `spec.md`'s own forms**: S2 rise **19** ticks of
  120; S3 peak **35.20632** under a ceiling of **35.232616**, slack `0.0263`;
  S4 sink **369** ticks of a **576** budget; S5 rest at **385** ticks against
  **52** through a sea that resists nothing; S6 a submerged step of
  **0.028846** against **0.075000** on the shore at `(63, 35)`. What a player
  meets: swimming at **38.5%** of walking speed, a terminal sink of `0.3125`
  blocks per second.

- **Three things measurement corrected in this spec's own documents, none of
  which is a closed-form disagreement.** Every scenario form holds at the derived
  value exactly as written, so nothing was softened to admit it.

  1. **`architecture.md` Decision 5's "its deepest is (63, 30)" reads as unique
     and is not.** There are **47** columns at depth 2 and 84 at depth 1, and the
     47 are **one fixture**: every one of them settles the feet at exactly
     `33.0`, a single distinct value across the set. `(63, 30)` is among them, so
     the named fixture is valid and nothing built on it is wrong — but a reader
     taking the sentence for a unique landmark would go looking for a property it
     does not have. The 131-column and 178-voxel census reproduce, as does the
     shallowest at `(61, 0)`.
  2. **"121 ticks at `r = 16`, so `r ≲ 16`" is off by one.** The built simulation
     gives **120** at `16.0` — which *passes* — and 121 at `16.05`. The
     conclusion survives untouched; only the count moves.
  3. **FR-6.1-S4's "constant 50% headroom everywhere in the band" is false at the
     bottom of the band, and the bounds table named the wrong scenario.** The
     `0.979`–`0.997` ratio holds for `r ≳ 1`, but at `r = 0.10` the sink takes 42
     ticks against a 24-tick estimate — a ratio of `1.75` — and breaches the
     budget outright. **So FR-6.1-S4 bounds `r` from below, at `≈ 0.16`**, while
     FR-6.1-S5 and FR-6.1-S6, which the table credited, are observable from
     `r = 0.05` and are the *looser* pair. The scenario is right and its budget is
     right; what was wrong is the claim about how much room the budget leaves, and
     which scenario is spending it.

     **Unlike corrections 1 and 2 this one is repaired in the artefact itself** —
     the table under "Both scenarios now scale with `r`" now names FR-6.1-S4, its
     measured `0.16`, and the raggedness below it. A note recording that a table
     is wrong, with the wrong table left standing, is two documents disagreeing
     and a reader who finds the wrong one first. Corrections 1 and 2 stay as notes
     because neither falsifies what its artefact *says*: a named fixture that is
     one of 47 interchangeable columns is still valid, and an off-by-one tick
     count leaves its conclusion exactly where it was.

- **`[0, 59, 119]` is written out three times and nothing compares the three.**
  `DECLARED_CAPTURE_TICKS` (`crates/mc-render/src/capture.rs`), `JUDGED_TICKS`
  (`crates/mc-client/tests/replay_oracle.rs`) and `SAMPLED_TICKS`
  (`crates/mc-client/tests/replay_determinism.rs`), with
  `support::goldens::DECLARED_TICKS` re-exporting the first. **Add a fourth
  declared capture tick and the oracle and the determinism suites go on judging
  three, silently** — no test fails, because each array is internally consistent
  and none is compared against the authority. That is exactly the
  hand-maintained-mirror shape `standards/global/testing.md` §2 records from this
  project's own scar, where two mirrors of one field list sat at six while the
  loader grew to nine and neither reddened.

  **Deferred, and the line is worth stating because this spec ruled the opposite
  way once already.** Phase 1's undefended ceiling was a promise *this spec
  created*, and a spec fixes what it breaks, so the guard was built. These mirrors
  **predate this spec** — PRO-852 and PRO-904 made them — so filing is the right
  answer and building is not. What makes the risk live rather than theoretical is
  that **this phase touched two of the three**: T10 moved the revision the first
  one names, and T12 re-derived what is stated about the frames the other two
  select. Tracked at completion; the guard is one assertion that the three agree.

- **The spec's own "what moves" enumeration was scoped by the wrong question, and
  it missed things in two different files for one reason.** `open-questions.md`
  lists what this spec moves, and every entry on it is *something stated against
  the published player camera* — the goldens, `JUDGED_TICKS`, `SAMPLED_TICKS`, the
  sea reading. **Two tests in `crates/mc-sim/tests/replay_solidity.rs` went red at
  T09 and are on no list at all**, because they are stated against a **fall and a
  walk through the sea**, which is a question that enumeration never asked. The
  same mechanism left `docs/user/gameplay.md:69-70` out of the Documentation
  deliverables' list of falsified passages.

  **The enumeration asked "what does this spec move the camera past" when the
  question was "what does this spec change the physics under."** No amount of care
  answering the first reaches the second, which is why this is a scoping failure
  rather than an oversight — and why it produced misses in a test file and a
  player-facing document at once.

  **The generalisation is worth more than either miss, and it has a sting in it.**
  An enumeration built by asking a narrower question than the one that matters
  does not miss randomly: it misses **precisely the thing the spec is most about**.
  Both misses here land on the subject — the falsified-passages list omitted the
  passage about *walking through water*, and the "Moves" list omitted the tests
  about *a fall and a walk through the sea*, in a spec whose title is that you can
  swim in it. The reason is structural rather than bad luck. A narrower question is
  narrower because it names a **mechanism** — a revision byte, a camera pose — and
  a mechanism is a thing the spec *uses*, while the subject is what the spec
  *changes*. Enumerating by mechanism therefore sweeps up everything the change
  travels through and drops what it arrives at.

  So the check that would have caught both is not "did we look carefully" but a
  different question asked once: **what does this spec change, and what stands on
  that?** Asked of "water becomes a medium", both the gameplay passage and
  `replay_solidity.rs` fall out immediately, and neither requires knowing anything
  about revisions or cameras.

  **The standing consequence, measured: 12 of `mc-sim`'s replay tests build the
  shipped world**, and any of them standing in the sea now measures a medium as
  well as whatever it is named for. The two that reddened are the two doing
  *arithmetic* over a distance or a tick count. **A test asserting only a
  direction or a contact would have absorbed the change in silence and would be
  green today** — so the two failures are not the population, they are the part of
  it that happened to be arithmetic.

- **The Bash tool silently corrupts string literals containing a backslash, and
  only `cat -A` can see it.** Two independent sightings in one session, which is
  this project's own trigger for writing something down. **Its durable home is
  `docs/technical/working-in-this-repo.md` and Phase 4 writes it** — batched with
  the two additions that phase already owes `docs/technical/testing.md`, rather
  than done here.

  **The mechanism.** Commands routed through the Bash tool are rewritten before
  they run, and the rewrite strips one level of backslash out of a quoted
  heredoc. A Rust line continuation written correctly as

  ```
  "… where this snapshot \
   records …"
  ```

  reaches disk as **one line**, with the second line's indentation surviving as
  literal text inside the string — so the message renders as
  `where this snapshot          records 1.6`.

  **Why it is worth an entry rather than a shrug: it compiles, it runs, and every
  instrument this project owns is blind to it.** `cargo build` is happy, `cargo
  fmt` does not touch it and does not reintroduce it, clippy at `-D warnings` says
  nothing, and the test passes — the damage is a run of spaces *inside* a string
  literal. `cat -A` is the only detector, and only if somebody already suspects.

  **The three-strike account, because a hazard hit once reads as a typo.** The
  test author hit it writing the tripwire's message, then **twice more while
  fixing that very defect** — once loudly, as a Python `SyntaxError` on `' \\'`,
  and once **silently again** — and only got a clean edit by constructing the
  backslash as `chr(92)`. The same wrapper also fails *loudly* in a second way,
  which is how the implementer met it: a `grep -n "^## " …` came back as
  `syntax error near unexpected token '&'`, the command having been rewritten into
  PowerShell syntax and handed to bash. **The loud failures are harmless and the
  quiet one is not**, and they share a cause, so meeting the loud one is a reason
  to suspect the quiet one rather than to work around it and move on.

  **What to do instead**: write anything containing a backslash — a Rust line
  continuation, a regex escape, a `\n` — with the Write or Edit tools rather than
  a shell heredoc. Where a script genuinely must emit one, build it as `chr(92)`.
  **This entry was written with Edit for exactly that reason.**

  **And the check that beats a spot-render**: scan the whole literal for the
  defect class rather than re-rendering the one line that was wrong — a regex for
  `\S {2,}\S` over the file after `cargo fmt`. It covers every line rather than
  the one you already know about, and it independently clears `cargo fmt` of being
  the culprit, which no single render can do.

- **The shipped resistance has exactly one witness, and the sea scenarios are
  structurally incapable of being it.** Measured as mutation M1, pre-registered
  before running: `move_resistance = 1.6` → `move_resistance = 2.0` in
  `content/base/blocks/water.luau` — another value the measured window admits, so
  this asks whether the **value** is guarded rather than whether the physics is.
  Baseline `702 tests run: 702 passed`; under the mutation `702 tests run: 701
  passed, 1 failed`, both bare counts and so both complete runs. **The one is
  `mc-client::terrain_goldens::every_declared_capture_matches_the_golden_committed_for_it`.**
  The prediction was 1 and named that test; it held.

  **The mechanism is a direct consequence of getting the scenarios right, and it
  is worth stating plainly because it looks like a defect and is half of one.**
  Every FR-6.1 test reads the declared resistance out of the registry and states
  its threshold as arithmetic over it — which is exactly what stops a constant
  threshold caging a value the spec deliberately leaves free (`architecture.md`
  Decision 5, and the two earlier ceilings that did cage it). The price is that
  those tests **move with the value and therefore cannot see it change**. A
  closed form over `r` is silent about which `r` was chosen, by construction.

  **What follows is a real gap, recorded rather than closed.** The golden set is
  the only thing that reports a changed value, and a golden set is re-minted as a
  matter of routine whenever a re-shoot is justified. **So a change to this number
  that re-mints the goldens in the same commit is a change nothing in the suite
  reports at all** — and the guard against that today is entirely the review rule
  that an unexplained golden update is a stop, not any test. Closing it means a
  test asserting something about the shipped value that is *not* derived from the
  shipped value: the honest form is a bound stating what the sea must feel like to
  a player — a surfacing time and a swim-to-walk ratio inside stated brackets — and
  those brackets are the sort of number this spec has already had wrong twice in
  opposite directions. Not attempted here for that reason. Tracked at completion.

- **The re-shoot's own premise was checked rather than assumed, and it holds
  exactly.** The ruling to re-shoot all four directories rests on the claim that
  the two tick-0 captures reproduce byte-identically, so the cost of the symmetric
  choice is two captures and no changed pixels. Measured by hashing the minted
  `r3` images against the `r2` blobs they replace: `player-walk-t000` **identical**
  (`137f0b58`), `player-walk-hud-t000` **identical** (`950c1950`),
  `player-walk-t059` and `player-walk-t119` both **moved**. The asymmetry the
  ruling turns on is therefore real and not merely argued — and had either dry
  capture moved, that would have been a finding about the physics reaching a tick
  the architecture measured as dry, not a reason to re-shoot fewer.

- **`docs/user/gameplay.md:69-70` promised the opposite of what T09 ships, and
  it is not one of the four passages this spec's Documentation deliverables
  names.** It read *"Nothing about walking through it has changed — it still does
  not hold you up, and you still walk straight into it."* T09 is precisely what
  falsifies that, so the repair is T09's rather than T15's: it is a sentence
  about walking through water, and T15's four passages are all about the save
  record. Repaired here with what a player can now do. **The mechanism worth
  keeping is that the spec's own enumeration of false passages was itself
  incomplete** — it was assembled by asking which passages `BEHAVIOUR_REVISION`
  falsifies, which is a narrower question than which passages *this spec*
  falsifies, and the missing one was the one about the feature the spec is named
  for.


- **All four FR-8.1 scenarios were green against the unmodified tree, and that is
  the design working rather than a gap.** `architecture.md` Decision 7 predicted
  it — `adopt` replaces every resolved view wholesale, so the medium became a
  third answer that rule already covered the moment Phase 2 put it there — and
  Phase 4 confirmed it: `changes_geometry` never learned either field, and the
  loader already refuses a bad `move_resistance` on the path a reload's build
  stage calls. **So T14's eight tests are witnesses to a standing property, not
  falsifiers waiting on an implementation**, and the honest thing to say about
  them is that they check something only because three pre-registered mutations
  say so. M1 predicted 5 and named them, and gave exactly those 5; M2a and M2b
  each predicted 1 and named it, and each reddened a *different* one of the two
  marking tests. What was left for the implementation was the record: the doc-list
  of fields that change no geometry, `drawn_of`'s reason for keeping the medium
  out, the reload page's two missing table rows, and `docs/INDEX.md`'s mirror of
  that enumeration. **No test asserts doc-comment prose**, so that half is held by
  review and by nothing else.

  **Checked, because a green-from-birth task is exactly where an artefact comes to
  claim work it did not do: no artefact describes T14 as introducing behaviour.**
  Every one of them already framed it as a standing property plus a record that
  moves — `tasks.md` T14 opens "Hot reload needs no new mechanism";
  `architecture.md` Decision 7 predicted it outright and its Integration table
  says only that the doc-comment list grows while the predicate does not; and
  *Existing Code to Leverage* says "neither field belongs in it". One sentence was
  loose rather than false and is corrected in place above: the `docs/INDEX.md`
  mirror entry said T14 is "where the reload contract for the two new fields is
  decided", which it is not — Decision 7 decided it and T14 writes it down.

- **The revision-2 fixture tells the two *lists* apart and is blind to the
  *byte*, which was pre-registered as a miss and held.** M4:
  `crates/mc-world/src/persistence/format.rs:309`, `const BEHAVIOUR_REVISION: u8
  = 3;` → `= 2;`, list left grown. Predicted **2**, named — `format_test.rs`'s
  behaviour half and `save_declarations.rs`'s pinned fold. Measured **2**, exactly
  those, `1502 tests run: 1500 passed, 2 failed, 1 skipped`, a bare count and so a
  complete run. Reverted by hand, `git diff --exit-code` clean.

  **The green half is the claim.** All five tests over
  `world_saved_against_behaviour_revision_2.mcw` stayed green under that mutation,
  as did all three of `save_folds_the_medium.rs` — because a fold over the grown
  list differs from the recorded one whatever the leading byte says. The fixture
  is the right instrument for the question it was minted for and would be the
  wrong one for this. Recorded in `docs/technical/world-format.md` so that nobody
  later reads it as closing the hole only a byte-stating test closes.

  **And that page's own stated instrument no longer finds one of its own
  members.** The enumeration of files stating a byte sequence was recorded as
  `grep -rn "REVISION" --include=*.rs crates/` plus a read of each hit. Run today
  it returns `format_test.rs` and `save_per_face_appearance.rs` and **not**
  `save_declarations.rs`, which writes `03` in a hand-built byte table and says
  "version 3" in prose without ever naming the constant. A needle spelled after a
  constant cannot see a file that states the sequence without it. Corrected in the
  page rather than re-derived from the grep.

- **Phase 1 moved the counts and left the sentence, and Phase 4 found it twice in
  two files.** This is the *third* instance of the enumeration failure recorded
  above, and the first two were enough to state the rule and not enough to apply
  it.

  `docs/modding/blocks-items.md` had its fold count corrected six → eight by T03,
  and four lines further on went on saying *"This build did the second:
  `targetable` joined the fold"* — false the moment T13 lands, and untouched,
  because the enumeration asked **which counts moved** rather than **which claims
  this spec falsifies**. `docs/INDEX.md` is the same miss in the other direction:
  Phase 1 corrected its three counts by hand precisely because nothing guards that
  file, and nobody looked at the **gameplay row**, which carried neither the swim
  nor the save-record repair and had no `SPEC-022` among its sources.

  **A count is a mechanism and a claim is what the change arrives at**, which is
  exactly the shape the entry above names. Both repaired in T13 and T15
  respectively. The standing consequence is unchanged and worth restating: **`docs/INDEX.md`
  sits outside `PAGES = ["docs", "modding"]` and nothing whatever would have told
  anyone** — it has now rotted twice on this branch alone, once in its counts and
  once in a row nobody thought to read.

- **Phase 4's refactor pass changed nothing, and the one thing it looked hardest
  at was `format.rs`'s size.** The phase's production diff is two files: two
  struct fields with their initialisers in `crates/mc-world/src/persistence/format.rs`,
  and doc comments in `crates/mc-sim/src/world/reload.rs`. The file went 417 → 474
  of 500, and **almost all of the growth is doc comment** — the record Key
  Principle 4 owes, not code. So the Phase 2 test applies and gives the opposite
  answer to the one it gave there: that phase took `resolved.rs` to 490 *itself*
  and split it, this one leaves 26 lines of headroom, no scenario needs a split,
  and a persistence format module is a poor thing to cut for headroom alone. The
  next reader who does need to split it should note that the seam is not obvious:
  the two declaration structs, their two folds and the encoder are one subject,
  and the file's own header argues at length for keeping the two lists side by
  side where the reason for their separation stays readable. Recorded rather than
  acted on. `crates/mc-world/src/content/luau_declaration/mod.rs` is still the
  urgent one, at 499, with its ready seam named in an entry above.

## What this spec learned about its own evidence

One lesson, recorded because it outlives every finding above and is a candidate
for `standards/global/testing.md` at consolidation.

**Every premise this spec got wrong was reached by reasoning. Every correction
came from someone running something.**

The count is not close. Wrong by reasoning: the `0.1417` ceiling derivation,
which was arithmetic that stopped one term early and was persuasive enough to
survive being written, reviewed and approved; the estimate that a closed-form
ceiling would be "a much heavier scenario", which turned out to be one line;
the assumption that two files stating the same rule needed the same repair,
when one was already scoped and needed an extension instead; a fixed `36.0`
ceiling proposed as the fix for a ceiling that bound `r`, which would have bound
`r` too; and the claim that a solidity-derived resistance is inert *because the
box can never overlap a solid cell*, which asserted **geometry** where the truth
is a **maintained invariant** — two rules hold it, neither binds a fixture, and
a rule leaning on it would have gone quiet the moment either changed. That last
one is the subtlest of the five: it reasoned about what the code *must* do
rather than reading what *maintains* it.

Caught by running something: the walk wading the sea at 60 of 120 ticks; the
ballistic overshoot to `36.26`; a citation "correction" that moved a line number
off by one in the other direction; a note about the bob peak that went stale the
moment the ceiling changed under it; `425 of 512` samples mismatching where a
40-sample reading had suggested a narrower trap; and `player_motion.rs` carrying
zero `jump` references, which cut the reporter from three files to two.

**The asymmetry is the lesson, not the tally.** A wrong premise arrives already
dressed as a conclusion — the `0.1417` derivation reads exactly like a correct
one, which is what let it through three reviews — so reading it again mostly
re-confirms it. Rebuilding the quantity independently is what disagrees, and the
disagreement is the signal. Twice here the independent rebuild was *itself*
wrong first, and finding that out was still what located the real answer.

**And the dressing is proportional to the care taken.** The `0.1417` derivation
survived drafting, audit and approval *because* it was carefully derived; a
sloppy guess would have been challenged on sight. So care raises rather than
lowers the need for an independent rebuild — the opposite of how it feels while
deriving it, and the reason re-reading mostly re-confirms.

**The practical form**: when a number is load-bearing, the cheapest sufficient
check is to derive it a second way and compare, and to treat the disagreement as
information rather than as a defect in the second derivation.

**The counter-example carries the other half, and it is not "distrust
derivations".** The one figure that held up under every re-run — the 178-voxel
water census, reproduced independently three times by three different parties —
is the one nobody argued about afterwards. It was cheap to reproduce, so it was
reproduced, so it was never in doubt. The lesson is that **a quantity nobody can
cheaply re-derive is one that will be argued about instead**, which makes
"can this be re-derived in one command?" a property worth designing *into* a
figure rather than a question asked after it is challenged.

## Open Questions

Empty before implementation starts. Both are **closed**; recorded rather than
deleted, because what settled them is the reasoning a later reader needs.

- ~~**Does the declared replay walk enter the sea?**~~ **CLOSED — it does.**
  Measured against the real simulation in the architect phase: the box overlaps
  `base:water` at 60 of the 120 ticks, including capture ticks 59 and 119. The
  goldens move, `SCENE_REVISION` goes to `r3`, all four directories are
  re-shot, and the constant's own doc is repaired — see *The revision rule* and
  the amended Out of Scope. `open-questions.md` carries the measurement.
- ~~**How does a per-voxel resistance value reach the tick inside 256 KiB?**~~
  **CLOSED as a blocker, open as a design task** for `architecture.md`. The
  budget is met with room: the shipped world holds four declarations, exactly
  one of which states either medium field, so a voxel carries one of **two**
  distinct `(swimmable, move_resistance)` answers, and one bit per voxel over
  1 048 576 voxels is **128 KiB — half the budget, for both questions
  together**. That is a property of *today's content* and not of the design, so
  `architecture.md` owes the generalisation as well as the number: what happens
  when a mod declares a third distinct answer. **The 256 KiB constraint binds
  even though nothing in this increment measures memory** — "no test watches it,
  so relax it" is an argument about the suite, not about the requirement.
