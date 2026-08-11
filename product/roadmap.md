# Product Roadmap

Tracked in Linear — team `prodigy-solutions` (`PRO-`), initiative
[**MyCraft**](https://linear.app/prodigy-solutions/initiative/57bd5125-175b-4347-8ab7-b3284f17d744).
Each MVP below is a Linear project; each spec is one flat `PRO-` issue inside it. Linear is
authoritative for issue status.

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

| Priority | Feature | Status | Spec |
|----------|---------|--------|------|
| P0 | Workspace, quality gate, **headless frame-capture harness** | Not Started | - |
| P0 | Chunk storage + palette, binary greedy mesher, wgpu terrain pipeline | Not Started | - |
| P0 | Camera, player physics, collision | Not Started | - |
| P0 | Raycast targeting, block break and place | Not Started | - |
| P1 | World persistence — quit and resume where you left off | Not Started | - |
| P1 | Minimal HUD: crosshair, held block, debug overlay | Not Started | - |

**Exit criteria** — all must hold:
- `sdd-gate.ps1` exits 0
- Mesher benchmark < 200 µs/section
- Scripted replay places and breaks 10 000 blocks with asserted world state
- **You can launch the client, walk around, break and place blocks, quit, relaunch, and your
  changes are still there**

The frame-capture harness is first and non-negotiable: without it the agent building the renderer
cannot see its own output.

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

---

## Completed

| Feature | Completed | Spec |
|---------|-----------|------|
| — | — | — |

---

## Notes

- Priorities: P0 = Critical, P1 = High, P2 = Medium, P3 = Low
- Features move to specs via `/sdd-start`; branches are `feature/PRO-123-short-name`
- Completed specs are recorded in `specs/REGISTRY.md`
- Default rigor is `high` — see `product/mission.md`
- No pull requests; `origin` is a backup remote. See `standards/global/git-workflow.md`.
- `/sdd-complete` runs after every spec — that is what keeps `docs/` as-built.
