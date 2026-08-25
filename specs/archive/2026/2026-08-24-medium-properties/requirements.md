# Requirements — PRO-957

Source: [PRO-957](https://linear.app/prodigy-solutions/issue/PRO-957/you-can-swim-in-water-the-medium-properties-and-one-resistance-field),
split out of PRO-904 on 2026-08-22. `product/roadmap.md:125` states the feature
as *"The medium properties: swimmable, and one resistance field — you can swim
in it"*.

Everything below marked **measured** was produced by a command run on
2026-08-24 against `feature/PRO-957-medium-properties` at `7f1f7db` (the tree
`main` merged PRO-904 into). Everything marked **derived** is arithmetic or a
reading of that tree, labelled as such.

---

## 1. What a declaration may say today

**Measured** — `crates/mc-world/src/content/luau_declaration/mod.rs:74`:
`RECOGNISED_FIELDS` holds **nine** names, in the order the documentation
introduces them: `name`, `texture`, `solid`, `replaceable`, `breakable`,
`breaks_into`, `drawn`, `occludes`, `targetable`.

**Every one of the nine is a string or a boolean.** There is no numeric field,
so `move_resistance` is the first, and the loader has no numeric reader to
extend — `required_boolean` and `optional_boolean`
(`luau_declaration/mod.rs:238,264`) are the whole of its value vocabulary.

**Measured** — the refusal for an unrecognised field quotes the whole list back
(`FieldFault::unrecognised`, `mod.rs:320`), and three pages under
`docs/modding/` quote that refusal verbatim: `README.md` (1 occurrence),
`blocks-items.md` (2), `hot-reload.md` (1). `documented_refusals.rs` compares a
quoted refusal against a real run line for line, so growing the list edits four
places in three files or the guard reddens.

## 2. Why `replaceable = 1` is refused rather than coerced

**Measured** — `optional_boolean` returns `FieldFault::wrong_kind` for any
`ScriptValue` that is not `Boolean`, and `kind_of` (`mod.rs:390`) renders both
`ScriptValue::Integer` and `ScriptValue::Number` as `a number`. So a mod author
writing `replaceable = 1` is told *"`replaceable` must be true or false, but is
a number"*. `docs/modding/blocks-items.md` documents that refusal.

**Derived**: the same rule inverted is what a numeric field owes. A
`move_resistance = true` must be refused rather than read as 1.0, and
`move_resistance = "1.0"` must be refused rather than parsed — the loader's
standing position is that a value of the wrong kind is a mistake an author makes
once, and coercing it is how they never find out.

## 3. Both Luau number variants reach the host

**Measured** — `ScriptValue` (`crates/mc-script/src/value.rs`) carries
`Integer(i64)` and `Number(f64)` as separate variants, and `kind_of` folds them
to one word for a refusal. Luau writes `move_resistance = 1` as an integer and
`move_resistance = 1.5` as a number, so a reader accepting one variant only
would refuse half of the values the documentation shows.

## 4. `0/0` and `1/0` are expressible in Luau

**Derived** from the language: Luau evaluates `0/0` to NaN and `1/0` to
positive infinity, both of which arrive as `ScriptValue::Number`. A finiteness
check is therefore reachable from content rather than decorative, and a NaN
resistance reaching the physics poisons a position permanently — the same
failure mode `requested_walk` (`crates/mc-sim/src/player/physics.rs:139`)
already guards against for a client-supplied intent.

`standards/global/testing.md` §5 puts validation rules at 100% coverage.

## 5. Fold membership — re-derived, not inherited

**Measured** — `crates/mc-world/src/persistence/format.rs:294`:
`BEHAVIOUR_REVISION` is **2** and `APPEARANCE_REVISION` is **3**. PRO-904 moved
the behaviour byte 1→2 when it added `targetable`, and the appearance byte 2→3
when it added `drawn` and `occludes`.

**Derivation** against the two lists' own doc comments:

- `DeclaredAppearance` (`format.rs:355`) admits a field when *"a block whose
  texture changed is the same block to stand on"* — a change no player has to
  decide anything about.
- `DeclaredBehaviour` (`format.rs:296`) admits a field when it changes what the
  world does to a player: `targetable` is there because *"a block that becomes
  aimable is a different block to stand in front of"*.

`swimmable` and `move_resistance` decide whether a player sinks, rises or is
slowed by a block's volume. Nothing about them is visible in a still frame and
everything about them changes what happens when you walk into the block. They
join **`DeclaredBehaviour`**, appended in that order after `targetable`, and
`BEHAVIOUR_REVISION` moves **2 → 3**.

**This agrees with the issue's claim and costs more than the issue assumed.**
The issue was written before PRO-904 shipped and asked whether the byte had
already moved. It has. So this is the **second consecutive** behaviour-byte move,
and every save written under revision 2 — which is every save written since
PRO-904 — reports every block it holds as `changed` on its next load, for the
second time. That cost is payable for the same reason PRO-904's was: such a save
loads and names its blocks rather than being refused.

## 6. The physics reads bitsets, never the registry

**Measured** — `crates/mc-sim/src/replay/resolved.rs:104`: `ResolvedVoxels`
carries two `Bitset`s, `solid` and `targetable`, resolved once at construction
so that every tick query is a bounds test and a bit test with no failure to
swallow. `World` (`crates/mc-sim/src/world/mod.rs:357,370`) forwards both.

**Derived**: `swimmable` is a bit and fits that shape unchanged.
`move_resistance` is **not** — it is a value per voxel, and a third bitset
cannot carry it. At the shipped world's 64 × 64 × 256 = 1 048 576 voxels, one
bit per voxel is 128 KiB and one `f32` per voxel is **4 MiB**, 32× the cost of
either existing view. How the value reaches the tick without paying that, and
without putting a registry lookup on the tick path, is the one thing this spec
declares an architecture delta for.

## 7. The shipped sea

**Measured** — `crates/mc-sim/src/replay/world.rs:33`: `SEA_LEVEL` is 34, and
water fills `(surface + 1)..=SEA_LEVEL`, so the topmost water voxel occupies
`[34, 35)` on y. PRO-904's architecture recorded 178 water voxels over 131
submerged columns, `y ∈ [33, 34]`, every top open to air.

**Derived**: the player's box is `[feet, feet + 1.8)` on y
(`crates/mc-sim/src/player/collide.rs:36`). A player whose feet sit anywhere in
`[33, 35)` overlaps a water voxel; feet at 35 or above overlap none. So a player
rising under its own effort in the shipped sea settles with its feet in
`[34, 35)` and cannot rise past it — the bound is derived from `SEA_LEVEL` and
the box, not chosen.

## 8. Gravity, and what a medium has to compete with

**Measured** — `crates/mc-sim/src/player/physics.rs`: `TICK_DURATION` 1/60 s,
`WALK_SPEED` 4.5 blocks/s, `GRAVITY` 30.0 blocks/s², `JUMP_SPEED` 9.0
blocks/s, `TERMINAL_SPEED` 48.0 blocks/s, `DISPLACEMENT_LIMIT` 1.0 block per
axis per tick.

**Derived**: gravity takes 0.5 blocks/s off the vertical velocity each tick, so
any swim-up rate has to exceed that to make progress. `launched`
(`physics.rs:186`) honours a jump *only* from ground contact, which is precisely
what makes a submerged player unable to rise today.

## 9. What content ships

**Measured** — `content/base/blocks/` holds four declarations: `dirt.luau`,
`grass.luau`, `stone.luau`, `water.luau`. Only `water.luau` states more than its
collision. **Derived**: three of the four state neither new field and must keep
meaning exactly what they mean now, which is what the constant defaults below
are for.

---

## Clarifications

- [resolved] Q: One number or two — is the field `density` or `move_resistance`?
  → A: One, named `move_resistance`. Owner ruling recorded in PRO-957,
  2026-08-22: drag and density are independent properties and one number cannot
  express both, the name is what mod authors write, and `density` is reserved
  for mass-per-volume if buoyancy is ever simulated.
- [resolved] Q: Which fold do the two new fields join, and has the behaviour
  byte already moved? → A: Behaviour, and yes — it is at 2 and moves to 3.
  Derived in §5 above from the folds' own doc comments rather than inherited
  from the issue, which was written before PRO-904 shipped.
- [resolved] Q: Does `swimmable` default to solidity, the way `drawn`,
  `occludes` and `targetable` do? → A: No. That derived default exists only to
  preserve the meaning of declarations written while one bit answered four
  questions. No bit ever answered this one, so a constant default is right and a
  derived one would invent a claim nobody made.
- [assumed] Q: What scale is `move_resistance` on, and what are its bounds? →
  A: A finite number ≥ 0, absent meaning 0.0, where the speed a player moves
  through the block's volume is divided by `1 + move_resistance`. 0.0 leaves
  movement untouched (which is what every existing declaration means by silence)
  and larger values slow it without bound and without ever reversing it. Assumed
  rather than resolved: the issue names the mechanic and not the scale.
- [assumed] Q: How does a player rise in a swimmable medium? → A: By holding
  jump, honoured every tick while the box overlaps a swimmable voxel rather than
  only from ground contact. Assumed from the issue's own prior art note —
  *"Neither simulates buoyancy; you rise in water by holding jump"* — and from
  the owner ruling, which leaves no second number for a buoyancy force.
- [assumed] Q: Is fall damage, drowning, or an oxygen meter in scope? → A: No.
  There is no damage or health system in the tree to attach any of them to.
  Recorded in Out of Scope.
