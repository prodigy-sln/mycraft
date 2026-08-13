//! Asking a save what it needs, and reading back the bytes it stored its world
//! in.
//!
//! **The table is decoded and the reading stops there.** That is the whole
//! reason the table and the world are two successive values rather than one
//! struct: the encoder decodes one value at a time from a reader and leaves it
//! positioned after that value, so answering "what does this save need" costs
//! the table and nothing else. It is what makes "without reading any of its
//! chunk data" a property rather than an intention — a save whose chunk data has
//! been cut away entirely still answers the question completely.
//!
//! And it is what the table is *for*. Resolving it once, up front, lets a load
//! report every missing block before a chunk is touched, instead of failing on
//! whichever section happens to reference a removed mod first.
//!
//! **A table that cannot be read is a refusal and never an empty answer.** An
//! empty set from a corrupt save is indistinguishable from a save that genuinely
//! needs nothing, the same way a reviewer who returned nothing is
//! indistinguishable from one who found nothing.
//!
//! # What is read before what, and why the order is the requirement
//!
//! The preamble is read by hand, in one order, and the order is the point: the
//! magic says whether this is a save at all, the version says whether it is one
//! this build is entitled to interpret, and only then does anything look at the
//! bytes the version governs. A save whose version is unrecognised *and* whose
//! table is unreadable is refused **by version** — what looks like a malformed
//! name might be a perfectly good entry of a format nobody has taught this
//! build, and reporting it would be asserting something about bytes it cannot
//! read.

use std::collections::BTreeSet;
use std::fs::{self, File};
use std::io::{self, BufReader, ErrorKind, Read};
use std::path::Path;

use mc_core::id::BlockName;

use crate::persistence::error::LoadError;
use crate::persistence::format::{
    DefinitionHash, FORMAT_VERSION, MAGIC, MAX_NAME_BYTES, MAX_SAVE_BYTES, PLAYER_AT,
    STORED_WORLD_DATA_AT, SavedPlayer, TableRecord, VERSION_AT,
};

/// How wide each of the preamble's five numbers is.
const NUMBER_BYTES: usize = 4;

/// One block a save needs, and what that block was declared to be when the save
/// was written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequiredBlock {
    pub name: BlockName,
    pub behaviour: DefinitionHash,
    pub appearance: DefinitionHash,
}

/// Every block a save needs, in the order its table holds them.
///
/// The order is the file's and not this build's: a save this build wrote holds
/// its names ascending, and a save some other build wrote may hold them any way
/// at all. Sorting happens where a verdict is reported, which is the one place
/// the order is anybody's to choose.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SaveRequirements {
    blocks: Vec<RequiredBlock>,
}

impl SaveRequirements {
    /// Everything the save needs, and what each of them was declared to be.
    #[must_use]
    pub fn blocks(&self) -> &[RequiredBlock] {
        &self.blocks
    }

    /// Every name the save needs.
    pub fn names(&self) -> impl Iterator<Item = &BlockName> {
        self.blocks.iter().map(|block| &block.name)
    }
}

/// What the save at `path` needs of a registry, read without touching any of its
/// chunk data.
///
/// This is the value a decision about whether to load is made from, and it is
/// answered from the table alone.
///
/// # Errors
///
/// Returns [`LoadError::Missing`] if nothing is at `path`,
/// [`LoadError::Unreadable`] if it exists and cannot be read, and
/// [`LoadError::Malformed`] if its table cannot be made sense of.
pub fn requirements(path: &Path) -> Result<SaveRequirements, LoadError> {
    let mut reader = opened(path)?;
    // The player is read past rather than judged: what a save needs of a
    // registry is a question about its blocks, and a save whose player cannot be
    // stood at still has a complete answer to it.
    let _place = preamble_of(&mut reader, path)?;
    required_of(&mut reader, path)
}

