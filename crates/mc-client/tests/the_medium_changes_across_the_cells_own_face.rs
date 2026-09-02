//! Two eyes four hundredths of a block apart, on either side of the sea's own
//! top face.
//!
//! # What this tells apart
//!
//! The rule is over the cell the eye's position falls in, and *falls in* is a
//! floor on all three axes and nothing else. The shipped sea's upper cells run
//! from `y = 34` to `y = 35`, so an eye at **34.98** is inside one and an eye at
//! **35.02** is in the empty cell above it. Any conversion that is not exactly a
//! floor — a round, a truncation toward zero, a half-block bias, a test against
//! the cell's centre — puts the boundary somewhere other than the face and one
//! of these two eyes on the wrong side of it.
//!
//! # Both halves are asserted, and so is the wiring between them
//!
//! The resolver's own answer is named for each height, which is what says the
//! boundary is where it should be. But a resolver answering correctly into a
//! frame that ignores it is the state this phase opens in, so the verdict also
//! carries the two comparisons that only a drawn tint can satisfy: the
//! submerged pose has to draw something a world declaring no tint does not, and
//! the pose above the face has to draw exactly what that world draws.

mod support;

use std::error::Error;

use mc_core::block::MediumTint;

use support::TestResult;
use support::content::{ContentRoot, SEA_FILE, shipped_copy};
use support::medium::{REACHES_AT, TINT, tinting};
use support::submerged::{
    EYE, JUST_OVER_THE_SURFACE, JUST_UNDER_THE_SURFACE, Shot, differing, drawn_from,
};

/// What the two heights came to.
#[derive(Debug, PartialEq)]
struct Straddling {
    the_medium_just_under_the_face: Option<MediumTint>,
    the_medium_just_over_it: Option<MediumTint>,
    drawn_under_it_at_something_other_than_the_predicted_colour: Vec<String>,
    drawn_over_it_at_something_other_than_the_predicted_colour: Vec<String>,
    /// Whether the frame from just under the face moves when the sea declares a
    /// tint.
    the_frame_under_it_moves_when_the_sea_declares_a_tint: bool,
    /// How many pixels of the frame from just over the face move when it does.
    pixels_over_it_that_move: usize,
}

#[test]
fn an_eye_a_hair_under_the_seas_top_face_is_inside_it_and_one_a_hair_over_is_not() -> TestResult {
    let Some(straddling) = what_the_two_heights_draw()? else {
        return Ok(());
    };
    assert_eq!(
        straddling,
        Straddling {
            the_medium_just_under_the_face: tinting(TINT),
            the_medium_just_over_it: None,
            drawn_under_it_at_something_other_than_the_predicted_colour: Vec::new(),
            drawn_over_it_at_something_other_than_the_predicted_colour: Vec::new(),
            the_frame_under_it_moves_when_the_sea_declares_a_tint: true,
            pixels_over_it_that_move: 0,
        },
        "the sea's upper cells end at y = 35.0, so an eye at {JUST_UNDER_THE_SURFACE} is inside \
         one and an eye at {JUST_OVER_THE_SURFACE} is in the empty cell over it. The medium \
         changes across that cell's own face and not at a distance from it, which is what a floor \
         on all three axes gives and what nothing else does. The last two elements are what keep \
         the first four from being satisfied by a resolver whose answer reaches no pixel"
    );
    Ok(())
}

/// What the two heights resolve to and draw, over a sea that declares a tint and
/// over one that declares none.
///
/// `None` where the opt-in permitted the absence of a device.
fn what_the_two_heights_draw() -> Result<Option<Straddling>, Box<dyn Error>> {
    let tinted = a_sea_that_tints()?;
    let plain = a_sea_declaring_nothing()?;
    let Some(under) = drawn_from(&tinted, under_the_face(), "face-under-tinted")? else {
        return Ok(None);
    };
    let Some(under_plain) = drawn_from(&plain, under_the_face(), "face-under-plain")? else {
        return Ok(None);
    };
    let Some(over) = drawn_from(&tinted, over_the_face(), "face-over-tinted")? else {
        return Ok(None);
    };
    let Some(over_plain) = drawn_from(&plain, over_the_face(), "face-over-plain")? else {
        return Ok(None);
    };
    Ok(Some(Straddling {
        the_medium_just_under_the_face: under.tint,
        the_medium_just_over_it: over.tint,
        drawn_under_it_at_something_other_than_the_predicted_colour: strays(
            &under,
            under_the_face(),
        )?,
        drawn_over_it_at_something_other_than_the_predicted_colour: strays(&over, over_the_face())?,
        the_frame_under_it_moves_when_the_sea_declares_a_tint: differing(
            &under.frame,
            &under_plain.frame,
        ) > 0,
        pixels_over_it_that_move: differing(&over.frame, &over_plain.frame),
    }))
}

/// Every declared sample `shot` drew away from the colour predicted for it.
fn strays(shot: &Shot, eye: [f32; 3]) -> Result<Vec<String>, Box<dyn Error>> {
    shot.straying(eye)
}

/// The declared pose with the eye a hair under the sea's own top face, and a
/// hair over it.
fn under_the_face() -> [f32; 3] {
    [EYE[0], JUST_UNDER_THE_SURFACE, EYE[2]]
}
fn over_the_face() -> [f32; 3] {
    [EYE[0], JUST_OVER_THE_SURFACE, EYE[2]]
}

/// A copy of the shipped root whose sea declares [`TINT`] at [`REACHES_AT`], and
/// one whose sea declares no tint at all.
fn a_sea_that_tints() -> Result<ContentRoot, Box<dyn Error>> {
    shipped_copy()?.whose_block_declares(SEA_FILE, Some((TINT, REACHES_AT)))
}
fn a_sea_declaring_nothing() -> Result<ContentRoot, Box<dyn Error>> {
    shipped_copy()?.whose_block_declares(SEA_FILE, None)
}
