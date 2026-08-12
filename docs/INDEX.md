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
├── user/       (planned) ← players and server operators
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
| architecture.md | Crate boundaries and dependency direction established by the block registry and chunk storage work | SPEC-002 |
| decisions.md | Architecture decision records — stack choices and their rejected alternatives | PLAN.md, SPEC-002 |
| rendering.md | The section mesher's quad, determinism and error contracts, and the draw-target conventions established ahead of `mc-render`: orientation and capture pixel-format contract | SPEC-001, SPEC-003 |
| testing.md | Quality gate stages, verification harnesses, and what automation cannot check | PLAN.md, sdd-init-project, SPEC-001, SPEC-002, SPEC-003 |
| world-format.md | Chunk section and column storage, the block palette, and block-identity stability across a registry change | SPEC-002, SPEC-003 |

### modding/

| File | Purpose | Sources |
|------|---------|---------|
| blocks-items.md | Block authoring contract: file layout, required fields, namespaced-id rule, failure reporting | SPEC-002 |

### user/ · ops/

Not yet written. These branches are created by the specs that produce the behaviour they document —
see the Routing Guide for where each topic lands.

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
| Player-facing behaviour and controls | `user/gameplay.md` |
| Server install, config, tuning | `ops/deployment.md` |
| Auth, identity, accounts, permissions | `ops/administration.md` (operator view) + `technical/protocol.md` (mechanism) |
| Anti-abuse, rollback, incident response | `ops/moderation.md` |
| Asset generation tooling, provider usage, cost control | `technical/asset-pipeline.md` (see ADR-009) |
| Quality gate stages, thresholds, verification harnesses | `technical/testing.md` |
