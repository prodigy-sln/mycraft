//! Meshing a whole world, once, on workers.
//!
//! Two worlds reach this: the replay the goldens are shot from, and the world a
//! launch plays — which is the saved one whenever there is a save. They go
//! through one crate-visible `mesh_world` either way, because two spellings of
//! the assembly order would be two orders the day one of them was edited.
//!
//! Meshing never runs on a frame path. It runs here, at preparation, and the
//! renderer's own source is scanned to prove the call appears nowhere in it.
//!
//! **The order the results come back in is the contract, not an incidental.**
//! Only an indexed `collect` into a `Vec` may be used on this path: a `for_each`
//! into a shared sink, or a collect into a set or a map, would let the worker
//! count decide the order quads reach the packer in, and every committed golden
//! frame depends on that order being the mesher's own loop nesting.
//!
//! A section that cannot be meshed fails the whole preparation, which is the
//! opposite of the rule for re-meshing a live world — and deliberately so. There
//! is no previous mesh to keep here: the replay is a fixed fixture built from
//! five declared blocks, so a section that will not mesh is a defect in this
//! crate, and continuing would leave every golden frame measuring a world with a
//! hole in it. The failure is also reported deterministically: the results are
//! collected in full and the **first** error in section order is the one raised,
//! because a short-circuiting collect surfaces whichever worker lost the race.

use mc_core::block::BlockRegistry;
use mc_world::column::{ColumnCoordinate, SECTIONS_PER_COLUMN};
use mc_world::mesh::{Facing, MeshError, Neighbours, Quad, beside, mesh_section};
use mc_world::section::{SECTION_SIZE, Section};
use mc_world::world::VoxelWorld;
use rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};
use thiserror::Error;

use super::world::ReplayWorld;

/// One section's visible faces, and where in the world they are.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SectionQuads {
    pub column: ColumnCoordinate,
    pub section_index: usize,
    pub origin: [i32; 3],
    pub quads: Vec<Quad>,
}

/// Why the replay could not be prepared.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum PrepareError {
    #[error("section {section_index} of the column at ({x}, {z}) cannot be meshed: {source}",
        x = column.x, z = column.z)]
    Mesh {
        column: ColumnCoordinate,
        section_index: usize,
        source: MeshError,
    },
}

/// Every section of `world`, meshed, in the declared assembly order.
///
/// Columns `(cz, cx)` ascending, then section index ascending, then the mesher's
/// own quad order, untouched.
///
/// # Errors
///
/// Returns [`PrepareError::Mesh`] naming the first section in that order which
/// could not be meshed.
pub fn mesh_all(
    world: &ReplayWorld,
    registry: &BlockRegistry,
) -> Result<Vec<SectionQuads>, PrepareError> {
    mesh_world(world.blocks(), registry)
}

/// Every section of `blocks`, meshed, in the declared assembly order.
///
/// **One meshing body with two doors**, and the second door is what a launch
/// resuming a save goes through: the world a player is handed is not the
/// replay's, so a mesher that could only be reached with a [`ReplayWorld`] in
/// hand could not be asked about it at all. Anything that decided the order
/// twice would decide it differently the first time one of the two was edited,
/// and the order is what every committed golden frame was shot under.
///
/// The footprint is the world's own rather than the replay's constant, which is
/// what makes this answerable for a world of any size. Nothing in this spec
/// produces one, and the generality is a consequence of asking the world rather
/// than a feature built for a caller that does not exist.
///
/// # Errors
///
/// Returns [`PrepareError::Mesh`] naming the first section in assembly order
/// which could not be meshed.
pub(crate) fn mesh_world(
    blocks: &VoxelWorld,
    registry: &BlockRegistry,
) -> Result<Vec<SectionQuads>, PrepareError> {
    let work = every_section(blocks);
    // `collect_into_vec` rather than `collect`: it exists only on an *indexed*
    // parallel iterator, so choosing it is what makes the result's order the
    // work's order by construction rather than by rayon's goodwill.
    let mut meshed: Vec<Result<SectionQuads, PrepareError>> = Vec::new();
    work.par_iter()
        .map(|section| mesh_one(section, registry))
        .collect_into_vec(&mut meshed);

    let mut prepared = Vec::with_capacity(meshed.len());
    for outcome in meshed {
        prepared.push(outcome?);
    }
    Ok(prepared)
}

