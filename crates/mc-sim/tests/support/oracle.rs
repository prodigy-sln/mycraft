//! An independent scan for the visible face area of the replay world.
//!
//! This is the judge, never the thing judged, and everything about it is chosen
//! so that it cannot agree with the mesher's mistakes. It walks the world one
//! voxel at a time through the public per-voxel accessor any caller outside the
//! crate would use, it re-derives adjacency from its own six explicit signed
//! offsets, and it reads anything outside the world as non-solid. It shares no
//! code with `mc_world::mesh` and none with `mc_render::geometry`: an oracle
//! that borrowed the mesher's adjacency table would agree with a sign inversion
//! or a swapped neighbour slot instead of catching it.
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
/// A side is visible when the voxel holding it is solid and the voxel one step
/// off that side is not — where "outside the world" counts as not solid, so the
/// world's outer shell and its floor show faces rather than being sealed shut.
///
/// # Errors
///
/// Returns [`RegistryError`] if a voxel this scan reads holds a block `registry`
/// does not register. Reported rather than read as non-solid: a silent non-solid
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
    /// all unless the voxel itself is solid.
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
        if !self.registry.resolve(name)?.is_solid {
            return Ok(());
        }
        for side in Side::ALL {
            self.add_if_visible(Face { voxel, side }, name, into)?;
        }
        Ok(())
    }

    /// Adds one unit of area for `face` unless the voxel beyond it is solid.
    ///
    /// Its own function rather than a branch inside the loop above only because
    /// `clippy.toml` allows two levels of nesting and a loop around a condition
    /// is already three. The block a face belongs to is passed in rather than
    /// looked up again: re-reading it here would be a second world access per
    /// side of every solid voxel, on the slowest walk in this suite.
    fn add_if_visible(
        &self,
        face: Face,
        block: &BlockName,
        into: &mut FaceArea,
    ) -> Result<(), RegistryError> {
        if !self.is_solid_beyond(face.voxel, face.side)? {
            *into.entry((block.clone(), face.side.facing())).or_default() += 1;
        }
        Ok(())
    }

    /// Whether the voxel one step off `side` of `voxel` is solid.
    fn is_solid_beyond(&self, voxel: Voxel, side: Side) -> Result<bool, RegistryError> {
        let (along_x, along_y, along_z) = side.step();
        let stepped = (
            i64::from(voxel.x) + along_x,
            i64::from(voxel.y) + along_y,
            i64::from(voxel.z) + along_z,
        );
        let Some(neighbour) = inside_the_world(stepped) else {
            return Ok(false);
        };
        self.is_solid_at(neighbour)
    }

    /// Whether the voxel at `voxel` is solid, reading a position the world has
    /// no answer for, and a cell holding nothing, as not solid.
    ///
    /// **Read back from the world and the registry, and from nothing the
    /// simulation computed.** The empty arm is the one this scan and the
    /// simulation's own resolution both gained at once, and an oracle that
    /// reached it by consulting a collision bitset — or by calling the same
    /// resolution the bitset was built from — would be the two of them agreeing
    /// with each other rather than one judging the other.
    fn is_solid_at(&self, voxel: Voxel) -> Result<bool, RegistryError> {
        match self.world.block_at(voxel.x, voxel.y, voxel.z) {
            None => Ok(false),
            Some(Contents::Empty) => Ok(false),
            Some(Contents::Holds(name)) => Ok(self.registry.resolve(name)?.is_solid),
        }
    }
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
