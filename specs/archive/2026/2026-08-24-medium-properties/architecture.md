# Architecture: a medium is one answer per voxel, and it enters the tick once

Spec: `spec.md` (SPEC-022, PRO-957, rigor `high`, 44 scenarios).
Requirements: `requirements.md`. Measurements: `open-questions.md`.

Every figure below was produced by a command run in this session against
`feature/PRO-957-medium-properties` at `8476cab`, or is arithmetic over such a
figure and labelled *derived*. Figures `requirements.md` recorded earlier are
relayed as **measured-by-the-spec-author** unless this document says it re-ran
them; where a re-run disagreed it is said so.

Architecture Delta item 3 — the behaviour fold gaining two fields and moving to
revision 3 — is settled in `requirements.md` §5 and is taken as given, not
re-argued. This document owes items 1 and 2, the integration point in
`advance_player`, and the corrected revision rule the spec assigns here.

---

## What this phase measured, and what it changed

Two measurements drove decisions that were not available when the spec was
written. Both were run against the **real simulation**, not against a model.

**One.** The declared replay walk wades through the sea: the scripted player's
box overlaps `base:water` at **60 of the 120 ticks** (44–99 and 116–119),
including capture ticks **59** and **119**. The spec has been amended and the
re-shoot is in scope; the per-tick box contents are in `open-questions.md`. The
consequence this document owes is the **corrected revision rule** (Decision 6).

**Two.** Water's admissible `move_resistance` window, re-derived by a discarded
spike against a built simulation, **disagreed with the model in `spec.md`'s
Technical Considerations on both bounds it stated.** The spec's response was to
remove the constants rather than move them: FR-6.1-S3 and FR-6.1-S4 now carry
closed forms in the declared resistance, so neither bounds the value, and what
bounds it is FR-6.1-S2 above and FR-6.1-S5 / S6 below. Decision 5 records the
window's shape, the method, and the obligation the implementer inherits — and **no
value**, by design.

---

## Drivers

**Quality attributes that matter here, with the evidence.**

- **Falsifiability, and it is dominant.** Two properties that are independent by
  declaration (FR-5.1-S4, FR-5.1-S5) will be read by one function one line
  apart; a design in which either can be derived from, or defaulted alongside,
  the other is a design in which a green suite says nothing. This is what decides
  Decision 2 and it is the reason the trait carries one method rather than two.
  `standards/global/testing.md` §2's "policy is not wiring" applies directly: a
  pure divisor is easy to test and easy to stop calling.
- **Totality on the tick path.** `ResolvedVoxels`' own header states the
  property: after construction "a query is a bounds test and a bit test, with no
  name to look up, no registry to consult and so no failure for anything on the
  tick path to swallow", and every position outside the volume answers by one
  bounds test rather than by a conversion that could saturate or wrap. A medium
  that broke that would put a fallible lookup inside `advance_player`, whose
  whole contract is that it is pure and total.
- **Memory at world scale.** The spec's **256 KiB** bound for everything a tick
  needs to answer both medium questions per voxel of the shipped world. Nothing
  in this increment measures memory, and that is not licence to relax it: "no
  test watches it" is a statement about the suite, not about the requirement.
  *Unrelated to the identically-valued `declaration file size | 256 KiB` bound at
  `docs/technical/architecture.md:1580`, which caps one file read into memory
  before evaluation. The coincidence is worth naming so a reader does not take one
  for the other.*
- **Save compatibility and the honesty of what a player is told.** This is the
  **second consecutive** behaviour-byte move, so every save written since PRO-904
  reports every block it holds as changed for the second time. The player-facing
  page that promises this happens once is a documentation deliverable of this
  spec (`spec.md`, Documentation deliverables).
- **Operability of the golden set.** Four directories re-shoot at `r3` through a
  procedure with a known corrupting failure mode.

**Constraints.**

- Invariant 1 — no block behaviour hardcoded in Rust. Both defaults are
  documented constants in the declaration loader and nowhere else; no engine
  module may read a block *name* to decide a medium.
- Invariant 4 — the server is authoritative. A medium is resolved server-side
  from an intent; `mc-proto` and the wire format are untouched.
- Invariant 3 — a bad mod never takes down the server. `move_resistance` is
  content-supplied and reaches floating-point arithmetic on the tick path, so the
  admissible set must be closed under the operation the physics performs.
- `Out of Scope` is binding, including its amendment: **no moved spawn, no moved
  `SEA_LEVEL`, no second declared scene**, and **no camera-path tripwire** — that
  last is recorded under `spec.md`'s Notes as deferred, and correcting the stale
  doc sentence is repair while adding the guard is a feature.
- No sensitive data, no regulatory exposure, no new source of nondeterminism.

**What is volatile, and what is expensive to reverse.**

- **Expensive**: the two published field names `swimmable` and `move_resistance`
  (a mod author writes them; a rename breaks third-party blocks); the
  `1 / (1 + r)` law, which every declared number is stated against;
  `BEHAVIOUR_REVISION = 3`, paid by every save in existence;
  `SCENE_REVISION = "r3"`, paid by four golden directories.
- **Cheap**: `VoxelMedium`, `Medium`, `Traversal`, the packed index and its
  width. No content and no save sees any of them, and the whole of the storage
  decision is reversible inside one file.

---

## Boundaries

This change touches no network, no vendor, no device and no clock. The table is
not empty because two boundaries already exist and both are crossed.

| External dependency | Volatility (Vendor / Regulatory / API / Substitutability) | Port | Adapter location | Direct-use justification |
|---|---|---|---|---|
| Content declarations in `*.luau`, read through the sandboxed host | none / none / low / low | already ported — `DefinitionSource` (`mc_core::block::source`); the engine never learns whether a definition came from a file or a script | `crates/mc-world/src/content/luau_declaration/` | — |
| The save file on disk (`redb` + `postcard`) | none / none / low / low | already ported — `DeclaredBehaviour` / `DeclaredAppearance` are the written-out-by-hand contract, deliberately not derived from `BlockDefinition` | `crates/mc-world/src/persistence/format.rs` | — |
| GPU / `wgpu` | — | reached only by the golden re-shoot, through the existing capture harness | `crates/mc-testkit/src/frame/gpu`, `crates/mc-client/tests/support/frames.rs` | Nothing in the design draws; the re-shoot uses the harness PRO-904 built and adds no device access. |
| Clock, random, filesystem | — | not reached | — | `advance_player` reads no clock (`crates/mc-sim/CLAUDE.md`); the resolve, the fold and the divisor are pure functions of their inputs. |

**No new external dependency, so no new port.** `architecture-principles.md`
§3's litmus test — "if this vendor disappeared tomorrow, how many files change?"
— has nothing new to ask it of. Both new fields ride the two ports that already
exist, which is why the change is as contained as it is.

**Three internal boundaries move, and they are the design's substance:**

- **The physics' door names the questions one tick asks.** It asks solidity
  today; it must ask a medium too, and it must still be unable to ask what a
  swing may aim at. See Decision 3.
- **`ResolvedVoxels` gains a third pre-resolved answer**, and that answer is a
  value rather than a bit. See Decision 1.
- **`SCENE_REVISION`'s stated contract is false and widens.** See Decision 6.

---

## Decisions

Every decision is **BINDING** unless marked otherwise. Tasks and implementation
are handed this document.

### 1. One packed per-voxel index into a per-registry medium table — BINDING

`ResolvedVoxels` gains **one** new per-voxel array: an index into a table of
distinct medium answers. It does **not** gain a `swimmable` bitset and a
resistance view; indexing the *pair* is the whole of the saving.

**Measured inputs**, this session, against the shipped content root:

```
resolved view: ResolvedVoxels { extent: Extent { x: 64, y: 256, z: 64 }, voxels: 1048576, .. }
registered blocks: 4
  base:dirt   is_solid=true  targetable=true drawn=true occludes=true
  base:grass  is_solid=true  targetable=true drawn=true occludes=true
  base:stone  is_solid=true  targetable=true drawn=true occludes=true
  base:water  is_solid=false targetable=true drawn=true occludes=false
```

`base:dirt`, `base:grass` and `base:stone` will state neither new field
(FR-6.1-S1), so all three answer the same medium — and so does an empty cell, and
so does every position outside the volume. `base:water` is the only other answer.
**`k = 2` distinct answers.**

*Derived*, at 1 048 576 voxels:

| distinct answers `k` | bits/voxel | bytes | | against the 256 KiB budget |
|---|---|---|---|---|
| ≤ 2 | 1 | 131 072 | **128 KiB** | **half** |
| 3–4 | 2 | 262 144 | 256 KiB | exactly at it |
| 5–16 | 4 | 524 288 | 512 KiB | twice over |
| 17–256 | 8 | 1 048 576 | 1 MiB | four times over |

