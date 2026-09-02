# Requirements — SPEC-032

Source: Linear **PRO-998** ("Underwater draws no water at all: a submerged
camera sees an untinted world"), an owner playtest of `de56ad2` (SPEC-031, the
sea passes half the light), and the measurements below taken on `de56ad2`.

## The request

The owner played the game after the sea became see-through and reported:

> "There is no tint / rendering applied when the camera is below the water
> surface. Everything looks like there is no water at all. Make sure to
> properly render the water when underwater."

Standing steering from the same owner, given on the preceding water spec and
still authoritative here:

> "Check how other games do it."

## This is not a regression, and the spec that shipped it says why

`crates/mc-client/tests/a_camera_inside_the_sea_tints_nothing.rs` is a
**shipped, deliberate, passing** test whose name is the defect the owner
reported. Its header states the mechanism:

> "The run the eye stands in draws nothing: the eye is past its entry face, and
> the exit face has its normal along every ray that leaves the eye and is
> back-facing. Nothing else in the world is water. So the sea contributes no
> fragment to this frame whatever degree it declares, and *identical* is the
> right expectation rather than a lucky one. Measured: 0 differing pixels of
> 921 600."

SPEC-031 listed the consequence in its own **Out of Scope**: "Underwater fog or
a submerged colour grade — no tint is applied to the frame *as a whole* when the
camera stands inside a translucent volume", pinned by FR-4.3-S1. So the engine
is not broken relative to its contract; **the contract is wrong about what water
is**, and this spec replaces that clause.

The test is correct about what the engine does and its name becomes a false
statement the moment this lands. It is replaced by a successor asserting the
opposite; nothing is deleted before the successor exists.

## What the fix cannot be

**Backface culling is not the thing to remove.** Making the sea's interior faces
draw would put a blue pane at arm's length in front of the camera, would need
`merges_with_self` (deferred by SPEC-031, a scope change rather than an
absorption), and still would not tint the sky. **Submersion has to be a state
the frame knows about**, not a consequence of geometry the eye is inside.

## Measured, on `de56ad2` (2026-08-31)

Each line names the command that produced it. Nothing here is relayed.

| Claim | Command | Reading |
|---|---|---|
| Nothing in a block declaration says anything about a tint | `grep -nic "tint\|fog\|medium_color" crates/mc-core/src/block/definition.rs` | `0` |
| Nothing in the renderer knows about a medium | `grep -rni "fog\|submerged\|underwater" crates/mc-render/src --include=*.rs \| wc -l` | `0` |
| Nothing in the workspace does | `grep -rnil "underwater" crates --include=*.rs` | no files |
| A declaration may state thirteen fields | `grep -n "const RECOGNISED_FIELDS" crates/mc-world/src/content/luau_declaration/mod.rs` | `92: const RECOGNISED_FIELDS: [&str; 13]` |
| The scene revision stands at `r5` | `grep -n "SCENE_REVISION: &str" crates/mc-render/src/capture.rs` | `74: pub const SCENE_REVISION: &str = "r5"` |
| Three terrain captures and one HUD capture are judged | `grep -n "DECLARED_CAPTURE_TICKS\|HUD_CAPTURE_TICKS" crates/mc-render/src/capture.rs` | `[0, 59, 119]` and `[0]` |
| No judged frame's camera is submerged, and it is asserted | `grep -n "stands_in_open_air" crates/mc-client/tests/replay_oracle.rs` | `324: fn the_camera_of_every_judged_frame_stands_in_open_air` |
| The sky colour is a Rust constant, not content | `grep -n "CLEAR_COLOR_SRGB" crates/mc-render/src/color.rs` | `42: pub const CLEAR_COLOR_SRGB: [u8; 3] = [135, 206, 235]` |
| Both save revision bytes stand at 4 | `grep -n "^const BEHAVIOUR_REVISION\|^const APPEARANCE_REVISION" crates/mc-world/src/persistence/format.rs` | `316: 4` and `317: 4` |
| The eye stands 1.62 blocks over the feet | `grep -n "EYE_HEIGHT: f32" crates/mc-sim/src/player/mod.rs` | `30: pub const EYE_HEIGHT: f32 = 1.62` |
| The sea declares half opacity and no colour of its own | `tail -14 content/base/blocks/water.luau` | `opacity = 0.5`, thirteen fields, none a colour |

### The shipped sea, recorded rather than re-measured

`a_camera_inside_the_sea_tints_nothing.rs` records the sea as **178 cells, 47 at
height 33 and 131 at height 34** — one to two deep, with no cell holding water
on all six sides. That figure is quoted from the file rather than re-taken here.

### Whether a player can reach submersion at all, derived from the above

