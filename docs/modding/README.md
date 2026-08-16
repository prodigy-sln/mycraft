# Making a mod

Start here. This page takes you from a clean checkout to a block of your own,
visible in the game, in one file and one command. Everything else in
`docs/modding/` is a reference for one kind of declaration; this is the way in.

## What a mod is here

A mod is a directory of data files. The engine reads them through a loading
contract and never learns where a definition came from — which is what lets the
base game be a mod like any other. **`content/base/` is the vanilla game**, it
holds no privileged declaration, and every field it uses is a field your own
content can use.

There is one thing to know before you write anything: **the client reads exactly
one content directory today, `content/base/`, resolved relative to the directory
you start it in.** The loading contract is already written in terms of
`content/<mod>/`, and the references below describe it that way, but nothing
reads a second root yet. So a declaration you add goes in `content/base/`
alongside the shipped ones. You can and should still use your own namespace —
`example:amber` rather than `base:amber` — because the namespace is what the id
means, not where the file sits.

## What you can define today

Four kinds of file, and they do not all reach the same place:

| You write | Under | It reaches | Reference |
|---|---|---|---|
| **A block** — a thing that occupies a cell, with solidity, replaceability and what it drops | `content/base/blocks/*.toml` | the running game | `blocks-items.md` |
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

Create `content/base/blocks/amber.toml`:

```toml
name = "example:amber"
texture = "example:amber"
solid = true
```

Three fields, all required, and that is a complete block. `name` is what the
block is called everywhere — in save data, in other declarations, in error
messages. `texture` is a **key**, never a file path; what pixels it resolves to
is the renderer's business. `solid` is whether it stops a player walking into
it.

Both ids follow one rule: **exactly one `:`, with non-empty text either side.**
`example:amber` is fine, `example:amber:top` is refused by name, and `amber` is
refused for having no namespace at all.

### 2. Run it

From the root of the checkout:

```
cargo run -p mc-client
```

The client looks for `content/base/` relative to where it was started, so start
it from the checkout root or it will exit saying it found no content there.

### 3. Look at the bottom of the screen

The small square at the bottom centre — the swatch showing the block a placement
would use — **is now your block**, in a colour it did not have before. Right-click
anywhere in reach and you place `example:amber`. Left-click breaks. Every block
you place is yours, and they persist: close the window normally and they are
still there next launch.

That is a mod. One file, no build step for the content itself, no registration
call anywhere.

## Why that worked, and how to make it work for your block

The swatch changed because of a rule worth knowing before it surprises you:

> **The block you hold is the first *solid* block in registration order, and
> registration order is the `blocks/` directory sorted by file name.**

`amber.toml` sorts before `dirt.toml`, and it declares `solid = true`, so it
displaces `base:dirt` as the held block. Name the file `zinc.toml` instead and
everything still loads — the block is registered, the world knows it, a save can
hold it — but you have no way to reach it, because there is no inventory, no
hotbar and no way to choose what you are holding. The first-solid-block rule is
a placeholder standing in for an inventory that does not exist yet, not a
designed feature.

So while there is no way to pick a block, **a file name that sorts before
`dirt.toml` is how you get your hands on the one you just wrote.**

Two consequences of the same rule:

- A block declaring `solid = false` is skipped when choosing what you hold, so a
  non-solid block cannot be the held one however its file sorts. If your content
  registers no solid block at all, the client refuses to start rather than
  opening a window you can place nothing in.
- The colour is generated from the **texture key** and from nothing else — the
  same key gives the same colour on every machine, on every run, forever. You
  cannot predict which colour from the name, and you should not try to: block
  art does not exist yet, and the client says so on startup. Stone and dirt draw
  teal, grass draws tan. Yours will be some other deterministic colour.

## When you get it wrong

**Loading is all-or-nothing.** A content directory either registers every
declaration in it or none of them — there is no partial load, and one bad file
does not cost you just that file. A failure leaves the registry exactly as it
was.

**A content failure stops the launch.** The client exits without opening a
window and prints a line beginning `mycraft:` on standard error. There is no
error screen, no safe mode, and no starting-anyway-without-it: a window that
opened with your block silently missing would tell you nothing.

**What that line says today is less than the engine knows.** Every refusal below
is constructed naming the file it came from, the block name it was declared
under and the field at fault — but the client prints only the outermost sentence
of it, so a bad block file reports `mycraft: the shipped content could not be
read` and stops there. Until it prints the whole chain, the practical advice is
the unsatisfying one: **change one file at a time**, so the file you last
touched is the file at fault.

Four ways to get it wrong, all of them refusals rather than surprises later:

- **A field the loader does not recognise.** A typo like `slid = true` is
  refused, not ignored. A silently-ignored typo is a debugging trap; being
  refused is what keeps you out of it.
- **A malformed id.** No colon, an empty side, or more than one colon. Note the
  last one especially: `example:amber:top` is *not* quietly read as a block
  called `amber:top` in namespace `example`.
- **A duplicate `name`.** Two files declaring the same block name is refused
  naming a first file and a second — which is why the directory is always read
  in the same order, so "first" and "second" mean something.
- **A `blocks/` directory that declares nothing.** A content root that exists
  and registers no blocks is an error naming the root, not an empty success. A
  loader asked for definitions that produced none is treated as broken.

One thing that is **not** a failure: adding a block does not invalidate a save
you already have. The save's compatibility check asks about the blocks the save
actually references, so a block you just invented cannot be missing from it and
cannot have changed. Delete `amber.toml` again *after* placing some, though, and
the save now references a block nothing declares — the game will refuse to start
and name it, because a block that no longer exists cannot be resolved at all.

## Where each contract is written down

| To declare | Read |
|---|---|
| Blocks — every field, the defaults and why they differ, break and placement rules | `blocks-items.md` |
| HUD elements — anchors, UI units, colours, outlines, draw kinds | `hud.md` |
| Voxel models and materials — the `.mcvox` document, palettes, parts, and the `voxforge` CLI | `voxel-models.md` |

Each one is the complete contract for its kind. Read the one you need; you do
not need the others.

## Where the scripting host fits

The engine carries a sandboxed Luau host: a place mod code will run, with a
budget on how long it may run, a cap on how much memory it may hold, and
isolation so that a broken mod stops acting instead of stopping the server.

**Nothing is authored in Luau today, and nothing in the game reaches the host.**
There is no `mycraft.*` binding of any kind — no block, world, entity or
registry access, and no way to declare or attach anything from script. The host
is machinery that has been built and measured and that no author can yet use.
The blocks and HUD elements above are the whole of what can be authored, and
they are data files, exactly as this page describes them.

`sandbox.md` documents that environment: what a script will be able to reach and
what stops it, every limit with its number, and what a failure looks like. It is
worth reading before you write Luau and it will not help you write content
today — **this page and the three references above are where authoring lives.**