**Today the whole of both medium questions costs 128 KiB — half the budget, and
the same as one existing bit view.** The naive shape, a `swimmable` bitset beside
a resistance view, costs 256 KiB before it answers anything general; the dense
`f32` costs 4 MiB. The index over the pair beats both because *what a voxel is,
as far as a medium is concerned* takes very few values.

**The table is built from the registry, never from the world's contents.**
`ResolvedVoxels::set` is called by `World::write` whenever a block is placed
(`crates/mc-sim/src/world/mod.rs:276`), and a block the world did not previously
hold must already have an index. Building the table from every registered
definition makes every writable answer present before any write, so the table
never grows and the packing is never widened under an edit. Index 0 is the
"nothing" answer, which is what an empty cell and everything outside the volume
share.

**`set` takes a minted index, never a `VoxelMedium` value, and this is the one
place the design would otherwise be a partial function.** `solid` and
`targetable` are total — every `bool` is writable — but a medium *value* is not:
writing one means finding its index in a table built at resolve time, and `set`
is `pub`. The existing caller does not resolve against the same registry:
`crates/mc-sim/tests/resolved_voxel_updates.rs:74` declares
`AN_OBSTACLE_NO_RAY_STOPS_AT: Answers = (true, false)` — a pair *invented* to be
unlike either registered block, written at two positions at `:84-85`. Handed a
`VoxelMedium` no registry produced, an implementation has four choices and this
module's own contract forbids three: a silent fallback is the swallowed error the
header refuses, a panic is a panic on a write path, and widening the packing
under an edit is forbidden by name above.

**The fix is this codebase's own idiom, and that is the argument.**
`crates/mc-sim/CLAUDE.md:56` states it for the neighbouring case — a re-mesh batch
carries its own serial and its own registry, "so a batch cannot be meshed against a
registry other than the one its world was resolved against — **that is unspellable
rather than checked**". A medium index is the same shape: a token that cannot name a
table it did not come from. It is also the standard `ResolvedVoxels` already holds
itself to when it refuses to let a caller write solidity without targetability. So
the table owner mints the only legal argument:

```rust
pub struct MediumIndex(/* opaque */);          // Copy, no public constructor
impl MediumIndex { pub const NOTHING: Self; }
impl ResolvedVoxels {
    pub fn medium_index_of(&self, declared: &BlockDefinition) -> MediumIndex;
    pub fn set(&mut self, at: WorldPos, answers: VoxelAnswers);   // AMENDED, see below
}
```

**AMENDED at implementation — the spelling only.** This document declared
`set(&mut self, at, solid: bool, targetable: bool, medium: MediumIndex)`, which is
**five arguments counting the receiver**, and `clippy::too_many_arguments` refuses
it: `clippy.toml:10` sets the threshold to 4, citing `code-quality.md` §2's *"max 4
parameters (use an object beyond that)"*. What shipped is one named
`VoxelAnswers { solid, targetable, medium }`, which is the remedy that standard
names in the same sentence as the rule.

**The original spelling was refused by the gate, not reconsidered on the merits.**
Every part of this decision holds unchanged: one call settles all three answers,
`set` stays total and infallible, and the index remains unspellable. Named fields at
the call site also keep the property loose arguments were chosen for — a caller
passing the same answer twice still reads as one doing so on purpose. This
architecture was right about the shape and wrong only about the arity it could
spend. Recorded in full at `spec.md`'s Notes, "`ResolvedVoxels::set` takes a named
triple".

`set` stays total and infallible because there is no way to name an index the table
does not hold — unspellable rather than checked, with no fallback to swallow, no
panic and no widening.

**This is where `MediumIndex` differs from `VoxelMedium`, and the difference is the
whole point.** `VoxelMedium` is a plain struct with public fields, so
`..VoxelMedium::NOTHING` can still inherit a field silently (Decision 2 says so and
declines to overclaim). `MediumIndex` has **no public constructor**, so the claim
that holds for it is the strong one.

**Not** `set(at, declared: Option<&BlockDefinition>)`, which is cleaner in the
abstract and wrong on the merits: it removes the ability to write a deliberately
inconsistent triple, and that ability is what
`crates/mc-sim/tests/resolved_voxel_updates.rs` is *for* — a view whose answers
disagree with any registry is the only way to state "exactly one of the two views
moved at each position". **A consequence worth noting, not the argument**: that file
is a test file, so had the interface been bent the other way the adaptation would
have been the test author's rather than the implementer's. Process does not pick an
API shape; if it were the only argument, the right answer would be to assign the
adaptation instead. Decision 7 inherits this shape.

**The width is chosen once at resolve, from `{1, 2, 4, 8, 16, 32}`.** A power of
two that divides 64 is what keeps an index from straddling a `u64` word, so a read
stays one shift and one mask — the property `Bitset::holds` has today. **The floor
is one bit**: `ceil(log2(k))` derives to zero at `k = 1`, which is a registry
whose blocks all answer the same medium, and a zero-width array has no bit to
read. **A read past the end answers index 0, which is `NOTHING`** — the same
totality `Bitset::holds` has, where "an offset past the end is unmarked rather
than a panic". A bare array index would panic on the one path that must not.

*Derived*: the resolve builds a `Vec` of one answer per voxel before packing it,
exactly as `Bitset::packing` does today, so there is a **transient** allocation of
about 1 MiB at `k <= 256` for a world this size. It is freed when the packed array
is built and it is not what the budget bounds; the budget is what the view *holds*
for the run.

**The word arithmetic is stated once.** `VOXELS_PER_WORD`, `WORD_SHIFT` and
`WORD_MASK` already exist in `resolved.rs`, and `Bitset` is the one-bit case of
the packed array. The implementer either generalises `Bitset` and keeps `solid`
and `targetable` as its one-bit instantiations, or builds the packed array on
those same three constants. A second copy of that arithmetic is "agreement
between two copies of one decision" (`testing.md` §2) and is refused.

**Totality survives.** A query becomes a bounds test, an index read, and an array
index into a table of at most `k` entries. No name, no hash, no registry, no
failure — `resolved.rs`'s header property holds verbatim, and the sentence in it
that says "a bounds test and a bit test" needs only widening to match.

#### The generalisation, and exactly where it breaks

`k` is a property of **content**, not of this design.

- Any number of blocks sharing one answer costs nothing. A hundred mod blocks
  that state neither field all share index 0 and `k` stays 2.
- **The first mod declaring a third distinct `(swimmable, move_resistance)`
  pair** takes the world to 2 bits — 256 KiB, exactly the budget, with no
  headroom left.
- **The fifth distinct answer** takes it to 4 bits — 512 KiB, twice the budget.

So the budget is met today with a factor of two in hand, and it is *content* that
consumes that margin rather than any later engine change.

**The trigger gets an instrument, because a trigger nobody measures is exactly
the defect Decision 6 spends a page repairing.** The draft of this document
called the trigger "mechanical" while Risks said "nothing enforces it"; both
cannot be true, and telling a reader a guard exists that does not is the thing
being fixed two decisions further down. So:

- a test asserting that the **shipped** registry resolves to a medium width of at
  most 2 bits, and
- **its positive control** — a synthetic registry declaring five distinct
  `(swimmable, move_resistance)` pairs, asserted to report a *wider* width.

Without the control the first test goes green forever the day the width accessor
comes to return a constant, which is `testing.md` §2's structural-invariant rule
verbatim. One `test-map.md` line under additional coverage, saying what each
catches.

#### Options rejected

| | Cost for the shipped world | Why not |
|---|---|---|
| Dense `Vec<f32>` + `swimmable` bitset | 4 MiB + 128 KiB | The budget rules it out; it is the option the budget exists to rule out. |
| One bitset per distinct non-zero resistance, plus a `swimmable` bitset | 256 KiB at `k = 2` | Exactly at budget with zero headroom, and a **second** non-zero resistance costs another 128 KiB. Worse than the index at every `k`. |
| Sparse map from voxel offset to answer | ≈ 1.4 KiB today | Degenerates without bound: a world that is mostly water costs 8 MiB and is slower than the dense array it replaced. A structure whose cost is a function of how much sea a world has is a structure that fails on the world this feature is for. |
| Per-voxel `BlockId` | 4 MiB | This is the world itself. The dedupe by *answer* is the entire saving. |

### 2. A third narrow trait, answering both medium questions in one call — BINDING

**One trait, one method, one value:**

```rust
pub struct VoxelMedium { pub swimmable: bool, pub resistance: f32 }
pub trait Medium { fn medium_at(&self, at: BlockPos) -> VoxelMedium; }
```

