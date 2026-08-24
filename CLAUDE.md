---
spec-disposal: archive
retention: 365
review-mode: solo
---
# MyCraft

A voxel sandbox whose entire game layer — blocks, items, tools, crafting, NPCs, quests and
storyline — is defined in sandboxed Luau scripts that reload live on a running server. It targets
32-player authoritative multiplayer on desktop, and the "vanilla" game is itself a mod with no
privileges a third-party mod lacks.

# Prospect — Spec-Driven Development Framework

This project uses **Prospect** for spec-driven development (SDD) with
test-driven development.

## Pipeline

`/sdd-start` classifies the work and creates the spec folder; from there a
deterministic resolver (`.prospect/scripts/sdd-next.sh`) reads the folder's
frontmatter and disk state, picks the next phase, and composes its prompt
from `.prospect/prompts/` fragments. `/sdd-next` runs one phase;
`/sdd-auto` loops phases unattended in fresh contexts under
`.prospect/autonomy.md`. The LLM never branches on work-type or rigor —
the matrix (`.prospect/prompts/matrix.tsv`) does.

Every spec carries `work-type:` and `rigor:` in its frontmatter.

| `work-type:` | Path (phases) | Deliverable |
|---|---|---|
| feature | specify → [architect] → [discuss] → tasks → implement → validate → complete | behavior, TDD per scenario |
| decision | specify → discuss → decide → [implement] → validate → complete | ADRs, contracts, enforcement checks |
| fix | specify (root cause; RCA at high+) → implement → validate → complete | regression tests + narrowest fix |
| docs | edit → complete | documentation only |
| chore | work → complete | non-behavioral change, gate-guarded |

Rigor scales verification, not ceremony: `low` gate-only · `medium`
(default) combined reviewer · `high` 3-reviewer workflow with verification
and sign-off · `xhigh` parallel personas · `max` negotiating personas.
The architect phase runs only when the spec declares a non-empty
`## Architecture Delta`. Escalate rigor whenever new risk appears (record
the reason); downgrade only with explicit user confirmation.

## Key Principles

1. **Specs are the source of truth.** Acceptance scenarios (EARS) are the
   test contract: each scenario is the floor of at least one test; the 1:N
   mapping lives in the spec folder's `test-map.md`. Test names stay
   behavioral; code never carries spec or scenario IDs. The floor is not a
   ceiling — add any test that would catch something real, and never write
   one that could not fail (`testing.md` §1).
2. **TDD is non-negotiable.** Failing test output is displayed before any
   implementation. At `medium+`, tests are authored and owned by the test
   author — implementation never edits test files; disputes go to
   arbitration.
3. **Architecture is standing.** `docs/technical/architecture.md` holds the
   crate map, boundaries, and project constants; specs declare deltas only.
4. **`docs/` is as-built reality.** Completed specs consolidate into
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
5. **Gates are deterministic.** `scripts/sdd-gate.ps1` must exit 0 at every
   phase end and before validation and completion. Gate amendments are a
   project-level decision with a runtime budget — never a spec deliverable.
6. **Artifacts have budgets.** `.prospect/scripts/sdd-artifact-lint.sh`
   enforces caps (tasks 60 lines, one map line per test, registry entries
   ≤50 words); overflow knowledge consolidates into `docs/`.
7. **Out of Scope is binding.** Unspecced work is recorded, not built.
8. **Phases resume from disk.** After any commit boundary `/clear` is safe;
   the resolver reconstructs state. `metrics.md` records per-phase
   timestamps for cost accountability.
9. **Every spec delivers something a stakeholder can actually use.** At
   least one capability a **player**, **mod author** or **server operator**
   can exercise for themselves — named in the spec, with the stakeholder
   named, and reachable by them without reading Rust. **Building the backend
   and never giving anyone access to it is not a complete spec.** It is half
   of one, and the missing half is where the design errors are: a surface
   nobody has used is a surface nobody has found the gaps in, and the
   documentation obligation in Principle 4 has nothing to bite on because
   there is nothing a stakeholder can do. If a capability genuinely cannot
   reach a stakeholder inside one spec, the spec is scoped wrongly — widen it
   to include the thinnest path that reaches one, or merge it with the spec
   that does. This is the vertical-slice rule with a test attached: name the
   stakeholder, name the thing they can now do, or the spec is not done.

## Commands

