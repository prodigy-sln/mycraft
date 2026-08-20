# Requirements — the grass block looks like a grass block

Source issue: PRO-947, absorbing PRO-914 (per-face texture keys) and PRO-869
part 1 (mips and filtering). Scope set by the conductor 2026-08-17 and
reaffirmed 2026-08-18; PRO-904's drawn/occludes split deliberately does **not**
ride along.

This file records what the codebase could not answer, what was measured rather
than assumed, and every decision the issue delegated to the spec. The spec is
the contract; this is why the contract reads the way it does.

---

## 1. What was measured, not assumed

### 1.1 The art is what the checked-in model bakes to, and the digests are recorded

PRO-947 carries an explicit falsifiability instruction, and it is the right one:
a pre-baked image believed to match a model is evidence of nothing until the
model has actually been baked. Shipping such an image on the assumption that it
matches would be the same failure as minting a golden from the renderer it
verifies.

**So nothing pre-baked ships.** The shipped art is defined as what
`content/base/models/grass-block.mcvox` bakes to, and nothing else. Two
invocations, both of which any reader can re-run:

```
voxforge texture content/base/models/grass-block.mcvox --out <dir> \
    --all-faces --pixels-per-voxel 1  --seamless
voxforge texture content/base/models/grass-block.mcvox --out <dir> \
    --all-faces --pixels-per-voxel 12 --seamless
```

Both were run at `8dea90d`. `--seamless` accepted all twelve faces, reporting
*tiles across every edge* for each. The sha256 of every face is recorded here so
that the claim stays falsifiable after this spec closes — a future reader
re-runs the two commands and compares, with no other artifact required:

| Face | `--pixels-per-voxel 1` (16×16) | `--pixels-per-voxel 12` (192×192) |
|------|--------------------------------|-----------------------------------|
| top | `af2e56c2dc832b44d41ce11cf9f29eb44…` | `01a4707c787dd55951c6fb88409a33e5…` |
| bottom | `cb03db5ab649871461164d992dae1866…` | `5361513a384df690afac4437e112fc4c…` |
| front | `ddd7634a2b8bb98a0e3c7747aaf4d1fc…` | `5720cfa4de539543e55038a67ae90181…` |
| back | `bd18378d5784a587fb00b08e80203c63…` | `7ce209921da1fbf3185187a6b63d4203…` |
| left | `5043aee043899be1a93c995f634c5eb0…` | `254deb379478044262ae63cff6e54822…` |
| right | `a5869c7d312d54f7ee8c90122f54e075…` | `287f0f853ed5c96af084146af7a458d8…` |

**The model and the tool are both intact and neither needs diagnosing.** The
bake was then viewed: the sides show a green fringe over dirt with the sod line
a few pixels down, the top a green mottle, the bottom a brown mottle. It is
recognisably a grass block.

The `--pixels-per-voxel` values are recorded because they are not obvious — the
tool's default is 8, and neither of these is the default.

### 1.2 The larger set carries no information — decided by measurement

PRO-947 left open which of the two baked resolutions ships, noting that 16×16 is
what the model header says the terrain wants while the larger set "may simply
have been a legibility aid for authoring".

It is a legibility aid, and the question is settled by decoding rather than by
argument. VoxForge renders a face with `Shading::Flat`, so one voxel is one flat
colour at any scale. Decoding both sets and comparing every sub-pixel:

| Face | Non-uniform sub-pixels in any 12×12 cell | Cells disagreeing with the 16×16 texel | Distinct colours |
|------|---|---|---|
| top | 0 | 0 | 5 |
| front | 0 | 0 | 6 |
| bottom | 0 | 0 | 3 |

The 192×192 set is an exact 12× nearest-neighbour magnification of the 16×16
set. It carries nothing the 16×16 does not, at 144× the texels.

**Decision: the shipped set is 16×16, at one pixel per voxel.** The model header
already states this is what the terrain wants; the measurement confirms the
larger set would cost memory and mip levels for no pixel of detail.

A useful by-product for the test author: a real face carries **three to six
distinct colours**, which directly falsifies the premise held by
`crates/mc-client/tests/support/swatch.rs` (`TEXEL_COLORS = 2`, the placeholder's
checkerboard). That test's premise changes with this spec rather than its
assertion being loosened.

### 1.3 Stone's six faces are six different images