**Argued against the stated reasoning on `Solidity` / `Targetable`**, which this
project writes down in three places: the standing architecture at
`docs/technical/architecture.md:555`, the directory rule governing the very crate
this trait lands in at `crates/mc-sim/CLAUDE.md:49-51`, and the doc comment on
`Targetable` in `crates/mc-sim/src/player/mod.rs`. All three say one thing; the doc
comment says it at length:

> "Collision reads solidity at nine sites and means 'does this stop a player' by
> it; the walk a swing travels means 'may this be aimed at', and content declares
> the two independently. One trait carrying both questions would give every one
> of those nine sites access to a question it must never ask, and a collision
> scenario could then exercise aiming by accident."

That reasoning has three parts: many consumers of one question, a *different*
consumer of the other, and a rule that the first set must not reach the second.

**The rule is not overridden here. It is vacuous over this case**, and the
distinction matters — overriding a rule is a licence a later reader inherits, while
finding it never engaged is not. Its mechanism needs a *set of sites* reading
question A that must not reach question B. For a medium that set is **the same
singleton**: `swimmable` is read once, in `launched`; `resistance` once, in
`slowed`; consecutive lines, one fold, one box, one instant. **Splitting `Medium`
would remove no site's reach to anything at all** — there is no site to protect from
a question, because every site that can ask either already asks both.

**The hazard here is the opposite one, and it is what decides the shape.** The
risk is not a consumer reaching a question it must not ask; it is a consumer or a
fixture supplying one property and letting the other default — which is precisely
what FR-5.1-S4 and FR-5.1-S5 exist to catch, and which no assertion inside the
physics can see. `ResolvedVoxels::set`'s own doc comment already states the
countermeasure on the write side:

> "Both, in one call, because a caller that could write one without the other is
> the disagreement this type exists to make unspellable."

One method returning both applies that same rule on the read side. Two methods,
or two traits, would let a fixture implement `is_swimmable` and inherit a
resistance nobody stated — a fixture that is *correct* and asserts truthfully
about a world the product does not inhabit, which is the failure `testing.md` §2
spends a paragraph on.

**The type makes that harder, not impossible, and the difference matters.**
Functional-update syntax — `VoxelMedium { swimmable: true, ..VoxelMedium::NOTHING }`
— reopens exactly the hole, so "unspellable" would be an overclaim. What one value
buys is that the *silent* form is gone: stating one field and inheriting the other
now takes a visible `..`, which a reader can see. **Binding: `VoxelMedium` does not
derive `Default`**, and the omission is deliberate rather than an oversight, so
`..Default::default()` cannot make the inheritance invisible again. The actual
falsifiers are FR-5.1-S4 and FR-5.1-S5, and they are what this design is arranged
to keep sharp — not the type system.

**The segregation that matters is preserved and is not weakened.**
`collide::resolved_position` and `collide::on_ground` keep taking
`&dyn Solidity`, so a collision site still cannot ask a medium question, and
neither the physics nor the medium can ask what a swing may aim at (Decision 3).
**`Solidity` and `Targetable` stay two traits, and this decision is what keeps them
that way** — see "What the standing architecture becomes" for the amendment, which
narrows the rule rather than lifting it.

| | Shape | Why not |
|---|---|---|
| A. Two traits, `Swimmable` and `Resistance` | symmetric with the existing pair | Segregates a question nobody asks separately; two lookups per voxel instead of one; and it admits a fixture stating one and defaulting the other. |
| B. One trait, two methods | one parameter | Same fixture hazard as A. |
| **C. One trait, one method, one value** | **chosen** | Both answers come from one read of one table entry, and a caller that reads one without the other is unspellable. |

**The strongest honest argument against C**: it puts a two-field value where the
existing views return `bool`, so a consumer wanting only buoyancy carries a
resistance it discards. That cost is one `f32` per voxel over at most twelve
voxels per tick, and no such consumer exists. If one ever does — something that
floats but is not slowed — the split is mechanical, because the value type already
names the two fields separately and the fold already treats them separately.

**The fold is a physics rule, not a world property.** `VoxelMedium::NOTHING` is
the identity (`swimmable: false`, `resistance: 0.0`) and combining is
`swimmable || swimmable`, `resistance.max(resistance)` — FR-4.3's "the greatest
among them decides", with a cell holding no block contributing the identity. It
lives in `collide.rs` beside the box it folds over, because the box's shape and
the half-open rule are stated once there (`Aabb::around` and `voxels`), and a
second enumeration would be a second copy of one decision.

FR-4.3-S3 (a box wholly outside the world) and FR-4.2-S1 (standing on a resistant
solid) both fall out with no special case: everything outside answers `NOTHING` by
the same bounds test the bitsets use, and a player standing on grass overlaps only
the air above it, so the block below is not in the enumeration at all.

### 3. The physics' door names the two questions one tick asks — BINDING

`advance_player` needs solidity and a medium, and **both must come from one
object.**

```rust
pub trait Traversal: Solidity + Medium {}
impl<T: Solidity + Medium + ?Sized> Traversal for T {}

pub fn advance_player(state: PlayerState, intent: &MovementIntent, world: &dyn Traversal) -> PlayerState
```

**Not two parameters, and the reason is a disagreement this project has already
paid for once.** `crates/mc-sim/src/world/mod.rs`'s header and its `adopt` are
entirely about that hazard: the two existing views are "replaced wholesale rather
than written bit by bit, so there is no arrangement of this function in which one
is carried over from the registry that has stopped serving". Two parameters would
let a caller hand a solidity view of one world and a medium view of another.

**What `Traversal` actually buys, stated exactly, because the obvious claim is an
overclaim.** It does **not** make an inconsistent pair unspellable — the blanket
impl composes two independently written halves, so a fixture implementing both
badly still satisfies it. What it buys is narrower and real:

- **One wiring argument at the one production site.** `simulation.rs:235` reads
  `advance_player(self.player, &intent.movement, &self.world)` — measured this
  session. With one argument there is no arrangement of that line that passes a
  stale view beside a fresh one, which is precisely what `adopt` exists to
  prevent and what a second parameter would reintroduce.
- **`Targetable`'s exclusion stated in the type system** rather than in a comment.

The blanket impl means no fixture writes anything extra: implement `Solidity` and
`Medium` and `Traversal` follows. `Targetable` is deliberately **not** a
supertrait — the composite names what one tick of motion may ask, and aiming is
not among it, which keeps Decision 2's segregation stated in the type system
rather than in a comment.

**The access test, measured this session.** There are **nine** production
`&dyn Solidity` parameters, and **eight keep their door verbatim**:

| Site | Door after this change |
|---|---|
| `collide.rs:101` `resolved_position`, `:125` `resolved_axis`, `:222` `on_ground`, `:250` `overlaps`, `:259` `overlaps_solid` | `&dyn Solidity`, unchanged |
| `world/clearing.rs:68` `cleared`, `:98` its helper, `:114` `eligible` | `&dyn Solidity`, unchanged |
| **`physics.rs:74`** `advance_player` | **widens to `&dyn Traversal`** |

Coercion only ever **narrows**, so nothing inside a `&dyn Solidity` function can
name `medium_at` — the widening is confined to the one door that needs it, and the
eight others are proof rather than assertion. `trace.rs:64`'s `&dyn Targetable` is
untouched, so the aiming walk gains nothing either.

**`medium_around` takes `&dyn Medium` and not `&dyn Traversal`, and that is a
decision rather than an accident.** At `&dyn Medium` it *cannot* call `overlaps` or
`on_ground`; handed a `&dyn Traversal` the lowered-box trick would be one call away.
`spec.md` names exactly that shortcut as the thing FR-4.2-S1 exists to stop — "the
obvious shortcut because `on_ground` already lowers the box to ask a similar
question" — so the narrow parameter forecloses it **by construction**. Said here
because a silent foreclosure is what a later reader takes for an oversight and
"fixes".

`collide` therefore keeps its narrow doors, and `advance_player` upcasts
its one object to each. **Measured**: the toolchain is `rustc 1.97.1 (8bab26f4f
2026-07-14)`, edition 2024, and trait-object upcasting has been stable since 1.86,
so this is available rather than assumed. The fallback, if it ever is not, is two
accessor methods on `Traversal` returning the narrow objects; the binding is *one
object answers both*, not the mechanism.

**Blast radius, and it decides who commits first.** Measured this session, there
are **five** implementors of `Solidity`, and **three of them are test files**:

| Implementor | Kind |
|---|---|
| `crates/mc-sim/src/replay/resolved.rs:274` | source |
| `crates/mc-sim/src/world/mod.rs:357` | source |
| `crates/mc-sim/tests/support/solidity.rs:70` (`Ground`) | **test file** |
| `crates/mc-sim/tests/support/chamber.rs:151` (`Chamber`) | **test file** |
| `crates/mc-client/tests/camera_lens.rs:60` (`WalledFloor`) | **test file** |

