//! What a refusal says twice, and what a person needs said once.
//!
//! # A layer that quotes its own cause says it again under a chain walk
//!
//! A report is the failure and every failure beneath it, joined outermost
//! first. A layer whose own sentence already interpolates the failure beneath it
//! therefore states that failure and then has it rendered again: the save's path
//! twice and the reason three times, the block a world could not be built
//! without three times, and the sentence offering a way out stranded in the
//! middle of the report rather than after the refusal it answers.
//!
//! None of the three is a wording somebody chose. Each is a joiner that used to
//! live inside a format string because nothing walked the chain, and each stops
//! being one now that something does.
//!
//! # Counting, because searching cannot see "once"
//!
//! A report saying the reason three times contains it exactly as readily as one
//! saying it once, so every scenario here counts occurrences rather than asking
//! whether a word is present. The whole of what was written is compared beside
//! the counts: a count cannot see a separator hung on an empty layer, and a
//! comparison alone would go on agreeing if the reader quietly stopped filling
//! the path in, because both sides would move together.
//!
//! # Every expectation is asked of the thing that produced it
//!
//! No refusal's wording is spelled out here. The reason a save could not be read
//! is asked of the reader, and the block a world could not be built without is
//! asked of the generator — both through a second call that reaches the failure
//! without going anywhere near the reporting. So a reworded refusal moves both
//! sides together and a *dropped* one moves only the printed side, which is the
//! asymmetry a snapshotted string does not have.

mod support;

#[path = "support/persistence.rs"]
mod persistence;

use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use mc_client::launch::simulation_to_play;
use mc_client::startup::PreparationError;
use mc_core::block::BlockRegistry;
use mc_core::id::BlockName;
use mc_render::window::Ending;
use mc_sim::replay::{ReplayWorld, WorldGenError};
use mc_world::content::LuauFileDefinitionSource;
use mc_world::persistence::{Acceptance, LoadError, SavedPlayer, load_world, save_world};
use mc_world::section::SectionError;
use mc_world::world::{VoxelWorld, WorldPos};
use tempfile::TempDir;

use persistence::{COLUMNS, GROUND, declared, registry_of, save_in, with_the_replay_blocks};
use support::{TestResult, content};

/// Nothing here is about the flag the player may pass, so the answer is the same
/// in all three: a save is read on its own terms.
const ACCEPTING: Acceptance = Acceptance::OnlyUnchangedBlocks;

/// Exactly what a player has to type to load a save whose blocks have changed.
///
/// Spelled out rather than read from the client for the reason the acceptance
/// suite gives at its own copy: a test reading the client's own constant would
/// agree with a message quoting a spelling nothing accepts.
const LOAD_CHANGED_BLOCKS: &str = "--load-changed-blocks";

/// What the level that knows where a save is says about one it could not read,
/// with the reason no longer written into it.
///
/// The path is what this level knows and the cause does not, which is why the
/// sentence survives at all; the reason is what the level beneath it knows, and
/// saying it here as well is what makes a reader read it twice.
const COULD_NOT_BE_READ: &str = " could not be read";

/// What the level that knows *why* a world was being built says, with the cause
/// no longer written into it.
const NO_WORLD_COULD_BE_GENERATED: &str = "a new world could not be generated";

/// A file that is present where a save belongs and is not one.
///
/// Present is the whole precondition: nothing at the path is a first launch and
/// generates a world, and this scenario is about a save that is there and
/// refuses.
const NOT_A_SAVE: &[u8] = b"this file is not a save";

/// The one solid block the fixture root declares, and the file it declares it
/// in.
///
/// Deliberately not one of the four the generator places: a root declaring one
/// of those is a root the generator can still partly build a world from, and
/// "no world can be generated here" would stop being true of it.
const BEACON_FILE: &str = "zz-beacon.luau";
const BEACON_DECLARATION: &str =
    "return {\n\tname = 'fixture:beacon',\n\ttexture = 'fixture:beacon',\n\tsolid = true,\n}\n";

/// The files the shipped root declares the generator's own blocks in.
///
/// All four go. Leaving one behind leaves a root that fails later in generation
/// instead of at the first block placed, which is still a refusal but about a
/// different block than the one the fixture means.
const THE_GENERATORS_DECLARATIONS: [&str; 4] =
    ["dirt.luau", "grass.luau", "stone.luau", "water.luau"];

