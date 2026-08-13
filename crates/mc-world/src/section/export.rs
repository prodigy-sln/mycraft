//! What a section says it holds when something outside it needs to know, and
//! what it takes to build one back from that description.
//!
//! The description carries namespaced names and one position into them per
//! voxel. Neither a runtime id nor this section's own bit packing appears in it:
//! ids are reassigned whenever the block set changes, and the packing is an
//! internal choice that whatever writes this to a disk should not inherit.
//!
//! Nothing here compacts. A caller wanting the shortest description asks for
//! compaction first, which is the whole reason it is a public operation — an
//! export that silently reorganised a section would be doing work on a caller's
//! behalf that the caller can see the cost of and this cannot.

use std::collections::BTreeSet;

use mc_core::block::BlockRegistry;
use mc_core::id::BlockName;
use thiserror::Error;

use super::palette::Palette;
use super::{Contents, PackedIndices, Section, SectionError, VOXELS_PER_SECTION};

/// Where a voxel's block sits in the palette that came with it.
///
/// A distinct type from a runtime id, and deliberately so: both are small
/// numbers standing for a block, and mistaking one for the other is exactly how
/// a section starts reporting whatever a registry happened to number the same.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PaletteIndex(u16);

impl PaletteIndex {
    /// The palette position `position`.
    #[must_use]
    pub const fn new(position: u16) -> Self {
        Self(position)
    }

    /// Which palette position this is.
    #[must_use]
    pub const fn get(self) -> u16 {
        self.0
    }
}

/// A section's contents, in a form that means the same thing under any registry.
///
/// A palette of names and a position per voxel, rather than packed bits: what
/// a section costs in memory is its own business, and a stored world should not
/// have to be rewritten the day that business changes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SectionData {
    /// What the section holds, in the order its own palette holds it.
    ///
    /// One list and not a list of names beside a separate note of which position
    /// is the empty one: emptiness *is* a position in the palette, so a second
    /// field saying which could disagree with this one, and every reader would
    /// consult two fields to answer one question.
    ///
    /// At most one entry is [`Contents::Empty`] when a section produced this. A
    /// description carrying two is accepted, exactly as one naming a block twice
    /// already is — both deduplicate downstream by what they hold.
    pub palette: Vec<Contents>,
    /// One palette position per voxel, x fastest, then y, then z.
    pub indices: Vec<PaletteIndex>,
}

/// Why a section could not be built from a description.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ImportError {
    #[error("no block is registered under the name `{name}`", name = name.as_str())]
    UnknownBlock { name: BlockName },
    #[error("palette position {index} is not a position in a palette holding {palette_len}")]
    PaletteIndexOutOfRange { index: u16, palette_len: usize },
    #[error("{found} voxels are not a section, which holds {expected}")]
    WrongVoxelCount { found: usize, expected: usize },
}

impl SectionData {
    /// The same contents with every entry no index references dropped, surviving
    /// entries keeping their relative order.
    ///
    /// Here rather than in whatever writes a description to a disk, because "the
    /// minimal form of a description" is a property of the description — putting
    /// it beside a file writer would put palette logic in the file writer.
    ///
    /// A description is compacted without a `&mut` where a section cannot be:
    /// [`Section::compact`] reads the reference counts the write path maintained
    /// and needs to own them, while an entry no *index* references is vacant by
    /// inspection. That is what lets a world hand out shared borrows and still be
    /// stored in its minimal form.
    ///
    /// Stable, matching [`Section::compact`]: compaction is a renumbering and
    /// never a reshuffle. It is also what keeps a stored position inside the
    /// width a compacted palette needs, for a section that was never compacted
    /// before it was written down.
    #[must_use]
    pub fn compacted(&self) -> Self {
        let surviving = self.referenced_entries();
        Self {
            palette: surviving
                .iter()
                .filter_map(|kept| self.palette.get(*kept))
                .cloned()
                .collect(),
            indices: self
                .indices
                .iter()
                .map(|index| Self::settled(*index, &surviving))
                .collect(),
        }
    }

    /// Which palette positions some index still names, ascending and each once.
    ///
    /// A position past the end of the palette is dropped here rather than
    /// carried: it names no entry, so there is no entry for it to keep alive.
    fn referenced_entries(&self) -> Vec<usize> {
        let referenced: BTreeSet<usize> = self
            .indices
            .iter()
            .map(|index| index.get() as usize)
            .filter(|position| *position < self.palette.len())
            .collect();
        referenced.into_iter().collect()
    }

    /// Where `index` ends up once the vacant entries are gone.
    ///
    /// Found by search rather than by a relocation table, which is what keeps
    /// this linear in a section's voxels rather than quadratic in its palette —
    /// a compacted palette runs to 4096 entries and the search runs over an
    /// ascending list.
    ///
    /// A position no surviving entry answers for is carried through unchanged.
    /// It cannot arise, since an entry survives precisely because some index
    /// names it, and rewriting it to something plausible would hide whatever
    /// produced it.
    fn settled(index: PaletteIndex, surviving: &[usize]) -> PaletteIndex {
        surviving
            .binary_search(&(index.get() as usize))
            .ok()
            .and_then(|position| u16::try_from(position).ok())
            .map_or(index, PaletteIndex::new)
    }
}

