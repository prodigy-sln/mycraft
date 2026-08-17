//! A change under the content root becomes one reload attempt, and no change
//! becomes none.
//!
//! # What counts an attempt, and why it is not a flag
//!
//! An attempt is counted by what a run of tick boundaries *reported* — a taking up
//! or a refusal — which is the same thing a person sees. There is no accessor for
//! "a build is in flight", and asking for one would move the count these scenarios
//! are about inside the value under test.
//!
//! # Each of these asserts an effect beside the count, and has to
//!
//! "Exactly one attempt" is satisfied by one attempt that did nothing, and every
//! run here would then be reporting on a client that noticed a change and threw the
//! candidate away. So each scenario names the change the author made *and* what the
//! content now serving says because of it: a block that appeared, a block that went,
//! a solidity that changed. A swap that never happened fails the second half while
//! the count still agrees.
//!
//! # The burst is one report, because that is what a report is
//!
//! The port answers with the paths that changed since it was last asked, so five
//! saves inside one settling window are one report carrying five paths — and the
//! coalescing this file grades is the domain's: reports that reach one boundary
//! begin one attempt. **It is one of the two instruments the settling window needs and
//! it does not cover the other**, which is the window the adapter hands
//! its debouncer; that one lives in `mc-world` — `tests/content_watch.rs`, which asks
//! the adapter which window it handed over, and needs no filesystem and no timer.
//! Boundaries are about
//! 16 ms apart and the window is 150, so five saves genuinely spread across one
//! window reach five different boundaries — and nothing here would notice.

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
use std::path::PathBuf;

use input::InputHarness;
use reload::{
    AMBER, AMBER_FILE, GRASS, STONE, STONE_FILE, WATER, WATER_FILE, amber, declaring, restating,
    shipped, stone_that_is_not_solid,
};
use reload_watch::{
    a_client_on, block_path, crossing_a_quiet_run, declaring as serves, ended, serving,
    taken_up_once, the_four_shipped_blocks, until_settled,
};
use reload_world::published_tick;
use support::TestResult;

/// How many times an editor saved one file inside one settling window.
///
/// The scenario's own number. What matters is that it is more than one: an
/// implementation queueing reports would report this many attempts for one edit.
const SAVES_IN_ONE_WINDOW: usize = 5;

#[test]
fn a_declaration_file_that_appears_where_the_loader_reads_begins_one_attempt() -> TestResult {
    let root = shipped()?;
    let (mut client, reports) = a_client_on(&root, GRASS)?;
    let root = declaring(root, AMBER_FILE, &amber())?;

    reports.changed(&[block_path(&root, AMBER_FILE)])?;
    let crossed = until_settled(&mut client);

    assert_eq!(
        (ended(&crossed), serving(&client)?),
        (taken_up_once(), a_new_block_beside_the_shipped_four()),
        "the file the author added is the file the client had never read, so an implementation \
         watching only what it already knew about notices nothing at all. The block appearing in \
         what the client serves is what says the attempt did something: a count of one is just as \
         true of an attempt that read the root and dropped it"
    );
    Ok(())
}

#[test]
fn a_declaration_file_deleted_from_where_the_loader_reads_begins_one_attempt() -> TestResult {
    let root = shipped()?;
    let (mut client, reports) = a_client_on(&root, GRASS)?;
    let root = root.not_declaring_blocks(&[WATER_FILE])?;

    reports.changed(&[block_path(&root, WATER_FILE)])?;
    let crossed = until_settled(&mut client);

    assert_eq!(
        (ended(&crossed), serving(&client)?),
        (taken_up_once(), the_three_left()),
        "a deletion is a change like any other, and the path it names is a path that no longer \
         exists — an implementation that read the changed file rather than the root would find \
         nothing there and have nothing to do. No cell of this world holds water, so dropping its \
         declaration is a candidate the running world can answer for"
    );
    Ok(())
}

#[test]
fn saves_that_reach_one_tick_boundary_begin_one_attempt_and_not_five() -> TestResult {
    let root = shipped()?;
    let (mut client, reports) = a_client_on(&root, GRASS)?;
    let root = restating(root, STONE_FILE, &stone_that_is_not_solid())?;

    reports.changed(&a_burst_of_saves(&root))?;
    let crossed = until_settled(&mut client);

    assert_eq!(
        (ended(&crossed), serving(&client)?),
        (taken_up_once(), stone_no_longer_solid()),
        "one save is several filesystem events and a fast typist is several saves, and a candidate \
         is built from the whole root — so any build started after the last of them observes every \
         one of them and {SAVES_IN_ONE_WINDOW} reports collapse to one attempt with nothing lost. \
         A queue would publish {SAVES_IN_ONE_WINDOW} serials for one edit; refusing while a build \
         is in flight would drop the edit silently, which is the worst outcome available"
    );
    Ok(())
}

#[test]
fn a_root_nothing_has_changed_under_begins_no_attempt_however_many_ticks_pass() -> TestResult {
    let root = shipped()?;
    let (mut client, _reports) = a_client_on(&root, GRASS)?;
    let before = advanced(&client)?;

    let crossed = crossing_a_quiet_run(&mut client);
    let boundaries = u32::try_from(crossed.len())?;

    assert_eq!(
        (ended(&crossed), advanced(&client)? - before),
        (Vec::new(), boundaries),
        "a tick boundary is not a reason to read a content root, and the ticks are counted beside \
         the attempts because a client that stopped advancing would report no attempt for a reason \
         that has nothing to do with the watching. **This is a weak instrument on its own** — a \
         watcher that never fires satisfies it — and its discriminating partners are the scenarios \
         where a real write and an irrelevant one go through the same instrument"
    );
    Ok(())
}

/// The five paths one burst of saves to one declaration reports.
fn a_burst_of_saves(root: &support::content::ContentRoot) -> Vec<PathBuf> {
    (0..SAVES_IN_ONE_WINDOW)
        .map(|_| block_path(root, STONE_FILE))
        .collect()
}

/// The shipped four with a block declared for the first time in front of them.
///
/// `amber.luau` sorts before `dirt.luau` and a root is read in file-name order, so
/// the new block is registered first — which is the same fact that puts it in the
/// player's hand.
fn a_new_block_beside_the_shipped_four() -> Vec<(String, bool)> {
    let mut blocks = vec![serves(AMBER, true)];
    blocks.extend(the_four_shipped_blocks());
    blocks
}

/// The shipped four with the one whose declaration went taken out of them.
fn the_three_left() -> Vec<(String, bool)> {
    the_four_shipped_blocks()
        .into_iter()
        .filter(|(block, _)| block.as_str() != WATER)
        .collect()
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

/// How many ticks this client has published.
///
/// # Errors
///
/// Returns an error where it has published nothing, which is a client with no
/// world rather than one that has advanced no tick.
fn advanced(client: &InputHarness) -> Result<u32, Box<dyn Error>> {
    let published = client
        .published()
        .ok_or("this fixture's client has published no tick to count")?;
    Ok(published_tick(&published))
}