/// The block this save holds, and which is declared differently by the time the
/// save is read again.
const REDECLARED: &str = "fixture:redeclared";

/// Where it stands in the saved world. Nothing asserts the cell; a save has to
/// hold the block somewhere for its name to be in the table at all.
const WHERE_IT_STANDS: WorldPos = WorldPos { x: 1, y: 1, z: 1 };

/// Where that save records the player. Nothing here asserts it, and a save
/// records somebody.
const RECORDED_PLAYER: SavedPlayer = SavedPlayer {
    position: [8.5, 12.25, 8.5],
    yaw: 0.75,
    pitch: -0.25,
};

#[test]
fn a_save_that_cannot_be_read_is_named_once_and_its_reason_given_once() -> TestResult {
    let directory = TempDir::new()?;
    let save = a_file_that_is_not_a_save(&directory)?;
    let registry = Arc::new(registry_of(vec![declared(GROUND, true)?])?);
    let named = save.display().to_string();
    let reason = why_the_save_cannot_be_read(&save, &registry)?;
    let whole = format!("mycraft: {named}{COULD_NOT_BE_READ}: {reason}\n");

    let said = refusal_shown_for(&turned_away_by(&save, &registry)?)?;

    assert_eq!(
        (
            occurrences_of(&said, &named),
            occurrences_of(&said, &reason),
            occurrences_of(&said, LOAD_CHANGED_BLOCKS),
            said.as_str(),
        ),
        (1, 1, 0, whole.as_str()),
        "a player is told which file could not be read and why, and each of the two once: the \
         level that knows the path says the path, the level beneath it says the reason, and a \
         level saying both says one of them twice over. The counts are asked beside the whole \
         comparison because a count cannot see a separator hung on an empty layer, and the \
         comparison alone would go on agreeing if the reader stopped filling the path in. No flag \
         turns a file that is not a save into one, so none is offered — a way out that is not one \
         sends a player round the same refusal a second time"
    );
    Ok(())
}

#[test]
fn a_world_that_could_not_be_generated_names_the_missing_block_once() -> TestResult {
    let root = a_root_no_world_can_be_generated_from()?;
    let registry = Arc::new(registry_over(root.path())?);
    let (missing, reason) = the_block_the_generator_could_not_place(&registry)?;
    let nowhere = TempDir::new()?;
    let whole = format!("mycraft: {NO_WORLD_COULD_BE_GENERATED}: {reason}\n");

    let said = refusal_shown_for(&turned_away_by(&save_in(&nowhere), &registry)?)?;

    assert_eq!(
        (
            occurrences_of(&said, missing.as_str()),
            occurrences_of(&said, LOAD_CHANGED_BLOCKS),
            said.as_str(),
        ),
        (1, 0, whole.as_str()),
        "there is no save here, so a world has to be built and cannot: the content root declares \
         no `{name}`. That name is the one thing a content author can act on and it is written \
         once — a report repeating it is a report in which the sentence saying why a world was \
         being built at all is the part that gets lost. No flag makes an undeclared block \
         declared, so none is offered",
        name = missing.as_str()
    );
    Ok(())
}

#[test]
fn a_save_refused_only_for_redeclared_blocks_offers_the_way_out_once_after_it() -> TestResult {
    let redeclared = a_save_whose_block_was_redeclared()?;
    let save = save_in(&redeclared.directory);
    let named = save.display().to_string();
    let reason = why_the_redeclared_save_is_refused(&save, &redeclared.registry)?;
    let turned_away = turned_away_by(&save, &redeclared.registry)?;
    let way_out = turned_away.way_out();
    let whole = format!("mycraft: {named}{COULD_NOT_BE_READ}: {reason}{way_out}\n");

    let said = refusal_shown_for(&turned_away)?;

    assert_eq!(
        (
            occurrences_of(&said, LOAD_CHANGED_BLOCKS),
            follows(&said, LOAD_CHANGED_BLOCKS, &reason),
            said.as_str(),
        ),
        (1, true, whole.as_str()),
        "the player's way back into their world is one sentence, and where it sits decides whether \
         they find it: said once, after the refusal it answers, it reads as the answer to what \
         they just read. Said in the middle of a report that then goes on restating the refusal, \
         it reads as part of the wreckage. A cause says what happened and a way out says what to \
         do about it, so the way out comes after the whole of the first"
    );
    Ok(())
}

