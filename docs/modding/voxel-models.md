# Authoring Voxel Models

How to describe a voxel model — a door, a torch, a chair, a prop — as a
text file, and how to see what you made. The format and the tool that
reads it are both called **VoxForge** (`tools/voxforge`, `voxforge` CLI).
The feedback loop is the feature: an agent authors a `.mcvox` file, runs
`voxforge preview` or `voxforge inspect`, and corrects the file from what
it sees. The editing verbs (`fill_box`, `mirror`, translate) are
deliberately not built — you edit the text.

**Nothing in `crates/` loads, meshes or draws a `.mcvox` file today.**
VoxForge is authoring and preview tooling only; engine consumption of
these assets is a later MVP (see `product/roadmap.md`). `tools/` may
depend on `crates/`; the reverse never holds — see
`technical/architecture.md`'s mechanically enforced invariants.

## File layout

One model per file, TOML, conventionally under `content/<mod>/models/` —
but the tool has no opinion about where a file lives and accepts any
path. One material per file under `content/<mod>/materials/`, mirroring
`content/<mod>/blocks/`'s one-definition-per-file convention
(`blocks-items.md`): the materials directory is read in **file-name
sorted order**, and that order is binding, not tidiness — a duplicate-name
error names a first file and a second file, which is only well defined
under a fixed order.

## The document shape

A `.mcvox` document declares `schema`, a namespaced `name`, a `scale`, a
`slice` axis, a `[palette]`, and geometry. Every id follows the same
namespaced-id rule blocks and textures use — exactly one `:`, non-empty on
both sides — reusing `mc_core::id::NamespacedId` rather than
reimplementing it (see ADR-020).

Geometry is either the **implicit single-part form** (a top-level `size`
and `origin`, no `[[parts]]`) or the **explicit form** (one or more
`[[parts]]` tables). The two may not be mixed: a one-part model, like a
door, costs nothing for a rig it does not use.

```toml
schema = 1
name   = "base:door_oak"
scale  = 16
size   = [4, 6, 2]
origin = [0, 0, 1]        # hinge axis: left edge, mid-thickness
slice  = "z"

[palette]
"." = "empty"
"w" = "base:oak_plank"
"h" = "base:iron"

[[layers]]                # z = 0, the outward face; first row is y = 5
z = 0
grid = """
wwww
wwww
wwwh
wwww
wwww
wwww
"""

[[layers]]                # z = 1, the back — plain
z = 1
grid = """
wwww
wwww
wwww
wwww
wwww
wwww
"""
```

`scale` declares how many voxels span one block edge — a model at `scale
= 16` matching the shipped 16×16 texture resolution is the convention the
base game's own assets use, not a rule the format enforces. No part and
no assembled model exceeds **64 voxels on any axis**, checked before any
grid is parsed — an over-large declaration is refused without its layer
art needing to exist. `schema` exists so the format can grow additive
fields later without invalidating anything written today; state
sequences and keyframed animation clips are the named follow-up (out of
scope here — see "What this format does not do", below).

**Unknown fields are rejected**, at every level: the document, each
`[[parts]]` table, and each layer. A typo fails to load rather than being
silently ignored, the same reasoning `blocks-items.md` gives for block
definitions.

## Layers, slicing and orientation

A layer is one 2-D slab of grid art; the `slice` axis says which plane it
is, and — because the same three characters can print floor plans or
front elevations depending on what reads best — `slice` may be declared
per model and **overridden per part**. A stool reads best as floor plans
(`slice = "y"`: legs, legs, legs, seat); a door reads best as front
elevations (`slice = "z"`: one large surface per layer).

| `slice` | Layer *k* is the plane | First row printed | First column |
|---|---|---|---|
| `y` | `y = k`, ascending from the ground | `z = 0` | `x = 0` |
| `z` | `z = k`, front to back | `y = extent.y − 1` | `x = 0` |
| `x` | `x = k`, left to right | `y = extent.y − 1` | `z = 0` |

A layer's grid must match the extent the slice axis implies **exactly**
— wrong row count, wrong row width, and a stray trailing character are
all refused, naming the layer, the row and the expected and found width.
A layer index outside the part's declared extent is refused. An omitted
layer index is legal and means an empty slab; two layers declaring the
same index is refused, naming the repeated index.

