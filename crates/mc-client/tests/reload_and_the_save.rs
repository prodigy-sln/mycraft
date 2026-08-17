//! A save written after a reload records what the blocks are now.
//!
//! # The save format is untouched, and that is the point
//!
//! A save records, per block its world holds, what the registry that world is
//! named against declared it to be. So a save written after a reload silently
//! resumes against the same content root **only if the swap reached the world** —
//! it is a property of the reload having landed rather than of anything new on
//! disk, and nothing here changes how a save is written or read.
//!
//! # The second scenario is what makes the first one evidence
//!
//! "The relaunch asked the player nothing" is exactly what a relaunch that
//! compares nothing at all would also produce. The control is the same save
//! written *before* the candidate was taken up, opened against the same changed
//! root: it has to name `base:stone` and turn the player away. Without it the
//! silence in the first scenario says nothing about whether anything was ever
//! compared.
//!
//! **The world holds all four shipped blocks so the control can be wrong in both
//! directions.** A save whose world was made of stone alone would name stone and
//! nothing else whatever the comparison did, and "it named only the block that
//! changed" would be true by construction.

#[path = "support/input/mod.rs"]
mod input;
#[path = "support/reload.rs"]
mod reload;
#[path = "support/reload_save.rs"]
mod reload_save;
#[path = "support/reload_world.rs"]
mod reload_world;
mod support;

use std::error::Error;
use std::path::Path;

use mc_render::window::Ending;
use tempfile::TempDir;

use input::InputHarness;
use reload::{
    Adoption, DIRT, GRASS, STONE, STONE_FILE, WATER, adoption, candidate, restating, shipped,
    stone_that_is_not_solid,
};
use reload_save::{STARTED, how_it_went, relaunch, resumed_at, save_in};
use reload_world::{FLOOR, UNDER_THE_SPAWN, floor_holding, playing, standing};
use support::{TestResult, content_root};

/// Three floor cells given over to the other three blocks, so that a save of this
/// world has to name all four and the control below can be wrong either way.
const A_PATCH_OF_DIRT: (i32, i32, i32) = (2, FLOOR, 2);
const A_PATCH_OF_GRASS: (i32, i32, i32) = (3, FLOOR, 2);
const A_PATCH_OF_WATER: (i32, i32, i32) = (4, FLOOR, 2);

#[test]
fn a_save_written_after_a_reload_resumes_against_that_content_without_asking_the_player()
-> TestResult {
    let saved_in = TempDir::new()?;
    let save = save_in(&saved_in);
    let mut client = a_client_holding_every_shipped_block(&content_root()?)?;
    let root = restating(shipped()?, STONE_FILE, &stone_that_is_not_solid())?;

    let answered = adoption(client.adopt(candidate(root.path())?));
    require_admitted(&answered)?;
    require_quit(&client, &save)?;

    let started_again = relaunch(root.path(), &save)?;

    assert_eq!(
        (
            how_it_went(&started_again),
            resumed_at(&started_again, UNDER_THE_SPAWN)
        ),
        (STARTED.to_owned(), Ok(STONE.to_owned())),
        "the player edited a block, carried on playing and quit. What they saved is a world named \
         against the content they were playing, so opening it against that same content is not a \
         change to accept — and the world that comes back is the one they left. A swap that never \
         reached the world writes a save describing blocks nobody is serving, and the player is \
         asked to accept an edit they made themselves an hour ago"
    );
    Ok(())
}

#[test]
fn a_save_written_before_the_reload_is_refused_by_that_same_changed_content_naming_the_block()
-> TestResult {
    let saved_in = TempDir::new()?;
    let save = save_in(&saved_in);
    let client = a_client_holding_every_shipped_block(&content_root()?)?;
    require_quit(&client, &save)?;

    let root = restating(shipped()?, STONE_FILE, &stone_that_is_not_solid())?;
    let started_again = relaunch(root.path(), &save)?;
    let told = how_it_went(&started_again);

    assert_eq!(
        (
            told.contains(STONE),
            [DIRT, GRASS, WATER]
                .iter()
                .any(|block| told.contains(block))
        ),
        (true, false),
        "this save was written before the edit, so the one block whose declaration moved is a \
         change the player has to accept — and the three they never touched are not. It is the \
         control the scenario beside it cannot do without: a comparison that had stopped \
         comparing, or one that reported every block whatever happened to it, produces a silent \
         resume there and reads as success"
    );
    Ok(())
}

/// A client standing on a floor of stone with one cell each of the other three
/// blocks, so its save has to name all four.
fn a_client_holding_every_shipped_block(root: &Path) -> Result<InputHarness, Box<dyn Error>> {
    let (simulation, holding) = playing(root, standing(), |registry| {
        floor_holding(
            registry,
            STONE,
            &[
                (A_PATCH_OF_DIRT, DIRT),
                (A_PATCH_OF_GRASS, GRASS),
                (A_PATCH_OF_WATER, WATER),
            ],
        )
    })?;
    let mut client = InputHarness::started();
    client.play(simulation, holding);
    Ok(client)
}

/// Refuses unless the candidate was admitted at all.
fn require_admitted(answered: &Adoption) -> Result<(), Box<dyn Error>> {
    if matches!(answered, Adoption::Accepted { .. }) {
        return Ok(());
    }
    Err(format!(
        "this scenario needs the candidate to be admitted before the quit, and the client answered \
         {answered:?}. The save would then record the content that was already serving, which is \
         the control's fixture and not this one's"
    )
    .into())
}

/// Refuses unless the run closed normally and wrote its world to `save`.
fn require_quit(client: &InputHarness, save: &Path) -> Result<(), Box<dyn Error>> {
    match client.quit(Ending::Closed, save) {
        Ending::Closed => Ok(()),
        otherwise => Err(format!(
            "this fixture has to write the world it is playing to {save:?}, and the run ended \
             {otherwise:?} instead. The relaunch below would be a first launch"
        )
        .into()),
    }
}
