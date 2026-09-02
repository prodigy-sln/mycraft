# Architecture Delta — SPEC-032

The medium the eye stands in reaches the image.

Standing architecture is `docs/technical/architecture.md` and
`docs/technical/rendering.md`; nothing they already settle is re-derived here.
This document settles the four decision spaces the spec's `## Architecture
Delta` enumerated and left open (**A** where the eye's medium comes from, **B**
how the tint reaches a fragment, **C** whether `SCENE_REVISION` moves, **D**
which save revision byte moves), plus **E**, which the spec named without
enumerating. Every decision below is **BINDING** unless it says otherwise.

Everything asserted about the tree was read on `3c7a21b` and the file and line
are named where it matters.

---

## Drivers

1. **Invariant 1 above everything.** The colour and the distance are content.
   No Rust constant may stand in for either, no path may compare a block name,
   and a block that passes light and declares no tint must tint nothing. This
   rules out the cheapest version of every candidate below and is the reason
   §A does not simply read the eye's cell in the renderer.
2. **The law is fixed by the spec and is not an implementation choice.**
   `min(1, d / D)` in linear light, `d` radial from the eye. FR-2.1-S4 is the
   scenario that tells radial distance from view depth, and it is derived from
   the shipped camera rather than chosen — see §B.4.
3. **The seam holds.** `mc-render` names `mc-sim` in no dependency of any kind
   (`architecture.md` §"The simulation/renderer seam"). Whatever crosses does so
   through a type both crates can already see.
4. **The committed golden set must not move.** No judged camera is submerged, so
   a correct implementation leaves all four captures byte-identical. §B and §C
   are chosen so that the dry case is the *exact arithmetic identity* and not a
   value that merely rounds back — which is what makes FR-3.1-S1 a byte-for-byte
   claim rather than a perceptual one.
5. **The storage-buffer budget is a build-time portability contract.**
   `downlevel_defaults()` allows four per stage; the cull shader binds exactly
   four (`crates/mc-render/build/validate.rs:98`). Nothing here may add a fifth
   to either stage. A uniform costs none of that budget.
6. **The HUD is composited over the terrain frame in a later pass**
   (`crates/mc-render/src/gpu/hud_pass.rs`, `loaded()` — it loads rather than
   clears). Whatever tints must act inside the terrain pass. FR-2.5-S1 catches
   getting it backwards.

---

## New boundaries and dependencies

**No new crate dependency, in either direction.**

| Edge | Before | After | Note |
|---|---|---|---|
| `mc-core` → nothing | — | unchanged | `MediumTint` is a plain value type; `mc-core` still performs no I/O |
| `mc-world` → `mc-core` | present | unchanged | the loader constructs `MediumTint` and the save folds it |
| `mc-sim` → `mc-core`, `mc-world` | present | unchanged | resolves the eye's cell through the registry it already holds |
| `mc-render` → `mc-core`, `mc-world` | present | unchanged | consumes `mc_core::block::MediumTint`; still names `mc-sim` nowhere |
| `mc-client` → both | present | unchanged | copies one field from `SimSnapshot` to `TerrainSnapshot` |
| `tools/voxforge` | — | **untouched** | its six-digit reader and its own `Srgb8` stay where they are (PRO-999) |

The one type that crosses the seam is `mc_core::block::MediumTint`, which lives
in `mc-core` for exactly the reason `Opacity` does: it is a thing a declaration
states, both sides of the seam need to name it, and neither side may learn about
the other to do so.

---

## A. Where the eye's medium comes from — **A1, BINDING**

**The simulation resolves it and publishes it on `SimSnapshot`; the client
copies it onto `TerrainSnapshot`.**

**Options weighed.**

- **A1 — a field on `SimSnapshot`, carried into `TerrainSnapshot`.** The
  simulation already owns the world, the registry and the eye
  (`SimSnapshot.camera: CameraPose` is derived from the player at
  `crates/mc-sim/src/player/mod.rs:221`), so the question "what block is the eye
  in" is answered where every other voxel question already is. Costs one field
  on each of two existing types.
- **A2 — the client queries the world each frame.** Rejected: it makes the
  composition root a second reader of the world beside the snapshot, and gives
  it a rule — "the eye's cell decides" — that the simulation owns. It also
  breaks the property `architecture.md` states about the seam, that a consumer
  holding an `Arc<SimSnapshot>` sees one coherent instant: a tint read from the
  live world beside a pose read from a snapshot could disagree by a tick.
- **A3 — the renderer resolves it.** Rejected outright: the renderer holds
  geometry and no registry, so it would have to learn what a block is, and
  Invariant 1's "no block name on this path" would become unenforceable.

**`VoxelMedium` is deliberately *not* reused, and this is a decision rather than
an omission.** `VoxelMedium` (`crates/mc-sim/src/player/mod.rs:128`) is a
**join over the cells a body occupies** — `swimmable || swimmable`,
`resistance.max(resistance)` — with `NOTHING` as the identity of all three. A
tint is a property of the **one cell the eye is in**, and it has no lattice join:
two overlapping media do not produce a third colour, and inventing one (`max`?
average?) would be an engine rule about content that no author declared. Adding
a fourth field to `VoxelMedium` would also widen `MediumTable`'s packed index
for every voxel in the world (`crates/mc-sim/src/replay/medium.rs`) to carry a
quantity the physics never reads. The eye's tint is resolved on its own, from
the block name in the eye's cell, through the registry.

**The resolver.**

```
// crates/mc-sim — beside the player, or in the world module
pub fn eye_medium(world: &World, at: Vec3) -> Option<MediumTint>
```

It floors `at` on all three axes into a `BlockPos`, asks `World::block_at`
(`crates/mc-sim/src/world/mod.rs:141`), resolves the held `BlockName` through
`world.registry()`, and returns the definition's `tint`. Empty cell, cell outside
the world, unresolvable name: `None`.

**Flooring, and why it is exactly floor.** FR-2.4-S2 is the falsifier: an eye at
`y = 34.98` is inside the cell whose top face is `y = 35.0`, and an eye at
`y = 35.02` is not. Only a floor gives that. Two copies of that conversion
already exist — `crates/mc-sim/src/world/action/trace.rs:169` (`containing`) and
`crates/mc-sim/src/player/collide.rs:307` (`floor_voxel`). **This is the third use, which is the
point at which `code-quality.md` §1 stops forbidding the abstraction and starts
asking for it**: lift one `containing(point: Vec3) -> BlockPos` and call it from
all three. A third hand-written `floor() as i32` triple is refused.

**Where it is called: both publishers, and there are exactly two.**
`Simulation::new` (the first publish, `crates/mc-sim/src/simulation.rs:197`) and
`Simulation::advance` (every later one, `:240`). A tint computed only in
`advance` leaves the first frame of every session untinted, which no scenario
would catch because every FR-2 reading declares its own pose.

**BINDING: the tint is resolved from the live registry at every publish and
never cached across ticks.** This is what makes FR-4 free, and it is exactly the
thing an optimiser removes — "the eye's block rarely changes, cache it" passes
every FR-2 reading, because each declares its own pose and renders one frame,
while silently breaking FR-4.1-S1, S2 and S4, whose whole content is that a
*second* frame after a reload differs from the first. The reload path needs no
change **because of** this rule, not independently of it.

---

## B. How the tint reaches a fragment — **B1, BINDING**

**Mixed in the terrain fragment stage, with the frame's clear colour set to the
medium's colour.**

**Options weighed.**

- **B1 — the terrain fragment stage mixes, the clear carries the sky.** One
  uniform block, one varying, four shader lines, one per-frame clear-colour
  choice. Reaches the sky through the clear, which is the same mechanism that
  already puts the sky there.
- **B2 — a full-screen pass after terrain, sampling depth. REJECTED, and the
  reason is a measured property of the shipped pipeline rather than a cost
  argument.** `TerrainLayer::Translucent` **does not write depth**
  (`crates/mc-render/src/gpu/pipeline.rs:396`, and the reason is recorded there:
  a translucent face that wrote depth would discard a second one behind it). So
  at a pixel where a translucent face at `3.0` blocks stands in front of an
  opaque surface at `9.0`, the depth attachment holds `9.0` and nothing anywhere
  holds `3.0`. A depth-sampling post pass has one distance per pixel and can only
  apply one mix to the already-blended colour — carrying both layers by `9.0`,
  which is a **third** answer, neither FR-2.1-S6's stated outcome nor the wrong
  one it names.

  **No depth-sampling variant of B2 can satisfy FR-2.1-S6, and one non-depth
  variant can.** Stating that plainly, because a later reader who finds the
  counterexample on their own has cause to distrust the rest of this document:
  a **multiple-render-target** B2 — terrain writes a second attachment
  accumulating the src-over-blended tint weight `a·t_s + (1−a)·t_d` while
  attachment 0 accumulates the correspondingly de-weighted colour, and a
  full-screen pass adds `T · weight` — is *exact*, because `src-over` is linear
  in both terms. It is still rejected, and for a reason that does not depend on
  arithmetic: it edits `terrain.wgsl` anyway, which forfeits B2's only claimed
  advantage over B1, and it additionally costs a second colour attachment, an
  MRT pipeline change, and a change to the readback path in
  `crates/mc-render/src/gpu/readback.rs` that every committed golden is shot
  through. A depth-sampling B2 additionally costs a `TEXTURE_BINDING` usage flag
  on the depth attachment, a new pipeline and a new bind group, and still cannot
  produce the right answer.
- **B3 — a constant full-screen wash. REJECTED** by the spec's own law, and
  independently by FR-2.1-S1 through S6, each of which predicts a different
  colour at a different distance.

**B.1 — What the fragment computes.**

```wgsl
let texel  = textureSample(terrain_textures, terrain_sampler, input.uv, input.layer);
let toward = min(1.0, length(input.world_position - frame.eye) * frame.tint_reach);
let tinted = mix(texel.rgb, frame.tint_color, toward);
return vec4<f32>(tinted, texel.a * input.opacity);
```

Three things about that are load-bearing:

1. **`world_position` is a varying and `length` is taken in the fragment.**
   Interpolating the *distance* per-vertex would be wrong: perspective-correct
   interpolation is exact for functions that are affine in world space, and
   `length` is not one. Over a merged quad spanning several blocks the error is
   visible, and it is exactly the error FR-2.1-S4's `6.0` versus `6.74`
   prediction is shaped to catch. World position is affine; interpolate that.
2. **The tint carries no alpha and does not touch the fragment's own.** The
   declared degree still decides coverage, so under `src-over` a translucent
   layer tinted at its own distance blends over an opaque layer tinted at its
   own — which is FR-2.1-S6's stated outcome, reached for free rather than
   arranged.
3. **`tint_reach` is `1 / D`, not `D`, and the absence of a tint is the
   arithmetic identity.** A frame whose eye declares no tint carries
   `tint_reach = 0.0` and `tint_color = (0, 0, 0)`, so `toward` is exactly `0.0`
   and `mix(a, b, 0.0)` returns `a` bit-exactly. **There is no `if` and no
   second code path for the dry case**, which is what makes FR-3.1-S1 a
   byte-for-byte claim: the dry frame is not "a frame that skipped the tint", it
   is the same arithmetic with a factor of zero. The reciprocal is also what
   makes the loader's *exclusive* floor on `tint_distance` load-bearing rather
   than fastidious — `D > 0` is what makes `1 / D` **defined**.

   **Corrected during implementation: this said "finite", and finite is not
   what the floor buys.** A subnormal distance such as `1e-45` passes `> 0.0`
   and its reciprocal is an infinity, so `min(1, d · ∞)` is `1` at every
   `d > 0` — a frame drawn wholly at the declared colour. That is the *right*
   answer for a medium you can see `1e-45` blocks through, so the floor stays
   `> 0` and is not raised to `f32::MIN_POSITIVE`: doing so would refuse a value
   that behaves correctly, and would make the refusal's own words a lie, since
   `` `tint_distance` must be greater than zero `` would then mean "at least
   1.18e-38". The bound's two real reasons are that zero is a claim no author
   means, and that a defined reciprocal is what removes the branch. Recorded
   durably in ADR-032 and in `MediumTint::new`'s own documentation, because this
   folder is archived and neither of those is.

   **This is derived, not assumed.** `length()` over a world of this extent
   cannot overflow, so `finite × 0.0` is `±0.0` and `min(1.0, ±0.0)` is `0.0`;
   and `a` comes back exactly under all three forms a backend compiles `mix`
   into — `a(1−t) + bt = a + 0.0`, `a + (b−a)t = a + (−0.0)`, and
   `fma(b−a, t, a)`. **BINDING, because the derivation rests on it:
   `tint_reach` is written as the literal `0.0` where there is no tint**, never
   as the reciprocal of a sentinel distance. The frame's byte-identity needs one
   further thing beyond the mix, which §B.5 supplies: the clear falls back to
   `config.clear_color_linear` untouched.

   The reciprocal costs one unit in the last place against `d / D`, which is
   about seven decimal digits below the `1/255` an 8-bit channel can express.
   The derived oracle computes `min(1, d / D)` directly and the difference is
   far inside any tolerance a frame reading can state.

**B.2 — What crosses into the shader.** The `Frame` uniform grows from 160 to
**192 bytes**:

| offset | field | was |
|---|---|---|
| 0 | `view_projection: mat4x4<f32>` | present |
| 64 | `planes: array<vec4<f32>, 6>` | present |
| 160 | `eye: vec3<f32>` | new |
| 172 | `tint_reach: f32` — `1/D`, zero where the eye declares no tint | new |
| 176 | `tint_color: vec3<f32>` — **linear** light | new |
| 188 | four bytes of declared padding | new |

`FRAME_UNIFORM_BYTES` in `crates/mc-render/src/gpu/buffers.rs:114` becomes
`64 + 96 + 32`. All three new fields sit **after** everything the cull shader
reads, so `cull.wgsl`'s `Frame` struct stays a valid prefix and is left
untouched — the compute stage neither declares nor reads them. `min_binding_size`
is `None` and the whole buffer is bound (`pipeline.rs:145`, `:227`), so a
192-byte buffer satisfies a 160-byte declaration.

**BINDING: `struct Frame` joins the build-time validator, in both shaders.**
The `Frame` uniform is a hand-laid-out CPU/GPU record written in three places —
`frame_uniform_bytes` (`crates/mc-render/src/gpu/record.rs:93`),
`terrain.wgsl:20` and `cull.wgsl:62` — and nothing checks the three against each
other. At two fields a mismatch was nearly inconceivable; at six it is a
plausible wrong picture with no error anywhere, because `min_binding_size: None`
catches only an *undersized* buffer, not a CPU that writes `tint_color` where the
shader reads `eye`. This spec also introduces a wholly new unenforced invariant
— *cull's struct is a valid prefix of terrain's* — which is exactly the class of
silent drift `build/validate.rs` exists for. The mechanism already exists and is
copied rather than invented: `SECTION_RECORD_DECLARATION`
(`crates/mc-render/build/validate.rs:126`) does this for `struct Section` in both
shaders, against a `validate_tables` copy. `struct Frame` gets the same
treatment, with the byte offsets and `FRAME_UNIFORM_BYTES` as the values
compared.

This is verification arriving with the thing it verifies (Invariant 5) rather
than a general clean-up: the check is over the record *this* change grows, and
it is the only instrument that can see the prefix invariant at all. It is hours
of work, not days, and the fallback if it proves otherwise is one unit test in
`crates/mc-render/tests/shader_validation.rs` asserting `FRAME_UNIFORM_BYTES`
against naga's derived size for `terrain.wgsl`'s `Frame` — weaker, since it sees
the size and not the field order, and to be taken only with the reason recorded.

**B.3 — Bindings.** `frame_bindings` in
`crates/mc-render/src/gpu/pipeline.rs` currently declares binding 0 as
`ShaderStages::VERTEX`. It becomes `VERTEX | FRAGMENT`. **No storage buffer is
added to either stage**, so the four-per-stage budget and the build-time check
that enforces it are untouched. This is the whole of the budget argument and it
is why B1 was affordable.

**B.4 — Radial, and how FR-2.1-S4's number is derived.** The camera is
`fov_y_radians = 60°` (`crates/mc-render/src/camera.rs:97`) over a 1280×720
capture. Half the horizontal field is `atan(tan(30°) · 16/9) = 45.75°`; a
quarter of the frame's width from centre is half of that half-width in *tangent*,
`θ = atan(0.5 · tan(45.75°)) = 27.17°`; and `6.0 / cos θ = 6.74`. The spec's
number reproduces. A depth implementation draws `6.00` there and is rejected —
which is what the reading is for.

**B.5 — The clear colour.** `TerrainPassConfig::clear_color_linear` **stays** and
keeps meaning the dry sky, so `TerrainPassConfig::offscreen()` and `windowed()`
still differ in colour format alone and `pass.rs`'s "one descriptor, two paths"
property is untouched. The per-frame choice happens where the attachment is
built, in `crates/mc-render/src/gpu/record.rs`:

```
the frame's clear = snapshot.tint.map(decoded linear).unwrap_or(config.clear_color_linear)
```

A `ClearOnly` frame — the scene is still being prepared — carries whatever tint
the snapshot carries, which is `None` before there is a world. Nothing special is
written for it.

**This is not the one-line change it reads as, and the cost is priced here
rather than discovered.** `record::draw` takes `(renderer, target, terrain)` and
has no snapshot (`crates/mc-render/src/gpu/record.rs:127`); the frame's tint has
to be threaded in, which is a fourth parameter — at `clippy.toml`'s cap, not over
it. And `clear_color` is a `const fn` (`crates/mc-render/src/gpu/mod.rs:325`),
while `Option::map` is not const, so it loses `const` or gains a second form.
Both are mechanical; neither is free.

This is what satisfies FR-2.2-S1, and it is also what makes the far field
seamless: a surface at or beyond `D` is drawn wholly at the tint by the mix, and
a pixel with no surface at all is drawn at the same colour by the clear. **The
two must come from one decode** — the sky and the far terrain disagreeing by a
transfer function is the "agreement between two wrong things" failure
`rendering.md` already records at this exact site. See §E.

---

## C. `SCENE_REVISION` — **does not move. BINDING, with the condition it was
conditional on discharged.**

`SCENE_REVISION` is bumped when a change makes a same-named capture
*incomparable* with the frames committed under that name, and
`crates/mc-render/src/capture.rs` names the two causes on record: the **mesh
contract** and the **declared camera path**. `r5` is the narrow case worth
checking against — it was bumped for the *packed vertex format* with no pixel
moving, because a packed vertex is a contract item a capture is a photograph of.

Item by item, for the mechanism chosen above:

| Scene-contract item | Moved? | Why not |
|---|---|---|
| Pose | no | no change to the intent script, the spawn, or `eye_pose` |
| World | no | worldgen untouched |
| Camera path | no | the path is the simulation's output; the tint enters no physics. `changes_geometry` is keyed positively on `drawn`/`occludes`/textures (`crates/mc-sim/src/world/reload.rs:73`), and the medium fields are already outside it by construction |
| Tick list | no | `DECLARED_CAPTURE_TICKS` and `HUD_CAPTURE_TICKS` untouched |
| Merge predicate | no | the tint is per-frame and per-eye; no face carries it, so no two faces stop merging |
| Vertex format | no | `PackedVertex` is untouched. The new `world_position` is an **inter-stage varying**, computed in the vertex stage from data it already reads — it is not a vertex-buffer attribute and nothing photographs it |

**The argument that actually carries this is operational, and it is not the one
about the `Frame` uniform.** The weak version — "the uniform's growth is
per-frame draw state of the same kind the clear colour already is" — does not
survive contact with `capture.rs:56`: the clear colour has never been a
hand-laid-out byte record shared with a second shader, and after §B.2 the `Frame`
record is one *and* build-time checked, which is the same class of thing `r5`
bumped for. The argument that does carry it:

**A bump is a rename by deletion and a fresh mint, and the freshly minted set
would be byte-identical to the deleted one.** The revision exists so that frames
which *cannot* be compared are not silently compared. Here they can be compared,
and the comparison **is the deliverable** — FR-3.1-S1 is the only thing standing
between this spec and a tint leaking into a dry frame. Bumping would delete that
evidence and replace it with a set comparable only against itself. A revision
taken where comparability is intact does not protect anything; it destroys the
one reading that was going to prove the change was safe.

**A standing condition, recorded rather than left implicit.** The next change to
the `Frame` record does *not* automatically owe a bump either: `r5`'s cause was
that a packed vertex is geometry a capture is a photograph of, and being
build-time checked is not what made it one. The rule stays the one `capture.rs`
states — a bump is owed when a same-named capture becomes **incomparable** with
the frames committed under it. Whoever next changes this record answers that
question afresh, and by rendering, not by classifying.

**And the expectation is not the deliverable.** FR-3.1-S1 is: every capture
committed before this spec's first implementation commit, compared byte for byte
against a frame rendered from the tree carrying the sea's declared tint. If any
of them moves, the bump is owed, and the rename is by **deletion and a fresh
mint, never `git mv`** — measured on the 2026-08-27 re-shoot, where a `git mv`
passed the comparison, `golden_mismatch` and `golden_inventory` while two
directories still held sidecars naming `r3`.

**The condition that would make the reading vacuous, and the constraint that
closes it.** `TerrainSnapshot` is built at **10 struct-literal sites across 10
files**, one of them the `mc-client` test helper `support/frames.rs:185`, which
has eleven callers of its own. **Corrected during implementation from "16 across
11 files", which counted grep lines rather than constructions**: of the sixteen
lines `TerrainSnapshot {` matches, one is the declaration and five are
`-> TerrainSnapshot {` return-type signatures, which no added field breaks. The
compiler's own census settles it — with `tint` on the struct and the production
sites filled, `cargo clippy --workspace --all-targets --all-features
--keep-going` names exactly nine `E0063` sites, one per test-owned literal. If
the golden shooter hard-codes `tint: None`, the
committed set is shot down a path where the tint is permanently off, FR-3.1-S1
passes about a renderer that cannot tint, and nothing reddens.

**BINDING, and deliberately narrower than "every capture path":**

1. **The golden shooter and every FR-2 / FR-3 reading derive their tint by
   calling §A's resolver against the world and eye they are drawing.** So the
   answer is `None` because
   `the_camera_of_every_judged_frame_stands_in_open_air`
   (`crates/mc-client/tests/replay_oracle.rs:324`) says the eye is in open air —
   not because a fixture said so. This costs almost nothing where it matters:
   `crates/mc-client/tests/support/goldens.rs:104` already holds
   `prepared.world` and `prepared.registry` two lines above the call.
2. **`App::snapshot` (`crates/mc-client/src/app/mod.rs:374`) reads the published
   field** and computes nothing of its own.
3. **`mc-render`'s own offscreen tests pass `None`, and that is correct rather
   than a fixture lie.** `crates/mc-render/tests/{terrain_offscreen.rs:414,
   frame_statistics.rs:90,support/mod.rs:313}` hold **three** of the ten sites —
   corrected from six, which counted each file's construction and its helper's
   return type as two — in a
   crate that may not name `mc-sim` and holds no world and no registry. They
   *cannot* call the resolver, and they draw no world to resolve against. Saying
   "every capture path" would send an implementer into the seam.

**Cost the table must carry.** `frames::snapshot(tick, camera, scene)` is three
parameters; threading a world and a registry makes five, over `clippy.toml`'s
`too-many-arguments-threshold = 4`. A **second constructor** —
`frames::snapshot_in(&prepared, tick, camera, scene)`, taking the prepared scene
it already has — is what rule 1's callers use, and the existing three-parameter
form stays for everything that draws no world.

This is the "what does the shipped caller supply, and which shipped path reaches
this" question from `testing.md` §2, asked before the fixture is written.

---

## D. Which save revision byte moves — **the appearance byte, 4 → 5. BINDING.**

`DeclaredAppearance` gains the tint, **appended after `opacity`**, and
`APPEARANCE_REVISION` moves from 4 to 5. `BEHAVIOUR_REVISION` stays at 4.

The test `format.rs` states for the split is whether the field changes what it is
to *stand on, build through or break* a block, and a colour seen from inside
changes none of those — it is the same test that put `drawn`, `occludes` and
`opacity` on this list, and the same one that kept `swimmable`,
`move_resistance` and `swim_ascent` off it. Routing a rendering field through the
behaviour byte tells every player holding any block that it behaves differently,
for a change nobody can act on.

**The record.** Appended as one `Option`, not two fields:

```rust
struct DeclaredAppearance<'a> {
    …
    opacity: f32,
    tint: Option<DeclaredTint>,   // postcard: 0x00, or 0x01 + 3 bytes + f32 LE
}
```

Two reasons it is one `Option` rather than a colour field and a distance field:
the loader's "both or neither" rule (FR-1.4) is then expressed in the record's
own shape and cannot drift from it, and a single tag byte is what distinguishes
"declares no tint" from every colour, including black at any distance.

**What is folded is what the declaration stated.** The three sRGB channel bytes,
exactly as `opacity` folds the `f32` a declaration stated and never the byte a
vertex carries. This is what makes the case-insensitivity and the two accepted
spellings free: `#3A6EA5`, `#3a6ea5` and `#3A6EA5FF` parse to the same three
bytes and therefore fold identically, so three spellings of one colour cannot
report a block as retextured — the same property the `-0.0 → 0.0` normalisation
buys for the numbers. The distance folds as its `f32` bit pattern at the width
the engine keeps, through the same reader that already normalises `-0.0`.

FR-5.1-S3 is the control: leaving the byte at 4 while the list grows must be
reported by the guards that build the expected byte sequence by hand, because
every comparative witness compares one fold to another and cannot see a leading
byte that moved in neither.

---

## E. The colour's representation across the seam — **sRGB crosses; the decode
stays in `mc-render::color`. BINDING.**

`MediumTint` carries the three **sRGB-encoded** channel bytes the declaration
states, unchanged, from the loader through `BlockDefinition`, `SimSnapshot` and
`TerrainSnapshot`. `crates/mc-render/src/color.rs` remains "the one place an
sRGB colour becomes a linear one", and it performs the decode once per frame,
feeding **both** the clear colour and the shader uniform from that one result.

- The alternative — decoding in the loader and carrying linear floats — puts a
  colour-space conversion in `mc-world`, gives the engine two places that know
  the transfer function, and makes the value a mod author reads in their file
  no longer the value any later reader sees.
- `srgb8_to_linear` returns `[f64; 3]` because `wgpu::Color` is `f64`. The
  uniform narrows that result to `f32`. **It narrows the same result**, so the
  sky and the far terrain cannot drift apart by a transfer function — which is
  the failure `rendering.md` §"One pass configuration, two targets" records at
  precisely this site: "a unit test of the conversion and a test comparing the
  two configurations to each other can both pass while every shipped frame is
  wrong". Only an assertion on a captured frame closes it, and FR-2.1-S2 (a
  surface at or beyond `D`) and FR-2.2-S1 (a pixel with no surface) are the two
  halves of that assertion — they predict *the same colour by two routes*, which
  is why they must not be allowed to share the decode's output through the
  fixture.