| Command | Purpose |
|---------|---------|
| `/sdd-init-project` | New project: Q&A → structure, standards, gate, docs index |
| `/sdd-onboard` | Existing project: detect toolchain → gate, docs index, standing architecture |
| `/sdd-start [desc]` | Classify work-type + rigor, branch, folder, hand off to resolver |
| `/sdd-next [folder]` | Resolve and run the next phase |
| `/sdd-auto [folder]` | Drive remaining phases unattended (autonomy policy) |
| `/sdd-clarify PROJ-123` | Fill the clarifications ledger via the issue tracker |
| `/sdd-discuss` | Persona challenge on demand (`--phase discuss`) |
| `/consolidate-docs [path]` | Merge any source material into docs/ |

The complete phase runs at the end of **every** spec, not once per release.
That is what keeps `docs/` as-built and `specs/active/` free of finished work.

## Locations

- Active specs: `specs/active/YYYY-MM-DD-name/` (spec, requirements ledger,
  test-map, metrics, decisions)
- Registry: `specs/REGISTRY.md` — one line ≤50 words per completed spec
- Framework runtime: `.prospect/` (resolver, prompts, templates, autonomy
  policy) — framework-owned, overwritten on update; do not edit in projects
- Living docs: `docs/` routed by `docs/INDEX.md` · Standing architecture:
  `docs/technical/architecture.md`
- Quality gate: `scripts/sdd-gate.ps1` (PowerShell 7, cross-platform — deliberately the only
  gate script, so there is no second implementation to drift). Stages and thresholds:
  `docs/technical/testing.md`. `-Quick` for edit loops; the full gate at every phase boundary.

## Prospect Settings

This file's frontmatter is authoritative: `spec-disposal: archive`,
`retention: 365`, `review-mode: solo`.

- `spec-disposal: archive` — the complete phase moves the spec folder to
  `specs/archive/YYYY/` on the feature branch before it merges, and prunes
  archived folders older than the retention window. Setting `delete` instead
  removes the folder outright, so `main`'s tree never carries it.
- `review-mode: solo` — there is no external review loop and no approval
  wait: a PASS validation plus a green gate is the merge condition, and the
  complete phase merges to `main` directly. The resolver reads this setting
  to pick its completion handoff, so it is the setting — not the prose in
  `standards/global/git-workflow.md` — that actually changes behavior.

**The archive is history, not documentation, and it is not a place to defer
consolidation to.** `docs/` remains the only as-built record and Key Principle
4 is unchanged: a capability is documented for every audience it reaches, as
part of the spec's definition of done. "It is in the archive" is never a reason
to leave something out of `docs/` — a reader edits a test file or reaches for
the modding guide, and neither of those goes looking through a folder of
superseded specs. What archiving buys is recovery of *reasoning* that was
otherwise destroyed at merge — a scenario's wording, a task's rationale, a
test-map entry recording why a mutation missed — not a second home for
anything a reader will actually need.

Outstanding Minor/Info findings become tracked issues at completion. The
tracker is Linear, driven by `linear-cli` (there is no Linear MCP). Specs
carry the tracker key as `issue:`, not the template's stock `jira:` — rename
it when starting from `.prospect/templates/spec.template.md`.

## Standards

@standards/global/code-quality.md
@standards/global/testing.md
@standards/global/git-workflow.md

Scenario rules (`standards/global/scenario-guidelines.md`) and review
calibration (`standards/global/validation-calibration.md`) are injected by
the phases that need them. The calibration file is tuned per project
(severity definitions, skip rules, Minor cap); its content must read as
direct reviewer instructions, since it lands verbatim in review prompts.

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
3. **A bad mod never takes down the server.** Sandbox, call-and-loop budget, memory cap, and
   per-callback fault isolation are load-bearing, not later hardening. The budget charges calls
   and loop edges, never instructions — a loop body of any size is free — so cost comes down by
   batching calls rather than by shortening code.
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

`sdd-conductor` (`.claude/agents/`) owns one MVP end to end, decomposing it into specs and driving
each through the pipeline by spawning a fresh subagent per phase, and managing the Linear issues.
Each phase is resolved by `/sdd-next` rather than by a per-stage skill, so the conductor never
decides which phase comes next — the resolver does. Because an MVP outlives a single context, the
conductor invokes `/loop` **on itself** using `.claude/loops/conductor-loop.md` and advances one
step per tick, reconstructing state from Linear and git each time. There is no outer supervisor —
the conductor is the only owner.
