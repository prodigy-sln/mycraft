# Hot reload

Save a declaration while the game is running and it is in the world about a sixth of
a second later. No restart, no reloading a save, nothing to press.

This page is the whole contract: what triggers a reload, what a reload reads, which
edits you will see and which are accepted and invisible, what survives it, and every
refusal you can meet with the words it prints.

## The loop, start to finish

Have a client running — `cargo run -p mc-client` from the checkout root. Then:

```
content/base/blocks/stone.luau        edit `solid = true` to `solid = false`, save
```

Walk into stone. You go through it. Change it back, save, and it stops you again.
Nothing was restarted, your world is exactly as you left it, and you are standing
where you were standing.

That is the shortest complete example. There is a longer one at the end of this page
that adds a block rather than changing one.

### Work in your own root, not in the one the repository ships

**The supported way to try a content edit, and not a workaround.** The client resolves
both its content root and its save relative to **the directory it was started in**, and
that is a deliberate choice rather than an accident — so give a play session its own
directory and it cannot disturb anybody else's:

```text
mkdir -p playground/content
cp -r content/base playground/content/base
cd playground
cargo run --manifest-path ../Cargo.toml -p mc-client
```

Now `playground/content/base/blocks/` is the root under watch, `playground/saves/` is
where the world is kept, and editing either touches nothing the repository's own tests,
golden frames or tooling read.

**Editing `content/base/` in the repository itself works and costs somebody else
something.** A block declared non-solid re-meshes the whole world, so a golden frame
shot while your edit is in the tree disagrees with its committed image by three quarters
of its pixels — which reads as a catastrophic rendering regression rather than as
somebody playing. If you must edit the shipped root, say so to whoever else is working
in the tree first.

## What triggers a reload

**A file saved directly inside `blocks/` with a `.luau` extension, or directly
inside `hud/` with a `.toml` extension.** That is the whole rule, and it is
deliberately narrow:

| You save | Reload? |
|---|---|
| `content/base/blocks/stone.luau` | yes |
| `content/base/hud/crosshair-horizontal.toml` | yes |
| `content/base/blocks/experiments/draft.luau` | **no** — not directly inside `blocks/` |
| `content/base/blocks/notes.txt` | **no** — not a declaration extension |
| `content/base/blocks/stone.luau.swp`, `.tmp`, `~` | **no** — and this is the point of the rule |

Editors write scratch files beside the file you are editing. A rule that watched
everything in the directory would try to load half of them.

**Creating, deleting and renaming all count**, because each of them changes what the
root declares.

### The settling window is 150 ms

A save is rarely one write. Editors commonly write a temporary file and rename it
over the original, or write in several pieces. Reading the file the instant the
first write lands means reading a half-written declaration and refusing a candidate
you never saved.

So a change is reported once the root has been quiet for **150 milliseconds**, and
several saves inside that window become one reload. If you save twice in quick
succession you get one reload carrying both.

You will notice this as a small, consistent delay and nothing else.

## What a reload reads

**The whole content root, every time.** Every declaration in `blocks/` and every
declaration in `hud/`, read together, exactly as a launch reads them — the same
loader, the same validation, the same refusals.

It is not a diff. Saving one block file re-reads all four of them plus the three HUD
declarations. That is what makes "the content the game is serving" one coherent
thing rather than a pile of files that happened to load at different times.

**Blocks and the HUD are accepted or refused together.** A broken HUD declaration
refuses your block edit along with it, and the reason names the HUD file. They are
one content set.

## All of it, or none of it

A candidate is built off to one side, checked completely, and only then swapped in at
a boundary between two ticks. There is no state in which half of your edit is live.

If anything about the candidate is refused, **the content already serving goes on
serving** and one line is printed on the terminal you started the game from. Fix the
file, save again, and the next candidate is taken up. Nothing accumulates: a refused
candidate leaves nothing behind.

## What survives a reload

Everything that is not a declaration:

- **Your world, cell for cell.** Every block you broke is still broken and every
  block you placed is still there.