**So the door-widening commit is the test author's, not the implementer's, or the
phase deadlocks at first compile**: at rigor `high` the implementer may not edit a
test file, and the tree does not compile until all five implement `Medium`.
`tasks.md` needs this stated as an ordering constraint rather than left to be
discovered. See Risks for what the gate cannot see while that window is open.

**The adaptation is eight edits, not a sweep, and the number changes the task
breakdown.** Three `impl Medium` blocks on the fixtures above, each returning
`VoxelMedium::NOTHING`; two production impls (`ResolvedVoxels`, `World`); and three
test-helper signatures — `player_collision.rs:223`, `player_ground.rs:212`,
`player_motion.rs:158`. **`player_limits.rs` and `player_input.rs` do not change at
all**: they pass concrete fixtures (`&floor`, `&Ground::Void`) which coerce through
the blanket impl, so the ~40 call sites in them are untouched. An earlier draft of
this document said "every fixture that advances a player must now state a medium",
which over-read the cost by roughly forty call sites. **Six of the eight edits are
in test files.**

**BINDING, and addressed to the test author, in the words `tasks.md` should carry
forward: every fixture that exists to state solidity implements `Medium` as
`VoxelMedium::NOTHING` unconditionally, never as a function of its own solidity.**
The temptation is concrete rather than hypothetical: those fixtures compute solidity
from a geometric rule, and in `WalledFloor` the negation of that rule *is* the air,
so "the air is the medium" is a one-line change that reads as insight. It would put
a resistance under dozens of collision assertions whose whole content is where a box
stops — and this is **this spec's own "absence means a constant, never solidity"
decision reappearing on the fixture side**, where no assertion can see it. It is the
read-side twin of `Resolved::of`'s doc comment, which refuses to derive either bit
from the other in the one place no declaration could override.

**What would catch it is not a scenario.** FR-4.1-S3 is about a *declared*
`move_resistance = 1.0` halving a fall and uses its own fixture, so it would stay
green if `Ground` or `WalledFloor` derived a medium from their solidity. The actual
falsifier is **the existing collision suite staying green through the adaptation** —
a regression property of tests already in the tree, not a spec scenario — and
`r = 0` being bit-exact identity (Decision 4) is what guarantees none of those
assertions moves. Stated precisely because attributing the guard to a scenario that
cannot see it is the same defect Decision 6 repairs.

**And the mechanism is written down, because a reporter whose mechanism is unwritten
is one a later refactor deletes without knowing what it was carrying.** This project
has the scar: a hand-maintained mirror compared by filtering, green for three names
it could no longer see (`testing.md` §2). Measured this session:

| File | `jumping()` helper | `jump` references |
|---|---|---|
| `crates/mc-sim/tests/player_collision.rs` | `:213` | **29** |
| `crates/mc-sim/tests/player_ground.rs` | `:192` | **37** |
| `crates/mc-sim/tests/player_motion.rs` | none | **0** |

Both helpers are the same four lines — `MovementIntent { jump: true, ..default() }`.
The reasoning was: a fixture deriving `swimmable` from its own solidity makes the
air swimmable, a held-jump test then starts *rising* instead of staying put, and
those two files go red.

> **MEASURED AT IMPLEMENTATION, AND THIS ATTRIBUTION IS INVERTED BY A FACTOR OF
> FORTY-SIX.** The table above counts how often those files *mention* jump. That is
> necessary and not sufficient, and the gap between the two is what this project
> keeps paying for. Run rather than read — the air itself made buoyant, then
> resistant:
>
> - **`swimmable` reddens one** pre-existing test, and **`player_collision.rs`,
>   named first here, does not move at all.** Nearly every jump in the suite is
>   asked *from the ground*, where `on_ground || buoyant` is already true, so the
>   buoyant half has almost no reach into the collision suite.
> - **`move_resistance` reddens 46**, across twelve files: every walk, every fall,
>   every jump arc, the replay poses and a golden frame.
>
> **The decision's conclusion is unchanged and this makes its case stronger, not
> weaker.** The rule stays unconditional over both halves — and the reason is now
> the measured one: *the buoyant half is the thin one*, carried by a single
> `player_ground` test rather than by two files. A rule that exempted the half
> nothing watches would have exempted exactly the half that turns out to be
> unguarded. What must be defended is that one test, not the two files named below.
> Full record in `test-map.md`'s Phase 2 mutation check.

**The two halves are guarded differently, and the asymmetry is in the *reporting*,
never in the rule.** A `move_resistance` derived from solidity looks inert, on the
reasoning that the box never overlaps a solid cell so such a resistance is zero
everywhere reachable. It is inert today — but what keeps the box out of solid cells
is a **maintained invariant, not geometry**, and neither half of it binds a fixture:

- placement into the player's own cell is refused (`Refusal::InsidePlayer`,
  `world/action/mod.rs:140` and `:251`), and that refusal's doc comment is explicit
  that choosing a different cell is not licence to skip the question;
- `clear_the_player` runs at exactly two moments — verified, two callers at
  `simulation.rs:158` and `:271` — which `world/clearing.rs:1-6` states as *"when
  they are seated in a world, and when a reload has made the cell they stand in
  solid"*.

**A test is free to seat a player overlapping a solid cell and nothing structural
stops it.** So the `move_resistance` half is inert only for as long as no tick
begins with the box inside a solid cell — which two rules maintain and no rule
enforces for fixtures. **Neither half is exempted**: `VoxelMedium::NOTHING`
unconditionally, both of them.

The sentence that stood here — *"the `swimmable` half is guarded by the suite"* —
was measured false at implementation, and the reasoning above about the resistance
half being "inert" was aimed at the wrong risk. Both halves keep the rule, and the
half that is genuinely unwatched is the **buoyant** one. A rule that exempts the
half nothing watches is the half that rots; the measurement above says which half
that is.

### 4. Where the medium enters one tick — BINDING

```
around       = medium_around(state.position, world)   // once, from the box at the START of the tick
look         = Look::of(&state).accumulate(intent)
walk         = walk_velocity(intent, look.yaw)
velocity     = slowed(walk.with_y(fallen(launched(&state, intent, around.swimmable))), around.resistance)
displacement = bounded(velocity * TICK_DURATION)
resolved     = collide::resolved_position(state.position, displacement, world)
on_ground    = collide::on_ground(resolved.feet, world)
-> PlayerState { position: resolved.feet, velocity: settled(velocity, on_ground, resolved.stopped_vertically), .. }
```

Every clause is load-bearing.

- **Read once, from the box at the start of the tick.** The divisor decides the
  displacement, so a medium read from the resolved end position would be read from
  a position the divisor produced. FR-4.1's "while the player's box overlaps" is a
  fact about the tick's beginning.
- **`launched` gains a `buoyant: bool`** and becomes
  `intent.jump && (state.on_ground || buoyant)`. One condition widened at the one
  site that already answers "may this tick launch", rather than a second launch
  path — which is what keeps FR-5.1-S3 (no swimmable overlap, no ground) reading
  as the same rule it reads as today.
- **`slowed(v, r) = v / (1.0 + r)`, applied to the whole velocity, after
  `fallen`.** A **division**, never `v * (1.0 / (1.0 + r))`: the two agree only
  where `1 + r` is a power of two, and they differ for whatever water ends up
  declaring. Gravity first is what makes the spec's stated medium terminal speed
  `GRAVITY · TICK_DURATION / r` the fixed point of `v <- (v − g·dt)/(1 + r)`. On
  every axis alike, per FR-4.1.
- **One `let`, two readers.** The divided velocity is what `bounded` multiplies
  into a displacement **and** what `settled` carries forward. That is the spec's
  ruling and it is what FR-4.1-S4 distinguishes; dividing only the displacement
  leaves a stored velocity accumulating as if in air.
- **`bounded` stays, after the divisor.** The divisor only shrinks, so
  `DISPLACEMENT_LIMIT` binds strictly less often than today. It is not removed:
  its doc comment says it exists to keep the per-axis resolution exact the day a
  constant changes, and that reason is untouched.
- **`TERMINAL_SPEED` stays inside `fallen`, before the division.** *Derived*: the
  medium's own terminal `g·dt/r` lies below `TERMINAL_SPEED` for every
  `r > g·dt/TERMINAL_SPEED = 0.5 / 48 ≈ 0.0104`, so in any medium heavier than
  that the two clamps never compete; below it the air clamp binds first, which is
  the right answer for a medium that is almost not there.
