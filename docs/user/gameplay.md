# Gameplay: Movement and Camera

Player-facing behaviour as built. This describes what a player experiences; see
`docs/technical/architecture.md` §"The player: intent, physics and collision" for how it is
implemented and why.

## Two commands, not one, if you run from source

The block art is generated, so a fresh clone has none of it, and the game will
not start until it has been made. Run this once, from the folder you cloned into:

```
cargo run -p voxforge -- build content/base/textures.toml
```

then start the game as usual. If you forget, nothing crashes and nothing opens —
the terminal you started from says exactly what to run:

```
mycraft: the generated texture set is not there; run `cargo run -p voxforge -- build content/base/textures.toml`
```

Run it again after changing anything the art is made from, and the same terminal
tells you when that is needed: *the generated texture set is stale against its
sources*, with the same command to fix it. Over unchanged files the command
finishes instantly and writes nothing, so there is no harm in running it every
time.

**This is what the world is made of**, so the check is not a formality: if you
skip the build you get a sentence in the terminal instead of a game, and if you
edit a model and forget to rebuild you get a different sentence instead of a
world that quietly shows you last week's art.

**If you played a build from before this one, this is new and you will meet it on
your first run.** The window used to open regardless and draw the placeholder
colours; the check existed but only the test suite went through it. Now the game
itself refuses, before the window opens and before it generates a world, and the
refusal above is the whole of what you get. A sentence naming the one command to
run is a better first run than a window full of art that is not the art.

## What the world looks like

The ground is **grass over dirt**, and below and through it there is **stone**.
Not coloured squares standing in for them — actual pictures, baked from voxel
models under `content/base/models/`:

- **Grass** has turf on top, bare dirt underneath, and a band of turf spilling
  over dirt on each of its four sides. Walk around one block and the four sides
  are four different pictures rather than the same one repeated, because the
  model is not symmetrical and each side of it is baked separately.
- **Dirt** is the same image the grass block wears on its underside. There is one
  picture and two blocks use it, which is why a grass block never sits on ground
  of a slightly different colour.
- **Stone** is grey with lighter and darker grain scattered through it.

Stand close to a face and you see the individual texels with hard edges between
them — that is deliberate, not a missing filter. Look at a hillside in the
distance and it stays still as you move instead of crawling and sparkling.

**You can see the sea.** Water is drawn now, where before it was a hole in the
world you walked through and could not see. A new world starts you on the shore
facing it, so the water is in front of you from the first frame — and it fills
more of the view as you walk toward it, from roughly a tenth of the screen at the
start to about a third partway down the beach.

What you see is the **surface and the edges only**. Water does not draw the
insides of itself, so a body of it looks like one sheet rather than a stack of
panes, and it does not hide what is underneath: the sand and stone of the lakebed
show through, which is what makes it read as water rather than as a lid.

**You can swim in it.** Walk off the shore and you no longer drop to the lakebed
like a stone — the sea slows everything that moves through it, so you sink into
it gently and land softly. Hold jump while you are in the water and you rise
instead of staying put, reaching the surface from the deepest part of the sea in
about a third of a second, and then floating there for as long as you keep
holding. Let go and you sink back down, slowly, and come to rest on the lakebed.

Swimming is **slower than walking** — a little over a third of your speed on
land — so crossing open water takes longer than crossing the same distance of
beach. That is the sea having its own rules rather than being a hole in the
world with a surface painted on it, which is what it was before this build.

**You can now aim at water, and swinging at it does nothing.** The crosshair used
to look straight through water and find whatever solid block was behind it, so a
swing aimed at the sea broke the lakebed under it. It stops at the water now — and
because the content declares water unbreakable, the swing is refused and the
water stays where it is. Nothing behind it is touched either: the swing never
reaches that far.

The practical difference is that **you can no longer dig through water to what is
under it by aiming at the water**. Aim at the lakebed from somewhere the water is
not in the way, and it breaks as it always did.

**Building into water is unchanged.** Aim at a cell of water, place a block, and
the block goes *into* that cell — water is declared as something a placement may
overwrite, so you never have to clear it first. That is worth stating beside the
paragraph above, because the two answers now differ: a swing aimed at water is
refused, and a placement aimed at the same cell goes through.

