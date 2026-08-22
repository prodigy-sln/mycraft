//! What each of a section's voxels holds, what the sections around it hold where
//! the two meet, and what each of those blocks declares about being drawn and
//! about hiding what is behind it — worked out once before the sweep starts.
//!
//! One pass over the 4096 voxels in the section's own linear order — x fastest,
//! then y, then z — hands every voxel a key, and the keys index a list holding
//! one entry per *distinct block the mesh is decided against*, in the order those
//! blocks were first encountered. The six shared faces are keyed into the same
//! list afterwards, so one key means one block whichever side of a boundary it
//! was reached from. Six things fall out of that shape, and none of them needs a
//! test to be true:
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
//! - **The meshed section is keyed before any neighbour**, so a section and a
//!   neighbour that both hold something unresolvable earn the section's own
//!   refusal, and every key a refusal could be reported against belongs to the
//!   content the caller asked about.
//! - **Nothing ordered by palette order, palette length, index width, reference
//!   count or runtime id reaches the output.** Keys are ordered by the contents,
//!   so identical contents produce identical keys whatever route the contents
//!   took to get there.
//! - **The same key means the same block**, including across two palette entries
//!   naming one block, and including across a section boundary. An import is not
//!   required to reject a palette that names a block twice, so comparing palette
//!   positions — the obvious fast merge predicate — would refuse to merge two
//!   faces of the same block and make the mesh depend on import history.
//!   Deduplicating by name removes that hole, and is also what lets the sweep ask
//!   whether the cell beyond a boundary holds the same block as the cell inside
//!   it.
//! - **The same block always makes the same two answers**, since both are read
//!   from the same entry.

use mc_core::block::BlockRegistry;
use mc_core::id::BlockName;

use crate::section::{Contents, Section, SectionError, VOXELS_PER_SECTION};

use super::plane;
use super::{Facing, MeshError, Neighbours};

/// How many sections surround one, and therefore how many boundary planes a
/// mesh is decided against.
const BOUNDARIES: usize = Facing::ALL.len();

/// Which of the distinct blocks a mesh is decided against a voxel holds.
///
/// A mesh is decided against the 4096 voxels of the section being meshed and the
/// 256 voxels each of its six neighbours shares with it, so it can hold at most
/// 4096 + 6 × 256 = 5632 distinct blocks — *derived* from those two shapes rather
/// than measured — and a key of this width can always name one.
pub(super) type Key = u16;

/// What one distinct thing a mesh is decided against declares.
///
/// Named rather than a triple of a `Contents` and two bare booleans: the two
/// answers are read at different ends of one face — the drawnness of the cell
/// showing it and the occlusion of whatever is beyond — so a pair that could be
/// swapped without a type error is a pair that will be.
#[derive(Debug)]
struct Declared {
    contents: Contents,
    drawn: bool,
    occludes: bool,
}

/// Every voxel's contents and what its block declares, keyed by those contents
/// rather than by storage.
#[derive(Debug)]
pub(super) struct Resolved {
    keys: [Key; VOXELS_PER_SECTION],
    blocks: Vec<Declared>,
}

impl Resolved {
    /// The key the voxel at `index` holds.
    pub(super) fn key_at(&self, index: usize) -> Option<Key> {
        self.keys.get(index).copied()
    }

    /// Whether what `key` names was registered drawn.
    pub(super) fn is_drawn(&self, key: Key) -> Option<bool> {
        self.blocks.get(key as usize).map(|held| held.drawn)
    }

    /// Whether what `key` names was registered as hiding what is behind it.
    pub(super) fn occludes(&self, key: Key) -> Option<bool> {
        self.blocks.get(key as usize).map(|held| held.occludes)
    }

    /// What `key` names — a block, or nothing.
    ///
    /// The `Option` says this mesh was decided against such a key and nothing
    /// else; whether that key is a block is the [`Contents`] inside it.
    pub(super) fn contents(&self, key: Key) -> Option<Contents<&BlockName>> {
        self.blocks
            .get(key as usize)
            .map(|held| held.contents.as_ref())
    }

    /// How many distinct things the mesh was decided against.
    pub(super) fn distinct_blocks(&self) -> usize {
        self.blocks.len()
    }
}

/// Which block faces the section being meshed across each of its six boundaries,
/// over the 256 voxels it shares with the section beyond.
///
/// A key rather than a flag, because a face is decided against two things about
/// whatever is beyond it: whether it hides what is behind it, and whether it is
/// the same block. A plane of booleans has no room for the second question, and
/// the boundary is exactly where it has to be asked — a body of water spans
/// sections, and a sea that showed a sheet at every chunk edge is what a mesher
/// that could not ask it produces.
///
/// A neighbour that was not supplied leaves its plane holding [`NOTHING_BEYOND`],
/// which is the key of `Contents::Empty` — so "absent" is a value here rather
/// than a branch in the sweep, and absence stays per neighbour without anything
/// having to keep it that way.
#[derive(Debug)]
pub(super) struct Boundaries {
    planes: [[Key; plane::CELLS]; BOUNDARIES],
}

