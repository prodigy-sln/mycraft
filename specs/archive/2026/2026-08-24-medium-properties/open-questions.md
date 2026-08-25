# Open Questions — SPEC-022 (PRO-957), architect phase

**Both questions are CLOSED.** The architect phase halted here, the owner
answered in one round, and the phase resumed — `architecture.md` is written. This
file is kept for the **measurement**, which `spec.md`'s amended Out of Scope and
its closed Open Questions both cite. The answers are recorded beside each
question; nothing here is still open.

Every figure below was produced by a command run on 2026-08-24 against an
isolated `git worktree` of `feature/PRO-957-medium-properties` at `8476cab`,
built with `cargo test -p mc-sim` (shared target dir, source tree exclusive to
this measurement). The worktree has been removed; the harness is preserved at
`C:\Users\sgrun\AppData\Local\Temp\claude\E---PROJEKTE-MyCraft\7829ccf7-a637-47c2-99af-92c0ca351032\scratchpad\oq1_water_overlap.rs`
and `…\oq1_confirm.rs`. Anything marked *derived* is arithmetic over a measured
figure and says so.

---

## Q1 — The declared replay walk wades through the sea for half its length. **CLOSED**

> **Answered 2026-08-24.** The re-shoot is in scope. `SCENE_REVISION` bumps to
> `r3` and **all four** golden directories are re-shot, including the two whose
> images reproduce byte-identically. The deliverable is the **corrected revision
> rule**, not the re-shoot — `architecture.md` Decision 6. Still forbidden:
> moving the declared spawn, moving `SEA_LEVEL`, adding a second declared scene,
> or adding a camera-path tripwire (`spec.md` Notes, deferred).

`spec.md`'s Open Questions and the phase brief both say: *if the scripted
player's box overlaps a swimmable voxel at any of the 120 ticks, stop and
escalate; do not re-shoot, do not adjust the walk, do not widen Out of Scope.*

**It does. At 60 of the 120 ticks, including two of the three declared golden
capture ticks.**

### The measurement

Against the **real simulation** — `simulation_for(ReplayWorld::generate(REPLAY_SEED, …))`
seated at the declared spawn, advanced under `scripted_intent` tick by tick, the
player state read off `simulation.latest().player` (not the camera, so no
`EYE_HEIGHT` subtraction stands between the reading and the box). Overlap is
judged by the half-open `[v, v+1)` rule written out independently, the same rule
`crates/mc-sim/tests/support/overlap.rs` re-derives for the replay's overlap
oracle: `floor(min) ..= ceil(max) − 1` per axis, box `[feet ± 0.3, feet + 1.8)`.

| | Result |
|---|---|
| Water voxels in the declared world | **178** — reproduces `requirements.md` §7 and PRO-904's census exactly, so that figure is confirmed rather than relayed |
| Ticks of `0..=119` whose box overlaps `base:water` | **60** — ticks **44–99** and **116–119** |
| First wet tick | **44** |
| Declared capture ticks (`DECLARED_CAPTURE_TICKS = [0, 59, 119]`) that are wet | **59 and 119**. Tick 0 is dry |
| Lowest feet `y` over the whole script | **34.0000** |
| Nearest water voxel centre to the box centre, over all ticks | **0.43 blocks**, voxel `(61, 34, 32)` |

Per-tick, with every voxel the box touches named:

| tick | feet | on ground | box voxels |
|---|---|---|---|
| 0 | `(63.5000, 37.0000, 35.5000)` | false | `(63,37,35)` nothing, `(63,38,35)` nothing |
| 43 | `(62.8733, 35.0000, 34.7531)` | — | eight voxels, all nothing — **the last dry tick** |
| 44 | `(62.8251, 34.9917, 34.6957)` | — | `(62,34,34)` **base:water**, `(63,34,34)` **base:water**, four nothing |
| 59 | `(62.3000, 34.0000, 33.8339)` | true | `(62,34,33)` **base:water**, `(62,34,34)` **base:water**, two nothing |
| 90 | `(61.4709, 34.0000, 31.7523)` | — | `(61,34,31)` **base:water**, `(61,34,32)` **base:water**, two nothing |
| 116 | `(61.1323, 34.9750, 29.8319)` | — | four **base:water** at `y = 34`, eight nothing |
| 119 | `(61.0932, 34.7250, 29.6103)` | — | `(60,34,29)` **base:water**, `(61,34,29)` **base:water**, four nothing |

### What that means, precisely

**The player wades; it is never submerged.** The water it meets is one voxel
layer deep — `y = 34` only, occupying `[34, 35)` — and the lowest the feet ever
go is exactly `34.0000`, standing on the lakebed. *Derived*: the eye sits at
`feet + EYE_HEIGHT = 1.62`, so it is above `35.0` at every wet tick and the
camera is never underwater. Nothing about how the *scene* is drawn changes.

**But the walk is slowed from tick 44, and that moves the camera.** FR-6.1-S1
requires `base:water` to declare `move_resistance` greater than zero, and FR-4.1
makes that a divisor `1 / (1 + r)` on the velocity a tick uses **and carries
forward**. So the player's position at ticks 59 and 119 is not the position it
holds today, whatever value water declares, and whatever the architecture
decides about items 1 and 2 — those decisions cannot avoid this, which is why
this is not a design question.

*Derived*, for magnitude: measured, the player travels from `(62.83, 34.70)` at
tick 44 to `(61.09, 29.61)` at tick 119 — about **5.38 blocks** over those 76
ticks. Under `1 / (1 + r)` that shrinks toward `5.38 / (1 + r)`, so at `r = 4`
the player ends roughly **4.3 blocks short** of where it stands today. This is
blocks, not pixels, and no tolerance absorbs it.

