//! The six sections around the one being meshed, each supplied or not.
//!
//! There is exactly one mapping from a facing to a slot in this crate, and it is
//! the facing's own discriminant. That is the whole reason the six sections are
//! not six named fields: a second place where the mapping is written down is a
//! second place it can be written down wrongly, and two slots wired to each
//! other produces a perfectly plausible mesh with one wall of the world decided
//! against the wrong chunk. Because the slot is the discriminant, a swapped slot
//! and a reordered emission are the same mistake and fail together.
//!
//! Absence is six independent options and never one flag. A section is routinely
//! meshed with the chunk below it loaded and the other five still streaming, and
//! reading that as "no neighbours at all" would put a seam under everything that
//! has not arrived yet.

use crate::column::ColumnCoordinate;
use crate::section::Section;

use super::Facing;

/// How many sections surround one.
const AROUND_A_SECTION: usize = Facing::ALL.len();

/// Where the section beyond `facing` sits, as a column and an index into it.
///
/// **The one statement of that arithmetic.** A facing's own axis and sign decide
/// both which column is beside this one and which section is above or below it,
/// and everything that needs to name a neighbour — meshing a world, meshing a
/// batch, marking an edit's neighbours dirty — needs the same answer. A second
/// spelling of it is a second place a neighbour can be named wrongly, which
/// produces a plausible mesh decided against the wrong section.
///
/// `None` where the arithmetic itself does not land anywhere: a column past the
/// far end of the coordinate, or a section below the bottom of a column. Whether
/// a world *holds* what this names is the caller's question, and a different one.
#[must_use]
pub fn beside(
    column: ColumnCoordinate,
    section_index: usize,
    facing: Facing,
) -> Option<(ColumnCoordinate, usize)> {
    let [across, up, along] = facing.step();
    let beside = ColumnCoordinate {
        x: column.x.checked_add(across)?,
        z: column.z.checked_add(along)?,
    };
    Some((beside, section_index.checked_add_signed(up as isize)?))
}

/// The sections beyond each of a section's six faces.
#[derive(Debug, Clone, Copy)]
pub struct Neighbours<'a> {
    around: [Option<&'a Section>; AROUND_A_SECTION],
}

impl Default for Neighbours<'_> {
    fn default() -> Self {
        Self {
            around: [None; AROUND_A_SECTION],
        }
    }
}

impl<'a> Neighbours<'a> {
    /// Nothing loaded around the section at all.
    ///
    /// Every boundary face is then decided as an absent neighbour is — visible,
    /// rather than sealed shut against content that is not there.
    #[must_use]
    pub fn none() -> Self {
        Self::default()
    }

    /// These neighbours with `section` beyond `facing`.
    ///
    /// Naming the facing rather than handing over an index is what makes a
    /// swapped neighbour visible at the call site, which is where a caller can
    /// still see it is wrong.
    #[must_use]
    pub fn with(mut self, facing: Facing, section: &'a Section) -> Self {
        if let Some(slot) = self.around.get_mut(facing as usize) {
            *slot = Some(section);
        }
        self
    }

    /// The section beyond `facing`, if one was supplied.
    ///
    /// Crate-internal, and staying that way. The boundary resolution is the only
    /// caller, and keeping it off the public surface is also what mechanically
    /// stops the independent visible-face oracle from reaching the very
    /// facing-to-slot mapping it exists to judge.
    pub(crate) fn at(&self, facing: Facing) -> Option<&'a Section> {
        self.around.get(facing as usize).copied().flatten()
    }
}

#[cfg(test)]
#[path = "neighbours_test.rs"]
mod tests;
