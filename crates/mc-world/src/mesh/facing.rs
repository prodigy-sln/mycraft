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
mod tests {
    //! Guard. The two authored facts about a facing, and everything derived from
    //! them.
    //!
    //! A facing is written down exactly twice: once as the order the variants are
    //! declared in, which is also the order faces are emitted in and the order the
    //! neighbour slots are in, and once as the axis-and-sign pair each variant
    //! carries. Everything else — which plane a face sits on, which two axes are the
    //! primary and the secondary, which end of the neighbour is consulted, where the
    //! step from a voxel lands — is computed from that pair.
    //!
    //! The first test is what keeps the list of all facings from becoming a third
    //! authored fact. It is written out once and indexed by the discriminant
    //! everywhere else, so if the two ever disagree the neighbour handed to one
    //! facing is read for another, and nothing else would say so.
    //!
    //! The second is the derivation itself, probed for every facing at three places:
    //! in the middle of the section, on the boundary plane at the facing's own end,
    //! and on the boundary plane at the far end. Those three separate the mistakes
    //! that hide in each other. A sign inversion swaps which end goes across. A
    //! wrong-side read makes the far boundary cross instead of the near one. An axis
    //! confusion moves the answer onto a coordinate that is not the facing's — which
    //! is why the two coordinates that are *not* the facing's own differ from each
    //! other and from the one that varies, so a mistake lands somewhere visible
    //! rather than on the same number by coincidence.

    use super::{Adjacent, Facing};
    use crate::section::LocalPos;

    /// A local position, spelled out.
    const fn at(x: u32, y: u32, z: u32) -> LocalPos {
        LocalPos { x, y, z }
    }

    /// Every facing, probed at three voxels, and where the step off that facing
    /// lands from each of them.
    ///
    /// `Across` is the position *within the neighbour section*, which is the
    /// mirrored coordinate: a step off the low face at 0 arrives at 15 of the
    /// section below, and a step off the high face at 15 arrives at 0 of the one
    /// above.
    const ADJACENCIES: [(Facing, LocalPos, Adjacent); 18] = [
        (Facing::NegX, at(0, 3, 5), Adjacent::Across(at(15, 3, 5))),
        (Facing::NegX, at(8, 3, 5), Adjacent::Inside(at(7, 3, 5))),
        (Facing::NegX, at(15, 3, 5), Adjacent::Inside(at(14, 3, 5))),
        (Facing::PosX, at(0, 3, 5), Adjacent::Inside(at(1, 3, 5))),
        (Facing::PosX, at(8, 3, 5), Adjacent::Inside(at(9, 3, 5))),
        (Facing::PosX, at(15, 3, 5), Adjacent::Across(at(0, 3, 5))),
        (Facing::NegY, at(3, 0, 5), Adjacent::Across(at(3, 15, 5))),
        (Facing::NegY, at(3, 8, 5), Adjacent::Inside(at(3, 7, 5))),
        (Facing::NegY, at(3, 15, 5), Adjacent::Inside(at(3, 14, 5))),
        (Facing::PosY, at(3, 0, 5), Adjacent::Inside(at(3, 1, 5))),
        (Facing::PosY, at(3, 8, 5), Adjacent::Inside(at(3, 9, 5))),
        (Facing::PosY, at(3, 15, 5), Adjacent::Across(at(3, 0, 5))),
        (Facing::NegZ, at(3, 5, 0), Adjacent::Across(at(3, 5, 15))),
        (Facing::NegZ, at(3, 5, 8), Adjacent::Inside(at(3, 5, 7))),
        (Facing::NegZ, at(3, 5, 15), Adjacent::Inside(at(3, 5, 14))),
        (Facing::PosZ, at(3, 5, 0), Adjacent::Inside(at(3, 5, 1))),
        (Facing::PosZ, at(3, 5, 8), Adjacent::Inside(at(3, 5, 9))),
        (Facing::PosZ, at(3, 5, 15), Adjacent::Across(at(3, 5, 0))),
    ];

    #[test]
    fn every_facing_is_listed_at_the_position_its_own_discriminant_names() {
        let listed: Vec<Option<Facing>> = Facing::ALL
            .iter()
            .map(|facing| Facing::ALL.get(*facing as usize).copied())
            .collect();

        assert_eq!(
            listed,
            Facing::ALL.map(Some).to_vec(),
            "the list of every facing and the discriminants are one fact, not two: a neighbour is \
             stored at the slot its facing's discriminant names and read back through this list. \
             A list whose order drifted from the declaration order would hand the section beyond \
             one facing to another, and would reorder the emitted quads by exactly the same amount \
             — so the mesh would still look self-consistent"
        );
    }

    #[test]
    fn each_facing_steps_to_its_own_neighbour_and_mirrors_across_its_own_boundary() {
        let stepped: Vec<Adjacent> = ADJACENCIES
            .iter()
            .map(|(facing, voxel, _)| facing.adjacent(*voxel))
            .collect();
        let landings: Vec<Adjacent> = ADJACENCIES.iter().map(|(_, _, landing)| *landing).collect();

        assert_eq!(
            stepped, landings,
            "each facing steps one voxel along its own axis, in its own direction, and leaves the \
             section only at the end it points at — arriving at the opposite end of the neighbour \
             there. Every row here is one of the six rows of the derived table, and a table with \
             one wrong row breaks the facing nobody happened to write a scenario about"
        );
    }
}
