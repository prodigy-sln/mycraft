//! A build that finds its output current does no work, and one that does not
//! rebuilds.
//!
//! **The cache key is the fold and it is whole-set**: fold the sources, and if
//! the value matches the one the index records and every image the index names
//! is there, touch no file. That is the decision these three grade, and the
//! first of them grades it the only way it can be graded — by leaving something
//! in the output directory that no build would ever write, and asking whether
//! it is still there afterwards. Comparing a set against itself cannot tell
//! *did no work* from *did the work again and got the same bytes*, and the
//! scenario is about the first.

#[path = "common/build.rs"]
mod build;
mod common;

use std::fs;

use common::TestResult;
use common::texture::{GREY, LIME};
use voxforge::inspect::ExitCode;

use build::{
    CUBE_MODEL, FIRST_KEY, MANIFEST_FILE, block_of, built, files_in, fingerprinted, image_named,
    root_of_one_cube, two_faces_of_the_cube,
};
use mc_core::content::TEXTURE_EDGE;

/// Bytes no PNG encoder produces, put where an image is so that a rebuild
/// announces itself.
const A_MARKER_NO_BUILD_WRITES: &[u8] = b"no build wrote this";

/// How many images the two-entry fixture manifest bakes.
const IMAGES_THE_FIXTURE_BAKES: usize = 2;

/// What a two-entry build leaves in its output directory: those images and one
/// index.
const FILES_A_TWO_ENTRY_BUILD_LEAVES: usize = IMAGES_THE_FIXTURE_BAKES + 1;

/// The word a build uses when it found its output current.
///
/// Contract rather than prose. Whoever runs the build reads this line to know
/// the difference between a set that was rebuilt and one that did not need to
/// be, and a mapping is decidable only where its spelling is pinned.
const NOTHING_REBUILT: &str = "nothing needed rebuilding";

#[test]
fn a_build_whose_output_is_current_leaves_every_images_bytes_unchanged() -> TestResult {
    let root = root_of_one_cube(GREY)?;
    root.holding(MANIFEST_FILE, &two_faces_of_the_cube())?;
    built(&root)?;
    fs::write(
        root.output().join(image_named(FIRST_KEY)),
        A_MARKER_NO_BUILD_WRITES,
    )?;
    let standing = fingerprinted(&files_in(&root.output())?);

    let second = built(&root)?;

    assert_eq!(
        (second.code, standing.len(), second.fingerprints()),
        (
            ExitCode::Success,
            FILES_A_TWO_ENTRY_BUILD_LEAVES,
            standing.clone()
        ),
        "the sources have not moved, so the fold has not moved, so nothing is opened. The marker \
         bytes are what makes that observable: a build that re-encoded and rewrote every image \
         would produce the same PNG it produced the first time and satisfy a comparison against \
         itself, and it would replace these. What the cache checks about an image is that it is \
         **there**. stderr said: {err}",
        err = second.err
    );
    Ok(())
}

#[test]
fn a_build_whose_output_is_current_reports_that_nothing_needed_rebuilding() -> TestResult {
    let root = root_of_one_cube(GREY)?;
    root.holding(MANIFEST_FILE, &two_faces_of_the_cube())?;

    let first = built(&root)?;
    let second = built(&root)?;

    assert_eq!(
        (
            second.code,
            first.out.contains(NOTHING_REBUILT),
            second.out.contains(NOTHING_REBUILT)
        ),
        (ExitCode::Success, false, true),
        "a run that did nothing has to say so, or whoever ran it cannot tell it from a run that \
         did nothing *wrong*. The middle member is the control: a build that printed this line \
         every time would satisfy the scenario and mean nothing at all. stderr said: {err}",
        err = second.err
    );
    Ok(())
}

#[test]
fn editing_a_model_the_manifest_names_rewrites_the_images_derived_from_it() -> TestResult {
    let root = root_of_one_cube(GREY)?;
    root.painted(&[LIME])?
        .holding(MANIFEST_FILE, &two_faces_of_the_cube())?;
    let first = built(&root)?;

    // The model and nothing else: both tones are already declared, so the only
    // source that moves between the two builds is the one the scenario names.
    let edge = TEXTURE_EDGE;
    root.holding(CUBE_MODEL, &block_of((edge, edge, edge), edge, LIME))?;
    let second = built(&root)?;

    let rewritten: Vec<String> = first
        .images()
        .into_iter()
        .filter(|name| first.written.get(name) != second.written.get(name))
        .collect();

    assert_eq!(
        (second.code, first.images().len(), rewritten),
        (ExitCode::Success, IMAGES_THE_FIXTURE_BAKES, first.images()),
        "every image the edited model painted carries different bytes afterwards — named, so a \
         failure says which image went stale rather than that some file somewhere is equal. A \
         cache that answered *current* off a timestamp, or off the manifest alone, leaves this \
         list empty while the set on disk shows the old colour. The count is the middle member \
         because without it a build that wrote nothing at all satisfies the rest: no image it \
         wrote has stale bytes, since it wrote none. stderr said: {err}",
        err = second.err
    );
    Ok(())
}
