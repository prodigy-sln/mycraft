# Authoring the HUD

How to declare what the player sees on top of the world — a crosshair, a
swatch showing the block a placement would use, a marker in a corner. The
HUD is **content**, exactly as blocks are: there is no HUD definition
anywhere in Rust, and the base game's crosshair is declared in a file a mod
could have written.

This contract deliberately mirrors block authoring
(`modding/blocks-items.md`) field for field where it can — one file per
element, a `<root>/<kind>/*.toml` directory read non-recursively in
file-name sorted order, namespaced ids, unknown fields rejected,
all-or-nothing loading, every failure naming file, element and field. A
content author learns one contract, and MVP 2 swaps the loader without
touching the model.

## File layout

One element per file, under `<mod>/hud/`:

```
content/<mod>/hud/<element>.toml
```

The loader reads every `*.toml` file directly under `<root>/hud/`
**non-recursively**, in file-name sorted order. Files that are not `*.toml`
are ignored — a `notes.md` beside your declarations is fine and registers
nothing. A subdirectory is not descended into: `hud/experiments/big.toml`
declares nothing.

**The sort is binding, not an implementation detail**, and it decides two
observable things rather than one:

- When two files declare the same element `name`, the failure names a
  *first* and a *second* file, and which is which is only well-defined if
  the directory is always read in the same order.
- **Overlapping elements resolve by file name.** Within each of the two
  composition passes (below), later-sorted files draw over earlier-sorted
  ones. `zulu.toml`'s fill covers `alpha.toml`'s where they overlap,
  whatever order the two files were created in.

Loading the same content root twice in one process produces the identical
element order both times.

## Fields

A declaration is a top-level TOML table with four required fields, two
conditionally required ones, and two optional ones:

```toml
name    = "base:crosshair-horizontal"  # namespaced, required
anchor  = "center"                     # required, one of the nine anchors
size    = [9, 1]                       # required, UI units, both strictly positive
draw    = "fill"                       # required: "fill" | "block-texture"

color   = "#FFFFFFFF"                  # required for draw = "fill", forbidden otherwise
source  = "held-block"                 # required for draw = "block-texture", forbidden otherwise

offset  = [0, 0]                       # optional, default [0, 0], UI units, +x right +y down
outline = "#000000FF"                  # optional, absent means no outline
```

- **`name`** — the element's namespaced id. Same syntax as a block name:
  **exactly one `:`, non-empty on both sides**. `crosshair` is rejected
  (no namespace); `base:hud:crosshair` is rejected naming the extra
  separator, rather than being read as an element called `hud:crosshair`.
  HUD element names and block names live in **separate namespaces** and may
  collide with no consequence — a mod may ship both a block and a HUD
  element called `mod:marker`.
- **`anchor`** — which part of the screen the element is placed against, one
  of the nine names below. A misspelling fails the load naming `anchor` and
  listing all nine accepted values.
- **`size`** — `[width, height]` in UI units. Both components must be
  **strictly positive**: `size = [0, 4]` is refused. There is no default —
  an element with no stated extent is a mistake, not an invisible element.
- **`draw`** — what kind of drawing this is: `"fill"` paints a solid
  rectangle in a declared colour, `"block-texture"` paints a block's texture
  into the rectangle. Any other value fails the load naming `draw` and
  listing the accepted kinds.
- **`color`** — the fill colour, `#RRGGBBAA`. **Required when
  `draw = "fill"` and forbidden otherwise.**
- **`source`** — which live engine value a `block-texture` draw reads.
  **Required when `draw = "block-texture"` and forbidden otherwise.** The
  published set is listed below.
- **`offset`** — `[x, y]` in UI units, displacing the element from where its
  anchor would otherwise put it. **+x is right and +y is down.** Absent
  means `[0, 0]`. Components may be negative.
- **`outline`** — a `#RRGGBBAA` colour for a one-UI-unit ring immediately
  surrounding the element's rectangle. Absent means no ring.

**Unknown fields are rejected**, naming the field. A declaration carrying
`wobble = true` fails to load rather than being registered with a field that
can have no effect — a silently ignored typo is a debugging trap for
whoever wrote the file.