- **Where you are and how you are moving.** Position and velocity both, to the bit —
  a reload is not a teleport. The one exception is a player the new content trapped;
  see below.
- **Your save.** A reload neither writes it nor reads it.
- **The tick.** The simulation does not restart, rewind or skip.

What does *not* survive is the block in your hand, and only because it is re-derived
rather than remembered — see the next section.

## The block in your hand

**The block you hold is the first solid block in registration order**, and
registration order is the `blocks/` directory sorted by file name. A reload works out
that block again from the content it just took up.

So: declare a new solid block whose file name sorts first, save, and **it is in your
hand and you can go and place it.** That is the whole reason the rule is re-derived
rather than preserved.

> This is a placeholder for an inventory, and it is worth knowing that before it
> surprises you. There is nothing to select from and nothing to carry; "held" means
> "the one block a placement uses". When there is an inventory, holding a block will
> be something you did rather than something the directory listing decided.

## Which edits you will see

This is the part most worth reading before you spend an afternoon on a change that
loads and does nothing.

| Field | Visible after a reload? |
|---|---|
| `solid` | **yes** — the world is re-meshed and you can walk into, or through, what changed |
| adding a declaration | **yes** — a new solid block sorting first arrives in your hand |
| removing a declaration | only if the world holds none of that block; otherwise the candidate is refused |
| `texture` | **accepted and invisible** — see below |
| `replaceable` | **yes**, in behaviour: whether a placement builds straight through it |
| `breakable` | **yes**, in behaviour: whether it can be broken at all |
| `breaks_into` | **yes**, in behaviour: what the cell holds afterwards |

### Editing `texture` is accepted and does not change what you see

**A standing limitation, not a bug.** What layer of the array texture a block draws
from is selected by the block's **name** today, not by its declared `texture` key.
So an edit to `texture` is read, validated and taken up, and the block goes on
drawing what it drew before.

What the field does do is take up an array-texture layer — see the budget below — so
declaring a texture key nothing else declares is not free even though it is not
visible.

**Declare `texture` equal to `name`.** Every shipped declaration does. A declaration
whose `texture` differs from its `name` loads, is accepted, and then cannot be
packed: the affected re-mesh batch fails, the failure is logged, and the picture you
are looking at simply stops being updated for those sections. That is a confusing
half-state to debug and it is entirely avoidable.

### `breaks_into` naming nothing is accepted, and fails when you break it

A declaration may name a residue no declaration registers. The candidate is accepted
— names are resolved where a break reads them, not where they are declared, because
a block may legitimately name a residue a later file registers.

The consequence is that a typo here reaches you as a broken *break* rather than a
refused reload. If breaking a block does nothing and prints a refusal naming a block
you do not recognise, look at its `breaks_into`.

## The array-texture layer budget

**One session may assign 256 array-texture layers.** That is not a configurable
limit: eight bits of every packed vertex carry a layer index, so 256 is the
content-to-renderer contract.

Layers are **appended and never renumbered** within a session. A texture key that
has been assigned a layer keeps that layer for the whole run, even after no
declaration names it any more — because a layer index is already sitting inside
vertices the renderer has been given, and moving it would repaint parts of your world
with somebody else's texture.

So the budget is spent by the number of **distinct texture keys the session has ever
seen**, not by the number live at once. Rename a key back and forth twenty times and
you have spent twenty layers.

You are very unlikely to meet this by authoring. You can meet it by scripting an
edit loop that renames a key.

**Relaunching reclaims every layer retired since the client started.** That is
literally the fix: quit and start again, and the count begins from the keys the
content actually declares.

## If a reload makes solid a cell you are standing in

You are moved to the nearest clear position rather than left inside rock.

- **Sideways in preference to upward**, and **never downward** — downward is not
  ranked last, it is not searched at all.
- Up to **eight blocks** out horizontally and eight up.
- **You arrive in the middle of a cell.** Being moved costs you your exact position
  within the block, which is a real cost and the reason a player who did *not* need
  moving is not moved at all rather than nudged to the centre of the cell they are
  already in.
