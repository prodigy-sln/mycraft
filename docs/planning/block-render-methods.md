# Blocks declare a render method (PRO-952)

> **Planning document** — not as-built documentation. Elaborates
> [PRO-952 "Blocks declare an engine render method; liquids get inset and wave"](https://linear.app/prodigy-solutions/issue/PRO-952/).
> Written 2026-08-18 against the tree at `5cad154`. Statements about the tree
> were verified by reading it and cite file and line; everything else is
> derivation and is labelled as such. Depends on PRO-904 and interacts with
> PRO-947 — the sequencing section is the part most likely to go stale.

## 1. What was asked for, and what it actually decides

The request was about water: its upper surface must not sit exactly as high as
the tops of the blocks surrounding it, and it should be slightly wavy. Stated
in full, with the generalisation that came with it:

> This is a property of the water block. Other liquids might render
> differently. The blocks define the render method. The engine controls the
> rendering and provides different methods.

That last pair of sentences is the decision, and it is much larger than water.
It settles a question this project had not yet had to answer: **when a block
needs to be drawn differently, who decides how?**

The answer is a split, and it is the same split invariant 1 makes everywhere
else:

- **The engine owns the set of render methods.** A method is real code —
  geometry generation, a shader, a place in the draw order. Content cannot
  author one, cannot supply a shader, and cannot reach the GPU.
- **The block owns the choice of method, and its parameters.** A declaration
  names a method from that set and supplies its numbers.

The consequence that matters for review: **a hardcoded `if name == "water"`
anywhere in the mesher or the renderer is a Blocker, not a style note.** It is
the same violation as a hardcoded block definition, one layer along. The
mesher already states this property about itself and holds it today —
`visible_face` carries the comment "no block name and no runtime id is looked
at anywhere in this file, which is what makes a block a mod ships behave
exactly as one the base game ships does"
(`crates/mc-world/src/mesh/sweep.rs:262`). Liquid rendering is the first
feature with a real incentive to break it.

## 2. What the tree does today (verified 2026-08-18)

**There is exactly one render method, and it is not named anywhere.** It is
implicit in the mesher: full cubes, opaque, faces emitted where a solid voxel
abuts a non-solid one. `visible_face` returns `Ok(None)` unless
`solidity(resolved, key)?` is true (`mesh/sweep.rs:275`), so a non-solid block
emits **no faces at all**.

**Water is therefore completely invisible right now.** `base:water` is the one
shipped block declared `solid = false` (`content/base/blocks/water.luau:13`),
and the replay world fills a sea with it (`crates/mc-sim/src/replay/world.rs`).
The sea exists, has a texture key, blocks nothing, and draws nothing. This was
noticed from the other end during PRO-918 acceptance — "is it expected that
non-solid blocks have absolutely no texture?" — and the answer is yes, by
construction, and it is not water's fault.

That is worth stating plainly because it reframes the work: **this is not
"make water prettier". Step one is making water visible at all**, and the
reason it is invisible is the same reason PRO-904 exists — one `solid` bit is
answering four unrelated questions.

**The declaration already has a field set with defaults and refusals.** Fields
are parsed by name in `crates/mc-world/src/content/luau_declaration.rs`
(`REPLACEABLE_FIELD`, `BREAKS_INTO_FIELD`, …), with a documented
misspelling-detection path. A `render` field is an ordinary addition to a
mechanism that already exists; it does not need new machinery.

**The persisted definition already splits behaviour from appearance.**
`DeclaredBehaviour` folds `is_solid`, `replaceable`, `breakable`, `breaks_into`;
`DeclaredAppearance` folds `texture` (`persistence/format.rs:278–304`). Both
carry an `input_version`. The doc comment states the intent — "a block whose
texture changed is the same block to stand on" — and a render method is
plainly appearance, so it belongs in `DeclaredAppearance`.

## 3. The design, as far as it can be taken before PRO-904

### 3.1 The declaration

A block names a method and supplies its parameters. Sketch, not a commitment:

```lua
return {
    name = "base:water",
    texture = "base:water",
    render = "liquid",
    surface_inset = 0.125,
    wave = { amplitude = 0.03, period = 2.5 },
}
```

Open, and deliberately not decided here: whether the parameters are flat
fields as above or a nested table under `render`. A nested table scopes them to
the method that consumes them and makes an unknown parameter attributable, at
the cost of a shape the rest of the declaration does not use. **The spec
decides this; it is not settled by the request.**

What *is* settled: **the refusal must name the file, the block, the field and
the known method set.** That is the shape every other declaration refusal
already takes and the one PRO-918's acceptance confirmed is readable by a human
mid-edit. An unknown method name is a refusal, not a silent fallback to the
default method — a typo that silently renders as a solid cube is exactly the
failure mode this project keeps paying for.

### 3.2 Inset and wave are method parameters, not water constants

This is the part of the request that is easiest to implement wrongly and hard
to detect afterwards. A `LIQUID_SURFACE_INSET` constant in Rust would produce
the requested visual result and would be a violation: it makes lava's surface
water's surface, and it puts a content decision in the engine. The test that
distinguishes them is cheap and should be written — **two liquids declaring
different insets render differently**, which no shipped content exercises
today and which is precisely why it needs a fixture.

### 3.3 Which methods exist at first

Two, on current evidence:

- **`solid` (or whatever the default is called)** — today's behaviour exactly.
  Every existing block gets it, by default, and its output must be
  byte-identical to what ships now or the whole golden set moves for no reason.
- **`liquid`** — inset top surface, animated vertical displacement,
  transparency, and the culling rules that come with a non-opaque block.

A third method is not designed here. Naming the set now is useful; growing it
speculatively is not.

### 3.4 The hard parts, named rather than solved

Derivation, not measurement — these come from reading the mesher, not from
having built it:

- **A non-opaque block breaks the binary occlusion rule.** `solid_beyond`
  culls a face when the neighbour is solid (`mesh/sweep.rs:283`). Water against
  water must not draw an internal face; water against air must; water against
  stone must draw the stone face and not the water one. That is three answers
  from one predicate, and it is PRO-904's split arriving whether or not it is
  invited.
- **Transparency needs a draw order.** The renderer has one opaque pass. Sorted
  transparency, or an unsorted pass that accepts artefacts at MVP-2 quality, is
  a decision the spec must make explicitly rather than discover.
- **The wave has to come from somewhere per frame.** Displacing vertices in the
  vertex shader from a time uniform keeps the mesh static and the animation
  free; re-meshing per frame is not viable. This bears on whether the wave is
  a *mesh* property or a *material* one, and the answer affects what the mesher
  needs to know at all.
- **The surface inset applies to the top face of the topmost liquid cell
  only.** A liquid cell with liquid above it has a full-height top. Otherwise
  the sea gets a ledge every metre down.

## 4. Cost, and why the sequencing matters

**A mesher-contract change renames the entire golden set.** The scene revision
is part of every capture id, so changing what the mesher emits invalidates
every golden frame — the policy is recorded in `docs/technical/rendering.md`
("Re-shooting a golden set"), together with the unfiltered command that
corrupts the set if used.

Three separate pieces of MVP 2 work each trigger that re-shoot:

| Issue | Change | Re-shoots goldens |
|-------|--------|-------------------|
| PRO-947 | per-face texture keys | yes |
| PRO-904 | splitting `solid` into its separate consumers | yes |
| PRO-952 | a second render method | yes |

**Decided: the drawn/occludes split rides along with PRO-947's per-face keys.**
Done separately they pay the re-shoot twice, and liquids would pay it a third
time. The re-shoot is not merely slow — it is the operation with a known
corrupting failure mode, and each repetition is another chance to mint goldens
from a renderer that is already wrong. Minimising the number of times it
happens is a correctness argument, not a throughput one.

**PRO-952 lands after PRO-904.** A render method cannot be declared coherently
while one `solid` bool still means drawn, occludes, collides and
stood-upon at once: choosing how a block draws presupposes that whether it
draws is its own property. Attempting the methods first means encoding the
conflation into the method set and unpicking it later.

**Adding `render` to `DeclaredAppearance` makes every existing save report a
changed appearance.** That is the designed behaviour of the fingerprint, and
the prompt it produces is the one the format's doc comment argues for. Worth
knowing before it looks like a bug, and worth landing in the same increment as
any other appearance-field change rather than in two consecutive ones.

## 5. Open questions for the spec

Questions, deliberately not answered here:

1. Flat parameter fields or a nested table under `render`? (§3.1)
2. Is the wave a mesh property or a material property — i.e. does the mesher
   need to know about it at all? (§3.4)
3. Sorted transparency, or an unsorted pass with accepted artefacts at MVP 2
   quality?
4. Does the default method get a spellable name in content, or is `render`
   omitted for ordinary blocks? An omittable field means no existing
   declaration changes; a required one means every block states its own
   rendering, which is more honest and touches four files today and every
   third-party block forever.
5. What does a golden frame of a *wave* assert? An animated surface sampled at
   a fixed tick is deterministic, but the property worth guarding is "it moves
   and stays inside its bounds", which a single frame cannot see.
