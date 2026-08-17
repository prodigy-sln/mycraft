//! Which content a batch's sections are resolved against: the one now serving,
//! never the one it replaced.
//!
//! # A batch hands in no registry, so there is nothing for a fixture to choose
//!
//! A re-mesh batch carries the registry the world that produced it was resolved
//! against, and meshing one takes no second opinion. That is what makes both
//! scenarios here structural confirmations of a design rather than defences against
//! a live hazard — and it is also why each is written to be *behavioural anyway*:
//! the content now serving declares a block the content it replaced does not, so a
//! batch resolved against the wrong one cannot be meshed at all and says so.
//!
//! # The oracle is an independent whole-world mesh
//!
//! The first scenario compares the batch against a mesh of the same declared blocks
//! against a *second read* of the candidate's own content root — no batch, no dirty
//! set, no client. Its control is the same mesh against the content that was
//! serving before, which has to differ: without that, a batch resolved against
//! either registry would satisfy the comparison and the scenario would say nothing.
//!
//! # The second scenario is about an *edit's* batch and not the reload's
//!
//! The reload's own whole-world batch is drained and discarded first, and the drain
//! after it is required to be empty — so the batch the break produces is the break's
//! alone. Without that the comparison would be over sections the reload marked, and
//! the scenario's subject is a player breaking a block *after* a candidate was
//! accepted.

#[path = "support/input/mod.rs"]
mod input;
#[path = "support/reload.rs"]
mod reload;
#[path = "support/reload_content.rs"]
mod reload_content;
#[path = "support/reload_remesh.rs"]
mod reload_remesh;
#[path = "support/reload_world.rs"]
mod reload_world;
mod support;

use std::error::Error;
use std::sync::Arc;

use mc_core::block::BlockRegistry;
use mc_sim::content::LoadedContent;
use mc_world::world::VoxelWorld;

use input::InputHarness;
use reload::{
    AMBER, AMBER_FILE, DIRT, Declaration, GRASS, STONE, accepted, adoption, amber, candidate,
    declaring, shipped_restating_stone, stone_that_is_not_solid,
};
use reload_content::candidate_against;
use reload_remesh::{
    Faces, Marking, NOTHING_WAS_LEFT_TO_MESH, a_client_over, breaking_the_far_cell, faces_shown,
    keys_of, marked, meshed, meshed_against, meshed_of, require,
};
use reload_world::{
    Cell, OVER_THE_NEAR_CELL, THE_FAR_CELL, floor_holding, floor_of, registry_of, standing, wrote,
};
use support::{TestResult, content_root};

/// Where the one grass block of the first scenario's world stands.
const GRASS_ON_THE_FLOOR: Cell = OVER_THE_NEAR_CELL;

/// How many faces a block left behind in the middle of a solid floor shows: its
/// top, because nothing stands on it, and its bottom, because the cell under the
/// floor is empty. Its four sides abut the floor and are buried.
const A_BLOCK_SET_INTO_A_FLOOR_SHOWS: usize = 2;

/// How many sections one break in a one-column world leaves to be meshed.
///
/// The section holding the edited cell and every face-adjacent neighbour the
/// footprint holds — which, in a world of one column with the edit in its lowest
/// section, is the section above it and nothing else.
const A_BREAK_IN_ONE_COLUMN_MARKS: usize = 2;

#[test]
fn every_section_a_reloads_batch_covers_is_meshed_against_the_content_now_serving() -> TestResult {
    let root = content_root()?;
    let replaced = registry_of(&root)?;
    let softened = shipped_restating_stone(&stone_that_is_not_solid())?;
    let now_serving = meshed_against(
        a_floor_holding_grass(&replaced)?,
        registry_of(softened.path())?,
    )?;
    let as_it_was = meshed_against(a_floor_holding_grass(&replaced)?, Arc::clone(&replaced))?;
    require(
        now_serving != as_it_was,
        "this scenario compares a batch against a mesh of the content now serving, so the two \
         contents have to mesh the same world differently — otherwise a batch resolved against \
         either of them satisfies the comparison and nothing is being graded"
            .to_owned(),
    )?;

    let mut client = a_client_over(&root, standing(), a_floor_holding_grass)?;
    require_nothing_outstanding(&mut client)?;
    let answered = adoption(client.adopt(candidate(softened.path())?));
    let batch = meshed(&mut client);

    assert_eq!(
        (answered, batch),
        (accepted(DIRT), now_serving),
        "every section of the batch, quad for quad and origin for origin, against a whole-world \
         mesh of the same declared blocks through a second read of the candidate's own root. A batch \
         that carried the registry the world was resolved against cannot be meshed through any \
         other, which is what makes this a confirmation of a structure rather than a defence — and \
         it is written as a total comparison so that a batch resolved against the content it \
         replaced fails on the sections it got wrong rather than on a count"
    );
    Ok(())
}

