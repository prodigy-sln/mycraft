//! An independent scan for the visible faces of a section.
//!
//! This is the judge, never the thing judged. It exists before the mesher does,
//! which is what makes "it shares no code with the mesher" a fact about the order
//! these files were written in rather than a discipline somebody has to keep: at
//! the moment this file was written there was no facing enumeration, no neighbour
//! container and no adjacency table to borrow. It has to stay that way. An oracle
//! that reached for the mesher's adjacency would agree with a sign inversion or a
//! swapped neighbour slot, and the scenarios that use it as a judge are precisely
//! the ones written to kill those two mistakes.
//!
//! So the six signed offsets below are its own, the six neighbours arrive in its
//! own struct, and every voxel is read through the public per-voxel API any
//! caller outside this crate would use. It is deliberately the slow, obvious
//! implementation — one registry lookup per voxel side, no bitmasks, no
//! resolution pass — because being obviously right is the only property it needs
//! and speed is the mesher's job, not the judge's.
//!
//! The neighbours are named fields rather than positional parameters because
//! `clippy.toml` caps a function at four arguments and clippy runs
//! `--all-targets`. Named fields keep what positional ones were for: a call site
//! where a swapped neighbour is visible to a reader.

use std::collections::BTreeSet;

use mc_core::block::BlockRegistry;
use mc_world::section::{LocalPos, SECTION_SIZE, Section, SectionError};

/// One of the six sides of a voxel, named by the direction it faces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
    ///
    /// Carries no promise about order. The order faces are emitted in is one of
    /// the things this file is here to judge, so a test comparing sequences must
    /// derive its order from the specification and never from this list.
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
    /// knows which way a side points. They are the oracle's own on purpose: the
    /// mesher's answer to this same question is the thing under test.
    const fn step(self) -> (i32, i32, i32) {
        match self {
            Self::NegX => (-1, 0, 0),
            Self::PosX => (1, 0, 0),
            Self::NegY => (0, -1, 0),
            Self::PosY => (0, 1, 0),
            Self::NegZ => (0, 0, -1),
            Self::PosZ => (0, 0, 1),
        }
    }
}

/// A face that would be drawn: the solid voxel that emits it, and the side of
/// that voxel it sits on.
///
/// The voxel is spelled out as three coordinates rather than carried as a
/// [`LocalPos`] so that a face can go into a set — which is the shape three of
/// the four scenarios this oracle serves compare against, and the fourth counts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VisibleFace {
    pub x: u32,
    pub y: u32,
    pub z: u32,
    pub side: Side,
}

impl VisibleFace {
    /// The face on `side` of the voxel at `voxel`.
    #[must_use]
    pub const fn at(voxel: LocalPos, side: Side) -> Self {
        Self {
            x: voxel.x,
            y: voxel.y,
            z: voxel.z,
            side,
        }
    }
}

/// The six sections around the one being scanned, each supplied or not.
///
/// Six independent options, so absence is per neighbour and never
/// all-or-nothing: a section may be scanned with its section below supplied and
/// the other five missing, and the answer must differ from the one it gives with
/// all six missing. [`Default`] is every neighbour absent.
#[derive(Debug, Default, Clone, Copy)]
pub struct Neighbourhood<'a> {
    pub neg_x: Option<&'a Section>,
    pub pos_x: Option<&'a Section>,
    pub neg_y: Option<&'a Section>,
    pub pos_y: Option<&'a Section>,
    pub neg_z: Option<&'a Section>,
    pub pos_z: Option<&'a Section>,
}

impl<'a> Neighbourhood<'a> {
    /// The section beyond `side`, if one was supplied.
    const fn beyond(&self, side: Side) -> Option<&'a Section> {
        match side {
            Side::NegX => self.neg_x,
            Side::PosX => self.pos_x,
            Side::NegY => self.neg_y,
            Side::PosY => self.pos_y,
            Side::NegZ => self.neg_z,
            Side::PosZ => self.pos_z,
        }
    }
}