What the unbreakable declaration also changes is your save — though this build
changes it more bluntly than that. Water's own declaration moved, and so did the
way the game records what *any* block does, so a world saved before this build
reports all four blocks on the terminal rather than water alone. That happens
once for this change — and if you already met that line at the last build, you
meet it again here, because this is a second change of the same kind. "An older
save opens" below says why, and why the launch after it is quiet.

**A block whose art nobody has drawn still works.** If you install a mod that
declares a block and ships no picture for it, that block draws a generated
stand-in — a two-colour pattern derived from the texture's name, deterministic
and unmistakably not real art — and the game runs. The terminal says so on
startup, so a stand-in never reads as something you did wrong.

## An older save opens, and is told about once for each change it crosses

A save records what each block *was* when it was written: what it looked like,
and what it did. The game has changed those records at three different builds so
far — the looks record once, and the behaviour record twice — and a save crosses
every one of them written after it was.

**"Once" here means once per change, not once ever**, and that is the part worth
having straight before you read the rest. Each change is reported on the first
launch that crosses it and never again, so a save old enough to cross two of them
is told twice: once now, and once more the next time the record moves. Nothing is
wrong when that happens, and there is nothing to fix.

**The looks record moved when the game got real art.** Grass, dirt and stone
draw pictures now where they drew generated stand-ins before, and grass draws six
of them where it drew one. A world saved before that build comes back with every
block counting as retextured — and **nothing is said about it**, because a
message after every art change is what teaches people that a message means
nothing. The world opens as normal: your terrain, your edits and where you were
standing all come back exactly as you left them.

**The behaviour record has moved twice, and this build is the second.** It moved
first when blocks could say whether a swing can find them, and again now that
they can say what their volume does to you — whether you can swim in it, and how
much it slows you down. Both are part of what a block *is*, so a world saved
before either one opens with one line naming **all four** of the game's blocks —
not because all four were changed by anybody, but because the old record and the
new one are not comparable, and the honest answer is to say so rather than to
guess block by block.

**Each move is reported once, so this build reports again to a save that already
paid for the last one.** If you played on the previous build, met that line, and
quit normally, your save was rewritten under the record of the day — and it is
still older than the record this build keeps, so you are told once more. That is
not the same notice repeating. It is a second change being reported for the first
time, and it costs you the same single line.

Quit normally again and the save is rewritten under the new record; the next
launch says nothing, until the day the record moves again. From then on that line
means what it says below: a mod changed something.

There is no way to decline the new art, and no mechanism keeps a save on the old
pictures: what a block looks like comes from the content the game is running, not
from the save.

## A save whose blocks *behave* differently also opens, and the terminal says which

That one-off aside, this is the line's ordinary job. A mod can change what a
block *is* rather than what it looks like — whether it is solid, whether it can
be broken, whether a swing can find it, what it drops. Your world opens anyway,
and the game names the blocks **that mod** touched, which is usually one:

```
mycraft: `base:water` no longer behaves as it did when this world was saved, and it was loaded anyway
```

One line, every block that moved, in alphabetical order, on the error stream. It
is a notice and not a refusal — the world opens. Nothing is asked of you and there
is no prompt of any kind: read the line, and if a mod changed something you did
not want changed, put it back and relaunch.

**If you would rather not be let in, say so:**

```
cargo run -p mc-client -- --refuse-changed-blocks
```

With that on the command line a world whose blocks behave differently is left
shut, and the game names them so you know which mod to look at. This is worth
knowing about for one specific reason: **playing on destroys the evidence.** Your
save records what each block was when you last quit, so opening a changed world
and quitting normally rewrites those records against the mod you are running now.
The next launch has nothing left to notice. If you have just restored a backup and
are not sure what content it belongs to, `--refuse-changed-blocks` is how you look
without touching anything.

A block the running content does not declare *at all* is different again, and no
argument changes it: there is nothing to put in those cells, so the launch is
refused either way and the missing blocks are named.

## Controls

| Input | Action |
|---|---|
| W / A / S / D | Walk forward / strafe left / back / strafe right, relative to the direction you're facing |
| Mouse movement | Look around — turn (yaw) and tilt (pitch) |
| Space | Jump, while standing on something |
| Left mouse button | Break the block the crosshair is on |
| Right mouse button | Place a block against the one the crosshair is on |
| Escape | Release the cursor |
| Click in the window | Re-capture the cursor |
| F3 | Show or hide the debug overlay |

