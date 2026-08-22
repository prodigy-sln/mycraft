//! The world a save stored, read back against a registry.
//!
//! **The table is resolved before a chunk is decoded, and that order is the one
//! property the whole table exists for.** A save naming blocks the registry does
//! not hold is turned away by naming all of them, whatever else is wrong with the
//! file — a refusal reporting the file's other problem instead would mean the
//! resolution had moved after the world, and a player would be told their save is
//! broken rather than which mod to put back.
//!
//! The route the reading takes is fixed and is not a detail: a stored section
//! becomes a [`SectionData`], which [`Section::import`] turns into a section,
//! which [`ChunkColumn::assembled`] stacks into a column, which
//! [`VoxelWorld::assembled`] puts back into a world. That is what keeps a load
//! out of the registry-validating per-voxel write path, which it would otherwise
//! re-enter a million times.
//!
//! **Every count and identifier here is attacker-controlled, and each is checked
//! by us rather than by the decoder.** The library's job is turning bytes into
//! typed values; what those values *mean* is this module's. A column list that
//! does not fill the footprint it declares, a palette entry naming a table
//! position the table lacks, a voxel naming a palette position its section lacks,
//! and bytes left over after the world are all decoded perfectly well and all
//! refused here.
//!
//! **A voxel reaches the save's table through its palette entry**, which is why
//! a voxel naming a position its palette lacks and a palette entry naming a name
//! the table lacks are two refusals and not one. Only the first can name a world
//! position, and only this module can: the section importer knows the index and
//! its palette's length but not which of four thousand voxels carried it.

use std::io::{self, Read};
use std::path::Path;

use mc_core::block::BlockRegistry;
use mc_core::id::BlockName;

use super::reader::{
    RequiredBlock, every_number_finite, malformed, opened_with_length, preamble_of, required_of,
};
use crate::column::{ChunkColumn, ColumnCoordinate};
use crate::persistence::error::LoadError;
use crate::persistence::format::{
    ColumnRecord, MAX_NAME_BYTES, PaletteEntry, SavedPlayer, SectionRecord, WorldRecord,
};
use crate::persistence::table::{Acceptance, resolve};
use crate::section::{
    Contents, PaletteIndex, SECTION_SIZE, Section, SectionData, VOXELS_PER_SECTION,
};
use crate::world::{VoxelWorld, WorldPos, every_column};

/// How far one axis is shifted in a voxel's linear position inside its section,
/// and the mask that reads it back.
///
/// A shift and a mask rather than a division and a remainder, which is the same
/// arithmetic for a power of two and is what `clippy::integer_division` leaves
/// available.
const AXIS_SHIFT: u32 = SECTION_SIZE.trailing_zeros();
const AXIS_MASK: u32 = SECTION_SIZE - 1;

/// The shift above only splits a linear position correctly while a section's
/// size is the power of two it was derived from.
const _: () = assert!(1 << AXIS_SHIFT == SECTION_SIZE);

/// What a save holds: the world it stored, where it left the player, and which
/// of its blocks the registry no longer declares the same way.
///
/// **The third field is what a load carries out rather than swallows.** The
/// verdict was computed here and dropped, so the one thing a player can act on —
/// which blocks their world disagrees with the content about — existed for the
/// length of one statement. It travels because a verdict computed and dropped
/// satisfies nothing, which is the same reason `Clearing` travels out of
/// `mc-sim`.
///
/// It carries the **changed** list alone and not the whole verdict. A missing
/// name is not reported here because it is never loaded at all: this value only
/// exists where the load succeeded. A retextured name is not reported because a
/// line after every art edit is noise the one that matters would hide in.
///
/// `PartialEq` without `Eq`, which is what a stored coordinate allows.
#[derive(Debug, Clone, PartialEq)]
pub struct LoadedWorld {
    pub world: VoxelWorld,
    pub player: SavedPlayer,
    /// The names this save records whose declared behaviour has since changed,
    /// ascending. Empty where every block is still what it was.
    pub changed: Vec<BlockName>,
}

/// What a stored section is being read into: the names the save's table carries,
/// and the registry the blocks are built against.
///
/// A value rather than two more arguments, because the section conversion
/// already takes the section and where in the world it sits, and a fifth
/// argument is a context struct rather than a fight with a lint.
#[derive(Clone, Copy)]
struct Reading<'a> {
    names: &'a [RequiredBlock],
    registry: &'a BlockRegistry,
}

/// Where a stored section sits in the world it is part of.
///
/// Carried down to the voxel because a cell naming a palette position its
/// section does not have is refused by **where it is** — which is the only part
/// of that failure a player can act on. "Palette position 5 of 1" is true of a
/// corrupt file in four thousand places and identifies none of them.
#[derive(Clone, Copy)]
struct At {
    column_x: u32,
    column_z: u32,
    section: usize,
}

