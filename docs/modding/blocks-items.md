# Authoring Blocks

How to declare a block so the engine registers it. A block declaration is a
**Luau chunk**, and this is the whole contract.

If you have never written anything for this engine before, start at
[README.md](README.md) — it walks you from an empty file to a block you can go
and stand on. This page is the reference you come back to.

## File layout

One block per file, under `<mod>/blocks/`:

```
content/<mod>/blocks/<block>.luau
```

The loader reads every `*.luau` file **directly** under `<root>/blocks/`, and
nothing else:

- A file with any other extension is passed over in silence. Keep notes,
  scratch files and working copies beside your declarations freely.
- A subdirectory is not descended into. `blocks/nested/hidden.luau` is not a
  declaration.
- An entry that **wears** a declaration's name without being a file — a
  directory called `nested.luau` — is refused rather than passed over. Naming a
  thing `.luau` is how you say *this is a declaration*, so the loader answers
  when something says it and is not one.

Declarations are read in **file-name sorted order**, and the sort is binding
rather than an implementation detail. Two things depend on it: which block ends
up in a new player's hand (the first solid block registered), and which of two
files declaring the same name is reported as the first and which as the second.

## A declaration is code that ran

A declaration file is a chunk that **returns a table**. That is a different thing
from a document that was parsed, and the difference is usable: a block may
compute what it declares.

```luau
local ORE = "amber"

return {
	name = "example:" .. ORE,
	texture = "example:" .. ORE,
	solid = true,
}
```

That registers `example:amber` even though the string `example:amber` appears
nowhere in the file.

What the chunk must hand back is a **table**. A chunk that returns a function, a
number, or nothing at all is refused naming the file.

Your chunk runs in the same sandbox every other script in this engine runs in —
see [script-limits.md](script-limits.md) for the budget and the memory cap, and
[script-surface.md](script-surface.md) for what a chunk can and cannot reach. A
declaration calls nothing the engine provides: it returns a table and that is
all. There is no `mycraft.*` binding to use here.

## The six fields

Three are required and three are optional.

| Field | Type | Required | Absent means | Bound |
|---|---|---|---|---|
| `name` | string, namespaced id | yes | — | 256 characters |
| `texture` | string, namespaced id | yes | — | 256 characters |
| `solid` | boolean | yes | — | — |
| `replaceable` | boolean | no | `false` | — |
| `breakable` | boolean | no | `true` | — |
| `breaks_into` | string, namespaced id | no | the cell is left empty | 256 characters |

```luau
return {
	name = "example:amber",        -- namespaced, required
	texture = "example:amber",     -- namespaced key, required, never a path
	solid = true,                  -- required boolean

	replaceable = false,           -- optional, absent means false
	breakable = true,              -- optional, absent means true
	breaks_into = "example:ash",   -- optional, absent means the cell empties
}
```

- **`name`** — the block's namespaced id. This is what the block is called
  everywhere: in other declarations, in save data, and in refusals.
- **`texture`** — a namespaced **key**, never a file path. What pixels a key
  resolves to is the renderer's concern, not a declaration's. **Read
  "Texture keys today", below, before you give a block a texture key that is not
  its own name.**
- **`solid`** — does this block stop a player. See "Solidity is data", below.
- **`replaceable`** — may a placement overwrite this block. Absent means **not
  replaceable**, which is the conservative reading: forgetting the key loses you
  a placement rather than a block.
- **`breakable`** — can this block be broken at all. Absent means **breakable**,
  the opposite default from `replaceable` and deliberately so — a sandbox whose
  blocks were indestructible until each said otherwise would be the wrong burden
  to put on every content file.
- **`breaks_into`** — the namespaced id of the block a break leaves behind.
  Absent means the cell is left **empty**, which is a different claim from the
  block being indestructible, which is why they are different fields.

**The three optional fields are independent of one another.** A block may declare
`breakable = false` *and* a `breaks_into`; the residue is simply never reached,
and it is still there the day you make the block breakable again by editing one
line.

**A residue is resolved when a break happens, not when the declaration is read.**
`breaks_into = "example:ash"` registers whether or not anything declares
`example:ash` — only the id's shape is checked here. That is what lets two mods
name each other's blocks without either having to load first.

**Optional means you may leave it out, never that any value will do.**
`replaceable = 1` is refused naming `replaceable`; it does not fall back to the
default. Falling back is the worst available outcome, because the block then
behaves exactly as it would have if you had written nothing at all — so there is
no symptom to notice and nothing to search for.

