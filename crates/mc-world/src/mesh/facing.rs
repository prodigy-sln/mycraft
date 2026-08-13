//! Which way a face points, and everything that follows from it.
//!
//! A facing is written down exactly twice in this crate. Once as the order the
//! six variants are declared in, which is simultaneously the order faces are
//! emitted in, the order the neighbour slots are in, and the order `Ord` sorts
//! them by — one order, not four. And once as the axis-and-sign pair each
//! variant carries.
//!
//! Everything else is computed from that pair: which plane a face sits on, which
//! two axes the face lies in and which of them is the primary one, which end of
//! a neighbour is consulted, and where a step from a voxel lands. Nothing is
//! written out per facing, because a table written out per facing is a table
//! with a row nobody checked — and the mistakes that hide in one are a sign
//! inversion, a read from the wrong side, and a primary/secondary swap, each of
//! which produces a mesh that looks entirely plausible for the five facings
//! somebody did check.

use std::fmt;

use crate::section::{Axis, LocalPos, SECTION_SIZE};

use super::PlanePos;

/// The last coordinate any axis of a section has.
const LAST_COORDINATE: u32 = SECTION_SIZE - 1;

/// One of the six directions a block face points in.
///
/// The declaration order is load-bearing: it is the emission order, the
/// neighbour slot order (`facing as usize`), and the order `Ord` gives. A
/// reordering here is a reordering of all four at once, which is the point —
/// they cannot drift apart.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Facing {
    NegX,
    PosX,
    NegY,
    PosY,
    NegZ,
    PosZ,
}

impl Facing {
    /// Every facing, in declaration order.
    ///
    /// Indexed by a facing's own discriminant everywhere it is used, and a unit
    /// test ties the two together — without it this list is a third authored
    /// fact that can quietly disagree with the discriminants it is read by.
    pub const ALL: [Self; 6] = [
        Self::NegX,
        Self::PosX,
        Self::NegY,
        Self::PosY,
        Self::NegZ,
        Self::PosZ,
    ];

    /// Which axis this facing points along.
    #[must_use]
    pub const fn axis(self) -> Axis {
        self.axis_and_sign().0
    }

    /// The one authored fact about a facing beyond where it sits in the
    /// declaration: the axis it points along, and whether it points towards
    /// higher coordinates.
    const fn axis_and_sign(self) -> (Axis, bool) {
        match self {
            Self::NegX => (Axis::X, false),
            Self::PosX => (Axis::X, true),
            Self::NegY => (Axis::Y, false),
            Self::PosY => (Axis::Y, true),
            Self::NegZ => (Axis::Z, false),
            Self::PosZ => (Axis::Z, true),
        }
    }

    /// Whether this facing points towards higher coordinates.
    const fn is_positive(self) -> bool {
        self.axis_and_sign().1
    }

    /// The offset one step off this facing carries, in x, y, z order.
    ///
    /// What `adjacent` does to a voxel inside a section, said in world
    /// coordinates instead — where there is no boundary to leave, so the step is
    /// only ever an offset. Derived from the same axis-and-sign pair
    /// as everything else here rather than written out per facing, because a
    /// table with six rows is a table with a row nobody checked.
    ///
    /// Returned as an array for callers to destructure (`let [dx, dy, dz] = …`)
    /// rather than index: `clippy::indexing_slicing` is denied workspace-wide.
    #[must_use]
    pub const fn step(self) -> [i32; 3] {
        let (axis, positive) = self.axis_and_sign();
        let distance = if positive { 1 } else { -1 };
        match axis {
            Axis::X => [distance, 0, 0],
            Axis::Y => [0, distance, 0],
            Axis::Z => [0, 0, distance],
        }
    }

    /// The two axes a face of this facing lies in, the primary one first.
    ///
    /// The two axes that are not this facing's own, in x < y < z order. A
    /// function of the axis alone, so there are three answers here and not six.
    pub(crate) const fn plane_axes(self) -> (Axis, Axis) {
        match self.axis() {
            Axis::X => (Axis::Y, Axis::Z),
            Axis::Y => (Axis::X, Axis::Z),
            Axis::Z => (Axis::X, Axis::Y),
        }
    }

