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

**Nothing you can see is scripted, and nothing in the game reaches the scripting host.** The engine
now carries one — a sandboxed place for mod code to run, with limits on how long it may run and how
much memory it may use — but it is machinery only: no block, no behaviour and no content anywhere in
the game is defined in script. The blocks you break and place, and the crosshair and swatch on your
screen, are declared in the game's own content data files exactly as they were before, and the
running game never calls into the scripting host at all. So nothing about how the game plays,
launches, saves or looks changed with it, and there is nothing here for a player to turn on, try or
configure.

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

## What this does not cover yet

This is walking, looking, jumping and saving on a fixed, already-generated world. It does not
include an inventory or other players — those are separate, not-yet-built pieces of the game.
