//! A finite world of chunk columns, addressed in world coordinates.
//!
//! A column knows where a height lives inside itself and a section knows where a
//! voxel lives inside itself; nothing until now knew which *column* a world
//! coordinate belonged to. That arithmetic existed twice — once in the replay's
//! own world and once in its solidity resolver — and this is where it is stated
//! instead.
//!
//! **The addressing is unsigned, and the refusal is what makes it safe.** Every
//! axis of the footprint is a power of two blocks wide, which is exactly the
//! shape in which "split a coordinate into a column and a position inside it" is
//! a shift and a mask — and a mask cannot fail. A coordinate one past the far
//! edge masks straight back onto the near one, so a write meant for outside the
//! world would land silently on a cell that is inside it. Every accessor here
//! therefore tests the extent *before* it masks, and answers
//! [`WorldError::OutsideWorld`] naming the position it was asked about. Nothing
//! in this module wraps, saturates or folds.
//!
//! Signs are not read here at all: a negative coordinate is refused one level up,
//! where the player's box asks about voxels it may legitimately be standing
//! outside of.

use mc_core::block::{BlockRegistry, RegistryError};
use mc_core::id::BlockName;
use thiserror::Error;

use crate::column::{COLUMN_HEIGHT, ChunkColumn, ColumnCoordinate, ColumnPos};
use crate::section::{Contents, SECTION_SIZE, Section, SectionError};

/// How far a world coordinate is shifted to name the column holding it, and the
/// mask that reads back the position inside that column.
///
/// A shift and a mask rather than a division and a remainder, which is the same
/// arithmetic for a power of two and is what `clippy::integer_division` leaves
/// available.
const SECTION_SHIFT: u32 = SECTION_SIZE.trailing_zeros();
const SECTION_MASK: u32 = SECTION_SIZE - 1;

/// The shift above only splits a coordinate correctly while a section's size is
/// the power of two it was derived from.
const _: () = assert!(1 << SECTION_SHIFT == SECTION_SIZE);

/// How many voxels a volume spans on each axis, counted from the origin.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Extent {
    pub x: u32,
    pub y: u32,
    pub z: u32,
}

impl Extent {
    /// How many voxels the extent holds.
    #[must_use]
    pub const fn voxel_count(self) -> usize {
        self.x as usize * self.y as usize * self.z as usize
    }

    /// Whether a position lies inside the extent.
    #[must_use]
    pub const fn contains(self, at: WorldPos) -> bool {
        at.x < self.x && at.y < self.y && at.z < self.z
    }

    /// Where a position sits in the linear numbering [`positions`](Extent::positions)
    /// walks: x fastest, then z, then y.
    ///
    /// Meaningful only for a position the extent [`contains`](Extent::contains);
    /// every caller tests that first, which is what keeps this arithmetic rather
    /// than fallible.
    #[must_use]
    pub const fn offset(self, at: WorldPos) -> usize {
        (at.y as usize * self.z as usize + at.z as usize) * self.x as usize + at.x as usize
    }

    /// Every position inside the extent, x fastest.
    pub fn positions(self) -> impl Iterator<Item = WorldPos> {
        (0..self.y).flat_map(move |y| {
            (0..self.z).flat_map(move |z| (0..self.x).map(move |x| WorldPos { x, y, z }))
        })
    }
}

/// A position inside a world, in the unsigned coordinates a world is indexed by.
///
/// Distinct from the simulation's own signed block position on purpose: the
/// player is not confined to the world and asks about voxels outside it, and
/// this is what is left of such a question once the sign has been read and
/// refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WorldPos {
    pub x: u32,
    pub y: u32,
    pub z: u32,
}

/// A square footprint of chunk columns, addressed in world coordinates.
///
/// The columns are held in one assembly order — `(cz, cx)` ascending — and that
/// order is what [`columns`](VoxelWorld::columns) hands out. It is a contract
/// rather than an incidental: the mesher walks columns in it, and every
/// committed golden frame depends on the order quads reach the packer in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VoxelWorld {
    /// Columns in the assembly order `(cz, cx)` ascending.
    columns: Vec<ChunkColumn>,
    /// How many columns the footprint spans along each of x and z.
    footprint_columns: u32,
}

