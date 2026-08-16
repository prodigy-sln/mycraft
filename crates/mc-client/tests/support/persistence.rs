//! What the quit-and-resume scenarios build their registry, their world and
//! their expectations from.
//!
//! # A refusal and a wrong answer are the same failed assertion
//!
//! Every accessor below reports `Err(the refusal, rendered)` where the launch
//! was turned away, so a scenario expecting a world or a player compares one
//! value and never asks `is_ok()` first. Asserting `Ok` and then reading the
//! answer would report "it refused to start" and "it started in the wrong
//! world" as two different kinds of failure, when they are the same thing: the
//! client did not do what the scenario says it does. It is also what keeps every
//! failure here an *assertion* failure rather than a propagated error.
//!
//! # The floor is emptiness with one solid layer in it, and that is deliberate
//!
//! The input harness's own fixture fills every open cell with a declared
//! non-solid block, which is a cell that *holds* something. "A block is placed
//! in an empty cell" is a claim about a cell holding nothing, so this fixture
//! starts from an empty world and lays one solid layer into it — and a break
//! that empties a cell and a placement into one that was already empty are then
//! the two halves of the same declaration rather than two spellings of
//! "replaceable".
//!
//! # Floats are compared as the integers they are
//!
//! "Where the player was" means the same value and not a nearly equal one: a
//! coordinate is written as four bytes and read back as the same four bytes, so
//! there is no arithmetic between the two ends for a tolerance to be about. It
//! is also the form `clippy::float_cmp` has no quarrel with.

// Each scenario binary links this whole module and drives a subset of it.
#![allow(dead_code)]

use std::error::Error;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use glam::Vec3;
use mc_client::startup::PreparationError;
use mc_core::block::source::InMemoryDefinitionSource;
use mc_core::block::{BlockDefinition, BlockRegistry, DefinitionOrigin};
use mc_core::id::{BlockName, TextureKey};
use mc_render::window::{Ending, report};
use mc_sim::action::default_held_block;
use mc_sim::player::{BlockPos, PlayerState};
use mc_sim::simulation::Simulation;
use mc_sim::world::World;
use mc_world::persistence::{Acceptance, load_world};
use mc_world::section::Contents;
use mc_world::world::{VoxelWorld, WorldPos};
use tempfile::TempDir;

/// The error type every scenario in this phase propagates with `?`.
pub type TestResult = Result<(), Box<dyn Error>>;

/// What a launch came to: the simulation it plays and the block a client holds
/// in it, or the refusal it gave instead.
pub type Launched = Result<(Simulation, BlockName), PreparationError>;

/// What a cell holding nothing is called wherever this suite compares contents
/// as text.
///
/// Not a block name and unable to become one: every namespaced name carries a
/// colon, so an expectation of an empty cell and one of a named block can sit
/// side by side without either impersonating the other.
pub const NOTHING: &str = "nothing";

/// What a cell outside the world a launch is playing is called.
///
/// A third answer beside a block and the word for nothing, never folded into
/// either: a world that reached nowhere would otherwise read as a world holding
/// nothing everywhere.
pub const OUTSIDE: &str = "no such cell";

/// The one solid block the fixture floor is made of, and the block a place
/// request over it names.
pub const GROUND: &str = "fixture:ground";

/// How many chunk columns the fixture world spans on each axis, how many blocks
/// across one column is, and how high a column reaches.
pub const COLUMNS: u32 = 1;
pub const ACROSS: u32 = 16;
pub const HEIGHT: u32 = 256;

/// The one solid voxel layer of the floor. Every other cell holds nothing.
pub const FLOOR: u32 = 9;

/// Where the player stands: on the floor, centred in the column, facing along
/// +x, holding still.
pub const SPAWN: Vec3 = Vec3::new(8.5, (FLOOR + 1) as f32, 8.5);

/// How many cells a comparison against a saved world has to look at.
///
/// Derived from the declaration rather than from the world it compares, so a
/// comparison that quietly visited a smaller world is a failed assertion instead
/// of a silent one.
pub const EVERY_DECLARED_CELL: usize = (COLUMNS * ACROSS * COLUMNS * ACROSS * HEIGHT) as usize;

/// The blocks the replay's own generator places, spelled as content spells them.
///
/// Named here only so that a world can be generated at all — the world a resume
/// never looks at, and the one a launch builds for itself exactly when there is
/// no save. Files under `tests/` are not read by `mc-world`'s hardcoded-name
/// scan, which is why this fixture may say them out loud.
const REPLAY_BLOCKS: [(&str, bool); 4] = [
    ("base:grass", true),
    ("base:dirt", true),
    ("base:stone", true),
    ("base:water", false),
];

/// What these definitions are attributed to. Nothing asserts it; a definition
/// has to say where it came from.
const FIXTURE_ORIGIN: &str = "this phase's own fixture";

/// How many disagreements a comparison reports before it stops collecting them.
const FIRST_FEW: usize = 5;

