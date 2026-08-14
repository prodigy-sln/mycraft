# Documentation Index

Registry and routing guide for the living documentation. `docs/` describes
the system **as built** — never planned or future behavior. Future concepts
live in active specs and `product/roadmap.md`.

Consolidation updates this file: add the source identifier to the Sources
column of every updated file, and register newly created files.

## Structure

```
docs/
├── INDEX.md              ← you are here
├── technical/            ← engine contributors: architecture, protocol, formats
├── modding/              ← content authors: block/item/... authoring and (eventually) the Luau API surface
├── user/                 ← players — currently just movement and controls; server operators still planned
└── ops/        (planned) ← running a public server: deploy, backup, moderation
```

Branches marked *planned* have no directory yet. They are created by the spec that first produces
documentable behaviour for them, not in advance.

`modding/` is the project's most externally-visible contract. Because the base game is itself a
mod, anything in `crates/mc-script`'s public binding surface belongs here — and a change that
breaks it is a breaking change for every server in existence.

## File Registry

Only files that exist are listed. A file appears here when it is written, not when it is planned —
the Routing Guide below is where future destinations are declared.

### technical/

| File | Purpose | Sources |
|------|---------|---------|
| architecture.md | Crate boundaries and dependency direction: the registry/loader seam, the simulation/renderer snapshot seam, the player's intent/physics/collision model, the client input dispatch (`Session`, the drivable core, and what stays unreachable), the editable world (break/place, the store/collision mirror, the raycast reach bound, the refusal vocabulary, and the three-armed reads that keep "no such cell" and "empty cell" apart), the remesh transport that makes an edit visible, saving and loading a world (the `mc_world::persistence` module and its encoder confinement, how the save path reaches the blocks through `mc-sim`, the resume decision's placement above scene preparation, and `mc-client`'s wiring-only role), the HUD across three crates (the `HudElementSource` port, its own fault type, the format-agnostic raw declaration, the single composition entry point with the zero-HUD path preserved beside it, and the debug overlay's clock port, non-scriptability and reopening condition), the pure/GPU feature seam inside `mc-render`, and the invariants asserted mechanically rather than by convention | SPEC-002, SPEC-004, SPEC-005, SPEC-006, SPEC-007, SPEC-008, SPEC-009, SPEC-010 |
| decisions.md | Architecture decision records — stack choices and their rejected alternatives | PLAN.md, SPEC-002, SPEC-004, SPEC-005, SPEC-007, SPEC-009, SPEC-010 |
| rendering.md | The section mesher's quad, determinism and error contracts (including the empty cell answered before the registry); the terrain draw path as built (packing, array texture, compute culling, single indirect draw, depth, pass configuration); the HUD pass (the pinned rectangle derivation, the outline ring composed before fills, alpha compositing in linear space, the borrowed array texture, the rectangle ceiling, and the derived prediction whose duplication is the oracle); the draw-target conventions (orientation, capture pixel format); the procedure for re-shooting a committed golden set; what golden-frame and probe verification cannot see, measured against the player's own camera and against the HUD's per-pixel prediction; and the excavated-world capacity gap against the scene's quad budget | SPEC-001, SPEC-003, SPEC-004, SPEC-005, SPEC-007, SPEC-008, SPEC-010 |
| testing.md | Quality gate stages (why its result may never be read through a pipe, why a zero exit code is not sufficient, and how its two file-length measures disagree), verification harnesses, derived probes and the scene contract, the derived-oracle pattern repeated at fixture and world scale, the headless client-input harness and its mutation-count discipline, the break/place mutation table and its fixture-geometry and refusal-by-name lessons, why a falsifier list must be measured rather than derived from call paths, why a diff-derived review manifest omits the riskiest files, the honesty conditions behind the 10 000-block exit criterion, persistence's second-entry-point rule and the `sync_all()` flush no test can catch, why nothing in the workspace runs `App` and why coverage cannot say so, why an instrument must be confirmed before a product is reported dead, and what automation cannot check (including the manual acceptance checks no harness can drive and the closed-set properties nothing grades) | PLAN.md, sdd-init-project, SPEC-001, SPEC-002, SPEC-003, SPEC-004, SPEC-005, SPEC-006, SPEC-007, SPEC-008, SPEC-009, SPEC-010 |
| world-format.md | What a cell holds — a block or nothing — and the named two-variant type that keeps that apart from "there is no such cell"; chunk section and column storage; the block palette (including its bound under repeated edits); a section's storable identity as the persistence layer inherits it; the world's `WorldPos`-addressed column footprint; block-identity stability across a registry change; and the on-disk save format — the hand-read preamble, the two-value encoding, atomic replacement, the save-table identifier's width, the allocation-bounding security properties, the three-outcome load decision, and the determinism of a save's stored world data | SPEC-002, SPEC-003, SPEC-007, SPEC-008, SPEC-009 |

