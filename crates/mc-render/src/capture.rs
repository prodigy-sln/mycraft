//! What a capture is called, and which captures the replay declares.
//!
//! A capture id names a directory of committed goldens, so it carries the
//! revision of the scene those goldens were shot from. That turns the day the
//! mesh contract changes from a silent re-shoot into a **rename**: the commit
//! shows added and removed files rather than a modified binary blob nobody can
//! read, and a bumped revision with no new goldens fails as a *missing* golden
//! naming the path it looked for.
//!
//! The revision is a parameter everywhere below and never read from
//! [`SCENE_REVISION`] inside. An id function that quietly substituted the
//! current revision would answer every question about the next one with the
//! current one's names — which is precisely the collision the revision exists to
//! prevent, arriving through the function meant to prevent it.
//!
//! # Why the alphabet is checked here
//!
//! An id becomes a path segment under the golden root. The capture harness
//! validates the same alphabet when it takes one, but it is a dev-dependency of
//! this crate and cannot be named from the library, so a revision that would not
//! survive that validation has to be refused where the id is built. The rule is
//! the harness's: lowercase `a-z`, `0-9`, `-` and `_`, which excludes both path
//! separators and `.`, so no revision can walk out of the directory it names.

use thiserror::Error;

/// The revision of the scene the committed goldens were captured from.
///
/// Bumped whenever a change to the mesh contract invalidates every committed
/// frame. `crates/mc-sim`'s scene contract is the tripwire that fails first and
/// names this constant as the remedy.
pub const SCENE_REVISION: &str = "r1";

/// The ticks of the replay that carry a committed golden.
pub const DECLARED_CAPTURE_TICKS: [u16; 3] = [0, 60, 119];

/// What every capture id of this replay begins with.
const CAPTURE_PREFIX: &str = "terrain-orbit";

/// Why a revision cannot appear in a capture id.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum CaptureIdShapeError {
    #[error(
        "a scene revision cannot be empty: the id would end in a bare `-` and two revisions \
         would collide"
    )]
    EmptyRevision,
    #[error(
        "the scene revision `{revision}` contains `{character}`, which is not one of a-z, 0-9, \
         `-` or `_`; a capture id becomes a directory name under the golden root"
    )]
    IllegalCharacter { revision: String, character: char },
}

/// The id of the capture declared at `tick` for scene revision `revision`.
///
/// # Errors
///
/// Returns [`CaptureIdShapeError`] when `revision` is empty or holds a character
/// that cannot appear in a directory the golden lifecycle writes.
pub fn capture_id(tick: u16, revision: &str) -> Result<String, CaptureIdShapeError> {
    check_revision(revision)?;
    // Three digits, zero-padded, so a directory listing sorts the captures in
    // tick order — which is the order a reader compares three frames of one
    // orbit in. The replay's tick count is 120, so the width never overflows.
    Ok(format!("{CAPTURE_PREFIX}-t{tick:03}-{revision}"))
}

/// The ids of every capture `revision` declares, in tick order.
///
/// Exactly the directory names the golden root may hold at that revision — which
/// is what lets an inventory check report a set left behind by a previous
/// revision as well as one that is missing.
///
/// # Errors
///
/// Returns [`CaptureIdShapeError`] when `revision` cannot appear in an id.
pub fn declared_capture_ids(revision: &str) -> Result<Vec<String>, CaptureIdShapeError> {
    DECLARED_CAPTURE_TICKS
        .iter()
        .map(|tick| capture_id(*tick, revision))
        .collect()
}

/// Whether `revision` can stand in a path segment the golden lifecycle writes.
fn check_revision(revision: &str) -> Result<(), CaptureIdShapeError> {
    if revision.is_empty() {
        return Err(CaptureIdShapeError::EmptyRevision);
    }
    match revision.chars().find(|character| !is_legal(*character)) {
        Some(character) => Err(CaptureIdShapeError::IllegalCharacter {
            revision: revision.to_owned(),
            character,
        }),
        None => Ok(()),
    }
}

/// Whether `character` may appear in a capture id.
fn is_legal(character: char) -> bool {
    character.is_ascii_lowercase() || character.is_ascii_digit() || matches!(character, '-' | '_')
}