/// One block, declared the way this fixture declares blocks.
///
/// # Errors
///
/// Returns an error if `name` is not a namespaced name.
pub fn declared(name: &str, is_solid: bool) -> Result<BlockDefinition, Box<dyn Error>> {
    Ok(BlockDefinition {
        name: BlockName::parse(name)?,
        texture: TextureKey::parse(name)?,
        is_solid,
        replaceable: !is_solid,
        breakable: true,
        breaks_into: None,
        origin: DefinitionOrigin::new(FIXTURE_ORIGIN),
    })
}

/// A registry holding exactly the definitions it is given, in the order it is
/// given them.
///
/// The order is the caller's because [`default_held_block`] reads it: whichever
/// solid block is registered first is the one a client holds.
///
/// # Errors
///
/// Returns an error if the registry refuses any of them.
pub fn registry_of(declarations: Vec<BlockDefinition>) -> Result<BlockRegistry, Box<dyn Error>> {
    let mut registry = BlockRegistry::new();
    registry.apply(&InMemoryDefinitionSource::new(
        DefinitionOrigin::new(FIXTURE_ORIGIN),
        declarations.into_iter().map(Ok).collect(),
    ))?;
    Ok(registry)
}

/// `registry` with the blocks the replay generator places added after it.
///
/// Added *after*, so every block the caller declared keeps the runtime id it
/// would have had and the block a client holds does not move — a save records no
/// runtime id, and a fixture that shifted them would be saying something these
/// scenarios are not about.
///
/// # Errors
///
/// Returns an error if a replay block is not a name or the registry refuses it.
pub fn with_the_replay_blocks(
    mut registry: BlockRegistry,
) -> Result<BlockRegistry, Box<dyn Error>> {
    let declarations = REPLAY_BLOCKS
        .into_iter()
        .map(|(name, is_solid)| declared(name, is_solid))
        .collect::<Result<Vec<_>, _>>()?;
    registry.apply(&InMemoryDefinitionSource::new(
        DefinitionOrigin::new(FIXTURE_ORIGIN),
        declarations.into_iter().map(Ok).collect(),
    ))?;
    Ok(registry)
}

/// An empty world with one solid layer laid into it at [`FLOOR`].
///
/// # Errors
///
/// Returns an error if the floor block is not a name, or if `registry` does not
/// hold it.
pub fn floor_world(registry: &BlockRegistry) -> Result<VoxelWorld, Box<dyn Error>> {
    let ground = BlockName::parse(GROUND)?;
    let mut blocks = VoxelWorld::empty(COLUMNS);
    for z in 0..ACROSS {
        for x in 0..ACROSS {
            blocks.set_block(WorldPos { x, y: FLOOR, z }, &ground, registry)?;
        }
    }
    Ok(blocks)
}

/// A simulation of that floor with the player standing on it, and the block a
/// place request over it names.
///
/// # Errors
///
/// Returns an error if the world does not build, or if `registry` declares no
/// solid block for a client to hold.
pub fn standing_on_the_floor(
    registry: Arc<BlockRegistry>,
) -> Result<(Simulation, BlockName), Box<dyn Error>> {
    let blocks = floor_world(&registry)?;
    let holding = default_held_block(&registry)
        .ok_or("this phase's registry declares no solid block to place")?;
    let world = World::new(blocks, registry)?;
    Ok((
        Simulation::new(
            PlayerState {
                position: SPAWN,
                velocity: Vec3::ZERO,
                yaw: 0.0,
                pitch: 0.0,
                on_ground: true,
            },
            world,
        ),
        holding,
    ))
}

/// Where a client looks for its save, inside `directory`.
///
/// Two components deep and neither of them made, because a first launch has no
/// directory waiting for it.
#[must_use]
pub fn save_in(directory: &TempDir) -> PathBuf {
    directory.path().join("saves").join("world.mcw")
}

/// What `contents` holds, as text: the block's own name, or [`NOTHING`].
#[must_use]
pub fn described(contents: Contents<&BlockName>) -> String {
    match contents {
        Contents::Empty => NOTHING.to_owned(),
        Contents::Holds(name) => name.as_str().to_owned(),
    }
}

/// What the world a launch plays holds at `at` — or the refusal it gave.
///
/// # Errors
///
/// Returns the rendered refusal where the launch was turned away.
pub fn held_at(launched: &Launched, at: (u32, u32, u32)) -> Result<String, String> {
    let (x, y, z) = at;
    let (playing, _) = played(launched)?;
    Ok(playing
        .world()
        .block_at(BlockPos {
            x: x as i32,
            y: y as i32,
            z: z as i32,
        })
        .map_or_else(|| OUTSIDE.to_owned(), described))
}

/// Where a launch stood the player, as the integers its floats are — or the
/// refusal it gave instead.
///
/// # Errors
///
/// Returns the rendered refusal where the launch was turned away.
pub fn stood_at(launched: &Launched) -> Result<[u32; 3], String> {
    Ok(played(launched)?
        .0
        .latest()
        .player
        .position
        .to_array()
        .map(f32::to_bits))
}

