# MyCraft — Architecture & Implementation Plan

A fully scriptable, hot-reloadable voxel sandbox with 32-player authoritative multiplayer,
script-defined blocks/items/tools/crafting, script-defined NPCs, and a script-defined
quest/storyline system.

> **This is a dated research snapshot (2026-08-11), not a living document.**
>
> It records the investigation and reasoning that produced the locked decisions below, and it is
> kept for that provenance. It is **not** updated as the project moves — several sections are
> already superseded:
>
> | Superseded here | Current source of truth |
> |---|---|
> | §2 stack rationale | `docs/technical/decisions.md` (ADR-001…009) |
> | §9 tooling and autonomy gaps | `docs/technical/testing.md` · asset APIs now available, see ADR-009 |
> | §10 milestones | `product/roadmap.md` and Linear (initiative **MyCraft**) |
> | "nothing implemented yet" | workspace, quality gate and Prospect config now exist |
>
> Start at `docs/INDEX.md` for current state. Read this only for *why* a decision was made.

## Decisions locked (2026-08-11)

| Decision | Choice | Consequence |
|---|---|---|
| Engine | **Custom wgpu renderer** + `bevy_ecs` standalone | §5 is ours. ~3–4 weeks of UI/audio/asset plumbing we write instead of inherit. |
| World scale | **Infinite streaming** | Chunk streaming, unbounded persistence, and region-based quest/story anchoring are in scope from M1. |
| Art | **Procedural placeholders** | I generate noise-based textures in code. Real assets swap in later via hot reload, no code change. |
| Hosting | **Public server with accounts** | Auth, identity, moderation and anti-grief become first-class in M4–M5. See §6.4. This was the one choice against my initial recommendation — it costs roughly 2–3 extra weeks but avoids a painful retrofit. |

---

## 0. The one architectural commitment everything else follows from

> **The base game is a mod.**

`content/base/` — every block, item, tool, recipe, NPC, biome, quest and dialogue line in the
"vanilla" game — is written in the same scripting language, against the same API, with the same
permissions as anything a third party would write. No engine-side hardcoded block list, no
privileged "vanilla" path.

This is the only way to guarantee "fully customizable." If the base game needs an engine hook that
mods can't reach, the API is incomplete and we fix the API. Luanti (ex-Minetest) proves this model
works for a voxel game at scale; Minecraft's retrofitted modding proves the alternative is painful
forever.

Corollary: **state lives in Rust, behavior lives in script.** That single rule is what makes
hot reload safe (§4).

---

## 1. Verified environment

Checked on this machine, 2026-08-11:

| Component | Version | Notes |
|---|---|---|
| rustc / cargo | 1.97.1 | `x86_64-pc-windows-msvc`, linking verified with a real build |
| MSVC linker | working | needed for `mlua`/Luau C sources — confirmed functional |
| CMake | 4.2.3 | available for vendored native deps |
| Node | 22.19.0 | tooling only (LSP/schema gen), not shipped |
| .NET | 10.0.1026 | not used — see §2 rejected alternatives |
| GPU | RTX 4090 (+ Intel UHD 770) | dev machine is far above target spec; must test on the iGPU too |
| CPU / RAM | i5-12600K (10C/16T) / 64 GB | comfortable for a 32-player dedicated server + client |

Not yet installed, needed later: `cargo-nextest`, `sccache`, optionally the Tracy profiler GUI.

---

## 2. Stack decision

### Recommended

| Layer | Choice | Version (verified on crates.io) |
|---|---|---|
| Language | **Rust** | 1.97.1 |
| ECS | **`bevy_ecs` standalone** (not full Bevy) | 0.19.0 |
| GPU | **`wgpu`** | 30.0.0 |
| Windowing | **`winit`** | 0.30.13 |
| Scripting | **Luau via `mlua`** | mlua 0.12.0 |
| Transport | **QUIC via `quinn`** | 0.11.11 |
| World storage | **`redb`** | 4.1.0 |
| Debug UI | **`egui` + `egui-wgpu`** | 0.36.1 |
| Serialization | `serde` 1.0.229, `bincode` 3.0.0, `rkyv` 0.8.18 (chunks) | |
| Compression | `zstd` 0.13.3 (persist), `lz4_flex` 0.14.0 (wire) | |
| Parallelism | `rayon` 1.12.0, `flume` 0.12.0, `parking_lot` 0.12.5 | |
| Hot-reload watch | `notify` 8.2.0 + `notify-debouncer-full` 0.7.0 | |
| Atomic registry swap | `arc-swap` 1.9.2 | |
| Profiling | `tracy-client` 0.18.4, `puffin` 0.20.0 | |
| Testing | `cargo-nextest`, `proptest` 1.11.0, `criterion` 0.8.2 | |

