//! Which faces are hidden, decided by what the block beyond them declares about
//! occluding rather than by what it declares about stopping a player.
//!
//! The two questions come apart in both directions and this file holds one
//! fixture for each. A block can stop a player and hide nothing — a cage, a
//! fence, a pane of glass — and a block can hide what is behind it while a player
//! walks straight through it. A mesher reading occlusion off solidity gets the
//! first of those wrong by showing nothing and the second wrong by showing
//! everything, so neither fixture alone would say which mistake was made.
//!
//! The same question is then asked three more times about a boundary, because a
//! boundary is where it is answered by different code: by the plane resolved for
//! a supplied neighbour, and by whatever an *unsupplied* neighbour amounts to. A
//! mesher can be right about a neighbour inside the section and wrong across
//! every chunk boundary in the world, and the shipped sea spans sections — so the
//! boundary cases are reachable rather than hypothetical.
//!
//! Nothing here is asserted as a count. Every comparison is the complete list of
//! what one fixture's mesh holds, so a mesher that emits nothing fails each of
//! them on the faces it owes rather than passing on the ones it withheld.

mod mesh_common;

use std::error::Error;

use mc_core::block::BlockRegistry;
use mc_core::id::BlockName;
use mc_world::mesh::{Facing, Neighbours, mesh_section};
use mc_world::section::{LocalPos, SECTION_SIZE, Section};
use mesh_common::{
    DRAWN_ONLY, Face, GHOST, HAZE, MIST, OCCLUDING_ONLY, SHROUD, SOLID_AND_OCCLUDING, SOLID_ONLY,
    TestResult, at, every_side_of, every_side_of_but, faces, registry_of_declarations,
    require_runtime_id, section_of_nothing_but,
};

/// The runtime id the drawn block holds, pinned for the same reason the sibling
/// file pins it: a mesher reading the lowest id as empty space would answer every
/// scenario here with silence, which is the wrong answer for the right-looking
/// reason.
const DRAWN_RUNTIME_ID: u32 = 0;

/// The drawn voxel in the two fixtures about a neighbour inside the section, and
/// the cell one step towards +X of it.
///
/// Pairwise distinct coordinates, so its six sides land on three different
/// planes and a face labelled by the wrong axis has somewhere wrong to land.
const DRAWN_VOXEL: LocalPos = at(1, 2, 3);
const BESIDE_IT: LocalPos = at(2, 2, 3);

/// The facing that step crosses.
const TOWARDS_IT: Facing = Facing::PosX;

/// The drawn voxel in the fixtures about a boundary.
///
/// It sits in the corner where three boundaries meet — the far +X face and the
/// near −Y and −Z ones — so one fixture can ask about a supplied neighbour that
/// occludes, a supplied one that does not, and the three unsupplied ones, without
/// six separate sections.
const DRAWN_IN_THE_CORNER: LocalPos = at(SECTION_SIZE - 1, 0, 0);

/// Which boundary is supplied with a neighbour that does not occlude, and which
/// two are supplied with neighbours that do.
const NON_OCCLUDING_BEYOND: Facing = Facing::PosX;
const OCCLUDING_BEYOND: [Facing; 2] = [Facing::NegY, Facing::NegZ];

/// A registry in which occlusion and solidity disagree in both directions, with
/// the drawn block's placement pinned.
///
/// # Errors
///
/// Returns an error if the registry refuses the batch, or if it numbered the
/// drawn block somewhere other than first.
fn registry_where_occlusion_and_solidity_disagree() -> Result<BlockRegistry, Box<dyn Error>> {
    let registry = registry_of_declarations(&[
        (HAZE, DRAWN_ONLY),
        (MIST, SOLID_ONLY),
        (SHROUD, OCCLUDING_ONLY),
        (GHOST, SOLID_AND_OCCLUDING),
    ])?;
    require_runtime_id(&registry, HAZE, DRAWN_RUNTIME_ID)?;
    Ok(registry)
}

/// The section every boundary scenario here is taken of: one drawn voxel in the
/// corner and nothing anywhere else.
fn one_drawn_voxel_in_the_corner(registry: &BlockRegistry) -> Result<Section, Box<dyn Error>> {
    section_of_nothing_but(&[(DRAWN_IN_THE_CORNER, HAZE)], registry)
}

