# Hot reload

Save a declaration while the game is running and it is in the world about a sixth of
a second later. No restart, no reloading a save, nothing to press.

This page is the whole contract: what triggers a reload, what a reload reads, which
edits you will see — every declared field, as it turns out — and the one thing a
reload cannot pick up, what survives it, and every refusal you can meet with the
words it prints.

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

**You get the verdict whatever your frame rate.** Whether the answer is a swap or a
refusal, it reaches you on a machine drawing 200 frames a second and on one struggling
at 15. That is worth stating because it was briefly untrue: for one build the game
advanced the world once per drawn frame, and when that was fixed so a slow frame
advances several ticks at once, a verdict produced by one of those ticks could be
overwritten by the quiet ticks after it before anything showed it to you. On a slow
machine your edit would go live with the screen never catching up, or be refused with
nothing printed at all — and the refusal for a content root that cannot be watched is
said **once**, so that one was lost outright rather than repeated. If you ever saved a
file, saw nothing happen and could not tell whether the game had read it, that build is
the likely reason. It is fixed: a tick with nothing to say now leaves the last answer
standing instead of erasing it.

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
| `drawn` | **yes** — the world is re-meshed and the block appears or disappears |
| `occludes` | **yes** — the world is re-meshed and the faces behind it appear or vanish |
| `solid` | **yes** in behaviour: you can walk into, or through, what changed. Whether it also *redraws* depends on the next paragraph |
| `targetable` | **yes**, in behaviour: whether your swing finds it. Nothing is re-meshed for it |
| adding a declaration | **yes** — a new solid block sorting first arrives in your hand |
| removing a declaration | only if the world holds none of that block; otherwise the candidate is refused |
| `texture`, as one string | **yes** — all six faces redraw from the new key |
| `texture`, as a table of six facings | **yes**, and it spends a layer per distinct key |
| one facing's key inside a table | **yes** — that face alone redraws, and the world is re-meshed for it |
| `replaceable` | **yes**, in behaviour: whether a placement builds straight through it |
| `breakable` | **yes**, in behaviour: whether it can be broken at all |
| `breaks_into` | **yes**, in behaviour: what the cell holds afterwards |
| `swimmable` | **yes**, in behaviour, on the next tick: whether holding jump lifts a player inside it. Nothing is re-meshed for it |
| `move_resistance` | **yes**, in behaviour, on the next tick: how much the block's volume slows what moves through it. Nothing is re-meshed for it |
| `swim_ascent` | **yes**, in behaviour, on the next tick: how fast the block's volume lifts a swimmer holding jump. Nothing is re-meshed for it |
| `opacity` | **yes** — how much you see through the block changes on the next batch. Nothing is re-meshed and no texture is uploaded: the faces move between the opaque and the blended draw and the degree is rewritten into the geometry already built |

**What decides a re-mesh is `drawn`, `occludes` and the six keys — and nothing
else.** Editing `solid`, `targetable`, `replaceable`, `breakable`, `breaks_into`,
`swimmable`, `move_resistance` or `swim_ascent` changes what the world *does*
without changing what it looks like, so no section is built again.

**`opacity` is the one field that changes what the world looks like without
re-meshing it**, and it is worth knowing which of the two you are watching.
Nothing about *which faces exist* depends on the degree, so the mesh the world
already has is the right one; what changes is which draw each face goes into and
what number it carries. That work happens when the geometry is packed, which is
the step after meshing and runs on every batch anyway. So editing a degree is
cheaper than editing `occludes` — and if you edit both at once you pay for the
re-mesh, because `occludes` asked for one.

**Editing `solid` alone still redraws, and it is worth knowing why**, because the
answer changes the moment you write one more line. `drawn` and `occludes` default
to whatever the same declaration says about `solid` — so a declaration that never
mentions them moves all three at once when you flip `solid`, and the world
re-meshes. Write `drawn` and `occludes` out explicitly, and from then on editing
`solid` moves only collision: your block goes on looking exactly as it did while
you walk through where you used to stand. That is the split working, not a reload
that missed something.

**A reload that changes only `targetable` is accepted and re-meshes nothing.**
Your swing starts or stops finding the block on the very next tick; the picture on
screen never flickers, because nothing about it changed.

