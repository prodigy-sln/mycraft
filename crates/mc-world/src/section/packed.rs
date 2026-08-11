//! The voxel indices themselves: 4096 palette positions, packed as narrowly as
//! the palette allows.
//!
//! A section holding one block spends nothing at all on indices, and one holding
//! two spends a bit per voxel. That is what makes a world of mostly stone and air
//! affordable: the cost follows the variety a section actually holds rather than
//! the variety it might one day hold.

use super::VOXELS_PER_SECTION;

/// How far a bit offset is shifted to name the byte holding it, and the mask
/// that reads back its position inside that byte.
///
/// Shifts and masks rather than division and remainder throughout this file:
/// `clippy::integer_division` is a gate error, and bit packing is the code it
/// exists for.
const BITS_PER_BYTE_SHIFT: u32 = u8::BITS.trailing_zeros();
const BITS_PER_BYTE_MASK: usize = (u8::BITS - 1) as usize;

/// How many bytes one index occupies at the widest tier.
const WIDE_INDEX_BYTES: usize = (u16::BITS >> BITS_PER_BYTE_SHIFT) as usize;

/// How many bits one voxel's index occupies.
///
/// Ordered, narrowest first, and the ordering is load-bearing: widening compares
/// tiers and a section never narrows on the write path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum IndexWidth {
    W0,
    W1,
    W2,
    W4,
    W8,
    W16,
}

impl IndexWidth {
    /// Every tier there is, narrowest first.
    const ALL: [Self; 6] = [Self::W0, Self::W1, Self::W2, Self::W4, Self::W8, Self::W16];

    /// How many bits one index occupies at this tier.
    ///
    /// Every one of these either divides 8 or is a multiple of it, so no packed
    /// index ever straddles a byte boundary — which is why nothing below handles
    /// a case that cannot occur.
    const fn bits(self) -> u32 {
        match self {
            Self::W0 => 0,
            Self::W1 => 1,
            Self::W2 => 2,
            Self::W4 => 4,
            Self::W8 => 8,
            Self::W16 => 16,
        }
    }

    /// How many distinct entries this tier can address.
    ///
    /// Derived from [`bits`](Self::bits) rather than written out beside it. A
    /// mistyped tier then produces an *inconsistent* pair — a width that does not
    /// match its own capacity — which the boundary cases catch, instead of a
    /// consistent wrong one that they cannot.
    const fn capacity(self) -> usize {
        1_usize << self.bits()
    }

    /// The narrowest tier that can address `palette_len` entries.
    ///
    /// A fold over the ordered tiers, never a hand-written range table. The
    /// specification's own audit found a table that was wrong at two tiers and
    /// still passed every scenario then written; here there is nothing to
    /// mistype. Seeding with the widest tier is what makes the fold total
    /// without an `unwrap`, which is a gate error.
    fn for_palette_len(palette_len: usize) -> Self {
        Self::ALL.iter().rev().fold(Self::W16, |narrowest, tier| {
            tier.or_wider(palette_len, narrowest)
        })
    }

    /// This tier if it addresses `palette_len`, and `wider` if it does not.
    const fn or_wider(self, palette_len: usize, wider: Self) -> Self {
        if self.capacity() >= palette_len {
            self
        } else {
            wider
        }
    }

    /// How many bytes a whole section's indices occupy at this tier.
    const fn storage_bytes(self) -> usize {
        (VOXELS_PER_SECTION * self.bits() as usize) >> BITS_PER_BYTE_SHIFT
    }
}

/// One palette position per voxel of a section.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PackedIndices {
    width: IndexWidth,
    buffer: Vec<u8>,
}

impl PackedIndices {
    /// Indices for a whole section, every one of them position 0.
    ///
    /// The narrowest tier owns no buffer at all, which is why a homogeneous
    /// section costs nothing: there is only one thing every voxel could hold, so
    /// there is nothing to write down.
    pub(super) fn new() -> Self {
        Self::of_width(IndexWidth::W0)
    }

    /// Indices for a whole section at the narrowest tier addressing
    /// `palette_len` entries, all of them position 0.
    pub(super) fn narrowest_for(palette_len: usize) -> Self {
        Self::of_width(IndexWidth::for_palette_len(palette_len))
    }

    /// Indices at `width`, all of them position 0.
    fn of_width(width: IndexWidth) -> Self {
        Self {
            width,
            buffer: vec![0; width.storage_bytes()],
        }
    }

