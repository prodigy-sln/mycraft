//! A section: 16x16x16 voxels, each holding one registered block.
//!
//! A section stores names and positions into its own palette, and nothing that
//! belongs to a registry. Reading a voxel therefore takes no registry at all,
//! which is why reading one "against the wrong registry" is not a mistake this
//! type can be asked to make.

mod export;
mod packed;
mod palette;

use std::fmt;

use mc_core::block::{BlockId, BlockRegistry, RegistryError};
use mc_core::id::BlockName;
use thiserror::Error;

pub use export::{ImportError, PaletteIndex, SectionData};
use packed::PackedIndices;
use palette::Palette;

/// How many voxels a section spans along each of its axes.
pub const SECTION_SIZE: u32 = 16;

/// How many voxels a section holds.
pub const VOXELS_PER_SECTION: usize = (SECTION_SIZE * SECTION_SIZE * SECTION_SIZE) as usize;

/// How far one axis is shifted in a voxel's linear index.
///
/// A shift rather than a multiplication, and its inverse a mask rather than a
/// division, because `clippy::integer_division` is a gate error — this is
/// exactly the code it exists for.
const AXIS_SHIFT: u32 = SECTION_SIZE.trailing_zeros();

/// The mask that reads one axis back out of a linear voxel index.
const AXIS_MASK: u32 = SECTION_SIZE - 1;

/// The shift above only addresses a section correctly while the section's size
/// is the power of two it was derived from.
const _: () = assert!(1 << AXIS_SHIFT == SECTION_SIZE);

/// A voxel's position inside its own section.
///
/// A plain value with no validation of its own. The accessor validates instead,
/// so reaching a voxel is a fallible operation rather than an index into a
/// slice — a section that panicked on a bad coordinate would end the tick for
/// everyone connected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocalPos {
    pub x: u32,
    pub y: u32,
    pub z: u32,
}

/// Which of a position's three coordinates a refusal is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Axis {
    X,
    Y,
    Z,
}

impl fmt::Display for Axis {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let axis = match self {
            Self::X => "x",
            Self::Y => "y",
            Self::Z => "z",
        };
        formatter.write_str(axis)
    }
}

/// Why a section refused.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum SectionError {
    #[error("{axis} = {value} is outside a section, whose {axis} positions are 0..{limit}")]
    OutOfBounds { axis: Axis, value: u32, limit: u32 },
    #[error("no block is registered under the name `{name}`", name = name.as_str())]
    UnknownBlock { name: BlockName },
    #[error(transparent)]
    Registry(#[from] RegistryError),
    /// An internal invariant, not anything a caller did: every packed index is
    /// supposed to be a position in this section's own palette.
    #[error("packed index {index} is not a position in a palette holding {palette_len}")]
    CorruptPaletteIndex { index: u16, palette_len: usize },
}

/// 4096 voxels, each holding one of the blocks in this section's palette.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Section {
    palette: Palette,
    indices: PackedIndices,
}

impl Section {
    /// A section every one of whose voxels holds `fill`.
    ///
    /// # Errors
    ///
    /// Returns [`SectionError::UnknownBlock`] if `registry` holds no block under
    /// that name.
    pub fn filled(fill: &BlockName, registry: &BlockRegistry) -> Result<Self, SectionError> {
        Self::require_registered(fill, registry)?;
        // The fill takes palette position 0 and every voxel index starts there,
        // so filling is the state a section is already in.
        Ok(Self {
            palette: Palette::filled_with(fill, VOXELS_PER_SECTION),
            indices: PackedIndices::new(),
        })
    }

    /// The block held at `pos`.
    ///
    /// Takes no registry: the palette holds names, so there is no id here that
    /// some other registry could read differently.
    ///
    /// # Errors
    ///
    /// Returns [`SectionError::OutOfBounds`] if `pos` is not a position in a
    /// section.
    pub fn block_at(&self, pos: LocalPos) -> Result<&BlockName, SectionError> {
        let voxel = Self::voxel_index(pos)?;
        let position = self.indices.get(voxel).ok_or_else(|| self.corrupt(voxel))?;
        self.palette
            .name_at(position)
            .ok_or_else(|| self.corrupt(position))
    }