impl Section {
    /// What this section holds, named rather than numbered.
    ///
    /// # Errors
    ///
    /// Returns [`SectionError::CorruptPaletteIndex`] if a voxel does not name a
    /// position this section's own palette has.
    pub fn export(&self) -> Result<SectionData, SectionError> {
        let mut indices = Vec::with_capacity(VOXELS_PER_SECTION);
        for voxel in 0..VOXELS_PER_SECTION {
            indices.push(self.exported_index(voxel)?);
        }
        Ok(SectionData {
            palette: self.palette.iter().map(Contents::cloned).collect(),
            indices,
        })
    }

    /// The section `described` describes, read against `registry`.
    ///
    /// Every name is checked before anything is built, because a description
    /// naming a block nothing is registered under has no honest reading: the
    /// nearest alternative would be a world quietly made of something else.
    ///
    /// # Errors
    ///
    /// Returns [`ImportError::WrongVoxelCount`] unless `described` carries one
    /// position per voxel of a section, [`ImportError::UnknownBlock`] if
    /// `registry` holds no block under one of the names, and
    /// [`ImportError::PaletteIndexOutOfRange`] if a voxel names a position the
    /// palette does not have.
    pub fn import(described: &SectionData, registry: &BlockRegistry) -> Result<Self, ImportError> {
        Self::require_whole_section(&described.indices)?;
        Self::require_all_registered(&described.palette, registry)?;
        let voxels_holding = Self::references_in(described)?;
        Ok(Self {
            palette: Palette::rebuilt(described.palette.iter().cloned().zip(voxels_holding)),
            indices: Self::packed_positions(described)?,
        })
    }

    /// The palette position `voxel` holds, as the description carries it.
    ///
    /// The narrowing cannot lose anything: a position the widest tier could not
    /// address is refused when it is written, never stored and truncated here.
    fn exported_index(&self, voxel: usize) -> Result<PaletteIndex, SectionError> {
        let position = self.indices.get(voxel).ok_or_else(|| self.corrupt(voxel))?;
        let carried = u16::try_from(position)
            .ok()
            .ok_or_else(|| self.corrupt(position))?;
        Ok(PaletteIndex::new(carried))
    }

    /// Refuses anything that is not one position per voxel of a section.
    ///
    /// A description is a plain value a caller can build, so it can genuinely
    /// arrive short. Padding it out would produce a section that looks whole and
    /// holds voxels nobody described.
    fn require_whole_section(indices: &[PaletteIndex]) -> Result<(), ImportError> {
        if indices.len() == VOXELS_PER_SECTION {
            return Ok(());
        }
        Err(ImportError::WrongVoxelCount {
            found: indices.len(),
            expected: VOXELS_PER_SECTION,
        })
    }

    /// Refuses a palette naming a block `registry` does not hold.
    ///
    /// **The empty entry is skipped and it is the only thing skipped.** There is
    /// no name in it for a registry to know, so requiring one would make an
    /// empty cell need a block registered to mean nothing — while skipping the
    /// check for every entry would let a description name a block that does not
    /// exist and build a world quietly made of something else.
    fn require_all_registered(
        palette: &[Contents],
        registry: &BlockRegistry,
    ) -> Result<(), ImportError> {
        let unregistered = palette.iter().find_map(|entry| match entry {
            Contents::Empty => None,
            Contents::Holds(name) => registry.id_of(name).is_err().then_some(name),
        });
        match unregistered {
            Some(name) => Err(ImportError::UnknownBlock { name: name.clone() }),
            None => Ok(()),
        }
    }

    /// How many voxels hold each palette entry.
    ///
    /// Counted from the voxels because a description is the only thing that
    /// knows — unlike the write path, which is handed one change at a time and
    /// keeps the counts as it goes. An entry no voxel names is still kept, since
    /// removing one is compaction's decision and not an import's.
    fn references_in(described: &SectionData) -> Result<Vec<usize>, ImportError> {
        let mut voxels_holding = vec![0_usize; described.palette.len()];
        for index in &described.indices {
            let held = voxels_holding
                .get_mut(index.get() as usize)
                .ok_or_else(|| Self::out_of_range(*index, described.palette.len()))?;
            *held += 1;
        }
        Ok(voxels_holding)
    }

    /// Every described position, packed as narrowly as the palette allows.
    fn packed_positions(described: &SectionData) -> Result<PackedIndices, ImportError> {
        let mut indices = PackedIndices::narrowest_for(described.palette.len());
        for (voxel, index) in described.indices.iter().enumerate() {
            indices
                .set(voxel, index.get() as usize)
                .ok_or_else(|| Self::out_of_range(*index, described.palette.len()))?;
        }
        Ok(indices)
    }

    /// A position the palette that came with it does not have.
    fn out_of_range(index: PaletteIndex, palette_len: usize) -> ImportError {
        ImportError::PaletteIndexOutOfRange {
            index: index.get(),
            palette_len,
        }
    }
}
