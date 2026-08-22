# Chunk Storage and the Block Palette

How a chunk's voxels are held in memory, why a runtime block id may never
leak into a saved or transmitted form, what a section's storable identity
actually is, and how that identity is turned into the bytes of a save file
on disk. The wire format (network replication) is still out of scope for
this document — `mc-proto` is a stub, and that work belongs to whichever
spec first gives it a dependency graph.

## A cell holds a block, or nothing

There is no block meaning "empty". A voxel cell either holds one registered
block or holds nothing at all, and nothing names nothing — the engine
carries no content-declared entity whose only job is to stand for absence.

What a cell holds is a named two-variant type, `mc_world::section::Contents`:

```rust
pub enum Contents<N = BlockName> {
    Empty,       // the cell holds nothing: not a block, not a name
    Holds(N),    // the cell holds this block
}
```

It is generic over the name the way `Option` is generic over its payload:
storage holds `Contents<BlockName>`, every accessor hands out
`Contents<&BlockName>`, and `as_ref` / `cloned` are the one step between
them — one type in two forms rather than two types that can drift. The
default type parameter does not participate in inference: a bare `Contents`
means the owned form in a *type* position only, and an unconstrained
`Contents::Empty` in an expression position is `type annotations needed`.

**Emptiness is never spelled as a bare `Option`, and that is the whole
point of the type.** Every `Option` on this read path already means
something else, and each of those meanings is a different question:

| Accessor | Returns | What the outer wrapper means |
|---|---|---|
| `Palette::contents_at(position)` | `Option<Contents<&BlockName>>` | this palette position exists |
| `Section::block_at(pos)` | `Result<Contents<&BlockName>, SectionError>` | the position is in bounds and the section is not corrupt |
| `ChunkColumn::block_at(pos)` | `Result<Contents<&BlockName>, SectionError>` | as above, plus below the column top |
| `VoxelWorld::block_at(at)` | `Result<Contents<&BlockName>, WorldError>` | the position is inside the world |
| `Section::palette()` | `impl ExactSizeIterator<Item = Contents<&BlockName>>` | — |

The rule is one sentence: **the outer wrapper keeps the meaning it already
had, and `Contents` is what it wraps.** Folding emptiness into that outer
wrapper instead would make "there is no such cell" and "this cell holds
nothing" the same answer, which is how a corrupt section, or a position
past the edge of the world, gets read as ordinary empty space. Reading a
cell is therefore spelled arm by arm — `None`, `Some(Contents::Empty)` and
`Some(Contents::Holds(name))` are three arms and never two — at every call
site where both wrappers survive.

`Section::block_at` is the one place the split is made, and it gains no
emptiness error to make it with: a position no section has is
`Err(SectionError::OutOfBounds { axis, .. })` naming the axis, and a cell
holding nothing is `Ok(Contents::Empty)`. No path may produce one where the
other belongs.

**`Palette::name_at` was renamed to `contents_at`; `Section::block_at` was
not.** `name_at` promised a name and `Contents::Empty` has none — a direct
contradiction. `block_at` asks which block is at a position, and "nothing"
is a legitimate answer to that question rather than a contradiction of it.

## Block identity: names, not runtime ids

A registered block gets two identities that must never be confused:

- A **runtime id** (`BlockId`, wraps `u32`) — dense, assigned in
  registration order, valid only for the `BlockRegistry` that assigned it.
  Runtime ids are **registry-local and freely reassigned** whenever the
  definition set changes (a mod is added, removed, or reordered). They are
  cheap, and nothing about them survives a change of registration order.
- A **namespaced name** (`BlockName`, e.g. `base:stone`) — stable across
  registries and across time. It means the same block regardless of when or
  in what order it was registered.

A raw palette index or a runtime id must **never** become an on-disk or
on-wire identity. A section's storable identity is:

```rust
pub struct SectionData {
    /// What the section holds, in the order its own palette holds it.
    pub palette: Vec<Contents>,
    /// One palette position per voxel, x fastest, then y, then z.
    pub indices: Vec<PaletteIndex>,
}
```

a palette of `Contents` — each entry either a namespaced name or nothing —
in the section's own palette order, plus one `PaletteIndex` per voxel
naming that voxel's position into it. `export` does not normalize or sort
the palette, and does not implicitly compact — the exported order is
exactly the section's own, vacant entries included. A caller wanting the
minimal form compacts first, which is why compaction is a public operation
rather than something export does on a caller's behalf.

**One list, not a list of names beside a note of which position is the
empty one.** Emptiness *is* a position in the palette, so export is a copy
and import is a copy. The two alternatives were considered and rejected for
reasons that outlive this shape: `palette: Vec<BlockName>` plus
`empty: Option<PaletteIndex>` is a second source of truth — the index can
be out of range, can duplicate, and can disagree with the palette's length,
and every reader consults two fields to answer one question;
`palette: Vec<Option<BlockName>>` is the bare `Option` this crate bans, at
the one place a stored format would make it permanent.

