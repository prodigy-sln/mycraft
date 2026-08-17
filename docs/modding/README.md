# Making a mod

Start here. This page takes you from a clean checkout to a block of your own,
visible in the game, in one file and one command. Everything else in
`docs/modding/` is a reference for one kind of declaration; this is the way in.

## What a mod is here

A mod is a directory of declarations. Blocks are written in **Luau**, this
engine's own scripting language; the HUD and the tooling formats are data files.
The engine reads all of them through a loading contract and never learns where a
declaration came from — which is what lets the base game be a mod like any other.
**`content/base/` is the vanilla game**, it holds no privileged declaration, and
every field it uses is a field your own content can use.

There is one thing to know before you write anything: **the game reads exactly
one content directory today, `content/base/`, resolved relative to the directory
you start it in.** The loading contract is already written in terms of
`content/<mod>/`, and the references below describe it that way, but nothing
reads a second root yet. So a declaration you add goes in `content/base/`
alongside the shipped ones. You can and should still use your own namespace —
`example:amber` rather than `base:amber` — because the namespace is what the id
means, not where the file sits.

### If you only play, nothing here changed

Blocks used to be data files and are now Luau. **A player sees no difference, and
that is the intended outcome rather than something left undone.** The same four
blocks, the same terrain, the same block in your hand, and every save you already
have still loads and still holds what it held. Nothing you can see, build, break
or reach is different.

What changed is who can add to it, and in what language.

## What you can define today

Four kinds of file, and they do not all reach the same place:

| You write | Under | It reaches | Reference |
|---|---|---|---|
| **A block** — a thing that occupies a cell, with solidity, replaceability and what it drops | `content/base/blocks/*.luau` | the running game | `blocks-items.md` |
| **A HUD element** — a rectangle drawn on top of the world, filled or showing a block's texture | `content/base/hud/*.toml` | the running game | `hud.md` |
| **A voxel model** — a door, a torch, a prop, described as text | `content/base/models/*.mcvox` | the `voxforge` tool only | `voxel-models.md` |
| **A material** — a named colour a voxel model's palette refers to | `content/base/materials/*.toml` | the `voxforge` tool only | `voxel-models.md` |

**Nothing in the engine loads, meshes or draws a `.mcvox` file.** Models and
materials are authoring and preview tooling — you write one, run `voxforge
preview`, and look at it. They reach no player. Blocks and HUD elements are the
two kinds that a player can actually see, which is why the walkthrough below is
about a block.

Items, tools, recipes, NPCs, quests and dialogue have no format at all yet.
There is nothing to write for them and nothing hidden that would accept it.

## Your first block, start to finish

### 1. Write the file

Create `content/base/blocks/amber.luau`:

```luau
return {
	name = "example:amber",
	texture = "example:amber",
	solid = true,
}
```

Three fields, all required, and that is a complete block. `name` is what the
block is called everywhere — in save data, in other declarations, in refusals.
`texture` is a **key**, never a file path; what pixels it resolves to is the
renderer's business. `solid` is whether it stops a player walking into it.

Both ids follow one rule: **exactly one `:`, with non-empty text either side.**
`example:amber` is fine, `example:amber:top` is refused by name, and `amber` is
refused for having no namespace at all.

**Declare `texture` equal to `name`, as this example does.** The two are separate
fields and the loader will accept different values, but the renderer picks a
block's image by its **name** today — so a block whose `texture` differs loads
and then does not draw, with nothing on your terminal to say why. See "Texture
keys today" in `blocks-items.md`.

Your file is a Luau chunk that **returns a table**, not a document that gets
parsed. That mostly does not matter on your first block, and then one day it
does: you can compute what you declare, loop to build a value, pull a shared
string out into a local. A declaration is code that ran.

### 2. Run it

From the root of the checkout:

```
cargo run -p mc-client
```

The game looks for `content/base/` relative to where it was started, so start it
from the checkout root or it will exit saying it found no content there.

### 3. Look at the bottom of the screen

The small square at the bottom centre — the swatch showing the block a placement
would use — **is now your block**, in a colour it did not have before.
Right-click anywhere in reach and you place `example:amber`. Left-click breaks.
Every block you place is yours, and they persist: close the window normally and
they are still there next launch.

That is a mod. One file, no build step for the content itself, no registration
call anywhere.

### 4. Leave the game running and edit the file again

Do not close the window. Open `content/base/blocks/amber.luau`, change
`solid = true` to `solid = false`, and save.

About a sixth of a second later you can walk through your own block. The world you
were standing in is untouched, you are where you left yourself, and nothing was
restarted. Change it back and save again, and it stops you again.

That is the shortest complete loop this project has: write, save, see. What is
visible, what is accepted and invisible, and every refusal you can meet on the way
are on `hot-reload.md`.

## Why that worked, and how to make it work for your block

The swatch changed because of a rule worth knowing before it surprises you:

> **The block you hold is the first *solid* block in registration order, and
> registration order is the `blocks/` directory sorted by file name.**

`amber.luau` sorts before `dirt.luau`, and it declares `solid = true`, so it
displaces `base:dirt` as the held block. Name the file `zinc.luau` instead and
everything still loads — the block is registered, the world knows it, a save can
hold it — but you have no way to reach it, because there is no inventory, no
hotbar and no way to choose what you are holding. The first-solid-block rule is a
placeholder standing in for an inventory that does not exist yet, not a designed
feature.

