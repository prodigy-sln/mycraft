//! Entry clearing runs on every entry, whatever the load reported about the
//! save's blocks.
//!
//! # The unchanged save is the falsifier, and it is the whole point of this file
//!
//! The cheap implementation asks persistence whether anything changed and only
//! then asks the physics a question. It passes the offline-edit journey below and
//! fails the one after it: that launch reads a save whose blocks all match the
//! declarations it is read against, and whose recorded feet are inside a cell
//! holding a solid block anyway. Nothing about that launch reports a change, and
//! the player is still moved clear. **Do not let that fixture drift into one where
//! a block changed** — it would then be a second copy of the journey above and the
//! decision it grades would be graded by nothing.
//!
//! **The two journeys differ in the save and never in the command line**, and
//! that is a tightening rather than a loss. They once differed in both, because
//! the changed-save launch had to pass a flag to be allowed at all; loading is now
//! what a client does when it is told nothing, so the only variable left is the
//! one the file is about.
//!
//! `mc-world`'s persistence return does now carry which blocks changed — that is
//! what the report on the error stream is composed from — so the cheap
//! implementation is *spellable* here in a way it was not. Nothing may branch the
//! clearing on it: `seat` asks the physics its question unconditionally, and this
//! file is what says so.
//!
//! # The journey is the player's own, run rather than described
//!
//! Quit standing in `base:water`, edit `content/base/blocks/water.luau` from
//! `solid = false` to `solid = true` while the game is not running, relaunch with
//! the flag that says "I have read what changed and I want my world anyway". Water
//! is the one shipped block declared not solid, and both roots here are real
//! content roots read through the one door — a registry assembled in Rust would be
//! asserting against content no author could have written.
//!
//! # A note on what "covers no solid cell" is asserted against
//!
//! Every block either fixture places is solid by the time the save is read, so a
//! cell holding nothing and a cell that is clear are the same cell here. The
//! assertion therefore reads the cells the box covers out of the world the launch
//! is playing and requires each of them to hold nothing — and the *number* of
//! cells is part of the expectation, because a check over an empty list of cells
//! agrees with everything.

#[path = "support/entry.rs"]
mod entry;
#[path = "support/persistence.rs"]
mod persistence;
#[path = "support/reload.rs"]
mod reload;
mod support;

use std::error::Error;
use std::path::Path;
use std::sync::Arc;

use glam::Vec3;
use mc_core::block::BlockRegistry;
use mc_core::id::BlockName;

use entry::{
    A_SEARCH_OF, ASave, FEET_ROW, NO_ARGUMENT, at, cells_a_box_covers, filling, floor_of,
    recorded_at, require, resumed, written,
};
use persistence::{
    EVERY_DECLARED_CELL, Launched, NOTHING, TestResult, against, held_at, refusal, stood_at,
};
use reload::{Declaration, GRASS, STONE, WATER, WATER_FILE, restating, shipped};

/// How many chunk columns square each world here is.
const ONE_COLUMN: u32 = 1;

/// How many cells the box of a player standing at a cell centre covers.
///
/// The box is 0.6 wide and 1.8 tall and a cell centre puts it inside one column,
/// so it stands in that column's row and the one above it and in no others.
/// Stated so that "every cell it covers holds nothing" is a claim about two cells
/// rather than about however many a broken oracle handed back.
const CELLS_A_STANDING_BOX_COVERS: usize = 2;

/// Where the save records the player, in both fixtures: standing on the floor,
/// centred in a column.
const SAVED_FEET: Vec3 = Vec3::new(8.5, FEET_ROW as f32, 8.5);

/// The two cells their box covers there, which each fixture fills with the block
/// that traps them.
const THE_CELLS_THEY_STAND_IN: [(u32, u32, u32); 2] = [(8, FEET_ROW, 8), (8, FEET_ROW + 1, 8)];

/// Where the search puts them: the first position the ring order reaches whose
/// box covers nothing, one cell back along both horizontal axes.
const PUT_CLEAR_AT: Vec3 = Vec3::new(7.5, FEET_ROW as f32, 7.5);

#[test]
fn water_made_solid_while_the_game_was_off_does_not_resume_the_player_inside_it() -> TestResult {
    let save = a_save_of_a_player_standing_in_water()?;

    let launched = resumed(&save, &NO_ARGUMENT)?;

    assert_eq!(
        (stood_at(&launched), what_the_box_covers(&launched)),
        (
            Ok(at(PUT_CLEAR_AT)),
            Ok(vec![NOTHING.to_owned(); CELLS_A_STANDING_BOX_COVERS])
        ),
        "the player quit standing in {WATER}, an author declared {WATER} solid while the game was \
         not running, and the save was resumed with the flag that accepts changed blocks. Today \
         that puts them inside solid rock with no move and no message, and the only way out is to \
         undo the edit. They start at {PUT_CLEAR_AT:?} instead, in a position whose box covers no \
         solid cell. The launch answered: {}",
        refusal(&launched)
    );
    Ok(())
}

#[test]
fn a_launch_that_accepted_no_changed_blocks_still_moves_a_player_saved_inside_stone() -> TestResult
{
    let save = a_save_of_a_player_standing_in_stone()?;

    let launched = resumed(&save, &NO_ARGUMENT)?;

    assert_eq!(
        (stood_at(&launched), what_the_box_covers(&launched)),
        (
            Ok(at(PUT_CLEAR_AT)),
            Ok(vec![NOTHING.to_owned(); CELLS_A_STANDING_BOX_COVERS])
        ),
        "nothing about this save's blocks changed — it is written against the declarations it is \
         read against, and no flag appears on the command line — so there is nothing for a \
         registry verdict or an acceptance mode to report, and the recorded feet are inside {STONE} \
         all the same. **The search runs anyway.** An implementation that asked persistence \
         whether to ask the physics a question passes the offline-edit journey beside this and \
         fails here, which is the whole reason this scenario is written the way it is. The launch \
         answered: {}",
        refusal(&launched)
    );
    Ok(())
}

