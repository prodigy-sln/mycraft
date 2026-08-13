//! How wide a section's voxel indices are, and what that costs in memory.
//!
//! A world of mostly stone and air has to be affordable in RAM, so a section that
//! holds one block stores no indices at all and one that holds two spends a single
//! bit per voxel. The width steps up only when a new distinct block will not fit,
//! and every step is a boundary: 1, 2, 4, 16 and 256 entries are the last that
//! fit in 0, 1, 2, 4 and 8 bits, and 3, 5, 17 and 257 are the first that do not.
//! All nine are asserted, because a plausible-looking width table that is wrong at
//! two of the tiers passes any test set that only samples them.
//!
//! Every byte figure below is the one a section must actually report for 4096
//! voxels at that width. None of them is recomputed here from the width: a test
//! that derived the size the same way the implementation does would agree with it
//! however wrong both were.

mod common;

use std::error::Error;

use common::{
    TestResult, all_positions, at, contents_at_every_position, generated_block, nth_position,
    registry_of_size,
};
use mc_core::block::BlockRegistry;
use mc_world::section::{LocalPos, Section};

/// A section whose palette holds exactly `distinct` blocks, taken in order from
/// the generated blocks `registry` holds.
///
/// Each new block goes at its own position, so the palette grows by one every
/// time and the section ends up holding exactly `distinct` entries.
fn section_holding(distinct: u32, registry: &BlockRegistry) -> Result<Section, Box<dyn Error>> {
    let mut section = Section::filled(&generated_block(0)?, registry)?;
    for entry in 1..distinct {
        section.set_block(nth_position(entry), &generated_block(entry)?, registry)?;
    }
    Ok(section)
}

/// The index width in bits and the index storage in bytes a section reports, as
/// one value so that a scenario naming both asserts both at once.
fn width_and_storage(section: &Section) -> (u32, usize) {
    (section.index_width_bits(), section.index_storage_bytes())
}

#[test]
fn a_palette_of_one_block_needs_no_index_storage_at_all() -> TestResult {
    let registry = registry_of_size(1)?;

    let section = section_holding(1, &registry)?;

    assert_eq!(
        width_and_storage(&section),
        (0, 0),
        "a section holding one distinct block has nothing to tell its voxels apart with, \
         so it stores no indices and owns no buffer to store them in"
    );
    Ok(())
}

#[test]
fn a_palette_of_two_blocks_indexes_each_voxel_with_one_bit() -> TestResult {
    let registry = registry_of_size(2)?;

    let section = section_holding(2, &registry)?;

    assert_eq!(
        width_and_storage(&section),
        (1, 512),
        "one bit distinguishes two entries, and 4096 of them occupy 512 bytes"
    );
    Ok(())
}

#[test]
fn writing_a_third_distinct_block_widens_the_index_to_two_bits() -> TestResult {
    let registry = registry_of_size(3)?;
    let mut section = section_holding(2, &registry)?;

    section.set_block(nth_position(2), &generated_block(2)?, &registry)?;

    assert_eq!(
        section.index_width_bits(),
        2,
        "a third entry no longer fits in one bit, so the section widens to the next tier"
    );
    Ok(())
}

#[test]
fn a_palette_of_four_blocks_indexes_each_voxel_with_two_bits() -> TestResult {
    let registry = registry_of_size(4)?;

    let section = section_holding(4, &registry)?;

    assert_eq!(
        width_and_storage(&section),
        (2, 1024),
        "four entries are the most two bits address, and 4096 of them occupy 1024 bytes"
    );
    Ok(())
}

#[test]
fn writing_a_fifth_distinct_block_widens_the_index_to_four_bits() -> TestResult {
    let registry = registry_of_size(5)?;
    let mut section = section_holding(4, &registry)?;

    section.set_block(nth_position(4), &generated_block(4)?, &registry)?;

    assert_eq!(
        section.index_width_bits(),
        4,
        "a fifth entry does not fit in two bits, and the next tier is four rather than \
         eight — a section of five blocks pays half of what a byte per voxel would cost"
    );
    Ok(())
}