impl VoxelWorld {
    /// A world of `footprint_columns` squared columns, holding nothing at all.
    ///
    /// Takes no registry and cannot fail, for the reason [`ChunkColumn::empty`]
    /// does not: nothing is not a block, so building an empty world cannot fail
    /// for a reason that has nothing to do with emptiness.
    #[must_use]
    pub fn empty(footprint_columns: u32) -> Self {
        let columns = every_column(footprint_columns)
            .map(|(column_x, column_z)| {
                ChunkColumn::empty(ColumnCoordinate {
                    x: column_x as i32,
                    z: column_z as i32,
                })
            })
            .collect();
        Self {
            columns,
            footprint_columns,
        }
    }

    /// A world of `footprint_columns` squared columns, every voxel holding
    /// `fill`.
    ///
    /// # Errors
    ///
    /// Returns [`WorldError::Section`] if `registry` holds no block under that
    /// name.
    pub fn filled(
        footprint_columns: u32,
        fill: &BlockName,
        registry: &BlockRegistry,
    ) -> Result<Self, WorldError> {
        let mut columns = Vec::with_capacity((footprint_columns * footprint_columns) as usize);
        for (column_x, column_z) in every_column(footprint_columns) {
            let coordinate = ColumnCoordinate {
                x: column_x as i32,
                z: column_z as i32,
            };
            columns.push(ChunkColumn::filled(coordinate, fill, registry)?);
        }
        Ok(Self {
            columns,
            footprint_columns,
        })
    }

    /// A world assembled from `columns`, which must already be in the assembly
    /// order `(cz, cx)` ascending.
    ///
    /// The order is the caller's to hold, and the length is what says the
    /// footprint is square and complete.
    ///
    /// # Errors
    ///
    /// Returns [`WorldError::WrongColumnCount`] if the columns do not fill a
    /// `footprint_columns` square exactly.
    pub fn assembled(
        footprint_columns: u32,
        columns: Vec<ChunkColumn>,
    ) -> Result<Self, WorldError> {
        let expected = (footprint_columns * footprint_columns) as usize;
        if columns.len() != expected {
            return Err(WorldError::WrongColumnCount {
                expected,
                found: columns.len(),
            });
        }
        Ok(Self {
            columns,
            footprint_columns,
        })
    }

    /// How far this world reaches on each axis, in voxels.
    #[must_use]
    pub const fn extent(&self) -> Extent {
        let across = self.footprint_columns * SECTION_SIZE;
        Extent {
            x: across,
            y: COLUMN_HEIGHT,
            z: across,
        }
    }

    /// What the cell at `at` holds — a block, or nothing.
    ///
    /// The `Result` says the position is inside this world; what it holds is the
    /// [`Contents`] inside it. A cell past the edge and a cell holding nothing
    /// are different answers and stay different all the way down.
    ///
    /// # Errors
    ///
    /// Returns [`WorldError::OutsideWorld`] if `at` is outside this world, and
    /// [`WorldError::Section`] if the column refuses the position it is asked
    /// for.
    pub fn block_at(&self, at: WorldPos) -> Result<Contents<&BlockName>, WorldError> {
        Ok(self.column_holding(at)?.block_at(inside_that_column(at))?)
    }

    /// Empties the cell at `at`.
    ///
    /// # Errors
    ///
    /// Returns [`WorldError::OutsideWorld`] if `at` is outside this world, and
    /// [`WorldError::Section`] if the column refuses the position.
    pub fn empty_at(&mut self, at: WorldPos) -> Result<(), WorldError> {
        if !self.extent().contains(at) {
            return Err(WorldError::OutsideWorld { at });
        }
        let index = self.column_index(at);
        self.columns
            .get_mut(index)
            .ok_or(WorldError::OutsideWorld { at })?
            .empty_at(inside_that_column(at))?;
        Ok(())
    }

    /// Writes `block` at `at`.
    ///
    /// # Errors
    ///
    /// Returns [`WorldError::OutsideWorld`] if `at` is outside this world, and
    /// [`WorldError::Section`] if the column refuses the position or `registry`
    /// holds no block under that name.
    pub fn set_block(
        &mut self,
        at: WorldPos,
        block: &BlockName,
        registry: &BlockRegistry,
    ) -> Result<(), WorldError> {
        let inside = self.extent().contains(at);
        if !inside {
            return Err(WorldError::OutsideWorld { at });
        }
        let index = self.column_index(at);
        self.columns
            .get_mut(index)
            .ok_or(WorldError::OutsideWorld { at })?
            .set_block(inside_that_column(at), block, registry)?;
        Ok(())
    }

