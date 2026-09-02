//! What a save is made of: the preamble read by hand, the records the encoder
//! carries, and what a block was declared to be when the save was written.
//!
//! A save is a **fixed preamble followed by two successive encoded values** — a
//! table of names, then the world.
//!
//! ```text
//! offset  field                 encoding
//!      0  magic                 [u8; 8] = b"MYCRAFT\x1A"     read by hand
//!      8  format version        u16 LE  = 1                   read by hand
//!     10  player position       3 × f32 LE  (x, y, z)         read by hand
//!     22  player yaw            f32 LE, radians               read by hand
//!     26  player pitch          f32 LE, radians               read by hand
//!     30  ─── stored world data begins ───
//!     30  encoded TableRecord
//!      …  encoded WorldRecord
//!         ─── end of file, exactly ───
//! ```
//!
//! **The player's place sits in the preamble and not in an encoded record**, so
//! that it is at a fixed offset and [`STORED_WORLD_DATA_AT`] stays a clean
//! suffix: the bytes that are about the world begin where the bytes that are
//! about the file stop, and every requirement that two saves hold the same world
//! compares that suffix. Nothing anywhere holds committed save bytes — every
//! byte-identity requirement compares two saves to each other, never a save to a
//! golden.
//!
//! **The preamble is read by hand and it is the only part that is.** The version
//! has to be readable before anything it governs, and a version read *through*
//! the encoder would depend on the encoder being able to decode a file whose
//! format this build does not recognise. Fixed width at a fixed offset for the
//! same reason: a variable-width version would make reading the version of an
//! unreadable file itself version-dependent.
//!
//! **The table and the world are two top-level values and not one struct.** The
//! encoder decodes one value at a time from a reader and leaves it positioned
//! after that value, so asking a save what it needs decodes the table and stops
//! there. One wrapping struct would decode the world too, and "without reading
//! any of its chunk data" would stop being true.

use serde::{Deserialize, Serialize};

/// The eight bytes every save begins with.
///
/// Ends in a byte no text encoding leaves alone, which is the trick PNG uses: a
/// save mangled by a text-mode transfer fails the format check rather than
/// parsing as something else.
pub(crate) const MAGIC: [u8; 8] = *b"MYCRAFT\x1A";

/// The version of this format that this build writes.
///
/// Numbered 1 and not 0, so that a zero-filled buffer which somehow got past the
/// magic declares a version this build does not recognise rather than declaring
/// the one it supports.
pub(crate) const FORMAT_VERSION: u16 = 1;

/// Where a save's declared version sits, and how wide it is.
///
/// Fixed width at a fixed offset, because a variable-width version would make
/// reading the version of a file this build cannot read itself depend on the
/// version.
pub(crate) const VERSION_AT: usize = MAGIC.len();
const VERSION_BYTES: usize = 2;

/// Where the player's place begins, and how many four-byte numbers it is: three
/// coordinates, then yaw, then pitch.
pub(crate) const PLAYER_AT: usize = VERSION_AT + VERSION_BYTES;
const PLAYER_NUMBERS: usize = 5;
const NUMBER_BYTES: usize = 4;

/// Where a save's stored world data begins — everything about the world, and
/// nothing about the file that carries it.
pub(crate) const STORED_WORLD_DATA_AT: usize = PLAYER_AT + PLAYER_NUMBERS * NUMBER_BYTES;

/// Where the player stood and which way they faced.
///
/// **Plain numbers and not `mc-sim`'s `PlayerState`**: that type belongs to the
/// simulation and the dependency runs the other way. Velocity is deliberately
/// absent — a resumed player is at rest, because restoring a mid-fall velocity
/// would resume the game by dropping them.
///
/// `PartialEq` and not `Eq`, which is what an `f32` allows.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SavedPlayer {
    /// Where they stood, in world coordinates.
    pub position: [f32; 3],
    /// Which way they faced about the vertical axis, in radians.
    pub yaw: f32,
    /// How far up or down they looked, in radians.
    pub pitch: f32,
}

/// The largest save this build will read.
///
/// **Derived from a memory ceiling and not from an intuition about file sizes**,
/// because it is the only thing converting one into the other. The encoder
/// bounds the *bytes it reads*, never the memory those bytes expand into, so a
/// file that is small on disk and enormous in memory has to be refused before it
/// is opened at all — by its length, from the filesystem, ahead of a single byte
/// being decoded.
///
/// The arithmetic: a column of empty sections encodes to a few bytes and occupies
/// roughly 24 times that in memory, and vector growth doubles the peak, so
/// worst-case amplification is about 48×. Sixteen mebibytes of hostile file is
/// therefore about 768 MiB of peak memory. A legitimate save of MVP 1's world is
/// around two megabytes, and the largest world any requirement asks for is under
/// four, so this is four times the largest legitimate save and eight times a
/// typical one.
///
/// **A runtime precheck and not a stored field**, so revising it later costs a
/// build and nothing on disk.
pub(crate) const MAX_SAVE_BYTES: u64 = 16 * 1024 * 1024;