    /// Whether the block held at `pos` is solid, according to `registry`.
    ///
    /// Solidity is a property a block was registered with, and this reads it
    /// back. Nothing here compares a name or a runtime id: an engine that knew
    /// which block was air would be a game rule written in Rust, and the block
    /// a mod ships would not get the same treatment as the one the base game
    /// does.
    ///
    /// A foreign registry answers what *it* says about the name, which is a
    /// well-defined question rather than a corruption.
    ///
    /// # Errors
    ///
    /// Returns [`SectionError::OutOfBounds`] if `pos` is not a position in a
    /// section, and [`SectionError::Registry`] if the block held there is not
    /// registered in `registry`.
    pub fn is_solid_at(
        &self,
        pos: LocalPos,
        registry: &BlockRegistry,
    ) -> Result<bool, SectionError> {
        Ok(registry.resolve(self.block_at(pos)?)?.is_solid)
    }

    /// Writes `block` at `pos`.
    ///
    /// # Errors
    ///
    /// Returns [`SectionError::OutOfBounds`] if `pos` is not a position in a
    /// section, and [`SectionError::UnknownBlock`] if `registry` holds no block
    /// under that name.
    pub fn set_block(
        &mut self,
        pos: LocalPos,
        block: &BlockName,
        registry: &BlockRegistry,
    ) -> Result<(), SectionError> {
        let voxel = Self::voxel_index(pos)?;
        Self::require_registered(block, registry)?;
        let vacated = self.indices.get(voxel).ok_or_else(|| self.corrupt(voxel))?;
        let position = self.palette.replace(vacated, block);
        self.store(voxel, position)
    }

    /// Gives back the palette entries no voxel holds any more, and the index
    /// width they were keeping wide.
    ///
    /// Off the edit path deliberately: an entry is vacated by an ordinary block
    /// placement, and reclaiming it there would put a renumbering of every voxel
    /// into a tick shared by everyone connected, for a saving only meshing and
    /// persistence ever collect. So a caller asks, and asks when it suits them.
    ///
    /// Which entries survive is read off the counts the write path maintained.
    /// Recounting them from the voxels here would make compaction come out right
    /// even when those counts had been wrong all along — hiding the one defect
    /// this operation is placed to expose.
    ///
    /// A section whose own indices no longer name its own palette is left
    /// exactly as it was. Nothing can produce that state, and quietly rebuilding
    /// a section around it would destroy the evidence of whatever did.
    pub fn compact(&mut self) {
        let surviving = self.palette.surviving_entries();
        if surviving.len() == self.palette.len() {
            return;
        }
        let Some(narrowed) = self.remap(&surviving) else {
            return;
        };
        self.indices = narrowed;
        self.palette.narrow_to(&surviving);
    }

    /// Writes the block `registry` assigned `id` to at `pos`.
    ///
    /// The one operation on a section that translates a runtime id, and it does
    /// so at the edit rather than storing the id.
    ///
    /// # Errors
    ///
    /// Returns [`SectionError::Registry`] if `registry` never assigned `id`,
    /// and [`SectionError::OutOfBounds`] if `pos` is not a position in a
    /// section.
    pub fn set_block_by_id(
        &mut self,
        pos: LocalPos,
        id: BlockId,
        registry: &BlockRegistry,
    ) -> Result<(), SectionError> {
        let name = registry.definition(id)?.name.clone();
        self.set_block(pos, &name, registry)
    }

    /// Every palette entry, in insertion order.
    pub fn palette(&self) -> impl ExactSizeIterator<Item = &BlockName> {
        self.palette.iter()
    }

    /// How many bits one voxel's index occupies.
    pub fn index_width_bits(&self) -> u32 {
        self.indices.width_bits()
    }

    /// How many bytes this section's voxel indices actually occupy.
    pub fn index_storage_bytes(&self) -> usize {
        self.indices.storage_bytes()
    }

