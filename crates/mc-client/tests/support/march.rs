//! Turning a pose and a pixel into a ray, and walking that ray through the voxel
//! grid one boundary crossing at a time.
//!
//! **Separated from the judge that uses it, and it is a separation rather than a
//! filing decision.** [`super::oracle`] answers *what is this pixel looking at*;
//! this answers *which way does a ray leave the eye, and which voxel does it
//! enter next by which face*. The second question has one right answer that has
//! nothing to do with blocks, opacity or textures, and writing it apart is what
//! keeps the judge's own rules readable beside each other.
//!
//! **The basis and the lens are built by hand and share no code with the
//! renderer's projection** — the constraint [`super::oracle`]'s header states,
//! which travels with the code it is about.
//!
//! Exact rather than sampled: stepping along a ray in fixed increments can pass
//! through a voxel whose chord is shorter than the increment, which is precisely
//! what happens where a ray clips the corner of a block — the places the oracle
//! most needs to be right about. Crossing one boundary at a time cannot skip a
//! voxel at all.

use glam::{IVec3, Vec3};
use mc_render::camera::projection_for;
use mc_render::surface::SurfaceSize;
use mc_sim::camera::CameraPose;
use mc_world::mesh::Facing;

/// The facing a voxel was entered by, given which way the march is travelling on
/// that axis: `lower` where it moves towards higher coordinates, and `higher`
/// where it moves the other way.
///
/// A march never steps zero on the axis it chose, so the middle case is
/// unreachable; it answers `lower` rather than carrying a fourth state nothing
/// can produce.
pub const fn entered_by(towards: i32, lower: Facing, higher: Facing) -> Facing {
    if towards < 0 { higher } else { lower }
}

/// A ray walking the voxel grid one boundary crossing at a time.
pub struct March {
    /// The voxel the ray is inside.
    pub voxel: IVec3,
    /// Which way each axis's voxel coordinate moves, as −1, 0 or +1.
    towards: IVec3,
    /// How far along the ray each axis's next boundary stands.
    next: Vec3,
    /// How far apart successive boundaries stand on each axis.
    between: Vec3,
    /// How far along the ray the current voxel was entered.
    pub travelled: f32,
}

impl March {
    /// The march of a ray leaving `origin` along the unit vector `direction`.
    pub fn of(origin: Vec3, direction: Vec3) -> Self {
        let voxel = origin.floor().as_ivec3();
        Self {
            voxel,
            towards: IVec3::new(
                towards(direction.x),
                towards(direction.y),
                towards(direction.z),
            ),
            next: Vec3::new(
                boundary(origin.x, direction.x, voxel.x),
                boundary(origin.y, direction.y, voxel.y),
                boundary(origin.z, direction.z, voxel.z),
            ),
            between: direction.abs().recip(),
            travelled: 0.0,
        }
    }

    /// Crosses into the next voxel, which is the one on whichever axis's
    /// boundary stands nearest, and answers the facing of it that was entered
    /// through.
    ///
    /// A ray moving towards higher x enters the voxel it reaches by that
    /// voxel's **negative** x side, which is what the pairing below says.
    pub fn step(&mut self) -> Facing {
        let next = self.next;
        if next.x <= next.y && next.x <= next.z {
            self.travelled = next.x;
            self.voxel.x += self.towards.x;
            self.next.x += self.between.x;
            entered_by(self.towards.x, Facing::NegX, Facing::PosX)
        } else if next.y <= next.z {
            self.travelled = next.y;
            self.voxel.y += self.towards.y;
            self.next.y += self.between.y;
            entered_by(self.towards.y, Facing::NegY, Facing::PosY)
        } else {
            self.travelled = next.z;
            self.voxel.z += self.towards.z;
            self.next.z += self.between.z;
            entered_by(self.towards.z, Facing::NegZ, Facing::PosZ)
        }
    }
}

/// Which way a voxel coordinate moves as the ray advances along one axis.
pub fn towards(direction: f32) -> i32 {
    if direction > 0.0 {
        1
    } else if direction < 0.0 {
        -1
    } else {
        0
    }
}

/// How far along the ray the next voxel boundary on one axis stands.
///
/// Infinite for an axis the ray does not move along, which is what keeps that
/// axis from ever being the nearest boundary.
pub fn boundary(origin: f32, direction: f32, voxel: i32) -> f32 {
    if direction > 0.0 {
        (voxel as f32 + 1.0 - origin) / direction
    } else if direction < 0.0 {
        (voxel as f32 - origin) / direction
    } else {
        f32::INFINITY
    }
}

/// Where the camera stands and the three directions it stands in.
///
/// Built by hand from the pose. `right = forward × up` and `up = right ×
/// forward` are the same two cross products the derived probes work the landmark
/// pixel out with, and they are written here rather than shared, because two
/// computations that share a step cannot check each other.
///
/// A forward direction parallel to the world's up axis would leave `right` with
/// no length. The player's pitch is clamped to ±89°, so it cannot arise from a
/// published camera; a caller handing this a degenerate pose gets a frame of
/// `NaN` rays and a prediction of nothing, which the prediction floor is what
/// catches.
pub struct Basis {
    pub eye: Vec3,
    pub forward: Vec3,
    pub right: Vec3,
    pub up: Vec3,
}

impl Basis {
    /// The basis `camera` implies.
    pub fn of(camera: &CameraPose) -> Self {
        let eye = Vec3::from_array(camera.eye);
        let forward = (Vec3::from_array(camera.target) - eye).normalize();
        let right = forward.cross(Vec3::Y).normalize();
        Self {
            eye,
            forward,
            right,
            up: right.cross(forward),
        }
    }

    /// The unit direction of the ray through the centre of `pixel`.
    ///
    /// The centre, at `pixel + 0.5`, because that is where a rasteriser samples
    /// the pixel it is filling — an oracle that marched through the pixel's
    /// corner would be judging the frame half a pixel away from where it looked.
    pub fn ray_through(&self, pixel: (u32, u32), lens: &Lens) -> Vec3 {
        let across = 2.0 * (pixel.0 as f32 + 0.5) / lens.width - 1.0;
        let down = 1.0 - 2.0 * (pixel.1 as f32 + 0.5) / lens.height;
        (self.forward
            + self.right * (across * lens.aspect * lens.tan_half_fov)
            + self.up * (down * lens.tan_half_fov))
            .normalize()
    }
}

/// How wide the frame is and how much of the world it takes in.
pub struct Lens {
    tan_half_fov: f32,
    aspect: f32,
    width: f32,
    height: f32,
}

impl Lens {
    /// The lens the renderer declares for a frame of `size`.
    pub fn of(size: SurfaceSize) -> Self {
        let projection = projection_for(size);
        Self {
            tan_half_fov: (projection.fov_y_radians * 0.5).tan(),
            aspect: projection.aspect,
            width: size.width as f32,
            height: size.height as f32,
        }
    }
}
