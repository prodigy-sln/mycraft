# MyCraft

A voxel sandbox whose entire game layer — blocks, items, tools, crafting, NPCs, quests and
storyline — is defined in sandboxed Luau scripts that reload live on a running server. It targets
32-player authoritative multiplayer on desktop, and the "vanilla" game is itself a mod with no
privileges a third-party mod lacks.

# Prospect — Spec-Driven Development Framework

This project uses **Prospect** for spec-driven development (SDD) with
test-driven development.

## Workflow

```
/sdd-start [feature]  →  (/sdd-architect)  →  (/sdd-discuss)  →  /sdd-tasks
    →  /sdd-implement  →  /sdd-validate  →  /sdd-complete
```

`/sdd-discuss` runs after the spec and (at high+) the architecture draft
exist, so personas challenge actual binding decisions instead of an early
plan.

Every spec carries a rigor tier in its frontmatter. Each tier strictly adds
to the previous one; escalate whenever new risk appears (record the reason),
downgrade only with explicit user confirmation.

| `rigor:` | Spec | Discuss | Tasks | Implement | Validate |
|----------|------|---------|-------|-----------|----------|
| low | mini-spec | — | — | inline TDD | gate script |
| medium (default) | full + scenarios | — | scenario groups | test author + inline | gate + combined reviewer |
| high | + architecture | — | same | same | gate + reviewer workflow, sign-off |
| xhigh | same | parallel persona reviews | same | same | same |
| max | same | negotiating agent team | same | same | same |

## Key Principles

1. **Specs are the source of truth.** Acceptance scenarios (EARS) are the
   test contract: each scenario becomes exactly one test. The mapping lives
   in the spec folder's `test-map.md` — test names stay behavioral, and
   code never carries spec or scenario IDs.
2. **TDD is non-negotiable.** Failing test output is displayed before any
   implementation. At `medium+`, tests are authored and owned by the test
   author — implementation never edits test files; disputes go to
   arbitration.
3. **`docs/` is as-built reality.** Completed specs consolidate into
   `docs/` via `docs/INDEX.md` routing. Future concepts live in
   `specs/active/` and `product/roadmap.md`, never in `docs/`.
4. **Gates are deterministic.** `scripts/sdd-gate.*` must exit 0 at every
   phase end, before validation, and before completion.
5. **Out of Scope is binding.** Unspecced work is recorded, not built.
6. **Phases resume from disk.** After spec or tasks approval, `/clear` is
   safe and recommended — no phase depends on conversation history.

## Commands

| Command | Purpose |
|---------|---------|
| `/sdd-init-project` | New project: Q&A → structure, standards, gate, docs index |
| `/sdd-onboard` | Existing project: detect toolchain → gate, docs index, UI standard |
| `/sdd-clarify PROJ-123` | Resolve requirement ambiguities via issue tracker |
| `/sdd-start [desc]` | Rigor, branch, shaping, spec, scenario audit, design exploration |
| `/sdd-architect` | Binding architecture plan (high+ default) |
| `/sdd-discuss` | Persona challenge of spec + architecture (xhigh/max default; on demand anywhere) |
| `/sdd-tasks` | Scenario-grouped task breakdown |
| `/sdd-implement` | TDD implementation per the tier's engine |
| `/sdd-validate` | Gate + tier-scaled review |
| `/sdd-complete` | Publish (docs, registry, PR); finalize after approval |
| `/consolidate-docs [path]` | Merge any source material into docs/ |

## Locations

- Active specs: `specs/active/YYYY-MM-DD-name/` (stay active through PR review)
- Registry: `specs/REGISTRY.md` — permanent one-line record per completed spec
- Templates: `specs/_templates/` · Living docs: `docs/` (routed by `INDEX.md`)
- Quality gate: `scripts/sdd-gate.ps1` (PowerShell 7, cross-platform — deliberately the only
  gate script, so there is no second implementation to drift). Stages and thresholds:
  `docs/technical/testing.md`. `-Quick` for edit loops; the full gate at every phase boundary.

## Prospect Settings

