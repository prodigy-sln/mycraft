//! The player: the state the simulation owns, the intent a client asks with, and
//! the camera that state implies.
//!
//! The split between the two types is invariant 4 in the type system. The
//! simulation owns a [`PlayerState`] and derives the next one from its own
//! previous one; a client owns a [`MovementIntent`], which has no field that
//! could carry a position, a velocity or an absolute orientation, so a client
//! cannot state where it is even by mistake.

pub(crate) mod collide;
pub mod input;
pub mod look;
pub mod physics;

use glam::Vec3;

use crate::camera::CameraPose;

pub use input::{InputState, PlayerAction};
pub use look::Look;
pub use physics::{TICK_QUANTUM, advance_player};

/// Where the player's own box stands, for the one caller outside the physics
/// that needs to know: a placement refuses a cell the player is standing in.
///
/// `pub(crate)` and not `pub`, because the box's dimensions stay this module's.
pub(crate) use collide::occupies;

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

/// Whether a ray may stop at a voxel.
///
/// **A second narrow trait rather than a second method on [`Solidity`], and the
/// separation is the point.** Collision reads solidity at nine sites and means
/// "does this stop a player" by it; the walk a swing travels means "may this be
/// aimed at", and content declares the two independently. One trait carrying
/// both questions would give every one of those nine sites access to a question
/// it must never ask, and a collision scenario could then exercise aiming by
/// accident. One type answers both; each consumer depends on the one question it
/// asks.
///
/// **Total**, for the same reason and by the same construction as [`Solidity`]:
/// every position has an answer, and everything outside the loaded world answers
/// `false`. The walk depends on that — it stops when the next voxel's entry
/// distance exceeds the reach, so a ray that meets nothing must terminate on the
/// bound rather than on running out of world.
pub trait Targetable {
    /// Whether a ray may stop at the voxel at `at`.
    fn is_targetable(&self, at: BlockPos) -> bool;
}

/// What a voxel's volume does to something moving through it.
///
/// Two independent declarations in one value, because a caller that could read
/// one without the other is the disagreement this type exists to make
/// unspellable — the same reason [`crate::replay::ResolvedVoxels::set`] writes
/// every answer at once.
///
/// **No [`Default`], deliberately**: `..Default::default()` would make
/// inheriting a field invisible again, and the scenarios that separate a
/// resistant block from a buoyant one depend on a fixture stating both.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VoxelMedium {
    /// Whether a player can hold itself up in this volume.
    pub swimmable: bool,
    /// How much this volume slows what moves through it: a speed through it is
    /// divided by `1 + resistance`. Finite and not less than zero.
    pub resistance: f32,
}

impl VoxelMedium {
    /// What a cell with no block in it answers, and what everything outside the
    /// world answers: neither buoyant nor resistant. The identity of
    /// [`with`](Self::with).
    pub const NOTHING: Self = Self {
        swimmable: false,
        resistance: 0.0,
    };

    /// The medium of two overlapped cells taken together: buoyant if either is,
    /// and the greater of the two resistances.
    #[must_use]
    pub fn with(self, other: Self) -> Self {
        Self {
            swimmable: self.swimmable || other.swimmable,
            resistance: self.resistance.max(other.resistance),
        }
    }
}

/// What medium a voxel is.
///
/// **One trait with one method returning both answers**, where [`Solidity`] and
/// [`Targetable`] are two traits — and the difference is which hazard is live.
/// Those two are read by different code that must not reach each other's
/// question. Both of these are read by [`physics::advance_player`] alone, one
/// line apart, folded over one box at one instant, so segregating them would
/// separate nothing. The live hazard here is the opposite one: a fixture
/// stating one property and inheriting the other, which no assertion inside the
/// physics can see.
///
/// **Total**, by the same construction as [`Solidity`] and [`Targetable`]:
/// every position has an answer, and everything outside the loaded world
/// answers [`VoxelMedium::NOTHING`].
pub trait Medium {
    /// What medium the voxel at `at` is.
    fn medium_at(&self, at: BlockPos) -> VoxelMedium;
}

/// What one tick of motion may ask of the world, and no more.
///
/// [`Targetable`] is deliberately absent: a tick of motion has no aiming
/// question to ask, and this composite is where that is stated. The blanket impl
/// means a fixture writes nothing extra — implement [`Solidity`] and [`Medium`]
/// and this follows.
///
/// It does **not** make an inconsistent pair unspellable; the blanket impl
/// composes two independently written halves. What it buys is one wiring
/// argument at the one production site, so there is no arrangement of that call
/// that passes a stale view beside a fresh one.
pub trait Traversal: Solidity + Medium {}

impl<T: Solidity + Medium + ?Sized> Traversal for T {}

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