impl Boundaries {
    /// Which block faces `cell` of the `facing` boundary.
    pub(super) fn key_at(&self, facing: Facing, cell: usize) -> Option<Key> {
        self.planes.get(facing as usize)?.get(cell).copied()
    }
}

/// The key `Contents::Empty` is seeded at, before any voxel has been read.
///
/// Fixed rather than earned, so that a plane of zeros is a plane of nothing:
/// nothing hides nothing and is not the same block as anything, which is what
/// makes an unsupplied neighbour a value the sweep reads instead of a case it
/// tests for.
const NOTHING_BEYOND: Key = 0;

/// Resolves `section` and the shared face of every supplied neighbour against
/// `registry`, into one key table.
///
/// **The meshed section is keyed first, and that ordering is load-bearing
/// twice.** It is what makes [`MeshError::UnresolvedBlock`] outrank
/// [`MeshError::UnresolvedNeighbourBlock`] when both hold something the registry
/// cannot resolve, and it is what keeps a refusal naming the lowest voxel of the
/// meshed section rather than the lowest of the two sections' union.
///
/// # Errors
///
/// Returns [`MeshError::UnresolvedBlock`] naming the lowest voxel in linear
/// order whose block `registry` does not register, or
/// [`MeshError::UnresolvedNeighbourBlock`] if a voxel of a supplied neighbour
/// that faces the meshed section holds one.
pub(super) fn resolve_surroundings(
    section: &Section,
    neighbours: &Neighbours<'_>,
    registry: &BlockRegistry,
) -> Result<(Resolved, Boundaries), MeshError> {
    let mut table = KeyTable::holding_nothing();
    let keys = section_keys(section, registry, &mut table)?;
    let planes = boundary_planes(neighbours, registry, &mut table)?;
    Ok((
        Resolved {
            keys,
            blocks: table.blocks,
        },
        Boundaries { planes },
    ))
}

/// The key every voxel of `section` holds.
fn section_keys(
    section: &Section,
    registry: &BlockRegistry,
    table: &mut KeyTable,
) -> Result<[Key; VOXELS_PER_SECTION], MeshError> {
    let mut resolver = Resolver::of(section, registry, Refusal::InTheMeshedSection, table);
    let mut keys = [NOTHING_BEYOND; VOXELS_PER_SECTION];
    for (index, slot) in keys.iter_mut().enumerate() {
        let position = section.palette_position_at_index(index)?;
        *slot = resolver.key_for(position, index)?;
    }
    Ok(keys)
}

/// The key facing each cell of each boundary, taking the neighbours in
/// declaration order.
///
/// Declaration order, so between two neighbours holding a block nothing resolves
/// it is the lower-ordered facing's that is reported — unspecified by the
/// scenarios, and fixed here so that a refusal is deterministic rather than
/// incidental.
fn boundary_planes(
    neighbours: &Neighbours<'_>,
    registry: &BlockRegistry,
    table: &mut KeyTable,
) -> Result<[[Key; plane::CELLS]; BOUNDARIES], MeshError> {
    let mut planes = [[NOTHING_BEYOND; plane::CELLS]; BOUNDARIES];
    for (facing, holder) in Facing::ALL.into_iter().zip(planes.iter_mut()) {
        if let Some(neighbour) = neighbours.at(facing) {
            *holder = shared_face(neighbour, facing, registry, table)?;
        }
    }
    Ok(planes)
}