- `spec-disposal: delete` — default: the spec folder is removed at finalize
  (after PR approval); `main` never carries it. Set `archive` (+
  `retention: [days]`, default 180) for regulated projects or when branch
  protection dismisses approvals on new commits: the folder moves to
  `specs/archive/YYYY/` at publish and expired folders are pruned there.

## Standards

@standards/global/code-quality.md
@standards/global/testing.md
@standards/global/git-workflow.md

Scenario rules (`standards/global/scenario-guidelines.md`) and review
calibration (`standards/global/validation-calibration.md`) are injected by
the skills that need them. The calibration file is meant to be tuned per
project (severity definitions, skip rules, Minor cap) — its content must
always read as direct reviewer instructions, since it lands verbatim in
review prompts.

## Before Writing Code

1. A spec must exist — otherwise suggest `/sdd-start`.
2. Follow `tasks.md`; reference scenario IDs (FR-x.y-Sz) in tasks and
   commit messages on the feature branch — never in code or test names.
3. At `medium+`, never edit test files from the implementation context —
   send disputes to the test author.

## Tech Stack

Rust 1.97 (edition 2024) workspace, 10 crates under `crates/`.

| Layer | Choice |
|-------|--------|
| ECS · Math | `bevy_ecs` 0.19 standalone (**not** full Bevy) · `glam` |
| Rendering | `wgpu` 30 + `winit` 0.30; `egui` for debug/tooling UI only |
| Scripting | Luau via `mlua` 0.12 (`luau`, `vendored`) |
| Networking | QUIC via `quinn` 0.11; `ed25519-dalek` identity |
| Storage | `redb` 4.1 + `zstd`; `serde`/`bincode`/`rkyv` |
| Testing | `cargo nextest`, `proptest`, `criterion`, golden frames via `mc-testkit` |

Dependency versions are pinned centrally in `[workspace.dependencies]`; crates opt in with
`dep = { workspace = true }`. Never version a dependency in a member crate.

Crate map — `mc-core` (primitives, no I/O) · `mc-world` (chunks, worldgen, persistence) ·
`mc-script` (Luau host, registry, hot reload) · `mc-proto` (wire format) · `mc-net` (QUIC) ·
`mc-sim` (ECS simulation = the server) · `mc-render` (wgpu) · `mc-client` / `mc-server` (binaries) ·
`mc-testkit` (harnesses). Dependencies flow inward; `mc-core` depends on nothing in the workspace.

## MyCraft Invariants

Non-negotiable. Violating any of these is a Blocker, not a style note.

1. **The base game is a mod.** All content lives in `content/base/` as Luau, with no privileged
   engine access. Zero hardcoded block, item, recipe, NPC, or quest definitions in Rust. A missing
   hook is fixed in the API, never special-cased.
2. **State in Rust, behaviour in script.** Runtime state lives in the ECS; Luau holds behaviour
   only. This is what makes hot reload lose nothing.
3. **A bad mod never takes down the server.** Sandbox, instruction budget, memory cap, and
   per-callback fault isolation are load-bearing, not later hardening.
4. **The server is authoritative.** Client input is a request, never a fact. Anything a client
   claims is recomputed server-side.
5. **Verification precedes the thing it verifies.** The capture harness lands before the renderer,
   the bot harness before multiplayer, the adversarial suite before public servers.

Default rigor for this project is **`high`**. Drop to `medium` only for self-contained, low-risk
work, and record why.

## Where things live

`PLAN.md` holds the original research and rationale. Everything else routes through `docs/INDEX.md`:

| Need | Read |
|------|------|
| Why a stack choice was made | `docs/technical/decisions.md` (ADRs) |
| Crate topology, threading, tick model | `docs/technical/architecture.md` |
| Chunk format, palette, ID stability | `docs/technical/world-format.md` |
| Wire protocol, replication | `docs/technical/protocol.md` |
| The Luau API surface | `docs/modding/` |
| Running a public server | `docs/ops/` |

Directory-specific rules live in nested `CLAUDE.md` files under `crates/mc-script/`,
`crates/mc-net/`, `crates/mc-render/`, and `content/`.
