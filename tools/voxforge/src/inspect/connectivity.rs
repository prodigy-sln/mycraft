//! What holds together, what does not, and what mirrors.
//!
//! This is the instrument the preview loop does not have. A part cantilevered
//! on nothing looks supported from every angle a renderer can offer — measured,
//! during this spec's own reference build — because something sits behind it in
//! every view. Face connectivity is decidable where a picture is not.
//!
//! **Connectivity is 6-connected**: two voxels are neighbours when they share a
//! *face*, never merely an edge or a corner. A model held together only at a
//! corner is two components, and reporting it as one would be the tool agreeing
//! with the mistake.
//!
//! **It is computed on the assembled model**, never per part. A correctly
//! attached two-part torch is one component; a per-part answer would report two
//! and be wrong about every model that has parts at all.

use std::collections::{BTreeMap, BTreeSet};

use crate::format::Voxel;
use crate::name::MaterialKey;
use crate::volume::Volume;

/// One face-connected group of filled voxels.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Component {
    /// How many voxels the group holds.
    pub voxels: usize,
    /// The group's lowest voxel, ascending by `x`, then `y`, then `z`.
    ///
    /// A member of the group rather than the corner of its box, so that it
    /// names somewhere art actually is — and, because components are disjoint,
    /// it is unique, which is what makes the reported order total.
    pub lowest: Voxel,
}

/// Which voxels touch nothing else.
///
/// A total enum rather than a possibly-empty list: "every voxel has a
/// neighbour" is the answer authors act on, and it must not read the same as a
/// check that could no longer look.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Floating {
    /// Every filled voxel shares a face with another.
    NoneDetached,
    /// These voxels each share a face with nothing, ascending by position.
    /// Never empty — that answer is [`Floating::NoneDetached`].
    Detached(Vec<Voxel>),
}

/// Whether a model mirrors about one axis' midplane.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymmetryVerdict {
    /// Every filled voxel has its mirror filled with the same material.
    Mirrored,
    /// At least one voxel's mirror is empty, or holds another material.
    NotMirrored,
}

/// The six directions a voxel shares a face with.
///
/// Six, not eighteen or twenty-six. A model held together at an edge or a
/// corner is two pieces of wood touching, not one.
const FACES: [(i64, i64, i64); 6] = [
    (-1, 0, 0),
    (1, 0, 0),
    (0, -1, 0),
    (0, 1, 0),
    (0, 0, -1),
    (0, 0, 1),
];

/// Every face-connected group of `volume`, ascending by lowest voxel.
pub(super) fn components(volume: &Volume) -> Vec<Component> {
    let filled: BTreeSet<Voxel> = volume
        .filled()
        .into_iter()
        .map(|cell| cell.position)
        .collect();
    let mut seen: BTreeSet<Voxel> = BTreeSet::new();
    let mut found = Vec::new();
    // `filled` is a `BTreeSet`, so this walks in ascending order and the first
    // voxel reached in each group is that group's lowest — which is what makes
    // the answer's order total without a sort.
    for start in &filled {
        if seen.contains(start) {
            continue;
        }
        found.push(group_from(*start, &filled, &mut seen));
    }
    found
}

/// The whole group `start` belongs to, marking every member as seen.
fn group_from(start: Voxel, filled: &BTreeSet<Voxel>, seen: &mut BTreeSet<Voxel>) -> Component {
    let mut voxels = 0_usize;
    let mut pending = vec![start];
    seen.insert(start);
    while let Some(at) = pending.pop() {
        voxels += 1;
        let reached = neighbours(at)
            .into_iter()
            .filter(|next| filled.contains(next) && seen.insert(*next));
        pending.extend(reached);
    }
    Component {
        voxels,
        lowest: start,
    }
}

/// Which voxels of `volume` share a face with nothing.
///
/// Derived from the components rather than counted again: a voxel touching no
/// other *is* a group of one, so taking both answers from one walk is what
/// stops the two disagreeing.
pub(super) fn floating(volume: &Volume) -> Floating {
    let lone: Vec<Voxel> = components(volume)
        .into_iter()
        .filter(|group| group.voxels == 1)
        .map(|group| group.lowest)
        .collect();
    if lone.is_empty() {
        return Floating::NoneDetached;
    }
    Floating::Detached(lone)
}

/// Whether `volume` mirrors about the midplane of the box its art occupies.
///
/// Mirrored **with its materials**: a chair with an oak arm and an iron arm is
/// not a mirrored chair, and reporting it as one would tell an author the
/// opposite of what they need. The spec leaves this open; this is the reading
/// that makes the observation worth printing.
///
/// The midplane comes from the filled bounds rather than the declared extent,
/// so a model that does not reach its own declared corners still mirrors about
/// the box it actually occupies.
pub(super) fn mirrors_on_x(volume: &Volume) -> SymmetryVerdict {
    let cells = volume.filled();
    let Some((lowest, highest)) = volume.filled_bounds() else {
        return SymmetryVerdict::Mirrored;
    };
    let span = lowest.x + highest.x;
    let mirrored: BTreeMap<Voxel, &MaterialKey> = cells
        .iter()
        .map(|cell| (cell.position, &cell.material))
        .collect();
    for cell in &cells {
        let Some(reflected) = span.checked_sub(cell.position.x) else {
            return SymmetryVerdict::NotMirrored;
        };
        let across = Voxel {
            x: reflected,
            ..cell.position
        };
        if mirrored.get(&across) != Some(&&cell.material) {
            return SymmetryVerdict::NotMirrored;
        }
    }
    SymmetryVerdict::Mirrored
}

/// The six voxels sharing a face with `at`, where each is a real position.
fn neighbours(at: Voxel) -> Vec<Voxel> {
    FACES
        .into_iter()
        .filter_map(|(dx, dy, dz)| {
            Some(Voxel {
                x: shifted(at.x, dx)?,
                y: shifted(at.y, dy)?,
                z: shifted(at.z, dz)?,
            })
        })
        .collect()
}

/// One coordinate moved by `step`, where that lands on a real position.
fn shifted(at: u32, step: i64) -> Option<u32> {
    u32::try_from(i64::from(at).checked_add(step)?).ok()
}
