//! A world saved while a block's declared behaviour was recorded under the list
//! before this one, opened afterwards, and what it is told about its blocks.
//!
//! `shipped_declarations_and_an_older_save.rs` asks this of a save written while
//! the blocks were still TOML — behaviour revision 1, and an appearance record
//! that predates the six texture keys. That fixture cannot answer the question
//! this file exists for, because **both** of its records are stale: it can say
//! nothing about a behaviour list that grew while the appearance list stood
//! perfectly still, which is the whole shape of this change.
//!
//! # The fixture, and the one thing that would stop it being evidence
//!
//! `tests/fixtures/world_saved_against_behaviour_revision_2.mcw` was written on
//! **2026-08-25**, from the repository at **`204b50c`**, against
//! `content/base/` exactly as that tree shipped it — behaviour revision **2**,
//! appearance revision **3** — by the shipped writer of that day, holding all
//! four base blocks: `base:dirt` at `(1, 1, 1)`, `base:grass` at `(2, 3, 4)`,
//! `base:stone` at `(5, 8, 13)` and `base:water` at `(15, 34, 15)`.
//!
//! **It is not regenerated, and the day it is, this file stops being evidence
//! about anything.** A save this suite wrote from the declarations under test
//! would agree with them by construction and could not fail — which is the same
//! rule the older fixture's own file states, and the reason both are committed
//! rather than minted.
//!
//! It had to be minted *before* the behaviour list grew, and there is no second
//! chance at it. `BEHAVIOUR_REVISION` is a compile-time constant, so a save
//! written at test runtime always carries whatever revision the tree it runs on
//! is at; once that constant moves, no run can produce a revision-2 save again.
//!
//! # What the two records are supposed to do here, and why it takes a pair
//!
//! Between the day this fixture was written and today, the behaviour list gained
//! the two properties that make a block's volume a medium, and the appearance
//! list gained nothing. So **every block this save holds behaves differently and
//! not one of them looks different**, and that asymmetry is the assertion.
//!
//! The verdict alone cannot see it. `resolve` asks about behaviour first and
//! answers alone, so a block whose behaviour moved never reaches the `retextured`
//! list whatever its appearance did — an implementation that bumped the
//! appearance byte along with the behaviour one produces a verdict identical to
//! one that left it alone. The appearance reading below therefore goes to the
//! recorded hashes directly and compares them against a save minted today, which
//! is the only place the standing-still half is visible at all.
//!
//! # This is the second consecutive move of the behaviour byte
//!
//! The last one was `targetable`. So a player who saved after that build, quit
//! normally, and opens their world under this one **is told again** — which is
//! the designed cost of a list that grew rather than a migration defect, and is
//! survivable for the reason it was survivable last time: such a save loads and
//! names its blocks instead of being refused.
//!
//! # An empty verdict is what the wrong answer looks like too
//!
//! "No block is missing" is an absence, and a comparison against it passes over a
//! save that needs no blocks at all just as happily as over one whose blocks all
//! resolve. So the fixture's own name table is asserted beside the readings that
//! rest on it: without that, a fixture that lost its table would read as agreement
//! forever.

mod common;

use std::error::Error;
use std::path::{Path, PathBuf};

use common::TestResult;
use common::persistence::{answer_at, saved_requirements, world_at, world_holding};
use mc_core::block::BlockRegistry;
use mc_core::id::BlockName;
use mc_world::content::LuauFileDefinitionSource;
use mc_world::persistence::{
    self, Acceptance, LoadError, RegistryVerdict, SaveRequirements, requirements, resolve,
};
use mc_world::world::WorldPos;
use tempfile::TempDir;

/// The save every reading here is judged against, written under behaviour
/// revision 2 and appearance revision 3.
const REVISION_2_SAVE: &str = "tests/fixtures/world_saved_against_behaviour_revision_2.mcw";

/// Every block that save holds, in the ascending order a requirements report
/// lists them in.
const HELD_BY_THE_REVISION_2_SAVE: [&str; 4] =
    ["base:dirt", "base:grass", "base:stone", "base:water"];