- **Whatever jump or fall you were in the middle of is taken away.** You have been
  teleported; finishing the arc you started somewhere else would put you back inside
  something.
- **If nothing within eight blocks is clear you are left where you are and told so.**
  The reload still stands — the content is not refused for this — and the terminal
  says how far was looked.

Everything above is something **your edit** can do to whoever is playing. Turning
`solid = false` into `solid = true` on a block the world is full of — water is the
easy one — moves every player standing in it, and eight blocks is the whole distance
the game will look. It is not an error and there is nothing to catch: the swap is
accepted, the player is told what happened to them, and the loop carries on.

### Near the edge of the world, the search has less to look at

The world is a fixed square of ground with an edge, and past that edge there is no
ground — not empty space someone could be put in, but nothing the game knows anything
about. **Only somewhere the world actually exists counts as clear.** The eight blocks
are measured the same way wherever a player stands, but within eight blocks of an edge
part of that box is outside the world and none of it is eligible, so there are fewer
places to be put and they are all inside the world.

Two consequences worth knowing before you meet them:

- A player cleared near an edge is moved **inward or upward**, never outward. They can
  end up further from where they were than the same edit would have moved them in the
  middle of the world, because the near side of the search is not available.
- Where the eligible ground is all solid too, they get the *nothing within eight blocks
  is clear* line and stay put — the same answer as a player wedged in the middle of a
  lake, and there is no separate message for being near an edge.

That refusal is **not** a fault in your content. Nothing is being refused, there is no
declaration you can change to avoid it, and the alternative it replaces is worse: a
player put past the edge stands where nothing is solid and falls out of the world.

## The HUD reloads too

`hud/` is read on the same pass and applied by the same swap. Change a crosshair's
colour or extent, save, and it is on screen at the next frame. A refused HUD
declaration refuses the whole candidate, blocks included.

The debug overlay (F3) is not content and is unaffected.

## Every refusal you can meet

All of them print one line beginning `mycraft: ` on the terminal you started the game
from, and all of them leave the content already serving in place. **Every line quoted
below is compared against a real run**, so a page that drifted from what the program
prints fails the build rather than misleading you.

Every one of them opens the same way — *the content root could not be taken up* — and
then says what stopped it, outermost first. Read them left to right: the stage, then
the file, then the field, then the cause.

### A declaration the loader will not accept

A chunk that will not compile, a misspelled field, a bad namespace, two files claiming
one name, a value past its bound, a declaration that loops or allocates past the
sandbox limits, an emptied `blocks/` directory, or a refused HUD declaration. **These
are the launch refusals** — same loader, same words — under the reload's own opening
sentence. This one is `solid` typed as `slid`:

```
mycraft: the content root could not be taken up: the content root could not be read: the content root's blocks could not be read: content/base/blocks/stone.luau, block `base:stone`, field `slid`: `slid` is not a field a declaration may state; a declaration may state `name`, `texture`, `solid`, `replaceable`, `breakable`, `breaks_into`
```

The fields it lists are the whole set a declaration may state, which makes this the one
refusal that also answers "what *can* I write?".

### A block that stops being declared while the world holds it

The one you will meet most often after a typo, because deleting a file is how you tidy
up. Here two declarations were removed while the world still held both:

```
mycraft: the content root could not be taken up: the world holds `base:grass` and `base:stone` that this content does not declare
```

**Every such block is named, ascending** — not just the first. Nothing can go in a cell
whose block no longer exists, and that is not a decision to make on your behalf. Break
the blocks first, or put the declaration back.

### Nothing solid left to place

Take the solidity off every block and there is nothing for a placement to use:

```
mycraft: the content root could not be taken up: the content registers no solid block, so a player would have nothing to place; the block a client holds is the first solid one in registration order
```

The second half is the rule that decides your held block, quoted back at you because it
is why the first half matters.

### The session is out of array-texture layers

A session may assign 256, they are never renumbered, and they are spent by distinct
texture keys ever seen rather than keys live at once. **The line below is one example
run, not the sentence** — the two counts in it are *what this content needed* and *what
the session had already assigned*, so yours will differ:

