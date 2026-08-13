//! What a run leaves behind when it ends: a save on a clean close, a report
//! when it could not be written, and the previous save untouched when there was
//! no clean close at all.
//!
//! Every scenario here is driven through the client's own dispatch in a process
//! that constructs no event loop, opens no window and acquires no GPU adapter.
//!
//! # The unclean shutdown is induced by absence, never by killing anything
//!
//! A run that ends without a clean close is a run in which nothing asks the
//! session for its ending: the session is dropped, and that is the whole of it.
//! Killing a process would test the operating system, would need a second binary
//! to kill, and would be the flakiest thing in this repository.
//!
//! # The save is read back rather than looked for
//!
//! "Wrote the world it was playing" is not "a file appeared". The session hands
//! out no borrow of what it owns, so the only two things a test here can compare
//! are the world the fixture declared and the world the file holds — which is
//! exactly the right pair: a client that wrote an empty world, or one that wrote
//! a world it invented, leaves a file either way.
//!
//! # The reason a failed write reports is derived, not spelled out
//!
//! A hand-written expectation would agree with a client quoting the wrong
//! refusal as readily as with one quoting the right one, so the reason is asked
//! of the save path itself through a second route to the same write.

#[path = "support/input/mod.rs"]
mod input;
#[path = "support/persistence.rs"]
mod persistence;

use std::error::Error;
use std::fs;
use std::path::Path;
use std::sync::Arc;

use mc_core::block::BlockRegistry;
use mc_render::window::Ending;
use mc_world::persistence::{SavedPlayer, save_world};
use mc_world::world::VoxelWorld;
use tempfile::TempDir;
use winit::event::MouseButton;

use input::InputHarness;
use persistence::{
    EVERY_DECLARED_CELL, GROUND, TestResult, declared, floor_world, registry_of, save_in,
    standing_on_the_floor, stored_against,
};

/// How far the pointer is pushed down before a click, in raw device counts.
///
/// The aim `click_dispatch.rs` derives from the same declared floor: 280 counts
/// at the declared sensitivity is 35.29° below level, which meets the floor
/// layer rather than passing over it. Restated rather than imported, because a
/// fixture reading the constant it depends on agrees with an aim that moved.
const AIM_DOWN_COUNTS: f64 = 280.0;

/// Where the fixture records the player when it writes a save by hand.
///
/// Only the failed-write scenario uses it, and only to reach the refusal the
/// path itself gives; nothing asserts these numbers.
const ANY_PLAYER: SavedPlayer = SavedPlayer {
    position: [8.5, 10.0, 8.5],
    yaw: 0.0,
    pitch: 0.0,
};

#[test]
fn a_run_that_closed_normally_leaves_the_world_it_was_playing_in_the_save() -> TestResult {
    let (registry, blocks) = a_declared_floor()?;
    let playing = a_client_playing(&registry)?;
    let directory = TempDir::new()?;
    let save = save_in(&directory);

    let ending = playing.quit(Ending::Closed, &save);

    assert_eq!(
        stored_against(&save, &registry, &blocks),
        Ok((EVERY_DECLARED_CELL, Vec::new())),
        "the player closed the window, so the world they were playing is in the file before the \
         process ends — every cell of it, matching what the session was handed. A run that wrote \
         nothing leaves nothing to read, and one that wrote a world of its own leaves a file that \
         is not this one. The run ended as {ending:?} and its save is at {}",
        save.display()
    );
    Ok(())
}

#[test]
fn a_close_whose_save_cannot_be_written_ends_by_reporting_the_path_and_the_reason() -> TestResult {
    let (registry, blocks) = a_declared_floor()?;
    let playing = a_client_playing(&registry)?;
    let directory = TempDir::new()?;
    let save = directory.path().join("world.mcw");
    fs::create_dir(&save)?;
    let reason = why_the_save_refuses(&save, &blocks, &registry);

    let told = report_of(&playing.quit(Ending::Closed, &save));

    assert_eq!(
        (
            told.contains(&save.display().to_string()),
            told.contains(&reason)
        ),
        (true, true),
        "a quit that could not write the save says so on the way out, naming the file it could not \
         write — {} — and why it could not: {reason}. Exiting as though it had saved is the one \
         thing it must not do, because the player is left believing their world is on disk. It \
         reported: {told}",
        save.display()
    );
    Ok(())
}

#[test]
fn a_run_that_ended_without_a_clean_close_leaves_the_previous_save_exactly_as_it_was() -> TestResult
{
    let (registry, blocks) = a_declared_floor()?;
    let directory = TempDir::new()?;
    let save = save_in(&directory);
    a_client_playing(&registry)?.quit(Ending::Closed, &save);
    let after_the_clean_quit = fs::read(&save).ok();

    let mut edited = a_client_playing(&registry)?;
    edited.move_pointer(0.0, AIM_DOWN_COUNTS);
    edited.click(MouseButton::Left);
    let broke = edited.edit();
    drop(edited);

    assert_eq!(
        (
            after_the_clean_quit.is_some(),
            fs::read(&save).ok() == after_the_clean_quit,
            stored_against(&save, &registry, &blocks).is_ok()
        ),
        (true, true, true),
        "the second run edited the world and then ended without ever closing — a killed process, a \
         power cut, a panic — so nothing asked it for an ending and nothing wrote anything. What \
         the previous clean quit left is still there, byte for byte, and still loads. Losing the \
         edits made since launch is what an unclean end costs; damaging the world the player \
         already had is not. The edit that run made was {broke:?}"
    );
    Ok(())
}

/// The registry the fixture floor is declared against, and the floor itself.
fn a_declared_floor() -> Result<(Arc<BlockRegistry>, VoxelWorld), Box<dyn Error>> {
    let registry = Arc::new(registry_of(vec![declared(GROUND, true)?])?);
    let blocks = floor_world(&registry)?;
    Ok((registry, blocks))
}

/// A client whose window is open and whose world has landed.
fn a_client_playing(registry: &Arc<BlockRegistry>) -> Result<InputHarness, Box<dyn Error>> {
    let (simulation, holding) = standing_on_the_floor(Arc::clone(registry))?;
    let mut harness = InputHarness::started();
    harness.play(simulation, holding);
    Ok(harness)
}

/// Why writing a save to `save` is refused, asked of the write path itself.
///
/// A second route to the same refusal the client's own quit has to meet, so the
/// expectation is derived from the file system rather than from a guess about
/// which wording the refusal carries.
fn why_the_save_refuses(save: &Path, blocks: &VoxelWorld, registry: &BlockRegistry) -> String {
    match save_world(save, blocks, ANY_PLAYER, registry) {
        Ok(()) => "the save path could be written after all".to_owned(),
        Err(refused) => refused.to_string(),
    }
}

/// What an ending reports, or what it did instead of reporting anything.
fn report_of(ending: &Ending) -> String {
    match ending {
        Ending::Failed { report } => report.clone(),
        other => format!("the run ended as {other:?}, reporting no failure at all"),
    }
}
