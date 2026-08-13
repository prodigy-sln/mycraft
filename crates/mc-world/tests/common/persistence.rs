//! Fixtures for the save suites: worlds to save, and the two ways a caller can
//! look at what was written.
//!
//! Every save here goes to a real file in a temporary directory of its own.
//! There is no fixed path shared between tests, because the write path replaces
//! a file by renaming a sibling over it and two tests sharing a directory would
//! be racing for the same target.
//!
//! **Saving is funnelled through the two helpers below rather than called
//! directly**, wherever the path itself is not the subject. That was written
//! down against the format growing a player record, and it has now grown one:
//! the writer's signature gained an argument and this file is where it was
//! absorbed, so the suites whose subject is not the player never mention it.

use std::error::Error;
use std::path::{Path, PathBuf};

use mc_core::block::BlockRegistry;
use mc_core::id::BlockName;
use mc_world::persistence::{
    self, Acceptance, LoadError, LoadedWorld, SaveRequirements, SavedPlayer,
};
use mc_world::world::{VoxelWorld, WorldPos};
use tempfile::TempDir;

use super::described;

/// Where the player stands in every save a fixture writes without caring.
///
/// Spelled out rather than left at the origin, and finite in all five numbers:
/// a writer that stored nothing at all would agree with a fixture of zeroes,
/// and a reader that refused a value would refuse for a reason no test here
/// means to state. The suites that *are* about the player hand the writer a
/// player of their own.
pub const STANDING_SOMEWHERE: SavedPlayer = SavedPlayer {
    position: [1.5, 66.0, 2.5],
    yaw: 0.25,
    pitch: -0.5,
};

/// What a save is called wherever a fixture does not care.
pub const SAVE_FILE: &str = "world.mcw";

/// How many columns a side the fixture worlds span, unless a scenario says
/// otherwise. One column is 16 × 256 × 16 cells, which is more than any fixture
/// here needs to hold.
pub const FIXTURE_FOOTPRINT: u32 = 1;

/// The save inside `directory`.
#[must_use]
pub fn save_in(directory: &TempDir) -> PathBuf {
    directory.path().join(SAVE_FILE)
}

/// A world position, spelled out.
#[must_use]
pub const fn world_at(x: u32, y: u32, z: u32) -> WorldPos {
    WorldPos { x, y, z }
}

/// A world holding each of `blocks` at the position beside it, and nothing
/// anywhere else.
///
/// # Errors
///
/// Returns an error if a name is not a namespaced id, or if the world refuses a
/// write — a position outside it, or a block `registry` does not hold.
pub fn world_holding(
    blocks: &[(WorldPos, &str)],
    registry: &BlockRegistry,
) -> Result<VoxelWorld, Box<dyn Error>> {
    let mut world = VoxelWorld::empty(FIXTURE_FOOTPRINT);
    for &(at, name) in blocks {
        world.set_block(at, &BlockName::parse(name)?, registry)?;
    }
    Ok(world)
}

/// A save of `world` written into `directory` as `file_name`, and the world data
/// it stored.
///
/// # Errors
///
/// Returns an error if the save cannot be written or its stored world data
/// cannot be read back.
pub fn stored_data(
    directory: &TempDir,
    file_name: &str,
    world: &VoxelWorld,
    registry: &BlockRegistry,
) -> Result<Vec<u8>, Box<dyn Error>> {
    let path = directory.path().join(file_name);
    persistence::save_world(&path, world, STANDING_SOMEWHERE, registry)?;
    Ok(persistence::stored_world_data(&path)?)
}

/// A save of `world` written into `directory`, and what it says it requires.
///
/// # Errors
///
/// Returns an error if the save cannot be written, or if it cannot answer what
/// it requires.
pub fn saved_requirements(
    directory: &TempDir,
    world: &VoxelWorld,
    registry: &BlockRegistry,
) -> Result<SaveRequirements, Box<dyn Error>> {
    let path = save_in(directory);
    persistence::save_world(&path, world, STANDING_SOMEWHERE, registry)?;
    Ok(persistence::requirements(&path)?)
}

/// Every byte a save of `world` is made of, without a filesystem in the way.
///
/// # Errors
///
/// Returns an error if the save cannot be written.
pub fn written_bytes(
    world: &VoxelWorld,
    registry: &BlockRegistry,
) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut bytes = Vec::new();
    persistence::write_save(&mut bytes, world, STANDING_SOMEWHERE, registry)?;
    Ok(bytes)
}

/// Every name a save says it needs, as text, in the order it reports them.
#[must_use]
pub fn required_names(requirements: &SaveRequirements) -> Vec<String> {
    requirements
        .names()
        .map(|name| name.as_str().to_owned())
        .collect()
}

