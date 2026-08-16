# Documentation Index

Registry and routing guide for the living documentation. `docs/` describes
the system **as built** — never planned or future behavior. Future concepts
live in active specs and `product/roadmap.md`.

**One deliberate exemption:** `docs/planning/` holds design discussions for
features still in the planning state (see `planning/README.md`). Nothing in it
describes built behaviour, nothing in it is binding, and a spec that lands
supersedes it. It exists so that deep pre-spec analysis has a home in the tree
instead of living only in an issue tracker.

Consolidation updates this file: add the source identifier to the Sources
column of every updated file, and register newly created files.

## Structure

```
docs/
├── INDEX.md              ← you are here
├── technical/            ← engine contributors: architecture, protocol, formats
├── modding/              ← content authors: block/item/... authoring and (eventually) the Luau API surface
├── user/                 ← players — currently just movement and controls; server operators still planned
├── planning/             ← pre-spec design discussions; exempt from the as-built rule (see planning/README.md)
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
| architecture.md | Crate boundaries and dependency direction: the registry/loader seam, the simulation/renderer snapshot seam, the player's intent/physics/collision model, the client input dispatch (`Session`, the drivable core, and what stays unreachable), the editable world (break/place, the store/collision mirror, the raycast reach bound, the refusal vocabulary, the three-armed reads that keep "no such cell" and "empty cell" apart, and the whole-world mesh that reads without marking anything dirty), the remesh transport that makes an edit visible, saving and loading a world (the `mc_world::persistence` module and its encoder confinement, how the save path reaches the blocks through `mc-sim`, the two preparation entry points and which one the resume decision sits inside, and `mc-client`'s wiring-only role), the HUD across three crates (the `HudElementSource` port, its own fault type, the format-agnostic raw declaration, the single composition entry point with the zero-HUD path preserved beside it, and the debug overlay's clock port, non-scriptability and reopening condition), the pure/GPU feature seam inside `mc-render`, and the invariants asserted mechanically rather than by convention (including the one-way `tools/ → crates/` dependency walk), and the scripting host's seams (the backend confined to one adapter directory, what is Rust-side and keyed by the attachment versus what lives behind that adapter, the load-bearing construction order, the one latch over two limits, the never-re-entrant dispatch queue drained across rounds, and the guards that keep the client's closure free of the host) | SPEC-002, SPEC-004, SPEC-005, SPEC-006, SPEC-007, SPEC-008, SPEC-009, SPEC-010, SPEC-012, SPEC-013, SPEC-014 |
| decisions.md | Architecture decision records — stack choices and their rejected alternatives | PLAN.md, SPEC-002, SPEC-004, SPEC-005, SPEC-007, SPEC-009, SPEC-010, SPEC-013, SPEC-014 |
| licensing.md | The shipped `LICENSE-MIT` and `LICENSE-APACHE`: their hashes and byte counts, the provenance chain a replacement must re-establish (and the ecosystem variant that would fail a check), why the Apache appendix stays unfilled and why that makes a shared placeholder detector report a byte-perfect file, where the copyright holder is written and that nothing derives from it, how much SPDX the drift check understands and every construct it refuses by name, the three vacuity guards, where the check lives and the three constraints that put it there, and what deliberately does not exist (`NOTICE`, `license-file`, per-crate texts, general link validation) | SPEC-011 |
| rendering.md | The section mesher's quad, determinism and error contracts (including the empty cell answered before the registry); the terrain draw path as built (packing, array texture, compute culling, single indirect draw, depth, pass configuration); texture layers and the registry-derived key set no world can move; the HUD pass (the pinned rectangle derivation, the outline ring composed before fills, alpha compositing in linear space, the borrowed array texture, the rectangle ceiling, and the derived prediction whose duplication is the oracle); the draw-target conventions (orientation, capture pixel format); the procedure for re-shooting a committed golden set; what golden-frame and probe verification cannot see, measured against the player's own camera and against the HUD's per-pixel prediction; and the excavated-world capacity gap against the scene's quad budget | SPEC-001, SPEC-003, SPEC-004, SPEC-005, SPEC-007, SPEC-008, SPEC-010, SPEC-012 |
| testing.md | Quality gate stages (why its result may never be read through a pipe, why a zero exit code is not sufficient, how its two file-length measures disagree, why a phase whose tree does not compile at its start has no gate evidence at its start, and the size stage's per-root file count across `crates/` and `tools/`), verification harnesses, derived probes and the scene contract, the derived-oracle pattern repeated at fixture and world scale (including a tileability verdict judged against a document's own declaration and against itself, never against a second render), the headless client-input harness and its mutation-count discipline, the break/place mutation table and its fixture-geometry and refusal-by-name lessons, why a requirement that nothing changes yields only guards, why a falsifier list must be measured rather than derived from call paths, why a diff-derived review manifest omits the riskiest files, the honesty conditions behind the 10 000-block exit criterion, persistence's second-entry-point rule and the `sync_all()` flush no test can catch, the three rules — a texture key's field, a section lookup's key, and a launch that generates nothing it does not need — that a reader holds because no fixture can, why nothing in the workspace runs `App` and why coverage cannot say so, why an instrument must be confirmed before a product is reported dead, why a test asserting the correct value can still be a defect and why a mutation claim must say how much it replaced, and what automation cannot check (including the manual acceptance checks no harness can drive, the closed-set properties nothing grades, and the human sign-off on VoxForge's preview loop — reliable for proportion and orientation, not for structure), the hostile-mod harness (its six named cases, the containment evidence each declares, the three-way verdict, and the text guard that stops it stating any policy of its own), where a missing mechanism wedges instead of failing and the exact-name overrides that bound it, the scripting host's mutation tables and the skeleton pairs beside them, the three lints denied at a crate root and the test target that denial does not reach, and what the sandbox verification does not establish | PLAN.md, sdd-init-project, SPEC-001, SPEC-002, SPEC-003, SPEC-004, SPEC-005, SPEC-006, SPEC-007, SPEC-008, SPEC-009, SPEC-010, SPEC-011, SPEC-012, SPEC-013, SPEC-014 |
| world-format.md | What a cell holds — a block or nothing — and the named two-variant type that keeps that apart from "there is no such cell"; chunk section and column storage; the block palette (including its bound under repeated edits); a section's storable identity as the persistence layer inherits it; the world's `WorldPos`-addressed column footprint; block-identity stability across a registry change; and the on-disk save format — the hand-read preamble, the two-value encoding, atomic replacement, the save-table identifier's width, the allocation-bounding security properties, the three-outcome load decision, and the determinism of a save's stored world data | SPEC-002, SPEC-003, SPEC-007, SPEC-008, SPEC-009 |

### modding/

| File | Purpose | Sources |
|------|---------|---------|
| blocks-items.md | Block authoring contract: file layout, required and optional fields (including `replaceable`, `breakable`, `breaks_into`), namespaced-id rule, failure reporting, and why there is no empty block to declare | SPEC-002, SPEC-007, SPEC-008 |
| hud.md | HUD authoring contract: file layout, every field with its requirement and mutual-exclusion rules, the nine anchors and the safe-area inset, UI units against the 720-pixel reference height, eight-digit colours and linear-space alpha, the outline convention and its two-pass ordering, the published draw kinds and readable values, why a root declaring no HUD is valid where one declaring no blocks is not, what content categorically cannot do, and why the debug overlay is unreachable from content | SPEC-010 |
| sandbox.md | The environment a script runs in, **not** an authoring API — nothing can be authored in Luau yet, and the page says so up front. What a script may reach and what stops it: the attachment a callback is registered under, the exact reachable and denied global sets and why the interesting denials are denials, the frozen per-chunk environment, every shipped limit with its default, why the budget counts calls and loop edges rather than instructions and what that means for sizing a workload, the two authoring rules that pull against each other (a cursor in a closure upvalue, and an abort no `pcall` can recover from), what a fault carries back to its author, quarantine and how it is lifted, and why a fault raised under host memory pressure names nobody | SPEC-014 |
| voxel-models.md | VoxForge (`tools/voxforge`) — the `.mcvox` authoring contract: file layout, the implicit-single-part and explicit `[[parts]]` document forms, the per-model/per-part `slice` orientation table, palette and namespaced material files, part attachment and states, the 64-per-axis size bound and (0,0,0)-normalised assembly, `inspect`'s stats/defect/observation partition and why connectivity is not a structural-soundness check, `preview`'s fourteen canonical views and contact sheet, `texture`'s flat-shaded faces and three-leg seam verdict, and the refusal contract | SPEC-013 |

### user/

| File | Purpose | Sources |
|------|---------|---------|
| gameplay.md | Player-facing controls and movement feel: WASD walking, mouse look, jump, cursor capture; what is on screen (crosshair, held-block swatch) and the F3 debug overlay; saving on quit and resuming on relaunch — drawn as saved from the first frame, not only walkable as saved — what a player sees and can do when a save can't be loaded outright, and that the scripting host is machinery a player never meets, nothing on screen being scripted | SPEC-005, SPEC-009, SPEC-010, SPEC-012, SPEC-014 |

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
| Sandbox limits, call-and-loop budgets, memory caps, fault isolation | `modding/sandbox.md` + `technical/architecture.md` |
| Hot-reload semantics and state migration | `modding/hot-reload.md` |
| NPC authoring | `modding/npcs.md` |
| Quests, dialogue, storyline authoring | `modding/quests.md` |
| Blocks, items, tools, crafting authoring | `modding/blocks-items.md` |
| Voxel model authoring (`.mcvox`), VoxForge's preview/inspect/texture CLI | `modding/voxel-models.md` |
| HUD element declarations, screen-space composition, readable values | `modding/hud.md` + `technical/rendering.md` (the pass) + `technical/architecture.md` (the seams) |
| Player-facing behaviour and controls | `user/gameplay.md` |
| Server install, config, tuning | `ops/deployment.md` |
| Auth, identity, accounts, permissions | `ops/administration.md` (operator view) + `technical/protocol.md` (mechanism) |
| Anti-abuse, rollback, incident response | `ops/moderation.md` |
| Asset generation tooling, provider usage, cost control | `technical/asset-pipeline.md` (see ADR-009) |
| Quality gate stages, thresholds, verification harnesses | `technical/testing.md` |
| Licence texts, the SPDX declaration, copyright holder, licence-file drift checks | `technical/licensing.md` (+ `technical/testing.md` for what grading an unalterable document costs) |