/// Where each of those four sits in the saved world, in the same order.
///
/// Stated so that the load below is read for the world it produced rather than
/// only for the report it carried: a load that returned the right verdict over an
/// empty world would otherwise pass.
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
///
/// The control inside the readback: a load that filled its world with one block
/// would satisfy every named cell above and fail here.
const AN_EMPTY_CELL: WorldPos = world_at(3, 3, 3);

#[test]
fn every_block_of_a_save_written_under_the_older_behaviour_list_is_reported_as_behaving_differently()
-> TestResult {
    let needed = requirements(&revision_2_save()?)?;

    let verdict = resolve(&needed, &shipped_registry()?);

    assert_eq!(
        verdict,
        RegistryVerdict {
            missing: Vec::new(),
            changed: every_block_the_save_holds()?,
            retextured: Vec::new(),
        },
        "this save was written while the behaviour list held everything but the two properties \
         that say what a block's volume is to move through, and a fold over the grown list \
         disagrees with the recorded one for every block whatever either block declares. That is \
         what a player pays for a behaviour list that grows, and it is the second time they pay it \
         — the last was `targetable`. The verdict is compared whole because the near miss is the \
         one where nothing was folded at all: a list that grew no field reports **no** block \
         changed, and `changed: []` is what an absence assertion here would have accepted"
    );
    Ok(())
}

#[test]
fn that_save_loads_with_its_changed_blocks_accepted_and_the_world_it_holds_comes_back_whole()
-> TestResult {
    let loaded = persistence::load_world(
        &revision_2_save()?,
        &shipped_registry()?,
        Acceptance::ChangedBlocksToo,
    )?;

    let held: Vec<String> = CELLS
        .into_iter()
        .map(|at| answer_at(&loaded.world, at))
        .chain([answer_at(&loaded.world, AN_EMPTY_CELL)])
        .collect();

    assert_eq!(
        (loaded.changed, held),
        (
            every_block_the_save_holds()?,
            HELD_BY_THE_REVISION_2_SAVE
                .into_iter()
                .chain([NOTHING])
                .map(str::to_owned)
                .collect::<Vec<String>>()
        ),
        "a player who has updated their content opens the world they built: they are told every \
         block in it behaves differently and they get their world back, block for block, in the \
         cells they left them in. **Both halves, because either alone is satisfied by a failure**: \
         a load that reported the four names and handed back an empty world satisfies the report, \
         and one that loaded silently satisfies the world. The empty cell on the end is what stops \
         a world filled with one block from answering the four named ones and passing"
    );
    Ok(())
}

#[test]
fn that_save_is_refused_and_told_every_block_when_only_unchanged_blocks_are_accepted() -> TestResult
{
    let refused = persistence::load_world(
        &revision_2_save()?,
        &shipped_registry()?,
        Acceptance::OnlyUnchangedBlocks,
    );

    assert_eq!(
        refused.err(),
        Some(LoadError::Unresolvable {
            missing: Vec::new(),
            changed: every_block_the_save_holds()?,
        }),
        "somebody who asked to be stopped if anything moved is stopped, and told all four names \
         rather than the first one the reader happened to reach — the list is what they act on. \
         **This arm is where the evidence is**: under the accepting default a refusal is `None` \
         for any changed list at all, so a reading of that arm alone passes however badly the fold \
         broke, while this one disagrees with an implementation that folded nothing by four names"
    );
    Ok(())
}

#[test]
fn no_block_of_that_save_is_judged_to_look_different_while_every_one_of_them_behaves_differently()
-> TestResult {
    let recorded_then = requirements(&revision_2_save()?)?;
    let recorded_now = saved_today()?;

    let moved: Vec<(String, bool, bool)> = HELD_BY_THE_REVISION_2_SAVE
        .into_iter()
        .map(|name| how_the_records_differ(name, &recorded_then, &recorded_now))
        .collect();

    assert_eq!(
        moved,
        HELD_BY_THE_REVISION_2_SAVE
            .into_iter()
            .map(|name| (name.to_owned(), true, false))
            .collect::<Vec<(String, bool, bool)>>(),
        "the behaviour list grew two fields and the appearance list grew nothing, so every one of \
         these four blocks behaves differently and not one of them looks different. **The verdict \
         cannot say this and the hashes can**: behaviour is asked first and answers alone, so an \
         appearance byte bumped along with the behaviour one produces exactly the verdict a \
         correct implementation produces, and this comparison against a save minted today is the \
         only thing in the workspace that would report it. Every block is named in the answer \
         rather than counted, so a fold that got one of the four wrong says which"
    );
    Ok(())
}