/// Every name a save records a declaration for, as text, in the order the
/// declarations are reported.
///
/// Read off the declarations rather than off the names, so that a report whose
/// two halves disagree cannot look complete through either one alone.
#[must_use]
pub fn declared_names(requirements: &SaveRequirements) -> Vec<String> {
    requirements
        .blocks()
        .iter()
        .map(|block| block.name.as_str().to_owned())
        .collect()
}

/// What a world answers about a position it does not have.
///
/// A third answer beside a block's name and [`NOTHING`](super::NOTHING), and it
/// has to stay a third one: "holds nothing" and "there is no such cell" are
/// different facts, and a round trip that turned the first into the second would
/// look like emptiness surviving while the world had quietly shrunk.
pub const NO_SUCH_CELL: &str = "no such cell";

/// What two worlds that agree everywhere are said to be.
pub const AGREES: &str = "the same contents at every position";

/// What `world` answers about `at`, as text: the block's name, the word for
/// nothing, or [`NO_SUCH_CELL`].
#[must_use]
pub fn answer_at(world: &VoxelWorld, at: WorldPos) -> String {
    world
        .block_at(at)
        .map_or_else(|_refused| NO_SUCH_CELL.to_owned(), described)
}

/// Where `produced` first disagrees with `expected`, in words — or [`AGREES`]
/// where they hold the same contents at every position of both.
///
/// **The comparison is cell by cell and never `==` between the two worlds.** A
/// world that has been through a save holds its palettes in the minimal form the
/// writer emitted, so two worlds holding exactly the same blocks are legitimately
/// unequal as values; what a round trip promises is about contents and not about
/// book-keeping.
///
/// The extent is compared first, so that a world which came back smaller is
/// reported as the wrong size rather than as a position it does not have.
///
/// # Errors
///
/// Returns an error if a position inside both extents cannot be read, which
/// would mean a world does not have the cells its own extent claims.
pub fn disagreement(
    produced: &VoxelWorld,
    expected: &VoxelWorld,
) -> Result<String, Box<dyn Error>> {
    if produced.extent() != expected.extent() {
        return Ok(format!(
            "it reaches {:?} where the world it was saved from reaches {:?}",
            produced.extent(),
            expected.extent()
        ));
    }
    for at in expected.extent().positions() {
        let held = described(produced.block_at(at)?);
        let wanted = described(expected.block_at(at)?);
        if held != wanted {
            return Ok(format!(
                "at ({}, {}, {}) it holds {held} where the world it was saved from holds {wanted}",
                at.x, at.y, at.z
            ));
        }
    }
    Ok(AGREES.to_owned())
}

/// The world the save at `path` holds, read against `registry`.
///
/// # Errors
///
/// Returns whatever the load refused with.
pub fn loaded_from(
    path: &Path,
    registry: &BlockRegistry,
    accepting: Acceptance,
) -> Result<VoxelWorld, Box<dyn Error>> {
    Ok(persistence::load_world(path, registry, accepting)?.world)
}

/// What loading `path` produced, compared cell by cell against `saved_from` —
/// or the refusal it answered with, in words.
///
/// **A refusal is folded into the answer rather than propagated**, so that a
/// load turned away and a load that produced the wrong world are both a failed
/// assertion about the same value. Asserting `Ok` and stopping there would be
/// satisfied by a loader that handed back an empty world, which is the one thing
/// "produces the world it was saved from" exists to rule out.
///
/// # Errors
///
/// Returns an error if the two worlds cannot be compared.
pub fn produced_from(
    path: &Path,
    registry: &BlockRegistry,
    accepting: Acceptance,
    saved_from: &VoxelWorld,
) -> Result<String, Box<dyn Error>> {
    match persistence::load_world(path, registry, accepting) {
        Ok(loaded) => disagreement(&loaded.world, saved_from),
        Err(refusal) => Ok(format!("it was refused: {refusal}")),
    }
}

/// What a load answered, with any world it produced reduced to its size.
///
/// For the suites whose subject is a refusal: printing a whole world beside a
/// refusal that did not arrive would bury the one in a page of the other.
///
/// # Errors
///
/// Returns whatever the load refused with, which is what those suites assert.
pub fn what_it_loaded(answer: Result<LoadedWorld, LoadError>) -> Result<String, LoadError> {
    answer.map(|loaded| format!("a world reaching {:?}", loaded.world.extent()))
}

/// What `requirements` recorded for `name` — its behaviour and its appearance —
/// or nothing where the save does not name it at all.
#[must_use]
pub fn declaration_of(requirements: &SaveRequirements, name: &str) -> Option<(u64, u64)> {
    requirements
        .blocks()
        .iter()
        .find(|block| block.name.as_str() == name)
        .map(|block| (block.behaviour.get(), block.appearance.get()))
}
