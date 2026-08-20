//! What a launch was asked, and what it answered — folded into one value each.
//!
//! **A refusal and a wrong answer are the same failed assertion.** Every
//! accessor below reports `Err(the refusal, rendered)` where the launch was
//! turned away, so a scenario expecting a world or a player compares one value
//! and never asks `is_ok()` first. Asserting `Ok` and then reading the answer
//! would report "it was turned away" and "it came back wrong" as two different
//! kinds of failure, when they are the same thing: the launch did not do what
//! the scenario says it does.
//!
//! **Floats are compared as the integers they are.** "Where the save recorded
//! them" means the same value and not a nearly equal one — a stored coordinate
//! is written as four bytes and read back as the same four bytes, so there is no
//! arithmetic between the two ends for a tolerance to be about. It is also the
//! form `clippy::float_cmp` has no quarrel with.

use std::error::Error;
use std::sync::Arc;

use mc_core::block::source::InMemoryDefinitionSource;
use mc_core::block::{BlockDefinition, BlockRegistry, DefinitionOrigin};
use mc_core::content::FaceTextures;
use mc_core::id::{BlockName, TextureKey};
use mc_sim::persistence::{LaunchError, Launching};
use mc_sim::player::{BlockPos, PlayerState};
use mc_sim::replay::ReplayWorld;
use mc_sim::simulation::{Seated, Simulation};
use mc_world::persistence::{Acceptance, SavedPlayer};
use mc_world::world::{VoxelWorld, WorldPos};
use tempfile::TempDir;

use super::{FOOTPRINT, content_registry, described, published_content, replay_world};

/// The block a save holds where the generated world holds nothing.
///
/// **A name the generator cannot produce.** The scene names four blocks and all
/// four are `base:`, so a launch playing the save's world and a launch playing
/// the generated one can never agree about this cell by accident — which is the
/// whole reason the fixture is a marker rather than one of the strata.
pub const MARKER: &str = "fixture:marker";

/// The cell the marker stands in: well above the declared surface band, so the
/// generated world holds nothing there.
pub const MARKER_CELL: (u32, u32, u32) = (8, 40, 8);

/// How high a column is declared to reach, in blocks: sixteen sections of
/// sixteen.
pub const WORLD_HEIGHT: u32 = 256;

/// How many cells a comparison against the generated world has to look at.
///
/// Derived from the declared footprint and column height rather than from the
/// world it compares, so a comparison that quietly visited a smaller world is a
/// failed assertion instead of a silent one.
pub const EVERY_DECLARED_CELL: usize = (FOOTPRINT * FOOTPRINT * WORLD_HEIGHT) as usize;

/// What a cell outside the world a launch is playing is called.
///
/// A third answer beside a block and the word for a cell holding nothing, never
/// folded into either: a world that reached nowhere would otherwise read as a
/// world holding nothing everywhere.
pub const OUTSIDE: &str = "no such cell";

/// How many disagreements a comparison against the generated world reports
/// before it stops collecting them.
const FIRST_FEW: usize = 5;

/// A registry holding everything content declares, plus [`MARKER`].
///
/// The marker is registered *after* the content root, so every shipped block
/// keeps the runtime id it would have had — a save records no runtime id, and a
/// fixture that shifted them would be saying something this phase is not about.
///
/// # Errors
///
/// Returns an error if the content root cannot be read, or if the registry
/// refuses the marker.
pub fn registry_with_the_marker() -> Result<BlockRegistry, Box<dyn Error>> {
    let origin = DefinitionOrigin::new("this phase's own fixture");
    let mut registry = content_registry()?;
    registry.apply(&InMemoryDefinitionSource::new(
        origin.clone(),
        vec![Ok(BlockDefinition {
            name: BlockName::parse(MARKER)?,
            textures: FaceTextures::uniform(TextureKey::parse(MARKER)?),
            is_solid: true,
            replaceable: false,
            breakable: true,
            breaks_into: None,
            origin,
        })],
    ))?;
    Ok(registry)
}

/// Everything a launch over `registry` needs beyond where its save is.
///
/// **The acceptance is a parameter and the content is not.** Whether a save whose
/// blocks have changed is loaded anyway is the question several scenarios here are
/// about, so it is stated at the call; the content a launch publishes is what
/// `registry` resolves to on the layers a session that has spent nothing hands
/// out, which is a fact about a launch rather than a decision.
///
/// # Errors
///
/// Returns an error if a registered id cannot be read back, or if the layers do
/// not fit a session's budget.
pub fn launching(
    registry: &Arc<BlockRegistry>,
    accepting: Acceptance,
) -> Result<Launching, Box<dyn Error>> {
    Ok(Launching {
        seed: mc_sim::REPLAY_SEED,
        registry: Arc::clone(registry),
        content: published_content(registry)?,
        accepting,
    })
}