impl At {
    /// Where the voxel at linear position `offset` of this section sits in the
    /// world.
    ///
    /// The offset counts x fastest, then y, then z — the order a section
    /// describes itself in. Every offset this is called with is inside a
    /// section, so the narrowing below cannot lose anything; it is written as a
    /// fallback rather than an unwrap because reading a save may not end the
    /// process.
    fn world_position(self, offset: usize) -> WorldPos {
        let local = u32::try_from(offset).unwrap_or_default();
        let height = u32::try_from(self.section).unwrap_or_default();
        WorldPos {
            x: self.column_x * SECTION_SIZE + (local & AXIS_MASK),
            y: height * SECTION_SIZE + ((local >> AXIS_SHIFT) & AXIS_MASK),
            z: self.column_z * SECTION_SIZE + ((local >> (AXIS_SHIFT * 2)) & AXIS_MASK),
        }
    }
}

/// The world and the player the save at `path` holds, resolved against
/// `registry`, together with the blocks it records differently from what
/// `registry` declares now.
///
/// `accepting` is the caller's decision about blocks whose declarations have
/// changed since the save was written. **The default is to load**: a save whose
/// blocks have changed is loadable data, and refusing it turns a content update
/// into a world nobody can open — so the changed names are reported and the
/// world is handed back. [`Acceptance::OnlyUnchangedBlocks`] asks for the
/// stricter answer, which exists because opening such a save and playing on
/// rewrites its hashes and destroys the evidence that anything moved. Neither
/// answer covers a name the registry does not hold at all.
///
/// # Errors
///
/// Returns [`LoadError::Missing`] if nothing is at `path`,
/// [`LoadError::Unresolvable`] if the registry cannot answer for the blocks the
/// save names, and otherwise whatever reading the save refuses with.
pub fn load_world(
    path: &Path,
    registry: &BlockRegistry,
    accepting: Acceptance,
) -> Result<LoadedWorld, LoadError> {
    let (reader, length) = opened_with_length(path)?;
    let mut counted = Counting::of(reader);
    let player = every_number_finite(preamble_of(&mut counted, path)?)?;
    let required = required_of(&mut counted, path)?;
    // Before a chunk is decoded, and that is the requirement rather than an
    // optimisation: the complete list of what is missing or changed is what a
    // player acts on, and a file with a second problem behind it must not get to
    // report that one instead.
    let verdict = resolve(&required, registry);
    if let Some(refusal) = verdict.refusal(accepting) {
        return Err(refusal);
    }
    let record = decoded_world(&mut counted, path)?;
    ended_where_the_world_did(counted.taken, length)?;
    let world = assembled_world(
        &record,
        Reading {
            names: required.blocks(),
            registry,
        },
        path,
    )?;
    Ok(LoadedWorld {
        world,
        player,
        // The verdict's own list, already ascending because `resolve` sorts what
        // it reports. Not recomputed and not re-sorted: a second answer to the
        // question the refusal above was asked is a second place for the two to
        // disagree about the same save.
        changed: verdict.changed,
    })
}

/// The world `reader` is positioned at.
///
/// The scratch buffer is the same bound every other decoded field gets: nothing
/// a file merely declares a length for is allocated before the bytes behind it
/// have arrived.
fn decoded_world(reader: &mut impl Read, path: &Path) -> Result<WorldRecord, LoadError> {
    let mut scratch = [0_u8; MAX_NAME_BYTES];
    postcard::from_io((reader, scratch.as_mut_slice()))
        .map(|(world, _positioned_after_it)| world)
        .map_err(|_refused| malformed(path))
}

/// Refuses a save carrying anything after the world it stored.
///
/// A save ends where its world ends, exactly. Bytes past that are a file
/// somebody appended to — a smuggled payload, a botched merge of two saves, a
/// download that overshot — and reading the world out and shrugging at the rest
/// would accept all three.
fn ended_where_the_world_did(taken: u64, length: u64) -> Result<(), LoadError> {
    if taken < length {
        return Err(LoadError::TrailingBytes {
            should_have_ended_at: taken,
        });
    }
    Ok(())
}

/// The world `record` describes, built against `reading`.
fn assembled_world(
    record: &WorldRecord,
    reading: Reading<'_>,
    path: &Path,
) -> Result<VoxelWorld, LoadError> {
    fills_its_footprint(record, path)?;
    let mut columns = Vec::with_capacity(record.columns.len());
    for ((column_x, column_z), stored) in every_column(record.footprint_side).zip(&record.columns) {
        columns.push(column_of(stored, column_x, column_z, reading)?);
    }
    Ok(VoxelWorld::assembled(record.footprint_side, columns)?)
}

