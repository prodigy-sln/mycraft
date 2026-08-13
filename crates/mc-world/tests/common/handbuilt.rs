//! Saves built byte by byte, for the shapes the writer cannot produce.
//!
//! The writer sorts its table, records each distinct name once, and only ever
//! writes names that were already namespaced ids. So a save naming the same
//! block twice, a save whose table holds text that is not a name, and two saves
//! naming the same three blocks in two different orders are all shapes no
//! fixture can reach through it — and every one of them is a shape a file on a
//! player's disk can have. They are written out here instead.
//!
//! **The layout is spelled out here rather than read back from the crate.** A
//! fixture that reads the constant it asserts against asserts nothing: it would
//! agree with a reader that moved the version field, and with one that changed
//! the magic, exactly as readily as with a correct one.
//!
//! ```text
//! offset  field                 encoding
//!      0  magic                 [u8; 8] = b"MYCRAFT\x1A"
//!      8  format version        u16 LE = 1
//!     10  player position       3 x f32 LE  (x, y, z)
//!     22  player yaw            f32 LE, radians
//!     26  player pitch          f32 LE, radians
//!     30  --- the stored world data begins: the table, then the world ---
//! ```
//!
//! The table is a length-prefixed sequence: a variable-length count, then per
//! entry a variable-length byte count followed by the name's UTF-8, then the
//! two recorded declarations as variable-length integers. A variable-length
//! integer is seven bits of the value per byte, lowest group first, with the
//! high bit set on every byte but the last — so every value under 128 is a
//! single byte and the fixtures below stay readable as hexadecimal.
//!
//! **A world record follows the table only where a fixture says so.** Everything
//! answered from the table alone is asked of a save with no chunk data behind it
//! at all, which is the sharpest available statement of that property: a reader
//! that reached the names by way of the world could not answer at all. The
//! fixtures that *are* about the world declare one, in the same spelled-out
//! layout — the footprint's side, then the columns, each a list of sections,
//! each a palette of entries beside one position into it per voxel. A palette
//! entry is a tag byte, `00` for nothing and `01` for a name the table carries,
//! and the name's table position follows the `01` as a variable-length integer.
//!
//! These are the shapes a writer cannot produce either: it writes as many
//! columns as the footprint has, four thousand and ninety-six positions per
//! section, and never a table position it has not just written into the table.

use std::error::Error;
use std::fs;
use std::path::PathBuf;

use mc_core::block::BlockRegistry;
use mc_world::persistence::SavedPlayer;
use tempfile::TempDir;

use super::persistence::{STANDING_SOMEWHERE, save_in, world_at, world_holding};

/// The eight bytes a save of this format begins with.
pub const MAGIC: [u8; 8] = *b"MYCRAFT\x1A";

/// The version of this format that this build writes and reads.
pub const FORMAT_VERSION: u16 = 1;

/// Where a save's declared version sits, and how wide it is.
pub const VERSION_AT: usize = MAGIC.len();
pub const VERSION_BYTES: usize = 2;

/// How long a save's preamble is: the magic, the version, and the five numbers
/// that say where the player stood and which way they faced.
pub const PREAMBLE_BYTES: usize = VERSION_AT + VERSION_BYTES + 5 * 4;

/// One entry of a hand-built table: a name, the behaviour recorded against it,
/// and the appearance recorded against it.
pub type Entry<'a> = (&'a str, u64, u64);

/// What one palette position of a hand-built section holds: nothing, or the
/// name the save's table carries at that position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cell {
    Empty,
    Holds(u32),
}

/// One section of a hand-built world: what its palette holds, and one position
/// into that palette per voxel.
#[derive(Debug, Clone, Copy)]
pub struct Stored<'a> {
    pub palette: &'a [Cell],
    pub indices: &'a [u16],
}

/// One column of a hand-built world, its sections bottom-up.
#[derive(Debug, Clone, Copy)]
pub struct Column<'a> {
    pub sections: &'a [Stored<'a>],
}

/// The world a hand-built save carries after its table: how many columns it
/// spans a side, and every column it holds.
#[derive(Debug, Clone, Copy)]
pub struct World<'a> {
    pub footprint_side: u32,
    pub columns: &'a [Column<'a>],
}

