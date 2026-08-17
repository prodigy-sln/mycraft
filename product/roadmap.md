# Product Roadmap

Tracked in Linear — team `prodigy-solutions` (`PRO-`), initiative
[**MyCraft**](https://linear.app/prodigy-solutions/initiative/57bd5125-175b-4347-8ab7-b3284f17d744).
Each MVP below is a Linear project; each spec is one flat `PRO-` issue inside it. Linear is
authoritative for issue status.

> **This file drives the autonomous build.** The conductor reads the MVP marked
> `← current`, its feature table, and its **Exit criteria** section to determine
> scope and what "done" means. Exactly one MVP must carry the marker, and it must
> have exit criteria — otherwise the conductor stops and asks rather than
> guessing. When promoting the next MVP, move the marker *and* write its exit
> criteria before starting a run.

## How this roadmap works

**Incremental, not waterfall.** Every MVP ends with a *playable game* — not a finished layer.
MVP 1 is a game you can start, walk around in, and build in. Each later increment adds capability
to a thing that already runs. There is no point on this roadmap where the project is "half a
renderer and no gameplay".

Two rules follow from that:

1. **An increment is not done until you can play it.** A green gate is necessary, not sufficient.
   Every MVP's exit criterion includes a human-playable build.
2. **Depth of planning decreases with distance.** MVP 1 is specified; later MVPs are a sketch and
   are expected to change once you have played the earlier ones. Planning MVP 6 in detail now
   would be the waterfall this structure exists to avoid.

Quality is *not* traded away for increment size. The gate, TDD discipline, and rigor tier apply
identically to a small MVP — small means narrow scope, never lower standards.

---

## MVP 1 — Playable Sandbox ← current

**Linear**: [MyCraft MVP 1: Playable Sandbox](https://linear.app/prodigy-solutions/project/mycraft-phase-1-foundation)
**Goal**: A game that starts, renders a world, and lets you move and build in it.

Decomposed into eleven specs, in dependency order. Each moves the playable thing
forward; none is a pure layer.

| Priority | Feature | Issue | Status |
|----------|---------|-------|--------|
| P0 | Headless frame-capture harness | PRO-849 | **Done** |
| P0 | Chunk storage and block palette | PRO-850 | **Done** |
| P0 | Binary greedy mesher | PRO-851 | **Done** |
| P0 | wgpu terrain pipeline and windowed client — *you can see terrain* | PRO-852 | **Done** |
| P0 | Camera, player physics and collision — *you can walk around* | PRO-853 | **Done** |
| P0 | Headless client-input harness — *prerequisite of PRO-854* | PRO-873 | **Done** |
| P0 | Raycast targeting, block break and place — *you can build* | PRO-854 | **Done** |
| P0 | A cell holds a block or nothing — *air is not a block* | PRO-876 | **Done** |
| P1 | World persistence — *quit and resume* | PRO-855 | **Done** |
| P1 | Minimal HUD: crosshair, held block, debug overlay — *the HUD is content* | PRO-856 | **Done** |
| P2 | Ship the LICENSE texts the workspace declares | PRO-874 | **Done** |

**PRO-873 goes before PRO-854, and the ordering is the point.** Every FR-5 scenario of
PRO-853 is verified as policy and none as product behaviour: the winit `ApplicationHandler`
in `crates/mc-client/src/events.rs` needs a real window and nothing constructs one. Two
mutations proved it — submitting an empty intent every tick, and deleting the
`accepts_pointer_motion` gate — each left the whole suite green. PRO-854 adds break and
place to that same unreachable dispatch, so invariant 5 applies directly: verification
precedes the thing it verifies. **Exit criterion: each of those two mutations turns at
least one test red.**

PRO-874 was small and unglamorous — `Cargo.toml` declared `MIT OR Apache-2.0` and the
workspace carried neither text. Both now ship, and a check on every gate run fails if the
declaration and the shipped texts drift apart again (`docs/technical/licensing.md`).

The workspace skeleton and quality gate predate the first spec (commits `f5780f3`,
`a93d2da`), which is why PRO-849 is the capture harness alone.

Two invariants that MVP 1 could plausibly breach, resolved up front:

- **No Luau host until MVP 2, yet invariant 1 still binds.** Blocks are registered
  at runtime from a data file under `content/base/` — never `enum Block` in Rust.
  MVP 2 swaps the loader, not the registry. **The HUD is content on the same
  terms**: the crosshair and held-block indicator are declared under
  `content/base/`, so a mod owns them and there are no HUD definitions in Rust.
  The debug overlay is the deliberate exception and stays engine-owned — it
  inspects the engine, including when content is broken, so a mod must not be
  able to disable the instrument used to diagnose that mod.
- **Singleplayer, yet invariant 4 still binds.** `mc-sim` is authoritative in-process;
  the client submits intents. Otherwise MVP 4 is a rewrite, not a transport swap.

**Exit criteria** — all must hold:
- `sdd-gate.ps1` exits 0
- Mesher benchmark < 200 µs/section — **met** (`cargo bench -p mc-world --bench meshing`,
  terrain ~136 µs; a standalone command, deliberately not a gate stage)
- Scripted replay places and breaks 10 000 blocks with asserted world state
- **You can launch the client, walk around, break and place blocks, quit, relaunch, and your
  changes are still there**

The frame-capture harness went first and was non-negotiable: without it the agent building the
renderer could not have seen its own output. PRO-873 is the same argument applied to input.

Everything is a placeholder — procedural textures, no audio, one block type family. That is
correct for MVP 1. Do not gold-plate it.

Terrain textures (PRO-869) moved to MVP 2, where texture-by-key becomes the feature being built
rather than an ornament on a pipeline that cannot yet express it.

---

## MVP 2 — Scriptable Content

**Linear**: [MyCraft MVP 2: Scriptable Content](https://linear.app/prodigy-solutions/project/mycraft-mvp-2-scriptable-content)
**Goal**: The base game becomes a mod. Blocks are defined in sandboxed Luau, reload live on a
running server, wear real textures, and go into your hand when you break them.

**Not current yet.** MVP 1 ends at the user's sign-off, which has not happened, so the `← current`
marker has not moved. These criteria are written first because this document's own header requires
it before a run starts.

| Priority | Feature | Issue | Status |
|----------|---------|-------|--------|
| P0 | Composition model and API surface policy land in the standards — *a docs commit, not a spec* | PRO-928, PRO-924 | Todo |
| P0 | Luau host, sandbox, hostile-mod harness — *verification precedes the thing it verifies* | PRO-916 | **Done** |
| P0 | A content refusal names the file, the declaration and the field — *fix a typo in one edit* | PRO-939 | **Done** |
| P0 | Blocks defined in Luau — pure loader swap, TOML retired — *the base game is a mod* | PRO-917 | **Done** |
| P0 | Hot reload — *edit a block while playing* | PRO-918 | **Done** |
| P0 | A player entering a world is never left inside solid rock — *the shove, at every door rather than only at a reload* | PRO-948 | Todo |
| P0 | The composition root moves to `mc-server`, and the registry stops travelling to the client — *the split becomes structural instead of a comment* | PRO-944 | Todo |
| P0 | Solid, drawn, occludes and targetable split, plus swimmable and density — *you can see water and swim in it* | PRO-904 | Todo |
| P0 | Where generated art comes from at build time — *a decision and an ADR amendment, not a spec* | PRO-930 | Todo — **gates all art below** |
| P0 | The grass block looks like a grass block: per-face keys, baked art, real pixels from disk — *the world stops being teal* | PRO-947, PRO-902 | Blocked on PRO-930 |
| P0 | The placeholder palette guarantees separation for blocks shipping no art | PRO-869 | Blocked on PRO-930 |
| P0 | Components attach behaviour; grass spreads onto dirt — *the model proven, not just moved* | PRO-919 | Todo |
| P0 | Break a block and hold it — *pick up what you took, place it again* | PRO-929 | Todo |

**Two ordering rules produced this sequence, and both cost something.**

- **The TOML loader dies before anything extends it.** The solid/drawn/occludes split and per-face
  textures need no scripting and could ship first — but both add fields to `BlockDefinition`, and
  adding them to a loader PRO-917 then deletes is work built to be thrown away. So the swap goes
  first and **every later field is born in Lua.** The cost is real: everything after PRO-917 depends
  on a new runtime. It is contained by PRO-917 being a *pure* swap — identical fields, no new
  surface, exit criterion "the world renders identically".
- **Components exist before any callback does.** No non-composition callback system is built and
  then migrated. Attachment is the mechanism from the first line of PRO-919.

**PRO-919 is not optional.** Without one real behaviour, MVP 2 only moves TOML into Lua: the sandbox
would have nothing to sandbox and exit criterion 3 would test a host with no API. Grass spreading
exercises random ticks, bounded neighbour reads, block replacement, determinism and the fault
machinery at once, and it is visible.

**Exit criteria** — all must hold:
- `sdd-gate.ps1` exits 0
- **Zero block definitions in Rust**, mechanically guarded — the hardcoded-name scan survives the
  loader swap
- **A hostile mod cannot take down the server.** Infinite loop, memory bomb, sandbox escape,
  faulting callback, runaway cascade, hostile `__index` — each contained, each named, and each
  proven by a mutation that reddens a test
- **You can edit a block definition in a text editor while the game is running, save, and see it
  change in the world without restarting**
- **You can launch the client, walk on terrain wearing real textures, break a block, see what you
  broke in your hand, place it again, quit, relaunch, and your changes are still there**

The last criterion subsumes MVP 1's, which is the point — every increment ends playable.

**Known gap, narrowed rather than closed.** Every mechanism spec above ships no new *content*, and
MVP 2 still ends with the same four blocks it started with — so "the base game is a mod" stays
weakly tested, because missing hooks are found by authoring real content and four trivial blocks
exercise almost nothing. A block-set spec belongs here; it needs two answers first, since **breaking
a block to obtain it makes the reachable set equal to the generated set**: which blocks, and how a
player gets one that worldgen never places.

What PRO-947 changes is narrower than that and worth stating precisely, because the two are easy to
confuse: it ships real *art* for blocks that already exist, not new blocks. It is the first spec in
this MVP a player can see at all — every one before it is invisible to them by design, and PRO-917's
spec states "nothing a player can see changes" as an intended outcome rather than an omission. So it
answers "the world is teal" and leaves "there are only four blocks" open.

**PRO-930 gates every art spec, and it is a decision rather than work.** Generated assets are not
committed — deterministic and free output is regenerated, nondeterministic or paid output is
committed, and golden frames stay committed because they are evidence rather than assets. What is
*not* settled is the mechanism, and it cannot be settled inside a spec that assumes an answer:
VoxForge lives in `tools/`, SPEC-013 FR-9.1 forbids anything in `crates/` depending on `tools/`, and
a `build.rs` that invokes VoxForge is exactly that dependency. The options are an explicit pre-build
step, splitting VoxForge so its render core is a crate, or amending FR-9.1. Two things belong in
whichever answer wins: a **manifest hash of the generated set**, so that a generator change fails
loudly instead of shifting every golden and reading as a renderer regression, and a **gate stage
that fails on a committed generated artifact**, because a `.gitignore` rule drifts back over time
and a stage does not.

**Deferred, recorded rather than dropped.** Per-cell state (PRO-911) and everything needing it —
multi-cell instances and collision shapes (PRO-912), placement orientation (PRO-913). Tags
(PRO-915), which wait only because no third-party content exists yet. The HUD content format
(PRO-893) and fonts as content (PRO-894). Worldgen as content (PRO-921), non-cube geometry
(PRO-925), derived geometry (PRO-926), propagation and lighting (PRO-922, PRO-923).

---

## Provisional — revised after you play MVP 1

Sketch only. Order and contents are expected to change based on feedback.

MVP 2 has left this table — it is specified above.

| MVP | Adds | Still playable because |
|-----|------|------------------------|
| **3 — Worldgen** | Terrain shape, biomes, surface rules, ores and structures declared as content instead of Rust; caves and trees | The world stops being one generated shape and becomes somewhere worth exploring |
| **4 — Multiplayer** | QUIC transport, ed25519 identity, replication, prediction; 2 players → 32, over LAN or a forwarded port | The same sandbox, now with someone else in it |
| **5 — Survival Loop** | Items, inventory, tools, crafting — all script-defined | Building gains purpose and progression |
| **6 — Living World** | NPCs with pathfinding and scripted brains; then quests, dialogue, storyline | The world is inhabited and has things to do |
| **7 — Public & Polished** | NAT hole punching and relay fallback; moderation, anti-abuse, rollback; lighting, audio, UI polish, mod packaging | Ready for strangers on a public server, and reachable without a router |

**Worldgen moved ahead of multiplayer, 2026-08-15.** Two reasons, and the second is the load-bearing one. It closes the last hardcoded-block-name exemption, which is what makes "the base game is a mod" true rather than nearly true. And **a wire format is far harder to change after it exists than a function call is to turn into a message** — content streaming is part of multiplayer's own design below, and content only becomes addressable and hashable once it is scriptable, so designing the protocol first would mean designing it before knowing what it carries. Multiplayer into a four-block world would be a demo; multiplayer into a generated world is a game.

The counter-argument, recorded because it is a good one: 32-player authoritative multiplayer is this project's headline claim, and this leaves its biggest technical risk unvalidated longest. That risk is retired by a **measurement** — snapshot size and tick cost at 32 simulated players — not by reordering an increment around it.

Backlog, unscheduled: WASM mod backend · cross-server identity · real art assets (decide by
MVP 5) · dimensions and portals · a redstone-equivalent logic system (a strong test of scripting
API expressiveness).

**That line now has a home in Linear:
[MyCraft: Tooling and Deferred Decisions](https://linear.app/prodigy-solutions/project/mycraft-tooling-and-deferred-decisions-2267497c6cce).**
Two kinds of thing live there and they share one property — **the conductor must never read
them as MVP scope**, since it determines scope from the MVP marked `← current` and that MVP's
feature table. The first kind is **developer tooling**: `tools/` members, which are not the
game and have their own lifecycle. The second is **deferred decisions**: cross-cutting choices
that outlive any single MVP, recorded so they are not re-derived from scratch when the MVP that
needs them arrives. An item leaves by being promoted into the feature table of the MVP that
adopts it.

**Every non-MVP `PRO-` issue belongs to that project explicitly**, including tooling that is
actively being built. Linear files a projectless issue into whichever project it guesses — it
put PRO-905 into *MVP 2: Scriptable Content* — and an MVP project is precisely where a
non-MVP issue must not sit, because the conductor will eventually read that table as its scope.

**Developer tooling is not on this roadmap and is not conductor scope.** `tools/` holds
developer tools that are not the game: ADR-009's `tools/asset-gen`, and **VoxForge**
(`tools/voxforge`, PRO-905, SPEC-013) — an AI-authorable voxel model format, a CPU preview
renderer and a CLI, so an agent can produce assets and see its own output well enough to
self-correct. It is deliberately absent from every MVP feature table above: the conductor
reads those tables to determine scope, and tooling built on demand must not be adopted as
MVP scope by a run that finds it. Its assets have no engine consumer until **MVP 6** — there
is no non-cube geometry or block-model path today (`docs/technical/rendering.md`) — so it
ships the pipeline ahead of the thing that draws it, which is why its preview renderer
carries the correctness weight the engine cannot yet carry for it.

### How a voxel model reaches the GPU

Settled 2026-08-15 in discussion, filed rather than built. A `.mcvox` file (SPEC-013) is
**source**; what the renderer consumes is a compiled artifact.

- **PRO-906 — the model compiler.** Interior face culling first and it is nearly free (a solid
  32³ model naively drawn as cubes is 196 608 faces; only 6 144 are on the surface), then a
  greedy merge of coplanar same-material faces — the algorithm `mc_world::mesh` already runs on
  terrain sections, at a different scale. **Baking a texture is the wrong reflex by default**:
  merging by material makes every quad uniform by construction, so there is nothing to bake and
  a per-quad material index beats a texture fetch. Baking only wins when colour varies every
  voxel and merge-by-material degenerates to one quad per face — true of a detailed character,
  false of a door. **Both are supported and the choice is made per model by measuring the merge
  ratio at compile time.** Binding constraint inherited from SPEC-013: the compiler emits one
  mesh *per part* and does not flatten, because a flattened mesh cannot rotate an arm about its
  attachment point, which is why parts exist. (`.vox` export flattens — that is an interchange
  lane, not the runtime path.)
- **PRO-907 — instanced batching, which is the larger win.** A compiled door is a few dozen
  quads whichever strategy it gets; what costs at 32 players is draw calls, and 500 doors as
  500 draws dwarfs any per-model quad count. The terrain path already found the answer — one
  `draw_indexed_indirect` for the whole world — and the open question is whether models can keep
  its portability property (`instance_count: 1` and `first_instance: 0` avoid two optional
  device features) or must pay for it.
- **PRO-908 — model AO is whole-model self-occlusion, baked, or nothing.** A model is shaded
  from its own voxels only and rendered in full even when partly occluded. This is the cheap
  variant, and it buys something specific: because AO then depends on nothing outside the model,
  the merge narrowing `technical/rendering.md` warns about is paid once, deterministically, in
  the compiler — so compiled model meshes stay reproducible. **Terrain gets no such reprieve**;
  its AO remains world-dependent and still invalidates terrain goldens whenever it arrives.
- **PRO-909 — where compilation happens.** A compiled artifact has two inputs, the source and
  the compiler, which does not fit the content-addressed cache's "changed and missing are the
  same case" property. Leaning toward streaming the small source and compiling client-side,
  keyed `(source hash, compiler version)`, so a compiler change invalidates local caches with no
  server involvement. To be settled **with** MVP 4's streaming work, not ahead of it.

**Art assets are already licensed and available** — see ADR-010 for the two-class handling.
Bundle purchases (music, SFX, GUI, 2.5D sprites) live at `\\ds01\assets\GameAssets`; CC0 material
is [PixVoxelAssets](https://github.com/tommyettinger/PixVoxelAssets) and
`VoxelCoreLab_Watercolor_Terrain_Textures_1024px` (dirt/grass/stone/water, four variants each).
Earliest use is **MVP 2**, where texture-by-key becomes a real feature; the voxel character and
dungeon models suit **MVP 6**, and audio and GUI belong to **MVP 7**. MVP 1 stays on placeholders.

### Content streaming, home hosting, and reachability

Still a sketch: each part becomes binding only when the MVP that owns it adopts it, and
**the two are now split.** Content streaming and the content-addressed cache are **MVP 4**'s,
and that substance must survive into its spec. From *"Home hosting without opening a port"*
onward — the rendezvous service, hole punching, relay assignment — is **MVP 7**'s, per the
decision recorded at the end of this section.

**Content is edited server-side and streams to players — textures included, not just
scripts.** A texture swapped on a running server reaches every connected client live.
Nothing extra is built for it: the hash changes, clients miss, they fetch, they swap. It is
fast because a texture is *data, not structure* — UVs are per-block-type, so no remeshing
and no chunk rebuild, just one array layer overwritten before the next frame. Two things
decide whether it feels instant: the server must downsample before streaming (1024 px
source × 32 players is a download; at the array's layer size it is a few KB), and replacing
a layer is cheap while *adding* one may force the array to be recreated and every bind
group rebuilt.

**A content-addressed client cache, keyed by hash rather than by name.** The client offers
the hashes it holds and the server sends the remainder, which buys the property the whole
scheme rests on: **"changed" and "missing" are the same case.** A changed texture is simply
a different hash, therefore a miss — no version numbers, no invalidation protocol, no
cache-busting, and so no second path to get wrong or forget to trigger. Base content ships
with the client and is therefore already cached; a mod's assets take the identical path and
merely miss the first time. Name-keying loses all of it.

**No CDN at this stage, and the reason is the shape of the problem rather than its size.** A
live swap is under a megabyte in total across every connected player, and QUIC's independent
streams already cover the shape: bulk asset transfer does not head-of-line-block tick
traffic as TCP would, and asset streams can be prioritised below gameplay. Latency here is a
*prioritisation* question; a CDN answers a *volume* question — and for live edits a CDN is
strictly slower, needing upload plus propagation first. The real CDN case is **cold joins
against a large modpack**, a first-join cost only since nobody fetches twice, and that
belongs to MVP 7.

**Home hosting without opening a port.** A lobby/rendezvous service introduces two peers and
performs NAT hole punching, so a player can host from home. The lobby is a **mode of
`mc-server`**, but the constraint that decides its shape is that a rendezvous must be
publicly reachable — which is exactly what a home host is not, since a machine behind NAT
cannot introduce itself. Some instance therefore has to sit on a public address, and that
makes the no-hosted-dependency rule structural rather than a promise: anyone can run their
own lobby and point friends at it. Two constraints on the shape:

- **The lobby role must be implementable from `mc-net` alone and must never instantiate a
  world.** If running a signalling service drags `mc-sim` in, the deployment ships a game
  simulation to introduce two sockets.
- **Rendezvous and relay must be separately switchable**, because their costs differ by
  orders of magnitude. Rendezvous is kilobytes per join and scales with joins; relay carries
  a whole session and scales with players × minutes, at roughly 0.4 GB/hour for a four-player
  session. Offering a cheap public rendezvous must not implicitly sign the operator up to
  relay everyone's traffic.

**Relay assignment is a control-path decision, not a data-path load balancer.** The lobby
answers once at join — "use relay-3 at this address" — and both peers connect *directly* to
it, so the lobby never sees a game packet. A balancer sitting in the traffic would have to
carry the combined packet rate of every relay behind it, making it simultaneously the
bottleneck and the single point of failure, and funnelling all the cheap horizontal scaling
back through one box. What the assigner needs is small: a registry of relay hosts with
heartbeats, capacity and health, plus a pick that **weighs latency to both peers** rather
than picking least-loaded — for a game, an idle relay on the wrong continent beats a busy
nearby one only on paper. The genuinely fiddly part is **a relay dying mid-session**: peers
must notice, re-request an assignment and reconnect without the session ending. Expect the
real work to be there, not in the choosing.

**Two rules that bind:**

1. **A self-hosted server must never require a CDN or any hosted service.** The server
   serves its own content; hosted infrastructure is an optional accelerator in front of
   that, never the only path.
2. **LAN and direct connection must always work.** Port forwarding stays supported and hole
   punching is additive. Otherwise the game dies when a service does, and offline LAN play
   dies with it.

Both are free to honour now and expensive to retrofit the moment something assumes a hosted
URL exists.

**Settled 2026-08-15: MVP 4 means *"multiplayer works"*** — direct connection and LAN.
**NAT hole punching, the rendezvous service and relay assignment all defer to MVP 7**,
alongside the public-server work they belong with.

So MVP 4 is the transport and the authority: QUIC, identity, replication, the client
submitting intents against a server that recomputes them, and 32 players on a LAN or
behind a forwarded port. Everything above about assigners, relays and dying relays stays
in this document as the sketch it is — none of it is MVP 4's to build.

Rule 2 above is why this costs nothing later: LAN and direct connection must work
regardless of what arrives afterwards, so punching is **additive to a shipped path**
rather than a rewrite of one. The order also holds the project's own invariant — a bot
harness exercising many clients has to exist before multiplayer, and an adversarial suite
before anything faces the public internet. Punching lands next to the second, not the
first.

---

## Completed

| Feature | Completed | Spec |
|---------|-----------|------|
| Headless frame-capture harness (PRO-849) | 2026-08-11 | `2026-08-11-frame-capture-harness` |
| Chunk storage and block palette (PRO-850) | 2026-08-12 | `2026-08-11-chunk-storage-palette` |
| Binary greedy mesher (PRO-851) | 2026-08-12 | `2026-08-12-greedy-mesher` |
| wgpu terrain pipeline and windowed client (PRO-852) | 2026-08-12 | `2026-08-12-terrain-render` |
| Camera, player physics and collision (PRO-853) | 2026-08-13 | `2026-08-12-player-movement` |
| Headless client-input harness (PRO-873) | 2026-08-13 | `2026-08-13-client-input-harness` |
| Raycast targeting, block break and place (PRO-854) | 2026-08-13 | `2026-08-13-break-place` |
| A cell holds a block or nothing (PRO-876) | 2026-08-13 | `2026-08-13-empty-cells` |
| World persistence (PRO-855) | 2026-08-14 | `2026-08-13-world-persistence` |
| Minimal HUD — crosshair, held block, debug overlay (PRO-856) | 2026-08-14 | `2026-08-14-minimal-hud` |
| A resumed save is rendered from the first frame (PRO-900) | 2026-08-15 | `2026-08-14-loaded-save-render` |
| Ship the LICENSE texts the workspace declares (PRO-874) | 2026-08-15 | `2026-08-14-license-texts` |

---

## Notes

- Priorities: P0 = Critical, P1 = High, P2 = Medium, P3 = Low
- Features move to specs via `/sdd-start`; branches are `feature/PRO-123-short-name`
- Completed specs are recorded in `specs/REGISTRY.md`
- Default rigor is `high` — see `product/mission.md`
- No pull requests; `origin` is a backup remote. See `standards/global/git-workflow.md`.
- `/sdd-complete` runs after every spec — that is what keeps `docs/` as-built.
