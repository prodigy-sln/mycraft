---
spec-disposal: archive
retention-days: 365
---
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
   test contract: each scenario gets at least one test, and that mapping is
   a floor rather than a ceiling — add any test that would catch something
   real, and never write one that could not fail (`testing.md` §1). The
   mapping lives in the spec folder's `test-map.md` — test names stay
   behavioral, and code never carries spec or scenario IDs.
2. **TDD is non-negotiable.** Failing test output is displayed before any
   implementation. At `medium+`, tests are authored and owned by the test
   author — implementation never edits test files; disputes go to
   arbitration.
3. **`docs/` is as-built reality.** Completed specs consolidate into
   `docs/` via `docs/INDEX.md` routing. Future concepts live in
   `specs/active/` and `product/roadmap.md`, never in `docs/`.

   **The spec that implements something documents it properly for all three
   audiences — that is part of its definition of done, not a follow-up and
   never a separate issue.** The audiences are the **engine reader**, the
   **mod author**, and the **player**, and *properly* means each one can act
   on it without reading the code:

   - **Mod author** — how to write it. Where the files live, the shape of a
     declaration, every field with its type and its bound, what a refusal
     looks like and how to read it, what a fault does, and a complete worked
     example that runs. A reference that lists names without showing a
     working example is not documentation.
   - **Player** — what is different when they play. What they can now see,
     build, break or reach, and how to get to it.
   - **Engine reader** — the as-built record: how it works, why it is shaped
     that way, and what a future change must not break.

   **Documentation is owed the moment something is implemented, even when
   nobody can use it yet.** Both "not applicable to that audience" and "there
   is nothing anyone can do with this yet" are refused. What exists is
   documented now, while the person who built it still knows why it is shaped
   that way, what it refuses, and what the numbers mean. Walking back over it
   an increment later means reconstructing intent from code, and what gets
   reconstructed is missing precisely the parts nobody thought to write down —
   that is how a gap becomes permanent. Documenting at the moment of
   implementation is what makes the documentation living rather than
   archaeological. Only a surface that genuinely does not exist yet belongs to
   a later spec, and its absence is stated plainly rather than left silent.
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
| `/sdd-complete` | Consolidate docs, register spec, dispose folder, merge to main, push. Runs after **every** spec. |
| `/consolidate-docs [path]` | Merge any source material into docs/ |

## Locations

- Active specs: `specs/active/YYYY-MM-DD-name/` (removed by `/sdd-complete` at the end of the spec)
- Registry: `specs/REGISTRY.md` — permanent one-line record per completed spec
- Templates: `specs/_templates/` · Living docs: `docs/` (routed by `INDEX.md`)
- Quality gate: `scripts/sdd-gate.ps1` (PowerShell 7, cross-platform — deliberately the only
  gate script, so there is no second implementation to drift). Stages and thresholds:
  `docs/technical/testing.md`. `-Quick` for edit loops; the full gate at every phase boundary.

## Prospect Settings

- `spec-disposal: archive`, `retention-days: 365` (see this file's frontmatter,
  which is authoritative) — `/sdd-complete` moves the spec folder to
  `specs/archive/YYYY/` on the feature branch before it merges, and prunes
  archived folders older than the retention window. There is no approval wait: a
  PASS validation plus a green gate is the merge condition. Setting `delete`
  instead removes the folder outright, so `main`'s tree never carries it.

  **The archive is history, not documentation, and it is not a place to defer
  consolidation to.** `docs/` remains the only as-built record and Key Principle
  3 is unchanged: a capability is documented for every audience it reaches, as
  part of the spec's definition of done. "It is in the archive" is never a reason
  to leave something out of `docs/` — a reader edits a test file or reaches for
  the modding guide, and neither of those goes looking through a folder of
  superseded specs. What archiving buys is recovery of *reasoning* that was
  otherwise destroyed at merge — a scenario's wording, a task's rationale, a
  test-map entry recording why a mutation missed — not a second home for
  anything a reader will actually need.

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

## Autonomous builds

`sdd-conductor` (`.claude/agents/`) owns one MVP end to end, spawning a fresh subagent per SDD
stage and managing the Linear issues. Because an MVP outlives a single context, the conductor
invokes `/loop` **on itself** using `.claude/loops/conductor-loop.md` and advances one step per
tick, reconstructing state from Linear and git each time. There is no outer supervisor — the
conductor is the only owner.
