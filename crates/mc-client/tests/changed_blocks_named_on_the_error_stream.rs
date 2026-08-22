//! The line a player reads when their world opened over blocks that have moved,
//! and the four cases that must produce no line at all.
//!
//! # Three of these are absence assertions and one of them carries them
//!
//! "No line" is satisfied by a client that never composes one, so the readings
//! below are worth nothing without the two that say a line *is* composed. They
//! are in this file for that reason rather than for tidiness: an over-eager
//! composer fails the three, an inert one fails the two, and nothing in between
//! passes all five.
//!
//! **What no test in this file can see is the client failing to print.** Every
//! reading here reaches the composer through a launch, which is the shape
//! `testing.md` §2 calls *policy is not wiring*: a composer answering correctly
//! while nothing calls its `say_*` sibling leaves all five green.
//! `tests/shipped_binary.rs` is the instrument for that, and it is a real
//! process for exactly this reason.
//!
//! # The words are the artefact
//!
//! The expected sentences are written out rather than assembled from the client's
//! own constants, on `notice_test.rs`'s rule: what a player reads is the thing
//! being verified, and a test that composed it from the same pieces the client
//! does would agree with the client about a rewording neither of them noticed.
//!
//! # The singular and the plural are both here
//!
//! One block is the common case and more than one is the case the line exists
//! for, and a sentence that reads correctly for one of them reads wrongly for the
//! other. So both counts are asserted, and the second of them over the content
//! this repository actually ships.

#[path = "support/changed_blocks.rs"]
mod changed_blocks;
#[path = "support/persistence.rs"]
mod persistence;
mod support;

use std::error::Error;
use std::path::PathBuf;
use std::sync::Arc;

use mc_client::launch::simulation_to_play;
use mc_client::notice::changed_blocks;
use mc_client::startup::acceptance_from;
use mc_sim::persistence::Launching;
use tempfile::TempDir;

use changed_blocks::{
    NO_ARGUMENT, a_save_whose_block_only_looks_different, a_save_whose_blocks_are_all_unchanged,
    a_save_whose_two_blocks_were_redeclared, launch, line_of,
};
use persistence::{TestResult, refusal, save_in};

/// The line a launch over the fixture's two redeclared blocks writes.
///
/// Ascending, both of them, and nothing about the block that did not change or
/// the one that only looks different.
const BOTH_OF_THEM: &str = "mycraft: `fixture:alpha`, `fixture:omega` no longer behave as they did \
                            when this world was saved, and it was loaded anyway";

/// The line a launch over the committed pre-Luau save writes against the content
/// this repository ships.
const WATER_ALONE: &str = "mycraft: `base:water` no longer behaves as it did when this world was \
                           saved, and it was loaded anyway";

/// The save written before this repository's blocks were Luau, relative to the
/// repository root.
///
/// **Never regenerated, and read rather than copied.** It is an oracle precisely
/// because it predates the declarations under test; a save this suite wrote would
/// agree with them by construction. Nothing here writes to it.
const OLDER_SAVE: [&str; 4] = ["crates", "mc-world", "tests", "fixtures"];
const OLDER_SAVE_FILE: &str = "world_saved_against_the_toml_declarations.mcw";

#[test]
fn a_save_whose_two_blocks_behave_differently_is_named_one_line_ascending() -> TestResult {
    let save = a_save_whose_two_blocks_were_redeclared()?;
    let path = save_in(&save.directory);

    let launched = launch(&save, &path, &NO_ARGUMENT)?;

    assert_eq!(
        line_of(&launched),
        Ok(Some(BOTH_OF_THEM.to_owned())),
        "a player whose world opened over blocks that have moved gets one line, and it names every \
         one of them in the order they read rather than the order a save's table happened to hold \
         them in. It is the only thing they are told, so a line naming one of two leaves them \
         hunting the other. It answered: {}",
        refusal(&launched)
    );
    Ok(())
}