/// What the table `reader` is positioned at says the save needs.
///
/// The header's second half, split out so that a load reads it from the same
/// stream the preamble came out of rather than opening the file a second time.
/// One read path with two entry points, which is what keeps "the table is
/// decoded and the reading stops there" a property of both.
pub(super) fn required_of(
    reader: &mut impl Read,
    path: &Path,
) -> Result<SaveRequirements, LoadError> {
    let table = decoded_table(reader, path)?;
    required_blocks(&table)
}

/// Where the save at `path` records the player, read from its preamble alone.
///
/// # Errors
///
/// Returns [`LoadError::Missing`] if nothing is at `path`,
/// [`LoadError::Unreadable`] if it exists and cannot be read,
/// [`LoadError::NotASave`] or [`LoadError::UnsupportedVersion`] if it is not a
/// save this build reads, and [`LoadError::NotFinite`] if a stored coordinate or
/// angle is not a finite number.
pub fn saved_player(path: &Path) -> Result<SavedPlayer, LoadError> {
    let mut reader = opened(path)?;
    let place = preamble_of(&mut reader, path)?;
    every_number_finite(place)
}

/// The save at `path`, opened and judged small enough to be read at all.
fn opened(path: &Path) -> Result<BufReader<File>, LoadError> {
    Ok(opened_with_length(path)?.0)
}

/// The same, beside how long the file is.
///
/// The length is what a load compares the bytes it consumed against, so that a
/// save carrying anything after its world is refused rather than read out of and
/// shrugged at. Taken here because it has already been asked for: judging the
/// size costs a `metadata` call whether or not anybody keeps the answer.
pub(super) fn opened_with_length(path: &Path) -> Result<(BufReader<File>, u64), LoadError> {
    let file = File::open(path).map_err(|failure| unreachable_file(path, &failure))?;
    let length = within_the_size_a_save_may_be(&file, path)?;
    Ok((BufReader::new(file), length))
}

/// Refuses a file too large to be a save before a byte of it is decoded.
///
/// **This is the only thing turning a bound on bytes read into a bound on
/// memory.** The decoder reads what a value needs and no more, which bounds the
/// reading but says nothing about what those bytes expand into: a file that is
/// small on disk and enormous once decoded is bounded by nothing the decoder
/// does. Asking the filesystem how long the file is costs nothing and is answered
/// before it is read at all.
///
/// Refused as [`LoadError::Malformed`] and not as a variant of its own, because
/// nothing here distinguishes it from any other refusal to make sense of a file —
/// a save this build will not read is a save this build will not read.
fn within_the_size_a_save_may_be(file: &File, path: &Path) -> Result<u64, LoadError> {
    let length = file
        .metadata()
        .map_err(|failure| LoadError::Unreadable {
            path: path.to_owned(),
            kind: failure.kind(),
        })?
        .len();
    if length > MAX_SAVE_BYTES {
        return Err(malformed(path));
    }
    Ok(length)
}

/// The table `reader` is positioned at, and nothing beyond it.
///
/// The scratch buffer is where every name in the table is read, one at a time:
/// the decoder refuses a declared length that will not fit rather than allocating
/// for it, so a file claiming a megabyte-long block name is turned away without
/// a megabyte ever being asked for.
fn decoded_table(reader: &mut impl Read, path: &Path) -> Result<TableRecord, LoadError> {
    let mut scratch = [0_u8; MAX_NAME_BYTES];
    postcard::from_io((reader, scratch.as_mut_slice()))
        .map(|(table, _positioned_after_it)| table)
        .map_err(|_refused| malformed(path))
}

