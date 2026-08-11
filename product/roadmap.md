# Product Roadmap

Tracked in Linear — team `prodigy-solutions` (`PRO-`), initiative
[**MyCraft**](https://linear.app/prodigy-solutions/initiative/57bd5125-175b-4347-8ab7-b3284f17d744).
Each phase below is a Linear project. This file is the local mirror; Linear is authoritative for
issue status.

Rationale for every locked decision lives in `PLAN.md`; as-built detail lands in `docs/`.

## Overview

Build order is driven by one constraint: **verification capability comes before the thing it
verifies.** The frame-capture harness precedes the renderer, the bot harness precedes multiplayer,
and the adversarial suite precedes public servers. Nothing gets built that cannot be checked
without a human watching a screen.

---

## Current Milestone: Phase 1 — Foundation

**Linear**: [MyCraft Phase 1: Foundation](https://linear.app/prodigy-solutions/project/mycraft-phase-1-foundation)
**Theme**: Make the project buildable, gated, and observable without a human at the screen.

### Features

| Priority | Feature | Status | Spec |
|----------|---------|--------|------|
| P0 | M0 — Workspace, quality gate, **headless frame-capture harness**, bot-client skeleton | Not Started | - |
| P0 | M1 — Chunk storage + palette, binary greedy mesher, wgpu terrain pipeline, fly camera | Not Started | - |
| P1 | M2 — Player physics, raycast, block place/break, singleplayer loop | Not Started | - |

**Exit criteria**: `sdd-gate` green · harness writes inspectable PNGs · mesher benchmark
< 200 µs/section · scripted replay places and breaks 10 000 blocks with asserted world state.

### To start a feature from this roadmap:

```
/sdd-start [feature name from above]
```

---

## Next Milestone: Phase 2 — Scripting & Hot Reload

**Linear**: [MyCraft Phase 2: Scripting & Hot Reload](https://linear.app/prodigy-solutions/project/mycraft-phase-2-scripting-and-hot-reload)
**Theme**: The headline feature. Everything after this is authored in Luau.

- [ ] M3 — Luau host: sandbox, instruction-budget interrupts, memory caps, fault isolation
- [ ] M3 — Definition `Registry` + `ArcSwap` swap at tick boundary
- [ ] M3 — Watch → candidate build → validate → mod self-tests → swap or roll back
- [ ] M3 — Stable string↔numeric ID mapping persisted with the world
- [ ] M3 — `content/base/` blocks defined entirely in Luau

**Exit criteria**: edit a `.luau` file on a running server; behaviour changes with no restart and
no loss of world or player state. A mod that fails to compile never reaches the live world.

---

## Later

### Phase 3 — Multiplayer & Public Servers
[Linear](https://linear.app/prodigy-solutions/project/mycraft-phase-3-multiplayer-and-public-servers)

- [ ] M4 — Client/server split, QUIC transport, protocol
- [ ] M4 — ed25519 identity, challenge-response auth, account store in `redb`
- [ ] M4.5 — Moderation: whitelist/ban, roles and permissions, rate limits, edit journal + region rollback
- [ ] M4.5 — Server-side validation of movement, reach, and inventory transactions
- [ ] M5 — Interest management, delta replication, prediction and reconciliation

**Exit criteria**: 32 authenticated bots, p99 tick < 25 ms, zero desyncs over 10 minutes; the
adversarial suite (speed, reach, edit flood, chat flood, inventory dupe) is fully rejected and
logged with no server impact.

### Phase 4 — Content Systems
[Linear](https://linear.app/prodigy-solutions/project/mycraft-phase-4-content-systems)

- [ ] M6 — Items, inventory, tools, crafting — all defined in `content/base/`
- [ ] M7 — NPCs: voxel pathfinding, coroutine brains, behaviour trees, LOD scheduler
- [ ] M8 — Quests, dialogue, storyline; event bus with Rust-side predicate matching

**Exit criteria**: 500 scripted NPCs inside a 50 ms tick with 32 bots connected; a multi-stage
quest survives both a server restart and a hot reload.

### Phase 5 — Polish & Release
[Linear](https://linear.app/prodigy-solutions/project/mycraft-phase-5-polish-and-release)

- [ ] M9 — Lighting polish, audio, game UI per `standards/global/ui-design.md`
- [ ] M9 — Mod packaging, dependency resolution and load order
- [ ] M9 — World save/load, migration, backup

---

## Backlog

Considered, not scheduled:

- WASM mod backend via `wasmtime` for compute-heavy mods (designed for in `PLAN.md` §4.6)
- Cross-server identity federation (the ed25519 key is already the right primitive)
- Real art assets — placeholders carry us to Phase 4; decide by M6
- Dimensions / portals
- Redstone-equivalent logic system (would be a strong test of the scripting API's expressiveness)

---

## Completed

| Feature | Completed | Spec |
|---------|-----------|------|
| — | — | — |

---

## Notes

- Priorities: P0 = Critical, P1 = High, P2 = Medium, P3 = Low
- Features move to specs via `/sdd-start`; branches are `feature/PRO-123-short-name`
- Completed specs are recorded in `specs/REGISTRY.md` (spec folders are removed at finalize)
- Default rigor is `high` — see `product/mission.md`