**At most one entry is `Contents::Empty` when a section produced the
description.** A description carrying two is nonetheless accepted, exactly
as one naming a block twice already is — both deduplicate downstream by
what they hold, and a refusal here would be a rule the write path cannot
produce and no reader needs.

`SectionData` carries no `serde` derive today. The shape is chosen so that
adding one is a derive rather than a redesign; it makes no claim about what
any particular encoding will emit.

This is what makes a section survive a change of registration order
**structurally**, not by a translation pass: the section stores no
registry-local data at all. `Section::block_at` takes no registry argument
— reading a voxel "against the wrong registry" is not an operation the
type can be asked to perform. Name-taking mutators (`filled`, `set_block`)
take a registry only as a **validator**: it answers "is this name
registered?" and never translates a name into anything, so handing a
section a foreign registry can only cause a spurious rejection of a write
that should have been allowed — it can never make the section store a
different block than the one named. `Section::set_block_by_id` is the one
operation that does translate a runtime id into a name, and it exists
because writes carrying a `BlockId` (rather than a `BlockName`) need
somewhere to go; the translation happens at that single call and nothing
downstream of it ever sees the id again.

A registry hot swap (MVP 2) is therefore a no-op for already-loaded world
data, and no migration pass is needed: nothing in a loaded section depends
on the registration order that produced it.

## Emptiness needs no registry, at any level

Building something empty takes no registry and has no failure path, all the
way up the containment chain:

```rust
Section::empty()                    -> Section        // infallible
ChunkColumn::empty(coordinate)      -> ChunkColumn    // infallible
VoxelWorld::empty(footprint_columns) -> VoxelWorld    // infallible
```

There is no block for a registry to know about, so there is nothing an
unknown-block failure could be about. Building an empty world cannot fail
for a reason that has nothing to do with emptiness — which is the property
this shape exists to deliver, and the reason none of these signatures
carries a `Result` or a `&BlockRegistry`.

Emptying is the same story: `Section::empty_at`, `ChunkColumn::empty_at`
and `VoxelWorld::empty_at` take no registry, and their only refusal is a
position the container does not have. That refusal is the *same* bounds
check the name-taking mutator beside each of them makes, made in the same
place, so a bounds check lost there is lost for both rather than for one.

Solidity follows: `Section::is_solid_at` answers `false` for
`Contents::Empty` **before** the registry is reached. Widening that arm to
cover every cell would answer "not solid" for a block the registry has
never heard of; narrowing it to consult the registry first would make an
empty cell need a block registered in order to mean nothing. A cell holding
a block still resolves through the registry and still fails if that block
is not registered.

Inside a section there is exactly one write path. Writing a block and
emptying a cell differ only in the `Contents` they resolve, so the
reference counting, the palette growth and the index widening are stated
once — an empty cell is not a special case of any of them.

## The palette-length bound, corrected

**A section's palette can hold more entries than the section has voxels.**
The 4096-voxel figure bounds the palette entries a section's voxels
*reference* at any one time; it does not bound palette *length*, because a
palette entry that no voxel references any more is retained until the
section is **compacted**. Overwriting a single voxel repeatedly with
different blocks lengthens the palette every time, without ever changing
what the section actually holds — a section can accumulate an arbitrarily
long palette of vacated entries between compactions.

What bounds palette length is compaction: a compacted section's palette
holds only entries at least one voxel still references, and so has at most
4096 of them (one per voxel, in the worst case of total variety). Between
compactions, the only bound on palette length is how many distinct blocks
the registry holds and how many times the section has been written to
without compacting — which is to say, no practical bound at all.

**`PaletteIndex` is a `u16` newtype, and `u16` is sufficient only for a
compacted section.** A persisted or wire form that reuses this width must
size itself against the *compacted* bound (≤4096, safely inside `u16`) and
treat the uncompacted case as unbounded. This is the single highest-risk
fact in this document to get wrong when building persistence: sizing a
persisted palette-position field against "4096 voxels" rather than against
"a compacted section" will overflow silently on a world that was never
compacted before being saved.

There is deliberately no overflow error path for the palette growing past
what an index tier can address — reaching one would need tens of thousands
of registered blocks and that many writes into a single section between
compactions. The failure mode, if it were ever reached, is a **refused
write**: a palette position past the widest tier (16 bits, 65536 entries)
is rejected by the packed-index writer, never truncated. A rejected write
is always the outcome — never a silently different block.

