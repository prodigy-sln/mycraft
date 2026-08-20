# Architecture: The grass block looks like a grass block

Spec: `spec.md` (SPEC-019, rigor `high`, **101** scenarios across 21
requirements). Reasoning behind the spec's own decisions: `requirements.md`. This
document decides how those scenarios are built, and it is binding on the test
author and the implementer except where a decision is marked DEFERRED.

Every claim about the tree below was read at **`133003e`** and is cited by file
and line.

**The scenario count is 101.** It was 104 when this document was first drafted and
108 after the four additions this architecture recommended were accepted
(FR-3.3-S12, S13, S14 and FR-5.1-S8). **FR-8.2 and all seven of its scenarios were
retired on 2026-08-18**, which is the drop from 108 to 101; `spec.md`'s FR-8.2
entry carries the ruling. Counted rather than derived:

```
$ grep -oE "FR-[0-9]+\.[0-9]+-S[0-9]+:" spec.md | sort -u | wc -l
101
```

101 unique ids, 101 occurrences, no duplicates and no gaps. The per-requirement
tally is in the `## Phases` table and its columns sum to 101.

Three stale figures were in circulation on the way here — a "96" that was already
wrong when written, a "100" that was that figure plus four, and a "108" that a
retirement made wrong. `requirements.md` §7 no longer restates a number at all; it
carries the command above instead. A count kept by hand beside the thing it counts
drifts, and this one has drifted three times.

Reviewed once by `persona-architect` (Mode B). Two blockers, six majors and seven
minors were raised; all but two were folded in, and the two that were not are
recorded at the end of `## Decisions` with the reason.

---

## Drivers

### Quality attributes that matter here

| Attribute | Why it matters for *this* feature | Evidence |
|---|---|---|
| **Falsifiability of the picture** | This is the first spec in which pixels come from disk, so every instrument that judges a frame is calibrated against a generator that is about to stop being the source. `swatch.rs:35` asserts `TEXEL_COLORS = 2`; `probe.rs:124` clusters against `placeholder_mean_color`; `terrain_offscreen.rs` compares a centre pixel to it. All three premises change. | `requirements.md` §5.3, and the three files named there |
| **Evolvability of the published declaration** | `texture` is a mod-author-facing field whose *type* changes. A form shipped now cannot be withdrawn. | `code-quality.md` §1, "breadth of capability, narrowness of commitment" |
| **Reload correctness under a retained mesh** | `PreparedScene.meshed` is retained across a reload deliberately (`startup.rs:60-64`), and `Retained::rebuilt` re-packs the *whole* retained list on every batch (`remesh.rs`). Anything resolved at mesh time and cached in a `Quad` is stale on exactly that path. | `remesh.rs` `Retained::rebuilt`; `startup.rs:60` |
| **Operability of a build that is no longer self-contained** | `cargo build` alone stops producing a complete game. The whole mitigation is one named refusal. | ADR-026 items 1–2, `decisions.md:1229` |
| **Determinism** | A golden frame is a claim about bytes. The bake must be byte-identical run to run (FR-3.1-S4) and the fold must not move with the toolchain (FR-3.4-S4). | `format.rs:346-357` states why the hand-written FNV exists |
| **Coverage honesty** | ADR-013 narrows the exclusion to `mc-render/src/gpu/`. Anything expressible as a pure function is counted. Mip generation, the sampler request and layer resolution are all pure. | `crates/mc-render/CLAUDE.md`, "Verification"; `sdd-gate.ps1:55-60` |

### Constraints, read from the tree rather than assumed

1. **`toml` may not reach `mc-core`.** Stated at `crates/mc-core/src/hud/mod.rs:5`
   ("its raw form is spelled in TOML and `toml` may not reach this crate") and
   restated in `crates/mc-world/Cargo.toml`'s comment on its `toml` entry. This
   decides the index format outright (D6).
2. **`crates/` may not depend on `tools/`.** SPEC-013 FR-9.1, enforced against the
   resolved graph by `crates/mc-testkit/tests/workspace_layering.rs`.
3. **A layer index rides inside a packed vertex, eight bits wide.**
   `LAYERS_A_SESSION_MAY_ASSIGN = 256` (`mc-core/src/content.rs`), asserted
   against `mc-render`'s `MAX_LAYER` at compile time. Layers are appended and
   never renumbered.
4. **`mc-render` has no filesystem edge.** No `std::fs`, no `PathBuf`, no image
   decoder in `src/`. The client is the composition root and the only crate that
   builds `TextureLayers` (`mc-client/src/content.rs`).
5. **wgpu 30 refuses anisotropy unless all three filters are `Linear`.** Read at
   `~/.cargo/registry/src/*/wgpu-core-30.0.0/src/device/resource.rs:2288-2316`:
   `anisotropy_clamp != 1` checks `min_filter`, `mag_filter` **and**
   `mipmap_filter` in three separate arms. Verified, not quoted from the spec.
6. **A `TextureKey` has no character set.** `namespaced.rs:48` says so:
   "No character set is imposed." A key is `namespace:path` with exactly one
   separator and both sides non-empty, and nothing else. A key may therefore
   contain whitespace, a newline, a path separator or `..`.
7. **`content/base/models/generators/` is already tracked**, all three files, and
   `assemble_grass.py` landed in `133003e`. `gen_stone.py:118,139` writes
   `content/base/models/stone.mcvox`; the tracked model is `stone-block.mcvox`.
   `grass-block.mcvox:57` cites `scratchpad/gen_grass.py`, which does not exist.
8. **A test content root is a full recursive copy of `content/base`.**
   `mc-client/tests/support/content.rs`'s `copy_tree` copies every file and every
   subdirectory. A copied root therefore carries the manifest, the models, the
   materials *and* the built set. This is what makes D7's relative source paths
   work and what keeps ~40 fixture tests alive.
9. **`Facing` cannot move to `mc-core`.** `mc-world/src/mesh/facing.rs` depends on
   `crate::section::{Axis, LocalPos, SECTION_SIZE}` and `super::PlanePos`.
10. **`mc-render/src/gpu/buffers.rs` is a private module** (`gpu/mod.rs:33`,
    `mod buffers;`), `sampler()` at `buffers.rs:263` is a private free function
    taking no request, and there is no `_test.rs` sibling anywhere under
    `src/gpu/`. Nothing under `crates/mc-render/tests/` can reach either today.
11. **`load_materials` reads only `*.toml`.** `tools/voxforge/src/material.rs:167`
    filters on the extension before sorting.
12. **`pollster` is already a `[dev-dependency]` of `mc-render`**
    (`Cargo.toml:76`) under a comment that says "Dev-only: adapter and device
    acquisition belongs to the client and to the harness, never to this crate's
    library."
13. **Python 3.13.5 is on the developer machine and appears nowhere in the build.**
    No `.rs` file, no script and no CI names it. FR-8.2-S1/S2 would have introduced
    the first dependency on it; with FR-8.2 retired on 2026-08-18 **nothing in this
    spec depends on a Python interpreter**, and the constraint stands unchanged.
    There is no CI at all (`.github/` does not exist).
14. **`take_remesh_work` drains the entire dirty set into one batch**
    (`mc-sim/src/world/remesh.rs:84-95`). There is no partial drain and no bound
    on a batch. D3 rests on this.

### What is volatile, and what is expensive to reverse

| | |
|---|---|
| **Expensive to reverse** | The declaration's accepted forms (published to mod authors); `INPUT_VERSION` 1→2 (every stored save); the index's byte format (written by one crate, read by another that may not depend on it); the six facing words and their axes; the texture edge as a content-to-renderer contract. |
| **Cheap to reverse** | The manifest's TOML shape; the image file-name derivation (recorded in the index, so only voxforge knows the rule); the mip filter's implementation. |
| **Volatile** | The probe and swatch constants — re-derived from art that is itself new. Expect one round of re-derivation to be wrong and to be caught by FR-8.1-S5 rather than by FR-8.1-S4. |

### Halt check

No driver material to a binding decision is unknown. Four gaps in the enumerated
scenarios were raised while designing, ruled on, and closed in the spec; they are
listed with their rulings under `## Open questions`. The one risk large enough to
have blocked — whether the shipped models reproduce from their generators — was
measured rather than assumed, and is recorded under `## The generator spike`.

---

## Boundaries

| External dependency | Volatility (Vendor/Regulatory/API/Substitutability) | Port | Adapter location | Direct-use justification |
|---|---|---|---|---|
| `wgpu` 30 — device, sampler, array texture, `write_texture` | V low / R none / **API high** / Sub low | already isolated: `mc-render`'s optional `gpu` feature makes `wgpu::` nameable only under `src/gpu/` | `mc-render/src/gpu/` | — |
| `image` 0.25 (PNG decode), **new to `mc-client`** | V low / R none / API medium / Sub medium | `mc_render::texture::SuppliedTexels` is the domain-shaped value; the decoder never crosses out of one file | `mc-client/src/textures/decode.rs` — the only file in `mc-client` that may name `image::` | — |
| `image` 0.25 (PNG encode) in voxforge | as above | already isolated at `tools/voxforge/src/render/mod.rs:394` (`to_png`) | unchanged | — |
| The filesystem (built set, index, manifest, models, materials) | source of environment-dependence | no port — see justification | `mc-client/src/textures/`, `tools/voxforge/src/cli/build.rs` | `architecture-principles` §3 exempts the standard library. The project's precedent is `mc-world/src/content/luau_source.rs` and `mc-sim/src/content.rs`, which read content roots with `std::fs` directly; the *watcher* is behind a port because `notify` is a vendor, and no vendor is involved here. |
| `pollster` — **promoted from `[dev-dependencies]` to an optional `[dependencies]` entry under `gpu`** | V low / R none / API low / Sub trivial | none needed — a blocking executor is a pure library | `mc-render/src/gpu/buffers.rs` | `architecture-principles` §3: in-process pure library. **The manifest comment at `Cargo.toml:73-75` currently forbids this and must be amended in the same commit** — see D13. |

No network, no vendor service and no nondeterministic source other than the
filesystem is introduced. A Python interpreter was on this list until FR-8.2 was
retired on 2026-08-18; nothing in this spec invokes one now.

---

## Decisions

Sixteen binding, one deferred. Trivial decisions are one line at the end.

### D1 — `Face` is a `mc-core` primitive; `Facing → Face` is one exhaustive mapping in `mc-world` — BINDING

Six per-facing keys have to be held in `mc-core` (`BlockDefinition`,
`ResolvedBlock`) and indexed in `mc-render` (packing, HUD). `mc-core` cannot see
`mc_world::mesh::Facing` and `Facing` cannot move (constraint 9).

