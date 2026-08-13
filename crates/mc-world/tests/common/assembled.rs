//! Worlds built by describing their sections, rather than by writing their
//! voxels one at a time.
//!
//! **The per-voxel write path is quadratic in a world's distinct blocks**: every
//! write scans the section's palette for the name it is placing, so a section
//! holding four thousand different blocks costs four thousand scans of an
//! average two thousand entries, and a world holding sixty-five thousand is
//! hopeless. Describing a section and importing it is linear, and it is the same
//! route a load takes — `SectionData` to `Section::import` to
//! `ChunkColumn::assembled` to `VoxelWorld::assembled`.
//!
//! **The described palette holds one entry per voxel, and the voxel at position
//! *n* names entry *n*.** Building the distinct set instead would be the very
//! scan this module exists to avoid, and it buys nothing: an import keeps the
//! palette it is given, a save compacts what no voxel refers to, and a table
//! records each distinct name once — so a description repeating a name says
//! exactly what one naming it once says.

use std::error::Error;

use mc_core::block::BlockRegistry;
use mc_world::column::{ChunkColumn, ColumnCoordinate, SECTIONS_PER_COLUMN};
use mc_world::section::{Contents, PaletteIndex, Section, SectionData, VOXELS_PER_SECTION};
use mc_world::world::VoxelWorld;

/// Which voxel of which section of which column is being described.
///
/// The column is its position in the world's own assembly order — `(cz, cx)`
/// ascending — the section is its height in the column bottom-up, and the offset
/// is the voxel's linear position inside the section, x fastest then y then z.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Voxel {
    pub column: usize,
    pub section: usize,
    pub offset: usize,
}

impl Voxel {
    /// Where this voxel sits in a world of `footprint` columns a side, counting
    /// every cell of a column before the first cell of the next.
    ///
    /// The numbering a fixture uses to say "the *n*th distinct block goes here".
    /// It is a fixture's own arithmetic and shares nothing with the world's
    /// addressing, which is what lets a comparison between two worlds mean
    /// something.
    #[must_use]
    pub const fn nth(self) -> usize {
        (self.column * SECTIONS_PER_COLUMN as usize + self.section) * VOXELS_PER_SECTION
            + self.offset
    }
}

/// A world of `footprint` columns a side, every voxel holding what `held` says
/// about it.
///
/// # Errors
///
/// Returns an error if a described section names a block `registry` does not
/// hold, or if the columns do not fill the footprint.
pub fn assembled_world(
    footprint: u32,
    registry: &BlockRegistry,
    held: &dyn Fn(Voxel) -> Contents,
) -> Result<VoxelWorld, Box<dyn Error>> {
    let mut columns = Vec::with_capacity((footprint * footprint) as usize);
    let mut column = 0;
    for column_z in 0..footprint {
        for column_x in 0..footprint {
            let coordinate = ColumnCoordinate {
                x: column_x as i32,
                z: column_z as i32,
            };
            let stacked = stacked_sections(column, registry, held)?;
            columns.push(ChunkColumn::assembled(coordinate, stacked)?);
            column += 1;
        }
    }
    Ok(VoxelWorld::assembled(footprint, columns)?)
}

/// Every section of the `column`th column, bottom-up.
fn stacked_sections(
    column: usize,
    registry: &BlockRegistry,
    held: &dyn Fn(Voxel) -> Contents,
) -> Result<Vec<Section>, Box<dyn Error>> {
    let mut sections = Vec::with_capacity(SECTIONS_PER_COLUMN as usize);
    for section in 0..SECTIONS_PER_COLUMN as usize {
        let described = described_section(column, section, held);
        sections.push(Section::import(&described, registry)?);
    }
    Ok(sections)
}

/// One section described as an identity palette: one entry per voxel, each voxel
/// naming its own entry.
fn described_section(
    column: usize,
    section: usize,
    held: &dyn Fn(Voxel) -> Contents,
) -> SectionData {
    let mut palette = Vec::with_capacity(VOXELS_PER_SECTION);
    let mut indices = Vec::with_capacity(VOXELS_PER_SECTION);
    for offset in 0..VOXELS_PER_SECTION {
        palette.push(held(Voxel {
            column,
            section,
            offset,
        }));
        // Every offset is below a section's voxel count, which is far below what
        // a palette position carries. Written as a fallback rather than an
        // unwrap because a fixture that can end the process is worse than one
        // that writes a zero somewhere visible.
        indices.push(PaletteIndex::new(u16::try_from(offset).unwrap_or_default()));
    }
    SectionData { palette, indices }
}