**A repeatedly-edited section is bounded by the names it sees, not by the edit count.**
`Palette::replace` finds an existing entry — including one whose refcount has
fallen to zero — before appending a new one, so a palette grows only on the
*first* write of a name it does not already hold. After *N* edits touching *K*
distinct names, a section's palette length is bounded by `initial + K`,
independent of *N*. A single voxel toggled back and forth between two blocks
ten thousand times does not grow the palette past two entries beyond its
starting length; ten thousand edits scattered across a handful of block
types bound the palette by that handful, not by ten thousand. This is what
keeps a section that a player edits over and over from growing an index tier
under the weight of repetition alone — the index width still only widens when
a genuinely new name is introduced, never merely because edits accumulated.

## Section format

A section is `SECTION_SIZE`³ = 4096 voxels (`SECTION_SIZE = 16`, and
`VOXELS_PER_SECTION` is derived from it, never written as a separate
literal). Each section holds:

- A **palette**: an insertion-ordered list of `(Contents, refcount)`
  entries — each entry either a block's name or nothing. `refcount` is the
  number of voxels currently holding that entry; an entry with a zero
  refcount is a vacated entry, kept until compaction. Emptiness is an entry
  here and not a state beside the palette, which is what leaves the packed
  indices, the index widths, the reference counting and compaction with
  nothing at all to know about it.
- A **packed index buffer**: one palette position per voxel, bit-packed
  into a `Vec<u8>` at the narrowest width the palette's length currently
  requires.

Index width is chosen from six tiers, ordered narrowest first:

| Tier | Bits/voxel | Addresses | Storage for a whole section |
|------|-----------:|----------:|-----------------------------:|
| W0   | 0  | 1 entry       | 0 bytes (no buffer at all) |
| W1   | 1  | 2 entries     | 512 bytes |
| W2   | 2  | 4 entries     | 1024 bytes |
| W4   | 4  | 16 entries    | 2048 bytes |
| W8   | 8  | 256 entries   | 4096 bytes |
| W16  | 16 | 65536 entries | 8192 bytes |

A section holding exactly one thing (the common case — a chunk of solid
stone, or an empty one) spends nothing at all on its index buffer: the
homogeneous, 0-bit tier owns no buffer. Every tier either divides 8 or is a
multiple of 8, so **a packed index never straddles a byte boundary** — the
implementation has no general straddle-handling code because the case
cannot occur.

**An empty section and a one-block section are stored identically.** A
fresh section is palette `[Contents::Empty]` at position 0, every voxel
index 0, index width `W0` and therefore zero bytes of index storage —
byte-for-byte the shape `Section::filled` produces with a name at position
0 instead. Nothing about the storage distinguishes the two, which is what
makes an empty section free rather than merely cheap, and what makes the
whole of emptiness a question about which entry sits at a palette position
rather than a question about the section.

`Section::index_storage_bytes()` reports the backing buffer's **real
length** (`buffer.len()`), never a figure recomputed from the width. This
matters: a recomputed figure agrees with every size assertion regardless
of whether the actual allocation is the right size, which is exactly the
kind of blind check that hides a broken allocator. Reading the buffer's
own length is what makes the size observable rather than merely computed.

## Compaction

Compaction is **explicit and public**, never eager. Reference counts are
maintained as voxels are written — incrementing the new entry's count
before decrementing the vacated one's, so an entry written back to the
block it already held never passes through a momentary zero — but dropping
vacant entries and narrowing the index width happen only when a caller
calls `Section::compact()`. This keeps that work off the block-editing hot
path (a 20 Hz authoritative tick shared by up to 32 players): only
persistence has a reason to want the minimal form, so only it pays for it.
Meshing (`rendering.md`) does **not** need the minimal form — it never calls
`compact()` on what it is given, and resolves only the palette entries at
least one voxel actually references, never consulting refcounts at all. A
section therefore meshes identically before and after compaction, which is
what makes a mesher indifferent to whichever caller happens to compact
first.

Compaction is **stable**: surviving entries keep their relative insertion
order, and every voxel's index is remapped to that entry's new position.
Compacting a section in which every palette entry is still referenced
changes nothing (same blocks, same palette length, same index width) — it
is not a no-op *implementation*, since it still has to check, but it is a
no-op *result*.

Compaction reads the counts the write path maintained; it does not
recompute them by walking every voxel. Recomputing would make compaction
come out correct even if the write-path refcounting were silently broken
— hiding exactly the defect compaction exists to expose. This is why
refcount transitions (write, overwrite-away, overwrite-back-to-the-same-block)
have their own direct tests, independent of anything compaction-visible.

## Column layout

A `ChunkColumn` is 16 stacked sections: `sections: [Section; 16]`
(`SECTIONS_PER_COLUMN = 16`). Column height (`COLUMN_HEIGHT`) is **derived**
as `SECTION_SIZE * SECTIONS_PER_COLUMN = 256` and is never written as a
literal anywhere — the height bound is the section array's own length, so
there is no second constant that could drift out of step with it. A
column-local y coordinate runs `0..=255`.

