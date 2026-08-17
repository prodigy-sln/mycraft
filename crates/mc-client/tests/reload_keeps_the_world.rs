//! The world a player has been living in survives a reload, cell for cell.
//!
//! # "Nothing changed" is worthless unless something plainly did
//!
//! Every scenario here would be satisfied by a reload that never happened, so
//! every one of them is driven with a candidate that takes `base:stone`'s
//! solidity away and observes that effect in the same test. Without it these are
//! assertions about a swap nobody can show took place.
//!
//! # The world is read back out of a save, because that is the only reading there
//!
//! `Session` hands out no borrow of what it owns, deliberately — no accessor for
//! the simulation and none for the world. So what survived is read the way a
//! player would read it: by quitting, and by loading what was written. The save
//! is written after the swap and read against a second read of the same content
//! root, so the blocks it records are compared against the declarations that were
//! serving when it was written.
//!
//! # What a save is compared against is a declaration and never another run
//!
//! The shipped world is regenerated from its declared seed, so a swap that
//! corrupted a cell has nothing to agree with. The count of cells compared is
//! part of the answer for the same reason it is everywhere else in this
//! repository: a walk that visited a smaller world agrees over fewer cells and
//! says nothing at all about the rest.

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

use glam::Vec3;
use mc_core::id::BlockName;
use mc_render::window::Ending;
use mc_sim::simulation::Simulation;
use tempfile::TempDir;
use winit::event::MouseButton;

use input::InputHarness;
use reload::{
    Adoption, DIRT, STONE, STONE_FILE, adoption, candidate, restating, shipped,
    stone_that_is_not_solid,
};
use reload_save::{save_in, saved_against, saved_at};
use reload_world::{
    AIM_AT_THE_FAR_CELL, AIM_ON_TO_THE_NEAR_CELL, EVERY_CELL_OF_THE_SHIPPED_WORLD, Edit, NOTHING,
    OVER_THE_NEAR_CELL, THE_FAR_CELL, UNDER_THE_SPAWN, edit, floor_of, playing, registry_of,
    resting, shipped_world, standing, standing_at, wrote,
};
use support::{TestResult, content_root};

/// Where a player stands on the shipped world's landmark pillar, whose topmost
/// block is stone and whose top face is therefore one above it.
const ON_THE_LANDMARK: Vec3 = Vec3::new(12.5, 65.0, 12.5);

/// How many ticks a run advances before the candidate is handed over, so the
/// player is settled rather than mid-step when it lands.
const SETTLED_AFTER: u32 = 2;

#[test]
fn a_broken_cell_is_still_empty_and_a_placed_one_still_holds_what_was_placed_after_a_reload()
-> TestResult {
    let saved_in = TempDir::new()?;
    let save = save_in(&saved_in);
    let mut client = a_client_on_a_stone_floor(&content_root()?)?;

    require_edited(
        &broke_the_far_cell(&mut client),
        &built_over_the_near_one(&mut client),
    )?;

    let root = restating(shipped()?, STONE_FILE, &stone_that_is_not_solid())?;
    let answered = adoption(client.adopt(candidate(root.path())?));
    require_admitted(&answered)?;
    client.ticks(1);
    let held_up = still_held_up(&client)?;
    require_quit(&client, &save)?;

    let registry = registry_of(root.path())?;
    assert_eq!(
        (
            saved_at(&save, &registry, THE_FAR_CELL),
            saved_at(&save, &registry, OVER_THE_NEAR_CELL),
            held_up
        ),
        (Ok(NOTHING.to_owned()), Ok(DIRT.to_owned()), false),
        "the hole the player dug and the block they built are the two things they would notice \
         losing, and content changing under them is not a reason to lose either. Stone has stopped \
         holding them up in the same breath, which is what says the swap happened at all — without \
         it a reload that did nothing whatsoever satisfies both readings"
    );
    Ok(())
}

#[test]
fn every_cell_of_the_shipped_world_holds_what_it_held_after_a_reload() -> TestResult {
    let saved_in = TempDir::new()?;
    let save = save_in(&saved_in);
    let serving = content_root()?;
    let serving_registry = registry_of(&serving)?;
    let declared = shipped_world(&serving_registry)?;
    let mut client = a_client_on_the_shipped_world(&serving)?;
    client.ticks(SETTLED_AFTER);

    let root = restating(shipped()?, STONE_FILE, &stone_that_is_not_solid())?;
    let answered = adoption(client.adopt(candidate(root.path())?));
    require_admitted(&answered)?;
    client.ticks(1);
    let held_up = still_held_up(&client)?;
    require_quit(&client, &save)?;

    let registry = registry_of(root.path())?;
    assert_eq!(
        (saved_against(&save, &registry, &declared), held_up),
        (Ok((EVERY_CELL_OF_THE_SHIPPED_WORLD, Vec::new())), false),
        "a reload replaces what blocks *are*, never where they are, and the whole world is the \
         only scope in which that is worth asserting: a swap that emptied a section nobody was \
         looking at, or wrote one block over another everywhere, is a world quietly replaced under \
         a player who was told nothing. The count is the fixture's own integrity — a walk over a \
         smaller world would agree over fewer cells and say nothing about the rest — and the \
         player leaving the pillar is what says the swap happened"
    );
    Ok(())
}