So while there is no way to pick a block, **a file name that sorts before
`dirt.luau` is how you get your hands on the one you just wrote.**

Two consequences of the same rule:

- A block declaring `solid = false` is skipped when choosing what you hold, so a
  non-solid block cannot be the held one however its file sorts. If your content
  registers no solid block at all, the game refuses to start rather than opening
  a window you can place nothing in.
- The colour is generated from the **texture key** and from nothing else — the
  same key gives the same colour on every machine, on every run, forever. You
  cannot predict which colour from the name, and you should not try to: block art
  does not exist yet, and the game says so on startup. Stone and dirt draw teal,
  grass draws tan. Yours will be some other deterministic colour.

## When you get it wrong

**Loading is all-or-nothing.** A content directory either registers every
declaration in it or none of them — there is no partial load, and one bad file
does not cost you just that file. A failure leaves the registry exactly as it
was.

**A content failure stops the launch**, and the game says why on standard error,
in text beginning `mycraft: `. There is no error screen, no safe mode, and no
starting-anyway-without-it: a window that opened with your block silently missing
would tell you nothing.

**Where it stops depends on what is wrong.** A content root that is not there at
all is noticed before anything is opened, so the process exits without a window.
A declaration the loader *refuses* is collected at the first frame, so you may
see a window appear and close again. Either way the refusal is on your terminal
when the process ends.

**The refusal names the file, the declaration and the field.** Put `slid = true`
in `blocks/amber.luau` — a typo for `solid` — and this is what you read:

```
mycraft: the shipped content could not be read: content/base/blocks/amber.luau, block `example:amber`, field `slid`: `slid` is not a field a declaration may state; a declaration may state `name`, `texture`, `solid`, `replaceable`, `breakable`, `breaks_into`
```

Read it outermost first: what failed, then which file, then which block, then the
field, then the reason — which lists every spelling the loader does accept. That
is one edit, in one named file, with the accepted spellings in front of you. You
do not need to change one file at a time to find out which one is at fault, and
you do not need to read any Rust.

**Where a part is missing, nothing is invented in its place.** Two files claiming
one name names both files and no field, because no single field is wrong. A file
that is not Luau at all names the file and the line the compiler named and no
block, because there was never a declaration to read a name out of. A `blocks/`
directory that declares nothing names the directory.

Five ways to get it wrong, all of them refusals rather than surprises later:

- **A field the loader does not recognise.** A typo like `slid = true` is refused,
  not ignored. A silently-ignored typo is a debugging trap; being refused is what
  keeps you out of it.
- **A malformed id.** No colon, an empty side, or more than one colon. Note the
  last one especially: `example:amber:top` is *not* quietly read as a block called
  `amber:top` in namespace `example`.
- **A duplicate `name`.** Two files declaring the same block name is refused
  naming a first file and a second — which is why the directory is always read in
  the same order, so "first" and "second" mean something.
- **A `blocks/` directory that declares nothing.** A content root that exists and
  registers no blocks is an error naming the root, not an empty success. A loader
  asked for definitions that produced none is treated as broken.
- **A chunk that does not return a table.** Returning a function, a number, or
  falling off the end without returning anything is refused naming the file. So
  is a chunk that stops itself with `error(...)` — and in that case what you said
  is quoted back, which makes `error` a legitimate way to say a block cannot be
  declared.

One thing that is **not** a failure: adding a block does not invalidate a save
you already have. The save's compatibility check asks about the blocks the save
actually references, so a block you just invented cannot be missing from it and
cannot have changed. Delete `amber.luau` again *after* placing some, though, and
the save now references a block nothing declares — the game will refuse to start
and name it, because a block that no longer exists cannot be resolved at all.

## Where each contract is written down

| To declare | Read |
|---|---|
| Blocks — every field, the defaults and why they differ, the bounds, break and placement rules | `blocks-items.md` |
| HUD elements — anchors, UI units, colours, outlines, draw kinds | `hud.md` |
| Voxel models and materials — the `.mcvox` document, palettes, parts, and the `voxforge` CLI | `voxel-models.md` |

Each one is the complete contract for its kind. Read the one you need; you do not
need the others.

## The scripting host your declaration runs in

A block declaration is Luau, so it runs in the same sandboxed host every script
in this engine runs in: a budget on how long it may run, a cap on how much memory
it may hold, and isolation so that a broken declaration is refused by name
instead of taking the game down.

You do not need any of that to write a block — the walkthrough above is the whole
of it — but it is what decides what your chunk may do while it computes what it
declares. **There is no `mycraft.*` binding of any kind yet**: no block, world,
entity or registry access, and no way to attach behaviour from script. A
declaration returns a table and calls nothing the engine provides.

Four pages document that environment, one question each.

| To find out | Read |
|---|---|
| How a chunk is shaped, what it returns, and the two rules you write it under | `script-writing.md` |
| Which globals it can reach, and what the environment refuses | `script-surface.md` |
| What it may spend, and how to tell in advance whether a workload fits | `script-limits.md` |
| What a failure says, whose fault it is, and when a callback stops being called | `script-faults.md` |

Start at `script-writing.md`; the other three assume it.
