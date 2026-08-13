//! The bounded walk from an eye to the first solid voxel it meets.
//!
//! **The bound is one site.** Voxels are walked in ascending entry distance and
//! the walk stops when the next voxel's entry distance exceeds the reach; there
//! is no second `distance <= reach` comparison anywhere. That is not tidiness:
//! [`Solidity`] is total and answers `false` everywhere outside the world, so an
//! unbounded traversal followed by a range check *does not terminate* for a ray
//! that hits nothing. A limit whose falsifier is a hang rather than a red
//! assertion is a limit nothing can measure.
//!
//! **This is deliberately not the golden frames' `March`.** That one lives in
//! `mc-client`'s test support and is what every committed frame is judged
//! against; promoting it to production would collapse the oracle and the
//! subject, which is the failure mode this project's testing document is largely
//! about. Two implementations, on purpose — and they differ in signature as well
//! as in body, because this one reports the face it entered through and that one
//! has no use for it.
//!
//! The voxel containing the origin is considered, at entry distance 0 and with
//! no entry face. An eye inside a solid block therefore has a target and no face
//! to place against, which is the one thing the two arms answer differently.

use std::cmp::Ordering;

use glam::Vec3;
use mc_world::mesh::Facing;
use mc_world::section::Axis;

use crate::player::{BlockPos, Solidity};

/// The voxel a ray met, how it entered, and how far along it that was.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Hit {
    pub cell: BlockPos,
    /// The face of `cell` the ray crossed to enter it, or nothing where the ray
    /// began inside it.
    pub face: Option<Facing>,
    /// How far from the origin the ray entered `cell`, in blocks.
    pub distance: f32,
}

/// The first solid voxel `direction` meets from `origin`, within `reach`
/// blocks.
///
/// `direction` need not be a unit vector; it is normalised here, which is what
/// makes `reach` mean blocks rather than multiples of whatever was passed. A
/// direction of no length points nowhere and meets nothing.
#[must_use]
pub fn targeted(origin: Vec3, direction: Vec3, reach: f32, world: &dyn Solidity) -> Option<Hit> {
    let ray = direction.normalize_or_zero();
    let mut crossings = [
        Crossing::of(Axis::X, origin.x, ray.x)?,
        Crossing::of(Axis::Y, origin.y, ray.y)?,
        Crossing::of(Axis::Z, origin.z, ray.z)?,
    ];
    let mut met = Hit {
        cell: containing(origin),
        face: None,
        distance: 0.0,
    };
    loop {
        if world.is_solid(met.cell) {
            return Some(met);
        }
        // A ray of no length leaves all three crossings at infinity, so the
        // first comparison below refuses it — the same site that bounds every
        // other ray, rather than a special case beside it.
        let next = crossings
            .iter_mut()
            .min_by(|one, other| one.at.total_cmp(&other.at))?;
        if next.at > reach {
            return None;
        }
        met = Hit {
            cell: stepped(met.cell, next.advance),
            face: Some(next.entered),
            distance: next.at,
        };
        next.at += next.apart;
    }
}

/// Where one axis' voxel boundaries are crossed, and what crossing one does.
#[derive(Debug, Clone, Copy)]
struct Crossing {
    /// Which way the cell moves when this boundary is the next one crossed.
    advance: Facing,
    /// The face of the new cell the ray comes in through — the near side going
    /// up, the far side going down.
    entered: Facing,
    /// How far along the ray the next boundary on this axis lies.
    at: f32,
    /// How far apart consecutive boundaries on this axis are, along the ray.
    apart: f32,
}

impl Crossing {
    /// This axis' crossings for a ray leaving `from` with a component of
    /// `direction` along it.
    ///
    /// A component of no length never crosses a boundary on its axis, which is
    /// an infinite distance rather than a case of its own: the walk picks the
    /// nearest crossing, and infinity is never the nearest unless every axis is
    /// standing still — in which case the reach comparison refuses it like any
    /// other ray that reaches nothing.
    fn of(axis: Axis, from: f32, direction: f32) -> Option<Self> {
        let climbing = direction.partial_cmp(&0.0)? != Ordering::Less;
        let (advance, entered) = facings_along(axis, climbing)?;
        let boundary = if climbing {
            from.floor() + 1.0
        } else {
            from.floor()
        };
        let (at, apart) = match direction.partial_cmp(&0.0)? {
            Ordering::Equal => (f32::INFINITY, f32::INFINITY),
            _ => ((boundary - from) / direction, (1.0 / direction).abs()),
        };
        Some(Self {
            advance,
            entered,
            at,
            apart,
        })
    }
}

/// The facing that steps along `axis` in the direction asked for, and the facing
/// of the face a ray travelling that way enters a cell through.
///
/// Found by asking each facing which axis it lies on and which way it steps,
/// rather than written out. `facing.rs` derives every fact it has from one
/// axis-and-sign pair precisely so that a second table cannot disagree with it,
/// and six rows here would be that second table.
fn facings_along(axis: Axis, climbing: bool) -> Option<(Facing, Facing)> {
    let on_this_axis = |facing: &Facing| facing.axis() == axis;
    let advance = Facing::ALL
        .into_iter()
        .find(|facing| on_this_axis(facing) && steps_up(*facing) == climbing)?;
    let entered = Facing::ALL
        .into_iter()
        .find(|facing| on_this_axis(facing) && steps_up(*facing) != climbing)?;
    Some((advance, entered))
}

/// Whether a facing steps towards higher coordinates.
///
/// Read off the offset it carries: exactly one component is non-zero, so the sum
/// is `+1` for the three facings that climb and `−1` for the three that do not.
fn steps_up(facing: Facing) -> bool {
    facing.step().into_iter().sum::<i32>() > 0
}

/// The voxel a point lies in.
fn containing(point: Vec3) -> BlockPos {
    BlockPos {
        x: point.x.floor() as i32,
        y: point.y.floor() as i32,
        z: point.z.floor() as i32,
    }
}

/// One step off `facing` from `cell`.
///
/// Saturating rather than wrapping: a coordinate far enough out stops moving,
/// and the walk ends on the reach comparison a step or two later instead of
/// naming a voxel on the opposite side of the world.
///
/// Visible to the resolution above because a placement takes exactly one step
/// too — back through the face the ray came in by — and a second spelling of the
/// step is a second place for it to wrap.
pub(super) fn stepped(cell: BlockPos, facing: Facing) -> BlockPos {
    let [across, up, along] = facing.step();
    BlockPos {
        x: cell.x.saturating_add(across),
        y: cell.y.saturating_add(up),
        z: cell.z.saturating_add(along),
    }
}
