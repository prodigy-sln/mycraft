# Product Roadmap

Tracked in Linear — team `prodigy-solutions` (`PRO-`), initiative
[**MyCraft**](https://linear.app/prodigy-solutions/initiative/57bd5125-175b-4347-8ab7-b3284f17d744).
Each MVP below is a Linear project; each spec is one flat `PRO-` issue inside it. Linear is
authoritative for issue status.

> **This file drives the autonomous build.** The conductor reads the MVP marked
> `← current`, its feature table, and its **Exit criteria** section to determine
> scope and what "done" means. Exactly one MVP must carry the marker, and it must
> have exit criteria — otherwise the conductor stops and asks rather than
> guessing. When promoting the next MVP, move the marker *and* write its exit
> criteria before starting a run.

## How this roadmap works

**Incremental, not waterfall.** Every MVP ends with a *playable game* — not a finished layer.
MVP 1 is a game you can start, walk around in, and build in. Each later increment adds capability
to a thing that already runs. There is no point on this roadmap where the project is "half a
renderer and no gameplay".

Two rules follow from that:

1. **An increment is not done until you can play it.** A green gate is necessary, not sufficient.
   Every MVP's exit criterion includes a human-playable build.
2. **Depth of planning decreases with distance.** MVP 1 is specified; later MVPs are a sketch and
   are expected to change once you have played the earlier ones. Planning MVP 5 in detail now
   would be the waterfall this structure exists to avoid.

Quality is *not* traded away for increment size. The gate, TDD discipline, and rigor tier apply
identically to a small MVP — small means narrow scope, never lower standards.

---

## MVP 1 — Playable Sandbox ← current

**Linear**: [MyCraft MVP 1: Playable Sandbox](https://linear.app/prodigy-solutions/project/mycraft-phase-1-foundation)
**Goal**: A game that starts, renders a world, and lets you move and build in it.

Decomposed into ten specs, in dependency order. Each moves the playable thing
forward; none is a pure layer.

| Priority | Feature | Issue | Status |
|----------|---------|-------|--------|
| P0 | Headless frame-capture harness | PRO-849 | **Done** |
| P0 | Chunk storage and block palette | PRO-850 | **Done** |
| P0 | Binary greedy mesher | PRO-851 | **Done** |
| P0 | wgpu terrain pipeline and windowed client — *you can see terrain* | PRO-852 | **Done** |
| P0 | Camera, player physics and collision — *you can walk around* | PRO-853 | **Done** |
| P0 | Headless client-input harness — *prerequisite of PRO-854* | PRO-873 | **Done** |
| P0 | Raycast targeting, block break and place — *you can build* | PRO-854 | **Done** |
| P0 | A cell holds a block or nothing — *air is not a block* | PRO-876 | **Done** |
| P1 | World persistence — *quit and resume* | PRO-855 | **Done** |
| P1 | Minimal HUD: crosshair, held block, debug overlay — *the HUD is content* | PRO-856 | **Done** |
| P2 | Ship the LICENSE texts the workspace declares | PRO-874 | **Done** |
| P2 | Terrain texture sampling and palette separation | PRO-869 | Todo |

**PRO-873 goes before PRO-854, and the ordering is the point.** Every FR-5 scenario of
PRO-853 is verified as policy and none as product behaviour: the winit `ApplicationHandler`
in `crates/mc-client/src/events.rs` needs a real window and nothing constructs one. Two
mutations proved it — submitting an empty intent every tick, and deleting the
`accepts_pointer_motion` gate — each left the whole suite green. PRO-854 adds break and
place to that same unreachable dispatch, so invariant 5 applies directly: verification
precedes the thing it verifies. **Exit criterion: each of those two mutations turns at
least one test red.**

PRO-874 was small and unglamorous — `Cargo.toml` declared `MIT OR Apache-2.0` and the
workspace carried neither text. Both now ship, and a check on every gate run fails if the
declaration and the shipped texts drift apart again (`docs/technical/licensing.md`).

The workspace skeleton and quality gate predate the first spec (commits `f5780f3`,
`a93d2da`), which is why PRO-849 is the capture harness alone.

Two invariants that MVP 1 could plausibly breach, resolved up front:

- **No Luau host until MVP 2, yet invariant 1 still binds.** Blocks are registered
  at runtime from a data file under `content/base/` — never `enum Block` in Rust.
  MVP 2 swaps the loader, not the registry. **The HUD is content on the same
  terms**: the crosshair and held-block indicator are declared under
  `content/base/`, so a mod owns them and there are no HUD definitions in Rust.
  The debug overlay is the deliberate exception and stays engine-owned — it
  inspects the engine, including when content is broken, so a mod must not be
  able to disable the instrument used to diagnose that mod.
- **Singleplayer, yet invariant 4 still binds.** `mc-sim` is authoritative in-process;
  the client submits intents. Otherwise MVP 3 is a rewrite, not a transport swap.

**Exit criteria** — all must hold:
- `sdd-gate.ps1` exits 0
- Mesher benchmark < 200 µs/section — **met** (`cargo bench -p mc-world --bench meshing`,
  terrain ~136 µs; a standalone command, deliberately not a gate stage)
- Scripted replay places and breaks 10 000 blocks with asserted world state
- **You can launch the client, walk around, break and place blocks, quit, relaunch, and your
  changes are still there**

The frame-capture harness went first and was non-negotiable: without it the agent building the
renderer could not have seen its own output. PRO-873 is the same argument applied to input.

Everything is a placeholder — procedural textures, no audio, one block type family. That is
correct for MVP 1. Do not gold-plate it.

---

## Provisional — revised after you play MVP 1

Sketch only. Order and contents are expected to change based on feedback.

| MVP | Adds | Still playable because |
|-----|------|------------------------|
| **2 — Scriptable Content** | Luau host, sandbox, registry, hot reload; `content/base/` blocks defined in script | The blocks you already had become script-defined; you can now add one live |
| **3 — Multiplayer** | QUIC transport, ed25519 identity, replication, prediction; 2 players → 32 | The same sandbox, now with someone else in it |
| **4 — Survival Loop** | Items, inventory, tools, crafting — all script-defined | Building gains purpose and progression |
| **5 — Living World** | NPCs with pathfinding and scripted brains; then quests, dialogue, storyline | The world is inhabited and has things to do |
| **6 — Public & Polished** | Moderation, anti-abuse, rollback; lighting, audio, UI polish, mod packaging | Ready for strangers on a public server |

Backlog, unscheduled: WASM mod backend · cross-server identity · real art assets (decide by
MVP 4) · dimensions and portals · a redstone-equivalent logic system (a strong test of scripting
API expressiveness).

**Art assets are already licensed and available** — see ADR-010 for the two-class handling.
Bundle purchases (music, SFX, GUI, 2.5D sprites) live at `\\ds01\assets\GameAssets`; CC0 material
is [PixVoxelAssets](https://github.com/tommyettinger/PixVoxelAssets) and
`VoxelCoreLab_Watercolor_Terrain_Textures_1024px` (dirt/grass/stone/water, four variants each).
Earliest use is **MVP 2**, where texture-by-key becomes a real feature; the voxel character and
dungeon models suit **MVP 5**, and audio and GUI belong to **MVP 6**. MVP 1 stays on placeholders.

### MVP 3 in more detail — content streaming and home hosting

Still a sketch: this becomes binding only when MVP 3's own spec adopts it. What follows is
the substance that must survive into that spec.

**Content is edited server-side and streams to players — textures included, not just
scripts.** A texture swapped on a running server reaches every connected client live.
Nothing extra is built for it: the hash changes, clients miss, they fetch, they swap. It is
fast because a texture is *data, not structure* — UVs are per-block-type, so no remeshing
and no chunk rebuild, just one array layer overwritten before the next frame. Two things
decide whether it feels instant: the server must downsample before streaming (1024 px
source × 32 players is a download; at the array's layer size it is a few KB), and replacing
a layer is cheap while *adding* one may force the array to be recreated and every bind
group rebuilt.

**A content-addressed client cache, keyed by hash rather than by name.** The client offers
the hashes it holds and the server sends the remainder, which buys the property the whole
scheme rests on: **"changed" and "missing" are the same case.** A changed texture is simply
a different hash, therefore a miss — no version numbers, no invalidation protocol, no
cache-busting, and so no second path to get wrong or forget to trigger. Base content ships
with the client and is therefore already cached; a mod's assets take the identical path and
merely miss the first time. Name-keying loses all of it.

**No CDN at this stage, and the reason is the shape of the problem rather than its size.** A
live swap is under a megabyte in total across every connected player, and QUIC's independent
streams already cover the shape: bulk asset transfer does not head-of-line-block tick
traffic as TCP would, and asset streams can be prioritised below gameplay. Latency here is a
*prioritisation* question; a CDN answers a *volume* question — and for live edits a CDN is
strictly slower, needing upload plus propagation first. The real CDN case is **cold joins
against a large modpack**, a first-join cost only since nobody fetches twice, and that
belongs to MVP 6.

**Home hosting without opening a port.** A lobby/rendezvous service introduces two peers and
performs NAT hole punching, so a player can host from home. The lobby is a **mode of
`mc-server`**, but the constraint that decides its shape is that a rendezvous must be
publicly reachable — which is exactly what a home host is not, since a machine behind NAT
cannot introduce itself. Some instance therefore has to sit on a public address, and that
makes the no-hosted-dependency rule structural rather than a promise: anyone can run their
own lobby and point friends at it. Two constraints on the shape:

- **The lobby role must be implementable from `mc-net` alone and must never instantiate a
  world.** If running a signalling service drags `mc-sim` in, the deployment ships a game
  simulation to introduce two sockets.
- **Rendezvous and relay must be separately switchable**, because their costs differ by
  orders of magnitude. Rendezvous is kilobytes per join and scales with joins; relay carries
  a whole session and scales with players × minutes, at roughly 0.4 GB/hour for a four-player
  session. Offering a cheap public rendezvous must not implicitly sign the operator up to
  relay everyone's traffic.

**Relay assignment is a control-path decision, not a data-path load balancer.** The lobby
answers once at join — "use relay-3 at this address" — and both peers connect *directly* to
it, so the lobby never sees a game packet. A balancer sitting in the traffic would have to
carry the combined packet rate of every relay behind it, making it simultaneously the
bottleneck and the single point of failure, and funnelling all the cheap horizontal scaling
back through one box. What the assigner needs is small: a registry of relay hosts with
heartbeats, capacity and health, plus a pick that **weighs latency to both peers** rather
than picking least-loaded — for a game, an idle relay on the wrong continent beats a busy
nearby one only on paper. The genuinely fiddly part is **a relay dying mid-session**: peers
must notice, re-request an assignment and reconnect without the session ending. Expect the
real work to be there, not in the choosing.

**Two rules that bind:**

1. **A self-hosted server must never require a CDN or any hosted service.** The server
   serves its own content; hosted infrastructure is an optional accelerator in front of
   that, never the only path.
2. **LAN and direct connection must always work.** Port forwarding stays supported and hole
   punching is additive. Otherwise the game dies when a service does, and offline LAN play
   dies with it.

Both are free to honour now and expensive to retrofit the moment something assumes a hosted
URL exists.

**Open, and not yet put to the user — do not assume a reading.** MVP 3's scope has two, and
they differ by real scope:

- *"Multiplayer works"* — direct connection and LAN, with NAT punching deferred to MVP 6
  alongside the public-server work.
- *"Play with friends without touching a router"* — punching lands in MVP 3, and the plan
  changes to match.

Unresolved.

---

## Completed

| Feature | Completed | Spec |
|---------|-----------|------|
| Headless frame-capture harness (PRO-849) | 2026-08-11 | `2026-08-11-frame-capture-harness` |
| Chunk storage and block palette (PRO-850) | 2026-08-12 | `2026-08-11-chunk-storage-palette` |
| Binary greedy mesher (PRO-851) | 2026-08-12 | `2026-08-12-greedy-mesher` |
| wgpu terrain pipeline and windowed client (PRO-852) | 2026-08-12 | `2026-08-12-terrain-render` |
| Camera, player physics and collision (PRO-853) | 2026-08-13 | `2026-08-12-player-movement` |
| Headless client-input harness (PRO-873) | 2026-08-13 | `2026-08-13-client-input-harness` |
| Raycast targeting, block break and place (PRO-854) | 2026-08-13 | `2026-08-13-break-place` |
| A cell holds a block or nothing (PRO-876) | 2026-08-13 | `2026-08-13-empty-cells` |
| World persistence (PRO-855) | 2026-08-14 | `2026-08-13-world-persistence` |
| Minimal HUD — crosshair, held block, debug overlay (PRO-856) | 2026-08-14 | `2026-08-14-minimal-hud` |
| A resumed save is rendered from the first frame (PRO-900) | 2026-08-15 | `2026-08-14-loaded-save-render` |
| Ship the LICENSE texts the workspace declares (PRO-874) | 2026-08-15 | `2026-08-14-license-texts` |

---

## Notes

- Priorities: P0 = Critical, P1 = High, P2 = Medium, P3 = Low
- Features move to specs via `/sdd-start`; branches are `feature/PRO-123-short-name`
- Completed specs are recorded in `specs/REGISTRY.md`
- Default rigor is `high` — see `product/mission.md`
- No pull requests; `origin` is a backup remote. See `standards/global/git-workflow.md`.
- `/sdd-complete` runs after every spec — that is what keeps `docs/` as-built.