- **`r = 0` is bit-exact identity.** `v / (1.0 + 0.0)` is `v / 1.0`, which is `v`
  exactly in IEEE-754 for every finite `v`. This is why FR-4.1-S2, FR-4.2-S1 and
  FR-5.1-S5 can say "exactly as far" and be asserted by bit equality rather than
  by a tolerance — and `testing.md` §2's warning about over-tight assertions does
  not apply here, because the arithmetic path is a division by one rather than a
  trigonometric identity.
- **A constraint on the test author's fixtures, in the words `tasks.md` should
  carry forward: a ratio scenario asserts on `PlayerState::velocity`, or on the
  displacement before it is added to a position — never on `end − start`.** The
  licence in the bullet above does not extend to the *ratio* scenarios, and the trap
  is one a test author walks into by generalising it. **The naive fixture is the one
  that passes**: the comparison holds exactly at `x = 0.0`, so a fixture placed at
  the origin confirms the assertion, and the same assertion reddens against correct
  code once the fixture stands where the replay actually puts a player. That is the
  worst possible distribution, and it springs only when somebody writes the fixture
  — which is why it is stated as the author's constraint and not as a note about
  arithmetic. FR-4.1-S1,
  FR-4.1-S3 and FR-4.3-S1/S2 compare one displacement against a fraction of
  another. **Measured this session**, `f32` arithmetic over 64 integer columns ×
  four fractional offsets × `r ∈ {1, 3}` (512 samples): comparing the ratio on the
  displacement itself gives **0 mismatches of 512**, while recovering the
  displacement as `end − start` and comparing gives **425 mismatches of 512**, the
  first already at a start of `0.3`. The subtraction loses low bits to the
  coordinate it is taken at, and the loss is **not** confined to large coordinates —
  `x = 0.0` is very nearly the only place it holds. The alternative to the binding
  above is a tolerance derived from the ulp at the fixture's own coordinate, derived
  from both directions per `testing.md` §2 and never loosened until green.
  (The architecture review measured the same effect over a smaller sample and
  reported 15 of 40, holding at `x = 0.0` and failing at replay coordinates; this
  session's wider sweep agrees on the direction and finds the hazard broader. Both
  readings give the same remedy.)
- **Totality against content.** The loader admits only finite `r >= 0` (FR-2.2-S1,
  FR-2.2-S2), so `1 + r >= 1`, so the division can neither produce a NaN from a
  finite velocity, nor amplify one, nor reverse its sign. FR-4.2-S2's `1e30`
  divides a 4.5 blocks/s walk to `4.5e-30`, a normal `f32`: finite, forward, and
  no further back than it began. That closure is what invariant 3 requires of a
  number content supplies to the tick path.

**Measured**: the integration point above was built in a discarded spike and the
whole of `mc-sim`'s suite was run against it with the medium answering `NOTHING`
everywhere — `cargo nextest run -p mc-sim --no-fail-fast`, **`181 tests run: 181
passed, 0 skipped`**. A bare `N tests run` rather than `N/M`, so the run was
complete and not cancelled. Nothing existing moves when the medium is absent,
which is the evidence behind the bit-exact-identity claim above.

### 5. Water's `move_resistance` is derived at implementation, from the built simulation, and no number is written here — BINDING

`spec.md` binds the value to be re-derived against the built simulation, and its
Test-first exceptions admit the discarded spike that did so in this phase. **The
spike found the model wrong on both of the bounds it stated**, and the spec's
response was not to move the numbers but to remove them: FR-6.1-S3 and FR-6.1-S4
now carry **closed forms in the declared resistance** rather than constants. What
this document records is the **shape of the window and the method**, deliberately
not a value: a number here would be copied instead of derived, and the binding
would be defeated while appearing satisfied.

**Two of the three sea scenarios no longer bound `r` at all, and that is the
point.** Re-read `spec.md` for the exact forms; the design consequence is:

- **FR-6.1-S3's ceiling scales with `r`.** It is stated in terms of the velocity
  the medium gives a rising player, asserted over *every* tick of the hold
  including the ticks the box spends clear of the water — which is where the
  overshoot actually happens, since a player leaves the water carrying the upward
  velocity and continues ballistically. That is the quantity the spec's own "the
  divisor acts on the velocity, not only on the displacement" ruling establishes,
  and the quantity the original arithmetic omitted.
- **FR-6.1-S4's budget scales with `r`** for the same reason: a sink that is
  slower by construction cannot be judged against a fixed tick count without the
  count silently becoming a bound.
- **FR-6.1-S2 is what bounds `r` from above** (`r ≲ 16`), and **FR-6.1-S5 and
  FR-6.1-S6 bound it from below** — a resistance near zero makes neither a slowed
  fall nor a slower-than-walking swim observable at all.

**Why the constants were removed rather than corrected, recorded because it is the
generalisable part.** The peak height is **monotone decreasing in `r`**, so a
*constant* ceiling cannot avoid binding `r` from below — it is not a matter of
picking a better one. The first ceiling forced one bound and a second constant
would have forced a looser one: a larger cage, the same mistake. A closed form
binds nothing, and it moves on its own when a physics constant moves. The same
argument applies to a fixed tick budget, which is sharp at one end of the range and
enormously slack at the other. **This is `testing.md` §2's derived-oracle rule
applied to a scenario's own threshold**, and it is the reason a threshold that
happens to pass today is not evidence that it is the right threshold.

**The fixture column is the spec's ruling, not this document's.** FR-6.1-S2 names
"the deepest column of the shipped sea" and S3 and S4 say "that column". The
derivation is recorded here only so a reader can check it: deepest is
simultaneously the worst case for the rise (furthest to climb) and for the sink
(furthest to fall), and a shallower column widens the window enough to admit a
value the sea's own worst case refuses. **Measured this session** as that
derivation: the shipped sea is **131 columns**; its deepest is **(63, 30)**, two
blocks deep, lakebed top at `y = 32`; its shallowest is **(61, 0)**, one block
deep, lakebed top at `y = 33`. The 178-voxel census `requirements.md` §7 relays was
re-run and **agrees**.

**Binding on whoever derives the value: record the measurement showing the shipped
`r` sits inside the window, and record it against `spec.md`'s own forms.** Not a
margin check against a constant — there is no constant left to check against. The
obligation is to show, from the built simulation, that the chosen value satisfies
FR-6.1-S2's tick bound and leaves FR-6.1-S5 and FR-6.1-S6 observable. The child
that derives `r` will not have read any of the correspondence that produced this
requirement, which is why it lives here rather than in a review.

**The method the implementer follows**, which reproduces the spike without
inheriting its answer:

1. Build the real registry from `content/base` and the real world from
   `REPLAY_SEED`; resolve the real views. No hand-built fixture — the question is
   about the shipped sea.
2. Take the fixture column the spec names, and settle a player onto its lakebed by
   advancing it under a no-op intent until it rests. Never place it at a computed
   height: the world decides where its lakebed is.
3. Sweep `r` and, for each, run the **real** `advance_player`: hold jump and record
   the tick at which the feet first reach `y >= 34.0` (FR-6.1-S2) and the greatest
   end-of-tick height over the whole hold (FR-6.1-S3); then stop asking and record
   the tick at which the feet return to the lakebed (FR-6.1-S4). Also record a fall
   into the sea (S5) and one tick of a submerged walk (S6), because those are the
   two that bound the value from below.
4. Read the window off those curves, then **narrow the sweep and read it again** —
   the height curve is not monotone in `r` near the surface, because the crossing
   of the topmost water voxel is discrete, so a coarse sweep reports the wrong
   endpoints.
5. Never adjust the value to make a scenario green, and never adjust a scenario's
   form to admit a value. If the window is empty, or if the built simulation
   disagrees with the forms `spec.md` states, that is a spec finding — see Risks.

### 6. The revision rule is corrected, and all four golden directories re-shoot at `r3` — BINDING

**The deliverable is the rule. The re-shoot is its consequence.**

`SCENE_REVISION`'s doc (`crates/mc-render/src/capture.rs:32`) says it is bumped
when "a change to the **mesh contract** invalidates every committed frame" and
that "`crates/mc-sim`'s scene contract is the tripwire that fails first and names
this constant as the remedy."

**Measurement falsifies both sentences at once.** The mesh contract is untouched —
the world is unchanged, so `SCENE_QUAD_COUNT`, `total_face_area` and
`area_by_block` all hold — and two committed frames are nonetheless invalidated.
So `scene_contract.rs` will **not** fire, and the first symptom anybody gets is
two image comparisons failing with none of the guidance PRO-904 wrote into that
message.

**Why it used to be true, recorded because nobody wrote it carelessly:** the
camera derived from the spawn and the spawn from the world, so the trajectory *was*
a function of the mesh contract. This spec adds a third input — **content-declared
physics** — and severs that chain. The doc is **stale, not permissive**; a reader
taking it as permissive would conclude no bump is needed.

