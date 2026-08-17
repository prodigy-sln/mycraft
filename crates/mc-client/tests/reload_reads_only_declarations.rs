//! Which changes under one content root are content, and which are somebody
//! else's files.
//!
//! # A content root is one root, and the loader reads two directories of it
//!
//! `mc_sim::content::load` reads `blocks/` and `hud/` and nothing else — the
//! material files beside them are read by `tools/voxforge` alone, and no block
//! declaration names one. So the rule that decides whether a change begins an
//! attempt has to be the loader's own, and these scenarios are what say it is: a
//! HUD declaration is content, an editor's scratch file next to a declaration is
//! not, and a material file is not.
//!
//! # An absence is asserted over a root that would have answered differently
//!
//! Each scenario here edits a declaration for real and then reports a path that is
//! not it. So "no attempt began" is not merely a count of zero: had the client read
//! the root at all it would be serving something else, and both halves of every
//! assertion below say which. A watcher that never fired and a relevance rule that
//! had come to refuse everything satisfy the count and fail nothing — which is why
//! the material scenario carries its discriminating half **in the same run, through
//! the same instrument**: one client, one watch, one edit, two reports.
//!
//! # The material file this names is the one the shipped root really holds
//!
//! `content/base/materials/dirt.toml` is a file in this repository, so the negative
//! case is a path a real save would produce rather than one invented for a test.
//! Content work under that directory is ordinary and cannot reach these scenarios;
//! a fixture that leant on it as a control would be leaning on a control nobody can
//! schedule, so every root below is the fixture's own copy.

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
use std::fs;
use std::path::PathBuf;

use input::InputHarness;
use reload::{GRASS, STONE, STONE_FILE, restating, shipped, stone_that_is_not_solid};
use reload_content::{
    CROSSBAR_FILE, SHIPPED_CROSSBAR_EXTENT, WIDENED_CROSSBAR, WIDENED_CROSSBAR_EXTENT,
    crossbar_extent,
};
use reload_watch::{
    A_MATERIAL_FILE, SCRATCH_SUFFIX, a_client_on, block_path, crossing_a_quiet_run, ended,
    hud_path, material_path, solidity_of, taken_up_once, until_settled,
};
use support::TestResult;
use support::content::{ContentRoot, HUD_DIRECTORY};

#[test]
fn a_hud_declaration_written_under_the_same_root_begins_an_attempt() -> TestResult {
    let root = shipped()?;
    let (mut client, reports) = a_client_on(&root, GRASS)?;
    let root = widening_the_crossbar(root)?;
    let before = crossbar_published(&client)?;

    reports.changed(&[hud_path(&root, CROSSBAR_FILE)])?;
    let crossed = until_settled(&mut client);

    assert_eq!(
        (ended(&crossed), before, crossbar_published(&client)?),
        (
            taken_up_once(),
            Some(SHIPPED_CROSSBAR_EXTENT),
            Some(WIDENED_CROSSBAR_EXTENT)
        ),
        "a crosshair the content declares is content exactly as a block is, and the root a reload \
         reads is the whole root: a rule that watched only the block directory would leave an \
         author widening a HUD element with nothing happening until they relaunched. The extent \
         the client now publishes is what says the attempt read the root rather than merely \
         beginning"
    );
    Ok(())
}

#[test]
fn an_editors_scratch_file_beside_a_declaration_begins_no_attempt() -> TestResult {
    let root = shipped()?;
    let (mut client, reports) = a_client_on(&root, GRASS)?;
    let root = restating(root, STONE_FILE, &stone_that_is_not_solid())?;
    let scratch = scratch_file_beside(&root);

    reports.changed(&[scratch])?;
    let crossed = crossing_a_quiet_run(&mut client);

    assert_eq!(
        (ended(&crossed), solidity_of(&client, STONE)?),
        (Vec::new(), Some(true)),
        "an editor writes its own files beside the one it is saving, and the loader opens none of \
         them: `{STONE_FILE}{SCRATCH_SUFFIX}` carries the declaration extension inside its name \
         and not at the end of it. The declaration in this root has already been edited, so a \
         client that read the root on this report would be serving stone as not solid — the \
         solidity is what makes the count of zero mean something"
    );
    Ok(())
}

#[test]
fn a_material_file_begins_no_attempt_while_the_same_watcher_begins_one_for_a_declaration()
-> TestResult {
    let root = shipped()?;
    let (mut client, reports) = a_client_on(&root, GRASS)?;
    let root = restating(root, STONE_FILE, &stone_that_is_not_solid())?;

    reports.changed(&[material_path(&root, A_MATERIAL_FILE)])?;
    let over_a_material = crossing_a_quiet_run(&mut client);
    let untouched = solidity_of(&client, STONE)?;
    reports.changed(&[block_path(&root, STONE_FILE)])?;
    let over_a_declaration = until_settled(&mut client);

    assert_eq!(
        (
            ended(&over_a_material),
            untouched,
            ended(&over_a_declaration),
            solidity_of(&client, STONE)?
        ),
        (Vec::new(), Some(true), taken_up_once(), Some(false)),
        "the loader reads no material file — `tools/voxforge` is their only reader and no \
         declaration names one — so a save under `materials/` is somebody else's work and not a \
         reload. **The two halves are one run through one watch**, because an absence on its own \
         is satisfied by a watcher that never fires and by a rule that has come to refuse \
         everything: the second report is what says this instrument can begin an attempt at all, \
         and the solidity either side of it is what says which report did"
    );
    Ok(())
}

/// The scratch file an editor leaves beside the declaration it is saving.
///
/// The suffix is appended to the declaration's whole file name, which is how a real
/// editor spells one — so the path holds the declaration extension in the middle
/// and something else at the end.
fn scratch_file_beside(root: &ContentRoot) -> PathBuf {
    block_path(root, &format!("{STONE_FILE}{SCRATCH_SUFFIX}"))
}

/// `root` with the crossbar declaration widened where it stands.
///
/// Written into the root the client is playing rather than into a second copy: the
/// file an author edits is the file the run was started from.
///
/// # Errors
///
/// Returns an error if the root does not declare the crossbar — a root that never
/// declared it is not a root whose declaration an author widened — or if the write
/// fails.
fn widening_the_crossbar(root: ContentRoot) -> Result<ContentRoot, Box<dyn Error>> {
    let declared = root.path().join(HUD_DIRECTORY).join(CROSSBAR_FILE);
    if !declared.is_file() {
        return Err(format!(
            "this fixture has to widen `{HUD_DIRECTORY}/{CROSSBAR_FILE}` in the root the client \
             is playing, and that root does not declare it. What it would build is a root that \
             gained a crosshair rather than one whose crosshair an author widened"
        )
        .into());
    }
    fs::write(&declared, WIDENED_CROSSBAR)?;
    Ok(root)
}

/// The extent the layout this client is serving states for the crossbar.
///
/// # Errors
///
/// Returns an error where nothing is being published.
fn crossbar_published(client: &InputHarness) -> Result<Option<[u32; 2]>, Box<dyn Error>> {
    let published = client
        .content()
        .ok_or("this fixture's client publishes no content, so it publishes no layout")?;
    Ok(crossbar_extent(&published.hud))
}
