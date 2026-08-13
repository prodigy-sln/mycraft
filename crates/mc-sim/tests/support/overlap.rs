//! An independent judge of whether the player's box is inside the world.
//!
//! This is the judge, never the thing judged, and its independence is the whole
//! of its value. The physics reads a bitset that was resolved once at
//! construction; this re-reads the world's own per-voxel accessor and asks the
//! registry about every name it finds. The two share no lookup chain, so an
//! adapter that transposed an axis, saturated a coordinate or resolved a name
//! wrongly cannot make both of them wrong in the same direction — which is
//! exactly what would happen if this borrowed [`SolidVoxels`](mc_sim::replay::SolidVoxels).
//!
//! The box is re-derived here too, from the specification's declared constants
//! rather than from the physics' own: half a width of 0.3 on each horizontal
//! axis and 1.8 blocks of height above the feet. A box imported from the subject
//! would agree with a subject that shrank it.
//!
//! **The half-open rule is written out once, in [`touched`].** A solid voxel
//! occupies `[v, v + 1)` on each axis, so the voxels a box `[min, max]` touches
//! on an axis are `floor(min) ..= ceil(max) − 1` — which is what makes a box
//! whose face lies exactly on a voxel's face touch it without overlapping it.
//! Getting that wrong in the strict direction would report an overlap for every
//! player standing on any floor at all.
//!
//! It is deliberately the slow, obvious implementation: one registry lookup per
//! voxel the box touches, no bitset and no caching. Being obviously right is the
//! only property it needs.

use glam::Vec3;
use mc_core::block::{BlockRegistry, RegistryError};
use mc_sim::replay::ReplayWorld;
use mc_world::section::Contents;

/// How far the player's box reaches from its feet centre on each horizontal
/// axis, in blocks. The specification's declared 0.6-block width, halved.
pub const HALF_WIDTH: f32 = 0.3;

/// How tall the player's box is, in blocks. Declared.
pub const HEIGHT: f32 = 1.8;

/// A solid voxel the box was found inside, and what the world holds there.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Overlap {
    pub voxel: (i32, i32, i32),
    pub block: String,
}

/// Every solid voxel the player's box overlaps with its feet centre at `feet`.
///
/// Empty means clear. Anything the world holds no block at, and anything outside
/// it, is not solid — a player that has left the loaded footprint is not inside
/// it.
///
/// # Errors
///
/// Returns [`RegistryError`] if the world holds a block `registry` does not
/// register. Reported rather than read as non-solid: an unresolvable name
/// answered "clear" would turn this judge into one that approves of everything
/// it does not understand.
pub fn overlapping_voxels(
    world: &ReplayWorld,
    registry: &BlockRegistry,
    feet: Vec3,
) -> Result<Vec<Overlap>, RegistryError> {
    let mut inside = Vec::new();
    for voxel in touched_voxels(feet) {
        if let Some(overlap) = solid_at(world, registry, voxel)? {
            inside.push(overlap);
        }
    }
    Ok(inside)
}

/// Every voxel the box standing at `feet` reaches into.
fn touched_voxels(feet: Vec3) -> impl Iterator<Item = (i32, i32, i32)> {
    let min = Vec3::new(feet.x - HALF_WIDTH, feet.y, feet.z - HALF_WIDTH);
    let max = Vec3::new(feet.x + HALF_WIDTH, feet.y + HEIGHT, feet.z + HALF_WIDTH);
    touched(min.x, max.x).flat_map(move |x| {
        touched(min.z, max.z).flat_map(move |z| touched(min.y, max.y).map(move |y| (x, y, z)))
    })
}

/// The voxels an axis' span from `min` to `max` touches.
///
/// The upper end is exclusive of an exact touch, which is the half-open rule: a
/// face lying exactly on `max` belongs to the voxel beyond, and the box does not
/// reach into it.
fn touched(min: f32, max: f32) -> std::ops::RangeInclusive<i32> {
    (min.floor() as i32)..=(max.ceil() as i32 - 1)
}

/// The overlap at `voxel`, if the world holds a block there whose definition
/// calls it solid.
fn solid_at(
    world: &ReplayWorld,
    registry: &BlockRegistry,
    voxel: (i32, i32, i32),
) -> Result<Option<Overlap>, RegistryError> {
    let (x, y, z) = voxel;
    let (Ok(x), Ok(y), Ok(z)) = (u32::try_from(x), u32::try_from(y), u32::try_from(z)) else {
        return Ok(None);
    };
    // Three answers and three arms. Two of them mean "nothing to be inside of"
    // and are still written separately: a cell the world does not reach and a
    // cell holding nothing are different facts, and this judge is the only
    // unscoped invariant the simulation has — a fold here would let the same
    // conflation the subject is being judged for pass unremarked.
    let name = match world.block_at(x, y, z) {
        None => return Ok(None),
        Some(Contents::Empty) => return Ok(None),
        Some(Contents::Holds(name)) => name,
    };
    Ok(registry.resolve(name)?.is_solid.then(|| Overlap {
        voxel,
        block: name.as_str().to_owned(),
    }))
}
