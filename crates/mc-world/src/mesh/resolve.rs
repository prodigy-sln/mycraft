//! What each of a section's voxels holds, and whether that block is solid,
//! worked out once before the sweep starts.
//!
//! One pass over the 4096 voxels in the section's own linear order — x fastest,
//! then y, then z — hands every voxel a key, and the keys index a list holding
//! one entry per *distinct block the section actually holds*, in the order those
//! blocks were first encountered. Five things fall out of that shape, and none
//! of them needs a test to be true:
//!
//! - **Only entries a voxel actually lands on are resolved.** An entry is
//!   resolved when a voxel reaches it and never otherwise, so an entry nothing
//!   holds any more — naming a block that has since been de-registered — is
//!   never touched. That happens without consulting a reference count at all,
//!   so it survives a refcounting bug rather than depending on one.
//! - **The voxel a refusal names is the lowest in linear order**, because the
//!   pass *is* linear order and stops at the first failure. Reporting the first
//!   failing palette entry — the natural implementation — is not expressible
//!   here.
//! - **Nothing ordered by palette order, palette length, index width, reference
//!   count or runtime id reaches the output.** Keys are ordered by the contents,
//!   so identical contents produce identical keys whatever route the contents
//!   took to get there.
//! - **The same key means the same block**, including across two palette entries
//!   naming one block. An import is not required to reject a palette that names
//!   a block twice, so comparing palette positions — the obvious fast merge
//!   predicate — would refuse to merge two faces of the same block and make the
//!   mesh depend on import history. Deduplicating by name removes that hole.
//! - **The same block always has the same solidity**, since both are read from
//!   the same entry.

use mc_core::block::BlockRegistry;
use mc_core::id::BlockName;

use crate::section::{Section, SectionError, VOXELS_PER_SECTION};

use super::plane;
use super::{Facing, MeshError, Neighbours};

/// How many sections surround one, and therefore how many boundary planes a
/// mesh is decided against.
const BOUNDARIES: usize = Facing::ALL.len();

/// Which of the distinct blocks a section holds a voxel holds.
///
/// A section has 4096 voxels, so it can hold at most 4096 distinct blocks and a
/// key of this width can always name one.
pub(super) type Key = u16;

/// Every voxel's block and solidity, keyed by contents rather than by storage.
#[derive(Debug)]
pub(super) struct Resolved {
    keys: [Key; VOXELS_PER_SECTION],
    blocks: Vec<(BlockName, bool)>,
}

impl Resolved {
    /// The key the voxel at `index` holds.
    pub(super) fn key_at(&self, index: usize) -> Option<Key> {
        self.keys.get(index).copied()
    }

    /// Whether the block `key` names was registered solid.
    pub(super) fn is_solid(&self, key: Key) -> Option<bool> {
        self.blocks.get(key as usize).map(|(_, solid)| *solid)
    }

    /// What the block `key` names is called.
    pub(super) fn name(&self, key: Key) -> Option<&BlockName> {
        self.blocks.get(key as usize).map(|(name, _)| name)
    }

    /// How many distinct blocks the section holds.
    pub(super) fn distinct_blocks(&self) -> usize {
        self.blocks.len()
    }
}

/// How solid the section beyond each of the six boundaries is, over the 256
/// voxels it shares with the section being meshed.
///
/// A neighbour that was not supplied leaves its plane holding nothing solid,
/// which is what an absent neighbour means — so "absent" is a value here rather
/// than a branch in the sweep, and absence stays per neighbour without anything
/// having to keep it that way.
#[derive(Debug)]
pub(super) struct Boundaries {
    planes: [[bool; plane::CELLS]; BOUNDARIES],
}

impl Boundaries {
    /// Whether the voxel facing `cell` of the `facing` boundary is solid.
    pub(super) fn is_solid(&self, facing: Facing, cell: usize) -> Option<bool> {
        self.planes.get(facing as usize)?.get(cell).copied()
    }
}

