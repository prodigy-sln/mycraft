//! The measurable facts about an assembled model.
//!
//! Nothing here is a judgement: a count is a count, and a bounding box is where
//! the art is. Everything that could be read as "this model is wrong" lives one
//! module up, in the defect and observation partition.

use std::collections::BTreeMap;

use crate::format::{FilledCell, Voxel};
use crate::name::MaterialKey;
use crate::volume::Volume;

/// The box the filled art occupies.
///
/// A total enum rather than an `Option<(Voxel, Voxel)>`: a report that says
/// "nothing is filled" and one that could not work out an answer must not read
/// the same, and the empty case is a legitimate answer rather than a missing
/// one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bounds {
    /// No voxel is filled, so there is no box.
    Empty,
    /// The filled art spans these two corners, **both inclusive**.
    ///
    /// A solid `4 × 4 × 4` model reports `(0, 0, 0)` to `(3, 3, 3)`. This is
    /// deliberately not `world-format.md`'s exclusive plane bound: that
    /// convention is about a coordinate inside a fixed 16-wide section, and an
    /// author reading an inspect report expects the corner to be a voxel that
    /// exists.
    Spanning {
        /// The lowest corner, inclusive.
        lowest: Voxel,
        /// The highest corner, inclusive.
        highest: Voxel,
    },
}

/// How many voxels one material fills.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaterialCount {
    /// The material counted.
    pub material: MaterialKey,
    /// How many voxels of the assembled model it fills.
    pub voxels: usize,
}

/// What an assembled model measurably is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stats {
    /// How many voxels the assembled model fills.
    pub filled: usize,
    /// The inclusive box the filled art occupies.
    pub bounds: Bounds,
    /// How many voxels each material fills, ascending by material key.
    pub materials: Vec<MaterialCount>,
}

/// The facts `volume` presents.
pub(super) fn of(volume: &Volume) -> Stats {
    let cells = volume.filled();
    let mut counts: BTreeMap<MaterialKey, usize> = BTreeMap::new();
    for cell in &cells {
        *counts.entry(cell.material.clone()).or_insert(0) += 1;
    }
    Stats {
        filled: cells.len(),
        bounds: bounds_of(&cells),
        // A `BTreeMap` rather than a sort afterwards: the ascending order is
        // contract, and taking it from the structure means nothing has to
        // remember to re-sort.
        materials: counts
            .into_iter()
            .map(|(material, voxels)| MaterialCount { material, voxels })
            .collect(),
    }
}

/// The inclusive box `cells` occupy.
fn bounds_of(cells: &[FilledCell]) -> Bounds {
    let mut positions = cells.iter().map(|cell| cell.position);
    let Some(first) = positions.next() else {
        return Bounds::Empty;
    };
    let (lowest, highest) = positions.fold((first, first), |(low, high), at| {
        (
            Voxel {
                x: low.x.min(at.x),
                y: low.y.min(at.y),
                z: low.z.min(at.z),
            },
            Voxel {
                x: high.x.max(at.x),
                y: high.y.max(at.y),
                z: high.z.max(at.z),
            },
        )
    });
    Bounds::Spanning { lowest, highest }
}
