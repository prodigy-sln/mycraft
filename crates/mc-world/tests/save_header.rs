//! What a save says it is, read before anything it governs.
//!
//! A save declares the format it is written in and the version of that format,
//! and both are read by hand from a fixed offset before a single byte is handed
//! to a decoder. That is not tidiness: **a version has to be readable out of a
//! file this build cannot otherwise read.** A version read through the decoder
//! would depend on the decoder being able to make sense of a format nobody has
//! taught it, which is exactly the case the version field exists for.
//!
//! The consequence is an ordering claim rather than a refusal claim, and it is
//! the one this file is really about. A save whose version is unrecognised *and*
//! whose table is unreadable must be refused **by version** — a build that
//! cannot read the format never reports a complaint about bytes it was never
//! entitled to interpret. A test asserting only that such a save was refused
//! could not fail for the reason the requirement is about, so the refusal is
//! asserted by name.
//!
//! **A path nothing is at is a different answer from a path that cannot be
//! read**, and the difference is load-bearing rather than cosmetic: a launch
//! decides whether to generate a world by branching on exactly it, so a
//! collapsed pair would generate a new world over a save that merely could not
//! be opened. The second half of that scenario is induced by a directory, which
//! is the only thing that exists, cannot be read as a file, and can be created
//! the same way on every platform this targets.

mod common;

use common::handbuilt::{self, HandBuilt, VERSION_AT, VERSION_BYTES};
use common::persistence::{STANDING_SOMEWHERE, save_in, world_at, world_holding};
use common::{TestResult, registry_of};
use mc_world::persistence::{self, LoadError};
use mc_world::world::WorldPos;
use std::fs;
use tempfile::TempDir;

/// The one block the written save holds, and where it sits.
const HELD: &str = "fixture:andesite";
const A_CELL: WorldPos = world_at(1, 1, 1);

/// The version of this format this build supports.
///
/// Spelled out here rather than read from the crate, because a test that reads
/// the number it asserts against would agree with a build that changed it.
const SUPPORTED_VERSION: u16 = 1;

/// A version this build has never heard of.
///
/// The next one up, which is the version this format will really acquire when
/// compression arrives — the case worth refusing well is the near one, not an
/// absurd one.
const A_VERSION_FROM_THE_FUTURE: u16 = 2;

/// Eight leading bytes that are not this format's.
///
/// Same length as the magic and plain ASCII, so a save mangled into text and a
/// file that was simply never a save are the same case here.
const NOT_THIS_FORMAT: [u8; 8] = *b"NOTASAVE";

/// How long the fixture that is not a save is.
///
/// Long enough to hold a whole preamble, so it is refused for its leading bytes
/// and unambiguously not for being short.
const A_PLAUSIBLE_LENGTH: usize = handbuilt::PREAMBLE_BYTES;

/// What every refusal in this file has to say out loud.
///
/// The requirement is to name the format that was expected, and the only thing
/// that names it is the message: the reported bytes say what was *found*.
const THE_FORMAT_EXPECTED: &str = "MYCRAFT";

/// A table entry whose text is not a namespaced name.
///
/// The malformation the ordering scenario hides behind an unrecognised version.
/// It is a real refusal on its own — a save declaring a version this build reads
/// and carrying this table is refused for the name — which is what makes the
/// ordering claim mean something.
const NOT_A_NAME: &str = "not a namespaced name";

