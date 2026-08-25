//! The world's voxels, resolved once into the three answers a tick reads: what
//! stops the player, what a ray may stop at, and what medium a voxel's volume
//! is.
//!
//! The tick asks a packed array and never a world. Resolving every voxel once,
//! at construction, is what makes each answer **total**: afterwards a query is a
//! bounds test, one packed read and — for the medium — one lookup in a table of
//! at most a handful of entries, with no name to look up, no registry to consult
//! and so no failure for anything on the tick path to swallow. It is also what
//! keeps the replay's overlap oracle independent — the oracle re-reads the
//! world's own block query and the registry, so the two judgements share no
//! lookup chain and cannot agree with each other's mistakes.
//!
//! **Three views and not one widened view.** Collision and aiming are separate
//! claims content declares separately, so they are separate bits and separate
//! traits; a medium is a third claim, and it is one view rather than two because
//! one site reads both of its properties from one fold over one box. What they
//! share is the walk that fills them and the write that keeps them in step. The
//! cost is derived rather than estimated: the shipped world is
//! 64 × 64 × 256 = 1 048 576 voxels, so each one-bit view is **+128 KiB**, once,
//! at world scale — and the medium view is an *index* whose width is chosen from
//! how many distinct media the registry declares, which for the shipped content
//! is one bit and so the same figure again.
//!
//! **The medium table is built from the registry and never from the world's
//! contents.** A block the world does not yet hold must already have an index,
//! because a later write may place it — so a table built from what a volume
//! happens to contain would have to grow, and widening the packing under an edit
//! is what [`ResolvedVoxels::set`] exists not to do.
//!
//! Every answer comes from `BlockDefinition` through the registry and from
//! nothing else. Comparing a block *name* here would be a game rule written in
//! Rust, which invariant 1 forbids, and nothing in this module knows what any
//! block is called.
//!
//! **Every coordinate outside the volume answers "nothing", and that is one
//! bounds test rather than a conversion.** A box that has walked off the
//! footprint or fallen below `y = 0` carries negative coordinates, and an
//! unsigned world query would have to convert them — where saturating stands the
//! player on column 0's terrain and wrapping stands it on the far edge's, both
//! silently and both while the player is nowhere near either. So the one
//! conversion in this module refuses instead of converting, and it is the only
//! place a sign is read.

use std::fmt;

use mc_core::block::{BlockDefinition, BlockRegistry, RegistryError};
use mc_core::id::BlockName;
use mc_world::column::COLUMN_HEIGHT;
use mc_world::section::Contents;
use mc_world::world::{Extent, VoxelWorld, WorldPos};

use crate::player::{BlockPos, Medium, Solidity, Targetable, VoxelMedium};

use super::medium::{MediumIndex, MediumTable};
use super::packed::PackedArray;

/// How wide a view answering one `bool` per voxel is.
const ONE_BIT: u32 = 1;

use super::world::{FOOTPRINT, ReplayWorld};

/// A finite volume of named voxels, which [`ResolvedVoxels`] resolves once.
///
/// The world is read through this rather than concretely, because the replay's
/// world can only place the blocks the scripted scene declares — so a scenario
/// about a block whose definition disagrees with its name has no other way to
/// state itself. Nothing here decides anything about a block: a volume says
/// which block is where, and the registry says what that block is.
pub trait BlockVolume {
    /// How far the volume reaches on each axis.
    fn extent(&self) -> Extent;

    /// What the cell at a position holds, or `None` where the volume does not
    /// reach it.
    ///
    /// **The `Option` says the volume reaches this position and nothing else.**
    /// A cell it reaches that holds nothing is `Some(Contents::Empty)`.
    fn block_at(&self, x: u32, y: u32, z: u32) -> Option<Contents<&BlockName>>;
}

impl BlockVolume for ReplayWorld {
    fn extent(&self) -> Extent {
        Extent {
            x: FOOTPRINT,
            y: COLUMN_HEIGHT,
            z: FOOTPRINT,
        }
    }

    fn block_at(&self, x: u32, y: u32, z: u32) -> Option<Contents<&BlockName>> {
        // The inherent query, named through the type so that a reader does not
        // have to know that an inherent method wins over a trait one.
        ReplayWorld::block_at(self, x, y, z)
    }
}