A column is addressed by `ColumnCoordinate { x: i32, z: i32 }` — signed,
because half of any world sits at negative x or z, and there is
deliberately no dimension concept: a column is identified by (x, z) alone.
`ChunkColumn` does not own a map, streaming, or eviction policy; columns
are held by whatever caller owns them.

## The world: columns addressed by `WorldPos`

Above a single column sits `mc_world::world::VoxelWorld`, which owns a footprint of columns and
exposes `block_at`, `set_block`, `column`, `columns` and `extent` over them — this is the type break
and place edits through (`docs/technical/architecture.md` §"The editable world"), and what a save
file persists (see "Saving and loading" below). Its coordinate type, `WorldPos { x: u32, y: u32, z:
u32 }`, is **unsigned** — the
project's world footprint sits entirely in the positive octant, so the one place a sign needs
checking is the conversion from a signed block position into a `WorldPos`, and that conversion is
where a negative coordinate is refused. Nothing downstream of it needs to check again.

`Extent { x, y, z }`, the type a caller uses to describe a world's footprint, lives here too — moved
from `mc-sim`, which re-exports it from its old path so existing fixtures keep compiling. A
`VoxelWorld` cannot hand back a type declared in a crate two levels further from `mc-core`, so the
type followed the data it describes rather than staying where its first caller happened to be.

## Two conventions worth stating explicitly

These are project-wide decisions, not local conveniences to this crate,
and are easy to get wrong by analogy with the wrong end:

- **`SectionError::OutOfBounds.limit` is the EXCLUSIVE upper bound.** For a
  section axis it is 16, not 15; for column height it is 256, not 255.
  This matches Rust's own `len()` and range convention: "you gave 16, the
  limit is 16" reads correctly as exclusive, where "the limit is 15"
  invites an off-by-one every time someone reasons about it.
- **The voxel layout is x-fastest, then y, then z**: a voxel's linear
  index within a section is `x + y*16 + z*256` (equivalently
  `x | (y << 4) | (z << 8)`, since 16 is a power of two).

## Module layout

```
mc_world::section   Section, Contents, LocalPos, Axis, SectionError,
                     SECTION_SIZE, VOXELS_PER_SECTION
                     — plus re-exports SectionData, PaletteIndex, ImportError
                       (defined in the private submodule section::export;
                        Contents likewise, from section::contents)
mc_world::column     ChunkColumn, ColumnPos, ColumnCoordinate,
                     SECTIONS_PER_COLUMN, COLUMN_HEIGHT, ColumnError
mc_world::persistence  save_world, write_save, replace_atomically,
                     load_world, LoadedWorld,
                     requirements, SaveRequirements, RequiredBlock,
                     saved_player, stored_world_data,
                     resolve, Acceptance, RegistryVerdict,
                     SavedPlayer, SaveNameId, DefinitionHash,
                     SaveError, LoadError
```

Every one of these is a module path, never a crate-root item — `Section`
and friends live under `mc_world::section`, `ChunkColumn` and friends
under `mc_world::column`, and the persistence surface under
`mc_world::persistence`.

## Saving and loading: the on-disk format

A save is a fixed 30-byte preamble, read by hand, followed by two successive
`postcard`-encoded values: a table of the block names the save needs, then
the world itself.

```text
offset  field                 encoding
     0  magic                 [u8; 8] = b"MYCRAFT\x1A"     read by hand
     8  format version        u16 LE  = 1                   read by hand
    10  player position       3 × f32 LE  (x, y, z)         read by hand
    22  player yaw            f32 LE, radians               read by hand
    26  player pitch          f32 LE, radians               read by hand
    30  ─── stored world data begins ───
    30  encoded table of names
     …  encoded world
        ─── end of file, exactly ───
```

Four things about this layout are decisions rather than incidental:

- **The magic is eight bytes ending in `0x1A`** — the trick PNG uses: a save
  mangled by a text-mode transfer fails the format check outright rather
  than parsing as something else.
- **Version 1 is numbered 1, not 0.** An all-zero buffer that somehow got
  past the magic check would otherwise declare the version this build
  *supports* rather than one it does not recognise; numbering the first
  version 1 keeps "all zeroes" a refusal instead of an accident of the
  numbering.
- **The version is read before anything it governs, by hand, at a fixed
  offset.** A version read *through* the encoder would depend on the
  encoder being able to decode a file whose format this build does not
  recognise — which defeats the point of having a version field at all.
  Reading it by hand means a build that cannot read a format never reports
  a complaint about bytes it was never entitled to interpret in the first
  place. Fixed width at a fixed offset for the same reason: a
  variable-width version would make reading the version of an unreadable
  file itself version-dependent. It also keeps the player's place at a
  fixed offset, which is what makes the stored world data below a clean
  suffix of the file.