| | Shape | Cost |
|---|---|---|
| (a) | `mc-core` holds `[TextureKey; 6]` in `Facing`'s declaration order, indexed by `facing as usize` | A second authored copy of a declaration order, in a crate that cannot see the enum. Exactly the failure `facing.rs:29-33` and `geometry/mod.rs:47-64` are both written to prevent — a row nobody checked, whose symptom is a texture drawn on the wrong face while everything still draws. |
| (b) | Move `Facing` to `mc-core` | Drags the section coordinate system into a crate whose remit is "primitives, no I/O". |
| (c) | **`mc-core` defines `Face { Up, Down, North, South, East, West }`** — content's own published vocabulary (FR-1.3) — and `mc-world` provides `Facing::face(self) -> Face`, one exhaustive `match` | One mapping, in the only crate that can see both types, checked by a round trip over `Facing::ALL` and `Face::ALL`. |

**Decision: (c).** It is also the honest one: `up/down/north/south/east/west` is
what a *declaration* says and `NegX…PosZ` is what a *mesher* says, and FR-1.3 is
a published contract between exactly those two vocabularies. Writing it as one
total function in one place is writing the requirement itself.

The mapping is `up = PosY`, `down = NegY`, `north = NegZ`, `south = PosZ`,
`east = PosX`, `west = NegX` (`requirements.md` §4.1). It is the only place in
the workspace where a compass word meets an axis.

**Strongest argument against.** Two enums for six directions reads as duplication
on sight. Answer: the drift is closed mechanically by an exhaustive round trip
over both `ALL` arrays, and option (a) has the same drift with *no* place to put
such a test.

### D2 — The `Quad` stays geometric; packing takes a `TextureResolution` — BINDING

Option (b) is decided by the spec. What is decided here is the type.

`build_section_geometry` and `held_swatch` both need block+facing → key → layer.
Two loose values travelling side by side through `PreparedScene`, `Unuploaded`,
`Retained`, `Remesher::retire` and `FrameRenderer` would create a new defect
class: a batch packed with a reload's *new* keys against its *old* layers
resolves to a wrong-but-valid layer — a plausible wrong picture with no error.

**Decision: one type carrying both**, in `mc-render::texture`:

```rust
pub struct TextureResolution {
    blocks: BTreeMap<BlockName, FaceTextures>,   // mc_core::content::FaceTextures
    layers: TextureLayers,
}
```

`TextureLayers` is **unchanged** — the spec's "unchanged contract" holds
literally. `TextureResolution` is what travels; `TextureLayers` is what it
contains and what fills the array texture. `FrameRenderer` holds it in place of
its current `layers: TextureLayers` field (`gpu/hud.rs:66`) and
`texture_layers()` becomes `texture_resolution()`, which is what `app/mod.rs:344`
already reaches for when it composes a swatch.

Blast radius (mechanical, compiler-guided): `startup.rs`, `content.rs`,
`upload.rs`, `remesh.rs`, `session/reload.rs`, `app/reload.rs`, `app/mod.rs`,
`geometry/mod.rs`, `hud/held.rs`, `gpu/hud.rs`.

**Strongest argument against — and it is not diff size.** Every site that carries
`TextureLayers` must carry the block→faces map anyway, so *not* bundling is the
wider diff, not the narrower one. The real objection is that a bundled value
invites being stamped with a `ContentSerial` so that "packed against the content
serving" becomes checkable — and it must **not** be, because FR-2.1-S4 depends on
retained quads being packed against a *newer* resolution than the one they were
meshed under. `TextureResolution` deliberately carries no serial, and that
absence is load-bearing.

### D3 — `changes_geometry` widens to all six keys; the whole-world marking rule is untouched — BINDING

`mc-sim/src/world/reload.rs:81` keys `drawn_of` on `(is_solid, &TextureKey)`.
That field is becoming six, so this function must be touched whatever else is
decided.

**Options.** (a) widen it to compare all six keys — a facing-key change keeps
marking every section. (b) drop the texture from it entirely — under option (b)
of the spec a texture key changes no *geometry*, so a facing-key change would
mark nothing and a **re-pack** of the retained mesh would have to be triggered
instead.

**Decision: (a), widen it.** Three reasons, in order of weight:

1. `docs/technical/rendering.md:490-510` states the binary rule as a specification
   and says outright: "Narrowing this is a specification change, not an
   optimisation somebody may take while passing." This spec does not specify it.
2. The correctness it would buy is zero. A whole-world re-mesh is a superset of a
   whole-world re-pack.
3. Option (b) needs a re-pack trigger that does not exist. With nothing dirty,
   `take_remesh_work` yields no batch and `scene_of` is never called, so the
   vertices on the GPU keep their old layer indices. Building that trigger means a
   `Message::Retire` that itself produces a scene, which perturbs the
   `busy`/`in_flight` bookkeeping in `remesh.rs`.

**Consequence the test author must know.** Under (a), FR-2.1-S4's stated
precondition — "the sections holding that block were retained rather than
re-meshed" — is **not reachable through a production reload**, and constraint 14
is why: `take_remesh_work` drains the *entire* dirty set into one batch, so there
is not even a partial-drain window in which a retained-but-not-yet-re-meshed
section is drawn against the new resolution.

The property is real and falsifiable one level down. `Retained::rebuilt` re-packs
the *entire* retained list on every batch, so a section that was not re-meshed is
re-packed from its retained quads against whatever `TextureResolution` the worker
currently holds. FR-2.1-S4's test drives that seam: retain a meshed list built
under content A, hand the packer content B's resolution, re-pack, read the layer
back through `SectionGeometry::layer_at` (`geometry/mod.rs:110-124`, which exists
for exactly this). That test is red under option (a) of the spec's design choice
and green under option (b) — which is the whole reason it exists.

**Flagged to the spec owner as a wording issue**, not resolved unilaterally.

**Deferred, with a revisit-when.** Narrowing `changes_geometry` to solidity and
the name set, plus a re-pack-only reload path — **revisit when** a measurement
shows the whole-world re-mesh on a texture edit costs a mod author real time
(`docs/technical/testing.md:2511` measures it at 9.1 ms, which is why nobody has
asked).

### D4 — `mc-render` gains `SuppliedTexels`; the client decodes — BINDING

Forced by constraint 4. `mc-render` gains the ability to be *handed* level-0
texels per key and falls back to `placeholder_texels` where it is handed none.

`SuppliedTexels` is held by `FrameRenderer` for the whole run, given at
construction. It is **not** carried by `Unuploaded`/`retire`: the built set is a
pre-build artefact that does not change while the client runs, so a reload that
appends a key finds either art that was already read or no art at all. That is
FR-4.2's fallback reached by a second road, needing no new machinery.

### D5 — The texture edge is a content-to-renderer contract in `mc-core`, and voxforge refuses a model that will not bake to it — BINDING, and now in the spec

Three numbers currently sit in three places with nothing connecting them: a
model's `scale` (16, in the `.mcvox` header), a manifest's `pixels_per_voxel`, and
`mc-render`'s `PLACEHOLDER_SIZE = 16` (`placeholder.rs:80`). A model with
`scale = 32` bakes a 32×32 set that builds cleanly, commits cleanly, passes the
gate — and refuses the launch under FR-4.3-S1 with a message about an *image*,
pointing a mod author at a file they never authored.

**Decision.** `mc_core::content::TEXTURE_EDGE: u32 = 16`, declared beside
`LAYERS_A_SESSION_MAY_ASSIGN` and for the same stated reason: it is a property of
the content-to-renderer contract and not of either side. `mc-render` asserts its
own array-texture extent against it at compile time, exactly as it already does
for the layer bound. `PLACEHOLDER_SIZE` is deleted;
`placeholder_texels(key, size)` keeps its parameter.

**`voxforge build` refuses at build time**: for each model the manifest names, if
`model.scale × pixels_per_voxel ≠ TEXTURE_EDGE`, the build is refused naming the
model, its scale, the manifest's `pixels_per_voxel` and the edge required.
voxforge already depends on `mc-core`, so this is one constant and no new edge.

FR-4.3-S1's launch refusal stays, as defence in depth against a hand-tampered
set — the set is derived and gitignored, but it is a directory a person can write
into.

**Accepted into the spec** as FR-3.3-S14.

### D6 — The set's verdict is a five-arm enumeration, and the fifth arm is new — BINDING, and now in the spec

FR-5.1 states four verdicts. Applied literally to `prepare_scene`, a content root
with **no manifest at all** has an absent index and therefore refuses the launch.
Constraint 8 keeps the existing fixtures alive, but a root built from scratch —
and any mod author's root that ships no art — would be told to run the art build
before content declaring no art will load.

**Decision: five arms**, with the presence of `content/base/textures.toml`
separating `NoArtDeclared` from `Absent`. A total enum, so a check that loses the
ability to look cannot report the healthy arm — the property `spec.md` asks
FR-5.1 for.

**The verdict is what FR-5.1's scenarios assert, and it is not an error.**
`built_set` returns the verdict in its `Ok`; a separate named function,
`refusal_for`, maps a verdict to the `PreparationError` a player reads, and
FR-5.2's two scenarios assert *that* text.

**Amended in P7: `refusal_for` lives in `textures/mod.rs`, which is where the
Interfaces section below has always listed it.** This prose said `startup.rs` and
the two disagreed; the listing wins, and P7's tests bind it. Beside the enum, a
new arm with nothing said about it is a non-exhaustive match in the file that
declared it rather than a silent `None` in another one — and `tasks.md` already
flags `startup.rs` for headroom. Returning the verdict only as an error
would leave three arms unconstructible in `Ok`, and "a total enumeration, never
an absence check" would not be what the suite was holding.

**Accepted into the spec** as FR-5.1-S8, with FR-5.1's lead line widened to name
the two verdicts that do not refuse.

### D7 — The index is a line format defined in `mc-core`, parsed by neither side twice — BINDING

voxforge writes the index; `mc-client` reads it; neither may depend on the other
(constraint 2), and `toml` may not reach `mc-core` (constraint 1). Two hand-rolled
parsers agreeing forever is what `requirements.md` §4.4 refuses for the hash, for
the same reason.

**Decision:** `mc_core::art::TextureSetIndex`, a pure parse/render pair over
`&str`/`String` with **no new dependency**. `mc-core` performs no I/O: voxforge
renders it and writes the bytes; the client reads the bytes and parses.