/// How many sections a column stacks, and how many voxels a section holds.
///
/// Spelled out here rather than read from the crate, for the reason the whole
/// layout is: a fixture that reads the number it asserts against would agree
/// with a build that changed it.
pub const SECTIONS_PER_COLUMN: usize = 16;
pub const VOXELS_PER_SECTION: usize = 16 * 16 * 16;

/// One position per voxel, every one of them the first palette entry.
pub const ALL_AT_THE_FIRST_ENTRY: [u16; VOXELS_PER_SECTION] = [0; VOXELS_PER_SECTION];

/// A section holding nothing at all: one palette entry, which is nothing, and
/// every voxel naming it.
pub const HOLDING_NOTHING: Stored<'static> = Stored {
    palette: &[Cell::Empty],
    indices: &ALL_AT_THE_FIRST_ENTRY,
};

/// A column of sixteen sections holding nothing at all.
pub const EMPTY_COLUMN: Column<'static> = Column {
    sections: &[HOLDING_NOTHING; SECTIONS_PER_COLUMN],
};

/// A save written out by hand: what version it declares, where it says the
/// player was, the table it carries, and the world behind it if it has one.
#[derive(Debug, Clone, Copy)]
pub struct HandBuilt<'a> {
    pub version: u16,
    pub player: SavedPlayer,
    pub table: &'a [Entry<'a>],
    pub world: Option<World<'a>>,
}

impl Default for HandBuilt<'_> {
    /// A save this build recognises, recording a player standing somewhere
    /// unremarkable and needing no block at all.
    ///
    /// Every fixture below states the one thing it is about and inherits the
    /// rest from here, so that a save refused by a test is refused for the
    /// reason that test names and not for a second thing it forgot to set.
    fn default() -> Self {
        Self {
            version: FORMAT_VERSION,
            player: STANDING_SOMEWHERE,
            table: &[],
            world: None,
        }
    }
}

/// Every byte `save` is made of.
#[must_use]
pub fn bytes_of(save: HandBuilt<'_>) -> Vec<u8> {
    let mut written = preamble(save.version, save.player);
    written.extend(encoded_table(save.table));
    if let Some(world) = save.world {
        written.extend(encoded_world(world));
    }
    written
}

/// Sixteen sections holding nothing, with `odd` in place of the one stacked at
/// `height`.
///
/// What every fixture about a stored section is: a column that is entirely
/// ordinary apart from the one thing the fixture is about, so that a refusal it
/// earns is the refusal that fixture names and not a second one it forgot.
#[must_use]
pub fn column_of(odd: Stored<'_>, height: usize) -> Vec<Stored<'_>> {
    let mut sections = vec![HOLDING_NOTHING; SECTIONS_PER_COLUMN];
    if let Some(stacked) = sections.get_mut(height) {
        *stacked = odd;
    }
    sections
}

/// `save` written into `directory` as `file_name`, and where it landed.
///
/// # Errors
///
/// Returns an error if the file cannot be written.
pub fn written(
    directory: &TempDir,
    file_name: &str,
    save: HandBuilt<'_>,
) -> Result<PathBuf, Box<dyn Error>> {
    file_holding(directory, file_name, &bytes_of(save))
}

/// A file in `directory` holding exactly `bytes`, and where it landed.
///
/// # Errors
///
/// Returns an error if the file cannot be written.
pub fn file_holding(
    directory: &TempDir,
    file_name: &str,
    bytes: &[u8],
) -> Result<PathBuf, Box<dyn Error>> {
    let path = directory.path().join(file_name);
    fs::write(&path, bytes)?;
    Ok(path)
}

/// What a save the writer produced records for `name` against `registry` — its
/// declared behaviour, then its declared appearance.
///
/// **The oracle a "changed" fixture needs.** A block counts as changed when the
/// declaration stored against it is not the one this registry would record, so a
/// fixture claiming to store a changed declaration has to know what the
/// unchanged one is. A constant picked out of the air could not be *known* to
/// differ from it; this is read back through the writer, and the fixtures move
/// one bit off it.
///
/// # Errors
///
/// Returns an error if the save cannot be written, or if it does not report the
/// one name it was built from.
pub fn recorded_for(name: &str, registry: &BlockRegistry) -> Result<(u64, u64), Box<dyn Error>> {
    let directory = TempDir::new()?;
    let world = world_holding(&[(world_at(1, 1, 1), name)], registry)?;
    let path = save_in(&directory);
    mc_world::persistence::save_world(&path, &world, STANDING_SOMEWHERE, registry)?;
    mc_world::persistence::requirements(&path)?
        .blocks()
        .iter()
        .find(|block| block.name.as_str() == name)
        .map(|block| (block.behaviour.get(), block.appearance.get()))
        .ok_or_else(|| format!("a save of a world holding `{name}` did not report it").into())
}

/// The declarations a registry declaring `names` records for each of them, with
/// every recorded behaviour moved one bit away from it.
///
/// One bit, because it is the smallest change that is *certainly* a change: a
/// value chosen freely could be the real one, with a probability nobody wants to
/// reason about in a fixture.
///
/// # Errors
///
/// Returns an error if a save of one of the names cannot be written or read.
pub fn recorded_as_changed<'a>(
    names: &[&'a str],
    registry: &BlockRegistry,
) -> Result<Vec<Entry<'a>>, Box<dyn Error>> {
    let mut entries = Vec::with_capacity(names.len());
    for &name in names {
        let (behaviour, appearance) = recorded_for(name, registry)?;
        entries.push((name, behaviour ^ 1, appearance));
    }
    Ok(entries)
}

