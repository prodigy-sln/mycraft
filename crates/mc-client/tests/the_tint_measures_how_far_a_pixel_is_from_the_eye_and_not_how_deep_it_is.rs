//! Two pixels of one flat wall, faced squarely, at two different distances from
//! the eye.
//!
//! # The whole reading is that those two distances differ
//!
//! A wall faced squarely stands at one *depth* along the view direction and at a
//! different *radius* from the eye everywhere but the frame's centre. An
//! implementation carrying the tint by depth — the natural one, because depth is
//! what a depth attachment already holds — draws both pixels at the same colour
//! and is indistinguishable from a correct one anywhere near the middle of the
//! frame. So this reading takes the centre pixel and one a quarter of the
//! frame's width away from it, and asks for two colours.
//!
//! **The second distance is derived from the declared camera and not stated.**
//! The lens takes in `tan(fov/2) · aspect` of the distance to either side, so a
//! quarter of the frame's width is half of that in tangent and the ray through
//! it reaches the same plane `√(1 + t²)` times further out. At six blocks that
//! is **6.744**. Writing the number here instead would make a widened field of
//! view a disagreement between this reading and the renderer that nothing could
//! tell from a draw-path defect — and would be a committed number besides.
//!
//! # The separation is asserted, not assumed
//!
//! The two predicted colours are only worth comparing a frame against while they
//! stand further apart than the tolerance, and how far apart they stand is a
//! property of the fixture's palette that no assertion elsewhere enforces. It is
//! measured here on every run.

mod support;

use std::error::Error;

use mc_core::block::MediumTint;

use support::TestResult;
use support::medium::{
    A_QUARTER_ACROSS, REACHES_AT, Standing, TELLS_THEM_APART, THE_CENTRE, TINT, WALL_COLOUR,
    carried, eye_facing_the_wall, radially_a_quarter_across, straying_at, tinting, told_apart,
};

/// How far the eye stands from the wall, along the view direction.
const DEPTH: f32 = REACHES_AT / 2.0;

/// How many pixels this reading examines.
const TWO: usize = 2;

/// What the two pixels of the wall came to.
#[derive(Debug, PartialEq)]
struct Faced {
    the_medium_the_eye_stands_in: Option<MediumTint>,
    pixels_examined: usize,
    /// Each examined pixel drawn further than the tolerance from the colour
    /// predicted for it, named with what it drew.
    drawn_at_something_other_than_the_predicted_colour: Vec<String>,
}

#[test]
fn a_pixel_away_from_the_centre_of_a_squarely_faced_wall_is_carried_further_than_the_centre_is()
-> TestResult {
    let radially = radially_a_quarter_across(DEPTH);
    let apart = told_apart(
        carried(WALL_COLOUR, TINT, DEPTH),
        carried(WALL_COLOUR, TINT, radially),
    )?;

    let Some(faced) = what_the_two_pixels_draw()? else {
        return Ok(());
    };
    assert_eq!(
        faced,
        Faced {
            the_medium_the_eye_stands_in: tinting(TINT),
            pixels_examined: TWO,
            drawn_at_something_other_than_the_predicted_colour: Vec::new(),
        },
        "the wall stands {DEPTH} blocks along the view direction and its face is square to it, so \
         the centre pixel is {DEPTH} blocks from the eye and the one a quarter of the frame's \
         width away is {radially:.3} — the lens's own arithmetic rather than a number written \
         here. The two colours those distances predict stand ΔE {apart:.2} apart, past the ΔE \
         {TELLS_THEM_APART} this reading calls two pixels the same. A draw path carrying the tint \
         by depth along the view direction rather than by distance from the eye draws both at the \
         centre's colour, and the second pixel alone is what catches it"
    );
    Ok(())
}

/// What the frame draws at the two declared pixels, against the colours
/// predicted for them.
///
/// `None` where the opt-in permitted the absence of a device.
fn what_the_two_pixels_draw() -> Result<Option<Faced>, Box<dyn Error>> {
    let standing = Standing::behind_a_wall(Some(TINT))?;
    let (eye, target) = eye_facing_the_wall(DEPTH);
    let Some(frame) = standing.drawn(eye, target, "medium-radial")? else {
        return Ok(None);
    };
    let owed = [
        (THE_CENTRE, carried(WALL_COLOUR, TINT, DEPTH)),
        (
            A_QUARTER_ACROSS,
            carried(WALL_COLOUR, TINT, radially_a_quarter_across(DEPTH)),
        ),
    ];
    Ok(Some(Faced {
        the_medium_the_eye_stands_in: standing.about(eye),
        pixels_examined: owed.len(),
        drawn_at_something_other_than_the_predicted_colour: straying_at(&frame, &owed)?,
    }))
}
