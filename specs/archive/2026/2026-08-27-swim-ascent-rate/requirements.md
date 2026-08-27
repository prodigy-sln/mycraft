# Requirements — PRO-992

Source: PRO-992. The owner played the shipped `0fdba08` bundle — the first time
anybody had swum in this game — and reported, verbatim: *"Water feels more like
a swamp than water. You sink too slowly."* Steering the repair: *"It's possible
we need a different resistance for gravity based movement vs. intentional
movement."* and *"Check how other games do it."*

**Measured** = a reading of `0fdba08` (`main` = `origin/main`) on 2026-08-27.
**Derived** = arithmetic over it. The argument is in `spec.md`; this is the
evidence under it.

---

## 1. Constants — measured, `crates/mc-sim/src/player/physics.rs`

`TICK_DURATION` (`dt`) `1/60` :29 · `WALK_SPEED` (`W`) `4.5` :49 · `GRAVITY`
(`g`) `30.0` :52 · `JUMP_SPEED` (`J`) `9.0` :58 · `TERMINAL_SPEED` `48.0` :66 ·
`DISPLACEMENT_LIMIT` `1.0` :74. `water.luau` declares `swimmable = true`,
`move_resistance = 1.6` (`r`). **Derived**: one tick of gravity is `g·dt = 0.5`.

## 2. How one tick composes them — measured, `physics.rs:110`

`slowed(walk.with_y(fallen(launched(state, intent, medium.swimmable))), r)`.
`launched` (:222) sets `vy = J` when `intent.jump && (on_ground || buoyant)`,
else carries the previous `vy`. `fallen` (:263) subtracts `g·dt` and clamps at
`−TERMINAL_SPEED` — a `.max`, so **downward only**. `slowed` (:248) divides the
**whole** velocity by `1 + r`. `bounded` (:199) clamps displacement to one block
per axis. Order is **launch, gravity, resistance**, which `launched`'s own doc
states outright: the declared jump speed is never a value a caller observes.

## 3. Today's water — derived from §1–§2 at `r = 1.6`

- **Terminal sink** = fixed point of `v ← (v − g·dt)/(1+r)` = `−g·dt/r` =
  **0.3125 b/s**. **Rise held** is *set* each tick, never accumulated:
  `(J − g·dt)/(1+r)` = **3.2692 b/s**. **Horizontal** = `W/(1+r)` = **1.7308
  b/s**, 38.5% of a walk. **Rise ÷ sink** = **10.46×**.
- **A sink is a sum, not a speed times a time.** From rest it approaches terminal
  geometrically at `q = 1/(1+r)`; `n` ticks cover `dt·[−ns + s·q(1−qⁿ)/(1−q)]`.
  The sea's two voxels take **385 ticks, 6.4 s**; the "3.2 s/block" terminal
  shortcut understates it. Every displacement figure here is a sum of this kind.
- **Measured** — `tests/support/sea.rs`: `REQUIRED_DEPTH = 2` :64,
  `SEA_LEVEL = 34` (`support/mod.rs:79`), `lakebed() = surface + 1` :117 → feet
  **33.0**, water **[33, 35)**.

## 4. Two corrections, both to relayed figures, neither to the code

**(a) The brief dropped the gravity term**, giving `rise = J/(1+r)` = 3.46 and
`rise/sink = 18r`. Gravity subtracts before the resistance divides (§2), so rise
is `(J − g·dt)/(1+r)` = **3.2692** and the ratio `17r/(1+r)` = **10.46×**, bounded
by **17**. Accepted by the conductor against the code.

**(b) The survey inherited (a), and the check that "confirmed" it was circular.**
The researcher corrected only the denominator (`18r → 18r/(1+r)`); a script then
reproduced `11.0769` from `3.4615/0.3125`. **Both sides carried the same wrong
numerator** — `testing.md` §2's *"agreement between two wrong things is not
evidence"*, committed while verifying. Every relayed `rise` figure (`3.4615`,
`11.08`, `18r/(1+r)`, "62% of a ceiling of 18") is void. **§6's per-game rows
survive**: each came from that game's own source, independent of our numerator.

## 5. The coupling no coefficient reaches — derived

`rise = (J − g·dt)/(1+r)`, `horizontal = W/(1+r)`, so the `(1+r)` cancels:
**`rise/horizontal = (J − g·dt)/W = 1.8889`**, free of `r`. Eliminating `r`
between `sink = g·dt/r` and the rise gives `rise(sink) = 8.5·sink/(sink + 0.5)`,
monotonic, equal to `W` at **sink = 0.5625 b/s = 1.778 s/block**; faster than
that and a held jump outruns a walk on land.
**The steering's literal form does not cross it.** With `r_g` for the sink and
`r_i` for intentional motion, rise and horizontal are *both* intentional, both
divide by `1 + r_i`, and 1.8889 survives: `r_i` for a 3.0 b/s swim forces a
**5.667 b/s** rise (above `W`); for a 2.0 b/s rise it forces a **1.06 b/s** swim,
**24% of walking**, worse than the 38.5% complained of. It trades the sink↔swim
lock for a swim↔rise lock.

