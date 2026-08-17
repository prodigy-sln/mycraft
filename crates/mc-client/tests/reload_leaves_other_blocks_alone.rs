//! A candidate that changed one declaration changed one declaration.
//!
//! # What a save records is the reading, and it is not a reading this spec added
//!
//! Every save carries, per block its world holds, what that block was *declared*
//! to be: one value folded over the rules by which it is mutated and one folded
//! over how it looks. That is exactly what "its declared fields" means to a
//! relaunch, and it is a reading of the registry the world is named against
//! rather than of anything a reload wrote — so two saves taken either side of a
//! swap say whether a block the author never touched moved.
//!
//! **The world holds all four shipped blocks on purpose.** A save records only
//! the names its world actually needs, so a world made of one block would leave
//! the other three out of the comparison altogether — and a reload that rewrote
//! every declaration it read would pass over it with nothing to disagree with.
//! The count of names the save carries is asserted for that reason: it is the
//! fixture's own integrity and not the claim.
//!
//! # This is green until something changes, and that is what it is for
//!
//! A reload that never happened moves nothing, so nothing the author did not edit
//! moves either. What it would catch is the opposite mistake — a candidate build
//! that applied one block's fields to the rest — and the scenarios that say the
//! author's own edit *did* land are the ones about the three mutation rules,
//! driven through the same door.

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
    Adoption, DIRT, Declaration, GRASS, STONE, STONE_FILE, WATER, adoption, candidate, restating,
    shipped,
};
use reload_save::{declared_by, declared_for, save_in};
use reload_world::{FLOOR, floor_holding, playing, standing};
use support::{TestResult, content_root};

/// The blocks this scenario is about: everything the shipped root declares apart
/// from the one the author edited.
const THE_ONES_NOBODY_EDITED: [&str; 3] = [DIRT, GRASS, WATER];

/// How many distinct names a save of this fixture's world carries. Derived from
/// the world the fixture declares rather than counted from a run: a save carrying
/// fewer is a world that stopped holding a block, and the comparison would then
/// be over a shorter list than it was written for.
const EVERY_SHIPPED_BLOCK: usize = 4;

/// Three floor cells given over to the other three blocks, so that the world's
/// save has to name all four.
const A_PATCH_OF_DIRT: (i32, i32, i32) = (2, FLOOR, 2);
const A_PATCH_OF_GRASS: (i32, i32, i32) = (3, FLOOR, 2);
const A_PATCH_OF_WATER: (i32, i32, i32) = (4, FLOOR, 2);

#[test]
fn a_candidate_editing_one_declaration_leaves_every_other_blocks_declared_fields_alone()
-> TestResult {
    let saved_in = TempDir::new()?;
    let before = save_in(&saved_in);
    let after = saved_in.path().join("afterwards").join("world.mcw");
    let mut client = a_client_holding_every_shipped_block(&content_root()?)?;
    require_quit(&client, &before)?;

    let root = restating(
        shipped()?,
        STONE_FILE,
        &Declaration::of(STONE).breakable(false),
    )?;
    let answered = adoption(client.adopt(candidate(root.path())?));
    require_admitted(&answered)?;
    require_quit(&client, &after)?;
    require_whole(&before)?;
    require_whole(&after)?;

    assert_eq!(
        declared_for(&after, &THE_ONES_NOBODY_EDITED),
        declared_for(&before, &THE_ONES_NOBODY_EDITED),
        "the author opened one file and changed one word in it. Everything else they wrote has to \
         come back out of the reload exactly as it went in — a candidate build that carried one \
         block's fields across the rest would redeclare a player's whole world behind an edit \
         they made to a single block, and the first they would hear of it is a save that will not \
         open"
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

/// Refuses unless the save at `save` names every block the fixture put in its
/// world.
fn require_whole(save: &Path) -> Result<(), Box<dyn Error>> {
    let recorded = declared_by(save).map_err(|refused| -> Box<dyn Error> { refused.into() })?;
    if recorded.len() == EVERY_SHIPPED_BLOCK {
        return Ok(());
    }
    Err(format!(
        "this fixture has to write a save naming all {EVERY_SHIPPED_BLOCK} shipped blocks, and \
         this one names {found}: {recorded:?}. The comparison would then be over a shorter list \
         than it was written for, and a block that moved could be one it never looked at",
        found = recorded.len()
    )
    .into())
}

/// Refuses unless the candidate was admitted at all.
fn require_admitted(answered: &Adoption) -> Result<(), Box<dyn Error>> {
    if matches!(answered, Adoption::Accepted { .. }) {
        return Ok(());
    }
    Err(format!(
        "this scenario needs the candidate editing one declaration to be admitted, and the client \
         answered {answered:?}. The two saves would then be two saves of the same content"
    )
    .into())
}

/// Refuses unless the run closed normally and wrote its world to `save`.
///
/// The client is quit twice against two different paths, which is what lets one
/// run be read either side of a swap. Quitting writes a save and leaves the
/// session playing, so nothing about the second reading depends on the first.
fn require_quit(client: &InputHarness, save: &Path) -> Result<(), Box<dyn Error>> {
    match client.quit(Ending::Closed, save) {
        Ending::Closed => Ok(()),
        otherwise => Err(format!(
            "this fixture has to write the world it is playing to {save:?}, and the run ended \
             {otherwise:?} instead"
        )
        .into()),
    }
}