/// The registry, the generated world and the directory a launch's save lives
/// in.
///
/// # Errors
///
/// Returns an error if the registry, the world or the directory cannot be made.
pub fn a_world_to_launch_into() -> Result<(Arc<BlockRegistry>, ReplayWorld, TempDir), Box<dyn Error>>
{
    let registry = Arc::new(registry_with_the_marker()?);
    let world = replay_world(&registry)?;
    Ok((registry, world, TempDir::new()?))
}

/// Where a launch looks for its save, inside `directory`.
///
/// Two components deep and neither of them made, because a first launch has no
/// directory waiting for it and that is the no-save case rather than a failure.
#[must_use]
pub fn save_path(directory: &TempDir) -> std::path::PathBuf {
    directory.path().join("saves").join("world.mcw")
}

/// The blocks of `generated` with [`MARKER`] standing in [`MARKER_CELL`].
///
/// # Errors
///
/// Returns an error if the marker is not a name, or if `registry` does not hold
/// it.
pub fn generated_with_the_marker(
    generated: &ReplayWorld,
    registry: &BlockRegistry,
) -> Result<VoxelWorld, Box<dyn Error>> {
    let (x, y, z) = MARKER_CELL;
    let mut blocks = generated.blocks().clone();
    blocks.set_block(WorldPos { x, y, z }, &BlockName::parse(MARKER)?, registry)?;
    Ok(blocks)
}

/// Where a save records the player, and which way it records them facing.
///
/// **Not one of the five numbers is zero, and not one is the spawn's own
/// value.** A launch that handed back a player it had zeroed, or one it had
/// placed from the generated world instead of from the save, has to disagree
/// with every field here rather than with some of them.
///
/// **The height is a quarter of a block off a whole number**, and every height a
/// heightmap can report is a whole number of blocks — so a launch that derived a
/// height instead of reading this one cannot arrive at it by accident, whichever
/// world it derived from.
#[must_use]
pub fn recorded_player() -> SavedPlayer {
    SavedPlayer {
        position: [12.5, 40.25, 8.5],
        yaw: 135_f32.to_radians(),
        pitch: -30_f32.to_radians(),
    }
}

/// Where a launch stood the player and which way they face, as the integers its
/// floats are — or the refusal it gave instead.
///
/// # Errors
///
/// Returns the rendered refusal where the launch was turned away.
pub fn placed(launched: &Result<Seated, LaunchError>) -> Result<([u32; 3], u32, u32), String> {
    let player = published_player(launched)?;
    Ok((
        player.position.to_array().map(f32::to_bits),
        player.yaw.to_bits(),
        player.pitch.to_bits(),
    ))
}

/// How high a launch stood the player's feet, as the integer its float is — or
/// the refusal it gave instead.
///
/// # Errors
///
/// Returns the rendered refusal where the launch was turned away.
pub fn stood(launched: &Result<Seated, LaunchError>) -> Result<u32, String> {
    Ok(published_player(launched)?.position.y.to_bits())
}

/// How fast a launch left the player going on each axis, as the integers those
/// floats are — or the refusal it gave instead.
///
/// The magnitude per axis rather than the signed value, so that a negative zero
/// counts as at rest: "at rest" is a claim about speed and −0.0 is a speed of
/// none.
///
/// # Errors
///
/// Returns the rendered refusal where the launch was turned away.
pub fn moving(launched: &Result<Seated, LaunchError>) -> Result<[u32; 3], String> {
    Ok(published_player(launched)?
        .velocity
        .abs()
        .to_array()
        .map(f32::to_bits))
}

/// What the world a launch plays holds at `at`: a block's own name, the word for
/// a cell holding nothing, or [`OUTSIDE`] — or the refusal the launch gave.
///
/// # Errors
///
/// Returns the rendered refusal where the launch was turned away.
pub fn held_at(
    launched: &Result<Seated, LaunchError>,
    at: (u32, u32, u32),
) -> Result<String, String> {
    let (x, y, z) = at;
    let played = playing(launched)?;
    Ok(played
        .world()
        .block_at(BlockPos {
            x: x as i32,
            y: y as i32,
            z: z as i32,
        })
        .map_or_else(|| OUTSIDE.to_owned(), described))
}