/// Which block `neighbour` holds at each of the 256 voxels it shares with the
/// section on the other side of `facing`.
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
    table: &mut KeyTable,
) -> Result<[Key; plane::CELLS], MeshError> {
    let mut resolver = Resolver::of(neighbour, registry, Refusal::InTheNeighbour(facing), table);
    let mut keys = [NOTHING_BEYOND; plane::CELLS];
    for (cell, holder) in keys.iter_mut().enumerate() {
        let voxel = facing.across_at(plane::position_in_plane(cell));
        let index = Section::voxel_index(voxel)?;
        let position = neighbour.palette_position_at_index(index)?;
        *holder = resolver.key_for(position, index)?;
    }
    Ok(keys)
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

/// Every distinct thing one mesh is decided against, and what each of them
/// declares.
///
/// One table for the section and all six of its shared faces, which is what
/// makes a key comparison mean "the same block" across a boundary and not only
/// inside a section.
struct KeyTable {
    blocks: Vec<Declared>,
}

impl KeyTable {
    /// A table that has resolved nothing, holding only nothing itself.
    ///
    /// `Contents::Empty` is keyed before any voxel is read, so it is always
    /// [`NOTHING_BEYOND`] — which is what an unsupplied neighbour's plane of
    /// zeros means. Emptiness reaches the table by this route rather than by
    /// being reached by a voxel, and by the same one arm the registry is
    /// short-circuited on below, so there is one answer about nothing rather
    /// than two that could drift apart at a chunk boundary.
    fn holding_nothing() -> Self {
        Self {
            blocks: vec![Declared {
                contents: Contents::Empty,
                drawn: false,
                occludes: false,
            }],
        }
    }

    /// The key `contents` already has, if something earlier reached it.
    ///
    /// A linear scan over the distinct things one mesh is decided against, which
    /// is a handful in any world this produces — and bounded by that handful
    /// rather than by the voxel count, whatever the content turns out to be.
    fn key_of(&self, contents: Contents<&BlockName>) -> Option<Key> {
        let found = self
            .blocks
            .iter()
            .position(|held| held.contents.as_ref() == contents)?;
        Key::try_from(found).ok()
    }

    /// Adds `declared` and hands back the key it was given.
    fn push(&mut self, declared: Declared) -> Key {
        // One key per distinct thing a mesh is decided against: 4096 voxels and
        // six shared faces of 256, so this counts no further than 5632.
        let key = self.blocks.len() as Key;
        self.blocks.push(declared);
        key
    }
}

/// One section's palette, the registry it is being read against, the table it is
/// keying into, and what has been worked out about it so far.
struct Resolver<'a> {
    /// Every palette entry, including the ones nothing holds, so that a position
    /// here is the position a packed index names.
    entries: Vec<Contents<&'a BlockName>>,
    registry: &'a BlockRegistry,
    /// The key each palette position has been mapped to, once a voxel reached
    /// it.
    mapped: Vec<Option<Key>>,
    table: &'a mut KeyTable,
    /// Whose section this is, which is all that separates the two refusals a
    /// block nothing registers can earn.
    refusal: Refusal,
}

impl<'a> Resolver<'a> {
    /// A resolver that has looked at nothing yet.
    ///
    /// The table is borrowed rather than owned, so the section being meshed and
    /// every shared face beyond it key into one of them. A resolver per section
    /// and a table across all of them is what keeps the palette bookkeeping
    /// per-section — two sections' palette positions mean nothing to each other
    /// — while identity stays shared.
    fn of(
        section: &'a Section,
        registry: &'a BlockRegistry,
        refusal: Refusal,
        table: &'a mut KeyTable,
    ) -> Self {
        let entries: Vec<Contents<&BlockName>> = section.palette().collect();
        let mapped = vec![None; entries.len()];
        Self {
            entries,
            registry,
            mapped,
            table,
            refusal,
        }
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
    /// Contents already in the table keep the key they have, whichever entry of
    /// whichever section held them first — that is what makes the key mean the
    /// contents and not the slot, and what makes it mean the same thing either
    /// side of a boundary.
    fn first_encounter(&mut self, position: usize, index: usize) -> Result<Key, MeshError> {
        let Some(contents) = self.entries.get(position).copied() else {
            return Err(no_such_entry(position, self.entries.len()));
        };
        if let Some(already) = self.table.key_of(contents) {
            return Ok(already);
        }
        let declared = self.declared_by(contents, index)?;
        Ok(self.table.push(declared))
    }

    /// What `contents` was registered as declaring, refusing the whole mesh if
    /// the registry does not register the block at all.
    ///
    /// The registry's own refusal is not carried through: it names the block and
    /// nothing else, while a caller looking for the problem needs the voxel, and
    /// this is the only place that still knows which voxel asked.
    fn declared_by(
        &self,
        contents: Contents<&BlockName>,
        index: usize,
    ) -> Result<Declared, MeshError> {
        match contents {
            // Answered before the registry is reached, and this one arm covers
            // both the meshed section and every supplied neighbour — which is
            // what keeps "an empty cell shows no face, and hides none either"
            // one rule rather than two that could drift apart at a chunk
            // boundary.
            Contents::Empty => Ok(Declared {
                contents: Contents::Empty,
                drawn: false,
                occludes: false,
            }),
            Contents::Holds(name) => match self.registry.resolve(name) {
                Ok(definition) => Ok(Declared {
                    contents: contents.cloned(),
                    drawn: definition.drawn,
                    occludes: definition.occludes,
                }),
                Err(_) => Err(self.refusal.about(name, index)),
            },
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
