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
use support::{GRASS, STONE, TestResult, block_name, content_registry, replay_world};

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
fn the_surface_shows_one_upward_face_per_column_and_the_landmark_caps_exactly_one() -> TestResult {
    let walked = walked_world()?;

    let upward = towards(&walked, Facing::PosY);

    assert_eq!(
        upward,
        BTreeMap::from([
            (block_name(GRASS)?, GRASS_UPWARD),
            (block_name(STONE)?, STONE_UPWARD),
        ]),
        "one top face per column, all grass but the landmark's stone cap; a world without \
         the landmark shows 4096 of grass and none of stone"
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
#[test]
fn the_meshed_quad_count_matches_the_committed_scene_contract_snapshot() -> TestResult {
    let (contract, _) = meshed_and_walked()?;

    assert_eq!(
        contract.quad_count, SCENE_QUAD_COUNT,
        "the replay's quad count moved, which means the mesh contract moved and every \
         committed golden is now a golden of a different scene. This number verifies \
         nothing on its own — the area assertions in this file do that — so do not simply \
         edit it. Bump SCENE_REVISION, delete the previous revision's golden \
         directories, re-shoot the goldens under MYCRAFT_UPDATE_GOLDENS, and justify the \
         change in the commit"
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
