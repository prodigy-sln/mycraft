//! A reload's whole-world re-mesh runs while the ticks go on being advanced.
//!
//! A reload that changed what is drawn marks every section of the world, and
//! meshing 256 of them is tens of milliseconds of work against a tick of
//! microseconds. If a tick waited for it, one save would freeze the game for as
//! long as the world takes to mesh — which is the whole of what "a reload does not
//! stall the game" means, and the reason the re-mesh transport was built to run on
//! a worker before this spec ever touched it.
//!
//! The ticks advanced while it runs are therefore **the ticks the same inputs would
//! have advanced with no reload in flight**, judged against a second run and never
//! against a threshold: a wall-clock assertion on shared hardware is a flake
//! generator, and one that fails intermittently teaches everybody to re-run it.
//!
//! # The candidate adds a block rather than changing one, and that is what makes
//! the two runs comparable
//!
//! The marking rule is binary: a candidate that changes some block's declared
//! solidity or texture key — **or adds or removes a block** — marks every section,
//! and one that does neither marks none. Adding `base:amber` therefore marks the
//! whole world while changing nothing any tick can feel: no cell's solidity moves,
//! so the two runs are physically identical for as many ticks as either is
//! advanced. A candidate taking stone's solidity away would have marked the same
//! 256 sections and *legitimately* parted the two runs the moment one of them fell
//! through something the other stood on — the over-tight assertion `testing.md` §2
//! names, whose cheapest repair is to freeze the tick.
//!
//! # What this grades, and what it does not
//!
//! It grades that the ticks either side of a whole-world re-mesh are the ticks a
//! run with no reload advances, on an independent-run oracle. **It does not grade
//! that the meshing ran off the tick thread**, and the reason is structural rather
//! than an omission: `Session::tick` has no handle on the worker at all — the
//! collect lives in the frame path and is a `try_recv` — so "the tick joined the
//! worker" is not a mutation anybody can write without changing a signature. That
//! is the same shape as a batch carrying the registry its own world was resolved
//! against: unspellable rather than checked — named here as a weak instrument rather
//! than claimed as a defence.
//!
//! The property that **is** writable and is not structural — the candidate *build*
//! running off the tick thread — is graded by
//! `tests/reload_build_runs_off_the_tick_thread.rs`, which is a separate
//! instrument for a separate transport. Neither covers the other.
//!
//! # The batch is genuinely in flight, and that is fixture construction rather
//! than an assertion
//!
//! A count of boundaries crossed with a batch outstanding would be a timing
//! assertion wearing a count's clothing — a tight test loop and a rendered frame
//! differ by orders of magnitude — so nothing here asserts one. What holds the
//! premise is the shape of the work: the batch is submitted before the ticks are
//! advanced and 256 sections mesh in tens of milliseconds against a loop that
//! crosses its ticks in microseconds. Said out loud because no assertion below can
//! enforce it.
//!
//! # No device, no window
//!
//! The worker packs a scene and nothing here uploads one.

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
use std::path::Path;

use glam::Vec3;

use mc_client::remesh::Remesher;
use mc_sim::simulation::SimSnapshot;

use input::InputHarness;
use reload::{AMBER, AMBER_FILE, Adoption, accepted, adoption, amber, declaring, shipped};
use reload_content::candidate_against;
use reload_remesh::{
    Collected, EVERY_SECTION_OF_THE_SHIPPED_WORLD, Marking, NOTHING_WAS_LEFT_TO_MESH,
    a_client_over, collected, every_section_once, keys_of, marked, marking_of, require,
    resolution_serving, retained_at_launch, serial_serving,
};
use reload_world::{published_tick, registry_of, shipped_world, standing_and_facing, standing_at};
use support::{TestResult, content_root};

/// Where both players stand: above the landmark pillar's top, in open air.
///
/// Nothing a tick advances from here can edit a cell, so every section marked was
/// marked by the reload — and a fall moves the player on **every** tick, which is
/// what an oracle advanced one tick short disagrees with.
const IN_OPEN_AIR: Vec3 = Vec3::new(8.5, 70.0, 8.5);

/// How many ticks both runs advance with the whole-world batch outstanding.
///
/// More than one, so a run that swallowed a single tick is visible, and few enough
/// that neither player has reached the terrain: 70 blocks above a surface that
/// tops out at 64 is a long fall at any plausible gravity.
const TICKS_WHILE_IT_MESHES: u32 = 8;

