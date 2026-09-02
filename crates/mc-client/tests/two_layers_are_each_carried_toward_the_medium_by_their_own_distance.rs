//! A ray crossing a block that passes light and then meeting an opaque wall,
//! with the two standing at different distances from the eye.
//!
//! # The two answers this tells apart
//!
//! Each layer is carried toward the medium's colour by **its own** distance and
//! the carried layers are then blended. The other answer — blend the two layers
//! untinted and carry the result by the nearer one's distance — is what a
//! full-screen pass sampling a depth attachment is obliged to produce, because a
//! translucent face writes no depth and nothing anywhere holds the nearer
//! distance. It is a third colour, neither this one nor an untinted frame, and
//! it is named in the assertion below so that a failure says which of the two
//! was drawn.
//!
//! # Where the numbers come from
//!
//! The pane's face stands at `x = 14.0` and the wall's at `x = 20.0`, so an eye
//! at `x = 11.0` sees them at exactly **3.0** and **9.0** blocks — the
//! difference of two declared numbers, with nothing measured off a march. The
//! pane declares half a degree, so the composition is an even one, and the
//! medium reaches its full strength at twelve so the two layers are carried a
//! quarter and three quarters of the way.

mod support;

use std::error::Error;

use mc_core::block::MediumTint;

use support::TestResult;
use support::art::composited;
use support::medium::{
    HALF, PANE_COLOUR, SAMPLED, Standing, TINT, WALL_COLOUR, carried, owed_across_the_wall,
    straying_at, the_grid_tells_radius_from_depth, tinting, told_apart,
};

/// Where the eye stands, and what it looks at: along `+X` down the row the pane
/// and the wall both cross.
const EYE: [f32; 3] = [11.0, support::medium::EYE_Y, support::medium::EYE_Z];
const LOOK_AT: [f32; 3] = [12.0, support::medium::EYE_Y, support::medium::EYE_Z];

/// How far each of the two surfaces stands from that eye.
const THE_PANE_STANDS_AT: f32 = 3.0;
const THE_WALL_STANDS_AT: f32 = 9.0;

/// What the frame came to.
#[derive(Debug, PartialEq)]
struct Composed {
    the_medium_the_eye_stands_in: Option<MediumTint>,
    samples_examined: usize,
    drawn_at_something_other_than_the_predicted_colour: Vec<String>,
}

#[test]
fn a_layer_and_what_stands_behind_it_are_each_carried_by_how_far_away_they_are() -> TestResult {
    let each_by_its_own = each_layer_by_its_own_distance();
    let blended_then_carried = blended_and_then_carried();
    let apart = told_apart(each_by_its_own, blended_then_carried)?;
    told_apart(each_by_its_own, untinted_blend())?;

    let Some(composed) = what_the_ray_draws()? else {
        return Ok(());
    };
    assert_eq!(
        composed,
        Composed {
            the_medium_the_eye_stands_in: tinting(TINT),
            samples_examined: SAMPLED,
            drawn_at_something_other_than_the_predicted_colour: Vec::new(),
        },
        "the pane stands {THE_PANE_STANDS_AT} blocks from the eye and the wall behind it \
         {THE_WALL_STANDS_AT}, so each is carried toward the medium's colour by its own distance \
         and the two are then blended — {each_by_its_own:?} at the frame's centre, and the same \
         composition at every other sample's own distance, both layers being carried further out \
         by the one factor that pixel's ray adds to the depth. Blending them first and carrying \
         the result by the nearer distance gives {blended_then_carried:?}, ΔE {apart:.2} away, \
         which is what a pass with one distance per pixel is obliged to draw"
    );
    Ok(())
}

/// Each layer carried toward the medium by its own distance, and then blended:
/// the colour the law states.
fn each_layer_by_its_own_distance() -> [u8; 3] {
    composited(
        carried(PANE_COLOUR, TINT, THE_PANE_STANDS_AT),
        carried(WALL_COLOUR, TINT, THE_WALL_STANDS_AT),
        f64::from(HALF),
    )
}

/// The two layers blended untinted and the result carried by the nearer
/// distance: what a pass holding one distance per pixel is obliged to draw.
fn blended_and_then_carried() -> [u8; 3] {
    carried(untinted_blend(), TINT, THE_PANE_STANDS_AT)
}

/// The two layers blended with no medium at all.
fn untinted_blend() -> [u8; 3] {
    composited(PANE_COLOUR, WALL_COLOUR, f64::from(HALF))
}

/// What the declared pose draws, against `expected`.
///
/// `None` where the opt-in permitted the absence of a device.
fn what_the_ray_draws() -> Result<Option<Composed>, Box<dyn Error>> {
    let standing = Standing::behind_a_pane_and_a_wall(Some(TINT))?;
    let Some(frame) = standing.drawn(EYE, LOOK_AT, "medium-two-layers")? else {
        return Ok(None);
    };
    let predict = |further: f32| {
        composited(
            carried(PANE_COLOUR, TINT, THE_PANE_STANDS_AT * further),
            carried(WALL_COLOUR, TINT, THE_WALL_STANDS_AT * further),
            f64::from(HALF),
        )
    };
    the_grid_tells_radius_from_depth(predict)?;
    let owed = owed_across_the_wall(&frame, predict)?;
    Ok(Some(Composed {
        the_medium_the_eye_stands_in: standing.about(EYE),
        samples_examined: SAMPLED,
        drawn_at_something_other_than_the_predicted_colour: straying_at(&frame, &owed)?,
    }))
}