**A field that is present but the wrong shape is rejected, naming the
field.** `size = ["9", 1]` and `offset = ["9", 1]` both fail rather than
being read as `[0, 0]`. That matters more for `offset` than it looks: an
element quietly registered at `[0, 0]` is a crosshair that silently moved,
with no fault to point at.

**A field that can have no effect is rejected rather than ignored.**
`draw = "fill"` with a `source`, or `draw = "block-texture"` with a
`color`, fails the load naming the surplus field. When a declaration is
wrong in both directions at once — a `"fill"` carrying a `source` and no
`color` — either field is a defensible thing for the message to name, and
which one it names is not pinned.

## The nine anchors

```
top-left        top        top-right
left           center           right
bottom-left    bottom    bottom-right
```

`center` is measured from the centre of the render target. **Every other
anchor is measured from a 5% safe-area inset on each axis, never from the
raw screen edge** — at 1280×720 that is 64 pixels horizontally and 36
vertically. An anchor's named edges sit on that safe-area box; the axis it
does not name is centred on the target. So `bottom-right` with no offset
puts the element's right edge 64 pixels from the target's right edge and its
bottom edge 36 from the bottom, and `bottom` centres the element
horizontally and puts its bottom edge 36 pixels up.

`center` is deliberately **not** inset — the screen centre is reserved for
the crosshair, and insetting it would move it off the point it exists to
mark.

## UI units, and why a declaration is resolution-independent

**One UI unit is one physical pixel at a render-target height of 720, and
scales linearly with height.** A `[9, 1]` element covers 9×1 pixels at
1280×720, 14×2 at 1920×1080 and 18×2 at 5120×1440. Nothing in a
declaration is an absolute pixel, so one set of declarations covers every
supported resolution.

Scaled extents round **half away from zero** and are floored at one pixel
per axis, so a thin bar never disappears entirely on a small target. Offsets
scale by the same factor. An outline is one UI unit thick and scales the same
way.

There is no user-facing UI-scale setting: composition is relative to the
render target, and the multiplier is not something a player or a
declaration adjusts.

A rectangle whose offset pushes it partly off the target is **clipped**.
Nothing is written outside the target and nothing wraps to the opposite
edge.

## Colours are `#RRGGBBAA`, with all eight digits

Eight hex digits, no shorthand. `#FFF` and `#000` are refused, naming the
field they were written in — for `color` and for `outline` alike. This is
strict now and relaxable later, the same direction the namespaced-id rule
takes: a stricter rule can be loosened without invalidating content already
written against it, and the reverse breaks everything.

**Alpha composites in linear space, so a translucent colour does not read
back as the hex digits suggest.** `#FFFFFF80` over black shows `#BCBCBC`,
not `#808080` — the blend happens in linear light and the result is
re-encoded for display (`technical/rendering.md` §"The HUD pass" has the
arithmetic). Fully opaque colours are unaffected: `#808080FF` shows
`#808080`. Every element the base game ships is opaque, so alpha is a
capability of the format rather than something the shipped HUD's legibility
rests on.

## Outlines, and why every base element declares one

Outlines compose in **their own pass, before any fill**: every declared
outline first, in file-name sorted order, then every fill, in file-name
sorted order. Two consequences worth knowing while authoring:

- **A single pass would let one element's outline cut a notch through
  another's fill.** That is exactly what the base crosshair's two crossing
  bars would do to each other, which is why the two-pass rule exists rather
  than being a polish detail.
- **An earlier-sorted element's fill covers a later-sorted element's
  outline**, because all fills land after all outlines.

The outline is drawn as a **ring** — the four strips between the expanded
rectangle and the fill rectangle — not as a solid rectangle underneath the
fill. A translucent fill therefore blends against the scene rather than
against its own outline colour.

**The world behind the HUD is any colour, any brightness, and it moves, so
no element should sit on it untreated.** A white crosshair has to survive
snow *and* an unlit cave, and the ring is what makes it visible against
terrain at all. Every element `content/base/` ships declares an `outline`,
and that is asserted mechanically over the shipped declarations rather than
left to review — the check reads the parsed field, not the file's text,
because two of the three shipped files also mention *outline* in a prose
comment and a text search would have been satisfied by the comment alone.
For third-party content this is guidance rather than an enforced rule.