```
mycraft-texture-set 1
fold 8f14e45fceea167a
source models/grass-block.mcvox
source materials/dirt.toml
key base__grass_top.png base:grass_top
```

- `mycraft-texture-set 1` must be the first line.
- `fold` is 16 lowercase hex digits.
- `source` lines are **in fold order**; the path is the rest of the line, relative
  to the manifest's own directory, `/`-separated. The client re-folds the recorded
  list in the recorded order; **it never reads the manifest.**
- `key <image> <key>` — the image name is a token, the key is the rest of the
  line. This order and not the other, because a `TextureKey` may contain
  whitespace (constraint 6) and an image name may not (D9).
- **Refused on both sides — render and parse — because the key is content text
  (constraint 6): any ASCII control character in a key or a source path.**
  Without this, `key = "base:a\nfold 0000000000000000"` is a spellable manifest
  entry that makes `rendered()` emit an index `parse` reads with a forged fold or
  a forged extra source. `stating` refuses it, `voxforge build` reports it naming
  the key, and `parse` refuses it again.
- Parse also refuses: an unknown leading word; a `source` or `key` path that is
  absolute, contains `\`, contains a `..` component, or is empty; a duplicate key;
  a malformed fold.

### D8 — Paths in the index are relative to the manifest's directory — BINDING

The manifest names its models and its materials directory relative to itself; the
index records the sources the same way; the client resolves them against the
content root it was given.

This is what makes constraint 8 work: a `copy_tree`'d root re-folds to the same
value inside the temp directory, so every existing fixture test stays green
without being touched. Absolute paths would make a copied root permanently stale
and would put developer home directories into a file the gate builds.

### D9 — The image file name is derived, validated, and recorded — BINDING, and now in the spec

A key has no character set (constraint 6), so `base:../../../etc/passwd` is a legal
`TextureKey`. Deriving a file name from a key is deriving a path from
unconstrained content text — `code-quality.md` §7.

**This is correctness and reproducibility, not a threat model, and it should be
written up that way.** A server owner chooses which mods are installed, so a
declaration is not hostile input and there is no attack surface to argue about.
The arguments that survive are plainer and stronger: a content string silently
becoming a filesystem path is a foot-gun whatever the author intended, a key that
will not round-trip through the index is a correctness defect in a file the client
parses, and a build whose output location depends on punctuation in a key is not
reproducible. `docs/modding/` should say exactly that and no more.

**Decision.** voxforge derives `image = key.replace(':', "__") + ".png"` and
**refuses the build** unless the result matches `[A-Za-z0-9._-]+\.png` and is
neither `.` nor `..`, naming the key and the rule (FR-3.3-S12). It refuses
separately a key carrying a line break, which would otherwise forge an index
record (FR-3.3-S13). The client never re-derives the name: it reads it from the
index and refuses any name failing the same shape (D7).

**Accepted into the spec** as FR-3.3-S12 and S13.

### D10 — `fnv_1a_64` moves to `mc-core::hash`; nothing else moves with it — BINDING

Only the byte fold moves (`format.rs:358-365`). `folded()` and `DefinitionHash`
stay in `mc-world`, because `folded` names `postcard` and `postcard` is confined
to `mc-world/src/persistence/` by that crate's own manifest comment.

`placeholder.rs:64-70` carries a **third** copy of the same two constants,
inlined, with its own reasoning. It is left alone: it hashes a key to a colour and
is not the value two independent programs must agree on. Folding it in is a
refactor with a golden-frame blast radius and no correctness gain — a deferred
observation, not taken.

### D11 — The fold's byte sequence, stated so a test can build an independent oracle — BINDING

FR-3.4-S4 and FR-9.1-S2 both demand a value derived from a *stated* byte sequence
rather than snapshotted from a run. The sequence is:

> For each source, in order: the source's recorded path as UTF-8 bytes, preceded
> by its length as a little-endian `u64`; then the file's bytes, preceded by their
> length as a little-endian `u64`. FNV-1a-64 over that concatenation, offset basis
> `0xcbf29ce484222325`, prime `0x100000001b3`.

Length prefixes rather than separators, so a file containing the separator byte
cannot forge a boundary — the reasoning `format.rs:330-333` gives for postcard's
own length prefixes.

Source order: the manifest first; then each model, in the order of its first
mention in the manifest, de-duplicated; then **every `*.toml` file in the
materials directory, sorted by file name**.

**Every material `.toml`, not only the ones a palette names, and nothing that is
not a `.toml`.** `load_materials` reads the whole directory but filters on the
extension first (constraint 11), so `*.toml` is exactly the build's input.
Folding a subset would record a value that is not a function of what the build
consumed; folding a stray `README.md` would make the set stale for an input the
build never read, which is the spurious-refusal failure FR-3.4-S3 exists to
prevent arriving by the other door.

FR-3.4-S3's negative control is about models under `content/base/models/`, which
this does not fold unless the manifest names them — that scenario stays satisfied
and stays the control it was written to be.

### D12 — The build groups entries by model, emits all six faces, and refuses on the selected ones — BINDING

FR-3.3-S9 requires a non-cubic model to be refused naming the axis. That check
lives in `emit`'s `FaceSelection::All` arm only (`emit.rs:120-151`); a per-entry
`FaceSelection::One` would never reach it.

**Decision.** For each distinct model the manifest names, load and assemble once,
call `emit` with `FaceSelection::All` and `SeamPolicy::Reported`. The cubic
precondition fires for free (FR-3.3-S9). The build then refuses on the first
failing seam verdict of a face **some entry selected** (FR-3.3-S7) and passes over
verdicts on faces nobody asked for — refusing on those would refuse a build for a
face the manifest never wanted. `EmittedFace.verdicts` is a never-empty
enumeration (`emit.rs:70-77`), so "this tiles" is an answer rather than an
absence.

This also removes six renders per model that a naive per-entry loop would do.
`written_together` (`cli/mod.rs`) already deletes landed files when a later write
fails; the build reuses it across the whole set, which is FR-3.3-S10.

### D13 — The build is all-or-nothing and the fold is its cache key — BINDING

ADR-026 item 5 asks for a cache keyed on `.mcvox` content. **Decision:** the fold
of D11 is that key, and it is whole-set. Fold the sources; if the value equals the
index's recorded value *and* every image the index names is present, report that
nothing needed rebuilding and touch no file (FR-3.2-S1, S2). Otherwise rebuild the
whole set (FR-3.2-S3).

Per-entry caching was rejected: it needs a second, finer-grained record the client
would then also have to understand, for seven 16×16 images.

### D14 — Mip levels are pure arithmetic outside `src/gpu/`; the sampler is a pure request the GPU layer translates, through a seam that exists — BINDING

The ADR-013 line, stated per piece:

| Piece | Side | Why |
|---|---|---|
| `texture::mip::{to_linear, to_stored}` — the sRGB transfer pair | **outside** `src/gpu/`, normal coverage | This is the conversion FR-6.1-S2 pins at byte 188. It must sit where a test can call it directly. |
| `texture::mip::{reduced, chain}` — box average in linear light | **outside**, normal coverage | Pure arithmetic. `crates/mc-render/CLAUDE.md`: "Anything expressible as a pure function … is not exempt." |
| `texture::mip::levels_for` — supplied-or-placeholder, dimension check, chain, level-count check | **outside**, normal coverage | FR-6.1-S5's refusal is reachable with no device. |
| `texture::sampler::{SamplerRequest, TERRAIN_SAMPLER, asks_for_anisotropy}` | **outside**, normal coverage | FR-6.2-S3 inspects the constant; FR-6.2-S4 inspects a fixture that does ask, as its positive control. |
| `gpu::buffers::array_texture` (`mip_level_count: MIP_LEVELS`) | **inside**, excluded | Device object. |
| `gpu::buffers::write_layer` (the per-level `write_texture` loop) | **inside**, excluded | Device call, carrying no arithmetic: it iterates what `levels_for` returned. |
| **`gpu::terrain_sampler(device, &SamplerRequest) -> Result<wgpu::Sampler, RendererError>`** — the translation, inside a validation error scope | **inside**, excluded | Device call. Its correctness is carried by FR-6.2-S1/S2 in a captured frame. |

`MIP_LEVELS = TEXTURE_EDGE.ilog2() + 1` — **derived**, never written as `5`. A
size and a level count that can disagree is a copy that overruns.

**The seam is the decision, not an implementation detail.** Constraint 10 says
`buffers` is private, `sampler()` is private and takes no request, and nothing in
`crates/mc-render/tests/` can reach either. Two scenarios need to:

- **FR-6.2-S5** builds a sampler from a deliberately invalid `SamplerRequest`
  (anisotropy 16 with nearest magnification) and asserts the refusal names the
  combination.
- **FR-6.2-S2** captures the same pair of frames through *unfiltered*
  minification, which needs a renderer built with a second sampler configuration.

So `terrain_sampler` is `pub` and re-exported from `gpu` under
`#[cfg(feature = "gpu")]`, and the request is threaded
`FrameRenderer::new → TerrainRenderer::new → SceneBuffers::new → terrain_sampler`
as one borrowed parameter carrying both it and the supplied texels
(`TerrainTextures<'_>`, see `## Interfaces`). Production passes `TERRAIN_SAMPLER`
from the composition root; the capture harness passes the other value. Without
this parameter D14 is a decision whose two witnesses cannot be written.

**FR-6.2-S5 uses a real device refusal, not a pre-check.** A pure pre-check
re-implementing wgpu's rule was rejected: it is a second copy of a vendor
constraint that drifts silently when the vendor changes, and the scenario's
subject is the device. `terrain_sampler` wraps `create_sampler` in
`device.push_error_scope(Validation)` / `pop_error_scope` and maps a refusal to
`RendererError::TerrainSampler { requested }`.

**`pollster` moves from `[dev-dependencies]` to an optional `[dependencies]` entry
under the `gpu` feature, and its manifest comment is amended in the same commit.**
Constraint 12: today that comment says device acquisition belongs to the client
and the harness, never to this crate's library. Blocking on an already-ready error
scope is not device acquisition, but the rule as written forbids it, so the rule
is amended deliberately rather than contradicted silently. If `pop_error_scope`
turns out not to be blockable inside `SceneBuffers::new`, the fallback is the pure
pre-check and FR-6.2-S5 is authored against that — Assumption 5.

### D15 — Two new gate stages, selectable alone, each parameterised on the path it looks at and never on the repository — BINDING