#[test]
fn the_cell_a_player_stood_on_still_holds_stone_once_stone_has_stopped_holding_them_up()
-> TestResult {
    let saved_in = TempDir::new()?;
    let save = save_in(&saved_in);
    let mut client = a_client_on_a_stone_floor(&content_root()?)?;
    client.ticks(SETTLED_AFTER);
    let held_up_before = still_held_up(&client)?;

    let root = restating(shipped()?, STONE_FILE, &stone_that_is_not_solid())?;
    let answered = adoption(client.adopt(candidate(root.path())?));
    require_admitted(&answered)?;
    client.ticks(1);
    let held_up_after = still_held_up(&client)?;
    require_quit(&client, &save)?;

    let registry = registry_of(root.path())?;
    assert_eq!(
        (
            saved_at(&save, &registry, UNDER_THE_SPAWN),
            held_up_before,
            held_up_after
        ),
        (Ok(STONE.to_owned()), true, false),
        "the block is exactly where it was and it has stopped stopping anybody, which is the whole \
         of what a reload does and the whole of what it must not do. A swap that emptied the cell \
         would produce the same fall from the same tick, and nothing that only watched the player \
         could tell the two apart"
    );
    Ok(())
}

/// The player digs the further of the two cells the derived aims meet.
fn broke_the_far_cell(client: &mut InputHarness) -> Edit {
    client.move_pointer(0.0, AIM_AT_THE_FAR_CELL);
    client.click(MouseButton::Left);
    edit(client.edit())
}

/// The player then looks steeper and builds against the nearer one, which is a
/// cell the break did not touch.
fn built_over_the_near_one(client: &mut InputHarness) -> Edit {
    client.move_pointer(0.0, AIM_ON_TO_THE_NEAR_CELL);
    client.click(MouseButton::Right);
    edit(client.edit())
}

/// A client playing a floor of stone, with the content root at `root` serving.
fn a_client_on_a_stone_floor(root: &Path) -> Result<InputHarness, Box<dyn Error>> {
    let (simulation, holding) = playing(root, standing(), |registry| floor_of(registry, STONE))?;
    Ok(playing_client(simulation, holding))
}

/// A client playing the world a launch generates, standing on its landmark.
fn a_client_on_the_shipped_world(root: &Path) -> Result<InputHarness, Box<dyn Error>> {
    let (simulation, holding) = playing(root, standing_at(ON_THE_LANDMARK), shipped_world)?;
    Ok(playing_client(simulation, holding))
}

/// A started client already playing what it was handed.
fn playing_client(simulation: Simulation, holding: BlockName) -> InputHarness {
    let mut client = InputHarness::started();
    client.play(simulation, holding);
    client
}

/// Whether the world is still holding the player up.
///
/// # Errors
///
/// Returns an error where the client has published no tick.
fn still_held_up(client: &InputHarness) -> Result<bool, Box<dyn Error>> {
    let published = client
        .published()
        .ok_or("this fixture's client has published no tick to stand in")?;
    Ok(resting(&published).1)
}

/// Refuses unless the two edits landed where this fixture's geometry says they
/// do, in two different cells.
///
/// The identity of the cells is the fixture's business and the contents are the
/// claim, so this is a guard rather than part of the comparison — but a run whose
/// break and placement landed in one cell would be asserting one cell twice.
fn require_edited(broke: &Edit, built: &Edit) -> Result<(), Box<dyn Error>> {
    let expected = (
        Edit::Emptied(THE_FAR_CELL),
        wrote(OVER_THE_NEAR_CELL, NOTHING, DIRT),
    );
    if (broke, built) == (&expected.0, &expected.1) {
        return Ok(());
    }
    Err(format!(
        "this fixture has to break {THE_FAR_CELL:?} and build in {OVER_THE_NEAR_CELL:?} before the \
         reload, and the client answered {broke:?} then {built:?}. What survives a swap would then \
         be read from cells nobody edited"
    )
    .into())
}

/// Refuses unless the candidate was admitted at all.
fn require_admitted(answered: &Adoption) -> Result<(), Box<dyn Error>> {
    if matches!(answered, Adoption::Accepted { .. }) {
        return Ok(());
    }
    Err(format!(
        "this scenario needs the candidate taking stone's solidity away to be admitted, and the \
         client answered {answered:?}. Nothing would then have crossed the swap for the world to \
         survive"
    )
    .into())
}

/// Refuses unless the run closed normally and wrote its world to `save`.
fn require_quit(client: &InputHarness, save: &Path) -> Result<(), Box<dyn Error>> {
    match client.quit(Ending::Closed, save) {
        Ending::Closed => Ok(()),
        otherwise => Err(format!(
            "this fixture has to write the world it was playing to a save, and the run ended \
             {otherwise:?} instead. Every reading below would be about a save that is not there"
        )
        .into()),
    }
}
