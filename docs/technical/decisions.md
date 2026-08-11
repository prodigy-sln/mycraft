# Architecture Decision Records

Sources: `PLAN.md` (research, 2026-08-11); ADR-011 and ADR-012 consolidated from SPEC-002.

Each record states a decision that is **binding now**. Status `Accepted` means the decision governs
all new work; where implementation has not yet landed, that is noted explicitly. Superseding a
record requires a new record, not an edit.

---

## ADR-001 — Rust as the implementation language

**Status**: Accepted · **Date**: 2026-08-11

**Context.** A 20 Hz authoritative tick serving 32 players has a 50 ms budget per tick. A
stop-the-world GC pause during a tick is a simultaneous visible hitch for every connected player.

**Decision.** Rust 1.97, edition 2024.

**Consequences.** No GC pauses. `rayon` makes parallel chunk meshing and worldgen cheap to express.
The voxel/wgpu/networking crate ecosystem is the strongest of the realistic options. Cost: slower
compile-edit-run cycles than a managed language, mitigated by `opt-level = 1` on dev profile and
`sccache`.

**Rejected.** C# / .NET 10 — real hot reload and good tooling, but GC pauses in the tick loop and a
much weaker voxel ecosystem. C++ — comparable performance, materially worse dependency and build
tooling.

---

## ADR-002 — Custom wgpu renderer, not full Bevy

**Status**: Accepted · **Date**: 2026-08-11 · **Closest call in the plan**

**Context.** Bevy 0.19 (June 2026) is genuinely capable — GPU-driven rendering, BSN scenes, editor
preview. The question was whether to adopt it wholesale.

**Decision.** Take `bevy_ecs` 0.19 standalone. Write the renderer on `wgpu` 30 + `winit` 0.30.

**Consequences.** Full control of the hot path — chunk meshing, palette storage, single indirect
terrain draw, custom light propagation, custom culling — which are custom under any engine. We give
up Bevy's free UI, audio, and asset pipeline: roughly 3–4 weeks of work spread across the project.
We avoid a breaking migration roughly every quarter on a layer we had replaced anyway.

**Rejected.** Full Bevy — faster to a playable demo, but its mesh/material/scene abstractions are
precisely what a voxel renderer bypasses.

---

## ADR-003 — Luau for scripting

**Status**: Accepted · **Date**: 2026-08-11 · **Most consequential decision in the project**

**Context.** Mods are untrusted code executing inside a 32-player authoritative server. That threat
model, not raw call performance, is the deciding constraint.

**Decision.** Luau via `mlua` 0.12, with `luau` + `vendored` features.

**Consequences.** `Lua::sandbox(true)` gives read-only stdlib and globals. `Lua::set_interrupt`
gives a per-callback instruction budget, so a runaway loop in a mod is killed rather than the
server. `Lua::set_memory_limit` caps allocation. Gradual typing gives mod authors real editor
tooling — which matters disproportionately for a scripting-first game. `mlua` is `!Send` by default;
the VM is therefore pinned to the tick thread, which we want anyway for determinism.

**Rejected.** LuaJIT — faster, but Lua 5.1, no sandbox mode, no interrupt hook; untrusted mods make
it a non-starter. JS via `rquickjs` — viable, weaker sandbox story for this threat model.

**Deferred, not rejected.** WASM via `wasmtime` 47 — strictly better isolation and any source
language, but ~164 ns/call and a compile step between saving a file and seeing the change, which
defeats the headline hot-reload requirement. Retained as a documented second backend for
compute-heavy mods; see `PLAN.md` §4.6.

---

## ADR-004 — State in Rust, behaviour in script

**Status**: Accepted · **Date**: 2026-08-11

**Context.** Live code reload is easy; not losing state across it is the hard part.

