//! A file that is not a save, sitting where the client looks for one, does not
//! stop a frame capture — and does not change the geometry it is shot from.
//!
//! # This is the sharper half of the pair
//!
//! Its sibling `capture_geometry_ignores_a_save.rs` says a *readable* save cannot
//! move a committed image. This one says a **broken** file cannot stop one being
//! captured at all, which is the failure a capture path that ever learned to read a
//! save would acquire: every golden run in a directory holding a stale or truncated
//! file would fail, as a refusal rather than as an image diff, and the cause would
//! be nowhere near the suite that reported it. The whole point of the frame-capture
//! path not reading saves is that there is no file on disk it can be refused by.
//!
//! # This binary holds exactly one test, and that is a constraint
//!
//! Where the client looks for its save is relative to the process's working
//! directory, which is global to the process. `cargo nextest`, which is what the
//! gate runs, gives every test its own process — but `cargo test` is a command any
//! contributor may type, and under it a second test in this binary would observe or
//! be observed by the move. A test whose soundness depends on which runner was
//! invoked passes for a reason nobody chose, so the property is held structurally:
//! one working-directory move per process, one test per binary.
//!
//! # The verdict is the reason, not the absence of a world
//!
//! The control here has to say that the file genuinely cannot be read, and
//! `is_err()` would say that just as well the day the loader started refusing every
//! save for an unrelated reason. So the refusal is compared against one named
//! outcome — the file does not begin the way a save does — and any other refusal,
//! or a successful load, reports itself instead.

#[path = "support/handed.rs"]
mod handed;

use std::error::Error;
use std::fs;
use std::path::Path;

use mc_client::launch::save_path;
use mc_client::startup::{PreparationError, PreparedScene, prepare_scene};
use mc_core::block::BlockRegistry;
use mc_render::geometry::scene::SceneGeometry;
use mc_world::content::TomlFileDefinitionSource;
use mc_world::persistence::{Acceptance, LoadError, load_world};
use tempfile::TempDir;

use handed::{NO_DIFFERENCE, TestResult, how_it_compares, shipped_content};

/// What is written where the save belongs: bytes no build of this game ever wrote.
const NOT_A_SAVE: &[u8] = b"these bytes are not a save and never were";

/// The one refusal this fixture is built to provoke, named so that any other
/// refusal reads as a different failure rather than as this one.
const REFUSED_AS_NOT_A_SAVE: &str = "the file does not begin the way a save does";

#[test]
fn an_unreadable_save_leaves_a_capture_shot_from_the_generated_worlds_geometry() -> TestResult {
    let content = shipped_content()?;
    let started_in = std::env::current_dir()?;
    let (nowhere, beside_a_save) = (TempDir::new()?, TempDir::new()?);
    let registry = the_shipped_registry(&content)?;

    std::env::set_current_dir(nowhere.path())?;
    let where_no_save_is = prepare_scene(&content);
    std::env::set_current_dir(beside_a_save.path())?;
    write_something_that_is_not_a_save(&save_path())?;
    let beside_the_save = prepare_scene(&content);
    let refusal = why_the_save_cannot_be_read(&save_path(), &registry);
    std::env::set_current_dir(&started_in)?;

    assert_eq!(
        (compared(&beside_the_save, &where_no_save_is), refusal),
        (
            Ok(NO_DIFFERENCE.to_owned()),
            REFUSED_AS_NOT_A_SAVE.to_owned()
        ),
        "a file that is not a save was sitting at {} — exactly where the client looks for one — \
         and the capture path packed byte for byte the geometry it packs where no file is there at \
         all. The second half is the control: that file really is unreadable, and it is unreadable \
         for the stated reason rather than for some other one, so a capture path that had tried to \
         read it would have been refused and this test would report the refusal instead of a \
         geometry",
        beside_a_save.path().join(save_path()).display()
    );
    Ok(())
}

/// A registry holding exactly what this repository ships.
///
/// The one the control reads the file with, and it is the shipped set rather than a
/// fixture's own so that "this file cannot be read" cannot be an artefact of a
/// registry that would have refused a perfectly good save.
fn the_shipped_registry(root: &Path) -> Result<BlockRegistry, Box<dyn Error>> {
    let mut registry = BlockRegistry::new();
    registry.apply(&TomlFileDefinitionSource::new(root.to_owned()))?;
    Ok(registry)
}

/// Puts [`NOT_A_SAVE`] at `save`, making the directories a first launch would not
/// find there.
fn write_something_that_is_not_a_save(save: &Path) -> Result<(), Box<dyn Error>> {
    if let Some(directory) = save.parent() {
        fs::create_dir_all(directory)?;
    }
    fs::write(save, NOT_A_SAVE)?;
    Ok(())
}

/// Why the file at `save` is not a world this build can load — or what it is, if it
/// turns out to be one.
fn why_the_save_cannot_be_read(save: &Path, registry: &BlockRegistry) -> String {
    match load_world(save, registry, Acceptance::OnlyUnchangedBlocks) {
        Ok(_) => "it is a save this build reads".to_owned(),
        Err(LoadError::NotASave { .. }) => REFUSED_AS_NOT_A_SAVE.to_owned(),
        Err(refused) => refused.to_string(),
    }
}

/// How the geometry of one preparation compares with another's — or the refusal
/// whichever of them gave one.
///
/// A refusal comes back as the failed comparison rather than as a propagated error,
/// so that "it refused to prepare a scene" and "it prepared the wrong geometry" are
/// one failed assertion instead of two kinds of failure. For this scenario the
/// first of those two is the whole point.
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
