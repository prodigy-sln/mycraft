# Gameplay: Movement and Camera

Player-facing behaviour as built. This describes what a player experiences; see
`docs/technical/architecture.md` §"The player: intent, physics and collision" for how it is
implemented and why.

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
has since changed, the refusal names every affected block and treats two cases differently: a block
that's missing outright — a mod that defined it is gone — can't be resolved at all, and the game
always refuses to start over it. A block the game still recognises by name but that behaves
differently than it did when you saved (solidity, breakability, what it drops) is a call you get to
make: pass `--load-changed-blocks` on the command line to load the world anyway, changed blocks and
all, or leave it off and the game keeps refusing until you either restore the mod or decide to pass
the flag. A block that only *looks* different — a retextured mod, with nothing about how it behaves
changed — never triggers this at all; that world loads normally, without asking.

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
load: it says which save, and why, once rather than twice, and where passing
`--load-changed-blocks` would let you in anyway it says so at the end, after the
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

**One thing to expect that looks like a bug and is not.** Changing a block's
*texture* is read and accepted, and you will not see it: what a block is drawn with
is still chosen by the block's name. Changing whether a block is solid, or adding a
block, does show. The rest of `docs/modding/hot-reload.md` says which edits are
visible today.

## What this does not cover yet

This is walking, looking, jumping and saving on a fixed, already-generated world. It does not
include an inventory or other players — those are separate, not-yet-built pieces of the game.
