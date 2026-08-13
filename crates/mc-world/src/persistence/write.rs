//! Writing a world down: where a save is allowed to go, and how it replaces the
//! one that was there.
//!
//! **A save replaces its predecessor by being renamed over it, never by being
//! written into it.** A writer that opens the target and streams into it has,
//! for the length of the write, neither the old world nor the new one — and a
//! machine that stops in that window costs a player everything they built. So
//! the bytes go into a file beside the target, are flushed to the disk, and only
//! then take its place.
//!
//! **The temporary file is a sibling of the save and never the system temporary
//! directory.** A rename is atomic only within one volume: across volumes it
//! becomes a copy and a delete, which is exactly the window the sibling exists to
//! close. This is the single platform-specific assumption the whole of the above
//! rests on.
//!
//! The refusals before any of that are an *order* and not a set. Each one exists
//! because the failure it names is one a player can act on and a bare write
//! failure is not — and one of them names something no filesystem call will name
//! for us: when a *component* of the parent is a file rather than a directory,
//! creating the directories reports that something was not a directory without
//! ever saying which something.

use std::fs::{self, File};
use std::io::{self, ErrorKind, Write};
use std::path::{Path, PathBuf};

use mc_core::block::BlockRegistry;
use serde::Serialize;

use super::error::SaveError;
use super::format::{
    ColumnRecord, FORMAT_VERSION, MAGIC, PaletteEntry, STORED_WORLD_DATA_AT, SavedPlayer,
    SectionRecord, WorldRecord,
};
use super::table::NameTable;
use crate::column::ChunkColumn;
use crate::section::{Contents, SectionData};
use crate::world::VoxelWorld;

/// Every section of every column of a world, in the order a save writes them:
/// columns in the world's own assembly order, sections bottom-up.
type DescribedWorld = Vec<Vec<SectionData>>;

/// Every byte of a save of `world`, in order, into `sink`.
///
/// Public because the format has to be usable without a filesystem, and because
/// an interrupted write can only be induced honestly from outside — by a sink
/// that stops accepting bytes part-way through, which is what a disk filling up
/// mid-quit looks like from in here.
///
/// `registry` is read for the **declarations** of the blocks the world holds and
/// never for their runtime ids: an id is dense, registry-local and reassigned
/// the moment the block set changes, so a save that stored one would start
/// reporting whichever block happened to be numbered the same after an update.
///
/// # Errors
///
/// Returns [`SaveError::UnknownBlock`] if the world holds a block `registry`
/// does not declare, [`SaveError::TooManyNames`] if it holds more distinct names
/// than a save's table can address, [`SaveError::Section`] if a section cannot
/// describe what it holds, and [`SaveError::Io`] if `sink` refuses the bytes.
pub fn write_save(
    sink: &mut dyn Write,
    world: &VoxelWorld,
    player: SavedPlayer,
    registry: &BlockRegistry,
) -> Result<(), SaveError> {
    let described = described_world(world)?;
    let table = NameTable::of(described.iter().flatten(), registry)?;
    let record = world_record(world, &described, &table)?;
    write_preamble(sink, player)?;
    encoded_into(sink, &table.record())?;
    encoded_into(sink, &record)
}

/// Replaces the file at `path` with whatever `fill` writes, atomically.
///
/// The mechanism [`save_world`] is built on, public for the same reason
/// [`write_save`] is: an interruption that cannot be induced through the public
/// surface is a requirement asserted against nothing.
///
/// # Errors
///
/// Returns [`SaveError::Io`] if the sibling cannot be created, flushed or
/// renamed, and whatever `fill` itself refused with.
pub fn replace_atomically(
    path: &Path,
    fill: impl FnOnce(&mut dyn Write) -> Result<(), SaveError>,
) -> Result<(), SaveError> {
    let beside = sibling_of(path)?;
    match filled(&beside, fill) {
        Ok(()) => fs::rename(&beside, path).map_err(|failure| io_failure(path, &failure)),
        Err(refusal) => {
            // The write error is the one the caller has to act on. A failure to
            // clear the half-written sibling would replace it with a strictly
            // less useful one, so it is discarded here rather than reported —
            // and discarded at a call site that says so.
            let _discarded = fs::remove_file(&beside);
            Err(attributed_to(&beside, refusal))
        }
    }
}

