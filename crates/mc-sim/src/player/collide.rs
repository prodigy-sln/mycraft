//! The player's box against the world's voxels: what a displacement is allowed
//! to become, and whether the world is holding the player up at the end of it.
//!
//! **A solid voxel occupies `[v, v + 1)` on each axis.** Overlap is strict on
//! both sides, so two boxes that merely touch do not overlap — which is what
//! removes the need for a skin distance, because a face resolved exactly onto a
//! blocking face is not detected again on the next tick.
//!
//! That same half-open rule is why ground contact cannot be asked as an overlap.
//! A player standing on a floor overlaps nothing at all, so contact is decided
//! by lowering the box a declared hair and asking again. Asking it *of the
//! world* at the end of every tick, rather than remembering what happened during
//! one, is what makes the answer describe where the player is — and it is what
//! makes a jump that lands exactly on a floor face well defined, since there is
//! nothing there to clamp and nothing to have remembered.
//!
//! **A displacement is resolved one axis at a time, x then z then y**, each axis
//! applied and resolved before the next begins. Resolving them together would
//! have to choose which axis to give up when a diagonal is blocked, and the
//! choice is only well defined one axis at a time: a walk pressed into an inside
//! corner stops on both walls, and a walk merely brushing past one keeps the
//! whole of the half the wall does not face. Vertical last is what makes ground
//! contact describe the *end* of the tick, which is the value the next tick's
//! jump reads.

use std::cmp::Ordering;

use glam::Vec3;

use crate::player::{BlockPos, Solidity};

/// How far the player's box reaches from the feet centre on x and z, in blocks.
const HALF_WIDTH: f32 = 0.3;

/// How tall the player's box is, in blocks.
const HEIGHT: f32 = 1.8;

/// How far the box is lowered to ask whether something is holding it up.
const CONTACT_DEPTH: f32 = 1e-4;

/// How far the box reaches from the feet centre toward its lower corner.
const REACH_LOW: Vec3 = Vec3::new(HALF_WIDTH, 0.0, HALF_WIDTH);

/// How far the box reaches from the feet centre toward its upper corner.
///
/// The feet are the centre of the box's *bottom* face, which is why the vertical
/// reach is the whole height on this side and nothing at all on the other.
const REACH_HIGH: Vec3 = Vec3::new(HALF_WIDTH, HEIGHT, HALF_WIDTH);

/// An axis-aligned box, in world coordinates.
#[derive(Debug, Clone, Copy)]
struct Aabb {
    min: Vec3,
    max: Vec3,
}

impl Aabb {
    /// The box a player whose feet centre is at `feet` occupies.
    ///
    /// A single box with no sub-boxes, no rotation and no change of shape: a
    /// voxel is a full cube or it is not solid at all, so there is no model data
    /// for a finer shape to be derived from.
    fn around(feet: Vec3) -> Self {
        Self {
            min: feet - REACH_LOW,
            max: feet + REACH_HIGH,
        }
    }

    /// The same box, dropped by `depth` blocks.
    fn lowered(self, depth: f32) -> Self {
        let descent = Vec3::Y * depth;
        Self {
            min: self.min - descent,
            max: self.max - descent,
        }
    }
}

/// Where a tick's displacement left the feet, and what the world had to say
/// about the vertical part of it.
#[derive(Debug, Clone, Copy)]
pub struct Resolution {
    pub feet: Vec3,
    /// Whether a solid voxel stopped the vertical move, in either direction.
    ///
    /// The velocity is zeroed on this and not on ground contact alone: a rise a
    /// ceiling cut short would otherwise spend the rest of its arc pressed
    /// against that ceiling while still reporting the climb it is no longer
    /// making.
    pub stopped_vertically: bool,
}

