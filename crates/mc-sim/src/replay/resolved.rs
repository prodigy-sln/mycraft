//! The world's voxels, resolved once into the two answers a tick reads: what
//! stops the player, and what a ray may stop at.
//!
//! The tick asks a bitset and never a world. Resolving every voxel once, at
//! construction, is what makes each answer **total**: afterwards a query is a
//! bounds test and a bit test, with no name to look up, no registry to consult
//! and so no failure for anything on the tick path to swallow. It is also what
//! keeps the replay's overlap oracle independent — the oracle re-reads the
//! world's own block query and the registry, so the two judgements share no
//! lookup chain and cannot agree with each other's mistakes.
//!
//! **Two views and not one widened view.** Collision and aiming are separate
//! claims content declares separately, so they are separate bits and separate
//! traits; what they share is the walk that fills them and the write that keeps
//! them in step. The cost is derived rather than estimated: the shipped world is
//! 64 × 64 × 256 = 1 048 576 voxels at one bit each, so the second view is
//! **+128 KiB**, once, at world scale.
//!
//! Both answers come from `BlockDefinition` through the registry and from
//! nothing else. Comparing a block *name* here would be a game rule written in
//! Rust, which invariant 1 forbids, and nothing in this module knows what any
//! block is called.
//!
//! **Every coordinate outside the volume answers `false`, and that is one bounds
//! test rather than a conversion.** A box that has walked off the footprint or
//! fallen below `y = 0` carries negative coordinates, and an unsigned world query
//! would have to convert them — where saturating stands the player on column 0's
//! terrain and wrapping stands it on the far edge's, both silently and both while
//! the player is nowhere near either. So the one conversion in this module
//! refuses instead of converting, and it is the only place a sign is read.

use std::fmt;

use mc_core::block::{BlockDefinition, BlockRegistry, RegistryError};
use mc_core::id::BlockName;
use mc_world::column::COLUMN_HEIGHT;
use mc_world::section::Contents;
use mc_world::world::{Extent, VoxelWorld, WorldPos};

use crate::player::{BlockPos, Solidity, Targetable};

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

/// What each voxel of a volume answers about stopping the player and about
/// stopping a ray, resolved once.
///
/// **Named for neither question, because it answers both.** A type called after
/// one of the two properties it carries is how a reader comes to believe the
/// other is derived from it, and they are independent declarations.
///
/// Total by construction: every position outside the volume — past its far
/// edges, above it, or negative on any axis — is neither solid nor targetable,
/// and that is a property of one bounds test rather than of a conversion that
/// could saturate or wrap into a column that is not the one asked about.
pub struct ResolvedVoxels {
    extent: Extent,
    solid: Bitset,
    targetable: Bitset,
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
        let mut recent = LastResolved::nothing();
        let mut solid = Vec::with_capacity(extent.voxel_count());
        let mut targetable = Vec::with_capacity(extent.voxel_count());
        for at in extent.positions() {
            let answers = match volume.block_at(at.x, at.y, at.z) {
                Some(Contents::Holds(name)) => recent.answer_for(name, registry)?,
                // Nothing to stand on, and nothing for a ray to stop at.
                Some(Contents::Empty) => Resolved::NOTHING,
                // Outside the volume, which the walk never produces.
                None => Resolved::NOTHING,
            };
            solid.push(answers.solid);
            targetable.push(answers.targetable);
        }
        Ok(Self {
            extent,
            solid: Bitset::packing(&solid),
            targetable: Bitset::packing(&targetable),
        })
    }

    /// Records what the voxel at `at` answers about both questions.
    ///
    /// **Both, in one call, because a caller that could write one without the
    /// other is the disagreement this type exists to make unspellable.** The
    /// arguments are separate rather than one packed value so that a caller
    /// passing the same answer twice reads as a caller doing so on purpose.
    ///
    /// A position outside the volume has no bit to write and none is written:
    /// the type's answer for it is `false` by construction and there is nothing
    /// a caller could do with a refusal here that it has not already done at the
    /// store, which refuses the same position first.
    pub fn set(&mut self, at: WorldPos, solid: bool, targetable: bool) {
        if self.extent.contains(at) {
            let offset = self.extent.offset(at);
            self.solid.set(offset, solid);
            self.targetable.set(offset, targetable);
        }
    }

    /// Whether the bit at `at` is marked in `view`, for a position that may lie
    /// anywhere in the signed space the tick asks about.
    ///
    /// The one place the bounds test and the sign refusal are spelled, so the
    /// two views cannot come to disagree about what "outside" means.
    fn marked(&self, view: &Bitset, at: BlockPos) -> bool {
        inside_the_positive_octant(at).is_some_and(|voxel| {
            self.extent.contains(voxel) && view.holds(self.extent.offset(voxel))
        })
    }
}

