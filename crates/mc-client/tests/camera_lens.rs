//! The lens does not clip away a block the player is standing against.
//!
//! `NEAR_PLANE` was half a block, justified by "the orbit never enters the
//! terrain" — a justification this feature deletes along with the orbit. A player
//! stands 0.3 blocks from the face it is pressed against, *inside* that plane, so
//! the wall in front of it is clipped and it sees through the world. The defect
//! was found by playing, not by a test.
//!
//! It is worse off-axis than on it. The near **rectangle**'s corners stand
//! further from the eye than its centre does, by the factor
//! `√(1 + tan²(fov/2) + (aspect · tan(fov/2))²)` — so the eye may stand closer to
//! a face than the near distance suggests and still lose it, and the corner is
//! where the loss shows first.
//!
//! # Neither side of the comparison is a constant written here
//!
//! A test pinning `NEAR_PLANE == 0.1` would be a committed number: it stays green
//! through a later widening of the field of view or of the aspect that reopens
//! exactly this defect, because the number it pins is not the quantity that
//! matters. So the lens is read where the renderer declares it and the half-width
//! is **measured from the simulation** — by walking a player into a declared wall
//! and asking where it stopped, which is the only definition of "how close the
//! player can get to a face" that survives a change to how collision resolves.
//! Either side moving is what the assertion sees.
//!
//! # Why this file is in the composition root
//!
//! It reads a lens the renderer declares against a box the simulation owns, and
//! neither of those crates may resolve the other in any dependency kind. This is
//! the one crate that resolves both.

use glam::Vec3;
use mc_render::camera::projection_for;
use mc_render::surface::SurfaceSize;
use mc_sim::player::{
    BlockPos, Medium, MovementIntent, PlayerState, Solidity, VoxelMedium, advance_player,
};

/// The frame the declared lens is asked about: the size every committed golden
/// is captured at, and the aspect the replay declares.
const FRAME: SurfaceSize = SurfaceSize {
    width: 1280,
    height: 720,
};

/// The declared fixture the half-width is measured against: a floor to stand on
/// and a wall to be stopped by.
///
/// Solid at and beyond `x = 13`, and everywhere at or below `y = 63`, so a
/// player standing on the floor at `y = 64` walking along +x is brought to rest
/// with the leading face of its box exactly on the wall's face. Where it comes to
/// rest *is* its half-width, and that is the number this test wants — not a
/// constant naming it.
struct WalledFloor;

/// The first voxel column of the wall.
const WALL: i32 = 13;

/// The last voxel layer of the floor. The feet come to rest one above it.
const FLOOR: i32 = 63;

impl Solidity for WalledFloor {
    fn is_solid(&self, at: BlockPos) -> bool {
        at.y <= FLOOR || at.x >= WALL
    }
}

/// [`VoxelMedium::NOTHING`] unconditionally, both halves, and never derived from
/// this fixture's own solidity — the rule stated at length on
/// `crates/mc-sim/tests/support/solidity.rs`'s own implementation.
impl Medium for WalledFloor {
    fn medium_at(&self, _: BlockPos) -> VoxelMedium {
        VoxelMedium::NOTHING
    }
}

/// Where the player starts: on the floor, a block clear of the wall, facing it.
const START: Vec3 = Vec3::new(12.0, 64.0, 32.0);

/// How many ticks of held forward it walks for.
///
/// The gap is 0.7 blocks and a tick walks 0.075, so ten ticks reach the wall and
/// the rest are pressed against it. Twenty is comfortably more than that and the
/// surplus changes nothing, because a walk into a face it already touches moves
/// nothing.
const HELD_TICKS: u32 = 20;

#[test]
fn the_near_planes_corners_stand_nearer_the_eye_than_the_player_can_stand_to_a_block() {
    let half_width = measured_half_width();
    let lens = projection_for(FRAME);
    let half_angle = (lens.fov_y_radians * 0.5).tan();
    let corner_radius =
        lens.near * (1.0 + half_angle.powi(2) + (lens.aspect * half_angle).powi(2)).sqrt();

    assert!(
        corner_radius < half_width,
        "a player pressed against a wall has its eye {half_width} blocks from that face, so \
         every corner of the near rectangle has to be nearer than that or the wall in front of \
         it is clipped away and it sees through the world. At a {:.1}° vertical field of view, \
         an aspect of {:.4} and a near distance of {}, the corners stand {corner_radius} blocks \
         out — the centre of the plane is only {} of that, which is why a near distance \
         compared against the half-width on its own would not have caught this. Neither number \
         here is written down: the lens is the renderer's declaration and the half-width is \
         where a walk into a declared wall actually stopped",
        lens.fov_y_radians.to_degrees(),
        lens.aspect,
        lens.near,
        lens.near
    );
}

/// How close the player's box can be brought to a block face, measured by
/// bringing it there.
fn measured_half_width() -> f32 {
    let mut player = PlayerState {
        position: START,
        velocity: Vec3::ZERO,
        yaw: 0.0,
        pitch: 0.0,
        on_ground: true,
    };
    let forward = MovementIntent {
        forward: 1.0,
        ..MovementIntent::default()
    };
    for _ in 0..HELD_TICKS {
        player = advance_player(player, &forward, &WalledFloor);
    }
    WALL as f32 - player.position.x
}