**Decision.** All runtime state — entity position, health, inventory, quest progress — lives in the
Rust ECS. Luau holds behaviour only, addressing state through handles. Mods needing their own
persistent state declare it explicitly via `mycraft.state(...)`, which is serde-backed and
Rust-side.

**Consequences.** Discarding and rebuilding the Lua VM loses nothing. Reload becomes: build a
candidate `Registry` in a scratch VM off the tick thread → validate → run the mod's own tests →
`ArcSwap::store` at a tick boundary. Failure keeps the previous registry serving. Cost: bindings
must be handle-based rather than letting scripts hold live object references.

---

## ADR-005 — The base game is a mod

**Status**: Accepted · **Date**: 2026-08-11

**Context.** "Fully customizable" is only true if the shipped content has no privileges third-party
content lacks. Retrofitted modding APIs (Minecraft) stay permanently second-class.

**Decision.** Every block, item, tool, recipe, NPC, biome, quest, and dialogue line in the base game
lives in `content/base/`, written in Luau against the public API with no privileged engine access.

**Consequences.** The API's completeness is continuously proven by the base game itself. A missing
hook is fixed in the API, never special-cased in the engine. Cost: some engine-internal operations
need a designed public binding earlier than they otherwise would. Prior art: Luanti (ex-Minetest).

---

## ADR-006 — QUIC transport

**Status**: Accepted · **Date**: 2026-08-11

**Context.** Two traffic classes with opposed requirements: bulk chunk data (reliable, large,
latency-tolerant) and entity snapshots (small, frequent, obsolete on arrival if late).

**Decision.** QUIC via `quinn` 0.11. Chunk sections on reliable streams; entity snapshots on
unreliable datagrams; block edits, chat, inventory and quests on reliable streams.

**Consequences.** Encryption, authentication, congestion control and connection migration come from
a hardened implementation rather than being reinvented. Independent streams mean bulk chunk
transfer cannot head-of-line-block gameplay traffic. TLS 1.3 secures the wire, so application-layer
auth only has to establish *who*.

**Rejected.** `renet` 2.0 — good and game-focused, but we would add encryption and congestion
control ourselves. ENet — mature, no built-in encryption, no stream multiplexing.

---

## ADR-007 — Public-key identity for accounts

**Status**: Accepted · **Date**: 2026-08-11 · **Implementation pending (Phase 3 / M4)**

**Context.** The project targets public servers. Passwords imply a credential database worth
stealing, plus reset flows and hashing policy.

**Decision.** Each client generates an `ed25519-dalek` 3.0 keypair on first run; the public key
*is* the account ID. Login is a signed challenge-response over the already-TLS'd QUIC channel.
`argon2` password login exists only as an opt-in secondary path for players moving between machines,
and is off by default. Each server is its own trust root — no central authority.

**Consequences.** No password to leak. Display names are claimed first-come and are not identity.
Accounts live in the same `redb` file as the world. Cross-server federation is not in scope but the
key is already the right primitive for it.

---

## ADR-008 — Coverage excludes GPU and binary crates

**Status**: Accepted · **Date**: 2026-08-11

**Context.** `standards/global/testing.md` sets 90% on business logic and 80% overall. wgpu pipeline
setup is largely untestable without a GPU and a live surface.

**Decision.** `mc-render`, `mc-client`, and `mc-server` are excluded from the coverage denominator
in `scripts/sdd-gate.ps1`. The renderer is verified instead by golden-frame perceptual-diff tests in
`mc-testkit`.

**Consequences.** The gate stays meaningfully green while keeping a real bar on `mc-sim`,
`mc-script`, `mc-world`, `mc-proto`, and `mc-core`, where correctness actually lives. Risk: renderer
regressions must be caught by golden frames — if that harness is weak, the exclusion hides real
defects. That is why the harness is M0, before the renderer exists.

---

## ADR-009 — Generated assets are a build-time artifact, never a runtime dependency

**Status**: Accepted · **Date**: 2026-08-11 · **Implementation pending (`tools/asset-gen`)**

