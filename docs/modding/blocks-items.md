# Authoring Blocks

How to declare a block so the engine registers it. This is the contract as
it stands today; see the note at the bottom about what changes in MVP 2.

## File layout

One block per file, under `<mod>/blocks/`:

```
content/<mod>/blocks/<block>.toml
```

The loader reads every `*.toml` file directly under `<root>/blocks/`
**non-recursively**, in file-name sorted order. The sort is binding, not
an implementation detail: when two files declare the same block name, the
resulting error must name a first file and a second file, and which is
which is only well-defined if the directory is always read in the same
order.

## Fields

A block declaration is a top-level TOML table with three required fields
and three optional ones:

```toml
name = "base:stone"        # namespaced, required
texture = "base:stone"     # namespaced texture key, required — never a path
solid = true               # required boolean

replaceable = false        # optional, absent means false
breakable = true           # optional, absent means true
breaks_into = "base:dirt"  # optional, absent means breaking empties the cell
```

- **`name`** — the block's namespaced id, e.g. `base:stone`. This is what
  the block is called everywhere: in other definitions, in save data, and
  in error messages.
- **`texture`** — a namespaced **key**, never a file path. What pixels a
  key resolves to is the renderer's concern, not the block definition's —
  a definition that named a file would be answering a question that
  belongs somewhere else.
- **`solid`** — a plain boolean. See "Solidity is data", below.
- **`replaceable`** — may a placement overwrite this block? Absent means
  **not replaceable**, the conservative reading: a block that says nothing
  about it cannot be built through, so a content author who forgets the
  key loses a placement rather than silently losing a block.
- **`breakable`** — can this block be broken at all? Absent means
  **breakable**, the opposite default from `replaceable` and deliberately
  so: a sandbox whose blocks were indestructible until each one said
  otherwise would be the wrong default to make every content file carry.
  `breakable = false` marks a block indestructible.
- **`breaks_into`** — the namespaced id of the block a break leaves behind.
  Absent means the cell is left **empty** rather than that the block is
  indestructible — those are two different claims, so they are two
  different fields. There is no block to name for the empty case: a cell
  holds a block or holds nothing (see "There is no empty block", below), so
  a block that simply disappears when broken — the common case — declares
  nothing here, and the engine picks no name on its behalf. The named block
  is resolved when the break happens, not when the definition is parsed, so
  `breaks_into` may legitimately name a block that a later-loaded content
  root registers — only the id's syntax is checked at load time.

**Unknown fields are rejected.** A declaration carrying a field the loader
does not recognise (a typo, most often) fails to load rather than being
silently ignored. A silently-ignored typo is a debugging trap for whoever
wrote the file, and rejecting it outright is what avoids that trap instead
of hiding it.

## Replaceability is not derived from solidity

**Placement reads `replaceable` and never consults `solid`.** Solidity is a
physics fact — does this block stop a player — and replaceability is a
placement rule — may a placement overwrite this block. A mod can declare a
non-solid block you cannot build through, and a solid block you can; the
engine derives neither from the other, because doing so would put a game
rule in code that content is supposed to own. `base:water` is both
non-solid *and* replaceable, but that pairing is not assumed anywhere in
the engine — each is its own declared fact, and a mod is free to ship a
non-solid, non-replaceable block (an obstacle you can see through but not
build over) or a solid, replaceable one.

**`replaceable` governs real blocks only.** An empty cell accepts a
placement because it is empty, not because content said so, and no
declaration can make an empty cell refuse one. That is not a gap: nothing
is not content, so there is nothing there for a content rule to be about.

The engine does not check that a placed block itself is solid, or anything
else about it beyond "is this name registered" — naming what to place is
an intent, and the only door that needs locking is the one `replaceable`
already locks on the *target* cell.

## The namespaced id rule

Every id — a block's `name`, and a `texture` key — follows one rule:
**exactly one `:`, with non-empty text on both sides.**

`base:stone:top` is rejected outright, with an error naming the extra
separator — it is *not* silently parsed as a block named `stone:top`
inside namespace `base`. Splitting on the first colon instead of enforcing
exactly one would turn a typo into a plausible-looking id that resolves to
nothing, with no diagnostic pointing at what went wrong.