/// What one voxel answers about the two questions a tick asks of it.
///
/// A named pair rather than a tuple of two booleans, because the two are the
/// same type and reading them the wrong way round is a resolve that still
/// compiles and still fills both bitsets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Resolved {
    solid: bool,
    targetable: bool,
}

impl Resolved {
    /// What a cell with no block in it answers: neither.
    const NOTHING: Self = Self {
        solid: false,
        targetable: false,
    };

    /// What `declared` says about both questions.
    ///
    /// Each read from its own field. Deriving either from the other here would
    /// put back the single bit this whole change exists to split, and it would
    /// do it in the one place no declaration could override.
    fn of(declared: &BlockDefinition) -> Self {
        Self {
            solid: declared.is_solid,
            targetable: declared.targetable,
        }
    }
}

/// The last name resolved, and what both of its answers were.
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
struct LastResolved(Option<(BlockName, Resolved)>);

impl LastResolved {
    /// Nothing seen yet.
    const fn nothing() -> Self {
        Self(None)
    }

    /// What `name` answers about both questions, reusing the previous answers
    /// where it is the previous name.
    ///
    /// **The pair is cached rather than either half.** Caching one and resolving
    /// the other would cost the lookup this type exists to avoid on every voxel
    /// of every run, and would leave two answers about one name reaching the
    /// bitsets from two different reads of the registry.
    fn answer_for(
        &mut self,
        name: &BlockName,
        registry: &BlockRegistry,
    ) -> Result<Resolved, RegistryError> {
        if let Some((seen, answers)) = &self.0
            && shares_an_allocation(seen, name)
        {
            return Ok(*answers);
        }
        let answers = Resolved::of(registry.resolve(name)?);
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
        self.marked(&self.solid, at)
    }
}

impl Targetable for ResolvedVoxels {
    fn is_targetable(&self, at: BlockPos) -> bool {
        self.marked(&self.targetable, at)
    }
}

/// The verbose half of this type is the two bitsets, and it says how many voxels
/// they cover rather than which — a million bits quoted into a panic message
/// would bury whatever the panic was about.
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

/// How many voxels one word of the bitset carries, and how an offset is split
/// into the word holding it and the bit of that word.
///
/// A shift and a mask rather than a division and a remainder, which is the same
/// arithmetic for a power of two and is how `replay::world` already reads a
/// world coordinate as a column and a position inside it.
const VOXELS_PER_WORD: usize = u64::BITS as usize;
const WORD_SHIFT: u32 = VOXELS_PER_WORD.trailing_zeros();
const WORD_MASK: usize = VOXELS_PER_WORD - 1;

/// One bit per voxel, in the order [`Extent::offset`] numbers them.
///
/// A bitset rather than a `Vec<bool>` because the replay's footprint is a
/// million voxels: 128 KB held for the run, against the megabyte a byte apiece
/// would cost for the same answer. That figure is per view, and there are two.
///
/// It carries no idea of *which* question it answers, which is what lets one
/// type serve both without either view's arithmetic being written twice.
#[derive(Debug)]
struct Bitset {
    words: Vec<u64>,
}

impl Bitset {
    /// Packs one flag per voxel, in offset order, into words.
    ///
    /// Built whole from the flags rather than filled in by offset, so there is
    /// no position a caller could mark that the bitset does not cover — the
    /// length is the flags' length by construction.
    fn packing(flags: &[bool]) -> Self {
        Self {
            words: flags.chunks(VOXELS_PER_WORD).map(packed).collect(),
        }
    }

    /// Whether the voxel at `offset` is marked.
    ///
    /// An offset past the end is unmarked rather than a panic, which is the
    /// answer that keeps this total; the bounds test that makes it unreachable
    /// is the extent's, in the one caller.
    fn holds(&self, offset: usize) -> bool {
        let carried = 1 << (offset & WORD_MASK);
        self.words
            .get(offset >> WORD_SHIFT)
            .is_some_and(|word| word & carried != 0)
    }

    /// Marks or unmarks the voxel at `offset`.
    ///
    /// An offset past the end writes nothing, for the same reason one past the
    /// end reads as unmarked: the length is the flags' length by construction,
    /// so there is no position a caller can reach that the bitset was built
    /// without, and the bound that makes this unreachable is the extent's.
    fn set(&mut self, offset: usize, holds: bool) {
        if let Some(word) = self.words.get_mut(offset >> WORD_SHIFT) {
            *word = with_bit(*word, offset & WORD_MASK, holds);
        }
    }
}

/// `word` with the flag at `bit` reading `holds`.
const fn with_bit(word: u64, bit: usize, holds: bool) -> u64 {
    let carried: u64 = 1 << bit;
    if holds {
        word | carried
    } else {
        word & !carried
    }
}

/// One word from up to [`VOXELS_PER_WORD`] flags, the lowest offset in the
/// lowest bit.
fn packed(flags: &[bool]) -> u64 {
    flags
        .iter()
        .enumerate()
        .fold(0, |word, (bit, &marked)| word | (u64::from(marked) << bit))
}