**This is the format's highest-risk convention, so it is worth
double-checking by eye against a `voxforge preview`, not just by
inspection of the grid.** A layer mapping that is mirrored or transposed
produces a *plausible* picture, and an agent self-correcting against a
wrong preview "fixes" correct geometry to match a broken one.

## Palette and materials

The `[palette]` table maps a single ASCII character either to the string
`"empty"` or to a namespaced material key — never an inline colour. A
grid character absent from the palette is refused, naming the character,
the part, the layer, the row and the column; a palette entry no grid uses
loads successfully but is reported as an `inspect` defect (below).

A material file declares:

```toml
# content/base/materials/flame.toml
name     = "base:flame"
color    = "#ff9a3c"
emissive = 0.8            # fraction of self-illumination; absent means 0.0
```

`color` is `#rrggbb` only, no shorthand — matching the HUD's rule that
shorthand accepted is shorthand silently mis-parsed. `emissive` is a
`0.0 … 1.0` **fraction of self-illumination**, not an integer light
level: the engine has no lighting model at all
(`technical/rendering.md`), so a 0–15 light level would be inventing an
engine decision inside an asset format. Materials are namespaced keys for
the same reason block textures are — a definition names a key and never
answers the renderer's question — which also keeps an art set consistent
across dozens of assets an agent generates at different times, rather
than picking `#8b5a2b` today and `#8a5b2c` tomorrow.

## Parts, attachment and states

Parts form a tree rooted at the one part declaring no `attach`. A child
part's local voxel at `p` occupies pre-normalisation position
`parent_position + attach.at + p − child.origin` — legitimately negative,
since a pivot exists precisely so a part may extend in `−x`, `−y` or
`−z` from it. The **assembled model is normalised** afterward so its
minimum corner is `(0, 0, 0)`.

A part may declare `states`; the first declared is the default. States
exist for content that is a genuinely different set of voxels between
frames — a flickering flame — as distinct from a rotation a script
performs at runtime (a door opening) or about an attachment point (an arm
swing), neither of which the format needs to represent as separate
geometry.

```toml
schema = 1
name   = "base:torch"
scale  = 16
slice  = "y"              # model default; a part may override it

[palette]
"." = "empty"
"w" = "base:oak_plank"
"f" = "base:flame"

[[parts]]
name   = "handle"
size   = [2, 10, 2]
origin = [1, 0, 1]
                          # no `attach` — this is the root
[[parts]]
name   = "flame"
size   = [4, 6, 4]
origin = [2, 0, 2]        # pivot at the flame's base centre
attach = { to = "handle", at = [1, 10, 1] }
states = ["low", "high"]  # first is the default
```

## What `inspect` reports

`voxforge inspect <file>` reports **defects**, which set a non-zero exit
code, and **observations**, which never do:

| | Reports | Exit code |
|---|---|---|
| **Stats** | Filled voxel count; inclusive bounding box (`(0,0,0)` to `(3,3,3)` for a solid 4³ model — inclusive, unlike `world-format.md`'s exclusive section-plane convention); per-material counts | — |
| **Defect** | An unused palette entry | non-zero |
| **Observation** | Connected components (6-connected, computed on the *assembled* model, so a correctly attached multi-part model reports one component); floating voxels; mirror symmetry about the x midplane | 0 |

Connectivity and symmetry are observations rather than defects because
the file gives no way to tell a detached hinge from a deliberate floating
element (a portal particle, a hanging lantern) apart — the tool reports
the fact and leaves the judgement to whoever asked.

**Connectivity is not a structural-soundness check.** A part attached by
a single voxel face is connected and reports cleanly, whether it is a
correctly braced joint or a cantilever hanging off nothing — 6-connectivity
is a graph property, and being physically supported is not one. See
`testing.md`'s account of the human sign-off for what this means in
practice for an authoring loop.

## `voxforge preview`

Renders the model from any of **fourteen** canonical views: `front`,
`back`, `left`, `right`, `top`, `bottom`, the four isometric corners
(`iso-fl`, `iso-fr`, `iso-bl`, `iso-br`), and the same four corners seen
from **below** (`iso-fl-under`, `iso-fr-under`, `iso-bl-under`,
`iso-br-under`) — added specifically because an overhead corner cannot
see the underside of a seat, shelf or overhang, and `bottom` alone is a
flat plan with no depth cue. Naming no `--view` renders a **contact
sheet** tiling all fourteen into one image, with the tile→view mapping
printed on stdout (there is no in-image legend — a rendered label cannot
be graded any more strongly than "some pixels are non-background", so the
mapping is a decidable stdout line instead).

Rendering is a CPU orthographic ray march: one opaque sample per pixel,
shaded in linear space before the sRGB encode (`+y` 1.00, `±x` 0.80, `±z`
0.65, `−y` 0.50 — a legibility aid for reading proportions, and not a
lighting model the engine is bound by), and deterministic — the same
model renders to byte-identical PNG output regardless of declaration
order. `--pixels-per-voxel` controls resolution (default 8); `--state`
selects a named part's state; `--materials` points at a materials
directory other than the default.

## `voxforge texture`

Emits a flat, unshaded, axis-aligned face of a model — `--face <name>`
for one of the six block faces, or `--all-faces` for all six from one
invocation (so a set can never disagree with itself: either all six
verdicts are computed and all six images written, or none are). Flatness
is a property of the *render*, not of a material: the only other flat
path is `emissive = 1.0`, and because the material table is shared across
every model, flattening a material to make one texture would silently
unshade every *model* using it.

Every emission computes and prints a **tileability verdict** — whether
the texture is fit to be drawn on a terrain face without showing a seam
where it repeats. It is always reported, and refuses the emission only
when requested with `--seamless` (default off: not every texture needs
to tile, and refusing a legitimate one-off decorative texture would be
the tool inventing a rule nobody asked for):

| Verdict | Meaning |
|---|---|
| `TilesAcrossEveryEdge` | Fit to be a block texture. |
| `PeriodIsNotOneBlock` | The model's extent on an in-plane axis is not exactly `scale` voxels, so the texture's period is not the block grid's. |
| `FaceIsNotOpaque` | Some pixel no voxel covers would show the void on a solid block face. |
| `EdgesDisagree` | Wrapping the texture introduces a bigger step than any the texture already contains within itself. |

The three legs (period, coverage, edges) are evaluated in that order.
Under `--seamless` the first failing leg refuses the emission; without
it, every leg is evaluated and every failure reported, because a texture
that can never pass one leg — glass, a leaf sprite — should still learn
whether its edges agree. See ADR-021 for why the verdict is judged
against the model's declared `scale` and against the texture's own
content, never against a second render of the same rasteriser.

Stdout prints one line per emitted face, in the fixed order `front`,
`back`, `left`, `right`, `top`, `bottom`: the face name, the path
written, the two model axes its columns and rows run along, and its seam
verdict. **The printed axis pair is the whole of what this tool does to
help a consumer avoid an orientation mismatch** — matching it to the
terrain mesher's own UV convention is deliberately left to whoever
consumes the texture (see `technical/rendering.md`); nothing here checks
it.

## Refusals

Every VoxForge refusal — a malformed document, an oversized part, an
unresolvable material, a bad view name — names the file, the offending
part or layer, and the specific field or value at fault, mirroring the
failure contract `blocks-items.md` sets for block definitions. `preview`
and `texture` never leave a partial or replacement file behind on
failure: a pre-existing output path is left byte-identical.

## What this format does not do

- **No engine consumption yet.** Nothing loads, meshes or draws a
  `.mcvox` file; voxel models are a later MVP.
- **No animation.** The format declares parts, pivots, the attachment
  hierarchy and discrete states — no rotations, keyframes, durations,
  interpolation or transitions. State sequences and keyframed clips are
  the named follow-up, deliberately deferred: something other than the
  engine will need to describe movement, and scripts are currently
  server-side only.
- **No editing verbs.** `fill_box`, `mirror`, `translate` and similar do
  not exist; you edit the grid text directly.
- **No `.vox` import or export.** Interchange with MagicaVoxel or vengi is
  deliberately not part of this format — it is a separate piece of work with
  its own trade-offs, and folding it in here would tie this format's shape to
  another tool's. Author in the grid text for now; if interchange lands it
  will read and write these files rather than replace them.
- **No procedural generation, morphology or image-to-voxel carving.**