**Context.** ElevenLabs (audio, voices) and fal.ai (textures, icons) are available for asset
production. The naive integration — call the API when an asset is needed — is wrong in four
separate ways: it ships an API key to every player, bills the developer per playthrough, makes the
game unplayable offline, and makes the same block texture differ between two players.

**Decision.** Asset generation is a **developer tool that runs on a developer machine**, not a game
feature.

- `tools/asset-gen` is a separate binary crate. It is not a dependency of `mc-client` or
  `mc-server`, and no vendor SDK may be referenced from any `mc-*` crate.
- Output lands in `assets/`, which is **committed to the repository**. The game reads local files
  only. A shipped client contains no API keys and makes no calls to these services.
- `assets/manifest.toml` is the source of truth: each entry declares an asset's id, kind, provider,
  and full generation spec (prompt, model, parameters, seed).

**Cost control is structural, not a matter of remembering.** The user's instruction was to keep
usage minimal and always check before regenerating, so the tool is built so that overspending
requires deliberate effort:

- Each manifest entry is hashed over its full generation spec. If the output file exists and its
  recorded spec hash matches, generation is **skipped**. Re-running the tool costs nothing.
- Regenerating an unchanged asset requires an explicit `--force` naming that specific asset.
  There is no "regenerate everything" path.
- `--dry-run` is the default posture for review: it lists exactly what would be generated and the
  estimated cost, and generates nothing.
- Every generation appends to `assets/generation-log.jsonl` — timestamp, asset id, provider, model,
  parameters, and cost estimate. Spend is auditable after the fact.
- The tool checks the fal.ai balance before a run and refuses to start if the estimated cost
  exceeds a configured ceiling.

**Consequences.** Assets are deterministic, offline, identical for every player, and paid for once.
The repository grows with binary content — acceptable, and the alternative is worse. Provider
outages cannot affect players, only asset authoring.

**Budget observability.** Both providers expose remaining budget, so the tool checks before every
run and refuses to start when the estimate would exceed the ceiling:

- ElevenLabs — `GET /v1/user/subscription` returns character count and limit (creator tier,
  300 000/month, resets monthly). Requires the key's `user_read` scope; without it the tool must
  refuse to run rather than spend blind.
- fal.ai — `GET /billing/user_balance` returns a dollar balance.

TTS bills roughly one character per character of input text. Sound-effect and music generation bill
against the same character pool at a different rate — the tool records observed cost per generation
in `generation-log.jsonl` on first use rather than assuming a conversion, so the estimator
calibrates from measurement instead of a guessed constant.

**Rejected.** Runtime generation with caching — still ships a key, still bills per new player, and
turns first-encounter latency into a gameplay problem. Committing only a manifest and generating on
first build — makes every clone cost money and every CI run non-deterministic.

---

## ADR-010 — Licensed third-party assets, in two classes

**Status**: Accepted · **Date**: 2026-08-11

