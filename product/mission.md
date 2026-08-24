# MyCraft

## Mission

A voxel sandbox whose entire game layer — blocks, items, tools, crafting, NPCs, quests and
storyline — is defined in sandboxed Luau scripts that reload live, so creators iterate on a running
32-player world without a restart. It exists for people who found Minecraft's modding ceiling
frustrating and want the game itself to be the moddable surface, not a thing mods are bolted onto.

## Target Users

- **Primary**: Modders and server operators who want a scriptable voxel game where the "vanilla"
  content has no privileges their own content lacks.
- **Secondary**: Players on those servers, who get the resulting worlds; contributors to the engine
  itself.

## Tech Stack

| Layer | Technology | Notes |
|-------|------------|-------|
| Language | Rust 1.97 (edition 2024) | No GC — a 20 Hz tick has a 50 ms budget for 32 players |
| ECS | `bevy_ecs` 0.19 standalone | Not full Bevy; the voxel renderer is ours (see `docs/technical/decisions.md`) |
| Rendering | `wgpu` 30 + `winit` 0.30 | Custom chunk mesher, GPU-driven indirect terrain draw |
| UI | `egui` 0.36 | Debug and tooling UI; game HUD is custom per `standards/global/ui-design.md` |
| Scripting | Luau via `mlua` 0.12 | Sandbox, instruction-budget interrupts, memory caps — mods are untrusted |
| Networking | QUIC via `quinn` 0.11 | Reliable streams for chunks, unreliable datagrams for entity state |
| Identity | `ed25519-dalek` 3.0 | Public key is the account; `argon2` only as an opt-in recovery path |
| Database | `redb` 4.1 | Embedded, ACID, single world file; `zstd` for chunk payloads |
| Testing | `cargo nextest`, `proptest` 1.11, `criterion` 0.8 | Plus golden-frame capture via `mc-testkit` |
| CI/CD | `scripts/sdd-gate.ps1` | fmt → clippy+complexity → size → deps → SAST → secrets → tests+coverage |
| Asset tooling | ElevenLabs, fal.ai | Build-time only, never a runtime dependency — see ADR-009 |

## Core Principles

1. **Spec-Driven Development (Prospect)**: All features start with a specification
2. **Test-Driven Development**: Tests written before implementation
3. **Quality First**: Follow standards in `standards/global/`
4. **Scope Control**: Out-of-scope items are NOT implemented
5. **The base game is a mod**: Everything in `content/base/` is written in Luau against the public
   API with no privileged engine access. If the base game needs a hook mods cannot reach, the API
   is incomplete and the API gets fixed — not the base game special-cased.
6. **State in Rust, behaviour in script**: Runtime state lives in the ECS; Luau holds only
   behaviour. This is what makes hot reload lose nothing.
7. **A bad mod must never take down the server**: Sandbox, call-and-loop budget, memory cap, and
   per-callback fault isolation are non-negotiable, not hardening to add later. The budget charges
   calls and loop edges rather than instructions, which is what a content author sizes against.

## Success Metrics

- **Tick budget**: p99 server tick < 25 ms with 32 connected players and 500 active scripted NPCs
- **Hot reload**: script change to observable in-world effect < 1 s, with zero loss of player,
  world, or quest state
- **Reload safety**: a mod that fails to compile or fails its own tests never reaches the running
  world — the previous registry keeps serving
- **Client frame rate**: 144 fps at 16-chunk view distance on an RTX 4090; 60 fps at 8 chunks on
  Intel UHD 770
- **Scripting completeness**: 100% of base-game content defined in `content/base/`, with zero
  hardcoded block, item, recipe, NPC, or quest definitions in Rust
- **Abuse resistance**: the M4.5 adversarial bot suite (speed, reach, edit flood, chat flood,
  inventory dupe) is fully rejected and logged with no server impact

## Getting Started

```powershell
# Prerequisites: Rust 1.97+ and MSVC build tools, plus:
#   cargo install cargo-nextest cargo-llvm-cov cargo-deny cargo-machete --locked
#   rustup component add llvm-tools-preview
#   winget install Gitleaks.Gitleaks        # optional locally, required in CI

cargo build
cargo nextest run

# Quality gate — must exit 0 before every phase end
./scripts/sdd-gate.ps1            # full: 7 stages
./scripts/sdd-gate.ps1 -Quick     # format + lint + size only, for edit loops

# Asset generation is build-time only and needs .env (see .env.example).
# It defaults to a dry run; regenerating an existing asset requires --force.

# Start developing
/sdd-start "your first feature"
```

## Development Workflow

```
/sdd-start [work]        # Classify work-type + rigor, branch, spec folder
/sdd-next                # Resolve and run the next phase (repeat to done)
/sdd-auto                # Or: drive remaining phases unattended
```

Default rigor for this project is **`high`** — the networking, authentication, anti-abuse and
hot-reload-safety work all carry real correctness risk, so specs get a binding architecture plan
and a reviewer workflow with sign-off. Drop to `medium` only for self-contained, low-risk work
(and record why).

## Links

- Repository: *(not yet published)*
- Documentation: `docs/INDEX.md`
- Issue Tracker: Linear — team `prodigy.solutions` (`PRO-`), initiative **MyCraft**
- Planning source: `PLAN.md` (research and rationale behind the locked decisions)
