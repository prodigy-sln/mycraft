//! The same sources produce the same bytes, twice.
//!
//! **Two separate copies of the shipped root, not one root built twice.** A
//! second build into the same directory finds its output current and does no
//! work, so it would compare a set against itself and pass for a tool that is
//! not reproducible at all. Two copies also say something the one-directory
//! form cannot: the paths the index records are relative to the manifest, so
//! two roots at two different absolute locations fold to one value.
//!
//! This scenario and the fold together are what replaces a table of digests. A
//! table copied into a page is a second copy that has to be updated on every art
//! change, and the copy that stops being updated is the one a reader trusts; the
//! durable form of a measurement is the command that reproduces it.

#[path = "common/build.rs"]
mod build;
mod common;

use voxforge::inspect::ExitCode;

use build::{Root, built, keys_stated};
use common::TestResult;

#[test]
fn two_builds_of_unchanged_sources_produce_byte_identical_images() -> TestResult {
    let here = Root::shipped()?;
    let elsewhere = Root::shipped()?;
    let owed = keys_stated(&here.manifest())?.len();

    let first = built(&here)?;
    let second = built(&elsewhere)?;

    assert_eq!(
        (first.code, first.images().len(), first.fingerprints()),
        (ExitCode::Success, owed, second.fingerprints()),
        "every file of the set, images and index alike, byte for byte from two roots that share \
         nothing but their contents. The image count is here so that two empty directories cannot \
         satisfy it, and it is read off the manifest rather than written down. The index is in the \
         comparison too: it carries the fold, and a fold that moved between two identical roots \
         would mean the recorded paths are absolute. stderr said: {first_err} / {second_err}",
        first_err = first.err,
        second_err = second.err
    );
    Ok(())
}
