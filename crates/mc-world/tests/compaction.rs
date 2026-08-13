//! What compaction gives back, and what it is not allowed to change.
//!
//! A palette entry outlives the last voxel that referred to it, because reclaiming
//! it on the edit path would put work into a 20 Hz tick shared by 32 players for a
//! benefit only meshing and persistence ever collect. Compaction is where that
//! debt is paid, and the whole of its contract is that paying it is invisible: a
//! section holds exactly the same block at exactly the same 4096 positions
//! afterwards, and only its palette and its index width got smaller.
//!
//! The interesting case is not the one where the last entry is dropped. It is the
//! one where a *middle* entry goes, so that every surviving entry moves to a new
//! position and every voxel index has to move with it. An implementation that
//! drops entries without renumbering passes the easy case and silently reports the
//! wrong block in the hard one, which is why the hard one is here.

mod common;

use std::error::Error;

use common::{
    NOTHING, TestResult, all_positions, at, contents_at_every_position, described, nth_position,
    registry_of,
};
use mc_core::id::BlockName;
use mc_world::section::{LocalPos, Section};

const STONE: &str = "base:stone";
const GRASS: &str = "base:grass";
const DIRT: &str = "base:dirt";

/// How many voxels the vacated-entry fixture writes and then writes back.
const REVISITED: u32 = 10;

/// The one cell the emptying scenarios write to and empty again.
const A_WRITTEN_CELL: LocalPos = at(3, 4, 5);

/// What a section's palette holds, in the order it holds them — a block by name,
/// and [`NOTHING`] for the entry that names none.
fn palette_names(section: &Section) -> Vec<String> {
    section.palette().map(described).collect()
}

/// How many of the blocks in `held` are `block`.
fn count_of(held: &[String], block: &str) -> usize {
    held.iter().filter(|entry| entry.as_str() == block).count()
}

/// A stone section in which ten voxels held grass and then held stone again.
///
/// Grass is still in the palette — nothing removes an entry on the write path —
/// and no voxel refers to it any more, which is the only state compaction has
/// anything to do.
fn section_with_a_vacated_entry() -> Result<Section, Box<dyn Error>> {
    let registry = registry_of(&[STONE, GRASS, DIRT])?;
    let (stone, grass) = (BlockName::parse(STONE)?, BlockName::parse(GRASS)?);
    let mut section = Section::filled(&stone, &registry)?;
    for voxel in 0..REVISITED {
        section.set_block(nth_position(voxel), &grass, &registry)?;
    }
    for voxel in 0..REVISITED {
        section.set_block(nth_position(voxel), &stone, &registry)?;
    }
    Ok(section)
}

/// A stone section whose every voxel was overwritten with grass, and whose very
/// first voxel then took dirt.
///
/// Stone is vacated and it is the *first* palette entry, so grass and dirt both
/// have to move down a position and all 4096 voxel indices have to follow them.
fn section_whose_fill_was_replaced_twice() -> Result<Section, Box<dyn Error>> {
    let registry = registry_of(&[STONE, GRASS, DIRT])?;
    let mut section = Section::filled(&BlockName::parse(STONE)?, &registry)?;
    let grass = BlockName::parse(GRASS)?;
    for position in all_positions() {
        section.set_block(position, &grass, &registry)?;
    }
    section.set_block(at(0, 0, 0), &BlockName::parse(DIRT)?, &registry)?;
    Ok(section)
}

#[test]
fn compacting_reclaims_an_entry_no_voxel_refers_to_any_more() -> TestResult {
    let mut section = section_with_a_vacated_entry()?;

    section.compact();

    assert_eq!(
        (palette_names(&section), section.index_width_bits()),
        (vec![STONE.to_owned()], 0),
        "the ten voxels that briefly held grass hold stone again, so grass is referred to \
         by nothing; reclaiming it leaves one distinct block, and a section with one \
         distinct block has nothing to tell its voxels apart with"
    );
    Ok(())
}

#[test]
fn a_reclaimed_entry_leaves_every_voxel_holding_the_block_it_already_held() -> TestResult {
    let mut section = section_with_a_vacated_entry()?;

    section.compact();

    let held = contents_at_every_position(&section)?;
    assert_eq!(
        (held.len(), count_of(&held, STONE)),
        (4096, 4096),
        "compaction is an internal saving and never an edit: giving back the space grass \
         left behind may not change what a single voxel answers with"
    );
    Ok(())
}