#[test]
fn a_palette_of_sixteen_blocks_indexes_each_voxel_with_four_bits() -> TestResult {
    let registry = registry_of_size(16)?;

    let section = section_holding(16, &registry)?;

    assert_eq!(
        width_and_storage(&section),
        (4, 2048),
        "sixteen entries are the most four bits address, and 4096 of them occupy 2048 bytes"
    );
    Ok(())
}

#[test]
fn a_palette_of_seventeen_blocks_indexes_each_voxel_with_eight_bits() -> TestResult {
    let registry = registry_of_size(17)?;

    let section = section_holding(17, &registry)?;

    assert_eq!(
        section.index_width_bits(),
        8,
        "a seventeenth entry does not fit in four bits"
    );
    Ok(())
}

#[test]
fn a_palette_of_two_hundred_and_fifty_six_blocks_indexes_each_voxel_with_eight_bits() -> TestResult
{
    let registry = registry_of_size(256)?;

    let section = section_holding(256, &registry)?;

    assert_eq!(
        width_and_storage(&section),
        (8, 4096),
        "256 entries are the most eight bits address, and one byte per voxel is 4096 bytes"
    );
    Ok(())
}

#[test]
fn a_palette_of_two_hundred_and_fifty_seven_blocks_indexes_each_voxel_with_sixteen_bits()
-> TestResult {
    let registry = registry_of_size(257)?;

    let section = section_holding(257, &registry)?;

    assert_eq!(
        width_and_storage(&section),
        (16, 8192),
        "a 257th entry does not fit in a byte, and 4096 two-byte indices occupy 8192 bytes"
    );
    Ok(())
}

#[test]
fn widening_the_index_leaves_every_untargeted_voxel_holding_what_it_held() -> TestResult {
    const TARGET: LocalPos = at(8, 8, 8);
    let registry = registry_of_size(3)?;
    let mut section = section_holding(2, &registry)?;
    let width_before = section.index_width_bits();
    let held_before = contents_at_every_position(&section)?;

    section.set_block(TARGET, &generated_block(2)?, &registry)?;

    let held_after = contents_at_every_position(&section)?;
    let mut disagreement = None;
    for ((position, before), after) in all_positions().zip(&held_before).zip(&held_after) {
        if position != TARGET && before != after {
            disagreement = Some(format!(
                "({}, {}, {}) held `{before}` and now holds `{after}`",
                position.x, position.y, position.z
            ));
            break;
        }
    }
    assert_eq!(
        (
            width_before,
            section.index_width_bits(),
            disagreement.as_deref()
        ),
        (1, 2, None),
        "widening re-packs every voxel index into a wider form, and all 4095 voxels the \
         write did not name must come back out of it unchanged"
    );
    Ok(())
}

#[test]
fn writing_a_block_the_palette_already_holds_neither_lengthens_nor_widens_it() -> TestResult {
    const FURTHER_WRITES: u32 = 100;
    // Well clear of the positions `section_holding` used, so every one of these
    // writes genuinely changes a voxel rather than rewriting the same value.
    const FIRST_POSITION: u32 = 100;
    let registry = registry_of_size(2)?;
    let mut section = section_holding(2, &registry)?;
    let already_held = generated_block(1)?;
    let before = (section.palette().len(), section.index_width_bits());

    for write in 0..FURTHER_WRITES {
        section.set_block(
            nth_position(FIRST_POSITION + write),
            &already_held,
            &registry,
        )?;
    }

    assert_eq!(
        (
            before.0,
            before.1,
            section.palette().len(),
            section.index_width_bits()
        ),
        (2, 1, 2, 1),
        "a block already in the palette is found rather than appended, however many times \
         it is written — otherwise a section a player edits repeatedly grows without bound"
    );
    Ok(())
}