### Why Rust

No GC. A 20 Hz server tick has a 50 ms budget; a stop-the-world pause during a tick is a visible
hitch for 32 players simultaneously. `rayon` makes parallel chunk meshing and worldgen nearly free
to write. The voxel/wgpu/networking crate ecosystem is the strongest of the realistic options.

### Why *not* full Bevy

This is the closest call in the whole plan. Bevy 0.19 (June 2026) has GPU-driven rendering, BSN
scenes and a real editor preview — it is genuinely good now. But:

- Voxel rendering is **custom no matter what**: greedy meshing, palette-compressed chunk storage,
  a single indirect draw for all terrain, custom light propagation, custom frustum/occlusion
  culling. Bevy's mesh/material/scene abstractions are things we'd bypass, not use.
- Bevy ships breaking releases roughly quarterly (0.17 late 2025 → 0.18 March 2026 → 0.19 June
  2026). Over a multi-year project that's recurring migration cost on the one layer we've replaced
  anyway.
- `bevy_ecs` is separable and excellent, so we take exactly the part that helps.
- `bevy_mod_scripting` (0.21.0, ~47k downloads) is not mature enough to bet the core feature of
  this project on. Our scripting layer is the product; it should be ours.

We take `bevy_ecs` and write the renderer. We give up Bevy's free UI/audio/asset pipeline — cost is
roughly 3–4 weeks of work spread across the project, paid back in control over the hot path.

**If you'd rather use full Bevy**, the whole plan still stands — §4 through §8 are engine-agnostic.
Only §5 (renderer) changes. Say so and I'll re-cut it.

### Why Luau (not LuaJIT, not WASM, not JS)

Mods are untrusted code running inside a 32-player server. That constraint decides it.

`mlua` 0.12 exposes exactly what's needed, and Luau-only:

- `Lua::sandbox(true)` — read-only stdlib and globals
- `Lua::set_interrupt(..)` — periodic callback during execution ⇒ **instruction budget per script
  call; a runaway `while true do end` in a mod gets killed, not the server**
- `Lua::set_memory_limit(..)` — per-VM allocation ceiling
- `Lua::set_compiler(..)` / optional `luau-jit`
- Gradual typing ⇒ real autocomplete and type-checking for mod authors, which matters enormously
  for a scripting-first game

Rejected:
- **LuaJIT** — faster, but Lua 5.1, no sandbox mode, no interrupt hook. Untrusted mods make this a
  non-starter.
- **WASM (`wasmtime` 47)** — best isolation, any source language, but ~164 ns/call vs ~10 ns for
  lighter runtimes, and a compile step between "edit file" and "see change." That kills the
  hot-reload experience, which is a headline requirement. **Kept as a documented escape hatch**
  for mods that need heavy compute (§4.6).
- **JS (`rquickjs` 0.12)** — viable, but Luau's sandbox + interrupt story is purpose-built for
  precisely this threat model (Roblox runs untrusted user scripts at enormous scale).

### Why QUIC over ENet/`renet`

`renet` 2.0 is good and game-focused. QUIC via `quinn` 0.11 wins because we get, for free and
already hardened: encryption + authentication, congestion control, connection migration, and —
critically — **independent streams plus unreliable datagrams in one connection**. That maps
perfectly onto our two traffic classes: bulk chunk data on reliable streams (no head-of-line
blocking against gameplay), entity snapshots on unreliable datagrams.

---

## 3. Process & crate layout

Two binaries. **Singleplayer runs the dedicated server in-process on a background thread** and
talks to it over the real protocol — so there is exactly one code path, and singleplayer is a
continuous integration test of multiplayer.

