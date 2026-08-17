//! A candidate is built from the whole root, on a thread that is not the tick's,
//! against the layers the session has already spent.
//!
//! # Every scenario here needs a boundary the build was still running over
//!
//! "While a candidate is being built" is a premise, and a build that ran on the
//! tick thread has no such boundary — its outcome is reported by the very boundary
//! that started it, and every assertion about those ticks holds over none of them.
//! So each scenario asks for at least one boundary between the change and the
//! outcome, and the fixture refuses rather than passes when there is none.
//!
//! It is the drive's *order* that makes such a boundary certain rather than likely:
//! a boundary collects a finished build before it starts a new one, so a build
//! started at one boundary is collected at the next one at the earliest, however
//! fast the worker is.
//!
//! # The oracle is a second run, driven by the same script, in lockstep
//!
//! The walking scenario runs two clients over two copies of one root and edits only
//! one of them, advancing both at every boundary so that the number of ticks each
//! has seen is equal by construction rather than by counting. A position copied
//! from a green run would be worse than useless: walking moves the player on every
//! tick, so "where they were" is what a frozen client produces.
//!
//! **The player walks in a circle**, holding one key and turning by a fixed amount
//! every tick, because how many boundaries a build takes is not something a test
//! may assume. At the declared walk speed of 4.5 blocks a second over a 60 Hz tick
//! and 20 raw counts of yaw per tick, the circle's radius is about 1.7 blocks — so
//! the walk stays well inside the floor for as many boundaries as the build wants,
//! and yaw, x and z all move on every tick, which is what an oracle advanced one
//! tick short disagrees with.
//!
//! **The candidate the walking scenario hands over changes only `breakable`.**
//! Nothing a player can feel differs across it, so the two runs are comparable at
//! the tick after the swap. A candidate that changed solidity under the feet of a
//! player standing on that block would drop them *legitimately* and put this red
//! against a correct client.
//!
//! # What the build reads is graded here, and only here
//!
//! A reload's build stage reads the content root against the layers the session is
//! already serving. Nothing else in this spec grades that argument: the phase that
//! built the layer policy handed it in from the test, so a product passing
//! `LayerAssignment::none()` instead would republish a lexicographic assignment on
//! every reload — renumbering every layer, invalidating every packed vertex already
//! on the GPU — with every layer scenario green. The last test in this file is that
//! instrument, and it is why it introduces a key the session has never assigned:
//! against four unchanged keys the two arguments produce the same answer.

#[path = "support/input/mod.rs"]
mod input;
#[path = "support/reload.rs"]
mod reload;
#[path = "support/reload_content.rs"]
mod reload_content;
#[path = "support/reload_watch.rs"]
mod reload_watch;
#[path = "support/reload_world.rs"]
mod reload_world;
mod support;

use std::error::Error;
use std::time::Instant;

use mc_sim::simulation::SimSnapshot;
use winit::keyboard::KeyCode;

use input::InputHarness;
use reload::{
    AMBER, AMBER_FILE, Declaration, GRASS, STONE, STONE_FILE, amber, declaring, restating, shipped,
    stone_that_is_not_solid,
};
use reload_content::{THE_NEXT_UNUSED_LAYER, fresh_layers, layers_beside, publishing};
use reload_watch::{
    Attempt, Reports, a_client_on, block_path, boundary, ended, may_cross_another,
    pause_between_boundaries, require_a_build_in_flight, serving, taken_up_once,
    the_four_shipped_blocks, until_settled,
};
use reload_world::{SPAWN, resting, standing_and_facing};
use support::TestResult;
use support::content::ContentRoot;

/// How far the walking player turns on every tick, in raw device counts.
///
/// Enough to bend the walk into a circle of about 1.7 blocks, so the run may cross
/// as many boundaries as a build takes without the player reaching the floor's
/// edge.
const TURN_PER_TICK: f64 = 20.0;

/// The one tick that carries what a swap wrote into what a reader can see.
///
/// A swap publishes no tick of its own, so the snapshot standing when a candidate
/// is taken up was written by the *previous* advance and nothing the swap did can
/// have changed it.
const THE_TICK_AFTER_THE_SWAP: u32 = 1;

