//! Which faces a section shows, merged into rectangles, in the one order it
//! shows them in.
//!
//! For each facing in declaration order, for each plane ascending, for each
//! secondary coordinate ascending, for each primary coordinate ascending: where
//! a face has not been covered yet, grow a run along the **primary** axis while
//! the face is there and holds the same block, then extend that run along the
//! **secondary** axis while a whole row matches, mark what it covers, and emit.
//!
//! **The order is the loop nesting and never a sort.** Cells are visited in
//! lexicographic order and at most one quad leaves each visit, so the sequence
//! is ordered by construction — there is no comparator anywhere that could
//! disagree with the sweep about what the order is. The tempting fast variant,
//! one face mask per plane and block merged independently, is forbidden for
//! exactly that reason: it emits all of one block's quads before another's
//! within a plane, and putting them back in order afterwards would write the
//! order down a second time.
//!
//! What comes out is the scanline-greedy decomposition and deliberately not the
//! fewest rectangles that would cover the same faces. Those are different
//! answers, minimum-rectangle partition is a much harder problem, and only the
//! first of them is a *single* answer — which is what makes the output
//! comparable at all.

use mc_core::block::BlockRegistry;

use crate::section::{Contents, LocalPos, SECTION_SIZE, Section, VOXELS_PER_SECTION};

use super::facing::Adjacent;
use super::plane::{CELLS as PLANE_CELLS, cell_of, position_in_plane};
use super::resolve::{self, Boundaries, Key, Resolved};
use super::{Facing, MeshError, Neighbours, PlaneExtent, PlanePos, Quad, SectionMesh};

/// Everything a face is decided against: what the section itself holds, and how
/// solid whatever lies beyond each of its six boundaries is where the two meet.
struct Surroundings {
    resolved: Resolved,
    boundaries: Boundaries,
}

/// The faces `section` shows, merged into rectangles and ordered.
///
/// Whether a face exists is decided by the solidity `registry` registered for
/// the block holding the voxel, and by that alone — no block name and no runtime
/// id is looked at, so a block a mod ships is treated exactly as one the base
/// game ships is.
///
/// Meshing reads and never writes: every parameter is a shared reference and the
/// mesh handed back is owned, so calling `compact()` on the input to simplify
/// the sweep — which would change the palette's length, its order and the width
/// the indices are packed at — is not expressible rather than merely forbidden.
///
/// A face on a boundary is decided against the voxel facing it in the neighbour
/// beyond, voxel by voxel. A neighbour that was not supplied is decided as
/// though the voxel beyond were non-solid, so the edge of loaded content shows a
/// face rather than being sealed shut against a chunk that has not arrived — and
/// that is decided one neighbour at a time, never all at once.
///
/// # Errors
///
/// Returns [`MeshError::UnresolvedBlock`] if a voxel of `section` holds a block
/// `registry` does not register, naming the lowest such voxel in the section's
/// own linear order; [`MeshError::UnresolvedNeighbourBlock`] if a voxel of a
/// supplied neighbour *that faces `section`* does; and [`MeshError::Section`] if
/// a section cannot answer for one of its own voxels.
pub fn mesh_section(
    section: &Section,
    neighbours: &Neighbours<'_>,
    registry: &BlockRegistry,
) -> Result<SectionMesh, MeshError> {
    // The meshed section resolves before any neighbour, so that when both hold
    // something unresolvable it is the section's own refusal a caller is given.
    // The scenarios leave that open; it is fixed here so the answer does not
    // depend on which of them happened to be looked at first.
    let resolved = resolve::resolve_section(section, registry)?;
    let boundaries = resolve::resolve_boundaries(neighbours, registry)?;
    let surroundings = Surroundings {
        resolved,
        boundaries,
    };
    let mut quads = Vec::new();
    for facing in Facing::ALL {
        emit_facing(&surroundings, facing, &mut quads)?;
    }
    Ok(SectionMesh::of(quads))
}

/// Every quad of one facing, plane by plane, lowest plane first.
fn emit_facing(
    surroundings: &Surroundings,
    facing: Facing,
    quads: &mut Vec<Quad>,
) -> Result<(), MeshError> {
    for plane in 0..SECTION_SIZE {
        Plane::of(surroundings, facing, plane)?.emit(quads)?;
    }
    Ok(())
}