**A field the loader does not recognise is refused, not ignored.** A misspelled
`replacable` is a word anybody types once, and a loader that read the six keys it
knows and never asked what else was there could not tell a typo from an absence.
The refusal names the field you wrote **and the six you may write**, because a
name is only recognisable as a typo once you can see what it was nearly.

## The namespaced id rule

Every id — a `name`, a `texture` key, a `breaks_into` — follows one rule:
**exactly one `:`, with non-empty text on both sides.**

`example:amber:top` is refused outright, naming the field and the rule. It is
*not* quietly read as a block named `amber:top` inside namespace `example`:
splitting on the first colon would turn a typo into a plausible-looking id that
resolves to nothing, with no diagnostic pointing at what went wrong.

No character-set rule is enforced beyond that today, so uppercase letters and
unusual punctuation inside a namespace or a path are currently accepted. That is
deliberate rather than an oversight — a mod-id character set is a decision for
when mod ids become a real identity rather than a label, and guessing at one now
would be a compatibility promise made blind. The direction is strict-now,
permissive-later: a stricter rule can be relaxed without invalidating content
already written against it, and the reverse breaks everything.

## Texture keys today

**A block's texture is selected by its `name`, not by its `texture` key.** The
two fields are genuinely independent at the loader — a declaration may state
different values and will register — but the renderer looks a block up by its
name when it decides which image to draw.

What follows for you, plainly:

- **A declaration whose `texture` differs from its `name` will load and then not
  draw.** The block registers, the world accepts it, and the face simply does not
  appear; its held-block indicator draws nothing either. There is no error on
  your terminal, because a batch that could not be drawn is logged and dropped
  rather than failing the run.
- **So declare the two equal for now.** Every block the base game ships does.
- Independent texture keys are coming, along with per-face keys — a distinct
  top, side and bottom for a block like grass. Until they land, `texture` is a
  field you state and the engine reads through your block's name.

This is written down rather than left silent because nothing mechanical will tell
you: the loader is perfectly happy, and the only symptom is a block you cannot
see.

## All-or-nothing loading

Loading a content root registers **every** declaration it holds or **none** of
them. A failure partway through — a chunk that will not compile, a duplicate
name, a bad field — leaves the registry exactly as it was, and never a partial
set of blocks.

A root that exists but declares **no blocks at all** is a refusal naming that
root, not an empty registry that loaded successfully. A root with no `blocks/`
directory is refused naming `<root>/blocks`.

## Reading a refusal

Every refusal names as much as it can: the **file**, the **block** as it named
itself, the **field** at fault, and the **cause**. Where there is nothing to
name — a chunk that never returned has no block to attribute — that part is
simply absent rather than guessed at.

A `blocks/amber.luau` declaring `slid = true` where it meant `solid` is refused
like this:

```
mycraft: the shipped content could not be read: content/base/blocks/amber.luau, block `example:amber`, field `slid`: `slid` is not a field a declaration may state; a declaration may state `name`, `texture`, `solid`, `replaceable`, `breakable`, `breaks_into`
```

The parts read outermost first, separated by `: ` — the stage that failed, then
the file, then the block, then the field, then the cause.

Three refusals whose shape is worth knowing before you meet them:

- **A chunk that will not compile** names the file and **the line the compiler
  named**, and no block — there is no declaration yet to attribute it to.
- **A chunk that raises an error of its own** names the file and the message you
  raised. `error("...")` in a declaration is a legitimate way to say a block
  cannot be declared, and what you said is what explains the refusal.
- **A name declared twice** names both files, in file-name order, and no field.

The file a refusal names is the **path you can open**, not the chunk name the
scripting host was given. Those are deliberately different things.

## The four bounds

Every quantity a content root supplies has a limit, and each accepts its own
limit and refuses the value above it.

| Bound | Limit | Refusal names |
|---|---|---|
| Declarations per content root | 4 096 | the `blocks/` directory, the count and the bound |
| Bytes per declaration file | 256 KiB | the file, its size and the bound |
| Characters per declared value | 256 | the field, the length and the bound |
| Fields per declaration | 64 | the file and the bound |

Two things worth knowing about how they are checked:

- **The count is checked before any file is opened**, so a root of 4 097 files is
  refused on the count even when one of them is also broken.
- **A file's size is taken before it is read**, so an oversized file is refused
  on its size and not on whatever its text turned out to say.

Characters, not bytes: a 256-character id is accepted however many bytes it
takes to spell, so non-ASCII ids refuse at the length this page states rather
than at some shorter one.

A declared value one character too long is refused like this — here a `texture`
key of 257 characters:

