//! A save whose blocks look different and behave identically, and the verdict
//! arm nothing in this repository had ever reached.
//!
//! # Why this fixture had to be minted and can never be minted again
//!
//! `RegistryVerdict::retextured` names blocks the registry holds whose declared
//! **appearance alone** has changed. A block whose behaviour also moved lands in
//! `changed` instead, and every committed save in this repository predates a
//! behaviour move — so all of them fill `changed`, `retextured` stays empty in
//! every one, and the arm has been dead code with a doc comment since the split
//! was written.
//!
//! This save was written at behaviour revision 4 and appearance revision 3, in
//! the window between the two commits of this spec that move the appearance byte.
//! That state cannot be produced by any run of this tree, and it will never occur
//! again: the next appearance move makes this fixture stale by two revisions and
//! the next behaviour move takes it out of the arm entirely. It is evidence with
//! an expiry, which is why what it proves is written down here rather than left
//! to be reconstructed.
//!
//! # It is not coverage — it is the upgrade the owner of this repository is
//! # about to perform
//!
//! Somebody with a world saved on the previous build opens it on this one. Their
//! blocks are the same blocks to stand on, build through and break; what changed
//! is that the record now carries how much light each one stops. `retextured` is
//! the word they get, and **the load is not refused even when they asked to be
//! stopped if anything moved** — which is the whole reason this spec put the
//! degree on the appearance list. Had it gone on the behaviour list, that same
//! player would be turned away from their own world over a rendering number.
//! Nothing else in the workspace can state that, because nothing else has a save
//! that is stale in one list and current in the other.
//!
//! # What each reading here can and cannot see
//!
//! The verdict reading is the one that names the arm. The load reading is a
//! second entry point onto it — `load_world` computes its own answer rather than
//! returning `resolve`'s, and a `retextured` list assembled correctly in one and
//! lost in the other is invisible to a reading that goes through only one of
//! them. The name-table reading is what stops both being claims about a save that
//! needs nothing: an empty verdict satisfies "no block changed" perfectly.

mod common;

use std::error::Error;
use std::path::{Path, PathBuf};

use common::TestResult;
use common::persistence::{answer_at, world_at};
use mc_core::block::BlockRegistry;
use mc_core::id::BlockName;
use mc_world::content::LuauFileDefinitionSource;
use mc_world::persistence::{self, Acceptance, RegistryVerdict, requirements, resolve};
use mc_world::world::WorldPos;

/// The save every reading here is judged against, written under behaviour
/// revision 4 and appearance revision 3.
const APPEARANCE_REVISION_3_SAVE: &str =
    "tests/fixtures/world_saved_against_appearance_revision_3.mcw";

/// Every block that save holds, in the ascending order a requirements report
/// lists them in.
const HELD_BY_THE_SAVE: [&str; 4] = ["base:dirt", "base:grass", "base:stone", "base:water"];

/// Where each of those four sits in the saved world, in the same order.
const CELLS: [WorldPos; 4] = [
    world_at(1, 1, 1),
    world_at(2, 3, 4),
    world_at(5, 8, 13),
    world_at(15, 34, 15),
];

/// What a world answers about a cell holding nothing, in the words
/// [`answer_at`] uses.
const NOTHING: &str = "nothing";

/// A cell of the saved world that was never written to, and what it must still
/// answer.
const AN_EMPTY_CELL: WorldPos = world_at(3, 3, 3);

#[test]
fn every_block_of_that_save_is_reported_retextured_and_not_one_of_them_as_behaving_differently()
-> TestResult {
    let needed = requirements(&appearance_revision_3_save()?)?;

    let verdict = resolve(&needed, &shipped_registry()?);

    assert_eq!(
        verdict,
        RegistryVerdict {
            missing: Vec::new(),
            changed: Vec::new(),
            retextured: every_block_the_save_holds()?,
        },
        "the arm nothing in this repository had ever reached. Every other committed save is stale \
         in behaviour as well, so its blocks land in `changed` and this list stays empty in every \
         one of them — which means a `retextured` that had come to be computed wrongly, or never \
         populated at all, would have looked exactly like a correct one for as long as the split \
         has existed. The verdict is compared **whole**, so the two near misses are two distinct \
         failures: a degree routed onto the behaviour list fills `changed` and empties this, and \
         a degree that reached neither record leaves all three empty"
    );
    Ok(())
}

