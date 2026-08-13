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
| Escape | Release the cursor |
| Click in the window | Re-capture the cursor |

The walk keys are bound by **physical position**, not by the letter printed on them, so the same four
keys under your left hand walk the player on a QWERTZ or AZERTY keyboard as on a QWERTY one.

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

## What this does not cover yet

This is walking, looking and jumping on a fixed, already-generated world. It does not include
breaking or placing blocks, an inventory, other players, or anything that survives past the current
session — those are separate, not-yet-built pieces of the game.
