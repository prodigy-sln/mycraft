//! What becomes of a batch that was meshed against content the world has stopped
//! serving, and of one that could not be turned into a scene at all.
//!
//! # The staleness comparison is read where the client makes it
//!
//! Every reading here goes through `Remesher::collect`, which is the call the frame
//! path makes. The worker cannot make this decision: the layers it holds were moved
//! into it when it was spawned, so a serial kept beside them is only current as far
//! as the last message it has dequeued — and on one ordered channel that leaves the
//! mismatch branch unreachable in production and constructible only by a test. What
//! is graded below is therefore the client's own comparison of the serial a batch
//! was drained under against the serial now serving.
//!
//! # A reload that changes no geometry still supersedes a batch in flight, and one
//! scenario needs exactly that
//!
//! It is an accepted cost of deciding staleness on one serial rather than on a
//! second one tracking only reloads that changed the picture. It is also what makes
//! the last scenario here sharp: a candidate that marks *nothing* leaves the batch
//! superseded all the same, so the sections waiting afterwards are exactly the ones
//! handed back and nothing else. Against a reload that marked the world, a
//! `mark_for_remesh` that did nothing at all would still leave those sections
//! waiting.
//!
//! # The batch drained after the swap comes from a second edit, never from the
//! reload's own marking
//!
//! Which sections a reload marks is another task's subject, and a scenario here that
//! needed the marking would redden over that instead of over the staleness question.
//! It is also what makes `retire` load-bearing: the second edit places the block the
//! reload declared, so a worker that was never told the layers now serving cannot
//! pack that batch at all.
//!
//! # Two readings on two sides of one seam, and mixing them up cost this file a
//! silent pass
//!
//! The first two scenarios read `Remesher::collect`, which is where the staleness
//! comparison is made and therefore the only place "this batch was superseded" is
//! observable. The third reads `Session::collect_remesh`, because its subject is not
//! the comparison but the **hand-back** — and the hand-back is a decision rather
//! than a value. The keys never reach the frame path at all now, so there is nothing
//! there to drop.
//!
//! **The first draft read the third through `Remesher::collect` too and then called
//! the hand-back itself.** What it graded was that keys handed to the client come
//! back out of its dirty set, which is true and is not the scenario: a frame path
//! that discarded the batch and dropped the keys satisfied it exactly, measured at
//! 77 of 77 green. That is *policy is not wiring* in its purest form — a test that
//! calls the function itself is agreement between two callers of one function — and
//! the correction is that **nothing in this file puts a section back.**
//!
//! # No device, no window
//!
//! A worker packs a scene; nothing here uploads one. What a scene *looks* like when
//! it reaches a device is a different scenario in a different binary.

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

use mc_client::remesh::Remesher;
use mc_render::texture::TextureLayers;
use mc_sim::world::SectionKey;

use input::InputHarness;
use reload::{
    AMBER, AMBER_FILE, Adoption, DIRT, Declaration, STONE, accepted, adoption, amber, candidate,
    declaring, shipped, shipped_restating_stone,
};
use reload_content::candidate_against;
use reload_remesh::{
    Collected, Handled, NOTHING_WAS_LEFT_TO_MESH, Reported, a_client_over, a_scene_of_one_column,
    breaking_the_far_cell, collected, handled, keys_of, layers_serving,
    placing_over_the_near_cell_after_the_far_aim, reported, require, retained_at_launch,
    serial_serving,
};
use reload_world::{
    Edit, NOTHING, OVER_THE_NEAR_CELL, THE_FAR_CELL, floor_of, registry_of, standing, wrote,
};
use support::{TestResult, content_root};

/// A client on a stone floor and the re-mesh worker a launch over it spawned.
struct Playing {
    client: InputHarness,
    remesher: Remesher,
}

#[test]
fn a_batch_meshed_against_content_that_stopped_serving_is_discarded_and_the_next_one_is_drawn()
-> TestResult {
    let mut playing = playing_with_a_worker()?;
    drop(one_edit_left_in_flight(&mut playing)?);
    let answered = a_reload_declaring_the_new_block(&mut playing)?;

    let discarded = collected(&mut playing.remesher);
    one_edit_after_the_swap(&mut playing)?;
    let drawn = collected(&mut playing.remesher);

    assert_eq!(
        (answered, discarded, drawn),
        (
            accepted(AMBER),
            Collected::Superseded,
            a_scene_of_one_column()
        ),
        "the batch was drained while the previous content was serving, and by the time it finished \
         the world was serving something else — so its scene is a picture of content nobody is \
         playing and is never drawn from. The third element is the other half and it is not \
         optional: what *is* drawn is a scene from a batch drained *after* the swap, and that batch \
         holds the newly declared block — so a worker that was never told the layers now serving \
         cannot pack it and reports a failure here rather than a scene"
    );
    Ok(())
}

#[test]
fn a_batch_that_cannot_be_packed_is_reported_and_the_worker_still_draws_the_next_one() -> TestResult
{
    let mut playing = playing_with_a_worker_that_holds_no_layers()?;
    drop(one_edit_left_in_flight(&mut playing)?);
    let refused = reported(collected(&mut playing.remesher), STONE);

    let answered = a_reload_declaring_the_new_block(&mut playing)?;
    one_edit_after_the_swap(&mut playing)?;
    let drawn = collected(&mut playing.remesher);

    assert_eq!(
        (refused, answered, drawn),
        (
            Reported::FailedNamingTheBlock,
            accepted(AMBER),
            a_scene_of_one_column()
        ),
        "a re-mesh that cannot be completed is stated once and dropped, and the run carries on with \
         the picture it already had — the opposite of what a failed preparation does, and \
         deliberately, because a preparation has no previous picture and a re-mesh has the one it \
         drew a moment ago. The third element is what says the run really did carry on: a later \
         batch is still meshed, packed and handed over"
    );
    Ok(())
}

