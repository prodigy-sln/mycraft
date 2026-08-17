# Requirements record — PRO-917

What was established from the tree rather than assumed, and the decisions
taken where the issue's framing and the repository disagreed.

## Source material

- Linear issue PRO-917, "Blocks are defined in Luau: swap the loader, not the
  registry", project *MyCraft MVP 2: Scriptable Content*.
- `product/roadmap.md`, MVP 2 section — the binding scope constraint.
- The tree itself, at `main` = `7ccc060`.
- `docs/planning/client-server-split.md`, dated 2026-08-16 — binding on the
  amendment below (Decision 5), and the source of FR-7.

## Ground truth, verified against the tree

### The trait the engine consumes

`mc_core::block::source::DefinitionSource` (`crates/mc-core/src/block/source.rs`):
`origin() -> DefinitionOrigin` and `definitions() -> DefinitionStream<'_>`, the
stream being `Box<dyn Iterator<Item = Result<BlockDefinition, DefinitionSourceError>>>`.
Its module doc already says a swap is "meant to be a new type, not a redesign".

`DefinitionOrigin` (`crates/mc-core/src/block/definition.rs`) is an opaque
`String` label; its doc comment states that a script chunk name is exactly as
expressible as a file path. **Verified — the issue's claim about this is
accurate.**

`BlockRegistry::apply` (`crates/mc-core/src/block/registry.rs`) is the only door
into the registry, takes a `&dyn DefinitionSource`, and is all-or-nothing:
everything fallible happens in a staging pass, and the commit returns nothing.
It refuses a duplicate name naming both origins, and refuses a source that
yielded nothing.

### What a block definition is today

`BlockDefinition` carries exactly: `name: BlockName`, `texture: TextureKey`,
`is_solid: bool`, `replaceable: bool`, `breakable: bool`,
`breaks_into: Option<BlockName>`, `origin: DefinitionOrigin`.

**There is not one numeric field.** No density, no emission level, no texture
dimensions.

### What `content/base/blocks/*.toml` contains

Four files, and between them they use four of the six fields:

| File | `name` | `texture` | `solid` | `replaceable` | `breakable` | `breaks_into` |
|---|---|---|---|---|---|---|
| `dirt.toml` | `base:dirt` | `base:dirt` | `true` | — | — | — |
| `grass.toml` | `base:grass` | `base:grass` | `true` | — | — | — |
| `stone.toml` | `base:stone` | `base:stone` | `true` | — | — | — |
| `water.toml` | `base:water` | `base:water` | `false` | `true` | — | — |

No shipped block declares `breakable` or `breaks_into`.

### Every field with its bound, as the loader enforces it today

Read from `crates/mc-world/src/content/raw.rs`:

| Field | Required | Type | Absent means | Checked as |
|---|---|---|---|---|
| `name` | yes | string | refusal | namespaced id |
| `texture` | yes | string | refusal | namespaced id |
| `solid` | yes | boolean | refusal | — |
| `replaceable` | no | boolean | `false` | — |
| `breakable` | no | boolean | `true` | — |
| `breaks_into` | no | string | cell empties | namespaced id, syntax only |

Unknown fields are refused outright (`#[serde(deny_unknown_fields)]`). A field
present but of the wrong kind is a refusal, never a silent fallback to the
default. The namespaced-id rule is exactly one `:` with non-empty text on both
sides.

**No field carries a length, size or count bound of any kind today**, because
TOML's own parser and the filesystem supplied the practical ones.

### How a malformed definition is refused today

`DefinitionFault { origin, block: Option<String>, field: Option<String>, cause }`,
rendered as `<origin>, block \`<name>\`, field \`<field>\`: <cause>`. The block
name is read out of the file *before* anything is checked, so a refusal can still
say which block it is about. `mc-client` wraps it under "the shipped content
could not be read" and writes it to standard error prefixed `mycraft: `.