/// Resolves every voxel of `section` against `registry`.
///
/// # Errors
///
/// Returns [`MeshError::UnresolvedBlock`] naming the lowest voxel in linear
/// order whose block `registry` does not register.
pub(super) fn resolve_section(
    section: &Section,
    registry: &BlockRegistry,
) -> Result<Resolved, MeshError> {
    let mut resolver = Resolver::of(section, registry, Refusal::InTheMeshedSection);
    let mut keys = [0; VOXELS_PER_SECTION];
    for (index, slot) in keys.iter_mut().enumerate() {
        let position = section.palette_position_at_index(index)?;
        *slot = resolver.key_for(position, index)?;
    }
    Ok(Resolved {
        keys,
        blocks: resolver.blocks,
    })
}

/// Resolves the shared face of every neighbour that was supplied.
///
/// The neighbours are taken in declaration order, so between two neighbours
/// holding a block nothing resolves it is the lower-ordered facing's that is
/// reported — unspecified by the scenarios, and fixed here so that a refusal is
/// deterministic rather than incidental.
///
/// # Errors
///
/// Returns [`MeshError::UnresolvedNeighbourBlock`] if a voxel of a supplied
/// neighbour that faces the meshed section holds a block `registry` does not
/// register.
pub(super) fn resolve_boundaries(
    neighbours: &Neighbours<'_>,
    registry: &BlockRegistry,
) -> Result<Boundaries, MeshError> {
    let mut planes = [[false; plane::CELLS]; BOUNDARIES];
    for (facing, holder) in Facing::ALL.into_iter().zip(planes.iter_mut()) {
        if let Some(neighbour) = neighbours.at(facing) {
            *holder = shared_face(neighbour, facing, registry)?;
        }
    }
    Ok(Boundaries { planes })
}

/// How solid `neighbour` is over the 256 voxels it shares with the section on
/// the other side of `facing`.
///
/// **Only those 256 are read.** The neighbour's other 3840 voxels never reach
/// the registry, so a block it holds away from the shared face is never resolved
/// and never refuses a mesh it could not have been seen in. That narrowing is
/// what makes refusing an unresolvable block on the shared face and accepting
/// one behind it two consistent rules rather than a contradiction.
///
/// The cells are walked in ascending order, which is ascending linear index
/// inside the neighbour as well: a plane's primary axis is always the
/// lower-numbered of its two, so a cell's `primary | secondary << 4` orders the
/// same way the neighbour's own `x | y << 4 | z << 8` does. The voxel a refusal
/// names is therefore the lowest of the shared face in the neighbour's own
/// order.
fn shared_face(
    neighbour: &Section,
    facing: Facing,
    registry: &BlockRegistry,
) -> Result<[bool; plane::CELLS], MeshError> {
    let mut resolver = Resolver::of(neighbour, registry, Refusal::InTheNeighbour(facing));
    let mut solid = [false; plane::CELLS];
    for (cell, holder) in solid.iter_mut().enumerate() {
        let voxel = facing.across_at(plane::position_in_plane(cell));
        let index = Section::voxel_index(voxel)?;
        let position = neighbour.palette_position_at_index(index)?;
        *holder = resolver.solidity_at(position, index)?;
    }
    Ok(solid)
}

/// Whose voxel a refusal is about: the section being meshed, or the neighbour
/// beyond one of its faces.
///
/// One condition found in two places rather than two conditions. Carrying it as
/// a value is what lets both be resolved by the same pass — a second resolution
/// path written for neighbours could drift from this one in exactly the way that
/// would only show at a chunk boundary.
#[derive(Debug, Clone, Copy)]
enum Refusal {
    InTheMeshedSection,
    InTheNeighbour(Facing),
}

impl Refusal {
    /// The refusal that `name`, held at `index`, earns.
    ///
    /// The position is in the frame of whichever section holds it, which for a
    /// neighbour is the neighbour's own — not the meshed section's, and not the
    /// mirrored coordinate. It is the one a person going to look for the block
    /// would use.
    fn about(self, name: &BlockName, index: usize) -> MeshError {
        let position = Section::position_of_voxel(index);
        match self {
            Self::InTheMeshedSection => MeshError::UnresolvedBlock {
                name: name.clone(),
                position,
            },
            Self::InTheNeighbour(facing) => MeshError::UnresolvedNeighbourBlock {
                name: name.clone(),
                facing,
                position,
            },
        }
    }
}

