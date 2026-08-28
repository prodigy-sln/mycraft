# Requirements — SPEC-031

Source: Linear **PRO-993** ("Water is not transparent: the terrain pipeline
has no blended pass"), owner playtest of the `0fdba08` bundle, and the
measurements below taken on `c8d99ca`.

## The request

The owner played the game and reported, of the sea:

> "Also it's not transparent at all."

He is right, and it is not a defect. **Water was never capable of being
translucent**: no block can declare that light passes through it, and the
terrain pipeline could not blend it if one did.

## Measured, on `c8d99ca` (2026-08-27)

Each line names the command that produced it. Nothing here is relayed.

| Claim | Command | Reading |
|---|---|---|
| The terrain pipeline cannot blend | `grep -n "blend" crates/mc-render/src/gpu/pipeline.rs` | one hit, `272: blend: None` |
| The HUD is the only blended target in the workspace | `grep -rn "blend: Some" crates/ --include=*.rs` | one hit, `crates/mc-render/src/gpu/hud_pass.rs:269` |
| No block declares translucency | `grep -nic "translucen\|opacity\|alpha_cutoff" crates/mc-core/src/block/definition.rs` | `0` |
| The scene revision stands at `r4` | `grep -n "SCENE_REVISION" crates/mc-render/src/capture.rs` | `55: pub const SCENE_REVISION: &str = "r4"` |

`BlockDefinition` declares `is_solid`, `replaceable`, `breakable`,
`breaks_into`, `drawn`, `occludes`, `targetable`, `swimmable`,
`move_resistance`, `swim_ascent` — and nothing about how much light passes
through. `occludes = false` stops water hiding the lakebed **from the
mesher**; it says nothing about the pixels water itself draws.

## Two standing debts this spec is the named discharger of

Both were written into the tree by earlier specs, against this work by name.
Neither was in the brief; both were found by reading.

1. **`crates/mc-render/src/texture/mip.rs:74-78`** — "No test here
   discriminates the alpha treatment: every texture this increment ships is
   opaque, and both treatments answer 255 for a constant 255. **The first
   translucent texture must bring a test with it.**" Colour is averaged in
   linear light and alpha is averaged where it stands; nothing today can tell
   a correct alpha mip from a wrong one.
2. **`crates/mc-client/tests/support/oracle.rs:55-72`** — the replay oracle
   marches to the first *drawn* voxel and calls that the pixel's subject.
   That is right "only while every drawn block is opaque", and the comment
   names its breaker. **This spec is the breaker**, whatever issue key the
   comment cites.

A third site, `crates/mc-world/src/mesh/sweep.rs:285-290`, names
`merges_with_self` as the field that would let content ask for the interior
faces of its own translucent volume. Whether this spec needs it is an
architecture question, recorded below.

## Clarifications

- [resolved] Q: PRO-952 ("Blocks declare an engine render method; liquids get
  inset and wave") covers transparency in its `liquid` method and has a
  planning document at `docs/planning/block-render-methods.md`. Is this spec a
  duplicate of it? → A: **No, it is a carve-out.** PRO-952 is `Backlog`;
  PRO-993 is `In Progress` and is scoped by its own body to "a
  content-declared translucency, an alpha channel that survives baking and
  mipping, and a draw ordered after the opaque pass". The surface inset, the
  wave, and the `render = "<method>"` framing stay with PRO-952. Verified with
  `linear-cli i get PRO-952` and `linear-cli i get PRO-993`.
- [resolved] Q: The three code comments naming this work cite **PRO-952**, not
  PRO-993. Does that make PRO-952 the owner? → A: No. The comments name a
  *breaking condition* — "the day a translucent block exists" — and this spec
  is what makes one exist. The citation is stale, not authoritative. Fixing
  those three citations is in scope for this spec.
- [resolved] Q: Does the spec choose the draw-ordering model? → A: No. The
  Architecture Delta enumerates the space with its couplings; the architect
  phase chooses. Deciding it here would settle by assertion the one thing that
  is expensive to reverse.
- [assumed] Q: Does `SCENE_REVISION` move `r4` → `r5`? → A: **Assumed
  conditional on the architect's choice, not assumed true.**
  `docs/technical/rendering.md` binds the revision to the *scene contract*
  (pose, world, camera path, tick list, merge predicate, vertex format) and
  records two re-shoots — 2026-08-19 and 2026-08-26 — where art moved and the
  revision deliberately held. A blended pass alone is a renderer change; a
  mesher that partitions faces into opaque and translucent sets moves the
  merge predicate and a vertex that grows a translucency bit moves the vertex
  format. **The goldens are re-shot either way** (the sea is 9.58–21.57 % of a
  frame, measured in `rendering.md`); whether the directories are *renamed* or
  *overwritten* follows from the model. The spec states the rule; the
  architect's choice determines the outcome.
- [assumed] Q: What shape does the declaration take — a boolean, a scalar
  opacity, or an alpha-cutoff threshold? → A: Assumed a content-declared
  field on the block declaration, defaulting so that every declaration written
  before it goes on meaning what it meant (the pattern SPEC-025 set for
  `drawn`/`occludes`/`targetable`). The exact type and bound is an
  architecture question; the *refusal* it owes is specified here regardless.
- [assumed] Q: Which save-format revision byte moves? → A: Assumed
  **appearance**, not behaviour: how much light passes through a block does
  not change what it is to stand on. `docs/technical/world-format.md` and
  SPEC-025's ruling on `drawn`/`occludes` are the precedent. Confirmed shape
  is the architect's.
- [resolved] Q: Can the committed golden frames witness translucency at all? The
  scripted walk wades through the sea, so the camera is *inside* water at ticks
  59 and 119 — which tests the near plane and not the see-through. PRO-992
  measured that goldens are structurally blind to a declared value the replay
  never exercises. → A: **Yes, and it is measured rather than assumed.**
  `docs/technical/rendering.md` records the committed `r3` set drawing **77 987**
  water pixels at tick 0 of 921 600 — 8.46 % of the frame — and tick 0 is the
  *dry, inland* capture. So a golden does see the sea from outside it. What a
  golden cannot do is tell a correct blend from a wrong one: it is a photograph
  of whatever shipped, which is exactly how SPEC-029's stand-in survived every
  committed reference image. **The probes and the replay oracle are the derived
  instruments; the goldens bound only *when* anyone looks.** No scenario in this
  spec rests on a golden alone.
- [resolved] Q: Does a translucent block need to draw the interior faces of its
  own volume (`merges_with_self`)? A sea whose every internal face draws is a
  stack of visible sheets; a sea with none may be correct. → A: **Not by this
  spec.** FR-4.2-S2 fixes today's behaviour — two adjacent cells of one kind draw
  no seam — as the requirement. It is recorded as **coupling B2** in the
  Architecture Delta: if the architect's chosen ordering model needs interior
  faces, that is a **scope change to raise, not to absorb**, and the field is
  listed Out of Scope under exactly that condition.