    /// How many bits one voxel's index occupies.
    pub(super) fn width_bits(&self) -> u32 {
        self.width.bits()
    }

    /// What the indices actually cost, read off the buffer rather than worked
    /// out from the width.
    ///
    /// This is the whole point of the method. A figure recomputed from the width
    /// would agree with every size assertion however large or small the
    /// allocation behind it happened to be — including one that never grew.
    pub(super) fn storage_bytes(&self) -> usize {
        self.buffer.len()
    }

    /// The palette position stored for `voxel`, or `None` if `voxel` is not one
    /// of this section's.
    pub(super) fn get(&self, voxel: usize) -> Option<usize> {
        if voxel >= VOXELS_PER_SECTION {
            return None;
        }
        match self.width {
            IndexWidth::W0 => Some(0),
            IndexWidth::W16 => self.wide_at(voxel),
            narrow => self.narrow_at(voxel, narrow.bits()),
        }
    }

    /// Stores `position` for `voxel`, or `None` if `voxel` is not one of this
    /// section's or `position` is wider than the current tier addresses.
    pub(super) fn set(&mut self, voxel: usize, position: usize) -> Option<()> {
        if voxel >= VOXELS_PER_SECTION {
            return None;
        }
        match self.width {
            IndexWidth::W0 => (position == 0).then_some(()),
            IndexWidth::W16 => self.set_wide(voxel, position),
            narrow => self.set_narrow(voxel, position, narrow.bits()),
        }
    }

    /// Re-packs every index into the narrowest tier addressing `palette_len`, if
    /// the current tier does not reach that far.
    ///
    /// Widening only. Giving back the space a vacated entry left behind is
    /// compaction's job, and it happens when a caller asks rather than in the
    /// middle of an edit.
    ///
    /// Returns `None` if an index could not be carried across, which would mean
    /// the section's own invariant had already been broken.
    pub(super) fn widen_for(&mut self, palette_len: usize) -> Option<()> {
        let needed = IndexWidth::for_palette_len(palette_len);
        if needed <= self.width {
            return Some(());
        }
        let mut widened = Self::of_width(needed);
        for voxel in 0..VOXELS_PER_SECTION {
            widened.set(voxel, self.get(voxel)?)?;
        }
        *self = widened;
        Some(())
    }

    /// The index for `voxel` at a tier of `bits` that share their bytes.
    fn narrow_at(&self, voxel: usize, bits: u32) -> Option<usize> {
        let offset = voxel * bits as usize;
        let byte = *self.buffer.get(offset >> BITS_PER_BYTE_SHIFT)?;
        let shift = offset & BITS_PER_BYTE_MASK;
        Some((byte as usize >> shift) & Self::mask(bits))
    }

    /// The index for `voxel` at the tier that spends two whole bytes on it.
    fn wide_at(&self, voxel: usize) -> Option<usize> {
        let stored = self.buffer.get(Self::wide_span(voxel))?;
        Some(u16::from_le_bytes([*stored.first()?, *stored.last()?]) as usize)
    }

    /// Stores `position` for `voxel` at a tier of `bits` that share their bytes,
    /// leaving the neighbours sharing that byte as they were.
    fn set_narrow(&mut self, voxel: usize, position: usize, bits: u32) -> Option<()> {
        let mask = Self::mask(bits);
        if position > mask {
            return None;
        }
        let (stored, keep) = (u8::try_from(position).ok()?, u8::try_from(mask).ok()?);
        let offset = voxel * bits as usize;
        let shift = offset & BITS_PER_BYTE_MASK;
        let byte = self.buffer.get_mut(offset >> BITS_PER_BYTE_SHIFT)?;
        *byte = (*byte & !(keep << shift)) | (stored << shift);
        Some(())
    }

    /// Stores `position` for `voxel` at the tier that spends two whole bytes on
    /// it.
    fn set_wide(&mut self, voxel: usize, position: usize) -> Option<()> {
        let stored = u16::try_from(position).ok()?.to_le_bytes();
        let slot = self.buffer.get_mut(Self::wide_span(voxel))?;
        slot.copy_from_slice(&stored);
        Some(())
    }

    /// The largest index a tier of `bits` can hold.
    const fn mask(bits: u32) -> usize {
        (1_usize << bits) - 1
    }

    /// Which bytes hold `voxel`'s index at the widest tier.
    const fn wide_span(voxel: usize) -> std::ops::Range<usize> {
        let start = voxel * WIDE_INDEX_BYTES;
        start..start + WIDE_INDEX_BYTES
    }
}