#[test]
fn compacting_renumbers_the_entries_that_survive_and_every_voxel_with_them() -> TestResult {
    let mut section = section_whose_fill_was_replaced_twice()?;

    section.compact();

    let held = contents_at_every_position(&section)?;
    assert_eq!(
        (
            palette_names(&section),
            described(section.block_at(at(0, 0, 0))?),
            count_of(&held, GRASS),
            count_of(&held, STONE),
        ),
        (
            vec![GRASS.to_owned(), DIRT.to_owned()],
            DIRT.to_owned(),
            4095,
            0,
        ),
        "the fill was the first palette entry and nothing refers to it now, so the two \
         entries that survive move down and keep their relative order — and every voxel \
         index has to be rewritten to follow them, or the section starts reporting \
         whichever block happens to sit at the old number"
    );
    Ok(())
}

#[test]
fn compacting_a_section_with_nothing_to_reclaim_changes_nothing() -> TestResult {
    let registry = registry_of(&[STONE, GRASS, DIRT])?;
    let mut section = Section::filled(&BlockName::parse(STONE)?, &registry)?;
    let grass = BlockName::parse(GRASS)?;
    for voxel in 0..REVISITED {
        section.set_block(nth_position(voxel), &grass, &registry)?;
    }
    section.set_block(nth_position(REVISITED), &BlockName::parse(DIRT)?, &registry)?;
    let before = (
        contents_at_every_position(&section)?,
        section.palette().len(),
        section.index_width_bits(),
    );

    section.compact();

    let after = (
        contents_at_every_position(&section)?,
        section.palette().len(),
        section.index_width_bits(),
    );
    assert_eq!(
        (
            after.0 == before.0,
            (before.1, after.1),
            (before.2, after.2)
        ),
        (true, (3, 3), (2, 2)),
        "stone, grass and dirt are each held by at least one voxel, so there is nothing \
         to give back: compaction never narrows away an entry something still refers to, \
         and never re-packs the indices it did not have to"
    );
    Ok(())
}

#[test]
fn compacting_after_every_cell_holding_a_block_was_emptied_stops_offering_that_block() -> TestResult
{
    let registry = registry_of(&[STONE, GRASS, DIRT])?;
    let mut section = Section::empty();
    section.set_block(A_WRITTEN_CELL, &BlockName::parse(STONE)?, &registry)?;
    section.empty_at(A_WRITTEN_CELL)?;

    section.compact();

    assert_eq!(
        palette_names(&section),
        vec![NOTHING.to_owned()],
        "the one cell that held stone holds nothing again, so stone is referred to by nothing \
         and compaction is what gives its entry back. The entry that survives is the one that \
         names no block at all, which is an ordinary entry to compaction — it counts references \
         and has no opinion about what sits at a position"
    );
    Ok(())
}

#[test]
fn a_section_whose_every_cell_was_emptied_still_holds_nothing_everywhere_after_compaction()
-> TestResult {
    let registry = registry_of(&[STONE, GRASS, DIRT])?;
    let mut section = Section::filled(&BlockName::parse(STONE)?, &registry)?;
    for position in all_positions() {
        section.empty_at(position)?;
    }

    section.compact();

    let held = contents_at_every_position(&section)?;
    assert_eq!(
        (held.len(), count_of(&held, NOTHING)),
        (4096, 4096),
        "the fill was the first palette entry and nothing refers to it now, so the entry that \
         names no block moves down to position 0 and every voxel index has to be rewritten to \
         follow it. Narrowing the palette without renumbering leaves 4096 voxels naming a \
         position that is no longer there"
    );
    Ok(())
}

/// Guard, and it belongs to this phase because the phase before it said so. A
/// freshly filled section owns no index buffer at all — its fill sits at palette
/// position 0 and every voxel index is already 0 — and compaction is the one
/// operation that can put a section back into that shape with a *different* block
/// in it. An implementation that narrows to nothing without moving the survivor
/// down to position 0 either refuses silently or answers with the entry that used
/// to be there, and no scenario in the specification combines the two halves of
/// that: one narrows to nothing with the survivor already at position 0, the other
/// moves survivors down but stops at one bit.
#[test]
fn a_section_whose_fill_was_wholly_replaced_still_reads_back_after_narrowing_to_nothing()
-> TestResult {
    let registry = registry_of(&[STONE, GRASS, DIRT])?;
    let mut section = Section::filled(&BlockName::parse(STONE)?, &registry)?;
    let grass = BlockName::parse(GRASS)?;
    for position in all_positions() {
        section.set_block(position, &grass, &registry)?;
    }

    section.compact();

    let held = contents_at_every_position(&section)?;
    assert_eq!(
        (
            palette_names(&section),
            section.index_width_bits(),
            count_of(&held, GRASS)
        ),
        (vec![GRASS.to_owned()], 0, 4096),
        "the only block left was never the one at palette position 0, and a section with \
         no index buffer can only answer with position 0 — so the survivor has to be \
         moved there rather than merely kept"
    );
    Ok(())
}