/// Writes a save of `world` at `path`, replacing any previous one.
///
/// # Errors
///
/// Returns [`SaveError::PathIsDirectory`] if `path` names a directory,
/// [`SaveError::NotADirectory`] if a component of its parent names a file, and
/// otherwise whatever [`write_save`] and [`replace_atomically`] refuse with.
pub fn save_world(
    path: &Path,
    world: &VoxelWorld,
    player: SavedPlayer,
    registry: &BlockRegistry,
) -> Result<(), SaveError> {
    // Asked outright rather than read off whatever creating the file reports:
    // on Windows that is a permission failure, which names neither the mistake
    // nor the path a player has to undo it at.
    if path.is_dir() {
        return Err(SaveError::PathIsDirectory {
            path: path.to_owned(),
        });
    }
    if let Some(parent) = path.parent() {
        made_ready(parent)?;
    }
    replace_atomically(path, |sink| write_save(sink, world, player, registry))
}

/// Makes sure `parent` is a directory a save can be written into.
///
/// A player's first save has no directory waiting for it, so the levels above it
/// are this path's to create — refusing a world because the folder it goes in has
/// never been made would make the very first quit the one that loses everything.
fn made_ready(parent: &Path) -> Result<(), SaveError> {
    if let Some(component) = first_component_that_is_a_file(parent) {
        return Err(SaveError::NotADirectory { component });
    }
    if parent.as_os_str().is_empty() {
        return Ok(());
    }
    fs::create_dir_all(parent).map_err(|failure| io_failure(parent, &failure))
}

/// The shallowest component of `parent` that exists and is not a directory.
///
/// Walked from the root down and ours to walk: creating the directories reports
/// that something along the way was not a directory without saying which, and the
/// component that is a file is the only thing a player can act on.
fn first_component_that_is_a_file(parent: &Path) -> Option<PathBuf> {
    let mut ancestors: Vec<&Path> = parent.ancestors().collect();
    ancestors.reverse();
    ancestors
        .into_iter()
        .find(|ancestor| ancestor.exists() && !ancestor.is_dir())
        .map(Path::to_owned)
}

/// The file a save is completed in before it takes `path`'s place.
///
/// Beside the target, therefore on the target's volume, therefore renamed over
/// it in one step. A deterministic name is safe while one process writes one save
/// at a time, which is the only arrangement this format is written for.
fn sibling_of(path: &Path) -> Result<PathBuf, SaveError> {
    let mut beside = path
        .file_name()
        .ok_or_else(|| SaveError::Io {
            path: path.to_owned(),
            kind: ErrorKind::InvalidInput,
        })?
        .to_owned();
    beside.push(".tmp");
    Ok(path.with_file_name(beside))
}

/// Fills `beside` with whatever `fill` writes, and puts it on the disk.
///
/// The flush is what makes "complete before it replaces" real rather than
/// nominal: a rename over a file whose bytes are still only in a cache promises
/// nothing about what survives the machine stopping.
fn filled(
    beside: &Path,
    fill: impl FnOnce(&mut dyn Write) -> Result<(), SaveError>,
) -> Result<(), SaveError> {
    let mut file = File::create(beside).map_err(|failure| io_failure(beside, &failure))?;
    fill(&mut file)?;
    file.sync_all()
        .map_err(|failure| io_failure(beside, &failure))
}

/// Every section of every column, described and reduced to its minimal form.
///
/// **The minimal form is what a save stores.** An entry no voxel refers to any
/// more is a palette's book-keeping, kept because reclaiming it on the edit path
/// would put a renumbering of every voxel into a tick shared by everyone
/// connected. The save is where that debt is paid — and paying it is what stops a
/// world refusing to load over a block that is not in it.
fn described_world(world: &VoxelWorld) -> Result<DescribedWorld, SaveError> {
    let mut columns = Vec::with_capacity(world.columns().count());
    for column in world.columns() {
        columns.push(described_column(column)?);
    }
    Ok(columns)
}

/// Every section of `column`, bottom-up, in its minimal form.
fn described_column(column: &ChunkColumn) -> Result<Vec<SectionData>, SaveError> {
    let mut sections = Vec::with_capacity(column.sections().len());
    for section in column.sections() {
        sections.push(section.export()?.compacted());
    }
    Ok(sections)
}

/// The world as a save carries it: its footprint, and every column in the
/// world's own assembly order.
fn world_record(
    world: &VoxelWorld,
    described: &DescribedWorld,
    table: &NameTable,
) -> Result<WorldRecord, SaveError> {
    let mut columns = Vec::with_capacity(described.len());
    for sections in described {
        columns.push(column_record(sections, table)?);
    }
    Ok(WorldRecord {
        footprint_side: world.footprint_columns(),
        columns,
    })
}