```
mycraft: the content root could not be taken up: the content root could not be read: the content root needs more texture layers than this session has left: this content needs 257 texture layers and a session has 256; 256 are already assigned, and relaunching reclaims every layer retired since the client started
```

It names both numbers and the way out. **Relaunching reclaims every layer retired since
the client started** — that is literally true, not a suggestion to try turning it off
and on again: the count starts again from the keys the content actually declares. The
256 is the session's bound and is fixed; the other two move with what you saved.

### A refusal is said once per distinct text

Save the same typo five times and you read it once. Save a *different* mistake and that
one is reported. So a terminal that has gone quiet after one refusal is not a terminal
that stopped watching — it is one telling you nothing has changed about why your last
save was turned away.

### Two more, described rather than quoted

Neither is an authoring mistake, so neither is on this page as a line to match:

- **The root cannot be watched at all.** Printed once, as soon as the world lands,
  naming the directory and the reason. Edits will not be noticed for the rest of the
  run; the game plays on with the content it read at launch.
- **The thread building a candidate ended without producing one.** Printed once. The
  next save starts a new one.

## A worked example that runs

From the checkout root, with a client already running.

**1. Add a block.** `content/base/blocks/amber.luau`:

```lua
-- A new solid block, added while the game is running.
--
-- `texture` is declared equal to `name` because the layer a block draws from is
-- selected by its name today. Declaring anything else here loads and then will
-- not pack.
return {
	name = "example:amber",
	texture = "example:amber",
	solid = true,
}
```

Save it. `amber.luau` sorts before `dirt.luau` and declares `solid = true`, so it is
now the first solid block in registration order: **the swatch at the bottom of the
screen is your block, and right-clicking places it.** You did not restart anything
and your world is untouched.

**2. Make it non-solid.** Change `solid = true` to `solid = false` and save.

Two things happen. You can now walk through every `example:amber` you placed — and
the faces the mesher had culled against them are drawn, so the world visibly opens up
around them. And `base:dirt` is back in your hand, because the first *solid* block in
registration order has changed.

**3. Break it.** Something to notice rather than something to do: with the block
non-solid, place is still available and break still works. Nothing about a reload
resets what you have built.

**4. Delete the file** while the world still holds your blocks, and save the
directory.

The candidate is refused and names `example:amber`. Your world goes on serving the
content that has that block in it. Put the file back and it is accepted again — or
break every `example:amber` you placed first, and then deleting it is accepted.

That last step is worth doing once. It is the clearest demonstration that a reload is
all-or-nothing and that it will not let you saw off the branch you are standing on.

**5. Move a player with an edit.** Walk into water somewhere near the middle of the
world, so that you are standing *in* a cell rather than beside one. In your root's
`blocks/water.luau`, change `solid = false` to `solid = true` and save.

You are moved — sideways if anything sideways is clear, otherwise upward — and the
terminal says where you were put. Note what it cost you: you are standing in the
middle of a cell, and any fall you were in the middle of is gone.

Now undo it, walk to a corner of the world, stand in water again within a few blocks
of the edge, and make water solid a second time. You are moved again — and the
position the terminal prints is **inside** the world, never past its edge, even though
most of the eight blocks around you at a corner is not world at all. Both runs
accepted your content; what differs is how much ground the search had, which is the
world's shape rather than anything in your declaration.

## What hot reload is not, yet

- **Behaviour.** A declaration says what a block *is*. Nothing reacts, ticks or runs
  code while you play, so there is no callback to reload and no state to migrate.
- **Mod tests.** There is no `tests/` directory that runs against a candidate before
  the swap. What gates a candidate today is the loader's own validation plus the two
  reload checks above.
- **`on_reload` and `mycraft.state(...)`.** Neither exists. There is no binding a
  declaration can call at all.
- **A second content root.** One root, `content/base/`, relative to where the client
  was started.
- **Selective re-meshing.** A candidate that changes what is drawn re-meshes the
  whole world rather than the parts that changed. At this world size that is cheap;
  it is why a reload is not free either.