/// Where a tick's displacement leaves the feet.
///
/// **The order is binding: x, then z, then y**, each axis applied and resolved
/// before the next begins, which is why it is written out here rather than
/// looped over. Only the vertical outcome is reported, because only the vertical
/// velocity is something the world takes away.
#[must_use]
pub fn resolved_position(feet: Vec3, displacement: Vec3, world: &dyn Solidity) -> Resolution {
    let across = resolved_axis(feet, Axis::X, displacement, world).feet;
    let along = resolved_axis(across, Axis::Z, displacement, world).feet;
    let vertical = resolved_axis(along, Axis::Y, displacement, world);
    Resolution {
        feet: vertical.feet,
        stopped_vertically: vertical.stopped,
    }
}

/// Where resolving one axis left the feet, and whether something stopped it.
#[derive(Debug, Clone, Copy)]
struct Step {
    feet: Vec3,
    stopped: bool,
}

/// Where one axis of a displacement leaves the feet, stopped by whatever the box
/// would otherwise have entered.
///
/// Reading the blocking face off the moved box's own leading corner is exact
/// only while the move is under a block long, because only then can the box
/// newly overlap the *adjacent* voxel layer and no further one. That bound is
/// the caller's to keep.
fn resolved_axis(feet: Vec3, axis: Axis, displacement: Vec3, world: &dyn Solidity) -> Step {
    let distance = axis.of(displacement);
    let Some(toward) = Toward::of(distance) else {
        return Step {
            feet,
            stopped: false,
        };
    };
    let moved = axis.set(feet, axis.of(feet) + distance);
    let area = Aabb::around(moved);
    if overlaps(area, world) {
        Step {
            feet: axis.set(moved, held_at(area, axis, toward)),
            stopped: true,
        }
    } else {
        Step {
            feet: moved,
            stopped: false,
        }
    }
}

/// Where the feet sit with the box's leading face exactly on the face that
/// stopped it.
///
/// The face is read off the *blocking voxel's* own coordinate rather than off
/// the position the box approached it from, which is why a player held against
/// something reports the same figure for as long as it is held there instead of
/// one that creeps by whatever the last approach happened to leave behind. It is
/// the face itself and not a hair short of it, because a touch is not an
/// overlap and so is not detected again on the next tick.
fn held_at(area: Aabb, axis: Axis, toward: Toward) -> f32 {
    match toward {
        Toward::High => axis.of(area.max).ceil() - 1.0 - axis.of(REACH_HIGH),
        Toward::Low => axis.of(area.min).floor() + 1.0 + axis.of(REACH_LOW),
    }
}

/// Which way along an axis a move goes.
#[derive(Debug, Clone, Copy)]
enum Toward {
    Low,
    High,
}

impl Toward {
    /// The direction a distance goes in, where it goes anywhere at all.
    ///
    /// A distance of neither sign has no leading face, so there is no face for
    /// it to be resolved onto and the axis is left exactly as it was — which is
    /// what keeps a box already resting flush against a wall from being snapped
    /// somewhere by a tick that asked it to move nowhere.
    fn of(distance: f32) -> Option<Self> {
        match distance.partial_cmp(&0.0)? {
            Ordering::Greater => Some(Self::High),
            Ordering::Less => Some(Self::Low),
            Ordering::Equal => None,
        }
    }
}

/// One axis of the world, and the component of a vector it names.
#[derive(Debug, Clone, Copy)]
enum Axis {
    X,
    Y,
    Z,
}

impl Axis {
    /// This axis's component of `vector`.
    const fn of(self, vector: Vec3) -> f32 {
        match self {
            Self::X => vector.x,
            Self::Y => vector.y,
            Self::Z => vector.z,
        }
    }

    /// `vector` with this axis's component replaced by `value`.
    fn set(self, vector: Vec3, value: f32) -> Vec3 {
        match self {
            Self::X => vector.with_x(value),
            Self::Y => vector.with_y(value),
            Self::Z => vector.with_z(value),
        }
    }
}