**The same holds for all three medium fields, and it is the fastest loop on this
page.** Each takes effect on the next tick and none rebuilds a section — so you
can retune how a liquid feels while standing in it. Raise `move_resistance` and
save, and the next step you take through the block is slower; lower
`swim_ascent` and the jump you are already holding lifts you more slowly from
that tick on; take a `swimmable = true` away and it stops lifting you at all,
wherever you happen to be. Nothing about the world redraws for any of them,
because there is nothing to redraw: a volume that stopped holding you up looks
exactly like one that still does.

That is the loop worth using on `swim_ascent` in particular, because the ratio
it sets — `(swim_ascent − 0.5) / 4.5`, which `move_resistance` cancels out of
entirely — is a feel rather than a figure. Sit in your liquid, edit the one
number, save, and hold jump again.

**A `move_resistance` or a `swim_ascent` the loader would refuse refuses the
whole reload**, exactly
as any other bad field does — the file and the field are named, nothing is half
applied, and the game goes on serving the content it was already serving until
you fix it and save again. So a mistyped number costs you a refusal you can read
rather than a sea that quietly stopped being one.

### Editing `texture` changes what you see

Save a new key against a facing and that face redraws. It holds for both forms of
the field: a single key repoints all six faces, and a table naming a key against
each of the six facings — see
[A texture per face](blocks-items.md#a-texture-per-face) — repoints exactly the ones
you changed.

**Re-pointing one facing is noticed.** The comparison that decides whether a reload
changes what is drawn reads all six keys, so editing `north` alone marks the world
for a re-mesh rather than passing as no change at all. The whole world is re-meshed
and re-drawn, and the face you changed comes back different.

What the edit costs is an array-texture layer — see the budget below — so naming a
key nothing else declares is not free. Two facings naming one key share one layer.

**What the facing comes back as is decided per key, not for the reload as a whole.**
Re-point a facing at a key your content root's built texture set covers and it comes
back drawing that key's image; re-point it at a key nothing has baked and it comes
back drawing a generated stand-in derived from the key's own spelling. Both answers
are ordinary and they can sit on two facings of one block — see
[Texture keys today](blocks-items.md#texture-keys-today).

**A reload draws from the art the launch read, and never bakes or re-reads any.**
The built set is handed to the renderer once, when the client starts, so the
pictures a reload can choose among are exactly the ones that were on disk then. Two
consequences, and the first is the one worth relying on: a facing already drawing
its baked image goes on drawing it across every reload, rather than falling back to
a stand-in. The second is the cost: run `voxforge build` while the client is up and
the new image is not picked up until you restart, so the facing keeps its stand-in
even though the file is there.

**A key the session cannot give a layer to refuses the whole reload**, and nothing
on screen moves — the sections you are looking at go on drawing exactly what they
drew. That is the budget refusal below, reached by re-pointing one facing rather
than by adding a block.

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

**Distinct keys, not faces.** A block declaring six facings spends one layer per
distinct key among them, so a grass block naming one key above and below and one
across its four sides costs five and not six. There is no per-block allowance: six
facing keys come out of the same 256 a single key comes out of, and the last layer
a session has is the last layer whichever kind of declaration wanted it.

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

## The same edit made while the game is not running

Everything above is what your edit does to somebody who is playing. Editing a content
root with nothing running is the other half of the same authoring loop — you quit, you
change a declaration, you start the client again — and it used to be the more dangerous
half: a `solid = false` block that became `solid = true` while the game was off left
whoever was standing in it inside solid rock on the next launch, with no move and no
line on the terminal.

**It is now answered at entry exactly as a live change is answered at the swap.** Same
search, same eight blocks, same sideways-before-upward order, same rule about ground
the world does not have. What differs is only the words, and they differ on purpose: a
player moved by a reload watched their cell become solid, and a player moved at entry
witnessed nothing at all, so the entry line names the state it found rather than an
event nobody saw.

**Both lines a launch can write, quoted:**

```
mycraft: you would have entered the world inside solid blocks, so you were moved to (12.5, 10, 12.5)
```

```
mycraft: you would have entered the world inside solid blocks and nothing within 8 blocks is clear, so you were left inside them
```

**The three numbers in the first are one example run, not part of the sentence** — they
are where that player's feet ended up, so yours will differ. The `8` in the second is
the search's reach and is the same for everyone.

The reload's own pair, for comparison, is `mycraft: the reload made your cell solid, so
you were moved to (x, y, z)` and `mycraft: the reload made your cell solid and nothing
within 8 blocks is clear, so you were left where you were`. Those two are written in
prose here rather than fenced, and the reason is worth knowing before you paste a line
onto this page: **every fenced block on these pages whose first line begins `mycraft: `
is compared, character for character, against text a real run produces** — so a fence
is a promise the build checks, and quoting a line no run in that guard produces fails
the build rather than documenting anything. The two entry lines above are produced; the
two reload lines are not yet.

Three things follow for you as an author:

- **Neither line is a refusal.** Nothing about your content is turned away, the launch
  proceeds, and there is no declaration you can change to make the message go away
  other than the solidity change you meant to make.
- **The edge rule is the same one.** Only ground the world actually holds counts as
  clear, so a player near the edge is moved inward or upward and never out over it —
  and where the eligible ground is all solid too, they get the second line and stay put.
  That is the same answer a player wedged in the middle of a lake gets; there is no
  separate message for being near an edge.
- **The launch is never refused for it**, which is what keeps the loop usable: you can
  always start, read the line, quit, and change the declaration back.

A save whose blocks *behave* differently than they did when it was written is a
separate question, answered before this one and answered the same way: the world is
loaded, and the blocks that moved are named on the error stream.

```
mycraft: `base:water` no longer behaves as it did when this world was saved, and it was loaded anyway
```

Every changed block, ascending, on one line, and nothing at all when nothing
changed. Making water solid is exactly such a change, so the offline half of the
loop prints two lines rather than one — first which blocks the save disagrees with
you about, then where the player who loaded it can stand.

**A launch is refused for it only if you ask.** `--refuse-changed-blocks` leaves
such a world shut and names the blocks, which is what somebody restoring a backup
they are unsure of wants: opening a changed save and quitting normally rewrites its
recorded hashes against the content you are running, and the next launch has nothing
left to notice. For the write-edit-relaunch loop this page is about, leave it off —
`docs/user/gameplay.md` has the player-facing half.

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
mycraft: the content root could not be taken up: the content root could not be read: the content root's blocks could not be read: content/base/blocks/stone.luau, block `base:stone`, field `slid`: `slid` is not a field a declaration may state; a declaration may state `name`, `texture`, `solid`, `replaceable`, `breakable`, `breaks_into`, `drawn`, `occludes`, `targetable`, `swimmable`, `move_resistance`, `swim_ascent`, `opacity`
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
-- `texture` happens to equal `name` here; it need not. The key a face draws comes
-- out of the declaration, so any key you name here is the key this block draws --
-- and a key nothing has baked art for draws a generated stand-in rather than
-- refusing. See "Which edits you will see" above.
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

**6. Make the same edit with the game shut down.** This is the offline half, and it is
the step that used to end with a player inside rock.

Put `water.luau` back to `solid = false` first. Walk into water somewhere near the
middle of the world, so that you are standing *in* a cell, and **quit normally** —
close the window rather than killing the process, so the save is written with you in
there. Now, with nothing running, edit `content/base/blocks/water.luau` and change
`solid = false` to `solid = true`. Then, from the checkout root:

```
cargo run -p mc-client
```

Nothing on the command line, and you are let in. **Two lines come back**, in this
order. The first is the save disagreeing with your declaration:

```
mycraft: `base:water` no longer behaves as it did when this world was saved, and it was loaded anyway
```

The second is the one this section is about, and it is answered before the window has
anything in it:

```
mycraft: you would have entered the world inside solid blocks, so you were moved to (12.5, 10, 12.5)
```

**Worth running once with `--refuse-changed-blocks` as well**, to see the other
answer. That launch is refused rather than let in: it names `saves/world.mcw`, then
the blocks the registry cannot answer for — no missing ones, `base:water` changed —
and it ends by telling you to drop the argument to load the world anyway. Nothing is
written and nothing is lost; the world is simply left shut.

The three numbers are wherever your own player was put — near where you quit, at the
centre of a cell, on that cell's floor. You are standing on the world and you can walk,
and you were told why you are not where you left off.

To undo it: quit, set `solid = false` again, and launch. The save now records water
as solid, so the first line comes back — the disagreement runs in this direction too,
and a declaration you have just put back is still a declaration the save does not
recognise. The clearing line is *not* printed this time, because a player whose box
covers no solid cell is not moved and is told nothing about it.

**Worth doing once for the contrast.** Step 5 and step 6 are the same one-word edit to
the same file. The only difference is whether the game was running when you made it,
and that difference used to decide whether the player was looked after at all.

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
