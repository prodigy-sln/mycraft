//! What a declaration that holds nobody up resolves to, however loudly it
//! declares an ascent.
//!
//! **This is the root cause where the physics is the symptom.** A swimmer beside
//! a plant that declares an ascent it cannot deliver rises correctly under two
//! different repairs, and only one of them is free: masking the ascent where the
//! *fold* joins two media leaves `{not swimmable, no resistance, an enormous
//! ascent}` in the table as a distinct entry for every ordinary block, which
//! widens the index every voxel in the world carries. Asking what the
//! declaration *resolves to* separates the two, where asking what the player
//! does cannot.
//!
//! **The index and not the value.** Two media that compare equal share one table
//! entry, and sharing the entry is the whole of what keeps the table narrow — so
//! the reading taken here is the index a definition resolves to, which is the
//! same question the width scenario asks one step further out.

mod support;

use mc_core::id::BlockName;
use mc_sim::replay::{MediumIndex, ResolvedVoxels};

use support::TestResult;
use support::medium::{
    ABSURD_ASCENT, HOLDS_NOBODY_UP, LIFTING, LIFTING_ASCENT, hollow, media_registry,
};

#[test]
fn a_block_nobody_can_be_held_up_in_resolves_to_the_medium_a_cell_holding_nothing_does()
-> TestResult {
    let registry = media_registry()?;
    let view = ResolvedVoxels::resolve(&hollow(), &registry)?;

    let holds_nobody_up =
        view.medium_index_of(registry.resolve(&BlockName::parse(HOLDS_NOBODY_UP)?)?);
    let lifting = view.medium_index_of(registry.resolve(&BlockName::parse(LIFTING)?)?);

    assert_ne!(
        lifting,
        MediumIndex::NOTHING,
        "the control: a block that does hold a swimmer up and declares {LIFTING_ASCENT} resolves \
         to a medium of its own, so this view is telling declarations apart rather than answering \
         one index for every one of them"
    );
    assert_eq!(
        holds_nobody_up,
        MediumIndex::NOTHING,
        "a volume holding nobody up is indistinguishable from nothing at all in what it does to a \
         swimmer, so a declaration stating no solidity, no buoyancy and an ascent of \
         {ABSURD_ASCENT} resolves to the very index an empty cell carries — sharing its table \
         entry rather than minting one, which is what keeps a declared ascent costing a voxel \
         nothing. Resolved to {holds_nobody_up:?}"
    );
    Ok(())
}