```
mycraft: the shipped content could not be read: content/base/blocks/amber.luau, block `example:amber`, field `texture`: `texture` holds 257 characters, and a declared value may hold at most 256
```

Both numbers are there on purpose: the one you wrote and the one you may write.
`name` and `breaks_into` are refused the same way, by the same bound.

Separately, what a declaration **prints** is bounded too. `print` works and is
useful while you are getting a declaration right; the host keeps the earliest
output up to its limit and then stops recording, and says how many lines it
stopped keeping. It does not refuse your declaration for printing too much.

## Replaceability is not derived from solidity

**Placement reads `replaceable` and never consults `solid`.** Solidity is a
physics fact — does this block stop a player — and replaceability is a placement
rule — may a placement overwrite this block. A mod can declare a non-solid block
you cannot build through, and a solid block you can; the engine derives neither
from the other, because doing so would put a game rule in code that content is
supposed to own.

`base:water` is both non-solid *and* replaceable, but that pairing is not assumed
anywhere in the engine — each is its own declared fact.

**`replaceable` governs real blocks only.** An empty cell accepts a placement
because it is empty, not because content said so, and no declaration can make an
empty cell refuse one. Nothing is not content, so there is nothing there for a
content rule to be about.

## There is no empty block

**A cell holds a block or holds nothing, and nothing names nothing.** The base
game declares no block meaning "empty", the engine knows no such block, and there
is no name you have to avoid, reserve or texture in order to describe empty
space.

- You declare only blocks that exist. There is nothing to write for the space
  between them.
- A break with no `breaks_into` leaves the cell empty. It does not leave behind
  some other block standing for emptiness.
- A name that *sounds* like empty space is an ordinary name. Declare
  `example:void` and it gets exactly the treatment every other block gets,
  including whatever solidity it declares.

## Solidity is data, not inference

Solidity is a **declared property**. Nothing about it is inferred from a block's
name or its runtime id, and no name and no id is special-cased anywhere in the
engine. An engine that treated some particular name as implicitly non-solid would
be writing a game rule into Rust that the base game's own mod-equivalent status
forbids: `base:stone`'s solidity is a fact `stone.luau` states, and your block is
solid or not for exactly the same reason and by exactly the same mechanism.

Declare a block named `example:air` as `solid = true` and cells holding it are
reported solid. Declare one named `example:stone` as `solid = false` and cells
holding it are reported non-solid.

## A complete example

A mod called `example` shipping two blocks: an ore that breaks into ash, and the
ash itself.

`content/example/blocks/amber.luau`:

```luau
-- Amber ore. Breaks into ash, which is declared beside it.
return {
	name = "example:amber",
	texture = "example:amber",
	solid = true,
	breaks_into = "example:ash",
}
```

`content/example/blocks/ash.luau`:

```luau
-- What amber leaves behind. Soft enough to build straight through.
return {
	name = "example:ash",
	texture = "example:ash",
	solid = true,
	replaceable = true,
}
```

Both register, and `amber.luau` sorts before `ash.luau`, so amber registers
first. If these were the only two blocks in the world, amber is what you would
find in your hand — the first solid block in registration order.

Note that `example:ash` is named by `amber.luau` before `ash.luau` has been read,
and that this is fine: a residue is resolved when a break happens.

## The base game's four blocks

`content/base/` ships exactly four declarations:

| File | `name` | `texture` | `solid` | `replaceable` | `breakable` | `breaks_into` |
|------|--------|-----------|---------|---------------|-------------|---------------|
| `dirt.luau` | `base:dirt` | `base:dirt` | `true` | *(absent)* | *(absent)* | *(absent)* |
| `grass.luau` | `base:grass` | `base:grass` | `true` | *(absent)* | *(absent)* | *(absent)* |
| `stone.luau` | `base:stone` | `base:stone` | `true` | *(absent)* | *(absent)* | *(absent)* |
| `water.luau` | `base:water` | `base:water` | `false` | `true` | *(absent)* | *(absent)* |

They are read in that order, which is why `base:dirt` — the first solid block —
is what a new player holds.

`base:water` is the only base block declaring `replaceable = true`, which is what
makes water placeable at all. No base block declares itself unbreakable or names
a residue: breaking any of the four leaves the cell empty. And every one declares
`texture` equal to `name`, for the reason under "Texture keys today".

Every block has exactly one texture key today — there is no per-face texture yet.

## What is not here yet

Hot reload of declarations: a declaration is read once, at load. Per-cell state,
callbacks and components. Worldgen in script. Reading a second content root. And
`extends`, in every form — a declaration states its own six fields and inherits
nothing.
