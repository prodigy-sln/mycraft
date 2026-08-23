//! What the replay's geometry adds up to.
//!
//! Two different jobs live here and they are deliberately not the same number.
//! The **areas** are correctness: greedy merging changes how visible faces are
//! grouped into rectangles but never which faces are visible, so summed area is
//! a quantity an independent per-voxel walk of the world can be compared
//! against, and that comparison commits no number at all.
//!
//! [`SCENE_QUAD_COUNT`] is **change detection** and verifies nothing. Its whole
//! job is to fail on the day the merge predicate moves — the day ambient
//! occlusion arrives, per-vertex, narrowing merges and changing quad counts —
//! so that the failure lands here, before any image is compared, with the remedy
//! in the test's own message rather than as an inscrutable golden diff.
//!
//! **A count is not derivable the way an area is**, which is why the two jobs
//! cannot be made one. The mesher emits the scanline-greedy decomposition and
//! deliberately not the fewest rectangles covering the same faces, so the count
//! is the sweep's answer rather than the geometry's: a merger growing columns
//! before rows would be equally correct and report a different number for
//! identical geometry. An independent walk agreeing with it would have to repeat
//! those ordering choices and would be a copy of its subject.

use std::collections::BTreeMap;

use mc_core::id::BlockName;

use super::prepare::SectionQuads;

/// How many quads the declared replay meshes into.
///
/// A committed snapshot, not an oracle. It was minted only after the area
/// assertions in `crates/mc-sim/tests/scene_contract.rs` were green, and those
/// assertions — not this number — are what say the geometry is right. Editing it
/// to reach green is the one thing it must not be used for; see the failure
/// message on the test that reads it.
///
/// **A move here does not by itself mean the goldens are stale**, and the test's
/// message carries both branches. Merging is pixel-neutral today: texture
/// coordinates come from a corner's own position under a repeating sampler, so
/// the same faces cut into different rectangles produce the same frames. What a
/// moved count means is that the *grouping* changed; whether the *visible faces*
/// changed with it is what decides whether a re-shoot is owed, and the area
/// assertions are what answer that.
pub const SCENE_QUAD_COUNT: u32 = 2770;

/// What one meshing of the replay world came to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SceneContract {
    pub quad_count: u32,
    pub total_face_area: u64,
    pub area_by_block: BTreeMap<BlockName, u64>,
}

/// The contract `sections` add up to.
///
/// A quad's area is its two in-plane extents multiplied, which is the count of
/// voxel faces it covers — the quantity that survives a change to how faces are
/// grouped.
#[must_use]
pub fn scene_contract(sections: &[SectionQuads]) -> SceneContract {
    let mut contract = SceneContract {
        quad_count: 0,
        total_face_area: 0,
        area_by_block: BTreeMap::new(),
    };
    for quad in sections.iter().flat_map(|section| section.quads.iter()) {
        let area = u64::from(quad.extent.primary) * u64::from(quad.extent.secondary);
        contract.quad_count += 1;
        contract.total_face_area += area;
        *contract
            .area_by_block
            .entry(quad.block.clone())
            .or_default() += area;
    }
    contract
}