## Draw kinds and readable values are closed, published sets

- **Draw kinds: exactly `{"fill", "block-texture"}`.**
- **Readable values: exactly `{"held-block"}`.**

`source = "hotbar"` fails the load naming `source` and listing the published
readable values — because there is no hotbar, and a declaration naming a
value the engine does not publish is a typo the author needs to see.

**Content names a readable value; it cannot compute with one.** `held-block`
resolves to the texture of the block a placement would currently use, and
that is the whole of the binding. There is no arithmetic, formatting or
comparison available anywhere in a declaration.

**A `block-texture` element whose source resolves to nothing draws nothing
at all — including its outline.** Before the world has been prepared there
is no held block, so the swatch is absent rather than being a black ring
around empty space. If a held block's texture key resolves to no texture
layer, the element again draws nothing and the unresolved key is reported
naming the block, rather than another block's texture being drawn in its
place.

## The engine knows a rectangle; it does not know what a crosshair is

`fill` and `block-texture` are **drawing capabilities**, not element types.
The base game's crosshair is *composed in content* from two crossing
`fill` bars — there is no `kind = "crosshair"`, because that would put the
shape of a crosshair into Rust, which is the violation this whole design
exists to avoid, and it would cost the same to build.

Consequently a mod's HUD is not a lesser citizen than the base game's:
`content/base/hud/` uses exactly the fields and exactly the two draw kinds
any other content root has.

## All-or-nothing loading, and a fault stops the launch

Loading a content root registers **every** element it declares or **none**
of them. One declaration stating `size = [0, 4]` means all three of a
three-file root are refused; the registry is never left holding a partial
HUD.

Every failure names its **origin** — the file it came from — the **element**
it was declared under, where the file could be read far enough to find a
name, and the **field** at fault, where the failure is about one field
rather than the file as a whole. Two files declaring the same `name` names
both files, first and second in file-name sorted order.

**A HUD fault refuses to start the client**, rather than starting with no
HUD. There is no error screen and no HUD-less fallback mode: a fault report
naming the file, the element and the field is a better diagnostic than a
game that launched and is quietly missing something.

Two read failures are told apart deliberately, by the operating system's own
answer and not by a probe:

- A **missing** `hud/` directory is a valid empty answer (below).
- Anything else that cannot be listed — a *regular file* named `hud`, a
  permissions failure — fails the load naming that path.

## A root declaring no HUD elements is valid — deliberately unlike `blocks/`

A content root with no `hud/` directory, or a `hud/` directory holding no
`*.toml` file, **loads successfully with zero HUD elements registered.**

This is the one place this contract parts company with block authoring,
where a root declaring nothing at all is an error. The reason is the
difference in what the absence means: a world with no blocks cannot be
rendered or played, so a block loader that produced nothing is broken. A
game with no HUD is merely bare, and **a mod that ships no HUD at all is
entirely ordinary** — most mods will add a block and no HUD. Treating that
as a failure would make every such mod declare an empty directory to say
"nothing", which is a declaration of absence rather than an absence.

With zero elements registered, a rendered frame is pixel-identical to the
same scene rendered with the HUD stage not run at all.

## Content cannot reach the debug overlay

The engine's debug overlay — position, column, frame rate and frame time,
bound to F3 (`user/gameplay.md`) — is **not content**, and this is a rule
rather than an omission.

- **There is no field by which a declaration can refer to the overlay**, no
  readable value that exposes it, and nothing a mod can set to hide, move,
  restyle or disable it.
- An element named `base:debug-overlay` is registered as an **ordinary
  element** and leaves the overlay's visibility exactly as it was. The name
  is not special to the engine; nothing about it is recognised.
- An element covering the whole screen with a fully opaque colour **does not
  obscure the overlay**: the overlay is composed after all content, so its
  pixels survive.