- **The name table and the world are two top-level encoded values, not one
  wrapping struct.** The encoder decodes one value at a time from a reader
  and leaves it positioned exactly after that value, so asking a save what
  it needs decodes the table and stops there — a caller never touches a
  byte of chunk data to answer "what does this save require?" One wrapping
  struct would decode the world too, and that property would not hold.

### Writing a save

**A save replaces its predecessor atomically.** The bytes are written to a
sibling temporary file beside the target — same directory, therefore the
same volume, therefore a rename between them is atomic — flushed to disk
with `File::sync_all`, and only then moved into place with `fs::rename`. A
write that stops partway leaves the sibling incomplete and the previous
save byte-for-byte untouched; the sibling is never treated as a save that
merely finished late. The temporary file has to be a *sibling*, never a
system temp path: a rename across volumes becomes a copy-and-delete, which
is not atomic, and that is the one platform-specific assumption the whole
of atomic replacement rests on.

The write path emits the world in its **compacted** form: every section is
reduced to the palette entries at least one voxel still references before
it is written. A save is therefore always the minimal encoding of what a
world holds, never of its edit history — and it is also what keeps a
section's per-voxel index inside `u16` in the save even for a world that
was never compacted in memory.

### Reading a save back

A stored section becomes a `SectionData`, which `Section::import` turns
into a `Section`, which `ChunkColumn::assembled` stacks into a column,
which `VoxelWorld::assembled` puts back into a world. That route — rather
than replaying the file's edits through the ordinary per-voxel write path —
is what keeps a load from re-entering the registry-validating write path
once per voxel, a million times over for a full world.

### The save-table identifier's width, now decided

The single highest-risk mistake persistence could make was sizing a
save-wide name identifier against `PaletteIndex`'s `u16` — a width only
sufficient for a *compacted section* (≤4096 entries), not for the distinct
names across a whole save. That mistake is not made: a save's block-name
table is addressed by `SaveNameId`, a `u32` newtype local to the
persistence module, deliberately distinct from both `PaletteIndex` and
`BlockId`. A section's own stored form still carries one `PaletteIndex` per
voxel — the writer emits the compacted form, so that stays inside `u16` —
and it is the *palette entries*, not the voxels, that carry a `SaveNameId`
into the save's table.

Two levels rather than one halves the per-section storage cost and keeps
two failure modes distinct that a single `u32`-per-voxel scheme would
collapse into one: a voxel naming a palette position its own section's
palette does not have, refused by naming the world position it is at; and
a palette entry naming a table position the save's table does not have,
refused by naming the identifier and how many entries the table holds.

### Security properties of a save read from disk

A save file is attacker-controlled data read by the authoritative process,
and the persistence module treats it exactly that way: every length, count
and identifier decoded out of one is checked before it is trusted, and
nothing is indexed on a number the file merely claims.

**`postcard` bounds the bytes it reads, not the memory a value expands
into, and closing that gap is this module's job, not the library's.** A
length prefix in the file drives no allocation ahead of the elements that
actually follow it — a `Vec` fills only as elements arrive — but a small
file can still expand into a large in-memory structure:
`size_of::<ColumnRecord>()` against a one-byte minimum encoding is a 24×
amplifier, and `Vec` growth slack roughly doubles that worst case to ~48×.
That figure is **measured** against a crafted input, not estimated. Two
ceilings close the gap it leaves open:

- **A 16 MiB precheck against the file's own length**, read from
  `File::metadata` and checked before a single byte is decoded. At the
  measured ~48× worst-case amplification, that bounds peak memory at
  roughly 768 MiB on a maximally hostile file — four times the size of the
  largest legitimate MVP 1 save (~4 MiB) and eight times a typical one
  (~2 MiB). This is the only thing converting a bound on bytes *read* into
  a bound on memory, so raising the constant means re-deriving the
  amplification arithmetic against whatever the record shapes have become
  by then, never just picking a bigger number.
- **A 256-byte scratch buffer**, handed to the decoder for every
  byte-shaped field it reads. The decoder refuses outright, allocating
  nothing, when a declared length will not fit the buffer it is given.

**The 256-byte bound is per field, not cumulative — and that is a property
of the DTOs, not of the decoder, and a trap for whoever next changes
them.** A save's name-table entries hold owned `String`s, and an owned
`String` decodes through a path that reads into the scratch buffer
*without advancing past it* — the same 256 bytes are reused for the next
field. A borrowed `&str` decodes through a different path that *does*
advance the buffer, consuming part of it permanently for the life of the
read. Changing a DTO field from `String` to `&str` would silently turn a
per-field ceiling on the longest single block name into a cumulative
working-memory budget shared across every name in the table — with no
compiler error and no failing test to say so, because nothing here asserts
on which decode path a field takes, only that the result decodes.