/// A world of columns is a volume of named voxels, and answers as one.
///
/// Its own refusal is richer than this trait's — it says *which* edge a position
/// was outside of — and that difference is deliberate: a resolve walks only
/// positions the extent produced, so there is nothing here for a caller to act
/// on, and the trait's `Option` is the same "nothing to stand on" the space
/// above a world's terrain answers with.
impl BlockVolume for VoxelWorld {
    fn extent(&self) -> Extent {
        VoxelWorld::extent(self)
    }

    fn block_at(&self, x: u32, y: u32, z: u32) -> Option<Contents<&BlockName>> {
        VoxelWorld::block_at(self, WorldPos { x, y, z }).ok()
    }
}

/// What each voxel of a volume answers about stopping the player, about stopping
/// a ray, and about what its volume does to something moving through it,
/// resolved once.
///
/// **Named for no one question, because it answers three.** A type called after
/// one of the properties it carries is how a reader comes to believe the others
/// are derived from it, and they are independent declarations.
///
/// Total by construction: every position outside the volume — past its far
/// edges, above it, or negative on any axis — is neither solid nor targetable
/// and is [`VoxelMedium::NOTHING`], and that is a property of one bounds test
/// rather than of a conversion that could saturate or wrap into a column that is
/// not the one asked about.
pub struct ResolvedVoxels {
    extent: Extent,
    solid: PackedArray,
    targetable: PackedArray,
    /// Which entry of [`media`](Self::media) each voxel's volume is.
    medium: PackedArray,
    /// The distinct media the registry declared, which
    /// [`medium`](Self::medium) indexes.
    media: MediumTable,
}

impl ResolvedVoxels {
    /// Resolves every voxel of `volume` through `registry`.
    ///
    /// A cell holding nothing contributes nothing, and so does a position the
    /// volume does not reach. **They are two facts and get two arms**, even
    /// though both answer `false`: an empty cell is the absence of anything to
    /// stand on, and a position outside the volume is a position the walk never
    /// produces. Collapsing them would make one edit break both, and a defect in
    /// the extent would arrive looking like a defect about emptiness.
    ///
    /// Neither is a failure the volume reports — the one fallible step here is
    /// resolving a name, and it is propagated.
    ///
    /// # Errors
    ///
    /// Returns [`RegistryError::UnknownName`] if the volume holds a block the
    /// registry does not know. A name that cannot be resolved is reported and
    /// never answered as "not solid": that would be a swallowed error on the one
    /// path nothing downstream can re-check.
    pub fn resolve(
        volume: &dyn BlockVolume,
        registry: &BlockRegistry,
    ) -> Result<Self, RegistryError> {
        let extent = volume.extent();
        let media = MediumTable::of(registry);
        let mut recent = LastResolved::nothing();
        let mut solid = Vec::with_capacity(extent.voxel_count());
        let mut targetable = Vec::with_capacity(extent.voxel_count());
        let mut medium = Vec::with_capacity(extent.voxel_count());
        for at in extent.positions() {
            let answers = match volume.block_at(at.x, at.y, at.z) {
                Some(Contents::Holds(name)) => recent.answer_for(name, registry, &media)?,
                // Nothing to stand on, nothing for a ray to stop at, and no
                // medium to move through.
                Some(Contents::Empty) => VoxelAnswers::NOTHING,
                // Outside the volume, which the walk never produces.
                None => VoxelAnswers::NOTHING,
            };
            solid.push(u32::from(answers.solid));
            targetable.push(u32::from(answers.targetable));
            medium.push(answers.medium.get());
        }
        let width = media.width_in_bits();
        Ok(Self {
            extent,
            solid: PackedArray::packing(solid, ONE_BIT),
            targetable: PackedArray::packing(targetable, ONE_BIT),
            medium: PackedArray::packing(medium, width),
            media,
        })
    }