The walk keys are bound by **physical position**, not by the letter printed on them, so the same four
keys under your left hand walk the player on a QWERTZ or AZERTY keyboard as on a QWERTY one.

## What's on screen

Two things are drawn on top of the world:

- **A crosshair at the exact centre of the screen** — two thin white bars crossing, each with a black
  outline so they stay visible against snow, sky and an unlit cave alike. It marks the point your next
  break or place is aimed at.
- **A small square at the bottom centre showing the block a placement would use** — the same texture
  that block is drawn with in the world, outlined the same way. There is no inventory or hotbar yet, so
  the block never changes: the swatch shows you which one it is rather than letting you pick.

Both scale with the window, so they look the same size at 1280×720 and on an ultrawide. Neither is
part of the engine — both are declared in the game's own content files, exactly as a mod would declare
its own (`docs/modding/hud.md`), which also means a mod shipping a broken HUD declaration stops the
game from starting, with a message naming the file, the element and the field at fault, rather than
launching with something quietly missing.

## The debug overlay (F3)

**F3 shows a small readout in the top-left corner, and it is hidden until you ask for it.** It exists
to diagnose the game and its mods rather than to play with, and it shows four things:

- your position, as x, y and z;
- the **column** you are standing in — the 16×16 chunk column's coordinate, which is how the world is
  stored;
- the current **frame rate**;
- the current **frame time**, in milliseconds.

Press F3 again to hide it. Showing or hiding it changes nothing about the game itself — the same
inputs produce the same world either way — and no mod can hide, move, restyle or disable it. That is
deliberate: it is the instrument you use when a mod is misbehaving, so a mod must not be able to turn
it off.

Nothing else is on screen. There is no hotbar, inventory, menu, pause screen or health display, and no
part of the HUD accepts a click.

## How movement feels

- Walking has no acceleration, deceleration or inertia: you move at a constant speed the instant a
  key is held and stop the instant it's released. There is no sprint, crouch or fly mode.
- A jump clears a one-block rise but not a two-block one — there is no automatic step-up yet, so a
  two-block ledge has to be jumped onto directly or walked around.
- Falling speeds up the longer you fall, up to a maximum speed, same as it would look intuitively.
- **Your speed is the same whatever your monitor is.** Walking, jumping and falling are measured in
  seconds, not in frames, so a 144 Hz or 240 Hz display shows the same motion more smoothly rather
  than faster, and a machine struggling at 20 frames a second runs the world at the same speed —
  jerkier, but not in slow motion. (This was not always true. A build shipped in which the world ran
  at your refresh rate divided by 60, so a 144 Hz monitor made you move nearly two and a half times
  too fast; a player described it as warping around with super speed. If you played that build, this
  is what changed.)
- A very long freeze — a laptop lid closed, the game paused under a debugger — does not get replayed
  when you come back. At most a quarter of a second of the missed time is caught up on; the rest is
  simply gone, which is the right answer, because you were not playing.
- Looking straight up or straight down is capped a few degrees short of vertical, so the view can't
  flip upside down.
- The generated terrain is finite and there is nothing beneath it. Walk off the edge and you fall,
  and keep falling: there is no floor under the world, no fall damage and no respawn, so the only way
  back is to restart.

## Cursor capture

Moving the mouse only turns the camera while the cursor is captured by the window. If the operating
system won't grant the tightest capture mode, the game falls back to a looser one automatically
rather than refusing to run. Escape always gives the cursor back so you can interact with other
windows; clicking back in the game window always takes it again.

## Saving and resuming

Closing the game normally saves the world you were playing in and where you were standing in it.
Starting the client again picks that world back up and draws it that way from the very first frame —
the blocks you broke and placed are still broken and placed, visibly as well as underfoot, and you
stand where you left off, facing the way you were. Killing the process instead of closing it normally
does not save: whatever you built or moved since the last clean close
is lost, but the save from that last clean close is left exactly as it was, so a crash or a forced
quit never puts something already saved at risk.

The save is a single file, `saves/world.mcw`, relative to the directory you started the client in.
There is one world and one save — no slots, no world names, no automatic backups — so if you want to
keep a copy of a world before playing on, copy that file yourself.