/// What the client writes when a launch is turned away: the whole chain, and
/// then the way out where the refusal has one.
///
/// **Rendered through the shipped reporting rather than composed here.** What a
/// turned-away player gets is one block of text on their terminal, and a suite
/// that built its own would agree with itself while the client printed something
/// else.
///
/// # Errors
///
/// Returns an error if the sink refuses the bytes, which a `Vec` does not, or if
/// what was written is not text.
fn refusal_shown_for(turned_away: &PreparationError) -> Result<String, Box<dyn Error>> {
    support::reported(&Ending::failed(turned_away, &turned_away.way_out()))
}

/// What the client makes of the save at `save`, read against `registry` — or
/// rather, the refusal it gives instead of a simulation.
///
/// # Errors
///
/// Returns an error if the launch started. A launch that was not turned away has
/// no refusal to read, and every word a scenario asks for would then be missing
/// for a reason that has nothing to do with what is printed.
fn turned_away_by(
    save: &Path,
    registry: &Arc<BlockRegistry>,
) -> Result<PreparationError, Box<dyn Error>> {
    match simulation_to_play(mc_sim::REPLAY_SEED, Arc::clone(registry), save, ACCEPTING) {
        Ok(_) => Err(format!(
            "this scenario needs the launch reading {} to be turned away, and it started a \
             simulation instead",
            save.display()
        )
        .into()),
        Err(turned_away) => Ok(turned_away),
    }
}

/// Why the save at `save` cannot be read, asked of the reader itself.
///
/// # Errors
///
/// Returns an error if the save was read after all.
fn why_the_save_cannot_be_read(
    save: &Path,
    registry: &BlockRegistry,
) -> Result<String, Box<dyn Error>> {
    match load_world(save, registry, ACCEPTING) {
        Ok(_) => Err(read_after_all(save)),
        Err(refused) => Ok(refused.to_string()),
    }
}

/// Why the save at `save` is refused when nothing about it is wrong except that
/// its blocks have been redeclared, asked of the reader itself.
///
/// **The shape is checked and not only the refusal.** A save refused for a block
/// the registry does not hold at all is not something the flag can load, so a
/// fixture that drifted into that would be asking for a way out where there is
/// none — and the scenario would be red for a reason nobody chose.
///
/// # Errors
///
/// Returns an error if the save was read, or if it was refused for anything
/// beyond blocks whose declarations changed.
fn why_the_redeclared_save_is_refused(
    save: &Path,
    registry: &BlockRegistry,
) -> Result<String, Box<dyn Error>> {
    let refused = match load_world(save, registry, ACCEPTING) {
        Ok(_) => return Err(read_after_all(save)),
        Err(refused) => refused,
    };
    let LoadError::Unresolvable { missing, changed } = &refused else {
        return Err(format!(
            "this scenario needs a save refused only because blocks it holds were redeclared, and \
             the reader refused it with: {refused}"
        )
        .into());
    };
    if !missing.is_empty() || changed.is_empty() {
        return Err(format!(
            "the way out is offered only where the player saying yes is all that stands between \
             the save and a world; this save is missing {} of its blocks and holds {} redeclared \
             ones",
            missing.len(),
            changed.len()
        )
        .into());
    }
    Ok(refused.to_string())
}

/// What to say when a scenario needing a refused save was handed a readable one.
fn read_after_all(save: &Path) -> Box<dyn Error> {
    format!(
        "this scenario needs the save at {} to be refused, and it was read",
        save.display()
    )
    .into()
}

/// The block the generator asked `registry` for and did not get, and the words
/// it refused in.
///
/// **Asked through the generator rather than spelled out**, so the name a
/// scenario counts is the one the refusal is genuinely about and does not go
/// stale the day the sentence is reworded.
///
/// **It is a block the registry does not declare, never a name that would not
/// parse.** On the unparseable path a reader sees the text twice already,
/// because that refusal and the naming refusal beneath it each quote it — which
/// is two layers naming one subject rather than a layer stating its own cause,
/// and is not what this scenario is about.
///
/// # Errors
///
/// Returns an error if the generator built a world after all, or if it refused
/// for anything other than a block the registry does not declare — in which case
/// the scenario has no missing block to count.
fn the_block_the_generator_could_not_place(
    registry: &BlockRegistry,
) -> Result<(BlockName, String), Box<dyn Error>> {
    let refused = match ReplayWorld::generate(mc_sim::REPLAY_SEED, registry) {
        Ok(_) => {
            return Err(
                "this scenario's content root generates a world after all, so a launch \
                 reaching the generator would not refuse and there would be no block for the \
                 report to name"
                    .into(),
            );
        }
        Err(refused) => refused,
    };
    let WorldGenError::Section(SectionError::UnknownBlock { name }) = &refused else {
        return Err(format!(
            "this scenario is about a block the content root does not declare, and the generator \
             refused with something else: {refused}"
        )
        .into());
    };
    Ok((name.clone(), refused.to_string()))
}

