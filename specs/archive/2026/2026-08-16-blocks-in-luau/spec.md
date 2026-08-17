---
id: SPEC-016
title: Blocks are defined in Luau — swap the loader, not the registry
status: implemented
completed: 2026-08-17
rigor: high
branch: feature/PRO-917-blocks-in-luau
issue: PRO-917
created: 2026-08-16
updated: 2026-08-16
author: spec-PRO-917
amended: 2026-08-16
---

# Specification: Blocks are defined in Luau — swap the loader, not the registry

## Goal

Block definitions move from `content/base/blocks/*.toml` to Luau chunks read
through the scripting host, so that the first thing a mod author ever writes in
this engine's own language is a block they can go and stand on. The registry, the
definition, the fields and the way a bad declaration is refused all stay exactly
as they are — this swaps where definitions come from and nothing else.

## The capability this delivers

**Stakeholder: the mod author.** They write one file,
`content/base/blocks/amber.luau`, run `cargo run -p mc-client`, and place their
own block in the world. Until this spec there was nothing anyone could author in
Luau and the documentation said so in six places; after it, `docs/modding/`
takes a person who has never read this codebase from an empty file to a block in
their hand without a line of Rust.

**Player: nothing they can see changes,** and that is the intended outcome rather
than an omission. The same four blocks, the same terrain, the same block in hand,
the same saves. A player who could tell the difference would mean the swap
changed something it was not supposed to.

## Amendment, 2026-08-16 — content is the simulation's, not the client's

`docs/planning/client-server-split.md` is binding on this spec and settles a
question phase 1 answered by accident. Phase 1 put the Luau loader in `mc-world`
and left `mc-client` calling `registry.apply(...)`, so the client evaluated
content it had sourced off disk itself. Two rules now govern:

- **The agreement test decides where code runs.** *The client never evaluates
  anything any other participant, the server included, must agree with.* Passing
  it makes client evaluation permissible, not obligatory — performance and
  isolation rules bind independently. A content set is the sharpest case there
  is: a layer index rides inside every packed vertex, so one block the server
  does not have shifts every index after it and the whole world is textured
  wrong, silently, with no error anywhere.
- **The declaration rule decides who may change what.** Every content concept has
  an agreed half and a free half. The server owns the agreed half, the client the
  free half.

What that adds here, as FR-7: the simulation is what loads a content root, the
client receives content already resolved, and the client's own sources reach
content through none of the doors that read it. What it does **not** add is the
crate move — `mc-server` becoming the composition root, `mc-client` shedding
`mc-sim`, and the restored dependency-closure guard are a later spec's, because
this spec's exit criterion is "the world renders identically" and the golden
suites are the instrument that decides it. Moving the instrument and the subject
together is the verification-first invariant exactly. See Out of Scope.

**The loader does not move.** It stays in `crates/mc-world/src/content/` where
phase 1 put it, `mc-script` keeps its no-workspace-crate property, and
`mc-client` names no scripting host today and must not begin to.

## User Stories

- As a **mod author**, I want to declare a block in a Luau file so that I can
  add a block to the game in the language the rest of the game layer will be
  written in.
- As a **mod author**, I want a declaration I got wrong to be refused by name —
  the file, the block and the field — so that I can fix it without reading Rust
  or bisecting my content directory.
- As a **mod author**, I want the contract I learned for TOML block declarations
  to still be the contract, so that changing language does not mean relearning
  what a block is.
- As a **server operator**, I want a careless or hostile declaration file to be
  refused rather than to hang or exhaust the process, so that installing an
  untrusted mod cannot cost me the server before a single callback exists.
- As an **engine reader**, I want exactly one path from content to the block
  registry, so that there is no second loader to keep in step.

## Functional Requirements

### FR-1 — A block declared in Luau reaches the running game

- **FR-1.1**: A declaration chunk is evaluated once and the table it returns is
  the block declaration.
  - FR-1.1-S1: WHEN a content root holds `blocks/amber.luau` whose chunk returns
    a table stating `name = "example:amber"`, `texture = "example:quartz"` and
    `solid = true` THE SYSTEM SHALL register a block named `example:amber` whose
    texture key is `example:quartz` and which reports solid.
  - FR-1.1-S2: WHEN a declaration chunk builds its `texture` value by
    concatenating `"example:"` with a string a loop assembled, so that the value
    appears nowhere in the file's text THE SYSTEM SHALL register the value the
    chunk computed, a declaration being code that ran rather than a document
    that was parsed.
  - FR-1.1-S3: IF a declaration chunk returns a function THEN THE SYSTEM SHALL
    refuse it naming the declaration file, and naming no block and no field.
  - FR-1.1-S4: IF a declaration chunk returns nothing at all THEN THE SYSTEM
    SHALL refuse it naming the declaration file, and naming no block and no
    field.
  - FR-1.1-S5: WHEN a source over a content root of three declarations is asked
    for its definitions twice THE SYSTEM SHALL yield the same three definitions
    both times, without panicking.

- **FR-1.2**: Declaration files are the `*.luau` files directly under
  `<root>/blocks/`, and registration order is their file-name sorted order.
  - FR-1.2-S1: WHEN a content root holds `blocks/amber.luau` declaring
    `example:zinc` and `blocks/zinc.luau` declaring `example:amber`, created in
    that order THE SYSTEM SHALL register `example:zinc` before `example:amber`.
  - FR-1.2-S2: WHEN a content root's `blocks/` directory holds `notes.txt` and a
    subdirectory `nested/` containing `hidden.luau`, beside one `amber.luau` THE
    SYSTEM SHALL register only the block `amber.luau` declares.
  - FR-1.2-S3: IF a content root has no `blocks/` directory THEN THE SYSTEM
    SHALL refuse naming `<root>/blocks`, and naming no block.
  - FR-1.2-S4: IF a content root's `blocks/` directory holds a subdirectory named
    `nested.luau` THEN THE SYSTEM SHALL refuse naming that path, rather than
    treating a directory as a declaration because of its name.

- **FR-1.3**: The base game ships its four blocks as Luau declarations, and the
  world they make is the world that was there before.
  - FR-1.3-S1: WHEN the simulation loads `content/base/` THE SYSTEM SHALL
    register exactly `base:dirt`, `base:grass`, `base:stone` and `base:water`, in
    that order, with `base:water` alone non-solid and `base:water` alone
    replaceable.
  - FR-1.3-S2: WHEN the simulation loads `content/base/` THE SYSTEM SHALL put
    `base:dirt` in the player's hand, it being the first solid block in
    registration order.
  - FR-1.3-S3: IF a content root registers no solid block at all THEN THE SYSTEM
    SHALL refuse to start rather than open a window in which nothing can be
    placed.
  - FR-1.3-S4: WHEN a save written against the four TOML declarations is loaded
    against the four Luau declarations THE SYSTEM SHALL load it reporting no
    block as changed.