/// Which way a launch faced the player — yaw and then pitch, as the integers
/// those floats are — or the refusal it gave instead.
///
/// # Errors
///
/// Returns the rendered refusal where the launch was turned away.
pub fn facing(launched: &Launched) -> Result<(u32, u32), String> {
    let player = played(launched)?.0.latest().player;
    Ok((player.yaw.to_bits(), player.pitch.to_bits()))
}

/// How the world a launch plays compares with `saved`: how many cells were
/// looked at, and the first few that disagreed — or the refusal it gave.
///
/// **The count is fixture integrity and not the scenario's claim.** A comparison
/// that visited nothing agrees with everything, so what was looked at is
/// reported beside what was found and asserted against the declaration.
///
/// # Errors
///
/// Returns the rendered refusal where the launch was turned away.
pub fn against(launched: &Launched, saved: &VoxelWorld) -> Result<(usize, Vec<String>), String> {
    let (playing, _) = played(launched)?;
    Ok(compared_with(saved, |at| {
        let (x, y, z) = at;
        playing
            .world()
            .block_at(BlockPos {
                x: x as i32,
                y: y as i32,
                z: z as i32,
            })
            .map_or_else(|| OUTSIDE.to_owned(), described)
    }))
}

/// How the world in the save at `save` compares with `blocks` — or why the save
/// could not be read at all.
///
/// The same comparison [`against`] makes, one level lower: what is on disk
/// rather than what a launch made of it.
///
/// # Errors
///
/// Returns the rendered refusal where the save could not be read.
pub fn stored_against(
    save: &Path,
    registry: &BlockRegistry,
    blocks: &VoxelWorld,
) -> Result<(usize, Vec<String>), String> {
    let loaded = load_world(save, registry, Acceptance::OnlyUnchangedBlocks)
        .map_err(|refusal| refusal.to_string())?;
    Ok(compared_with(blocks, |at| {
        let (x, y, z) = at;
        loaded
            .world
            .block_at(WorldPos { x, y, z })
            .map_or_else(|_| OUTSIDE.to_owned(), described)
    }))
}

/// Every declared cell of `saved` against what `here` answers for it.
fn compared_with(
    saved: &VoxelWorld,
    here: impl Fn((u32, u32, u32)) -> String,
) -> (usize, Vec<String>) {
    let mut compared = 0;
    let mut wrong = Vec::new();
    for (x, y, z) in every_declared_cell() {
        compared += 1;
        let found = here((x, y, z));
        let expected = saved
            .block_at(WorldPos { x, y, z })
            .map_or_else(|_| OUTSIDE.to_owned(), described);
        if found != expected && wrong.len() < FIRST_FEW {
            wrong.push(format!(
                "({x}, {y}, {z}) holds {found} where {expected} was written"
            ));
        }
    }
    (compared, wrong)
}

/// The refusal a launch gave, as a player reads it — or what it did instead of
/// refusing.
///
/// **Taken through the door the client goes through, not composed here.** A
/// refusal is the failure and every failure beneath it, and then the way out
/// where the refusal has one; the outermost layer's own sentence is a fraction
/// of that. A helper reading that sentence would be asserting against a string
/// the client never prints — which is exactly the state in which a message can
/// lose the one line telling a player how to get their world back while every
/// test stays green.
///
/// It goes through [`Ending::failed`] and [`report`] rather than assembling the
/// same two pieces itself, and that is the load-bearing part: a `failed` that
/// stopped appending guidance altogether would leave a hand-assembled helper
/// green, because the helper would be a second copy of the decision rather than
/// a reader of it.
#[must_use]
pub fn refusal(launched: &Launched) -> String {
    match launched {
        Ok(_) => "it started a simulation".to_owned(),
        Err(turned_away) => shown_to_a_player(turned_away),
    }
}

/// What the client writes for `turned_away`, captured whole.
///
/// A `Vec` never refuses bytes, so the refusing arm is unreachable; it says so
/// in words rather than unwrapping, because a helper that panicked here would
/// report a sink as a failed launch.
fn shown_to_a_player(turned_away: &PreparationError) -> String {
    let mut sink = Vec::new();
    match report(
        &Ending::failed(turned_away, &turned_away.way_out()),
        &mut sink,
    ) {
        Ok(()) => String::from_utf8_lossy(&sink).into_owned(),
        Err(refused) => format!("what a player would be shown could not be written: {refused}"),
    }
}

/// The simulation a launch started and the block it holds, or the refusal it
/// gave instead.
fn played(launched: &Launched) -> Result<&(Simulation, BlockName), String> {
    launched.as_ref().map_err(PreparationError::to_string)
}

/// Every cell the declaration says the fixture world spans, y fastest.
fn every_declared_cell() -> impl Iterator<Item = (u32, u32, u32)> {
    let across = COLUMNS * ACROSS;
    (0..across)
        .flat_map(move |z| (0..across).map(move |x| (x, z)))
        .flat_map(|(x, z)| (0..HEIGHT).map(move |y| (x, y, z)))
}