#[test]
fn a_client_that_discards_a_batch_leaves_the_sections_it_would_have_meshed_waiting() -> TestResult {
    let mut playing = playing_with_a_worker()?;
    let keys = one_edit_left_in_flight(&mut playing)?;
    let unbreakable = shipped_restating_stone(&Declaration::of(STONE).breakable(false))?;
    let answered = adoption(playing.client.adopt(candidate(unbreakable.path())?));
    retiring(&mut playing)?;

    let discarded = handled(&mut playing.client, &mut playing.remesher);
    let waiting = playing.client.take_remesh_work().map(|work| keys_of(&work));

    assert_eq!(
        (answered, discarded, waiting),
        (accepted(DIRT), Handled::Discarded, Some(keys)),
        "the discarded batch's sections go back among the ones waiting to be meshed, or they stay \
         stale for the rest of the run — a wrong picture with no error anywhere. **Nothing in this \
         test puts them back**: the reading goes through the client's own collect, which is where \
         that decision is made, so a caller that discarded the batch and dropped its keys fails \
         here. The candidate changes only `breakable`, so the reload marks nothing of its own — \
         which is what makes the last element the sections handed back and nothing else, where \
         against a reload that marked the world a hand-back doing nothing would leave the same set \
         waiting"
    );
    Ok(())
}

/// Breaks the cell the spawn's look meets, hands the worker whatever that left to
/// mesh, and says which sections that was.
///
/// **A batch in flight is the premise of every scenario here**, and a click that
/// changed nothing leaves nothing to submit — which would report as a missing batch
/// rather than as the staleness question these scenarios are about.
///
/// # Errors
///
/// Returns an error if the break did not reach the world, or if it left nothing to
/// mesh.
fn one_edit_left_in_flight(playing: &mut Playing) -> Result<Vec<SectionKey>, Box<dyn Error>> {
    let broke = breaking_the_far_cell(&mut playing.client);
    require(
        broke == Edit::Emptied(THE_FAR_CELL),
        format!(
            "these scenarios need one edit to leave a batch in flight, and the break the run opens \
             with came to {broke:?}"
        ),
    )?;
    let stale = playing
        .client
        .take_remesh_work()
        .ok_or(NOTHING_WAS_LEFT_TO_MESH)?;
    let keys = keys_of(&stale);
    playing.remesher.submit(stale);
    Ok(keys)
}

/// Takes up a candidate declaring a block for the first time, and tells the worker
/// the layers now serving.
///
/// # Errors
///
/// Returns an error if the root cannot be copied or written, or if the client
/// publishes nothing to read the spent layers and the serial out of.
fn a_reload_declaring_the_new_block(playing: &mut Playing) -> Result<Adoption, Box<dyn Error>> {
    let with_amber = declaring(shipped()?, AMBER_FILE, &amber())?;
    let against_the_layers_spent = candidate_against(&with_amber, playing.client.content())?;
    let answered = adoption(playing.client.adopt(against_the_layers_spent));
    retiring(playing)?;
    Ok(answered)
}

/// Tells the worker the layers the client is publishing, and which serial they are.
///
/// # Errors
///
/// Returns an error where the client publishes nothing.
fn retiring(playing: &mut Playing) -> Result<(), Box<dyn Error>> {
    let layers = layers_serving(&playing.client)?;
    let serial = serial_serving(&playing.client)?;
    playing.remesher.retire(layers, serial);
    Ok(())
}

/// Places the newly declared block and hands the worker whatever that left to mesh.
///
/// **The batch drained here holds a block only the content now serving declares**,
/// so a worker that was never told the layers that content states cannot pack it.
///
/// # Errors
///
/// Returns an error if the placement did not reach the world, or if it left nothing
/// to mesh.
fn one_edit_after_the_swap(playing: &mut Playing) -> Result<(), Box<dyn Error>> {
    let built = placing_over_the_near_cell_after_the_far_aim(&mut playing.client);
    require(
        built == wrote(OVER_THE_NEAR_CELL, NOTHING, AMBER),
        format!(
            "the batch drained after the swap has to be the one an edit made *after* it, and to hold \
             the block the reload declared. The placement came to {built:?}"
        ),
    )?;
    let fresh = playing
        .client
        .take_remesh_work()
        .ok_or(NOTHING_WAS_LEFT_TO_MESH)?;
    playing.remesher.submit(fresh);
    Ok(())
}

/// A client on a stone floor and the worker a launch over it would have spawned.
fn playing_with_a_worker() -> Result<Playing, Box<dyn Error>> {
    spawning(|serving| serving)
}

/// The same, with a worker that was handed no texture layers at all — so every batch
/// it finishes refuses at the packing.
fn playing_with_a_worker_that_holds_no_layers() -> Result<Playing, Box<dyn Error>> {
    spawning(|_| TextureLayers::default())
}

/// A client and a worker over one stone floor, with the worker's layers decided by
/// `worker_layers` from the ones the client is publishing.
fn spawning(
    worker_layers: impl FnOnce(TextureLayers) -> TextureLayers,
) -> Result<Playing, Box<dyn Error>> {
    let root = content_root()?;
    let registry = registry_of(&root)?;
    let blocks = floor_of(&registry, STONE)?;
    let client = a_client_over(&root, standing(), |declared| floor_of(declared, STONE))?;
    let serving = serial_serving(&client)?;
    let retained = retained_at_launch(blocks, registry, worker_layers(layers_serving(&client)?))?;
    Ok(Playing {
        client,
        remesher: Remesher::spawn(retained, serving),
    })
}
