//! The distinct media a registry declares, and the index a voxel carries into
//! them.
//!
//! **Indexing the whole answer is the whole of the saving.** What a voxel is, as
//! far as a medium is concerned, takes very few values — the shipped content
//! declares four blocks answering at most two media between them — so a table of
//! the distinct answers and one narrow index per voxel costs a fraction of what
//! any of the properties stored densely would. A `swimmable` bitset beside a
//! resistance view costs twice as much before it answers anything general, and a
//! dense `f32` costs thirty times as much — twice over, now that a medium
//! carries two of them.
//!
//! **The table is built from the registry and never from a world's contents.**
//! A block the world does not hold yet must already have an index, because a
//! later write may place it — so a table sized from what a volume happens to
//! contain would have to grow, and widening the packing under an edit is the one
//! thing the write path must not do. Every writable answer is present before any
//! write, so the table never grows.
//!
//! Nothing here reads a block *name*. Every answer a medium carries comes from
//! the field that declares it, which is invariant 1 in the one place no
//! declaration could override — the single exception being the ascent of a
//! volume that holds nobody up, which [`declared_by`] masks away for the reason
//! recorded there.

use mc_core::block::{BlockDefinition, BlockRegistry};

use crate::player::VoxelMedium;

use super::packed::PackedArray;

/// An index into one medium table.
///
/// Opaque and `Copy`, **with no public constructor**: the table mints every legal
/// value, which is what keeps `ResolvedVoxels::set` total for a caller that
/// resolved against some other registry. A token that cannot name a table it did
/// not come from is unspellable rather than checked — the same standard a re-mesh
/// batch is already held to, where a batch cannot be meshed against a registry
/// other than the one its world was resolved against.
///
/// This is where it differs from [`VoxelMedium`], and the difference is the
/// point: that is a plain struct with public fields, so a caller can still
/// inherit a field from `..VoxelMedium::NOTHING`. Here there is nothing to
/// inherit and nothing to construct.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MediumIndex(u32);

impl MediumIndex {
    /// The index of "no medium here" — an empty cell, and everything outside the
    /// volume. Always present, at every width.
    pub const NOTHING: Self = Self(0);

    /// The index's number, for the packing that holds it.
    pub(super) const fn get(self) -> u32 {
        self.0
    }
}

/// The distinct media one registry declares, entry 0 being
/// [`VoxelMedium::NOTHING`].
#[derive(Debug)]
pub(super) struct MediumTable {
    media: Vec<VoxelMedium>,
}

impl MediumTable {
    /// Every distinct medium `registry` declares, "nothing" first.
    ///
    /// A linear scan rather than a hash: the count is the number of distinct
    /// `(swimmable, move_resistance, swim_ascent)` answers content declares —
    /// as [`declared_by`] resolves them, so a non-swimmable definition's ascent
    /// is already masked away and mints no entry — which for the shipped game is
    /// one and for any plausible registry is a handful. Hashing an `f32` would
    /// also have to decide what to do about a value that is not its own key.
    pub(super) fn of(registry: &BlockRegistry) -> Self {
        let mut media = vec![VoxelMedium::NOTHING];
        for stated in registry.definitions().map(declared_by) {
            let unseen = !media.contains(&stated);
            media.extend(unseen.then_some(stated));
        }
        Self { media }
    }

    /// The index this table holds for `declared`'s medium.
    ///
    /// A definition the table's registry never declared answers
    /// [`MediumIndex::NOTHING`]. Unreachable through the one caller, which
    /// resolves names through the very registry the table was built from, and
    /// "no medium" is the conservative reading for anything else.
    pub(super) fn index_of(&self, declared: &BlockDefinition) -> MediumIndex {
        let stated = declared_by(declared);
        self.media
            .iter()
            .position(|held| *held == stated)
            .and_then(|at| u32::try_from(at).ok())
            .map_or(MediumIndex::NOTHING, MediumIndex)
    }

    /// The medium `index` names, and [`VoxelMedium::NOTHING`] for one this table
    /// does not hold.
    ///
    /// Total, which is what lets a read past the end of a packed array answer
    /// zero rather than panicking on the tick path.
    pub(super) fn at(&self, index: u32) -> VoxelMedium {
        usize::try_from(index)
            .ok()
            .and_then(|at| self.media.get(at))
            .copied()
            .unwrap_or(VoxelMedium::NOTHING)
    }

    /// How many bits an index into this table needs.
    pub(super) fn width_in_bits(&self) -> u32 {
        PackedArray::width_for(self.media.len())
    }
}

/// What `declared` says its volume does to something moving through it.
///
/// Every answer reads from its own field. A medium derived from `is_solid` would
/// make every solid block in existence swimmable and would invent a claim no
/// author made.
///
/// **The one exception is the ascent, and it is a mask rather than a
/// derivation.** A definition that is not swimmable contributes an ascent of
/// `0.0`, so a declaration's default of `9.0` cannot reach a swimmer through a
/// neighbouring cell: a box overlapping water and an ordinary block would
/// otherwise fold the block's unstated `9.0` over the water's declared rate and
/// launch the swimmer at the very speed this property exists to replace. Both
/// fields read here are declared, both belong to the medium, and the registry
/// still reports each independently — this states nothing the physics does not
/// already say, that a volume holding nobody up lifts nobody.
///
/// **It belongs here and in neither of the two places it could go instead.** Not
/// in [`VoxelMedium::with`], which would give the same physics while leaving
/// `{not swimmable, no resistance, 9.0}` in the table as a distinct entry for
/// every ordinary block — the shipped index doubling from one bit to two, 128
/// KiB over the world, and the width reading beside it is what reports that. Not
/// in the loader either, which deletes this arm and looks strictly simpler: it
/// loses the author's declared number permanently, and a registry must report a
/// stated ascent whether or not the same declaration states a buoyancy, the two
/// being independent in both directions.
fn declared_by(declared: &BlockDefinition) -> VoxelMedium {
    VoxelMedium {
        swimmable: declared.swimmable,
        resistance: declared.move_resistance,
        swim_ascent: if declared.swimmable {
            declared.swim_ascent
        } else {
            0.0
        },
    }
}
