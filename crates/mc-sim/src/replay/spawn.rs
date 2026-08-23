//! Where the player starts, and the simulation that starts it there.
//!
//! The spawn is derived from the world rather than committed as a coordinate: a
//! declared column's own surface height decides how high the feet begin, so the
//! same declaration holds for any seed. That is also why a simulation cannot
//! exist before a world does — the world arrives on the preparation worker
//! several frames after the window opens, and [`simulation_for`] is what the
//! composition root calls once it has.
//!
//! Only the *height* is derived. The column, the standing height above it and
//! the facing are declarations, and they are what the replay's own assertions
//! are stated against: the feet fall exactly two blocks whatever the terrain is,
//! because three blocks up and one block above the surface are both declared
//! here.

use std::sync::Arc;

use glam::Vec3;
use mc_core::block::{BlockRegistry, RegistryError};
use thiserror::Error;

use crate::player::PlayerState;
use crate::simulation::{PublishedContent, Seated, seat};
use crate::world::World;

use super::world::ReplayWorld;

/// Why the player could not be placed in a world.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum SpawnError {
    #[error("the replay world reports no surface height for the spawn column ({x}, {z})")]
    NoSurface { x: u32, z: u32 },
    #[error("the replay world holds a block the registry does not know")]
    Solidity(#[from] RegistryError),
}

/// The block column the player spawns over: a dry column on the coast, from
/// which the sea is in frame at every declared capture tick.
///
/// **Derived by advancing the simulation, not by looking at the map.** A column
/// is only a candidate if the camera the script publishes at each declared
/// capture tick has water in it — which is a different question from whether the
/// sea is nearby, because the spawn falls three blocks and then walks, and the
/// declared world puts its water in a strip on the far `+x` edge that terrain
/// hides from anywhere further inland. The column that stood here before showed
/// no water at any tick and from no yaw at all.
///
/// This column stands at the waterline: its own surface height is exactly the
/// declared sea level, so it is dry — water fills a column only where the
/// surface is *below* that level — and the sea begins at the next column over.
/// A column below the sea level would put the player in the water, which is a
/// different scene from the one the spec asks for.
const SPAWN_COLUMN: (u32, u32) = (63, 35);

/// How far above its column's own surface height the feet start, in blocks.
///
/// High enough that the first frames show the spawn falling onto the world
/// rather than already standing on it, and low enough that the fall is over
/// long before the walk begins.
const SPAWN_ABOVE_SURFACE: u32 = 3;

/// Which way the player faces at the spawn, in degrees: toward the sea, so the
/// captured frames hold the water the content declares.
///
/// **This replaces the landmark as the reason the constant exists.** It was 225°
/// to put the pillar at column (12, 12) in the first frame. The pair was
/// re-derived together for the sea, and the pillar is now a residue rather than
/// a purpose — measured, not assumed: it reaches the first frame at four of the
/// judge's 576 sample pixels and has left by the end of the walk. Anything that
/// needs the pillar in frame should say so itself rather than lean on this.
const SPAWN_YAW_DEGREES: f32 = 230.0;

/// Where the player starts in `world`.
///
/// The horizontal centre of the declared column rather than its corner, so the
/// player's box stands over one column instead of straddling four — which is
/// what makes the height it comes to rest at that column's own surface height
/// and not the tallest of its neighbours'.
///
/// # Errors
///
/// Returns [`SpawnError::NoSurface`] when the world reports no surface height
/// for the declared spawn column. A refusal rather than a fallback height: a
/// player placed at an invented height in a world that could not say where its
/// ground is would be a replay nothing downstream could be a claim about.
pub fn spawn(world: &ReplayWorld) -> Result<PlayerState, SpawnError> {
    let (column_x, column_z) = SPAWN_COLUMN;
    let surface = world
        .surface_height(column_x, column_z)
        .ok_or(SpawnError::NoSurface {
            x: column_x,
            z: column_z,
        })?;
    Ok(PlayerState {
        position: Vec3::new(
            column_x as f32 + 0.5,
            (surface + SPAWN_ABOVE_SURFACE) as f32,
            column_z as f32 + 0.5,
        ),
        velocity: Vec3::ZERO,
        yaw: SPAWN_YAW_DEGREES.to_radians(),
        pitch: 0.0,
        on_ground: false,
    })
}

/// The simulation of `world`, with the player at the spawn `world` implies.
///
/// **The blocks are cloned rather than taken.** A few hundred kilobytes once, at
/// startup, against the alternative of consuming the `ReplayWorld` — which would
/// reorder the golden suites, since they mesh the world and build a simulation
/// from the same value. The world a frame is drawn of stays exactly as it was
/// generated; what a player edits is the simulation's own copy.
///
/// # Errors
///
/// Returns [`SpawnError::Solidity`] when the world holds a block `registry` does
/// not know, and [`SpawnError::NoSurface`] when it reports no surface height for
/// the declared spawn column.
pub fn simulation_for(
    world: &ReplayWorld,
    registry: Arc<BlockRegistry>,
    content: PublishedContent,
) -> Result<Seated, SpawnError> {
    Ok(seat(
        spawn(world)?,
        World::new(world.blocks().clone(), registry)?,
        content,
    ))
}
