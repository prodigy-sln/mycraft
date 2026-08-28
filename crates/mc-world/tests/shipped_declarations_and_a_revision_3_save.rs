//! A world saved while a block's declared behaviour was recorded under revision
//! 3, opened after the ascent joined that list, and what it is told about its
//! blocks.
//!
//! `shipped_declarations_and_a_revision_2_save.rs` is this same file one move
//! back, and it cannot answer for this one: its fixture is stale in the behaviour
//! record by *two* list growths, so a fold that appended the ascent and forgot
//! the revision byte — or moved the byte and forgot the ascent — disagrees with
//! it either way and it reports the same four names for both. What is needed to
//! separate this move from the last one is a save that agreed with the tree
//! exactly, right up until the ascent was appended.
//!
//! # The fixture, and the one thing that would stop it being evidence
//!
//! `tests/fixtures/world_saved_against_behaviour_revision_3.mcw` was written on
//! **2026-08-27**, from the repository at **`6c2ed61`**, against `content/base/`
//! exactly as that tree shipped it — behaviour revision **3**, appearance
//! revision **3**, water declaring `move_resistance = 0.5` and `swim_ascent =
//! 3.5` — by the shipped writer of that day, holding all four base blocks:
//! `base:dirt` at `(1, 1, 1)`, `base:grass` at `(2, 3, 4)`, `base:stone` at
//! `(5, 8, 13)` and `base:water` at `(15, 34, 15)`.
//!
//! **It is not regenerated, and the day it is, this file stops being evidence
//! about anything.** A save this suite wrote from the declarations under test
//! would agree with them by construction and could not fail — which is the rule
//! both older fixtures' own files state, and the reason all three are committed
//! rather than minted.
//!
//! It had to be minted *before* the behaviour list grew, and there is no second
//! chance at it. `BEHAVIOUR_REVISION` is a compile-time constant, so a save
//! written at test runtime always carries whatever revision the tree it runs on
//! is at; once that constant moves, no run can produce a revision-3 save again.
//!
//! **At the moment it was minted its behaviour records were identical to a save
//! minted today**, which is not a defect in the fixture but the whole of what
//! makes these readings falsifiable: every one of them is red until the fold
//! actually grows, and none of them could have been written afterwards and still
//! say that.
//!
//! # What the two records are supposed to do here, and why it takes a pair
//!
//! Between the day this fixture was written and today, the behaviour list gained
//! the ascent a medium carries a swimmer at, and the appearance list gained
//! nothing. So **every block this save holds behaves differently and not one of
//! them looks different**, and that asymmetry is the assertion.
//!
//! The verdict alone cannot see it. `resolve` asks about behaviour first and
//! answers alone, so a block whose behaviour moved never reaches the `retextured`
//! list whatever its appearance did — an implementation that bumped the
//! appearance byte along with the behaviour one produces a verdict identical to
//! one that left it alone. The appearance reading below therefore goes to the
//! recorded hashes directly and compares them against a save minted today, which
//! is the only place the standing-still half is visible at all.
//!
//! # This is the third consecutive move of the behaviour byte
//!
//! `targetable`, then the two properties that make a volume a medium, and now the
//! ascent. So a player who saved under any of those builds, quit normally, and
//! opens their world under this one **is told again** — which is the designed
//! cost of a list that grew rather than a migration defect, and is survivable for
//! the reason it was survivable the last two times: such a save loads and names
//! its blocks instead of being refused.
//!
//! # An empty verdict is what the wrong answer looks like too
//!
//! "No block is missing" is an absence, and a comparison against it passes over a
//! save that needs no blocks at all just as happily as over one whose blocks all
//! resolve. So the fixture's own name table is asserted beside the readings that
//! rest on it: without that, a fixture that lost its table would read as agreement
//! forever.

mod common;
mod luau_common;

use std::error::Error;
use std::path::{Path, PathBuf};

use common::persistence::{answer_at, saved_requirements, world_at, world_holding};
use common::{TestResult, content_root};
use luau_common::{QUARTZ, declaration_of, raw_field, text_field};
use mc_core::block::BlockRegistry;
use mc_core::id::BlockName;
use mc_world::content::LuauFileDefinitionSource;
use mc_world::persistence::{
    self, Acceptance, LoadError, RegistryVerdict, SaveRequirements, requirements, resolve,
};
use mc_world::world::WorldPos;
use tempfile::TempDir;

/// The save every reading here is judged against, written under behaviour
/// revision 3 and appearance revision 3.
const REVISION_3_SAVE: &str = "tests/fixtures/world_saved_against_behaviour_revision_3.mcw";