#[test]
fn a_candidate_built_after_one_declaration_changed_carries_the_whole_root() -> TestResult {
    let root = shipped()?;
    let (mut client, reports) = a_client_on(&root, GRASS)?;
    let root = restating(root, STONE_FILE, &stone_that_is_not_solid())?;

    reports.changed(&[block_path(&root, STONE_FILE)])?;
    let crossed = until_settled(&mut client);
    require_a_build_in_flight(&crossed)?;

    assert_eq!(
        (serving(&client)?, publishing(client.content())?.layers),
        (stone_no_longer_solid(), fresh_layers()?),
        "one file changed and the whole root is read again, because there is no incremental door \
         into content and a candidate carrying one block is a candidate the running world cannot \
         answer for. The layer assignment is read beside the blocks because it is the other half \
         of what a partial read would produce: four live keys is the whole root's answer and one \
         is the changed file's"
    );
    Ok(())
}

#[test]
fn the_ticks_a_candidate_is_built_over_put_the_player_where_a_run_with_no_reload_would()
-> TestResult {
    let root = shipped()?;
    let untouched = shipped()?;
    let (mut reloading, reports) = a_client_walking(&root)?;
    let (mut oracle, _quiet) = a_client_walking(&untouched)?;
    let root = restating(root, STONE_FILE, &only_breakable_changed())?;

    reports.changed(&[block_path(&root, STONE_FILE)])?;
    let crossed = walking_until_reported(&mut reloading, &mut oracle)?;
    require_a_build_in_flight(&crossed)?;
    walk_on(&mut reloading);
    walk_on(&mut oracle);

    assert_eq!(
        (
            standing_and_facing(&published(&reloading)?),
            ended(&crossed),
            moved(&published(&oracle)?)
        ),
        (
            standing_and_facing(&published(&oracle)?),
            taken_up_once(),
            true
        ),
        "a build runs on a thread that is not the tick's, so the ticks it spans are the ticks the \
         same inputs would have advanced with nothing in flight — a build on the tick thread \
         stutters the game on every save, and one that swallowed a tick would leave the player a \
         step behind where they walked. The oracle is a second run rather than a remembered \
         position, and the walk is what makes an oracle advanced one tick short disagree"
    );
    Ok(())
}

#[test]
fn every_tick_a_candidate_is_built_over_is_answered_by_the_content_in_force() -> TestResult {
    let root = shipped()?;
    let (mut client, reports) = a_client_on(&root, STONE)?;
    let root = restating(root, STONE_FILE, &stone_that_is_not_solid())?;

    reports.changed(&[block_path(&root, STONE_FILE)])?;
    let (supported, crossed) = supported_while_building(&mut client)?;
    require_a_build_in_flight(&crossed)?;
    client.ticks(THE_TICK_AFTER_THE_SWAP);

    assert_eq!(
        (supported.iter().all(|held| *held), held_up(&client)?),
        (true, false),
        "the player is standing on a floor of the block the candidate takes the solidity away \
         from, so every tick before the swap has to answer from the content that was in force when \
         the build began — a build that published its registry as it went, or one that swapped \
         where it started rather than at a boundary, drops them mid-air while the file is still \
         being read. The tick after it is what says the candidate really landed"
    );
    Ok(())
}

#[test]
fn a_build_the_client_drove_reads_the_layers_the_session_has_already_spent() -> TestResult {
    let root = shipped()?;
    let (mut client, reports) = a_client_on(&root, GRASS)?;
    let root = declaring(root, AMBER_FILE, &amber())?;

    reports.changed(&[block_path(&root, AMBER_FILE)])?;
    let crossed = until_settled(&mut client);
    let published = publishing(client.content())?;

    assert_eq!(
        (ended(&crossed), published.layers, published.spent),
        (
            taken_up_once(),
            layers_beside(&[(AMBER, THE_NEXT_UNUSED_LAYER)])?,
            THE_NEXT_UNUSED_LAYER + 1
        ),
        "the build reads the root against what the session has already spent, so a key declared \
         for the first time takes the next unused layer and the four already serving keep theirs. \
         A build that read against a fresh assignment would number all five from zero — every \
         layer renumbered, every vertex already on the GPU sampling the wrong texture, no error \
         anywhere — and this is the only test in the spec that would notice"
    );
    Ok(())
}

