//! An independent scan for the visible face area of the replay world.
//!
//! This is the judge, never the thing judged, and everything about it is chosen
//! so that it cannot agree with the mesher's mistakes. It walks the world one
//! voxel at a time through the public per-voxel accessor any caller outside the
//! crate would use, it re-derives adjacency from its own six explicit signed
//! offsets, and it reads anything outside the world as hiding nothing. It shares
//! no code with `mc_world::mesh` and none with `mc_render::geometry`: an oracle
//! that borrowed the mesher's adjacency table would agree with a sign inversion
//! or a swapped neighbour slot instead of catching it.
//!
//! # Three questions, answered from the specification and not from the mesher
//!
//! A side of a voxel is visible when three things hold at once: the block is
//! **drawn**, whatever lies beyond the side does not **occlude**, and whatever
//! lies beyond it is not the **same block** — nothing draws a face against its
//! own kind.
//!
//! **The third question is answered by comparing block names, and that is
//! deliberate.** The mesher answers it by comparing keys in a table it
//! deduplicates by name, over a boundary plane carrying one key per cell. This
//! walk has neither, must not acquire either, and would be a second copy of the
//! culling predicate if it did. Block names appear in this suite in full — files
//! under `tests/` are outside `mc-world`'s hardcoded-name scan and the module
//! this one belongs to says so at its head — so a name comparison here is a
//! *different* implementation of the same rule rather than a second call to the
//! one under test.
//!
//! Before the shipped water declared anything, every shipped block had
//! `drawn == occludes == solid`, so this walk and the two-question one it
//! replaces answer identically over the shipped world. **That agreement is not
//! evidence and never was**: what makes this pair meaningful is the world having
//! a block where the three come apart, and the third question being a
//! reimplementation the mesher does not share.
//!
//! [`Side`] exists rather than reusing the mesher's own facing enumeration for
//! exactly that reason — the six offsets below are the oracle's own. The
//! translation from a side to the facing a quad carries is one hand-written
//! match at the very end, which is the only place the two vocabularies meet.
//!
//! It is deliberately the slow, obvious implementation: one registry lookup per
//! voxel side, no bitmasks and no resolution pass. Being obviously right is the
//! only property it needs.
//!
//! **Area, not quad count.** Greedy merging changes how visible faces are
//! grouped into rectangles but never which faces are visible, so summed area is
//! the invariant the two sides can be compared on and a count is not.

use std::collections::BTreeMap;

use mc_core::block::{BlockRegistry, RegistryError};
use mc_core::id::BlockName;
use mc_sim::replay::ReplayWorld;
use mc_world::column::COLUMN_HEIGHT;
use mc_world::mesh::Facing;
use mc_world::section::Contents;

use super::FOOTPRINT;

/// Visible face area, per block and per direction the face points.
pub type FaceArea = BTreeMap<(BlockName, Facing), u64>;

/// A voxel of the world, in world coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Voxel {
    pub x: u32,
    pub y: u32,
    pub z: u32,
}

/// One of the six sides of a voxel, named by the direction it faces.
///
/// Carries no promise about order; nothing here depends on one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Side {
    NegX,
    PosX,
    NegY,
    PosY,
    NegZ,
    PosZ,
}

impl Side {
    /// Every side.
    pub const ALL: [Self; 6] = [
        Self::NegX,
        Self::PosX,
        Self::NegY,
        Self::PosY,
        Self::NegZ,
        Self::PosZ,
    ];

    /// The step from a voxel to the voxel this side faces.
    ///
    /// The six offsets are written out once, here, and nothing else in this file
    /// knows which way a side points. They are the oracle's own: the mesher's
    /// answer to this same question is the thing under test.
    const fn step(self) -> (i64, i64, i64) {
        match self {
            Self::NegX => (-1, 0, 0),
            Self::PosX => (1, 0, 0),
            Self::NegY => (0, -1, 0),
            Self::PosY => (0, 1, 0),
            Self::NegZ => (0, 0, -1),
            Self::PosZ => (0, 0, 1),
        }
    }

    /// Which way a quad covering this side points.
    ///
    /// The one place this file names the mesher's vocabulary. Written out by
    /// hand rather than derived, so a change to the facing enumeration cannot
    /// silently carry the oracle with it.
    const fn facing(self) -> Facing {
        match self {
            Self::NegX => Facing::NegX,
            Self::PosX => Facing::PosX,
            Self::NegY => Facing::NegY,
            Self::PosY => Facing::PosY,
            Self::NegZ => Facing::NegZ,
            Self::PosZ => Facing::PosZ,
        }
    }
}

/// The visible face area of every block in `world`, one unit per exposed voxel
/// side.
///
/// A side is visible when the block holding it is drawn, and whatever lies one
/// step off that side neither occludes nor is the same block — where "outside
/// the world" and "this cell holds nothing" both hide nothing and are the same
/// kind as nothing, so the world's outer shell and its floor show faces rather
/// than being sealed shut.
///
/// # Errors
///
/// Returns [`RegistryError`] if a voxel this scan reads holds a block `registry`
/// does not register. Reported rather than read as not drawn: a silent not-drawn
/// would delete faces from the answer the caller trusts this to give, and
/// nothing downstream could tell.
pub fn visible_face_area(
    world: &ReplayWorld,
    registry: &BlockRegistry,
) -> Result<FaceArea, RegistryError> {
    let scan = Scan { world, registry };
    let mut area = FaceArea::new();
    for voxel in every_voxel() {
        scan.add_visible_sides_of(voxel, &mut area)?;
    }
    Ok(area)
}

