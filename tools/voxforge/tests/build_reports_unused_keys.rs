//! A key the manifest bakes that no block file spells is reported, never
//! refused.
//!
//! **The scan is over text, and that is a decision rather than a shortcut.**
//! Read literally, "a key no loadable block declares" puts a Luau host inside a
//! texture baker and lets a broken block declaration refuse an art build that
//! has nothing to do with it. So the build reads each `blocks/*.luau` as text
//! and reports a key whose spelling appears in none of them. A declaration that
//! *computes* its key is not seen and is reported unused — a limitation that is
//! acceptable precisely because the report is advisory and a false positive
//! costs one line of output.
//!
//! The absence half of each assertion below is safe for a reason worth stating:
//! the paths this build prints spell a key with two underscores where its colon
//! was, so `base:used` cannot reach stdout by being a written path. The only
//! way it appears is if the scan called it unused.

#[path = "common/build.rs"]
mod build;
mod common;

use std::error::Error;

use common::TestResult;
use common::texture::GREY;
use mc_core::content::TEXTURE_EDGE;
use voxforge::inspect::ExitCode;

use build::{MANIFEST_FILE, Root, block_of, built, entry, manifest};

/// The model both entries are baked from.
const MODEL: &str = "models/cube.mcvox";

/// A key one of the root's block files declares.
const DECLARED: &str = "base:used";

/// A key none of them declares.
const UNDECLARED: &str = "example:unused";

/// How many images a two-entry manifest bakes.
const IMAGES: usize = 2;

/// A root holding one cube and a manifest baking both keys from it.
///
/// `declaring` says which block files it carries; the caller passing none is
/// what gives the second test a root with no `blocks` directory at all.
fn root_declaring(declared: &[&str]) -> Result<Root, Box<dyn Error>> {
    let root = Root::bare()?;
    let edge = TEXTURE_EDGE;
    root.holding(MODEL, &block_of((edge, edge, edge), edge, GREY))?
        .painted(&[GREY])?
        .declaring(declared)?
        .holding(
            MANIFEST_FILE,
            &manifest(
                1,
                &[
                    entry(DECLARED, MODEL, "front"),
                    entry(UNDECLARED, MODEL, "top"),
                ],
            ),
        )?;
    Ok(root)
}

#[test]
fn a_key_no_block_file_spells_is_named_as_unused_and_the_build_completes() -> TestResult {
    let root = root_declaring(&[DECLARED])?;

    let made = built(&root)?;

    assert_eq!(
        (
            made.code,
            made.images().len(),
            made.out.contains(UNDECLARED),
            made.out.contains(DECLARED)
        ),
        (ExitCode::Success, IMAGES, true, false),
        "a key nothing draws with is almost always a typo, and saying so is worth a line. \
         Refusing would not be: the manifest and the block files are edited by different hands at \
         different times, and an art build that stopped because a block had not been written yet \
         would be wrong about which of the two is unfinished. The last member is the control — a \
         scan that reported every key would satisfy the first three and mean nothing. It said: \
         {err}",
        err = made.err
    );
    Ok(())
}

#[test]
fn a_blocks_directory_that_is_not_there_reports_every_key_as_unused_and_completes() -> TestResult {
    let root = root_declaring(&[])?;

    let made = built(&root)?;

    assert_eq!(
        (
            made.code,
            made.images().len(),
            made.out.contains(UNDECLARED),
            made.out.contains(DECLARED)
        ),
        (ExitCode::Success, IMAGES, true, true),
        "the other door into the same refusal. A build that read the block directory the way it \
         reads a model — as something that has to be there — would refuse a perfectly good art \
         build because a root ships no blocks yet, and the scenario's own test would not see it: \
         that root has a blocks directory. Advisory means advisory whether the files are missing \
         or the keys are. It said: {err}",
        err = made.err
    );
    Ok(())
}