/// The shipped four with stone's solidity taken away and nothing else touched.
fn stone_no_longer_solid() -> Vec<(String, bool)> {
    the_four_shipped_blocks()
        .into_iter()
        .map(|(block, solid)| {
            let still_solid = solid && block.as_str() != STONE;
            (block, still_solid)
        })
        .collect()
}

/// Stone as it ships, with the one field a reload changes that no player can feel.
///
/// Solidity moves the player and the texture key moves a layer; `breakable` decides
/// what a break does and nothing else, so a run either side of this candidate walks
/// identically.
fn only_breakable_changed() -> Declaration {
    Declaration::of(STONE).breakable(false)
}

/// A client walking in a circle on a floor of grass over the root at `root`, and
/// the handle changes to that same root are reported on.
///
/// The key is pressed and never released, so the player is still walking at the
/// tick the comparison is taken on.
///
/// # Errors
///
/// Returns an error if the root does not read or the world does not build.
fn a_client_walking(root: &ContentRoot) -> Result<(InputHarness, Reports), Box<dyn Error>> {
    let (mut client, reports) = a_client_on(root, GRASS)?;
    client.press(KeyCode::KeyW);
    Ok((client, reports))
}

/// One tick of a walking client: the turn, then the tick that spends it.
fn walk_on(client: &mut InputHarness) {
    client.move_pointer(TURN_PER_TICK, 0.0);
    client.tick();
}

/// Both clients walked in lockstep until the reloading one reported something.
///
/// One boundary of each per turn of the loop, so the number of ticks the two have
/// seen is equal by construction rather than by a count somebody kept.
///
/// # Errors
///
/// Returns an error if the run gives up without either client reporting.
fn walking_until_reported(
    reloading: &mut InputHarness,
    oracle: &mut InputHarness,
) -> Result<Vec<Option<Attempt>>, Box<dyn Error>> {
    let started = Instant::now();
    let mut crossed = Vec::new();
    while may_cross_another(started) {
        reloading.move_pointer(TURN_PER_TICK, 0.0);
        oracle.move_pointer(TURN_PER_TICK, 0.0);
        let attempt = boundary(reloading);
        oracle.tick();
        let reported = attempt.is_some();
        crossed.push(attempt);
        if reported {
            return Ok(crossed);
        }
        pause_between_boundaries();
    }
    Err(NOTHING_WAS_REPORTED.into())
}

/// What a run whose attempt never ended is told.
const NOTHING_WAS_REPORTED: &str = "this scenario needs an attempt to run to its end while the player walks, and the run gave up \
     without one being reported at all";

/// Whether the world held the player up at each tick before a build ended, beside
/// what every boundary of that run reported.
type HeldUpWhileBuilding = (Vec<bool>, Vec<Option<Attempt>>);

/// Whether the world held the player up at each tick before the build ended, and
/// what every boundary of the run reported.
///
/// # Errors
///
/// Returns an error if the client publishes nothing, or if the run gives up.
fn supported_while_building(
    client: &mut InputHarness,
) -> Result<HeldUpWhileBuilding, Box<dyn Error>> {
    let started = Instant::now();
    let mut supported = Vec::new();
    let mut crossed = Vec::new();
    while may_cross_another(started) {
        let attempt = boundary(client);
        let reported = attempt.is_some();
        crossed.push(attempt);
        if reported {
            return Ok((supported, crossed));
        }
        supported.push(held_up(client)?);
        pause_between_boundaries();
    }
    Err(NOTHING_WAS_REPORTED.into())
}

/// Whatever the client has published.
///
/// # Errors
///
/// Returns an error where it has published nothing.
fn published(client: &InputHarness) -> Result<SimSnapshot, Box<dyn Error>> {
    client
        .published()
        .map(|published| *published)
        .ok_or_else(|| "this fixture's client has published no tick to compare".into())
}

/// Whether the world is still holding the client's player up.
///
/// # Errors
///
/// Returns an error where the client has published nothing.
fn held_up(client: &InputHarness) -> Result<bool, Box<dyn Error>> {
    Ok(resting(&published(client)?).1)
}

/// Whether the walk has taken the player off the spawn.
///
/// A player who never moved publishes the same position every tick, and an oracle
/// advanced one tick too few would then agree with the run it is judging.
fn moved(snapshot: &SimSnapshot) -> bool {
    standing_and_facing(snapshot).0 != SPAWN.to_array().map(f32::to_bits)
}