#[test]
fn the_committed_save_really_does_hold_all_four_of_the_blocks_the_base_game_ships() -> TestResult {
    let needed = requirements(&revision_2_save()?)?;

    let mut names: Vec<String> = needed
        .names()
        .map(|name| name.as_str().to_owned())
        .collect();
    names.sort();

    assert_eq!(
        names, HELD_BY_THE_REVISION_2_SAVE,
        "this fixture cannot be minted again, and every reading above is a claim about the blocks \
         it holds. A verdict naming nothing is what a save needing nothing produces, so if this \
         file's name table were ever lost the readings would go on agreeing while comparing four \
         blocks against none of them"
    );
    Ok(())
}

/// How the two saves' records of `name` stand against each other: the name, then
/// whether its behaviour moved and whether its appearance did.
///
/// The name travels with the pair so a failure says *which* block disagreed
/// rather than only that one did.
fn how_the_records_differ(
    name: &str,
    then: &SaveRequirements,
    now: &SaveRequirements,
) -> (String, bool, bool) {
    let (then, now) = (recorded_in(then, name), recorded_in(now, name));
    // A save that does not name the block reports as both halves moved, which is
    // an answer no correct implementation gives and so cannot be mistaken for one.
    (
        name.to_owned(),
        then.map(|folds| folds.0) != now.map(|folds| folds.0) || then.is_none(),
        then.map(|folds| folds.1) != now.map(|folds| folds.1) || then.is_none(),
    )
}

/// The two folds `requirements` records for `name`, or nothing where the save
/// does not name it.
fn recorded_in(requirements: &SaveRequirements, name: &str) -> Option<(u64, u64)> {
    requirements
        .blocks()
        .iter()
        .find(|block| block.name.as_str() == name)
        .map(|block| (block.behaviour.get(), block.appearance.get()))
}

/// What a save written today against the shipped content records about the same
/// four blocks.
///
/// The oracle for the appearance half. It shares the writer with the fixture and
/// nothing else: what it can say is that the appearance list did not move between
/// the two, which is precisely the claim no verdict carries.
///
/// # Errors
///
/// Returns an error if the shipped root cannot be read, or if the save cannot be
/// written or read back.
fn saved_today() -> Result<SaveRequirements, Box<dyn Error>> {
    let directory = TempDir::new()?;
    let registry = shipped_registry()?;
    let placed: Vec<(WorldPos, &str)> =
        CELLS.into_iter().zip(HELD_BY_THE_REVISION_2_SAVE).collect();
    let world = world_holding(&placed, &registry)?;
    saved_requirements(&directory, &world, &registry)
}

/// Every block the committed save holds, as a `BlockName`, ascending.
///
/// # Errors
///
/// Returns an error if one of the four is not a namespaced id.
fn every_block_the_save_holds() -> Result<Vec<BlockName>, Box<dyn Error>> {
    HELD_BY_THE_REVISION_2_SAVE
        .iter()
        .map(|name| Ok(BlockName::parse(name)?))
        .collect()
}

/// The save the readings above are judged against.
///
/// # Errors
///
/// Returns an error if the crate's manifest directory cannot be located.
fn revision_2_save() -> Result<PathBuf, Box<dyn Error>> {
    Ok(Path::new(env!("CARGO_MANIFEST_DIR")).join(REVISION_2_SAVE))
}

/// A registry holding what the shipped content declares.
///
/// # Errors
///
/// Returns an error if the shipped root cannot be located or is refused.
fn shipped_registry() -> Result<BlockRegistry, Box<dyn Error>> {
    let root = common::repository_root()?.join("content").join("base");
    let mut registry = BlockRegistry::new();
    registry.apply(&LuauFileDefinitionSource::new(root))?;
    Ok(registry)
}