/// The bytes the save at `path` stores its world in — everything about the world
/// and nothing about the file that carries it.
///
/// The comparand for every requirement that two saves hold the same world. It is
/// the stored world data rather than the whole file so that the requirement does
/// not silently decide what a container is allowed to carry of its own.
///
/// # Errors
///
/// Returns [`LoadError::Missing`] if nothing is at `path`,
/// [`LoadError::Unreadable`] if it exists and cannot be read, and
/// [`LoadError::Malformed`] if it is too short to be a save at all.
pub fn stored_world_data(path: &Path) -> Result<Vec<u8>, LoadError> {
    // This is the one place a whole save is read into memory at once, so it is
    // also the one place the file's length has to be judged before it is read
    // rather than after.
    let file = File::open(path).map_err(|failure| unreachable_file(path, &failure))?;
    let _within_bounds = within_the_size_a_save_may_be(&file, path)?;
    let whole = fs::read(path).map_err(|failure| unreachable_file(path, &failure))?;
    whole
        .get(STORED_WORLD_DATA_AT..)
        .map(<[u8]>::to_vec)
        .ok_or_else(|| malformed(path))
}

/// The preamble `reader` begins with: what format it says it is, what version of
/// it, and where it says the player stood.
///
/// Read by hand and in this order, because **a version has to be readable out of
/// a file this build cannot otherwise read**. A version read through the decoder
/// would depend on the decoder making sense of a format nobody taught it, which
/// is exactly the case a version field exists for.
///
/// The player's five numbers are returned as they were stored, unjudged. Whether
/// they can be stood at is a separate question and one only a caller that wants
/// the player has to ask.
pub(super) fn preamble_of(reader: &mut impl Read, path: &Path) -> Result<SavedPlayer, LoadError> {
    let mut preamble = [0_u8; STORED_WORLD_DATA_AT];
    let filled = filled_from(reader, &mut preamble).map_err(|failure| LoadError::Unreadable {
        path: path.to_owned(),
        kind: failure.kind(),
    })?;
    if filled < STORED_WORLD_DATA_AT {
        return Err(not_a_save(preamble.get(..filled).unwrap_or_default()));
    }
    this_format(&preamble)?;
    this_version(&preamble)?;
    Ok(player_in(&preamble))
}

/// As much of `buffer` as `reader` had to give, and how much that was.
///
/// Read in a loop rather than with a single exact read, because a file too short
/// to hold a preamble has to be refused **carrying the bytes it did hold** — and
/// an exact read that comes up short leaves the buffer's contents unspecified,
/// which is the one thing this refusal cannot afford.
fn filled_from(reader: &mut impl Read, buffer: &mut [u8]) -> io::Result<usize> {
    let mut filled = 0;
    while filled < buffer.len() {
        let taken = reader.read(buffer.get_mut(filled..).unwrap_or_default())?;
        if taken == 0 {
            break;
        }
        filled += taken;
    }
    Ok(filled)
}

/// Refuses `preamble` unless it begins the way a save does.
fn this_format(preamble: &[u8; STORED_WORLD_DATA_AT]) -> Result<(), LoadError> {
    let leading = preamble.get(..MAGIC.len()).unwrap_or_default();
    if leading == MAGIC {
        return Ok(());
    }
    Err(not_a_save(leading))
}

/// Refuses `preamble` unless it declares a version this build reads.
///
/// Read before anything it governs, and refused before anything it governs is
/// looked at.
fn this_version(preamble: &[u8; STORED_WORLD_DATA_AT]) -> Result<(), LoadError> {
    let found = preamble
        .get(VERSION_AT..VERSION_AT + 2)
        .and_then(|declared| <[u8; 2]>::try_from(declared).ok())
        .map(u16::from_le_bytes)
        .unwrap_or_default();
    if found == FORMAT_VERSION {
        return Ok(());
    }
    Err(LoadError::UnsupportedVersion {
        found,
        supported: FORMAT_VERSION,
    })
}

/// Where `preamble` says the player stood, exactly as it stored them.
fn player_in(preamble: &[u8; STORED_WORLD_DATA_AT]) -> SavedPlayer {
    SavedPlayer {
        position: [
            number_at(preamble, PLAYER_AT),
            number_at(preamble, PLAYER_AT + NUMBER_BYTES),
            number_at(preamble, PLAYER_AT + 2 * NUMBER_BYTES),
        ],
        yaw: number_at(preamble, PLAYER_AT + 3 * NUMBER_BYTES),
        pitch: number_at(preamble, PLAYER_AT + 4 * NUMBER_BYTES),
    }
}

