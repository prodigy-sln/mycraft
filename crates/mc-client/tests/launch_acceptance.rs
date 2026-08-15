//! What the client does about a save whose blocks are no longer what they were:
//! the flag a player passes, and the message they are shown when they have not.
//!
//! # The refusal message is the whole user interface this decision has
//!
//! There is no dialog and no HUD in this build, so a player who is turned away
//! sees one line of text and nothing else. If it does not name **every** block
//! that changed, they cannot tell which mod to put back; if it does not say
//! exactly what to type, they cannot say yes. Either omission is a dead end no
//! amount of re-reading gets them out of, which is why the message is asserted
//! here rather than left to whatever `Display` derives.
//!
//! # The three scenarios are each other's controls, in both directions
//!
//! A client that always demanded acceptance would refuse the unchanged save; a
//! client that always assumed it would not refuse the changed one; and a client
//! that parsed the flag perfectly and then dropped it on the floor fails
//! whichever of the two the dropped value disagreed with. So the value driven
//! into every one of them comes out of the client's own parse of a real argv,
//! and each drives a save the whole way through the launch rather than stopping
//! at the parse.
//!
//! # Two blocks changed, and a third did not
//!
//! "Every changed block" is only observable against more than one, and "every"
//! is only distinguishable from "everything" against a block that did not
//! change. The unchanged one is what stops a report that simply lists the save's
//! whole table from reading as complete.

#[path = "support/persistence.rs"]
mod persistence;

use std::error::Error;
use std::sync::Arc;

use mc_client::launch::simulation_to_play;
use mc_client::startup::acceptance_from;
use mc_core::block::{BlockDefinition, BlockRegistry, DefinitionOrigin};
use mc_core::id::{BlockName, TextureKey};
use mc_world::persistence::{SavedPlayer, save_world};
use mc_world::world::{VoxelWorld, WorldPos};
use tempfile::TempDir;

use persistence::{
    EVERY_DECLARED_CELL, Launched, TestResult, against, refusal, registry_of, save_in,
    with_the_replay_blocks,
};

/// The two blocks whose declared behaviour changed between the save being
/// written and the client being started again.
const ALPHA: &str = "fixture:alpha";
const OMEGA: &str = "fixture:omega";

/// The block whose declaration did not change, and which therefore has no
/// business appearing in a report about what did.
const STEADY: &str = "fixture:steady";

/// Where each of the three stands in the saved world.
const ALPHA_CELL: (u32, u32, u32) = (1, 1, 1);
const OMEGA_CELL: (u32, u32, u32) = (2, 1, 1);
const STEADY_CELL: (u32, u32, u32) = (3, 1, 1);

/// Exactly what a player has to type to load a save whose blocks have changed.
///
/// Spelled out here rather than read from the client, because this is the thing
/// the message has to tell them and a test reading the client's own constant
/// would agree with a message quoting a spelling nothing accepts.
const LOAD_CHANGED_BLOCKS: &str = "--load-changed-blocks";

/// The client's own argv, as a shell hands it over — the program's own name
/// first, which is what `std::env::args` yields and what the parse has to step
/// past.
const NO_ACCEPTANCE: [&str; 1] = ["mycraft"];
const ACCEPTANCE_GIVEN: [&str; 2] = ["mycraft", LOAD_CHANGED_BLOCKS];

/// Where the save records the player. Nothing here asserts it; a save has to
/// record somebody.
const RECORDED_PLAYER: SavedPlayer = SavedPlayer {
    position: [8.5, 12.25, 8.5],
    yaw: 0.75,
    pitch: -0.25,
};

/// A save and the registry a client would read it against now.
#[derive(Debug)]
struct ASave {
    written: VoxelWorld,
    registry: Arc<BlockRegistry>,
    directory: TempDir,
}

#[test]
fn a_save_whose_blocks_were_redeclared_refuses_the_start_naming_them_and_what_to_pass() -> TestResult
{
    let save = a_save_whose_two_blocks_were_redeclared()?;
    let path = save_in(&save.directory);

    let told = refusal(&launch(&save, &path, &NO_ACCEPTANCE));

    assert_eq!(
        (
            told.contains(ALPHA),
            told.contains(OMEGA),
            told.contains(LOAD_CHANGED_BLOCKS),
            told.contains(STEADY)
        ),
        (true, true, true, false),
        "a player whose mods have changed is turned away with one line of text and nothing else, \
         so that line has to name every block that is no longer what it was — {ALPHA} and {OMEGA}, \
         both of them, not the first one it met — and say exactly what to pass to load the world \
         anyway, which is `{LOAD_CHANGED_BLOCKS}`. {STEADY} did not change and naming it would \
         send them looking for a mod that is fine. They were told: {told}"
    );
    Ok(())
}