## 6. Surveyed games — relayed, safe per §4(b), tabulated in `spec.md`

Derived against §5's boundary: of MC ≤1.12 (0.50 s/block), MC 1.13+ (2.00),
Luanti (0.50), Source (1.09) and Quake (1.25), **only MC 1.13+ is reachable by
one coefficient**; the other four sink faster than 1.778 s/block. Aquatic cut
water gravity **4×** and left both drag coefficients at `0.8`. **No surveyed game
splits damping by motion source; none ships quadratic drag.** **Drag rate**:
`60·ln(1+r)` s⁻¹ — **57.33** at `r = 1.6`, **24.33** at `0.5`, against
Minecraft's `−20·ln(0.8)` = **4.463**. Still 5.45× harsher after the retune.

## 7. What the loader already has — measured

`content/luau_declaration/mod.rs`: `RECOGNISED_FIELDS` (:79) holds **eleven**
names in documentation order, ending `swimmable`, `move_resistance`.
`declared_resistance` (:233) is one call to
`number::optional_number_at_least_zero`, which already carries the whole numeric
refusal vocabulary. `MOVE_RESISTANCE_BY_DEFAULT` (:142) records why an absent
numeric field means a constant. `documented_declaration_fields.rs` reads the list
**out of** the refusal and the docs table, whole and in order, so a twelfth field
reddens it until four quotations across three `docs/modding/` pages move.
**`JUMP_SPEED` is private to `mc-sim` (:58, no `pub`) and `mc-world` does not
depend on `mc-sim`** (`mc-sim/Cargo.toml` names `mc-world`), so a loader default
cannot read it.

## 8. What a new declared field costs elsewhere — measured

`persistence/format.rs:309` — `BEHAVIOUR_REVISION = 3`, `APPEARANCE_REVISION = 3`;
a behaviour property moves the first only, and the second is already pinned by
hand at `format_test.rs:133` and `save_per_face_appearance.rs:111`.
`mc-render/src/capture.rs:47` — `SCENE_REVISION = "r3"`, four golden directories.
The two comments naming the hole this spec must not inherit (`water.luau`'s
closing paragraph and `sea.rs:5`) are quoted in `spec.md`. **Baseline** —
`0fdba08` reads `1544 tests run: 1544 passed`, a bare count and so a complete run
rather than a cancelled one.

---

## Clarifications

- [resolved] work-type/rigor → `feature` at `high`; a published declaration
  surface later specs and mods must not break. Owner's call, not re-litigated.
- [resolved] May the rate be a Rust constant? → No. Invariant 1; a missing hook
  is fixed in the scripting API.
- [resolved] Can one number fix this? → **Partly, and this spec's first draft
  overstated it.** `r = 1.0` reaches Minecraft's sink exactly at 50% swim speed;
  what one number cannot reach is any sink faster than 1.778 s/block (§5), which
  excludes four of five surveyed entries. The freedom is bought on that.
- [resolved] Does splitting drag by motion source solve it? → No; §5 measures it.
  Right instinct about where the freedom is missing, wrong term — it comes from
  splitting the **source**, which is what every surveyed game does.
- [resolved] Which model, then? → Not this phase's to settle; four candidates,
  their couplings and the deciding ground are in the Architecture Delta. A leads
  as the smallest reaching every target; D is the survey's pick on composability.
- [resolved] Jump from the lakebed while submerged? → The ground's speed.
  `launched`'s two conditions are independent reasons to leave, the firmer wins.
- [resolved] What does an empty cell contribute to the fold? → Nothing; the inert
  value, never the declaration default. Audit finding, pinned by FR-4.1-S8.
- [resolved] Absent-ascent default vs `JUMP_SPEED`, given §7? → **Checked
  behaviourally, never claimed in prose.** `SWIM_ASCENT_BY_DEFAULT = 9.0` in
  `mc-world` plus FR-1.1-S2 asserting the relationship from `mc-sim`. Promoting
  `JUMP_SPEED` into `mc-core` refused as standing architecture; a hand-carry note
  refused as a claim nothing checks.
- [resolved] Descend input? → No, by conductor ruling; reasoning in the spec's
  Out of Scope. MC 1.21 sneak `−0.04` and Luanti sneak are evidence it is
  *normal*, not that it is needed here.
- [resolved] Tick-rate binding decided? → No, and deciding it is a deliverable.
  Coefficients are per-tick at 60 Hz, 3× harsher than at Minecraft's 20 Hz, and
  every water speed moves if the tick rate does. Express as a rate raised to `dt`
  or state the binding in `docs/`; leaving it unwritten is what is refused.
- [assumed] What does "water not swamp" mean numerically? → sink 1.0, rise 2.0,
  horizontal 3.0 b/s (`move_resistance = 0.5`, ascent `3.5`) — all inside §6's
  range. Only play settles feel, so FR-6 pins them absolutely.
- [open] Does the retuned water read as water when played? → Unresolved by
  construction; this spec exists because that question was answered by playing.
  If it does not land, the residual drag rate and the velocity-target horizontal
  axis are where the spec says to look first.