/// The four-byte number `preamble` holds at `offset`.
///
/// Every offset this is called with is inside a fixed-length array, so the
/// fallback is unreachable; it is written as a fallback rather than an index
/// because reading a save may not end the process.
fn number_at(preamble: &[u8; STORED_WORLD_DATA_AT], offset: usize) -> f32 {
    preamble
        .get(offset..offset + NUMBER_BYTES)
        .and_then(|stored| <[u8; NUMBER_BYTES]>::try_from(stored).ok())
        .map(f32::from_le_bytes)
        .unwrap_or_default()
}

/// `player` unchanged, or the first of its five numbers that cannot be stood at.
///
/// Refused here because this is the last place such a value is still one number
/// with a name: carried into a simulation it reaches a velocity, a collision box
/// and a camera before anything looks wrong on screen. **Only a value that is not
/// finite is refused** — a finite position outside the world's footprint is a
/// save a player produces by walking off the edge, and the player is not confined
/// to the world.
pub(super) fn every_number_finite(player: SavedPlayer) -> Result<SavedPlayer, LoadError> {
    let [x, y, z] = player.position;
    let stored = [
        ("x", x),
        ("y", y),
        ("z", z),
        ("yaw", player.yaw),
        ("pitch", player.pitch),
    ];
    for (axis, value) in stored {
        if !value.is_finite() {
            return Err(LoadError::NotFinite { axis, value });
        }
    }
    Ok(player)
}

/// A file that does not begin the way a save does, carrying what it began with.
fn not_a_save(leading: &[u8]) -> LoadError {
    LoadError::NotASave {
        found: leading.to_vec(),
    }
}

/// What a decoded table says the save needs.
///
/// The names are parsed here, at this module's edge, which is where every value
/// the encoder handed back stops being a plain string and becomes something this
/// crate is willing to reason about. Both refusals below are ours rather than the
/// decoder's: a table naming the same block twice decodes perfectly well, and so
/// does one holding the word "andesite".
///
/// **The order the table holds its names in is accepted as it is found.** This
/// build's writer sorts, but a save is a file — written by another build, an
/// older one, or a tool nobody here has seen — and a reader that required
/// ascending order would refuse files that are perfectly readable. The report is
/// sorted where it is reported, which is the one place the order is anybody's to
/// choose.
fn required_blocks(table: &TableRecord) -> Result<SaveRequirements, LoadError> {
    let mut named_already = BTreeSet::new();
    let mut blocks = Vec::with_capacity(table.names.len());
    for entry in &table.names {
        let name = BlockName::parse(&entry.name).map_err(|source| LoadError::MalformedName {
            text: entry.name.clone(),
            source,
        })?;
        if !named_already.insert(name.clone()) {
            return Err(LoadError::DuplicateName { name });
        }
        blocks.push(RequiredBlock {
            name,
            behaviour: DefinitionHash::from_raw(entry.behaviour),
            appearance: DefinitionHash::from_raw(entry.appearance),
        });
    }
    Ok(SaveRequirements { blocks })
}

/// Why a save could not be opened at all.
///
/// **A path nothing is at is a different answer from a path that cannot be
/// read**, and it stays different all the way up: a launch decides whether to
/// generate a world on exactly this distinction.
fn unreachable_file(path: &Path, failure: &std::io::Error) -> LoadError {
    match failure.kind() {
        ErrorKind::NotFound => LoadError::Missing {
            path: path.to_owned(),
        },
        kind => LoadError::Unreadable {
            path: path.to_owned(),
            kind,
        },
    }
}

/// The one refusal every way of failing to make sense of a save's bytes
/// collapses into.
pub(super) fn malformed(path: &Path) -> LoadError {
    LoadError::Malformed {
        path: path.to_owned(),
    }
}