- **The residual, named so a future reader knows where to look.** "One decode
  feeds both" closes at the `f64`. The two consumers narrow that `f64` to `f32`
  by different code — ours with an `as`, `wgpu`'s inside its own backend — and
  both are round-to-nearest of the same value, so they agree. If FR-2.1-S2 and
  FR-2.2-S1 ever disagree by one least-significant bit, that pair of narrowings
  is the place to start.

---

## F. The declaration surface

**Two fields, `tint` and `tint_distance`.** The spelling is chosen to match the
spec's own scenario prose word for word ("a tint of `#3A6EA5`", "a tint distance
of `12.0`"), and it follows the loader's existing habit of a base noun with a
qualified sibling (`swim_ascent`, `move_resistance`).

| Field | Type | Bound | Absent means |
|---|---|---|---|
| `tint` | string | `#RRGGBB`, or `#RRGGBBAA` whose alpha is `FF`; case-insensitive; refused with any other alpha | no tint — **provided `tint_distance` is also absent** |
| `tint_distance` | number | finite, **strictly greater than zero**, at most `f32::MAX`; in blocks | no tint — **provided `tint` is also absent** |

`RECOGNISED_FIELDS` grows from 13 to **15**, appended in that order after
`opacity`, matching FR-1.5-S1's "fifteen names in the loader's own order". Three
pages quote the list (`docs/modding/blocks-items.md`, `hot-reload.md`,
`README.md`) and `documented_refusals.rs` compares a quoted refusal against a
real run line for line, so all three are edited with the loader.

