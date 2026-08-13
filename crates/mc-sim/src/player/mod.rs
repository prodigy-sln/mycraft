//! The player: the state the simulation owns, the intent a client asks with, and
//! the camera that state implies.
//!
//! The split between the two types is invariant 4 in the type system. The
//! simulation owns a [`PlayerState`] and derives the next one from its own
//! previous one; a client owns a [`MovementIntent`], which has no field that
//! could carry a position, a velocity or an absolute orientation, so a client
//! cannot state where it is even by mistake.

mod collide;
pub mod input;
pub mod look;
pub mod physics;

use glam::Vec3;

use crate::camera::CameraPose;

pub use input::{InputState, PlayerAction};
pub use look::Look;
pub use physics::advance_player;

/// How far above the feet the eyes sit, in blocks.
pub const EYE_HEIGHT: f32 = 1.62;

/// Everything the simulation knows about the player.
///
/// `position` is the centre of the box's bottom face — the feet, not the eyes —
/// because that is the point every collision result and every surface height is
/// expressed against.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlayerState {
    pub position: Vec3,
    pub velocity: Vec3,
    /// Radians in `[0, 2π)`. 0 faces +x and +π/2 faces +z.
    pub yaw: f32,
    /// Radians in `[-89°, +89°]`. Positive looks up.
    pub pitch: f32,
    pub on_ground: bool,
}

/// What a client asks of one tick.
///
/// Every field is a *request*: a direction and a magnitude to walk in, a change
/// of view, and whether a jump is wanted. What comes of any of them is the
/// simulation's answer, and there is deliberately no field through which a
/// client could supply one.
///
/// [`Default`] is "asks for nothing" — no movement, no look and no jump — which
/// is the value a tick that carries no input submits.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct MovementIntent {
    pub forward: f32,
    pub strafe: f32,
    pub yaw_delta: f32,
    pub pitch_delta: f32,
    pub jump: bool,
}

/// Which voxel, in world coordinates.
///
/// Signed on every axis because the player is not confined to the world: it can
/// walk off the loaded footprint and fall below `y = 0`, and the box it carries
/// with it asks about voxels there. An unsigned coordinate would have to be
/// converted at the query, and a saturating or wrapping conversion would stand
/// the player on terrain that is not beneath it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlockPos {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

/// Whether a voxel blocks the player.
///
/// The physics reads the world through this and never through a world type. The
/// replay's world is a fixture a chunk store replaces, so binding collision to
/// it concretely would bind the simulation to the throwaway — and would force
/// every exact-position scenario to generate a 64 × 64 × 256 world in order to
/// assert that a player stops at a wall.
///
/// **Total**: every position has an answer. Outside the loaded world, below
/// `y = 0` and every negative coordinate answer `false`, so there is no failure
/// for a caller to handle and none to swallow.
pub trait Solidity {
    /// Whether the voxel at `at` blocks the player.
    fn is_solid(&self, at: BlockPos) -> bool;
}

/// The camera the player's state implies.
///
/// Derived rather than driven: the eye stands over the feet at [`EYE_HEIGHT`]
/// and nowhere else, so displacing the player displaces the camera by exactly
/// the same amount without anything having to keep the two in step.
///
/// The target is the eye plus the unit direction the yaw and pitch name. Any
/// positive length would do — a look-at only reads the direction — and one is
/// the least surprising.
#[must_use]
pub fn eye_pose(state: &PlayerState) -> CameraPose {
    let eye = state.position + Vec3::Y * EYE_HEIGHT;
    CameraPose {
        eye: eye.to_array(),
        target: (eye + look_direction(state.yaw, state.pitch)).to_array(),
    }
}

/// The unit vector a yaw and a pitch point along.
///
/// The declared basis: yaw 0 faces +x, yaw +π/2 faces +z, and positive pitch
/// looks up. Which axis takes the sine and which the cosine *is* that
/// declaration — exchanging them is a quarter turn, and negating the pitch is an
/// upside-down world, and both stay smooth, total and reproducible while being
/// wrong.
fn look_direction(yaw: f32, pitch: f32) -> Vec3 {
    let horizontal = pitch.cos();
    Vec3::new(horizontal * yaw.cos(), pitch.sin(), horizontal * yaw.sin())
}