### FR-2 — The declaration contract is the one already documented

- **FR-2.1**: `name`, `texture` and `solid` are required, they are three
  independent fields, and one of the wrong kind is a refusal rather than a
  fallback.
  - FR-2.1-S1: WHEN a declaration states `solid = false` THE SYSTEM SHALL
    register a block that reports non-solid.
  - FR-2.1-S2: WHEN a declaration states `name = "example:amber"` and
    `texture = "example:quartz"` THE SYSTEM SHALL register a block named
    `example:amber` whose texture key is `example:quartz`.
  - FR-2.1-S3: IF a declaration omits `solid` THEN THE SYSTEM SHALL refuse it
    naming the declaration file, the block as it named itself, and the field
    `solid`.
  - FR-2.1-S4: IF a declaration states `solid = "yes"` THEN THE SYSTEM SHALL
    refuse it naming the field `solid` and the kind of value it holds, rather
    than reading it as true.
  - FR-2.1-S5: IF a declaration omits `texture` THEN THE SYSTEM SHALL refuse it
    naming the declaration file, the block as it named itself, and the field
    `texture`.
  - FR-2.1-S6: IF a declaration omits `name` THEN THE SYSTEM SHALL refuse it
    naming the declaration file and the field `name`, and naming no block.
  - FR-2.1-S7: IF a declaration states `name = 42` THEN THE SYSTEM SHALL refuse
    it naming the field `name`, and naming no block, there being no name to
    quote back.

- **FR-2.2**: `replaceable`, `breakable` and `breaks_into` are optional,
  independent, and keep the defaults they have today.
  - FR-2.2-S1: WHEN a declaration states none of `replaceable`, `breakable` or
    `breaks_into` THE SYSTEM SHALL register a block that is not replaceable, is
    breakable, and leaves the cell empty when it is broken.
  - FR-2.2-S2: WHEN a declaration states `breakable = false` together with
    `breaks_into = "example:ash"` THE SYSTEM SHALL register both, the block
    being unbreakable and its residue simply never reached.
  - FR-2.2-S3: IF a declaration states `replaceable = 1` THEN THE SYSTEM SHALL
    refuse it naming the field `replaceable`, rather than falling back to the
    absent-means-false default.
  - FR-2.2-S4: IF a declaration states `breaks_into = 3` THEN THE SYSTEM SHALL
    refuse it naming the field `breaks_into`, rather than reading it as a
    residue nobody stated.

- **FR-2.3**: Every id is namespaced by the same rule: exactly one `:`, with
  non-empty text on both sides.
  - FR-2.3-S1: IF a declaration names itself `amber` THEN THE SYSTEM SHALL
    refuse it naming the field `name` and the rule the id broke.
  - FR-2.3-S2: IF a declaration states `texture = "example:amber:top"` THEN THE
    SYSTEM SHALL refuse it naming the field `texture`, rather than reading the
    id as `amber:top` inside namespace `example`.
  - FR-2.3-S3: IF a declaration states `breaks_into = "ash"` THEN THE SYSTEM
    SHALL refuse it naming the field `breaks_into` and the rule the id broke.
  - FR-2.3-S4: WHEN a declaration states `breaks_into = "example:ash"` and
    nothing in the content root declares `example:ash` THE SYSTEM SHALL register
    the block, a residue being resolved where a break reads it and not where it
    is declared.

- **FR-2.4**: A field the loader does not recognise is refused, not ignored.
  - FR-2.4-S1: IF a declaration carries `slid = true` beside its three required
    fields THEN THE SYSTEM SHALL refuse it naming the declaration file, the
    block, the field `slid`, and the fields it does recognise.
  - FR-2.4-S2: WHEN a declaration carries all six recognised fields and nothing
    else THE SYSTEM SHALL register it.
  - FR-2.4-S3: IF a declaration carries both `slid = true` and
    `replacable = true` beside its three required fields THEN THE SYSTEM SHALL
    refuse it naming both unrecognised fields in the same order on every run.

### FR-3 — Loading is all-or-nothing, and every refusal locates itself

- **FR-3.1**: A content root registers every declaration it holds or none of
  them.
  - FR-3.1-S1: IF one of three declaration files is refused THEN THE SYSTEM
    SHALL register none of the three and leave the registry holding exactly what
    it held before.
  - FR-3.1-S2: IF a content root's `blocks/` directory holds no declaration at
    all THEN THE SYSTEM SHALL refuse naming that root, rather than registering an
    empty set.
  - FR-3.1-S3: WHEN the declaration file that caused a root to be refused is
    repaired and that same registry is applied to the root again THE SYSTEM
    SHALL register all three of the root's blocks.

- **FR-3.2**: A name declared twice names both places that declared it. *(Its
  positive control is FR-1.2-S1, where two files declaring two distinct names
  both register. A scenario of its own here would re-prove that through the same
  code path, which is why there is not one.)*
  - FR-3.2-S1: IF `blocks/amber.luau` and `blocks/zinc.luau` both declare
    `example:amber` THEN THE SYSTEM SHALL refuse naming `amber.luau` as the first
    and `zinc.luau` as the second, and naming no field.

- **FR-3.3**: A chunk that will not compile or that raises is still located by
  the file a person can open.
  - FR-3.3-S1: WHEN a declaration in `blocks/amber.luau` under content root
    `content/base` fails to compile THE SYSTEM SHALL give its origin as
    `content/base/blocks/amber.luau`, and not as the chunk name the scripting
    host was given.
  - FR-3.3-S2: IF a declaration file is not valid Luau THEN THE SYSTEM SHALL
    refuse naming that file and the line the compiler named, and naming no block.
  - FR-3.3-S3: IF a declaration chunk raises an error of its own before returning
    THEN THE SYSTEM SHALL refuse naming that file and the error the chunk raised.

### FR-4 — A careless or hostile declaration cannot take the loader down

Invariant 3 applies here, before any callback exists.

