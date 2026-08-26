# Authoring Voxel Models

How to describe a voxel model — a door, a torch, a chair, a prop — as a
text file, and how to see what you made. The format and the tool that
reads it are both called **VoxForge** (`tools/voxforge`, `voxforge` CLI).
The feedback loop is the feature: an agent authors a `.mcvox` file, runs
`voxforge preview` or `voxforge inspect`, and corrects the file from what
it sees. The editing verbs (`fill_box`, `mirror`, translate) are
deliberately not built — you edit the text.

**Nothing in `crates/` loads, meshes or draws a `.mcvox` file** — that is
still true, and it is not the same as saying a model reaches nobody.
**A model reaches the running game by being baked first:** `voxforge
build` turns a model's faces into PNG images and an index, and the client
reads *those* at launch. So the model is read by this tool alone, and
what a player sees is its output — the base game's grass, dirt and stone
are drawn that way today (see "`voxforge build`" and "The set at launch",
below). `tools/` may
depend on `crates/`; the reverse never holds — see
`technical/architecture.md`'s mechanically enforced invariants, which is
why the client reads baked images rather than calling VoxForge.

## File layout

One model per file, TOML, conventionally under `content/<mod>/models/` —
but the tool has no opinion about where a file lives and accepts any
path. One material per file under `content/<mod>/materials/`, mirroring
`content/<mod>/blocks/`'s one-definition-per-file convention
(`blocks-items.md`): the materials directory is read in **file-name
sorted order**, and that order is binding, not tidiness — a duplicate-name
error names a first file and a second file, which is only well defined
under a fixed order.

## Where the shipped models came from, and what is the source of truth

The `.mcvox` file **is** the source of truth for a model's art. It is what the
tools read, what the repository tracks, and what a change to the art is made
against.

`content/base/models/generators/` holds the scripts that produced the shipped
block models — `gen_grass.py` and `assemble_grass.py` for `grass-block.mcvox`,
`gen_stone.py` for `stone-block.mcvox`, and `gen_water.py` for
`water-block.mcvox` — beside two more that only read and measure. They are
**provenance, not a build step**. Nothing in the build, the gate or the client
runs them; `cargo build` and the quality gate never invoke Python, and a
contributor never needs it installed. Each script's own header says the same
thing.

**A model citing a generator says where the file came from. It does not claim
that re-running the generator would reproduce it** — that direction is not a
claim of any kind. Two of the four do not run as they stand, and this is
deliberate: `assemble_grass.py` names an absolute path from the machine it ran
on and reads an intermediate that was never committed, and `gen_stone.py`
writes a file name the repository does not use. Repairing them would invite
exactly the belief the headers exist to remove.

`gen_water.py` is the exception in the other direction — it writes the very file
the repository tracks — and its own header says why that is still not licence to
re-run it: changing the density constant would silently change the art under a
manifest that would go on folding to a new value and calling every checkout
stale.

They are kept for what cannot be recovered from the pixels. The five
hand-authored courses in `assemble_grass.py` — the sod shadow and three lone
blades at deliberately different depths — are design intent recorded nowhere
else. The model carries the result and not the reasoning.

