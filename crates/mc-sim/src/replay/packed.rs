//! One fixed-width value per voxel, packed into words.
//!
//! **The word arithmetic is stated here and nowhere else.** A resolved view
//! carries four answers per voxel — three of them one bit wide and the fourth an
//! index into a table of media — and a second copy of "which word holds this
//! offset, and which bits of it" would be agreement between two copies of one
//! decision rather than one decision read twice.
//!
//! **A width is a power of two that divides 64**, which is what keeps a value
//! from straddling a word: a read stays one shift and one mask however wide the
//! value is. A width that did not divide 64 would make the read two words at
//! some offsets and one at others, and which offsets those were would depend on
//! how many distinct media a registry happened to declare.
//!
//! That is also what keeps every split here a shift and a mask rather than a
//! division and a remainder — the same arithmetic for a power of two, and the
//! form `replay::world` already reads a world coordinate as a column and a
//! position inside it with.
//!
//! **A read past the end answers zero rather than panicking.** That is what
//! keeps the views above this total: zero is "not solid", "not targetable" and
//! "no medium here" alike, so a position no array covers answers what a position
//! outside the world answers, and the bounds test that makes it unreachable is
//! the extent's, in the one caller.

/// How many bits one word holds.
const BITS_PER_WORD: u32 = u64::BITS;

/// The widths a packed array may use, narrowest first.
///
/// The floor is one bit rather than zero: a registry whose blocks all answer the
/// same medium needs one value, and `ceil(log2(1))` is zero — a zero-width array
/// has no bit to read and no offset to read it at.
const WIDTHS: [u32; 6] = [1, 2, 4, 8, 16, 32];

/// One value per voxel, in the order `Extent::offset` numbers them, at a width
/// chosen once when the array is built.
///
/// A packed array rather than a `Vec<u32>` because the replay's footprint is a
/// million voxels: at one bit that is 128 KiB held for the run, against the four
/// megabytes a word apiece would cost for the same answer.
///
/// It carries no idea of *which* question it answers, which is what lets one
/// type serve all three without any view's arithmetic being written twice.
#[derive(Debug)]
pub(crate) struct PackedArray {
    words: Vec<u64>,
    width: u32,
}

impl PackedArray {
    /// The narrowest width that can tell `distinct` values apart, in bits.
    ///
    /// The widest is 32, which is more distinct answers than a registry could
    /// hold definitions to declare — so the fallback is unreachable rather than
    /// a silent truncation waiting for a large enough registry.
    pub(crate) fn width_for(distinct: usize) -> u32 {
        WIDTHS
            .into_iter()
            .find(|width| distinct <= 1usize << width)
            .unwrap_or(u32::BITS)
    }

    /// Packs `values` at `width`, in the order they arrive.
    ///
    /// Built whole from the values rather than filled in by offset, so there is
    /// no position a caller could write that the array does not cover — the
    /// length is the values' length by construction.
    ///
    /// A value too wide for `width` is masked rather than refused, and nothing
    /// can hand one over: every index comes from the table whose size chose the
    /// width.
    pub(crate) fn packing(values: impl IntoIterator<Item = u32>, width: u32) -> Self {
        let mut words: Vec<u64> = Vec::new();
        for (offset, value) in values.into_iter().enumerate() {
            let slot = offset & slot_mask(width);
            let carrying = carried(value, width, slot);
            match words.last_mut() {
                // Slot zero opens a word; every other slot lands in the open one.
                Some(word) if slot != 0 => *word |= carrying,
                _ => words.push(carrying),
            }
        }
        Self { words, width }
    }

    /// How many bits this array spends on each value.
    pub(crate) const fn width(&self) -> u32 {
        self.width
    }

    /// The value at `offset`, or zero past the end.
    pub(crate) fn get(&self, offset: usize) -> u32 {
        self.words
            .get(offset >> per_word_shift(self.width))
            .map_or(0, |word| {
                read(*word, self.width, offset & slot_mask(self.width))
            })
    }

    /// Writes the value at `offset`.
    ///
    /// An offset past the end writes nothing, for the same reason one past the
    /// end reads as zero: the length is the values' length by construction, so
    /// there is no position a caller can reach that the array was built without.
    pub(crate) fn set(&mut self, offset: usize, value: u32) {
        let width = self.width;
        let slot = offset & slot_mask(width);
        if let Some(word) = self.words.get_mut(offset >> per_word_shift(width)) {
            *word = (*word & !carried(mask(width), width, slot)) | carried(value, width, slot);
        }
    }
}

/// The shift that turns an offset into the index of the word holding it.
///
/// How many values of `width` bits one word holds, as a shift: 64 over the width
/// is a power of two because the width is.
const fn per_word_shift(width: u32) -> u32 {
    BITS_PER_WORD.trailing_zeros() - width.trailing_zeros()
}

/// The mask that turns an offset into its slot inside a word.
const fn slot_mask(width: u32) -> usize {
    (1usize << per_word_shift(width)) - 1
}

/// The low `width` bits, set.
const fn mask(width: u32) -> u32 {
    if width >= u32::BITS {
        // Shifting a `u32` by 32 overflows, and 32 bits wide is all of one.
        u32::MAX
    } else {
        (1 << width) - 1
    }
}

/// `value`, narrowed to `width` bits and moved to slot `slot` of a word.
const fn carried(value: u32, width: u32, slot: usize) -> u64 {
    ((value & mask(width)) as u64) << (slot as u32 * width)
}

/// The value slot `slot` of `word` holds, at `width` bits.
const fn read(word: u64, width: u32, slot: usize) -> u32 {
    ((word >> (slot as u32 * width)) as u32) & mask(width)
}