/// The thirty bytes every save begins with.
fn preamble(version: u16, player: SavedPlayer) -> Vec<u8> {
    let mut written = Vec::with_capacity(PREAMBLE_BYTES);
    written.extend_from_slice(&MAGIC);
    written.extend_from_slice(&version.to_le_bytes());
    for coordinate in player.position {
        written.extend_from_slice(&coordinate.to_le_bytes());
    }
    written.extend_from_slice(&player.yaw.to_le_bytes());
    written.extend_from_slice(&player.pitch.to_le_bytes());
    written
}

/// `entries` as the stored table: how many there are, then each of them.
fn encoded_table(entries: &[Entry<'_>]) -> Vec<u8> {
    let mut written = variable_length(counted(entries.len()));
    for &(name, behaviour, appearance) in entries {
        written.extend(variable_length(counted(name.len())));
        written.extend_from_slice(name.as_bytes());
        written.extend(variable_length(behaviour));
        written.extend(variable_length(appearance));
    }
    written
}

/// `world` as the stored world: its footprint's side, then every column.
fn encoded_world(world: World<'_>) -> Vec<u8> {
    let mut written = variable_length(u64::from(world.footprint_side));
    written.extend(variable_length(counted(world.columns.len())));
    for column in world.columns {
        written.extend(encoded_column(*column));
    }
    written
}

/// One column: how many sections it stacks, then each of them bottom-up.
fn encoded_column(column: Column<'_>) -> Vec<u8> {
    let mut written = variable_length(counted(column.sections.len()));
    for section in column.sections {
        written.extend(encoded_section(*section));
    }
    written
}

/// One section: its palette, then one position into that palette per voxel.
fn encoded_section(section: Stored<'_>) -> Vec<u8> {
    let mut written = variable_length(counted(section.palette.len()));
    for cell in section.palette {
        written.extend(encoded_cell(*cell));
    }
    written.extend(variable_length(counted(section.indices.len())));
    for index in section.indices {
        written.extend(variable_length(u64::from(*index)));
    }
    written
}

/// One palette position: a tag, and the table position where there is one.
fn encoded_cell(cell: Cell) -> Vec<u8> {
    match cell {
        Cell::Empty => vec![0],
        Cell::Holds(name) => {
            let mut written = vec![1];
            written.extend(variable_length(u64::from(name)));
            written
        }
    }
}

/// `value` as the format's variable-length integer.
///
/// Seven bits per byte, lowest group first, the high bit set on every byte but
/// the last. The mask is what makes the narrowing total: seven bits always fit
/// in a byte, so the fallback below is unreachable and is written as a fallback
/// only because a fixture that could end the process is worse than one that
/// writes a zero.
fn variable_length(value: u64) -> Vec<u8> {
    let mut written = Vec::new();
    let mut remaining = value;
    loop {
        let group = u8::try_from(remaining & 0x7f).unwrap_or_default();
        remaining >>= 7;
        if remaining == 0 {
            written.push(group);
            return written;
        }
        written.push(group | 0x80);
    }
}

/// A count, widened to what a variable-length integer carries.
fn counted(length: usize) -> u64 {
    u64::try_from(length).unwrap_or_default()
}
