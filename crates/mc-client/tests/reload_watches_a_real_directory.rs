//! A real save on a real filesystem reaches the running client, and a root that
//! cannot be watched is said so.
//!
//! # The only two scenarios in this phase that go through the shipped watcher
//!
//! Everything else about noticing a change is driven through an in-memory double,
//! deliberately: a suite whose acceptance depends on inotify latency has no
//! deterministic gate. What that leaves ungraded is the adapter itself — the
//! debouncer, the channel, the paths the platform reports — and these two are what
//! grade it. Both go through `watching_shipped_content`, the one door a client
//! goes through, so they also say that door wires the real watcher rather than
//! something that reports nothing.
//!
//! # The one timing-sensitive test in this phase, and how to read a failure
//!
//! The write is a real write and the wait is generous: a save is given fifteen
//! seconds to arrive, and the run then goes on for ten settling windows to see
//! whether a second attempt follows. **A second attempt here is the settling window
//! not reaching the debouncer before it is a flake** — one save is several
//! filesystem events and absorbing them is the whole of what the window is for, so
//! read this as `Duration::ZERO` having reached the builder and check the boundary
//! assertion in `mc-world` before re-running anything.
//!
//! A save that never arrives at all is the adapter not watching: the paths it hands
//! over are the domain's only evidence that anything happened.
//!
//! # A root that is not there is deterministic, and needs no clock at all
//!
//! Watching begins against a path nothing ever created, so the failure is at
//! construction and the first ask reports it. What the scenario requires beside the
//! report is that the run carries on — the ticks keep advancing and the content
//! already loaded goes on serving — because a client that ended the run over an
//! unwatchable directory would be a client that will not start without one.

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
use std::fs;
use std::path::Path;
use std::thread;
use std::time::{Duration, Instant};

use mc_sim::reload::watching_shipped_content;
use tempfile::TempDir;

use input::InputHarness;
use reload::{GRASS, STONE, STONE_FILE, shipped, stone_that_is_not_solid};
use reload_watch::{
    AN_ATTEMPT_MAY_NOT_OUTLAST, Attempt, Refusal, boundary, crossing_a_quiet_run, ended, naming,
    refusal_naming, serving, solidity_of, taken_up_once, the_four_shipped_blocks,
};
use reload_world::{floor_of, playing, published_tick, standing};
use support::TestResult;
use support::content::{BLOCK_DIRECTORY, ContentRoot};

/// How long a real save is given to reach the client.
const WAITING_FOR_A_SAVE: Duration = Duration::from_secs(15);

/// How long a boundary waits for the next one.
///
/// A frame is about sixteen milliseconds and this is faster, so the run crosses
/// more boundaries than a windowed client would in the same time — which is the
/// direction that would catch an attempt beginning more than once.
const BETWEEN_BOUNDARIES: Duration = Duration::from_millis(5);

/// A directory name nothing creates.
const A_ROOT_THAT_IS_NOT_THERE: &str = "content-that-was-never-put-here";

#[test]
fn a_declaration_saved_on_disk_while_the_client_is_running_begins_one_attempt() -> TestResult {
    let root = shipped()?;
    let (simulation, holding) = playing(root.path(), standing(), |registry| {
        floor_of(registry, GRASS)
    })?;
    let mut client = InputHarness::started();
    client.play(simulation, holding);
    client.attach_reload(watching_shipped_content(root.path().to_owned()));

    fs::write(
        root.path().join(BLOCK_DIRECTORY).join(STONE_FILE),
        stone_that_is_not_solid().text(),
    )?;
    let crossed = crossing_at_a_human_pace(&mut client);

    assert_eq!(
        (ended(&crossed), solidity_of(&client, STONE)?),
        (taken_up_once(), Some(false)),
        "this is the capability: the author saves the file they were editing and the running game \
         reads it, with no relaunch and nothing else asked of them. It is the only test in this \
         phase that touches a real filesystem, so it is also the only one that would notice the \
         adapter reporting nothing at all — and the only one that would notice a save arriving as \
         several attempts, which is the settling window failing to reach the debouncer"
    );
    Ok(())
}

#[test]
fn a_content_root_that_cannot_be_watched_is_reported_once_and_the_run_carries_on() -> TestResult {
    let root = shipped()?;
    let elsewhere = TempDir::new()?;
    let absent = elsewhere.path().join(A_ROOT_THAT_IS_NOT_THERE);
    let mut client = a_client_watching(&root, &absent)?;
    let before = advanced(&client)?;

    let crossed = crossing_a_quiet_run(&mut client);
    let boundaries = u32::try_from(crossed.len())?;

    assert_eq!(
        (
            refusal_naming(&crossed, &naming(&[&absent.display().to_string()])),
            advanced(&client)? - before,
            serving(&client)?
        ),
        (
            Refusal::NamedEverythingAsked,
            boundaries,
            the_four_shipped_blocks()
        ),
        "somebody who moved their content directory is told which directory could not be watched, \
         once, and goes on playing the content the run already loaded. The ticks and the blocks are \
         counted beside the refusal because ending the run would satisfy 'no further attempt' \
         perfectly — and because a client that reported this on every boundary would bury the \
         terminal in it"
    );
    Ok(())
}

/// A client playing the root at `root` while watching `watched`.
///
/// The two are the same directory in every scenario but the unwatchable one, which
/// is what that scenario is: content that was read and a directory that is no
/// longer there to watch.
///
/// # Errors
///
/// Returns an error if the root does not read, the world does not build, or the
/// content declares no solid block.
fn a_client_watching(root: &ContentRoot, watched: &Path) -> Result<InputHarness, Box<dyn Error>> {
    let (simulation, holding) = playing(root.path(), standing(), |registry| {
        floor_of(registry, GRASS)
    })?;
    let mut client = InputHarness::started();
    client.play(simulation, holding);
    client.attach_reload(watching_shipped_content(watched.to_owned()));
    Ok(client)
}

/// Boundaries crossed at a human pace until an attempt has ended and the run has
/// gone on long enough to see a second one.
///
/// Gives up after [`WAITING_FOR_A_SAVE`] if nothing is ever reported, so the
/// assertion is what says a save never arrived rather than a hung test.
fn crossing_at_a_human_pace(client: &mut InputHarness) -> Vec<Option<Attempt>> {
    let started = Instant::now();
    let mut crossed = Vec::new();
    let mut reported = None;
    while waiting(started, reported) {
        let attempt = boundary(client);
        if attempt.is_some() {
            reported = reported.or_else(|| Some(Instant::now()));
        }
        crossed.push(attempt);
        thread::sleep(BETWEEN_BOUNDARIES);
    }
    crossed
}

/// Whether a run that started at `started` and first reported at `reported` has
/// more to wait for.
fn waiting(started: Instant, reported: Option<Instant>) -> bool {
    match reported {
        None => started.elapsed() < WAITING_FOR_A_SAVE,
        Some(first) => first.elapsed() < AN_ATTEMPT_MAY_NOT_OUTLAST,
    }
}

/// How many ticks this client has published.
///
/// # Errors
///
/// Returns an error where it has published nothing.
fn advanced(client: &InputHarness) -> Result<u32, Box<dyn Error>> {
    let published = client
        .published()
        .ok_or("this fixture's client has published no tick to count")?;
    Ok(published_tick(&published))
}