/// Whether the world is holding the player up.
///
/// A question asked of the world rather than a memory of what happened during
/// the tick: the box is lowered by [`CONTACT_DEPTH`] and tested for overlap,
/// because a box resting exactly on a voxel's top face overlaps nothing and an
/// overlap test alone would call a standing player airborne.
#[must_use]
pub fn on_ground(feet: Vec3, world: &dyn Solidity) -> bool {
    overlaps(Aabb::around(feet).lowered(CONTACT_DEPTH), world)
}

/// Whether the player's own box stands in the voxel `at`.
///
/// Built from the same [`Aabb::around`] and [`voxels`] the collision resolution
/// is, so [`HALF_WIDTH`], [`HEIGHT`] and the half-open `[v, v + 1)` rule are
/// stated once and read here rather than restated. A placement asks this of the
/// cell it would land in, and the box is 1.8 blocks tall — so it stands in **two**
/// voxel rows, and an answer about the row the feet are in is an answer about
/// half the player.
#[must_use]
pub(crate) fn occupies(feet: Vec3, at: BlockPos) -> bool {
    covers(feet).any(|voxel| voxel == at)
}

/// Every voxel the box a player standing at `feet` would carry covers.
///
/// The cells [`overlaps_solid`] asks about, handed out rather than answered, for the
/// one caller that has a second question about the same set: whether the world is
/// loaded there at all. Built from [`Aabb::around`] and [`voxels`] like everything
/// else here, so the box's shape and the half-open rule stay stated once.
pub(crate) fn covers(feet: Vec3) -> impl Iterator<Item = BlockPos> {
    voxels(Aabb::around(feet))
}

/// Whether any solid voxel lies inside `area`.
fn overlaps(area: Aabb, world: &dyn Solidity) -> bool {
    voxels(area).any(|at| world.is_solid(at))
}

/// Whether the box a player standing at `feet` would carry overlaps anything solid.
///
/// **The one statement of that question outside the physics**, so the box's shape and
/// the half-open rule are read here rather than restated. A second copy would be
/// agreement between two copies of one decision.
pub(crate) fn overlaps_solid(feet: Vec3, world: &dyn Solidity) -> bool {
    overlaps(Aabb::around(feet), world)
}

/// The voxel a point lies in.
pub(crate) fn cell_of(point: Vec3) -> BlockPos {
    floor_voxel(point)
}

/// Every voxel `area` touches.
///
/// A voxel fills `[v, v + 1)`, so the ones an interval `[min, max]` touches run
/// `floor(min)` up to and including `ceil(max) − 1` — and that upper bound is
/// where the half-open rule earns its keep: a box whose face lands exactly on
/// `max` stops one voxel short of the one beginning there.
fn voxels(area: Aabb) -> impl Iterator<Item = BlockPos> {
    let low = floor_voxel(area.min);
    let high = ceil_voxel(area.max);
    (low.y..=high.y).flat_map(move |y| {
        (low.z..=high.z).flat_map(move |z| (low.x..=high.x).map(move |x| BlockPos { x, y, z }))
    })
}

/// The voxel a box's lower corner lies in.
fn floor_voxel(corner: Vec3) -> BlockPos {
    BlockPos {
        x: corner.x.floor() as i32,
        y: corner.y.floor() as i32,
        z: corner.z.floor() as i32,
    }
}

/// The last voxel a box's upper corner reaches.
///
/// The subtraction saturates because the conversion above it does: a coordinate
/// far enough out saturates to the smallest `i32`, and taking one from that
/// overflows and panics in a debug build — in the one loop this project does not
/// accept a panic in.
fn ceil_voxel(corner: Vec3) -> BlockPos {
    BlockPos {
        x: (corner.x.ceil() as i32).saturating_sub(1),
        y: (corner.y.ceil() as i32).saturating_sub(1),
        z: (corner.z.ceil() as i32).saturating_sub(1),
    }
}