**Context.** Substantial licensed asset libraries are available to this project: bundle purchases
under `\ds01\assets\GameAssets` (music, SFX, GUI kits, 2.5D sprite art), and CC0 material —
[PixVoxelAssets](https://github.com/tommyettinger/PixVoxelAssets) (`.vox` models and sprites) and
`VoxelCoreLab_Watercolor_Terrain_Textures_1024px` (16 dirt/grass/stone/water PNGs). Licences are
held for all of them. ADR-009 governs assets this project *generates*; this ADR governs assets it
*licenses*.

Two pressures collide. Restricted-licence assets should not be trivially extractable from a shipped
build. But ADR-005 makes the base game a mod with no privileged engine access — and content that
only the base game can reach is exactly the privilege ADR-005 forbids.

**Decision.** Assets are handled in two classes by licence, not one.

- **Restricted-licence assets are embedded in the binary** and referenced from script by a fixed
  string key. They are not shipped as loose files and not extractable from `content/base/`.
- **CC0 assets ship as plain files under `content/base/`**, extractable, with their `LICENCE.txt`
  copied alongside so provenance travels with the bytes.

**Binding condition — this is what keeps ADR-005 intact.** A third-party mod MUST reach assets
through the *same* script-facing API, with a key resolving from either the embedded table or from
files the mod ships. Same call site, same API, different backing store. The base game then merely
*happens* to use embedded assets for licensing reasons; it gains no capability a mod lacks. An
asset API reachable only by the base game is a Blocker, not a style note.

**Consequences.** The CC0 class is what makes this honest: because the base game itself ships loose
files, the mod-supplied resolution path is exercised by the base game rather than left as an
untested theoretical capability — the same reasoning that turned invariant 5 into a failing test
(`mc-testkit` FR-6.1) instead of prose. Embedded assets additionally gain integrity, since a texture
cannot be silently swapped.

Three practical constraints follow:

- The terrain textures are 1024 px. `mc-render` uses an **array texture**, which requires a uniform
  layer size, so they need downsampling to a single much smaller base resolution. Choosing that
  number belongs with the array-texture design, not with whoever copies the files.
- Adopting the watercolour set commits an **art direction**. That is a product decision, to be made
  deliberately rather than by default because those were the textures to hand.
- PixVoxelAssets bundles third-party tooling under **GPL** (XBRZ) alongside its CC0 art. Only the
  art is CC0. Vendoring the repository wholesale would pull GPL into this tree.

**Scope.** None of this enters MVP 1, which stays on placeholder procedural textures — real art
moves no MVP 1 exit criterion and would pull the texture pipeline ahead of the mesher and renderer
that justify it. It lands in MVP 2, where script-defined blocks make "reference a texture by key"
the actual feature being built.

**Rejected.** Embedding everything uniformly — needless for CC0 material, and it would leave the
mod-supplied path unexercised until the first third-party mod discovered it broken. Shipping
everything as loose files — cannot satisfy the restricted licences. Hardcoding asset references in
Rust beside the embedded bytes — that is a content definition in Rust and breaches ADR-005; only the
*bytes* are embedded, never what a block *is*.

---

## ADR-011 — A section's palette holds namespaced names, not runtime ids

**Status**: Accepted · **Date**: 2026-08-11

**Context.** A chunk section has to store which block occupies each voxel,
and that storage has to remain meaningful across an MVP 2 registry hot
swap — the moment `mc-script` builds a new candidate registry and
`ArcSwap::store`s it at a tick boundary. Runtime ids are registry-local and
reassigned freely whenever the definition set changes, so storing them
directly ties a section's meaning to a registry instance that may not
exist by the time the section is read again.

**Decision.** A section's palette holds `BlockName`s. `Section::block_at`
takes no registry argument at all — reading a voxel "against the wrong
registry" is not an operation the type exposes. Name-taking mutators
(`filled`, `set_block`) take a registry only as a validator, never a
translator: it answers "is this name registered?" and cannot make the
section store anything other than the name the caller supplied.
`set_block_by_id` is the one operation that translates a runtime id into a
name, confined to that single call.

**Rejected.**

- **`Section<'r>` borrowing `&'r BlockRegistry`.** Not viable at all:
  `bevy_ecs::Component` requires `Send + Sync + 'static` (ADR-004 puts all
  runtime state in the ECS), so a lifetime-parameterised section cannot be
  a component. Independently fatal on its own terms: a borrow would pin
  the registry it was built against for the lifetime of every loaded
  chunk, which an MVP 2 hot swap cannot tolerate.
- **`Section` owning `Arc<BlockRegistry>`.** `'static` and ECS-storable,
  and a wrong-registry read becomes unrepresentable — but after a hot
  swap, sections still holding the previous `Arc` keep serving the
  previous definitions, so two loaded chunks can disagree about what
  `base:stone` even is. This trades one silent-wrongness class for a
  subtler one.
- **A palette of `BlockId` plus a `RegistryId` token, validated on every
  access.** Smallest memory footprint and directly usable ids. But every
  accessor gains a mismatch-error case, export needs a fallible
  id-to-name translation pass that can itself be wrong, and after an
  MVP 2 registry swap *every already-loaded section* becomes unreadable
  until a migration pass rewrites it — a migration pass MVP 2 would have
  to design and get right, for a class of bug the chosen option removes
  outright.

**Consequences.** A registry hot swap is a no-op for already-loaded world
data, and MVP 2 needs no migration pass over loaded chunks. Export is a
pure projection of a section's own state, so there is no remapping code
that could get the projection wrong. Costs, stated plainly: a palette
entry becomes a 16-byte `Arc<str>` plus a shared allocation rather than a
2-byte id, and any consumer wanting a numeric property of a block (the
mesher, eventually) has to resolve each palette entry through the
registry rather than indexing an array directly — cheap once per palette
entry per mesh, but there is no per-voxel palette-index read path yet to
make that cheap at the per-voxel level (see `technical/world-format.md`).
The representation is private behind a newtype, so replacing `Arc<str>`
with an interned id later is a one-file change if it ever becomes
necessary. The remaining price is the de-registered-name failure class
described in `technical/world-format.md`: a section can end up holding a
name its current registry no longer recognises, and every solidity check
against it then fails.

---

## ADR-012 — The block registry contract lives in `mc-core`

**Status**: Accepted · **Date**: 2026-08-11

**Context.** Something has to own the block registry's contract — the
namespaced name, block definition, runtime id, and how definitions get
registered. MVP 1's chunk storage needs it; MVP 2's Luau scripting host
will need to populate the very same registry, which fixes the question of
where it can live without inverting the workspace's inward dependency
rule.

**Decision.** The contract — `BlockName`, `TextureKey`, `BlockDefinition`,
`BlockId`, `BlockRegistry`, and the `DefinitionSource` port — lives in
`mc-core`. Chunk storage and the file-backed TOML loader live in
`mc-world`, which depends on `mc-core`. `mc-core` performs no I/O and
depends on nothing else in the workspace.

**Rejected.**

- **The contract in `mc-world`.** Forces `mc-script → mc-world` in MVP 2,
  because the scripting host must populate the same registry chunk
  storage reads from — inverting `CLAUDE.md`'s inward-dependency rule and
  dragging chunk storage, worldgen, and eventually `redb`-backed
  persistence into the scripting host's dependency graph. It also cannot
  keep `toml` out of the contract-owning crate's graph by construction,
  since the loader that parses TOML would live in the same crate as the
  contract it populates.
- **A new `mc-registry` crate.** Would satisfy the dependency-direction
  concern, but adds an eleventh crate to a workspace whose ten-crate map
  is deliberately fixed, for no capability `mc-core` does not already
  provide — `mc-core`'s own purpose is exactly to hold primitives and
  contracts other crates share.

**Strongest argument against the decision taken, stated honestly:**
`mc-core` is meant to hold primitives, and a registry is a stateful
container with a registration lifecycle — not a primitive by any
reasonable reading of the word. Putting mutable state in the primitives
crate is a real smell, and it would compound if `mc-core` gained further
stateful services over time. Accepted anyway, because the crate holds only
the **contract** — what a definition is and how a name resolves to one —
and populating that contract is done entirely from outside it, through the
`DefinitionSource` port; `mc-core` itself never learns where a definition
came from.

**Consequences.** `mc-world → mc-core` is the only edge between the two
crates. MVP 2's `mc-script` can depend on `mc-core` alone to populate the
registry, without any dependency on chunk storage, worldgen, or
persistence. `toml` is absent from `mc-core`'s entire resolved dependency
graph, mechanically asserted (`technical/architecture.md`).