    /// Records what the voxel at `at` answers about every question.
    ///
    /// **All of them, in one call, because a caller that could write one without
    /// the others is the disagreement this type exists to make unspellable.**
    /// One [`VoxelAnswers`] rather than a run of loose arguments, and its fields
    /// are named at the call site — so a caller passing the same answer to two
    /// of them still reads as a caller doing so on purpose, which is the
    /// property the loose form was chosen for when there were two of them.
    ///
    /// **The medium is a minted index and never a value**, which is what keeps
    /// this total. Every `bool` is writable, but a medium value is not: writing
    /// one means finding it in a table built at resolve time, and this is `pub`.
    /// Handed a value no registry produced, an implementation could only fall
    /// back silently, panic on a write path, or widen the packing under an edit
    /// — and this module's contract refuses all three. There is no way to name
    /// an index the table does not hold, so the question never arises.
    ///
    /// A position outside the volume has nothing to write and nothing is
    /// written: the type's answer for it is "nothing" by construction, and there
    /// is nothing a caller could do with a refusal here that it has not already
    /// done at the store, which refuses the same position first.
    pub fn set(&mut self, at: WorldPos, answers: VoxelAnswers) {
        if self.extent.contains(at) {
            let offset = self.extent.offset(at);
            self.solid.set(offset, u32::from(answers.solid));
            self.targetable.set(offset, u32::from(answers.targetable));
            self.medium.set(offset, answers.medium.get());
        }
    }

    /// The index this view's table holds for `declared`'s medium.
    ///
    /// The only door a [`MediumIndex`] comes through, and the reason
    /// [`set`](Self::set) is infallible.
    #[must_use]
    pub fn medium_index_of(&self, declared: &BlockDefinition) -> MediumIndex {
        self.media.index_of(declared)
    }

    /// How many bits this view spends on each voxel's medium index.
    ///
    /// One of `{1, 2, 4, 8, 16, 32}`, chosen once at resolve from how many
    /// distinct media the registry declares. A property of *content* rather than
    /// of this design: any number of blocks sharing one answer costs nothing,
    /// and only the count of distinct answers moves it.
    #[must_use]
    pub fn medium_width_in_bits(&self) -> u32 {
        self.medium.width()
    }

    /// The value `view` holds for `at`, for a position that may lie anywhere in
    /// the signed space the tick asks about, and zero outside the volume.
    ///
    /// The one place the bounds test and the sign refusal are spelled, so the
    /// three views cannot come to disagree about what "outside" means. Zero is
    /// the outside answer for all of them: not solid, not targetable, and entry
    /// zero of the medium table, which is always [`VoxelMedium::NOTHING`].
    fn held(&self, view: &PackedArray, at: BlockPos) -> u32 {
        inside_the_positive_octant(at)
            .filter(|voxel| self.extent.contains(*voxel))
            .map_or(0, |voxel| view.get(self.extent.offset(voxel)))
    }
}

/// What one voxel answers about the three questions a tick asks of it.
///
/// A named triple rather than a tuple, because two of the three are the same
/// type and reading them the wrong way round is a resolve that still compiles
/// and still fills every view. Naming them is also what lets
/// [`ResolvedVoxels::set`] take one value without losing the property loose
/// arguments were chosen for: a call still says which answer is which.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VoxelAnswers {
    /// Whether this voxel stops the player.
    pub solid: bool,
    /// Whether a ray may stop at this voxel.
    pub targetable: bool,
    /// Which medium this voxel's volume is, as an index minted by the view that
    /// will hold it.
    pub medium: MediumIndex,
}

impl VoxelAnswers {
    /// What a cell with no block in it answers: none of them.
    ///
    /// The one value a caller can name without a table having minted it, and it
    /// is the same "nothing" a position outside the volume already reads.
    pub const NOTHING: Self = Self {
        solid: false,
        targetable: false,
        medium: MediumIndex::NOTHING,
    };

    /// What `declared` says about all three questions, against the table `media`.
    ///
    /// Each read from its own field. Deriving any of them from another here
    /// would put back the single bit this whole change exists to split, and it
    /// would do it in the one place no declaration could override — which is why
    /// a medium is read from the two fields that state it and never from
    /// `is_solid`.
    fn of(declared: &BlockDefinition, media: &MediumTable) -> Self {
        Self {
            solid: declared.is_solid,
            targetable: declared.targetable,
            medium: media.index_of(declared),
        }
    }
}