/// Every block that save holds, in the ascending order a requirements report
/// lists them in.
const HELD_BY_THE_REVISION_3_SAVE: [&str; 4] =
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
fn every_block_of_a_save_written_before_the_ascent_joined_the_list_is_reported_as_behaving_differently()
-> TestResult {
    let needed = requirements(&revision_3_save()?)?;

    let verdict = resolve(&needed, &shipped_registry()?);

    assert_eq!(
        verdict,
        RegistryVerdict {
            missing: Vec::new(),
            changed: every_block_the_save_holds()?,
            retextured: Vec::new(),
        },
        "this save was written while the behaviour list held everything but the rate a volume \
         carries a swimmer upward at, and a fold over the grown list disagrees with the recorded \
         one for every block whatever that block declares — dirt and stone say nothing about \
         swimming and are told all the same, because the revision byte is the first thing folded. \
         That is what a player pays for a behaviour list that grows, and it is the third time they \
         pay it. The verdict is compared whole because the near miss is the one where nothing was \
         appended at all: a list that grew no field reports **no** block changed, and `changed: []` \
         is what an absence assertion here would have accepted"
    );
    Ok(())
}

#[test]
fn that_save_loads_with_its_changed_blocks_accepted_and_the_world_it_holds_comes_back_whole()
-> TestResult {
    let loaded = persistence::load_world(
        &revision_3_save()?,
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
        (every_block_the_save_holds()?, the_cells_it_was_left_in()),
        "a player who has updated their content opens the world they built: they are told every \
         block in it behaves differently and they get their world back, block for block, in the \
         cells they left them in. **Both halves, because either alone is satisfied by a failure**: \
         a load that reported the four names and handed back an empty world satisfies the report, \
         and one that loaded silently satisfies the world. **And this is a second entry point onto \
         the path the verdict reading above already walks** — `load_world` reports its own changed \
         list rather than the one `resolve` answered, and a list assembled correctly in one and \
         lost in the other is invisible to the reading that goes through `resolve`. The empty cell \
         on the end is what stops a world filled with one block from answering the four named ones \
         and passing"
    );
    Ok(())
}

#[test]
fn that_save_is_refused_and_told_every_block_when_only_unchanged_blocks_are_accepted() -> TestResult
{
    let refused = persistence::load_world(
        &revision_3_save()?,
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
         broke, while this one disagrees with an implementation that appended nothing by four names"
    );
    Ok(())
}

#[test]
fn every_block_of_that_save_is_judged_to_look_different_as_well_as_to_behave_differently()
-> TestResult {
    let recorded_then = requirements(&revision_3_save()?)?;
    let recorded_now = saved_today()?;

    let moved: Vec<(String, bool, bool)> = HELD_BY_THE_REVISION_3_SAVE
        .into_iter()
        .map(|name| how_the_records_differ(name, &recorded_then, &recorded_now))
        .collect();

    assert_eq!(
        moved,
        HELD_BY_THE_REVISION_3_SAVE
            .into_iter()
            .map(|name| (name.to_owned(), true, true))
            .collect::<Vec<(String, bool, bool)>>(),
        "**Both halves now, and the second half is new.** This save was written under behaviour \
         revision 3 and appearance revision 3; the behaviour list had already grown an ascent past \
         it, and the appearance list has now grown a degree of opacity — so every one of these four \
         blocks both behaves differently and looks different to a save minted today. The reading is \
         kept as a comparison against a fresh mint rather than as a claim about one list, because \
         that is what would report an appearance byte bumped without the list growing and a list \
         grown without the byte moving. It is the arm that must not be quietly repaired by widening: \
         `no block looks different` was true of this fixture across two consecutive behaviour moves, \
         and the moment it stops being true is the moment a reader has to be told which change made \
         it stop. Every block is named in the answer rather than counted, so a fold that got one of \
         the four wrong says which"
    );
    Ok(())
}

#[test]
fn the_committed_revision_3_save_really_does_hold_all_four_of_the_blocks_the_base_game_ships()
-> TestResult {
    let needed = requirements(&revision_3_save()?)?;

    let mut names: Vec<String> = needed
        .names()
        .map(|name| name.as_str().to_owned())
        .collect();
    names.sort();

    assert_eq!(
        names, HELD_BY_THE_REVISION_3_SAVE,
        "this fixture cannot be minted again, and every reading above is a claim about the blocks \
         it holds. A verdict naming nothing is what a save needing nothing produces, so if this \
         file's name table were ever lost the readings would go on agreeing while comparing four \
         blocks against none of them"
    );
    Ok(())
}

/// What the four named cells and the empty one must answer once the save has been
/// loaded, in the order [`CELLS`] states them.
///
/// The empty cell's answer is on the end rather than asserted separately, so that
/// a world filled with one block fails the same comparison the four named cells
/// are read through.
fn the_cells_it_was_left_in() -> Vec<String> {
    HELD_BY_THE_REVISION_3_SAVE
        .into_iter()
        .chain([NOTHING])
        .map(str::to_owned)
        .collect()
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
        CELLS.into_iter().zip(HELD_BY_THE_REVISION_3_SAVE).collect();
    let world = world_holding(&placed, &registry)?;
    saved_requirements(&directory, &world, &registry)
}

/// Every block the committed save holds, as a `BlockName`, ascending.
///
/// # Errors
///
/// Returns an error if one of the four is not a namespaced id.
fn every_block_the_save_holds() -> Result<Vec<BlockName>, Box<dyn Error>> {
    HELD_BY_THE_REVISION_3_SAVE
        .iter()
        .map(|name| Ok(BlockName::parse(name)?))
        .collect()
}

