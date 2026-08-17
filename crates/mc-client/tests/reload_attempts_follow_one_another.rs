//! One attempt follows another: a change that arrives during a build, a builder
//! that never comes back, and a change that arrives before there is a tick at all.
//!
//! # What each of these is really about is that the watching survives
//!
//! An attempt that ends badly must not end the *watching*, and none of the three
//! failures here is visible in the attempt that met it: a queue would run one build
//! per save, a refusal-while-busy would drop an edit silently, a lost worker could
//! wedge the flag for the rest of the run, and a change reported before the world
//! landed could be forgotten. Every scenario below therefore ends with a candidate
//! that *did* land, and the effect it had is the assertion's other half.
//!
//! # A builder that ends without producing anything
//!
//! The worker is a thread, and a thread can stop without an answer. The reload's
//! own contract for that is a refusal like any other — the previous content goes on
//! serving and a person is told once — and the clause that costs the most to get
//! wrong is the third: later changes still get attempts. Reporting it twice would be
//! deduplicated and invisible, so the builder here is lost **once** and reads the
//! root properly afterwards, which is what makes all three clauses observable in one
//! run.
//!
//! The losing builder is handed over at construction, beside the watch, because a
//! thread that dies is not a state a fixture can reach by writing files. It carries
//! its "once" in a process-wide flag, which is sound here because the test runner
//! gives every test its own process — one test in this binary reads it, and its
//! transition is one-way.
//!
//! # A change before the first tick
//!
//! A client watches its content root from before there is a world: the preparation
//! is still generating one, no tick has been advanced, and a swap into a world no
//! tick has run is not something to do — it is something to hold. So the change is
//! remembered by the same flag that coalesces a burst, and the first boundary after
//! the world lands is the one that spends it.

#[path = "support/input/mod.rs"]
mod input;
#[path = "support/reload.rs"]
mod reload;
#[path = "support/reload_watch.rs"]
mod reload_watch;
#[path = "support/reload_world.rs"]
mod reload_world;
mod support;

use std::error::Error;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use mc_core::content::LayerAssignment;
use mc_sim::content::{ContentError, LoadedContent};
use mc_sim::reload::ContentReload;

use input::InputHarness;
use reload::{
    Declaration, GRASS, STONE, STONE_FILE, WATER, WATER_FILE, restating, shipped,
    stone_that_is_not_solid,
};
use reload_watch::{
    Attempt, Refusal, Reports, a_client_on, a_client_with_no_world, block_path, boundary,
    crossing_a_quiet_run, declaring as serves, ended, naming, refusal_naming, serving, solidity_of,
    taken_up_once, the_four_shipped_blocks, until_settled, watch,
};
use reload_world::{floor_of, playing, standing};
use support::TestResult;
use support::content::ContentRoot;

/// Whether the builder below has already ended a thread.
///
/// Process-wide because a `fn` pointer captures nothing, and sound because the test
/// runner gives every test its own process: one test in this binary reads it, and it
/// only ever goes one way.
static A_BUILD_HAS_BEEN_LOST: AtomicBool = AtomicBool::new(false);

#[test]
fn a_change_reported_while_a_candidate_is_being_built_begins_exactly_one_further_attempt()
-> TestResult {
    let root = shipped()?;
    let (mut client, reports) = a_client_on(&root, GRASS)?;
    let root = restating(root, STONE_FILE, &stone_that_is_not_solid())?;

    reports.changed(&[block_path(&root, STONE_FILE)])?;
    let started = boundary(&mut client);
    let root = restating(root, WATER_FILE, &water_that_is_solid())?;
    reports.changed(&[block_path(&root, WATER_FILE)])?;
    let crossed = until_settled(&mut client);

    assert_eq!(
        (started, ended(&crossed), serving(&client)?),
        (None, taken_up_once_more(), both_edits_landed()),
        "a save while a build is in flight is an edit the author expects to see, so it becomes one \
         further attempt at the boundary after that build ends: a queue would run one build per \
         save and publish a serial for each, and refusing while busy would drop the edit with \
         nothing said. The boundary that started the first build reports nothing, which is the \
         other half of 'off the tick thread' — and both edits being in force at the end is what \
         says the further attempt read the root rather than merely running"
    );
    Ok(())
}