`stone-block.mcvox` bakes to six distinct PNGs (six distinct hashes), even
though the block is uniform noise on every face and the issue describes it as
needing no per-face keys. Both are true: any one of the six is an acceptable
image for all six faces, but *which one* is not something the engine may guess.

**Consequence for the manifest design:** a manifest entry names a model **and a
face**, always. There is no "emit the block's texture" shorthand that would have
to pick a face silently.

---

## 2. Premises in the issue that the tree contradicts

Recorded rather than silently corrected, because a reader who saw the issue
needs to know which parts did not survive contact with the tree.

### 2.1 There is no absence assertion in `command_line.rs`

PRO-947 records, as verified empirically, that
`tools/voxforge/tests/command_line.rs` "reserves `#00c8ff` and `#ffb000` as
colours no file under `content/base/materials` declares", calls it "A LATENT
TRAP FOR THIS ISSUE", and obliges this spec to re-check it and consider deriving
the reserved colours from the shipped set.

**There is no such assertion.** `command_line.rs:52-57` carries two `const`
declarations whose *doc comments* claim the property:

```rust
/// A colour no file under `content/base/materials` declares.
const CYAN: &str = "#00c8ff";
```

Both hexes appear in exactly one file in the whole repository. Nothing reads
`content/base/materials` looking for them; both constants are used only to build
**temp-directory** material fixtures, whose own self-check asserts the two
fixtures differ from each other. No material this spec adds can invalidate
anything, because nothing is asserted.

**The obligation dissolves.** Deriving reserved colours from the shipped set is
not in this spec — there is nothing to derive them for. What is left is a stale
doc comment, and the spec does not chase it: it is accurate today and asserts
nothing either way.

### 2.2 The generators are already tracked; the assembler was not

> **Retired in part, 2026-08-18.** FR-8.2 and all seven of its scenarios were
> retired; `spec.md`'s FR-8.2 entry carries the ruling. The three defects named
> below are all real and all still in the tree — they are simply no longer
> **repaired under test**. The scripts are kept as provenance and documented as
> not runnable as-is, and only `grass-block.mcvox`'s citation is repaired. The
> siting decision in §2.3 stands unchanged.

PRO-947 asks this spec to land `gen_grass.py` and `assemble_grass.py` under
`tools/`. The tree at `8dea90d` already carried two of the three:

- `content/base/models/generators/gen_grass.py` — **tracked**
- `content/base/models/generators/gen_stone.py` — **tracked**
- `assemble_grass.py` — **absent from the tree and from the whole of history.**

`assemble_grass.py` is landed by this spec at
`content/base/models/generators/assemble_grass.py`. It is the one artifact here
that could not be reconstructed from anything the repository held: it carries
the **hand-authored** courses of the grass model — the sod shadow, the three
lone blades at three deliberately different depths — as literal grids that exist
in no generator and are not derivable from the model, because the model is their
output rather than their source. Everything else in this spec reproduces from
the checked-in model, which is what §1.1 establishes.

It is landed **verbatim**, and it does not run as landed. Two defects it arrives
with, plus one already in the tree, are what FR-8.2 exists to fix under test
rather than being quietly patched in a preservation commit:

- `assemble_grass.py:140` writes to a hardcoded absolute path naming one
  machine's checkout, and `:141` reads a header file that is not in the tree.
  The model's header prose is not at risk despite that — it is present verbatim
  in the tracked `grass-block.mcvox`, which is where the assembler should read
  it from rather than from a second copy that can drift.
- `gen_stone.py:118` writes `content/base/models/stone.mcvox`. The tracked model
  is `stone-block.mcvox`. Re-running the generator produces a **second file**
  rather than regenerating the real one, so the stone art is not in fact
  reproducible from its generator today.
- `grass-block.mcvox`'s header cites `scratchpad/gen_grass.py` and
  `assemble_grass.py`. Neither path exists; both now have tracked homes to cite.

### 2.3 The siting question the issue delegated

PRO-947 asks this spec to decide between `content/base/models/generators/` and
`tools/`, "with the argument recorded rather than by preference".

**Decision: beside the model, at `content/base/models/generators/`.**

- They are **already tracked there**. Moving them is churn against a location
  that has been in `main` since `be1eeb6`, and it would break nothing and fix
  nothing.
- The discoverability argument is real and the counter-argument is not. A mod
  author who opens `content/base/models/` to see how the shipped art was made
  finds the generator in the same directory as the art.