/// The last name resolved, and what all of its answers were.
///
/// **Run coherence, and it is worth the type.** A resolve walks a whole world —
/// the smallest one is 16 × 256 × 16 = 65 536 voxels — and every voxel of it
/// would otherwise cost one `HashMap<BlockName, BlockId>` hash. Worlds are
/// coherent: a run of air, a run of stone. `BlockName` is `Arc`-backed and every
/// clone of a name shares one allocation, so consecutive voxels of a run are the
/// *same* allocation and can be answered without hashing anything.
///
/// It can only ever be conservative. Two distinct allocations holding the same
/// text simply miss and are resolved the slow way, which is the answer they
/// would have got anyway — so the resolution rule is still stated exactly once,
/// at `registry.resolve`.
struct LastResolved(Option<(BlockName, VoxelAnswers)>);

impl LastResolved {
    /// Nothing seen yet.
    const fn nothing() -> Self {
        Self(None)
    }

    /// What `name` answers about every question, reusing the previous answers
    /// where it is the previous name.
    ///
    /// **The triple is cached rather than any one of it.** Caching one and
    /// resolving the rest would cost the lookup this type exists to avoid on
    /// every voxel of every run, and would leave answers about one name reaching
    /// the views from different reads of the registry.
    fn answer_for(
        &mut self,
        name: &BlockName,
        registry: &BlockRegistry,
        media: &MediumTable,
    ) -> Result<VoxelAnswers, RegistryError> {
        if let Some((seen, answers)) = &self.0
            && shares_an_allocation(seen, name)
        {
            return Ok(*answers);
        }
        let answers = VoxelAnswers::of(registry.resolve(name)?, media);
        self.0 = Some((name.clone(), answers));
        Ok(answers)
    }
}

/// Whether two names are the same allocation and not merely the same text.
///
/// The address and the length rather than the fat pointer as a whole, because
/// comparing wide pointers directly is the shape `ambiguous_wide_pointer_comparisons`
/// is about.
fn shares_an_allocation(one: &BlockName, other: &BlockName) -> bool {
    let (left, right) = (one.as_str(), other.as_str());
    std::ptr::eq(left.as_ptr(), right.as_ptr()) && left.len() == right.len()
}

impl Solidity for ResolvedVoxels {
    fn is_solid(&self, at: BlockPos) -> bool {
        self.held(&self.solid, at) != 0
    }
}

impl Targetable for ResolvedVoxels {
    fn is_targetable(&self, at: BlockPos) -> bool {
        self.held(&self.targetable, at) != 0
    }
}

/// The medium is the **table entry the index names**, and the index is the third
/// packed view.
///
/// A position outside the volume reads index zero, which is
/// [`VoxelMedium::NOTHING`] by construction — the same totality the other two
/// views have, arrived at by the same bounds test.
impl Medium for ResolvedVoxels {
    fn medium_at(&self, at: BlockPos) -> VoxelMedium {
        self.media.at(self.held(&self.medium, at))
    }
}

/// The verbose half of this type is the three packed views, and it says how many
/// voxels they cover rather than which — a million values quoted into a panic
/// message would bury whatever the panic was about.
impl fmt::Debug for ResolvedVoxels {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ResolvedVoxels")
            .field("extent", &self.extent)
            .field("voxels", &self.extent.voxel_count())
            .finish_non_exhaustive()
    }
}

/// A signed position as the unsigned one a volume is indexed by, or nothing if
/// any coordinate is negative.
///
/// **The only conversion in this module, and it refuses rather than converting.**
/// A negative coordinate names a position outside the world on that axis, so
/// there is nothing there by definition; saturating it would answer for column 0
/// and wrapping it for the far edge of the footprint, and both would stand a
/// player on terrain that is not beneath it while every existing test stayed
/// green.
fn inside_the_positive_octant(at: BlockPos) -> Option<WorldPos> {
    Some(WorldPos {
        x: at.x.try_into().ok()?,
        y: at.y.try_into().ok()?,
        z: at.z.try_into().ok()?,
    })
}
