//! Two preparation entry points, one picture: what a launch with no save hands the
//! renderer, against what the frame-capture path packs.
//!
//! # This is the assertion that licenses the split
//!
//! There are two functions that turn a content root into geometry — one for the
//! world a launch plays, one for the world the goldens are shot from — and the
//! strongest argument against having two is on record in this very suite: the
//! goldens once drifted precisely because they were captured through a pipeline the
//! product does not run, and evidence gathered from a second pipeline does not
//! transfer to the first. The answer to that objection is structural, in that both
//! doors go through one mesher, one definition of the texture key set and one
//! packer — and this is the test that says the structure held. If it cannot be made
//! to hold, the two-entry-point design is wrong and that is a finding, not
//! something to work around.
//!
//! # Bytes, in order, and not per-section quad counts
//!
//! A texture layer index is recorded inside a packed vertex, and the order sections
//! and quads come out in is itself the contract the committed images were shot
//! under. A per-section quad count sees neither: a scene resolved against a
//! different key set, or assembled in a different order, carries the same counts and
//! draws a different picture. So the comparison is over the section table and the
//! packed vertex buffer, as bytes, in the order they are handed over. The comparison
//! also refuses to call two empty scenes equal — see `support/handed.rs`.
//!
//! # Neither side is told to look at the working directory
//!
//! The launch is handed a save path explicitly, pointing into a fresh temporary
//! directory nothing has written, so this binary changes no process-global state and
//! is free to hold more than one test.
//!
//! # A save that changes nothing is the tripwire the no-save path cannot be
//!
//! The comparison above can only ever see the arm a launch takes when there is no
//! save, and a fix that resolved texture layers over the *played* world's blocks, or
//! that assembled a loaded world's sections in a different order, would leave it
//! perfectly green while moving every vertex a resuming player is handed. Nothing
//! upstream would notice: no golden frame is shot after a resume. So a second
//! scenario resumes a save holding the generated world **with nothing changed in
//! it**, and requires it to be indistinguishable from having no save at all, down to
//! the packed bytes.
//!
//! Both sides of that comparison are the launch path, so what it grades is the save
//! and only the save. It is stated over the same two byte views as the comparison
//! above rather than over the packed vertices alone: a section record is derived from
//! the very quads the vertices are packed from, so the extra half cannot fail on its
//! own for a preparation the scenario would otherwise accept.

#[path = "support/handed.rs"]
mod handed;

use mc_client::launch::{PreparedLaunch, prepare_launch};
use mc_client::startup::{PreparationError, PreparedScene, prepare_scene};
use mc_render::geometry::scene::SceneGeometry;
use mc_world::persistence::{Acceptance, SavedPlayer};
use tempfile::TempDir;

use handed::{
    NO_DIFFERENCE, TestResult, generated_blocks, how_it_compares, resumed, shipped_content,
    where_no_save_is,
};

/// The launch here has no save to read, so what a player said about loading one
/// whose blocks have changed decides nothing.
const ACCEPTING: Acceptance = Acceptance::OnlyUnchangedBlocks;

/// Where the save records the player. Nothing asserts it, and a save records
/// somebody.
const RECORDED_PLAYER: SavedPlayer = SavedPlayer {
    position: [12.5, 67.0, 12.5],
    yaw: 0.0,
    pitch: 0.0,
};

/// The landmark pillar's topmost block, which the generated world fills and the save
/// therefore has to come back holding.
///
/// The cell this scenario reads to know its save was really there: it is the one
/// place in the declared world where a single named block stands with nothing around
/// it, so a save that came back holding it came back holding the world.
const THE_LANDMARKS_TOP: (u32, u32, u32) = (12, 64, 12);

#[test]
fn a_launch_with_no_save_hands_over_the_geometry_the_capture_path_packs() -> TestResult {
    let content = shipped_content()?;
    let nowhere = TempDir::new()?;

    let launched = prepare_launch(&content, &where_no_save_is(&nowhere), ACCEPTING);
    let captured = prepare_scene(&content);

    assert_eq!(
        compared(&launched, &captured),
        Ok(NO_DIFFERENCE.to_owned()),
        "with no save to resume, the two preparation paths are the same pipeline over the same \
         world and must produce the same render input down to the byte — the same section records \
         in the same order, and the same packed vertices in the same order inside them. Every \
         committed golden frame is shot through the capture path while every frame a player sees \
         comes through the launch path, so any difference here means the images this project \
         verifies itself against depict something no player is ever handed"
    );
    Ok(())
}

#[test]
fn a_save_that_changes_nothing_hands_over_the_geometry_a_launch_with_no_save_hands() -> TestResult {
    let content = shipped_content()?;
    let nowhere = TempDir::new()?;
    let saved = resumed(&content, RECORDED_PLAYER, generated_blocks)?;

    let resuming = prepare_launch(&content, &saved.save(), ACCEPTING);
    let starting_fresh = prepare_launch(&content, &where_no_save_is(&nowhere), ACCEPTING);

    assert_eq!(
        (
            both_launches_compared(&resuming, &starting_fresh),
            saved.stored_at(THE_LANDMARKS_TOP)
        ),
        (
            Ok(NO_DIFFERENCE.to_owned()),
            Ok(saved.written_at(THE_LANDMARKS_TOP))
        ),
        "the save holds the generated world with nothing at all changed in it, so resuming it has \
         to hand the renderer exactly what starting fresh hands it — the same packed vertices in \
         the same order, each carrying the same texture array layer. This is the only scenario that \
         can see a change to what a *resuming* player is shown: a key set resolved over the played \
         world's blocks, or a loaded world assembled in a different order, moves every vertex here \
         and moves nothing the golden frames are shot through. The second half is what stops the \
         comparison from being between two launches that both generated: the file really is a \
         readable save, and it really does come back holding the world it was written from"
    );
    Ok(())
}

/// How the geometry two launches hand over compares — or the refusal whichever of
/// them gave one.
fn both_launches_compared(
    resuming: &Result<PreparedLaunch, PreparationError>,
    starting_fresh: &Result<PreparedLaunch, PreparationError>,
) -> Result<String, String> {
    Ok(how_it_compares(
        launch_geometry_of(resuming)?,
        launch_geometry_of(starting_fresh)?,
    ))
}

/// How the geometry a launch hands over compares with what the capture path packs —
/// or the refusal whichever of them gave one.
///
/// A refusal comes back as the failed comparison rather than as a propagated error,
/// so that "it refused to prepare anything" and "it prepared different geometry" are
/// one failed assertion instead of two kinds of failure.
fn compared(
    launched: &Result<PreparedLaunch, PreparationError>,
    captured: &Result<PreparedScene, PreparationError>,
) -> Result<String, String> {
    Ok(how_it_compares(
        launch_geometry_of(launched)?,
        capture_geometry_of(captured)?,
    ))
}

/// The scene a launch would hand the renderer, or its refusal, rendered.
fn launch_geometry_of(
    prepared: &Result<PreparedLaunch, PreparationError>,
) -> Result<&SceneGeometry, String> {
    Ok(&prepared
        .as_ref()
        .map_err(std::string::ToString::to_string)?
        .scene)
}

/// The scene the capture path packed, or its refusal, rendered.
fn capture_geometry_of(
    prepared: &Result<PreparedScene, PreparationError>,
) -> Result<&SceneGeometry, String> {
    Ok(&prepared
        .as_ref()
        .map_err(std::string::ToString::to_string)?
        .scene)
}