**If the save can't be read, the game refuses to start rather than quietly generating a new world in
its place.** The refusal names the file and the reason. If the save was written under a mod set that
has since changed, what happens next depends on *what* changed, and the three cases are treated
differently:

- **A block that's missing outright** — a mod that defined it is gone — can't be resolved at all:
  there is nothing to put in those cells. The game always refuses to start over it, whatever you put
  on the command line, and names every missing block.
- **A block that behaves differently** than it did when you saved (solidity, breakability, what it
  drops) does not stop you. The world opens and the terminal names the blocks that moved, one line,
  all of them. Pass `--refuse-changed-blocks` if you would rather be turned away — see "A save whose
  blocks *behave* differently also opens" above for when that is worth doing.
- **A block that only *looks* different** — a retextured mod, with nothing about how it behaves
  changed — is not reported at all. That world loads with nothing said about it.

**A world you saved before one of these builds reports every one of its blocks at
once**, and that is expected rather than a sign anything is wrong. The game has
three times changed how it records what a block is: once for what a block looks
like, and twice for what it does — the second of those being this build, which
gave blocks a volume you can swim in. Across any of them the old record and the
new one are not comparable, so the honest answer is that they all moved. Which
ones you crossed decides what you see: the looks change is the case nothing is
said about, and each behaviour change is one line naming every block, once, on
the first launch that crosses it and never again. **A save old enough to cross
both behaviour changes is told once for each**, which is why a line you have seen
before can come back after a clean quit.

A save that loads can still put you somewhere that stopped being standable while the
game was off. That is answered rather than refused, and "Coming back to a save never
leaves you inside a block" below says what happens and what you read.

## Mods and scripting

**Every block you break and place is now defined in a script, and nothing about playing the game
changed because of it.** Dirt, grass, stone and water are declared in the game's own Luau files under
`content/base/blocks/`, and the game runs those files when it starts. Same four blocks, same terrain,
same block in your hand, same saves — **a world you saved before this change still loads, unchanged,
and looks the same.** If you could see a difference, something would be wrong.

What that buys is who can add one: a block is now written in the same language the rest of the game
layer will be written in, so somebody can add a block of their own without touching the engine. If
you want to try it, `docs/modding/README.md` walks from an empty file to a block in your hand.

**Behaviour is still not scripted.** Nothing reacts, ticks or runs code while you play — a
declaration says what a block *is* and then it is finished. The scripting host that runs those
declarations is sandboxed, with limits on how long a file may run and how much memory it may use, so a
mod file that loops forever or eats memory is refused by name instead of hanging the game. The
crosshair and swatch on your screen are still content data files rather than script. There is nothing
here for a player to turn on, try or configure beyond adding content of your own.

**If the game will not start, it now tells you what it could not read.** The
blocks and the on-screen crosshair come from content files, and a broken one
stops the game rather than letting it open with pieces silently missing. Where
that used to end in a single line saying the content could not be read, the game
now prints the file it was reading, the thing declared in it, and the part it
could not accept — so a file you or a mod author edited can be found and fixed
without hunting through all of them. The same holds for a saved world it cannot
load: it says which save, and why, once rather than twice, and where dropping
`--refuse-changed-blocks` would let you in anyway it says so at the end, after the
reason. Look on the terminal you started the game from; the text begins
`mycraft: `.

## Content edited while you play reaches the world without a restart

Leave the game running, edit a content file, save it, and the change is in the world
about a sixth of a second later. No restart, no reloading a save, nothing to press.
Blocks and the on-screen crosshair are both read again together, because both come
from the same content directory.

**Your world is not touched by it.** Everything you broke is still broken and
everything you placed is still there, cell for cell. You stay exactly where you were
standing, still moving however you were moving, with the same block in your hand —
unless the block in your hand stopped existing, in which case you are given another
one. Your save is not written and not read.

**A change either all happens or none of it does.** If the file you saved has a
mistake in it, the game keeps playing the content it already had and prints one line
on the terminal you started it from saying what it could not accept. Fix the file,
save again, and it takes. You never end up half-way between two versions, and you
are never left with a world the game has stopped understanding — a change that would
delete a block you have already placed somewhere is turned away for that reason and
named.

