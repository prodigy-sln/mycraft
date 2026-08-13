//! A world coordinate past an edge is refused, and never folded back inside.
//!
//! The world is addressed in unsigned world coordinates, and every axis of it is
//! a power of two blocks wide — a footprint of whole chunk columns, a column of
//! whole sections. That is exactly the shape in which "split the coordinate into
//! a column and a position inside it" is a shift and a mask, and a mask **cannot
//! fail**: a coordinate one past the far edge masks straight back to the near
//! one, and a write meant for outside the world lands silently on a cell that is
//! inside it.
//!
//! So the refusal is asserted twice over in one test: that the write comes back
//! refused, naming the position it refused, and that the cell an index which
//! wrapped would have landed on still holds what the world was filled with. The
//! second half is the one a masking implementation fails — it refuses nothing, so
//! the first half alone would be measuring an error path nobody reached.

mod common;

use std::error::Error;

use common::{TestResult, described, registry_declaring};
use mc_core::block::BlockRegistry;
use mc_core::id::BlockName;
use mc_world::world::{Extent, VoxelWorld, WorldError, WorldPos};

/// The two blocks the fixture world is declared with.
///
/// Named for the fixture rather than for content, and given opposite solidity so
/// that nothing here can be satisfied by a world that recognised a name.
const FILL: &str = "fixture:fill";
const OTHER: &str = "fixture:other";

/// How many chunk columns the fixture world spans on each axis.
const COLUMNS: u32 = 1;

/// The cell every out-of-range position below would fold back onto, were its
/// index masked into range instead of refused.
const WOULD_WRAP_ONTO: WorldPos = WorldPos { x: 0, y: 0, z: 0 };

#[test]
fn a_write_past_any_edge_is_refused_and_never_wraps_into_the_world() -> TestResult {
    let registry = registry_declaring(&[(FILL, false), (OTHER, true)])?;
    let mut world = VoxelWorld::filled(COLUMNS, &BlockName::parse(FILL)?, &registry)?;
    let past_an_edge = one_past_each_edge(world.extent());

    let refused = refusals(&mut world, &past_an_edge, &registry)?;

    assert_eq!(
        (refused, described(world.block_at(WOULD_WRAP_ONTO)?)),
        (past_an_edge.map(Some).to_vec(), FILL.to_owned()),
        "one step past each of the three far edges is outside the world on that axis, and each \
         has to come back refused naming the position it was asked about. The cell at the origin \
         is where all three of them land if the index is masked into range instead — an edit \
         meant for outside the world, written silently onto the near side of it, with no refusal \
         reported and nothing else in the path to notice"
    );
    Ok(())
}

/// One step past the far edge on each axis in turn.
fn one_past_each_edge(extent: Extent) -> [WorldPos; 3] {
    [
        WorldPos {
            x: extent.x,
            y: 0,
            z: 0,
        },
        WorldPos {
            x: 0,
            y: extent.y,
            z: 0,
        },
        WorldPos {
            x: 0,
            y: 0,
            z: extent.z,
        },
    ]
}

/// The position each write was refused with, or nothing where a write was not
/// refused as an out-of-world position at all.
fn refusals(
    world: &mut VoxelWorld,
    at: &[WorldPos],
    registry: &BlockRegistry,
) -> Result<Vec<Option<WorldPos>>, Box<dyn Error>> {
    let other = BlockName::parse(OTHER)?;
    Ok(at
        .iter()
        .map(
            |position| match world.set_block(*position, &other, registry) {
                Err(WorldError::OutsideWorld { at: reported }) => Some(reported),
                _ => None,
            },
        )
        .collect())
}