    /// The voxel of a section a face of this facing sits on, given the plane it
    /// sits in and where in that plane it starts.
    pub(crate) const fn voxel_at(self, plane: u32, position: PlanePos) -> LocalPos {
        let (primary, secondary) = self.plane_axes();
        let origin = LocalPos { x: 0, y: 0, z: 0 };
        let along_the_plane = placed(origin, self.axis(), plane);
        let along_the_primary = placed(along_the_plane, primary, position.primary);
        placed(along_the_primary, secondary, position.secondary)
    }

    /// Where in the neighbour beyond this facing the voxel facing `position`
    /// lives.
    ///
    /// A face and the voxel across the boundary from it agree about where they
    /// are in the plane they share — only the coordinate along this facing's own
    /// axis differs, and it is the mirrored one, because leaving at one end
    /// arrives at the other. Both of those come from the same axis-and-sign pair
    /// as every other fact here, so reading a neighbour at the wrong end is the
    /// same mistake as stepping the wrong way rather than a second one, and
    /// [`adjacent`](Self::adjacent) answers the same question from the other
    /// direction.
    pub(crate) const fn across_at(self, position: PlanePos) -> LocalPos {
        self.voxel_at(self.mirrored_coordinate(), position)
    }

    /// Where the voxel one step off this facing of `voxel` lives.
    ///
    /// Inside the same section, unless `voxel` already sits on the boundary this
    /// facing points at — in which case the step leaves, and lands at the
    /// mirrored coordinate inside the section beyond.
    pub(crate) const fn adjacent(self, voxel: LocalPos) -> Adjacent {
        let axis = self.axis();
        let here = coordinate(voxel, axis);
        if here == self.boundary_plane() {
            return Adjacent::Across(placed(voxel, axis, self.mirrored_coordinate()));
        }
        Adjacent::Inside(placed(voxel, axis, self.stepped(here)))
    }

    /// The plane at which a step off this facing leaves the section.
    const fn boundary_plane(self) -> u32 {
        if self.is_positive() {
            LAST_COORDINATE
        } else {
            0
        }
    }

    /// What the voxel just beyond that boundary is called inside the neighbour.
    ///
    /// Leaving at one end arrives at the other: a step off the low face at 0
    /// lands on the high face at 15 of the section below.
    const fn mirrored_coordinate(self) -> u32 {
        if self.is_positive() {
            0
        } else {
            LAST_COORDINATE
        }
    }

    /// One step along this facing's own axis from `from`.
    ///
    /// `from` is never the boundary this facing points at, because the caller
    /// has already answered that case — so the decrement cannot reach below
    /// zero and the increment cannot reach past the last coordinate. It
    /// saturates rather than wrapping because a coordinate that wrapped would
    /// name a voxel at the far side of the section, and there is nothing at this
    /// depth to refuse with.
    const fn stepped(self, from: u32) -> u32 {
        if self.is_positive() {
            from.saturating_add(1)
        } else {
            from.saturating_sub(1)
        }
    }
}

impl fmt::Display for Facing {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (axis, positive) = self.axis_and_sign();
        let sign = if positive { '+' } else { '-' };
        write!(formatter, "{sign}{axis}")
    }
}

/// Where the voxel beside another one lives.
///
/// `Across` carries the position **within the neighbour section**, not within
/// the section the step started in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Adjacent {
    Inside(LocalPos),
    Across(LocalPos),
}

/// The coordinate `voxel` has on `axis`.
const fn coordinate(voxel: LocalPos, axis: Axis) -> u32 {
    match axis {
        Axis::X => voxel.x,
        Axis::Y => voxel.y,
        Axis::Z => voxel.z,
    }
}

/// `voxel` with its coordinate on `axis` replaced by `value`.
const fn placed(voxel: LocalPos, axis: Axis, value: u32) -> LocalPos {
    match axis {
        Axis::X => LocalPos {
            x: value,
            y: voxel.y,
            z: voxel.z,
        },
        Axis::Y => LocalPos {
            x: voxel.x,
            y: value,
            z: voxel.z,
        },
        Axis::Z => LocalPos {
            x: voxel.x,
            y: voxel.y,
            z: value,
        },
    }
}

#[cfg(test)]
#[path = "facing_test.rs"]
mod tests;
