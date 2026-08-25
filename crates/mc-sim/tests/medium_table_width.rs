//! How many bits a resolved view spends per voxel on what medium that voxel is.
//!
//! The medium is one packed index into a table of the distinct answers a
//! *registry* declares, so the width is a property of content and not of the
//! engine: the shipped game declares four blocks, at most one of which states
//! either medium field, so a voxel has two answers to choose between and the
//! whole of both medium questions costs one bit — 128 KiB over the shipped
//! world's 1 048 576 voxels, half the 256 KiB a resolved view is budgeted. Two
//! bits is where that budget is exactly spent, and it is the ceiling asserted
//! here.
//!
//! **The second test is the first one's positive control and is not optional.**
//! An assertion that a number stays under a ceiling goes green forever the day
//! the accessor comes to return a constant, so a registry declaring five
//! distinct media is asserted to report a *wider* width than the shipped one —
//! which no constant can satisfy.
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
        "the blocks this game ships answer at most two distinct media between them, so a voxel's \
         medium index fits in one bit and must not exceed {SHIPPED_CEILING} — {width} bits over \
         a million voxels is past the whole budget a resolved view has for one answer"
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