#[test]
fn the_ticks_a_whole_world_re_mesh_runs_over_are_the_ticks_a_run_with_no_reload_advances()
-> TestResult {
    let serving = content_root()?;
    let (mut reloading, mut untouched) = two_clients_falling_over_the_shipped_world(&serving)?;
    let mut worker = a_worker_a_launch_would_have_spawned(&serving, &reloading)?;

    let answered = a_reload_declaring_a_new_block(&mut reloading)?;
    let marking = whole_world_batch_in_flight(&mut reloading, &mut worker)?;
    let ticked = advanced_in_lockstep(&mut reloading, &mut untouched)?;

    assert_eq!(
        (
            ticked.across_the_remesh,
            ticked.advanced,
            answered,
            marking,
            collected(&mut worker)
        ),
        (
            ticked.oracle,
            TICKS_WHILE_IT_MESHES,
            accepted(AMBER),
            every_section_once(),
            a_scene_of_every_section()
        ),
        "meshing the whole world is tens of milliseconds of work and a tick is microseconds, so a \
         tick that waited for one would freeze the game for as long as a save takes to redraw. The \
         ticks either side of it are the ticks the same inputs advance with nothing in flight, \
         judged against a second run rather than a clock — and the last three elements are what say \
         there was a whole-world re-mesh to be blocked by at all: the candidate was taken up, every \
         section of the world was left to mesh, and a scene of all of them came back"
    );
    Ok(())
}

/// Two clients falling in open air over the world a launch builds, both playing the
/// root at `root` and neither with anything outstanding to mesh.
///
/// **One script, written once and run on both**, so the two runs differ in the
/// candidate and in nothing else. Draining both is a guard as much as a
/// preparation: a launch that left sections marked would make the batch this
/// scenario submits the reload's plus something else.
///
/// # Errors
///
/// Returns an error if the root does not read, if the world does not build, or if
/// either client has anything left to mesh before the reload.
fn two_clients_falling_over_the_shipped_world(
    root: &Path,
) -> Result<(InputHarness, InputHarness), Box<dyn Error>> {
    let mut reloading = a_client_over(root, standing_at(IN_OPEN_AIR), shipped_world)?;
    let mut untouched = a_client_over(root, standing_at(IN_OPEN_AIR), shipped_world)?;
    require_nothing_outstanding(&mut reloading)?;
    require_nothing_outstanding(&mut untouched)?;
    Ok((reloading, untouched))
}

/// What both runs came to over the ticks the re-mesh spanned.
struct Lockstep {
    /// Where the reloading run's player is standing and looking.
    across_the_remesh: ([u32; 3], u32, u32),
    /// Where the run with no reload in flight is.
    oracle: ([u32; 3], u32, u32),
    /// How many ticks the reloading run published over them.
    advanced: u32,
}

/// Both clients advanced the same ticks with the batch outstanding, and what each
/// published afterwards.
///
/// One loop rather than two, so the number of ticks the two have seen is equal by
/// construction rather than by a count somebody kept.
///
/// # Errors
///
/// Returns an error if either client publishes nothing, or if the oracle did not
/// move over those ticks — a player who never moves agrees with a run advanced one
/// tick less.
fn advanced_in_lockstep(
    reloading: &mut InputHarness,
    untouched: &mut InputHarness,
) -> Result<Lockstep, Box<dyn Error>> {
    let before = published_tick_of(untouched)?;
    for _ in 0..TICKS_WHILE_IT_MESHES {
        reloading.tick();
        untouched.tick();
    }
    require_moved(untouched, before)?;
    Ok(Lockstep {
        across_the_remesh: standing_and_facing(&published(reloading)?),
        oracle: standing_and_facing(&published(untouched)?),
        advanced: advanced_by(reloading, before)?,
    })
}

/// A scene of every section of the shipped world, which is what a whole-world batch
/// packs.
fn a_scene_of_every_section() -> Collected {
    Collected::Scene {
        sections: EVERY_SECTION_OF_THE_SHIPPED_WORLD,
    }
}

