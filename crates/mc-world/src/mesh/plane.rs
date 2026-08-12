//! Addressing the 256 cells of one plane of a section.
//!
//! Two things need this. The sweep walks a facing's faces plane by plane, and
//! the boundary resolution reads the 256 voxels a neighbour shares with the
//! section being meshed. A plane addressed one way in the first and another way
//! in the second is a mesh whose boundary is decided against a transposed copy
//! of what is actually beyond it — visible only as a seam in content nobody
//! generated yet. So the addressing is written down once, here, and both read
//! it.

use crate::section::SECTION_SIZE;

use super::PlanePos;

/// How far a plane's secondary coordinate is shifted to address one of its
/// cells, and the mask that reads a coordinate back out.
///
/// Shifts and masks rather than a multiplication and a division, because
/// `clippy::integer_division` is a gate error.
const SHIFT: u32 = SECTION_SIZE.trailing_zeros();
const MASK: u32 = SECTION_SIZE - 1;

/// How many cells one plane of a section has.
pub(super) const CELLS: usize = (SECTION_SIZE * SECTION_SIZE) as usize;

/// Which cell of a plane holds the face at `at`.
pub(super) const fn cell_of(at: PlanePos) -> usize {
    (at.primary | (at.secondary << SHIFT)) as usize
}

/// Where in its plane `cell` sits.
///
/// The primary coordinate is the one that varies fastest, so walking the cells
/// in ascending order walks the plane primary-first — which is the order the
/// sweep emits in and, for a boundary plane, the order the neighbour's own
/// voxels are numbered in.
pub(super) const fn position_in_plane(cell: usize) -> PlanePos {
    let addressed = cell as u32;
    PlanePos {
        primary: addressed & MASK,
        secondary: (addressed >> SHIFT) & MASK,
    }
}