**Amended 2026-08-18.** This decision carried three stages until FR-8.2 was
retired; the stage that scanned the generators for absolute output paths went with
it, and so did its `-GeneratorRoot` seam. What that stage was to do is described in
`spec.md`'s FR-8.2 entry rather than kept here, because it never reached the
script.

FR-7.1 has six scenarios and **nothing in this repository has ever tested
`scripts/sdd-gate.ps1`**. A structural text scan can honestly answer the ordering
and guarding scenarios; it cannot answer the ones about what a stage *does*.

**Decision.** `sdd-gate.ps1` gains two stages and two parameters. The current
stage 7 (tests + coverage) becomes stage 9.

| # | Stage | Does | Scenarios |
|---|---|---|---|
| 7 | `art (generated set not committed)` | fails when `git ls-files -- <ContentRoot>/textures` reports anything, naming each path | FR-7.1-S1, S2 |
| 8 | `art (voxforge build)` | runs `cargo run -p voxforge --quiet -- build <Manifest>`, writing the tool's own output through; **on failure stage 9 is skipped** | FR-7.1-S3, S4, S5, S6 |

**A narrow instrument reports its own blind spot as a zero, and naming the family
is the point** — it outlives the stage that prompted it. The zero is
indistinguishable from a clean result, and this project met the family three times
in one run: a `^FAIL` filter that cannot match nextest's indented line, a `grep`
that matched leading indentation rather than the mid-sentence gaps it was aimed
at, and a rule that would have tested only for a leading `/` while the path it
existed to catch was a Windows drive letter. Two of the three were *positive
controls* — the worst version, because a control that passes for the wrong reason
certifies the instrument instead of testing it. Whenever a scan is written here,
the question to ask is not "does it find the thing" but **"what shape of the thing
would it miss, and is the defect I am guarding against that shape?"** The
path-shaped instance is carried into `docs/technical/testing.md`, since this
document is archived and then pruned.

- `-ContentRoot` (default `content/base`) and `-Manifest` (default
  `content/base/textures.toml`) are the fixture seams. `-ArtOnly` runs stages 7–8
  and nothing else. Both stages sit **outside** the `if ($SkipCoverage)`
  block and after the `-Quick` early exit (`sdd-gate.ps1:257`), which is what
  makes FR-7.1-S3 and S4 one placement rather than two.
- **There is deliberately no `-RepoRoot`.** Pointing the whole script at a
  temporary tree makes both stages answer the wrong question: `git ls-files` fails
  with "not a git repository", so a clean fixture reddens for a reason unrelated
  to the property, and `cargo run -p voxforge` fails with "no such package", so a
  broken-manifest fixture greens for a reason unrelated to it — and FR-7.1-S5 asks
  the stage to *reproduce the build's refusal*, which "no such package" is not.
  Parameterising each stage on the path it inspects keeps `git` and `cargo`
  running against the real repository and the real workspace, which is what they
  are actually about.
- A Rust test drives `pwsh -File scripts/sdd-gate.ps1 -ArtOnly -Manifest <temp>`
  against fixture trees and asserts an enumerated verdict over exit code and
  output.

**Amended 2026-08-19, from implementing it.** This bullet said `-ContentRoot
<temp>` as well, and that is wrong for the same reason there is no `-RepoRoot`,
one level further down. `git ls-files` on a pathspec **outside the worktree**
exits 128 with `is outside repository at 'E:/…'` — measured on this checkout —
so a clean fixture and a dirty one fail identically, for a reason unrelated to
the property. A pathspec *inside* the repository naming a directory that does
not exist is not refused: it reports nothing and exits 0, which is what makes a
clean fixture legitimate. So the two content roots are **committed fixtures**
under `crates/mc-client/tests/fixtures/gate/`, differing by exactly one tracked
file — the built image — while `-Manifest` still takes a temporary path, because
`voxforge build` writes beside the manifest it is handed and a run aimed at a
tracked fixture would leave built images in the repository. The finding is
carried into `docs/technical/testing.md`, since this document is archived and
then pruned.

**The skip is a deliberate exception to a documented property.** The script's
header says "Every stage runs even if an earlier one fails". FR-7.1-S6 requires
the opposite here, because `voxforge build` leaves the previous set intact when it
refuses (FR-3.3-S10), so running the suite anyway would grade a stale set. The
header must state the exception in one sentence beside the property.

**Skipping means failing fast, never proceeding without the tests, and the
script's own structure already guarantees it — with one thing that must be
added.** Every stage records its failure in `$Failures`, and the summary is
`if ($Failures.Count -eq 0) { GATE PASSED; exit 0 }` followed by `exit 1`
(`sdd-gate.ps1:398-408`). A refused build adds to `$Failures`, so the gate exits
non-zero whatever happens afterwards, and not running stage 9 cannot remove a
failure already recorded. **What must be added is that the skip records itself**
— `$Failures.Add('tests (not run: art build failed)')` — because otherwise the
summary simply lists one fewer stage, and a reader cannot tell "the tests were not
run" from "the tests are not in this list". A gate that omits a stage silently is
one step from a gate that skips its way to green.

**On `-ArtOnly` being a test-only mode.** It is not a defect: it selects which
stage bodies run, exactly as `-Quick` already does, and restates no stage. There
is one implementation of each stage, and the structural test on stage ordering is
what ties the selection to the real sequence. CLAUDE.md's "deliberately the only
gate script" is respected — no second script.

### D16 — ~~The generators are run against a staged repository root, never against the working tree~~ — RETIRED 2026-08-18

**Retired with FR-8.2, and the heading is kept so the number is not silently
reused.** D16 governed how the tests for FR-8.2-S1 and S2 were to run the
generators: against a staged temporary root laid out as a repository, never
against the working tree, because a failing test run literally against the
checkout overwrites a tracked model and a run that fails half way leaves the
repository in whatever state it ended in. With those scenarios retired there are
no such tests, so there is nothing left for the decision to bind. `spec.md`'s
FR-8.2 entry carries the retirement and the owner's ruling.

**One thing D16 established survives the decision and is not retired**, because it
is a property of the artefacts rather than of a test: **the reproducibility runs
one way only.** `assemble_grass.py` carries hand-authored courses as literal grids
— the sod shadow, three lone blades at deliberately different depths — that appear
in no generator and cannot be recovered from the model, because the model is their
*output*. So generator + assembler → model is a claim about how the file came to
exist; **model → generator is not a claim of any kind.** That is why the scripts
are kept rather than deleted, and it is what P4's documentation task states for a
mod author in `docs/modding/voxel-models.md`.

---

### The two review findings not folded in

- **Splitting FR-7.2 out of P9 into P7.** Not taken. The reviewer is right that
  FR-7.2's discriminating half ("*rather than* a golden-frame mismatch") cannot be
  exercised in P7, where no pixel depends on the set — which is exactly why FR-7.2
  is assigned to **P9** below and not to P7 at all. Recorded here so the next
  reader does not re-derive it.
- **Landing P9's art and its sampler wiring as two commits with an uncommitted
  reference capture between them.** Taken as guidance in P9's notes rather than as
  a binding decision: it is a working practice inside one phase, and binding it
  would be prescribing how an implementer bisects.

### Trivial decisions

- `build` is a fourth `Command` variant beside `preview`, `inspect`, `texture`
  (`cli/args.rs:41-48`). Its body lives in a new `cli/build.rs` — `cli/mod.rs` is
  392 lines against a 500-line limit.
- `Command::document()` returns the manifest path for `Build`; `states()` returns
  `&[]`. A manifest states no part states.
- The manifest is `content/base/textures.toml`; the set is
  `content/base/textures/`; the index is `content/base/textures/index.txt`.
- `.gitignore` gains `/content/base/textures/` — the *directory*, so a stray file
  in it is ignored too, and the manifest one level up is untouched.
- The held-block indicator draws `Face::North`, named once as
  `mc_render::hud::held::INDICATOR_FACE` with `requirements.md` §4.7's reason.
- `mc-client` gains a `textures` module (`mod.rs`, `index.rs`, `decode.rs`);
  `image` is added to its manifest with the boundary comment its other confined
  dependencies carry.
- The manifest states `pixels_per_voxel = 1` at the top level, so the 16×16
  decision is visible in a committed, reviewable file rather than in a default.

---

## Interfaces

### `mc-core`

```rust
// mc-core/src/content.rs
/// The edge of one block texture, in texels.
///
/// A property of the content-to-renderer contract and not of either side, on the
/// same terms as `LAYERS_A_SESSION_MAY_ASSIGN`: `mc-render` allocates its array
/// texture to it and asserts agreement at compile time, and `voxforge` refuses a
/// model that will not bake to it. **It is never restated elsewhere.**
pub const TEXTURE_EDGE: u32 = 16;

/// A block face, in the vocabulary a declaration writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Face { Up, Down, North, South, East, West }

impl Face {
    /// Every face, in the order a refusal lists them.
    pub const ALL: [Self; 6] = [Self::Up, Self::Down, Self::North, Self::South, Self::East, Self::West];
    /// The word a declaration spells this face with.
    pub const fn as_str(self) -> &'static str;
    /// The face `word` names, or nothing where it names none.
    /// Exact match: `Up` is not `up` (FR-1.2-S4).
    pub fn named(word: &str) -> Option<Self>;
}

/// Which texture key each of a block's six faces draws from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FaceTextures { /* private */ }

impl FaceTextures {
    /// One key on all six faces.
    pub fn uniform(key: TextureKey) -> Self;
    /// A key per face, **positionally in `Face::ALL` order**.
    ///
    /// Positional rather than `[(Face, TextureKey); 6]`, which would let two
    /// entries name `Up` and make `at` a lookup that can miss.
    pub fn stating(keys: [TextureKey; 6]) -> Self;
    /// The key `face` draws from. Total: no `Option`, no indexing.
    pub fn at(&self, face: Face) -> &TextureKey;
    /// Every distinct key, in lexicographic order.
    pub fn keys(&self) -> BTreeSet<TextureKey>;
}
```

`BlockDefinition.texture: TextureKey` → `BlockDefinition.textures: FaceTextures`.
`ResolvedBlock.texture: TextureKey` → `ResolvedBlock.textures: FaceTextures`.
`BlockRegistry::texture_keys` unions `definition.textures.keys()`.