/// Refuses a column list that does not fill the footprint the save declares.
///
/// The square is checked rather than taken: an overflow is a refusal and never a
/// wrap, because a wrapped expectation is one a hostile file gets to choose. A
/// side whose square does not fit an address at all is refused as a file this
/// build cannot make sense of, which is what it is — there is no count to
/// report, only a number no world could have.
fn fills_its_footprint(record: &WorldRecord, path: &Path) -> Result<(), LoadError> {
    let expected = usize::try_from(record.footprint_side)
        .ok()
        .and_then(|side| side.checked_mul(side))
        .ok_or_else(|| malformed(path))?;
    let found = record.columns.len();
    if found != expected {
        return Err(LoadError::WrongColumnCount { expected, found });
    }
    Ok(())
}

/// The column `stored` describes, at `(column_x, column_z)`.
fn column_of(
    stored: &ColumnRecord,
    column_x: u32,
    column_z: u32,
    reading: Reading<'_>,
) -> Result<ChunkColumn, LoadError> {
    let mut sections = Vec::with_capacity(stored.sections.len());
    for (section, described) in stored.sections.iter().enumerate() {
        let at = At {
            column_x,
            column_z,
            section,
        };
        sections.push(section_of(described, at, reading)?);
    }
    let coordinate = ColumnCoordinate {
        x: column_x as i32,
        z: column_z as i32,
    };
    Ok(ChunkColumn::assembled(coordinate, sections)?)
}

/// The section `stored` describes, sitting at `at`.
fn section_of(stored: &SectionRecord, at: At, reading: Reading<'_>) -> Result<Section, LoadError> {
    let described = SectionData {
        palette: palette_of(&stored.palette, reading.names)?,
        indices: stored
            .indices
            .iter()
            .copied()
            .map(PaletteIndex::new)
            .collect(),
    };
    every_cell_names_an_entry(&described, at)?;
    Ok(Section::import(&described, reading.registry)?)
}

/// What each palette position of `stored` holds, as names rather than as the
/// save's own numbering.
///
/// **The file's identifier stops here.** Nothing past this point knows which
/// position a name landed at in the file it came out of, which is what makes a
/// save independent of the registry it is read against.
fn palette_of(
    stored: &[PaletteEntry],
    names: &[RequiredBlock],
) -> Result<Vec<Contents>, LoadError> {
    let mut palette = Vec::with_capacity(stored.len());
    for entry in stored {
        palette.push(match entry {
            PaletteEntry::Empty => Contents::Empty,
            PaletteEntry::Holds(id) => Contents::Holds(named(*id, names)?),
        });
    }
    Ok(palette)
}

/// The name the save's table carries at `id`.
///
/// Looked up rather than indexed, and refused naming how many the table holds:
/// indexing on a number a file chose is the thing a hostile save is written to
/// make a reader do, and a count with no scale beside it is a number rather than
/// a fact about the file.
fn named(id: u32, names: &[RequiredBlock]) -> Result<BlockName, LoadError> {
    usize::try_from(id)
        .ok()
        .and_then(|position| names.get(position))
        .map(|block| block.name.clone())
        .ok_or(LoadError::UnknownNameId {
            id,
            table_len: names.len(),
        })
}

/// Refuses the first cell of `described` naming a palette position its section
/// does not have, saying where in the world that cell is.
///
/// Checked here rather than left to the import, because the import knows the
/// position and the palette's length and not which voxel carried them — and the
/// world position is the whole of what a player can look at.
///
/// Only a whole section's worth is examined: a description carrying more
/// positions than a section has is a different refusal about a different thing,
/// and it is the import's, so an offset past the end of a section is left to it
/// rather than given a world position it does not have.
fn every_cell_names_an_entry(described: &SectionData, at: At) -> Result<(), LoadError> {
    let palette_len = described.palette.len();
    for (offset, index) in described
        .indices
        .iter()
        .enumerate()
        .take(VOXELS_PER_SECTION)
    {
        if usize::from(index.get()) >= palette_len {
            return Err(LoadError::UnknownCellEntry {
                at: at.world_position(offset),
                index: index.get(),
                palette_len,
            });
        }
    }
    Ok(())
}

/// A reader that remembers how many bytes were taken through it.
///
/// **Where the save ended is a fact only the reading knows.** A file's length
/// says how many bytes are there and the decoder says how many it needed; the
/// difference is what says a save was appended to, and neither half can report
/// it alone. Ten lines around a `Read`, and deliberately not a parser.
///
/// It sits *outside* the buffering, so what it counts is what the decoder
/// consumed rather than what was read ahead on its behalf.
#[derive(Debug)]
struct Counting<R> {
    inner: R,
    taken: u64,
}

impl<R: Read> Counting<R> {
    /// `inner`, counting from nothing.
    const fn of(inner: R) -> Self {
        Self { inner, taken: 0 }
    }
}

impl<R: Read> Read for Counting<R> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let taken = self.inner.read(buffer)?;
        // Saturating rather than wrapping: a count that wrapped would make a
        // file look shorter than it is, which is the one direction this number
        // may not be wrong in.
        self.taken = self
            .taken
            .saturating_add(u64::try_from(taken).unwrap_or(u64::MAX));
        Ok(taken)
    }
}