/// One facing's faces in one plane, and which of them have been covered.
struct Plane<'a> {
    surroundings: &'a Surroundings,
    facing: Facing,
    plane: u32,
    /// The block each cell's face holds, or nothing where there is no face.
    faces: [Option<Key>; PLANE_CELLS],
    /// Which cells a quad already covers.
    covered: [bool; PLANE_CELLS],
}

impl<'a> Plane<'a> {
    /// Which faces of `facing` are visible in `plane`.
    fn of(surroundings: &'a Surroundings, facing: Facing, plane: u32) -> Result<Self, MeshError> {
        let mut faces = [None; PLANE_CELLS];
        for (cell, holder) in faces.iter_mut().enumerate() {
            *holder = visible_face(surroundings, facing, plane, cell)?;
        }
        Ok(Self {
            surroundings,
            facing,
            plane,
            faces,
            covered: [false; PLANE_CELLS],
        })
    }

    /// Every quad this plane shows, row by row.
    fn emit(&mut self, quads: &mut Vec<Quad>) -> Result<(), MeshError> {
        for secondary in 0..SECTION_SIZE {
            self.emit_row(secondary, quads)?;
        }
        Ok(())
    }

    /// Every quad starting in one row of this plane, lowest primary first.
    fn emit_row(&mut self, secondary: u32, quads: &mut Vec<Quad>) -> Result<(), MeshError> {
        for primary in 0..SECTION_SIZE {
            self.emit_at(PlanePos { primary, secondary }, quads)?;
        }
        Ok(())
    }

    /// The quad starting at `origin`, if a face waits there that no quad covers
    /// yet.
    fn emit_at(&mut self, origin: PlanePos, quads: &mut Vec<Quad>) -> Result<(), MeshError> {
        let Some(key) = self.uncovered_key(origin) else {
            return Ok(());
        };
        let extent = self.grown_from(origin, key);
        self.cover(origin, extent);
        quads.push(self.quad(origin, extent, key)?);
        Ok(())
    }

    /// How far a run starting at `origin` reaches, along the primary axis first
    /// and then along whole rows.
    fn grown_from(&self, origin: PlanePos, key: Key) -> PlaneExtent {
        let primary = self.run_length(origin, key);
        PlaneExtent {
            primary,
            secondary: self.rows_matching(origin, primary, key),
        }
    }

    /// How many cells a run along the primary axis from `origin` covers.
    fn run_length(&self, origin: PlanePos, key: Key) -> u32 {
        (origin.primary..SECTION_SIZE)
            .take_while(|primary| self.holds(*primary, origin.secondary, key))
            .count() as u32
    }

    /// How many whole rows, starting at `origin`'s, match that run.
    fn rows_matching(&self, origin: PlanePos, width: u32, key: Key) -> u32 {
        (origin.secondary..SECTION_SIZE)
            .take_while(|secondary| {
                self.row_matches(
                    PlanePos {
                        primary: origin.primary,
                        secondary: *secondary,
                    },
                    width,
                    key,
                )
            })
            .count() as u32
    }

    /// Whether a face of `key` that no quad covers yet waits at these
    /// coordinates.
    fn holds(&self, primary: u32, secondary: u32, key: Key) -> bool {
        self.uncovered_key(PlanePos { primary, secondary }) == Some(key)
    }

    /// Whether `width` cells from `start` all show a face of the same block that
    /// no quad covers yet.
    fn row_matches(&self, start: PlanePos, width: u32, key: Key) -> bool {
        (start.primary..start.primary.saturating_add(width))
            .all(|primary| self.holds(primary, start.secondary, key))
    }

    /// Records that a quad of `extent` at `origin` covers what it covers.
    fn cover(&mut self, origin: PlanePos, extent: PlaneExtent) {
        let last_row = origin.secondary.saturating_add(extent.secondary);
        for secondary in origin.secondary..last_row {
            self.cover_row(origin.primary, secondary, extent.primary);
        }
    }

    /// Records that one row of a quad covers what it covers.
    fn cover_row(&mut self, primary_origin: u32, secondary: u32, width: u32) {
        for primary in primary_origin..primary_origin.saturating_add(width) {
            self.cover_cell(PlanePos { primary, secondary });
        }
    }

