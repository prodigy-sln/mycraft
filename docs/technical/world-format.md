# Chunk Storage and the Block Palette

How a chunk's voxels are held in memory, why a runtime block id may never
leak into a saved or transmitted form, and what a section's storable
identity actually is. This is the in-memory representation; turning it into
bytes on disk is `mc-world`'s persistence work, not this.

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
    pub palette: Vec<BlockName>,
    pub indices: Vec<PaletteIndex>,
}
```

a palette of namespaced names in the section's own palette order, plus one
`PaletteIndex` per voxel naming that voxel's position into it. `export`
does not normalize or sort the palette, and does not implicitly compact —
the exported order is exactly the section's own, vacant entries included.
A caller wanting the minimal form compacts first, which is why compaction
is a public operation rather than something export does on a caller's
behalf.

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

## Section format

A section is `SECTION_SIZE`³ = 4096 voxels (`SECTION_SIZE = 16`, and
`VOXELS_PER_SECTION` is derived from it, never written as a separate
literal). Each section holds:

- A **palette**: an insertion-ordered list of `(BlockName, refcount)`
  entries. `refcount` is the number of voxels currently holding that
  entry; an entry with a zero refcount is a vacated entry, kept until
  compaction.
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

A section holding exactly one distinct block (the common case — a chunk of
solid stone, or of air) spends nothing at all on its index buffer: the
homogeneous, 0-bit tier owns no buffer. Every tier either divides 8 or is a
multiple of 8, so **a packed index never straddles a byte boundary** — the
implementation has no general straddle-handling code because the case
cannot occur.

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
mc_world::section   Section, LocalPos, Axis, SectionError,
                     SECTION_SIZE, VOXELS_PER_SECTION
                     — plus re-exports SectionData, PaletteIndex, ImportError
                       (defined in the private submodule section::export)
mc_world::column     ChunkColumn, ColumnPos, ColumnCoordinate,
                     SECTIONS_PER_COLUMN, COLUMN_HEIGHT
```

Every one of these is a module path, never a crate-root item — `Section`
and friends live under `mc_world::section`, `ChunkColumn` and friends
under `mc_world::column`.

## Known limitations, as built

These are current behaviour, not a roadmap.

- **Importing a section that names an unregistered block is an error**
  (`ImportError::UnknownBlock`). There are no unknown-block placeholders
  yet, so a section produced under a mod set that has since removed a mod
  does not round-trip through import. Persistence work owns adding
  placeholders.
- **A section holding a name the current registry has stopped registering
  fails every `is_solid_at` call on it.** After a live registry swap
  (arriving with the Luau scripting host), this failure is delivered
  mid-tick to whichever systems call `is_solid_at` — typically physics, one
  of the systems least able to react gracefully. The mesher (`rendering.md`)
  is the other system this would have hit; it now fails a different,
  sharper way: a section holding a name the registry cannot resolve fails
  the **whole mesh** with `MeshError::UnresolvedBlock`, rather than failing
  per `is_solid_at` call, and it refuses outright rather than inventing a
  placeholder policy of its own. The import-path placeholder work above
  still needs to cover this **in-memory** case, not only the import path, or
  the failure mode simply moves rather than disappearing.
- **"Not registered" is spelled two different ways on the public
  surface.** `Section::is_solid_at` propagates
  `SectionError::Registry(RegistryError::UnknownName)` for a name the
  supplied registry does not hold; the name-taking mutators
  (`set_block`, `filled`) report the same condition as
  `SectionError::UnknownBlock`. Both are reachable, neither is tested
  against the other, and the inconsistency is unresolved.
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
