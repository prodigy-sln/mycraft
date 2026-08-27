---
id: SPEC-030
title: Water carries a swimmer at rates content declares
status: implemented
work-type: feature
rigor: high
rigor-reason: >
  This adds to the published block-declaration surface mod authors write, and it
  moves the save format's behaviour revision. Both are things a later spec and a
  third-party mod must not break. It also changes the physics the committed
  golden frames are shot through, so the evidence that guards the shipped sea
  moves with it.
branch: feature/PRO-992-swim-ascent-rate
issue: PRO-992
created: 2026-08-27
updated: 2026-08-27
approved: 2026-08-27
completed: 2026-08-27
author: Sebastian Grunow
---

# Specification: Water carries a swimmer at rates content declares

## Goal

The first person ever to swim in this game reported that *"water feels more like
a swamp than water — you sink too slowly"*. Sinking through the sea's two voxels
takes six and a half seconds. Make the sea sink, lift and carry a swimmer at
rates a mod author declares, and ship values that put the base game's water
inside the range shipped games occupy instead of outside it.

## Stakeholder capability delivered

Named, per Key Principle 9:

- **Player** — the sea reads as water. You sink through the deepest part of it
  in about two seconds instead of six and a half, and swimming carries you at
  two thirds of your walking speed instead of a little over a third.
- **Mod author** — you can declare how your own liquid carries a swimmer, in
  each of the directions a swimmer moves, and predict the result from the
  declaration without running it.

## The diagnosis, corrected twice

**Every figure here is re-derived from `crates/mc-sim/src/player/physics.rs` in
`requirements.md` §1–§5. Two earlier statements of this diagnosis were wrong and
both corrections are recorded there rather than quietly dropped.**

One tick sets the vertical velocity to `(launch − g·dt)/(1 + r)` and the
horizontal to `W·deflection/(1 + r)`. Sinking is a *terminal-velocity balance*
against gravity, settling at `g·dt/r`. Rising is a *raw impulse re-applied every
tick*, so it is `(J − g·dt)/(1 + r)` from the first tick and never accumulates.
One declared number, `move_resistance`, is the only free parameter in any of
them.

### What one coefficient can and cannot reach

| `r` | sink | s/block | swim | % walk | rise |
|---|---|---|---|---|---|
| 1.6 *(shipped)* | 0.3125 | 3.20 | 1.731 | 38% | 3.269 |
| 1.0 | 0.500 | 2.00 | 2.250 | 50% | 4.250 |
| 0.889 | 0.5625 | **1.78** | 2.382 | 53% | **4.500** = `W` |
| 0.5 | 1.000 | 1.00 | 3.000 | 67% | 5.667 |

**The binding constraint is a hard boundary at a sink of 0.5625 blocks per
second — 1.78 seconds per block.** Below it the rise exceeds `WALK_SPEED`: a
held jump in water carries you upward faster than a walk carries you forward on
land. Against the surveyed games (figures relayed from the survey; see
`requirements.md` §4(b) for which of its numbers are void and why these are not):

| | sink b/s | s/block | swim b/s | % walk | rise/sink | one coefficient reaches it? |
|---|---|---|---|---|---|---|
| MC Java ≤1.12 | 2.00 | 0.50 | 1.96 | 45% | 1.0 | **no** |
| MC Java 1.13+ | 0.50 | 2.00 | 1.96 | 45% | 7.0 | yes |
| Luanti | 2.0 | 0.50 | ~3.0 | ~75% | 1.5 | **no** |
| Source | 0.91 | 1.09 | 2.90–4.88 | 80% | 1.0 | **no** |
| Quake | 0.80 | 1.25 | 4.27 | 70% | 1.0 | **no** |
| **MyCraft now** | **0.3125** | **3.20** | **1.73** | **38%** | **10.46** | — |

Aquatic (1.13) cut water gravity **4×** and left both drag coefficients at
`0.8`: a team owning this exact complaint, with a year to think, moved the
source term and not the damping. **No surveyed game splits a damping coefficient
by motion source, and none ships quadratic drag.**

**So a single-value retune is not useless — it reaches Minecraft's sink exactly,
at `r = 1.0`, with the swim at 50% of walking.** That is the honest correction
to this spec's first draft, which claimed no retune could fix anything. What a
single coefficient cannot do is reach *three of the four* surveyed games, and
Minecraft is the one whose sink is slowest — the property the complaint is
about. The freedom is worth buying on that evidence, not on the claim that
nothing else works.

### Why the owner's hypothesis, taken literally, does not buy it

The steering on this spec is that the coefficient may need to differ **by the
source of the motion** — that what gravity drags you down with need not equal
what your own strokes are damped by. It is the right instinct about where the
missing freedom lives, and it is worth stating precisely why the literal form
does not deliver it, because the arithmetic is not obvious.

Rise and horizontal swim are one expression over two numerators, so **their
ratio is free of every damping coefficient in the model**:

```
rise / horizontal = (JUMP_SPEED − GRAVITY × TICK_DURATION) / WALK_SPEED
                  = 8.5 / 4.5 = 1.8889
```

Splitting the coefficient by source gives `r_g` for the sink and `r_i` for
intentional motion — but the rise and the horizontal swim are *both* intentional,
so both are divided by `1 + r_i` and the ratio above survives untouched. The
consequence is measured:

- Set `r_i` for a swim of 3.0 b/s and the rise is forced to **5.667 b/s**, above
  walking speed. The fountain is still there.