No character-set rule is enforced beyond that today (so, for instance,
uppercase letters or unusual punctuation inside a namespace or path
segment are currently accepted). That is deliberate, not an oversight: a
mod-id character set is a decision the Luau scripting layer will make when
mod ids become a real identity rather than a label, and guessing at a
charset now would be a compatibility promise made blind. The direction is
strict-now, permissive-later — a stricter rule can always be relaxed
without invalidating content already written against it; the reverse
breaks everything already written.

## All-or-nothing loading

Loading a content root either registers **every** definition it declares,
or **none** of them. A failure partway through — an unparseable file, a
duplicate name, a bad field — leaves the registry exactly as it was before
the load was attempted; it never leaves a partial set of blocks
registered.

Every failure names its **origin**: the file it came from, the block name
it was declared under (when the file's contents could be read far enough
to find one), and the specific field at fault, where the failure is about
one field rather than the file as a whole.

Two boundary cases worth knowing:

- A content root that **exists** but declares **no blocks at all** is an
  error naming that root — not an empty, successfully-loaded registry. A
  loader that was asked for definitions and produced none is treated as
  broken, not as a legitimate empty answer.
- A root with **no `blocks/` subdirectory** reports the same
  can't-be-read error as a root that doesn't exist on disk at all — both
  are a failure to list the `blocks/` directory, and are reported
  identically.

## There is no empty block

**A cell holds a block or holds nothing, and nothing names nothing.** The
base game declares no block meaning "empty", the engine knows no such
block, and there is no name a content author has to avoid, reserve, or
texture in order to describe empty space. Empty space is the absence of a
declaration, not a declaration of absence.

Three consequences for a content author:

- You declare only blocks that exist. There is nothing to write for the
  space between them.
- A break with no `breaks_into` leaves the cell empty. It does not leave
  behind some other block that stands for emptiness.
- A name that *sounds* like empty space is an ordinary name. If your mod
  declares a block called `base:air`, `mod:void` or `mod:nothing`, it gets
  exactly the treatment every other block gets, including whatever solidity
  it declares — see below.

## Solidity is data, not inference

Solidity is a **registered property** of a block, declared explicitly in
its file. Nothing about it is inferred from a block's name or its runtime
id — no name and no runtime id is special-cased anywhere in the engine.
An engine that treated any particular name or id as implicitly non-solid
would be writing a game rule into Rust that the base game's own
mod-equivalent status forbids: `base:stone`'s solidity is a fact
`stone.toml` states, and a mod's block is solid or not for exactly the same
reason and by exactly the same mechanism.

The test of that claim is a name the engine could plausibly have been
tempted to recognise. Declare a block named `base:air` as `solid = true`
and cells holding it are reported solid; declare one named `base:stone` as
`solid = false` and cells holding it are reported non-solid. Both hold, and
the engine's indifference to the name is asserted rather than assumed.

## The base game's four blocks

`content/base/` ships exactly four block definitions:

| File | `name` | `texture` | `solid` | `replaceable` | `breakable` | `breaks_into` |
|------|--------|-----------|---------|---------------|-------------|---------------|
| `stone.toml` | `base:stone` | `base:stone` | `true` | *(absent)* | *(absent)* | *(absent)* |
| `dirt.toml` | `base:dirt` | `base:dirt` | `true` | *(absent)* | *(absent)* | *(absent)* |
| `grass.toml` | `base:grass` | `base:grass` | `true` | *(absent)* | *(absent)* | *(absent)* |
| `water.toml` | `base:water` | `base:water` | `false` | `true` | *(absent)* | *(absent)* |

`base:water` is the only base block declaring `replaceable = true` — it is
what makes water placeable at all, since placement never falls back to
checking solidity. No base block declares itself unbreakable or names a
residue: breaking any of the four leaves the cell empty.

Every block currently has exactly one texture key — there is no per-face
texture (a distinct top/side/bottom key, needed for a block like grass
with different faces) yet. Per-face texture keys are expected once real
art arrives, not before.

## What MVP 2 changes

**MVP 2 replaces this TOML file loader with a Luau scripting host,
through the same `DefinitionSource` port described in
`technical/architecture.md`.** The registry contract itself — what a block
definition is, how a name resolves to one, that solidity is a declared
property — does not change. What changes is only where the definitions
come from: today, a directory of `.toml` files read by
`TomlFileDefinitionSource`; from MVP 2 onward, Luau declarations read by a
scripting-host-backed source. A block author who understands this contract
today does not need to relearn it when that arrives.
