//! What the replay's geometry is, checked against something that is not its
//! geometry.
//!
//! Everything else in this feature asserts the world's *voxels* on one side and
//! the *quads* on the other, and nothing ties the two together — which is the
//! gap that lets an all-stone world, or a world at the wrong heights, satisfy
//! every geometry assertion and then be captured into its own goldens. These
//! tests close it, and the order they are written in is binding: the area
//! assertions are the correctness ones and are never waived; the quad-count
//! snapshot at the end verifies nothing and says so.
//!
//! **No expected quantity here is copied from a run.** The two equalities
//! compare the mesher against an independent per-voxel walk that shares no code
//! with it. The four vertical figures are arithmetic over the declared world:
//! 64 by 64 columns with a heightmap and no overhangs shows one upward face per
//! column and one downward face per column, and the landmark's stone cap takes
//! exactly one of the upward ones off grass.
//!
//! **The sea's upward area is the fifth such figure, and it is counted from the
//! heightmap rather than written down.** Water fills every column whose surface
//! stands below the declared sea level, from one block above that surface up to
//! the sea level itself, and nothing is declared above the sea level anywhere —
//! so each such column shows exactly one upward water face and no other column
//! shows any. [`submerged_columns`] counts them, reading the surface heights and
//! the declared sea level and touching neither the mesher nor the walk. Water
//! shows no *downward* face at all for the same reason its neighbours cull it:
//! the cell below the lowest water in a column is that column's surface block,
//! which occludes, and the cell below any other water is water.
//!
//! The one committed number is `SCENE_QUAD_COUNT`, and it is committed as a
//! tripwire rather than as an oracle — see the test that reads it.

mod support;

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;

use mc_core::id::BlockName;
use mc_sim::replay::contract::{SCENE_QUAD_COUNT, SceneContract, scene_contract};
use mc_sim::replay::mesh_all;
use mc_world::mesh::Facing;

use support::oracle::{FaceArea, visible_face_area};
use support::{
    GRASS, SEA_LEVEL, STONE, TestResult, WATER, block_name, content_registry, replay_world,
    submerged_columns,
};

/// Upward face area belonging to grass: one top face per column of the 64 by 64
/// footprint, less the landmark column, whose surface block the declaration
/// overwrites with stone.
const GRASS_UPWARD: u64 = 4095;

/// Upward face area belonging to stone: the landmark's cap, and nothing else
/// reaches above its own column.
const STONE_UPWARD: u64 = 1;

/// Downward face area over the whole world: the floor at y = 0, whose neighbour
/// below is absent. Nothing above it has a non-solid voxel underneath, because
/// the surface is a heightmap and heightmaps have no overhangs.
const DOWNWARD: u64 = 4096;

#[test]
fn the_meshed_area_equals_an_independent_per_voxel_walk_of_the_world() -> TestResult {
    let (contract, walked) = meshed_and_walked()?;

    let total: u64 = walked.values().sum();

    assert!(
        total > 0,
        "the independent walk found no visible face at all, so every equality here would \
         hold over an empty world"
    );
    assert_eq!(
        contract.total_face_area, total,
        "merging changes how visible faces are grouped into rectangles and never which \
         faces are visible, so the summed area has to agree exactly"
    );
    Ok(())
}

#[test]
fn every_blocks_meshed_area_equals_the_independent_walks_area_for_that_block() -> TestResult {
    let (contract, walked) = meshed_and_walked()?;

    let by_block = area_by_block(&walked);

    assert_eq!(
        contract.area_by_block,
        by_block,
        "a world made of the wrong blocks agrees with its own geometry and disagrees \
         here — {}",
        first_disagreement(&contract.area_by_block, &by_block)
    );
    Ok(())
}

#[test]
fn the_surface_shows_one_upward_face_per_column_with_the_landmark_capping_one_and_the_sea_over_the_rest()
-> TestResult {
    let registry = content_registry()?;
    let world = replay_world(&registry)?;
    let submerged = submerged_columns(&world)?;
    let walked = visible_face_area(&world, &registry)?;

    let upward = towards(&walked, Facing::PosY);

    assert!(
        submerged > 0,
        "no column of the declared world stands below the sea level at {SEA_LEVEL}, so the \
         water entry below would be zero and every claim this test makes about the sea \
         would be a claim about nothing"
    );
    assert_eq!(
        upward,
        BTreeMap::from([
            (block_name(GRASS)?, GRASS_UPWARD),
            (block_name(STONE)?, STONE_UPWARD),
            (block_name(WATER)?, submerged),
        ]),
        "one top face per column, all grass but the landmark's stone cap — and one more per \
         submerged column, belonging to the water standing over it. The two ground figures \
         are what they were before water was drawn at all, and that is the point of them \
         here: water hides nothing, so the grass under the sea still shows the top face it \
         always did. A grass figure that came out short by exactly the {submerged} columns \
         the sea covers is water occluding what it stands on"
    );
    Ok(())
}