**Water's palette is the other piece of reasoning the pixels do not carry.**
`base:water` is `#4c799e`; `base:water_dark` and `base:water_light` sit exactly
eight bytes below and above it on every channel. Among the four terrain blocks
stone is the only other one whose accents step uniformly, and it steps 29 — the
step tracks how rough the surface is, and open water is the smoothest thing the
base game ships. (Dirt's and grass's accents step by a different amount on each
channel; `grass_dark.toml` states the rule underneath all of them, and
`water_light.toml` says why water's two are symmetric about the base.)
Saturation and value were read off the two chromatic terrain bases: water's
S 52 % and V 62 % each sit between dirt's (S 50 %, V 55 %) and grass's (S 53 %,
V 66 %), so a shoreline reads as one palette rather than as two. **The hue was
not read off anything** — 207° was approved by a person, because "water is
blue" is not a judgement a measurement settles, and it is one hex character to
reverse.

For the record, because it is the measurement that settled the question: **every
shipped block model has been reproduced from its generator byte for byte.**
Grass and stone were reproduced once, under a temporary repository root, before
this documentation was written; `water-block.mcvox` was re-derived from
`gen_water.py`'s own arithmetic on 2026-08-26, the day the sea's art was baked.
That is what a standing test would have re-proved, and it is why there is none —
the scripts are frozen and the models are committed, so nothing can change the
answer.

**To change a shipped block's art, edit the `.mcvox` file.** Do not attempt to
regenerate it.

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
verdict.

**The convention this tool bakes under, stated rather than left to a
consumer to match.** For each face, the image's top edge runs toward the
face's own up and its right edge toward the right of a viewer standing
outside it looking at it. The two horizontal faces have no world up in
them, so theirs is chosen: the top image's top edge runs toward `-z` and
the bottom image's toward `+z`. Measured against
`content/base/models/grass-block.mcvox`'s own outermost voxels, every one
of the six shipped images agrees with that texel for texel.

**This used to say that matching the axis pair to the renderer's UV
convention was "deliberately left to whoever consumes the texture", and
that sentence is the record of how a defect shipped.** The renderer
matched it wrongly on five of six faces — east and west turned a quarter,
north and south and the underside flipped, north laterally reversed as
well — and every reading in the suite was blind to it, because each one
measured *which colours* a face holds and none measured *where they sit*.
An orientation contract that neither side states is one neither side can
be wrong about, so both were. It is stated here now, `technical/rendering.md`
holds the renderer's half, and `FR-8.1-S6` compares this tool's output
against the model it is a view of.

## `voxforge build`

A block declaration names a **texture key** — `texture = "base:stone"` —
and never a file path (`blocks-items.md`). Something has to say which
pixels that key stands for, and `voxforge build` is it: one command that
reads a **texture manifest**, bakes the faces the manifest names into a
directory of PNGs, and writes an **index** beside them recording what the
set was built from.

`voxforge texture` emits face images to a path you choose, one model at a
time. `build` bakes a whole named set from a file you commit, so that
"what art does this mod ship" is a reviewable diff rather than a
remembered command line.

```
voxforge build content/base/textures.toml
```

**The client now judges a set at every launch, and refuses to start on
one that is missing or out of date.** It also *draws* from it: a key your
set covers is painted with the image you baked for it, and a key it does
not cover falls back to a generated stand-in — a flat two-tone
checkerboard whose colour pair is derived from the key's own name, which
is what a first block looks like before you have baked anything. So the
set you build today is both a set the client holds you to and the
pictures you see.
[The set at launch](#the-set-at-launch) is what it checks, what each
refusal says, and how to clear it, and
[Two things a missing texture can mean](#two-things-a-missing-texture-can-mean)
is why the stand-in exists rather than an error.

**A checkerboard on screen means that key is not in your manifest.** It is
not a broken image and not a launch failure — it is the one thing the
client cannot tell you about, because a block declaring a key nothing has
baked is a legitimate state and the launch is designed to proceed. The
base game shipped a sea drawing exactly that for three days. The colour
pair is the key's own, so it is the flat two-tone checkerboard and not any
particular colour that is the tell: `base:water` comes up magenta, but
`mymod:ore` comes up whatever its name hashes to. If a face draws one, the
key it draws is missing an entry in your `textures.toml`; add one and
re-run `voxforge build`.

**A built set is not committed.** It is deterministic and free to
reproduce, so the repository carries the models and the manifest and
regenerates the images — see ADR-026 in `technical/decisions.md`. The
base game's own output directory, `content/base/textures/`, is in
`.gitignore` for that reason.

**The quality gate builds the base game's set and refuses a committed
one.** A full `scripts/sdd-gate.ps1` run bakes
`content/base/textures.toml` before it runs the tests, and fails if `git`
reports any file under `content/base/textures/` — naming each path, so a
deleted `.gitignore` line is a failure with an address rather than a
silent regression. Both are described for contributors in
`technical/testing.md`.

That is the *base game's* set and no other. A mod of your own is not
built by this repository's gate: you run `voxforge build` against your
own manifest, and you keep your own output directory out of version
control the same way. The gate takes a `-ContentRoot` and a `-Manifest`,
but they exist so its own tests can point it at a fixture, not as a way
to enrol a mod in somebody else's gate.

### Where the files live

```
content/<mod>/
  textures.toml         the manifest — committed
  models/*.mcvox        the models it names — committed
  materials/*.toml      the materials those models paint from — committed
  blocks/*.luau         the block declarations, scanned for unused keys
  textures/             the built set — derived, never committed
    <key>.png           one image per entry
    index.txt           what the set was built from
```

Every path a manifest states is **relative to the manifest's own
directory**, and the index records them the same way. That is what lets a
content root be copied somewhere else and still be recognised as current:
an index of absolute paths would be stale the moment the tree moved.

### The manifest

| Field | Type | Bound |
|---|---|---|
| `output` | directory path, relative to the manifest | Required. Made if it is not there. The images and the index are written directly inside it. |
| `materials` | directory path, relative to the manifest | Required. Every `*.toml` directly inside it is a material declaration, read in file-name order — and every one of them is a source of the set, whether or not a model paints with it. |
| `blocks` | directory path, relative to the manifest | Required as a *field*; the directory itself need not exist. Every `*.luau` directly inside it is scanned as text for unused keys, and nothing else is done with it. |
| `pixels_per_voxel` | integer, at least 1 | Required. A block texture is 16 texels on an edge, so `model.scale` times `pixels_per_voxel` must equal exactly 16 — a `scale = 16` model bakes at 1, a `scale = 8` model at 2. |
| `[[texture]]` | array of tables | Zero or more. A manifest stating none is legal and writes an index naming nothing. |

Each `[[texture]]` table states three fields and no others:

| Field | Type | Bound |
|---|---|---|
| `key` | namespaced id | Exactly one `:`, non-empty on both sides. No two entries may state the same key. It must carry no control character, and its derived image name must be an ordinary file name — see below. |
| `model` | path to a `.mcvox` file, relative to the manifest | The file must exist. The model must be a cube of its declared scale, because a face set is a block's six faces. |
| `face` | one of `front`, `back`, `left`, `right`, `top`, `bottom` | The six faces a **block** has. The isometric views `preview` offers are not faces and are not accepted here. |

Unknown fields are refused, at the manifest level and inside each entry,
for the reason the rest of this format refuses them: a typo that is
silently ignored is a manifest that does not do what it says.

**One model may paint several blocks.** The base game bakes six of the
grass block's faces to six keys, and its underside to `base:dirt` — dirt
is the bottom of the grass model rather than a second model holding the
same voxels. That is the whole reason an entry names a *face* and not
just a model.

### The image file name, and why a key does not become a path

A key's image is written under its own text with each `:` replaced by
`__` and `.png` appended: `base:grass_top` becomes `base__grass_top.png`.

A texture key has **no character set imposed on it** — it is
`namespace:path` with exactly one separator, and nothing else — so
`base:../../x` is a perfectly legal key. Deriving a file name from one is
therefore deriving a path from unconstrained content text, and the build
refuses any key whose derived name is not a single ordinary file name:
letters, digits, `.`, `-` and `_`, ending in `.png`.

**This is a correctness and reproducibility rule and not a threat
model.** A server owner chooses which mods are installed, so a manifest
is not hostile input. The reasons that hold without that argument are
plainer: a build whose output location depends on punctuation in a key is
not reproducible, and a key that will not round-trip through the index is
a defect in a file another program parses.

### What the build writes, and what it reports

Every source is read, every model rendered, every image encoded and the
index rendered **before any file is opened**. A manifest refused on its
fourth entry therefore leaves the previous build's set exactly as it was:
a set on disk is the whole of one build or the whole of another, never
three entries of a run that stopped.

On success, stdout carries one line per file written — the images and
then the index — and a path on stdout is a promise that the file is
there. Any advisory lines follow after them.

The index is a text file, one record to a line:

```
mycraft-texture-set 1
fold b9a7b041e0760eed
source textures.toml
source models/plank-block.mcvox
source materials/plank.toml
key example__plank_top.png example:plank_top
key example__plank_side.png example:plank_side
```

`fold` is a single FNV-1a-64 value over **exactly the sources the
manifest reached**, in the order the `source` lines record them: the
manifest itself, then each model in the order of its first mention, then
every `*.toml` in the materials directory sorted by file name. A model
under `models/` that no entry names is **not** folded — otherwise editing
an unrelated model would make your textures stale.

A `key` record is written **image first, key last**, because a key may
contain whitespace and an image file name may not: only one of the two
can be the rest of the line.

### The cache

A second build over unchanged sources does nothing:

```
nothing needed rebuilding
```

The fold is the cache key, and it is whole-set: if it matches the value
the index records **and** every image the index names is present, no file
is opened and no byte is written. Otherwise the whole set is rebuilt.
There is no per-entry caching, and there deliberately is not: it would
need a second, finer-grained record that every reader of the index would
then also have to understand, for seven small images.

The images are checked for **presence** and not for content, which has
one consequence worth stating: hand-editing a built image survives a
build. What says a set is current is the fold over its sources, and a
built image is not a source. Delete the index — or the directory — to
force a rebuild.

### Keys nothing draws with

A key the manifest bakes that no block declaration spells is reported and
the build completes:

```
`example:plank_side` is baked here and named by no block declaration
```

Never a refusal: the manifest and the block files are edited by different
hands at different times, and a build that stopped because a block had
not been written yet would be wrong about which of the two is unfinished.

**The scan reads each `blocks/*.luau` as text, and has one limitation you
should know about.** It looks for the key's spelling; it does not run
your script. A declaration that *computes* its key — `texture = "base:" ..
kind` — is not seen, and its key is reported unused. That is the price of
not starting a script host inside an art tool, where one broken block
declaration would otherwise refuse a texture bake that has nothing to do
with it. A false positive costs one line of output. A `blocks` directory
that is not there reports every key, for the same reason.

### Every refusal, and how to read it

Each names the file it is about, the field at fault where one field is,
and the value you typed. The build exits non-zero and writes nothing.

The manifest itself:

```
nowhere.toml: no texture manifest could be read there: The system cannot find the file specified. (os error 2)
```

```
bad.toml: TOML parse error at line 3, column 6
  |
3 | this is not toml
  |      ^
key with no value, expected `=`
```

An entry:

```
face.toml, field `face`: `side` is not a face a texture entry may select — a block has six, front, back, left, right, top, bottom
```

```
dupe.toml, field `key`: `example:plank_top` is stated by two entries, and one key names one image
```

```
sep.toml, field `key`: `example:plank:top` has more than one namespace separator — a namespaced id is written `namespace:path`
```

```
slash.toml, field `key`: `example:plank/top` would have its art written as `example__plank/top.png`, which is not a single ordinary file name — a key's image is its text with each `:` replaced by `__` and `.png` appended, and the result may hold only letters, digits, `.`, `-` and `_`
```

A key carrying a line break is refused separately, because written down
it would forge a record in a file another program parses — and the
refusal quotes the key across the break it carries:

```
linebreak.toml, field `key`: `example:plank
top` carries a control character, so it cannot be written to an index, which states one record to a line
```

A source the manifest reaches. The refusal names the file and, where an
entry named it, the key whose entry did — a manifest of seven entries
naming one missing model otherwise leaves you reading seven lines to find
out which:

```
models/missing.mcvox: this source could not be read, named by the entry for `example:plank_top`: The system cannot find the file specified. (os error 2)
```

A model that will not bake to a block texture. All three name the model
rather than the manifest, because the model is the file to change:

```
models/short-block.mcvox, field `all-faces`: a face set is a block's six faces, so the model must be a cube of the declared scale 16 — this one is 15 voxels on the z axis, and assembles to 16 by 16 by 15
```

```
models/plank-block.mcvox, field `scale`: a block texture is 16 pixels on an edge, but this model's declared scale of 16 at 2 pixel(s) per voxel bakes 32
```

```
models/ramp-block.mcvox, field `face`: the top face, which the entry for `example:plank_top` bakes, will not tile: the edges disagree along the x axis: row 0 steps 255 across the wrap, where its largest step within is 85
```

The last is the seam verdict from `voxforge texture` above, now binding —
but binding **only on the faces some entry selected**. A model whose
`front` face does not tile builds perfectly well if no entry asks for
`front`; refusing on a verdict for a face your manifest never wanted
would refuse a set for a picture nobody is going to draw. The scale
refusal exists because a model's `scale`, a manifest's
`pixels_per_voxel` and the 16-texel edge a block texture has are three
numbers with nothing connecting them: caught here, you are pointed at the
model; uncaught, a 32x32 set builds cleanly, commits cleanly, and refuses
a launch later with a message about an image you never authored.

### A complete example, from nothing to a named set

Four files and one command. Written out under `content/example/`, though
the tool has no opinion about where a content root lives.

`content/example/materials/plank.toml`:

```toml
name     = "example:plank"
color    = "#b07d4a"
emissive = 0.0
```

`content/example/models/plank-block.mcvox` — a solid 16-cube, sixteen
identical layers. The first is written out; the other fifteen repeat it
with `y = 1` through `y = 15`:

```toml
schema = 1
name   = "example:plank_block"
scale  = 16
size   = [16, 16, 16]
origin = [0, 0, 0]
slice  = "y"

[palette]
"." = "empty"
"p" = "example:plank"

[[layers]]
y = 0
grid = """
pppppppppppppppp
pppppppppppppppp
pppppppppppppppp
pppppppppppppppp
pppppppppppppppp
pppppppppppppppp
pppppppppppppppp
pppppppppppppppp
pppppppppppppppp
pppppppppppppppp
pppppppppppppppp
pppppppppppppppp
pppppppppppppppp
pppppppppppppppp
pppppppppppppppp
pppppppppppppppp
"""
```

`content/example/blocks/plank.luau`:

```lua
return {
	name = "example:plank_block",
	texture = "example:plank_top",
}
```

`content/example/textures.toml`:

```toml
output           = "textures"
materials        = "materials"
blocks           = "blocks"
pixels_per_voxel = 1

[[texture]]
key   = "example:plank_top"
model = "models/plank-block.mcvox"
face  = "top"

[[texture]]
key   = "example:plank_side"
model = "models/plank-block.mcvox"
face  = "front"
```

Then:

```
$ voxforge build content/example/textures.toml
content/example/textures/example__plank_top.png
content/example/textures/example__plank_side.png
content/example/textures/index.txt
`example:plank_side` is baked here and named by no block declaration
```

Two 16x16 images and an index. The advisory line is correct, and it is
the loop working: `plank.luau` declares `example:plank_top` and nothing
declares `example:plank_side` yet. Run it again and it says `nothing
needed rebuilding`; touch the model, or any material file, and it bakes
the set afresh.

## The set at launch

`voxforge build` produces a set. The client **judges** one, once, at every
launch, before it generates anything — and either starts or tells you in
one sentence what to run. Then it **draws** it: every key the set covers has
its layer of the array texture filled from that key's PNG, decoded once at
startup, and every key the set does not cover falls back to the generated
stand-in. So this section is about two things at once — whether your launch
begins, and what your blocks are painted with when it does.

The client comes to exactly one of six verdicts about a content root.
Four of them stop the launch and two of them do not, and the split is the
whole point: a set that was never built is a command away from working,
and a content root that ships no art at all is not broken.

| The verdict | What it means | Does it stop the launch? |
|---|---|---|
| declares no art | the root has no `textures.toml` | no |
| absent | there is a `textures.toml` and no set built from it | **yes** |
| stale against its sources | a source has been edited since the build | **yes** |
| a source is missing | a file the index records is no longer there | **yes** |
| an image is missing | the index names a PNG the set does not hold | **yes** |
| current | the set matches the sources it records | no |

### It re-folds what the index recorded, and never reads your manifest

The index the build wrote carries the list of sources it folded, in fold
order, each path relative to the manifest's own directory. The client
reads *that list* back, re-reads those same files in that same order, and
folds them again. It does not open `textures.toml`.

Two consequences you can rely on:

- **A source you add after a build does not make the set stale.** Drop a
  new material into `materials/` and the client says nothing, because the
  index does not record it. The *build* folds it and rebuilds the set the
  next time you run one, and the launch after that is what sees the
  change.
- **Moving or copying a content root does not make its set stale.** Every
  recorded path is relative, so a root copied anywhere re-folds to the
  value it was built with.

### Nothing to build, and nothing to be stale against

A content root with no `textures.toml` declares no art. It launches, and
every face of every block it declares is drawn from a generated texture —
the same placeholder art the base game shipped before any of this
existed.

This is deliberate, and it is the reason the "absent" verdict is separate
from it. Your first mod is a `blocks/` directory and nothing else, and
being told to run an art build for art you have not written yet would
blame the wrong party. Write `textures.toml` on the day you have models
to bake, and not before.

### The four refusals, and how to clear each

Each is one line, printed as the client exits. None of them leaves a
window open.

**The set was never built.** The most common one by a distance, and the
one a fresh checkout gets:

```
mycraft: the generated texture set is not there; run `cargo run -p voxforge -- build content/base/textures.toml`
```

Run the command it names, against your own manifest. It is the same
sentence whether you have never built the set or deleted its `index.txt`
by hand: the index *is* the set, as far as the client is concerned.

**A source changed after the build.** You edited a model, a material or
the manifest itself:

```
mycraft: the generated texture set is stale against its sources; run `cargo run -p voxforge -- build content/base/textures.toml`
```

The same command. The build folds the sources afresh, finds the fold does
not match what the index records, and bakes the whole set again.

**A source the build recorded is gone.** You moved or deleted a file the
set was baked from, so there is nothing to fold and nothing to compare
against:

```
mycraft: the generated texture set was built from `materials/dirt.toml`, which is no longer there; run `cargo run -p voxforge -- build content/base/textures.toml`
```

It names the path **as the index records it** — relative to the content
root, `/`-separated on every platform — so you can search `index.txt` for
exactly that text. If the file moved on purpose, rebuild; if it moved by
accident, put it back.

**An image the index names is not in the set.** The index says a key's
art lives in a file, and the file is not there:

```
mycraft: the generated texture set names the art for `base:stone` as `base__stone.png`, which is not there; run `cargo run -p voxforge -- build content/base/textures.toml`
```

Usually somebody emptied the output directory and left the index behind.
Rebuilding writes both.

### The four ways a set is not readable at all

These are a different kind of answer. The four verdicts above say *what
the set is*; these four are a set the client can come to no verdict about
or cannot fill a layer from. Every one of them names the record at fault,
and the two about an image name the **key** as well as the file — the file
name is derived from the key and is one you never typed, so a message
carrying only the file leaves you holding a name that appears in nothing
you wrote.

**A recorded path an index may not carry.** A manifest may name a model
outside its own directory — `model = "../shared/grass-block.mcvox"` —
and the build accepts it, bakes it, and writes it into the index. The
client then refuses to read that index:

```
mycraft: `content/base/textures/index.txt` is not a texture set index this client can read: line 3: `../shared/grass-block.mcvox` is not a path an index may record — every one is relative to the content root, `/`-separated, and names no parent directory
```

**This is a real dead end, and it is worth knowing before you arrange a
repository around it.** One model tree shared by several content roots is
an ordinary thing to want, and it is not supported: every source a set is
built from has to live under the content root that declares it. Move the
models in, or give each root its own copy. The build ought to refuse this
at bake time rather than letting you find out at launch, and that it does
not is written down here rather than left for you to discover.

**An image name that is a path.** A key may contain almost anything; an
image file name may not. The build derives the name once and refuses to
bake one it would derive badly, and the client applies the same rule to
the name it reads back:

```
mycraft: the texture set index names the art for `base:grass_top` as `elsewhere/base__stone.png`, which is not a name a set's image may be written under
```

You will not see this from a set the current tool built. It is what
stands between you and a set built by an older or a patched one, since
the client joins that name onto a directory and opens it.

**An image no layer can be filled from.** A layer of the array texture is
sixteen texels on a side, and the build refuses a model whose scale and
`pixels_per_voxel` do not come to that — naming the *model*, which is the
thing you can fix. The set on disk is checked again anyway, because it is
an ordinary directory somebody can write into:

```
mycraft: the art for `base:stone` is 32x32 and a layer of the array texture holds 16x16
```

**A file that is not a PNG.** The same reasoning, for a file that never
decoded at all:

```
mycraft: the art for `base:stone` at `content/base/textures/base__stone.png` is not a PNG this client can read
```

Neither is reachable from a set this tool built, and both are here
because the set is derived, git-ignored and therefore not reviewed: a set
assembled by hand, patched, or copied out of an older build is a set the
client is handed all the same. Uploading a 32 x 32 image into a 16 x 16
layer is a buffer overrun, so it is refused rather than trimmed.

### The directory the client looks in is `textures/`, always

**A manifest states its own `output`, and the client does not read it.**
The client looks for the set under `<content root>/textures/` and nothing
tells it otherwise, because the manifest is the build's input and the
index is the client's — two programs parsing one file format is the drift
this arrangement exists to make impossible.

The consequence is sharp, and you should meet it here rather than at
midnight. Write

```toml
output = "art"
```

and `voxforge build` succeeds, prints the files it wrote into
`content/yourmod/art/`, and everything about the build is correct. The
launch then refuses with *the generated texture set is not there; run*
— the build you have just run — and it will say that forever, however
many times you run it. Nothing in either message mentions `output`, so
there is nothing to lead you to the cause.

**So: leave `output` at `textures`.** It is stated in the manifest to
keep the set's location reviewable, not to be varied. Closing this
properly means recording the output directory in the index, which changes
a format two programs share, and it belongs to the change that makes it
rather than to a footnote here.

### Two things a missing texture can mean

The refusals above are all about the *set*. A key the set does not cover
is a different thing entirely: it is the ordinary state of a mod author's
first block, it costs you a generated texture rather than a launch, and
it is never a refusal. A set that is current while covering none of the
keys your blocks declare starts the client without a word.

That fallback is per key and not per set. One key covered and one not, in
the same content root, gives you one block drawing its art and one drawing
a stand-in — there is no all-or-nothing about it, so adding a model at a
time works.

That separation is deliberate and it is held by a test. If the two ever
came to say the same thing, somebody who forgot to run a build would go
looking for a declaration they never wrote.

**What a stand-in looks like, so you can recognise one on sight.** It is a
sixteen-by-sixteen checkerboard of two colours a byte-hash of the key
itself picks — deliberately implausible, so no stand-in is ever mistaken
for art somebody drew. `base:water` generates a magenta pair,
`(160, 58, 151)` against `(140, 38, 131)`; a different key generates a
different pair. Far enough from the camera the two average into one flat
colour and the checkerboard disappears, so a distant surface reads as a
solid implausible colour rather than as a grid.

**Nothing will tell you about it, and that is the design.** The launch is
correct, the set is current, the block is drawn, and no refusal is owed —
so the only instrument that reports a stand-in is your own eye. The base
game shipped a magenta sea for three days for exactly that reason, past
1 300 passing tests: the key was declared in `water.luau`, the manifest
had no entry for it, and every layer of the chain behaved as designed. If
you ship a content root and want the stricter rule the base game holds
itself to, compare the keys your `blocks/*.luau` declare against the `key`
lines of your built `textures/index.txt` — they should be the same list.

## Refusals

Every VoxForge refusal — a malformed document, an oversized part, an
unresolvable material, a bad view name — names the file, the offending
part or layer, and the specific field or value at fault, mirroring the
failure contract `blocks-items.md` sets for block definitions. `preview`
and `texture` never leave a partial or replacement file behind on
failure: a pre-existing output path is left byte-identical. `build`
extends that across a whole set — a refusal anywhere leaves every image
and the index from the previous build exactly as they were, which is why
its refusals are all raised before the first file is opened.

## What this format does not do

- **No engine consumption of the model itself.** Nothing loads, meshes or
  draws a `.mcvox` file — voxel models as *geometry* are a later MVP. What
  the engine consumes is the flat texture `voxforge build` bakes out of a
  model's face, which is a picture and not a shape.
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