**If something you are standing in becomes solid, you are moved clear.** Make water
solid while you are swimming in it and you are put in the nearest clear space
instead of being stuck inside rock: sideways in preference to upward, never
downward, and searching up to eight blocks out. Being moved costs you your exact
position within the block — you arrive standing in the middle of a cell — and it
takes away whatever jump or fall you were in the middle of. **If there is nowhere
clear within those eight blocks, you are left where you are and told so** rather
than moved somewhere arbitrary.

**And you are never put outside the world.** The world you play is a fixed square of
ground with an edge, and past that edge there is no ground at all — not empty space
you could stand in, but nowhere. Only somewhere the world actually exists counts as
clear, so near an edge you are moved inward or upward and never out over it. If the
ground that is left is solid too, you get the same answer as anyone else with nowhere
to go: you stay where you are and are told so. Stuck but standing on the world is the
better of the two outcomes — the other one is falling with nothing underneath you.

**Changing a block's *texture* now shows.** Save a new texture key against a block
and the world comes back drawing it — and a block may name a different key on each
of its six faces, so a block's top, bottom and four sides can differ. Changing
whether a block is solid, or adding a block, shows as it did before. The rest of
`docs/modding/hot-reload.md` says which edits are visible today.

**What the face comes back as depends on the key you pointed it at.** Point a face
at a key the art build has baked — anything under "What the world looks like"
above — and the face comes back drawing that picture. Point it at a key nothing
has baked and it comes back drawing a generated stand-in: a flat two-colour
pattern derived from the key's own spelling. Both are ordinary, and which one you
get is decided one key at a time, so re-pointing one face of one block never
changes what anything else is drawing.

**Art baked while the game is running needs a restart, not a reload.** The
pictures are read from disk once, when the game starts. A reload re-points faces
among the pictures that were already read; it does not go looking for new ones. So
if you bake art for a key mid-session, the face keeps its stand-in until you quit
and start again — and a key that was already drawing its picture goes on drawing
it across every reload, rather than dropping back to a stand-in.

## Coming back to a save never leaves you inside a block

The section above is about an edit landing while you watch. This one is about an edit
that landed while you were away, and the two are different events: nobody was playing
when this one happened, and the first you see of it is the world you arrive in.

**Resuming a save now puts you somewhere you can move, even when a block became solid
while the game was off.** Quit while you are standing in water, change that block to
solid in the content files, start the client again, and you begin the session standing
somewhere clear instead of stuck inside rock. The same search the section above
describes decides where: sideways in preference to upward, never downward, up to eight
blocks out, and never anywhere the world does not actually have ground.

**You are told, on the terminal you started the game from**, and the line names where
you ended up:

```
mycraft: you would have entered the world inside solid blocks, so you were moved to (12.5, 10, 12.5)
```

It says *you would have entered* rather than blaming a change, because you were not
there for the change and there is nothing you did to undo. The three numbers are where
your feet are now.

**If nothing within those eight blocks is clear, you are left inside them and told
so** — and the game still starts:

```
mycraft: you would have entered the world inside solid blocks and nothing within 8 blocks is clear, so you were left inside them
```

Starting anyway is deliberate. A launch refused for this would take the save away along
with the edit, and the way out of the situation is to quit, change the block back or
change something near you, and start again — which is only possible if the game runs.

**Starting a fresh world moves you not at all.** A first launch with no save to resume
puts you exactly where the world's own spawn is, says nothing on the terminal, and
looks the way it always has. You only ever read one of the two lines above after a
resume, and only when the world you saved into stopped being somewhere you could stand.

## What this does not cover yet

This is walking, looking, jumping and saving on a fixed, already-generated world. It does not
include an inventory or other players — those are separate, not-yet-built pieces of the game.

**Authored block art has landed, and "What the world looks like" above is what it looks like.**
Grass, dirt and stone draw pictures baked from voxel models under `content/base/models/`
(`docs/modding/voxel-models.md` is how that tooling works), and the game checks that art at every
launch and will not start without it — the section at the top of this page is what that costs you.

What is *not* here is art for everything. A block nobody has baked a picture for draws a generated
stand-in derived from its key's spelling, and that is the ordinary answer rather than a gap being
worked on — so a mod adding a block gets a stand-in until it ships a model. Baking one mid-session
needs a restart before the game reads it, as the section on live edits above says.