`crates/mc-client/tests/documented_refusals.rs` compares every fenced block under
`docs/modding/` whose first line begins `mycraft: ` against a refusal produced by
a **real run** over a fixture content root. Its block-declaration fixture is
`amber.toml` carrying `slid = true`, and the refusal it produces is the TOML
parser's five-line caret diagnostic. It reports one of three enumerated verdicts
and carries a positive control for each. **This guard's block half cannot survive
the swap unchanged** — its fixture becomes a Luau declaration and the refusal it
produces becomes one MyCraft writes itself. The guard *mechanism* survives; the
fixture and the pages it compares against move together.

### What the scripting host offers a loader

`crates/mc-script`, public surface (`lib.rs`, `host.rs`):

- `ScriptHost::evaluate(name, source) -> Result<ScriptValue, ScriptFault>` —
  evaluates a chunk in its own frozen environment, under the call-and-loop
  budget. A chunk that never returns is aborted, not waited on.
- `ScriptHost::read_field(&ScriptTable, field: &str) -> Option<ScriptValue>` —
  reads **raw**, so a hostile `__index` neither runs on the host's schedule nor
  observes which fields were read.
- `ScriptValue` — `Nil`, `Boolean`, `Integer`, `Number`, `Text`, `Table`,
  `Function`, `Opaque`.
- Fourteen denied globals removed before the sandbox is closed; the environment,
  its metatable and the table read through are all frozen.

`mc-script` depends on `mlua` alone — nothing in the workspace. It does not know
what a block is, and its `SubjectName`/`ComponentName` are deliberately opaque.

**The gap:** there is no way to enumerate a script table's keys, read an element
by integer index, or ask its length. `read_field` takes a `&str` and nothing
else. This is the one thing the loader needs that the host does not offer — see
Decision 2.

### The guards that must survive

- `crates/mc-world/tests/no_hardcoded_block_names.rs` — scans `.rs` files only,
  under `crates/` and `tools/`, per root, with a positive control for each of its
  two lists and a behaviourally-pinned single exemption
  (`crates/mc-sim/src/replay/world.rs`). **Moving content from `.toml` to `.luau`
  does not touch it.** It goes on watching `base:stone`, `base:dirt`,
  `base:grass`, `base:water` and the retired `base:air`.
- `crates/mc-world/tests/dependency_graph.rs` — asserts `mc-core` does not
  resolve to `toml` and `mc-world` does. **HUD declarations stay TOML**
  (`content/base/hud/*.toml`, `hud_toml_source.rs`), so `mc-world` keeps its
  `toml` edge and this test is unaffected.
- `crates/mc-testkit/tests/workspace_layering.rs` — about the `tools/` boundary
  only; a new `mc-world → mc-script` edge does not concern it, though its
  `INSPECTED` table is a per-crate roster that a reviewer should re-read.

### What the modding documentation promises today, and must stop promising

Six places state that nothing can be authored in Luau:

| Where | What it says |
|---|---|
| `docs/INDEX.md:58` | routes `script-writing.md` as "the page that states once that nothing can be authored in Luau yet" |
| `docs/modding/README.md:198–203` | "Nothing is authored in Luau today… There is no `mycraft.*` binding of any kind" |
| `docs/modding/script-writing.md:11–33` | a whole section headed "Nothing can be authored in Luau today" |
| `docs/modding/script-surface.md:7` | same, by cross-reference |
| `docs/modding/script-limits.md:7` | same |
| `docs/modding/script-faults.md:8` | same |

Beyond those six, the following are invalidated by this change and are the
spec's to repair:

- `docs/modding/README.md` — "A mod is a directory of data files"; the
  "What you can define today" table row pointing blocks at `*.toml`; the entire
  "Your first block, start to finish" walkthrough including its quoted refusal;
  the held-block explanation naming `amber.toml` and `dirt.toml`.
- `docs/modding/blocks-items.md` — the file-layout section, the TOML field
  block, the quoted refusal, the base-game table naming `.toml` files, and the
  closing "What MVP 2 changes" section, which is about to become the present
  tense.
