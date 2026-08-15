//! One ray, marched through the voxel grid until it hits something.
//!
//! **One march serves all ten views.** A direct scan for the six axis views and
//! a ray march for the four corners would be two code paths producing "the same"
//! picture — two chances to get orientation wrong, with the cheap path covered
//! by most of the scenarios while the subtler one carried the defect.
//!
//! A painter's algorithm was the other candidate and was rejected on one
//! ground: depth order for an orthographic view of an axis-aligned grid is a
//! per-axis sort key, and a wrong sign there assembles a *plausible* picture out
//! of far faces rather than an obviously blank one. A first hit is correct by
//! construction, and it yields the face normal the shading needs for free.

use glam::DVec3;

use crate::format::Voxel;
use crate::render::shade::Face;
use crate::volume::Volume;

/// How far past a boundary a ray is nudged so that it lands in the cell it is
/// entering rather than the one it is leaving.
const INSIDE: f64 = 1e-9;

/// Which axis a step crossed: 0 for `x`, 1 for `y`, 2 for `z`.
type Axis = usize;

/// Which voxel a ray reached first, and through which face.
#[derive(Debug, Clone, Copy)]
pub struct Hit {
    /// The voxel the ray stopped in.
    pub voxel: Voxel,
    /// The face it arrived through.
    pub face: Face,
}

/// The first filled voxel `origin` reaches travelling along `direction`.
///
/// `None` where the ray misses the volume, or crosses it without meeting
/// anything filled.
#[must_use]
pub fn first_hit(volume: &Volume, origin: DVec3, direction: DVec3) -> Option<Hit> {
    let extent = volume.extent();
    let limits = [
        i64::from(extent.x),
        i64::from(extent.y),
        i64::from(extent.z),
    ];
    let (entry, entry_axis) = enters_at(origin, direction, limits)?;
    let inside = origin + direction * (entry + INSIDE);
    let mut cell = cell_of(inside, limits)?;
    let mut axis = entry_axis;
    let mut walk = Walk::new(inside, direction, cell);
    loop {
        if let Some(hit) = filled_at(volume, cell, axis, direction) {
            return Some(hit);
        }
        axis = walk.advance(&mut cell, limits)?;
    }
}

/// The hit `cell` stands for, when something fills it.
fn filled_at(volume: &Volume, cell: [i64; 3], axis: Axis, direction: DVec3) -> Option<Hit> {
    let voxel = as_voxel(cell)?;
    volume.material_at(voxel)?;
    Some(Hit {
        voxel,
        face: face_of(axis, direction),
    })
}

/// A ray's progress along one axis, in Amanatides–Woo terms.
#[derive(Debug, Clone, Copy)]
struct Arm {
    /// How far along the ray the next boundary on this axis lies.
    next: f64,
    /// How far along the ray one whole voxel is.
    delta: f64,
    /// Which way the ray moves: `1`, `-1`, or `0` where it does not.
    step: i64,
}

/// A ray's progress through the grid.
struct Walk {
    /// One arm per axis, in `x`, `y`, `z` order.
    arms: [Arm; 3],
}

impl Walk {
    /// The walk beginning at `inside`, moving along `direction` from `cell`.
    fn new(inside: DVec3, direction: DVec3, cell: [i64; 3]) -> Self {
        Self {
            arms: [
                arm_of(0, inside, direction, cell),
                arm_of(1, inside, direction, cell),
                arm_of(2, inside, direction, cell),
            ],
        }
    }

    /// Which axis's boundary the ray meets next.
    fn nearest(&self) -> Option<Axis> {
        (0..3).min_by(|left, right| {
            let left = self.arms.get(*left).map_or(f64::INFINITY, |arm| arm.next);
            let right = self.arms.get(*right).map_or(f64::INFINITY, |arm| arm.next);
            left.total_cmp(&right)
        })
    }

