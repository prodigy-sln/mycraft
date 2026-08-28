# Architecture Decision Records

Sources: `PLAN.md` (research, 2026-08-11); ADR-011 and ADR-012 consolidated from SPEC-002; ADR-013
from SPEC-004; ADR-016 from SPEC-009.

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
gives a per-callback budget, so a runaway loop in a mod is killed rather than the server. Gradual
typing gives mod authors real editor tooling — which matters disproportionately for a
scripting-first game. `mlua` is `!Send` by default; the VM is therefore pinned to the tick thread,
which we want anyway for determinism.

**Two claims in this record were corrected against measurement once the host was built.** The
decision stands; these are factual repairs to its consequences, not a change of direction.

- **The budget is not an instruction budget.** The Luau interrupt is emitted at seven opcodes —
  calls, returns and loop edges — and at nothing else. A loop body of any size is free, a thousand
  straight-line statements cost one, a call within Luau costs two and a call into the host costs
  one. Sizing it against VM instructions is wrong by the size of the loop body. It is a
  call-and-loop budget and is named that everywhere the host uses it.
- **`Lua::set_memory_limit` does not cap allocation on its own.** The allocator-raised error is an
  ordinary catchable Lua error: measured, under a 1 MiB limit with no other mechanism, a
  `pcall`-wrapped allocation bomb caught it, dropped the table, let the collector reclaim it and
  looped ten times before returning normally. What enforces the per-invocation cap is the interrupt
  reading `Lua::used_memory()` each tick against the baseline the invocation started from, and
  latching — once a limit trips, no further script frame runs, including the handler that would
  have caught it. `set_memory_limit` is set *above* the enforced cap as an absolute backstop, so a
  single allocation large enough to jump the gap between two interrupt ticks still fails rather
  than sailing past. It bounds peak allocation; it does not by itself contain.

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

**Status**: Accepted · **Date**: 2026-08-11 · **Narrowed in part by ADR-013**

**Context.** `standards/global/testing.md` sets 90% on business logic and 80% overall. wgpu pipeline
setup is largely untestable without a GPU and a live surface.

**Decision.** `mc-render`, `mc-client`, and `mc-server` are excluded from the coverage denominator
in `scripts/sdd-gate.ps1`. The renderer is verified instead by golden-frame perceptual-diff tests in
`mc-testkit`.

**Consequences.** The gate stays meaningfully green while keeping a real bar on `mc-sim`,
`mc-script`, `mc-world`, `mc-proto`, and `mc-core`, where correctness actually lives. Risk: renderer
regressions must be caught by golden frames — if that harness is weak, the exclusion hides real
defects. That is why the harness is M0, before the renderer exists.

**Record note (2026-08-12).** ADR-013 narrows this record's `mc-render` exclusion to that crate's
GPU-resident subtree, `crates/mc-render/src/gpu/`. Everything above stands as written about
GPU-resident work and about `mc-client` and `mc-server`; only the extent of the `mc-render`
exclusion changed, and only because a compile-enforced boundary now exists that did not when this
record was made.

---

## ADR-009 — Generated assets are a build-time artifact, never a runtime dependency

**Status**: Accepted · **Date**: 2026-08-11 · **Implementation pending (`tools/asset-gen`)** ·
**Narrowed in part by ADR-026**

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

**Record note (2026-08-18).** ADR-026 narrows this record's "output lands in `assets/`, which is
committed to the repository" to the class of output this record was actually reasoning about:
provider output that cannot be reproduced. Everything above stands as written about ElevenLabs and
fal.ai assets, about cost control, and about the game reading local files only; what changed is that
"generated" stopped being the property that decides whether output is committed, because a
deterministic and free generator now exists that this record's reasoning never covered.

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

---

## ADR-013 — Coverage excludes the GPU-resident subtree, not the whole renderer

**Status**: Accepted · **Date**: 2026-08-12 · **Supersedes ADR-008 in part**

**Context.** ADR-008 excluded `mc-render`, `mc-client` and `mc-server` from the coverage
denominator on the grounds that "wgpu pipeline setup is largely untestable without a GPU and a live
surface". That was and remains correct about wgpu pipeline setup: creating an instance, enumerating
adapters, requesting a device, allocating buffers and textures, building pipelines, encoding passes,
submitting and presenting cannot be exercised without a device, and golden-frame perceptual diffing
is the right verification strategy for them. What changed is that the exclusion was written **by
path**, and `mc-render` is no longer only pipeline setup. `crates/mc-render/CLAUDE.md` already
stated the rule the path-based exclusion contradicted — anything expressible as a pure function is
unit-tested normally and is not exempt, only GPU-resident work gets the exclusion — and the terrain
renderer put real substance behind that sentence: quad → vertex/index geometry and winding, vertex
bit packing and its range refusals, frustum-plane extraction and frustum ∩ AABB visibility,
texture-key → array-layer resolution, procedural placeholder texel generation, sRGB → linear
conversion, surface-format selection, resize and depth-reallocation policy, surface-error →
frame-action policy, window-event → action policy, and the device-request description. All of it is
pure, all of it is unit-tested, and under a by-crate exclusion none of it was counted. A coverage
figure that omits every tested line and reports on none of them vouches for nothing while appearing
to.

**Why this is possible now and was not when ADR-008 was written.** ADR-008 predates any `mc-render`
code, so there was no boundary to draw the exclusion at other than the crate. There is one now: a
default-on `gpu` Cargo feature under which `wgpu::` is nameable only inside
`crates/mc-render/src/gpu/`, so `--no-default-features` removes wgpu from the resolved dependency
graph entirely and a stray `use wgpu::` in the pure layer is a build error. That subtree is a
mechanical, compile-enforced boundary rather than a naming convention, which is what makes a
narrower exclusion honest rather than aspirational. `mc-testkit` has carried the identical seam
since the frame harness landed.

**Decision.** The exclusion is the GPU-resident subtree of `mc-render`, not the crate.
`$CoverageExclude` in `scripts/sdd-gate.ps1` is

```
crates[/\\](mc-client|mc-server)[/\\]|crates[/\\]mc-render[/\\]src[/\\]gpu[/\\]|_test\.rs$
```

- `crates/mc-render/src/gpu/**` — excluded. Instance, adapter, device, surface, buffer and texture
  allocation and upload, pipeline creation, pass encoding, submission, present. Verified by golden
  frames and by golden-independent derived probes.
- Everything else in `crates/mc-render/` — **counted**, at the workspace's ordinary 80% line
  threshold.
- `mc-client` and `mc-server` — excluded, unchanged. `mc-client` holds only the `winit` event-loop
  adapter and composition wiring; every policy it would otherwise have owned lives in `mc-render`'s
  pure layer precisely so that it is counted. If logic ever accretes in `mc-client`, that is a new
  record, not a quiet edit to this regex.
- The `_test.rs$` clause and the reasoning recorded above it in the gate script are untouched.

The gate's GPU-free stage runs `cargo clippy -p mc-render --no-default-features` and
`cargo nextest run -p mc-render --no-default-features` alongside the same pair for `mc-testkit`, so
the seam this record depends on is a checked fact at every gate run rather than a convention that
can rot.

**Consequences.** The coverage percentage vouches for the renderer's pure layer, which is where a
maths bug — a sign inversion in a frustum plane, a truncating vertex pack, a layer index resolved
to zero — actually lives. Golden frames remain the only automated check on GPU-resident work, so
ADR-008's central bet is unchanged and its risk statement still applies: if the frame harness is
weak, the exclusion hides real defects. The renderer strengthens that bet from the other side with
derived probes that assert properties of a captured frame against computed expectations rather than
against a committed image — a golden re-shot from a broken renderer is a golden of a broken
renderer, and only an assertion that does not come from the renderer can catch that. Adding a pure
module to `mc-render` now carries the same testing obligation as adding one to `mc-world`.

**Rejected.**

- **Leaving ADR-008 as written and backing the pure layer with named scenarios only.** The honest
  short-term answer, and rejected because scenarios bind only their own spec: the next renderer
  feature would add pure, uncounted code with no scenario obliging it to be tested, and the gate
  would report a figure that silently excludes it. The exclusion needs to describe the boundary that
  actually exists, not the crate that used to be a proxy for it.
- **Excluding by feature rather than by path**, counting only the `--no-default-features`
  configuration. Cleaner in principle, but `cargo llvm-cov` filters filenames, not cfg
  configurations, and a second instrumented run purely to produce a denominator would double the
  slowest gate stage for no additional signal.
- **Dropping the exclusion entirely** and letting golden frames raise the number. Golden tests
  execute GPU code without asserting over its lines in any way a coverage tool understands; the
  figure would rise while meaning less.
- **Narrowing `mc-client` at the same time.** Nothing puts logic there, so the change would be
  speculative. Revisit when something does.

## ADR-014 — Movement is submitted as an intent, never a position

**Status**: Accepted · **Date**: 2026-08-13