- The stated risk was that a scan walking `content/` as a source root might
  treat a `.py` as content. The issue said "probably safe" and that "probably is
  not the standard". It was checked. **Three scans exist and none can see the
  directory**: the block loader reads `<root>/blocks` and accepts `.luau` only,
  one level, silently passing over anything else; the HUD loader reads
  `<root>/hud` and `.toml` only, one level; the reload watcher filters to
  `.luau` and `.toml` immediate children of those two directories, with a
  standing comment that "a material, a model, an editor's scratch file, a
  declaration nested one directory deeper — is not content". A `.py` under
  `content/base/models/generators/` is not read, not refused, and does not
  trigger a reload.

The spec therefore lands `assemble_grass.py` beside the two that are already
there, repairs `gen_stone.py`'s output path so the stone art is genuinely
reproducible, and repairs the model header's citation.

---

## 3. The one real design decision

`layer_for` (`crates/mc-render/src/geometry/mod.rs:180`) parses the block's
**name** as a texture key and never consults `BlockDefinition.texture`. Per-face
keys make that impossible, because a facing cannot be derived from a name.

- **(a)** the mesher resolves block + facing to a `TextureKey` and stamps it
  into the `Quad`
- **(b)** the packer takes a key source alongside layers, and the `Quad` stays
  purely geometric

**Decision: (b).** The issue and the conductor both recommended it; the
arguments they gave hold, and one more was found in the tree that is stronger
than either:

**Meshed sections are retained across a reload.** `PreparedScene` carries
`meshed: Vec<SectionQuads>` for the explicit reason recorded at
`crates/mc-client/src/startup.rs`: a reload "keeps the sections it touched and
puts them back where they were", and re-meshing the whole world to recover that
list would be "a second answer to a question the first one already answered".

A `Quad` that carried a resolved `TextureKey` would therefore carry a **stale**
one the moment a reload changed a block's face keys — and the retained-section
optimisation is precisely the path that would not notice. Keeping the `Quad`
purely geometric means the retained mesh re-resolves against the new registry
for nothing. This is the same property PRO-918 protects from the other side: a
layer index inside a packed vertex is never renumbered, so resolution must
happen where vertices are built and not before.

Option (a) additionally puts content resolution inside `mc-world`'s mesher,
whose own module header keeps it rendering-free, and grows every `Quad` by an
owned key.

### PRO-902 is resolved, not deferred

The name-as-key coincidence has **two** sites, and `crates/mc-render/CLAUDE.md`
warns that closing one leaves a block that draws in-world with a blank
indicator:

- `geometry/mod.rs:186` — `TextureKey::parse(quad.block.as_str())`
- `hud/held.rs:109` — `TextureKey::parse(block.as_str())`

Both are replaced by lookups into one per-block, per-facing layer table derived
from the registry and the stated layer assignment. Neither site parses a block
name afterwards. A mod that names a texture differently from its block — valid
content that cannot be expressed today — draws correctly in both places.

---

## 4. Decisions the issue left to the spec

### 4.1 How a face key is spelled in a declaration

PRO-947: the four sides are per-facing, so the names must not be model-relative
words like `front` and `left`; the mapping from a declaration to a `Facing` is
content's word rather than the engine's inference.

**Decision: two forms, and no third.**

- A **string** — one key for all six faces. `base:dirt`, `base:stone` and
  `base:water` use this and nothing about them changes.
- A **table stating all six facings** by name: `up`, `down`, `north`, `south`,
  `east`, `west`. `base:grass` uses this.

The vocabulary is the engine's published contract, mapped once:
`up` = `PosY`, `down` = `NegY`, `north` = `NegZ`, `south` = `PosZ`,
`east` = `PosX`, `west` = `NegX`. The repository had no compass convention
before this spec (one incidental use in a test doc comment); this is the
convention players and mod authors in this genre already expect, and it is
defined rather than inferred.

A declaration names a **facing** directly, which is what the issue asked for.
The engine never sees VoxForge's model-relative face names — those appear only
in the texture manifest, where each is mapped to a key explicitly.

**A partial table is refused**, naming the facings that are missing. There is no
default and no precedence rule.

### 4.2 Why the top/side/bottom triple is excluded

PRO-914's inherited shape called the triple "the common case". It is not built,
and this is the reasoning:

