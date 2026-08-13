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

use crate::player::{BlockPos, Solidity};

use super::world::{FOOTPRINT, ReplayWorld};

/// How many voxels a volume spans on each axis, counted from the origin.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Extent {
    pub x: u32,
    pub y: u32,
    pub z: u32,
}

impl Extent {
    /// How many voxels the extent holds.
    const fn voxel_count(self) -> usize {
        self.x as usize * self.y as usize * self.z as usize
    }

    /// Whether a position lies inside the extent.
    const fn contains(self, at: Voxel) -> bool {
        at.x < self.x && at.y < self.y && at.z < self.z
    }

    /// Where the voxel at a position sits in the bitset — x fastest, then z,
    /// then y, which is the order [`Extent::positions`] walks them in.
    ///
    /// Meaningful only for a position the extent [`contains`](Extent::contains);
    /// both callers below test that first, which is what keeps this arithmetic
    /// rather than fallible.
    const fn offset(self, at: Voxel) -> usize {
        (at.y as usize * self.z as usize + at.z as usize) * self.x as usize + at.x as usize
    }

    /// Every position inside the extent, x fastest.
    fn positions(self) -> impl Iterator<Item = Voxel> {
        (0..self.y).flat_map(move |y| {
            (0..self.z).flat_map(move |z| (0..self.x).map(move |x| Voxel { x, y, z }))
        })
    }
}

/// A position inside a volume, in the unsigned coordinates a volume is indexed
/// by.
///
/// Distinct from [`BlockPos`] on purpose: that one is signed because the player
/// is not confined to the world, and this one is what is left of it once the
/// sign has been read and refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Voxel {
    x: u32,
    y: u32,
    z: u32,
}

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
        let solid: Vec<bool> = extent
            .positions()
            .map(|at| blocks_the_player(volume, registry, at))
            .collect::<Result<_, _>>()?;
        Ok(Self {
            extent,
            solid: Bitset::packing(&solid),
        })
    }
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

/// Whether the volume holds a block at `at` that the registry calls solid.
fn blocks_the_player(
    volume: &dyn BlockVolume,
    registry: &BlockRegistry,
    at: Voxel,
) -> Result<bool, RegistryError> {
    match volume.block_at(at.x, at.y, at.z) {
        Some(name) => Ok(registry.resolve(name)?.is_solid),
        None => Ok(false),
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
fn inside_the_positive_octant(at: BlockPos) -> Option<Voxel> {
    Some(Voxel {
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
}

/// One word from up to [`VOXELS_PER_WORD`] flags, the lowest offset in the
/// lowest bit.
fn packed(solid: &[bool]) -> u64 {
    solid
        .iter()
        .enumerate()
        .fold(0, |word, (bit, &blocks)| word | (u64::from(blocks) << bit))
}
