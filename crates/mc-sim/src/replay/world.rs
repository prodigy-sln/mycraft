//! The replay world: sixteen columns of declared strata under a declared
//! surface.
//!
//! Every value the generator uses is fixed by the specification's declaration of
//! the replay rather than chosen here, because the whole point of the world is
//! that a test can state what it should contain without having run it. The
//! heightmap is the one part with freedom in it, and even that is constrained:
//! see [`height`](super::height).
//!
//! **The blocks are named here, and that is a recorded exception rather than a
//! habit.** Nothing about a block is *defined* in Rust — texture and solidity
//! come only from `content/base/blocks/*.toml` through the registry this is
//! handed. What is written down is which of those content-defined blocks the
//! scripted scene places where, which in the finished engine is content itself.
//! See `crates/mc-sim/CLAUDE.md`.

use mc_core::block::BlockRegistry;
use mc_core::id::{BlockName, NamespacedIdError};
use mc_world::column::{ChunkColumn, ColumnCoordinate, ColumnPos};
use mc_world::section::{SECTION_SIZE, SectionError};
use thiserror::Error;

use super::height::heightmap;

/// How many columns the replay spans along each of x and z.
pub const FOOTPRINT_COLUMNS: u32 = 4;

/// How many blocks the replay spans along each of x and z.
pub const FOOTPRINT: u32 = FOOTPRINT_COLUMNS * SECTION_SIZE;

/// The height water fills up to, where a column's surface is lower than it.
const SEA_LEVEL: u32 = 34;

/// How many blocks of dirt sit directly under a surface.
const DIRT_DEPTH: u32 = 3;

/// The column the landmark pillar stands in, and the height its stone reaches.
const LANDMARK_COLUMN: (u32, u32) = (12, 12);
const LANDMARK_TOP: u32 = 64;

/// Which content-defined block the scene places in each of its roles.
struct Strata {
    surface: BlockName,
    subsurface: BlockName,
    depths: BlockName,
    sea: BlockName,
    sky: BlockName,
}

impl Strata {
    /// The roles as the specification declares them.
    fn declared() -> Result<Self, WorldGenError> {
        Ok(Self {
            surface: named("base:grass")?,
            subsurface: named("base:dirt")?,
            depths: named("base:stone")?,
            sea: named("base:water")?,
            sky: named("base:air")?,
        })
    }
}

/// A block name, or the refusal that it is not one.
fn named(text: &str) -> Result<BlockName, WorldGenError> {
    BlockName::parse(text).map_err(|source| WorldGenError::UnnamedBlock {
        text: text.to_owned(),
        source,
    })
}

/// The replay's world: a fixed footprint of columns, generated from a seed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplayWorld {
    /// Columns in the declared assembly order: `(cz, cx)` ascending.
    columns: Vec<ChunkColumn>,
    /// One surface height per block column, x fastest.
    heights: Vec<u32>,
}

impl ReplayWorld {
    /// Generates the replay world from `seed`, out of blocks `registry` knows.
    ///
    /// # Errors
    ///
    /// Returns [`WorldGenError::UnnamedBlock`] if a declared block name is not a
    /// namespaced name, and [`WorldGenError::Section`] if `registry` does not
    /// register one of them or a write lands outside a column.
    pub fn generate(seed: u64, registry: &BlockRegistry) -> Result<Self, WorldGenError> {
        let strata = Strata::declared()?;
        let heights = heightmap(seed);
        let mut columns = Vec::with_capacity((FOOTPRINT_COLUMNS * FOOTPRINT_COLUMNS) as usize);
        for (column_x, column_z) in every_column() {
            let coordinate = ColumnCoordinate {
                x: column_x as i32,
                z: column_z as i32,
            };
            columns.push(draw_column(&strata, registry, coordinate, &heights)?);
        }
        Ok(Self { columns, heights })
    }

    /// Every column, in the declared assembly order: `(cz, cx)` ascending.
    pub fn columns(&self) -> impl Iterator<Item = &ChunkColumn> {
        self.columns.iter()
    }

    /// The surface height of the block column at `(x, z)`, or nothing outside
    /// the footprint.
    #[must_use]
    pub fn surface_height(&self, x: u32, z: u32) -> Option<u32> {
        self.heights.get(column_offset(x, z)?).copied()
    }

    /// The block held at a world position, or nothing outside the world.
    #[must_use]
    pub fn block_at(&self, x: u32, y: u32, z: u32) -> Option<&BlockName> {
        let column = self.column(x >> SECTION_SHIFT, z >> SECTION_SHIFT)?;
        column
            .block_at(ColumnPos {
                x: x & SECTION_MASK,
                y,
                z: z & SECTION_MASK,
            })
            .ok()
    }

    /// The column at `(column_x, column_z)` in column coordinates.
    pub(super) fn column(&self, column_x: u32, column_z: u32) -> Option<&ChunkColumn> {
        if column_x >= FOOTPRINT_COLUMNS || column_z >= FOOTPRINT_COLUMNS {
            return None;
        }
        self.columns
            .get((column_z * FOOTPRINT_COLUMNS + column_x) as usize)
    }
}

/// How far a world coordinate is shifted to name the column holding it, and the
/// mask that reads back the position inside that column.
const SECTION_SHIFT: u32 = SECTION_SIZE.trailing_zeros();
const SECTION_MASK: u32 = SECTION_SIZE - 1;