```
mycraft/
├─ Cargo.toml                  # workspace
├─ crates/
│  ├─ mc-core/                 # ids, math, registry types, event defs. No I/O. No deps on the rest.
│  ├─ mc-world/                # chunk storage, palette, lighting, worldgen, redb persistence
│  ├─ mc-script/               # Luau host: VM, bindings, sandbox, scheduler, hot reload
│  ├─ mc-proto/                # wire format, packet defs, delta codec (shared)
│  ├─ mc-net/                  # quinn transport, channels, connection lifecycle
│  ├─ mc-sim/                  # ECS: physics, entities, NPC runtime, inventory, crafting, quests
│  │                           #   headless-capable — this IS the server core
│  ├─ mc-render/               # wgpu: mesher, GPU-driven terrain, atlas, text, egui
│  ├─ mc-client/               # bin: window, input, prediction, interpolation, render loop
│  ├─ mc-server/               # bin: dedicated headless server
│  └─ mc-testkit/              # bot clients, headless frame capture, determinism harness
├─ content/
│  └─ base/                    # THE GAME. 100% Luau.
│     ├─ mod.toml
│     ├─ init.luau
│     ├─ blocks/  items/  recipes/  npcs/  biomes/  quests/  dialogue/
│     └─ tests/                # mod self-tests, run on every hot reload
└─ docs/
```

Threading model:

- **Tick thread** — owns the ECS *and the Lua VM*. `mlua` is `!Send` by default; keeping Lua
  pinned to one thread is both simplest and best for determinism. 20 Hz.
- **Worker pool (rayon)** — chunk meshing, worldgen, compression, pathfinding. Never touches Lua.
- **Network thread (tokio)** — quinn I/O, encode/decode. Talks to the tick thread over `flume`.
- **Render thread** (client only) — reads a snapshot of world+entity state, never blocks the tick.

---

## 4. Scripting & hot reload — the core system

### 4.1 Two kinds of scripted content

**Definitions (declarative)** — blocks, items, tools, recipes, biomes, NPC archetypes, quests,
dialogue. These are *pure data produced by running mod scripts*. They contain Lua function values
for behavior, but they are otherwise inert.

**Behavior (imperative)** — `on_tick`, `on_use`, `on_break`, NPC brains, quest triggers. Lua
functions stored inside the definitions above.

Everything a mod produces at load time ends up in one immutable `Registry`.

### 4.2 The reload mechanism

```
content/ changes
   ↓ notify (debounced 150ms)
build candidate Registry in a FRESH scratch VM, off the tick thread
   ↓
validate: ID stability, dangling references, removed-blocks-still-in-world,
          recipe cycles, schema conformance
   ↓
run each mod's tests/ suite against the candidate
   ↓
   ├─ FAIL → keep the old Registry running. Log a structured diff to console + in-game.
   │         Nothing breaks. This is the normal case while iterating.
   └─ OK   → at the next tick boundary: ArcSwap::store(new_registry)
             fire on_reload(old_state) hooks
             broadcast RegistryChanged to clients (atlas + models rebuilt client-side)
```

`arc_swap::ArcSwap<Registry>` makes the swap wait-free for readers. Systems mid-tick hold a
consistent `Guard`; the swap only takes effect at a boundary. No locks in the hot path.

### 4.3 Why state survives reload

Because **runtime state is never in Lua**. Entity position, health, inventory, quest progress —
all live in the Rust ECS and are addressed from Lua via lightweight handles. Throwing away the Lua
VM throws away nothing but code.

Mods that genuinely need their own persistent state declare it explicitly:

```lua
local state = mycraft.state("mymod", { totem_charges = 0 })   -- serde-backed, Rust-side
```

and may migrate it across a reload:

```lua
mycraft.on_reload(function(old_version, old_state) ... end)
```

### 4.4 Stable IDs

Mods use string IDs (`"base:stone"`). The registry assigns dense numeric IDs at build time for
storage and the wire. **The string↔numeric mapping is persisted with the world**, so numeric IDs
can be reassigned freely on reload or on load with a different mod set. Removing a mod whose blocks
are placed in the world leaves them as tagged unknown-block placeholders that round-trip losslessly
and come back when the mod returns. (Minecraft took years to fix this; we get it right on day one.)

### 4.5 Fault isolation

A bad mod must never take the server down.

- `Lua::sandbox(true)`, `set_memory_limit`, `set_interrupt` instruction budget per callback
- No `io`, no `os.execute`, no `package.loadlib`; `require` is a custom loader confined to the mod
  directory
- Every callback invoked through a catch-all wrapper: error → log with mod attribution → after
  N consecutive failures the callback is disabled and the mod is marked degraded, server continues
- Per-mod CPU accounting surfaced in a profiler view, so "which mod is eating the tick" is a
  question with an answer

### 4.6 WASM escape hatch

