# Architecture Delta — PRO-992 · Water carries a swimmer at rates content declares

Designs only what `spec.md`'s `## Architecture Delta` declares. Project
constants — the crate map, the `Solidity`/`Targetable`/`Medium` split, the
composition root, the tick model — come from `docs/technical/architecture.md`
and are not re-derived here.

**Every figure below was re-simulated from `crates/mc-sim/src/player/physics.rs`
by transcribing `advance_player`'s launch → gravity → resistance → clamp order
tick by tick, rather than by re-evaluating the closed forms the spec states.**
That is deliberate: `requirements.md` §4(b) records a check that reproduced a
wrong numerator from itself. The simulator and the closed forms agree on all
seventeen figures the spec asserts, including FR-6.1-S2's `0.9667` blocks, which
a terminal-velocity shortcut gets wrong by 3.4%.

**One figure disagreed, the first draft called it a counting convention, and that
was wrong.** FR-6.1-S7's two-block sink is **122 ticks in exact arithmetic and
123 as the engine accumulates it**. Re-derived in exact rationals: `d(121) =
−1.98333`, `d(122) = −2.0` to within `1.1e−23`, `d(123) = −2.01667`. So 122 is
analytically right and lands *on* the threshold rather than past it — and 122
`f32` additions leave the running sum a hair short, so the lakebed is reached on
123. Not a convention: **an accumulation error the shipped code has too**, which
means a bound derived from 122 and asserted tightly would be off by one against
the real thing. `spec.md` now states both figures and which is which. The
150-tick budget makes it harmless *here*, and that is what makes it worth writing
down rather than what makes it correct.

---

## Drivers

| # | Driver | Source |
|---|---|---|
| D1 | The rise must become independently tunable; it is `(JUMP_SPEED − g·dt)/(1+r)` and no damping coefficient reaches past an engine constant | `requirements.md` §5 |
| D2 | Whatever is added is **content-declared**; zero hardcoded water in Rust | MyCraft invariant 1 |
| D3 | Whatever is added folds over overlapped voxels with a defensible rule **and a defensible identity** | `spec.md` Architecture Delta ¶1; FR-4.1-S5, FR-4.1-S8 |
| D4 | The tick-rate binding is decided and written, either way | `spec.md` Architecture Delta ¶3 |
| D5 | Land movement is a surface nobody has complained about | conductor brief |
| D6 | The observable rates in FR-6 are settled and hold whichever model wins | `spec.md` "What is settled here" |

---

## Decision 1 — the model · **BINDING: Candidate A, a declared ascent**

`swim_ascent` replaces `JUMP_SPEED` on the buoyant branch of `launched`, and
nowhere else. The observable rise becomes `(ascent − g·dt)/(1 + r)`.

### What decided it

Each candidate was measured against the assumed target (sink 1.0, rise 2.0,
swim 3.0 b/s) and against the surveyed swim band of 45–80 % of walking speed.
Two parameters, three observables, in every candidate except D.

| | reaches the target? | the coupling it keeps | is that coupling slack at the target? |
|---|---|---|---|
| **A** `(r, ascent)` | **yes**, exactly: `r = 0.5`, `ascent = 3.5` | sink ↔ swim, via `r` | **yes — slack across the whole surveyed band** |
| B `(r_g, r_i)` | no | swim ↔ rise, pinned at `8.5/4.5` | no — violated at the target and beyond it |
| C `(g_f, r)` | no | swim ↔ rise | no — violated worse than B |
| D force+drag, both axes | yes | none | n/a |

**A's remaining coupling is the one that does not bind.** Under A, `swim` and
`sink` are locked as `swim = 4.5·sink/(sink + 0.5)`. Solving that for the edges
of the surveyed band: a swim at 45 % of walk requires a sink of `0.4091` b/s
(2.444 s/block), and a swim at 80 % requires `2.0` b/s (0.500 s/block). **The
chosen sink of 1.0 b/s sits between them**, so the lock can be moved anywhere
inside the surveyed range without either observable leaving it. That is the
positive case for A, and it is stronger than "A is the smallest model".

**B's and C's remaining coupling is violated at the target.** Re-measured here
rather than relayed:

- B, swim `3.0` ⇒ `r_i = 0.5` ⇒ rise **5.6667** b/s — above `WALK_SPEED`. Fountain.
- B, rise `2.0` ⇒ `r_i = 3.25` ⇒ swim **1.0588** b/s = **23.5 %** of walk, worse
  than the 38.5 % complained of.