```rust
// mc-core/src/hash.rs — moved verbatim from mc-world/src/persistence/format.rs:358
pub fn fnv_1a_64(bytes: &[u8]) -> u64;

// mc-core/src/art.rs — the index contract, pure, no I/O
pub struct TextureSetIndex { /* private */ }
pub struct IndexEntry { pub key: TextureKey, pub image: String }

pub enum IndexError {
    NotAnIndex { first_line: String },
    UnknownRecord { line: usize, word: String },
    MalformedFold { line: usize },
    UnsafePath { line: usize, path: String },
    ControlCharacter { line: usize, field: &'static str },
    DuplicateKey { line: usize, key: TextureKey },
}

impl TextureSetIndex {
    /// # Errors
    /// Returns [`IndexError::ControlCharacter`] where a key or a source path
    /// carries one — see D7 for why rendering it would be an injection.
    pub fn stating(fold: u64, sources: Vec<String>, entries: Vec<IndexEntry>) -> Result<Self, IndexError>;
    /// The index as the bytes voxforge writes.
    pub fn rendered(&self) -> String;
    /// # Errors
    /// Returns [`IndexError`] naming the line and what is wrong with it.
    pub fn parse(text: &str) -> Result<Self, IndexError>;
    pub fn fold(&self) -> u64;
    pub fn sources(&self) -> &[String];
    pub fn entries(&self) -> &[IndexEntry];
}

/// The fold of D11, over sources already read into memory.
pub fn folded_sources(sources: &[(&str, &[u8])]) -> u64;
```

`folded_sources` is where both sides meet: voxforge and `mc-client` each read the
bytes themselves and hand them here. `mc-core` still opens no file.

### `mc-world`

```rust
impl Facing {
    /// This facing in the vocabulary a declaration writes.
    ///
    /// The one place a compass word meets an axis (FR-1.3).
    pub const fn face(self) -> mc_core::content::Face;
}
```

`luau_declaration.rs` gains a `declared_textures` reading either a string or a
table. The table is read through `host.field_names(table, SIX_FACINGS_READ)` and
`host.read_field(table, word)`, raw, so a declaration's metatable neither supplies
a facing it did not state nor hides one it did — the property the module header
already states, extended one level down.

| Scenario | Refusal |
|---|---|
| FR-1.2-S1/S2 | ``texture` states no key for `south`, `east`, `west`; a texture table states all six of `up`, `down`, `north`, `south`, `east`, `west`` |
| FR-1.2-S3/S4 | ``top` is not a facing a texture table may state; a texture table may state `up`, `down`, `north`, `south`, `east`, `west`` |
| FR-1.2-S5 | ``up` must be a string, but is a number` (existing `wrong_kind`) |
| FR-1.2-S6 | the `NamespacedIdError::MultipleSeparators` sentence, unchanged (existing `invalid`) |
| FR-1.2-S7 | ``texture` must be a string or a table of six facings, but is a boolean` |

Final wording is the implementer's against `documented_refusals.rs`, which compares
`docs/modding/blocks-items.md` to a real run line for line. `RECOGNISED_FIELDS`
order is untouched: `texture` keeps its name and its position.

`persistence/format.rs`: `DeclaredAppearance` gains the six keys in `Face::ALL`
order, `INPUT_VERSION` goes 1 → 2, `fnv_1a_64` is imported from `mc-core`.

### `mc-render`

```rust
// texture/mod.rs
pub const MIP_LEVELS: u32 = mc_core::content::TEXTURE_EDGE.ilog2() + 1;   // 5

pub struct TextureResolution { /* private */ }
impl TextureResolution {
    pub fn stating(blocks: impl IntoIterator<Item = (BlockName, FaceTextures)>, layers: TextureLayers) -> Self;
    pub fn key_of(&self, block: &BlockName, face: Face) -> Option<&TextureKey>;
    pub fn layers(&self) -> &TextureLayers;
}

// texture/supplied.rs — pure
pub struct SuppliedTexels { /* private */ }
impl SuppliedTexels {
    pub fn stating(entries: impl IntoIterator<Item = (TextureKey, Vec<[u8; 4]>)>) -> Self;
    pub fn none() -> Self;
    pub fn covering(&self, key: &TextureKey) -> Option<&[[u8; 4]]>;
}

// texture/mip.rs — pure
pub fn to_linear(stored: u8) -> f32;
pub fn to_stored(linear: f32) -> u8;
pub fn reduced(level: &[[u8; 4]], size: u32) -> Vec<[u8; 4]>;
pub fn chain(level0: &[[u8; 4]], size: u32) -> Vec<Vec<[u8; 4]>>;
pub fn levels_for(key: &TextureKey, supplied: &SuppliedTexels, size: u32)
    -> Result<Vec<Vec<[u8; 4]>>, TextureError>;

// texture/sampler.rs — pure, wgpu-free. Both types carry `Display`, because
// FR-6.2-S5 requires the refusal to name the combination it requested.
pub enum Filter { Nearest, Linear }
pub struct SamplerRequest { pub magnify: Filter, pub minify: Filter, pub between_levels: Filter, pub anisotropy: u16 }
pub const TERRAIN_SAMPLER: SamplerRequest = SamplerRequest {
    magnify: Filter::Nearest, minify: Filter::Linear, between_levels: Filter::Linear, anisotropy: 1,
};
pub fn asks_for_anisotropy(request: &SamplerRequest) -> bool;

// geometry/mod.rs
pub fn build_section_geometry(quads: &[Quad], origin: SectionOrigin, resolution: &TextureResolution)
    -> Result<SectionGeometry, GeometryError>;

// hud/held.rs
pub const INDICATOR_FACE: Face = Face::North;
pub fn held_swatch(held: Option<&BlockName>, resolution: &TextureResolution) -> HeldSwatch;

// gpu/mod.rs — the seam D14 requires, under #[cfg(feature = "gpu")]
/// Everything the array texture and its sampler are built from.
///
/// Borrowed and passed as one parameter so that no function on the path exceeds
/// four arguments (`clippy::too_many_arguments`, threshold 4).
pub struct TerrainTextures<'a> { pub supplied: &'a SuppliedTexels, pub sampler: SamplerRequest }

/// # Errors
/// Returns [`RendererError::TerrainSampler`] when the device refuses `request`.
pub fn terrain_sampler(device: &wgpu::Device, request: &SamplerRequest)
    -> Result<wgpu::Sampler, RendererError>;

impl FrameRenderer {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, config: &TerrainPassConfig,
               textures: &TerrainTextures<'_>) -> Result<Self, RendererError>;
    pub fn upload_textures(&mut self, queue: &wgpu::Queue, resolution: &TextureResolution)
        -> Result<(), RendererError>;
    /// Replaces `texture_layers`. What a swatch is looked up in has to be what
    /// the array texture was filled from, which is why it is lent from here.
    pub const fn texture_resolution(&self) -> &TextureResolution;
}
```

Error contracts:

```rust
pub enum GeometryError {
    /// FR-2.1-S3. Replaces `UnresolvedTexture { block }`; the face and the key
    /// are both named, because a block with six keys leaves a reader guessing
    /// otherwise.
    UnresolvedTexture { block: BlockName, face: Face, key: Option<TextureKey> },
    Pack(#[from] PackError),
}

pub enum TextureError {
    /// FR-6.1-S5.
    TooFewLevels { key: TextureKey, offered: usize, declared: usize },
    /// Supplied texels whose count is not `size * size`.
    WrongTexelCount { key: TextureKey, offered: usize, declared: usize },
}

pub enum RendererError { /* … existing … */
    /// FR-6.2-S5. `SamplerRequest: Display` is what lets this name the
    /// combination.
    TerrainSampler { requested: SamplerRequest },
    Texture(#[from] TextureError),
}
```

`HeldSwatch` keeps its three-arm totality (FR-2.2-S4) and gains the face it was
looking at.

### `mc-client`

```rust
// textures/mod.rs
/// What the built set under a content root is. Total: a check that stops looking
/// cannot report `Current`.
pub enum SetVerdict {
    /// The root declares no texture manifest, so there is nothing to build.
    NoArtDeclared,
    /// A manifest is present and the index is not.
    Absent,
    /// The index's recorded fold no longer matches its sources.
    StaleAgainstSources,
    /// A source the index recorded is no longer there (FR-5.1-S4).
    SourceMissing { source: PathBuf },
    /// The index names an image that is not present (FR-5.1-S5).
    ImageMissing { key: TextureKey, image: PathBuf },
    Current,
}

/// Why the set could not be read at all — a different axis from what it *is*.
pub enum TextureSetError {
    Unreadable { path: PathBuf, cause: io::Error },
    Index { path: PathBuf, cause: IndexError },
    /// FR-4.3-S1.
    Size { key: TextureKey, found: (u32, u32) },
    /// FR-4.3-S2.
    NotAPng { key: TextureKey, image: PathBuf },
}

/// What the set under `root` is, and the texels it offers when it is current.
///
/// **The verdict is returned, never raised.** FR-5.1's seven scenarios assert it;
/// `refusal_for` below is what FR-5.2's two assert.
pub fn built_set(root: &Path) -> Result<(SetVerdict, SuppliedTexels), TextureSetError>;

/// The refusal a verdict becomes, or `None` where the launch continues.
pub fn refusal_for(verdict: &SetVerdict) -> Option<PreparationError>;
```

`PreparationError` gains, beside `NoContentRoot` — five variants, not four, and
the stale/source-missing split is because an `Option<PathBuf>` cannot be
conditionally rendered inside one `thiserror` format string:

```rust
#[error("the generated texture set is not there; run `{BUILD_THE_TEXTURE_SET}`")]
TextureSetAbsent,
#[error("the generated texture set is stale against its sources; run `{BUILD_THE_TEXTURE_SET}`")]
TextureSetStale,
#[error("the generated texture set was built from `{missing}`, which is no longer there; run `{BUILD_THE_TEXTURE_SET}`", missing = missing.display())]
TextureSetSourceMissing { missing: PathBuf },
#[error("the generated texture set names an image for `{key}` that is not there: {image}", …)]
TextureSetImageMissing { key: TextureKey, image: PathBuf },
#[error(transparent)]
TextureSetUnreadable(#[from] TextureSetError),
```

**Amended in P7, measured rather than chosen.** The field is `missing` and not
`source`: `thiserror` reads any field named `source` as the error's cause, and the
variant as first written does not compile — *the method `as_dyn_error` exists for
reference `&PathBuf`, but its trait bounds were not satisfied*. The `Display` text
above is unchanged and no scenario names the field.

**`TextureSetError` gains a fourth arm in P7, `UnusableImageName { key, image }`,
approved by the conductor and recorded here.** The client takes an image name out
of the index and joins it onto a path. `TextureSetIndex::parse` accepts
`elsewhere/base__stone.png` — relative, `/`-separated, naming no parent — and D9
binds the reader to refuse a name failing `mc_core::art::is_an_ordinary_image_name`
rather than deriving the name a second time. No arm listed above says that, and
this client is that function's first caller inside `crates/`. `Size` and `NotAPng`
stay listed and unbuilt: they are FR-4.3 and arrive with the decode in P9.