### What moves, and what does not

Measured by reading the tree:

**Moves** — everything stated against the published player camera at ticks 59
and 119:

- `crates/mc-render/goldens/player-walk-t059-r2/` and `…/player-walk-t119-r2/`
  (two of the four golden directories).
- `crates/mc-client/tests/replay_oracle.rs` — `JUDGED_TICKS = [0, 59, 119]`.
- `crates/mc-client/tests/replay_determinism.rs` — `SAMPLED_TICKS = [0, 59, 119]`.
- `crates/mc-client/tests/the_sea_the_camera_sees_is_the_water_layer.rs`.
- PRO-904's recorded water sample counts **56 / 200 / 111** across the three
  declared ticks — the second and third are stated against poses that move.

**Does not move:**

- `player-walk-t000-r2` and `player-walk-hud-t000-r2`. Tick 0 is dry, and
  `HUD_CAPTURE_TICKS = [0]`, so the HUD golden is untouched.
- **The scene contract.** The world is unchanged, so `SCENE_QUAD_COUNT`,
  `total_face_area` and `area_by_block` are properties of a mesh nothing here
  touches. This is the distinction PRO-904's architecture drew and it holds:
  moving the camera moves the goldens, moving the world moves the contract.
  Only the camera moves.
- **FR-6.1-S6's fixture premise.** Column `(63, 35)` has surface height 34 and
  is dry — *derived* from the measured tick-0 feet at `y = 37.0`, which is
  `surface + SPAWN_ABOVE_SURFACE = surface + 3`. "A player standing on the shore
  at column (63, 35)" is still a real pose.
- The spawn itself. Ticks 0 through 43 are dry, so the fall and the first half
  second are exactly as PRO-904 shot them.

### The question, as it was asked

**`Out of Scope` said this spec re-shoots no golden frame, and it must now
re-shoot two.** No option was recommended here and none was designed against.
The facts the decision rested on:

- The spec's `Out of Scope` bullet reads *"PRO-904 settled how water is drawn
  and this spec re-shoots no golden frame."* Two of four move regardless of any
  design choice available to this spec.
- PRO-904 shot these goldens at revision `r2` (`SCENE_REVISION = "r2"`,
  `crates/mc-render/src/capture.rs:32`), and its architecture recorded the
  re-shoot procedure and its one known corrupting failure mode.
- The alternatives PRO-904 weighed for a related problem — moving the declared
  spawn, moving `SEA_LEVEL`, adding a second declared scene — each carry costs
  that document measured, and at least one of them (`SEA_LEVEL`) would move the
  scene contract, which nothing in this spec otherwise touches.

The decision landed on the third of those: the mesh contract is untouched, so
the tripwire that is supposed to name `SCENE_REVISION` cannot fire, and that
stale guarantee — not the two moved images — is what `architecture.md` repairs.

---

## Q2 — May the architect prototype the integration point to re-derive water's `move_resistance` window? **CLOSED**

> **Answered 2026-08-24.** Approved as a documented test-first exception
> (`spec.md`, Test-first exceptions), under four conditions: a throwaway worktree
> discarded with nothing reaching the branch; `architecture.md` records the
> window and the method and **never a value**; the exception is documented in the
> spec; and a disagreement with the model is escalated rather than silently
> corrected. **The spike ran, the worktree was discarded, and the model was wrong
> on both bounds** — see `architecture.md` Decision 5 and Risks.

`spec.md`'s Technical Considerations and the brief both bind: the admissible
window for water's resistance (`r ≲ 16` from FR-6.1-S2, `r ≲ 4` from FR-6.1-S4,
the sink binding) is **arithmetic over a model**, it is a screen in the sense
`standards/global/testing.md` §2 means, and it must be re-derived against the
**built simulation** — *"If the built simulation disagrees with the window
above, that is a finding to escalate rather than a number to adjust."*

**That instrument does not exist at architect time.** The divisor and the
swim-jump are what this phase is deciding where to put; there is no built
simulation to measure against until they are implemented. So the re-derivation
is reachable in exactly two ways:

1. **The architect spikes it** — apply the candidate integration to `physics.rs`
   in an isolated worktree, sweep `r` over the shipped sea with the real
   `advance_player`, record the window, throw the spike away. `testing.md` §1
   admits exploratory spikes explicitly. This produces a measured window before
   `tasks.md` is written, so a disagreement with the model escalates while the
   spec is still open rather than mid-implementation.
2. **The implementer derives it** at the phase that lands the integration, with
   the architecture recording the model's window as a screen and the obligation
   to replace it.

The difference is *when* a disagreement between the model and the simulation
surfaces. Option 1 was chosen, and the disagreement it surfaced would otherwise
have reopened a closed spec mid-implementation.

*No value is proposed here and none will be tuned.* Whichever way this is
answered, the value is derived and never adjusted until FR-6.1-S2 and FR-6.1-S4
go green.

---

## Not blocked — recorded at the time so the 256 KiB budget was not mistaken for a second escalation

Architecture Delta items 1 and 2 had no unknown in them and no question here.
The generalisation and the mechanism are `architecture.md` Decision 1; what
follows is what was known when this file was written. The shipped world holds
four block declarations, of which exactly
one (`base:water`) will state either medium field, so the number of distinct
*(swimmable, move_resistance)* answers a voxel of that world can carry is two.
*Derived*: at 64 × 64 × 256 = 1 048 576 voxels, one bit per voxel is 131 072
bytes = **128 KiB**, half the stated budget, for both questions together. The
budget is met with room. The mechanism, its generalisation beyond two distinct
answers, and the point at which it breaks are `architecture.md` Decision 1.