/// Where a block column sits in the heightmap, or nothing outside the footprint.
fn column_offset(x: u32, z: u32) -> Option<usize> {
    (x < FOOTPRINT && z < FOOTPRINT).then(|| (z * FOOTPRINT + x) as usize)
}

/// Every column of the footprint in the declared assembly order — `(cz, cx)`
/// ascending — as `(column_x, column_z)`.
pub(super) fn every_column() -> impl Iterator<Item = (u32, u32)> {
    (0..FOOTPRINT_COLUMNS)
        .flat_map(|column_z| (0..FOOTPRINT_COLUMNS).map(move |column_x| (column_x, column_z)))
}

/// Every position inside one column's footprint, as `(local_x, local_z)`.
fn every_position() -> impl Iterator<Item = (u32, u32)> {
    (0..SECTION_SIZE).flat_map(|local_z| (0..SECTION_SIZE).map(move |local_x| (local_x, local_z)))
}

/// One column, filled with sky and then written down to the world floor.
fn draw_column(
    strata: &Strata,
    registry: &BlockRegistry,
    coordinate: ColumnCoordinate,
    heights: &[u32],
) -> Result<ChunkColumn, WorldGenError> {
    let mut pen = Pen::over(strata, registry, coordinate)?;
    for local in every_position() {
        let (x, z) = world_position(coordinate, local);
        let surface = surface_of(heights, x, z)?;
        pen.draw_block_column(local, surface, (x, z) == LANDMARK_COLUMN)?;
    }
    Ok(pen.finish())
}

/// Where a position inside a column sits in the world.
fn world_position(coordinate: ColumnCoordinate, local: (u32, u32)) -> (u32, u32) {
    let (local_x, local_z) = local;
    (
        coordinate.x as u32 * SECTION_SIZE + local_x,
        coordinate.z as u32 * SECTION_SIZE + local_z,
    )
}

/// The declared surface height at a world position.
fn surface_of(heights: &[u32], x: u32, z: u32) -> Result<u32, WorldGenError> {
    heights
        .get(column_offset(x, z).ok_or(WorldGenError::OutsideFootprint { x, z })?)
        .copied()
        .ok_or(WorldGenError::OutsideFootprint { x, z })
}

/// The column being drawn, and what it may be drawn out of.
///
/// One value carrying the column, the strata and the registry, so that placing a
/// block stays inside the four-argument limit without any of the three becoming
/// implicit.
struct Pen<'a> {
    strata: &'a Strata,
    registry: &'a BlockRegistry,
    column: ChunkColumn,
}

impl<'a> Pen<'a> {
    /// A pen over a fresh column of nothing but sky.
    fn over(
        strata: &'a Strata,
        registry: &'a BlockRegistry,
        coordinate: ColumnCoordinate,
    ) -> Result<Self, WorldGenError> {
        Ok(Self {
            strata,
            registry,
            column: ChunkColumn::filled(coordinate, &strata.sky, registry)?,
        })
    }

    /// The column that was drawn.
    fn finish(self) -> ChunkColumn {
        self.column
    }

    /// One block column: its strata, and the landmark pillar over them where the
    /// landmark stands.
    fn draw_block_column(
        &mut self,
        local: (u32, u32),
        surface: u32,
        landmark: bool,
    ) -> Result<(), WorldGenError> {
        self.draw_strata(local, surface)?;
        if landmark {
            self.draw_landmark(local, surface)?;
        }
        Ok(())
    }

    /// Grass at the surface, dirt under it, stone to the floor, and water above
    /// the surface where the surface is under the sea. Where the surface stands
    /// above the sea that last range is empty and the column is left open to the
    /// sky.
    fn draw_strata(&mut self, local: (u32, u32), surface: u32) -> Result<(), WorldGenError> {
        // The strata reference is copied out of `self` first: it outlives this
        // call, so reading a name from it does not hold a borrow of the pen the
        // writes below need mutably.
        let strata = self.strata;
        let first_dirt = surface.saturating_sub(DIRT_DEPTH);
        for y in 0..first_dirt {
            self.place(local, y, &strata.depths)?;
        }
        for y in first_dirt..surface {
            self.place(local, y, &strata.subsurface)?;
        }
        self.place(local, surface, &strata.surface)?;
        for y in (surface + 1)..=SEA_LEVEL {
            self.place(local, y, &strata.sea)?;
        }
        Ok(())
    }

    /// The landmark pillar: stone from its column's surface to the declared top,
    /// over whatever the strata put there — including the surface block itself,
    /// which is why the world's upward grass area is one short of its column
    /// count.
    fn draw_landmark(&mut self, local: (u32, u32), surface: u32) -> Result<(), WorldGenError> {
        let strata = self.strata;
        for y in surface..=LANDMARK_TOP {
            self.place(local, y, &strata.depths)?;
        }
        Ok(())
    }

    /// One block, at one height of one block column.
    fn place(&mut self, local: (u32, u32), y: u32, block: &BlockName) -> Result<(), WorldGenError> {
        let (x, z) = local;
        self.column
            .set_block(ColumnPos { x, y, z }, block, self.registry)?;
        Ok(())
    }
}

/// Why the replay world could not be generated.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum WorldGenError {
    #[error("`{text}` is not a namespaced block name")]
    UnnamedBlock {
        text: String,
        #[source]
        source: NamespacedIdError,
    },
    #[error("({x}, {z}) is not a column of a replay {FOOTPRINT} blocks across")]
    OutsideFootprint { x: u32, z: u32 },
    #[error(transparent)]
    Section(#[from] SectionError),
}