`wasmtime` 47 stays a supported second backend for mods doing heavy compute (custom worldgen
noise, large simulations). Same registry API, compiled artifact instead of source, no live reload.
Deferred to post-M7 — designed for, not built early.

### 4.7 Client-side scripting

A **separate, more restricted VM** on the client for HUD, custom UI, particles, dialogue
presentation. It is never authoritative — it can request, never decide. Client scripts ship with
the mod and are delivered by the server on join, hash-verified and cached.

---

## 5. Voxel world & renderer

- **Chunks**: 16×16×16 sections in 16×16×N columns. Palette-compressed (1/2/4/8 bits per voxel by
  palette size); homogeneous sections collapse to a single value and cost nothing.
- **Meshing**: binary greedy meshing — the modern bit-manipulation formulation meshes a section in
  roughly 50–200 µs single-threaded, and it parallelizes trivially across `rayon`. Typical
  geometry reduction is 75 %+ vs naive per-face.
- **Draw**: one indirect draw call for all terrain, per-chunk draw commands built by a compute
  shader doing frustum + occlusion culling. Vertices packed to 8 bytes (position/normal/AO/UV all
  bit-packed) — a 16³ section fits comfortably in cache.
- **Lighting**: flood-fill sky + block light, incremental on block change, computed on workers.
- **Textures**: array texture, not an atlas — no bleeding, mipmaps work, and hot-reloading a
  single block texture doesn't rebuild everything.
- **Storage**: `redb` keyed by `(dimension, chunk_pos)`, values zstd-compressed. Single-file world,
  ACID, no corruption on crash.
- **Worldgen**: base terrain in Rust (must be fast + parallel); Lua controls it at a coarse
  granularity — biome definitions, ore/structure placement rules, decorators. Per-voxel worldgen
  callbacks into Lua would be far too slow and are deliberately not offered.

Target: 12–16 chunk view distance at 144 fps on the 4090, 60 fps at 8 chunks on the UHD 770.

---

## 6. Multiplayer — 32 players

Authoritative server. Clients predict only their own movement and block placement, and reconcile.
Other entities are interpolated ~100 ms in the past. No client is ever trusted.

**Traffic classes over one QUIC connection:**

| Data | Mode | Rate |
|---|---|---|
| Chunk sections | reliable stream, lz4 | on demand, priority by distance |
| Entity snapshots | unreliable datagram, delta vs last-acked | 20 Hz |
| Block edits | reliable stream | immediate |
| Chat / inventory / quests | reliable stream | event-driven |

**Budget (worked, not guessed):**

- Entity snapshot: ~150 visible entities × ~16 B delta = 2.4 KB @ 20 Hz ≈ **48 KB/s ≈ 384 kbit/s
  per player** → ~12 Mbit/s aggregate server upstream at 32 players. Fine for a VPS; marginal on a
  home upload.
- Initial chunk load at 12-chunk radius: ~3 750 non-empty sections × ~600 B ≈ **2.2 MB**, a
  1–2 second stream-in.
- Steady state while sprinting: crossing a chunk boundary every ~3 s ⇒ ~90 KB ⇒ **~30 KB/s**.

So chunk streaming dominates the join, entity replication dominates steady state, and neither is
close to a problem at 32. The real scaling risk is **server CPU in scripted NPC brains**, not
bandwidth — which is why §7 has a LOD budget.

**Interest management**: per-player AoI grid; entities outside radius aren't replicated at all.
This is what keeps the above numbers flat as the world grows.

### 6.4 Public servers: identity, auth, moderation

Locked in as a first-class M4–M5 concern rather than a retrofit.

**Identity — keypair first, not passwords.** Each client generates an `ed25519-dalek` 3.0 keypair
on first run; the public key *is* the account ID. Login is a signed challenge-response over the
already-authenticated QUIC channel. No password to leak, no password reset flow, no credential
database worth stealing. Optional `argon2` 0.5 password login exists only as a recovery/secondary
path for players who want to move between machines, and is off by default.

QUIC gives us TLS 1.3 transport security for free, so the application-layer auth only has to prove
*who*, never protect the wire.

**Server-side account store** in the same `redb` file as the world: pubkey → display name, first
seen, last seen, playtime, permissions, ban state. Display names are claimed first-come and are
*not* an identity — the key is.

**Moderation primitives** (engine-level, exposed to script):
- whitelist / blacklist by pubkey, with expiry and reason
- IP-level and pubkey-level rate limiting on connect, chat, and block edits
- role/permission system (`mycraft.permission.define`), scriptable so a server can define its own
- rollback: block edits are journalled with actor + timestamp, so griefing is undoable by region
  and time range rather than by hand