### modding/

| File | Purpose | Sources |
|------|---------|---------|
| blocks-items.md | Block authoring contract: file layout, required and optional fields (including `replaceable`, `breakable`, `breaks_into`), namespaced-id rule, failure reporting, and why there is no empty block to declare | SPEC-002, SPEC-007, SPEC-008 |
| hud.md | HUD authoring contract: file layout, every field with its requirement and mutual-exclusion rules, the nine anchors and the safe-area inset, UI units against the 720-pixel reference height, eight-digit colours and linear-space alpha, the outline convention and its two-pass ordering, the published draw kinds and readable values, why a root declaring no HUD is valid where one declaring no blocks is not, what content categorically cannot do, and why the debug overlay is unreachable from content | SPEC-010 |

### user/

| File | Purpose | Sources |
|------|---------|---------|
| gameplay.md | Player-facing controls and movement feel: WASD walking, mouse look, jump, cursor capture; what is on screen (crosshair, held-block swatch) and the F3 debug overlay; saving on quit and resuming on relaunch, and what a player sees and can do when a save can't be loaded outright | SPEC-005, SPEC-009, SPEC-010 |

### ops/

Not yet written. This branch is created by the spec that produces the behaviour it documents — see
the Routing Guide for where each topic lands.

## Routing Guide

When consolidating source material, update the files mapped to its topics:

| Source material about... | Update |
|--------------------------|--------|
| Architecture decisions, stack choices, rejected alternatives | `technical/decisions.md` |
| Crate boundaries, threading, tick model | `technical/architecture.md` |
| Chunk storage, palette, persistence, block-ID stability | `technical/world-format.md` |
| Packets, replication, interest management, QUIC usage | `technical/protocol.md` |
| Meshing, draw path, lighting, texture handling | `technical/rendering.md` |
| Test harnesses, golden frames, load rigs, benchmarks | `technical/testing.md` |
| Any `mycraft.*` Luau binding | `modding/api-reference.md` + the relevant topic file |
| Sandbox limits, instruction budgets, fault isolation | `modding/sandbox.md` + `technical/architecture.md` |
| Hot-reload semantics and state migration | `modding/hot-reload.md` |
| NPC authoring | `modding/npcs.md` |
| Quests, dialogue, storyline authoring | `modding/quests.md` |
| Blocks, items, tools, crafting authoring | `modding/blocks-items.md` |
| HUD element declarations, screen-space composition, readable values | `modding/hud.md` + `technical/rendering.md` (the pass) + `technical/architecture.md` (the seams) |
| Player-facing behaviour and controls | `user/gameplay.md` |
| Server install, config, tuning | `ops/deployment.md` |
| Auth, identity, accounts, permissions | `ops/administration.md` (operator view) + `technical/protocol.md` (mechanism) |
| Anti-abuse, rollback, incident response | `ops/moderation.md` |
| Asset generation tooling, provider usage, cost control | `technical/asset-pipeline.md` (see ADR-009) |
| Quality gate stages, thresholds, verification harnesses | `technical/testing.md` |