    /// Records that one cell is covered.
    ///
    /// A cell outside this plane is left alone rather than refused: every
    /// coordinate reaching here came from a run that stopped at the plane's own
    /// bounds, so there is none, and marking is not a step a refusal could
    /// usefully interrupt.
    fn cover_cell(&mut self, at: PlanePos) {
        if let Some(covered) = self.covered.get_mut(cell_of(at)) {
            *covered = true;
        }
    }

    /// The block whose face waits at `at`, or nothing if there is no face there
    /// or a quad already covers it.
    fn uncovered_key(&self, at: PlanePos) -> Option<Key> {
        let cell = cell_of(at);
        if *self.covered.get(cell)? {
            return None;
        }
        *self.faces.get(cell)?
    }

    /// One merged rectangle, named by the block every voxel under it holds.
    fn quad(&self, origin: PlanePos, extent: PlaneExtent, key: Key) -> Result<Quad, MeshError> {
        let resolved = &self.surroundings.resolved;
        let contents = present(
            resolved.contents(key),
            key as usize,
            resolved.distinct_blocks(),
        )?;
        let block = match contents {
            Contents::Empty => return Err(MeshError::EmptyBlockFace { key: key as usize }),
            Contents::Holds(block) => block,
        };
        Ok(Quad {
            facing: self.facing,
            plane: self.plane,
            origin,
            extent,
            block: block.clone(),
        })
    }
}

/// The block whose face is visible at one cell of one plane, if any is.
///
/// A face is there when the voxel is solid and the voxel one step off this
/// facing is not. Solidity comes from the registered definition alone: no block
/// name and no runtime id is looked at anywhere in this file, which is what
/// makes a block a mod ships behave exactly as one the base game ships does.
fn visible_face(
    surroundings: &Surroundings,
    facing: Facing,
    plane: u32,
    cell: usize,
) -> Result<Option<Key>, MeshError> {
    let resolved = &surroundings.resolved;
    let voxel = facing.voxel_at(plane, position_in_plane(cell));
    let key = key_at(resolved, voxel)?;
    if !solidity(resolved, key)? || solid_beyond(surroundings, facing, voxel, cell)? {
        return Ok(None);
    }
    Ok(Some(key))
}

/// Whether the voxel one step off `facing` of `voxel` is solid.
///
/// A step that leaves the section is answered by the boundary plane resolved for
/// that facing, at the cell the face itself sits in — the two agree about where
/// they are in the plane they share, so no coordinate is converted here and
/// nothing has to know a second time which end of a neighbour is read. An absent
/// neighbour resolved to a plane holding nothing solid, so absence needs no
/// branch of its own.
fn solid_beyond(
    surroundings: &Surroundings,
    facing: Facing,
    voxel: LocalPos,
    cell: usize,
) -> Result<bool, MeshError> {
    match facing.adjacent(voxel) {
        Adjacent::Inside(beside) => {
            let resolved = &surroundings.resolved;
            solidity(resolved, key_at(resolved, beside)?)
        }
        Adjacent::Across(_) => present(
            surroundings.boundaries.is_solid(facing, cell),
            cell,
            PLANE_CELLS,
        ),
    }
}

/// Which block the voxel at `voxel` holds.
fn key_at(resolved: &Resolved, voxel: LocalPos) -> Result<Key, MeshError> {
    let index = Section::voxel_index(voxel)?;
    present(resolved.key_at(index), index, VOXELS_PER_SECTION)
}

/// Whether the block `key` names was registered solid.
fn solidity(resolved: &Resolved, key: Key) -> Result<bool, MeshError> {
    present(
        resolved.is_solid(key),
        key as usize,
        resolved.distinct_blocks(),
    )
}

/// The one place an index into the mesher's own arrays becomes a refusal.
///
/// Every one of those indices is composed from coordinates already inside
/// `0..16`, or from a key the resolution pass handed out, so none of them can be
/// missing. The arrays cannot promise that in their types and raw indexing is
/// lint-denied, so the residual `None` needs somewhere to go — and reporting it
/// as a section's own corruption would blame storage for a mesher bug.
fn present<T>(found: Option<T>, index: usize, length: usize) -> Result<T, MeshError> {
    found.ok_or(MeshError::CorruptMeshIndex { index, length })
}
