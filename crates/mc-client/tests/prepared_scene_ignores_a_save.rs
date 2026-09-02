//! A scene prepared for a frame capture is built from the world the generator
//! makes, whatever save happens to be lying in the directory it was started in.
//!
//! # This binary holds exactly one test, and that is a constraint rather than a
//! coincidence
//!
//! The save path is relative to the process's working directory, so proving that
//! the preparation ignores a save that is present means putting one where the
//! client would look — which means setting the working directory, which is
//! global to the process. A second test in this binary could observe or be
//! observed by that change. The content root is therefore absolute and this file
//! stays one test long.
//!
//! # Why the whole feature depends on this
//!
//! `prepare_scene` is public precisely so the suites that shoot the golden
//! frames run the pipeline a player launches rather than a copy of it. A "load
//! the save if one exists" branch inside it would let a stray file in a
//! capture's working directory change what a golden frame shows — a committed
//! image moving for a reason no reader of the diff could see. The resume
//! decision sits above it, where the client makes it, and this is what says so.
//!
//! # The marker is a cell the generator cannot produce
//!
//! The scene names four blocks and all four are `base:`, and the declared
//! surface band leaves the cell below empty in the world the seed makes. So the
//! save and the generated world disagree about exactly one position, and a scene
//! built from either cannot be mistaken for one built from the other.

use std::error::Error;
use std::path::{Path, PathBuf};

use mc_client::launch::save_path;
use mc_client::startup::{PreparationError, PreparedScene, prepare_scene};
use mc_core::block::source::InMemoryDefinitionSource;
use mc_core::block::{BlockDefinition, BlockRegistry, DefinitionOrigin, Opacity};
use mc_core::content::FaceTextures;
use mc_core::id::{BlockName, TextureKey};
use mc_world::persistence::{Acceptance, SavedPlayer, load_world, save_world};
use mc_world::section::Contents;
use mc_world::world::{VoxelWorld, WorldPos};
use tempfile::TempDir;

type TestResult = Result<(), Box<dyn Error>>;

/// The block the save holds where the generated world holds nothing.
const MARKER: &str = "fixture:marker";

/// The cell it stands in: above the declared surface band's reach in this
/// column, so the generator puts nothing there.
const MARKER_CELL: (u32, u32, u32) = (8, 40, 8);

/// How many chunk columns the save's world spans on each axis.
const COLUMNS: u32 = 1;

/// What a cell holding nothing is called here.
const NOTHING: &str = "nothing";

/// What a cell the world does not reach is called — a third answer beside a
/// block and the word for nothing, so a world reaching nowhere cannot read as a
/// world holding nothing.
const OUTSIDE: &str = "no such cell";

/// Where the save records the player. Nothing asserts it; a save records
/// somebody.
const RECORDED_PLAYER: SavedPlayer = SavedPlayer {
    position: [8.5, 41.0, 8.5],
    yaw: 0.0,
    pitch: 0.0,
};

#[test]
fn a_scene_prepared_with_a_save_present_is_built_from_the_world_the_generator_makes() -> TestResult
{
    let content = shipped_content()?;
    let started_in = std::env::current_dir()?;
    let directory = TempDir::new()?;
    std::env::set_current_dir(directory.path())?;
    let registry = a_registry_holding_the_marker()?;
    save_world(
        &save_path(),
        &a_world_holding_the_marker(&registry)?,
        RECORDED_PLAYER,
        &registry,
    )?;

    let prepared = prepare_scene(&content);

    let waiting = what_the_save_holds(&save_path(), &registry);
    std::env::set_current_dir(&started_in)?;
    assert_eq!(
        (scene_holds(&prepared, MARKER_CELL), waiting),
        (Ok(NOTHING.to_owned()), Ok(MARKER.to_owned())),
        "a save was sitting at {} — exactly where the client looks for one — holding {MARKER} at \
         {MARKER_CELL:?}, and the scene prepared beside it holds nothing there. The second half is \
         what stops the first from being vacuous: the save really is readable and really does hold \
         the marker, so the two answers genuinely differ and a preparation that had read the file \
         would have said so. A golden frame is shot through this same function, and a capture run \
         in a directory that happens to have a save in it must draw the same picture as one run \
         anywhere else",
        save_path().display()
    );
    Ok(())
}

/// The content root, absolute, because the working directory is about to move.
fn shipped_content() -> Result<PathBuf, Box<dyn Error>> {
    Ok(Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .ok_or("the crate manifest directory has no repository root above it")?
        .join("content")
        .join("base"))
}

/// A registry holding the one block the save names.
fn a_registry_holding_the_marker() -> Result<BlockRegistry, Box<dyn Error>> {
    let origin = DefinitionOrigin::new("the prepared-scene fixture");
    let mut registry = BlockRegistry::new();
    registry.apply(&InMemoryDefinitionSource::new(
        origin.clone(),
        vec![Ok(BlockDefinition {
            name: BlockName::parse(MARKER)?,
            textures: FaceTextures::uniform(TextureKey::parse(MARKER)?),
            is_solid: true,
            replaceable: false,
            breakable: true,
            breaks_into: None,
            drawn: true,
            occludes: true,
            targetable: true,
            swimmable: false,
            move_resistance: 0.0,
            swim_ascent: 9.0,
            opacity: Opacity::OPAQUE,
            origin,
            tint: None,
        })],
    ))?;
    Ok(registry)
}

/// An otherwise empty world with the marker standing in [`MARKER_CELL`].
fn a_world_holding_the_marker(registry: &BlockRegistry) -> Result<VoxelWorld, Box<dyn Error>> {
    let (x, y, z) = MARKER_CELL;
    let mut blocks = VoxelWorld::empty(COLUMNS);
    blocks.set_block(WorldPos { x, y, z }, &BlockName::parse(MARKER)?, registry)?;
    Ok(blocks)
}

/// What the world a prepared scene was built from holds at `at` — or why the
/// preparation refused.
fn scene_holds(
    prepared: &Result<PreparedScene, PreparationError>,
    at: (u32, u32, u32),
) -> Result<String, String> {
    let (x, y, z) = at;
    let scene = prepared
        .as_ref()
        .map_err(std::string::ToString::to_string)?;
    Ok(scene
        .world
        .block_at(x, y, z)
        .map_or_else(|| OUTSIDE.to_owned(), described))
}

/// What the save at `save` holds at [`MARKER_CELL`] — or why it could not be
/// read.
fn what_the_save_holds(save: &Path, registry: &BlockRegistry) -> Result<String, String> {
    let (x, y, z) = MARKER_CELL;
    let loaded = load_world(save, registry, Acceptance::OnlyUnchangedBlocks)
        .map_err(|refused| refused.to_string())?;
    Ok(loaded
        .world
        .block_at(WorldPos { x, y, z })
        .map_or_else(|refused| refused.to_string(), described))
}

/// What `contents` holds, as text: the block's own name, or [`NOTHING`].
fn described(contents: Contents<&BlockName>) -> String {
    match contents {
        Contents::Empty => NOTHING.to_owned(),
        Contents::Holds(name) => name.as_str().to_owned(),
    }
}