- Set `r_i` for a rise of 2.0 b/s instead and the swim is crushed to **1.06 b/s,
  24% of walking** — worse than the 38% being complained about.

**Splitting drag by source frees the sink from the swim, and locks the swim to
the rise in its place.** It trades one coupling for another. The survey found
that no shipped game splits a damping coefficient by motion source, and this is
the arithmetic reason why: what actually buys freedom is splitting the **source
term**, not the damping.

### What does buy it

The rise is `(J − g·dt)/(1 + r)` and `J` is `JUMP_SPEED`, an engine constant.
**Every model that solves this makes the thing driving the rise a declared
quantity**; they differ in what else they declare. That is the decision the
architect phase owns, and the candidates are in the Architecture Delta below.

## What is settled, and by which phase

**Settled — the observable rates.** FR-6 states the sink, rise and horizontal
swim of shipped water as absolute displacements in absolute tick counts. Those
hold whichever model the architect picks, and they are the contract the player
and the mod author actually meet.

**Settled by the architect phase — the declared surface.** The model is
**Candidate A**: one declared field, `swim_ascent`, replacing `JUMP_SPEED` on the
buoyant path and nowhere else. FR-1, FR-2, FR-3, FR-4, FR-5, FR-7 and FR-8 were
written against exactly that candidate and are **confirmed as written**; their
`[model-dependent]` markers are removed and no field name or count moves. The
decision, the measurements that reached it, and what happens to land movement
under each rejected candidate are in `architecture.md`.

**Two scenarios were added by that phase**, FR-4.1-S9 and FR-4.1-S10. Both exist
because `swim_ascent` is the first medium property whose loader default (`9.0`)
differs from its fold identity (`0.0`), which makes a swimmer beside a
non-swimmable block rise at `5.667` b/s unless the fold masks it —
`architecture.md` Decisions 3 and 4.

## The evidence hole this spec must not inherit

`content/base/blocks/water.luau` says of its own resistance:

> *"Exactly one test guards this number… a change here that re-mints the goldens
> in the same breath is a change nothing in the suite reports at all."*

`crates/mc-sim/tests/support/sea.rs:5` says why: *"every threshold a scenario
carries is arithmetic over the value read back out of the shipped registry."*

That was right when the number was unjudged — a derived threshold stops a
scenario caging a value it should leave free. It is wrong now that play has
judged it, because **every existing sea scenario moves with the declared number
and none can see it change.** FR-6 reverses it deliberately.

## Known, measured, and deliberately not addressed

Recorded so that a second "it still feels like a swamp" starts from a hypothesis
instead of re-deriving this. Neither is in scope.

- **The residual drag rate.** `r = 0.5` at 60 Hz is a continuous rate of
  **24.33 s⁻¹**, against Minecraft's water at **4.463 s⁻¹** — still **5.45×
  harsher**. The shipped 1.6 is 57.33 s⁻¹, 12.85×. Terminal speeds are reached
  in about two ticks either way.
- **The control-law asymmetry.** Horizontal motion is a *velocity target* —
  `walk_velocity` sets the speed each tick, with its own doc saying *"no
  acceleration to build up and no inertia to shed"* — while vertical motion is
  force-driven. A swimmer therefore stops dead the tick input is released, at
  **any** resistance. No surveyed game ships water without coasting.

If the retune does not land, these two are where to look before touching the
declared values again.

## Documentation deliverables

Per Key Principle 4, owed as part of this spec's definition of done.

- **Mod author** — `docs/modding/blocks-items.md` gains every property the
  chosen model adds: field table row, annotated example, per-field prose with
  refusals quoted from real runs, and the recognised-field list in all four
  quotations across three pages that `documented_declaration_fields.rs` reads.
  `docs/modding/hot-reload.md` gains its row. **State the ratio an author can
  predict from**: under the leading candidate `rise/horizontal = (ascent −
  0.5)/4.5`, free of the resistance entirely — 1.89 today, 0.667 at the shipped
  values. It is the thing an author most needs and cannot derive from the field
  table.
- **Player** — `docs/user/gameplay.md:82–92` states the old feel in numbers that
  all become false. Re-derive both. `:85` promises *"reaching the surface from
  the deepest part of the sea in about a third of a second"*: under the retune
  that is **0.5 s** to `y = 34.0` and **~1.0 s** to breach at `y = 35.0`, and
  the existing sentence conflates them — pick which the player is told.
- **Player, second repair** — `:352` reads *"The game has three times changed how
  it records what a block is: once for what a block looks like, and twice for
  what it does"*, and *"A save old enough to cross both behaviour changes is told
  once for each"*. Four and three; *"both"* becomes a count. `:113` likewise
  promises a report *"once for this change"*.
- **Engine reader** — `physics.rs`'s module doc states the launch order; whatever
  replaces `JUMP_SPEED` on the buoyant path belongs there.

## The revision consequences, and why both are mandatory

- **`BEHAVIOUR_REVISION` 3 → 4** (`persistence/format.rs:309`). A declared
  behaviour property folds into the behaviour hash; `APPEARANCE_REVISION` stays
  at 3. **That it stays is guarded by the tree already** — `format_test.rs:133`
  and `save_per_face_appearance.rs:111` both hold `STATED_APPEARANCE_REVISION =
  3` and build the expected bytes by hand. FR-7.1-S3 compares two appearance
  hashes *to each other* and would stay green if both moved together, so it is
  not that guard and must not be cited as one.