- server-side audit log of privileged actions

**Anti-abuse, given that the server is authoritative anyway:**
- movement validated server-side against terrain and a speed/flight envelope; violations clamp and
  log rather than kick, to survive lag spikes
- block reach distance, break time, and inventory transactions all recomputed server-side from the
  registry — the client's claim is a request, never a fact
- per-connection bandwidth ceiling so one client cannot force unbounded chunk streaming
- **mod scripts are a supply-chain surface**: server-delivered client scripts are hash-pinned, and
  the client shows what it's about to run on first join to a given server

**Scriptability boundary:** mods can *define* permissions, roles and moderation commands, and can
observe auth events. Mods can never mint identities, bypass the ban list, or read another player's
key material. That boundary is enforced in Rust, not by convention.

**Not in scope**: a central account authority across servers (Mojang-style). Each server is its own
trust root. If cross-server identity is wanted later, the ed25519 key is already the right
primitive for it.

---

## 7. NPCs — scripted end to end

Split by hot path:

- **Rust owns**: spatial index, A* over the voxel grid (with jump/fall/swim edge costs,
  hierarchical for long paths), collision, perception queries, animation state, the scheduler.
- **Luau owns**: everything about *what the NPC is and does*.

Two authoring styles, both fully scripted:

**Coroutine brains** — reads like intent, yields across ticks:

```lua
mycraft.npc("base:villager", {
  model = "base:villager", health = 20,
  senses = { sight = 24, hearing = 16 },

  brain = function(self)
    while true do
      local threat = self:nearest(function(e) return e:hostile_to(self) end, 16)
      if threat then
        self:flee_from(threat, 20)            -- yields until arrival or interrupt
      elseif self:time_of_day() > 0.75 then
        self:goto(self:home())
        self:sleep_until_dawn()
      else
        self:wander(self:home(), 12)
      end
      coroutine.yield()
    end
  end,

  on_interact = function(self, player)
    mycraft.dialogue.start(player, "base:villager_greeting")
  end,
})
```

The Rust scheduler resumes each brain under an instruction budget; movement itself happens in Rust
across many ticks while the coroutine is suspended.

**Declarative behavior trees** — for crowds. Compiled to a Rust-evaluated tree at registry build,
so common NPCs cost no Lua call per tick.

**LOD budget** (this is the actual scaling constraint): NPCs within ~48 m of a player run full
brains at 20 Hz; 48–128 m run at 2 Hz with simplified movement; beyond that they're frozen and
simulated statistically on load. Target ≥ 500 active NPCs inside a 50 ms tick alongside 32 players.

---

## 8. Items, tools, crafting, quests, storyline — all script-defined

**Items & tools** — tiers and speeds are data; break time is computed in Rust from the tables with
an optional Lua override:

```lua
mycraft.item("base:iron_pickaxe", {
  stack = 1,
  tool = { class = "pickaxe", tier = 3, speed = 6.0, durability = 250 },
})

mycraft.recipe.shaped {
  output  = { "base:iron_pickaxe", 1 },
  pattern = { "III", " S ", " S " },
  keys    = { I = "base:iron_ingot", S = "base:stick" },
}
```

**Quests & storyline** — staged, with declarative objectives:

```lua
mycraft.quest("base:the_deep_road", {
  title = "The Deep Road",
  stages = {
    { id = "descend", objective = { type = "reach_depth", y = -48 } },
    { id = "recover", objective = { type = "obtain", item = "base:ancient_core", count = 1 } },
    { id = "return",  objective = { type = "deliver", item = "base:ancient_core",
                                    to = "base:archivist" },
                      on_complete = function(player) ... end },
  },
  rewards = { xp = 500, items = { { "base:deepsteel_ingot", 3 } } },
})
```

**The event bus is the perf-critical piece here.** Quests, achievements and NPC reactions subscribe
with *declarative predicates* (`block_broken where block == "base:diamond_ore"`). Predicates are
matched in Rust; Lua is only entered on an actual match. Calling into script for every block break
by 32 players would not survive contact with reality.

Per-player quest state lives in the Rust ECS and is persisted with the player record — so quest
scripts hot-reload without resetting anyone's progress (§4.3).

---

## 9. What I need to build this autonomously

### Already available
Rust 1.97.1 + working MSVC linker, CMake, git, Node, Python, and a machine that far exceeds the
target spec.

