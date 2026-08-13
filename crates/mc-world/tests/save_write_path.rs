//! Where a save is allowed to be written, and what a half-finished write is not
//! allowed to destroy.
//!
//! Every refusal here names the thing that was wrong, and one of them names
//! something no filesystem call will name for us: when a *component* of the save
//! path's parent is a file rather than a directory, `create_dir_all` reports
//! that something was not a directory without ever saying which something. A
//! player whose `saves` is a stray file learns nothing from that, so the walk is
//! ours and the component is quoted back.
//!
//! The atomicity pair is the important one. A save that replaces its predecessor
//! by writing over it in place has, for the length of the write, neither the old
//! world nor the new one — and a crash in that window costs a player everything
//! they built. So a save is written beside its target and renamed over it, which
//! is atomic only while the two sit on one volume: hence *sibling*, never the
//! system temporary directory. **The leftover-file assertion cannot see that on
//! its own** — an in-place overwrite leaves exactly one entry too — which is why
//! the interrupted write is here beside it. That one an in-place writer fails.

mod common;

use std::error::Error;
use std::fmt;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use common::persistence::{
    STANDING_SOMEWHERE, required_names, save_in, world_at, world_holding, written_bytes,
};
use common::{TestResult, registry_of};
use mc_core::block::BlockRegistry;
use mc_core::id::BlockName;
use mc_world::persistence::{self, SaveError};
use mc_world::world::{VoxelWorld, WorldPos};
use tempfile::TempDir;

/// The blocks the fixture worlds are made of.
const BLOCKS: [&str; 4] = [
    "fixture:andesite",
    "fixture:basalt",
    "fixture:chert",
    "fixture:diorite",
];

/// Where each of [`BLOCKS`] sits in the world that holds them all.
const CELLS: [WorldPos; 4] = [
    world_at(1, 1, 1),
    world_at(2, 3, 4),
    world_at(5, 8, 13),
    world_at(15, 200, 15),
];

/// The one cell the replacement world differs from the previous one in, and what
/// is put there.
const AN_EDITED_CELL: WorldPos = world_at(7, 7, 7);
const A_PLACED_BLOCK: &str = "fixture:andesite";

/// How many bytes of the interrupted save the sink accepts before it fails.
///
/// The scenario asks for the window after the block names are written and before
/// the chunk data is complete, so this is derived from the format rather than
/// sampled from a run. The preamble is thirty bytes — a magic, a version, and
/// the five numbers saying where the player stood; the table is one length
/// followed by four entries, each a length, a name of under twenty bytes and two
/// recorded declarations — under 200 bytes in total, five times short of this.
/// The far end is not left to arithmetic at all: the assertion carries it as
/// something it observes.
const ACCEPTED_BEFORE_THE_SINK_FAILS: usize = 1024;

/// A sink that takes a fixed number of bytes and then stops taking any.
///
/// The one honest way to interrupt a save from outside it: the writer is handed
/// something that behaves exactly like a disk filling up part-way through.
struct LimitedWriter<'a> {
    sink: &'a mut dyn Write,
    remaining: usize,
}

impl<'a> LimitedWriter<'a> {
    fn new(sink: &'a mut dyn Write, accepted: usize) -> Self {
        Self {
            sink,
            remaining: accepted,
        }
    }
}

impl fmt::Debug for LimitedWriter<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LimitedWriter")
            .field("remaining", &self.remaining)
            .finish_non_exhaustive()
    }
}

impl Write for LimitedWriter<'_> {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        if self.remaining == 0 {
            return Err(io::Error::from(io::ErrorKind::BrokenPipe));
        }
        let offered = buffer
            .get(..self.remaining.min(buffer.len()))
            .unwrap_or(buffer);
        let accepted = self.sink.write(offered)?;
        self.remaining -= accepted.min(self.remaining);
        Ok(accepted)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.sink.flush()
    }
}