/// Every face of `section` that is visible: a side of a solid voxel whose
/// adjacent voxel is not solid.
///
/// A neighbour that was not supplied is read as if every voxel of it were
/// non-solid, decided per neighbour — so a boundary face at the edge of loaded
/// content is visible rather than hidden.
///
/// # Errors
///
/// Returns [`SectionError::Registry`] if a voxel this scan reads holds a block
/// `registry` does not register. A block nothing can resolve is reported rather
/// than read as non-solid: a silent non-solid would delete faces from the answer
/// this oracle is trusted to give, and nothing downstream could tell.
///
/// Only the 256 voxels of a neighbour's shared face are ever read, so a block a
/// neighbour holds away from that face is never resolved and never reported.
pub fn visible_faces<'a>(
    section: &'a Section,
    neighbourhood: &Neighbourhood<'a>,
    registry: &'a BlockRegistry,
) -> Result<BTreeSet<VisibleFace>, SectionError> {
    let scan = Scan {
        section,
        neighbourhood: *neighbourhood,
        registry,
    };
    let mut visible = BTreeSet::new();
    for voxel in every_position() {
        for side in scan.visible_sides_of(voxel)? {
            visible.insert(VisibleFace::at(voxel, side));
        }
    }
    Ok(visible)
}

/// One section, what surrounds it, and what to ask about the blocks in either.
struct Scan<'a> {
    section: &'a Section,
    neighbourhood: Neighbourhood<'a>,
    registry: &'a BlockRegistry,
}

impl Scan<'_> {
    /// Which sides of `voxel` are visible, which is none at all unless the voxel
    /// itself is solid.
    fn visible_sides_of(&self, voxel: LocalPos) -> Result<Vec<Side>, SectionError> {
        if !self.section.is_solid_at(voxel, self.registry)? {
            return Ok(Vec::new());
        }
        let mut visible = Vec::new();
        for side in Side::ALL {
            self.keep_if_visible(voxel, side, &mut visible)?;
        }
        Ok(visible)
    }

    /// Adds `side` to `visible` unless the voxel beyond it is solid.
    ///
    /// Its own function rather than a branch inside the loop above only because
    /// `clippy.toml` allows two levels of nesting and a loop around a condition
    /// is already three.
    fn keep_if_visible(
        &self,
        voxel: LocalPos,
        side: Side,
        visible: &mut Vec<Side>,
    ) -> Result<(), SectionError> {
        if !self.is_solid_beyond(voxel, side)? {
            visible.push(side);
        }
        Ok(())
    }

    /// Whether the voxel one step off `side` of `voxel` is solid.
    ///
    /// A step that leaves the section is answered by the neighbour beyond that
    /// side, at the mirrored coordinate; an absent neighbour answers non-solid.
    fn is_solid_beyond(&self, voxel: LocalPos, side: Side) -> Result<bool, SectionError> {
        match step_from(voxel, side) {
            Adjacent::Inside(pos) => self.section.is_solid_at(pos, self.registry),
            Adjacent::Across(pos) => match self.neighbourhood.beyond(side) {
                Some(neighbour) => neighbour.is_solid_at(pos, self.registry),
                None => Ok(false),
            },
        }
    }
}

/// Where the voxel one side of another one lives: still in this section, or in
/// the neighbour beyond that side, at the position named within that neighbour.
enum Adjacent {
    Inside(LocalPos),
    Across(LocalPos),
}

/// Where a step off `side` of `voxel` lands.
fn step_from(voxel: LocalPos, side: Side) -> Adjacent {
    let (along_x, along_y, along_z) = side.step();
    let (x, left_on_x) = stepped(voxel.x, along_x);
    let (y, left_on_y) = stepped(voxel.y, along_y);
    let (z, left_on_z) = stepped(voxel.z, along_z);
    let landed = LocalPos { x, y, z };
    if left_on_x || left_on_y || left_on_z {
        return Adjacent::Across(landed);
    }
    Adjacent::Inside(landed)
}

/// One axis of a step: where it lands, and whether it left the section to get
/// there.
///
/// Leaving at one end lands at the other, which is what "the mirrored coordinate
/// inside the neighbour" means — a step off the low face at 0 lands on the high
/// face at 15 of the section below.
const fn stepped(coordinate: u32, along: i32) -> (u32, bool) {
    let moved = coordinate as i32 + along;
    if moved < 0 {
        return (SECTION_SIZE - 1, true);
    }
    if moved >= SECTION_SIZE as i32 {
        return (0, true);
    }
    (moved as u32, false)
}

/// Every position a section has, x fastest, then y, then z.
fn every_position() -> impl Iterator<Item = LocalPos> {
    (0..SECTION_SIZE).flat_map(|z| {
        (0..SECTION_SIZE).flat_map(move |y| (0..SECTION_SIZE).map(move |x| LocalPos { x, y, z }))
    })
}
