//! Writing what a simulation is playing, and deciding which simulation a launch
//! plays.
//!
//! **The decision lives here and not in the client.** Which world a launch
//! plays, and what a save it cannot read does to a start, are policy — and a
//! save is server state, which is what this crate is. The client wires this up
//! and decides nothing.
//!
//! **Above the scene preparation and never inside it.** A save file lying in a
//! capture's working directory must not be able to change what a golden frame
//! shows, so the branch sits where the client makes it rather than inside the
//! path the golden suites shoot through.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use glam::Vec3;
use mc_core::block::{BlockRegistry, RegistryError};
use mc_world::persistence::{
    Acceptance, LoadError, SaveError, SavedPlayer, load_world, save_world,
};
use thiserror::Error;

use crate::player::PlayerState;
use crate::replay::{ReplayWorld, SpawnError, simulation_for};
use crate::simulation::Simulation;
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
    /// **It names the file as well as the reason**, which is why it carries the
    /// path rather than deriving from [`LoadError`] alone: most of the refusals
    /// a save can raise are about its bytes and know nothing about where those
    /// bytes came from, and "the save could not be read" without saying *which*
    /// file is a message a player cannot act on. This is the level that knows
    /// the path, so this is the level that says it.
    ///
    /// **The reason is boxed**, because [`LoadError`] is a wide enum — it names
    /// every list, count and position a save can be wrong about — and a launch
    /// that succeeds would otherwise carry all of it in every `Result` it
    /// returns. That is what `clippy::result_large_err` is about, and the answer
    /// it asks for.
    #[error("{save} could not be read: {source}", save = save.display())]
    Load {
        save: PathBuf,
        #[source]
        source: Box<LoadError>,
    },
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
/// A resumed player is placed **from the save**, deriving no height from the
/// loaded world's blocks — the stored position is the whole of it. Only the
/// generated path still derives a spawn, and it derives it exactly as it always
/// has.
///
/// # Errors
///
/// Returns [`LaunchError::Load`] naming the save and why it could not be read,
/// [`LaunchError::Spawn`] where the generated world cannot place a player, and
/// [`LaunchError::Registry`] where a world holds a block `registry` does not
/// know.
pub fn simulation_at_launch(
    save: &Path,
    generated: &ReplayWorld,
    registry: Arc<BlockRegistry>,
    accepting: Acceptance,
) -> Result<Simulation, LaunchError> {
    match load_world(save, &registry, accepting) {
        Ok(loaded) => Ok(Simulation::new(
            resuming(&loaded.player),
            World::new(loaded.world, registry)?,
        )),
        Err(LoadError::Missing { .. }) => Ok(simulation_for(generated, registry)?),
        Err(refusal) => Err(LaunchError::Load {
            save: save.to_owned(),
            source: Box::new(refusal),
        }),
    }
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
