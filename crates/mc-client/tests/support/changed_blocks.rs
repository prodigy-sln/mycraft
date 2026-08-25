//! Saves whose blocks the running content no longer declares the same way, and
//! what a launch over each of them answers.
//!
//! # Two blocks changed, a third did not, and a fourth only looks different
//!
//! "Every changed block" is only observable against more than one, and "every" is
//! only distinguishable from "everything" against a block that did not change. The
//! unchanged one is what stops a report that simply lists the save's whole table
//! from reading as complete. The retextured one is the other control: a report
//! that named it would put a line on a player's terminal after an art edit, which
//! is the noise the line that matters would hide in.
//!
//! # Every acceptance decision comes out of the client's own parse of a real argv
//!
//! A client that parsed its argument perfectly and then dropped it on the floor
//! fails whichever scenario the dropped value disagreed with — but only if the
//! value driven through the launch came from the parse rather than from a fixture
//! naming the variant it wanted. So every launch here spells a command line and
//! hands it to [`acceptance_from`].
//!
//! # The line is composed from a list a real load produced
//!
//! [`line_of`] asks `mc_client::notice::changed_blocks` about the list that came
//! back on `Seated`, so what is under test is the composer over data the
//! persistence layer really reported. **It does not witness that the client says
//! it out loud** — that is `tests/shipped_binary.rs`, and the reason the two are
//! separate is written in its header.

// Each scenario binary links this whole module and drives a subset of it.
#![allow(dead_code)]

use std::error::Error;
use std::path::Path;
use std::sync::Arc;

use mc_client::launch::simulation_to_play;
use mc_client::notice::changed_blocks;
use mc_client::startup::acceptance_from;
use mc_core::block::{BlockDefinition, BlockRegistry, DefinitionOrigin};
use mc_core::content::FaceTextures;
use mc_core::id::{BlockName, TextureKey};
use mc_sim::persistence::Launching;
use mc_world::persistence::{SavedPlayer, save_world};
use mc_world::world::{VoxelWorld, WorldPos};
use tempfile::TempDir;

use crate::persistence::{self, Launched, registry_of, save_in, with_the_replay_blocks};

/// The two blocks whose declared behaviour changed between the save being
/// written and the client being started again.
///
/// Named so that ascending order is not the order they are declared in or placed
/// in: a report that happened to emit its table's order would agree with an
/// ascending expectation by accident if the two coincided.
pub const ALPHA: &str = "fixture:alpha";
pub const OMEGA: &str = "fixture:omega";

/// The block whose declaration did not change at all, and which therefore has no
/// business appearing in a report about what did.
pub const STEADY: &str = "fixture:steady";

/// The block that draws from a different texture key than it did, and is
/// otherwise the block it always was.
pub const RETEXTURED: &str = "fixture:retextured";

/// The key `RETEXTURED` draws from now, which is not the one the save recorded.
const A_DIFFERENT_KEY: &str = "fixture:repainted";

/// Where each of the four stands in the saved world.
const ALPHA_CELL: (u32, u32, u32) = (1, 1, 1);
const OMEGA_CELL: (u32, u32, u32) = (2, 1, 1);
const STEADY_CELL: (u32, u32, u32) = (3, 1, 1);
const RETEXTURED_CELL: (u32, u32, u32) = (4, 1, 1);

/// Exactly what a player has to type to have a changed save refused, and the
/// same argument one letter short.
///
/// Spelled out here rather than read from the client, because this is the thing
/// the refusal has to tell them: a test reading the client's own constant would
/// agree with a message quoting a spelling nothing accepts.
pub const REFUSE_CHANGED_BLOCKS: &str = "--refuse-changed-blocks";
const ONE_LETTER_SHORT: &str = "--refuse-changed-block";

/// The client's own argv, as a shell hands it over — the program's own name
/// first, which is what `std::env::args` yields and what the parse has to step
/// past.
pub const NO_ARGUMENT: [&str; 1] = ["mycraft"];
pub const REFUSING: [&str; 2] = ["mycraft", REFUSE_CHANGED_BLOCKS];
pub const REFUSING_MISSPELLED: [&str; 2] = ["mycraft", ONE_LETTER_SHORT];

/// Where the save records the player, so that a resume has a position, a yaw and
/// a pitch to be judged against.
///
/// All three differ from anything a generated spawn produces and from each
/// other, so a launch that placed the player from the world rather than from the
/// save disagrees here rather than coinciding.
pub const RECORDED_PLAYER: SavedPlayer = SavedPlayer {
    position: [8.5, 12.25, 8.5],
    yaw: 0.75,
    pitch: -0.25,
};

/// A save and the registry a client would read it against now.
#[derive(Debug)]
pub struct ASave {
    pub written: VoxelWorld,
    pub registry: Arc<BlockRegistry>,
    pub directory: TempDir,
}

/// A save holding all four blocks, read against a registry in which two of them
/// have been redeclared.
///
/// **The redeclarations are of behaviour and not of appearance**: solidity for
/// one and breakability for the other, which are two different fields of the
/// declaration, so a comparison that watched only one of them reports only one
/// changed block and fails the scenario that asks for every one.
///
/// # Errors
///
/// Returns an error if a name or key is not parseable, if a registry refuses a
/// declaration, or if the save cannot be written.
pub fn a_save_whose_two_blocks_were_redeclared() -> Result<ASave, Box<dyn Error>> {
    a_save_read_against(vec![
        block(ALPHA, false, true, ALPHA)?,
        block(OMEGA, true, false, OMEGA)?,
        block(STEADY, false, true, STEADY)?,
        block(RETEXTURED, false, true, RETEXTURED)?,
    ])
}

