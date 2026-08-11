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

A block declaration is a top-level TOML table with exactly three fields,
all required:

```toml
name = "base:stone"     # namespaced, required
texture = "base:stone"  # namespaced texture key, required — never a path
solid = true             # required boolean
```

- **`name`** — the block's namespaced id, e.g. `base:stone`. This is what
  the block is called everywhere: in other definitions, in save data, and
  in error messages.
- **`texture`** — a namespaced **key**, never a file path. What pixels a
  key resolves to is the renderer's concern, not the block definition's —
  a definition that named a file would be answering a question that
  belongs somewhere else.
- **`solid`** — a plain boolean. See "Solidity is data", below.

**Unknown fields are rejected.** A declaration carrying a field the loader
does not recognise (a typo, most often) fails to load rather than being
silently ignored. A silently-ignored typo is a debugging trap for whoever
wrote the file, and rejecting it outright is what avoids that trap instead
of hiding it.

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

## Solidity is data, not inference

Solidity is a **registered property** of a block, declared explicitly in
its file. Nothing about it is inferred from a block's name or its runtime
id — no name and no runtime id is special-cased anywhere in the engine,
**including `base:air`**. An engine that treated any particular name or id
as implicitly non-solid would be writing a game rule into Rust that the
base game's own mod-equivalent status forbids: `base:air`'s non-solidity
is a fact `air.toml` states, exactly the way `base:stone`'s solidity is a
fact `stone.toml` states.

## The base game's five blocks

`content/base/` ships exactly five block definitions:

| File | `name` | `texture` | `solid` |
|------|--------|-----------|---------|
| `air.toml` | `base:air` | `base:air` | `false` |
| `stone.toml` | `base:stone` | `base:stone` | `true` |
| `dirt.toml` | `base:dirt` | `base:dirt` | `true` |
| `grass.toml` | `base:grass` | `base:grass` | `true` |
| `water.toml` | `base:water` | `base:water` | `false` |

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
