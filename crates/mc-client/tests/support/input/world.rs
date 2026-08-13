//! The world a driven tick resolves the player's motion against: a floor, and
//! nothing else.
//!
//! **A floor rather than an empty world.** Gravity acts on every tick, so a world
//! that answered "nothing is solid" would drop the player continuously and every
//! scenario asserting that a run matches a no-input control would be comparing
//! two falls — red against a *correct* client, for a reason nothing in the
//! assertion could show.
//!
//! **A floor rather than the walled one the lens scenarios use.** A wall stops a
//! player after about ten ticks, and the binding table is driven for twenty in
//! four directions; walking into a face already touched moves nothing, so a row
//! stopped by a wall would be indistinguishable from a row that never reached the
//! player at all.

use glam::Vec3;
use mc_sim::player::{BlockPos, PlayerState, Solidity};
use mc_sim::simulation::Simulation;

/// The last voxel layer of the floor. The feet come to rest one above it.
const FLOOR: i32 = 63;

/// Where the player stands: on the floor, facing along +x, holding still.
const SPAWN: Vec3 = Vec3::new(32.0, 64.0, 32.0);

/// Solid at and below the floor, and open everywhere above it.
#[derive(Debug)]
struct GroundPlane;

impl Solidity for GroundPlane {
    fn is_solid(&self, at: BlockPos) -> bool {
        at.y <= FLOOR
    }
}

/// A simulation of that world, with the player standing on it.
pub fn ground_plane() -> Simulation {
    Simulation::new(
        PlayerState {
            position: SPAWN,
            velocity: Vec3::ZERO,
            yaw: 0.0,
            pitch: 0.0,
            on_ground: true,
        },
        Box::new(GroundPlane),
    )
}
