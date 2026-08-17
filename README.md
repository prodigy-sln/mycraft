# MyCraft

A voxel sandbox whose entire game layer — blocks, items, tools, crafting, NPCs, quests and
storyline — is defined in sandboxed Luau scripts that reload live on a running server. It targets
32-player authoritative multiplayer on desktop, and the "vanilla" game is itself a mod, holding no
privilege a third-party mod lacks.

*By [prodigy.solutions](https://prodigy.solutions)*

## Where the project actually is

**Early. The two things that identity rests on — the Luau scripting host and the network — do not
exist yet.** `mc-script`, `mc-net`, `mc-proto` and `mc-server` are doc-comment skeletons of three to
seven lines. There is no mod API to write against, no server to connect to, and no multiplayer.

What does exist is a singleplayer client you can build, launch and walk around in. From MVP 1 so
far:

- **Generated terrain** — value noise over a fixed lattice, a 64 × 64-block footprint with a sea
  level and a landmark pillar, deterministic from a seed.
- **Palette-compressed chunk storage** — 16³ sections in 256-block columns, where a section's
  storable identity is namespaced block *names* rather than runtime ids, so a world survives a
  change in registration order.
- **A binary greedy mesher** — visible faces merged into quads by a fixed scanline sweep, culled
  against six neighbour sections, emitted in an order that *is* the loop nesting, so identical
  contents mesh byte-identically.
- **A wgpu renderer** — a compute pass compacting indices into a single `draw_indexed_indirect`
  against an array texture. Flat-shaded: no lighting, no shadows, no transparency pass.
- **A player** — free-look camera, WASD under gravity, AABB-vs-voxel collision, cursor capture.
  Movement reaches the simulation only as an intent that has no way to state a position, so the
  clamp already lives on the authoritative side.

Not yet: breaking or placing blocks, an inventory, persistence (quit and the world is gone), a HUD,
audio, or anything past the current session.

**The textures are placeholders and do not look like the blocks they are on** — stone and dirt draw
teal, grass draws tan. That is deliberate: this increment asks textures to be deterministic and
distinguishable, never plausible, and correcting a colour per block name would be block content
hardcoded in Rust, which is the one thing the base game may not be. Real artwork ships as content.

Blocks are content, and as of MVP 2 they are **content written in Luau** — `content/base/blocks/*.luau`,
each a chunk the sandboxed scripting host evaluates, loaded through a `DefinitionSource` port with no
public way to register a definition from Rust. The registry contract did not change with the swap:
same six fields, same refusals, same saves. Behaviour is not scriptable yet — a declaration says what
a block *is* and calls nothing the engine provides. Writing one is
[`docs/modding/README.md`](docs/modding/README.md); the full contract is
[`docs/modding/blocks-items.md`](docs/modding/blocks-items.md).

## Build and run

Needs Rust 1.97 (edition 2024) and a GPU and driver that `wgpu` 30 can open (Vulkan, DX12 or
Metal).

```bash
cargo run -p mc-client
```

Run it from the repository root: the client resolves its content at `content/base`, relative to the
working directory. The dev profile builds dependencies at `opt-level = 3` and the workspace at `1`,
so it is playable without `--release`; voxel meshing and worldgen are unusably slow at
`opt-level = 0`, which is why the profile is set that way.

W / A / S / D walk, the mouse looks, space jumps, escape releases the cursor and clicking recaptures
it. The walk keys are bound by physical position, so they work on QWERTZ and AZERTY too. Full
behaviour, including what happens when you walk off the edge of the finite world, is in
[`docs/user/gameplay.md`](docs/user/gameplay.md).

`cargo run -p mc-server` builds and prints that it is not implemented yet.

## Checks

```bash
cargo nextest run --workspace          # the whole suite
pwsh scripts/sdd-gate.ps1              # the full quality gate
pwsh scripts/sdd-gate.ps1 -Quick       # the fast subset, for edit loops
cargo bench -p mc-world --bench meshing
```

`scripts/sdd-gate.ps1` is the deterministic quality gate and must exit 0 at every phase boundary.
PowerShell 7 is cross-platform, so it is deliberately the only gate script — there is no `.sh` twin
to drift out of sync. It runs format, lint and complexity, a GPU-free build of `mc-testkit` and
`mc-render`, rustdoc, file-size limits, unused-dependency and supply-chain scans, a secret scan, and
the test suite under coverage instrumentation with an 80% threshold. Every stage runs even when an
earlier one fails.

Beyond the toolchain it wants `cargo-nextest`, `cargo-llvm-cov`, `cargo-machete` and `cargo-deny`
(`cargo install <tool> --locked`); `gitleaks` is optional and its stage is skipped when absent. The
stages and thresholds are documented in
[`docs/technical/testing.md`](docs/technical/testing.md).

## The workspace

Ten crates under `crates/`. Dependencies flow inward; `mc-core` depends on nothing in the
workspace. Versions are pinned centrally in `[workspace.dependencies]` and crates opt in with
`dep = { workspace = true }` — never a version in a member crate.

| Crate | Role | State |
|-------|------|-------|
| `mc-core` | Primitives shared by everything: block registry contract, namespaced ids. No I/O. | Built |
| `mc-world` | Chunk sections and columns, the block palette, the content loader, the greedy mesher | Built |
| `mc-sim` | The simulation — the server side, in-process for now: worldgen, player physics, collision, camera | Built |
| `mc-render` | The wgpu draw path, behind a compile-enforced `gpu` feature seam over a pure layer | Built |
| `mc-client` | The windowed binary and composition root — the only crate that resolves both sim and renderer | Built |
| `mc-testkit` | Harnesses: offscreen frame capture, CIELAB ΔE golden comparison, artifact reporting | Built |
| `mc-script` | Luau host: sandboxed VMs, engine bindings, definition registry, hot reload | Skeleton |
| `mc-proto` | Wire format: packet definitions, delta codec, versioning | Skeleton |
| `mc-net` | QUIC transport on `quinn`, identity handshake | Skeleton |
| `mc-server` | Headless authoritative server | Skeleton |

In use today: `glam`, `wgpu` 30 with `winit`, `rayon`, `arc-swap`, and `cargo nextest` with
`proptest` and `criterion`. Chosen and pinned but not yet reached by any crate: `bevy_ecs` 0.19
standalone (not full Bevy) for the simulation, `egui` for debug and tooling UI only, `mlua` 0.12 for
Luau, `quinn` for QUIC with `ed25519-dalek` identity, and `redb` for storage.

## Reading further

Everything routes through [`docs/INDEX.md`](docs/INDEX.md), which describes the system **as built** —
never planned behaviour.

| Need | Read |
|------|------|
| Why a stack choice was made | [`docs/technical/decisions.md`](docs/technical/decisions.md) (ADRs) |
| Crate topology, seams, dependency direction | [`docs/technical/architecture.md`](docs/technical/architecture.md) |
| Chunk format, palette, block-identity stability | [`docs/technical/world-format.md`](docs/technical/world-format.md) |
| The mesher's contract and the draw path | [`docs/technical/rendering.md`](docs/technical/rendering.md) |
| Gate stages, harnesses, what automation cannot check | [`docs/technical/testing.md`](docs/technical/testing.md) |
| Authoring a block today | [`docs/modding/blocks-items.md`](docs/modding/blocks-items.md) |
| Controls and how movement feels | [`docs/user/gameplay.md`](docs/user/gameplay.md) |

[`PLAN.md`](PLAN.md) holds the original research and rationale. [`product/roadmap.md`](product/roadmap.md)
has the MVP sequence: scriptable content, then multiplayer, then the survival loop, then a living
world. [`CLAUDE.md`](CLAUDE.md) states the invariants the project is not allowed to break — the base
game is a mod, state in Rust and behaviour in script, a bad mod never takes down the server, the
server is authoritative, and verification precedes the thing it verifies.

## How it is built

MyCraft is developed with **Prospect**, a spec-driven development framework for Claude Code:
BDD scenarios as the test contract, TDD with an isolated test author, deterministic gates, tiered
rigor, and living documentation. Every feature runs `/sdd-start → /sdd-tasks → /sdd-implement →
/sdd-validate → /sdd-complete`; completed specs consolidate into `docs/` and leave a line in
[`specs/REGISTRY.md`](specs/REGISTRY.md).

The framework itself lives at
[prodigy-sln/prospect](https://github.com/prodigy-sln/prospect). Its rules as they apply here are in
[`CLAUDE.md`](CLAUDE.md) and [`standards/global/`](standards/global).

## License

Dual-licensed under either [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option, as
declared in `Cargo.toml`. A check in the test suite fails if the declaration and the shipped texts
ever drift apart.