- **FR-4.1**: Evaluating a declaration is an entry into script and is guarded
  like every other one.
  - FR-4.1-S1: IF a declaration chunk loops without returning THEN THE SYSTEM
    SHALL refuse naming the file and the call-and-loop budget as the limit
    exceeded, rather than waiting on it.
  - FR-4.1-S2: IF a declaration chunk allocates past the memory cap THEN THE
    SYSTEM SHALL refuse naming the file and the memory cap as the limit
    exceeded, and not the call-and-loop budget.
  - FR-4.1-S3: IF a declaration chunk calls `os.time` THEN THE SYSTEM SHALL
    refuse naming the file, the environment a declaration is evaluated in being
    the sandboxed one.
  - FR-4.1-S4: IF a declaration chunk assigns a global THEN THE SYSTEM SHALL
    refuse naming the file, the environment being frozen.

- **FR-4.2**: No metamethod of a declaration's own runs while the loader reads
  it — neither the one that answers a field nor the one that answers which
  fields exist.
  - FR-4.2-S1: WHEN a declaration returns a table carrying a metatable that
    supplies nothing the loader reads THE SYSTEM SHALL register the block from
    the table's own fields.
  - FR-4.2-S2: IF a declaration returns a table stating no `solid` of its own and
    supplying one through an `__index` metatable THEN THE SYSTEM SHALL refuse it
    naming the field `solid`.
  - FR-4.2-S3: WHEN a declaration returns a table whose `__index` metatable
    prints on every access THE SYSTEM SHALL finish the load with the host having
    recorded nothing printed on that chunk's behalf.
  - FR-4.2-S4: IF a declaration holds `slid = true` beside its three required
    fields and carries a metatable whose `__iter` reports only `name`, `texture`
    and `solid` THEN THE SYSTEM SHALL refuse it naming the field `slid`.
  - FR-4.2-S5: WHEN a declaration returns a table carrying a metatable whose
    `__iter` reports a field the table does not hold THE SYSTEM SHALL register
    the block from the table's own fields.

- **FR-4.3**: Every content-supplied quantity has a bound and a named refusal,
  and each bound accepts its own limit. Script output is the one bound that
  truncates rather than refuses — a chunk that printed too much is not a
  malformed declaration — and it is named on the same terms.
  - FR-4.3-S1: IF a content root's `blocks/` directory holds 4 097 declaration
    files, one of which is not valid Luau THEN THE SYSTEM SHALL refuse naming
    that directory and the declaration-count bound, and not naming the invalid
    file.
  - FR-4.3-S2: IF a declaration file of 300 KiB contains a syntax error THEN THE
    SYSTEM SHALL refuse naming that file and the file-size bound, and not naming
    the syntax error.
  - FR-4.3-S3: IF a declaration states a `name` of 257 characters THEN THE SYSTEM
    SHALL refuse naming the field `name` and the declared-text bound.
  - FR-4.3-S4: IF a declaration states a `texture` of 257 characters THEN THE
    SYSTEM SHALL refuse naming the field `texture` and the declared-text bound.
  - FR-4.3-S5: IF a declaration returns a table carrying 65 field names THEN THE
    SYSTEM SHALL refuse naming the declaration file and the field-count bound.
  - FR-4.3-S6: WHEN a content root's `blocks/` directory holds exactly 4 096
    well-formed declaration files THE SYSTEM SHALL register all 4 096 blocks.
  - FR-4.3-S7: WHEN a well-formed declaration file is exactly 256 KiB THE SYSTEM
    SHALL register the block it declares.
  - FR-4.3-S8: WHEN a declaration states a `name` of exactly 256 characters THE
    SYSTEM SHALL register the block under that name.
  - FR-4.3-S9: IF a declaration chunk prints more than the host will retain THEN
    THE SYSTEM SHALL finish the load having kept the earliest output up to the
    retained-output bound and having reported that it stopped keeping the rest,
    a record the host truncated reading differently from one nothing printed to.

### FR-5 — The TOML block loader is retired

- **FR-5.1**: There is exactly one path from content to the block registry.
  - FR-5.1-S1: IF a content root's `blocks/` directory holds `amber.toml` and no
    Luau declaration THEN THE SYSTEM SHALL refuse naming the root as declaring no
    blocks, rather than reading the TOML file.
  - FR-5.1-S2: WHEN a `blocks/` directory holds `amber.luau` declaring
    `example:amber` and `brass.toml` declaring `example:brass` THE SYSTEM SHALL
    register `example:amber` alone and SHALL NOT resolve `example:brass`.
  - FR-5.1-S3: WHEN the scene the golden frames are shot through is prepared from
    `content/base/` THE SYSTEM SHALL register the four blocks the Luau
    declarations state.

### FR-6 — The documentation is true, and is held to it mechanically