`BUILD_THE_TEXTURE_SET` is a `const` beside `LOAD_CHANGED_BLOCKS` in
`startup.rs`, spelled **once**, for the reason that constant carries: a message
quoting a command nothing accepts reads as a way out and is not one. `README.md`
and `docs/modding/voxel-models.md` quote the same string.

`prepare_scene` calls `built_set(root)`, maps the verdict through `refusal_for`,
and carries the texels into `PreparedScene` — which is what makes FR-7.2 true of
the golden suites, since they go through `prepare_scene`.

### `tools/voxforge`

```rust
// cli/args.rs
pub enum Command { Preview(..), Inspect(..), Texture(..), Build(BuildArgs) }
pub struct BuildArgs { pub manifest: PathBuf }

// texture/manifest.rs
pub struct Manifest { pub output: PathBuf, pub materials: PathBuf, pub blocks: PathBuf,
                      pub pixels_per_voxel: NonZeroU32, pub entries: Vec<ManifestEntry> }
pub struct ManifestEntry { pub key: TextureKey, pub model: PathBuf, pub face: AxisAlignedView }
pub fn load_manifest(path: &Path) -> Result<Manifest, Fault>;
```

Every DTO field is `Option<toml::Value>` with `deny_unknown_fields` — the shape
`format/dto.rs` already uses — so a manifest key nobody recognises is refused in
the author's terms rather than serde's.

The manifest, as committed:

```toml
# content/base/textures.toml
output           = "textures"
materials        = "materials"
blocks           = "blocks"
pixels_per_voxel = 1

[[texture]]
key   = "base:grass_top"
model = "models/grass-block.mcvox"
face  = "top"
# … six more
```

`face` parses through `AxisAlignedView`'s own vocabulary — `front`, `back`,
`left`, `right`, `top`, `bottom` (`texture/set.rs:24-30`) — and a value that is
not one of the six is refused naming all six (FR-3.3-S4). It does **not** go
through `view_named`, which would name the isometric views too.

**FR-3.3-S11's unused-key report is a text scan, not a content load.** The build
reads each `blocks/*.luau` as text and reports a manifest key whose exact spelling
appears in none of them.

The reason generalises past this scenario and belongs in the record: **an art
tool must not depend on the script host.** Reading FR-3.3-S11 literally puts a
Luau VM inside a texture baker and lets a broken block declaration refuse an art
build that has nothing to do with it — a coupling that would outlive whatever
convenience it bought.

**The limitation is documented where the scan lives**, in a comment on the scan
itself as well as in `docs/modding/voxel-models.md`: a declaration that *computes*
its key is not seen, so its key is reported unused. The report is advisory and
never a refusal, so a false positive costs one line of output — which is what
makes the limitation acceptable rather than merely disclosed.

---

## Data

### The manifest — source, committed

`content/base/textures.toml`, as above. It is the thing that says which face of
which model becomes which key, and it is reviewable in a diff.

### The built set — derived, gitignored

`content/base/textures/`: one PNG per entry plus `index.txt` (D7). Never
committed; gate stage 7 enforces it.

### The appearance hash — migration

`DeclaredAppearance` gains the six resolved keys, and **the appearance list's
revision byte** goes 1 → 2.

**Corrected 2026-08-18.** This section, and `tasks.md` T09, previously said
"`INPUT_VERSION` goes 1 → 2" as a single number. As written that **breaks
FR-9.1-S4**: `format.rs:249` holds one `INPUT_VERSION` folded as the first field
of *both* `DeclaredBehaviour` and `DeclaredAppearance`, so a shared bump moves
every block's behaviour hash too — which the scenario forbids in as many words.
**The version byte becomes per field list: behaviour stays at revision 1,
appearance goes to 2.** `format.rs:244`'s own doc comment already says "adding a
field to **one of them**", so the single shared constant was conflating two
independent revisions before this spec arrived. The spec is the source of truth
and this document is a plan for satisfying it; where they disagree the scenario
wins.

Consequence, intended and player-visible: **every save written before this spec
reports every block's appearance as changed on next load**, routed through the
existing `--load-changed-blocks` path. No new migration machinery and no new flag
— `format.rs:264` documents the version byte as exactly the mechanism for this
case.

No personal data, no retention rule, no regulatory surface.

---

## Integration — what is touched and what must not break

| File | What connects | What must not break |
|---|---|---|
| `mc-core/src/block/definition.rs` | `texture` → `textures` | Nothing derives `Hash`/`Serialize` on `BlockDefinition`; the save's field list is authored separately at `format.rs:277` and must stay that way |
| `mc-core/src/block/registry.rs:57` | `texture_keys` unions six | It must keep asking the *registry* and never a world (`registry.rs:40-45`) |
| `mc-core/src/content.rs` | `ResolvedBlock.textures`, `TEXTURE_EDGE`; `LayerAssignment` untouched | Appended-never-renumbered; `spent` stays a primary field |
| `mc-world/src/content/luau_declaration.rs` | the table form | `RECOGNISED_FIELDS` order; every read stays raw (`read_field`/`field_names`) |
| `mc-world/src/persistence/format.rs` | `DeclaredAppearance`, `INPUT_VERSION`, `fnv_1a_64` import | **No stored hash other than an appearance may change value** (FR-9.1-S4) |
| `mc-world/src/mesh/facing.rs` | `Facing::face` | The declaration order is the emission order, the neighbour slot order and `Ord` — do not reorder to make the mapping tidier |
| `mc-sim/src/content.rs:164` | `ResolvedBlock` construction | The seam: `replaceable`, `breakable`, `breaks_into` still do not cross |
| `mc-sim/src/world/reload.rs:81` | `drawn_of` compares six keys | The binary marking rule (D3) and the section-count assertions that guard it (`rendering.md:497-510`) |
| `mc-render/src/geometry/mod.rs:186` | `layer_for` takes a resolution | `PLANE_AXES`, `QUAD_INDEX_PATTERN`, the winding derivation, `layer_at` |
| `mc-render/src/hud/held.rs:109` | `held_swatch` takes a resolution | `HeldSwatch`'s three arms stay total; the report still names the block |
| `mc-render/src/gpu/hud.rs:57-92` | `FrameRenderer` holds a `TextureResolution`; `new` gains `TerrainTextures` | `array_texture()` staying lent, so the HUD and terrain sample one texture |
| `mc-render/src/gpu/buffers.rs:238-275` | `mip_level_count`, the write loop, `terrain_sampler` | The layer-range refusal; `Rgba8UnormSrgb` — the sRGB choice is load-bearing and the mip filter now depends on it |
| `mc-render/src/gpu/pipeline.rs:94-103` | nothing | Already `filterable: true` / `SamplerBindingType::Filtering`; **no layout change is needed** and none should be made |
| `mc-render/Cargo.toml:73-76` | `pollster` promoted, comment amended | The `gpu` feature seam — `pollster` is featureless and must not drag `wgpu` into the `--no-default-features` graph |
| `mc-client/src/{startup,content,upload,remesh,app}.rs` | `TextureResolution` replaces `TextureLayers` as the travelling value | `Unuploaded`'s one-way door — `uploaded_to` stays the only route to an owned value |
| `crates/mc-render/CLAUDE.md` | the "Known gap" section is **deleted** | Both sites closed, so the warning goes with them |
| `scripts/sdd-gate.ps1` | three stages, four parameters | The `-Quick` early exit; the "every stage runs" property, now with one stated exception |
| `content/base/models/generators/*.py` | a header on each saying it ran once, produced the tracked model, and is kept as provenance — **not** maintained and not runnable as-is | Nothing. No test grades a comment, and after FR-8.2's retirement no test runs these scripts at all |
| `content/base/models/grass-block.mcvox:57` | citation repaired to `generators/gen_grass.py` and `generators/assemble_grass.py` | The header is part of the document the parser reads — repair the comment, not the schema |

**Test premises that change, listed so none is loosened until green**
(`requirements.md` §5.3): `swatch.rs:35` `TEXEL_COLORS = 2`; `probe.rs:124`
`STRATA` and the ΔE constants; `terrain_offscreen.rs`'s centre-pixel comparison.
`placeholder_test.rs` stays valid — the generator is not deleted and still guards
the fallback.

---

## Phases

Nine, deliberately more and smaller than the work would ordinarily take, because
each boundary is where a fresh test author and a fresh implementer are switched in
and the previous pair's closing report is the whole inheritance. Every phase ends
on a green gate and a tree that runs. The columns sum to **101**.

**The cut lines are chosen against one constraint above all: the golden set is
re-shot exactly once.** Everything that moves a pixel is in P9 and nothing else
is. Everything that *could* move a pixel but does not have to — the mip
arithmetic, the sampler request, the set verdict, per-face resolution against
content that still declares uniform keys — lands earlier, picture-neutral, where a
golden diff would be a defect rather than an expectation.