- B at 80 % of walk (Source's figure) ⇒ rise **6.80** b/s. B cannot reach three
  of the four surveyed swim fractions without a rise above walking speed.
- **C is worse than the spec's "nearly as B" allowed.** Solving C for sink `1.0`
  and rise `2.0` gives `g_f = 140`, `r = 2.333`, and a swim of **1.35 b/s = 30 %
  of walk** — below today's 38.5 %. Solving C for sink `1.0` and swim `3.0`
  instead forces `g_f = GRAVITY` exactly, i.e. C degenerates to today's model and
  returns the 5.667 b/s fountain. C buys nothing at any operating point.

**B is not implemented, and is not dropped.** It is the steering on this spec, it
is the right instinct about where the freedom is missing, and the arithmetic
reason it fails — the freedom comes from splitting the *source term*, not the
damping — is recorded in `spec.md` §"Why the owner's hypothesis, taken literally,
does not buy it" and stays there.

### D is the closest rejection, and why it is rejected

D is what the survey recommends and the only candidate that beats A on
capability. Three grounds, on the merits:

1. **It buys a third degree of freedom the targets do not need.** A's remaining
   lock is slack across the whole surveyed band, measured above: every swim
   fraction from 45 % to 80 % of walking speed is reachable under A by moving one
   coefficient. D's extra freedom purchases movement inside a range A already
   spans.
2. **The survey's composability ground does not separate A from D.** `swim_ascent`
   is a *source term* — a launch impulse, the same category as Minecraft's own
   per-fluid upward delta — and not a swim *speed*. The only merge this engine
   performs is the per-voxel fold, and a join with an identity composes exactly as
   well for a source term as for a drag (Decision 3). Two mods declaring different
   values for one block id is a *definition* conflict, which drag has identically
   and which this engine settles by last-declaration-wins, not by merging.
   Stated as the survey would: **its objection is to a declared speed that
   carries no merge rule at all, and A ships one** — `max`, the same join
   `move_resistance` already ships, with the same identity and the same
   associativity. D is not more mergeable than that; it is mergeable in exactly
   the same way.
3. **It changes land movement, against D5** — see the table below — and the owner
   has complained about water, not walking.

**A note on Out of Scope, because the spec was in tension with itself here and is
no longer.** `spec.md`'s Out of Scope reads *"Fixing coasting means changing how
horizontal motion is integrated, which is a larger change than one medium's
properties"*, which reads as D's horizontal half excluded — while the Architecture
Delta puts D on the ballot as a candidate to be chosen. Both could not hold.

**The resolution: Out of Scope binds what gets *built*; it does not dispose of a
candidate the same document asks the architect to weigh.** The sentence about
integrating horizontal motion is that item's *justification for scoping out the
repair*, not a veto on a model. A spec that both offers a candidate and vetoes it
lets the architect decline the argument it was convened to make — so D is rejected
here on the merits above, and the scope clause is not even corroboration for that
rejection; it is a different question.

This is now repaired in `spec.md` itself, on both sides — the Out of Scope item
and the Architecture Delta's statement of D's cost each carry a cross-reference,
so a reader landing on either arrives at the same place. **Not flagged and left:
fixed.**

The branch variant — an acceleration law for swimmable media only — is also
rejected: it is two control laws inside one function for the benefit of one
medium, it still needs the acceleration path specified and tested, and
`settled()` would have to know which law ran.

### What happens to land movement, per candidate

D5 asks this explicitly, so it is answered for every candidate seriously
considered and not only for the winner.

| | land movement |
|---|---|
| **A** | **Unchanged, structurally.** Inside `physics.rs` the edit is confined to `launched`: it takes the folded `VoxelMedium` instead of a `bool`, and its buoyant arm returns the declared ascent instead of `JUMP_SPEED`. Its `on_ground` arm still returns `JUMP_SPEED`, its condition is the expression it already was, and `walk_velocity`, `slowed`, `fallen`, `bounded` and `settled` are untouched. On land the folded medium is `VoxelMedium::NOTHING`, whose ascent is never read because `swimmable` is false. FR-5.1-S2 is the control that pins it. |
| B | Unchanged — a split damping coefficient is read only where a medium resists. |
| C | Unchanged in the walk, but `fallen` would have to read the medium, giving every falling body a per-medium gravity. |
| D | **Changed.** `walk_velocity`'s stated contract — *"set rather than accumulated… no acceleration to build up and no inertia to shed"* — is exactly what D removes. Consequences reach the start-up ramp, coasting after release, air control, `settled()`'s meaning, every replay pose, the scripted camera path beyond the re-shoot this spec already owes, and client-side prediction. |

**No escalation is raised**: the chosen candidate changes land movement in no
respect. The brief's escalation condition was reaching a candidate that does.

---

## Decision 2 — the declared surface · **BINDING: one field, `swim_ascent`**

The `[model-dependent]` groups FR-1, FR-2, FR-3, FR-4, FR-5, FR-7 and FR-8 were
written against exactly this candidate and are **confirmed as written**, with one
addition (Decision 4). No field names or counts move.

| Layer | Delta |
|---|---|
| `mc-world` loader | `SWIM_ASCENT_FIELD = "swim_ascent"`; `RECOGNISED_FIELDS` grows `[&str; 11] → [&str; 12]`, appended last, after `move_resistance`, in documentation order; `declared_ascent` is one call to `number::optional_number_at_least_zero` — **no new validation vocabulary** (FR-2 is satisfied entirely by the inherited refusals, including the `-0.0 → 0.0` normalisation the fold depends on) |
| `mc-world` default | `SWIM_ASCENT_BY_DEFAULT: f32 = 9.0`, a **constant**, for the reason `MOVE_RESISTANCE_BY_DEFAULT` is one: a declaration saying nothing must move as it always did. Deliberately **not** a `defaulting_to_solidity`-style derived default |
| `mc-core` | `BlockDefinition` gains `swim_ascent: f32` |
| `mc-world` persistence | `DeclaredBehaviour` gains `swim_ascent: f32`, **appended last**; `BEHAVIOUR_REVISION` 3 → 4; `APPEARANCE_REVISION` stays 3 |
| `mc-sim` | `VoxelMedium` gains `swim_ascent: f32`; `NOTHING` gains `swim_ascent: 0.0`; `with` gains one `.max` line; `declared_by` gains one arm (Decision 3); `launched` takes the whole `VoxelMedium` |
| `mc-render` | `SCENE_REVISION` `r3` → `r4`, four golden directories re-shot |

**No new crate dependency, and no dependency-direction change.** Every edit is
inside a crate that already holds the type it touches, and the delta adds no edge
to the crate graph — so the boundaries table this phase would otherwise carry is
empty, which is stated rather than omitted.

### `launched` takes both answers, never the ascent alone

The draft reviewed by `persona-architect` moved `launched`'s third parameter from
`buoyant: bool` to the ascent value. **That is unimplementable and the review was
right to block it.** The two questions are independent, and one number cannot
carry both:

**Decision 3 is what makes the collision structural rather than incidental.**
Masking sends every non-swimmable definition to ascent `0.0`, so the two media
that reach `launched` in FR-4.1-S3 and FR-4.1-S4 carry the *same* ascent and
differ only in the field an ascent-only parameter has thrown away:

| what reaches `launched` | previous `vy` | must return | pinned by |
|---|---|---|---|
| `{swimmable: true, ascent: 0.0}` | `−1.0` | `0.0` — launch at the declared lift | FR-4.1-S3 |
| `{swimmable: false, ascent: 0.0}` *(9000.0 masked)* | `−1.0` | `−1.0` — carry the fall | FR-4.1-S4 |

An ascent-only parameter cannot tell "buoyant, and the declared lift is zero"
from "not buoyant at all", and after masking those are the only two things it is
ever handed.

**Neither FR-4.1-S3 nor FR-4.1-S4 reddens it**, which is worth stating because it
is the shape `testing.md` §2 calls a pass for the wrong reason. S4's fixture
starts at rest, so the wrong answer (`0.0`) and the right answer (its `vy`, also
`0.0`) coincide; and S3 is a buoyant case, which the wrong implementation gets
right for the ordinary reason that it launches there too.

What the wrong implementation actually breaks is elsewhere: **an ascent-only
`launched` arrests a fall the moment a falling player presses jump in mid-air.**
Someone falling off a cliff taps jump and stops dead.

**The suite already catches it, and the first statement of this finding said
otherwise.** `crates/mc-sim/tests/player_ground.rs:413`,
`a_jump_asked_for_in_mid_air_does_nothing_to_the_fall`, starts at
`AIRBORNE_SPEED = −2.0` (:131) and asserts `REFUSED_JUMP = −2.5` (:135) — so an
ascent-only `launched`, which would end that tick at `−0.5`, reddens it by two
blocks per second. The defect would **not** have shipped silently, and any claim
that it would is withdrawn. Two things follow rather than one:

- The numbers were derived here independently and land on **exactly** that test's
  constants, which is corroboration that `−2.0 → −2.5` is the natural statement
  of this case rather than a figure chosen to be convenient.
- The other mid-air test is blind to it. `player_buoyancy.rs:177` builds its start
  from `adrift()`, whose velocity is `Vec3::ZERO`, and compares *positions*
  between a jumping and a non-jumping tick — at rest both end at `−0.5` under the
  broken implementation as well as the correct one, so its `(true, false)` verdict
  is unchanged. **Single witness**, and it is in another spec's file.

### `player_buoyancy.rs:177` is strengthened by this spec, not left as a wart

**In scope by conductor ruling, and the reasoning is worth keeping.** That test is
blind to the change being made to the function beneath it — `launched` is exactly
what this spec re-signatures — and a test that cannot see the change under it is
worse than no test, because it reads as coverage. It sits inside the blast radius
rather than beside it.

**The repair is local to that test and must not touch `adrift()`.** The helper is
called at four sites — :136, :153, :180 and :201 — of which :180 is this test's,
so three others would move with any change to it. At least two of those three are
*about* a fall from rest by name: `asking_for_no_jump_inside_a_swimmable_block_still_sinks`
(:152) and `a_jump_asked_for_in_midair_inside_a_resistant_block_that_nobody_can_swim_in_still_sinks`
(:200). Give this one test a falling start in the shape the file already uses at
:77 — `PlayerState { velocity: Vec3::new(0.0, −2.0, 0.0), ..adrift() }`.

Derived, so the test author does not have to: at `−2.0` the correct
implementation ends both the jumping and the non-jumping tick at `−2.5` in
`hollow()`, so the pair stays `(true, false)`; the ascent-only implementation ends
the jumping tick at `−0.5` against the other's `−2.5`, the jumping tick therefore
ends **higher**, and the pair becomes `(true, true)`. Red. The first element is
unaffected — `BUOYANT` launches at the `9.0` default either way.

**The assertion shape does not change**, and that is deliberate: the test compares
positions rather than heights for a stated reason — *"a tick held against a block
can end higher than it began without any jump having been honoured"* — and a
falling start fixes the blindness without spending that. An absolute alongside the
pair would also work and costs more; it is the fallback, not the first choice.

Owned by the test author, like every other test-file edit this delta names.

**That scenario has been rewritten, because as originally worded it could not see
it either.** It stated the overlap and the ground condition and *nothing about
the velocity the tick begins with*, so a fixture author would have started it at
rest — and at rest the jump tick and the no-jump tick both end at `−0.5` under
the correct implementation and the broken one alike. It now states a start at
`−2.0` b/s and asserts the absolute `−2.5` **alongside** the equality: the stated
declaration pins `r` at zero, `fallen` gives `−2.5`, and `slowed` is bit-exact
identity at `r = 0` — its own doc guarantees `v / 1.0` is `v` for every finite
`v` in IEEE-754 — so the figure can be asserted by equality rather than by a
tolerance. The ascent-only implementation ends that tick at `−0.5`: two blocks
per second apart, in a scenario that previously could not distinguish them.

**Adding the absolute forced the fixture to be stated, and that is worth naming
because it nearly went in wrong.** *"Overlaps no swimmable block"* was sufficient
while the assertion was a pure equality between two ticks — that holds at any
resistance. It is **not** sufficient for an absolute: a resistant non-swimmable
block satisfies that wording too, and divides by `1 + r`, so the tick ends
nowhere near `−2.5`. The scenario now states the declaration — `solid = false`,
`swimmable = false`, no `move_resistance` — which pins the resistance at zero and
keeps the case on the resolved medium path rather than in bare air. **A
strengthened assertion can invalidate fixture wording that was adequate for the
weaker one**, and the two have to move together.

Keeping the equality *and* adding the absolute is deliberate and is the same
argument the review made about FR-7.1-S3: two quantities compared only to each
other cannot see a defect that moved both. The equality states the rule; the
absolute stops the pair agreeing for the wrong reason — and `player_buoyancy.rs`
above is that exact failure, live in the tree, comparing two positions that a
broken implementation moves together.

**S3 is a second witness, not a duplicate, and the distinction is the code path.**
`player_ground.rs`'s test reaches `launched` through a solidity-only fixture whose
`medium_at` is `NOTHING` unconditionally; FR-5.1-S3 reaches it through the
**resolved medium view**, which is the path this spec changes and the one every
other FR-4/FR-5 scenario travels. `testing.md` §2 names this shape explicitly — *a
second witness on a path with only one* — and it is recorded here rather than left
for a reviewer to challenge as re-proving what another test proves.

**This belongs in the scenario and not in this paragraph.** At `high` the test
author is a fresh context writing fixtures from scenario text; a constraint that
lives only in an architecture document is one that depends on somebody reading
the right paragraph at the right moment. The paragraph explains *why*; the
scenario is the instruction.

`launched(&state, intent, medium: VoxelMedium)` — the folded value whole, both
answers together. That is the same argument `Medium` is one trait returning one
value for (`docs/technical/architecture.md`): one site reads both properties, one
line apart, from one fold over one box.

---

## Decision 3 — the fold rule, its identity, and the one place they disagree · **BINDING**

D3, and the part of this delta that is not mechanical.

```
VoxelMedium::NOTHING.swim_ascent = 0.0                      // identity
with(a, b).swim_ascent = a.swim_ascent.max(b.swim_ascent)   // join
```

`max` is commutative, associative and idempotent, with `0.0` as its identity —
the same lattice join `resistance` already uses, and the rule FR-4.1-S5 states.
`0.0` is the identity FR-4.1-S8 requires: an empty cell contributes no lift.

### The hazard this creates, which the existing two properties do not have

**`swimmable` and `move_resistance` both have `default == identity`.** Absent
`swimmable` is `false`, the identity of `||`; absent `move_resistance` is `0.0`,
the identity of `max`. So for both, a block that declares nothing folds away.

**`swim_ascent` does not: its default is `9.0` and its identity is `0.0`.** Both
values are right for their own job — `9.0` is what keeps an existing swimmable
declaration behaving as it did, `0.0` is what keeps an empty cell inert — and
their disagreement is a defect the moment a box overlaps a swimmable cell and a
non-swimmable one at the same time:

> water `{swimmable: true, r: 0.5, ascent: 3.5}` folded with an ordinary block
> `{swimmable: false, r: 0.0, ascent: 9.0 by default}` gives
> `{swimmable: true, r: 0.5, ascent: 9.0}` — and a swimmer beside it rises at
> **5.667 b/s**, the fountain this spec exists to remove.

**This is reachable in content — not in *shipped* content today, and the
difference matters.** The first draft said "shipped" and that was wrong: this
game ships dirt, grass and stone (all solid, neither medium field stated) plus
water, so no shipped block can produce the mixed fold. What produces it is a
**non-solid, non-swimmable** block sharing a voxel with water — seaweed, kelp, a
plant growing in a lake — which is third-party content and **every fixture
registry in this tree**. Overstating it invites a reviewer to check the shipped
blocks, find nothing, and discount a hazard that is real.

A *solid* block is not the case, and the load-bearing reason is not the one the
first draft gave: it is that **placement refuses to build into the player's
box**, not that collision keeps the box out. `medium_around`
(`crates/mc-sim/src/player/collide.rs:244`) takes `&dyn Medium`, so `on_ground`
and `overlaps` are unspellable inside it and it structurally cannot reach the
block underfoot — `CLINGING_STONE` (`tests/support/medium.rs:153`) is its
witness.

**Hot reload strengthens this placement rather than threatening it.**
`World::adopt` (`crates/mc-sim/src/world/mod.rs:315`) re-resolves the whole
volume and rebuilds the table, so the mask applies automatically to reloaded
content with nothing extra to remember. And a reload that turns a non-solid block
**solid while the player's box already overlaps it** is a case nobody designed
this mask for in which the mask is the thing that stops a fountain.

### The rule

**A definition that is not swimmable contributes an ascent of `0.0` to the medium
view**, applied at `declared_by` in `crates/mc-sim/src/replay/medium.rs` — the
single production door from a `BlockDefinition` to a `VoxelMedium`:

```rust
fn declared_by(declared: &BlockDefinition) -> VoxelMedium {
    VoxelMedium {
        swimmable: declared.swimmable,
        resistance: declared.move_resistance,
        // A volume that holds nobody up lifts nobody: the ascent of a
        // non-swimmable definition is inert in the fold, so a declaration's
        // default of 9.0 cannot reach a swimmer through a neighbouring cell.
        swim_ascent: if declared.swimmable { declared.swim_ascent } else { 0.0 },
    }
}
```

**This is not the derivation `declared_by`'s doc forbids.** That doc forbids
deriving a medium from `is_solid` — a field of a *different* resolved view, which
would invent a claim no author made. Both fields read here are declared, both
belong to the medium, the registry still reports each independently (FR-1.1-S4),
and the rule states nothing more than FR-4.1-S4 already does in words: *"a
declared ascent lifts nobody the volume does not hold up."* What FR-4.1-S4 pins
for a lone block, this makes true in a fold.

**It is placed at `declared_by` and not inside `with`.** Masking inside `with`
would also work and would be associative — but it would leave the medium table
holding `{false, 0.0, 9.0}` as a distinct entry for every ordinary block, taking
the shipped table from two entries to three and the packed index from **1 bit to
2 bits, +128 KiB at world scale**.

Under the rule above, **a non-swimmable definition resolves to exactly the medium
it resolves to today** — its declared ascent is masked away and its resistance is
untouched, so the table gains no entry it did not already have. That is the
general statement; the shipped case is the strong one, because no block this game
ships except water declares either medium field, so every one of them resolves to
`VoxelMedium::NOTHING` and the table stays at two entries and one bit. FR-4.1-S11
is what turns that from a claim into a checked one.

**`medium_table_width.rs`'s module prose keeps its *number* and loses its
*count*.** The one bit survives, but the prose says *"the whole of both medium
questions costs one bit"* and there are now three — the same "both halves" repair
`crates/mc-sim/CLAUDE.md` needs, in a second place. It is in the integration
table.

**A non-swimmable *resistant* block is the case worth stating separately** — a
cobweb, say, declaring `swimmable = false, move_resistance = 2.0`. It resolves to
`{false, 2.0, 0.0}`, which is its own table entry, exactly as it would be today.
The masking costs it nothing and buys it the same protection: sharing a voxel
with water, it contributes its resistance to the fold and no lift at all. **This
is not hypothetical**: `media_registry()` already declares five such blocks —
`THICK`, `THICKER`, `SETTING`, `AWKWARD` and `CLINGING_STONE`
(`crates/mc-sim/tests/support/medium.rs:145-153`). So the two-entry claim above
rests on a premise about *shipped content* specifically, and is stated that way
rather than as a general property of the masking.

### The placement an implementer reaches for first, and why it is wrong

Not `declared_by` versus `with` — the first instinct is neither. It is to mask in
the **loader**: resolve `swim_ascent` to `0.0` whenever `swimmable` is false, at
the point the declaration is read, which deletes the `mc-sim` mask entirely and
looks strictly simpler. It is excluded, and by scenarios rather than by taste:

- **FR-1.1-S4** requires a declaration stating `swim_ascent` and no `swimmable`
  to **register it** — the two are independent in both directions, so the
  registry must report the declared number.
- **FR-7.1-S2** requires two definitions differing only in `swim_ascent` to fold
  to different behaviour hashes, which needs the declared number in the fold.

A loader mask makes both of those report `0.0` and loses the author's value
permanently. The mask belongs where the *medium view* is built, which is
`declared_by`, and nowhere earlier. This is the "simplification" most likely to
arrive in a later refactor, which is why it is written down rather than left to be
rediscovered.

**The trap to name for the implementer:** if the masking is put nowhere at all,
`medium_table_width.rs`'s two tests **both stay green** — the shipped width moves
1 → 2 bits, still under its `SHIPPED_CEILING = 2`, and the five-media control
still reports wider. The prose in that module goes silently false and the
fountain ships. The falsifier is Decision 4, not the width tests.

---

## Decision 4 — the three falsifiers this model requires and the spec did not carry · **BINDING**

FR-4.1-S8 pins water folded with an **empty** cell. Nothing pins water folded
with an **occupied non-swimmable** cell, which is the case Decision 3 is about.
Without it, Decision 3 is unfalsifiable and an implementer who omits it ships
green.

> **FR-4.1-S9**: WHILE the player's box overlaps a block declaring
> `swimmable = true`, `swim_ascent = 3.5` and `move_resistance = 0.5`, and in
> another cell a block declaring `solid = false`, `swimmable = false` and
> `swim_ascent = 9000.0`, and the player is not on the ground, WHEN a tick's
> intent asks to jump THE SYSTEM SHALL end that tick at `2.0` blocks per second,
> so that a volume holding nobody up contributes no lift to a volume that does.

Simulated: correct behaviour `2.0` b/s; the unmasked fold gives `5999.6667` b/s,
with the displacement clamped to one block for the tick. The two are not close,
which is what makes this a cheap falsifier.

### S9 alone cannot see where the masking was put, and that is why there are three

**A blocker from the review, and it is correct.** S9 asserts *physics*, and
masking inside `VoxelMedium::with` produces exactly the same physics as masking
at `declared_by` — while leaving `{false, 0.0, 9.0}` in the medium table for
every ordinary block, taking the shipped index from 1 bit to 2 and +128 KiB at
world scale. S9 stays green under it. So do both width tests, because
`SHIPPED_CEILING = 2` is a `≤` ceiling with exactly one bit of headroom, and that
headroom is what the variant spends.

**The draft rejected tightening the width test to exactly one bit; the conductor
has ruled it binding, and the ruling is right.** The draft's ground was that the
`≤ 2` ceiling is a deliberate content budget which pinning at one bit would
redden the day content legitimately declares a third distinct medium. That
ground is weaker than it looked: **reddening on a deliberate content change is
the cost becoming visible, which is the thing the budget exists to do.** A test
that silently absorbs a doubling is not protecting a budget, it is hiding a spend.

And the conductor found something sharper than either the draft or the review.
`medium_table_width.rs`'s assertion permits two bits while **its own failure
message claims one**: *"medium index fits in one bit and must not exceed
{SHIPPED_CEILING}"*. A reader of that test is told a property the test does not
check. That is this project's own *"an absence assertion cannot tell an empty
answer from a scan that can no longer look"*, one layer in — the documented claim
and the checked claim have come apart.

So, binding, and both halves are owed:

> **FR-4.1-S11**: THE SYSTEM SHALL resolve the blocks it ships to a medium index
> exactly one bit wide, so that adding a declared medium property costs a voxel
> nothing and any future widening is reported rather than absorbed.

Asserted by **equality**, not by a ceiling — an enumerated verdict in the sense
`testing.md` §2 asks for. It reddens for the `with` variant, which FR-4.1-S9
cannot see. It does **not** see the loader variant, which is why the table below
enumerates three placements rather than claiming any one scenario covers them.

**The existing ceiling test stays, and its message is a separate defect that
stands on its own.** `medium_table_width.rs` asserts `width <= SHIPPED_CEILING`
with `SHIPPED_CEILING = 2`, while its failure message reads *"medium index **fits
in one bit** and must not exceed {SHIPPED_CEILING}"*. **The message states a
property the assertion does not check** — a reader of that test is told something
false, and this project has a documented history of exactly that shape surviving
because the number printed beside it reproduces. The assertion is *not* tightened:
the `≤ 2` ceiling is a deliberate content budget and it is the right instrument
for the question it asks. What changes is the message, which must say what the
assertion enforces and **name FR-4.1-S10 as the guard that holds the one-bit
property** — S10 catches the `with` variant at its root cause (a non-lifting
declaration failing to resolve to `NOTHING`) rather than at the symptom the width
would report.

**This phase does not land that edit, and the reason is a standard rather than
reluctance.** `standards/global/testing.md` gives test files to the test author
at `medium+`, and this spec runs at `high`; an architect-authored string sitting
in a test file is precisely the ownership confusion that rule prevents, and it
would also put an unreviewed code change under a gate reading taken for a
documentation phase. It is specified here tightly enough to be executed without
re-deriving it, and it is in the integration table.

A second falsifier, aimed at the rule rather than at its cost, asserts the
resolved medium directly through the public `ResolvedVoxels::medium_index_of`:

> **FR-4.1-S10**: WHEN a declaration states `solid = false`, `swimmable = false`
> and `swim_ascent = 9000.0` THE SYSTEM SHALL resolve its voxels to the same
> medium as a cell holding no block at all, so that a volume holding nobody up is
> indistinguishable from nothing at all in what it does to a swimmer.

Red under masking-in-`with` (the definition mints a distinct table entry), red
under no masking at all, green only under Decision 3's placement.

**The three are not redundant, and each is the only one that sees its own
defect.** Written out, because "add a test per scenario" would not have produced
this set:

| variant an implementer might ship | S9 | S10 | S11 | other |
|---|---|---|---|---|
| no masking anywhere | **red** | **red** | **red** | — |
| masking inside `with` | green | **red** | **red** | — |
| masking in the loader | green | green | green | **FR-1.1-S4 and FR-7.1-S2 red** |
| Decision 3's placement | green | green | green | green |

S9 says the fountain does not happen; S10 says why it does not; S11 says what it
costs. The loader variant is caught by neither of the three and by two scenarios
that already exist — which is the reason the loader placement is written out
above rather than assumed away.

All three appended to `spec.md` FR-4.1 by this phase, together with the removal of
the seven `[model-dependent]` markers those groups no longer carry.

---

## Decision 5 — the tick-rate binding · **BINDING: tick-rate-bound, stated, and already guarded**

D4 requires this decided either way. It is decided **against** expressing the
damping as a rate raised to `dt`, and the ground is a measurement rather than a
preference.

A continuous drag rate matching `r = 0.5` at 60 Hz is `60·ln(1.5) = 24.3279 s⁻¹`.
Holding the declared numbers fixed and moving the tick rate to 20 Hz:

| | sink b/s | rise b/s | swim b/s |
|---|---|---|---|
| 60 Hz, either form | 1.0000 | 2.0000 | 3.0000 |
| 20 Hz, **per-tick `r` fixed** | 3.0000 | 1.3333 | **3.0000** |
| 20 Hz, **rate raised to `dt`** | 0.6316 | 0.5926 | 1.3333 |

Mean `|ln|` drift across the three observables: **0.501** for the per-tick
divisor, **0.829** for the continuous rate. **The "fix" is measurably worse than
what ships.**

The reason is structural, not numerical. **Only one of the three observables is
an integrated state.** The sink is a terminal-velocity balance and does drift
with the tick rate. The rise and the horizontal swim are *velocity targets* —
`launched` and `walk_velocity` both **set** their value every tick — so a
constant per-tick divisor is *exactly* tick-rate-independent for the horizontal
swim (3.0000 at both rates, drift 0.000), and a continuous rate **introduces** a
drift where there was none.

**The tick-rate dependence lives in the control law, not in the coefficient's
form**, so no re-expression of the *coefficient* removes it. That is the whole of
what this comparison establishes, and it is narrower than the first draft claimed:
the **source term's placement** is a third lever, it does reduce the drift, and it
is treated on its own below rather than being covered by this sentence. What
remains irreducible without changing the control law is the **sink's** drift —
the swim's independence is already free, and the rise's is reachable but declined
for reasons that are about scenarios rather than about arithmetic.

### The third form, which the first draft failed to consider

The review raised it and it is a fair hit: **applying the ascent *after* gravity
rather than before it** removes the rise's drift entirely, because the rise then
becomes `ascent/(1 + r)` with no `dt` in it at all. Measured, with the ascent
retuned to `3.0` so that 60 Hz still observes the settled `2.0` b/s:

| | sink | rise | swim | mean `|ln|` drift, 60 → 20 Hz |
|---|---|---|---|---|
| shipped order (launch → gravity) | 3.0000 | 1.3333 | 3.0000 | **0.501** |
| ascent after gravity | 3.0000 | **2.0000** | 3.0000 | **0.366** |
| rate raised to `dt` | 0.6316 | 0.5926 | 1.3333 | 0.829 |

**It is the best of the three and it is still rejected**, because `spec.md`
settles the ordering and does so in two places that are not `[model-dependent]`:

- **FR-4.1-S2** requires `swim_ascent = 0.5` at no resistance to end a tick at
  exactly `0.0` — *"a declared ascent of exactly one tick of gravity holds a
  swimmer's depth"*. Under the reorder that same declaration ends at `0.5`. The
  phrase *"exactly one tick of gravity"* only means anything while gravity bites
  the launch.
- **FR-4.1-S3** requires a declared ascent of zero to arrest a sink without
  reversing it, ending at `−0.3333` from `−1.0`. Under the reorder it ends at
  `0.0` — the sink is not arrested, it is cancelled.
- **Technical Considerations** state it outright: *"A declared ascent is a launch
  speed, not an observed rise… the path `JUMP_SPEED` takes today"*, and the
  documented author-facing formula `(ascent − 0.5)/(1 + r)` is that ordering.

**And it is the same objection that killed D's branch variant, one axis over**:
applying the reorder to the ground jump as well changes land movement (D5), while
applying it only on the buoyant path puts a **second control law in the vertical
path** — which is precisely why the acceleration-for-swimmable-media-only variant
was rejected in Decision 1. A phase cannot refuse a branch on one axis and take
one on the other.

It also buys the smaller half. **The drift it removes is the rise's; the drift it
leaves is the sink's**, and the sink is what the complaint was about.

**The draft did not consider this form at all, and the review was right that the
omission mattered.** Decision 1's whole finding is that the freedom lives in the
source term, and the first draft of Decision 5 never turned that instrument on
itself — so its answer read as *"we compared the obvious alternative and it was
worse"* when the obvious alternative had not been compared. The conclusion is
unchanged; the ground for it is now the scenarios and the branch, not the metric.
Recorded rather than dropped, because it is the change to reach for if the tick
rate ever does move.

Accordingly:

- `move_resistance` and `swim_ascent` are **declared per tick at 60 Hz**, and this
  is stated where an author reads it (`docs/modding/blocks-items.md`) and where an
  engine reader does (`physics.rs`'s module doc, beside the launch order the same
  doc already states).
- `TICK_DURATION` is a private compile-time constant in `physics.rs` with no
  configuration path; `TICK_QUANTUM` is its `Duration` twin and `physics_test.rs`
  already asserts the two agree to within a nanosecond. The tick rate is not a
  knob that can turn quietly.
- **The falsifier already exists and no new guard is added.** FR-6.1-S2, S3 and S4
  assert absolute block displacements over absolute tick counts. At 20 Hz, S3's
  60-tick hold covers 4.0 blocks against its asserted 2.0. A tick-rate change
  reddens them, which is the property D4 asks for.

---

## Integration points

| Site | Change |
|---|---|
| `mc-core` `BlockDefinition` | field |
| `mc-world` `luau_declaration/mod.rs` | field constant, `RECOGNISED_FIELDS` 11 → 12, `declared_ascent`, `SWIM_ASCENT_BY_DEFAULT` |
| `mc-world` `persistence/format.rs` | `DeclaredBehaviour` appended field; `BEHAVIOUR_REVISION` 3 → 4 |
| `mc-sim` `player/mod.rs` | `VoxelMedium` field, `NOTHING`, `with` |
| `mc-sim` `replay/medium.rs` | `declared_by` masking arm (Decision 3) |
| `mc-sim` `player/physics.rs` | `launched` takes the folded `VoxelMedium`; module doc gains the tick-rate note |
| `mc-sim` `tests/support/volume.rs` | `Declaration` (:117) gains the field; `stating_a_medium(swimmable, move_resistance)` (:157) becomes three-arg — its doc forbids stating one medium field without the others — and ~13 call sites in `support/medium.rs` move with it |
| `mc-sim` `tests/support/` | **a new fixture**: shipped `base:water` at least four voxels deep, resolved through `content_registry()` — see below |
| workspace-wide | **23 `BlockDefinition {` constructions across 20 files** — `grep -rn "BlockDefinition {" --include=*.rs crates/ | grep -v "pub struct"` — see below |
| `mc-render` `capture.rs` | `SCENE_REVISION` `r3` → `r4`; four golden directories |
| `docs/technical/architecture.md` | `:569` `VoxelMedium { swimmable, resistance }`; **`:579` the table keyed on `(swimmable, move_resistance)` and its 1-bit / 128 KiB derivation**; `:550-557` the player-constants table, whose `Jump speed 9.0` row becomes conditional on the medium |
| `docs/technical/world-format.md` | `:672-780` — the behaviour list at revision 3, the per-move narrative, the "told once per move" prose, and the measured mutation record at `:772` |
| `crates/mc-sim/CLAUDE.md` | `:59-60` and `:69` — `VoxelMedium { swimmable, resistance }` and "both halves" become three |
| `crates/mc-sim/src/replay/medium.rs` | module doc and `MediumTable::of`'s doc, both of which key the table on `(swimmable, move_resistance)` by name |
| `crates/mc-sim/tests/player_buoyancy.rs` | `:177` is **blind to the signature change this spec makes** — it starts from `adrift()` at `Vec3::ZERO` and compares two positions a broken `launched` moves together. Give that one test a falling start (`−2.0`); **do not touch `adrift()`**, which feeds three other tests that are about starting at rest. Owned by the test author (Decision 2) |
| `crates/mc-sim/tests/medium_table_width.rs` | **the ceiling test's failure message**, which claims *"fits in one bit"* while the assertion permits two — repaired to state what it enforces and to name FR-4.1-S10 as the guard holding the one-bit property. The assertion itself is **not** tightened. Also its module prose, which says *"both medium questions"* where there are now three. Owned by the test author, not by this phase (Decision 4) |
| `content/base/blocks/water.luau` | `move_resistance` 1.6 → 0.5, `swim_ascent = 3.5` |
| `docs/modding/` | field row, example, per-field prose, the recognised-field list in four quotations across three pages, the hot-reload row, **and the predictable ratio `rise/horizontal = (ascent − 0.5)/4.5`** |
| `docs/user/gameplay.md` | `:82–92` re-derived, `:85` disambiguated, `:113` and `:352` counts repaired |

`sea.rs` cites `FR-6.1-S6` at :43 and :178 and `FR-6.1-S4` at :61, :82, :329 and
:336 — all **archived**-spec IDs that this spec reuses for different scenarios.
They are retargeted, per `spec.md`'s Technical Considerations. The panic message
at :336 additionally spells out a sink budget *derived from the declared value*,
which is the shape FR-6 exists to reverse, inside the fixture FR-6 leans on
hardest.

### The engine-reader documentation, which the first draft omitted entirely

**Binding, per the conductor's ruling, and the omission was a real one.** The
draft's table listed `docs/modding/` and `docs/user/gameplay.md` and no
`docs/technical/` at all. Six as-built statements go false when this lands, and
they are enumerated in the table above rather than left to be found.

**One of them is the statement Decision 3 exists to defend.**
`docs/technical/architecture.md:579` records the medium view as an index into a
table of the distinct `(swimmable, move_resistance)` answers, with the 1-bit /
128 KiB derivation resting on it. Decision 3's whole argument is that the masking
keeps that derivation true; carrying the decision without carrying its own
standing statement is how the two drift apart. Similarly `world-format.md`'s
behaviour-list narrative is the **engine-side mirror** of the
`gameplay.md:352` "four and three" repair the draft already carried — carrying one
without the other is the same drift with the audiences swapped. Key Principle 3
(architecture is standing) and Principle 4 (documented at the moment of
implementation, for every audience) both bite here, and "a follow-up will carry
it" is refused.

### Three costs the first draft's table understated

All raised by the review, all confirmed against the tree.

**`chamber.rs` cannot carry a medium, and the FR groups were confirmed without
checking where their tests would live.** `crates/mc-sim/tests/support/chamber.rs:161`
implements `medium_at` as `VoxelMedium::NOTHING` **unconditionally** — deliberately,
with a doc comment saying why, and `crates/mc-sim/CLAUDE.md` records the
measurement behind it: making that fixture's air a medium puts a resistance under
**46 pre-existing tests across twelve files**. It is not to be changed. What the
scenarios actually need:

- **FR-4 and FR-5 belong in `crates/mc-sim/tests/support/medium.rs`**, not
  `chamber.rs`. That module resolves every fixture through `ResolvedVoxels` on
  purpose — *"a fixture that answered `medium_at` directly would be a second
  statement of the rule under test"* — which means **FR-4.1-S9 genuinely
  exercises `declared_by` and the falsifier is real**. This is worth stating
  outright: a test author who reaches for `chamber.rs` instead writes an S9 that
  is green and tests nothing, because a fixture answering `medium_at` directly
  never reaches the masking at all.
- **FR-6.1-S2..S4 need a fixture that does not exist.** They require shipped
  `base:water` at least four voxels deep resolved through `content_registry()`.
  `support/medium.rs` resolves through the *fixture* registry; `support/sea.rs`
  gives only the generated two-voxel sea. That is a new fixture, and three of
  FR-6's sharpest scenarios stand on it. Named here so the tasks phase budgets it
  rather than discovering it.

**A field on `BlockDefinition` breaks every literal of it.** The type carries
public fields and **no `Default`, deliberately** — `..Default::default()` would
make inheriting a field invisible, which is the hazard the whole medium design is
organised around. Measured with the canonical command —

```
grep -rn "BlockDefinition {" --include=*.rs crates/ | grep -v "pub struct"
```

— which prints **23 constructions across 20 files**, spanning `mc-core`,
`mc-world`, `mc-sim` and `mc-client`, production and fixtures alike. **The
unfiltered command prints 25 across 22 and both extra lines carry `pub struct`**:
the type's own declaration in `crates/mc-core/src/block/definition.rs`, and a doc
comment in `crates/mc-world/tests/luau_declaration_properties.rs` quoting this
very command. Of the 23, exactly one is production
(`crates/mc-world/src/content/luau_declaration/mod.rs`), one is a bench, and 21
across 18 files are the test author's. **Run the command rather than copying the
figure**: five different values for this one number have been in circulation —
22, 23, 24 and 25, this document holding 25 in three places — and every wrong one
came from relaying rather than measuring. Two consequences the tasks phase
owns:

- The adaptation is mechanical but wide, and it is **not** a licence to introduce
  `Default` on the way past.
- It opens a window in which **the workspace does not compile**, and
  `testing.md` §2 records what that costs: *"a phase opening with an adaptation
  commit has no compilable tree for the gate to run on"*, so anything only the
  gate can see accumulates silently across it. Whoever authors tests inside that
  window runs `cargo clippy --workspace --all-targets --all-features -- -D
  warnings` **and `cargo fmt --check`** directly rather than waiting for the
  gate. **Both, and the second is the one that gets forgotten**: clippy cannot
  see formatting, and `cargo fmt --check` parses rather than compiles, so it is
  available throughout the window. Measured on this spec — the window's tests
  came out clippy-clean at `-D warnings` and failed the gate's `format` stage in
  two files.

The fixture builder moves with it. `Declaration`
(`crates/mc-sim/tests/support/volume.rs:117`) gains the field, and
`stating_a_medium(swimmable, move_resistance)` (:157) becomes three-arg rather
than gaining a default — its own doc forbids the alternative: *"a builder that
could state one and leave the other standing is how a resistance nobody wrote
arrives under a buoyancy somebody did"*, which is the same hazard, one field
wider. About thirteen call sites in `support/medium.rs` move with it.

**Three things checked and *not* in the blast radius**, recorded so nobody spends
a second pass on them: `mc-proto` carries no block definition, so there is **no
wire-format change**; `FORMAT_VERSION`
(`crates/mc-world/src/persistence/format.rs:59`) is the container version and was
unmoved across the previous 1→2→3 behaviour moves, so it stays; and the golden
count is four directories, `player-walk{,-hud}-t{000,059,119}-r3`.

---

## Assumptions

- **A1** The shipped `(0.5, 3.5)` plays as water. Unverified by construction —
  only play settles feel. FR-6 pins the observables absolutely so that a second
  round of feedback moves a declared number and not a model.

**Only A1 is an assumption. The draft carried a second one and it is now closed**
rather than deferred to implementation: `registry_of_many_media()`
(`crates/mc-sim/tests/support/medium.rs:190-203`) declares `(true, 0.0)`,
`(false, 1.0)`, `(false, 2.0)`, `(true, 1.0)`, `(false, 3.0)`. Masked, the three
non-swimmable ones go to ascent `0.0` and stay distinct by resistance; the two
swimmable ones take the `9.0` default and stay distinct by resistance. Five
distinct plus `NOTHING` is six entries and four bits — **with or without the
mask** — so `medium_table_width.rs`'s positive control keeps reporting wider than
the shipped one either way. Nothing to verify later.

## Risks

| # | Risk | Mitigation |
|---|---|---|
| R1 | The mixed-medium fountain (Decision 3) ships with a green suite | FR-4.1-S9 is its falsifier and is binding |
| R2 | An implementer "simplifies" Decision 3 into `with`, into the loader, or out entirely | **Three placements, three different falsifiers, tabulated in Decision 4.** `with` → S10 and S11; nowhere → S9, S10 and S11; the loader → FR-1.1-S4 and FR-7.1-S2. No single scenario covers all three, which is why the draft's original one-falsifier mitigation was wrong |
| R3 | **A test author writes FR-4.1-S9 against `chamber.rs`** — a fixture answering `medium_at` directly never reaches `declared_by`, so S9 goes green while testing nothing | FR-4 and FR-5 belong in `support/medium.rs`, which resolves through `ResolvedVoxels` on purpose. Named in Integration points; this is the failure mode that would silently retire R1's mitigation |
| R4 | `chamber.rs` is *changed* to carry a medium | It must not be. Its unconditional `NOTHING` is deliberate and `crates/mc-sim/CLAUDE.md` records the measurement — making that fixture's air a medium puts a resistance under 46 tests across twelve files. FR-6.1-S2..S4 get a **new** fixture instead |
| R5 | The `BlockDefinition` adaptation lands `Default` "to save 25 edits" | Explicitly refused above; the absence of `Default` is load-bearing for every medium scenario, and the same argument forbids a defaulted third arg on `stating_a_medium` |
| R6 | The `player_buoyancy.rs` repair is made by changing `adrift()` itself — the obvious edit, and one line shorter | It feeds three other tests, at least two of them about a fall **from rest** by name. The repair is a falling start local to `:177`. Named in Integration points and in Decision 2 |
| R7 | `−2.5` in FR-5.1-S3 is later "relaxed" to a tolerance | The scenario itself now records why the figure is exact — `slowed` is bit-exact identity at `r = 0` — so the reason travels with the assertion rather than living only here |
| R8 | FR-6's `1e-3` tolerances are chosen rather than measured | `spec.md` already requires the test author to derive the `f32` accumulation error; `testing.md` §2 records what an over-tight assertion on an arithmetic path costs |
| R9 | The residual drag rate (24.33 s⁻¹ against Minecraft's 4.463) and the velocity-target horizontal axis remain | Carried unmeasured and **not relaxed**. `spec.md` §"Known, measured, and deliberately not addressed" stands. Decision 5 adds that the horizontal axis is also what makes the *swim's* tick-rate independence free and the sink's drift irreducible without changing the control law — the rise's drift, by contrast, is reducible and is left in place for stated scenario reasons |

## Architecture review round

One round with `persona-architect` (Mode B), against the draft that carried
Decisions 1–5 without the subsections added above. **Nothing was overridden.**

**All ten findings are folded. None was overridden.** Every one was re-verified
against the tree here before being accepted, rather than taken from the review's
text.

| Verdict | Where it landed |
|---|---|
| **B1** — FR-4.1-S9 cannot see the masking-inside-`with` variant; both width tests keep their one bit of headroom | Accepted. The draft's counter-proposal (assert the resolved medium instead of tightening the ceiling) is kept as FR-4.1-S10, **and the reviewer's repair is added as binding**: FR-4.1-S11 asserts exactly one bit by equality. Decision 4, with a table of which falsifier sees which variant |
| **B2** — `launched(ascent)` is unimplementable; FR-4.1-S3 and FR-4.1-S4 demand opposite answers and coincide at `0.0` only by accident of S4's fixture | Accepted. Decision 2 §"`launched` takes both answers" |
| **M3** — `chamber.rs:161` answers `NOTHING` unconditionally; FR-4/FR-5 belong in `support/medium.rs`, and FR-6.1-S2..S4 need a fixture that does not exist | Accepted, both halves. Integration points §"Three costs…". The observation that `support/medium.rs` resolves through `ResolvedVoxels` — so S9 really does exercise `declared_by` — is recorded, since a test author reaching for `chamber.rs` writes a green S9 that tests nothing |
| **M4** — the delta is roughly three times the integration table | Accepted; **23 constructions across 20 files** (`grep -rn "BlockDefinition {" --include=*.rs crates/ | grep -v "pub struct"`; the unfiltered form overcounts by two, both `pub struct` lines), plus the three-arg `stating_a_medium` and its ~13 call sites. The three non-costs are recorded too |
| **M5** — Decision 5 never compared applying the ascent *after* gravity, which beats the shipped form on its own metric | Accepted and re-measured here (0.366 against 0.501). The conclusion is unchanged; **its stated ground is replaced** — FR-4.1-S2, FR-4.1-S3, and the second-control-law objection that already killed D's branch |
| **M6** — Decision 1 led with a scope clause where it owed an argument | Accepted, and the conductor has since ruled the same way. Grounds reordered onto the merits; the composability claim, which previously deferred to Decision 3 without discharging itself, is now argued in place |
| **m7** — "resolves to `NOTHING` exactly" is false in general | Accepted. Narrowed, with the shipped-content premise stated and `media_registry()`'s five resistant non-swimmable blocks cited |
| **m8** — "reachable in shipped content" overstates it | Accepted; corrected to *reachable in content*. The reviewer's two confirmations are folded in, including the hot-reload one |
| **m9** — the placement an implementer reaches for first is the loader, and the draft never excluded it | Accepted. Decision 3 §"The placement an implementer reaches for first" |
| **m10** — A2 can close now | Accepted; the deferral is deleted and only A1 remains an assumption |

Three things the reviewer confirmed rather than challenged, which are load-bearing
for Decision 3 and worth having on the record: `medium_around` structurally cannot
reach the block underfoot; **`World::adopt` re-resolves whole, so hot reload
strengthens the masking rather than threatening it**; and the "both width tests
stay green" trap holds against the fold path, the reload path and the table
arithmetic.

**One repair came from the conductor's own reading, not from the review.**
`medium_table_width.rs`'s failure message claims the index *"fits in one bit"*
while its assertion permits two — a documented claim and a checked claim that have
come apart. It is what turned FR-4.1-S11 from a rejected suggestion into a binding
one, and the draft's ground for rejecting it ("the ceiling is a deliberate
budget") is withdrawn: a test that silently absorbs a doubling is not protecting a
budget.

## Deferred

- **The horizontal control law** (D's acceleration half, and with it coasting).
  Not built here — `spec.md`'s Out of Scope scopes out the repair, and D was
  rejected on the merits besides. Two things Decision 5 hands whoever picks it up:
  the **sink's** tick-rate drift is the one that needs this change, because the
  swim's independence is already free under the current control law; and the
  **rise's** drift is reachable *without* it, by applying the declared ascent
  after gravity, which is declined here only because FR-4.1-S2 and FR-4.1-S3 are
  worded on the current order.