- `content/CLAUDE.md` — "The Luau host is built; authoring in Luau is not", the
  whole "MVP 1 today vs. MVP 2" section, and the `mycraft.state(...)` note
  claiming nothing is authored in Luau.

## Where the issue did not survive contact with the tree

1. **`docs/modding/sandbox.md` does not exist.** The issue names it as the page
   stating that nothing can be authored in Luau. The eight pages under
   `docs/modding/` are `README.md`, `blocks-items.md`, `hud.md`,
   `script-faults.md`, `script-limits.md`, `script-surface.md`,
   `script-writing.md`, `voxel-models.md`. The statement the issue means is real
   and lives in the six places tabled above — the obligation stands, the filename
   does not.
2. **"Texture dimensions, density, emission level" are not block fields.** A
   `texture` is an opaque namespaced *key* and never a path or a size; density
   and emission arrive with PRO-904. There is no numeric field on
   `BlockDefinition` to bound. The content-supplied quantities this spec can
   actually bound are the declaration count, the length of declared text, and the
   size of a declaration file — see Decision 3.
3. **`extends` is out of scope.** See Decision 1.
4. **The roadmap's MVP 2 table lists PRO-916 as "Todo"** while the host is in the
   tree and its four author-facing pages are published. A stale status line, not
   this spec's to fix, recorded so nobody plans around it.

## Decisions taken

### Decision 1 — `extends` is not built here

The issue devotes a section to `extends`: flattening, cross-mod reuse, cycle
checks, depth caps, and an implied tag.

`product/roadmap.md` constrains this spec in terms that leave no room for it:

> It is contained by PRO-917 being a *pure* swap — **identical fields, no new
> surface**, exit criterion "the world renders identically".

`extends` is a new content-facing field. It also depends on the tags issue, which
is not in this spec's dependency set, and its own cycle-checking and depth cap
are hardening for a mechanism that would not otherwise exist here. Building it
would make the loader swap something other than a swap, and every field of it
would be born in a loader that no later spec has agreed to.

**Recorded in Out of Scope, not dropped.** The issue's design reasoning for it —
flatten at load rather than inherit through a metatable, because a hostile
`__index` on every field access is a denial of service — is sound and is
preserved here for whichever spec builds it.

### Decision 2 — the host gains raw key enumeration, which is not content-facing

A block declaration must refuse a field nobody recognises. That is a promise
`docs/modding/blocks-items.md` makes today in as many words, it is what stops a
misspelled `replacable` shipping as a silently-lost declaration, and it is one
half of what `documented_refusals.rs` guards.

TOML gave it for free through `deny_unknown_fields`. Luau gives nothing: the host
can read a named field and cannot ask what fields exist. Without key enumeration
the promise cannot be kept and a mod author's typo becomes exactly the debugging
trap the current documentation says it is not.

So `mc-script` gains a way to enumerate a script table's field names **raw**, on
the same terms as `read_field`: no metatable consulted, nothing of the mod's run
on the host's schedule.

**This is engine-facing Rust, not new content-facing surface.** A mod author
writes nothing they did not write before and reads nothing new — the constraint
the roadmap sets is about the authoring surface, and this is a method on a Rust
type no content can reach. `crates/mc-script/CLAUDE.md`'s "breadth of capability,
narrowness of commitment" exemption is not even needed: this has a concrete
consumer in the same change.

### Decision 3 — what actually needs a bound

Invariant 3 applies before any callback exists, and the issue is right about that
even where its list of numbers is wrong. Against this field set the
content-supplied quantities are:

- **the number of declarations** a content root offers — a directory of a million
  files is a content-controlled allocation;
- **the length of declared text** — `name`, `texture` and `breaks_into` are
  strings a chunk chooses, copied out of the script state into Rust, where the
  host's memory cap no longer covers them;
- **the size of a declaration file** — read into memory before it is evaluated,
  and unbounded until something says otherwise.

