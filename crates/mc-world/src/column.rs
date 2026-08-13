//! A chunk column: sixteen sections stacked at one world coordinate.
//!
//! The only interesting thing a column does is decide which of its sections owns
//! a given height and what that height is called once it gets there. Both halves
//! of that answer can be wrong on their own, and neither is loud about it — a
//! column that dropped the section would report the bottom of the world for every
//! height, and one that was off by a section would report a plausible block from
//! the wrong place.

use mc_core::block::BlockRegistry;
use mc_core::id::BlockName;
use thiserror::Error;

use crate::section::{Axis, Contents, LocalPos, SECTION_SIZE, Section, SectionError};

/// How many sections a column stacks.
pub const SECTIONS_PER_COLUMN: u32 = 16;

/// How many voxels tall a column is.
///
/// Derived from the two constants it is the product of, so that 256 appears
/// nowhere as a number somebody would have to keep in step by hand.
pub const COLUMN_HEIGHT: u32 = SECTION_SIZE * SECTIONS_PER_COLUMN;

/// How far a column-local height is shifted to name the section holding it, and
/// the mask that reads back the height inside that section.
///
/// A shift and a mask rather than a division and a remainder, because
/// `clippy::integer_division` is a gate error.
const SECTION_SHIFT: u32 = SECTION_SIZE.trailing_zeros();
const SECTION_MASK: u32 = SECTION_SIZE - 1;

/// The shift above only splits a height correctly while a section's size is the
/// power of two it was derived from.
const _: () = assert!(1 << SECTION_SHIFT == SECTION_SIZE);

/// Why a column could not be assembled.
///
/// A type of its own rather than a variant on [`SectionError`], because it is
/// not a section's refusal: every section handed over was fine, and what was
/// wrong was how many of them there were.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ColumnError {
    #[error("a column of {expected} sections cannot be assembled from {found}")]
    WrongSectionCount { expected: usize, found: usize },
}

/// A voxel's position inside its own column.
///
/// Plain values, like [`LocalPos`]: the accessor is what validates, so reaching
/// a voxel stays a fallible operation rather than an index into an array.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ColumnPos {
    pub x: u32,
    pub y: u32,
    pub z: u32,
}

/// Where a column sits in the world.
///
/// Signed on both axes, because half of any world sits at a negative x or z. A
/// coordinate that could not carry -2 would not fail loudly; the column would
/// quietly be a different column.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ColumnCoordinate {
    pub x: i32,
    pub z: i32,
}

/// Sixteen sections stacked at one coordinate.
///
/// The array is what makes "a column has sixteen sections" a fact about the type
/// rather than a rule someone has to enforce — and it is also the height bound,
/// since a height no section covers has no index in it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChunkColumn {
    coordinate: ColumnCoordinate,
    sections: [Section; SECTIONS_PER_COLUMN as usize],
}

impl ChunkColumn {
    /// A column at `coordinate` holding nothing at all.
    ///
    /// Takes no registry and cannot fail, for the reason
    /// [`Section::empty`] does not: nothing is not a block.
    #[must_use]
    pub fn empty(coordinate: ColumnCoordinate) -> Self {
        Self {
            coordinate,
            sections: std::array::from_fn(|_| Section::empty()),
        }
    }

    /// A column at `coordinate` every one of whose voxels holds `fill`.
    ///
    /// # Errors
    ///
    /// Returns [`SectionError::UnknownBlock`] if `registry` holds no block under
    /// that name.
    pub fn filled(
        coordinate: ColumnCoordinate,
        fill: &BlockName,
        registry: &BlockRegistry,
    ) -> Result<Self, SectionError> {
        let section = Section::filled(fill, registry)?;
        Ok(Self {
            coordinate,
            sections: std::array::from_fn(|_| section.clone()),
        })
    }

    /// A column at `coordinate` stacking `sections`, bottom-up.
    ///
    /// The order is the caller's to hold and the length is what says the column
    /// is complete, mirroring [`VoxelWorld::assembled`](crate::world::VoxelWorld::assembled)
    /// one level up. A `Vec` that refuses rather than an array that cannot,
    /// because the refusal has to be reachable by something a file can carry:
    /// a save is written by another build, an older one, or a tool nobody here
    /// has seen, and "sixteen sections" is a claim it makes rather than a fact
    /// about it.
    ///
    /// This is also what keeps a load out of the registry-validating per-voxel
    /// write path: a column arrives already built from imported sections.
    ///
    /// # Errors
    ///
    /// Returns [`ColumnError::WrongSectionCount`] unless there are exactly
    /// [`SECTIONS_PER_COLUMN`] of them.
    pub fn assembled(
        coordinate: ColumnCoordinate,
        sections: Vec<Section>,
    ) -> Result<Self, ColumnError> {
        let stacked = <[Section; SECTIONS_PER_COLUMN as usize]>::try_from(sections).map_err(
            |given: Vec<Section>| ColumnError::WrongSectionCount {
                expected: SECTIONS_PER_COLUMN as usize,
                found: given.len(),
            },
        )?;
        Ok(Self {
            coordinate,
            sections: stacked,
        })
    }