**The corrected rule, as the doc must state it:**

> `SCENE_REVISION` names a declared observation. It is bumped whenever a change
> makes a same-named capture incomparable with the frames already committed under
> that name — whether the change is to the **mesh contract** or to the **declared
> camera path**, which since PRO-957 includes the physics the scripted walk runs
> under. **The tripwire sees only the first of those.** `crates/mc-sim`'s scene
> contract compares quad count and per-block area, both properties of the mesh
> alone; a camera path that moved over an unchanged world leaves every one of them
> green, and the failure arrives as an image comparison instead. There is no guard
> for the second case.

That last sentence is required. Telling the next reader a guard exists that does
not is the defect being repaired, and `testing.md` §2's "an absent reviewer and a
clean reviewer look identical" is the general form of it.

**All four directories re-shoot**, including `player-walk-t000-r2` and
`player-walk-hud-t000-r2`, whose images reproduce byte-identically because tick 0
is dry (measured: the first wet tick is 44). The asymmetry decides it: bumping
costs two captures that reproduce identical images, while not bumping leaves
`player-walk-t059-r2` meaning one thing before this spec and another after — a name
silently redefined, permanently, in a repository that archives specs and expects
them to be re-readable.

**What is stated against the moved poses is re-derived, never copied from a run**:
`JUDGED_TICKS` (`replay_oracle.rs`), `SAMPLED_TICKS` (`replay_determinism.rs`),
`the_sea_the_camera_sees_is_the_water_layer.rs`, and the second and third of
PRO-904's `56 / 200 / 111` water sample counts. A count snapshotted from the first
green run commits whatever the code happened to do that day (`testing.md` §2).

**No camera-path tripwire is designed here.** `spec.md`'s Notes records it as
deferred, with the mechanism this phase measured. Correcting a false sentence is
repair; adding a guard is a feature.

### 7. The medium is a third thing the reload's wholesale replacement covers — BINDING

**Measured** by reading `crates/mc-sim/src/world/mod.rs:301`: `adopt` rebuilds
`ResolvedVoxels` wholesale on every registry swap. So a reload rebuilds the medium
table, its width and every index, and **FR-8.1-S1 and FR-8.1-S3 need no new
mechanism** — the medium becomes a third answer that rule already covers, and
`adopt`'s doc comment, which says "both views", must say three.

`World::write` settles the medium at the write site from the **same single**
`self.registry.resolve(block)?` that already settles `solid` and `targetable`, so a
placement cannot write two of the three answers from one resolve and the third from
another. It mints the index from that same resolved definition —
`self.resolved.medium_index_of(declared)` — and a cell being emptied passes
`MediumIndex::NOTHING`, which settles all three answers without a registry at all,
exactly as `write`'s doc comment already says of the first two.

`ResolvedVoxels::set` grows a fourth argument, of a different type from the two
booleans, so its doc comment's reason for separate arguments — that a caller
passing the same answer twice reads as deliberate — is unaffected. That the fourth
argument is a **minted index and not a value** is Decision 1's blocker fix, and it
is what keeps `set` total for the `pub` callers that do not resolve against this
registry.

**FR-8.1-S2** (no re-mesh): `changes_geometry`
(`crates/mc-sim/src/world/reload.rs`) does **not** learn either field. Its doc
comment's list of non-geometry fields grows by two; the predicate does not. Neither
field is visible in a still frame, which is the same test `requirements.md` §5 used
to put both on the behaviour fold rather than the appearance fold.

### 7a. The move-clear search cannot see a medium, by construction — BINDING

**Measured**: three of the nine production `&dyn Solidity` sites are not in
`collide` at all — `crates/mc-sim/src/world/clearing.rs:68` (`cleared`), `:98` (its
helper) and `:114` (`eligible`). They are the search that moves a player clear of a
cell that became solid under a reload, so they are reached from `adopt`, which is
Decision 7's territory, and Decision 7 would otherwise be silent about them.

Their parameter stays `&dyn Solidity`, so **the search cannot tell water from air**:
a cell holding a swimmable, resistant block is exactly as "clear" as an empty one.
That is not a defect and nothing in this spec asks otherwise — a medium does not
stop a player, and `spec.md`'s Out of Scope keeps a medium acting on movement alone.
It is stated because *"should the move-clear search prefer land over water?"* is a
real question a later increment will ask, and this design's answer is that it
**cannot currently ask it** — which is a fact about the design rather than an
oversight in it. Widening those three doors is what that increment would have to do,
and doing so would hand a search that means "is there ground here" a question about
what a volume does to movement.

### 8. Per-section medium storage — DEFERRED

A section holding one medium answer would cost O(1) rather than 4 096 indices,
mirroring `mc_world::section`'s own palette-per-section storage. *Derived* from the
water footprint `requirements.md` §7 relays (`x ∈ [60, 63]`, `z ∈ [0, 34]`,
`y ∈ [33, 34]` — consistent with the sea columns measured above, whose extremes are
(61, 0) and (63, 30)): the shipped world is 256 sections of 4 096 voxels and only
the three holding water would be mixed, so the storage would fall from 128 KiB to
under 2 KiB.

**Revisit when a registry's medium table exceeds four distinct answers** — the
point at which the flat form exceeds 256 KiB for a world this size. Not built now:
the flat form has a factor of two in hand today, `code-quality.md` §8 puts "measure
before optimizing" ahead of a structure whose only justification is a world nobody
ships, and the section form's worst case (every section mixed) is no better than the
flat form's.

### 9. Trivial, one line each

- `BlockDefinition` gains `swimmable: bool` and `move_resistance: f32`, appended,
  each with a doc comment stating the one question it answers. Forced: the fields
  exist or nothing can read them.