/// How the world a launch plays compares with `generated`: how many cells were
/// looked at, and the first few that disagreed — or the refusal it gave instead.
///
/// **The count is fixture integrity and not the scenario's claim.** A comparison
/// that visited nothing agrees with everything, so what was looked at is
/// reported beside what was found and asserted against the declaration.
///
/// # Errors
///
/// Returns the rendered refusal where the launch was turned away.
pub fn against_generated(
    launched: &Result<Seated, LaunchError>,
    generated: &ReplayWorld,
) -> Result<(usize, Vec<String>), String> {
    let played = playing(launched)?;
    let mut compared = 0;
    let mut wrong = Vec::new();
    for (x, y, z) in every_declared_cell() {
        compared += 1;
        let here = played.world().block_at(BlockPos {
            x: x as i32,
            y: y as i32,
            z: z as i32,
        });
        let there = generated.block_at(x, y, z);
        if here != there && wrong.len() < FIRST_FEW {
            wrong.push(disagreement((x, y, z), here, there));
        }
    }
    Ok((compared, wrong))
}

/// The refusal a launch gave, in its own words — or what it did instead of
/// refusing.
///
/// **Its own words and not the whole report a player reads.** What reaches a
/// terminal is this sentence and every failure beneath it, joined and printed by
/// `mc-render`, and this crate may not resolve `mc-render` in any dependency
/// kind. Walking the chain here instead would put a second renderer in the
/// workspace, asserted against its own output and reaching no printing — which
/// is the defect a whole spec was spent removing. So what this crate's own tests
/// hold it to is the typed obligation: this layer's sentence, and separately
/// what it carries beneath (see [`beneath`]).
#[must_use]
pub fn answered(launched: &Result<Seated, LaunchError>) -> String {
    match launched {
        Ok(_) => "it started a simulation".to_owned(),
        Err(refusal) => refusal.to_string(),
    }
}

/// What the refusal a launch gave carries beneath it, in that layer's own words
/// — or [`NOTHING_BENEATH`] where it carries nothing.
///
/// One hop and no joining, which is the whole difference between this and a
/// renderer: it answers *what is carried*, never *what is printed*. A refusal
/// that stopped carrying its reason is now invisible to every message anybody
/// reads at this level, so this is the one thing standing between a player and
/// the reason their save would not load.
#[must_use]
pub fn beneath(launched: &Result<Seated, LaunchError>) -> String {
    match launched {
        Ok(_) => "it started a simulation".to_owned(),
        Err(refusal) => refusal
            .source()
            .map_or_else(|| NOTHING_BENEATH.to_owned(), ToString::to_string),
    }
}

/// What a refusal carrying nothing beneath it is called.
///
/// A sentence rather than an empty string, so "it carries nothing" and "it
/// carries a reason with nothing to say" cannot be mistaken for one another.
pub const NOTHING_BENEATH: &str = "it carries no reason beneath it";

/// The simulation a launch started, or the refusal it gave instead.
fn playing(launched: &Result<Seated, LaunchError>) -> Result<&Simulation, String> {
    Ok(&launched
        .as_ref()
        .map_err(LaunchError::to_string)?
        .simulation)
}

/// The player a launch published, or the refusal it gave instead.
fn published_player(launched: &Result<Seated, LaunchError>) -> Result<PlayerState, String> {
    Ok(playing(launched)?.latest().player)
}

/// Every cell the declaration says a launched world spans, y fastest.
fn every_declared_cell() -> impl Iterator<Item = (u32, u32, u32)> {
    (0..FOOTPRINT)
        .flat_map(|z| (0..FOOTPRINT).map(move |x| (x, z)))
        .flat_map(|(x, z)| (0..WORLD_HEIGHT).map(move |y| (x, y, z)))
}

/// One cell two worlds disagree about, as what each of them holds there.
fn disagreement(
    at: (u32, u32, u32),
    here: Option<mc_world::section::Contents<&BlockName>>,
    there: Option<mc_world::section::Contents<&BlockName>>,
) -> String {
    let (x, y, z) = at;
    let played = here.map_or_else(|| OUTSIDE.to_owned(), described);
    let declared = there.map_or_else(|| OUTSIDE.to_owned(), described);
    format!("({x}, {y}, {z}) holds {played} where the generated world holds {declared}")
}
