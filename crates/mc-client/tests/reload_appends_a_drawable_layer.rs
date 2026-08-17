//! The layer a reload appended, where it crosses out of the client's core: the
//! value the report hands the frame path, and what a packer writes with it.
//!
//! # The layers come out of the report, and that is the whole point of the siting
//!
//! `App`'s share of a reload is one upload of layers somebody else built, in a file
//! that needs a real window nothing in this workspace constructs and that sits in
//! the one crate excluded from the coverage denominator wholesale. So the value is
//! asserted where it leaves the part a test can drive: the report of an accepted
//! reload. A test that rebuilt the layers from a second read of the same content
//! root would agree with itself while the report carried nothing at all.
//!
//! # The layer is read back out of the packed corners, never asked of the
//! assignment
//!
//! Asking the assignment which layer it holds leaves the consumer free to derive
//! one of its own, which is the exact failure appending exists to close. What is
//! read here is what `build_section_geometry` wrote into every corner of every one
//! of the new block's faces — the same function the client's own `scene_of` calls.
//!
//! # `texture` equals `name`, and that is the fixture's constraint rather than the
//! requirement's
//!
//! The packer selects an entry of the assignment by parsing the block's own *name*
//! as a texture key. SPEC-016's pin on that substitution turns red the day the gap
//! is closed and that red is its success signal, so nothing here may need a texture
//! resolved through the registry — which is why the block declared below states a
//! texture equal to its name.

#[path = "support/input/mod.rs"]
mod input;
#[path = "support/reload.rs"]
mod reload;
#[path = "support/reload_content.rs"]
mod reload_content;
#[path = "support/reload_remesh.rs"]
mod reload_remesh;
#[path = "support/reload_upload.rs"]
mod reload_upload;
#[path = "support/reload_watch.rs"]
mod reload_watch;
#[path = "support/reload_world.rs"]
mod reload_world;
mod support;

use std::error::Error;

use mc_core::id::BlockName;

use input::InputHarness;
use reload::{AMBER, AMBER_FILE, STONE, amber, shipped};
use reload_content::THE_NEXT_UNUSED_LAYER;
use reload_remesh::{NOTHING_WAS_LEFT_TO_MESH, meshed_of, placing_over_the_near_cell, require};
use reload_upload::{
    A_BLOCK_ON_A_FLOOR_SHOWS, CORNERS_PER_QUAD, Packed, declaring_after_launch, layers_handed_over,
    packed, until_taken_up,
};
use reload_watch::a_client_on;
use reload_world::{NOTHING, OVER_THE_NEAR_CELL, wrote};
use support::TestResult;

#[test]
fn a_placement_of_a_newly_declared_block_is_packed_from_the_layer_that_reload_appended()
-> TestResult {
    let root = shipped()?;
    let (mut client, reports) = a_client_on(&root, STONE)?;
    let declared = declaring_after_launch(&root, AMBER_FILE, &amber())?;
    reports.changed(&[declared])?;

    let layers = layers_handed_over(until_taken_up(&mut client))?;
    require_it_is_in_hand(&client)?;
    let built = placing_over_the_near_cell(&mut client);
    let work = client.take_remesh_work().ok_or(NOTHING_WAS_LEFT_TO_MESH)?;

    assert_eq!(
        (built, packed(&meshed_of(&work), AMBER, layers.stated())),
        (
            wrote(OVER_THE_NEAR_CELL, NOTHING, AMBER),
            Packed::Faces {
                corners: A_BLOCK_ON_A_FLOOR_SHOWS * CORNERS_PER_QUAD,
                layer: THE_NEXT_UNUSED_LAYER,
                sharing: Vec::new(),
            }
        ),
        "a texture key no registered block named takes the first layer nothing holds, the report of \
         the accepted reload hands those layers over, and a placement of the block is packed from \
         that layer. Every corner of every face carries it — a packer writing it into the first \
         corner and something else into the other three draws three quarters of the block from \
         somebody else's texture — and no other block shares it, which is the difference between \
         drawing the new block and drawing whichever block owns the layer a re-derived assignment \
         would have given it"
    );
    Ok(())
}

/// Refuses unless the newly declared block is the one a placement would name.
///
/// The scenario places what the client is holding, so a client still holding the
/// block it launched with would build that instead and the packing below would be
/// about the wrong block.
fn require_it_is_in_hand(client: &InputHarness) -> Result<(), Box<dyn Error>> {
    let held = client.held_block();
    require(
        held.as_ref().map(BlockName::as_str) == Some(AMBER),
        format!(
            "this scenario places the block the reload put in the player's hand, so that block has \
             to be `{AMBER}` — and the client holds {held:?}"
        ),
    )
}
