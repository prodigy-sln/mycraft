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

/// Everything a face is decided against: what the section itself holds, and
/// which block lies beyond each of its six boundaries where the two meet.
struct Surroundings {
    resolved: Resolved,
    boundaries: Boundaries,
}

/// The faces `section` shows, merged into rectangles and ordered.
///
/// A face exists at a cell when three things hold, and by these alone:
///
/// 1. the block the cell holds was registered `drawn`;
/// 2. the block beyond that face was **not** registered `occludes`;
/// 3. the two are not the same block.
///
/// The first two are what each block declares; the third is an engine rule — a
/// block never draws a face against its own kind, which is what stops a body of
/// one non-occluding block being a stack of visible sheets. None of the three is
/// derived from solidity, which means collision and nothing else.
///
/// **No block name and no runtime id is looked at anywhere in this module**, so
/// a block a mod ships is treated exactly as one the base game ships is. The
/// third rule does not weaken that: it compares identity, under a table
/// deduplicated by name, and reads neither.
///
/// Meshing reads and never writes: every parameter is a shared reference and the
/// mesh handed back is owned, so calling `compact()` on the input to simplify
/// the sweep — which would change the palette's length, its order and the width
/// the indices are packed at — is not expressible rather than merely forbidden.
///
/// A face on a boundary is decided against the voxel facing it in the neighbour
/// beyond, voxel by voxel. A neighbour that was not supplied is decided as
/// though the voxel beyond held nothing, so the edge of loaded content shows a
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
    // The section and its boundaries key into one table, and the section keys
    // first — which is what makes its own refusal outrank a neighbour's when
    // both hold something unresolvable, and what makes a key comparison mean
    // "the same block" across a boundary.
    let (resolved, boundaries) = resolve::resolve_surroundings(section, neighbours, registry)?;
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
/// Three questions, and a face is there only when all three answer yes:
///
/// 1. **Is this block drawn?** What a face is made of, and the only question
///    about the cell showing it. A block that stops a player and shows nothing
///    reaches here and emits nothing.
/// 2. **Does whatever is beyond it fail to occlude?** What a face is hidden by.
///    Separate from the first because a block may be seen without hiding what is
///    behind it, which is the whole of what makes water look like water.
/// 3. **Is whatever is beyond it a different block?** The engine rule that a
///    block never draws a face against its own kind — without it a body of water
///    is a stack of visible sheets, one per cell.
///
/// **The third is a rule the engine derives and not something content states.**
/// It names no block and it is stated here and nowhere else. `merges_with_self`
/// would be the field that let content override it, and **PRO-952 is its named
/// breaker**: the day a translucent block wants the interior faces of its own
/// volume drawn, this rule has to become a declaration. Until then two adjacent
/// cells of one non-occluding block show no seam and a mod author cannot ask for
/// one.
///
/// All three answers come from the registered definition alone, reached through
/// a key. **No block name and no runtime id is looked at anywhere in this file**,
/// which is what makes a block a mod ships behave exactly as one the base game
/// ships does — and a key comparison does not weaken that: keys are handed out
/// per distinct *contents* over a table deduplicated by name, so comparing two
/// of them compares identity and reads neither name.
fn visible_face(
    surroundings: &Surroundings,
    facing: Facing,
    plane: u32,
    cell: usize,
) -> Result<Option<Key>, MeshError> {
    let resolved = &surroundings.resolved;
    let voxel = facing.voxel_at(plane, position_in_plane(cell));
    let key = key_at(resolved, voxel)?;
    if !drawn(resolved, key)? {
        return Ok(None);
    }
    let beyond = key_beyond(surroundings, facing, voxel, cell)?;
    if occludes(resolved, beyond)? || beyond == key {
        return Ok(None);
    }
    Ok(Some(key))
}

/// Which block the voxel one step off `facing` of `voxel` holds.
///
/// A step that leaves the section is answered by the boundary plane resolved for
/// that facing, at the cell the face itself sits in — the two agree about where
/// they are in the plane they share, so no coordinate is converted here and
/// nothing has to know a second time which end of a neighbour is read. An absent
/// neighbour resolved to a plane holding the key of nothing, which hides nothing
/// and is the same kind as nothing, so absence needs no branch of its own.
fn key_beyond(
    surroundings: &Surroundings,
    facing: Facing,
    voxel: LocalPos,
    cell: usize,
) -> Result<Key, MeshError> {
    match facing.adjacent(voxel) {
        Adjacent::Inside(beside) => {
            let resolved = &surroundings.resolved;
            key_at(resolved, beside)
        }
        Adjacent::Across(_) => present(
            surroundings.boundaries.key_at(facing, cell),
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

/// Whether the block `key` names was registered drawn.
fn drawn(resolved: &Resolved, key: Key) -> Result<bool, MeshError> {
    present(
        resolved.is_drawn(key),
        key as usize,
        resolved.distinct_blocks(),
    )
}

/// Whether the block `key` names was registered as hiding what is behind it.
fn occludes(resolved: &Resolved, key: Key) -> Result<bool, MeshError> {
    present(
        resolved.occludes(key),
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