/// Each of [`BLOCKS`] at its cell.
fn every_block_placed() -> Vec<(WorldPos, &'static str)> {
    CELLS.into_iter().zip(BLOCKS).collect()
}

/// A save already on disk, the world meant to replace it, and every byte that
/// replacement would be made of.
#[derive(Debug)]
struct AlreadyThere {
    previous_bytes: Vec<u8>,
    replacement: VoxelWorld,
    whole_replacement: Vec<u8>,
}

/// A save written at `path`, and a world one cell away from the one it holds.
///
/// One cell, because the replacement has to be a world the writer would produce
/// different bytes for — otherwise an interrupted write leaving the file
/// unchanged would prove nothing about the write and everything about the two
/// worlds being the same.
fn a_save_and_the_world_meant_to_replace_it(
    path: &Path,
    registry: &BlockRegistry,
) -> Result<AlreadyThere, Box<dyn Error>> {
    let previous = world_holding(&every_block_placed(), registry)?;
    persistence::save_world(path, &previous, STANDING_SOMEWHERE, registry)?;
    let mut replacement = previous.clone();
    replacement.set_block(AN_EDITED_CELL, &BlockName::parse(A_PLACED_BLOCK)?, registry)?;
    let whole_replacement = written_bytes(&replacement, registry)?;
    Ok(AlreadyThere {
        previous_bytes: fs::read(path)?,
        replacement,
        whole_replacement,
    })
}

/// Every name the save at `path` still says it needs, and none at all where it
/// can no longer answer.
fn names_the_save_still_reports(path: &Path) -> Vec<String> {
    match persistence::requirements(path) {
        Ok(required) => required_names(&required),
        Err(_) => Vec::new(),
    }
}

/// How many entries `directory` holds.
fn entries_in(directory: &TempDir) -> Result<usize, Box<dyn Error>> {
    let mut counted = 0;
    for entry in fs::read_dir(directory.path())? {
        entry?;
        counted += 1;
    }
    Ok(counted)
}

#[test]
fn saving_beneath_a_directory_that_does_not_exist_creates_it_and_writes_the_file() -> TestResult {
    let directory = TempDir::new()?;
    let registry = registry_of(&BLOCKS)?;
    let world = world_holding(&every_block_placed(), &registry)?;
    let path = directory
        .path()
        .join("saves")
        .join("slot_one")
        .join("world.mcw");

    let saved = persistence::save_world(&path, &world, STANDING_SOMEWHERE, &registry);

    assert_eq!(
        (saved, path.is_file()),
        (Ok(()), true),
        "a player's first save has no directory waiting for it, so the two missing levels of \
         `saves/slot_one` are the save path's to create — refusing a world because the folder \
         it goes in has never been made would make the very first quit the one that loses \
         everything"
    );
    Ok(())
}

#[test]
fn saving_onto_a_path_that_is_already_a_directory_is_refused_naming_that_path() -> TestResult {
    let directory = TempDir::new()?;
    let registry = registry_of(&BLOCKS)?;
    let world = world_holding(&every_block_placed(), &registry)?;
    let occupied = save_in(&directory);
    fs::create_dir(&occupied)?;

    let saved = persistence::save_world(&occupied, &world, STANDING_SOMEWHERE, &registry);

    assert_eq!(
        saved,
        Err(SaveError::PathIsDirectory {
            path: occupied.clone()
        }),
        "a directory sitting where the save file goes is a mistake only the person who made it \
         can undo, and they can only undo it if they are told which path is occupied — a bare \
         write failure leaves them guessing at a file they cannot see"
    );
    Ok(())
}