/// A version this build recognises, and a table it could read.
const ONE_NAME: [handbuilt::Entry<'static>; 1] = [("fixture:andesite", 11, 12)];

/// The version `written` declares, read out of the offset this suite's fixture
/// says it sits at.
fn version_declared_by(written: &[u8]) -> Option<u16> {
    written
        .get(VERSION_AT..VERSION_AT + VERSION_BYTES)
        .and_then(|declared| <[u8; 2]>::try_from(declared).ok())
        .map(u16::from_le_bytes)
}

/// What a refusal said, or nothing at all where there was no refusal.
fn what_it_said<T>(answer: &Result<T, LoadError>) -> String {
    answer
        .as_ref()
        .err()
        .map(ToString::to_string)
        .unwrap_or_default()
}

#[test]
fn a_save_this_build_wrote_declares_a_version_this_build_reads() -> TestResult {
    let directory = TempDir::new()?;
    let registry = registry_of(&[HELD])?;
    let world = world_holding(&[(A_CELL, HELD)], &registry)?;
    let path = save_in(&directory);
    persistence::save_world(&path, &world, STANDING_SOMEWHERE, &registry)?;

    let written = fs::read(&path)?;

    assert_eq!(
        (
            version_declared_by(&written),
            persistence::requirements(&path).is_ok()
        ),
        (Some(SUPPORTED_VERSION), true),
        "the version a build writes and the version it accepts are two decisions, and a build \
         whose reader had moved on from its writer would refuse its own saves — which is a player \
         losing a world to an upgrade nobody warned them about. This is also the control for the \
         refusal below: a reader that turned every version away would satisfy that one and fail \
         here"
    );
    Ok(())
}

#[test]
fn a_save_declaring_a_version_this_build_does_not_know_is_refused_by_version() -> TestResult {
    let directory = TempDir::new()?;
    let path = handbuilt::written(
        &directory,
        "from_the_future.mcw",
        HandBuilt {
            version: A_VERSION_FROM_THE_FUTURE,
            table: &ONE_NAME,
            ..HandBuilt::default()
        },
    )?;

    assert_eq!(
        persistence::requirements(&path),
        Err(LoadError::UnsupportedVersion {
            found: A_VERSION_FROM_THE_FUTURE,
            supported: SUPPORTED_VERSION
        }),
        "compression is excluded from this spec and certain to arrive, so a save from a later \
         build is a file a player will really have. Both numbers are named because only both \
         together tell them what to do: the version found says which build wrote it and the \
         version supported says which build can read it, and a refusal carrying one of the two \
         leaves them guessing at the other"
    );
    Ok(())
}

#[test]
fn a_save_from_the_future_whose_table_is_unreadable_is_refused_by_version_first() -> TestResult {
    let directory = TempDir::new()?;
    let path = handbuilt::written(
        &directory,
        "from_the_future_and_broken.mcw",
        HandBuilt {
            version: A_VERSION_FROM_THE_FUTURE,
            table: &[(NOT_A_NAME, 11, 12)],
            ..HandBuilt::default()
        },
    )?;

    assert_eq!(
        persistence::requirements(&path),
        Err(LoadError::UnsupportedVersion {
            found: A_VERSION_FROM_THE_FUTURE,
            supported: SUPPORTED_VERSION
        }),
        "this file is wrong in two ways at once and only one of them is a complaint this build is \
         entitled to make. The table belongs to a format this build has never been taught, so \
         what looks like a malformed name might be a perfectly good entry of version two — \
         reporting it would be this build asserting something about bytes it cannot read. \
         Asserting the variant rather than merely that it was refused is the whole test: a \
         refusal by name would satisfy `is_err` just as well"
    );
    Ok(())
}

#[test]
fn a_file_that_does_not_begin_the_way_a_save_does_is_refused_naming_the_format() -> TestResult {
    let directory = TempDir::new()?;
    let mut bytes = NOT_THIS_FORMAT.to_vec();
    bytes.resize(A_PLAUSIBLE_LENGTH, 0);
    let path = handbuilt::file_holding(&directory, "something_else.mcw", &bytes)?;

    let asked = persistence::requirements(&path);
    let said = what_it_said(&asked);

    assert_eq!(
        (asked, said.contains(THE_FORMAT_EXPECTED)),
        (
            Err(LoadError::NotASave {
                found: NOT_THIS_FORMAT.to_vec()
            }),
            true
        ),
        "the leading bytes are the whole of what says this file is one of ours, and a player \
         pointed at the wrong file — a screenshot, a half-downloaded archive, last year's backup \
         of something else — learns nothing from being told it could not be read. Saying what was \
         found and what a save begins with turns that into a mistake they can see"
    );
    Ok(())
}

#[test]
fn a_save_file_of_no_bytes_at_all_is_refused_naming_the_format() -> TestResult {
    let directory = TempDir::new()?;
    let path = handbuilt::file_holding(&directory, "empty.mcw", &[])?;

    let asked = persistence::requirements(&path);
    let said = what_it_said(&asked);

    assert_eq!(
        (asked, said.contains(THE_FORMAT_EXPECTED)),
        (Err(LoadError::NotASave { found: Vec::new() }), true),
        "an empty file is what a disk full at the wrong moment leaves behind, and it is the one \
         corrupt save a player is most likely to meet. It is not missing — something is there, \
         and generating a new world over it would be writing across whatever is left — so it \
         reports the same refusal as any other file that is not a save, carrying the nothing it \
         found"
    );
    Ok(())
}

#[test]
fn a_save_path_nothing_is_at_is_reported_as_missing_and_not_as_unreadable() -> TestResult {
    let directory = TempDir::new()?;
    let nowhere = directory.path().join("no_save_here.mcw");
    let unreadable = directory.path().join("a_directory.mcw");
    fs::create_dir(&unreadable)?;

    let absent = persistence::requirements(&nowhere);
    let present_but_unreadable = persistence::requirements(&unreadable);

    assert_eq!(
        (
            absent,
            matches!(present_but_unreadable, Err(LoadError::Missing { .. }))
        ),
        (
            Err(LoadError::Missing {
                path: nowhere.clone()
            }),
            false
        ),
        "a launch decides whether to generate a world on exactly this distinction, so collapsing \
         the two would make a save that merely could not be opened look like no save at all — and \
         the next quit would write a fresh world over it. The second half is what makes the first \
         mean anything: a reader that called everything missing would satisfy the first on its own"
    );
    Ok(())
}