#[test]
fn that_save_loads_for_a_player_who_asked_to_be_stopped_if_anything_moved() -> TestResult {
    let loaded = persistence::load_world(
        &appearance_revision_3_save()?,
        &shipped_registry()?,
        Acceptance::OnlyUnchangedBlocks,
    )?;

    let held: Vec<String> = CELLS
        .into_iter()
        .map(|at| answer_at(&loaded.world, at))
        .chain([answer_at(&loaded.world, AN_EMPTY_CELL)])
        .collect();

    assert_eq!(
        (loaded.changed, held),
        (Vec::new(), the_cells_it_was_left_in()),
        "**this is the reading the whole appearance-versus-behaviour split is for.** A player who \
         asked to be refused if anything about their blocks moved opens a world saved on the \
         previous build and gets it back — because what moved is how much light each block stops, \
         which is not something they stand on, build through or break. Route the degree onto the \
         behaviour list instead and this exact call is refused, for every player at once, over a \
         rendering number; no other save in this repository can show that, because every other one \
         is stale in behaviour too and is refused here for a reason that has nothing to do with \
         this spec. Both halves travel together because either alone is satisfied by a failure: an \
         empty changed list is what a load that decoded nothing reports, and the cells are what a \
         load that reported correctly and handed back an empty world would fail. The empty cell on \
         the end stops a world filled with one block from answering the four named ones and passing"
    );
    Ok(())
}

#[test]
fn the_committed_appearance_revision_3_save_really_does_hold_all_four_of_the_shipped_blocks()
-> TestResult {
    let needed = requirements(&appearance_revision_3_save()?)?;

    let mut names: Vec<String> = needed
        .names()
        .map(|name| name.as_str().to_owned())
        .collect();
    names.sort();

    assert_eq!(
        names, HELD_BY_THE_SAVE,
        "this fixture cannot be minted again — the tree that could write it no longer exists — and \
         both readings above are claims about the blocks it holds. A save needing nothing produces \
         an empty verdict, which is exactly the value the first of them asserts for two of its \
         three lists, so if this file's name table were ever lost that reading would go on \
         agreeing while comparing four blocks against none of them"
    );
    Ok(())
}

/// What the four named cells and the empty one must answer once the save has
/// been loaded, in the order [`CELLS`] states them.
fn the_cells_it_was_left_in() -> Vec<String> {
    HELD_BY_THE_SAVE
        .into_iter()
        .chain([NOTHING])
        .map(str::to_owned)
        .collect()
}

/// Every block the committed save holds, as a `BlockName`, ascending.
///
/// # Errors
///
/// Returns an error if one of the four is not a namespaced id.
fn every_block_the_save_holds() -> Result<Vec<BlockName>, Box<dyn Error>> {
    HELD_BY_THE_SAVE
        .iter()
        .map(|name| Ok(BlockName::parse(name)?))
        .collect()
}

/// The save the readings above are judged against.
///
/// # Errors
///
/// Returns an error if the crate's manifest directory cannot be located.
fn appearance_revision_3_save() -> Result<PathBuf, Box<dyn Error>> {
    Ok(Path::new(env!("CARGO_MANIFEST_DIR")).join(APPEARANCE_REVISION_3_SAVE))
}

/// A registry holding what the shipped content declares.
///
/// **The shipped root and not a fixture's**, which is the opposite of the choice
/// `shipped_declarations_and_a_revision_3_save.rs` makes for its opacity reading
/// and is right for the same reason that one is. What this file is about is the
/// upgrade a player performs against the content the game ships, so the shipped
/// declarations are the subject rather than an incidental dependency: a reading
/// taken against a fixture registry would be about a world nobody has.
///
/// It follows that phase 3, which declares an opacity on the shipped sea, moves
/// water's appearance fold again — and leaves every reading here true, because
/// water is already in this list and its behaviour does not move.
///
/// # Errors
///
/// Returns an error if the shipped root cannot be located or is refused.
fn shipped_registry() -> Result<BlockRegistry, Box<dyn Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .ok_or("the crate manifest directory has no repository root above it")?
        .join("content")
        .join("base");
    let mut registry = BlockRegistry::new();
    registry.apply(&LuauFileDefinitionSource::new(root))?;
    Ok(registry)
}