/// A save present where a save belongs, holding bytes that are not one.
///
/// # Errors
///
/// Returns an error if the directory above it cannot be made or the write fails.
fn a_file_that_is_not_a_save(directory: &TempDir) -> Result<PathBuf, Box<dyn Error>> {
    let save = save_in(directory);
    let holding = save
        .parent()
        .ok_or("a save path with no directory above it")?;
    fs::create_dir_all(holding)?;
    fs::write(&save, NOT_A_SAVE)?;
    Ok(save)
}

/// The shipped content root with one solid block of this file's own added and
/// every block the generator places taken out.
///
/// # Errors
///
/// Returns an error if the root cannot be copied, if the shipped root already
/// declares the beacon, or if it does not declare one of the four.
fn a_root_no_world_can_be_generated_from() -> Result<content::ContentRoot, Box<dyn Error>> {
    content::shipped_copy()?
        .declaring_block(BEACON_FILE, BEACON_DECLARATION)?
        .not_declaring_blocks(&THE_GENERATORS_DECLARATIONS)
}

/// A registry holding exactly what the content root at `root` declares.
///
/// # Errors
///
/// Returns an error if the root cannot be read or does not register.
fn registry_over(root: &Path) -> Result<BlockRegistry, Box<dyn Error>> {
    let mut registry = BlockRegistry::new();
    registry.apply(&LuauFileDefinitionSource::new(root.to_owned()))?;
    Ok(registry)
}

/// A save holding one block, and the registry a client would read it against
/// after that block was redeclared.
#[derive(Debug)]
struct ARedeclaredSave {
    directory: TempDir,
    registry: Arc<BlockRegistry>,
}

/// That save, written against one declaration and read against another.
///
/// **The redeclaration is of behaviour rather than of appearance**: solidity,
/// which is what a save's table records a declaration by. The texture is derived
/// from the name at both ends, so nothing here changes a block's *look* — an
/// appearance-only change is loaded without asking, and a fixture making one
/// would be building the opposite of what this scenario is about.
///
/// The registry it is read against also holds the blocks the replay generator
/// places, so that a client has a solid block to hold and is turned away by the
/// save rather than before it ever reaches one.
///
/// # Errors
///
/// Returns an error if either registry refuses its declarations, if the world
/// cannot be built, or if the save cannot be written.
fn a_save_whose_block_was_redeclared() -> Result<ARedeclaredSave, Box<dyn Error>> {
    let written = registry_of(vec![declared(REDECLARED, true)?])?;
    let mut blocks = VoxelWorld::empty(COLUMNS);
    blocks.set_block(WHERE_IT_STANDS, &BlockName::parse(REDECLARED)?, &written)?;
    let directory = TempDir::new()?;
    save_world(&save_in(&directory), &blocks, RECORDED_PLAYER, &written)?;

    let now = with_the_replay_blocks(registry_of(vec![declared(REDECLARED, false)?])?)?;
    Ok(ARedeclaredSave {
        directory,
        registry: Arc::new(now),
    })
}

/// How many times `said` holds `needle`.
fn occurrences_of(said: &str, needle: &str) -> usize {
    said.matches(needle).count()
}

/// Whether `said` holds `needle` after the last `after` it holds.
///
/// The *last*, because a report that restates the refusal below the way out has
/// put the answer before part of the question — which is the arrangement this
/// asks about and the one a report reading forwards cannot recover from.
fn follows(said: &str, needle: &str, after: &str) -> bool {
    match (said.rfind(needle), said.rfind(after)) {
        (Some(offered), Some(answered)) => offered > answered,
        _ => false,
    }
}