The reason is that the overlay is the instrument used to diagnose a
misbehaving mod, and a mod that could disable it could disable the tool
pointed at it. Note the narrowing this guarantee carries: it is about
**loaded, running** content. A declaration that fails to load never runs at
all, because the fault refuses the launch — and that refusal, naming file,
element and field, diagnoses a malformed declaration better than an overlay
showing a position and a frame rate could.

## What content categorically cannot do

Binding limits of the model today, not a list of things not yet
implemented — each is an explicit boundary:

- **No text.** No font, no glyph atlas, no text primitive. There is no way
  to declare a readout, a label or a number.
- **No expressions, conditionals, formatting or arithmetic.** A declaration
  states *what* and *where*. It never computes.
- **No layout engine.** No rows, columns, stacks, flow, wrapping, padding or
  parent-child nesting. Nine anchors and an offset.
- **No readable value other than `held-block`.** No health, hunger,
  coordinates, time of day or selected slot.
- **No input.** The HUD displays; it accepts no clicks, keys or focus. There
  is no hotbar, inventory, menu or pause screen.
- **No animation or transitions.** Nothing in a HUD declaration moves.
- **No colour tokens, theming or palette variants.** Colours are literals in
  the declaration, which is where a reskin edits them.
- **No hot reload.** Declarations load once, at startup, exactly as blocks
  do.

**A practical ceiling worth knowing:** the composition pass carries at most
**256 rectangles per frame**, and anything past that is dropped
**silently**. An element contributes one rectangle for its fill plus four
for its ring, so 256 rectangles is roughly 51 outlined elements. Nothing
`content/base/` ships comes near it; a large third-party HUD could.

## The three elements the base game ships

`content/base/hud/` ships exactly three files, each named for the element it
declares:

| File | `name` | `anchor` | `size` | `draw` | `color` | `source` | `outline` |
|------|--------|----------|--------|--------|---------|----------|-----------|
| `crosshair-horizontal.toml` | `base:crosshair-horizontal` | `center` | `[9, 1]` | `fill` | `#FFFFFFFF` | — | `#000000FF` |
| `crosshair-vertical.toml` | `base:crosshair-vertical` | `center` | `[1, 9]` | `fill` | `#FFFFFFFF` | — | `#000000FF` |
| `held-block.toml` | `base:held-block` | `bottom` | `[24, 24]` | `block-texture` | — | `held-block` | `#000000FF` |

None uses `offset`; all three take the default `[0, 0]`. All three are fully
opaque. At 1280×720 the two bars give a crosshair whose fill is exactly 17
white pixels (9 + 9 − 1, the two bars sharing the centre pixel) with 40
black ring pixels around it, and the swatch is a 24×24 block texture at
bottom-centre with a one-pixel ring.

**No block, colour or element name from this table appears in production
Rust anywhere in the workspace**, and that is asserted by a scan whose watch
list is derived from these files at test time rather than hand-copied — so
deleting a declaration cannot silently stop the scan watching its name. The
one hole that construction leaves is a *rename*: an old name stops being
watched the moment it leaves the directory, and nothing mechanical prompts
adding it to the retired list. **If you rename a shipped element, adding its
old name to that list is part of that commit** (`technical/architecture.md`
§"Mechanically enforced invariants").

## What MVP 2 changes

**MVP 2 replaces this TOML file loader with a Luau scripting host, through
the same `HudElementSource` port described in
`technical/architecture.md`.** Everything above — the fields, the anchors,
the two draw kinds, the published readable values, the all-or-nothing rule,
the failure vocabulary — is unchanged by that swap. What changes is only
where a declaration comes from: today a directory of `.toml` files read by
`TomlFileHudSource`; from MVP 2 onward a Luau declaration read by a
scripting-host-backed source. A HUD author who understands this contract
today does not need to relearn it when that arrives.

One thing does change with it. Once declarations are hot-reloadable, a HUD
fault can arrive **mid-session**, where refusing the launch is no longer
available — there is nothing left to refuse to. The overlay guarantee above
is narrowed to loaded, running content precisely because that cannot happen
today, so the commit that makes declarations reloadable reopens the
question, along with this contract's "no error screen" position.