| # | Name | Scenarios | n | Picture | Why the cut is here |
|---|---|---|---|---|---|
| **P1** | **The stable fold moves to `mc-core`** | — (additional coverage only) | 0 | unchanged | `fnv_1a_64` → `mc_core::hash`, unchanged, with `mc-world` still calling it. No spec scenario, and one additional-coverage test that is only available *now*: every `behaviour_of` and `appearance_of` value the shipped content produces is unchanged, against values an independent FNV in the test computes over the same postcard bytes. Once P2 changes what an appearance *is*, that assertion can no longer be made about both halves. P5 and P6 need the moved function. |
| **P2** | **A block declares six facings, and a save records them** | FR-1.1, FR-1.2, FR-1.4, FR-9.1 | 19 | unchanged | `Face`, `FaceTextures`, `Facing::face`, `TEXTURE_EDGE`, the Luau table form, the refusals, `texture_keys`. **`drawn_of` and `DeclaredAppearance` land here too, and that is forced, not chosen:** `reload.rs:81` and `format.rs:320` both read the field this phase replaces, so a phase that changed the field without them would need a stopgap in each, governed by no scenario, leaving a reload that changes only `north` marking nothing. `drawn_of` needs no new scenario — `rendering.md:497-510`'s section-count assertions already guard it. Goldens stay green: the shipped blocks still declare one string. |
| **P3** | **Resolution at packing, and at the indicator** | FR-1.3, FR-2.1, FR-2.2 | 12 | unchanged | `TextureResolution`, both signatures, the whole plumbing, `FrameRenderer` holding it. **PRO-902 closes at both sites here.** Picture-neutral because all four shipped blocks declare `texture == name`, so every key resolves to the layer it already had. The phase the team lead flagged: FR-2.1-S4 must be authored against the retained-packing seam (D3), not against a reload that re-meshes. |
| ~~**P4**~~ | ~~The models say truthfully where they came from~~ **Retired 2026-08-18 with FR-8.2.** What survived was five documentation edits with no scenarios, no tests and no gate stage — the `grass-block.mcvox:57` citation repair, a provenance header on each of the three generators, and `docs/modding/voxel-models.md`. Too small to be a phase, so they were made directly rather than run as one. The number and T20–T24 are left unused; the stage, the seams and the harness moved to P6. | — | 0 | unchanged | — |
| **P5** | **`voxforge build`** | FR-3.1, FR-3.2, FR-3.3, FR-3.4 | 28 | unchanged | The manifest, `mc_core::art`, the fold, the grouping, every refusal, `.gitignore`. Nothing consumes the set yet, so the client is untouched. Ends with a mod author able to bake a model's faces into a named set with one command. The largest phase; if it needs splitting, the seam is FR-3.1/FR-3.2 (write and cache) before FR-3.3/FR-3.4 (refuse and fold). |
| **P6** | **The gate builds the art and refuses a committed one** | FR-7.1 | 6 | unchanged | Both new gate stages, `-ContentRoot`, `-Manifest`, `-ArtOnly` and the PowerShell-driving harness — all of which were P4's until FR-8.2 was retired and left P6 the first stage that needs them. Must follow P5 (stage 8 runs `build`) and must precede P7 (where a launch starts depending on the set existing). |
| **P7** | **The client judges the set and refuses by name** | FR-5.1, FR-5.2 | 10 | unchanged | `mc-client/src/textures/`, the five-arm verdict, the new `PreparationError` variants, `prepare_scene` calling `built_set`. **No texels reach the array texture yet** — the set is judged and then not used, which is what keeps this phase picture-neutral. Ends with a contributor who has not run the build getting one sentence telling them what to run. |
| **P8** | **The mip chain, as arithmetic** | FR-6.1 | 5 | unchanged | `to_linear`/`to_stored`/`reduced`/`chain`/`levels_for`, all pure, all under normal coverage, **none wired**. `mip_level_count` is still 1 and the sampler is still nearest, and no scenario in this phase claims otherwise — FR-6.2 is deliberately not here, because a `TERRAIN_SAMPLER` constant nothing consults is policy without wiring. |
| **P9** | **Real pixels** | FR-4.1, FR-4.2, FR-4.3, FR-6.2, FR-7.2, FR-8.1 | 21 | **changes once** | `SuppliedTexels` reach `write_layer`; `mip_level_count = MIP_LEVELS`; the sampler is wired through `terrain_sampler`; `grass.luau` declares six facings; the goldens are re-shot at `r1`; the probes and swatch constants are re-derived. Everything here moves a pixel, which is why it is one phase. |

### Notes the phase boundaries depend on

- **P7 makes a bare `cargo nextest run` fail without a built set.** That is
  FR-7.2-S2 working, not a regression. The gate is green because P6 taught it to
  build first. Say so in P7's closing report, or the next pair will "fix" it.
- **FR-7.2 belongs to P9, not P7.** Its discriminating half — *report the set as
  stale **rather than** as a golden-frame mismatch* — cannot be exercised where no
  pixel depends on the set, because there is no golden mismatch to be distinguished
  from. In P7 it would pass vacuously.
- **P9's re-shoot follows `docs/technical/rendering.md` verbatim** — probes, then
  oracle, then HUD prediction, then a mint naming **only** the `terrain_goldens`
  and `hud_goldens` binaries. A bare `MYCRAFT_UPDATE_GOLDENS=1 cargo nextest run`
  reaches `golden_mismatch` and corrupts the set permanently. `SCENE_REVISION` is
  **not** bumped (`rendering.md:980`; `:878` is the precedent); the commit message
  carries the why (`:975`).
- **P9 lands two independent pixel changes.** A golden diff cannot attribute
  between art bytes reaching `write_layer` and the sampler-plus-mip wiring.
  Suggested, and costing no second committed re-shoot: land the art with
  `mip_level_count = 1` and the nearest sampler, take an *uncommitted* reference
  capture, then land the sampler and the mips and mint once. A bad diff then has a
  bisect point.
- **FR-8.1-S4 and FR-8.1-S5 must not come to share a path.** S4 is a golden minted
  from the renderer it verifies. S5 is the derived witness: the grass top face
  judged against a mean computed from the built PNG, decoded by the *client's*
  decoder and never by the draw. If a refactor lets S5 read a value the frame
  produced, the pair collapses into one snapshot.
- **P3's FR-2.1-S4 is the option-(b) witness and the only one.** Under option (a)
  the entire spec passes.

---

## The generator spike

Run before tasks were written, to convert the phase's largest risk into a fact.
**Both shipped models reproduce from their generators, byte for byte.**

**This section stays, and it is why P4 has no tests.** The measurement below is
what made a standing reproduction test unnecessary — a test is worth building when
something can change the answer, and nothing re-runs these one-off scripts. It is
therefore the evidence for FR-8.2's retirement on 2026-08-18, and deleting it would
delete the reasoning for the decision. Because this document is archived and then
pruned, **the fact itself is carried into `docs/modding/voxel-models.md`**, which
is its only durable home.

| | Regenerated | Tracked | |
|---|---|---|---|
| `grass-block.mcvox` | 10691 bytes, `c5544442e70c595d…` | 10691 bytes, `c5544442e70c595d…` | identical |
| `stone-block.mcvox` | 5451 bytes, `3c0345f90ad46acb…` | 5451 bytes, `3c0345f90ad46acb…` | identical |

Method: the two
generators and the assembler were **copied out of the repository** into a scratch
directory, patched there, and run with that directory as the working directory.
The repository was never written to — which matters, because `assemble_grass.py`
writes a tracked model and a failing run would otherwise leave the tree in
whatever state it ended in. The patched copies were discarded; `git status`
showed only this document modified throughout. This was an exploratory spike
under `testing.md`'s exception, and it was performed **once**. Nothing repeats it,
and after FR-8.2's retirement nothing is meant to.

**The equality is carried entirely by the grid, which is the half that matters.**
The header was sourced from the tracked model, so its 5167 bytes are equal by
construction and prove nothing. The remaining **5524 bytes are the assembler's
own output** — sixteen `[[layers]]` blocks — and they match at sha256
`d5e1483c6e0b4c9c363c400de5ace73028b893e837ebf2cf69e3706c457b6919`. The five
hand-authored courses (`y = 10`…`14`), which exist nowhere but in the assembler
and cannot be recovered from the model, were also compared one at a time and each
is equal. So this is a grid test, not a file test that a trivially-equal header
could have flattered.

Two things the spike establishes beyond the byte equality:

- **`grass_head.txt` reconstructs exactly.** The header the assembler needs is
  the tracked model's own text up to its first `[[layers]]` — 5155 bytes — and
  feeding that back produced the tracked file. The second copy the script reads is
  not in the tree, which is one of the three reasons `assemble_grass.py` does not
  run as landed and is documented as not runnable as-is.
- **Both generators choose a salt by search**, terminating on a score threshold
  (`gen_grass.py`, `gen_stone.py` tails). They came out identical here, so the
  search is deterministic on this input — but it is a search, and it is the one
  place a future edit could break reproduction without touching a grid. Nothing
  guards it now, which is correct: nothing re-runs the generators.

## Assumptions

Listed so a reviewer can veto them.

1. ~~**A Python interpreter is on the machine that runs the gate.**~~ **Withdrawn
   2026-08-18 with FR-8.2.** No test, stage or build step in this spec invokes an
   interpreter any more, so there is no assumption left to veto. Python 3.13.5 is
   still on the developer machine and still appears nowhere in the build
   (constraint 13); the difference is that this spec no longer proposed to be the
   first thing to need it.
2. **Nothing pre-baked ships, and this design keeps it that way.** The art is
   *defined* as what `content/base/models/grass-block.mcvox` bakes to;
   `requirements.md` §1.1 records the two invocations and the sha256 of all
   twelve faces so a future reader can re-run and compare with no other artifact.
   The design preserves that structurally rather than by reference: the set is
   derived, gitignored, built from the model by `voxforge build`, folded over its
   sources, and gate stage 7 refuses a committed image. **No part of this
   architecture depends on an artifact nobody can regenerate.** Not re-verified
   here.

   The digest table's permanent successor is **FR-3.1-S4 plus the fold**, not a
   copy of the table in `docs/`. Copying digests into a page that must then be
   updated on every art change makes a second copy that has to be maintained, and
   the copy that stops being maintained is the one a reader trusts.
3. **A copied content root is a complete content root** (constraint 8). If a
   future fixture copies selectively, D6's `NoArtDeclared` is what catches it and
   the launch still completes.
4. **`content/base/textures.toml` is invisible to all three content scanners.**
   The spec verified this for the *models* directory. The manifest is a file at
   the root of the content directory, which none of the three reads either — they
   read `<root>/blocks` and `<root>/hud`, one level, by extension. **The
   implementer must re-check this for the manifest specifically.**
5. **`pop_error_scope` can be blocked on with `pollster` inside
   `SceneBuffers::new`.** If it cannot, D14's error scope is replaced by the pure
   pre-check it rejected and FR-6.2-S5 is authored against that instead. This is
   the one interface in this document not read out of the tree.
6. ~~**`assemble_grass.py` can be made to reproduce the tracked grass model.**~~
   **Withdrawn 2026-08-18.** It stopped being an assumption when the spike
   measured it — both generators reproduce their tracked models byte for byte
   (`## The generator spike` above) — and it stopped being *relevant* when FR-8.2
   was retired. Nothing runs `assemble_grass.py`, so whether it can be made to run
   is no longer a question this spec asks. The measurement is kept because it is
   the evidence for the retirement, and it is carried into
   `docs/modding/voxel-models.md` because this document is archived and pruned.

---

## Risks