/// A section filled with `name`, to stand beyond a boundary.
fn filled_with(name: &str, registry: &BlockRegistry) -> Result<Section, Box<dyn Error>> {
    Ok(Section::filled(&BlockName::parse(name)?, registry)?)
}

/// Every side of the corner voxel except the two an occluding neighbour hides.
///
/// Filtered from the whole six rather than listed, so the four that remain keep
/// the emission order the other six-sided expectations in this file are in.
fn every_side_except_the_occluded_ones() -> Vec<Face> {
    every_side_of(DRAWN_IN_THE_CORNER)
        .into_iter()
        .filter(|side| !OCCLUDING_BEYOND.contains(&side.facing))
        .collect()
}

#[test]
fn a_solid_neighbour_that_does_not_occlude_leaves_the_face_toward_it_showing() -> TestResult {
    let registry = registry_where_occlusion_and_solidity_disagree()?;
    let section = section_of_nothing_but(&[(DRAWN_VOXEL, HAZE), (BESIDE_IT, MIST)], &registry)?;

    let mesh = mesh_section(&section, &Neighbours::none(), &registry)?;

    assert_eq!(
        faces(mesh.quads()),
        every_side_of(DRAWN_VOXEL),
        "the neighbour stops a player and hides nothing, so the drawn voxel shows the side \
         facing it exactly as it shows the five facing empty space — all six, and the one \
         towards the neighbour is the scenario. A mesher culling on solidity hides that sixth \
         side, and hides it in the one place a fixture built from solid rock could never say \
         was wrong"
    );
    Ok(())
}

#[test]
fn a_neighbour_that_occludes_without_being_solid_hides_the_face_toward_it() -> TestResult {
    let registry = registry_where_occlusion_and_solidity_disagree()?;
    let section = section_of_nothing_but(&[(DRAWN_VOXEL, HAZE), (BESIDE_IT, SHROUD)], &registry)?;

    let mesh = mesh_section(&section, &Neighbours::none(), &registry)?;

    assert_eq!(
        faces(mesh.quads()),
        every_side_of_but(DRAWN_VOXEL, TOWARDS_IT),
        "this neighbour hides what is behind it while stopping nobody, so the side towards it is \
         culled and the other five are not. It is the exact inverse of the scenario above and it \
         is failed by the same mistake read the other way: a mesher culling on solidity shows \
         this sixth side, because nothing here is solid at all"
    );
    Ok(())
}

#[test]
fn a_neighbouring_section_holding_a_non_occluding_block_leaves_the_shared_face_showing()
-> TestResult {
    let registry = registry_where_occlusion_and_solidity_disagree()?;
    let section = one_drawn_voxel_in_the_corner(&registry)?;
    let does_not_occlude = filled_with(MIST, &registry)?;
    let occludes = filled_with(GHOST, &registry)?;

    let neighbours = OCCLUDING_BEYOND.into_iter().fold(
        Neighbours::none().with(NON_OCCLUDING_BEYOND, &does_not_occlude),
        |so_far, facing| so_far.with(facing, &occludes),
    );
    let mesh = mesh_section(&section, &neighbours, &registry)?;

    assert_eq!(
        faces(mesh.quads()),
        every_side_except_the_occluded_ones(),
        "three of this voxel's sides look out of the section and the decision is taken against \
         the voxel facing it in the section beyond, one voxel at a time. The +X neighbour holds \
         a block that stops a player and hides nothing, so that face is shown; the −Y and −Z \
         neighbours hold one that hides, so those two are not. Both halves are in one list, so a \
         mesher that read a boundary as all-or-nothing cannot satisfy it either way"
    );
    Ok(())
}

#[test]
fn a_boundary_with_no_neighbouring_section_supplied_shows_its_face() -> TestResult {
    let registry = registry_where_occlusion_and_solidity_disagree()?;
    let section = one_drawn_voxel_in_the_corner(&registry)?;

    let mesh = mesh_section(&section, &Neighbours::none(), &registry)?;

    assert_eq!(
        faces(mesh.quads()),
        every_side_of(DRAWN_IN_THE_CORNER),
        "no section was supplied beyond any of the three boundaries this voxel touches, and a \
         boundary with nothing beyond it shows its face — that is what keeps the edge of loaded \
         content visible while a world streams instead of sealing it against a chunk that has \
         not arrived. Compared against the scenario above, the same three sides answer \
         differently because the neighbours differ and for no other reason"
    );
    Ok(())
}