- `RECOGNISED_FIELDS` grows from nine names to eleven, appended in the documented
  order. **Order is load-bearing** — the refusal quotes the list back and
  `documented_refusals.rs` compares a real run line for line.

  **The mirrors, enumerated, because a short list here is the exact hazard Risks
  names.** Verified this session:

  | Site | Kind | What happens if it is missed |
  |---|---|---|
  | `crates/mc-world/src/content/luau_declaration/mod.rs`, `RECOGNISED_FIELDS` | source | the feature does not exist |
  | `crates/mc-world/tests/luau_declaration_keys.rs` | **test file** | reddens |
  | `crates/mc-client/tests/support/quoted_refusals.rs:66`, `FIELDS_IN_THE_ORDER_THE_GUIDE_STATES: [&str; 9]` | **test file** | reddens loudly |
  | four quoted refusals across `docs/modding/README.md` (1), `blocks-items.md` (2), `hot-reload.md` (1) | guarded docs | `documented_refusals.rs` reddens |
  | `docs/modding/blocks-items.md:63` "## The nine fields", `:160` "all nine you may write", `:718` "one of the six fields a save folds", `:755` "its own nine" | **unguarded prose** | rots silently |
  | `docs/INDEX.md:76` "all nine fields" and "which six fields a save folds"; `:77` "which of the nine fields re-mesh" | **unguarded prose, outside `PAGES`** | rots silently |

  **Two of these are test files** the implementer may not edit at rigor `high`;
  they are the test author's. **`docs/INDEX.md` has no guard at all** — `PAGES =
  ["docs", "modding"]` (`quoted_refusals.rs:54`) walks that tree alone, and
  `INDEX.md` is not in it, which is the same hole `spec.md`'s Notes records for
  `gameplay.md`. It is **checked by hand at completion** and nothing else will
  catch it. The behaviour-fold count those pages state as six grows by two.
- `swimmable` is read by the existing `optional_boolean` with `false` as the absent
  value. Forced: the reader exists and already refuses a wrong kind rather than
  falling back to the default.
- `move_resistance` needs the loader's **first numeric reader**. It accepts
  `ScriptValue::Integer` and `ScriptValue::Number` alike (`requirements.md` §3),
  refuses every other kind through `FieldFault::wrong_kind` with `kind_of`'s
  wording, and refuses non-finite and negative values through `FieldFault::invalid`.
  It is the numeric-validation vocabulary every later number inherits, which is
  `spec.md`'s stated reason for rigor `high`.
- The declared resistance is normalised on read so `-0.0` registers as `0.0`,
  because the fold serialises by bits. `spec.md`'s Technical Considerations settles
  it; the mechanism is whichever of `+ 0.0` or an explicit zero test the implementer
  can justify in one line.
- Invariant 1 is satisfied **here and only here**: both defaults are documented
  constants in the declaration loader, and no engine module reads a block name to
  decide a medium. `crates/mc-world/tests/no_hardcoded_block_names.rs` is the
  standing check.

---

## What the standing architecture becomes

`docs/technical/architecture.md` is the standing record, and **four passages in it
become false with this change.** They are listed with their replacement wording so
that consolidation *applies* this rather than reconstructing it from the code — the
same reason Decision 6 rewrites a doc rather than leaving the next reader to infer
it. Every line number was verified this session.

**The two copies of the two-traits rule need *different* repairs, and treating them
alike would damage one of them.** Verified this session:

- **`crates/mc-sim/CLAUDE.md:43-51` is already scoped** and needs **extension, not
  narrowing.** Its bullet opens "There are **two** resolved views, not one", names
  `Solidity` and `Targetable`, and says "Collision reads `Solidity` at nine sites…
  the walk a swing travels reads `Targetable` and nothing else does. **Keep them two
  traits**." *Them* is that named pair. Decision 2 never engages it, so there is
  nothing there to narrow — what is false is only the count.
- **`docs/technical/architecture.md:555` is the unscoped one** — "never one trait
  with two methods" — and it is the only sentence that needs **narrowing**.

**The "never" is not struck in either.** Strike it and a later spec merges
`Solidity` and `Targetable`, and the original hazard — every collision site handed a
question it must never ask — comes straight back. `Solidity` and `Targetable` remain
two traits under both repaired wordings. This is not an exception carved out for a
medium: one sentence was stated too broadly, and the other was simply written when
there were two views.

**`:555` — the unscoped rule, which is the one that narrows.** It currently reads:

> "**Two narrow traits over one type, never one trait with two methods.** Content
> declares 'does this stop a player' and 'may a swing find this' independently, and
> the nine sites that read `Solidity` mean collision by it. A single trait carrying
> both would hand every one of those sites a question it must never ask, and a
> collision test could then exercise aiming by accident."

It is **unqualified**, and Decision 2 is a `Medium` trait with one method returning
two answers, so the rule must be amended rather than quietly worked around. The
replacement keeps the rule and states its premise, which is what makes the
exception legible instead of arbitrary:

> **Two narrow traits over one type where the questions have different consumers;
> one call where they have the same one.** Collision and aiming are read by
> different code — nine sites mean collision by `Solidity`, and the swing's walk
> means aiming by `Targetable` — so they are separate traits, and a collision site
> cannot reach an aiming question. A **medium** is the other case: both of its
> questions are read by `advance_player` alone, one line apart, folded over the same
> box at the same instant, so segregating them buys nothing and the live hazard is
> the opposite one — a fixture stating one and inheriting the other. `Medium`
> therefore answers both in one call, the way `ResolvedVoxels::set` writes both in
> one call, and the composite `Traversal: Solidity + Medium` is where the physics'
> exclusion of `Targetable` is stated.

Its replacement wording:

> **Narrow traits where the consumers differ.** `Solidity` and `Targetable` are two
> traits because collision reads one at nine sites and the walk a swing travels reads
> the other, and neither set may reach the other's question. Where one site reads two
> properties of one question, they travel in one value: `Medium` returns
> `VoxelMedium { swimmable, resistance }`, because splitting it would segregate
> nothing — `advance_player` reads both, one line apart, from one fold over one box —
> while admitting a fixture that states one and inherits the other.

**`crates/mc-sim/CLAUDE.md:43-51` — the scoped copy, which *extends*.** A
directory-scoped `CLAUDE.md` is read by every agent working in `mc-sim` and is the
rule an implementer meets first, so its count must be right even though its rule is
not wrong. Its replacement wording:

> **There are three resolved views, not two.** …`Solidity`, `Targetable`, and
> `Medium`, which answers what a voxel's volume does to something moving through it.
> Keep `Solidity` and `Targetable` two traits, for the reason above. `Medium` is one
> trait returning one value, because one site reads both of its properties —
> splitting it would separate nothing and would let a fixture state one and inherit
> the other. The composite `Traversal: Solidity + Medium` names what **one tick of
> motion** may ask: `Targetable` is deliberately not among it.

**`:563` — the cost figure, which says two grids.** "The second grid costs
1 048 576 voxels × 1 bit = **+128 KiB** once, at world scale." There are now
**three** per-voxel arrays, and the third is an index rather than a bit. The
replacement states the third array, its width rule, and that the shipped registry
puts it at one bit — 128 KiB — with the table it indexes being a handful of
entries.

**`:567-580` — the two-views totality paragraph**, including "`ResolvedVoxels::resolve`
then answers `false` for both, in both views — a position the volume does not reach
and a cell holding nothing are alike neither solid nor targetable", and the
observation at `:576` that this is "the one place in the codebase where the two
readings genuinely **coincide in outcome**". Both still hold and both grow a third
member: the third answer for those positions is `VoxelMedium::NOTHING`, and the
coincidence is now three-way. The paragraph's conclusion — that no assertion on the
resolved view can tell the two readings apart, so the split is held by the arms
being written separately and by review — is **unchanged and applies to the medium
arm too**, which is worth saying explicitly because the medium's arm is the newest
and the least exercised.

**`:744-751` — the `write` snippet**, whose last comment reads
`self.resolved.set(at, solid, targetable); // both resolved views, together`.
It becomes three views and a minted index. **This snippet is already stale for a
second, unrelated reason**: it shows `fn write(&mut self, at: WorldPos, block:
&BlockName)` while the shipped signature is `fn write(&mut self, at: WorldPos,
contents: Contents<&BlockName>)`, which is what lets a cell be emptied. Consolidation
fixes both, and the pre-existing drift is recorded here rather than silently
repaired so that whoever reads the diff can see the two changes are independent.

---

## Interfaces

```rust
// crates/mc-core/src/block/definition.rs
pub struct BlockDefinition {
    // ... existing fields ...
    /// Whether a player can hold itself up in this block's volume.
    pub swimmable: bool,
    /// How much this block's volume slows what moves through it. Finite and not
    /// less than zero; `0.0` is exactly "unaffected". The speed through the
    /// volume is divided by `1 + move_resistance`.
    pub move_resistance: f32,
}

// crates/mc-sim/src/player/mod.rs
/// What a voxel's volume does to something moving through it.
///
/// Two independent declarations in one value, because a caller that could read
/// one without the other is the disagreement this type exists to make
/// unspellable — the same reason `ResolvedVoxels::set` writes both at once.
/// **No `Default`, deliberately**: `..Default::default()` would make inheriting a
/// field invisible again, and FR-5.1-S4 / FR-5.1-S5 are the falsifiers that
/// depend on a fixture stating both.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VoxelMedium {
    pub swimmable: bool,
    pub resistance: f32,
}

impl VoxelMedium {
    /// What a cell with no block in it answers, and what everything outside the
    /// world answers: neither buoyant nor resistant. The identity of `with`.
    pub const NOTHING: Self;

    /// The medium of two overlapped cells taken together: buoyant if either is,
    /// and the greater of the two resistances.
    #[must_use]
    pub fn with(self, other: Self) -> Self;
}

/// What medium a voxel is.
///
/// **Total**, by the same construction as `Solidity` and `Targetable`: every
/// position has an answer, and everything outside the loaded world answers
/// `NOTHING`.
pub trait Medium {
    fn medium_at(&self, at: BlockPos) -> VoxelMedium;
}

/// What one tick of motion may ask of the world, and no more.
///
/// `Targetable` is deliberately absent: a tick of motion has no aiming question
/// to ask, and this composite is where that is stated.
pub trait Traversal: Solidity + Medium {}
impl<T: Solidity + Medium + ?Sized> Traversal for T {}

// crates/mc-sim/src/player/physics.rs
pub fn advance_player(
    state: PlayerState,
    intent: &MovementIntent,
    world: &dyn Traversal,
) -> PlayerState;

// crates/mc-sim/src/player/collide.rs
/// The medium acting on the box standing at `feet`: the greatest resistance
/// among the cells it overlaps, and whether any of them is swimmable.
pub(crate) fn medium_around(feet: Vec3, world: &dyn Medium) -> VoxelMedium;

// crates/mc-sim/src/replay/resolved.rs
/// An index into one `ResolvedVoxels`' medium table. Opaque and `Copy`, with no
/// public constructor: the table owner mints every legal value, which is what
/// keeps `set` total for callers that did not resolve against this registry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MediumIndex(/* private */);

impl MediumIndex {
    /// The index of "no medium here" — an empty cell, and everything outside the
    /// volume. Always present, at width one bit and above.
    pub const NOTHING: Self;
}

/// What one voxel answers about all three questions. Public so that `set` can
/// take one value: the declared four-argument form is five counting the
/// receiver, which the complexity gate refuses — see Decision 1's amendment.
pub struct VoxelAnswers {
    pub solid: bool,
    pub targetable: bool,
    pub medium: MediumIndex,
}

impl ResolvedVoxels {
    /// The index this view's table holds for `declared`'s medium.
    pub fn medium_index_of(&self, declared: &BlockDefinition) -> MediumIndex;

    pub fn set(&mut self, at: WorldPos, answers: VoxelAnswers);
}
impl Medium for ResolvedVoxels { /* bounds test, index read, table lookup */ }

// crates/mc-sim/src/world/mod.rs
impl Medium for World { /* forwards to the resolved view, never to the registry */ }
```