#[test]
fn a_builder_that_ends_without_a_candidate_is_reported_once_and_the_next_change_still_lands()
-> TestResult {
    let root = shipped()?;
    let (mut client, reports) = a_client_whose_first_builder_is_lost(&root)?;
    let root = restating(root, STONE_FILE, &stone_that_is_not_solid())?;

    reports.changed(&[block_path(&root, STONE_FILE)])?;
    let lost = until_settled(&mut client);
    let still_serving = serving(&client)?;
    reports.changed(&[block_path(&root, STONE_FILE)])?;
    let recovered = until_settled(&mut client);

    assert_eq!(
        (
            refusal_naming(&lost, &naming(&[])),
            still_serving,
            ended(&recovered),
            solidity_of(&client, STONE)?
        ),
        (
            Refusal::NamedEverythingAsked,
            the_four_shipped_blocks(),
            taken_up_once(),
            Some(false)
        ),
        "a worker that stops without an answer leaves the run with the content it had, tells the \
         person once, and does not end the watching — the third clause is the one that costs, \
         because a flag left set by a lost build would swallow every later save in silence. No \
         needle is asked of the refusal's words: nothing else in the workspace produces that \
         sentence, so there is no second reader to compare it against, and what the scenario \
         requires is that exactly one refusal reached a person"
    );
    Ok(())
}

#[test]
fn a_change_reported_before_any_tick_is_held_until_a_tick_boundary_exists() -> TestResult {
    let root = shipped()?;
    let (simulation, holding) = playing(root.path(), standing(), |registry| {
        floor_of(registry, GRASS)
    })?;
    let (mut client, reports) = a_client_with_no_world(&root);
    let root = restating(root, STONE_FILE, &stone_that_is_not_solid())?;

    reports.changed(&[block_path(&root, STONE_FILE)])?;
    let before_a_world = crossing_a_quiet_run(&mut client);
    let published_nothing = client.published().is_none();
    client.play(simulation, holding);
    let after_a_world = until_settled(&mut client);

    assert_eq!(
        (
            ended(&before_a_world),
            published_nothing,
            ended(&after_a_world),
            solidity_of(&client, STONE)?
        ),
        (Vec::new(), true, taken_up_once(), Some(false)),
        "a client watches its content root from before it has a world, so an author who saves \
         while the world is still being generated is an author whose edit has to be waiting when \
         it lands. Swapping into a world no tick has run is the alternative and it is not one: \
         there is no boundary there. The change is held by the same flag that collapses a burst, \
         which is why it costs nothing"
    );
    Ok(())
}

/// Water as it ships, made solid — the second edit, whose effect is separate from
/// the first one's.
///
/// No cell of these worlds holds water, so the candidate is one the running world
/// can answer for, and `solid` is a field the published content carries.
fn water_that_is_solid() -> Declaration {
    Declaration::of(WATER).solid(true)
}

/// Two attempts, which is what one edit during a build has to become.
fn taken_up_once_more() -> Vec<Attempt> {
    vec![Attempt::TakenUp, Attempt::TakenUp]
}

/// The shipped four with stone's solidity taken away and water's given to it.
fn both_edits_landed() -> Vec<(String, bool)> {
    the_four_shipped_blocks()
        .into_iter()
        .map(|(block, solid)| match block.as_str() {
            STONE => serves(STONE, false),
            WATER => serves(WATER, true),
            _ => (block, solid),
        })
        .collect()
}

/// A client whose reload builds through a worker that is lost the first time it is
/// asked, and the handle its changes are reported on.
///
/// # Errors
///
/// Returns an error if the root does not read or the world does not build.
fn a_client_whose_first_builder_is_lost(
    root: &ContentRoot,
) -> Result<(InputHarness, Reports), Box<dyn Error>> {
    let (simulation, holding) = playing(root.path(), standing(), |registry| {
        floor_of(registry, GRASS)
    })?;
    let (watching, reports) = watch();
    let mut client = InputHarness::started();
    client.play(simulation, holding);
    client.attach_reload(ContentReload::building(
        root.path().to_owned(),
        Box::new(watching),
        lost_once_then_reading,
    ));
    Ok((client, reports))
}

/// A build that ends the thread it was called on the first time, and reads the
/// content root every time after.
///
/// **The panic is the subject**: a worker that stops without an answer is what
/// `join` reports as a lost thread, and there is no way to write a file that
/// produces one. The message goes to the terminal, which is expected output for
/// this test rather than a failure.
fn lost_once_then_reading(
    root: &Path,
    spent: &LayerAssignment,
) -> Result<LoadedContent, ContentError> {
    if A_BUILD_HAS_BEEN_LOST.swap(true, Ordering::SeqCst) {
        return mc_sim::content::load(root, spent);
    }
    unimplemented!(
        "a builder this fixture supplied so that a lost worker is a state a scenario can reach"
    )
}