- **`SCENE_REVISION` `r3` → `r4`** (`mc-render/src/capture.rs:47`) and all four
  committed golden directories re-shot. The scripted walk wades the sea, so
  declared physics is an input to the camera path. The constant's doc was
  rewritten by SPEC-022 to cover *"the declared camera path, which now includes
  the physics the script runs under"* — this is the first change to land under
  that wording. Re-shooting is what makes `terrain_goldens` unable to report
  this change, which is why FR-6 exists.

## User Stories

- As a **player**, I want to sink through water at a rate I can read as sinking,
  so that the sea is water rather than a swamp.
- As a **player**, I want swimming to carry me at a pace comparable to walking,
  so that crossing water is travel rather than a penalty.
- As a **mod author**, I want to declare how my own liquid carries a swimmer
  upward, so that a pool of tar and a pool of water differ in the thing that
  makes them feel different.
- As a **mod author**, I want a value I got wrong refused with the field named,
  so that a typo costs me a refusal I can read rather than a liquid that quietly
  behaves like water.

## Functional Requirements

**44 scenarios across 8 groups — four past the ~40 threshold, flagged rather
than trimmed, and confirmed twice.** Three of the four are FR-4.1-S9, S10 and
S11, added by the architect phase and **not optional**: `swim_ascent` is the
first medium property whose loader default (`9.0`) differs from its fold identity
(`0.0`), and without them a swimmer beside a non-swimmable block rises at `5.667`
b/s with the whole suite green. **They are three falsifiers because there are
three wrong places to put the repair, and no one of them sees all three** — S9
says the fountain does not happen, S10 says why it does not, S11 says what it
costs, and the loader placement is caught by neither but by FR-1.1-S4 and
FR-7.1-S2. The table is in `architecture.md` Decision 4. The fourth is
FR-7.1-S7, the load-time half of the appearance/behaviour
claim, carried forward from the archived spec because it is the one a player's
experience rests on: crossing this build costs one report, not two. **The merge
available if 41 is refused** is FR-7.1-S2 with FR-7.1-S3 (one fixture, two
complementary hashes); I recommend against it, because a scenario asserting that
two hashes differ and one asserting a third stays equal fail for different
defects. Thirteen scenarios were added or rewritten by the scenario audit;
FR-6.1-S7 was dropped as a pure sign test that already exists in the suite and
reddens for nothing this spec changes, and FR-1.1's two "a number registers"
cases were merged.

Every fixture stating a medium property states `solid = false` alongside it
unless the scenario says otherwise: a solid fixture is never overlapped, because
collision stops the walk first, so a test over one would measure collision and
report a clean pass.

**Where a scenario states a velocity, it states the velocity the tick ends at**,
which is after gravity and after the resistance — the only vertical quantity a
caller can read back off the state.

### FR-1 — `swim_ascent` on a declaration

- **FR-1.1**: A declaration may state `swim_ascent` as a finite number no less
  than zero, and its absence lifts a swimmer by what the player's own jump does.
  - FR-1.1-S1: WHEN a declaration states `swim_ascent = 3.5`, or the Luau
    integer `swim_ascent = 4`, THE SYSTEM SHALL register `3.5` and `4.0`
    respectively.
  - FR-1.1-S2: WHILE the player's box overlaps a block declaring
    `swimmable = true`, `move_resistance = 0.5` and **no** `swim_ascent`, and
    the player is not on the ground, WHEN a tick's intent asks to jump THE
    SYSTEM SHALL end that tick at `5.6667` blocks per second — exactly what a
    jump from the ground under that same resistance leaves at.
  - FR-1.1-S3: WHEN a declaration states `swim_ascent = 0` THE SYSTEM SHALL
    register `0.0` rather than treating zero as absent.
  - FR-1.1-S4: WHEN a declaration states `swim_ascent` and no `swimmable` THE
    SYSTEM SHALL register it and leave the block one no player can hold itself
    up in, the two being independent in both directions.

### FR-2 — A `swim_ascent` the engine cannot use is refused

- **FR-2.1**: A `swim_ascent` outside what the engine can use is refused, naming
  the field, and never coerced or ignored.
  - FR-2.1-S1: IF a declaration states `swim_ascent = -1` THEN THE SYSTEM SHALL
    refuse it, naming `swim_ascent`, and say it may not be less than zero.
  - FR-2.1-S2: IF a declaration states `swim_ascent = 0/0`, or
    `swim_ascent = math.huge`, THEN THE SYSTEM SHALL refuse it, naming
    `swim_ascent`, and say it must be a finite number.
  - FR-2.1-S3: IF a declaration states `swim_ascent = true` THEN THE SYSTEM
    SHALL refuse it, naming `swim_ascent`, and say it must be a number while
    naming the kind it is.
  - FR-2.1-S4: IF a declaration states `swim_ascent = "3.5"` THEN THE SYSTEM
    SHALL refuse it, naming `swim_ascent`, and say it must be a number while
    naming the kind it is, so that text Luau would coerce in an arithmetic
    context is refused rather than parsed.
  - FR-2.1-S5: IF a declaration states both a refusable `swim_ascent` and a
    refusable `move_resistance` THEN THE SYSTEM SHALL refuse the declaration
    naming one field, and never register the block.

### FR-3 — What a declaration may state, as an author reads it

- **FR-3.1**: The recognised-field list an unrecognised field is quoted back
  against holds every property this model adds, and the documentation agrees.
  - FR-3.1-S1: WHEN a declaration states a field the loader has no meaning for
    THE SYSTEM SHALL quote back a recognised-field list holding `swim_ascent` in
    the position the documentation introduces it.
  - FR-3.1-S2: THE SYSTEM SHALL state the same field list, whole and in the same
    order, in every quotation of that refusal under `docs/modding/` and in the
    field table of the block declaration page.