- **No shipped block wants it.** Grass needs six genuinely different images —
  measured, in the issue and again here. Dirt, stone and water each need one.
- **It is the only form that needs precedence rules.** A string and an
  all-six table are both total: every facing is answered exactly once, by
  exactly one written value. A triple has to say what happens when `side` and
  `north` are both present, and that rule is a second thing to document, test
  and get wrong.
- **Adding it later is a pure relaxation; removing it later is not.** A
  declaration valid under the two-form contract stays valid if optional facings
  are added afterwards. This is the "breadth of capability, narrowness of
  commitment" filter from `code-quality.md` §1 applied to a published extension
  API: the *capability* — per-face art — ships whole; the *commitment* to a
  particular sugar does not.

Recorded in the spec's Out of Scope, which is binding.

### 4.3 What a declared key with no art does

PRO-947: this is the placeholder's new contract, and it must be a stated
fallback, never a panic — a mod author hits it on their first block.

**Decision: the key resolves to its generated placeholder, exactly as every key
does today, and the run continues.** The placeholder generator is not deleted by
this spec and is not repurposed; it becomes the fallback for a key the texture
set does not cover.

ADR-026 is explicit that this is a different question from a missing generated
set, and that the two messages must not be collapsed:

- **"This key was never authored"** — one key, no art. Placeholder, no refusal,
  no message on the terminal. A mod author's first block draws in a
  deterministic colour derived from the key, which is what happens today and
  what `docs/modding/README.md` already documents.
- **"The build step was not run"** — the whole set is missing or stale. The
  client refuses to launch and says what to run. §4.4.

### 4.4 Where the baked PNGs come from, and how a stale set is caught

Settled by ADR-026 before this spec: an explicit pre-build step, a
`voxforge build <manifest>` subcommand. VoxForge is not split, FR-9.1 is not
amended, and `build.rs` is refused. This spec is the first consumer, so
ADR-026's owed items land here.

**The manifest is content and is committed; the output is derived and is not.**

- `content/base/textures.toml` — the manifest. Source, committed, reviewable in
  a diff. Each entry names a model, a face, and the texture key that face
  becomes.
- `content/base/textures/` — the generated PNGs plus an index recording what was
  built from what. Derived, gitignored, never committed.

Neither path is visible to the block loader, the HUD loader or the reload
watcher, all three of which were checked in §2.3.

**The staleness check needs one hash reachable from two places that may not
depend on each other.** `tools/voxforge` writes the index; `crates/mc-client`
verifies it; `crates/` may not depend on `tools/` (FR-9.1, mechanically enforced
by `crates/mc-testkit/tests/workspace_layering.rs`).

The project already has the right primitive, with its rationale written down:
`fnv_1a_64` in `crates/mc-world/src/persistence/format.rs`, hand-written
deliberately because the standard library's hasher "is documented as unspecified
and may change between compiler releases, and a hash that moves with the
toolchain invalidates every save on an upgrade", and non-cryptographic because
forgery resistance buys nothing against a local file.

**Decision: move `fnv_1a_64` to `mc-core` unchanged.** It is a pure primitive
over bytes with no I/O, which is `mc-core`'s remit; `mc-world` keeps using it,
so no stored save hash changes value; and `tools/voxforge` (which already
depends on `mc-core`) and `crates/mc-client` share one implementation instead of
two that must agree forever. Two callers is below the three-use threshold for
DRY-driven abstraction, but this is not an abstraction — it is one function
moved so that two independent users cannot drift apart on a value they must
compute identically.

### 4.5 Mips and filtering — the sampler is forced, not chosen

PRO-869 part 1 asks for "the mip chain and filtered or anisotropic sampling".
Today `mip_level_count` is 1 and every filter is `Nearest`
(`crates/mc-render/src/gpu/buffers.rs:238-274`).

**Anisotropic filtering and crisp voxel magnification are mutually exclusive in
wgpu 30.** Read at
`wgpu-core-30.0.0/src/device/resource.rs:2288-2316`: when `anisotropy_clamp != 1`,
`min_filter`, **`mag_filter`** and `mipmap_filter` must all be `Linear`, each
with its own refusal. Choosing anisotropy means choosing a blurred close-up
texel, which is the whole aesthetic of a voxel game.

