//! The geometry a frame capture is shot from is the generated world's, whatever
//! save is lying in the directory the capture was started in.
//!
//! # This is one level up from the world, and that is the whole reason it exists
//!
//! `prepared_scene_ignores_a_save.rs` already asserts which *world* a prepared
//! scene was built from. A golden frame is not shot from a world; it is shot from
//! packed vertices and a section table, and those are two answers to one question.
//! A preparation could read the right world and still hand over geometry meshed
//! from something else, packed in a different order, or resolved against a
//! different texture key set — and every one of those moves a committed image while
//! the world assertion stays green. So this compares what the renderer would
//! actually be given.
//!
//! # This binary holds exactly one test, and that is a constraint rather than a
//! coincidence
//!
//! Where the client looks for its save is relative to the process's working
//! directory, so putting a save *where the client looks for one* means moving that
//! directory, which is global to the process. `cargo nextest`, which is what the
//! gate runs, gives every test its own process and would make a second test here
//! harmless — but `cargo test` is a command any contributor may type, and under it
//! a second test would observe or be observed by the change. A test whose
//! soundness depends on which runner was invoked passes for a reason nobody chose,
//! so the property is held structurally instead: one working-directory move per
//! process, one test per binary. Its sibling
//! `capture_geometry_ignores_an_unreadable_save.rs` is the other half of the pair,
//! separate for the same reason.
//!
//! # The marker is a cell the generator cannot produce
//!
//! The shipped content names four blocks and all four are `base:`, and the
//! declared surface band leaves the cell below empty in the world the seed makes.
//! So the save on disk and the generated world disagree about exactly one
//! position, and the second half of the assertion below reads that position back
//! off the disk: the file really is a save, and it really does hold something the
//! generated world does not. Without it, an unreadable or empty save would satisfy
//! the first half by having nothing to say.
//!
//! # The baseline is the same function run where no save is, never a second
//! pipeline
//!
//! What the geometry is compared against is `prepare_scene` itself, run in a
//! directory holding no save at all. Meshing the generated world here instead
//! would be a second spelling of the pipeline the goldens are shot through, which
//! is exactly how the two drifted apart before (`support/mod.rs`).

#[path = "support/handed.rs"]
mod handed;

use std::error::Error;
use std::path::Path;

use mc_client::launch::save_path;
use mc_client::startup::{PreparationError, PreparedScene, prepare_scene};
use mc_core::block::source::InMemoryDefinitionSource;
use mc_core::block::{BlockDefinition, BlockRegistry, DefinitionOrigin};
use mc_core::content::FaceTextures;
use mc_core::id::{BlockName, TextureKey};
use mc_render::geometry::scene::SceneGeometry;
use mc_world::persistence::{Acceptance, SavedPlayer, load_world, save_world};
use mc_world::section::Contents;
use mc_world::world::{VoxelWorld, WorldPos};
use tempfile::TempDir;

use handed::{NO_DIFFERENCE, TestResult, how_it_compares, shipped_content};

/// The block the save holds where the generated world holds nothing.
const MARKER: &str = "fixture:marker";

/// The cell it stands in: above the declared surface band's reach in this column,
/// so the generator puts nothing there.
const MARKER_CELL: (u32, u32, u32) = (8, 40, 8);

/// How many chunk columns the save's world spans on each axis.
const COLUMNS: u32 = 1;

/// What a cell holding nothing is called here.
const NOTHING: &str = "nothing";

/// Where the save records the player. Nothing asserts it; a save records somebody.
const RECORDED_PLAYER: SavedPlayer = SavedPlayer {
    position: [8.5, 41.0, 8.5],
    yaw: 0.0,
    pitch: 0.0,
};

#[test]
fn a_capture_beside_a_readable_save_is_shot_from_the_generated_worlds_geometry() -> TestResult {
    let content = shipped_content()?;
    let started_in = std::env::current_dir()?;
    let (nowhere, beside_a_save) = (TempDir::new()?, TempDir::new()?);
    let registry = a_registry_holding_the_marker()?;

    std::env::set_current_dir(nowhere.path())?;
    let where_no_save_is = prepare_scene(&content);
    std::env::set_current_dir(beside_a_save.path())?;
    save_world(
        &save_path(),
        &a_world_holding_the_marker(&registry)?,
        RECORDED_PLAYER,
        &registry,
    )?;
    let beside_the_save = prepare_scene(&content);
    let waiting = what_the_save_holds(&save_path(), &registry);
    std::env::set_current_dir(&started_in)?;

    assert_eq!(
        (compared(&beside_the_save, &where_no_save_is), waiting),
        (Ok(NO_DIFFERENCE.to_owned()), Ok(MARKER.to_owned())),
        "a save was sitting at {} — exactly where the client looks for one — holding {MARKER} at \
         {MARKER_CELL:?}, and the geometry prepared beside it is byte for byte the geometry \
         prepared where no save is. The second half is what stops the first from being vacuous: \
         the file really is a readable save and really does hold the marker, so a preparation that \
         had read it would have said so. Every committed golden frame is shot through this same \
         function, and a capture run in a directory that happens to have a save in it has to pack \
         the same vertices as one run anywhere else",
        beside_a_save.path().join(save_path()).display()
    );
    Ok(())
}

/// A registry holding the one block the save names.
fn a_registry_holding_the_marker() -> Result<BlockRegistry, Box<dyn Error>> {
    let origin = DefinitionOrigin::new("the capture-geometry fixture");
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
            origin,
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

/// What the save at `save` holds at [`MARKER_CELL`] — or why it could not be read.
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

/// How the geometry of one preparation compares with another's — or the refusal
/// whichever of them gave one.
///
/// A refusal comes back as the failed comparison rather than as a propagated
/// error, so that "it refused to prepare a scene" and "it prepared the wrong
/// geometry" are one failed assertion instead of two kinds of failure.
fn compared(
    beside: &Result<PreparedScene, PreparationError>,
    against: &Result<PreparedScene, PreparationError>,
) -> Result<String, String> {
    Ok(how_it_compares(geometry_of(beside)?, geometry_of(against)?))
}

/// The scene one preparation produced, or its refusal, rendered.
fn geometry_of(
    prepared: &Result<PreparedScene, PreparationError>,
) -> Result<&SceneGeometry, String> {
    Ok(&prepared
        .as_ref()
        .map_err(std::string::ToString::to_string)?
        .scene)
}