#[test]
fn the_world_floor_shows_one_downward_face_per_column() -> TestResult {
    let walked = walked_world()?;

    let downward: u64 = towards(&walked, Facing::NegY).values().sum();

    assert_eq!(
        downward, DOWNWARD,
        "the world floor is one downward face per column and there is nothing else \
         underhanging anywhere in a heightmap world"
    );
    Ok(())
}

/// A snapshot, deliberately not an oracle: it verifies nothing, and the tests
/// above are what say the geometry is right. Its only job is to fail on the day
/// the merge predicate moves — which is the day ambient occlusion arrives — so
/// that the failure lands here, before any image is compared, with the remedy in
/// its own message.
///
/// **It is also the only instrument that can see the merge shape over this
/// world.** A re-partition of the same visible faces is pixel-neutral: a corner's
/// texture coordinates come from its own position under a repeating sampler, so
/// one four-block quad shows the texture four times rather than stretched once,
/// and four one-block quads emit the same texels at the same depths. What pins
/// the shape itself is `mc-world`'s `mesh_properties`, whose three proptests hold
/// the quads to an exact partition of the visible faces, per face and per
/// position.
#[test]
fn the_meshed_quad_count_matches_the_committed_scene_contract_snapshot() -> TestResult {
    let (contract, _) = meshed_and_walked()?;

    assert_eq!(
        contract.quad_count, SCENE_QUAD_COUNT,
        "the replay's quad count moved, which means the way visible faces are grouped into \
         rectangles moved. This number verifies nothing on its own — the area assertions in \
         this file do that — so do not simply edit it. Which remedy is owed depends on what \
         moved, and the two are not the same. If the *visible faces* changed, every committed \
         golden is now a golden of a different scene: bump SCENE_REVISION, delete the previous \
         revision's golden directories, re-shoot under MYCRAFT_UPDATE_GOLDENS, and justify the \
         change in the commit. If only the *grouping* changed — the same faces cut into \
         different rectangles, which is what a new merge strategy does — the frames are \
         identical and re-shooting would churn the whole set to reproduce the same images: \
         confirm mc-world's mesh_properties still passes, then re-mint this number alone and \
         say in the commit why the merge moved"
    );
    Ok(())
}

/// The mesher's contract over the replay, and the independent walk of the same
/// world.
fn meshed_and_walked() -> Result<(SceneContract, FaceArea), Box<dyn Error>> {
    let registry = content_registry()?;
    let world = replay_world(&registry)?;
    let sections = mesh_all(&world, &registry)?;
    let walked = visible_face_area(&world, &registry)?;
    Ok((scene_contract(&sections), walked))
}

/// The independent walk of the replay world, on its own.
fn walked_world() -> Result<FaceArea, Box<dyn Error>> {
    let registry = content_registry()?;
    let world = replay_world(&registry)?;
    Ok(visible_face_area(&world, &registry)?)
}

/// The walk's area per block, summed over every direction.
fn area_by_block(walked: &FaceArea) -> BTreeMap<BlockName, u64> {
    let mut summed = BTreeMap::new();
    for ((block, _), area) in walked {
        *summed.entry(block.clone()).or_default() += area;
    }
    summed
}

/// The walk's area per block, in one direction only.
fn towards(walked: &FaceArea, facing: Facing) -> BTreeMap<BlockName, u64> {
    walked
        .iter()
        .filter(|((_, side), _)| *side == facing)
        .map(|((block, _), area)| (block.clone(), *area))
        .collect()
}

/// The first block the two sides disagree about, named.
fn first_disagreement(
    meshed: &BTreeMap<BlockName, u64>,
    walked: &BTreeMap<BlockName, u64>,
) -> String {
    let names: BTreeSet<&BlockName> = meshed.keys().chain(walked.keys()).collect();
    for name in names {
        let meshed_area = meshed.get(name).copied().unwrap_or_default();
        let walked_area = walked.get(name).copied().unwrap_or_default();
        if meshed_area != walked_area {
            return format!(
                "`{name}` meshes {meshed_area} and walks {walked_area}",
                name = name.as_str()
            );
        }
    }
    "no single block disagrees".to_owned()
}