/// The same save, read against the very declarations it was written against.
///
/// # Errors
///
/// Returns an error for the reasons above.
pub fn a_save_whose_blocks_are_all_unchanged() -> Result<ASave, Box<dyn Error>> {
    a_save_read_against(as_written()?)
}

/// The same save, read against declarations differing in one texture key and in
/// nothing else.
///
/// # Errors
///
/// Returns an error for the reasons above.
pub fn a_save_whose_block_only_looks_different() -> Result<ASave, Box<dyn Error>> {
    a_save_read_against(vec![
        block(ALPHA, true, true, ALPHA)?,
        block(OMEGA, true, true, OMEGA)?,
        block(STEADY, false, true, STEADY)?,
        block(RETEXTURED, false, true, A_DIFFERENT_KEY)?,
    ])
}

/// A save written against [`as_written`] and read against `now`.
///
/// # Errors
///
/// Returns an error if a declaration is refused or the save cannot be written.
pub fn a_save_read_against(now: Vec<BlockDefinition>) -> Result<ASave, Box<dyn Error>> {
    let written = registry_of(as_written()?)?;
    let blocks = four_blocks_standing_in_a_row(&written)?;
    let directory = TempDir::new()?;
    save_world(&save_in(&directory), &blocks, RECORDED_PLAYER, &written)?;

    let registry = Arc::new(with_the_replay_blocks(registry_of(now)?)?);
    Ok(ASave {
        written: blocks,
        registry,
        directory,
    })
}

/// What the four blocks were declared to be when the save was written.
///
/// # Errors
///
/// Returns an error if a name or a texture key is not parseable.
pub fn as_written() -> Result<Vec<BlockDefinition>, Box<dyn Error>> {
    Ok(vec![
        block(ALPHA, true, true, ALPHA)?,
        block(OMEGA, true, true, OMEGA)?,
        block(STEADY, false, true, STEADY)?,
        block(RETEXTURED, false, true, RETEXTURED)?,
    ])
}

/// One block, declared solid or not, breakable or not, drawing from `texture`.
///
/// Solidity and breakability are behaviour; the key every face draws from is
/// appearance. Both are stated rather than derived so that a fixture can move one
/// without moving the other — an appearance-only change is loaded with nothing
/// said about it, and a fixture that moved a look while meaning to move a
/// behaviour would be asserting the opposite of what it says.
///
/// # Errors
///
/// Returns an error if `name` is not a namespaced id or `texture` is not a key.
pub fn block(
    name: &str,
    is_solid: bool,
    breakable: bool,
    texture: &str,
) -> Result<BlockDefinition, Box<dyn Error>> {
    Ok(BlockDefinition {
        name: BlockName::parse(name)?,
        textures: FaceTextures::uniform(TextureKey::parse(texture)?),
        is_solid,
        replaceable: !is_solid,
        breakable,
        breaks_into: None,
        drawn: is_solid,
        occludes: is_solid,
        targetable: is_solid,
        // Constants, never derived from this fixture's own solidity: nothing
        // has ever answered these two, so a derived medium would make the air
        // swimmable and no assertion in this file could see it.
        swimmable: false,
        move_resistance: 0.0,
        origin: DefinitionOrigin::new("the changed-blocks fixture"),
    })
}

/// An empty world with the four blocks standing in a row near its floor.
///
/// All four are placed, because a block no voxel refers to is not a name the save
/// needs — the unchanged one has to be in the table for its absence from the
/// report to mean anything.
///
/// # Errors
///
/// Returns an error if a name is not parseable or the registry refuses it.
pub fn four_blocks_standing_in_a_row(
    registry: &BlockRegistry,
) -> Result<VoxelWorld, Box<dyn Error>> {
    let mut blocks = VoxelWorld::empty(persistence::COLUMNS);
    for (name, (x, y, z)) in [
        (ALPHA, ALPHA_CELL),
        (OMEGA, OMEGA_CELL),
        (STEADY, STEADY_CELL),
        (RETEXTURED, RETEXTURED_CELL),
    ] {
        blocks.set_block(WorldPos { x, y, z }, &BlockName::parse(name)?, registry)?;
    }
    Ok(blocks)
}

/// What the client makes of `save` when it is started with `argv`.
///
/// # Errors
///
/// Returns an error if the content a simulation publishes cannot be assembled.
pub fn launch(save: &ASave, path: &Path, argv: &[&str]) -> Result<Launched, Box<dyn Error>> {
    Ok(simulation_to_play(
        path,
        Launching {
            seed: mc_sim::REPLAY_SEED,
            registry: Arc::clone(&save.registry),
            content: persistence::published_content(&save.registry)?,
            accepting: acceptance_from(argv.iter().map(|argument| (*argument).to_string())),
        },
    ))
}

/// The line the client composes about what `launched` reported, or `None` where
/// it composes nothing — and the rendered refusal where the launch was turned
/// away, so that "it refused" and "it said the wrong thing" are one failed
/// comparison rather than two kinds of failure.
///
/// # Errors
///
/// Returns the rendered refusal where the launch was turned away.
pub fn line_of(launched: &Launched) -> Result<Option<String>, String> {
    match launched {
        Ok((seated, _)) => Ok(changed_blocks(&seated.changed)),
        Err(_) => Err(persistence::refusal(launched)),
    }
}

/// Which blocks `launched` reported as no longer behaving as they did, or the
/// rendered refusal where it was turned away.
///
/// # Errors
///
/// Returns the rendered refusal where the launch was turned away.
pub fn reported_changed(launched: &Launched) -> Result<Vec<String>, String> {
    match launched {
        Ok((seated, _)) => Ok(seated
            .changed
            .iter()
            .map(|name| name.as_str().to_owned())
            .collect()),
        Err(_) => Err(persistence::refusal(launched)),
    }
}