**Context.** SPEC-005 gave the player a free-look camera, WASD movement and collision. The client
has to tell the simulation what the player is asking to do, and there are two shapes that message
could take: the client computes where the player ends up and tells the simulation the result, or the
client says what it is *asking for* and the simulation computes the result itself. Invariant 4 ("the
server is authoritative — client input is a request, never a fact") already rules on this in
principle; this feature is where it first has a concrete message to apply to, and singleplayer is
where the rule is easiest to skip, because there is no network yet to make a submitted position look
suspicious.

**Decision.** `MovementIntent` carries exactly five fields — `forward`, `strafe`, `yaw_delta`,
`pitch_delta`, `jump` — a direction, a magnitude, a change of view and whether a jump is wanted.
There is no field it could use to state a position, a velocity or an absolute orientation, so a
client cannot claim where it is even by mistake. `advance_player` is the only path from an intent to
a new `PlayerState`, and it clamps on the receiving side before doing anything else with the intent:
magnitude capped at 1, non-finite values rejected, per-axis displacement capped at 1.0 block — so a
client sending `1000.0` or a `NaN` gets exactly the same treatment as a well-behaved one.
`InputState` (held keys, pending look delta, `take_intent()`) accumulates the intent and lives in
`mc-sim` rather than in `mc-client`, because it is *client-side behaviour that happens to live in the
authority's crate*: in MVP 3 the client still runs this same accumulation and sends the resulting
`MovementIntent` over the wire, and the server still clamps it on receipt, unchanged.

**Rejected: the client computes and submits a position (or a velocity).** Cheaper today — no
sanitising, no per-axis clamp, the simulation just copies the number in — and wrong for exactly the
reason invariant 4 exists: a modified client could submit any position or velocity at all, and
nothing downstream would have grounds to doubt it. It also does not survive contact with MVP 3: a
server that trusts a submitted position has no work left to do except forward it, which is not
authority, it is relay. Building the trusted-intent shape now, while the "network" is a same-process
function call, means MVP 3 is a transport swap — the client still sends a `MovementIntent`, a server
still owns `advance_player` — rather than a rewrite discovered under replication.

**Consequences.** Every physics scenario is stated against `advance_player` and never against a
client-reported outcome, so the physics is fully testable without a client, a window, or a world
(`crates/mc-sim/tests/`). What is *not* reachable that way is the other side of the seam — whether
the client actually wires its keys and pointer into the intent it submits — and that gap is real
rather than theoretical: it is why the manual acceptance in `docs/technical/testing.md` exists and
why PRO-873 owns its executable closure. The cost is on the simulation's side: it must sanitise an input
it does not control, and every one of the four rejection scenarios (non-finite forward/strafe,
non-finite look, over-deflected magnitude, over-length displacement) is a scenario that would not
exist if the client were trusted. Nothing about this decision is specific to singleplayer — the
clamp is exercised on every tick today, not added when a network arrives.

---

## ADR-015 — The editable world keeps a mirrored collision view, resolved through one private write path

**Status**: Accepted · **Date**: 2026-08-13

**Context.** Raycast targeting, block break and place (SPEC-007) gave the simulation its first
editable world. Before it, `Simulation` held `Box<dyn Solidity + Send>` over a `SolidVoxels` bitset
resolved once at construction; nothing in the workspace mutated a world after building it. Editing
needs the block store and the collision view — used by the raycast and by player physics — to agree
after every write, and something has to guarantee that.

**Decision.** `mc_sim::world::World` owns the block store, a `SolidVoxels` bitset, the dirty-section
set and the registry they were all resolved against, keeping all four private. Exactly one private
function, `write`, can mutate either the store or the bitset, and it always writes both together:
resolve the block's solidity once, write the store, write the bit, mark the section dirty. The two
domain operations, `break_at` and `place_at`, live in the same module and call `write`; action
resolution lives in a **child** module so it inherits access to `write` without widening its
visibility. Physics keeps taking `&dyn Solidity`, unaffected.

**Rejected — deriving solidity from the store on every query, with no bitset at all.** Genuinely
attractive: it makes a store/collision-disagreement defect **unspellable by construction**, since a
single source of truth has no second view to disagree with, which is the kind of structural
elimination this codebase generally prefers over a test guarding the same property. What rules it out
is a specific committed test it would silently destroy: the replay's overlap oracle judges the
physics by re-deriving overlap from the world's own per-voxel accessor and the registry directly,
sharing no lookup chain with the physics' own resolved bitset — deliberately, so an adapter bug in the
bitset cannot make both sides wrong the same way. Deriving solidity from the store on every query
would make that oracle's lookup chain *identical* to the code path it exists to check: it would pass
forever regardless of what broke, silently, and even its own positive control (a box placed inside the
world's landmark) would keep passing. The trade is a whole-run, adapter-independent invariant on one
side against a whole defect *class* made unspellable on the other — decided in favour of keeping the
oracle's independence, with a private invariant standing in for the structural guarantee the rejected
option would have bought. A reviewer weighing whether to revisit this should read it as this trade,
not as an oversight.

**Consequences.** A store/collision disagreement (the deleted-write defect this design closes) is
caught by fixture-scoped scenarios that walk a player through the affected cell — costing four such
scenarios their ability to fail if the rejected option were chosen instead, since both views would
already be provably one. A world-wide bitset sized at construction is also the one structure that
MVP 3's chunk streaming cannot keep as-is; moving to a per-section bitset is a change confined inside
`SolidVoxels` and is deferred until the world footprint stops being fixed.

**Amendment (SPEC-021, splitting `solid` into four declared properties).** The decision above is
recorded as it was taken and is not rewritten; what follows is what has changed since, and the
decision was **extended rather than reversed**.

`SolidVoxels` is now **`ResolvedVoxels`** — the type answers two questions and should not be named
for one of them. It carries **two** bitsets: one for what stops the player, read through `Solidity`,
and one for what a ray may stop at, read through a second narrow trait `Targetable`. Content declares
the two independently (`base:water` stops nobody and a swing finds it), so neither may be derived
from the other. `write` resolves the block once and settles **both** answers before it touches the
store; `set` takes both in one call, so a caller able to write one without the other is unspellable.

**This ADR's principle held under exactly the pressure that would have broken it.** What was decided
here was not "one bitset" but a mirrored view kept in step through **one private write path**. A
second mirrored view was added, and it went through that same path — the same two functions, `write`
and `adopt`, and no third. The count changed; the guarantee did not, and the doc comments that name
it now read "the one place **any** view is written" where they read "either".

The deferred per-section change above is unaffected in kind and doubled in size: both bitsets are
world-wide and sized at construction, and both move together when the footprint stops being fixed.

---

## ADR-016 — A save is one `postcard`-encoded plain file, replaced atomically, not a `redb` database

**Status**: Accepted · **Date**: 2026-08-13

**Context.** A save has to hold a world's blocks and the player's place in it, written once at quit
and read back once at launch — never incrementally, never per-chunk, never while the game is
running. The file is also attacker-controlled input to the authoritative process: every length,
count and identifier read out of it has to be checked before anything indexes on it. Two questions
had to be settled together: what container the save lives in, and what turns its structure into
bytes.

**Decision.** A save is one plain file, written to a sibling temporary file, flushed with
`File::sync_all`, and moved into place with `fs::rename` — never a `redb` database, and never an
in-place overwrite. Its contents are encoded with `postcard` 1.1.3 over plain `serde` DTOs.

**The encoder was chosen against four criteria, applied in this order, because each one is a hard
requirement the criteria after it cannot outweigh:**

1. **Streaming, one value at a time.** A save has to report the complete set of block names it
   requires without reading any of its chunk data, which means the reader must be able to decode
   the name table and stop — never load the whole file to answer a question about its first part.
2. **Bounded allocation on untrusted input.** A length prefix in the file must never drive an
   allocation before the bytes behind it have actually arrived.
3. **`serde`-based, preferred but not required.** `serde` is already a dependency for the content
   format, so a `serde`-based encoder costs one new crate rather than a second derive ecosystem.
4. **Maintenance status, checked against the advisory database rather than against impressions.**

`bitcode` and `rkyv` fail criterion 1 outright — both need the whole buffer in memory before
anything is readable, which makes "report what a save needs without reading its chunk data"
literally false under either. `wincode` passes criteria 1 and 2 — it has an explicit
`PreallocationSizeLimit` that is, if anything, a more direct answer to criterion 2 than `postcard`'s
own mechanisms — but fails criterion 3 (its `Serialize`/`Deserialize` are its own traits, not
`serde`'s; adopting it means a second derive ecosystem) and answers criterion 4 worse than
`postcard` for a format that defines bytes on disk: it is pre-1.0, with no wire-format stability
commitment yet. `postcard` passes all four: it decodes one value from a `Read` and hands the reader
back positioned after it; it bounds allocation structurally (below); it uses plain `serde` derives;
and it is actively maintained, with a wire format frozen and specified independently of the crate,
at 1.0.

**`postcard`'s allocation story is structural, not a configured limit, and it differs from what it
replaced in one respect that mattered enough to change a constant.** A declared length never drives
an allocation ahead of the elements actually arriving — a `Vec` grows only as elements are read.
Byte-shaped fields (block names) decode into a caller-supplied scratch buffer and are refused
outright, allocating nothing, if the declared length will not fit. What this bounds is **bytes
read**, not the memory those bytes expand into — measured at up to ~48× amplification for the
save's own record shapes — so the file-length precheck that converts a read bound into a memory
bound had to be re-derived rather than carried across unchanged (`technical/world-format.md` carries
the arithmetic).

**`postcard` was not the first choice. `bincode` 2.0.1 was, and it was removed after being found
permanently unmaintained.** The first evaluation ranked candidates against criteria 1 through 3
only — maintenance status was not among them, which is precisely the gap that let a discontinued
crate be selected on its remaining merits. `cargo deny`'s advisories stage subsequently failed on
RUSTSEC-2025-0141: `bincode`'s maintainers ceased development permanently, with no safe upgrade
available. Removal beat suppression because the two were not equally costly: no save format had
shipped yet and no world existed anywhere, so replacing the encoder was a same-day refactor rather
than a migration, and a suppressed advisory would have left a dependency nobody could fix defining
every byte this feature ever writes. Maintenance status was added to the criteria list above as a
direct result — the criterion its absence had cost.

**Departing from `redb` was re-argued from first principles rather than carried over, and two of
its original three supporting arguments did not survive re-examination.** The first — that a
hand-decoded byte format gives sharper diagnostics than a paged database can — was largely an
artefact of a hand-written codec that was itself abandoned in favour of `postcard`; the diagnostics
that survive are semantic checks over already-decoded values, and those are available under any
container. The second — that avoiding a container avoids being the first thing hostile bytes meet —
does not distinguish `redb` from any other library, since whichever encoder is chosen meets the
bytes first regardless. What stands, and what the decision now rests on: every feature that
distinguishes `redb` from a plain file — incremental writes, per-chunk access, transactions — serves
an access pattern this feature does not have, since a save is read and written whole, once, at
launch and at quit. And `redb` does not substitute for an encoder; a `redb` value is still a byte
blob, so choosing `redb` means adopting `redb` *and* an encoder — two dependencies for an access
pattern that needs neither's distinguishing feature.

**Consequence for ADR-007.** ADR-007 records "Accounts live in the same `redb` file as the world"
as a consequence of its own decision. Removing the world's `redb` file leaves that sentence with
nothing to co-locate with. ADR-007's actual decision — `ed25519-dalek` keypairs, the public key as
account id, signed challenge-response over QUIC, `argon2` as an opt-in secondary path, each server
its own trust root — is untouched by this and is not re-argued here. What is superseded is that one
consequence alone: accounts get their own store when they are built, which may still be `redb` — a
transactional database is a reasonable shape for an account table, and nothing here argues
otherwise. **ADR-007 itself is not edited; this paragraph is the record of the supersession.**

**Consequences.**

- The on-disk format is defined by reference to `postcard`'s own published wire specification
  (frozen and versioned independently of the crate since 1.0), not by the behaviour of a particular
  library release. A `postcard` major-version upgrade whose wire format differs is therefore a
  **format-version decision** for the save format, to be treated with the same care as introducing a
  second save-format version — never a routine dependency bump.
- `postcard::` is nameable only inside `mc_world::persistence`, and every decode failure collapses
  to one variant at that module's edge (`technical/architecture.md`). That confinement is what kept
  the `bincode` removal to four files and five call sites and zero test changes, and it is expected
  to do the same for whatever supply-chain event comes next — though a future swap after real saves
  exist stops being a refactor and becomes a migration, since bytes already on disk would then
  depend on the encoder being replaced.
- Blast radius of the container choice, if `redb` is ever needed sooner than the access pattern
  above implies: confined to `mc_world::persistence`'s implementation behind its public functions,
  plus a format-version bump. Nothing in `mc-sim` or `mc-client` names a byte layout directly.

**Rejected.** `bitcode`, `rkyv` — cannot stream a value at a time, which makes a load-time
requirement false under either regardless of their other merits. `wincode` — not `serde`, and
pre-1.0 while defining the bytes of a file meant to outlive its own crate version; deferred rather
than ruled out permanently, revisited if it reaches 1.0 with a stability commitment before any save
format has shipped. `redb` (or any transactional container) — solves nothing this access pattern
needs and does not eliminate the need for an encoder; deferred to whichever spec introduces
incremental, per-chunk or streamed persistence, which is the access pattern it is for. A
hand-written codec — vetoed before this record settled: parsing untrusted bytes is a discipline a
widely-used, adversarially-tested library has orders of magnitude more experience at than one
feature's implementation can produce.

---

## ADR-017 — `egui` for the debug overlay, without `egui-winit`, with the project's first per-crate licence exception

**Status**: Accepted · **Date**: 2026-08-14

**Context.** The debug overlay (SPEC-010) has to render four lines of text — position, column,
frame rate, frame time — and nothing in `mc-render` can draw a glyph. `egui`, `egui-wgpu` and
`egui-winit` were pinned at 0.36.1 in `[workspace.dependencies]` and **resolved by nothing**;
`crates/mc-render/CLAUDE.md` already reserves `egui` for debug and tooling UI, and the issue names
it. Taking it is therefore not a technology *search* but a decision about a fresh dependency edge
that two gate stages will see: `cargo machete` fails a declared-but-unused dependency, and
`cargo deny` runs advisories, licences, bans and sources with `all-features = true` across three
targets. The alternative to a toolkit is a glyph atlas, which needs its own determinism argument
before any frame containing text can be trusted, and which the spec puts out of scope.

**Verified by a real resolve rather than by reading manifests.** All three crates exist at 0.36.1
with `rust-version` 1.95 ≤ 1.97; `egui-wgpu` 0.36.1 declares `wgpu = "30.0"` and
`winit = "0.30.13"`, which are exactly the workspace pins; a scratch crate outside the repository
resolved 275 packages with `wgpu 30.0.0` and `winit 0.30.13` unchanged; `Renderer::{new,
update_buffers, render}` are unconditional and carry no `compile_error!`, so this is not a stub
whose useful half is feature-gated. Twenty-six new crates enter the graph — egui's own
(`emath`, `ecolor`, `epaint`, `accesskit`) and the Linebender/Google-Fonts lineage (`skrifa`,
`read-fonts`, `harfrust`, `kurbo`, `peniko`, `vello_common`, `vello_cpu`, `fearless_simd`) — all
actively maintained, none advisory-flagged.

**Decision.** Take `egui` with default features (for `default_fonts`) and
`egui-wgpu` with `default-features = false`, both **optional under `mc-render`'s existing `gpu`
feature** so that `--no-default-features` keeps the dependency-graph seam green, and both added in
the phase that first uses them. Do **not** take `egui-winit`. Add one per-crate licence exception
for `epaint_default_fonts`. Confine every `egui::` and `egui_wgpu::` path to
`crates/mc-render/src/gpu/overlay.rs` — the litmus being that egui disappearing changes one file.

**`egui-winit` is refused, and its unused pin is *disarmed* rather than deleted.** Its `default`
features include `winit/default`, which re-enables `wayland-csd-adwaita` → `sctk-adwaita 0.10.1` →
`ab_glyph` → `owned_ttf_parser` → **`ttf-parser 0.25.1`** — unmaintained with no safe upgrade, and
the exact crate the workspace's `winit` entry was hand-trimmed to remove
(`technical/testing.md` §"Supply chain"). `cargo deny check advisories` **failed** on that graph,
naming `egui-winit` as the second parent of `winit`, and **feature unification is additive, so
`mc-client`'s own `default-features = false` on `winit` cannot undo it.** The pin therefore gains
`default-features = false` plus a dated comment mirroring the `winit` entry's: deleting the pin
would lose the knowledge, and leaving it bare arms the trap for whoever opts in next. Nothing is
lost by refusing it — the overlay is non-interactive and constructs `egui::RawInput` directly — and
refusing it also keeps `winit` out of a second `mc-client` file, which
`crates/mc-client/tests/winit_boundary.rs` fails the build over, and out of `mc-render` entirely.

**The licence exception, measured before it was written.**

```toml
[[licenses.exceptions]]
name = "epaint_default_fonts"
allow = ["OFL-1.1", "Ubuntu-font-1.0"]
```

This is the **first per-crate exception in `deny.toml`**, and it is per crate rather than a global
allowlist entry because `deny.toml`'s own rule says an exception is a decision about one dependency
and not a hole in the allowlist — a font licence does not generalise to code. Three measurements
decided its shape:

- **The mechanism.** The licence is `(MIT OR Apache-2.0) AND OFL-1.1 AND Ubuntu-font-1.0`: the code
  half is satisfied by the allowlist and the two font licences are **ANDed onto it**, so no choice
  of branch escapes them and only an exception can admit the crate.
- **It is *not* technically unavoidable, and saying otherwise would be overclaiming.** With
  `egui = { default-features = false }` the crate leaves the lock entirely (156 → 155 packages) and
  licences go `ok`. But `epaint`'s `FontDefinitions::default()` is `Self::empty()` without that
  feature, so the overlay would lay out four lines and rasterise **nothing**. The honest form of
  "unavoidable" is conditional, on three things: the overlay must render text, **this repository
  ships no font**, and vendoring one is rejected below.
- **The premise that a font was already here was false, and was checked rather than argued with.**
  `git ls-files` matches no `.ttf`, `.otf`, `.ttc`, `.woff`, `.pcf`, `.bdf` or `.fon` anywhere; the
  only committed binary assets are the golden PNGs; `include_bytes!` appears nowhere and the two
  `include_str!`s are WGSL shaders.

**Rejected — vendoring a font and dropping `default_fonts`.** The reason this lost is recorded in
full, because a reviewer re-deriving it will find the alternative **cheaper than it was first
written down as being**: of the four fonts the crate bundles, `Hack-Regular.ttf` is MIT plus
Bitstream Vera — both permissive, MIT already on the allowlist — at 309 KB of the crate's ~1.4 MB,
and the two offending licences come from Ubuntu-Light (UFL) and NotoEmoji (OFL), neither of which
the overlay uses. So "a new licence question" understates how tractable it is. **What decides it
instead is visibility.** A `.ttf` in this tree is not a crate, so no gate stage would ever read its
licence again, and Bitstream Vera's reserved-font-name clause — a real distribution condition —
would then be held by nobody. `cargo deny` re-reads the exception on **every** gate run, names the
crate, names both licences, and carries the reason beside them. This project holds every other guard
mechanically, and has already recorded two "nothing mechanical prompts the entry" hazards as known
holes rather than as designs to copy (`technical/architecture.md` §"Mechanically enforced
invariants", `technical/rendering.md` §"The HUD's derived prediction"); moving a licence obligation
out of the one place that re-checks one would be a third.

**Rejected — taking no toolkit at all.** A hidden-by-default tooling overlay is the sole consumer of
26 crates and ~1.4 MB of font data, which is a real argument against. It loses because the only
alternative is a glyph atlas with its own determinism argument, which is out of scope, and because
the pin and the reservation for debug UI both already existed.

**Consequences.**

- **Binding: egui's glyph rasterisation must never reach a committed golden.** Drivers legitimately
  disagree about glyph rasterisation, so the first golden to hold text makes whatever produced it
  the ground truth every machine must then reproduce. The overlay is hidden by default, no declared
  capture is taken with it shown, and the assertion that content cannot obscure it is a **difference
  between two frames**, never a golden. The price is that the overlay's readout *content* is graded
  by no automated check — an adapter painting a fixed string passes every test — recorded in
  `technical/testing.md` and filed as **PRO-897**.
- **`RendererOptions::PREDICTABLE`, not `default()`.** `default()` enables dithering — noise applied
  to values falling between two 8-bit steps, to hide banding in gradients — and disables the
  software texture filtering that option set exists to provide. Four lines of text have no
  gradients, so dithering could only contribute nondeterminism to the one crate whose verification
  story is frames a second machine reproduces, and target hardware spans an RTX 4090 and an Intel
  UHD 770. **Nothing grades this choice**: restoring `default()` leaves the whole suite green,
  because the only oracle over the overlay's pixels compares two frames of one run and dithering is
  a deterministic function of the fragment's value and position, so it cancels. Taken on judgement,
  recorded as earned by no test.
- **The one-file confinement paid for itself immediately.** The first implementation applied epaint's
  texture deltas and never consumed them; `TexturesDelta` asserts on drop that somebody did, so
  three frame scenarios failed with a **panic in the render loop** — which
  `crates/mc-render/CLAUDE.md` bans outright. The fix (clear the delta after both halves are
  applied, freeing *after* the pass rather than before, because a texture this frame stopped using
  may still be sampled by the draw this frame recorded) touched that one file.
- **The exception carries a live obligation, and only a public-domain face can retire it.**
  **PRO-894** owns it. A permissively-licensed-but-conditioned face does not clear it — Bitstream
  Vera's reserved-font-name clause is precisely the kind of condition that would move back to being
  held by a human — so the exit is a face with no conditions at all, or no bundled fonts.
- The overlay renders as egui's stock light grey (140) directly over terrain with no panel behind
  it: legible against sky, marginal against bright ground. This is within spec — the contrast rule
  is about *content* HUD elements, and the stock look is explicitly permitted for
  hidden-by-default tooling — and is filed as **PRO-898** rather than fixed mid-flight.

## ADR-018 — VoxForge's preview renders every view through one DDA ray march and one camera-basis formula

**Status**: Accepted · **Date**: 2026-08-15

**Context.** `tools/voxforge` (SPEC-013) previews a `.mcvox` model from fourteen fixed orthographic
views — six axis-aligned, eight isometric. Three shapes were open: (a) one per-pixel DDA ray march
serving every view; (b) a painter's algorithm — depth-sort voxels, rasterise each as a screen quad;
(c) a fast direct scan for the six axis views, DDA only for the isometric ones. Correctness of
orientation is this tool's dominant risk: `technical/rendering.md` already records that a silently
row-flipped capture path would have made every golden wrong in the same invisible direction, and
here it is worse — an agent self-corrects *against* the preview, so a mirrored view makes it "fix"
correct geometry to match a broken picture.

(b) was first rejected on the wrong ground (assumed O(voxels) cost) and the real objection is
orientation: an orthographic depth sort is a per-axis sort key, and a wrong sign produces a
*plausible* picture assembled from far faces rather than an obviously blank one — the same class of
invisible defect as a row flip, with better camouflage. (c) is rejected for the reason this feature
exists at all: two code paths producing "the same" picture is two chances to get orientation wrong,
with the cheap path drawing the most test coverage while the isometric path carries the subtler bug.

**Decision.** One DDA ray march serves all fourteen views. Its first hit is correct by construction
and yields the face normal shading needs, with no depth-sort step to get a sign wrong. All fourteen
camera bases derive from one formula rather than being fourteen hand-written cases: given a unit view
direction `d` and an up-hint `w`,

```
right = normalize(d × w)
up    = right × d
```

with `w = (0, 1, 0)` for every view except `top` and `bottom`, where `d` is parallel to it and `w`
is chosen orthogonal instead. A pixel at `(col, row)` samples along `right` and `−up`, which makes
image row 0 the top **by construction**, not by a flip applied afterward.

**Consequences.** The four "under" isometric corners — added after the first real asset review
because no view could show the underside of a horizontal surface — cost no new derivation: the same
formula absorbed all four with no eleventh up-hint and no special case, which is the strongest
evidence the shape was right. Whether "isometric" means true isometric (equal foreshortening on all
three axes) or 2:1 dimetric (integer pixel steps, why pixel art conventionally uses it) is
**deferred**, defaulting to true isometric; revisit once real previews are read at working
resolution — a two-way door affecting no interface. The isometric AABB is larger than the axis
views' — roughly 724 × 836 px at 8 px/voxel for a 64³ model, not the ≤512×512 an early estimate
assumed — which the dense-volume representation of ADR-019 keeps fast enough for a sub-second
edit-preview loop.

## ADR-019 — VoxForge's assembled model is a dense positional array, never a hash- or tree-keyed map

**Status**: Accepted · **Date**: 2026-08-15

**Context.** VoxForge's preview ray march (ADR-018) and its `inspect` connectivity check both walk
an assembled model's voxels by position, and its byte-identical-PNG requirement (deterministic
output regardless of a document's declaration order) forbids any iteration order that depends on
hashing. A first draft keyed the assembled volume as `BTreeMap<UVec3, MaterialKey>`. It did not
compile — glam derives `Clone, Copy, PartialEq, Eq, Hash` on `UVec3` and no `Ord` — and, fixed to
compile, would have missed the sub-second edit-loop target by roughly 50×: a B-tree lookup in the
ray march's inner loop is on the order of 18 pointer-chased comparisons at the scale involved, against
roughly 565 million total steps across all fourteen views of a worst-case 64³ model.

**Decision.** The assembled model is a dense array over its own bounding box (at most 64³ = 262,144
cells), indexed `x + y·extent.x + z·extent.x·extent.y`:

```rust
pub struct Volume {
    extent: Extent,                    // ≤ 64 on every axis
    cells: Vec<Option<MaterialSlot>>,  // x-fastest
    palette: Vec<MaterialKey>,         // ascending key order — deterministic
}
pub struct MaterialSlot(NonZeroU16);   // index+1 into palette; 2 bytes, Copy
```

No `HashMap`/`HashSet` appears anywhere in `tools/voxforge`, following the precedent
`world-format.md`'s save-table format already sets for the same reason. Where a filesystem read
order is genuinely nondeterministic (`read_dir` over a materials directory), it is sorted before
use rather than trusted, and that sort is contract, not tidiness — a duplicate-name error names a
first and second file, which is only well defined under a fixed order.

**Consequences.** The inner ray-march step becomes an integer index and a `Copy`, closing the ~50×
gap. `inspect`'s connected-component flood fill becomes O(n) over the same array. The representation
is *more* deterministic than the map it replaced, not merely as deterministic: positional iteration
has no ordering question to answer at all.

## ADR-020 — `mc_core::id::NamespacedId` is public, so VoxForge does not reimplement the namespaced-id rule

**Status**: Accepted · **Date**: 2026-08-15

**Context.** VoxForge's model names and material keys need the same rule blocks and textures already
enforce — exactly one `:`, non-empty on both sides — and the same diagnostics `blocks-items.md`
documents. `mc_core::id::NamespacedId` already implements this, but its field was private and only
purpose-named aliases (`BlockName`, `HudElementName`, `TextureKey`) were exported; nothing let a
crate outside `mc-core` express "this is a namespaced id, checked the same way" for a concept that
is not one of those four things. The alternatives — reimplement the rule inside `tools/voxforge`, or
newtype over an existing alias like `BlockName` — both fail on inspection: reimplementing risks the
rule drifting between two copies, and newtyping over `BlockName` is exactly what `mc-core`'s own
`namespaced.rs` argues against, since a model is not a block.

**Decision.** `NamespacedId` becomes a public type in `mc-core`, with `parse` and `as_str` as its
public surface; its inner `Arc<str>` field stays private. Two additive lines, no behaviour change,
no existing export touched. `tools/voxforge` defines `MaterialKey(NamespacedId)` and
`ModelName(NamespacedId)` as newtypes over it — `ModelName` rather than reusing `BlockName`, because
a voxel model is not engine content and is not a block.

**Consequences.** VoxForge's refusal diagnostics for a malformed `name` or palette material key are
the same diagnostics blocks already give, for free. If materials become engine content later, the
newtypes move with them; nothing about today's placement forecloses that.

## ADR-021 — VoxForge's texture emission: shading is a render input, and tileability is a total verdict judged against the model, never against a second render

**Status**: Accepted · **Date**: 2026-08-15

**Context.** VoxForge's `texture` command emits a flat, unshaded block face — added after preview
rendering (ADR-018) was already built and shipping shaded output. Two questions needed answers:
where does "flat" live, and how is "this texture tiles" decided.

**Flatness.** The only existing flat path was `emissive = 1.0`, and the material table is shared
across the whole art set. Forcing a material's `emissive` to 1.0 to get a flat texture would
silently unshade *every model* using that material too — one material cannot serve both a shaded
preview and a flat texture. The alternative that changes `render`'s own signature was rejected only
because it would edit test files a completed implementation phase already owned; the design it lost
to differs from it by exactly one forwarder.

**Decision (flatness).** Shading is passed to the render as an explicit setting
(`Shading::Shaded | Flat`) consumed by a private core, reached through two public forwarders:
`render` (shaded, unchanged) and `render_texture` (flat). `Flat` emits a material's declared colour
for every facing regardless of that material's `emissive` — it does not reuse the emissive blend
path. No scenario can currently tell this apart from the emissive-forcing alternative; the
preference is **structural** (about which future shading term — ambient occlusion is already a
named candidate — silently stops being flattened by the cheaper design) rather than behavioural, and
is recorded as such rather than claimed as tested.

**Tileability.** The terrain mesher (`terrain.wgsl`, `sweep.rs`) merges runs of matching faces into
one quad under `AddressMode::Repeat`, so a texture whose opposing edges disagree draws a visible grid
across any surface wider than one block — a real correctness question for a texture generator, even
though nothing in `mc-render` is touched by this decision. Comparing a texture against a 3×3
replicated re-render of the same model was considered and rejected: on an axis-aligned orthographic
render with one opaque sample per pixel, the centre tile of such a replication is byte-identical to
the single render *by construction, for any rasteriser* — the comparison is a tautology that could
only ever catch what a silhouette-size check already catches directly, and a uniform sampling shift
would move both sides together and stay green regardless.

**Decision (tileability).** A total `SeamVerdict` enum, judged over three independent, ordered legs,
each measured against something the rasteriser did not itself produce:

1. **Period** — the assembled model's extent on each in-plane axis equals the document's declared
   `scale`, checked against the *document*, never a render.
2. **Coverage** — every pixel is opaque.
3. **Edges** — integer, no declared threshold: for each row (and, on the other axis, each column)
   independently, the wrap-around step may not exceed the largest step already present within that
   row. Compared **per row rather than per image**, because a per-image maximum lets an extreme step
   anywhere license a discontinuity anywhere else.

The verdict is **always computed and reported**; it **refuses the emission only when the emission
declares itself `--seamless`** (default off), mirroring the defect/observation partition `inspect`
already draws — not every texture needs to tile, and refusing a legitimate one-off would be the tool
inventing a rule nobody asked for. Under `--seamless` the first failing leg is the answer, for a
reproducible diagnostic; without it, every leg is evaluated and reported, since a texture that can
never pass leg 2 (glass, a leaf sprite) should still learn whether its edges agree.

**Consequences.** The edge leg is deliberately permissive — it never false-refuses correct art, and
under-refuses on high-contrast voxel art where the largest interior step already saturates; it earns
its keep on low-contrast content carrying a high-contrast boundary. Tightening it later changes no
interface. Where the "this texture is meant to be seamless" claim should live — the invocation
(today) versus the document itself (a `seamless = true` field would be additive and one-to-one with
a model, where the invocation is one-to-many and forgettable) — is an open, deliberately deferred
question; revisit once an art set is regenerated more than once.

---

## ADR-022 — One Luau state for the whole host; the isolation unit is a loader's to choose, not content's

**Status**: Accepted · **Date**: 2026-08-16 · **Named revisit**

**Context.** ADR-003 embeds Luau and ADR-004 pins the VM to the tick thread, but neither says how
many VMs exist. The question is forced by where the memory limit lives. Measured against this
toolchain:

| Measurement | Value |
|---|---|
| Footprint of `Lua::new()` + `sandbox(true)` | **385,952 B** Lua-reported, **389,169 B** process-side |
| Linearity at 1 / 8 / 64 / 256 states | **exactly 389,169 B per state at every count** — nothing amortises |
| Construction cost | ~95–130 µs per state |
| Footprint with `StdLib::NONE` instead of `ALL_SAFE` | 364,617 B — trimming libraries saves 6 % |
| Scope of `set_memory_limit` | **per `Lua` state** (it attaches to that state's `MemoryState`) |
| Scope of `set_interrupt` | per `Lua` state |

So isolation per state is genuinely cheap, and **a memory cap that is per mod requires a state that
is per mod.** There is no way to divide one state's allowance between the things running inside it.

**Decision.** One state, owned by the host. The per-invocation memory cap is enforced by the
interrupt against the baseline an invocation started from (ADR-003), and every policy mechanism —
the call-and-loop budget, quarantine, fault counting and the follow-up queue — is Rust-side and
keyed by the `(subject, component)` attachment, touching the state not at all.

**Rejected: one state per component.** This is the option that looks safe and is not, and the
argument that carries is not arithmetic. **Component count is controlled by content; mod count is
controlled by a loader, which is to say by the operator.** Making the number of states a function of
a content-controlled quantity turns 389 KiB of unreclaimable fixed overhead into a per-registration
multiplier — spent at registration, and therefore invisible to every containment mechanism the host
has, since the budget, the memory delta and quarantine all act during an *invocation* and this cost
is incurred before one happens. That is a breach of "a bad mod never takes down the server"
manufactured by the isolation mechanism itself, and it needs no hostile intent. **The unit that
makes per-state isolation safe is one a loader controls.**

**Deferred, not rejected: one state per mod.** This is the answer the memory measurement points at,
and it cannot be taken yet because there is no mod. Mod loading, multi-file content and `require`
confinement do not exist; the host's own vocabulary defines a subject and a component as opaque
identities it *stores and never interprets*, and a host forbidden from reading structure into a
namespace cannot derive a mod from a component name. The decision belongs to the work that first
makes a mod a loader-controlled unit, and it should be treated there as a correctness requirement
rather than an optimisation. Reversal is cheap and the seam is named: the host would hold a map from
isolation unit to state and dispatch would look one up. Only `crates/mc-script/src/luau/` changes.

**Consequences — the residual, stated because it is real.** Under one state, memory *retained
across* invocations is not bounded per attachment. A callback is a closure and a closure holds
upvalues, so `local kept = {} return function() kept[#kept+1] = buffer.create(1024) end` retains
with no state API at all, and a suspended coroutine is a second door into the same room — it holds
its own stack and everything that stack references, for as long as anything references the
coroutine. Neither trips the per-invocation cap, which bounds what one entry *adds* and not what the
state already holds. Aggregate retention is bounded, by the absolute backstop; what is unbounded is
retention per attachment.

**The damage is misattribution, and the framing is the accidental one.** As the backstop is
approached, *every* attachment's allocations begin failing, and each failure would otherwise be
charged to whoever happened to be running — disabling a mod that did nothing and sending an operator
to remove the wrong file. The population this concerns is careless authors, not hostile ones: a mod
weaponising containment is not the threat this record is about. So the host detects the condition at
entry, from its own definition rather than from a tuned fraction:

> `entry_baseline + memory_cap > memory_backstop`

— *this invocation could fail for a reason that is not its own*. While that holds, a fault is
attributed to the **host** rather than to whoever was running: it carries no subject and no
component, and it does not count toward the attachment's consecutive-fault total. There is no
constant to choose, defend, or re-choose when the backstop moves.

**The cost of that exclusion is named rather than hidden.** The condition is a property of the whole
state, not of an attachment, so while it holds a genuinely looping mod is not quarantined either,
and an attachment whose own retention raised the baseline is the one the rule excuses. That was
weighed on which failure is *visible*: excusing means a slow server an operator notices and acts on;
not excusing means an innocent mod permanently disabled with the blame filed against the wrong
author. Quarantine would not have reclaimed anything in any case — retention lives in closure
upvalues, which survive it.

**Refused, and priced: a per-attachment retention ledger.** Attributing retained bytes to whoever
allocated them would name the offender directly. It is refused rather than deferred, because it is a
second unverified mechanism layered on a mitigation that already closes the gap **by construction**:
the derived condition above makes the misattribution impossible rather than merely detectable, which
is the stronger property, and a ledger's own attribution would be a heuristic needing its own
evidence before anyone believed it. The price of refusing is stated plainly — **under pressure,
nothing names which attachment caused it.** The operator sees that scripting is degraded and not who
degraded it, and finds out by removing mods until it stops. That is accepted for a server whose
operator chose every mod installed; it would not be accepted for a host running mods it did not
choose, and that is the condition under which to reopen it.

**One structural consequence taken now.** Script handles (`ScriptFunction`, `ScriptTable`) carry an
opaque isolation-unit tag from the first design, even though exactly one unit value exists. The
reason is **hot reload, not speculation about a second state**: reload builds its candidate registry
in a scratch state off the tick thread, and its whole job is substituting a scratch-state function
for a live one. A handle that does not say which state it came from makes that substitution
unverifiable in the one path whose partial-failure mode is a hard stop. It is explicitly **not**
justified by the modding API's exemption from "no abstraction before three concrete uses" — that
exemption is scoped to the published scripting surface, and these are Rust handles consumed by
sibling crates. Recording the wrong reason would be worse than recording none, because anyone
reading the standard correctly would find the justification void and delete the field.

**Revisit when** a mod becomes a loader-controlled unit. Escalate sooner if any work gives content a
further way to retain state across invocations.

## ADR-023 — A reported failure renders through one seam in `mc-render`, and construction is closed to three doors

**Status**: Accepted · **Date**: 2026-08-16

**Context.** A client refusal used to be composed at each call site by flattening a typed failure
with `.to_string()` and printing its outermost sentence — every layer beneath the top one was
discarded, so a mod author saw "the shipped content could not be read" and nothing else. Fixing the
text at each site would leave the same shape: as many places to get it right as there are call sites,
and no way to state that all of them do.

**Decision.** One function walks an error's `source()` chain and renders it outermost-first, joined
by `": "`; one function writes the rendered text to a caller-supplied sink. Both live in
`mc-render::window`, beside the ending-to-exit-code mapping they now match: `mc-client` is excluded
from the coverage denominator wholesale, so reporting composed there is reporting nothing measures,
which is how the original defect survived unnoticed. The reported variant of the run's ending is
`#[non_exhaustive]`, so no crate outside `mc-render` can write its struct literal, and the enum
exposes exactly three constructors — one for a failure plus guidance on what to do about it, one for
a failure prefixed with a context sentence the failure itself does not know, and one for a sentence
with nothing beneath it. Every call site in `mc-client` goes through one of the three; none composes
report text by hand.

**Consequences.** After this change, a failure that never reaches the renderer, or reaches it through
a hand-built string, does not compile — the property does not depend on a scan noticing a new site
that forgot to use it. A companion source scan still runs, because the compiler only closes the
*composition* hole: a call site can still build its context sentence by interpolating a failure's
text into a differently-named binding and handing that string to the context parameter, which is a
narrower, naming-convention-shaped hole the scan is left to catch. Guidance ("what to do about it")
travels with the call rather than with the failure's type, so a future construction site could in
principle omit it silently; nothing but the parameter being non-optional and the three-door
constructor set holds against that today.

**Rejected.**

- **Composing the report in `mc-client` itself** (a new module or its existing `lib.rs`). Shortest
  diff, but it lands the one thing this change exists to make observable inside the crate the
  coverage gate does not measure at all — the same blindness that let the original defect ship.
- **A chain-walking primitive in `mc-core`, with the ending's constructors left in `mc-render`.**
  Layering purity — a chain walk depending on nothing else — at the cost of splitting one concept
  across two crates for a walk that has exactly one caller and no reason yet to be shared further.
- **A private newtype payload (`Failed { report: Report }`) instead of `#[non_exhaustive]` on the
  variant.** Equal strength, larger diff: every site reading the ending changes shape, not just its
  construction, to close a door a struct-level attribute already closes.
- **Leaving construction open and relying on the source scan alone.** The scan's criterion is
  necessarily a spelling — a needle it looks for — so a differently-spelled composition site escapes
  it by construction. Closing composition at the type level turns the central property from "scanned
  for" into "does not compile", and leaves the scan to guard only the narrower hole neither door nor
  scan closes alone.

## ADR-024 — Array-texture layers are appended within a session and never renumbered

**Status**: Accepted · **Date**: 2026-08-17

**Context.** Hot reload replaces the block registry whole while the game is running. The layer each
texture key occupies in the array texture is stated by whoever read the content, and **a layer index
sits inside every packed vertex** — so the renderer is holding, at the moment of a swap, geometry
whose texture selection is already committed. A reload therefore has to decide what happens to an
assignment that has been handed out.

**Decision.** A reload **appends** assignments for keys the session has not seen and changes none it
has. A key that stops being declared keeps its layer and its texels for the rest of the session:
retired, but not reclaimed. The bound is 256, which is not a preference but the width of the packed
vertex's layer field, declared once in `mc-core` and asserted against `mc-render`'s own capacity at
compile time. A candidate needing more than the session has left is refused whole, and the refusal
names the counts. Relaunching reclaims the difference, which is exactly `spent − live`.

The append operation returns a new assignment rather than mutating one, so a partial append is not
merely unchecked — **it cannot be written.** That is load-bearing and easy to destroy: a future
refactor to `&mut self` would remove the property while every test stayed green, because the
greedy-but-still-refusing mutation leaves the caller an `Err` either way.

**Consequences.** A session that renames a texture key repeatedly can exhaust 256 layers while
declaring a handful, and its way out is a relaunch. Appending writes one layer's texels but the write
path iterates every live entry, so the cost of a reload's upload is proportional to the live set
rather than to what changed.

**One consequence has been closed, and the decision above is unchanged by it.** This record read
that a mod author's edit to `texture` is accepted and invisible, because the layer a block draws
from was selected by its *name* — and it named `modding/hot-reload.md` as where that limitation was
stated. **SPEC-019 lifted it** (PRO-902, absorbed into PRO-947). Resolution now reads the block's own
declaration at both consuming sites — `layer_for` (`crates/mc-render/src/geometry/mod.rs`) and
`hud::held::held_swatch` (`crates/mc-render/src/hud/held.rs`), neither of which parses a block's name
— so a `texture` edit is visible, per facing. The as-built record is
`technical/rendering.md` §"A face draws what its block declared, and a `Quad` carries no key",
and `technical/architecture.md` §"Making an edit visible: the remesh transport" states it
alongside the assignment this record governs.
Append-never-renumber, the 256 bound and the exhaustion-by-renaming consequence are untouched: they
are properties of the *assignment*, and what changed is only which key a face asks it for.

**Recorded here because pointing at another document was the failure.** A limitation stated in two
places costs two edits to lift, and the second one was missed for a full validation pass — an ADR
whose Consequences name another file as their record is exactly the one nobody re-reads. See
`technical/working-in-this-repo.md` §"Lifting a limitation costs one grep per place it was stated".

**Rejected.**

- **Renumbering the assignment on every reload**, deriving it lexicographically over whatever the new
  content declares. Simplest to state and the reason it fails is decisive: a key inserted anywhere but
  last shifts every key after it, and every packed vertex the renderer holds then draws from another
  block's texture until the whole world has been re-meshed and re-uploaded. Not a transient artefact —
  a wrong picture with no error, lasting as long as any un-re-meshed section does.
- **Reclaiming retired layers by compaction.** Compaction *is* renumbering, with the same consequence
  and a harder-to-reason-about trigger.
- **Widening the packed vertex's layer field.** Moves a contract that two crates agree on today, costs
  bandwidth in the hottest buffer in the renderer, and buys headroom against a limit no authoring
  workflow reaches. The session-lifetime bound is the honest constraint and a relaunch is the honest
  answer to it.
- **A second "geometry serial" so that a reload changing nothing geometric does not supersede a batch
  in flight.** Deferred rather than rejected on principle: it costs one batch, and that batch can only
  re-mesh sections which were dirty anyway.

## ADR-025 — A reload candidate that stops declaring a block the world holds is refused whole

**Status**: Accepted · **Date**: 2026-08-17

**Context.** A reload takes content read while a world is live. That world's cells hold blocks by
name, and a candidate is free to stop declaring one — most often because deleting a file is how an
author tidies up. Something has to happen to the cells holding it.

**Decision.** The candidate is **refused whole**, before anything is published, and the refusal names
every such block ascending. The content already serving goes on serving; nothing is half-applied and
nothing accumulates. The check runs inside the world's own write door, which resolves the new solidity
before writing either view, so a refusal returns with the world exactly as it was rather than
partially updated.

**Consequences.** An author who wants a declaration gone breaks the blocks first, or puts the file
back. The refusal is one of the two a reload adds beyond the loader's own — the other being content
that registers no solid block — and both are quoted for authors in `modding/hot-reload.md`. It also
means a reload cannot corrupt a world into a state the save format would refuse, which matters because
the save path already refuses a missing name for the same reason: the two now agree rather than
disagreeing at the worst possible moment.

**Rejected.**

- **Replacing the orphaned cells with air.** Deletes a player's build silently on a file save. The
  worst property a reload can have is doing something irreversible that nobody asked for.
- **Substituting a placeholder block.** Same objection one step softer, plus it invents an engine-side
  block definition, which invariant 1 forbids outright.
- **Accepting the candidate and leaving the cells holding an unregistered name.** Every read of those
  cells then has to answer for a name no registry knows, which pushes a refusal into the tick path —
  the one place this project will not put one.
- **Warning and accepting.** A warning nobody reads is an acceptance, and the state it accepts is the
  one the previous option describes.

## ADR-026 — Committed-ness follows reproducibility, and the generator runs as an explicit pre-build step

**Status**: Accepted · **Date**: 2026-08-18 · **Narrows ADR-009 in part** ·
**Implemented 2026-08-19 by SPEC "grass block art", its first consumer**

**Context.** ADR-009 wrote committed-ness into the record as a property of *generated* output:
"Output lands in `assets/`, which is **committed to the repository**." That sentence is about to be
applied to a kind of output the record never reasoned about. `tools/voxforge` (SPEC-013) renders
`.mcvox` sources to textures deterministically and for nothing, and the base game's terrain faces
are to be authored that way. Taken literally, ADR-009 would commit a binary set every clone can
reproduce byte for byte — and a texture set regenerated fifty times leaves fifty blobs in the pack
forever, because git keeps them all.

ADR-009's actual reason was never the word "generated". It was that fal.ai and ElevenLabs output
**cannot be reproduced**: regenerating costs money and may differ run to run. That is why the record
hashes each manifest entry over its full generation spec, why a matching hash skips generation, and
why regenerating an unchanged asset needs an explicit `--force`. Every one of those mechanisms is
protection against a re-run that is expensive or that does not come back the same. None of them says
anything about a generator that is neither.

**Decision — the rule.** Committed-ness follows **reproducibility**, not "generated".

- **Deterministic and free to reproduce → not committed.** It is regenerated from the source that
  produces it, and the source is what the repository carries.
- **Nondeterministic, or paid for → committed**, exactly as ADR-009 says, for exactly ADR-009's
  reasons.

VoxForge output is the first thing on the near side of that line: byte-identical by requirement
(SPEC-013 FR-5.3-S1) and costing nothing to produce. It is therefore a **different class, not an
exception**. ADR-009's provider assets stay committed under the sharpened rule for the same reason
they were committed under the old one, which is what makes this a narrowing rather than a reversal.

**Golden frames stay committed. They are evidence, not assets.** The sets under
`crates/mc-render/goldens/`, `crates/mc-testkit/goldens/` and `tools/voxforge/tests/goldens/` are
reproducible in the trivial sense that re-running the renderer produces them — and that is precisely
why they are not derived output. A golden is the committed record of what the renderer produced on
the day a human looked at it and agreed; regenerating it from the code under test destroys the only
property it ever had. ADR-013 states the same thing from the other side: a golden re-shot from a
broken renderer is a golden of a broken renderer. The reproducibility test above asks whether the
output can be re-derived **from a source that is not the thing being checked**. A golden's only
source *is* the thing being checked, so it never satisfies the test and never falls under the rule.
This paragraph exists because someone will eventually read the rule, see a directory of PNGs that a
program can obviously reproduce, and tidy them away.

**Decision — the mechanism.** The generator runs as an **explicit pre-build step**: a
`voxforge build <manifest>` subcommand of the existing tool. **VoxForge is not split, and SPEC-013
FR-9.1 — `tools/` may depend inward on `crates/`, nothing in `crates/` may depend on `tools/` — is
not amended.** Its as-built statement and mechanical enforcement are in `technical/architecture.md`.

Five things follow, and the first two are binding on whichever spec first consumes generated art.

1. **The client refuses, by name, when the generated set is missing or stale against the manifest
   hash**, and the refusal says what to run. It never draws a texture-less world. This is the
   failure contract this codebase already applies everywhere else — `PreparationError` in
   `crates/mc-client/src/startup.rs:92` already fails a run rather than degrading it, and already
   carries the neighbouring case of a content root that is not there. A newcomer who has not run the
   generator gets a sentence telling them what to do, not a broken picture, and that is what makes
   the cost stated below tolerable rather than merely accepted.
2. **The gate generates before it tests**, or the golden suites read a stale set. One line in
   `scripts/sdd-gate.ps1`, ahead of stage 7, so that both the instrumented path and the
   `-SkipCoverage` path see a fresh set. This is the single real advantage a `build.rs` had, and it
   costs a line.
3. **The manifest hash of the generated set is asserted.** While a texture PNG was committed it was
   pinned; once it is derived, a generator change silently shifts every golden that samples it and
   reads as a renderer regression. Hashing the set makes "the generator changed" fail loudly and
   distinctly from "the renderer changed" — the same reasoning as the licence-text drift check
   already on the gate (`technical/licensing.md`).
4. **A gate stage fails on a committed generated artefact.** A `.gitignore` rule drifts back over
   time; a stage that refuses the artefact is what makes the rule hold.
5. **The build step needs a cache keyed on `.mcvox` content**, or every incremental build
   regenerates the whole set. Cheap to design in and annoying to retrofit.

The developer step is `cargo run -p voxforge -- build`, one README line above
`cargo run -p mc-client`.

**Consequences.** `cargo build` alone no longer produces a complete game. That property was worth
something and it is being spent deliberately; item 1 above is the whole of the mitigation, and if it
is ever weakened this decision should be reopened rather than quietly lived with. The repository
stops accumulating binary revisions of output it can derive, and `.mcvox` sources become the thing
under review in a diff. VoxForge gains one `Command` variant and one dispatch arm — no new crate, no
new workspace member, no change to the dependency topology, and nothing added to any engine crate's
resolved closure.

Item 1 is about the generated **set** measured against its manifest, and is not the same question as
a single declared texture key with no art behind it. A mod author trips the latter on their first
block, and the per-key placeholder that answers it is unaffected by this record: one is "the build
step was not run", the other is "this key was never authored", and they must not be collapsed into
one message.

FR-9.1 stays a checked fact rather than prose. `crates/mc-testkit/tests/workspace_layering.rs` walks
each engine crate's resolved closure from `cargo metadata` and collects every `deps[].pkg` without
filtering `dep_kinds` (`workspace_layering.rs:99`), and those nodes do carry build-dependency edges —
`mc-render`'s edge onto `naga` reports `dep_kinds` of `dev` and `build`. So a `build.rs` reaching
VoxForge fails that test today, under a total-enum verdict with a positive control on both
directions.

**As built.** The spec that first consumed generated art landed all five items, and each is
somewhere a reader can go and look:

1. **The refusal.** `crates/mc-client/src/textures/mod.rs` judges a content root's set into a total
   `SetVerdict` and `refusal_for` turns each arm into a `PreparationError` naming
   `cargo run -p voxforge -- build content/base/textures.toml` — one constant, `BUILD_THE_TEXTURE_SET`
   in `crates/mc-client/src/startup.rs`, so a message quoting a command nothing accepts is
   unspellable. `launch::start` performs that judgement before the window opens, so a contributor
   who has not run the build reads the sentence instead of waiting out a world nobody will show
   them.
2. **The gate builds first.** `scripts/sdd-gate.ps1` runs the art build ahead of the test stage on
   both the instrumented and the `-SkipCoverage` path, and a refused build fails that stage,
   reproduces the build's own words, and **records the skipped test stage by name** rather than
   omitting it — a gate that drops a stage silently is one step from a gate that skips its way to
   green.
3. **The set is pinned to its sources, not to its output.** The index records one FNV-1a-64 value
   folded over exactly the manifest, the models and the materials it reached, in fold order, and the
   client re-folds the same bytes in the same order. So "the generator changed" is
   `SetVerdict::StaleAgainstSources` and reads nothing like a renderer regression — which is the
   whole of the distinction item 3 asked for, reached through the *sources* rather than through a
   hash of the images. Hashing the images would have re-pinned the derived artefact this record
   exists to stop committing.
4. **A committed built image fails a stage** and the stage names the path.
5. **The cache is content-keyed**: a build whose output is current does no work, which is what makes
   the gate's extra stage cost nothing on an unchanged tree.

The developer step is one README line, and the first thing a fresh checkout does.

**Rejected.**

- **A `build.rs` in the consuming crate that invokes VoxForge**, whether reached directly or through
  a split-out crate. This is refused for reasons that hold *independently* of FR-9.1, which is why
  the split below does not rescue it. **Generated block textures are content.**
  `crates/mc-sim/src/content.rs:42` declares the shipped content root as the relative path
  `["content", "base"]`, resolved against the process's working directory at runtime
  (`content.rs:105`), and its own doc comment says so. A build script may write only to `OUT_DIR`,
  which that loader cannot see. Both ways out are refused: `include_bytes!` from `OUT_DIR` embeds
  base-game block textures in the engine binary, against invariant 1 and against ADR-009's own "the
  game reads local files only"; and writing into `content/base/` has cargo mutating git-visible
  state, which cargo explicitly reserves against. The mechanism does not fit the thing it would be
  producing.
- **Splitting VoxForge — render core as a crate under `crates/`, CLI staying in `tools/`.** This was
  the recorded preference, and it is rejected although it is **cheap**. Cheapness was measured and is
  not in doubt: `cli/` is a leaf reached from exactly one place (`tools/voxforge/src/main.rs:12`),
  `clap` is named in one file (`tools/voxforge/src/cli/args.rs:11`), and the module graph is already
  an acyclic layering. It is the wrong question. The split buys nothing except permission to build
  the `build.rs` above, and the paragraph above declines to build it — so the crate boundary would be
  paid to unlock a door into a room we do not want to enter. Worse, it satisfies FR-9.1's letter
  while delivering the coupling the rule exists to forbid: relocating the whole authoring tool into
  `crates/` so that `crates/` is allowed to depend on it is relocation, not architecture.
- **Amending FR-9.1.** The honest alternative if we did want the engine build to invoke the asset
  tool, listed here rather than left unsaid, because pretending the rule was never in the way is how
  a rule gets hollowed out by the option that merely looks compliant. We do not want that coupling,
  so there is nothing to amend for.
- **Committing the generated set anyway, treating VoxForge output as an exception to ADR-009.**
  Keeps `cargo build` complete, at the cost of the repository carrying every revision of something
  any clone can re-derive for free, forever. It also leaves the rule stated as "generated output is
  committed" — a rule whose reason nobody can recover, which is how the next deterministic generator
  inherits a constraint written for a paid one.
---

## ADR-027 — Terrain magnifies with nearest and minifies with linear; anisotropy is refused

**Status**: Accepted · **Date**: 2026-08-19 · **Extends ADR-013**

**Context.** A block texture is sixteen texels of deliberate pattern, and until the base game's art
was baked every one of them was a generated stand-in nobody stood close to. Real art changes both
ends of the range at once. Up close a face fills hundreds of screen pixels, so what a magnification
filter does to sixteen texels is now the difference between a visible pattern and a coloured square.
Far away a face falls under a pixel, so what a minification filter does decides whether a distant
hillside is stable or shimmers as the camera moves. The sampler had been nearest on all three
filters and the array texture had declared one mip level, which is the right answer for neither end.

wgpu offers a third knob — `anisotropy_clamp` — and it is the one that cannot be had. Its
validation refuses a clamp above one unless magnification, minification **and** mip interpolation
are all `Linear`, in three separate arms
(`wgpu-core-30.0.0/src/device/resource.rs:2288-2316`). Anisotropy and crisp voxel magnification are
therefore mutually exclusive at the vendor, not at our discretion.

**Decision.**

- **Magnification is `Nearest`.** Linear magnification interpolates between texel centres, so a
  magnified face becomes a blend of its own colours — which is its own mean, which is the value the
  probes cluster against. A filtered frame would then agree with the probes for a reason that has
  nothing to do with the texture being right.
- **Minification is `Linear`, interpolation between mip levels is `Linear`, and the array texture
  declares a full chain** — five levels for a sixteen-texel edge, derived from the edge rather than
  written as a literal.
- **The chain is averaged in linear light, not over the stored bytes.** The array texture is
  `Rgba8UnormSrgb`, so a texel is decoded on sample; averaging the stored bytes puts 0 and 255 at
  128, which decodes to 0.216 rather than 0.5, and every level comes out darker than the one above
  it. Decoding, averaging and re-encoding puts the same pair at 188. The transfer pair is
  IEC 61966-2-1 itself and not an approximation: a gamma-2.2 curve answers 186 for that pair, which
  is close enough to look right and is a different function.
- **Anisotropy is refused, and the refusal is the device's.** `terrain_sampler` builds the request
  inside a `wgpu::ErrorFilter::Validation` scope and maps a captured error to
  `RendererError::TerrainSampler`, which carries the whole request because the vendor's rule is over
  the combination and names no single field. There is no pre-check on our side: a rule copied out of
  a vendor is a second copy that goes on agreeing with itself the day the vendor changes it.

**Consequences.** This is a one-way door in the sense that matters: every committed golden frame is
now a picture taken through this sampler, so reversing it is a re-shoot. It is recorded here rather
than left in a constant so that nobody re-opens "why is anisotropic filtering off" as a bug — the
answer is that this project bought crisp magnification with it, deliberately, and the vendor priced
the trade.

The sampler is a **value** (`mc_render::texture::sampler::SamplerRequest`) rather than a descriptor
built inside a private function, and that is load-bearing rather than tidy. What a sampler *asks
for* and what a sampled frame *looks like* are different claims; a test that reads back the
descriptor it caused to be built is agreement between two copies of one decision. Threading the
request from the composition root is what lets a capture ask for a **second** configuration —
unfiltered minification — so the difference linear minification makes is measured against a run that
does not have it rather than asserted. `crates/mc-render/tests/terrain_sampling.rs` is where that is
done, and `crates/mc-render/src/texture/sampler_test.rs` is the other half.

`pollster` moves into `mc-render` as an optional dependency under the `gpu` feature, and the
manifest comment reserving device acquisition to the client and the harness is **amended** in the
same commit rather than contradicted silently. Popping an error scope the driver has already
recorded into is not device acquisition, but the rule as written forbade it. It stays featureless
and stays under the feature, so `--no-default-features` resolves neither it nor `wgpu`;
`crates/mc-render/tests/dependency_graph.rs` asserts both directions.

**Rejected.**

- **Linear magnification throughout.** Consistent, one fewer thing to explain, and it blurs a
  sixteen-texel pattern towards its own mean. The whole reason the art exists is that a player can
  see it.
- **Nearest minification, keeping today's sampler and adding no mip chain.** Free, and it shimmers:
  a distant face samples one texel, so a sub-texel camera movement flips whichever pixels crossed a
  texel boundary by the whole contrast between two texels. That is precisely the observable
  FR-6.2-S2 measures, and it measures it because the alternative was believing it.
- **A pure pre-check that refuses an invalid combination before the device sees it.** Faster to
  fail, and it is a second copy of `wgpu`'s validation living on this side of the boundary. The
  scenario's subject is the device.
- **Anisotropy with linear magnification.** The combination the vendor accepts, and it costs exactly
  the property this decision is about.

---

## ADR-028 — An image's basis is not the geometry's plane pair, and both image tables are derived

**Status**: Accepted · **Date**: 2026-08-19 · **Extends ADR-013** ·
**Found by the project owner looking at a frame, after 1370 green tests**

**Context.** The terrain shader needs, per facing, the two components of a corner's position that
its texture coordinates are read from. `mc_render::geometry::PLANE_AXES` already held a pair of axis
indices per facing, so the shader read that pair — primary into the image's horizontal, secondary
into its vertical — and had done for five increments.

**`PLANE_AXES` is the geometry's table and it answers a different question.** `placed()` reads it to
put a quad's primary and secondary extents into world components, honouring `mc_world`'s definition
of those extents as the facing's other two axes in `x < y < z` order. A row changed in it *moves the
mesh*, not the texture. That was tried as the fix and is what proved the point:
`a_drawn_side_shows_turf_above_dirt` kept east and west red at 1104 of 1104 while north and south
went green — one table serving two masters, and correcting it for one breaks the other.

**A pair of axis indices cannot express an image's basis.** It names two axes and an order; it
cannot say which of the two the image's *own* horizontal runs along, nor which way either of the
image's coordinates runs down the axis it was matched to. All three differ per facing:

- An image's rows run **downward** while the world's vertical axis runs up, so every face with world
  up in it needs its vertical coordinate negated.
- Two faces looking at each other along one axis see their in-plane axes in **opposite** horizontal
  order, so one of each pair needs its horizontal negated — without which north and south are forced
  to share a horizontal direction and one of them draws laterally reversed.
- An `X` face's pair is `(y, z)` with `y` primary, so its image's horizontal runs along the
  **secondary**.

Reading the pair alone as though it were an image basis therefore drew five of the six faces wrong:
east and west a quarter turn out, north and south and the underside flipped, north laterally
reversed as well. Only the top was right, which is why FR-8.1-S5 was green and honestly so. Nothing
in the suite could see the rest, because every reading measured *which colours* a face holds and a
rotation or a reflection changes none of them — the reasoning is written out in
`technical/rendering.md` beside the re-shoot it forced.

**Decision.**

- **The image basis is its own thing.** `IMAGE_SWAPS` says whether a facing's image runs its
  horizontal along the plane pair's secondary rather than its primary; `IMAGE_SIGNS` says whether
  each of the image's two coordinates runs against its axis. `PLANE_AXES` is left untouched and
  keeps its single meaning. A future reader who finds a face's texture turned changes an image
  table; one who finds a face in the wrong place changes the plane pair.
- **Both image tables are derived, not tabulated.** A viewer standing outside a face looks along the
  face's inward direction with the world's up as their up, and the image's right edge is then
  forward crossed with up. That is two lines of vector arithmetic over each facing's own outward
  normal (`geometry::image_basis`), and the tables are `const fn` results rather than literals.
- **Derivation was chosen over a hand-written table because a table of conventions cannot be checked
  by reading it.** That is not a preference for elegance: a six-row table of swaps and signs is
  exactly the artefact this project already got wrong and then held wrong mechanically for five
  increments. A normal, by contrast, says which way a face points and carries no convention at all,
  so there is nothing in the input to get wrong. What remains hand-written is the six normals, named
  at the call sites rather than walked, so the declaration order the rows depend on is visible in
  the source instead of implied by an index.
- **The two horizontal facings' convention is chosen, and written down.** A top or bottom texture
  has no world up in it, so no derivation can settle its orientation. The choice is to match what
  `voxforge` bakes — the top image's top edge toward `-z`, the bottom image's toward `+z` — and both
  halves of that contract are now stated: `modding/voxel-models.md` for the baker, this record and
  `technical/rendering.md` for the renderer. The sentence that used to stand in the baker's
  documentation, leaving the match "to whoever consumes the texture", is the record of how the
  defect shipped: an orientation contract neither side states is one neither side can be wrong
  about, so both were.
- **The bottom row was corrected on the strength of the bake rather than of any test, and that is
  recorded rather than smoothed over.** A block's underside is never seen in this world, so it was
  measured wrong and no reading complained. **No test discriminates either horizontal row today** —
  every orientation of a top or bottom texture looks equally plausible, and `base:grass_top`'s noise
  is near-uniform under rotation. The first anisotropic top or bottom texture owes a scenario, and
  until then those two rows rest on the bake and on this paragraph.

**Consequences.** The fix re-shoots every golden frame that shows terrain, which is the second
re-shoot of that set; it was forced by a defect rather than chosen, and `technical/rendering.md`
carries the procedure and the reasoning. The shader now holds three hand-written tables where it
held one, and the two new ones are the only hand-written copies of values that are derived on the
Rust side.

**`build/validate.rs` compares all three against the shader's literals, and that comparison is not
evidence that any of them is right.** It text-compared the plane pair for five increments while
three hand-written copies of it agreed with each other and all three agreed on the wrong answer. A
mechanical check that two copies of a constant match closes drift between the copies and says
nothing whatever about the constant. This is stated here because the check reads like a guard and
will be mistaken for one: what can say a table is right is a reading of a drawn face — FR-8.1-S7 for
where a face's bands sit, FR-8.1-S8 for which way it runs, and FR-8.1-S6 for the bake against the
model it is a view of, under all eight dihedral transforms.

**Rejected.**

- **Exchanging the two `X` rows of `PLANE_AXES`.** The first fix attempted, and the one that looks
  right from the shader's side. It moves the mesh: north and south went green while east and west
  stayed red at 1104 of 1104. The symptom it treats is real and the table it treats is not the one
  that holds the fault.
- **A hand-written six-row table of swaps and signs.** Shorter, and readable in the sense that all
  twelve numbers are visible at once — which is precisely the property the shipped defect also had.
  Nobody can check twelve convention bits by looking at them, and a build-time comparison against a
  second copy of them cannot either.
- **Leaving the top and bottom convention unstated because nothing can see it.** What actually
  happened: unobservable is not the same as free, the bottom row was wrong for five increments, and
  the first texture with a direction in it inherits the error as a mystery rather than as a
  decision.
- **Treating `build/validate.rs` as the guard and adding a fourth copy to it.** More agreement
  between copies, no more evidence about the values. The scenarios above are what the tables are
  answerable to.

---

## ADR-029 — A save whose blocks changed behaviour is loaded and reported; only a missing block refuses

**Status**: Accepted · **Date**: 2026-08-22 · **Narrows ADR-025 in part** ·
**Implemented 2026-08-22 by SPEC-020**

**Context.** Two paths answer the same content edit and they answered it in opposite
directions. Loading a save whose blocks' declared *behaviour* had changed was
**refused** until the player passed `--load-changed-blocks`; the live reload of that
very same edit **accepted** it and, where it was genuinely dangerous, moved the
player rather than refusing (ADR-025's siblings, and the entry-clearing search). The
refusal protected nothing: the world was already written, the data was loadable, and
what the player got for a content update was a game that would not start. The reload
path, which acts on a world somebody is standing in *right now*, is the one that
accepts.

Two further facts decided the shape rather than the direction. First, the refusal
already named every changed block, so the information a player acts on existed and
was being spent on turning them away. Second — and this is the one that is easy to
miss — **a save is rewritten against the current declarations on quit.**
`NameTable::record` writes `behaviour_of`/`appearance_of` from the registry the
process is running, so *load a changed save → play → quit* replaces the recorded
hashes and the mismatch is gone from the file for good. Whatever this path does, it
gets one chance to do it.

**Decision.**

1. **Loading is the default.** A save whose blocks changed only in declared behaviour
   is loaded, cell for cell, with the player where it recorded them.
2. **The changed blocks are named on the error stream**: one line, every one of them,
   ascending, complete, never truncated, and nothing at all when nothing changed. Not
   a prompt — there is no prompt anywhere in this system and there never was.
3. **Behaviour only.** A block whose declared *appearance* alone moved is not
   reported. The reasoning that keeps a retexture out of the refusal transfers
   unchanged to a line.
4. **`--refuse-changed-blocks` keeps the strict answer**, per run, and the refusal it
   causes names it as the thing to drop.
5. **A missing block still refuses unconditionally**, whatever the argument. Nothing
   can go in the cell, so there is no answer a caller could give that would make the
   save loadable.
6. **The line is said where the load completes, above any device.** `LoadedWorld`
   carries the changed names out, they ride `Seated` beside the clearing verdict, and
   `launch::played_and_reported` says them before a window or a GPU exists.

**Why this does not contradict ADR-025.** That record refuses a *reload candidate*
that stops declaring a block the world holds, and rejects "warning and accepting" on
the grounds that a warning nobody reads is an acceptance. Both still hold, because
both are about a **missing** name — point 5 here, unchanged, on the load path. The
distinction ADR-025 did not have to draw is the one this record turns on: a missing
name admits no answer, a changed one does. Where the two paths do meet, they now
agree — a reload refuses a missing block, a load refuses a missing block, and neither
refuses a block that is merely no longer what it was.

**Consequences.**

- **The report is a one-shot and playing on destroys the evidence** — see
  `world-format.md` §"Known limitations, as built". That is what earns
  `--refuse-changed-blocks` its keep: somebody restoring a backup they are unsure of
  wants the world left shut, because opening it is what erases the record of what it
  was. No backup, copy-on-open, or warning at the moment of rewriting is added here.
- **Point 6 is a verifiability decision, not a tidy-up.** `notice::say_entering` sits
  below the frame path's uploads because "you were moved" needs a world to have been
  moved in. "These blocks changed" is already true when `load_world` returns, so it is
  said above the device — which is the only reason a run of the *built binary* can
  read it off the real stream. Measured: deleting that call leaves 1 384 of 1 385
  tests green, and the one that reddens is the subprocess. Moving the line down for
  symmetry with its sibling would take the only instrument that can see it away.
- **`LoadedWorld` is wider than `architecture.md` said it was.** Widening it to let
  persistence be *asked about physics* was refused and stays refused; carrying a list
  of names out to be printed adds no such dependency. Nothing may branch the
  entry-clearing search on that list, and
  `crates/mc-client/tests/entry_clears_whatever_the_load_reported.rs` is what reads
  for it — the cheap implementation is now spellable where it previously was not.
- **A player can be moved by the entry search on a load they were never asked
  about.** That is the pre-existing behaviour this leans on rather than a new
  exposure: `seat` clears unconditionally, on every entry, with no branch on what the
  load reported.

**Rejected.**

- **Keeping the refusal and improving its wording.** The refusal was not unclear; it
  was the wrong answer. No wording makes "your world will not open because a mod you
  installed changed a number" into something a player can act on beyond uninstalling
  the mod.
- **A prompt.** There is no dialog, HUD or stdin channel in this build, so the only
  prompt available is a terminal question at startup — and a question asked on every
  content update is one people learn to answer without reading, which is the same
  objection that keeps retextures out of the report.
- **Reporting retextured blocks too.** A line after every art edit is the noise the
  line that matters would hide in. `RegistryVerdict.retextured` exists to make the
  classification *total* so a test can compare a whole verdict, and its doc comment's
  claim that "a caller can say so" is withdrawn rather than kept: no caller ever did.
- **Bounding the list at the first few names.** `LoadError::Unresolvable`'s own
  `Display` prints both lists whole, so a bounded line would report *less* than the
  refusal it replaces.
- **One shared revision byte over both hash lists**, which keeps coming up as an
  obvious tidy-up. Under the old default it refused every save in existence; under
  this one it mislabels every one of them on the way in — a smaller failure and a
  harder one to notice, because a world that opens looks fine.
- **Strict argument parsing.** An unrecognised argument stays ignored, which is also
  why the retired `--load-changed-blocks` needs no handling: it stops matching, and
  the load it used to ask for is now what happens anyway.

## ADR-030 — The refusal on a translucent occluder reads the resolved `occludes`, which couples the opacity default to every solid block

**Status**: Accepted · **Date**: 2026-08-27 ·
**Implemented 2026-08-27 by SPEC-031**

**Context.** `opacity` is refused when it states a degree below `1.0` on a block
that also occludes: `occludes` suppresses the meeting face of whatever the block
touches, so a block light was meant to pass through would have nothing behind it
left to show. The question the implementation had to settle is *which* occlusion —
the line the author wrote, or the value the declaration resolves to. `occludes`
falls back to whatever the same declaration said about `solid`.

**Decision.** The **resolved** value. The mesher reads the resolved value, so the
written-line reading would let `solid = true, opacity = 0.5` — glass — register,
draw a translucent face, have the geometry behind it culled, and be refused by
nothing. That is the outcome `optional_boolean`'s doc already names as the worst
available: the block behaves exactly as if the line had never been written, so
there is no symptom to notice. **A refusal that misses the case it was written for
is worse than no refusal, because it reads as coverage.** The cost is one line an
author has to write anyway for the feature to work.

The refusal names **whichever line actually caused the occlusion**, in two
distinct causes: a written `occludes = true` names a line to *delete*, while a bare
`solid = true` names `solid` and a line to *add*. Quoting an `occludes = true` at
an author whose file has no such line sends them searching for something that is
not there — the same defect as blaming a floor for a value that is not a number,
which this loader already avoids in the other direction.

**Consequence, and it is not obvious from any test.** The two rules together make
`OPACITY_BY_DEFAULT` **load-bearing for every solid block in the shipped content
root**, not for one scenario. Measured: setting it below `1.0` makes every solid
block occlude by solidity *and* pass light, so the cross-field refusal rejects the
whole root — 352 of 1 614 tests red, not the two that name the default.

**That coupling is a robustness property and should not be "fixed".** Because it
exists, a future change to the default **fails loudly at load** — with a refusal
naming a file, a block and a field — rather than silently at draw, where a world
of half-transparent stone would look like a renderer defect. Anyone tempted to
decouple them should be clear they are trading a refusal for a wrong picture.

**Water is unaffected either way**, being non-solid; this decision is entirely
about the see-through blocks that come after it.

---

## ADR-031 — The Windows coverage target is a short absolute path, namespaced per worktree

**Status**: Accepted · **Date**: 2026-08-28 · **Windows only** ·
**Reprieve, not a fix — the durable remedy is tracked as PRO-997**

**Context.** `cargo llvm-cov` generates its report by invoking `llvm-cov export`
with one `-object <path>` argument per test binary in the workspace. At 383
binaries that command line crossed `CreateProcess`'s hard limit of 32 767
characters, and the gate's coverage stage failed with `os error 206`
(`ERROR_FILENAME_EXCED_RANGE`) **after every test had passed** — a
report-generation failure wearing a test failure's costume, with no failing test
to name because the coverage half runs once the suite is green. Every phase that
adds a test file walks the workspace back into it.

Two figures for that crossing are on record and they do not agree: PRO-997 says
32 756 and the gate's own comment says ~32 883. The gap is 127 characters, which
is about what the `-ignore-filename-regex` argument costs — measured at 99
characters on the invocation today — so the smaller figure is most likely the
object list alone and the larger the whole invocation. That is offered as a
reconciliation and not as a measurement: the `target/` layout that produced both
is gone and neither can be re-taken. **The figures below can be, and were.**

Only the **repeated prefix** matters. Each argument carries the coverage target
directory's path once, so a character removed from that prefix is removed once per
binary while a character removed from anything else is removed once. Under
`target/` each argument carries `target\llvm-cov-target\debug\deps\` — 34
characters; from a short absolute root it carries `C:\mcv\debug\deps\` — 18.

**That was measured, not derived**, on a throwaway one-test crate rather than by
reasoning from the layout, and the measurement is what caught the part a derivation
would have got wrong: cargo-llvm-cov emits the object path **absolute** when the
target directory sits outside the working directory, which is why a short absolute
root beats a long relative one instead of merely trading one prefix for another.

**Decision.** On Windows the gate sets `CARGO_LLVM_COV_TARGET_DIR` to
`%SystemDrive%\mcv\<slot>`, where `<slot>` is the first four hex characters of the
SHA-256 of the lower-cased absolute repository root. Nothing changes on other
platforms, where the argument limit is orders of magnitude larger and `target/`
stays put.

**The slot exists because the directory would otherwise be one fixed location
shared by every gate run on the machine.** `standards/global/git-workflow.md` §5
states that more than one agent may hold this working tree at once, and the project
uses `git worktree`; two concurrent `cargo llvm-cov nextest --workspace` runs from
sibling worktrees would write coverage artefacts into the same place, producing
either a percentage that silently reflects a mixture of two branches or lock
contention that reads as an unrelated gate failure. **A coverage number that is
quietly wrong is worse than one that fails loudly.** The key is the repository
root, not the branch name: a branch moves, and would orphan the directory it named.
The slot separates **worktrees, not contexts** — two gate runs in one checkout
still share one directory and still fail as `docs/technical/testing.md` describes.

**Measured cost, on the tree that carries this record.** Both readings were taken
from `cargo llvm-cov report -vv`, which prints the `llvm-cov export` invocation it
is about to make, run with the gate's own `--ignore-filename-regex` and `--json`
so the printed invocation is the one the gate makes. The figure is that command
line with the printer's own quoting removed, which is the string `CreateProcess`
actually receives; both rows are the same 391 test binaries.

| prefix | chars | command line | headroom | further test binaries |
|---|---|---|---|---|
| `C:\mcv\debug\deps\` | 18 | 27 722 | 5 045 | 72 |
| `C:\mcv\<slot>\debug\deps\` | 23 | **29 688** | **3 079** | **41** |

At a measured mean of 74.4 characters per `-object` argument, the slot costs
**1 966 characters — 31 of the 72 binaries of headroom the un-namespaced path
had**, or 43% of it. That is a large proportional bite and it is accepted
knowingly: the hazard it removes is a silently wrong number, while the headroom it
spends is headroom PRO-997 has to reclaim either way. For scale, the workspace
went from 383 to 391 test binaries over the single spec that produced this record.

Four hex characters were chosen for a collision probability that is effectively
zero across any plausible number of worktrees. Two would return 13 of those 31
binaries at 256 slots, which is a few percent of collision across a handful of
worktrees — and a collision is silent, reinstating exactly the hazard the slot
exists to remove. **Neither length changes the wall's position by more than a few
specs**, which is the argument for spending the headroom rather than shaving it.

**This is a reprieve and not a fix.** The limit returns at the rate the workspace
adds test files, and no directory name is short enough to be a solution. The real
remedy is an **LLVM response file** (`llvm-cov @args`, the object list read from a
file instead of the command line), which is **cargo-llvm-cov's to emit** — it
builds the invocation, so the fix is an upstream request rather than a local
workaround. It is tracked as **PRO-997**, named here and in the gate comment so the
next person to meet `os error 206` finds the issue rather than re-deriving it.

**Consequences.**

- **Coverage artefacts now live outside the repository.** They are several
  gigabytes, and a cleaner that only knows about `target/` will miss them entirely.
  Anything that reclaims disk space, or that wipes coverage state to get a clean
  reading, has a second location to visit.
- **The pre-namespacing `C:\mcv\debug\` is now litter.** It was the target for the
  window between the original change and this record, nothing writes to it any
  more, and nothing will clean it. It is safe to delete by hand; it is not safe to
  delete `C:\mcv` itself, which is now the parent of every worktree's slot.
- **The coverage figure is unchanged.** This is worth recording because it was the
  live risk rather than the arithmetic: `--ignore-filename-regex` is built from
  `crates/*` patterns, so had the report's **source** paths gone absolute along with
  the object paths, the ADR-008/ADR-013 exclusions would have silently stopped
  applying and the percentage would have moved for a reason unrelated to any code.
  Measured across the change on one tree: **93.78% lines, 92.32% regions, 11 118
  lines tracked, `1665 tests run: 1665 passed, 1 skipped`** — identical before and
  after.