/// The longest single block name a save may hold, in bytes.
///
/// The scratch buffer the decoder is given for byte-shaped fields: it reads each
/// one into this buffer and refuses outright when the declared length will not
/// fit, so nothing is allocated for a length a file merely claims. The bound is
/// per field rather than cumulative, because a name is decoded into an owned
/// `String` — a borrowed `&str` would consume the buffer instead of reusing it,
/// which is a reason not to introduce one here.
///
/// A namespaced id imposes no length of its own, so without this the only bound
/// on a stored name would be the size of the file. 256 bytes is far above any
/// real `namespace:path`.
pub(crate) const MAX_NAME_BYTES: usize = 256;

/// Where a block's declaration sits in a save's own name table.
///
/// **Sized independently of a section's palette position, and that is the
/// point.** A palette position is bounded by a compacted section's 4096 entries;
/// a save-wide table is bounded by the distinct names across the whole save,
/// which is a different bound entirely. A type of its own for the same reason a
/// palette position is one: all three are small numbers standing for a block,
/// and mistaking one for another is how a world starts reporting whatever
/// something else happened to number the same.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SaveNameId(u32);

impl SaveNameId {
    /// The table position `position`.
    #[must_use]
    pub const fn new(position: u32) -> Self {
        Self(position)
    }

    /// Which table position this is.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

/// What a block was declared to be, re-exported from the module that answers it.
///
/// The folds moved to [`super::declaration`] when this file outgrew the size
/// `code-quality.md` allows one, and they are named through here because that is
/// what the rest of `persistence` already asks about a save's records — the file
/// layout and what a record says a block was are one question to a caller and two
/// to whoever maintains them.
pub(crate) use super::declaration::{appearance_of, behaviour_of};

/// A save's table of names, and what each of those blocks was declared to be.
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct TableRecord {
    pub names: Vec<NameEntry>,
}

/// One name a save needs, addressed by its position in the table.
///
/// The name is a plain string and not a [`BlockName`], so the file format is not
/// welded to a domain type that changes for reasons having nothing to do with
/// storage. It is parsed at this module's edge.
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct NameEntry {
    pub name: String,
    pub behaviour: u64,
    pub appearance: u64,
}

/// A save's world: how many columns it spans a side, and every one of them.
///
/// The **side** rather than the count, mirroring how a world is assembled, and
/// the square of it is what a decoded column list is checked against.
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct WorldRecord {
    pub footprint_side: u32,
    pub columns: Vec<ColumnRecord>,
}

/// One column, its sections bottom-up.
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct ColumnRecord {
    pub sections: Vec<SectionRecord>,
}

/// One section, in the shape a section already describes itself in: a palette,
/// and one position into it per voxel.
///
/// Inherited rather than invented, so there is nothing to migrate — and it is
/// the *palette entries* that carry save-table identifiers, not the voxels. Two
/// levels rather than one costs half the bytes per section and keeps a voxel
/// naming a position its palette lacks distinct from a palette entry naming a
/// name the table lacks; one level collapses those into a single failure.
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct SectionRecord {
    pub palette: Vec<PaletteEntry>,
    pub indices: Vec<u16>,
}

/// What one palette position holds — a name the save's table carries, or
/// nothing.
///
/// **Emptiness is never a table entry.** Nothing names nothing: a palette entry
/// distinguishes "holds nothing" from "holds the table's *n*th name" without
/// reserving a name for it. Reserving one would give emptiness a name at the one
/// place a stored format makes it permanent, and would put it in every
/// missing-block report ever written.
#[derive(Debug, Serialize, Deserialize)]
pub(crate) enum PaletteEntry {
    Empty,
    Holds(u32),
}

/// What a block was declared to be, folded into 64 bits.
///
/// Compared once per distinct name against a *specific* other version of the
/// same block, which is what makes 64 non-cryptographic bits enough: an
/// accidental collision would be one changed block loading unremarked on, not a
/// corrupted world. These detect change and not tampering — a local save
/// can be edited directly, so a player forging their own record has easier ways
/// to change their world.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DefinitionHash(u64);

impl DefinitionHash {
    /// The hash whose value is `raw`.
    #[must_use]
    pub const fn from_raw(raw: u64) -> Self {
        Self(raw)
    }

    /// The hash's value.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

#[cfg(test)]
#[path = "format_test.rs"]
mod tests;
