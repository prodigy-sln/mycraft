//! A voxel's solidity written after the fact lands where the offset names it and
//! nowhere else.
//!
//! The bitset resolved once at construction is what the physics reads, and an
//! editable world writes it beside the block store on every edit. A write that
//! transposed two axes, or that landed in the wrong word of the packed bitset,
//! would put the collision view out of step with the store while every scenario
//! about a *declared* world stayed green — the fixture would still be the fixture
//! and the physics would still be reading a bitset, just not the one the edit
//! meant.
//!
//! The fixture is deliberately not a cube of equal coordinates. The two positions
//! written below have all three coordinates different from each other, so an
//! exchange of any two axes lands on a position this assertion reports rather than
//! on the one it asked for.

mod support;

use mc_sim::player::{BlockPos, Solidity};
use mc_sim::replay::{Extent, SolidVoxels};
use mc_world::world::WorldPos;

use support::TestResult;
use support::volume::{NamedSlab, registry_declaring};

/// The two blocks the volume is declared with, and their solidity.
///
/// Named for what they are rather than for anything content ships, because what
/// this asserts is that solidity comes from the definition beside the name.
const PACKED: &str = "fixture:packed";
const HOLLOW: &str = "fixture:hollow";

/// How far the declared volume reaches on each axis.
///
/// Small, and the same on each axis only because the *positions* written below
/// are what has to be asymmetric, not the box around them.
const EXTENT: Extent = Extent { x: 4, y: 4, z: 4 };

/// The highest layer of the volume that holds the solid block.
const TOP: u32 = 1;

/// A position above the slab, which is not solid until it is written.
const RAISED: WorldPos = WorldPos { x: 1, y: 3, z: 2 };

/// A position inside the slab, which is solid until it is written.
const HOLLOWED: WorldPos = WorldPos { x: 3, y: 0, z: 1 };

#[test]
fn setting_one_voxels_solidity_changes_that_voxel_and_no_other() -> TestResult {
    let registry = registry_declaring(&[(PACKED, true), (HOLLOW, false)])?;
    let mut solid = SolidVoxels::resolve(&NamedSlab::new(EXTENT, TOP, PACKED, HOLLOW)?, &registry)?;
    let before: Vec<bool> = every_position().map(|at| solid.is_solid(at)).collect();

    solid.set(RAISED, true);
    solid.set(HOLLOWED, false);

    let flipped: Vec<BlockPos> = every_position()
        .zip(before)
        .filter(|(at, was)| solid.is_solid(*at) != *was)
        .map(|(at, _)| at)
        .collect();
    assert_eq!(
        flipped,
        vec![signed(HOLLOWED), signed(RAISED)],
        "each write flips the voxel its own position names and leaves every other voxel of the \
         volume answering what it answered before. The two positions have no coordinate in \
         common, so a write that exchanged two axes or that addressed the wrong word of the \
         packed bitset reports a position here instead of these two"
    );
    Ok(())
}

/// Every position inside the declared volume, in the order the bitset numbers
/// them: x fastest, then z, then y.
fn every_position() -> impl Iterator<Item = BlockPos> {
    (0..EXTENT.y).flat_map(|y| {
        (0..EXTENT.z).flat_map(move |z| (0..EXTENT.x).map(move |x| signed(WorldPos { x, y, z })))
    })
}

/// A volume position as the signed one the physics asks about.
fn signed(at: WorldPos) -> BlockPos {
    BlockPos {
        x: at.x as i32,
        y: at.y as i32,
        z: at.z as i32,
    }
}
