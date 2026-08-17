//! Which world a launch plays: the one a save holds, the one the generator
//! makes, or none at all.
//!
//! The three scenarios here are each other's controls, and none of them can be
//! satisfied by the answer another one wants. A launch that always generated
//! plays the generated world where a save exists; a launch that always loaded
//! refuses where none does; a launch that propagated every refusal would never
//! generate at all. The one distinction all three rest on is between a save that
//! is *not there* and a save that is there and cannot be read — collapse the two
//! and a missing save generates a world over an unreadable one.
//!
//! **Nothing here is asserted through the loader.** Every answer is read off the
//! simulation the launch handed back, because the two entry points onto one read
//! path are indistinguishable to anything that asks the loader directly.

mod support;

use std::fs;
use std::path::Path;

use mc_sim::persistence::simulation_at_launch;
use mc_world::persistence::{Acceptance, requirements, save_world};

use support::launch::{
    EVERY_DECLARED_CELL, MARKER, MARKER_CELL, a_world_to_launch_into, against_generated, answered,
    beneath, generated_with_the_marker, held_at, launching, recorded_player, save_path,
};
use support::{NOTHING, TestResult, block_at};

/// A file that exists, is long enough to be a save, and is not one.
///
/// Its refusal is one this project raises by name over bytes it read itself, and
/// not one the encoder produced — the encoder is a library treated as working,
/// and how it classifies a corrupt input is nobody's contract here.
const NOT_A_SAVE: &[u8] =
    b"this file is not a save, and a launch that meets one has to say so by name";

/// The decision a launch is asked to make when nothing about the blocks has
/// changed.
///
/// Every save in this file is written against the registry it is then read
/// against, so no block can be missing or redeclared and the acceptance never
/// decides anything. That is deliberate: a fixture whose blocks had changed
/// would be asserting a different scenario's refusal.
const ACCEPTING: Acceptance = Acceptance::OnlyUnchangedBlocks;

#[test]
fn a_launch_with_a_readable_save_plays_in_the_world_the_save_holds() -> TestResult {
    let (registry, generated, directory) = a_world_to_launch_into()?;
    let save = save_path(&directory);
    let held = generated_with_the_marker(&generated, &registry)?;
    save_world(&save, &held, recorded_player(), &registry)?;

    let launched = simulation_at_launch(&save, launching(&registry, ACCEPTING)?);

    let (x, y, z) = MARKER_CELL;
    assert_eq!(
        (
            held_at(&launched, MARKER_CELL),
            block_at(&generated, x, y, z)?
        ),
        (Ok(MARKER.to_owned()), NOTHING.to_owned()),
        "a launch plays the world its save holds: the save holds `{MARKER}` at ({x}, {y}, {z}) \
         and the generated world holds {NOTHING} there. `{MARKER}` is a name the generator \
         cannot produce, so the two worlds agreeing is not something this can pass by. The \
         launch answered {launched:?}"
    );
    Ok(())
}

#[test]
fn a_launch_with_no_save_plays_the_world_the_generator_makes() -> TestResult {
    let (registry, generated, directory) = a_world_to_launch_into()?;
    let save = save_path(&directory);

    let launched = simulation_at_launch(&save, launching(&registry, ACCEPTING)?);

    assert_eq!(
        (against_generated(&launched, &generated), save.exists()),
        (Ok((EVERY_DECLARED_CELL, Vec::new())), false),
        "with nothing at {}, a launch plays the world the generator makes — every one of \
         {EVERY_DECLARED_CELL} declared cells holding what the generated world holds there, \
         which an empty world and a refused start both fail. The launch answered {launched:?}",
        save.display()
    );
    Ok(())
}

/// **This asserts structure, not what a person reads, and the difference is the
/// point.** What it holds the launch to is the shape of the refusal: this layer
/// names the file, because this is the level that knows where the file is; it
/// does not restate the reason, because a message quoting its own source has
/// that source read out twice by anything that walks the chain; and it carries
/// the reason as its source, because that is now the only thing keeping the
/// reason reachable at all.
///
/// **What a turned-away player actually reads is witnessed in the client's own
/// reporting, deliberately not here.** Two reasons, and both are binding. This
/// crate may not resolve the crate that renders a refusal, in any dependency
/// kind — so the text cannot be produced here at all. And the alternative,
/// walking `source()` here and asserting the string it would join into, is a
/// second renderer asserted against its own output: it would claim a guarantee
/// about a terminal while reaching no printing whatsoever, which is exactly the
/// shape of test that once stayed green for years over a client that printed one
/// sentence. A test that read as though it witnessed a refusal a player sees
/// would hand the next reader that same false confidence, in a crate where
/// nothing could catch it.
#[test]
fn a_launch_with_a_save_it_cannot_read_refuses_naming_the_file_and_carrying_the_reason()
-> TestResult {
    let (registry, _, directory) = a_world_to_launch_into()?;
    let save = save_path(&directory);
    written(&save, NOT_A_SAVE)?;
    let reason = why_it_cannot_be_read(&save)?;

    let launched = simulation_at_launch(&save, launching(&registry, ACCEPTING)?);

    let answer = answered(&launched);
    assert_eq!(
        (
            answer.contains(&save.display().to_string()),
            answer.contains(&reason),
            beneath(&launched),
            launched.is_ok()
        ),
        (true, false, reason.clone(), false),
        "a save that is there and cannot be read refuses the start rather than generating a new \
         world over it, and the refusal splits the two things a player needs across the two \
         levels that know them. This level knows where the file is and says so. It does not \
         restate `{reason}`, which the level beneath it says — a message quoting its own source \
         has that source read out twice by anything that walks the chain. And it carries that \
         level rather than dropping it, which is now the only thing keeping the reason reachable \
         at all. The launch answered: {answer}"
    );
    Ok(())
}

/// Writes `bytes` at `path`, making the directories above it first.
fn written(path: &Path, bytes: &[u8]) -> TestResult {
    fs::create_dir_all(
        path.parent()
            .ok_or("the save path has no directory above it")?,
    )?;
    fs::write(path, bytes)?;
    Ok(())
}

/// Why the save at `path` cannot be read, as this project's own reader words it.
///
/// **Asked through a different entry point onto the same file**, so the reason a
/// launch has to quote is derived rather than copied out of a message: a fixture
/// spelling the wording by hand would agree with a launch that quoted the wrong
/// refusal as readily as with one that quoted the right one.
fn why_it_cannot_be_read(path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    match requirements(path) {
        Ok(_) => Err("the fixture is readable as a save, so no launch would refuse it".into()),
        Err(refusal) => Ok(refusal.to_string()),
    }
}