    /// Where this column sits in the world.
    pub fn coordinate(&self) -> ColumnCoordinate {
        self.coordinate
    }

    /// What the cell at `pos` holds — a block, or nothing.
    ///
    /// The `Result` says the position is one this column has; what it holds is
    /// the [`Contents`] inside it.
    ///
    /// # Errors
    ///
    /// Returns [`SectionError::OutOfBounds`] if `pos` is above the top of a
    /// column, or outside the one section's width a column spans.
    pub fn block_at(&self, pos: ColumnPos) -> Result<Contents<&BlockName>, SectionError> {
        self.sections
            .get(Self::owning_section(pos.y))
            .ok_or_else(|| Self::above_the_top(pos.y))?
            .block_at(Self::inside_that_section(pos))
    }

    /// Empties the cell at `pos`.
    ///
    /// # Errors
    ///
    /// Returns [`SectionError::OutOfBounds`] if `pos` is above the top of a
    /// column, or outside the one section's width a column spans.
    pub fn empty_at(&mut self, pos: ColumnPos) -> Result<(), SectionError> {
        self.sections
            .get_mut(Self::owning_section(pos.y))
            .ok_or_else(|| Self::above_the_top(pos.y))?
            .empty_at(Self::inside_that_section(pos))
    }

    /// Writes `block` at `pos`.
    ///
    /// # Errors
    ///
    /// Returns [`SectionError::OutOfBounds`] if `pos` is above the top of a
    /// column or outside the one section's width a column spans, and
    /// [`SectionError::UnknownBlock`] if `registry` holds no block under that
    /// name.
    pub fn set_block(
        &mut self,
        pos: ColumnPos,
        block: &BlockName,
        registry: &BlockRegistry,
    ) -> Result<(), SectionError> {
        self.sections
            .get_mut(Self::owning_section(pos.y))
            .ok_or_else(|| Self::above_the_top(pos.y))?
            .set_block(Self::inside_that_section(pos), block, registry)
    }

    /// The section this column stacks at `index`, or nothing if it stacks no
    /// such section.
    ///
    /// Bounded by the section array itself, exactly as [`block_at`](Self::block_at)
    /// is: how tall a column is is its own shape, and a second constant stating
    /// it again would be a second thing to keep in step.
    pub fn section(&self, index: usize) -> Option<&Section> {
        self.sections.get(index)
    }

    /// Every section this column stacks, bottom-up.
    ///
    /// The order is the contract, exactly as a world's column order is: whatever
    /// writes a column down writes its sections in this order, and whatever
    /// stacks one back reads them in it.
    ///
    /// Beside [`section`](Self::section) rather than in place of it, because a
    /// caller walking every section should not have to ask for indices it would
    /// then have to handle the absence of — a column has sixteen sections, and
    /// this is where the type says so.
    pub fn sections(&self) -> impl ExactSizeIterator<Item = &Section> {
        self.sections.iter()
    }

    /// Which of a column's sections would own `height`.
    ///
    /// Deliberately not bounded here. The array is asked for that index and
    /// answers `None` when it has no such section, so the height bound is the
    /// column's own shape rather than a second constant that could drift out of
    /// step with it.
    ///
    /// Public because a world asked which *section* holds a position has to
    /// answer it too, and answering it there would be the same shift written
    /// twice.
    #[must_use]
    pub const fn owning_section(height: u32) -> usize {
        (height >> SECTION_SHIFT) as usize
    }

    /// What `pos` is called inside the section that owns its height.
    ///
    /// x and z are carried through untouched: a column is exactly one section
    /// wide, so those two bounds are the section's to enforce and are already
    /// enforced there.
    const fn inside_that_section(pos: ColumnPos) -> LocalPos {
        LocalPos {
            x: pos.x,
            y: pos.y & SECTION_MASK,
            z: pos.z,
        }
    }

    /// The refusal a height no section of this column covers earns.
    ///
    /// Folding it back into the lowest section instead would report a voxel from
    /// the other end of the world, and a wrapped *write* would silently destroy
    /// one. The limit is the first height a column does not have, matching how
    /// a section reports its own bounds.
    const fn above_the_top(height: u32) -> SectionError {
        SectionError::OutOfBounds {
            axis: Axis::Y,
            value: height,
            limit: COLUMN_HEIGHT,
        }
    }
}