/// The save the readings above are judged against.
///
/// # Errors
///
/// Returns an error if the crate's manifest directory cannot be located.
fn revision_3_save() -> Result<PathBuf, Box<dyn Error>> {
    Ok(Path::new(env!("CARGO_MANIFEST_DIR")).join(REVISION_3_SAVE))
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

/// What one block of the save answered once it was loaded: its name, and the
/// degree of light the registry says it stops.
type Opaqueness = (String, u32);

/// The degree of light a block stops where nothing has ever said otherwise.
///
/// Written out here rather than read from `Opacity::OPAQUE`, which is the value
/// under test: an expectation taken from it agrees with whatever it becomes. Held
/// by its bits for the reason `luau_declaration_opacity.rs` holds one — the
/// record this save folds is folded by bits, and `-0.0 == 0.0` is true.
const STOPPING_ALL_OF_IT: f32 = 1.0;

/// A registry built by **reading declarations that state no degree**, holding
/// the four names this save needs.
///
/// **Read through the Luau loader and never assembled from `BlockDefinition`
/// literals**, and that is the difference between a reading and a tautology. A
/// registry built in memory carries whatever `Opacity` the fixture typed, so an
/// assertion that its blocks are opaque asserts the fixture — measured, and it is
/// how the first version of the reading below came to have a falsifier that could
/// not reach it: the loader's absent-field default was mutated and this test
/// stayed green, because the loader never ran.
///
/// **Its own root and not `content/base`**, for the reason this file's reading
/// states: phase 3 declares an opacity on the shipped sea, so a reading taken
/// there would stop being about an all-opaque registry on the commit that needs
/// it.
///
/// `occludes = false` is stated outright on all four. Without it, a solid block
/// occludes by its own solidity, and a mutated default below one would make this
/// root **refused** rather than opaque — which reddens the reading for the wrong
/// reason and hides what it is watching for.
fn registry_stating_no_degree(directory: &TempDir) -> Result<BlockRegistry, Box<dyn Error>> {
    let declared: Vec<(&str, String)> = HELD_BY_THE_REVISION_3_SAVE
        .iter()
        .map(|name| {
            (
                *name,
                declaration_of(&[
                    text_field("name", name),
                    text_field("texture", QUARTZ),
                    raw_field("solid", "true"),
                    raw_field("occludes", "false"),
                ]),
            )
        })
        .collect();
    let files: Vec<(&str, String)> = declared
        .into_iter()
        .map(|(name, body)| (declaration_file_of(name), body))
        .collect();
    let root = content_root(directory, &files)?;
    let mut registry = BlockRegistry::new();
    registry.apply(&LuauFileDefinitionSource::new(root))?;
    Ok(registry)
}

/// The file name a fixture declares `name` in: its bare half, which is unique
/// across the four.
fn declaration_file_of(name: &str) -> &'static str {
    match name {
        "base:dirt" => "dirt.luau",
        "base:grass" => "grass.luau",
        "base:stone" => "stone.luau",
        _ => "water.luau",
    }
}

/// The degree every block of the save is held at once it is loaded, in the order
/// [`HELD_BY_THE_REVISION_3_SAVE`] names them.
///
/// # Errors
///
/// Returns an error if a name does not parse or the registry does not hold it.
fn how_opaque_each_loaded_block_is() -> Result<Vec<Opaqueness>, Box<dyn Error>> {
    let directory = TempDir::new()?;
    let registry = registry_stating_no_degree(&directory)?;
    persistence::load_world(&revision_3_save()?, &registry, Acceptance::ChangedBlocksToo)?;
    let mut held = Vec::new();
    for name in HELD_BY_THE_REVISION_3_SAVE {
        let definition = registry.resolve(&BlockName::parse(name)?)?;
        held.push((name.to_owned(), definition.opacity.get().to_bits()));
    }
    Ok(held)
}

#[test]
fn a_world_saved_before_the_degree_existed_opens_with_every_block_stopping_all_the_light()
-> TestResult {
    let held = how_opaque_each_loaded_block_is()?;

    assert_eq!(
        held,
        HELD_BY_THE_REVISION_3_SAVE
            .into_iter()
            .map(|name| (name.to_owned(), STOPPING_ALL_OF_IT.to_bits()))
            .collect::<Vec<Opaqueness>>(),
        "this file is the only genuinely pre-field save in the repository, and what a player who \
         has one is owed is that it opens and looks the way it looked. Nothing in the save says \
         anything about a degree — the field did not exist when it was written — so every block \
         it holds takes the constant an unstated field means, and a default derived from \
         anything at all would make some of these four invisible in a world somebody built. The          registry is built by **reading four declarations that state no degree**, so the loader's          own default is in the path — an assertion over `BlockDefinition` literals would be an          assertion about the fixture, and was. \
         **The registry is one this test declares rather than the shipped one**, and that is not \
         tidiness: phase 3 of this spec declares an opacity on the shipped sea, so a reading \
         taken against `content/base` would stop being a reading of an all-opaque registry on \
         the very commit that needs it, and would then be repaired by weakening it. Every block \
         is named in the answer rather than counted, so a default that reached three of the four \
         says which one it missed"
    );
    Ok(())
}
