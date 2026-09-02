//! What a camera standing inside the shipped sea draws, what one standing over
//! it draws, and what a sea declaring nothing does to either.
//!
//! **This reading supersedes `a_camera_inside_the_sea_tints_nothing`, whose name
//! became a false statement the moment the sea could declare a tint.** The pose
//! it declared, the filter that admitted it and the ranking that chose it move
//! with it and live in `support/submerged.rs`; what survives unchanged is its
//! negative half — a camera inside a block that declares no tint still draws a
//! dry frame — and the reasoning that made it carry two claims rather than one.
//!
//! # A frame-to-frame identity claim is satisfied by a constant wash
//!
//! Two frames differing only in whether a tint is declared can agree while both
//! are wrong, which is a failure this project has shipped. So the superseded
//! reading carried an **absolute per-sample claim beside its pixel comparison**,
//! and dropping that half would have read as a clean replacement while being a
//! strict weakening. Every reading below keeps both halves in one assertion.
//!
//! # And an identity claim needs a control
//!
//! A build that never writes the tint into the frame at all satisfies every
//! "nothing moved" claim here, absolute half included, because the absolute
//! prediction for an untinted eye *is* the untinted colour. So each verdict
//! carries a case in which something must move — the same pose over a sea that
//! does declare a tint, or the same declared world seen from an eye that does
//! stand inside it. An implementation drawing no tint anywhere fails on that
//! element rather than passing quietly.
//!
//! # What the controls do not check, and what does
//!
//! **A control asks only that the picture moved, never that it moved to the
//! right colour.** `differing(...) > 0` is satisfied by any difference at all,
//! including a draw path that writes something wrong when a tint is declared —
//! the control frames are compared against each other and judged against no
//! prediction. What constrains the tinted picture is the **first reading in this
//! file**, which drives the *same* pose over the *same* tinting root and asserts
//! every declared sample against the colour the world's own voxels predict for
//! it.
//!
//! **So the controls depend on that reading surviving, and this paragraph is the
//! only place that is written down.** Five scenarios across this phase are red
//! on a control of this shape — the two here, the composited-HUD reading, the
//! committed-capture reading and the reload that removes both fields — and every
//! one of them would be hollowed by somebody "simplifying" the absolute reading
//! that closes them, without a single test going red to say so.
//!
//! **They are also one control observed five times, not five witnesses.** All
//! five reduce to *a submerged eye over a tinting sea draws something a
//! submerged eye over an untinted sea does not*: they go green together and stay
//! red together. A reviewer counting red scenarios sees five and is looking at
//! one fact.

mod support;

use std::error::Error;

use mc_core::block::MediumTint;

use support::TestResult;
use support::content::{ContentRoot, SEA_FILE, SURFACE_FILE, shipped_copy};
use support::medium::{REACHES_AT, TINT, tinting};
use support::submerged::{DRY_EYE, EYE, Shot, differing, drawn_from};

/// How many sky samples a reading about the sky needs before it means anything.
const MANY: usize = 100;

/// What one pose over one declared world came to.
#[derive(Debug, PartialEq)]
struct Seen {
    /// What the simulation's own resolver answered for the eye drawn from.
    the_medium_the_eye_stands_in: Option<MediumTint>,
    /// Every declared sample drawn further than the tolerance from the colour
    /// predicted for it from the world's own voxels and declarations.
    drawn_at_something_other_than_the_predicted_colour: Vec<String>,
    /// How many pixels of the whole frame differ from the same pose over a world
    /// declaring no tint anywhere.
    pixels_differing_from_a_world_declaring_no_tint_anywhere: usize,
    /// Whether the control — the same pose over a root whose tint the eye *is*
    /// inside — draws a different picture.
    the_control_moves_when_the_eye_is_inside_a_declared_tint: bool,
}

#[test]
fn a_pixel_the_frame_draws_no_terrain_at_is_the_declared_colour_and_not_the_sky() -> TestResult {
    let Some(shot) = drawn_from(&a_sea_that_tints()?, EYE, "sea-tinted-sky")? else {
        return Ok(());
    };
    let sky = shot.sky_samples(EYE)?.len();
    assert_eq!(
        (shot.tint, sky >= MANY, shot.straying(EYE)?,),
        (tinting(TINT), true, Vec::new()),
        "the eye stands inside a cell whose block declares a tint, so a ray leaving the world \
         without meeting a drawn face is looking through the medium and not at the sky. Each of \
         the {sky} samples the world predicts as sky is therefore the declared colour {TINT:?} \
         rather than the colour a dry camera's sky is given, and the far terrain arrives at that \
         same colour by the other route — the mix reaching one at the declared distance"
    );
    Ok(())
}

#[test]
fn a_camera_inside_a_sea_declaring_no_tint_draws_the_frame_a_world_with_no_tints_draws()
-> TestResult {
    let Some(seen) = what_an_untinted_sea_draws()? else {
        return Ok(());
    };
    assert_eq!(
        seen,
        untouched(),
        "the eye stands inside a block that passes light and declares no tint, in a world where          another block does. It tints nothing: every pixel is the pixel a world declaring no tint          anywhere draws, and every declared sample is at the colour the world's own voxels          predict for it. The third element is the control, and it is what keeps the first two          from being satisfied by a renderer that cannot tint at all — the same pose over a sea          that does declare a tint has to draw a different picture"
    );
    Ok(())
}

