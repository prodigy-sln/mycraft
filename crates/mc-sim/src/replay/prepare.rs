//! Meshing the whole replay world, once, on workers.
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
use mc_world::mesh::{Facing, MeshError, Neighbours, Quad, mesh_section};
use mc_world::section::{SECTION_SIZE, Section};
use rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};
use thiserror::Error;

use super::world::{ReplayWorld, every_column};

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
    let work = every_section(world);
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
struct SectionWork<'a> {
    column: ColumnCoordinate,
    section_index: usize,
    origin: [i32; 3],
    section: &'a Section,
    neighbours: Neighbours<'a>,
}

/// Every section of the world, in the declared assembly order.
fn every_section(world: &ReplayWorld) -> Vec<SectionWork<'_>> {
    every_column()
        .flat_map(|(column_x, column_z)| {
            (0..SECTIONS_PER_COLUMN as usize)
                .map(move |section_index| (column_x, column_z, section_index))
        })
        .filter_map(|(column_x, column_z, section_index)| {
            section_work(world, column_x, column_z, section_index)
        })
        .collect()
}

/// One section's work, or nothing if the world stacks no such section.
fn section_work(
    world: &ReplayWorld,
    column_x: u32,
    column_z: u32,
    section_index: usize,
) -> Option<SectionWork<'_>> {
    let column = world.column(column_x, column_z)?;
    // Derived from the column's own coordinate rather than from the indices it
    // was found by, so a world that ever sits somewhere other than the origin
    // reports where its sections actually are.
    let coordinate = column.coordinate();
    let size = SECTION_SIZE as i32;
    Some(SectionWork {
        column: coordinate,
        section_index,
        origin: [
            coordinate.x * size,
            section_index as i32 * size,
            coordinate.z * size,
        ],
        section: column.section(section_index)?,
        neighbours: around(world, column_x, column_z, section_index),
    })
}

/// The six sections around one, each supplied when the footprint has it.
///
/// Anything outside the footprint is left absent, so the world's outer shell and
/// its floor show faces rather than being sealed against content that does not
/// exist. Every interior boundary *is* supplied — a section meshed in isolation
/// would emit six walls of faces buried inside the world, which the independent
/// per-voxel walk reports as a disagreement of exactly that size.
fn around(
    world: &ReplayWorld,
    column_x: u32,
    column_z: u32,
    section_index: usize,
) -> Neighbours<'_> {
    let mut neighbours = Neighbours::none();
    let sideways = [
        (Facing::NegX, column_x.checked_sub(1), Some(column_z)),
        (Facing::PosX, Some(column_x + 1), Some(column_z)),
        (Facing::NegZ, Some(column_x), column_z.checked_sub(1)),
        (Facing::PosZ, Some(column_x), Some(column_z + 1)),
    ];
    for (facing, beside_x, beside_z) in sideways {
        if let Some(section) = beside_x
            .zip(beside_z)
            .and_then(|(x, z)| world.column(x, z))
            .and_then(|column| column.section(section_index))
        {
            neighbours = neighbours.with(facing, section);
        }
    }

    let column = world.column(column_x, column_z);
    let vertically = [
        (Facing::NegY, section_index.checked_sub(1)),
        (Facing::PosY, Some(section_index + 1)),
    ];
    for (facing, above_or_below) in vertically {
        if let Some(section) = above_or_below.and_then(|index| column?.section(index)) {
            neighbours = neighbours.with(facing, section);
        }
    }
    neighbours
}

/// One section's quads, or the refusal naming where it sits.
fn mesh_one(
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
