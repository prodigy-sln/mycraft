//! How coplanar faces of the same facing become rectangles, and where merging
//! stops.
//!
//! The merge is the result of one fixed sweep and not "the fewest rectangles
//! that cover the same faces" — those are different answers, and only the first
//! of them is a single answer. So the sweep is pinned by fixtures rather than by
//! prose: a run grows along the primary axis first and is then extended along the
//! secondary axis while a whole row matches.
//!
//! The first fixture below is **transpose-asymmetric on purpose**. Run with the
//! two axis roles swapped, its footprint decomposes into a 3x3 rectangle and a
//! 2x1 one instead of a 4x2 and a 3x1, so a mesher that swapped primary for
//! secondary produces a visibly different list rather than the same one. An
//! earlier fixture for this scenario was symmetric under that swap and could not
//! have failed.
//!
//! Every scenario here constrains the +Y-facing quads and says nothing about the
//! other five facings, which are filtered out rather than asserted. Each of these
//! sections shows plenty of other faces — a layer sitting on nothing has an
//! underside, and every run of it has ends.

mod mesh_common;

use mc_world::mesh::{Facing, Neighbours, mesh_section};
use mc_world::section::LocalPos;
use mesh_common::{
    ALPHA, BETA, TestResult, VOID, at, blocks_towards, face, faces_towards, plain_registry,
    registry_declaring, scattered_solids, section_holding, single_face,
};

/// Where the two differently-blocked voxels sit in the palette a section is
/// built from: nothing at position 0, one block at 1 and the other at 2.
const HOLDS_VOID: u16 = 0;
const HOLDS_ALPHA: u16 = 1;
const HOLDS_BETA: u16 = 2;

/// One solid layer at the bottom of a section: two full rows of four and one
/// short row of three beside them.
///
/// Read as a footprint on the x/z plane, with x running across:
///
/// ```text
/// z = 0   # # # #
/// z = 1   # # # #
/// z = 2   # # #
/// ```
///
/// Swapping the roles of the two axes turns the same footprint into a 3x3 block
/// with a 2x1 tail, which is why this shape and not a symmetric one.
fn a_layer_of_two_full_rows_and_one_short_one(voxel: LocalPos) -> bool {
    if voxel.y != 0 {
        return false;
    }
    match voxel.z {
        0 | 1 => voxel.x < 4,
        2 => voxel.x < 3,
        _ => false,
    }
}

/// Two runs of three along x at the bottom of a section, with one non-solid voxel
/// between them.
///
/// ```text
/// z = 0   # # # . # # #
/// ```
fn two_runs_of_three_with_a_gap(voxel: LocalPos) -> bool {
    let along_the_row = voxel.y == 0 && voxel.z == 0;
    along_the_row && (voxel.x < 3 || (4..7).contains(&voxel.x))
}

/// Which palette entry each voxel of the two-block fixture holds: one block at
/// (0, 0, 0), a different one beside it at (1, 0, 0), and nothing solid anywhere
/// else.
fn one_block_beside_another(voxel: LocalPos) -> u16 {
    match (voxel.x, voxel.y, voxel.z) {
        (0, 0, 0) => HOLDS_ALPHA,
        (1, 0, 0) => HOLDS_BETA,
        _ => HOLDS_VOID,
    }
}

#[test]
fn a_run_extends_along_the_primary_axis_first_and_then_along_whole_rows() -> TestResult {
    let registry = plain_registry()?;
    let section = scattered_solids(a_layer_of_two_full_rows_and_one_short_one, &registry)?;

    let mesh = mesh_section(&section, &Neighbours::none(), &registry)?;

    assert_eq!(
        faces_towards(mesh.quads(), Facing::PosY),
        vec![
            face(Facing::PosY, 0, (0, 0), (4, 2)),
            face(Facing::PosY, 0, (0, 2), (3, 1)),
        ],
        "the sweep runs along x first — the primary axis of a ±Y face — and then extends that \
         run along z while a whole row matches. The two rows of four merge into one 4x2 \
         rectangle, and the row of three cannot join them because it is short, so it becomes a \
         3x1 of its own. A mesher that swapped the two roles covers the same faces with a 3x3 \
         and a 2x1 instead, which is the same area and a different list"
    );
    Ok(())
}