- **FR-6.1**: Every refusal the modding pages quote is a refusal MyCraft writes.
  *(The guard exists; its block-declaration fixture becomes a Luau declaration
  and the refusal it produces becomes one MyCraft writes itself rather than the
  TOML parser's caret diagnostic. See Technical Considerations.)*
  - FR-6.1-S1: WHEN the modding pages are scanned against a real run over a Luau
    block declaration carrying an unrecognised field and a HUD declaration
    stating an extent of zero THE SYSTEM SHALL report that every quoted refusal
    is one that run prints.
  - FR-6.1-S2: IF a page quotes a refusal with a field name no run ever writes
    THEN THE SYSTEM SHALL report the mismatch with both the quoted text and the
    printed text.
  - FR-6.1-S3: IF the modding pages quote no refusal at all THEN THE SYSTEM SHALL
    report that none was found, rather than reporting agreement.

- **FR-6.2**: The block declaration the first-block walkthrough shows a mod
  author is a declaration that loads.
  - FR-6.2-S1: WHEN the Luau declaration quoted in the walkthrough is written
    into a content root and loaded THE SYSTEM SHALL register a block under the
    name that quoted declaration states.
  - FR-6.2-S2: IF the walkthrough quotes no Luau declaration at all THEN THE
    SYSTEM SHALL fail rather than report agreement over nothing.

### FR-7 — Content is the simulation's, and the client receives it resolved

Added by the 2026-08-16 amendment. `docs/planning/client-server-split.md` is the
binding reasoning; it is not re-derived here.

- **FR-7.1**: The client's own sources reach content through none of the doors
  that read it. *(The needles are chokepoints and not type names, because
  renaming a source does not rename the door: `registry.apply(` is the only way
  to populate a block registry at all, `HudLayout::load` is the only door into a
  layout, `BlockRegistry::new` is where a registry comes into existence, and
  `content_root` is where a content directory is resolved.)*
  - FR-7.1-S1: WHEN the client's own production sources are scanned for
    `registry.apply(`, `HudLayout::load`, `BlockRegistry::new` and `content_root`
    THE SYSTEM SHALL report that every one of the four is unnamed.
  - FR-7.1-S2: IF a client source names one of those four THEN THE SYSTEM SHALL
    report that file and that spelling, rather than reporting that none was
    named.
  - FR-7.1-S3: IF the scan reads no client source at all THEN THE SYSTEM SHALL
    report that it read nothing, rather than reporting that no door was named.

- **FR-7.2**: The resolved content a client receives carries what it draws and
  predicts with, and none of the rules by which the world is mutated.
  - FR-7.2-S1: WHEN two content roots declare the same blocks alike in every
    field but `replaceable`, `breakable` and `breaks_into` THE SYSTEM SHALL
    resolve one and the same client content from both, the rules by which a world
    is mutated being the simulation's alone.
  - FR-7.2-S2: WHEN two content roots declare the same blocks alike but for one
    block's `texture` and another block's `solid` THE SYSTEM SHALL resolve client
    content that differs in both, those being what the client draws and predicts
    with.

- **FR-7.3**: The client's view of content is built from that resolved value and
  from nothing else.
  - FR-7.3-S1: WHEN a resolved content value stating three blocks, their texture
    keys, their solidity and a layer assignment — a value written down rather
    than read from anywhere — is handed to the client THE SYSTEM SHALL build the
    view its renderer and its mesher read from that value alone, reporting each
    block's layer and its solidity as the value states them, with no registry
    built, no path opened and no scripting host constructed.

- **FR-7.4**: The layer assignment is honoured, not derived. *(What selects an
  entry of that assignment is the block's own **name** today and not its declared
  `texture` — the two agree only because every shipped block declares them
  identical. FR-7.4-S3 pins that rather than leaving it to a comment; closing it
  is PRO-902/PRO-914's. See Technical Considerations.)*
  - FR-7.4-S1: WHEN a resolved content value states a layer assignment that is
    deliberately not the positional order over its sorted keys THE SYSTEM SHALL
    pack each block's corners with the layer that assignment names for it.
  - FR-7.4-S2: IF a resolved content value states a block for which its layer
    assignment names no layer THEN THE SYSTEM SHALL refuse naming that block,
    rather than packing its corners with layer zero.
  - FR-7.4-S3: WHEN a block declares a `texture` that is not its own name, and
    the layer assignment names a layer for that declared texture key, THE SYSTEM
    SHALL refuse to pack that block's corners naming the block — the layer being
    selected by the block's name today — rather than drawing it from whichever
    layer another block's key occupies.

## Technical Considerations

### Where the new source lives, and what it costs the dependency graph

A `LuauFileDefinitionSource` joins `TomlFileHudSource` in
`crates/mc-world/src/content/`, implementing the same
`mc_core::block::source::DefinitionSource`. `mc-world` gains an `mc-script`
dependency, confined to `src/content/` exactly as `toml` and `postcard` are
confined today, and declared with the same kind of note in `Cargo.toml`.

No cycle: `mc-script` depends on `mlua` alone and on nothing in the workspace.
The alternative — putting the source in `mc-script` — would require `mc-script`
to depend on `mc-core` and to learn what a block is, which is precisely what its
opaque `SubjectName`/`ComponentName` design exists to refuse.

`mc-world` keeps its `toml` dependency, because HUD declarations remain TOML.
`crates/mc-world/tests/dependency_graph.rs` therefore continues to hold
unchanged: `mc-core` still resolves without `toml`, `mc-world` still resolves
with it.

### Two new host capabilities, engine-facing

`mc-script` gains a way to enumerate a script table's field names **raw** — no
metatable consulted, nothing of the mod's code run on the host's schedule — on
the same terms as the existing `ScriptHost::read_field`.

It is needed because FR-2.4 cannot otherwise be kept: `deny_unknown_fields` was
TOML's, and a host that can read a named field but cannot ask what fields exist
can never tell a typo from an absence. Without it a misspelled `replacable`
becomes the silently-lost declaration that today's documentation promises it is
not.

**This is not new content-facing surface.** A mod author writes nothing they did
not write before; this is a method on a Rust type no content can reach. The
roadmap's "no new surface" constraint is about the authoring surface, and the
authoring surface here is strictly the six fields that already exist.

**Its hostile metamethod is not `__index`.** `read_field` is already defended
against `__index` and PRO-916 tests it; the metamethod a key enumeration is
exposed to is `__iter` — `pairs` is one of the globals a chunk may reach, so a
declaration can carry one — and `__len`. Enumeration must be raw on those terms
too, which is what FR-4.2-S4 and FR-4.2-S5 hold it to.

**Its output must be ordered by the loader, not by the table.** Lua leaves the
key order of a table's hash part unspecified. A refusal that named whichever
unrecognised field enumeration happened to reach first would be
run-to-run unstable text, and `documented_refusals.rs` compares refusal text
against a page **line for line** — an unstable refusal makes that guard
intermittently red and the page it guards unwritable. FR-2.4-S3 is what holds
the order settled; the same applies to the recognised-field list FR-2.4-S1 asks
a refusal to carry.

**The second capability is the bound on retained script output**, and it is here
for a different reason: not because the loader needs it, but because the loader
is what makes the existing unbounded buffer reachable from content at all. It is
stated with the other bounds, below.

### The refusal text changes, and that is the point

Today a block refusal ends in the TOML parser's five-line caret diagnostic —
which is why `documented_refusals.rs` carries a note that a parser version bump
reddens it. After this change the block half of that guard compares against a
refusal MyCraft writes itself: origin, block, field, cause, in the shape
`DefinitionFault` already renders. The guard's mechanism, its three enumerated
verdicts and its three controls are unchanged; its fixture moves from
`amber.toml` to `amber.luau` and the pages that quote it move with it. **The HUD
half of the guard is untouched** and keeps quoting a TOML refusal, so the
parser-bump note stays true for that half.

### Bounds, and why these numbers

None of these exists today, because TOML's parser and the filesystem supplied
the practical limits. Each is a first cut, sized to be far above any real content
and far below anything that costs the process:

| Bound | Value | Why a bound at all |
|---|---|---|
| declarations per root | 4 096 | a directory listing is a content-controlled allocation |
| declaration file size | 256 KiB | read into memory before evaluation, unbounded until stated |
| declared text length | 256 characters | it bounds what a `BlockDefinition` **retains** — three strings apiece across 4 096 of them — rather than the copy out of the script state, which is already made by the time `read_field` hands back a `ScriptValue::Text` and is separately bounded by the host's memory backstop |
| field names per declaration | 64 | the key enumeration this spec adds copies every key name out of the script state, and a table of a hundred thousand one-character keys is otherwise allocated in full before the refusal that names one of them — which is why the bound is enforced inside the enumeration and not after it |
| script output the host retains | in `HostLimits`, beside the other limits | see below |

Each bound accepts its own limit and refuses the value above it, which is what
FR-4.3-S6 to S8 pin — a bound stated only from the refusing side leaves `>`
and `>=` indistinguishable. The retained-output bound is the exception: it
truncates rather than refuses, so what FR-4.3-S9 pins instead is that the
truncation is visible.

### Why script output needs a bound, and why it is this spec's to add

`ScriptHost::printed` is a `Vec<String>` that grows without limit, and its own
doc comment says so. Until now nothing in production called `evaluate` or
`dispatch`, so that buffer had no route a mod author could reach — it was an
observable for tests. **This spec builds the first one.** Under the shipped
limits a single declaration chunk can make on the order of half a million `print`
calls inside its call-and-loop budget, each handing over a string built within
the per-entry memory cap; the script-side string then becomes garbage while the
host-side copy is retained outside every limit that exists. Across a full content
root that is tens of gigabytes, which is Invariant 3 breached by a file anybody
can write.

Draining between declarations does not answer it, because one chunk alone
exceeds any per-file drain. So the bound goes where the growth is:

- **In `HostLimits`**, beside the call-and-loop budget and the memory cap, so it
  is a number an operator can read and set rather than a constant buried in the
  host.
- **Reaching it stops recording rather than dropping the oldest.** Whoever is
  debugging a load wants the beginning of the story: the first line a chunk
  printed is the one that locates the problem, and the millionth is not.
- **Truncation is reported.** "The mod printed nothing" and "the host stopped
  keeping what the mod printed" are different facts, and a record that cannot
  tell them apart is the absence-that-reads-as-agreement this project keeps
  paying for.

Chunk evaluation itself needs no new bound: the call-and-loop budget and the
memory cap already apply — *provided the loader goes through
`ScriptHost::evaluate`* rather than round the side of it, which is wiring and is
why FR-4.1 asserts through the loader rather than against the host. FR-4.1 is
all-unwanted by nature, so the thing that stops a host configured with an absurdly
small budget passing all four is FR-1.1-S1 — **which must therefore run against
the shipped limits and not against a test-sized host.**

`mc-script/CLAUDE.md` records that one limit masks another: filling a megabyte
costs more interrupt ticks than a small budget allows, so a memory test under a
small budget dies of ticks and reports the wrong limit while passing. FR-4.1-S2
names the memory cap for that reason and not for tidiness.

### Both call sites, not one

`TomlFileDefinitionSource::new` is constructed in two production places —
`crates/mc-client/src/launch.rs:188` (`prepare_launch`) and
`crates/mc-client/src/startup.rs:267` (`prepare_scene`, the path the golden-frame
suites run and therefore the path the roadmap's "the world renders identically"
criterion is decided on). FR-1.3-S1 and FR-1.3-S2 assert through the first;
FR-5.1-S3 exists because the second is a separate entry point onto the same path
and is untested until something asserts through it.

**Both of them move to `mc-sim`.** The simulation is what loads a content root;
the client is handed what came back. The two entry points stay where they are and
keep their names — `prepare_launch` and `prepare_scene` still mesh, pack and
answer with a `PreparedLaunch`/`PreparedScene`, because the golden frames are
shot through them and the exit criterion is decided there. What leaves them is
the construction: the definition source, the HUD source, the registry they were
applied to, and the resolution of the content directory itself.

### The seam, and what it is not

**What moves is the construction, not the loader.** `LuauFileDefinitionSource`
and `TomlFileHudSource` stay in `crates/mc-world/src/content/`; `mc-script` keeps
its no-workspace-crate property; no crate moves and `mc-client` gains no
dependency. The change is which crate says `new`.

**Four chokepoints, and they are chokepoints on purpose.** A guard whose needles
were type names would go green the day somebody renamed `LuauFileDefinitionSource`
— renaming a source does not rename the door. The tree already documents that
`BlockRegistry::apply` is the only way to populate a registry
(`crates/mc-core/src/block/registry.rs`) and that `HudLayout::load` is the only
door into a layout (`crates/mc-core/src/hud/layout.rs`), which is what makes
those two needles total rather than representative. `BlockRegistry::new` catches
a client that builds an empty registry to fill by some other route, and
`content_root` catches a client that resolves the content directory for itself
even if it never reads it.

`crates/mc-client/tests/seam_boundaries.rs` is the shape to follow: a whole-path
exemption comparison rather than a bare file name, doc comments stripped so prose
about a door is not a use of it, sibling `*_test.rs` files skipped, and a
positive control asking whether the same scan reports a fixture that *does*
commit the offence. That file's own doc comment records the trap the exemption
form exists for, and its `files_read > 0` check is what FR-7.1-S3 generalises
into an enumerated verdict: an empty answer and a scan that can no longer look
must not read the same.

**Three call sites, not two.** `crates/mc-client/src/startup.rs:370`
(`empty_hud`) names `HudLayout::load` for a third reason — the HUD of a client
that has not read its content yet — and it moves with the other two. Leaving it
behind would need an exemption, and an exemption on the one door the guard exists
to watch is how a guard stops being one.

**The residue, stated rather than hidden.** `PreparedScene` and `PreparedLaunch`
still hand the client an `Arc<BlockRegistry>` carrying every field, because in
this arrangement the client binary *is* the server and the simulation inside it
holds that registry for the whole run. FR-7.2 and FR-7.3 are about the client's
*own view*: what it draws and meshes from is built from the resolved value and
from nothing else. The registry stops travelling when the composition root moves,
which is the later spec's, and a dependency-closure guard is that spec's exit
criterion rather than something this one can assert — a binary's closure is the
union of everything inside it, so the only arrangement in which such a guard
passes today is the one where the client sources content itself, and a guard
green exactly when the rule is broken is inverted rather than weak.

### The layer assignment is honoured, and why that is in scope while a hash is not

A layer index rides inside every packed vertex. Under derivation — the index
being a key's position in the lexicographically sorted key set, which is what
`startup.rs:335` does today — inserting one block renumbers every index after it,
and the world is textured wrong with no error anywhere. That is not a networking
problem: it is a live defect on hot reload, in one process, today.

So the resolved content states key-to-index pairs and the client honours them.
The falsifier is the one thing that separates this from a rename: hand the client
an assignment that is deliberately *not* the positional one and assert the
rendered indices follow it. A test comparing two copies of the same sort cannot
fail, which is why FR-7.4-S1 is worded around a disagreeing assignment rather
than around agreement. FR-7.4-S2 covers the other half: a key the assignment does
not name must be refused loudly rather than drawn as layer zero, which is the
silent-corruption shape this whole requirement exists to close.

**A content-set identity or hash is the opposite case and is out of scope**, for
a reason recorded there rather than here.

### What a packed vertex actually carries, traced 2026-08-16

The question was whether a layer index is the **only** registry-derived value
riding inside a packed vertex, because a second one would let FR-7.4-S1 pass
while something else still derived from the registry independently.

**It is the only one.** `crates/mc-render/src/geometry/vertex.rs:75–81` — a
`Vertex` is three section-local coordinates, a `Facing` discriminant, a layer
index and a scene section index. Position and facing are geometric. The section
index is written `0` by `build_section_geometry` and filled in when the scene is
assembled, so it indexes the scene's own table and nothing else. That leaves the
layer. FR-7.4-S1 is therefore not undermined by a second value travelling beside
it.

**What the trace found instead is the same hole one level down, and it is
worse.** The layer index is not derived *through the registry* on the consuming
side at all. There are two independent derivations that agree only by
coincidence:

| | Where | Keyed on |
|---|---|---|
| The assignment | `crates/mc-client/src/startup.rs:336` → `TextureLayers::resolve(&registry.texture_keys())` | each block's declared **`texture`** |
| The lookup | `crates/mc-render/src/geometry/mod.rs:171` → `TextureKey::parse(quad.block.as_str())` | the block's **`name`** |

They match only because all four shipped blocks declare `texture` equal to
`name`. `crates/mc-render/CLAUDE.md` records this as a known gap and says "MVP 2
must close it, and will hit it immediately";
`docs/technical/architecture.md:589–590` flags it too; `product/roadmap.md:122`
assigns it to **PRO-902/PRO-914**, Todo. It is not this spec's to close — that
changes `build_section_geometry`'s binding signature, and the roadmap constrains
this spec to identical fields and no new surface.

**There is a second consumer with the same coincidence, which none of those
notes mention.** `mc_render::hud::held::held_swatch`
(`crates/mc-render/src/hud/held.rs:109`) resolves the held-block indicator by
parsing the block's *name* as a texture key. Its own doc comment says so. So the
name-for-texture substitution lives in two places, not one, and a spec closing
the gap must find both.

**What this costs this spec, stated rather than left to be discovered.** A
declaration whose `texture` differs from its `name` **loads and then does not
draw**: the packer answers `GeometryError::UnresolvedTexture` naming the block,
and a failed remesh batch is logged and dropped rather than failing the run
(`docs/technical/architecture.md`, remesh section) — so an author sees their
block simply not appear, and its held indicator draw nothing. That is reachable
from this spec's own stakeholder capability, and FR-1.1-S1 and FR-2.1-S2 use
exactly such a declaration (`name = example:amber`, `texture = example:quartz`)
because they exist to prove the two fields are independent at the *loader*. They
are independent there and not yet independent at the packer. FR-7.4-S3 is what
makes that a test rather than a note, and it goes red the day PRO-902 closes the
gap — which is precisely when somebody should be made to look at it.

### `definitions(&self)` against `evaluate(&mut self)`

`DefinitionSource::definitions` takes `&self`; `ScriptHost::evaluate` takes
`&mut self`. The source therefore needs interior mutability or a host per stream,
and the failure mode of the obvious choice is a re-entrant borrow — which panics,
and `mc-script/CLAUDE.md` invariant 4 forbids a panic on any path content can
reach. FR-1.1-S5 is what holds that: a source asked for its definitions twice
answers twice.

### Why a save written before the swap must still load

`crates/mc-world/src/persistence/format.rs` folds a definition into two hashes —
`DeclaredBehaviour` over `name`, `is_solid`, `replaceable`, `breakable`,
`breaks_into`, and `DeclaredAppearance` over `name` and `texture` — and
**deliberately excludes `origin`**, precisely so that a save does not depend on
the path a definition was read from. That is what makes FR-1.3-S4 both possible
and worth having: it is the one scenario that would catch a field mapped to the
wrong place or a default resolved differently by the new reader, because it
compares against a hash computed before this spec existed. Nothing else in the
spec compares the whole resolved definition against a fixed oracle.

### Registration order and the held block

Sorted file names remain the registration order, which is what keeps
`docs/modding/README.md`'s first-solid-block rule true and keeps a duplicate-name
refusal able to say which file was first. One declaration per chunk is what makes
that expressible; see the requirements record, Decision 4.

## Existing Code to Leverage

| What | Location | Reuse |
|------|----------|-------|
| The port being implemented | `crates/mc-core/src/block/source.rs` | `DefinitionSource`, `DefinitionStream`, `DefinitionFault`, `DefinitionSourceError` — unchanged |
| The registry and its all-or-nothing apply | `crates/mc-core/src/block/registry.rs` | unchanged; duplicate and empty-source refusals come free |
| Field checking and its fault shape | `crates/mc-world/src/content/raw.rs` | the field/default/refusal logic ports across; the `serde` derive does not |
| Directory walking, sort order, origin labelling | `crates/mc-world/src/content/toml_source.rs` | the shape of the source, with the extension and the reader swapped |
| Chunk evaluation, budget, memory cap, sandbox | `crates/mc-script/src/host.rs` | `ScriptHost::evaluate`, `ScriptHost::read_field`, `ScriptFault` |
| The documentation-agreement guard | `crates/mc-client/tests/documented_refusals.rs` | verdicts, controls and normalisation kept; block fixture swapped |
| Content-root fixtures | `crates/mc-client/tests/support/content.rs` | the shipped-copy helpers, retargeted at `.luau` |
| A whole-path source guard with its controls | `crates/mc-client/tests/seam_boundaries.rs` | the `Guard`/`Scan` shape, the whole-path exemption, the doc-comment strip, the `files_read` vacuity check and the positive-control pattern — FR-7.1's guard follows it |
| Positional layer resolution, the thing being replaced | `crates/mc-client/src/startup.rs:335` (`layers_of`) and `mc_render::texture::TextureLayers::resolve` | the one place the key set is chosen; it stops deriving an index and starts honouring one |

## Documentation deliverable

Owed at implementation, for all three audiences, and part of the definition of
done.

**Mod author.** `docs/modding/blocks-items.md` becomes the Luau contract: file
layout, the shape of a declaration, all six fields with type, bound and default,
the namespaced-id rule, all-or-nothing loading, what each refusal looks like and
how to read it, what the three new bounds refuse, and a complete worked example
that runs. `docs/modding/README.md`'s first-block walkthrough is rewritten
end-to-end against `amber.luau` — it is the page a person with no knowledge of
this codebase starts at, and FR-6.2 holds its example to actually loading.

**The worked example must not declare a `texture` that differs from its `name`,
and the pages must say why.** A declaration that does loads and then does not
draw, for the reason traced under "What a packed vertex actually carries" — and
FR-6.2 checks that the example *loads*, not that it draws, so nothing mechanical
would catch it. Key Principle 3 refuses silence, so the page states four facts in
the author's own terms: a block's texture is selected by its name today; a
declaration whose `texture` differs from its `name` will load and then not draw;
declare the two equal for now; independent texture keys are coming. The field is
still documented as independent because it is independent at the loader, which is
what a mod author is being taught here.

**The page names no issue.** Nothing under `docs/` outside `docs/planning/` may
name the issue tracker — a reader of the modding guide cannot open it, the
reference means nothing to them, and it dangles permanently once the issue
closes. It is the same rule that keeps scenario IDs out of code and test names.
State the decision, drop the pointer; which issue closes the gap is recorded in
`tasks.md`, which is archived with this spec.

**Player.** Stated plainly in the modding entry point and in the consolidated
docs: nothing a player can see changes. Same blocks, same terrain, same block in
hand, same saves.

**Engine reader.** `docs/technical/architecture.md` records the second
implementation of `DefinitionSource`, the `mc-world → mc-script` edge and why the
source does not live in `mc-script`, the raw key-enumeration capability and what
it is for, and the three bounds with their rationale. It additionally records the
seam: that the simulation loads content and the client receives it resolved, that
the client's view is built from the resolved value alone, that the layer
assignment is shipped and honoured rather than derived, and which four
chokepoints the source guard watches and why they are chokepoints rather than
type names.

**The passage at `docs/technical/architecture.md` asserting that "the client has
to know what blocks exist, and it learns that by evaluating the same block
declarations the server evaluates" is corrected ahead of the implementation**,
because it currently reads as an as-built record and a future spec author would
follow it. It is a non-sequitur: the client needs *resolved definitions*, never
the evaluator. It is corrected rather than deleted — a reader who arrives with
"should the client evaluate content?" must find the answer and not silence.

**Statements that must be retired, all of them:** the six places listed in
`requirements.md` that say nothing can be authored in Luau
(`docs/INDEX.md:58`, `docs/modding/README.md`, `script-writing.md`,
`script-surface.md`, `script-limits.md`, `script-faults.md`), plus
`docs/modding/blocks-items.md`'s "What MVP 2 changes" section, `README.md`'s
"a mod is a directory of data files" and its `*.toml` routing table, and
`content/CLAUDE.md`'s "MVP 1 today vs. MVP 2" section and its `mycraft.state`
note. Leaving any of them standing while shipping the loader is a defect.

## Out of Scope

Binding. Recorded, not built.

- **`extends`**, in every form the issue describes: flattening at load,
  cross-mod reuse under a declared dependency, cycle checking, the depth cap, and
  the tag it would imply. It is a new content-facing field, and
  `product/roadmap.md` constrains this spec to identical fields and no new
  surface. Its design reasoning is preserved in `requirements.md`, Decision 1,
  for the spec that builds it.
- **Any new block field** — density, emission, swimmable, the
  solid/drawn/occludes/targetable split, per-face texture keys. Those are PRO-904
  and PRO-902/PRO-914, and they are sequenced *after* this swap precisely so that
  they are born in Luau.
- **Hot reload** of declarations (PRO-918). A declaration is read once at load.
- **Per-cell state**, the callback surface, and component attachment (PRO-919).
- **Worldgen in script.**
- **Any `mycraft.*` binding.** A declaration chunk returns a table; it calls
  nothing the engine provides.
- **The HUD loader**, the voxel-model and material formats. They stay TOML and
  `.mcvox`, and `docs/modding/hud.md` and `voxel-models.md` are unchanged by
  this spec except where they route to a page that moved.
- **A mod-id character set.** `blocks-items.md` records that this waits for mod
  ids to become a real identity; declaring blocks in Luau does not make them one.
- **Reading a second content root.** One content root, `content/base/`, still.

Added by the 2026-08-16 amendment, each with the reasoning preserved so that the
spec which builds it inherits the *reason* rather than rediscovering it:

- **Moving the composition root.** `mc-server` becoming the root and the
  singleplayer host, `mc-client` shedding `mc-sim` and becoming a pure client
  library, `mc-sim` gaining the scripting host and `mc-world` dropping it, and
  the restored dependency-closure guard asserting that `mc-client`'s resolved
  closure excludes the scripting host in every dependency kind. **Reason:** this
  spec's exit criterion is "the world renders identically" and the golden suites
  are the instrument that decides it; the roughly forty test binaries under
  `crates/mc-client/tests/` and the published startup path they are shot through
  both move with the root. Changing the instrument and the subject in one spec is
  the verification-first invariant exactly. The restored guard is that spec's
  exit criterion and a better job for it than guarding — and it cannot pass
  before then, because a binary's dependency closure is the union of everything
  inside it and this binary is both halves.
- **Transport and the wire format.** How resolved content reaches a client that
  is not in the same process: message framing, versioning, chunked delivery,
  what a join handshake carries. **Reason:** nothing crosses a process boundary
  in MVP 2, so every choice here would be made against no consumer and pinned by
  no test. It is MVP 4's, and the seam this spec cuts is what it will have to
  serve.
- **The content-addressed cache.** Base content ships with the client and is
  therefore already present; a cache keyed by content hash is what would make
  serving it to yourself safe. **Reason:** it depends on the content-set identity
  below, which is itself out of scope. Recorded because the hazard is specific:
  the safe version is hash-keyed, and a *name*-keyed lookup — or a direct disk
  read added to "optimise" singleplayer — reopens the consensus hole, will
  arrive inside a spec about startup latency, will look entirely sensible in
  review, and nobody will connect it to consensus.
- **Client-side script evaluation of any kind.** No path by which a client
  evaluates content, and no mechanism by which a server ships script to one.
  **Reason:** the agreement test makes client evaluation permissible only for
  what nobody else must agree with, and nothing in MVP 2 qualifies. Server-shipped
  client script additionally carries a consent problem the existing sandbox was
  never built for — that sandbox protects a *server* from a *mod*, and this would
  make it protect a *player* from a *server operator*, a direction nobody has
  examined. The proposal on record is to close that door consent-gated rather
  than welded: if a server ever ships client-side script, the client asks the
  player.
- **A content-set identity or hash.** No digest of the resolved content set, and
  nothing that compares one participant's set against another's. **Reason, and it
  is the whole reason:** with one process nothing can disagree, so nothing could
  falsify such a field — a test that cannot fail reads as evidence and is not
  (`standards/global/testing.md` §2). The field is small whenever it is wanted,
  and the moment it becomes falsifiable is the moment a second participant
  exists. **This is the opposite of the layer assignment**, which is in scope
  precisely because its consumer exists today: the assignment is read by the
  mesher and the packer on every frame this project already draws, and the defect
  it closes is live in one process.

## Dependencies

- The Luau host (PRO-916) is in the tree at `main` = `7ccc060`:
  `ScriptHost::evaluate`, `ScriptHost::read_field`, the fourteen denied globals,
  the frozen environment and its frozen metatable, the call-and-loop budget and
  the memory cap. This spec consumes all of it and adds two things to it: a raw
  key enumeration, and a bound on the script output the host retains.
- Nothing external.

## Assumptions

- A mod author has the checkout and runs `cargo run -p mc-client` from its root,
  as `docs/modding/README.md` describes today.
- One declaration per file stays the shape for the whole of MVP 2. If a later
  spec needs many declarations from one chunk, it adds an indexed read to the
  host; nothing here forecloses that.
- `.luau` is the extension, matching the four `script-*.md` pages and the worked
  example in `script-writing.md`.

## Open Questions

None.

## Clarifications

### Session 2026-08-16

- Q: The issue specifies `extends`; the roadmap constrains this spec to
  "identical fields, no new surface". Which binds? → A: The roadmap. `extends` is
  a new content-facing field and goes to Out of Scope with its design reasoning
  preserved. (Requirements record, Decision 1.)
- Q: The issue requires bounds on "texture dimensions, density, emission level".
  `BlockDefinition` has no numeric field at all. → A: Those fields do not exist
  and arrive with PRO-904/PRO-902. The content-supplied quantities that do exist
  here are the declaration count, the declaration file size and the length of
  declared text; each is bounded in FR-4.3. (Decision 3.)
- Q: The issue names `docs/modding/sandbox.md` as the page stating that nothing
  can be authored in Luau. → A: No such file exists. The statement is real and
  lives in six other places, all listed in the requirements record and all
  retired by this spec.
- Q: Unknown-field refusal was `serde`'s `deny_unknown_fields`; the host offers
  no way to enumerate a table's keys. Drop the promise or extend the host? →
  A: Extend the host, with a raw key enumeration. It is engine-facing Rust and
  not content-facing surface, and dropping the promise would break a documented
  contract and half of an existing guard. (Decision 2.)

### Session 2026-08-16 — amendment

- Q: Phase 1 left `mc-client` calling `registry.apply(...)`, so the client
  evaluates content it sourced off disk. Is that right? → A: No. Content is the
  server's and the client receives it resolved
  (`docs/planning/client-server-split.md`, settled by two independent design
  reviews and approved by the project owner). The simulation constructs the
  definition source and the HUD source; FR-7 is what holds it.
- Q: Does the crate move come with it — `mc-server` as the composition root,
  `mc-client` shedding `mc-sim`, the closure guard restored? → A: No, and it is
  in Out of Scope with the reasoning. This spec's exit criterion is decided by
  the golden suites, which move with the root.
- Q: A guard asserting that `mc-client`'s resolved closure excludes the scripting
  host was deleted on instruction earlier in this spec. Is it restored here? →
  A: No. It cannot pass while one binary hosts both halves, and a guard green
  exactly when the rule is broken is inverted rather than weak. The property is
  carried in the meantime by FR-7.1's source scan, which is the weaker instrument
  and is recorded as such.
- Q: Is a locality field reserved on component declarations? → A: No. Components
  are a later spec's and there is nothing here to attach one to.

## Scenario audit

Audited against `standards/global/scenario-guidelines.md` on 2026-08-16. The
first draft carried 45 scenarios; the audit returned 14 gaps and 9 defects in
existing scenarios. All 14 gap drafts were accepted bar one, and all 9 repairs
were applied. The spec carried **63 scenarios** after the audit.

**FR-4.3-S9 was added afterwards, at the architecture stage, taking the spec to
64. The 2026-08-16 amendment added FR-7's nine, taking it to 73.** Eight are not
audit findings either; they come from `docs/planning/client-server-split.md` and
the reasoning is in the amendment note at the top of this spec. The ninth,
FR-7.4-S3, came from tracing the packing path — see "What a packed vertex
actually carries" above. FR-4.3-S9's own reason follows: the design pass
established that
`ScriptHost::printed` grows without limit and that this spec is what first lets
content reach it, which is a bound the scenario set had nothing to say about
because nobody had looked at that buffer. The requirement and its reasoning are
in Technical Considerations, above.

The three that changed the spec most:

- **Nothing distinguished `texture` from `name`.** Every scenario in the first
  draft gave a block a texture equal to its name, so a loader that read `name`
  into both fields was green throughout — the exact confusion
  `BlockRegistry::texture_keys` warns about in its own doc comment. FR-1.1-S1
  now uses `example:quartz`, and FR-2.1-S2 asserts the distinction directly.
- **The metamethod the new capability is exposed to is `__iter`, not
  `__index`.** `read_field` already defends against `__index`; nothing in the
  first draft touched the metamethod a key enumeration actually meets.
  FR-4.2-S4 and FR-4.2-S5 do.
- **Registration order was asserted with a degenerate fixture.** Three files
  whose file-name order, block-name order and directory-listing order all agreed
  would have stayed green against a loader that dropped the sort entirely.
  FR-1.2-S1 now makes the two orders disagree.

**One draft was rejected.** The audit suggested a structural scenario — "the
system shall hold no production Rust that constructs a TOML block definition
source". Once `TomlFileDefinitionSource` is deleted, nothing can name it and the
scan can never fail, which is the shape `standards/global/testing.md` §2 calls a
test that reads as evidence and is not. Its real risk — a call site left behind —
is behavioural and is covered by FR-5.1-S3.