### FR-4 — What a swim ascent does to a rise

- **FR-4.1**: While a swimmer is held up by a block's volume and not on the
  ground, a jump request sets its upward speed from that block's declared
  ascent rather than from the player's own jump.
  - FR-4.1-S1: WHILE the player's box overlaps a block declaring
    `swimmable = true`, `swim_ascent = 3.5` and `move_resistance = 0.5`, and the
    player is not on the ground, WHEN a tick's intent asks to jump THE SYSTEM
    SHALL end that tick at `2.0` blocks per second.
  - FR-4.1-S2: WHILE the player's box overlaps a block declaring
    `swimmable = true`, `swim_ascent = 0.5` and no `move_resistance`, and the
    player is not on the ground, WHEN a tick's intent asks to jump THE SYSTEM
    SHALL end that tick at `0.0` blocks per second, so that a declared ascent of
    exactly one tick of gravity holds a swimmer's depth.
  - FR-4.1-S3: WHILE the player's box overlaps a block declaring
    `swimmable = true`, `swim_ascent = 0.0` and `move_resistance = 0.5`, the
    player is not on the ground, and it begins the tick at `−1.0` blocks per
    second, WHEN a tick's intent asks to jump THE SYSTEM SHALL end that tick at
    `−0.3333` blocks per second, where an identical tick asking for no jump ends
    at `−1.0`: a declared ascent of zero arrests a sink without reversing it.
  - FR-4.1-S4: WHILE the player's box overlaps a block declaring
    `swimmable = false` and `swim_ascent = 9000.0`, the player is not on the
    ground, and it begins the tick at `0.0` blocks per second, WHEN a tick's
    intent asks to jump THE SYSTEM SHALL end that tick at `−0.5` blocks per
    second: a declared ascent lifts nobody the volume does not hold up.
  - FR-4.1-S5: WHILE the player's box overlaps two blocks both declaring
    `swimmable = true` and `move_resistance = 0.5`, one declaring
    `swim_ascent = 3.5` and the other `swim_ascent = 1.5`, and the player is not
    on the ground, WHEN a tick's intent asks to jump THE SYSTEM SHALL end that
    tick at `2.0` blocks per second — the greater of the two, matching the rule
    an overlapped pair's resistance already follows.
  - FR-4.1-S6: WHILE the player's box overlaps a swimmable block, WHEN a tick's
    intent asks for no jump THE SYSTEM SHALL end that tick at the same vertical
    velocity as an identical tick over a block differing only in its declared
    ascent, so that a sink is governed by resistance alone.
  - FR-4.1-S7: WHILE the player's box overlaps a block declaring
    `swimmable = true`, `swim_ascent = 9000.0` and no `move_resistance`, the
    player is not on the ground, and nothing stands within one block above it,
    WHEN a tick's intent asks to jump THE SYSTEM SHALL raise its feet by exactly
    `1.0` block — the tick's displacement bound, not the `149.99` blocks the
    declared ascent asks for.
  - FR-4.1-S8: WHILE the player's box overlaps a block declaring
    `swimmable = true`, `swim_ascent = 3.5` and `move_resistance = 0.5` in one
    voxel row and cells holding no block at all in the other, and the player is
    not on the ground, WHEN a tick's intent asks to jump THE SYSTEM SHALL end
    that tick at `2.0` blocks per second, so that an empty cell contributes no
    ascent to the fold.
  - FR-4.1-S9: WHILE the player's box overlaps a block declaring
    `swimmable = true`, `swim_ascent = 3.5` and `move_resistance = 0.5`, and in
    another cell a block declaring `solid = false`, `swimmable = false` and
    `swim_ascent = 9000.0`, and the player is not on the ground, WHEN a tick's
    intent asks to jump THE SYSTEM SHALL end that tick at `2.0` blocks per
    second, so that a volume holding nobody up contributes no lift to a volume
    that does.
  - FR-4.1-S10: WHEN a declaration states `solid = false`, `swimmable = false`
    and `swim_ascent = 9000.0` THE SYSTEM SHALL resolve its voxels to the same
    medium as a cell holding no block at all, so that a volume holding nobody up
    is indistinguishable from nothing at all in what it does to a swimmer.
  - FR-4.1-S11: THE SYSTEM SHALL resolve the blocks it ships to a medium index
    exactly one bit wide, so that adding a declared medium property costs a voxel
    nothing and any future widening is reported rather than absorbed.

### FR-5 — Ground beats medium