**The type.** `mc_core::block::MediumTint`, beside `Opacity` and built on its
precedent — a type that cannot hold a value the engine would not accept:

```rust
pub struct MediumTint { /* three sRGB channel bytes, and a distance in blocks */ }

impl MediumTint {
    /// The tint of `color` reaching its full strength at `distance` blocks, or
    /// nothing where the distance is not finite and greater than zero.
    pub fn new(color: [u8; 3], distance: f32) -> Option<Self>;
    pub fn color(self) -> [u8; 3];
    pub fn distance(self) -> f32;
}
```

`Copy + PartialEq`, not `Eq` — `BlockDefinition` and `SimSnapshot` are already
`PartialEq`-only for the same reason. `BlockDefinition` gains
`pub tint: Option<MediumTint>`.

**The loader.** A fourth child module,
`crates/mc-world/src/content/luau_declaration/tint.rs`, on the exact precedent of
`opacity.rs`: it is the module for the field whose acceptance depends on
**another field**, and it raises the parent's `FieldFault`s so the refusal
vocabulary stays one vocabulary. It reads both fields and answers
`Result<Option<MediumTint>, FieldFault>`.

- The distance reads through `number::optional_number_within`, unchanged, so it
  inherits all four existing refusal branches and the `-0.0 → 0.0`
  normalisation. **`Bounds` gains an exclusive floor** — the one genuinely new
  branch, and it carries the sentence it is refused in beside the number, the
  way `floor_in_words` already does. The exclusivity travels *inside* `Bounds`
  rather than as a fifth parameter, which is the reason `Bounds` exists
  (`clippy.toml` caps a function at four).
