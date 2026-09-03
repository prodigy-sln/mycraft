//! A voxel's resolved answers written after the fact land where the offset names
//! them and nowhere else.
//!
//! The views resolved once at construction are what the tick reads — one says
//! what stops the player, the other what a ray may stop at — and an editable
//! world writes both beside the block store on every edit. A write that
//! transposed two axes, or that landed in the wrong word of a packed bitset,
//! would put a view out of step with the store while every scenario about a
//! *declared* world stayed green: the fixture would still be the fixture and the
//! tick would still be reading a bitset, just not the one the edit meant.
//!
//! **There are two bitsets now, so there are two ways for that to be wrong and a
//! third way for them to disagree.** Nothing else in the suite can see any of
//! them: the scenarios about aiming write a cell and then read *that same cell*
//! back, so a pair of bitsets addressed wrongly in the same way answers them
//! correctly, and a write that settled one view from the other is invisible to
//! any reading that asks a cell only one question.
//!
//! **So the two positions are written to the *same* pair of answers and started
//! from different ones, and every voxel is read back through both views.** Both
//! are settled as something that stops a player and that no ray may stop at.
//! The raised one held neither of those before, so only the collision view may
//! move there; the hollowed one held both, so only the aiming view may move
//! there. A `set` that wrote its solidity into both bitsets moves the aiming
//! view at the raised position, one that wrote its targetability into both moves
//! nothing there at all, and one that ignored either argument leaves one of the
//! two positions out of the list entirely.
//!
//! **A write now settles a third answer, and this file deliberately does not
//! move it.** The write is one [`VoxelAnswers`], whose fields are named at the
//! call site so the pair below still reads as two answers deliberately unlike
//! each other. Every block the fixture registry declares states no medium, so
//! both positions already answer "no medium here" before anything is written and
//! [`MediumIndex::NOTHING`] is what leaves that true — which is what keeps this
//! file about the two views whose disagreement it was built to catch. A medium
//! that *did* move at one position would add a row to the list below that no
//! sentence here explains.
//!
//! The fixture is deliberately not a cube of equal coordinates. The two
//! positions written below have all three coordinates different from each other,
//! so an exchange of any two axes lands on a position this assertion reports
//! rather than on the one it asked for.

mod support;

use mc_sim::player::{BlockPos, Occluding, Solidity, Targetable};
use mc_sim::replay::{Extent, MediumIndex, ResolvedVoxels, VoxelAnswers};
use mc_world::world::WorldPos;

use support::TestResult;
use support::volume::{NamedSlab, registry_declaring};

/// The two blocks the volume is declared with, and their solidity.
///
/// Named for what they are rather than for anything content ships, because what
/// this asserts is that solidity comes from the definition beside the name.
const PACKED: &str = "fixture:packed";
const HOLLOW: &str = "fixture:hollow";

/// How far the declared volume reaches on each axis.
///
/// Small, and the same on each axis only because the *positions* written below
/// are what has to be asymmetric, not the box around them.
const EXTENT: Extent = Extent { x: 4, y: 4, z: 4 };

/// The highest layer of the volume that holds the solid block.
const TOP: u32 = 1;

/// A position above the slab, which neither stops a player nor stops a ray
/// until it is written.
const RAISED: WorldPos = WorldPos { x: 1, y: 3, z: 2 };

/// A position inside the slab, which does both until it is written.
const HOLLOWED: WorldPos = WorldPos { x: 3, y: 0, z: 1 };

/// What one voxel answers about the three `bool` questions asked of it: whether
/// the player is stopped by it, whether a ray may stop at it, and whether it
/// stops sight.
type Answers = (bool, bool, bool);

/// What both positions are settled as: an obstacle that blocks sight and that no
/// ray stops at.
///
/// One triple written at two positions that started from different ones, so that
/// each of the three views is the only one that may move at one of them, and
/// they are not the same view. The occlusion answer is what the position above
/// the slab moves on — it started answering that nothing is there — so a write
/// that settled two views and dropped the third lands that row on the wrong
/// triple rather than going unnoticed.
const AN_OBSTACLE_NO_RAY_STOPS_AT: Answers = (true, false, true);

/// A pair of answers as the one value a write takes, carrying the medium every
/// block this fixture's registry already answers.
///
/// The three views' answers stay a *triple* declared once and named here, rather
/// than fields spelled at the call site: what this file is about is that the
/// same triple reaches two positions that started from different ones, and a
/// second literal at the second position is a second place for that to drift.
const fn settling(answers: Answers) -> VoxelAnswers {
    let (solid, targetable, occludes) = answers;
    VoxelAnswers {
        solid,
        targetable,
        occludes,
        medium: MediumIndex::NOTHING,
    }
}

#[test]
fn setting_one_voxels_answers_changes_that_voxel_and_no_other() -> TestResult {
    let registry = registry_declaring(&[(PACKED, true), (HOLLOW, false)])?;
    let mut resolved =
        ResolvedVoxels::resolve(&NamedSlab::new(EXTENT, TOP, PACKED, HOLLOW)?, &registry)?;
    let before: Vec<Answers> = every_position().map(|at| answers(&resolved, at)).collect();

    let settled = settling(AN_OBSTACLE_NO_RAY_STOPS_AT);
    resolved.set(RAISED, settled);
    resolved.set(HOLLOWED, settled);

    let moved: Vec<(BlockPos, Answers)> = every_position()
        .zip(before)
        .filter(|(at, was)| answers(&resolved, *at) != *was)
        .map(|(at, _)| (at, answers(&resolved, at)))
        .collect();
    assert_eq!(
        moved,
        vec![
            (signed(HOLLOWED), AN_OBSTACLE_NO_RAY_STOPS_AT),
            (signed(RAISED), AN_OBSTACLE_NO_RAY_STOPS_AT)
        ],
        "each write settles the voxel its own position names and leaves every other voxel of the \
         volume answering what it answered before. The two positions have no coordinate in \
         common, so a write that exchanged two axes or that addressed the wrong word of a packed \
         bitset reports a position here instead of these two. They are settled to the same pair \
         from different starting answers, so the collision view is the only one that may move at \
         the raised position and the aiming view the only one that may move at the hollowed one — \
         a write that settled either view from the other's argument, or that ignored one of them, \
         lands one of these two rows on the wrong pair or drops it from the list"
    );
    Ok(())
}

/// What the three `bool` views say about `at`: whether the player is stopped by
/// it, whether a ray may stop at it, and whether it stops sight.
fn answers(resolved: &ResolvedVoxels, at: BlockPos) -> Answers {
    (
        resolved.is_solid(at),
        resolved.is_targetable(at),
        resolved.occludes(at),
    )
}

/// Every position inside the declared volume, in the order the bitset numbers
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
