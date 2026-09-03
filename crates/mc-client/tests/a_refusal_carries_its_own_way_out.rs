//! The way out travels with the failure, so the site reporting it cannot leave it
//! behind.
//!
//! # What was wrong with it being an argument
//!
//! `Ending::failed(failure, guidance)` took the sentence as a second parameter,
//! and what held the property was that the parameter was not optional and that the
//! ending could not be built outside its three doors. **That is a shape argument
//! and not a test**: the one production line that can actually emit the sentence
//! runs inside a redraw needing a graphics device and a display server, so no test
//! reaches it — and PRO-940 measured the consequence, that replacing the guidance
//! argument on that line leaves the whole suite green.
//!
//! Every suite reading of the sentence *supplied it itself*, which is why. A test
//! calling `Ending::failed(&refused, &refused.way_out())` proves the constructor
//! concatenates two strings it was handed. It says nothing about whether the
//! client would have handed it the right one.
//!
//! # So the readings below hand it nothing
//!
//! With the guidance a property of the failure, `Ending::failed(&refused)` is the
//! whole call and a site has nothing left to get wrong. These ask what a player
//! reads for a refusal that has a way out, for one that has none, and for the
//! refusal a *running session* reports — which is the scenario PRO-940 says
//! nothing can currently reach, and which is reachable here precisely because the
//! guidance no longer travels with the call.
//!
//! # The sentences are written out, never asked for
//!
//! `notice_test.rs`'s rule, and it is the whole point here. Building the expected
//! text out of `way_out()` would be the defect one level up: a reading that agrees
//! with whatever the failure says, including nothing at all.

mod support;

use std::error::Error;
use std::path::PathBuf;

use mc_client::startup::PreparationError;
use mc_core::id::BlockName;
use mc_render::window::Ending;
use mc_sim::persistence::LaunchError;
use mc_world::persistence::LoadError;

use support::{TestResult, reported};

/// Where the client reads its save from, relative to the directory it was started
/// in — the path a refusal about a save names.
const SAVE: [&str; 2] = ["saves", "world.mcw"];

/// The block a save and the content disagree about, for the refusal that has a
/// way out.
const THE_CHANGED_BLOCK: &str = "base:water";

/// The whole way-out sentence, including the separator it carries.
///
/// Written out rather than asked of the failure. A reading that built this from
/// `way_out()` would pass against a client that had stopped saying anything.
const DROP_THE_ARGUMENT: &str = ". Those blocks are no longer what they were when this world was \
                                 saved; drop `--refuse-changed-blocks` to load it anyway";

#[test]
fn a_refusal_the_player_asked_for_is_reported_with_the_way_out_the_failure_carries() -> TestResult {
    let refused = a_save_refused_only_for_a_redeclared_block()?;

    let said = reported(&Ending::failed(&refused))?;

    assert!(
        said.ends_with(&format!("{DROP_THE_ARGUMENT}\n")),
        "this is the only refusal in the client a player can undo, and the sentence saying how is \
         the whole of what they get. Nothing was handed to the constructor here: if it reaches the \
         terminal it is because the failure carried it. What was said was:\n{said}"
    );
    Ok(())
}

#[test]
fn a_refusal_with_no_way_out_is_reported_with_no_guidance_appended() -> TestResult {
    let refused = PreparationError::NoContentRoot {
        root: ["content", "base"].iter().collect::<PathBuf>(),
    };

    let said = reported(&Ending::failed(&refused))?;

    assert_eq!(
        said,
        format!("mycraft: {refused}\n"),
        "a content root that is not there is not something an argument changes, so there is \
         nothing to tell anybody beyond the refusal itself. A constructor that appended a sentence \
         of its own would put advice under every refusal in the client, and advice that does not \
         apply is worse than none — it sends somebody to try a flag that will refuse them again"
    );
    Ok(())
}

#[test]
fn the_refusal_a_running_session_reports_carries_the_same_way_out_it_carries_at_launch()
-> TestResult {
    let collected = a_save_refused_only_for_a_redeclared_block()?;

    let in_session = reported(&Ending::failed(&collected))?;
    let at_launch = reported(&Ending::failed(
        &a_save_refused_only_for_a_redeclared_block()?,
    ))?;

    assert_eq!(
        (in_session.as_str(), in_session.contains(DROP_THE_ARGUMENT)),
        (at_launch.as_str(), true),
        "a save refusal is discovered on the preparation worker and surfaces where the frame path \
         collects it, inside a redraw — the one production line that emits this sentence, and the \
         one no test can reach. What makes the two agree now is that neither site is asked for the \
         guidance: it is the failure's, so a site cannot supply the wrong thing or nothing at all. \
         The second element is what stops this reading passing on two reports that are equally \
         empty"
    );
    Ok(())
}

/// The refusal a launch gives for a save whose one block has been redeclared —
/// loadable data the player asked to be turned away from.
///
/// The one shape in this client that has a way out at all: the argument the player
/// typed is the whole of what stands between them and their world, so dropping it
/// is advice that works. A missing name is deliberately not one of these.
///
/// # Errors
///
/// Returns the parse failure of a fixture block name that is not a namespaced id,
/// which is a broken fixture rather than a claim about the reporting.
fn a_save_refused_only_for_a_redeclared_block() -> Result<PreparationError, Box<dyn Error>> {
    Ok(PreparationError::Launch(LaunchError::Load {
        save: SAVE.iter().collect(),
        source: Box::new(LoadError::Unresolvable {
            missing: Vec::new(),
            changed: vec![BlockName::parse(THE_CHANGED_BLOCK)?],
        }),
    }))
}
