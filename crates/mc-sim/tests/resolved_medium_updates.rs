//! A medium written after the fact lands in the slot its offset names, in a view
//! that spends more than one bit on each of them.
//!
//! `resolved_voxel_updates.rs` asks the same question of the two bitset views,
//! and it cannot ask this one: every block its registry declares states no
//! medium, so its table holds one answer and its medium view is one bit wide.
//! That is the degenerate case — a value fills its slot exactly, and a write has
//! nothing beside it to preserve.
//!
//! **At any wider width a write is a read, a mask and a write back**, and the
//! arithmetic deciding which bits of which word it touches is reached by nothing
//! else in the suite: the packing walk builds a whole array at once and never
//! edits one, and every other view is a single bit. **Measured rather than
//! reasoned** — taking the slot within a word at the one-bit mask instead of at
//! the view's own left `mc-sim` and `mc-client` entirely green, 560 of 560, so a
//! defect there shipped with no witness at all. This file is that witness.
//!
//! **So the two positions written below sit in different words**, and every
//! voxel of the volume is read back afterwards. A write that landed in the wrong
//! slot clobbers a neighbour sharing its word and puts a third row in the list;
//! one that landed in the wrong word moves a voxel sixteen away from the one it
//! was asked about; and one that computed the slot as though the view were one
//! bit wide reaches past the end of a word entirely.
//!
//! The two positions have no coordinate in common and none of their own three
//! alike, so an exchange of any two axes lands on a position this assertion
//! reports rather than on the one it asked for. They are written to the same
//! medium from **different** starting ones — one cell holds a block already, the
//! other holds nothing — so a write that ignored its argument and left each cell
//! as it found it drops a row rather than passing.
//!
//! The other two answers are written to what each cell already held, so every
//! row in the list below is a medium that moved and nothing else.

mod support;

use std::error::Error;

use mc_core::id::BlockName;
use mc_sim::player::{BlockPos, Medium};
use mc_sim::replay::{Extent, ResolvedVoxels, VoxelAnswers};
use mc_world::world::WorldPos;

use support::TestResult;
use support::medium::{ONE_RESISTANCE, SLOWED_ONCE, SLOWED_THRICE, registry_of_many_media};
use support::volume::Cells;

/// How far the declared volume reaches on each axis.
///
/// Sixty-four voxels, which at the four bits five distinct media need is four
/// words of sixteen — enough for the two positions below to sit in different
/// ones and for each to have fifteen neighbours a careless write could reach.
const EXTENT: Extent = Extent { x: 4, y: 4, z: 4 };

/// The rows the volume holds a block in, exclusive: the lowest layer only.
const HELD_TOP: u32 = 1;

/// A position in that lowest layer, which holds [`SLOWED_ONCE`] until it is
/// written. Word 0 of the packed array.
const STEEPED: WorldPos = WorldPos { x: 1, y: 0, z: 2 };

/// A position above it, which holds no block at all until it is written. Word 2
/// of the packed array, so the two writes cannot share one word.
const HOLLOW: WorldPos = WorldPos { x: 3, y: 2, z: 1 };

/// What one voxel answers about its medium, as a comparable value.
///
/// The resistance as the integer its float is: "this voxel's medium did not
/// move" is a question about bits, which is both its exact form and the form
/// `clippy::float_cmp` has no quarrel with.
type Answer = (bool, u32);

/// The medium both positions are settled to, as this file compares media.
const SLOWED_THRICE_ANSWER: Answer = (false, (3.0f32).to_bits());

#[test]
fn writing_a_medium_into_a_view_wider_than_one_bit_moves_that_voxel_and_no_other() -> TestResult {
    let registry = registry_of_many_media()?;
    let volume = holding(SLOWED_ONCE)?;
    let mut resolved = ResolvedVoxels::resolve(&volume, &registry)?;
    let before: Vec<Answer> = every_position().map(|at| answer(&resolved, at)).collect();
    let settled = VoxelAnswers {
        solid: false,
        targetable: false,
        medium: resolved.medium_index_of(registry.resolve(&BlockName::parse(SLOWED_THRICE)?)?),
    };

    resolved.set(STEEPED, settled);
    resolved.set(HOLLOW, settled);

    let moved: Vec<(BlockPos, Answer)> = every_position()
        .zip(before)
        .filter(|(at, was)| answer(&resolved, *at) != *was)
        .map(|(at, _)| (at, answer(&resolved, at)))
        .collect();
    assert_eq!(
        moved,
        vec![
            (signed(STEEPED), SLOWED_THRICE_ANSWER),
            (signed(HOLLOW), SLOWED_THRICE_ANSWER)
        ],
        "each write settles the medium its own position names and leaves every other voxel of \
         the volume answering what it answered before. The two sit in different words of a \
         four-bit packing, so a write reaching the wrong slot clobbers a neighbour sharing its \
         word and reports a third row here, and one computing its slot as though the view were \
         one bit wide reaches past the word entirely. They are settled to one medium from \
         different starting ones, so a write that ignored its argument drops a row instead"
    );
    Ok(())
}

#[test]
fn the_view_these_writes_land_in_spends_more_than_one_bit_on_a_voxels_medium() -> TestResult {
    let resolved = ResolvedVoxels::resolve(&holding(SLOWED_ONCE)?, &registry_of_many_media()?)?;

    let width = resolved.medium_width_in_bits();

    assert!(
        width > 1,
        "the control its sibling cannot do without: at one bit a value fills its slot exactly \
         and a write has nothing beside it to preserve, so the slot arithmetic that test exists \
         for is not reached at all. This registry declares five distinct media and must resolve \
         wider than that — {width} bits"
    );
    Ok(())
}

/// A volume of [`EXTENT`] holding `block` up to but not including [`HELD_TOP`],
/// and nothing above it.
fn holding(block: &str) -> Result<Cells, Box<dyn Error>> {
    Cells::empty(EXTENT).holding(
        WorldPos { x: 0, y: 0, z: 0 },
        WorldPos {
            x: EXTENT.x,
            y: HELD_TOP,
            z: EXTENT.z,
        },
        block,
    )
}

/// What the medium view says about `at`.
fn answer(resolved: &ResolvedVoxels, at: BlockPos) -> Answer {
    let medium = resolved.medium_at(at);
    (medium.swimmable, medium.resistance.to_bits())
}

/// Every position inside the declared volume, in the order the packing numbers
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

/// Keeps the declared resistance of the block the lowest layer holds in view: a
/// starting medium equal to what is written would make the first position's row
/// vanish and the assertion above weaker by half.
const _: () = assert!(ONE_RESISTANCE != 3.0);