    /// Moves `cell` one voxel along the ray, answering which axis was crossed.
    ///
    /// `None` once the ray has left the volume.
    fn advance(&mut self, cell: &mut [i64; 3], limits: [i64; 3]) -> Option<Axis> {
        let axis = self.nearest()?;
        let arm = self.arms.get_mut(axis)?;
        if arm.step == 0 {
            return None;
        }
        let moved = cell.get_mut(axis)?;
        *moved = moved.checked_add(arm.step)?;
        if *moved < 0 || *moved >= limits.get(axis).copied()? {
            return None;
        }
        arm.next += arm.delta;
        Some(axis)
    }
}

/// The ray's progress along one axis, from where it currently sits.
fn arm_of(axis: Axis, inside: DVec3, direction: DVec3, cell: [i64; 3]) -> Arm {
    let along = component(direction, axis);
    if along == 0.0 {
        return Arm {
            next: f64::INFINITY,
            delta: f64::INFINITY,
            step: 0,
        };
    }
    let forward = along > 0.0;
    let here = cell.get(axis).copied().unwrap_or(0);
    let boundary = as_f64(if forward {
        here.saturating_add(1)
    } else {
        here
    });
    Arm {
        next: (boundary - component(inside, axis)) / along,
        delta: along.abs().recip(),
        step: if forward { 1 } else { -1 },
    }
}

/// How far along the ray it first meets the box, and through which axis.
fn enters_at(origin: DVec3, direction: DVec3, limits: [i64; 3]) -> Option<(f64, Axis)> {
    let mut entry = f64::NEG_INFINITY;
    let mut exit = f64::INFINITY;
    let mut axis = 0;
    for candidate in 0..3 {
        let high = as_f64(limits.get(candidate).copied()?);
        let (near, far) = slab(origin, direction, candidate, high)?;
        if near > entry {
            entry = near;
            axis = candidate;
        }
        exit = exit.min(far);
    }
    if entry > exit || exit < 0.0 {
        return None;
    }
    Some((entry.max(0.0), axis))
}

/// Where the ray enters and leaves one axis's slab.
///
/// `None` where the ray runs parallel to the slab and outside it, which no
/// amount of travel can fix.
fn slab(origin: DVec3, direction: DVec3, axis: Axis, high: f64) -> Option<(f64, f64)> {
    let along = component(direction, axis);
    let at = component(origin, axis);
    if along == 0.0 {
        return (at >= 0.0 && at <= high).then_some((f64::NEG_INFINITY, f64::INFINITY));
    }
    let first = -at / along;
    let second = (high - at) / along;
    Some(if first <= second {
        (first, second)
    } else {
        (second, first)
    })
}

/// Which cell a point sits in.
fn cell_of(point: DVec3, limits: [i64; 3]) -> Option<[i64; 3]> {
    let mut cell = [0_i64; 3];
    for axis in 0..3 {
        let at = floored(component(point, axis))?;
        if at < 0 || at >= limits.get(axis).copied()? {
            return None;
        }
        *cell.get_mut(axis)? = at;
    }
    Some(cell)
}

/// The whole number below `value`, where that is a number at all.
fn floored(value: f64) -> Option<i64> {
    let floor = value.floor();
    floor.is_finite().then_some(floor as i64)
}

/// A cell index as a coordinate.
fn as_f64(value: i64) -> f64 {
    value as f64
}

/// The voxel a cell names, where every axis is within range.
fn as_voxel(cell: [i64; 3]) -> Option<Voxel> {
    Some(Voxel {
        x: u32::try_from(cell.first().copied()?).ok()?,
        y: u32::try_from(cell.get(1).copied()?).ok()?,
        z: u32::try_from(cell.get(2).copied()?).ok()?,
    })
}

/// Which face a ray crossing `axis` in `direction` arrived through.
///
/// A ray moving along `+y` enters through the voxel's underside, so the face it
/// meets points back the way the ray came.
fn face_of(axis: Axis, direction: DVec3) -> Face {
    let along = component(direction, axis);
    match axis {
        1 if along > 0.0 => Face::Down,
        1 => Face::Up,
        2 => Face::SideZ,
        _ => Face::SideX,
    }
}

/// One axis of a vector, by index.
fn component(vector: DVec3, axis: Axis) -> f64 {
    match axis {
        0 => vector.x,
        1 => vector.y,
        _ => vector.z,
    }
}
