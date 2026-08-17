//! Where a reload puts a player whose box it made solid.
//!
//! **The candidates are cell centres**, so the 0.6-wide box lies strictly inside
//! one cell column and clearance is a question about the two cells the 1.8-tall box
//! occupies rather than about four. A cleared player therefore loses their sub-block
//! position, which is why a player who needed no clearing is not moved at all rather
//! than moved to the middle of the cell they are already in.
//!
//! **Downward is absent from the candidate set rather than ranked last.** That makes
//! "never downward" a property of the set, so it survives any future reordering.
//!
//! **A candidate is eligible only if every cell its box would cover is *known* and
//! clear**, which is why the ground the search may consider is passed in beside the
//! solidity. `is_solid` answers `false` past the edge of the loaded world — because
//! nothing is there, not because it is clear — and the search reaches 8 blocks, so
//! without this a player trapped near an edge is put outside the world and falls out
//! of it. Outside is *unknown*, and a search over unknown ground is not a search.
//!
//! Outside is **not** read as solid instead: that is a claim the world model does not
//! have, `is_solid` is read by collision, meshing and the physics alike, and it
//! inverts the moment the world streams. Eligibility is sited here alone.

use glam::Vec3;
use mc_world::world::Extent;

use crate::player::{BlockPos, Solidity, collide};

use super::inside_the_world;

/// How far the search looks, in blocks, on each axis it looks along.
///
/// The cube is therefore `dx, dz ∈ [-8, 8]` with `dy ∈ [0, 8]` — 2 601 positions, a
/// deliberate narrowing under the spec's 17³ ceiling rather than a different bound.
const REACH: u32 = 8;

/// The same, as the offsets are counted.
const CELLS: i32 = REACH as i32;

/// A cell offset from the one the player is standing in.
type Offset = (i32, i32, i32);

/// What a swap did about the player, as a value rather than as an absence.
///
/// `NoClearSpaceWithin` is what lets "could not be cleared" be reported to a person
/// at all.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Clearing {
    /// No cell the player's box overlaps became solid.
    Unneeded,
    /// The player was put here, and their velocity was taken away with the move.
    MovedTo(Vec3),
    /// Nothing clear was found inside this many blocks.
    NoClearSpaceWithin { blocks: u32 },
}

/// Where a player whose feet are at `feet` should stand, given `world`'s solidity
/// over `ground` — the extent the search may consider, outside which the world is
/// unknown rather than clear.
///
/// Ordered `(dy, max(|dx|, |dz|), dz, dx)` ascending: `dy` first is what makes a
/// sideways cell win over an upward one at the same distance, and the last two are a
/// declared tie-break so two runs agree.
pub(crate) fn cleared(feet: Vec3, world: &dyn Solidity, ground: Extent) -> Clearing {
    if !collide::overlaps_solid(feet, world) {
        return Clearing::Unneeded;
    }
    let standing = collide::cell_of(feet);
    candidates()
        .map(|offset| centre_of(standing, offset))
        .find(|position| eligible(*position, world, ground))
        .map_or(
            Clearing::NoClearSpaceWithin { blocks: REACH },
            Clearing::MovedTo,
        )
}

/// Whether a player may be put at `position`: every cell their box would cover is
/// inside `ground` and none of them is solid.
///
/// The knowing comes first because it is what makes the clearance mean anything —
/// "not solid" about a cell the world does not hold is not an answer about ground.
fn eligible(position: Vec3, world: &dyn Solidity, ground: Extent) -> bool {
    known(position, ground) && !collide::overlaps_solid(position, world)
}

/// Whether the world holds every cell a player's box at `position` would cover.
fn known(position: Vec3, ground: Extent) -> bool {
    collide::covers(position).all(|cell| holds(ground, cell))
}

/// Whether `ground` holds `cell`, a negative coordinate naming nothing it holds.
fn holds(ground: Extent, cell: BlockPos) -> bool {
    inside_the_world(cell).is_some_and(|at| ground.contains(at))
}

/// Every offset the search may look at, in the declared order.
fn candidates() -> impl Iterator<Item = Offset> {
    (0..=CELLS).flat_map(rings_over)
}

/// The rings at one height, nearest first.
fn rings_over(dy: i32) -> impl Iterator<Item = Offset> {
    (0..=CELLS).flat_map(move |ring| ring_of(dy, ring))
}

/// One ring's offsets, in ascending `dz`.
fn ring_of(dy: i32, ring: i32) -> impl Iterator<Item = Offset> {
    (-CELLS..=CELLS).flat_map(move |dz| row_of(dy, ring, dz))
}

/// The offsets of one ring that lie on one `dz`, in ascending `dx`.
fn row_of(dy: i32, ring: i32, dz: i32) -> impl Iterator<Item = Offset> {
    (-CELLS..=CELLS)
        .filter(move |dx| dx.abs().max(dz.abs()) == ring)
        .map(move |dx| (dx, dy, dz))
}

/// Where a player standing in `standing` would stand `offset` away: the centre of
/// that cell, feet on its floor.
fn centre_of(standing: BlockPos, offset: Offset) -> Vec3 {
    let (across, up, along) = offset;
    Vec3::new(
        (standing.x + across) as f32 + 0.5,
        (standing.y + up) as f32,
        (standing.z + along) as f32 + 0.5,
    )
}