**Decision: `mag_filter: Nearest`, `min_filter: Linear`, `mipmap_filter: Linear`,
`anisotropy_clamp` left at 1.** The issue's own wording is "filtered **or**
anisotropic", and filtered is the branch that keeps the look. Trilinear
minification is what removes the aliasing that motivated PRO-869 part 1 in the
first place — a real 16×16 texture carries far more high-frequency detail than a
near-flat placeholder.

The bind-group layout already declares `filterable: true` and
`SamplerBindingType::Filtering` (`gpu/pipeline.rs:94-103`), so no layout change
is needed. Anisotropy is not a wgpu feature flag and needs no discrete-GPU
fallback; it is simply not taken.

**Mip levels are generated on the CPU by box filter and uploaded per level.**
wgpu has no built-in mip generation, and the alternative is a GPU blit or
compute pass. A 16×16 chain is five levels of trivial arithmetic, and
`crates/mc-render/CLAUDE.md` is explicit that "anything expressible as a pure
function — meshing, packing, culling maths, atlas layout — is unit-tested
normally and is **not** exempt. Only GPU-resident work gets the exclusion." A
CPU box filter is a pure function under normal coverage; a GPU pass would land
in the one subtree where golden frames are the only thing standing between a
regression and shipping it.

### 4.6 Goldens are re-shot at `r1`; `SCENE_REVISION` is not bumped

`docs/technical/rendering.md` ties a revision bump to a change in the **mesh
contract**: "When the cause is a change to the mesh contract rather than to the
renderer, bump `mc_render::capture::SCENE_REVISION` instead of overwriting."

Nothing about the scene contract changes here. The pose, the world, the camera
path, the tick list, the merge predicate and the vertex format are all
untouched; the layer field stays 8 bits and the world is meshed identically.
What changes is which texels sit in a layer and how they are sampled.

**Decision: re-shoot in place at `r1`.** Bumping would redefine the revision as
"something visible changed", a weaker meaning that would then oblige a bump for
every future art change and every sampler tweak, and would leave the inventory
test enforcing a number that no longer identifies anything in particular. The
"why" that rendering.md asks a re-shoot to carry lives in the commit message and
in this spec, which is what it asks for.

The re-shoot follows the documented procedure verbatim — probes, oracle, HUD
prediction, then a mint naming **only** the `terrain_goldens` and `hud_goldens`
binaries. A bare `MYCRAFT_UPDATE_GOLDENS=1 cargo nextest run` reaches
`golden_mismatch` and corrupts the set permanently.

### 4.7 Which face the held-block indicator draws

`hud::held::held_swatch` draws one image for a block that now has six.

**Decision: the indicator draws the block's `north` face**, stated once as a
constant with its reason. A side face rather than the top, because a side is
what makes the canonical block recognisable: grass's side carries both the
growth and the earth, and a top-only indicator would show a green square that
says "grass" only to someone who already knows. `north` rather than another
side because the four are interchangeable for this purpose and an arbitrary
choice made once and written down is better than an arbitrary choice made
implicitly.

---

## 5. Consequences found while specifying

### 5.1 The save format's appearance hash changes, and every save notices

`crates/mc-world/src/persistence/format.rs:319` folds a block's **appearance**
over `name + texture`:

```rust
struct DeclaredAppearance<'a> { input_version: u8, name: &'a str, texture: &'a str }
```

Per-face keys change what an appearance *is*, so `DeclaredAppearance` gains the
six resolved keys and `INPUT_VERSION` goes 1 → 2. That is exactly the mechanism
the file documents for this case: the version is "its own leading byte, so that
adding a field to one of them is a deliberate act that says so in the value
rather than silently reinterpreting every hash already stored."

**Consequence: loading a save written before this spec reports every block's
appearance as changed**, routing through the existing `--load-changed-blocks`
path rather than through anything new. This is true rather than spurious — every
block's appearance really did change, from a hash-derived colour to real art —
and it is the first time that path fires for a reason a player will recognise.
It is player-visible and is documented as such.

### 5.2 The array texture's premises change

- `TEXTURE_LAYERS = 256` and `PLACEHOLDER_SIZE = 16` are the array's extent
  today. Real art is also 16×16, so the extent is unchanged — but
  `PLACEHOLDER_SIZE` is now the wrong name for a dimension that real textures
  also have to match, and a texture set entry whose PNG is not 16×16 must be
  refused rather than uploaded into a mismatched layer.