A standing player's eye is `EYE_HEIGHT = 1.62` blocks over its feet, and a
swimmer asking for nothing sinks (`content/base/blocks/water.luau`), so a player
in the sea comes to rest with its feet on the lakebed. Where the sea is **two**
cells deep the surface stands 2.0 blocks over the bed and the eye stands at
1.62 — **submerged by 0.38 blocks**. Where it is one deep the eye stands at 1.62
over a surface 1.0 above the bed and is dry. So the 47 two-deep columns are
where a player's own eye goes under, and the 131 one-deep cells are not.

**This is arithmetic over two recorded numbers, not an observation**, and it is
the premise of the player-facing half of this spec. A phase that can run the
world should confirm it; a specify phase cannot.

## Clarifications

- [resolved] Q: Is this a fix against broken behaviour, or a feature against a
  contract that is wrong? → A: a feature. The behaviour is shipped, asserted and
  documented as intended; what changes is what the engine claims water is.
- [resolved] Q: What rigor? → A: `high`, by owner ruling. It adds a
  content-declared field that mod authors write (a published API surface) and it
  can change what a judged golden frame contains (a format bump). Either trigger
  alone is the project's `high` floor.
- [resolved] Q: Which stakeholder does this spec commit to? → A: the **player**,
  who goes under and sees water. The **mod author** is reached by the same field
  and named as the second; the spec fails if the player cannot get there.
- [resolved] Q: Does the tint colour live in Rust? → A: never. Invariant 1. It
  is declared in `content/base/blocks/water.luau` beside `opacity`, and a block
  that passes light and declares no tint tints nothing.
- [resolved] Q: Is a flat whole-frame colour filter enough? → A: no, and this is
  what "check how other games do it" answers. Every reference implementation
  makes the medium **thicken with distance** rather than applying a constant
  wash: Minecraft replaces the fog colour, collapses the fog distance and draws
  no sky while the camera is in a fluid; the Source engine declares a fog colour
  and a start/end distance **per water volume in the map**, content-declared in
  exactly the shape Invariant 1 asks for. A wash reads as a colour grade;
  distance is what reads as a medium, and what makes the sky read as water.
- [assumed] Q: Does the tint reach the HUD? → A: no. A tinted crosshair is the
  tell that a submerged overlay was applied at the wrong point in the frame.
  Stated as a scenario so it is guarded rather than assumed.
- [assumed] Q: What decides "the eye is in the medium"? → A: the block occupying
  the eye's cell, read the way every other voxel question is read. Not the
  player's feet, not a volume test, not a surface height.
- [open] Q: Does `SCENE_REVISION` bump `r5` → `r6`? → A: **to be settled by
  measurement, not by argument.** No judged frame's camera is submerged (asserted
  above), so a correct implementation should leave all four committed captures
  byte-identical, and *that* is the deliverable reading. A bump is owed only if
  the vertex format, the merge predicate or another scene-contract item moves —
  which the chosen mechanism decides, so the architect phase closes this. A bump
  renames by **deletion and a fresh mint, never `git mv`**.
- [resolved] Q: Does this spec add a judged capture whose camera is submerged? →
  **No.** A golden is a photograph of whatever shipped and cannot tell a correct
  tint from a wrong one; what can is a per-pixel derived reading at a declared
  pose, which is what the superseded test already is. Derive, not photograph.
- [open] Q: Does the eye's medium arrive from the server or is it computed
  where the frame is drawn? → For the architect. Invariant 4 makes the server
  authoritative over what the world holds; what colour a client paints its own
  frame is not a claim about the world. Carried into the spec as Architecture
  Delta item A.
- [resolved] Q: How does the medium thicken with distance — a linear ramp to a
  stated distance, or an exponential density? → A: **a linear ramp**,
  `min(1, d / D)`. "You can see twelve blocks through this" is a statement an
  author can make and a reading can check; an exponential never fully hides
  anything, so no distance an author writes would mean what it says. The
  parameterisation is the field's semantics and is fixed in the spec; only the
  spelling is the architect's.
- [resolved] Q: Two shipped readers take colours — eight digits in the HUD, six
  in voxforge — and each refusal claims its own form is the only one. Which does
  a tint take? → A: **both**, case-insensitive, with an alpha other than `FF`
  refused because a tint has no alpha to state. Content already speaks both
  dialects, so this adds no third rule. Reconciling the two is **PRO-999**.
- [resolved] Q: Is the shipped sea deep enough for a player's eye to go under? →
  A: derived as yes, in 47 columns, by 0.38 blocks — and the derivation is no
  longer only an assumption: FR-2.6-S1 asserts it and FR-2.6-S2 is its control.