**The division of labour is exact, and it is why no test in this codebase
asserts a `postcard` error variant or message.** The library's job is
turning bytes into typed values, and it is treated as working: a
widely-used decoder has had orders of magnitude more adversarial attention
than one feature can produce, and testing how it classifies a corrupt
input would be testing its release notes. Every refusal *this* module
raises is over an already-decoded value — a name that is not a namespaced
id, a count that does not match a footprint, a stored coordinate that is
not finite — and every one of them names the value that was wrong. A
`postcard` refusal crosses the boundary as `LoadError::Malformed { path }`
and nothing else; which way the library declined the bytes is not part of
the contract.

**Allocation follows data all the way through the read**, which is a
property to actively preserve rather than one that comes for free: a
future convenience such as reading a whole save into memory before
decoding it would throw this away silently, without changing a single
test's pass/fail outcome, because nothing here is sized to catch it. The
reader has to stay streaming for the property to keep holding.

### The three-outcome load decision

Resolving a save's name table against a registry produces exactly three
outcomes, judged per block and reported all at once rather than one at a
time:

- **Missing** — the registry does not hold the name at all. A hard
  refusal, unconditionally: nothing can go in the cell, which is not a
  judgement a caller is in a position to override.
- **Changed** — the registry holds the name, but its declared *behaviour*
  (solidity, replaceability, breakability, what it breaks into) hashes
  differently than the save recorded. **Loaded, and reported.** The names
  travel out of `load_world` on `LoadedWorld.changed` and the client says
  them on the error stream; `Acceptance::OnlyUnchangedBlocks` asks for the
  refusal instead, and a caller has to ask for it explicitly.
- **Unchanged, or changed only in declared *appearance*** (the keys its six
  faces draw from) — loads with nothing said about it. A texture edit
  cannot damage a world: the blocks are the same blocks and only look
  different.

**Only the first of the three is unconditional, and that asymmetry is the
whole of the decision.** A missing name means nothing can go in the cell,
so there is no answer a caller could give that would make the save
loadable. A changed name means the data is loadable, and refusing it turned
a content update into a world nobody could open — while the live reload of
the very same edit accepted it and moved the player where that was
genuinely dangerous. The two answers were inverted relative to risk.

Behaviour and appearance are hashed **separately**, into two independent
64-bit values, and that split is deliberate rather than an optimisation: a
single hash covering both would make a retextured mod indistinguishable
from a rebalanced one, and the only safe response to that ambiguity would
be to report every texture edit — teaching a player that a report means
nothing, which destroys the one thing the report is for.

### What each of the two hashes folds, and how its revision moves

Each hash is FNV-1a-64 over the block's declaration in the save's own
canonical encoding, which is what gives every variable-length field its
own length prefix — so `("ab", "c")` and `("a", "bc")` cannot fold to the
same value. Each list is written out by hand rather than derived from
`BlockDefinition`: a derive would bind every save in existence to a struct
that changes for reasons having nothing to do with storage.

| List | Fields, in order | Revision |
|---|---|---|
| Behaviour | revision byte, `name`, `is_solid`, `replaceable`, `breakable`, `breaks_into` | 1 |
| Appearance | revision byte, `name`, then the six declared keys in `up`, `down`, `north`, `south`, `east`, `west` order | 2 |

The **origin is in neither list**, and it is the field that would have
broken everything: it is a label derived from the path a definition was
read out of, so folding it would make a save written from a repository at
one checkout refuse to load from another, for a reason with nothing to do
with content and with a refusal a player could not tell apart from
corruption. The name is in **both**, so a block's two hashes cannot be
swapped for each other and one block's appearance cannot collide with
another's behaviour.

**The revision byte is per field list and never one number shared between
them, and the cost of unifying them has been measured rather than
argued.** The two lists grow for unrelated reasons, so a shared byte
bumped because the appearance list gained a field moves every block's
*behaviour* hash in every save in existence. Run against the committed
pre-spec save at the moment the appearance list grew to six keys, the two
arrangements answer:

```
changed:    [base:dirt, base:grass, base:stone, base:water]   // one shared byte
retextured: [base:dirt, base:grass, base:stone, base:water]   // a byte per list
```

Those are not two shades of the same answer. Read the three-outcome
decision above: a non-empty `changed` is **named on the player's terminal**,
and refuses the load outright under `Acceptance::OnlyUnchangedBlocks`, while
`retextured` is neither reported nor refused. So one shared constant means
**every save written before this spec tells its player that every block they
built with behaves differently** — when nothing behaves differently, and all
that happened is that they were retextured. The measurement was 5 of 1 224
tests red, and the fifth is the behaviour-half guard reporting exactly the
defect it was written for.