### To install (I can do this)
`cargo-nextest`, `sccache`, `cargo-flamegraph`. Optionally the Tracy GUI for you to look at.

### I need from you
1. **Permission to run** `cargo build/test/run`, and to spawn long-running background processes
   (dedicated server + bot clients) during load tests.
2. **`git init`** in `E:\_PROJEKTE\MyCraft` — currently not a repo. I'd like commits per milestone.
3. Network access to crates.io (already working).

### Real gaps in autonomy, and how I'll close them

**I cannot see the screen.** This is the biggest one and it gets solved in M0, before any gameplay
work:
- a headless render mode that renders to an offscreen texture and writes PNGs, which I then read
  back and inspect;
- a deterministic replay format (fixed seed, scripted camera path, fixed tick count) so
  screenshots are comparable across runs;
- perceptual-diff regression tests against golden frames, so I catch rendering regressions without
  needing to look at every frame.

**I cannot test 32 real players.** Solved with `mc-testkit`: 32 headless bot clients running the
*real* network stack against the real server, driven by scripts, reporting tick time, bandwidth,
p99 latency and desync counts as machine-readable output.

**I cannot author art.** I can generate decent procedural placeholder textures in code (noise-based
16×16 block textures, flat-shaded blocky models) and that's what I'll start with. Real textures,
models and audio need either you or a CC0 pack. **This does not block any system work** — but the
game will look like programmer art until it's addressed. Flagging it now rather than at M9.

**I cannot judge game feel.** Movement acceleration, jump arc, block-break timing, camera FOV,
mining feedback — these need a human. Mitigation: every one of them is a script/config value you
can tune live via hot reload, no recompile. You playtest, you tweak the numbers.

---

## 10. Milestones

Each has an exit criterion I can verify without you looking at a screen.

| # | Milestone | Exit criterion |
|---|---|---|
| **M0** | Workspace, CI, **headless capture harness**, bot-client skeleton | `cargo nextest run` green; harness writes a PNG I can read |
| **M1** | Chunk storage, palette, binary greedy mesher, wgpu terrain pipeline, fly camera | Golden-frame test of a generated world; mesher benchmark < 200 µs/section |
| **M2** | Player physics, raycast, block place/break, singleplayer loop | Scripted replay places/breaks 10k blocks, world state asserted |
| **M3** | **Luau host, registry, sandbox, hot reload** — `base:` blocks defined in Luau | Edit a `.luau` file mid-run; block behavior changes with no restart, no state loss |
| **M4** | Client/server split, quinn transport, protocol, **ed25519 identity + challenge-response auth**, account store | Bot client generates a key, authenticates, joins, moves, edits, disconnects cleanly; unauthenticated connect is rejected |
| **M4.5** | **Moderation & anti-abuse**: whitelist/ban, roles/permissions, rate limits, edit journal + region rollback, server-side movement/reach/inventory validation | Adversarial bot suite — speed hack, reach hack, edit flood, chat flood, inventory dupe — all rejected and logged, server unaffected |
| **M5** | Interest management, delta replication, prediction/reconciliation | **32 authenticated bots**, p99 tick < 25 ms, zero desyncs over 10 min |
| **M6** | Items, inventory, tools, crafting — all defined in `content/base/` | Craft chain stone→iron pickaxe driven entirely from Luau |
| **M7** | NPC runtime: pathfinding, coroutine brains, behavior trees, LOD scheduler | 500 scripted NPCs inside a 50 ms tick with 32 bots connected |
| **M8** | Quests, dialogue, event bus with Rust-side predicates, persistence | Multi-stage quest completes; survives server restart and a hot reload |
| **M9** | Lighting polish, audio, UI, mod packaging + loading order, world save/load | Full playthrough of a scripted storyline via bot replay |

M0 first is not negotiable — without the capture harness I'd be building blind.

---

## 11. Remaining open questions

All four launch decisions are settled (see top of document). What's left is deferrable — none of it
blocks M0 through M3:

1. **Real art assets** — placeholders carry us to a playable game, but at some point someone has to
   make or source textures, models and audio. Decide by ~M6.
2. **Cross-server identity** — out of scope now; the ed25519 key is the right primitive if we ever
   want it.
3. **WASM mod backend** — designed for in §4.6, built post-M7 only if a real mod needs it.
4. **Game feel tuning** — needs you at a keyboard, from M2 onward. All values are hot-reloadable so
   this never blocks me.
