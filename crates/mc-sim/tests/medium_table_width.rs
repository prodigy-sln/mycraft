//! How many bits a resolved view spends per voxel on what medium that voxel is.
//!
//! The medium is one packed index into a table of the distinct answers a
//! *registry* declares, so the width is a property of content and not of the
//! engine: the shipped game declares four blocks, at most one of which states
//! any medium field, so a voxel has two answers to choose between and the whole
//! of all three medium questions costs one bit — 128 KiB over the shipped
//! world's 1 048 576 voxels, half the 256 KiB a resolved view is budgeted. Two
//! bits is where that budget is exactly spent, and it is the ceiling asserted
//! here.
//!
//! **The ceiling and the one bit are two different claims and this file makes
//! both.** The `≤ 2` ceiling is a deliberate content budget: it is the right
//! instrument for "has content spent past what a voxel may afford", and
//! reddening the day a third distinct medium is legitimately declared is what it
//! is for. The `= 1` reading beside it is an enumerated verdict about the blocks
//! this game *ships*, and it is the one that reports a doubling rather than
//! absorbing it — a repair that mints a table entry for every ordinary block
//! spends exactly the ceiling's one bit of headroom, and nothing but an equality
//! can see that.
//!
//! **The last two tests are the first one's positive control and are not
//! optional.** An assertion that a number stays under a ceiling — or lands on a
//! value — goes green forever the day the accessor comes to return a constant,
//! so a registry declaring five distinct media is asserted to report a *wider*
//! width than the shipped one, which no constant can satisfy.
//!
//! **It is a second witness for free.** Its volume holds nothing at all, so a
//! width sized from the world's *contents* rather than from the registry sees
//! one answer, floors at one bit, and reddens — while the shipped reading, whose
//! world does hold water, would stay green under the same defect and report
//! nothing.

mod support;

use std::error::Error;

use mc_sim::replay::{Extent, ResolvedVoxels};

use support::medium::registry_of_many_media;
use support::volume::Cells;
use support::{TestResult, content_registry, replay_world};

/// The widest index the shipped registry may resolve to, in bits.
///
/// Derived rather than measured: two bits over 1 048 576 voxels is 256 KiB,
/// which is the whole of what a resolved view is budgeted for one answer.
const SHIPPED_CEILING: u32 = 2;

/// How wide the shipped registry's index actually is, in bits.
///
/// Derived rather than measured: no block this game ships but water states any
/// medium field, so the whole registry answers "no medium" or "water" and a
/// table of two entries indexes in one bit.
const SHIPPED_WIDTH: u32 = 1;

/// How far the synthetic registry's volume reaches. Small, and holding nothing:
/// the table is built from what a registry declares and never from what a world
/// happens to contain.
const NOWHERE_MUCH: Extent = Extent { x: 2, y: 2, z: 2 };

/// How wide the shipped registry's medium index is, resolved over the world the
/// shipped content generates.
fn shipped_width() -> Result<u32, Box<dyn Error>> {
    let registry = content_registry()?;
    let world = replay_world(&registry)?;
    Ok(ResolvedVoxels::resolve(&world, &registry)?.medium_width_in_bits())
}

#[test]
fn the_shipped_registry_spends_at_most_two_bits_a_voxel_on_what_medium_it_is() -> TestResult {
    let width = shipped_width()?;

    assert!(
        width <= SHIPPED_CEILING,
        "a voxel's medium index must not exceed {SHIPPED_CEILING} bits, which is the whole budget \
         a resolved view has for one answer over a million voxels — and this checks that ceiling \
         and nothing narrower. **The one-bit property is a separate claim and this is not its \
         guard**: it is held by the equality below, and at its root cause by \
         `non_lifting_medium.rs`, where a declaration holding nobody up is asserted to resolve to \
         the index an empty cell carries. Measured {width}"
    );
    Ok(())
}

#[test]
fn the_shipped_registry_spends_exactly_one_bit_a_voxel_on_what_medium_it_is() -> TestResult {
    let width = shipped_width()?;

    assert_eq!(
        width, SHIPPED_WIDTH,
        "no block this game ships but water states any medium field, so every other one resolves \
         to the same 'no medium here' an empty cell does and the table stays at two entries — \
         which is {SHIPPED_WIDTH} bit a voxel. Adding a declared medium property must cost a \
         voxel nothing: a wider reading means some ordinary block has come to resolve to a medium \
         of its own, and 128 KiB of the world went with it"
    );
    Ok(())
}

#[test]
fn a_registry_declaring_five_distinct_media_spends_more_bits_a_voxel_than_the_shipped_one()
-> TestResult {
    let shipped = shipped_width()?;

    let many = ResolvedVoxels::resolve(&Cells::empty(NOWHERE_MUCH), &registry_of_many_media()?)?
        .medium_width_in_bits();

    assert!(
        many > shipped,
        "the width is chosen from how many distinct media the registry declares, so a registry \
         declaring five of them indexes a voxel more widely than the shipped four do — {many} \
         bits against {shipped}. Equal widths mean either an accessor answering a constant, or a \
         table sized from what this volume happens to hold, which is nothing at all"
    );
    Ok(())
}