#[test]
fn saving_beneath_a_component_that_is_a_file_is_refused_naming_that_component() -> TestResult {
    let directory = TempDir::new()?;
    let registry = registry_of(&BLOCKS)?;
    let world = world_holding(&every_block_placed(), &registry)?;
    let blocking_file = directory.path().join("saves");
    fs::write(&blocking_file, b"not a directory")?;
    let beneath_it: PathBuf = blocking_file.join("slot_one").join("world.mcw");

    let saved = persistence::save_world(&beneath_it, &world, STANDING_SOMEWHERE, &registry);

    assert_eq!(
        saved,
        Err(SaveError::NotADirectory {
            component: blocking_file.clone()
        }),
        "the refusal has to name `saves` itself, not the save file two levels below it and not \
         the whole path: the component that is a file is the only thing a player can act on, \
         and it is precisely what creating the directories reports without saying"
    );
    Ok(())
}

#[test]
fn saving_over_a_previous_save_leaves_one_entry_in_the_directory_it_lives_in() -> TestResult {
    let directory = TempDir::new()?;
    let registry = registry_of(&BLOCKS)?;
    let world = world_holding(&every_block_placed(), &registry)?;
    let path = save_in(&directory);
    persistence::save_world(&path, &world, STANDING_SOMEWHERE, &registry)?;

    persistence::save_world(&path, &world, STANDING_SOMEWHERE, &registry)?;

    assert_eq!(
        entries_in(&directory)?,
        1,
        "a save is completed beside its target and renamed over it, and the rename is what \
         makes the replacement atomic — so once the save has finished the only thing left in \
         the directory is the save. A half-written sibling that outlives the write is a file a \
         player will one day find and wonder about, and on the next launch it is indexed as \
         though it meant something"
    );
    Ok(())
}

#[test]
fn a_write_that_stops_after_the_block_names_leaves_the_previous_save_untouched() -> TestResult {
    let directory = TempDir::new()?;
    let registry = registry_of(&BLOCKS)?;
    let path = save_in(&directory);
    let already_there = a_save_and_the_world_meant_to_replace_it(&path, &registry)?;

    let interrupted = persistence::replace_atomically(&path, |sink| {
        let mut limited = LimitedWriter::new(sink, ACCEPTED_BEFORE_THE_SINK_FAILS);
        persistence::write_save(
            &mut limited,
            &already_there.replacement,
            STANDING_SOMEWHERE,
            &registry,
        )
    });

    assert_eq!(
        (
            ACCEPTED_BEFORE_THE_SINK_FAILS < already_there.whole_replacement.len(),
            interrupted.is_err(),
            fs::read(&path)? == already_there.previous_bytes,
            names_the_save_still_reports(&path)
        ),
        (true, true, true, BLOCKS.map(str::to_owned).to_vec()),
        "the sink took the block names and then stopped, which is the disk filling up mid-quit. \
         The first two entries are what make the rest mean anything — the interruption has to \
         land inside the save and the writer has to report it — and the last two are the whole \
         requirement: the world a player already had is byte-for-byte where it was and still \
         answers what it needs. A writer that opens the target and streams into it fails here \
         and nowhere else"
    );
    Ok(())
}

#[test]
fn every_save_declares_the_format_it_is_and_the_version_it_was_written_in() -> TestResult {
    let directory = TempDir::new()?;
    let registry = registry_of(&BLOCKS)?;
    let world = world_holding(&every_block_placed(), &registry)?;
    let path = save_in(&directory);
    persistence::save_world(&path, &world, STANDING_SOMEWHERE, &registry)?;

    let written = fs::read(&path)?;

    let mut expected = b"MYCRAFT\x1A".to_vec();
    expected.extend_from_slice(&1_u16.to_le_bytes());
    assert_eq!(
        written.get(..expected.len()),
        Some(expected.as_slice()),
        "compression is excluded from this spec and certain to arrive, and the version field is \
         what makes adding it a bump rather than a guess about what an old file meant. It is \
         numbered 1 and not 0 so that a zero-filled buffer which somehow got past the magic \
         declares a version this build does not recognise instead of declaring the one it \
         supports"
    );
    Ok(())
}