| Risk | Blast radius | Verify early |
|---|---|---|
| ~~**`assemble_grass.py` does not run, and may not reproduce once it does.**~~ **Closed 2026-08-18 — it is no longer a risk because it is no longer a property this spec holds.** The spike measured both models reproducing byte for byte (`## The generator spike`), and FR-8.2's retirement means nothing re-runs the generators: the tracked models are the source of truth and the scripts are provenance. That the salt searches in `gen_grass.py` and `gen_stone.py` are unguarded is the accepted consequence, recorded in `tasks.md`'s Trap 11. | None. The art, the digests in §1.1 and P9's re-shoot all come from the tracked models and were never downstream of a generator run. | Nothing to verify. P4 instead makes the scripts *say* they are not runnable as-is, so nobody is misled into treating a run as a check. |
| **The probe and swatch constants cannot be re-derived to pass.** Real art has 3–6 colours with far less separation than three hash-derived ones; `COVERAGE_FLOOR`, `SAME_COLOR = 2.0` and `DIFFERENT_COLOR = 10.0` were chosen against the placeholder. | P9 stalls; the temptation is to loosen a tolerance until green. | **In P8**, before any pixel moves: decode the built PNGs and compute the strata means and pairwise ΔE offline. If two strata land inside `SAME_COLOR` of each other, that is a spec conversation, not a tolerance edit. |
| **`TextureResolution` ripples wider than expected** — ten files across two crates, on the reload path. | P3. | The compiler is the instrument; there is no silent version. But check that `uploaded_to` is still the only route to an owned value afterwards. |
| **`INPUT_VERSION` 1 → 2 changes a hash it should not.** FR-9.1-S4 is a *negative* assertion about hashes that did not move. | Every save in existence. | P1 exists to take the fold's move out of P2 for exactly this reason. In P2, fold a definition through both `behaviour_of` and `appearance_of` and assert the behaviour value against a value derived by hand, not snapshotted. |
| **Testing a PowerShell script is new ground.** `-ArtOnly` could pass its own tests and diverge from a real gate run. | P6, which is where the harness is now born — it was P4's until FR-8.2 was retired. | The structural test on stage ordering ties the selection to the real sequence. If it cannot be written honestly, say so and fall back to structural-only, recording which scenarios are then unwitnessed. |
| **A `voxforge build` that refuses leaves the previous set on disk** (FR-3.3-S10), and the gate must not then test against it (FR-7.1-S6). | Every gate run after a bad manifest edit. | Confirm by hand once in P6: break the manifest, run the gate, check stage 9 is reported as *not run* rather than run and passing. |
| **Anisotropy cannot be revisited without giving up nearest magnification.** | Permanent. | Recorded in the ADR recommended below, so nobody re-opens it as a bug. |

### Vendor-failure blast radius

There is no vendor. `wgpu` is the only volatile external surface, already isolated
to `mc-render/src/gpu/` by a Cargo feature; this spec adds `pollster` beside it and
touches four functions in that subtree. If `image` were replaced,
`mc-client/src/textures/decode.rs` and `tools/voxforge/src/render/mod.rs:394`
change and nothing else does.

---

## What a future change must not break

For the reader who has only `docs/` — the spec folder is archived and pruned.
These belong in `docs/technical/rendering.md`, `architecture.md`,
`world-format.md`, `testing.md` and `docs/modding/` as part of this spec's
definition of done.

1. **A `Quad` carries no resolved texture.** Resolution happens where vertices are
   built, because a retained mesh is re-packed and never re-resolved at mesh time.
   Stamping a key into a `Quad` re-introduces a stale key on the one path built not
   to re-mesh.
2. **A texture-key change marks every section, and `take_remesh_work` drains the
   whole dirty set into one batch.** Those two facts together are what make the
   retained-but-not-re-meshed state unreachable in production today.
   **Bounding the re-mesh batch turns it into a production path** — and a
   whole-world re-mesh measured at 9.1 ms is exactly the thing somebody will one
   day bound. Whoever does must make the retained sections re-pack against the
   serving resolution before that batch is drawn.
3. **A layer index is never renumbered within a session**, and the packed vertex's
   layer field is eight bits.
4. **Mip levels are averaged in linear light.** The array texture is
   `Rgba8UnormSrgb` and decodes on sample. Box-filtering the stored bytes gives 128
   where the correct answer is 188, and every level comes out darker than the one
   above it — plausible-looking, and wrong in the direction nothing notices.
5. **Anisotropy and nearest magnification are mutually exclusive in wgpu**
   (`wgpu-core-30.0.0/src/device/resource.rs:2288-2316`). This was chosen, not
   overlooked.
6. **The set's verdict is a total enumeration returned in `Ok`, never an error and
   never an absence check.** A client that lost the ability to check must redden.
7. **"The build step was not run" and "this key was never authored" are two
   messages and must not be collapsed.** The first refuses the launch by name; the
   second is a silent, documented per-key fallback a mod author hits on their first
   block.
8. **The index's format and the fold's byte sequence are a contract between two
   programs that may not depend on each other.** Both live in `mc-core`. A second
   implementation on either side is the defect this arrangement exists to make
   unspellable.
9. **A `TextureKey` imposes no character set, so anything deriving a path or a
   line-oriented record from one must stay validated** — the safe-file-name rule
   and the control-character refusal are both load-bearing, not tidiness.
10. **The block texture's edge is one constant in `mc-core`**, asserted by the
    renderer at compile time and enforced by the build tool. Three numbers that
    can disagree is how a 32×32 set builds cleanly and refuses at launch.
11. **`content/base/textures/` is derived and is never committed.** The gate stage
    is what keeps the `.gitignore` rule from drifting back. Nothing pre-baked
    ships: the art is what the checked-in model bakes to, and every step between
    the two is re-runnable.
12. **The generators reproduce the models; the models do not reproduce the
    generators.** The assembler carries hand-authored courses — the sod shadow,
    three blades at different depths — that appear in no generator and cannot be
    recovered from the model, because the model is their output. Generator +
    assembler → model is a byte-equality claim; the reverse is not a claim.
    Separately, the assembler reads the *header prose* out of the tracked model,
    which looks circular and is not: prose flows model → assembler so there is
    only one copy of it, while the grid flows assembler → model. Deriving the
    grid from the model would be the circular thing.
13. **The gate builds the art before it tests, on both branches**, and skips the
    test stage when the build refuses — the one stated exception to "every stage
    runs".

### One rule about how all of this is written down

**The durable form of a measurement is the command that reproduces it, not the
number it produced.** A recorded number must be maintained, and a number nobody
maintains becomes a confident lie — this spec has met that at four levels in a
single day: a scenario count restated in prose and wrong twice over, a routing
summary, an arithmetic total, and the digest table this rule was drawn from.

Applied here: the twelve sha256 values in `requirements.md` §1.1 do **not** move
into `docs/`, and neither do the spike's digests above. Their permanent successor
is FR-3.1-S4's byte-identical rebuild plus the index fold — a claim a future
reader can re-run, rather than one they can only compare against and must keep
current. A digest table in a maintained page is a second copy that must be updated
on every art change, and the copy that stops being updated is the one a reader
trusts.

Digests in `requirements.md` are fine for the opposite reason: that file is
archived and pruned, so it is a **dated observation**, not a maintained record.
The distinction is the rule — observations may carry numbers, living
documentation carries the means of reproducing them.

### Records to write

- **ADR-026** moves from "Implementation pending" to implemented, naming this spec
  as its first consumer and pointing at where each of its five items landed.
- **A new ADR is recommended: "Terrain magnifies with nearest and minifies with
  linear; anisotropy is refused."** It is a one-way door with a vendor constraint
  behind it, which is the shape `decisions.md` exists for, and item 5 above needs a
  home that outlives this spec folder.

### Documentation owed (Key Principle 3)

| Audience | Where | What |
|---|---|---|
| Mod author | `docs/modding/blocks-items.md` | `texture`'s two forms, all six facing words and their axes, every refusal in the guide's field order (FR-1.2-S8), and a worked grass declaration that runs |
| Mod author | `docs/modding/voxel-models.md` | `voxforge build`, every manifest field with its bound, the index, every refusal including the file-name and edge rules, the generators, and a worked example from model to drawn block |
| Mod author | `docs/modding/hot-reload.md` | a `texture` edit is now visible; the per-field table gains the per-facing row |
| Player | `docs/user/gameplay.md` | the world is grass, dirt and stone; a save from before this build reports its blocks as changed and why |
| Engine | `docs/technical/rendering.md` | resolution at packing (replacing the "known gap"), the mip chain and linear-light averaging, the sampler and why anisotropy is refused, the re-shoot at `r1` — **and, in the "a reload that changes what is drawn re-meshes the whole world" section, item 2 of the list above verbatim.** Bounding that batch turns FR-2.1-S4 into a production path, and whoever bounds it will do so with every test green. That sentence is the single most important thing this spec has to leave behind, and it must not be what the prune deletes. |
| Engine | `docs/technical/world-format.md` | `INPUT_VERSION` 1→2 and the appearance fold's six keys |
| Engine | `docs/technical/architecture.md` | `fnv_1a_64` and the index contract in `mc-core`, `voxforge build` in the topology, `TEXTURE_EDGE` as a contract constant |
| Engine | `docs/technical/testing.md` | the three new gate stages and the one stated exception to "every stage runs"; the re-derived probe constants |
| Contributor | `README.md` | `cargo run -p voxforge -- build content/base/textures.toml`, one line above `cargo run -p mc-client` (ADR-026) |

---

## Open questions

**None.** Every item this document raised has been ruled on and closed in the
spec:

| Raised | Ruling | Landed as |
|---|---|---|
| **D6** — a root declaring no texture manifest | accepted; telling such a root to run the art build blames the wrong party | FR-5.1-S8, and FR-5.1's lead line widened to two non-refusing verdicts |
| **D9** — a key whose derived image name is unsafe, or which will not round-trip an index | accepted, framed as correctness and reproducibility rather than as a threat model | FR-3.3-S12, FR-3.3-S13 |
| **D5** — a model whose scale × pixels-per-voxel is not the texture edge | accepted; three numbers with nothing connecting them, failing as a launch refusing an image the author never wrote | FR-3.3-S14 |
| **D3** — FR-2.1-S4 described a path production does not take | wording moved, narrowing refused: it is a change to a rule `rendering.md` states outright and belongs to whichever spec takes it | FR-2.1-S4 rewritten at the packer, with the reasoning in Technical Considerations |
| **FR-3.3-S11** — "loadable" read literally needs a Luau VM in an art tool | text scan, advisory, never a refusal; limitation documented where the scan lives | unchanged in the spec; recorded under `## Interfaces` |
| **The count drift** | fixed in a form that does not restate the number | `requirements.md` §7 now carries the counting command instead |