#[test]
fn a_break_after_a_reload_is_meshed_against_the_block_the_new_content_left_behind() -> TestResult {
    let root = content_root()?;
    let mut client = a_client_over(&root, standing(), |declared| floor_of(declared, STONE))?;
    require_nothing_outstanding(&mut client)?;

    let answered = adoption(client.adopt(a_candidate_breaking_stone_into_amber(&client)?));
    drop(meshed(&mut client));
    require_nothing_outstanding(&mut client)?;

    let broke = breaking_the_far_cell(&mut client);
    let work = client.take_remesh_work().ok_or(NOTHING_WAS_LEFT_TO_MESH)?;
    let marked_by_the_break = keys_of(&work).len();
    let edits_own = meshed_of(&work);

    assert_eq!(
        (
            answered,
            broke,
            marked_by_the_break,
            faces_shown(&edits_own, AMBER)
        ),
        (
            accepted(AMBER),
            wrote(THE_FAR_CELL, STONE, AMBER),
            A_BREAK_IN_ONE_COLUMN_MARKS,
            Faces::Showing(A_BLOCK_SET_INTO_A_FLOOR_SHOWS)
        ),
        "the break leaves behind the block the content now serving says stone breaks into, and that \
         block is one the content the world launched with never declared. So the edit's own batch \
         either resolves against what is serving now and shows the two faces the floor leaves \
         uncovered, or it resolves against what it replaced and cannot be meshed at all — which is \
         the answer the refused arm of this verdict carries"
    );
    Ok(())
}

/// A candidate that declares a block for the first time and says stone breaks into
/// it, read against the layers `client` has already spent.
///
/// **Both edits together are what makes the break's batch discriminating**: the
/// residue is resolved when the break happens, so the cell ends up holding a block
/// the content the world launched with never declared.
///
/// # Errors
///
/// Returns an error if the root cannot be copied or written, or if the client
/// publishes nothing to read the spent layers out of.
fn a_candidate_breaking_stone_into_amber(
    client: &InputHarness,
) -> Result<LoadedContent, Box<dyn Error>> {
    let with_amber = declaring(
        shipped_restating_stone(&Declaration::of(STONE).breaking_into(AMBER))?,
        AMBER_FILE,
        &amber(),
    )?;
    candidate_against(&with_amber, client.content())
}

/// One column whose only solid layer is a stone floor, with a single grass block
/// standing on it.
fn a_floor_holding_grass(registry: &BlockRegistry) -> Result<VoxelWorld, Box<dyn Error>> {
    floor_holding(registry, STONE, &[(GRASS_ON_THE_FLOOR, GRASS)])
}

/// Refuses unless the client has nothing outstanding to mesh, and drains it.
///
/// Called twice in the second scenario: once for the launch, and once after the
/// reload's own batch has been drained and dropped. **The batch that scenario reads
/// has to be the break's own**, so nothing may be waiting when the break happens —
/// otherwise the faces counted would include sections the reload marked, and the
/// count of sections the break marked would be the whole world's.
fn require_nothing_outstanding(client: &mut InputHarness) -> Result<(), Box<dyn Error>> {
    let outstanding = marked(client);
    require(
        outstanding == Marking::NoSectionAtAll,
        format!(
            "this scenario reads one batch, so nothing may be outstanding before the edit it is \
             about — and {outstanding:?} was"
        ),
    )
}
