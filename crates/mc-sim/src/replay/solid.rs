//! The world's voxels, resolved once into the solidity the physics reads.
//!
//! The physics asks a bitset and never a world. Resolving every voxel once, at
//! construction, is what makes the answer **total**: afterwards a query is a
//! bounds test and a bit test, with no name to look up, no registry to consult
//! and so no failure for anything on the tick path to swallow. It is also what
//! keeps the replay's overlap oracle independent — the oracle re-reads the
//! world's own block query and the registry, so the two judgements share no
//! lookup chain and cannot agree with each other's mistakes.
//!
//! Solidity comes from `BlockDefinition::is_solid` through the registry and from
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

use mc_core::block::{BlockRegistry, RegistryError};
use mc_core::id::BlockName;
use mc_world::column::COLUMN_HEIGHT;
use mc_world::world::{Extent, VoxelWorld, WorldPos};

use crate::player::{BlockPos, Solidity};

use super::world::{FOOTPRINT, ReplayWorld};

/// A finite volume of named voxels, which [`SolidVoxels`] resolves once.
///
/// The world is read through this rather than concretely, because the replay's
/// world can only place the blocks the scripted scene declares — so a scenario
/// about a block whose definition disagrees with its name has no other way to
/// state itself. Nothing here decides anything about a block: a volume says
/// which block is where, and the registry says what that block is.
pub trait BlockVolume {
    /// How far the volume reaches on each axis.
    fn extent(&self) -> Extent;

    /// The block held at a position, or nothing where the volume holds none.
    fn block_at(&self, x: u32, y: u32, z: u32) -> Option<&BlockName>;
}

impl BlockVolume for ReplayWorld {
    fn extent(&self) -> Extent {
        Extent {
            x: FOOTPRINT,
            y: COLUMN_HEIGHT,
            z: FOOTPRINT,
        }
    }

    fn block_at(&self, x: u32, y: u32, z: u32) -> Option<&BlockName> {
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

    fn block_at(&self, x: u32, y: u32, z: u32) -> Option<&BlockName> {
        VoxelWorld::block_at(self, WorldPos { x, y, z }).ok()
    }
}

/// Whether each voxel of a volume blocks the player, resolved once.
///
/// Total by construction: every position outside the volume — past its far
/// edges, above it, or negative on any axis — is not solid, and that is a
/// property of one bounds test rather than of a conversion that could saturate
/// or wrap into a column that is not the one asked about.
pub struct SolidVoxels {
    extent: Extent,
    solid: Bitset,
}

impl SolidVoxels {
    /// Resolves every voxel of `volume` through `registry`.
    ///
    /// A position the volume holds no block at contributes nothing: an absent
    /// block is the absence of anything to stand on, which is the same answer
    /// the space above a world's terrain gives. That is a value the volume
    /// returns and not a failure it reports — the one fallible step here is
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
        for at in extent.positions() {
            solid.push(match volume.block_at(at.x, at.y, at.z) {
                Some(name) => recent.answer_for(name, registry)?,
                None => false,
            });
        }
        Ok(Self {
            extent,
            solid: Bitset::packing(&solid),
        })
    }

    /// Records whether the voxel at `at` blocks the player.
    ///
    /// A position outside the volume has no bit to write and none is written:
    /// the type's answer for it is `false` by construction and there is nothing
    /// a caller could do with a refusal here that it has not already done at the
    /// store, which refuses the same position first.
    pub fn set(&mut self, at: WorldPos, solid: bool) {
        if self.extent.contains(at) {
            self.solid.set(self.extent.offset(at), solid);
        }
    }
}

/// The last name resolved, and what it resolved to.
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
struct LastResolved(Option<(BlockName, bool)>);

impl LastResolved {
    /// Nothing seen yet.
    const fn nothing() -> Self {
        Self(None)
    }

    /// Whether `name` is solid, reusing the previous answer where it is the
    /// previous name.
    fn answer_for(
        &mut self,
        name: &BlockName,
        registry: &BlockRegistry,
    ) -> Result<bool, RegistryError> {
        if let Some((seen, answer)) = &self.0
            && shares_an_allocation(seen, name)
        {
            return Ok(*answer);
        }
        let answer = registry.resolve(name)?.is_solid;
        self.0 = Some((name.clone(), answer));
        Ok(answer)
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

impl Solidity for SolidVoxels {
    fn is_solid(&self, at: BlockPos) -> bool {
        inside_the_positive_octant(at).is_some_and(|voxel| {
            self.extent.contains(voxel) && self.solid.holds(self.extent.offset(voxel))
        })
    }
}

/// The verbose half of this type is the bitset, and it says how many voxels it
/// covers rather than which — a million bits quoted into a panic message would
/// bury whatever the panic was about.
impl fmt::Debug for SolidVoxels {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SolidVoxels")
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
/// would cost for the same answer.
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
    fn packing(solid: &[bool]) -> Self {
        Self {
            words: solid.chunks(VOXELS_PER_WORD).map(packed).collect(),
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
fn packed(solid: &[bool]) -> u64 {
    solid
        .iter()
        .enumerate()
        .fold(0, |word, (bit, &blocks)| word | (u64::from(blocks) << bit))
}
