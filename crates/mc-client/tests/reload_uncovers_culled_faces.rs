//! What a reload draws once a block it culled faces against has stopped being
//! solid.
//!
//! # The observable is a face, not a count of quads
//!
//! Taking stone's solidity away uncovers every stone-against-stone face in the
//! world, so a whole-section quad count would move by hundreds and say nothing
//! about *which* face appeared. What this reads instead is one block's one
//! downward face: a grass block set on a stone floor buries its `-Y` face against
//! the stone under it, and that face is drawn — exactly once, because there is one
//! grass voxel in the world — the moment stone is not solid.
//!
//! # The control is an independent mesh of the same declared world
//!
//! The reading before the reload does not come from a snapshot of the client. It
//! comes from meshing the same declared blocks against the registry the content
//! root serving at launch produces, which shares no batch, no dirty set and no
//! registry with the client under test. So "the face was absent and is now
//! present" is a comparison between two derivations rather than between a value
//! and itself.
//!
//! # It rides on the marking, and that is why it is here
//!
//! The face can only be drawn if the section holding it was marked and then meshed
//! against the content now serving. A client that swapped the registry and marked
//! nothing leaves the picture exactly as it was, with the physics already correct —
//! which is the split this scenario exists to close.

#[path = "support/input/mod.rs"]
mod input;
#[path = "support/reload.rs"]
mod reload;
#[path = "support/reload_remesh.rs"]
mod reload_remesh;
#[path = "support/reload_world.rs"]
mod reload_world;
mod support;

use std::error::Error;
use std::path::Path;

use mc_core::block::BlockRegistry;
use mc_world::mesh::Facing;
use mc_world::world::VoxelWorld;

use reload::{
    DIRT, GRASS, STONE, accepted, adoption, candidate, shipped_restating_stone,
    stone_that_is_not_solid,
};
use reload_remesh::{
    Faces, Marking, a_client_over, faces_facing, faces_shown_facing, marked, meshed,
    meshed_against, require, sections_meshed,
};
use reload_world::{Cell, OVER_THE_NEAR_CELL, floor_holding, registry_of, standing};
use support::{TestResult, content_root};

/// Where the one grass block of this world stands: on the stone floor, in a cell
/// the player's own box does not reach.
const GRASS_ON_THE_FLOOR: Cell = OVER_THE_NEAR_CELL;

/// How many downward faces one block has.
///
/// There is exactly one grass voxel in this world, so the count of its `-Y` faces
/// is one when that face is drawn and zero when it is culled. Nothing here is a
/// quad count that greedy merging could move.
const A_BLOCK_HAS_ONE_DOWNWARD_FACE: usize = 1;

#[test]
fn a_face_culled_against_solid_stone_is_drawn_once_stone_has_stopped_being_solid() -> TestResult {
    let root = content_root()?;
    require_the_face_is_buried_while_stone_is_solid(&root)?;

    let mut client = a_client_over(&root, standing(), a_floor_holding_grass)?;
    require_nothing_outstanding(&mut client)?;
    let softened = shipped_restating_stone(&stone_that_is_not_solid())?;

    let answered = adoption(client.adopt(candidate(softened.path())?));
    let uncovered = meshed(&mut client);

    assert_eq!(
        (
            answered,
            faces_shown_facing(&uncovered, GRASS, Facing::NegY)
        ),
        (
            accepted(DIRT),
            Faces::Showing(A_BLOCK_HAS_ONE_DOWNWARD_FACE)
        ),
        "the grass block's downward face is decided by what is under it, and what is under it is \
         stone. Once stone is not solid that face is drawn, and the only way it reaches a picture is \
         through the section being marked and then meshed against the content now serving. A client \
         whose physics let the player through stone while the mesh stayed as it was leaves the \
         player looking at a world that still has a floor — half-applied, with no error anywhere"
    );
    Ok(())
}

/// Refuses unless the face this scenario is about is buried while stone is solid.
///
/// **The reading before the reload, and it is an independent one**: the same
/// declared blocks meshed against the registry the content root serving at launch
/// produces, sharing no batch, no dirty set and no registry with the client under
/// test. Without it, "the face is drawn afterwards" would be a statement about one
/// value rather than a comparison between two.
///
/// # Errors
///
/// Returns an error if the root does not read, if the world does not build or mesh,
/// or if the face is already drawn while stone is solid.
fn require_the_face_is_buried_while_stone_is_solid(root: &Path) -> Result<(), Box<dyn Error>> {
    let serving = registry_of(root)?;
    let culled = sections_meshed(meshed_against(a_floor_holding_grass(&serving)?, serving)?)?;
    let shown = faces_facing(&culled, GRASS, Facing::NegY);
    require(
        shown == 0,
        format!(
            "this scenario is about a face that is culled before the reload and drawn after it, so \
             the world it is driven over has to bury that face while stone is solid — and an \
             independent mesh of the same declared blocks against the content serving at launch \
             already shows {shown} of them"
        ),
    )
}

/// One column whose only solid layer is a stone floor, with a single grass block
/// standing on it.
fn a_floor_holding_grass(registry: &BlockRegistry) -> Result<VoxelWorld, Box<dyn Error>> {
    floor_holding(registry, STONE, &[(GRASS_ON_THE_FLOOR, GRASS)])
}

/// Refuses unless the client has nothing outstanding to mesh, and drains it.
fn require_nothing_outstanding(client: &mut input::InputHarness) -> Result<(), Box<dyn Error>> {
    let outstanding = marked(client);
    require(
        outstanding == Marking::NoSectionAtAll,
        format!(
            "this scenario reads what one reload left to be meshed, so the launch has to have left \
             nothing — and it left {outstanding:?}"
        ),
    )
}