    /// The column at `(column_x, column_z)` in column coordinates, or nothing
    /// outside the footprint.
    #[must_use]
    pub fn column(&self, column_x: u32, column_z: u32) -> Option<&ChunkColumn> {
        if column_x >= self.footprint_columns || column_z >= self.footprint_columns {
            return None;
        }
        self.columns
            .get((column_z * self.footprint_columns + column_x) as usize)
    }

    /// Every column, in the assembly order `(cz, cx)` ascending.
    pub fn columns(&self) -> impl Iterator<Item = &ChunkColumn> {
        self.columns.iter()
    }

    /// Which section of which column holds `at`, or nothing where this world
    /// does not reach it.
    ///
    /// The column's own coordinate rather than the indices it was found by, so a
    /// world that ever sits somewhere other than the origin names where its
    /// sections actually are — and the height is split by the column's own
    /// answer rather than by a second shift written here.
    #[must_use]
    pub fn section_holding(&self, at: WorldPos) -> Option<(ColumnCoordinate, usize)> {
        let column = self.column_holding(at).ok()?;
        Some((column.coordinate(), ChunkColumn::owning_section(at.y)))
    }

    /// The section this world holds at `column` and `index`, or nothing where it
    /// holds no such section.
    ///
    /// A signed column coordinate read back as the unsigned footprint index the
    /// world is addressed by, refusing a negative rather than converting it.
    #[must_use]
    pub fn section_at(&self, column: ColumnCoordinate, index: usize) -> Option<&Section> {
        let column_x = u32::try_from(column.x).ok()?;
        let column_z = u32::try_from(column.z).ok()?;
        self.column(column_x, column_z)?.section(index)
    }

    /// How many columns the footprint spans along each of x and z.
    #[must_use]
    pub const fn footprint_columns(&self) -> u32 {
        self.footprint_columns
    }

    /// The column holding `at`, or the refusal that the world does not reach it.
    fn column_holding(&self, at: WorldPos) -> Result<&ChunkColumn, WorldError> {
        if !self.extent().contains(at) {
            return Err(WorldError::OutsideWorld { at });
        }
        self.columns
            .get(self.column_index(at))
            .ok_or(WorldError::OutsideWorld { at })
    }

    /// Where the column holding `at` sits in the assembly order.
    ///
    /// Meaningful only once the extent has admitted `at`; both callers test that
    /// first, and the `get` they hand it to refuses anything the arithmetic
    /// could still get wrong.
    const fn column_index(&self, at: WorldPos) -> usize {
        (((at.z >> SECTION_SHIFT) * self.footprint_columns) + (at.x >> SECTION_SHIFT)) as usize
    }
}

/// Every column of a square footprint in the assembly order `(cz, cx)`
/// ascending, as `(column_x, column_z)`.
fn every_column(footprint_columns: u32) -> impl Iterator<Item = (u32, u32)> {
    (0..footprint_columns)
        .flat_map(move |column_z| (0..footprint_columns).map(move |column_x| (column_x, column_z)))
}

/// What a world position is called inside the column that holds it.
///
/// The height is carried through whole: a column stacks every height the world
/// has, so that bound is the column's own and is already enforced there.
const fn inside_that_column(at: WorldPos) -> ColumnPos {
    ColumnPos {
        x: at.x & SECTION_MASK,
        y: at.y,
        z: at.z & SECTION_MASK,
    }
}

/// Why a world refused.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum WorldError {
    /// **Never a wrapped index.** The position is quoted back as it was asked
    /// about, because the whole point of the refusal is that the caller learns
    /// where it aimed rather than where a mask would have put it.
    #[error("({x}, {y}, {z}) is outside the world", x = at.x, y = at.y, z = at.z)]
    OutsideWorld { at: WorldPos },
    #[error("a world of {expected} columns cannot be assembled from {found}")]
    WrongColumnCount { expected: usize, found: usize },
    #[error(transparent)]
    Section(#[from] SectionError),
    #[error(transparent)]
    Registry(#[from] RegistryError),
}