/// One column as a save carries it.
fn column_record(described: &[SectionData], table: &NameTable) -> Result<ColumnRecord, SaveError> {
    let mut sections = Vec::with_capacity(described.len());
    for section in described {
        sections.push(section_record(section, table)?);
    }
    Ok(ColumnRecord { sections })
}

/// One section as a save carries it: its palette pointed at the save's table,
/// and one position per voxel.
fn section_record(described: &SectionData, table: &NameTable) -> Result<SectionRecord, SaveError> {
    let mut palette = Vec::with_capacity(described.palette.len());
    for entry in &described.palette {
        palette.push(palette_entry(entry, table)?);
    }
    Ok(SectionRecord {
        palette,
        indices: described.indices.iter().map(|index| index.get()).collect(),
    })
}

/// What one palette position holds, as the save's table numbers it.
fn palette_entry(entry: &Contents, table: &NameTable) -> Result<PaletteEntry, SaveError> {
    match entry {
        Contents::Empty => Ok(PaletteEntry::Empty),
        Contents::Holds(name) => table
            .id_of(name)
            .map(|id| PaletteEntry::Holds(id.get()))
            .ok_or_else(|| SaveError::UnknownBlock { name: name.clone() }),
    }
}

/// The magic, the format version and the player's place, by hand, ahead of
/// everything the encoder carries.
///
/// By hand because the version has to be readable out of a file this build
/// cannot otherwise read, and the player's place rides along in it so that
/// everything about the file sits ahead of everything about the world.
fn write_preamble(sink: &mut dyn Write, player: SavedPlayer) -> Result<(), SaveError> {
    let mut preamble = Vec::with_capacity(STORED_WORLD_DATA_AT);
    preamble.extend_from_slice(&MAGIC);
    preamble.extend_from_slice(&FORMAT_VERSION.to_le_bytes());
    for coordinate in player.position {
        preamble.extend_from_slice(&coordinate.to_le_bytes());
    }
    preamble.extend_from_slice(&player.yaw.to_le_bytes());
    preamble.extend_from_slice(&player.pitch.to_le_bytes());
    sink.write_all(&preamble)
        .map_err(|failure| unattributed(&failure))
}

#[cfg(test)]
#[path = "write_test.rs"]
mod tests;

/// `value` encoded into `sink`, as one of the save's successive top-level
/// values.
fn encoded_into(sink: &mut dyn Write, value: &impl Serialize) -> Result<(), SaveError> {
    postcard::to_io(value, sink)
        .map(|_sink| ())
        .map_err(|_refused| sink_refused())
}

/// A sink that stopped accepting the save's bytes.
///
/// **The only refusal this can be.** The values being encoded are this module's
/// own records — fixed shapes over types the encoder always accepts — so the one
/// thing left to fail is the thing they are being written into, which is what a
/// disk filling up mid-quit looks like from in here.
///
/// The sink's own [`ErrorKind`] does not survive: the encoder reports that it
/// could not place the bytes and does not carry why. That is the same boundary
/// translation every other library failure gets here, one step blunter — the
/// caller is told the save did not get written, and the file it was going into is
/// attached by [`attributed_to`].
fn sink_refused() -> SaveError {
    SaveError::Io {
        path: PathBuf::new(),
        kind: ErrorKind::Other,
    }
}

/// A failure reported by a sink that has no name of its own.
///
/// [`write_save`] takes something to write into and not a path, so it cannot name
/// what refused its bytes. A caller writing into memory keeps the empty path,
/// which is the honest answer for a sink that is not a file.
fn unattributed(failure: &io::Error) -> SaveError {
    SaveError::Io {
        path: PathBuf::new(),
        kind: failure.kind(),
    }
}

/// The same refusal, attributed to the file it happened to.
///
/// [`replace_atomically`] knows the file it was filling, so it names it — and
/// leaves alone anything that already named something, because a refusal that
/// arrived with a path is about that path and not about this one.
fn attributed_to(beside: &Path, refusal: SaveError) -> SaveError {
    match refusal {
        SaveError::Io { path, kind } if path.as_os_str().is_empty() => SaveError::Io {
            path: beside.to_owned(),
            kind,
        },
        named => named,
    }
}

/// An I/O failure recorded in this module's own vocabulary.
fn io_failure(path: &Path, failure: &io::Error) -> SaveError {
    SaveError::Io {
        path: path.to_owned(),
        kind: failure.kind(),
    }
}
