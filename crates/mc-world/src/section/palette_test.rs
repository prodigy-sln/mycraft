//! Guard. What a palette counts, and when.
//!
//! Compaction is only allowed to reclaim an entry that nothing refers to, and the
//! only honest way to know that is a count maintained as voxels are written.
//! Recounting from the voxel array at compaction time would make compaction come
//! out right even when the write path had been keeping the wrong numbers all along
//! — which is the very defect the scenarios about compaction exist to expose. So
//! the counts are asserted here, directly, because they have no public surface at
//! all and no behavioural test can see them.
//!
//! The third case below is the one worth reading twice. Overwriting a voxel with
//! the block it already holds must take the new reference *before* giving the old
//! one back. Done the other way round the entry passes through zero references —
//! the exact condition that means "nothing holds this any more" — and whatever
//! reads that condition, now or later, is entitled to act on it.

use mc_core::id::{BlockName, NamespacedIdError};

use super::Palette;
use crate::section::VOXELS_PER_SECTION;

/// Parsing the fixture names is the only fallible step in any guard here.
type GuardResult = Result<(), NamespacedIdError>;

const FILL: &str = "fixture:fill";
const WRITTEN: &str = "fixture:written";
const FURTHER: &str = "fixture:further";

/// A palette in the state a freshly filled section leaves one in: a single entry,
/// held by every voxel there is.
fn filled() -> Result<Palette, NamespacedIdError> {
    Ok(Palette::filled_with(
        &BlockName::parse(FILL)?,
        VOXELS_PER_SECTION,
    ))
}

#[test]
fn a_write_takes_a_reference_from_the_block_it_replaced() -> GuardResult {
    let mut palette = filled()?;

    let written = palette.replace(0, &BlockName::parse(WRITTEN)?);

    assert_eq!(
        (written, palette.refcount(0), palette.refcount(written)),
        (1, Some(VOXELS_PER_SECTION - 1), Some(1)),
        "one voxel of a filled section now holds something else, so the fill is held by \
         exactly one fewer voxel and the block that displaced it by exactly one"
    );
    Ok(())
}

#[test]
fn overwriting_the_last_voxel_holding_a_block_leaves_its_entry_unreferenced() -> GuardResult {
    let mut palette = filled()?;
    let written = palette.replace(0, &BlockName::parse(WRITTEN)?);

    let further = palette.replace(written, &BlockName::parse(FURTHER)?);

    assert_eq!(
        (
            palette.refcount(written),
            palette.refcount(further),
            palette.len()
        ),
        (Some(0), Some(1), 3),
        "the entry that overwrite vacated is now held by nothing, which is the one thing \
         compaction may reclaim by — and it stays in the palette until compaction does, \
         because dropping it here would renumber every entry above it in the middle of an \
         edit"
    );
    Ok(())
}

#[test]
fn overwriting_a_voxel_with_the_block_it_already_holds_keeps_its_entry_referenced() -> GuardResult {
    let mut palette = filled()?;
    let written = BlockName::parse(WRITTEN)?;
    let entry = palette.replace(0, &written);

    let again = palette.replace(entry, &written);

    assert_eq!(
        (again, palette.refcount(entry), palette.len()),
        (entry, Some(1), 2),
        "the reference has to be taken before the old one is given back: released first, \
         this entry would pass through zero references — indistinguishable from an entry \
         nothing holds — and come back out of the write either duplicated or reclaimable"
    );
    Ok(())
}