    /// Where in the buffer `pos` lives, counting x fastest, then y, then z.
    ///
    /// Every coordinate is checked before any of them is folded into the linear
    /// index, because folding first is what makes an out-of-range coordinate
    /// land on some other voxel instead: (0, 16, 0) and (0, 0, 1) fold to the
    /// same number, and answering the second when the first was asked would be
    /// a silent lie rather than a refusal.
    pub(crate) fn voxel_index(pos: LocalPos) -> Result<usize, SectionError> {
        Self::within_section(Axis::X, pos.x)?;
        Self::within_section(Axis::Y, pos.y)?;
        Self::within_section(Axis::Z, pos.z)?;
        Ok((pos.x | (pos.y << AXIS_SHIFT) | (pos.z << (AXIS_SHIFT * 2))) as usize)
    }

    /// Which voxel `index` names — the inverse of
    /// [`voxel_index`](Self::voxel_index).
    ///
    /// Every accessor folds a position into an index and nothing ever unfolded
    /// one until the mesher had to name the voxel it stopped at. The two halves
    /// are the same layout written twice and a layout written twice can be half
    /// right, which is why they are checked against each other over every
    /// position a section has rather than over a handful.
    ///
    /// Takes no bound of its own: an index past the last voxel folds back into
    /// one, exactly as the coordinates it is built from would.
    pub(crate) const fn position_of_voxel(index: usize) -> LocalPos {
        let linear = index as u32;
        LocalPos {
            x: linear & AXIS_MASK,
            y: (linear >> AXIS_SHIFT) & AXIS_MASK,
            z: (linear >> (AXIS_SHIFT * 2)) & AXIS_MASK,
        }
    }

    /// The palette position the voxel at `index` holds.
    ///
    /// The per-voxel read the mesher's resolution pass is built on, and
    /// deliberately not public: the mesher lives in this crate, so nothing
    /// outside it needs a per-voxel palette position, and a public one would be
    /// a promise about the packing that the packing does not want to make.
    ///
    /// # Errors
    ///
    /// Returns [`SectionError::CorruptPaletteIndex`] if `index` is not a voxel
    /// of a section.
    pub(crate) fn palette_position_at_index(&self, index: usize) -> Result<usize, SectionError> {
        self.indices.get(index).ok_or_else(|| self.corrupt(index))
    }

    /// Every voxel's index rewritten to the position its block occupies once the
    /// entries nothing holds are gone, packed as narrowly as what is left allows.
    ///
    /// Returns `None` if a voxel names a palette position that is not there,
    /// which would mean this section's own invariant was already broken.
    fn remap(&self, surviving: &[usize]) -> Option<PackedIndices> {
        let moved = Self::relocations(surviving, self.palette.len());
        let mut narrowed = PackedIndices::narrowest_for(surviving.len());
        for voxel in 0..VOXELS_PER_SECTION {
            let held = self.indices.get(voxel)?;
            let settled = (*moved.get(held)?)?;
            narrowed.set(voxel, settled)?;
        }
        Some(narrowed)
    }

    /// Where each palette position ends up, and `None` for the ones that go.
    ///
    /// Built once per compaction rather than consulted per voxel, and surviving
    /// entries keep their relative order — so compaction is a renumbering and
    /// never a reshuffle. The survivor of a section that narrows to a single
    /// entry therefore lands at position 0, which is the only position a section
    /// with no index buffer at all can answer with.
    fn relocations(surviving: &[usize], palette_len: usize) -> Vec<Option<usize>> {
        (0..palette_len)
            .map(|position| surviving.iter().position(|kept| *kept == position))
            .collect()
    }

    /// Widens the indices if the palette has outgrown the current tier, then
    /// records `position` at `voxel`.
    ///
    /// Widening before storing rather than after is the whole of it: a section
    /// that stored first would have to truncate the position it could not
    /// address, and a truncated palette position is a different block.
    fn store(&mut self, voxel: usize, position: usize) -> Result<(), SectionError> {
        let widened = self.indices.widen_for(self.palette.len());
        widened.ok_or_else(|| self.corrupt(position))?;
        self.indices
            .set(voxel, position)
            .ok_or_else(|| self.corrupt(position))
    }