/// The worker a launch over the shipped world would have spawned, holding the
/// sections and the layers that launch meshed.
///
/// Built from the same whole-world mesh a launch runs, so the list a re-meshed
/// section is spliced back into is the one the product would have held.
///
/// # Errors
///
/// Returns an error if the root does not read, if the world does not build or mesh,
/// or if the client publishes nothing to read the layers and the serial out of.
fn a_worker_a_launch_would_have_spawned(
    root: &Path,
    client: &InputHarness,
) -> Result<Remesher, Box<dyn Error>> {
    let registry = registry_of(root)?;
    let blocks = shipped_world(&registry)?;
    let retained = retained_at_launch(blocks, registry, resolution_serving(client)?)?;
    Ok(Remesher::spawn(retained, serial_serving(client)?))
}

/// Takes up a candidate that adds `base:amber` and says what the client answered.
///
/// Read against the layers the session has already spent, which is what a reload's
/// own build stage reads — a candidate built against a fresh assignment would
/// renumber every layer, which is a different scenario's subject and not something
/// this one should be quietly doing.
///
/// # Errors
///
/// Returns an error if the root cannot be copied or written, or if the client
/// publishes nothing.
fn a_reload_declaring_a_new_block(client: &mut InputHarness) -> Result<Adoption, Box<dyn Error>> {
    let with_amber = declaring(shipped()?, AMBER_FILE, &amber())?;
    let candidate = candidate_against(&with_amber, client.content())?;
    Ok(adoption(client.adopt(candidate)))
}

/// Hands the worker the layers now serving and then the whole batch the reload
/// left, and says which sections that was.
///
/// **The layers go first and on the same ordered channel**, which is what a frame
/// path does after an accepted reload: a worker told afterwards would judge this
/// batch against the serial it held before and discard it, and the scenario would
/// be about a supersession instead.
///
/// # Errors
///
/// Returns an error if the reload left nothing to mesh, or if the client publishes
/// nothing.
fn whole_world_batch_in_flight(
    client: &mut InputHarness,
    worker: &mut Remesher,
) -> Result<Marking, Box<dyn Error>> {
    worker.retire(resolution_serving(client)?, serial_serving(client)?);
    let batch = client.take_remesh_work().ok_or(NOTHING_WAS_LEFT_TO_MESH)?;
    let marking = marking_of(&keys_of(&batch));
    worker.submit(batch);
    Ok(marking)
}

/// Refuses unless the client has nothing outstanding to mesh.
///
/// Both a guard and the reason the reading afterwards means anything: a launch that
/// left sections marked would make the batch this submits the reload's plus
/// something else.
fn require_nothing_outstanding(client: &mut InputHarness) -> Result<(), Box<dyn Error>> {
    let outstanding = marked(client);
    require(
        outstanding == Marking::NoSectionAtAll,
        format!(
            "this scenario submits the batch one reload left, so the launch has to have left \
             nothing — and it left {outstanding:?}"
        ),
    )
}

/// Whatever the client has published.
///
/// # Errors
///
/// Returns an error where it has published nothing, which is a client with no world
/// rather than one standing anywhere.
fn published(client: &InputHarness) -> Result<SimSnapshot, Box<dyn Error>> {
    client
        .published()
        .map(|published| *published)
        .ok_or_else(|| "this fixture's client has published no tick to compare".into())
}

/// Which tick the client last published.
///
/// # Errors
///
/// Returns an error where it has published nothing.
fn published_tick_of(client: &InputHarness) -> Result<u32, Box<dyn Error>> {
    Ok(published_tick(&published(client)?))
}

/// How many ticks the client has published since `before`.
///
/// # Errors
///
/// Returns an error where it has published nothing.
fn advanced_by(client: &InputHarness, before: u32) -> Result<u32, Box<dyn Error>> {
    Ok(published_tick_of(client)?.saturating_sub(before))
}

/// Refuses unless the run that is being judged actually moved over those ticks.
///
/// A player who published the same position on every tick would agree with an
/// oracle advanced one tick too few, which retires the only defence this comparison
/// has against a swallowed tick.
fn require_moved(client: &InputHarness, before: u32) -> Result<(), Box<dyn Error>> {
    let standing = standing_and_facing(&published(client)?).0;
    let advanced = advanced_by(client, before)?;
    require(
        advanced == TICKS_WHILE_IT_MESHES && standing != IN_OPEN_AIR.to_array().map(f32::to_bits),
        format!(
            "this comparison needs the oracle to have moved over the ticks it is judged across, \
             and it advanced {advanced} ticks standing at {standing:?}. A player who never moves \
             agrees with a run advanced one tick less"
        ),
    )
}