**The cost of a shared byte fell when the default flipped, and it did not
fall to nothing.** Before, it refused every save in existence; now it
mislabels every one of them on the way in. That is a smaller failure and it
is a worse one to *find*, because a world that opens looks fine — which is
why the guard is still a whole-verdict comparison against a committed save
rather than a check that the load succeeded. Measured again on the tree
that flipped the default: reporting a retextured block as changed reddens 7
of 1 385, one of them the run of the shipped binary itself.

**This paragraph exists because the next reader is right by every rule
they can see.** Two constants where there was one is duplication, two
numbers doing one job, and unifying them is the obvious tidy-up — obvious,
one line, and it turns a silent retexture into a refused world. So there
are two constants, only one of them has ever moved, and the reason is
written here rather than left to be rediscovered.

**The appearance list is at revision 2 because an appearance became six
keys instead of one.** The consequence is player-visible and intended:
every save written before that revision reports **every block it holds as
retextured** on its next load. That is correct rather than a migration
defect — every block's appearance really did change — and a retexture is
in the arm that is neither reported nor refused, so nothing about opening
an older world changed. **The shipped content has since moved a behaviour
hash as well**: `content/base/blocks/water.luau` declares
`breakable = false`, so the committed pre-Luau save now reports `base:water`
as changed and its other three blocks as retextured, which is the two-hash
separation firing on a real content edit rather than on a fixture.

A future change to what a block looks like moves this byte again and no
other; a future change to what a block *is* moves the other one.

A save written before a revision is never compared field-by-field against
one written after it. Two values folded over different field lists agree
or disagree for reasons nothing in the save records, so the honest answer
is that the whole list moved, which is what the revision byte says.

**If you move a revision, know which tests can see it: only the ones that
state the byte sequence.** This was measured rather than assumed. Leaving
the appearance byte at 1 while the list grew to six keys reddens exactly
the two guards that build the expected bytes by hand — `format_test.rs`'s
appearance half and `tests/save_per_face_appearance.rs`'s stated-bytes
guard — and **nothing else in the workspace**. In particular the guard
over the committed pre-spec save stays green, because six keys under
revision 1 still fold differently from one key under revision 1, so an
older save still reports its blocks retextured either way.

The consequence for whoever changes one of these: **a green suite is not
evidence that the byte is right.** Every other witness compares one fold
to another, and a comparison between two folds cannot see a leading byte
that moved in both. Change the constant and the stating tests are the
whole of what reports it.

**The asymmetry behind refusing rather than substituting a placeholder for
a missing block is about which failure is recoverable.** A refused load
leaves the save file exactly as it was on disk; a player can restore the
mod that defined the missing block and try again, at the cost of one
restart. A placeholder — a stand-in that lets the load proceed and
remembers the original name so it can be restored later — has no such
recovery: the moment a player saves over a world whose absent blocks were
silently substituted, the original names are gone, permanently, in the one
file that held them. Refusing costs a restart; a placeholder can cost a
world. No unknown-block placeholder is built, and that is not an
unfinished edge of this feature — it is the decided boundary, for that
reason (see "Known limitations, as built" below for the in-memory case
this leaves alone). A save naming a missing block is refused with the
complete list of what is missing, and that refusal is not something any
caller acceptance can override.

### A save's stored world data is a deterministic image of the world

The block-name table is built once per save from the union of every
distinct name the world's (compacted) sections hold, kept in a `BTreeMap`
and written out in ascending lexicographic order — never in registration
order, and never via a hash-ordered collection: `HashMap`/`HashSet` do not
appear anywhere in the persistence module, because Rust's per-instance
`RandomState` would make two saves of the same world differ by nothing but
which hash seed one process happened to pick. One `SaveNameId` is then
assigned to each name by its position in that ascending order, after the
whole set is known — an identifier is a position in an order, and nothing
is in order until the last name has arrived.

The consequence: saving one world twice, against two registries that
register the same blocks in different orders, or at different runtime
ids, or built from definitions read from two different origins, produces
byte-identical stored world data. What is compared for that identity is
the save's **stored world data** — the bytes from the end of the preamble
onward — never the whole file: a container carrying allocator or
transaction state of its own would fail a whole-file comparison for
reasons that have nothing to do with the world it holds, and the
plain-file format described above is what makes the two coincide.

## Known limitations, as built

These are current behaviour, not a roadmap.

- **The changed-blocks report is a one-shot, and playing on destroys the
  evidence.** A save is rewritten against the current declarations on quit:
  `NameTable::record` writes `behaviour_of`/`appearance_of` from the registry
  the process is running, not the values the file arrived with. So *load a
  changed save → play → quit normally* replaces the recorded hashes, the
  mismatch is gone from the file for good, and the next launch has nothing
  left to notice. **This is the whole reason
  `Acceptance::OnlyUnchangedBlocks` exists** and is not a flag nobody
  passes: somebody restoring a backup they are unsure of wants the world
  left shut, because opening it is what destroys the record of what it was.
  Nothing warns about this at the moment the save is rewritten, and no
  test asserts it — it is existing behaviour that this note records rather
  than behaviour anything here created.

  There is no per-load backup and no copy-on-open. A player who wants the
  old record keeps their own copy of `saves/world.mcw`, which is what
  `docs/user/gameplay.md` tells them.