---

## Data

- **`DeclaredBehaviour`** gains `swimmable` then `move_resistance`, appended after
  `targetable`; `BEHAVIOUR_REVISION` moves **2 → 3**. `DeclaredAppearance` and
  `APPEARANCE_REVISION` are untouched — FR-7.1-S7 is what catches an appearance
  byte bumped alongside.
- Both structs stay written out by hand and never derived from `BlockDefinition`;
  the existing doc comment's reason holds unchanged.
- **Field position is part of the record**: `postcard` encodes positionally, so
  appending reads as an addition while an insertion in the middle changes every
  byte.
- **`move_resistance` is folded as an `f32` by its bits**, at the same width the
  physics divides by, so no declared value is retained at a precision the tick
  cannot use — which is also why `-0.0` is normalised on read.
- **`BlockDefinition` stops deriving `Eq`.** Verified this session:
  `crates/mc-core/src/block/definition.rs:34` is
  `#[derive(Debug, Clone, PartialEq, Eq)]`, and an `f32` field ends `Eq` because
  `f32` is not `Eq`. That is a change to a **public `mc-core` contract**, not a
  local edit: anything using a `BlockDefinition` as a `HashMap` key, in a `BTreeSet`,
  or behind a derived `Eq` of its own stops compiling. `PartialEq` survives and is
  what the existing comparisons use. Recorded here because it is the one part of
  this change that reaches a crate boundary without any scenario naming it.
- **Nothing crosses the wire.** `mc-proto` is untouched; a medium is resolved
  server-side from an intent.
- **The medium table is not persisted.** It is derived from the registry at resolve
  time and rebuilt on every reload, so there is no second place a block's medium is
  recorded and no way for two records to disagree.

---

## Integration

| Where | What changes |
|---|---|
| `crates/mc-world/src/content/luau_declaration/mod.rs` | `swimmable` through the existing `optional_boolean`; a new numeric reader for `move_resistance`; `RECOGNISED_FIELDS` nine → eleven |
| `crates/mc-world/tests/luau_declaration_keys.rs` | **test file** — the hand-maintained mirror of that list, read whole and in order (FR-3.1-S1) |
| `crates/mc-client/tests/support/quoted_refusals.rs:66` | **test file** — `FIELDS_IN_THE_ORDER_THE_GUIDE_STATES` nine → eleven |
| `crates/mc-sim/tests/support/{solidity,chamber}.rs`, `crates/mc-client/tests/camera_lens.rs` | **test files** — the three fixture `Solidity` implementors that must also implement `Medium`; Decision 3 says this commit comes first |
| `docs/INDEX.md:76-77` | three prose counts, **no guard of any kind** — checked by hand at completion |
| `crates/mc-core/src/block/definition.rs` | two fields |
| `crates/mc-world/src/persistence/format.rs` | `DeclaredBehaviour` + 2; `BEHAVIOUR_REVISION` 2 → 3 |
| `crates/mc-sim/src/replay/resolved.rs` | the medium table, the packed index, `Resolved::of` reading two more fields each from its own field, `set` + 1 argument, `impl Medium` |
| `crates/mc-sim/src/player/mod.rs` | `VoxelMedium`, `Medium`, `Traversal` |
| `crates/mc-sim/src/player/collide.rs` | `medium_around`, folding over the existing voxel enumeration |
| `crates/mc-sim/src/player/physics.rs` | `slowed`, `launched` + `buoyant`, the door widened to `&dyn Traversal` |
| `crates/mc-sim/src/world/mod.rs` | `write` settles a third answer from the one resolve; `adopt`'s doc says three; `impl Medium for World` |
| `crates/mc-sim/src/world/reload.rs` | `changes_geometry`'s doc-comment list grows; the predicate does not |
| `crates/mc-render/src/capture.rs` | `SCENE_REVISION` `"r2"` → `"r3"`, and the corrected doc of Decision 6 |
| `crates/mc-render/goldens/` | four directories re-shot at `r3` |
| `replay_oracle.rs`, `replay_determinism.rs`, `the_sea_the_camera_sees_is_the_water_layer.rs` | re-derived against the moved poses, never copied from a run |
| `content/base/blocks/water.luau` | `swimmable = true` and a derived `move_resistance` |
| `docs/modding/`, `docs/user/gameplay.md`, `docs/technical/` | the three audiences of Key Principle 4; the player page's broken "It happens once" promise is the repair nobody would find later |

---

## Assumptions

- **The shipped spawn is on the shore.** Measured this session: the spawn column
  `(63, 35)` has surface height 34 — *derived* from the tick-0 feet at `y = 37.0`,
  which is `surface + SPAWN_ABOVE_SURFACE` — so it is dry, because water fills a
  column only where its surface is below sea level. FR-6.1-S6 asserts against that
  column directly.
- **The player is the only thing a medium acts on, and it moves only through
  `advance_player`.** Read from `crates/mc-sim/src/player/`. If a second mover ever
  appears it takes the same door, which is why the door is a trait.
- **`base:dirt`, `base:grass` and `base:stone` state neither field** (FR-6.1-S1).
  This is what makes `k = 2`, and it is asserted by a scenario rather than merely
  held.

---

## Risks

- **The window for water's resistance is no longer bounded by the scenarios that
  look like they bound it.** *Escalated, and resolved by a spec change.* The spike
  found the original model wrong on both stated bounds; `spec.md` responded by
  replacing FR-6.1-S3's and FR-6.1-S4's constants with closed forms in `r`, so
  neither bounds the value any more. The live risk is that an implementer reads
  those two scenarios as the window — they are the ones that *mention* `r` — and
  never derives against FR-6.1-S2, S5 and S6, which are what actually bound it.
  Decision 5 names them explicitly for that reason.
- **A scenario's threshold is an oracle, and a threshold that passes is not evidence
  it is right.** This spec has now had three thresholds on one scenario, two of them
  wrong in opposite directions, and every one of them looked defensible when
  written. If the built simulation disagrees with the forms `spec.md` now states,
  that is a **spec finding** and never a tuned number — and equally, a value must
  never be admitted by softening a form. Named here because it is the failure most
  likely to be resolved the wrong way under time pressure, and because the person
  meeting it will not have seen the two earlier attempts.
- **The adaptation window has no compilable tree, so the gate cannot run.** Five
  `Solidity` implementors gain a `Medium`, three of them in test files, and
  `testing.md` §2 records that a defect only the gate can see accumulates silently
  across such a window. Whoever authors tests inside it runs
  `cargo clippy --workspace --all-targets --all-features -- -D warnings` directly,
  at that severity, rather than waiting for the gate. Decision 3 states who commits
  first; getting that order wrong deadlocks the phase rather than degrading it.
- **The golden re-shoot has a known corrupting failure mode** — a run reaching
  `golden_mismatch` with the opt-in set. PRO-904's procedure is followed as written;
  four directories move at once, which is more than that procedure has been
  exercised with.
- **The recognised-field list is mirrored in eleven places, and four of them have
  no guard.** Decision 9 enumerates them. `testing.md` §2 records two mirrors of
  this same list held at six while it grew to nine, neither reddening, because both
  compared by filtering; FR-3.1-S1 reading the list whole and in order is the
  repair for the guarded ones. **The unguarded ones are the live risk**:
  `docs/modding/blocks-items.md:63`, `:160`, `:718`, `:755` and `docs/INDEX.md:76-77`
  carry prose counts that nothing compares against anything, and `INDEX.md` sits
  outside `PAGES = ["docs", "modding"]` entirely. They are checked by hand at
  completion. Widening `PAGES` is `spec.md`'s Notes, deferred — not this spec's.
- **`k = 2` is a property of content, not of the design, and what enforces the
  budget is one test with one control.** Decision 1 adds a width assertion over the
  shipped registry plus a five-answer positive control, so the trigger is measured
  rather than merely named — the draft of this document claimed "mechanical" in
  Decision 1 and "nothing enforces it" here, which is the contradiction Decision 6
  exists to make unacceptable. **What remains genuinely unenforced** is a *third-party*
  registry at run time: the guard is a test over the shipped content, not a runtime
  refusal, so a mod root that pushes a real world past 256 KiB does so silently.
  That is accepted, not closed, and Decision 8 is the answer when it arrives.