#[test]
fn a_redeclared_save_is_played_when_the_player_passed_the_flag() -> TestResult {
    let save = a_save_whose_two_blocks_were_redeclared()?;
    let path = save_in(&save.directory);

    let launched = launch(&save, &path, &ACCEPTANCE_GIVEN);

    assert_eq!(
        against(&launched, &save.written),
        Ok((EVERY_DECLARED_CELL, Vec::new())),
        "the player passed `{LOAD_CHANGED_BLOCKS}`, which is them saying they have read what \
         changed and want their world anyway — so the client plays the world the save holds, cell \
         for cell, and not the one it would have generated. A client that parsed the flag and then \
         launched with the answer it always gives is turned away here with its own refusal. It \
         answered: {}",
        refusal(&launched)
    );
    Ok(())
}

#[test]
fn a_save_whose_blocks_are_unchanged_is_played_with_no_flag_passed() -> TestResult {
    let save = a_save_whose_blocks_are_all_unchanged()?;
    let path = save_in(&save.directory);

    let launched = launch(&save, &path, &NO_ACCEPTANCE);

    assert_eq!(
        against(&launched, &save.written),
        Ok((EVERY_DECLARED_CELL, Vec::new())),
        "nothing about this save's blocks has changed, so there is nothing for the player to \
         decide and no reason to ask them: the client plays the world the save holds without a \
         flag anywhere on its command line. This is the control the other two need — a client that \
         demanded acceptance always would satisfy both of them and fail only here. It answered: {}",
        refusal(&launched)
    );
    Ok(())
}

/// What the client makes of `save` when it is started with `argv`.
fn launch(save: &ASave, path: &std::path::Path, argv: &[&str]) -> Launched {
    simulation_to_play(
        mc_sim::REPLAY_SEED,
        Arc::clone(&save.registry),
        path,
        acceptance_from(argv.iter().map(|argument| (*argument).to_string())),
    )
}

/// A save holding all three blocks, read against a registry in which two of them
/// have been redeclared.
///
/// **The redeclarations are of behaviour and not of appearance**: solidity for
/// one and breakability for the other, which are two different fields of the
/// declaration, so a comparison that watched only one of them reports only one
/// changed block and fails the scenario that asks for every one.
fn a_save_whose_two_blocks_were_redeclared() -> Result<ASave, Box<dyn Error>> {
    a_save_read_against(vec![
        block(ALPHA, false, true)?,
        block(OMEGA, true, false)?,
        block(STEADY, false, true)?,
    ])
}

/// The same save, read against the very declarations it was written against.
fn a_save_whose_blocks_are_all_unchanged() -> Result<ASave, Box<dyn Error>> {
    a_save_read_against(as_written()?)
}

/// A save written against [`as_written`] and read against `now`.
fn a_save_read_against(now: Vec<BlockDefinition>) -> Result<ASave, Box<dyn Error>> {
    let written = registry_of(as_written()?)?;
    let blocks = three_blocks_standing_in_a_row(&written)?;
    let directory = TempDir::new()?;
    save_world(&save_in(&directory), &blocks, RECORDED_PLAYER, &written)?;

    let registry = Arc::new(with_the_replay_blocks(registry_of(now)?)?);
    Ok(ASave {
        written: blocks,
        registry,
        directory,
    })
}

/// What the three blocks were declared to be when the save was written.
fn as_written() -> Result<Vec<BlockDefinition>, Box<dyn Error>> {
    Ok(vec![
        block(ALPHA, true, true)?,
        block(OMEGA, true, true)?,
        block(STEADY, false, true)?,
    ])
}

/// One block, declared solid or not and breakable or not.
///
/// Both are behaviour rather than appearance, and the texture is derived from
/// the name so that nothing here can change a block's *look* by accident — an
/// appearance-only change is loaded without asking, and a fixture that made one
/// would be asserting the opposite of what these scenarios are about.
fn block(name: &str, is_solid: bool, breakable: bool) -> Result<BlockDefinition, Box<dyn Error>> {
    Ok(BlockDefinition {
        name: BlockName::parse(name)?,
        texture: TextureKey::parse(name)?,
        is_solid,
        replaceable: !is_solid,
        breakable,
        breaks_into: None,
        origin: DefinitionOrigin::new("the changed-blocks fixture"),
    })
}

/// An empty world with the three blocks standing in a row near its floor.
///
/// All three are placed, because a block no voxel refers to is not a name the
/// save needs — the unchanged one has to be in the table for its absence from
/// the report to mean anything.
fn three_blocks_standing_in_a_row(registry: &BlockRegistry) -> Result<VoxelWorld, Box<dyn Error>> {
    let mut blocks = VoxelWorld::empty(persistence::COLUMNS);
    for (name, (x, y, z)) in [
        (ALPHA, ALPHA_CELL),
        (OMEGA, OMEGA_CELL),
        (STEADY, STEADY_CELL),
    ] {
        blocks.set_block(WorldPos { x, y, z }, &BlockName::parse(name)?, registry)?;
    }
    Ok(blocks)
}