/// One section, the six sections around it, and where it sits in the world.
///
/// Resolved before any worker starts, so the work a worker does holds no
/// question that could be answered `None` halfway through. A section the world
/// does not stack simply produces no entry here, and a caller counting the
/// results is what notices — an error variant for it would be an error nothing
/// could raise.
/// Crate-visible because a re-mesh assembles the same five facts out of an owned
/// batch instead of out of a world, and then wants the *same* meshing step —
/// down to how a failure names where it happened.
pub(crate) struct SectionWork<'a> {
    pub(crate) column: ColumnCoordinate,
    pub(crate) section_index: usize,
    pub(crate) origin: [i32; 3],
    pub(crate) section: &'a Section,
    pub(crate) neighbours: Neighbours<'a>,
}

/// Every section of the world, in the declared assembly order: `cz` ascending
/// outermost, then `cx`, then section index.
///
/// The bounds are the world's own footprint and not the replay's constant, so a
/// world of another size is walked whole rather than truncated to four columns
/// or asked about columns it does not have.
fn every_section(blocks: &VoxelWorld) -> Vec<SectionWork<'_>> {
    let across = blocks.footprint_columns();
    (0..across)
        .flat_map(move |column_z| (0..across).map(move |column_x| (column_x, column_z)))
        .flat_map(|(column_x, column_z)| {
            (0..SECTIONS_PER_COLUMN as usize)
                .map(move |section_index| (column_x, column_z, section_index))
        })
        .filter_map(|(column_x, column_z, section_index)| {
            section_work(blocks, column_x, column_z, section_index)
        })
        .collect()
}

/// One section's work, or nothing if the world stacks no such section.
///
/// **Every section is looked up by where it *is*, never by where it sits in a
/// `Vec`.** A world hands out its columns in insertion order, and one whose
/// insertion order ever disagreed with its coordinates would relabel every
/// section with another section's origin — geometry that reads back correctly
/// section by section and draws the world inside out.
fn section_work(
    blocks: &VoxelWorld,
    column_x: u32,
    column_z: u32,
    section_index: usize,
) -> Option<SectionWork<'_>> {
    let column = blocks.column(column_x, column_z)?;
    // Derived from the column's own coordinate rather than from the indices it
    // was found by, so a world that ever sits somewhere other than the origin
    // reports where its sections actually are.
    let coordinate = column.coordinate();
    Some(SectionWork {
        column: coordinate,
        section_index,
        origin: section_origin(coordinate, section_index),
        section: column.section(section_index)?,
        neighbours: around(
            |at, index| blocks.section_at(at, index),
            coordinate,
            section_index,
        ),
    })
}

/// Where a section's near corner sits in the world, in blocks.
///
/// Stated once because a re-mesh needs the same answer for a section it was
/// handed on its own, with no column in scope to read it off.
pub(crate) const fn section_origin(column: ColumnCoordinate, section_index: usize) -> [i32; 3] {
    let size = SECTION_SIZE as i32;
    [
        column.x * size,
        section_index as i32 * size,
        column.z * size,
    ]
}

/// The six sections around one, each supplied when `section_at` has it.
///
/// Anything the source does not hold is left absent, so the world's outer shell
/// and its floor show faces rather than being sealed against content that does
/// not exist. Every interior boundary *is* supplied — a section meshed in
/// isolation would emit six walls of faces buried inside the world, which the
/// independent per-voxel walk reports as a disagreement of exactly that size.
///
/// **Keyed by where a section is and not by where it was found.** The lookup is
/// a parameter because a re-mesh resolves the same six neighbours out of an
/// owned batch rather than out of a world, and the arithmetic that names them is
/// the part that must not be written twice.
///
/// Where each neighbour *is* comes from [`beside`], so a facing's own axis and
/// sign decide both which column is beside this one and which section is above
/// or below it. Writing them out here would be a second table of the same fact —
/// and a re-mesh marking an edit's neighbours dirty asks that same question
/// without a world in hand to answer it with.
pub(crate) fn around<'a>(
    section_at: impl Fn(ColumnCoordinate, usize) -> Option<&'a Section>,
    column: ColumnCoordinate,
    section_index: usize,
) -> Neighbours<'a> {
    let mut neighbours = Neighbours::none();
    for facing in Facing::ALL {
        if let Some(section) = beside(column, section_index, facing)
            .and_then(|(column, index)| section_at(column, index))
        {
            neighbours = neighbours.with(facing, section);
        }
    }
    neighbours
}

/// One section's quads, or the refusal naming where it sits.
pub(crate) fn mesh_one(
    work: &SectionWork<'_>,
    registry: &BlockRegistry,
) -> Result<SectionQuads, PrepareError> {
    let mesh = mesh_section(work.section, &work.neighbours, registry).map_err(|source| {
        PrepareError::Mesh {
            column: work.column,
            section_index: work.section_index,
            source,
        }
    })?;
    Ok(SectionQuads {
        column: work.column,
        section_index: work.section_index,
        origin: work.origin,
        quads: mesh.into_quads(),
    })
}