#[test]
fn a_save_whose_blocks_all_still_match_is_loaded_with_nothing_said_about_them() -> TestResult {
    let save = a_save_whose_blocks_are_all_unchanged()?;
    let path = save_in(&save.directory);

    let launched = launch(&save, &path, &NO_ARGUMENT)?;

    assert_eq!(
        line_of(&launched),
        Ok(None),
        "nothing about this save's blocks has changed, so there is nothing to say and saying it \
         anyway would put a line on every player's terminal on every run — which is how the one \
         line that matters stops being read. It answered: {}",
        refusal(&launched)
    );
    Ok(())
}

#[test]
fn a_save_whose_block_only_looks_different_is_loaded_with_nothing_said_about_it() -> TestResult {
    let save = a_save_whose_block_only_looks_different()?;
    let path = save_in(&save.directory);

    let launched = launch(&save, &path, &NO_ARGUMENT)?;

    assert_eq!(
        line_of(&launched),
        Ok(None),
        "one block draws from a different texture key and behaves exactly as it always did. That is \
         an art edit, and a line after every art edit is the noise the line about a rebalance would \
         hide in — which is the same reasoning that keeps a retexture out of the strict refusal. It \
         answered: {}",
        refusal(&launched)
    );
    Ok(())
}

#[test]
fn a_launch_with_no_save_to_read_generates_a_world_and_says_nothing_about_blocks() -> TestResult {
    let save = a_save_whose_two_blocks_were_redeclared()?;
    let nowhere = TempDir::new()?;

    let launched = launch(&save, &save_in(&nowhere), &NO_ARGUMENT)?;

    assert_eq!(
        line_of(&launched),
        Ok(None),
        "there is no save at this path, so this is a first launch: a world is generated and no save \
         recorded anything for the content to disagree with. A client that reported the changed \
         blocks here would be reporting them about a world that has just been built from the very \
         declarations it is comparing against. It answered: {}",
        refusal(&launched)
    );
    Ok(())
}

#[test]
fn the_committed_pre_luau_save_names_water_and_no_other_block_against_the_shipped_content()
-> TestResult {
    let launched = over_the_older_save()?;

    assert_eq!(
        line_of(&launched),
        Ok(Some(WATER_ALONE.to_owned())),
        "this is the shipped content against a save written before its blocks were Luau, which is \
         the one comparison in this suite neither side of which was written to agree with the \
         other. `base:water` states `breakable = false` and lands in the line; the other three \
         blocks the save holds differ only in the keys they draw from and must not. A line naming \
         four blocks is a revision byte shared between the two halves of a block's record. It \
         answered: {}",
        refusal(&launched)
    );
    Ok(())
}

/// A launch over the committed save, against the content this repository ships,
/// with nothing on the command line.
///
/// # Errors
///
/// Returns an error if the repository root cannot be located, if the shipped
/// content cannot be read, or if the content a simulation publishes cannot be
/// assembled.
fn over_the_older_save() -> Result<persistence::Launched, Box<dyn Error>> {
    let registry = Arc::new(support::content_registry()?);
    Ok(simulation_to_play(
        &older_save()?,
        Launching {
            seed: mc_sim::REPLAY_SEED,
            registry: Arc::clone(&registry),
            content: support::published_content(&registry)?,
            accepting: acceptance_from(NO_ARGUMENT.iter().map(|argument| (*argument).to_string())),
        },
    ))
}

/// Where the committed pre-Luau save sits, located from the repository rather
/// than from the directory this test binary happens to start in.
///
/// # Errors
///
/// Returns an error if the repository root cannot be located.
fn older_save() -> Result<PathBuf, Box<dyn Error>> {
    let mut path = support::repository_root()?;
    for component in OLDER_SAVE {
        path.push(component);
    }
    path.push(OLDER_SAVE_FILE);
    Ok(path)
}

/// Named so the composer this file is about is reachable by the one reading that
/// asks it directly, rather than only through a launch.
///
/// A list held by no save at all is the input the launches above cannot supply,
/// and it is what says the emptiness rule lives in the composer rather than in
/// the loads that happen to hand it empty lists.
#[test]
fn the_composer_says_nothing_for_a_list_holding_nothing() {
    assert_eq!(
        changed_blocks(&[]),
        None,
        "the rule is the composer's own and not a property of the saves above: handed no names it \
         answers with no line, so a caller cannot print an empty sentence by forgetting to check"
    );
}