- **FR-5.1**: A jump made from ground contact leaves at the player's own jump
  speed even when the box is submerged; a declared ascent governs only a jump
  the medium alone admits.
  - FR-5.1-S1: WHILE the player is on the ground and its box overlaps a block
    declaring `swimmable = true`, `swim_ascent = 3.5` and
    `move_resistance = 0.5`, WHEN a tick's intent asks to jump THE SYSTEM SHALL
    end that tick at `5.6667` blocks per second.
  - FR-5.1-S2: WHILE the player is on the ground in air, WHEN a tick's intent
    asks to jump THE SYSTEM SHALL end that tick at `8.5` blocks per second,
    unchanged by this feature.
  - FR-5.1-S3: WHILE the player's box overlaps a block declaring `solid = false`,
    `swimmable = false` and no `move_resistance`, the player is not on the
    ground, and it begins the tick falling at `−2.0` blocks per second, WHEN a
    tick's intent asks to jump THE SYSTEM SHALL end that tick at `−2.5` blocks
    per second, exactly the same vertical velocity as an identical tick whose
    intent asked for no jump, so that a jump asked for in mid-air neither lifts a
    falling player nor arrests its fall. **The declaration is stated rather than
    left as "no swimmable block" because the absolute is only `−2.5` at zero
    resistance**; a resistant non-swimmable fixture would satisfy the older
    wording, divide by `1 + r`, and end the tick somewhere else entirely — and
    five such blocks already exist in `support/medium.rs`. **`−2.5` is asserted
    by equality and not by a tolerance**, which pinning the resistance is what
    buys: `slowed` divides by `1 + r`, and at `r = 0` that is bit-exact identity
    — `v / 1.0` is `v` for every finite `v` in IEEE-754, as its own doc states.
    Relaxing this to a tolerance would give back the exactness the stated
    declaration was chosen to earn.
  - FR-5.1-S4: WHILE the player is on the ground and its box overlaps a block
    declaring `swimmable = true`, `swim_ascent = 0.0` and
    `move_resistance = 0.5`, WHEN a tick's intent asks to jump THE SYSTEM SHALL
    end that tick at `5.6667` blocks per second, an ascent of zero
    notwithstanding.

### FR-6 — The shipped water, in absolute blocks and ticks

**No threshold this group asserts against is arithmetic over a declared value.**
That is the precise claim and it is narrower than "nothing here touches the
registry" in two ways, and the difference is exactly what the predecessor group
got wrong. S1 *states* the declaration rather than reading one. S5 and S6 ask the
resolved world where its lakebed is, which is geometry.

**And the existing `Sea` fixture reaches the lakebed on a wait of
`2 × 360 × resistance + THROUGH_OPEN_AIR` ticks (`sea.rs`, `watch_for`), which
*is* arithmetic over the number under test.** That is legitimate in a settle — a
wait that scales with the resistance is the correct wait — but it must not become
an assertion. **A watch length may be derived; a threshold may not.**