    /// Refuses a name no registry entry matches.
    ///
    /// A registry reaches the name-taking mutators as a validator and nothing
    /// else — it answers whether a name is registered and never translates one,
    /// so handing a section the wrong registry can only refuse a write that
    /// should have been allowed. It can never store a different block. What it
    /// buys is that "a section holds registered blocks" is true by construction,
    /// and that an unregistered name is discovered at the edit, where a caller
    /// can still do something about it, rather than three ticks later in the
    /// mesher.
    fn require_registered(name: &BlockName, registry: &BlockRegistry) -> Result<(), SectionError> {
        if registry.id_of(name).is_ok() {
            return Ok(());
        }
        Err(SectionError::UnknownBlock { name: name.clone() })
    }

    /// Refuses a coordinate a section has no such position for, naming the axis
    /// it was on, what it was, and where the axis stops.
    fn within_section(axis: Axis, value: u32) -> Result<(), SectionError> {
        if value < SECTION_SIZE {
            return Ok(());
        }
        Err(SectionError::OutOfBounds {
            axis,
            value,
            limit: SECTION_SIZE,
        })
    }

    /// The one place this section's internal invariant — every packed index is
    /// a position in its own palette — becomes an error.
    ///
    /// Neither the packed indices nor the palette can promise it in their types,
    /// so both answer with `Option`; collapsing every one of those here keeps
    /// the invariant stated once and the refusal constructed once.
    ///
    /// The reported index saturates rather than failing again, because the
    /// number is a diagnostic on a path nothing reaches: the accessor has
    /// already refused every coordinate a section has no position for, and the
    /// indices are widened before anything is written into them. What matters
    /// is that the write is refused; what it prints is a courtesy.
    fn corrupt(&self, index: usize) -> SectionError {
        SectionError::CorruptPaletteIndex {
            index: u16::try_from(index).unwrap_or(u16::MAX),
            palette_len: self.palette.len(),
        }
    }
}

#[cfg(test)]
mod tests {
    //! Guard. A position and its linear index name the same voxel, both ways round.
    //!
    //! Folding three coordinates into one number and reading them back out are two
    //! halves of the same layout written twice, and a layout written twice can be
    //! half right. A fold that swapped two axes, or an unfold that shifted by the
    //! wrong amount, is invisible to anything that only ever goes one way — the
    //! section accessors all fold, so every one of them would agree with a wrong
    //! fold, and only the mesher, which has to name the voxel it stopped at, ever
    //! unfolds.
    //!
    //! So the two are checked against each other over every position a section has,
    //! rather than over a handful. Half of a wrong pair is a mistake at one axis or
    //! at one bit, and a spot check lands on it only by luck.

    use super::{LocalPos, SECTION_SIZE, Section, SectionError};

    /// Every position a section has, x fastest, then y, then z.
    fn every_position() -> impl Iterator<Item = LocalPos> {
        (0..SECTION_SIZE).flat_map(|z| {
            (0..SECTION_SIZE)
                .flat_map(move |y| (0..SECTION_SIZE).map(move |x| LocalPos { x, y, z }))
        })
    }

    #[test]
    fn every_position_comes_back_out_of_its_own_linear_index_unchanged() -> Result<(), SectionError>
    {
        let mut round_tripped = Vec::new();
        for asked in every_position() {
            round_tripped.push((
                asked,
                Section::position_of_voxel(Section::voxel_index(asked)?),
            ));
        }

        let disagreed: Vec<(LocalPos, LocalPos)> = round_tripped
            .into_iter()
            .filter(|(asked, answered)| asked != answered)
            .collect();

        assert!(
            disagreed.is_empty(),
            "a position folded into a linear index and unfolded again is the position that went \
             in. These were not, which means the two directions disagree about which bits of an \
             index belong to which axis — and the fold is what every accessor uses while the \
             unfold is what names the voxel in a refusal, so a caller would be pointed at some \
             other voxel entirely: {disagreed:?}"
        );
        Ok(())
    }
}