- `mc-render` has **no filesystem edge and no image decoder**, deliberately —
  no `std::fs`, no `PathBuf`, no `env::var` anywhere in `src/`. This spec does
  not give it one. The client decodes PNGs and hands `mc-render` texels in
  memory, which is exactly how `TextureLayers` already arrives.
- Decoding in `mc-sim` was considered and rejected: pixels are the client's half
  of the split, and a server that needed them would break the asymmetry that
  makes a texture pack a legal client modification.

### 5.3 Test premises that this spec invalidates

Named here so the test author is not surprised by them, and so none is
"loosened until green":

- `crates/mc-client/tests/support/swatch.rs` — `TEXEL_COLORS = 2`. A real face
  has three to six colours (§1.2). The premise changes; the assertion is
  re-derived against the shipped art, not relaxed.
- `crates/mc-client/tests/support/probe.rs` — `STRATA` clusters captured pixels
  against `placeholder_mean_color`. Means must be re-derived from the real
  texels. `COVERAGE_FLOOR` and the ΔE constants are re-checked against art whose
  colours are far less separated than three hash-derived colours were.
- `crates/mc-render/tests/terrain_offscreen.rs` — compares a centre pixel to
  `placeholder_mean_color` under a `SAME_TEXTURE` tolerance.
- `crates/mc-render/src/texture/placeholder_test.rs` — still valid: the
  placeholder is not deleted, and its pairwise-separation and variation tests
  keep guarding the fallback path.
- `crates/mc-client/tests/documented_refusals.rs` — asserts the recognised-field
  list line-for-line against `docs/modding/blocks-items.md`. The `texture`
  field's *type* changes but its name and position do not; the refusal texts
  gain the per-facing cases.

---

## 6. Stakeholder capability (Key Principle 7)

**The player.** Every spec in MVP 2 so far has been deliberately invisible to
them — PRO-917's spec names "nothing a player can see changes" as an intended
outcome. This is the increment where that ends: the player starts the client and
the world is grass, dirt and stone rather than tan and teal. Reachable by
running the game; no Rust read, no flag.

**The mod author**, secondarily: declaring six per-facing keys on a block and
seeing them on its faces, and adding a model plus a manifest entry and seeing
their own art in the world. This is the other half of the declaration capability
PRO-917 delivered.

---

## 7. Scenario audit — what it found and what was done

The audit reviewed 66 scenarios across 21 requirements and reported 14 gaps, 14
guideline violations, 13 falsifiability risks and one internal contradiction.
**All 14 gap drafts were accepted**, all 14 violations rewritten, and every
falsifiability note acted on. Landing the grass assembler then exposed four more
scenarios the audit could not have seen, because the defects were inside a file
the tree did not yet carry (§7.8), and the architecture added four more again.

**The count is not restated here, because it has now drifted twice.** What the
spec carries is whatever this reports, and nothing else should be believed:

```
grep -oE "FR-[0-9]+\.[0-9]+-S[0-9]+:" spec.md | sort -u | wc -l
```

What the audit changed that is worth recording:

### 7.1 It found an undecided design question, not a wording problem

**Mip levels must be averaged in linear light, and the spec had not said so.**
The array texture is `Rgba8UnormSrgb`, and `crates/mc-render/src/gpu/buffers.rs`
opens with a paragraph headed "**The array texture is sRGB, and that is
load-bearing**": a texel is decoded to linear on sample. Box-filtering the
*stored bytes* therefore does not average the colours. Bytes 0 and 255 average
to 128, which decodes to linear 0.216 rather than 0.5 — so every mip level would
come out darker than the level above it. That is the classic sRGB mipping fault,
and it is precisely the shape the same module already warns about in its
neighbouring case: "plausible-looking, and wrong in the direction nothing
notices."

The original FR-6.1-S4 said the reduced texel would be "midway between them
rather than either of them", which **both implementations satisfy** — 128 is
midway between the stored bytes and 188 is midway between the colours. It could
not have caught this. It is now FR-6.1-S2 and pins the byte at **188**, which is
the sRGB encoding of linear 0.5 (`1.055 × 0.5^(1/2.4) − 0.055 = 0.7354`,
×255 = 187.5). The wrong implementation produces 128 and fails.

### 7.2 It found that nothing tested the reason option (b) was chosen

§3 argues for option (b) on retained meshed sections, and no scenario mentioned
a reload. **Option (a) would have passed the entire spec.** FR-2.1-S4 and S5 now
exercise a reload that changes a facing key against sections that were retained
rather than re-meshed, which is the one observable that separates the two
options.