**S2 and S3 only cage the values together, and S2 is the one an implementer would
cut.** S3's `2.0` blocks is one equation in two unknowns — `(a, r)` of `(4.5,
1.0)` and `(2.5, 0)` both give exactly 2.0 — so S3 alone cages nothing. S2 pins
`r` through the sink, and only then does S3 pin the ascent. Neither is redundant.

**S2 to S4 use a chamber; S5 and S6 use the generated sea.** Both are needed and
neither substitutes: the generated sea is two voxels deep, which is less water
than a one-second rise crosses, so a rate has nowhere to happen in it — and a
chamber says nothing about whether the sea a player meets is swimmable at all.
**The chamber's stated minimum was wrong and the scenarios were not**, so it is
derived here rather than restated. The player's box is `0.6` wide and `1.8` tall
(`player/collide.rs`), S2 sinks `0.9667`, S3 raises the feet `2.0` and S4 walks
`3.0`. Keeping a block of water between the box and every solid *throughout* each
of those therefore demands **two blocks under the feet** (`1.0 + 0.9667`), **three
above the box** (`1.0 + 2.0`), and a walk axis spanning at least **`5.6`**
(`1.0 + 0.3 + 3.0 + 0.3 + 1.0`): **seven voxels deep and six wide on each
horizontal axis, minimum.** The four-and-four this paragraph used to state cannot
carry its own scenarios — four wide leaves `1.4` blocks of travel and S4 walks
into a wall, and one block of ceiling clearance is consumed by S3's own rise. The
built fixture is twelve deep and twenty wide, which is margin over the
derivation and not the derivation.

- **FR-6.1**: The shipped water sinks, lifts and carries a player at stated
  rates, and the generated sea is deep enough to swim in.
  - FR-6.1-S1: THE SYSTEM SHALL ship `base:water` declaring `swimmable = true`,
    `move_resistance = 0.5` and `swim_ascent = 3.5`.
  - FR-6.1-S2: WHEN a player at rest and clear of the floor in such a chamber
    asks for nothing for 60 ticks THE SYSTEM SHALL lower its feet by `0.9667`
    blocks, within `1e-3`.
  - FR-6.1-S3: WHILE a player is submerged in such a chamber, not on the ground
    and clear of its ceiling, WHEN it holds jump for 60 ticks THE SYSTEM SHALL
    raise its feet by `2.0` blocks, within `1e-3`.
  - FR-6.1-S4: WHILE a player is submerged in such a chamber and standing on its
    floor, WHEN it walks at full deflection for 60 ticks THE SYSTEM SHALL carry
    it `3.0` blocks, within `1e-3`.
  - FR-6.1-S5: THE SYSTEM SHALL generate a sea whose deepest column stands
    exactly two water voxels over its lakebed, with that lakebed's top face at
    `y = 33.0`.
  - FR-6.1-S6: WHEN a player standing on the lakebed of that column holds jump
    THE SYSTEM SHALL raise its feet to `y ≥ 34.0` on a tick no earlier than the
    25th and no later than the 45th.
  - FR-6.1-S7: WHEN a player floating at the surface of that column stops asking
    to jump THE SYSTEM SHALL lower its feet back onto the lakebed within 150
    ticks.
  - FR-6.1-S8: WHILE a player holds jump in that column for 600 ticks THE SYSTEM
    SHALL end every tick of that hold with its feet below `35.1` blocks, so that
    a swimmer surfaces rather than being expelled.

### FR-7 — What a save records

- **FR-7.1**: Every property this model adds is folded into a block's declared
  behaviour, never its appearance, and the behaviour record moves to revision 4.
  - FR-7.1-S1: THE SYSTEM SHALL fold a block's declared behaviour over a byte
    sequence stated whole by hand, beginning with the byte `4` and ending with
    the bytes of `swim_ascent`.
  - FR-7.1-S2: WHEN two definitions differ only in `swim_ascent` THE SYSTEM
    SHALL record different behaviour hashes for them.
  - FR-7.1-S3: WHEN two definitions differ only in `swim_ascent` THE SYSTEM
    SHALL record the *same* appearance hash for them.
  - FR-7.1-S4: WHEN a world saved under behaviour revision 3 is loaded while
    changed blocks are accepted THE SYSTEM SHALL report every block it holds as
    behaving differently and load the world anyway.
  - FR-7.1-S5: IF a world saved under behaviour revision 3 is loaded while only
    unchanged blocks are accepted THEN THE SYSTEM SHALL refuse the load and name
    every block whose behaviour changed.
  - FR-7.1-S6: THE SYSTEM SHALL name the committed golden frames under a scene
    revision distinct from `r3`, so that frames shot before this change and
    frames shot after it cannot be compared as if they described one contract.
  - FR-7.1-S7: WHEN a world saved under appearance revision 3 is loaded THE
    SYSTEM SHALL judge no block it holds to have changed appearance, while
    judging every one of them to have changed behaviour, so that crossing this
    build costs a player one report and not two.

### FR-8 — Editing a medium while the game runs

- **FR-8.1**: Every property this model adds is hot-reloadable, takes effect on
  the next tick, and re-meshes nothing.
  - FR-8.1-S1: WHEN a reload changes `swim_ascent` from `3.5` to `1.5` on a
    block a player is currently held up by THE SYSTEM SHALL end the next tick's
    held jump at `0.6667` blocks per second, where the tick before it ended at
    `2.0`.
  - FR-8.1-S2: WHEN a reload changes only `swim_ascent` THE SYSTEM SHALL mark no
    section for re-meshing.
  - FR-8.1-S3: IF a reload's declaration states a `swim_ascent` that would be
    refused THEN THE SYSTEM SHALL refuse the reload whole, name the field, and go
    on serving the content it was already serving.

## Architecture Delta

**Not `none`, and the model itself was the decision.** **Settled by the architect
phase: Candidate A.** The `[model-dependent]` FR groups are restated against it —
confirmed as written, plus FR-4.1-S9 and FR-4.1-S10. The candidate table below is
kept as the record of what was weighed; `architecture.md` carries the
measurements, the rejections, and three further bindings (the fold rule and its
identity, the `launched` signature, and the tick-rate binding).

| # | Model | Declares | Frees | Locks together |
|---|---|---|---|---|
| **A** | **Declared ascent** *(leading)* | `swim_ascent` | rise from `JUMP_SPEED` | sink ↔ swim, both via `r` |
| B | Drag split by motion source *(the steering's literal form)* | `r_g`, `r_i` | sink from swim | **swim ↔ rise at 1.889** |
| C | Fluid gravity | `fluid_gravity` | sink from `GRAVITY` | swim ↔ rise, nearly as B |
| D | Force + drag on both axes *(Minecraft's shape)* | drag, fluid gravity, stroke acceleration | all three | nothing |

**B is measured not to solve the reported complaint** — see the diagnosis above:
it leaves a 3.0 b/s swim forcing a 5.667 b/s rise, or a 2.0 b/s rise forcing a
1.06 b/s swim. **A is the leading candidate** because it is the smallest model
that reaches every target, and because A + B together is D's freedom at D's cost.

**D is what the survey recommends**, on a ground that is an invariant-1 concern
rather than a taste one: two mods each declaring a swim *speed* for one fluid
have no meaningful merge, whereas two declaring *drag* do. Its cost is real and
belongs in the decision — horizontal motion must become an acceleration, which
changes land movement or forces a branch.

**That cost is what Out of Scope's control-law item is about, and the two must
not be read as one.** D is a live candidate here; the item there scopes out
*fixing coasting* as a repair. Whichever of the two a reader lands on first, the
resolution is the same: Out of Scope binds what gets built and does not veto a
candidate named on this ballot. **D was rejected on the merits** — its third
degree of freedom buys motion inside a range A already spans, and A's remaining
sink↔swim lock is slack across the whole surveyed band. `architecture.md`
Decision 1 carries the measurement.

Three further bindings, whichever model wins:

1. **A per-voxel value on the tick path, per property.** `VoxelMedium` carries
   `swimmable` and `resistance` today; each added property is read by the same
   fold over the same box, one line apart — the stated reason `Medium` is one
   trait returning one value (`crates/mc-sim/CLAUDE.md`). The greatest-value rule
   for an overlapped pair extends to it, and **an empty cell contributes the
   inert value and never the declaration default** (FR-4.1-S8).
2. **The numeric contract is inherited, not invented.**
   `number::optional_number_at_least_zero` already carries the whole refusal
   vocabulary, so no model adds validation vocabulary — only fields.
3. **Tick-rate binding, which is currently undecided and unwritten.** Every
   coefficient is applied per tick at 60 Hz, so the same nominal value is three
   times harsher than at Minecraft's 20 Hz and **every water speed moves if the
   tick rate ever changes**. Either express the damping as a rate raised to `dt`,
   or state in `docs/` that it is tick-rate-bound and why that is acceptable.
   Not leaving it undecided is the requirement.

The `Solidity` / `Targetable` / `Medium` split is untouched, and nothing here
gives a collision site access to a question it must not ask.

## Technical Considerations

- **A declared ascent is a launch speed, not an observed rise.** A tick sets
  `vy`, then gravity takes `0.5`, then the resistance divides — the path
  `JUMP_SPEED` takes today. The observed rise is `(ascent − 0.5)/(1 + r)` and the
  documentation states that formula rather than leaving an author to find the
  offset. Declaring the *observed* rise instead was refused: it would make one
  field's meaning depend on another field's value.
- **The absent-ascent default is a stated constant of `9.0` in `mc-world`, and
  its relationship to `JUMP_SPEED` is asserted behaviourally or not at all.**
  `JUMP_SPEED` is private to `mc-sim` and `mc-world` does not depend on `mc-sim`,
  so a loader default cannot read it; promoting a player-physics constant into
  `mc-core` to serve a coincidence of values was refused as standing
  architecture. FR-1.1-S2 pins the claim through public API from `mc-sim`, which
  sees both crates — it reddens if either constant moves and needs no visibility
  change. A prose note saying "carry this by hand" would be a claim nothing
  checks.
- **The upper bound is `move_resistance`'s, and the displacement clamp is why
  that is safe.** A tick's displacement is clamped to one block per axis
  (`bounded`, `physics.rs:199`); `fallen` (:263) is a `.max`, so it clamps only
  the downward side and the displacement bound is the whole of the upward guard.
  **FR-4.1-S7 is its witness and FR-4.1-S4 is not** — S4's fixture declares
  `swimmable = false`, so the ascent is never read and the clamp never
  approached. Both scenarios exist; one of them is about the clamp.
- **An empty cell is not a block with an absent field.** An absent declaration
  field means the player's own jump; a cell holding no block must contribute
  nothing. Conflate them and a player whose box is half out of the water rises at
  the jump speed — visible only at the surface, which is where a swimmer spends
  its time. `crates/mc-sim/CLAUDE.md` records the same hazard for the two
  existing medium properties. FR-4.1-S8 pins it.
- **FR-6's tolerances must be measured, not chosen.** The rise and the walk are
  exact in real arithmetic — the velocity is *set* each tick — but the position
  accumulates 60 additions of `2.0/60`, which is not exact in `f32`. `1e-3` is a
  ceiling; the test author derives the accumulation error and confirms it sits
  below the smallest difference the test must catch. `testing.md` §2 records what
  an over-tight assertion on an arithmetic path costs.
- **FR-6.1-S2's figure is a sum, not a terminal speed.** Sinking from rest
  approaches `1.0` b/s geometrically at ratio `2/3`, so 60 ticks cover
  `(1/60)·[60 − 2(1 − (2/3)^60)]` = `0.9667` blocks, not `1.0`. The `(2/3)^60`
  term is `2.7e-11` and vanishes; the `−2` does not. The terminal shortcut gives
  `1.0` and is wrong by 3.4%, which is 33× the stated tolerance.
- **FR-6.1-S6 is a two-sided band because a one-sided budget cannot see the
  feature missing.** Leave the ascent unimplemented and the `9.0` default gives
  `5.667` b/s, crossing the block in 11 ticks — under any upper-only budget. The
  derived crossing is 29 ticks (one from the ground at `5.6667` b/s, the rest at
  `2.0`); `25` and `45` bracket it.
- **FR-6.1-S8's `35.1` is a ceiling with margin, not the apex.** A swimmer stops
  being buoyant once its feet clear `35.0`, then coasts ballistically: the
  continuous apex above the surface is `v²/2g` = `0.0667` and one tick's travel
  at `v = 2.0` is `0.0333`, so `35.0 + 0.1` bounds a discrete arc whose peak
  is **measured at `35.0778`**, with the worst phase of the crossing tick at
  **`35.0833`** — a margin of `0.017`, not the `0.05` the continuous estimate
  reads as. The bound holds and it rejects the pre-retune `35.206` comfortably,
  but nobody should take that prose as saying there is room: measured, moving the
  ascent to `4.0` — a further **14%** — already breaches it, and it is one of only
  three tests that a change to this field reddens at all. It is the archived scenario's own form re-evaluated at the
  new rise, and it tightens as the rise falls — which is the point, since a
  swimmer expelled from the sea is exactly what a too-fast ascent looks like.
- **FR-6.1-S7 is blunt and S2 is the sharp instrument.** Two blocks of sink from
  rest is **122 ticks exactly in real arithmetic, and 123 as the engine actually
  accumulates it** — and the gap is not a counting convention. The closed form
  `(1/60)·[−n + 2(1 − (2/3)ⁿ)]` gives `d(121) = −1.98333`, `d(122) = −2.0` to
  within `1.1e−23`, and `d(123) = −2.01667`; so 122 is the analytic answer and
  it lands on the threshold rather than past it. The tick path accumulates 122
  `f32` additions, which leaves the sum a hair *short* of two blocks, and the
  lakebed is reached on **123**. A bound derived from 122 and asserted tightly
  would be off by one against the shipped code. The 150-tick budget is what
  makes that harmless here, and it tolerates roughly a 20% move in `r`.
  What catches that is S2's `1e-3` band. Each declared value has one sharp
  witness — `move_resistance` has S2, the ascent has S3 — and the budgets say
  only that the sea a player meets is still crossable. **Both figures describe a
  sink beginning inside the water and the fixture's does not**: it releases a
  player floating at about `35.03`, part of whose fall is above the water line and
  unresisted, and reaches the lakebed on **122**. Far enough inside the 150-tick
  budget that nothing moves.
- **`sea.rs` cites six scenario IDs that this spec re-points at different
  scenarios, and retargeting them is a deliverable rather than a note.** Verified
  by reading the file: `FR-6.1-S6` at **:43** and **:178**, `FR-6.1-S4` at
  **:61**, **:82**, **:329** and — worst — inside the panic message at **:336**,
  which spells out *"FR-6.1-S4's budget is one and a half times `120 × depth ×
  resistance`"*. That is the **archived** S4, a sink budget derived from the
  declared value; **this** spec's S4 is the chamber walk of 3.0 blocks, and the
  archived phrasing is an instance of the very derived-threshold shape FR-6
  exists to reverse. A test author implementing this spec's FR-6 reads that
  message inside the fixture FR-6 leans on hardest. Retarget all six.
- **The existing sea tests are kept, not retired, and the spec says so rather
  than leaving the test author to guess.** `the_shipped_sea_can_be_swum_in.rs`
  states its thresholds as arithmetic over the registry, so every one of them
  still passes after the retune — they are blind to this change, not wrong by it.
  They keep their value as sign tests that the sea is swimmable at all. **What
  they must not do is be counted as evidence that the new values landed**; FR-6's
  S2 and S3 are the only things that report that.
- **`SHORE_COLUMN` and `Sea::shore_player()` are not orphaned by dropping the
  archived FR-6.1-S6**, contrary to a note raised against this spec. Verified:
  both have live callers at `the_shipped_sea_can_be_swum_in.rs:238` and `:262`.
  Nothing is deleted here.

## Existing Code to Leverage

| What | Location | Reuse |
|------|----------|-------|
| Numeric field reader and its refusals | `crates/mc-world/src/content/luau_declaration/number.rs` | `optional_number_at_least_zero` whole — no new validation vocabulary |
| Recognised-field list and its refusal | `.../luau_declaration/mod.rs:79` | one constant per added property, in documented order |
| Field-list mirror guard | `crates/mc-client/tests/documented_declaration_fields.rs` | reads the list whole and in order; reddens until the docs move |
| Open-water fixture | `crates/mc-sim/tests/support/chamber.rs` | FR-4, FR-5 and FR-6.1-S2..S4 |
| Shipped-sea fixture | `crates/mc-sim/tests/support/sea.rs` | FR-6.1-S5..S8; its derived-threshold helpers are what FR-6 must not assert against |
| Appearance-revision guards | `format_test.rs:133`, `save_per_face_appearance.rs:111` | already pin `APPEARANCE_REVISION = 3` by hand |
| Behaviour/appearance fold | `crates/mc-world/src/persistence/format.rs` | revision move, hand-stated byte sequence |
| Golden re-shoot procedure | `docs/technical/rendering.md`, `terrain_goldens.rs` | named binary, mint opt-in |

## Out of Scope

Binding. Recorded, not built.

- **A descend input.** Decided, not deferred. At the shipped values a player
  descends a block per second — faster than Minecraft's deliberate post-Aquatic
  sink — so a key to descend faster answers no complaint that was made. A
  descend control is *normal* in the genre (Minecraft 1.21 ships
  `goDownInWater()` at `−0.04` on sneak; Luanti has had sneak-descend for years),
  which is evidence that it is available rather than that it is needed here. It
  is also a new `MovementIntent` field, hence a wire-format change in `mc-proto`
  and an authority question of its own.
- **The residual drag rate and the control-law asymmetry.** Both measured above,
  both left alone. Fixing coasting means changing how horizontal motion is
  integrated, which is a larger change than one medium's properties.

  **This item scopes out the *repair*, and does not veto candidate D.** The
  sentence about integrating horizontal motion is this item's justification, not
  a prohibition on a model — and D is put on the architect's ballot by name in
  the Architecture Delta below, with its cost stated there. A spec cannot both
  offer a candidate and veto it, and reading it as a veto is how the architect
  declines the argument it was convened to make. **Out of Scope binds what gets
  built; it does not dispose of a candidate this document asks to be weighed.**
  D was in fact rejected, on the merits and not on scope — `architecture.md`
  Decision 1.
- **A separate horizontal swim speed** under model A. Sink and swim stay locked
  via `move_resistance`, landing at two thirds of a walk — inside the 45–80%
  range shipped games occupy.
- Currents, oxygen, drowning, damage, swim animation, buoyant items, boats, fluid
  flow, and water levels below a full voxel.
- Any change to what `swimmable` or `move_resistance` *mean*. This spec changes
  water's declared resistance value and adds to the surface; it redefines
  nothing.
- Retuning `WALK_SPEED`, `JUMP_SPEED`, `GRAVITY` or `TICK_DURATION`. The repair
  is content-side, and FR-5.1-S2 pins one engine constant as a control.

## Dependencies

- None external. Everything this touches is in the workspace.

## Assumptions

- The shipped values in FR-6.1-S1 produce a sea that plays as water. Assumed, not
  verified — only play settles feel, which is why this spec exists and why FR-6
  pins the observables absolutely.
- The scripted replay's walk still wades the sea after the retune. If the faster
  sink moves where it ends up, the re-shot goldens record that; the scene
  contract's quad counts are unaffected either way.

## Open Questions

None blocking. The model choice is an architectural decision with a stated
leading candidate, four measured alternatives and a stated deciding ground — it
belongs to the architect phase by design, not to this one.