#[test]
fn an_eye_in_the_open_air_over_a_sea_that_tints_is_untouched_by_it() -> TestResult {
    let Some(seen) = what_an_eye_over_the_sea_draws()? else {
        return Ok(());
    };
    assert_eq!(
        seen,
        untouched(),
        "the eye stands in a cell holding nothing, over a sea that declares a tint reaching its          full strength at {REACHES_AT} blocks. What the eye is *in* decides, so the declaration          reaches no pixel: the frame is the one the same pose draws in a world declaring no tint          anywhere, and every declared sample is at the colour predicted for it. The third element          is the control, and it is the same pose over the two roots rather than the two poses          over one: an eye that *is* inside the declared sea has to draw something the same eye          over an undeclared one does not"
    );
    Ok(())
}

/// The verdict a pose the medium does not reach owes: an untinted eye, every
/// sample at its predicted colour, no pixel moved, and the control moving.
fn untouched() -> Seen {
    Seen {
        the_medium_the_eye_stands_in: None,
        drawn_at_something_other_than_the_predicted_colour: Vec::new(),
        pixels_differing_from_a_world_declaring_no_tint_anywhere: 0,
        the_control_moves_when_the_eye_is_inside_a_declared_tint: true,
    }
}

/// What the submerged pose draws over a root whose sea declares no tint and
/// whose surface layer does, against one declaring none anywhere and one whose
/// sea does.
fn what_an_untinted_sea_draws() -> Result<Option<Seen>, Box<dyn Error>> {
    let Some(elsewhere) = drawn_from(&a_tint_somewhere_else()?, EYE, "sea-untinted")? else {
        return Ok(None);
    };
    let Some(nowhere) = drawn_from(&no_tint_anywhere()?, EYE, "sea-no-tints")? else {
        return Ok(None);
    };
    let Some(tinting_sea) = drawn_from(&a_sea_that_tints()?, EYE, "sea-tinted-control")? else {
        return Ok(None);
    };
    Ok(Some(Seen {
        pixels_differing_from_a_world_declaring_no_tint_anywhere: differing(
            &elsewhere.frame,
            &nowhere.frame,
        ),
        the_control_moves_when_the_eye_is_inside_a_declared_tint: differing(
            &elsewhere.frame,
            &tinting_sea.frame,
        ) > 0,
        ..what_it_drew(&elsewhere, EYE)?
    }))
}

/// What the pose over the sea's own top face draws over a tinting root and over
/// one declaring nothing.
///
/// The control is the *same* pose over the two roots, never the two poses over
/// one root: two poses draw different pictures whatever any declaration says, so
/// a comparison between them would be satisfied by a renderer that cannot tint
/// at all — which is exactly what this element exists to refuse.
fn what_an_eye_over_the_sea_draws() -> Result<Option<Seen>, Box<dyn Error>> {
    let tinted = a_sea_that_tints()?;
    let plain = no_tint_anywhere()?;
    let Some(dry) = drawn_from(&tinted, DRY_EYE, "sea-from-above")? else {
        return Ok(None);
    };
    let Some(dry_plain) = drawn_from(&plain, DRY_EYE, "sea-from-above-no-tints")? else {
        return Ok(None);
    };
    let Some(under) = drawn_from(&tinted, EYE, "sea-from-under-control")? else {
        return Ok(None);
    };
    let Some(under_plain) = drawn_from(&plain, EYE, "sea-from-under-no-tints")? else {
        return Ok(None);
    };
    Ok(Some(Seen {
        pixels_differing_from_a_world_declaring_no_tint_anywhere: differing(
            &dry.frame,
            &dry_plain.frame,
        ),
        the_control_moves_when_the_eye_is_inside_a_declared_tint: differing(
            &under.frame,
            &under_plain.frame,
        ) > 0,
        ..what_it_drew(&dry, DRY_EYE)?
    }))
}

/// What `shot` drew, beside what the resolver found for the eye it was drawn
/// from. The two comparison fields are the caller's to fill.
fn what_it_drew(shot: &Shot, eye: [f32; 3]) -> Result<Seen, Box<dyn Error>> {
    Ok(Seen {
        the_medium_the_eye_stands_in: shot.tint,
        drawn_at_something_other_than_the_predicted_colour: shot.straying(eye)?,
        pixels_differing_from_a_world_declaring_no_tint_anywhere: 0,
        the_control_moves_when_the_eye_is_inside_a_declared_tint: true,
    })
}

/// A copy of the shipped root whose sea declares [`TINT`] at [`REACHES_AT`].
fn a_sea_that_tints() -> Result<ContentRoot, Box<dyn Error>> {
    shipped_copy()?.whose_block_declares(SEA_FILE, Some((TINT, REACHES_AT)))
}

/// A copy whose sea declares no tint and whose surface layer does.
///
/// **The world holds a tint and the eye's own cell does not**, which is the
/// state the reading is about: a comparison against a world with no tints
/// anywhere would otherwise be a comparison of one root against itself.
fn a_tint_somewhere_else() -> Result<ContentRoot, Box<dyn Error>> {
    shipped_copy()?
        .whose_block_declares(SEA_FILE, None)?
        .whose_block_declares(SURFACE_FILE, Some((TINT, REACHES_AT)))
}

/// A copy in which nothing declares a tint at all.
fn no_tint_anywhere() -> Result<ContentRoot, Box<dyn Error>> {
    shipped_copy()?
        .whose_block_declares(SEA_FILE, None)?
        .whose_block_declares(SURFACE_FILE, None)
}