### 7.3 It found the golden minted from the renderer it verifies

FR-8.1-S4 compares a captured frame against its committed golden — and the
golden is re-shot from that same renderer as part of this spec. On its own it is
the snapshot-from-a-run-of-the-code-under-test that `testing.md` §2 forbids.
FR-8.1-S5 is now the derived witness: the grass block's top face is judged
against a mean computed from the built PNG, which shares no code with the draw.

### 7.4 It found a contradiction

The old FR-4.2-S3 said an unbuilt set must still complete the launch; FR-5.1-S1
and FR-5.2-S1 said an absent set refuses it. An absent set covers no key, so the
two were in direct conflict under a literal reading. FR-4.2-S3 now reads "IF the
built set is **present and current** and covers no key…".

### 7.5 Absence assertions replaced with total verdicts

Four scenarios asserted that something did *not* happen — no refusal printed,
the launch completed, anisotropy unrequested, no dangling generator path — and
each would go green forever the day the check stopped looking. Following
`testing.md` §2 and the project's own preference for an enumerated verdict:

- FR-5.1 is now written as **one verdict per launch** from a fixed set
  (current · absent · stale against sources · image missing), so a client that
  lost the ability to check reddens for free.
- FR-6.2-S4 is a **positive control** — the same inspection, run against a
  fixture that does contain the thing, must report it. FR-8.2-S3 was the second
  such control; it was retired with FR-8.2 on 2026-08-18, and so was the dangling
  generator path this subsection lists among the four absence assertions.

### 7.6 Two vacuity risks the audit raised that were already structural

- "Counts the key once" (old FR-1.1-S3, FR-2.1-S2) is guaranteed by
  `LayerAssignment`'s `BTreeSet` intake, so a broken resolver passed it anyway.
  Both now assert the **spent-layer count** instead: six facings with one
  repeated key spends five (FR-1.1-S4), two blocks sharing one key spend one
  (FR-2.1-S2).
- "Byte-identical on a different machine and a different toolchain release"
  (old FR-3.3-S3, FR-9.1-S2) names a trigger no test can stage. Both now assert
  against an **independent oracle**: the value an FNV-1a-64 fold over the stated
  byte sequence computes, which is what the toolchain-stability argument
  actually rests on.

### 7.7 One gap the audit found that the fold's design needed

FR-3.4-S3 is a **negative control**: editing a model under
`content/base/models/` that no manifest entry names must leave the recorded
value unchanged. Without it, "fold the whole `content/` tree" satisfies every
positive staleness scenario while turning every unrelated content edit into a
spurious launch refusal — a defect that would have shipped looking like correct
staleness detection.

### 7.8 Landing the assembler widened FR-8.2 — and FR-8.2 was then retired

> **Retired 2026-08-18.** The widening recorded here happened and is left standing
> as a record of the reasoning; what followed is that FR-8.2 was struck out
> entirely, on the ground that the generators are one-off scripts nothing re-runs,
> so a standing test re-proves a fact nothing can change. `spec.md`'s FR-8.2 entry
> carries the ruling. The two defects below are still in the tree and are now
> **documented rather than repaired**.

FR-8.2 originally asked only that the **stone** generator write the tracked
model and that model headers cite paths that exist. Landing
`assemble_grass.py` made two things visible that no scenario covered, both of
which would have shipped a generator that cannot run on any checkout but one:

- it writes to a **hardcoded absolute path** naming a single machine, and
- it reads the model's header prose from a **second copy** of it rather than
  from the model, which is a drift hazard between two files that must agree.

FR-8.2 now covers grass symmetrically with stone (S2), requires the header to
be read from the tracked model rather than a copy (S3), and adds a gate stage
refusing an absolute output path with its own positive control (S6, S7). The
generators are landed verbatim rather than quietly patched, so the fixes happen
under test where they can be seen.

This is the same lesson as §7.2 in a different place: a file that is not in the
tree is a file no audit can read.

---

## 8. Open questions carried into the spec

None. Every question PRO-947 left open is answered above, three of them by
measurement rather than by argument. The two premises the issue asserted that
the tree contradicts are recorded in §2 rather than quietly dropped, and the one
question the audit surfaced — the colour space of the mip filter — is answered
in §7.1.