/// One section's palette, the registry it is being read against, and what has
/// been worked out about it so far.
struct Resolver<'a> {
    /// Every palette entry, including the ones nothing holds, so that a position
    /// here is the position a packed index names.
    entries: Vec<&'a BlockName>,
    registry: &'a BlockRegistry,
    /// The key each palette position has been mapped to, once a voxel reached
    /// it.
    mapped: Vec<Option<Key>>,
    blocks: Vec<(BlockName, bool)>,
    /// Whose section this is, which is all that separates the two refusals a
    /// block nothing registers can earn.
    refusal: Refusal,
}

impl<'a> Resolver<'a> {
    /// A resolver that has looked at nothing yet.
    fn of(section: &'a Section, registry: &'a BlockRegistry, refusal: Refusal) -> Self {
        let entries: Vec<&BlockName> = section.palette().collect();
        let mapped = vec![None; entries.len()];
        Self {
            entries,
            registry,
            mapped,
            blocks: Vec::new(),
            refusal,
        }
    }

    /// Whether the block the entry at `position` names was registered solid,
    /// resolving it if the voxel at `index` is the first to reach it.
    ///
    /// What a boundary needs, and all it needs: nothing across a boundary is
    /// ever merged with anything, so a neighbour's block names never leave this
    /// call.
    fn solidity_at(&mut self, position: usize, index: usize) -> Result<bool, MeshError> {
        let key = self.key_for(position, index)?;
        self.blocks
            .get(key as usize)
            .map(|(_, solid)| *solid)
            .ok_or(MeshError::CorruptMeshIndex {
                index: key as usize,
                length: self.blocks.len(),
            })
    }

    /// The key for the palette entry at `position`, resolving it if the voxel at
    /// `index` is the first to reach it.
    fn key_for(&mut self, position: usize, index: usize) -> Result<Key, MeshError> {
        if let Some(known) = self.mapped.get(position).copied().flatten() {
            return Ok(known);
        }
        let key = self.first_encounter(position, index)?;
        if let Some(slot) = self.mapped.get_mut(position) {
            *slot = Some(key);
        }
        Ok(key)
    }

    /// The key for a palette entry no voxel had reached until now.
    ///
    /// A block already in the list keeps the key it has, whichever entry named
    /// it first — that is what makes the key mean the block and not the slot.
    fn first_encounter(&mut self, position: usize, index: usize) -> Result<Key, MeshError> {
        let Some(name) = self.entries.get(position).copied() else {
            return Err(no_such_entry(position, self.entries.len()));
        };
        if let Some(already) = self.key_of(name) {
            return Ok(already);
        }
        let solid = self.solidity_of(name, index)?;
        // One key per distinct block a section holds, and a section holds 4096
        // voxels, so this counts no further than 4096.
        let key = self.blocks.len() as Key;
        self.blocks.push((name.clone(), solid));
        Ok(key)
    }

    /// The key `name` already has, if some earlier voxel reached it.
    ///
    /// A linear scan over the distinct blocks one section holds, which is a
    /// handful in any world this produces — and bounded by that handful rather
    /// than by the voxel count, whatever a section turns out to contain.
    fn key_of(&self, name: &BlockName) -> Option<Key> {
        let found = self.blocks.iter().position(|(held, _)| held == name)?;
        Key::try_from(found).ok()
    }

    /// Whether `name` was registered solid, refusing the whole mesh if the
    /// registry does not register it at all.
    ///
    /// The registry's own refusal is not carried through: it names the block and
    /// nothing else, while a caller looking for the problem needs the voxel, and
    /// this is the only place that still knows which voxel asked.
    fn solidity_of(&self, name: &BlockName, index: usize) -> Result<bool, MeshError> {
        match self.registry.resolve(name) {
            Ok(definition) => Ok(definition.is_solid),
            Err(_) => Err(self.refusal.about(name, index)),
        }
    }
}

/// The refusal a packed index naming a palette entry that is not there earns.
///
/// The section's own, rather than one of the mesher's: this is a copy of that
/// palette, and a copy cannot know something the original does not. Nothing
/// reaches it — a section keeps its indices inside its own palette — and it
/// exists because the copy cannot say so in its type.
fn no_such_entry(position: usize, palette_len: usize) -> MeshError {
    MeshError::Section(SectionError::CorruptPaletteIndex {
        index: u16::try_from(position).unwrap_or(u16::MAX),
        palette_len,
    })
}