- **Importing a section that names an unregistered block is still an
  error** (`ImportError::UnknownBlock`), and there is still no
  unknown-block placeholder — but this is now a decided boundary rather
  than an outstanding gap. A save naming a block the registry does not
  hold is refused before any section reaches `Section::import` at all
  (see "Saving and loading" above): a load resolves the whole name table
  first, and a missing name is a hard refusal regardless of any caller
  acceptance. By the time a section is imported during a load, every name
  it can reference has already been confirmed present — `Section::import`'s
  own check exists for the general case (any caller building a section
  from a `SectionData`, not only a load), and it is not what a hostile or
  outdated save is actually refused by.

  A placeholder that preserves an unresolved name was considered and
  rejected: it would put a `BlockName` the registry does not know into a
  loaded `Section`, which is exactly the precondition of the in-memory
  failure described in the next entry — `is_solid_at` raising
  `RegistryError::UnknownName` mid-tick, and the mesher failing the whole
  section with `MeshError::UnresolvedBlock`. A refused load leaves the
  save file untouched on disk and is recoverable by restoring the mod
  that defined the missing block; a placeholder is the only version of
  this that can destroy data, silently, the moment a player saves over a
  world whose absent blocks were substituted. Refusing costs a restart; a
  placeholder can cost a world — which is why this stays a refusal.

  The check still runs over the palette's `Contents::Holds` entries and
  still skips `Contents::Empty` — **the empty entry only, and that is the
  whole of the exemption**. Requiring registration for it would make an
  empty cell need a block registered in order to mean nothing; skipping
  the check for every entry instead would let a description name a block
  that does not exist and build a world quietly made of something else.
- **A section holding a *block* the current registry has stopped
  registering fails every `is_solid_at` call on it.** After a live registry
  swap (arriving with the Luau scripting host), this failure is delivered
  mid-tick to whichever systems call `is_solid_at` — typically physics, one
  of the systems least able to react gracefully. An *empty* cell is not
  affected: it answers not-solid before the registry is consulted, so no
  registry change can make an empty cell fail. The mesher (`rendering.md`)
  is the other system this would have hit; it fails a different, sharper
  way: a section holding a name the registry cannot resolve fails the
  **whole mesh** with `MeshError::UnresolvedBlock`, rather than failing per
  `is_solid_at` call, and it refuses outright rather than inventing a
  placeholder policy of its own. This in-memory case remains unreachable
  in today's product — the registry is built once at startup and never
  swapped — and stays owned by whatever future work makes a live registry
  swap reachable. A load-time refusal cannot pre-empt it: it is triggered
  by a mod disappearing from an already-running server, not by loading a
  save, and the entry above's decision not to build a placeholder applies
  only to the load path, not to this one.
- **"Not registered" is spelled two different ways on the public
  surface.** `Section::is_solid_at` propagates
  `SectionError::Registry(RegistryError::UnknownName)` for a name the
  supplied registry does not hold; the name-taking mutators
  (`set_block`, `filled`) report the same condition as
  `SectionError::UnknownBlock`. Both are reachable, neither is tested
  against the other, and the inconsistency is unresolved. The
  emptiness-taking operations (`empty`, `empty_at`) are outside this
  entirely — they have no unregistered-block failure to spell either way.
- **Palette lookup is a linear scan over its entries.** Acceptable while
  a section's palette stays small (a handful of entries in any world a
  real workload produces); worth revisiting if a mesher benchmark shows it
  material, or if a real workload produces palettes longer than roughly
  64 entries.
- **`ChunkColumn::section(index) -> Option<&Section>` now exists and is
  public**, bounded by the section array itself — like `block_at` — never by
  a second height constant. The per-voxel palette-position read path the
  mesher needed also exists, but stays `pub(crate)`: the mesher lives inside
  `mc-world`, so it needs no public accessor, and no per-voxel public read
  API was added alongside it. `Section::is_solid_at` remains **not** the
  intended per-voxel API — a mesher resolves solidity once per *referenced
  palette entry*, not once per voxel; see `rendering.md` for how that keeps
  meshing off the string-keyed-lookup cost this entry used to warn about.
- **`SectionError::CorruptPaletteIndex` is unreachable by design.** It
  represents an internal invariant (every packed index names a position
  this section's own palette actually has) that no public API can violate,
  constructed at exactly one private call site. Its one line is
  deliberately uncovered.