/// One side of one voxel, which is what a unit of visible area is.
///
/// A pair rather than two parameters: `clippy.toml` caps a function at four
/// arguments and clippy runs `--all-targets`, so naming the pair is what keeps
/// the helper below inside the cap without dropping the block it is counting.
#[derive(Debug, Clone, Copy)]
struct Face {
    voxel: Voxel,
    side: Side,
}

/// One world and what to ask about the blocks in it.
struct Scan<'a> {
    world: &'a ReplayWorld,
    registry: &'a BlockRegistry,
}

impl Scan<'_> {
    /// Adds one unit of area for every visible side of `voxel`, which is none at
    /// all unless the block it holds is drawn.
    fn add_visible_sides_of(&self, voxel: Voxel, into: &mut FaceArea) -> Result<(), RegistryError> {
        // Three answers, three arms, and never two of them folded together. "The
        // world does not reach here" and "this cell holds nothing" both end in no
        // face, which is exactly what would make writing them as one invisible in
        // the output — and this scan is the independent judge of a world whose own
        // reader gained the same distinction in the same change.
        let name = match self.world.block_at(voxel.x, voxel.y, voxel.z) {
            None => return Ok(()),
            Some(Contents::Empty) => return Ok(()),
            Some(Contents::Holds(name)) => name,
        };
        if !self.registry.resolve(name)?.drawn {
            return Ok(());
        }
        for side in Side::ALL {
            self.add_if_visible(Face { voxel, side }, name, into)?;
        }
        Ok(())
    }

    /// Adds one unit of area for `face` unless whatever lies beyond it hides it.
    ///
    /// Its own function rather than a branch inside the loop above only because
    /// `clippy.toml` allows two levels of nesting and a loop around a condition
    /// is already three. The block a face belongs to is passed in rather than
    /// looked up again: re-reading it here would be a second world access per
    /// side of every drawn voxel, on the slowest walk in this suite — and the
    /// same-kind question below needs it anyway.
    fn add_if_visible(
        &self,
        face: Face,
        block: &BlockName,
        into: &mut FaceArea,
    ) -> Result<(), RegistryError> {
        if self.shows(face, block)? {
            *into.entry((block.clone(), face.side.facing())).or_default() += 1;
        }
        Ok(())
    }

    /// Whether the side `face` names is visible from outside: nothing beyond it
    /// occludes, and whatever is beyond it is not the same block.
    ///
    /// **Nothing and the world's edge answer both questions the same way** — a
    /// cell holding nothing hides nothing and is not the same kind as any block,
    /// and a step out of the world reaches the same absence.
    fn shows(&self, face: Face, block: &BlockName) -> Result<bool, RegistryError> {
        match self.beyond(face.voxel, face.side) {
            Beyond::Outside | Beyond::Nothing => Ok(true),
            Beyond::Holds(name) => Ok(!self.registry.resolve(name)?.occludes && name != block),
        }
    }

    /// What lies one step off `side` of `voxel`.
    ///
    /// **Read back from the world, and from nothing the simulation computed.**
    /// An oracle that reached this by consulting a resolved bitset — or by
    /// calling the same resolution the bitset was built from — would be the two
    /// of them agreeing with each other rather than one judging the other.
    fn beyond(&self, voxel: Voxel, side: Side) -> Beyond<'_> {
        let (along_x, along_y, along_z) = side.step();
        let stepped = (
            i64::from(voxel.x) + along_x,
            i64::from(voxel.y) + along_y,
            i64::from(voxel.z) + along_z,
        );
        let Some(neighbour) = inside_the_world(stepped) else {
            return Beyond::Outside;
        };
        // Two arms rather than one for the two ways there is nothing there: a
        // cell the world has no answer for and a cell holding nothing are
        // different facts about the world that happen to end the same way, and
        // writing them as one would make a world that had stopped answering
        // indistinguishable from one full of air.
        match self.world.block_at(neighbour.x, neighbour.y, neighbour.z) {
            None => Beyond::Nothing,
            Some(Contents::Empty) => Beyond::Nothing,
            Some(Contents::Holds(name)) => Beyond::Holds(name),
        }
    }
}

/// What a step off one side of a voxel reaches.
///
/// Three answers and no fourth, so that "the world does not reach here", "this
/// cell holds nothing" and "this cell holds a block" cannot arrive under one
/// another's name.
#[derive(Debug, Clone, Copy)]
enum Beyond<'a> {
    /// The step left the loaded world.
    Outside,
    /// The cell is inside the world and holds nothing.
    Nothing,
    /// The cell holds this block.
    Holds(&'a BlockName),
}

/// Where a stepped position lands, or nothing if it left the world.
fn inside_the_world(stepped: (i64, i64, i64)) -> Option<Voxel> {
    let (x, y, z) = stepped;
    Some(Voxel {
        x: bounded(x, FOOTPRINT)?,
        y: bounded(y, COLUMN_HEIGHT)?,
        z: bounded(z, FOOTPRINT)?,
    })
}

/// One axis of a position, if it lies in `0..limit`.
fn bounded(coordinate: i64, limit: u32) -> Option<u32> {
    (coordinate >= 0 && coordinate < i64::from(limit)).then_some(coordinate as u32)
}

/// Every voxel the world spans, x fastest, then y, then z.
fn every_voxel() -> impl Iterator<Item = Voxel> {
    (0..FOOTPRINT).flat_map(|z| {
        (0..COLUMN_HEIGHT).flat_map(move |y| (0..FOOTPRINT).map(move |x| Voxel { x, y, z }))
    })
}