- Finiteness stays asked **before** the floor. FR-1.3-S2 is the reading that
  pins the ordering, and it names both `math.huge` and `0/0` deliberately:
  infinity passes a `> 0.0` comparison and NaN fails one, so the pair catches a
  wrong ordering whichever way it is wrong.
- The distance's ceiling is `Bounds::at_least_zero()`'s existing `f32::MAX` with
  an exclusive floor substituted — unreachable, stated so there is one reader
  rather than two.

**The colour reader is new work and reuses neither shipped parser.**
`crates/mc-core/src/hud/element.rs:148` takes eight digits and refuses six
("must be `#RRGGBBAA` with 8 hex digits and no shorthand",
`crates/mc-core/src/hud/raw.rs:281`); `tools/voxforge/src/material.rs:210` takes
six and refuses eight ("is not written `#rrggbb`, which is the only form a
colour takes"). Both refusals claim exclusivity and both claims are false about
the tree. **Verified on `3c7a21b` at both line numbers rather than relayed.**
`crates/mc-core/src/art.rs:350` greps as a hex reader and is not one — it reads a
sixteen-digit fold digest. There is no third. Neither shipped reader is touched
here; relaxing or unifying them is **PRO-999**, and doing it inside a water spec
would put a HUD change in a water spec.

Four distinct refusal causes, because four scenarios need to tell them apart:

| Input | Cause |
|---|---|
| `5` (a number) | wrong kind — `wrong_kind(field, found, "a string")`, the existing vocabulary |
| `"#GG0000"`, `"3A6EA5"`, `"#3A6EA"` | not one of the two accepted forms; **names both forms** |
| `"#3A6EA580"` | well-formed eight digits, alpha is not `FF` — *a tint states no alpha; the distance carries its strength*. **Distinct from the row above**, which FR-1.2-S3 asserts |
| `"far"` for the distance | wrong kind — `wrong_kind(field, found, "a number")` |

Case-insensitivity is a property of the parse, not of the text: digits go
through `is_ascii_hexdigit` and `u8::from_str_radix(_, 16)`, and the alpha is
compared as the parsed **byte** against `0xFF`, so `ff`, `Ff` and `FF` all pass.

**Both or neither** (FR-1.4-S1) is decided in `tint.rs` after both reads, in two
distinct sentences naming the *missing* field as the thing to add — the same
reasoning `opacity.rs`'s two-sentence refusal is built on: a refusal naming a
line the author's file does not contain sends them hunting for something that is
not there.

**The rule is over the eye's own cell and nothing else.** Whether that cell's
block is drawn, occludes, is solid or is swimmable does not enter into the
resolver. FR-1.1-S4 admits `tint` beside `opacity = 1.0`; §A resolves it and
§B draws the whole frame at the declared colour. No shipped block reaches that
state.

---

## Integration points

| Site | Change |
|---|---|
| `crates/mc-core/src/block/` | `MediumTint`; `BlockDefinition.tint: Option<MediumTint>` |
| `luau_declaration/mod.rs` | two field-name constants; `RECOGNISED_FIELDS` 13 → 15; one line in `check` |
| `luau_declaration/tint.rs` | **new** — the colour reader, the both-or-neither rule, four refusals |
| `luau_declaration/number.rs` | `Bounds` gains an exclusive floor and the words it is refused in |
| `persistence/format.rs` | `DeclaredAppearance.tint` appended; `APPEARANCE_REVISION` 4 → 5 |
| `mc-sim` — the flooring helper | `trace.rs`'s `containing` lifted and shared; the third use is the one that earns it |
| `mc-sim` — `eye_medium` | **new** — the resolver of §A |
| `mc-sim/src/simulation.rs` | `SimSnapshot.tint: Option<MediumTint>`; both publishers fill it |
| `mc-render/src/snapshot.rs` | `TerrainSnapshot.tint: Option<MediumTint>` |
| `mc-render/src/color.rs` | unchanged; its `srgb8_to_linear` gains a second caller |
| `mc-sim/src/world/reload.rs` | **no change**, and the reason is §A's no-caching rule plus `changes_geometry`'s positive keying — a tint edit reloads without a re-mesh |
| `mc-render/src/gpu/buffers.rs` | `FRAME_UNIFORM_BYTES` 160 → 192 |
| `mc-render/src/gpu/pipeline.rs` | binding 0 visibility `VERTEX` → `VERTEX \| FRAGMENT` |
| `mc-render/src/gpu/record.rs` | uniform bytes gain eye + tint; the tint threaded into `draw`; the clear colour becomes a per-frame choice |
| `mc-render/src/gpu/mod.rs` | `clear_color` loses `const` or gains a second form |
| `mc-render/shaders/terrain.wgsl` | `Frame` gains three fields; `VertexOutput` gains `world_position`; four lines in the fragment stage |
| `mc-render/shaders/cull.wgsl` | **untouched** — its `Frame` stays a valid prefix |
| `mc-render/build/validate.rs`, `build/validate_tables.rs` | **new check** — `struct Frame` in both shaders against a copy, on `SECTION_RECORD`'s pattern (§B.2) |
| `mc-client/src/app/mod.rs` | `snapshot()` copies the published tint |
| `mc-client/tests/support/frames.rs` | second constructor `snapshot_in(&prepared, …)`; the three-parameter form stays |
| `mc-client/tests/support/goldens.rs` | shoots through `snapshot_in`, using the `prepared.world` and `prepared.registry` it already holds |
| `mc-render/tests/*` (3 sites) | pass `None` — they draw no world, and the crate cannot name `mc-sim` |
| `content/base/blocks/water.luau` | declares the sea's tint and distance |

**`content/base/blocks/water.luau`'s values are content and are the author's
choice, not this document's.** The only engine-side constraint is that the
distance be finite and greater than zero.

---

## What FR-6.1's scan must look for, and what it must not

FR-6.1-S1 asks for a verdict that **no comparison against a block name** exists
on the path deciding the eye's medium or drawing the frame. The resolver in §A
necessarily *handles* a `BlockName` — `World::block_at` returns one and the
registry is keyed by it. A scan for "does this file mention `BlockName`" would
report a correct implementation.

**BINDING: the scan looks for a comparison against a name literal** — a string
literal containing `:` in a namespaced-id position, or an equality against a
constructed `BlockName`, on a declared set of sources: the resolver, the uniform
writer, the clear-colour chooser, and `shaders/terrain.wgsl`. FR-6.1-S2's
positive control is a fixture holding exactly one such comparison, and it is what
distinguishes a clean verdict from a scan that can no longer look.

Prefer an **enumerated verdict** over an emptiness assertion here
(`testing.md` §2): `NoNameComparisonOnAnyDeclaredSource` rejects every other
answer, including "I could not find the sources", for free.

---

## Assumptions

1. **A player of the shipped world can get their eye under the sea, by 0.38
   blocks.** `EYE_HEIGHT = 1.62` (`crates/mc-sim/src/player/mod.rs:30`) against a
   sea recorded as 178 cells, 47 at height 33 and 131 at height 34
   (`a_camera_inside_the_sea_tints_nothing.rs`). In the 47 two-deep columns the
   bed's top is `y = 33.0` and the surface `y = 35.0`, so a resting eye stands at
   `34.62`, `0.38` below. In a one-deep column the eye stands at `35.62`, `0.62`
   above a surface at `35.0`. **The arithmetic reproduces and nothing in this
   phase contradicts it** — but it remains arithmetic over two recorded numbers.
   FR-2.6-S1 is what turns it into an observation and FR-2.6-S2 is its control.
   **If it turns out false, the phase that finds out escalates to the owner and
   stops.** Deepening the sea, widening the water and lowering the eye are each
   refused in advance.
2. The existing pose fixture `EYE = [60.5, 34.5, 8.5]` sits at the centre of a
   water cell whose top face is `y = 35.0`, which is what FR-2.4-S2's `34.98` /
   `35.02` boundary is stated against. Read from the superseded test.
3. **Not an assumption.** What used to sit here — "`mix(a, b, 0.0)` is exact on
   every backend" — is derived in §B.1 from the three forms a backend may
   compile `mix` into, all of which reduce to `a` for finite operands. It is
   recorded there, with the one thing that *is* binding (`tint_reach` written as
   a literal zero), rather than here where it would read as work still owed.

---

## Risks

| Risk | Severity | Response |
|---|---|---|
| The golden shooter hard-codes `tint: None`, and FR-3.1-S1 passes about a renderer that cannot tint | **high** — invisible from inside the test | §C's binding constraint, narrowed to the paths that draw a world |
| The `Frame` record drifts between its three hand-written copies, or cull stops being a valid prefix | **high** — a plausible wrong picture with no error | §B.2's build-time check. `min_binding_size: None` catches only an undersized buffer, never a mis-ordered one |
| The resolved tint is cached across ticks as an optimisation | **high** — passes every FR-2 reading | §A's binding no-caching rule. FR-4.1-S1/S2/S4 are the only witnesses |
| **Three** scenarios (FR-2.3-S1, FR-2.4-S1 and **FR-4.1-S3**) are satisfied by a build that never writes the uniform at all — a zero-filled buffer is `tint_reach = 0` | medium | true, and stated rather than papered over. **The count was two here until the mutation ran**: forcing the reach to no tint reddened nine of ten readings, and the tenth was FR-4.1-S3, which compares a frame before a refused reload against one after — with no tint anywhere those two are still equal, so a refusal that changed nothing is indistinguishable from a renderer that draws nothing. Their absolute halves are dry predictions and pass too. **The wiring witnesses are FR-2.1-S1…S6, FR-2.2-S1, FR-2.4-S2 and FR-4.1** — a suite that lost those would lose the wiring, and no shape of assertion inside the two identity scenarios recovers it |
| A per-vertex `distance` varying instead of a per-vertex world position | medium | wrong by construction over merged quads; FR-2.1-S4 reddens. Recorded in §B.1 so it is not rediscovered |
| The clear colour and the shader tint decoded by two routes | medium | one decode feeds both (§E); FR-2.1-S2 and FR-2.2-S1 predict the same colour by two routes |
| `cull.wgsl` edited to declare the three new fields | low | unnecessary — it reads none of them and a valid prefix is enough. The drift risk it would otherwise carry is what §B.2's check closes |
| A tint change triggers a whole-world re-mesh | low | `changes_geometry` is keyed **positively** on the three drawing fields, so a new property is outside it the moment it exists (`reload.rs:104`) — a default that needs nobody to remember anything. **Corrected after implementation: "`reload_marks_sections.rs` is what reddens if somebody adds it" was false for `tint`.** That file reddens per *field*, and only for a field with a reading of its own there; the tint had none, and adding it to the key moved the workspace by zero tests. `a_candidate_that_only_changes_what_a_volume_does_to_the_light_marks_no_section` is now that reading and the same mutation moves exactly one — and it lives in **`reload_marks_no_section.rs`**, which the excluded fields' readings were split into when adding it took the original past the 600-line cap |
| The 0.38-block margin is silently killed by a later change to `EYE_HEIGHT` or the sea's depth | medium, deferred | recorded as debt by the spec; FR-2.6-S2 is the only thing that would say so |

---

## The ADR, and where it landed

**ADR-032 — "A medium tints on a linear ramp to a stated distance, not an
exponential density" — is written into `docs/technical/decisions.md` in this
phase, not owed at completion.**

The convention check: `docs/technical/decisions.md` already holds ADR-030 and
ADR-031 (SPEC-031 / PRO-993), and no `*adr*draft*` file exists anywhere under
`specs/`. So the newer convention is that the ADR lands in the living document
directly, and PRO-852's `adr-013-draft.md` hand-off is superseded. That is also
the right answer on the merits: this spec folder is **archived** at completion,
so a rationale living only here is one a future reader reconstructs from code.

The ADR names the exponential / Beer-Lambert alternative and states why it was
rejected: a distance an author writes is a claim they can check and a reading can
assert, whereas an exponential never fully hides anything, so no number an author
wrote would mean what it said. Both reference implementations surveyed declare a
colour and a distance per volume rather than a density.

---

## The review round

`persona-architect` reviewed this document in Mode B before it was committed, at
rigor `high`, against the tree at `3c7a21b`. It re-verified every load-bearing
claim about the tree independently — the depth-write, the storage bindings, the
prefix, `changes_geometry`, `RECOGNISED_FIELDS`, both revision bytes, the two
publishers, the WGSL offsets — and found all of them true. It returned **no
blocker**, two Majors and three Minors. Every one is folded into the text above:

| Verdict | Where it landed |
|---|---|
| Major — the `Frame` record is hand-duplicated three ways with no check, and this spec triples its surface | §B.2's build-time check, now BINDING; two rows in the risks table |
| Major — §C's constraint said "every capture path", which is unmeetable for `mc-render`'s own six sites | §C rewritten as three narrower rules, with the `snapshot_in` cost and the six `None` sites priced |
| Minor — "B2 cannot satisfy FR-2.1-S6 **at all**" is too strong; an MRT variant is exact | §B states the counterexample and rejects B2 on grounds that do not depend on arithmetic. The depth-sampling slip (`9.0`, not `3.0`) is corrected |
| Minor — §C's stated argument does not survive `capture.rs`'s own rule | replaced by the operational one: a fresh mint would be byte-identical, which destroys FR-3.1-S1's evidence |
| Minor — `record::draw` has no snapshot and `clear_color` is `const fn` | priced in §B.5 and in the integration table |
| Info — the `mix` identity is derivable, not an assumption | moved from Assumptions into §B.1, with the literal-zero rule made BINDING |
| Info — §E's "one decode" closes at the `f64`, not at the device | the residual is now named in §E |
| Info — FR-4 had no integration point, and the design that satisfies it is what an optimiser removes | §A's no-caching rule, now BINDING, plus a `reload.rs` row saying why it needs no change |

**One suggestion is recorded as declined, with the reason.** The reviewer
proposed a standing condition that once `Frame` acquires a build-time check it
becomes a contract item of the `r5` kind, so the *next* change to it owes a bump.
Declined: `r5`'s cause was that a packed vertex is geometry a capture is a
photograph of, and being build-time checked is not what made it one — the
`Section` record is checked by the same validator and has never bumped anything.
Substituting "is it checked?" for `capture.rs`'s own test would be a second,
easier question standing in for the real one. §C states the condition the other
way instead: the rule stays comparability, and whoever changes the record next
answers it by rendering.

---

## Deferred observations, recorded and not built

- **PRO-999** — the two shipped colour readers, each refusing the other's form as
  the only one there is. Both line numbers and both refusal texts are in the
  spec's `## Notes`. Not touched here.
- `CLEAR_COLOR_SRGB` remains a Rust constant naming the sky. Out of scope by
  name; worth an issue of its own.
- The tint has no per-medium sky colour, no depth gradient, no biome and no time
  of day. One block, one colour, one distance.
- `containing` existing three times is fixed here **because this spec creates the
  third use**; no other duplication in the neighbourhood is touched.