#[test]
fn a_player_moved_at_entry_starts_within_eight_blocks_of_the_save_in_a_world_still_holding_its_blocks()
-> TestResult {
    let save = a_save_of_a_player_standing_in_stone()?;

    let launched = resumed(&save, &NO_ARGUMENT)?;

    assert_eq!(
        (
            moved_at_all(&launched),
            moved_no_further_than_the_search_looks(&launched),
            against(&launched, &save.blocks)
        ),
        (Ok(true), Ok(true), Ok((EVERY_DECLARED_CELL, Vec::new()))),
        "entry moves the player and not the world. They come to rest inside the {A_SEARCH_OF} \
         blocks the search may look at — a clearing that teleported somebody across the map would \
         be a worse answer than leaving them where they were — and every one of the \
         {EVERY_DECLARED_CELL} cells the save recorded still holds what it recorded, so the block \
         that trapped them is still standing there. The first half is the premise: they really \
         were moved, or \"when entry moves a resumed player\" would be a case that never arose. \
         The launch answered: {}",
        refusal(&launched)
    );
    Ok(())
}

/// A save written of a player standing in water, to be read against a root that
/// has since declared water solid.
///
/// # Errors
///
/// Returns an error if either root cannot be read or written, if the world cannot
/// be built, or if the premise fails: water has to be declared one way when the
/// save is written and the other way when it is read, or this is not the journey.
fn a_save_of_a_player_standing_in_water() -> Result<ASave, Box<dyn Error>> {
    let as_it_was = shipped()?;
    let written_against = registry_of(as_it_was.path())?;
    let mut blocks = floor_of(&written_against, ONE_COLUMN, STONE)?;
    filling(
        &mut blocks,
        &written_against,
        &THE_CELLS_THEY_STAND_IN,
        WATER,
    )?;

    let edited = restating(shipped()?, WATER_FILE, &Declaration::of(WATER).solid(true))?;
    let read_against = registry_of(edited.path())?;

    require_solidity_of(&written_against, WATER, false)?;
    require_solidity_of(&read_against, WATER, true)?;
    written(
        blocks,
        &written_against,
        read_against,
        recorded_at(SAVED_FEET, 0.0, 0.0),
    )
}

/// A save written of a player standing in stone, read against the very
/// declarations it was written against.
///
/// **One root, read twice, with nothing edited between.** That is what makes the
/// launch below one where nothing changed, which is the state the scenario is
/// about.
///
/// # Errors
///
/// Returns an error if the root cannot be read, if the world cannot be built or
/// written, or if the premise fails: the block the player stands in has to be
/// declared solid already, or the save records nobody trapped.
fn a_save_of_a_player_standing_in_stone() -> Result<ASave, Box<dyn Error>> {
    let root = shipped()?;
    let registry = registry_of(root.path())?;
    let mut blocks = floor_of(&registry, ONE_COLUMN, GRASS)?;
    filling(&mut blocks, &registry, &THE_CELLS_THEY_STAND_IN, STONE)?;

    require_solidity_of(&registry, STONE, true)?;
    written(
        blocks,
        &registry,
        Arc::clone(&registry),
        recorded_at(SAVED_FEET, 0.0, 0.0),
    )
}

/// Every block the content root at `root` declares, read through the one door a
/// launch reads content by.
///
/// # Errors
///
/// Returns whichever reader refused the root.
fn registry_of(root: &Path) -> Result<Arc<BlockRegistry>, Box<dyn Error>> {
    Ok(Arc::new(reload::candidate(root)?.registry))
}

/// Refuses unless `registry` declares `block`'s solidity to be `is_solid`.
///
/// # Errors
///
/// Returns an error naming what it declares instead.
fn require_solidity_of(
    registry: &BlockRegistry,
    block: &str,
    is_solid: bool,
) -> Result<(), Box<dyn Error>> {
    let declared = registry.resolve(&BlockName::parse(block)?)?.is_solid;
    require(
        declared == is_solid,
        format!(
            "this fixture needs {block} to be declared solid = {is_solid} in the root it reads, \
             and the root declares solid = {declared}"
        ),
    )
}

/// What the world a launch plays holds in every cell the player's box covers
/// where the launch put them — or the refusal it gave instead.
///
/// # Errors
///
/// Returns the rendered refusal where the launch was turned away.
fn what_the_box_covers(launched: &Launched) -> Result<Vec<String>, String> {
    cells_a_box_covers(placed_at(launched)?)
        .into_iter()
        .map(|cell| held_at(launched, cell))
        .collect()
}

/// Whether the launch put the player anywhere other than where the save recorded
/// them.
///
/// # Errors
///
/// Returns the rendered refusal where the launch was turned away.
fn moved_at_all(launched: &Launched) -> Result<bool, String> {
    Ok(stood_at(launched)? != at(SAVED_FEET))
}

/// Whether every axis of the move is inside the reach the search declares.
///
/// # Errors
///
/// Returns the rendered refusal where the launch was turned away.
fn moved_no_further_than_the_search_looks(launched: &Launched) -> Result<bool, String> {
    let moved = placed_at(launched)? - SAVED_FEET;
    let reach = A_SEARCH_OF as f32;
    Ok(moved.abs().max_element() <= reach)
}

/// Where the launch put the player — or the refusal it gave instead.
///
/// # Errors
///
/// Returns the rendered refusal where the launch was turned away.
fn placed_at(launched: &Launched) -> Result<Vec3, String> {
    Ok(Vec3::from_array(stood_at(launched)?.map(f32::from_bits)))
}