#[test]
fn two_neighbouring_faces_holding_different_blocks_stay_two_quads() -> TestResult {
    let registry = registry_declaring(&[(VOID, false), (ALPHA, true), (BETA, true)])?;
    let section = section_holding(&[VOID, ALPHA, BETA], one_block_beside_another, &registry)?;

    let mesh = mesh_section(&section, &Neighbours::none(), &registry)?;

    assert_eq!(
        faces_towards(mesh.quads(), Facing::PosY),
        vec![
            single_face(Facing::PosY, 0, (0, 0)),
            single_face(Facing::PosY, 0, (1, 0)),
        ],
        "these two faces are adjacent, coplanar and identically facing, and they still do not \
         merge, because a quad names one block and these voxels hold two. A merge predicate \
         reading only 'is there a face here' answers with a single 2x1 quad that would then be \
         drawn with one of the two textures across both blocks"
    );
    Ok(())
}

#[test]
fn two_same_block_faces_in_different_planes_stay_two_quads() -> TestResult {
    let registry = plain_registry()?;
    let section = scattered_solids(
        |voxel| voxel == at(0, 0, 0) || voxel == at(1, 1, 0),
        &registry,
    )?;

    let mesh = mesh_section(&section, &Neighbours::none(), &registry)?;

    assert_eq!(
        faces_towards(mesh.quads(), Facing::PosY),
        vec![
            single_face(Facing::PosY, 0, (0, 0)),
            single_face(Facing::PosY, 1, (1, 0)),
        ],
        "these two faces hold the same block and point the same way, and they are neighbours \
         when the plane is thrown away — (0, 0) and (1, 0) in the x/z footprint. They sit one \
         step apart in y, so they are on different planes and cannot be one rectangle. A sweep \
         that merged across its own plane would flatten a staircase into a floor"
    );
    Ok(())
}

#[test]
fn each_quad_names_the_block_the_voxels_under_it_hold() -> TestResult {
    let registry = registry_declaring(&[(VOID, false), (ALPHA, true), (BETA, true)])?;
    let section = section_holding(&[VOID, ALPHA, BETA], one_block_beside_another, &registry)?;

    let mesh = mesh_section(&section, &Neighbours::none(), &registry)?;

    assert_eq!(
        blocks_towards(mesh.quads(), Facing::PosY),
        vec![((0, 0), ALPHA.to_owned()), ((1, 0), BETA.to_owned()),],
        "a quad carries the name of the block every voxel under it holds, and the two here \
         differ. Without this comparison a mesher that stamped every quad with the first \
         palette entry's name passes the whole of the rest of this suite — including the \
         scenarios that compare two meshes for equality, whose answers would be identically \
         wrong under both"
    );
    Ok(())
}

#[test]
fn two_runs_separated_by_a_gap_arrive_in_ascending_primary_order() -> TestResult {
    let registry = plain_registry()?;
    let section = scattered_solids(two_runs_of_three_with_a_gap, &registry)?;

    let mesh = mesh_section(&section, &Neighbours::none(), &registry)?;

    assert_eq!(
        faces_towards(mesh.quads(), Facing::PosY),
        vec![
            face(Facing::PosY, 0, (0, 0), (3, 1)),
            face(Facing::PosY, 0, (4, 0), (3, 1)),
        ],
        "one non-solid voxel between the two runs is enough to end the first of them, so this \
         is two 3x1 quads and never one 7x1. They share a facing, a plane and a secondary \
         coordinate, so the only key left to order them by is the primary one — and the run \
         starting at x = 0 comes before the run starting at x = 4. This is the pair that shows \
         the order is the sweep's own and not a sort applied afterwards, since a sort by \
         anything but primary would leave them in whichever order they were produced"
    );
    Ok(())
}
