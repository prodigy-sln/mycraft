//! Writing what a simulation is playing, and deciding which simulation a launch
//! plays.
//!
//! **The decision lives here and not in the client.** Which world a launch
//! plays, and what a save it cannot read does to a start, are policy — and a
//! save is server state, which is what this crate is. The client wires this up
//! and decides nothing.
//!
//! **Nowhere the golden suites can reach it.** A save file lying in a capture's
//! working directory must not be able to change what a golden frame shows. So
//! this is asked by the client's *launch* preparation and by nothing on the
//! path the golden suites shoot through — the two are separate entry points,
//! rather than one with the question asked above it, which is what makes the
//! capture path unable to read a save rather than merely not doing so.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use glam::Vec3;
use mc_core::block::{BlockRegistry, RegistryError};
use mc_world::persistence::{
    Acceptance, LoadError, SaveError, SavedPlayer, load_world, save_world,
};
use thiserror::Error;

use crate::player::PlayerState;
use crate::replay::{ReplayWorld, SpawnError, WorldGenError, simulation_for};
use crate::simulation::{PublishedContent, Simulation};
use crate::world::World;

/// Why a launch could not start.
///
/// **`Eq` is the one derive this enum cannot carry**, inherited rather than
/// chosen: [`LoadError`] refuses a stored coordinate by naming the `f32` it
/// found, and an `f32` is not `Eq` because a NaN is not equal to itself.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum LaunchError {
    /// A save is there and cannot be read.
    ///
    /// **It names the file**, which is why it carries the path rather than
    /// deriving from [`LoadError`] alone: most of the refusals a save can raise
    /// are about its bytes and know nothing about where those bytes came from,
    /// and "the save could not be read" without saying *which* file is a message
    /// a player cannot act on. This is the level that knows the path, so this is
    /// the level that says it.
    ///
    /// **It does not name the reason, and it did once.** The reason is what the
    /// level beneath knows, and it is rendered from there: a report is a failure
    /// and every failure under it, so a message quoting its own source has that
    /// source read out twice. What a player reads is unchanged to the character —
    /// the joiner moved out of this string and into the rendering, and it is the
    /// same `": "`.
    ///
    /// **The reason is boxed**, because [`LoadError`] is a wide enum — it names
    /// every list, count and position a save can be wrong about — and a launch
    /// that succeeds would otherwise carry all of it in every `Result` it
    /// returns. That is what `clippy::result_large_err` is about, and the answer
    /// it asks for.
    #[error("{save} could not be read", save = save.display())]
    Load {
        save: PathBuf,
        #[source]
        source: Box<LoadError>,
    },
    /// There is no save to resume and no world could be generated in its place.
    ///
    /// **It carries its cause rather than interpolating it**, and it did the
    /// opposite once. The refusal a turned-away player reads is rendered by
    /// walking the source chain, so a message quoting its own source hands a
    /// content author the block twice and buries the sentence saying why a world
    /// was being built at all. Carrying it loses nothing: the cause is still
    /// read out, one layer down, joined with the same `": "` this string used to
    /// spell itself.
    ///
    /// It is not transparent either, because the sentence a player needs *is*
    /// why the world was being generated, and that is what this level knows and
    /// the cause does not.
    #[error("a new world could not be generated")]
    WorldGen(#[from] WorldGenError),
    /// The generated world cannot place a player.
    #[error(transparent)]
    Spawn(#[from] SpawnError),
    /// A world holds a block the registry does not know.
    #[error(transparent)]
    Registry(#[from] RegistryError),
}

/// Writes what `simulation` is playing to `path`.
///
/// The blocks come from the simulation's own world and the player from the
/// snapshot it last published, which is the state anything reading this
/// simulation would have seen at that moment.
///
/// # Errors
///
/// Returns whatever [`save_world`] refuses: a path that is a directory, a
/// component of it that is a file, a block the registry does not declare, or a
/// write that failed.
pub fn save(simulation: &Simulation, path: &Path) -> Result<(), SaveError> {
    let published = simulation.latest();
    let world = simulation.world();
    save_world(
        path,
        world.blocks(),
        SavedPlayer {
            position: published.player.position.to_array(),
            yaw: published.player.yaw,
            pitch: published.player.pitch,
        },
        world.registry(),
    )
}

/// The simulation a launch starts: the saved one where a save exists, the
/// generated one where none does.
///
/// **One distinction decides it, and only one**: whether there is a save at
/// `save` at all. Nothing there is a first launch and generates; anything else
/// the reader refuses is a refusal of the launch, never a reason to generate a
/// new world over a save that is sitting right there. That is why
/// [`LoadError::Missing`] is a variant of its own rather than folded into
/// "could not be read".
///
/// **The seed is taken rather than a world built from it, and that is what makes
/// the distinction above mean anything.** A caller handing over a
/// [`ReplayWorld`] has already generated one, so "a resume derives no world from
/// the seed" would be a claim about this function while being false of the
/// process running it. Generation happens in the [`LoadError::Missing`] arm
/// below and nowhere else, which is the whole of the claim — a resumed launch
/// never reaches it, so a content root that could not generate a world is still
/// a content root a save plays against.
///
/// A resumed player is placed **from the save**, deriving no height from the
/// loaded world's blocks — the stored position is the whole of it. Only the
/// generated path still derives a spawn, and it derives it exactly as it always
/// has.
///
/// # Errors
///
/// Returns [`LaunchError::Load`] naming the save and why it could not be read,
/// [`LaunchError::WorldGen`] naming the block a first launch's world could not
/// be built without, [`LaunchError::Spawn`] where the generated world cannot
/// place a player, and [`LaunchError::Registry`] where a world holds a block
/// `registry` does not know.
pub fn simulation_at_launch(save: &Path, launching: Launching) -> Result<Simulation, LaunchError> {
    let Launching {
        seed,
        registry,
        content,
        accepting,
    } = launching;
    match load_world(save, &registry, accepting) {
        Ok(loaded) => Ok(Simulation::new(
            resuming(&loaded.player),
            World::new(loaded.world, registry)?,
            content,
        )),
        Err(LoadError::Missing { .. }) => {
            let generated = ReplayWorld::generate(seed, &registry)?;
            Ok(simulation_for(&generated, registry, content)?)
        }
        Err(refusal) => Err(LaunchError::Load {
            save: save.to_owned(),
            source: Box::new(refusal),
        }),
    }
}

/// Everything a launch needs beyond where its save is.
///
/// A group rather than four more parameters: the constitutional limit is four
/// including the receiver, and `content` is the fifth thing a launch takes.
#[derive(Debug)]
pub struct Launching {
    /// Derives a world only where there is no save to resume.
    pub seed: u64,
    /// What the world's blocks are named against.
    pub registry: Arc<BlockRegistry>,
    /// What a reader draws with, published under the first serial.
    pub content: PublishedContent,
    /// Whether a save whose blocks have changed is loaded anyway.
    pub accepting: Acceptance,
}

/// The player a save records, as the simulation resumes them.
///
/// **At rest, then gravity applies** — the spawn path's own values for the two
/// fields a save does not carry. Restoring a mid-fall velocity would resume the
/// game by dropping the player, which is why velocity is not stored at all;
/// standing them on the ground they were saved above would be a claim about
/// contact that nothing checked.
fn resuming(saved: &SavedPlayer) -> PlayerState {
    PlayerState {
        position: Vec3::from_array(saved.position),
        velocity: Vec3::ZERO,
        yaw: saved.yaw,
        pitch: saved.pitch,
        on_ground: false,
    }
}
