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
///
/// Three ticks of the declared intent script, chosen because they are three
/// different things the script puts in front of the camera: the spawn still
/// falling, the end of the straight walk, and the tick after the turn and the
/// jump. 59 rather than the 60 the orbit's half turn used to name — there is no
/// half turn any more, and 59 is the last tick before the script starts turning.
pub const DECLARED_CAPTURE_TICKS: [u16; 3] = [0, 59, 119];

/// The ticks of the replay that carry a committed golden *of a composed HUD*.
///
/// One, and deliberately not three. The HUD does not animate and the held block
/// is set once, so ticks 59 and 119 would assert the same rectangles against
/// different terrain. Tick 0 is the frame with the least terrain coverage, so the
/// crosshair stands against the most sky.
///
/// A set of its own rather than a second use of [`DECLARED_CAPTURE_TICKS`]: the
/// two sets are shot through different calls — the terrain captures through
/// `record_terrain` and this one through the frame call the windowed client makes
/// — and a shared list would make one of those two choices look like a
/// consequence of the other.
pub const HUD_CAPTURE_TICKS: [u16; 1] = [0];

/// What every capture id of this replay begins with.
///
/// It names what the frames are *of*, so it changed with them: these are a
/// player walking, not a camera orbiting. Renaming rather than overwriting is
/// what makes the re-shoot a diff a reader can judge — three directories
/// removed and three added, instead of three binary blobs quietly modified.
const CAPTURE_PREFIX: &str = "player-walk";

/// What a HUD capture's id carries that a terrain capture's does not.
///
/// The two sets stand side by side under one golden root, so the id has to say
/// which of the two draw paths a frame came through — otherwise a HUD capture at
/// tick 0 and the terrain capture at tick 0 would name the same directory and one
/// would overwrite the other's reference.
const HUD_SEGMENT: &str = "-hud";

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
    id_of("", tick, revision)
}

/// The id of the HUD capture declared at `tick` for scene revision `revision`.
///
/// # Errors
///
/// Returns [`CaptureIdShapeError`] when `revision` is empty or holds a character
/// that cannot appear in a directory the golden lifecycle writes.
pub fn hud_capture_id(tick: u16, revision: &str) -> Result<String, CaptureIdShapeError> {
    id_of(HUD_SEGMENT, tick, revision)
}

/// The id of the capture declared at `tick`, with `segment` saying which draw
/// path it came through.
///
/// One statement of the shape for both sets. Written twice, the tick width or the
/// separator could drift on one side only, and a directory listing would stop
/// sorting the two sets together for a reason nothing states.
fn id_of(segment: &str, tick: u16, revision: &str) -> Result<String, CaptureIdShapeError> {
    check_revision(revision)?;
    // Three digits, zero-padded, so a directory listing sorts the captures in
    // tick order — which is the order a reader compares three frames of one
    // walk in. The declared intent script is 120 ticks long, and it is the
    // script's length rather than any period of the simulation that bounds
    // this, so the width never overflows.
    Ok(format!("{CAPTURE_PREFIX}{segment}-t{tick:03}-{revision}"))
}

/// The ids of every capture `revision` declares: the terrain set in tick order,
/// then the HUD set in tick order.
///
/// Exactly the directory names the golden root may hold at that revision — which
/// is what lets an inventory check report a set left behind by a previous
/// revision as well as one that is missing.
///
/// **Both sets, because only the pair covers the path the product draws
/// through.** The terrain captures go through `record_terrain` and the HUD
/// capture through the client's own frame call; a list holding one of the two
/// would leave the other's directory reported as one no test declares.
///
/// # Errors
///
/// Returns [`CaptureIdShapeError`] when `revision` cannot appear in an id.
pub fn declared_capture_ids(revision: &str) -> Result<Vec<String>, CaptureIdShapeError> {
    let terrain = DECLARED_CAPTURE_TICKS
        .iter()
        .map(|tick| capture_id(*tick, revision));
    let hud = HUD_CAPTURE_TICKS
        .iter()
        .map(|tick| hud_capture_id(*tick, revision));
    terrain.chain(hud).collect()
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