Chunk evaluation itself is already budgeted and memory-capped by the host, so an
infinite loop or an allocation bomb in a declaration file is contained by
machinery that exists — but only if the loader goes through `ScriptHost::evaluate`
rather than round the side of it, which is wiring and therefore gets its own
scenario.

### Decision 4 — one declaration per chunk, and the chunk returns it

The host's established contract is that a chunk is evaluated once and the value
it returns is kept (`docs/modding/script-writing.md`). A declaration file returns
one table. That keeps the file-name sort order that registration order and the
held-block rule both depend on, keeps `DefinitionOrigin` naming one file and one
block, and needs no indexed reads or length queries on the host.

The alternative — one chunk returning a list — buys batching nobody has asked for
and costs the ordering contract, a second host capability, and a refusal that can
no longer name a file.

### Decision 5 — the simulation loads content; the client receives it resolved *(amendment, 2026-08-16)*

Phase 1 put the loader in `mc-world` and left `mc-client` calling
`registry.apply(...)`, so the client evaluated content it had sourced off disk
itself. `docs/planning/client-server-split.md` — settled by two independent
design reviews and approved by the project owner — rules that wrong, and it is
binding here rather than re-derived. Two rules govern: **the agreement test**
decides where code runs (the client never evaluates anything any other
participant, the server included, must agree with), and **the declaration rule**
decides who may change what (every content concept has an agreed half and a free
half).

**What the tree actually holds, read on 2026-08-16 at `9d42d3d`** — these are
observations, not inferences:

| Door | Where |
|---|---|
| `BlockRegistry::new` + `registry.apply(` | `crates/mc-client/src/launch.rs:187–188`, `crates/mc-client/src/startup.rs:266–267` |
| `HudLayout::load` | `crates/mc-client/src/launch.rs:192`, `crates/mc-client/src/startup.rs:272`, and **`startup.rs:370` (`empty_hud`)** |
| `content_root` | defined `crates/mc-client/src/startup.rs:188`, called `crates/mc-client/src/main.rs:50` |

`crates/mc-client/src/startup.rs:335` (`layers_of`) resolves a texture layer as a
key's **position in the lexicographically sorted key set**, and both preparation
paths call it. That index rides inside every packed vertex, which is why an
inserted block renumbers every index after it.

**What was decided, and what was refused.** The construction moves to `mc-sim`;
the loader stays where phase 1 put it; no crate moves; `mc-script` keeps its
no-workspace-crate property and `mc-client` names no scripting host. Refused
here and recorded in Out of Scope with their reasoning: moving the composition
root and restoring the dependency-closure guard, transport and the wire format,
the content-addressed cache, client-side script evaluation of any kind, and a
content-set identity or hash. **No locality field is reserved on component
declarations** — components are a later spec's and there is nothing here to
attach one to.

**A guard was deleted earlier in this spec on reasoning that has been
discarded.** It asserted that `mc-client`'s resolved closure excludes the
scripting host, in every dependency kind, with positive controls. It is **not**
restored here: it cannot pass while one binary hosts both halves, since a
binary's closure is the union of everything inside it, and a guard green exactly
when the rule is broken is inverted rather than weak. It becomes the exit
criterion of the spec that moves the composition root. The property is carried
meanwhile by FR-7.1's source scan over `crates/mc-client/src/`, which is the
weaker instrument and is recorded as such rather than presented as equivalent.

## The stakeholder capability

**Stakeholder:** the mod author.

**What they can now do:** write `content/base/blocks/amber.luau`, start the
client, and place their own block in the world — the first thing anyone has ever
authored in Luau for this engine, reachable from `docs/modding/README.md` without
reading a line of Rust.

**Player:** nothing they can see changes. The same four blocks, the same terrain,
the same held block. Stated in the spec rather than left unaddressed.

## Open questions

None. The two places the issue and the roadmap disagreed are resolved above
against the roadmap, which `product/roadmap.md` and the team brief both make
binding.
