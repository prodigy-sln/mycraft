//! What a medium does to a surface seen through it, at three distances and in
//! two declared colours.
//!
//! # Every expectation is absolute, and none is a comparison
//!
//! Each expected triple is composed by hand from the wall's own flat colour, the
//! colour the medium declares and the distance the fixture placed the eye at,
//! through `support::art`'s transfer pair — written from IEC 61966-2-1 and
//! sharing no code with the draw path. **Nothing here compares one frame against
//! another**: two frames differing only in the declared distance can agree while
//! both are wrong, and a draw path applying a constant wash regardless of the
//! eye's medium satisfies every comparison and none of these.
//!
//! The arithmetic is in linear light because the colour target is
//! `Rgba8UnormSrgb` and the hardware decodes both operands, mixes and
//! re-encodes. The same mix computed on the stored bytes lands far outside the
//! tolerance, so a prediction that did it there would be red against a correct
//! frame with a looser tolerance as its cheapest green.
//!
//! # Every sample is predicted at its own distance, not at the wall's depth
//!
//! The wall is flat and faced squarely, so it stands at one *depth* along the
//! view direction and at a different *distance from the eye* at every pixel of
//! it. The sampled grid runs to a quarter of the frame's width and a quarter of
//! its height from the centre, where the ray reaches the plane **1.161** times
//! further out — so a grid judged against the depth's own colour is red against
//! a **correct** radial draw path, and the cheapest way to green it is to carry
//! the tint by depth instead. That is the very defect
//! `the_tint_measures_how_far_a_pixel_is_from_the_eye_and_not_how_deep_it_is`
//! exists to catch, so the two readings would have demanded opposite things of
//! pixel `(960, 360)`, which both of them sample.
//!
//! Measured on the tree before this was repaired: judging the grid against one
//! colour costs **ΔE 9.75** at six blocks and **9.21** for the two-layer reading,
//! against the ΔE 3.0 that calls two pixels the same. At `1.2` blocks it costs
//! 2.23 — inside the tolerance, but by 0.77, which is a margin nobody chose. At
//! the full reach it costs 0.00, because `min(1, d / D)` clamps every sample to
//! one and that reading is the only one radial distance cannot reach.
//!
//! `medium::the_grid_tells_radius_from_depth` now asserts that spread on every
//! run for the readings that have one, so this cannot silently return.
//!
//! # The prediction this repair is judged by, written down before it can be run
//!
//! **No draw path carries a tint yet**, so the mutation that would prove this
//! repair cannot be performed on the tree it was made on. Once the draw path
//! lands it is not a mutation at all but the live question, and the answer is
//! recorded here rather than reconstructed then:
//!
//! > Carrying the tint by depth rather than by radial distance must redden
//! > **both** `a_wall_half_the_mediums_reach_away_…` and
//! > `a_pixel_away_from_the_centre_…`. **Reddening only the second means this
//! > repair did not take** — which is the state this file was in before it, with
//! > the sign flipped.
//!
//! `the_same_wall_seen_through_two_declared_colours_…` and the two-layer reading
//! carry the same guard and are expected to move with them.
//!
//! # The premise is asserted beside the colours
//!
//! Every verdict below names what the **simulation's own resolver** answered for
//! the eye the frame was drawn from. A reading that asked for colours without it
//! would be asking a prediction about an eye standing in nothing, whose frame is
//! untinted for a reason that has nothing to do with the draw path — and would
//! read as a green.

mod support;

use std::error::Error;

use mc_core::block::MediumTint;

use support::TestResult;
use support::medium::{
    OTHER_TINT, REACHES_AT, SAMPLED, Standing, TINT, WALL_COLOUR, carried, eye_facing_the_wall,
    owed_across_the_wall, straying_at, the_grid_tells_radius_from_depth, tinting, told_apart,
};

/// Whether a pose's own grid can tell a radial draw path from a depth-carrying
/// one. Both extremes of the ramp cannot, and that is a property of where they
/// sit on it rather than a gap: see [`the_grid_tells_radius_from_depth`].
const TELLS_RADIUS_FROM_DEPTH: bool = true;
const CANNOT_TELL_RADIUS_FROM_DEPTH: bool = false;

/// What one pose over one declared medium came to.
#[derive(Debug, PartialEq)]
struct Seen {
    /// What the resolver answered for the eye the frame was drawn from.
    the_medium_the_eye_stands_in: Option<MediumTint>,
    samples_examined: usize,
    /// Every sampled pixel drawn further than the tolerance from the colour
    /// predicted for it, named with what it drew.
    drawn_at_something_other_than_the_predicted_colour: Vec<String>,
}

#[test]
fn a_wall_half_the_mediums_reach_away_is_drawn_as_the_even_mix_of_its_colour_and_the_mediums()
-> TestResult {
    let half_the_reach = REACHES_AT / 2.0;
    let expected = carried(WALL_COLOUR, TINT, half_the_reach);
    told_apart(expected, WALL_COLOUR)?;
    told_apart(expected, TINT)?;

    let Some(seen) =
        what_the_eye_draws(TINT, half_the_reach, "medium-even", TELLS_RADIUS_FROM_DEPTH)?
    else {
        return Ok(());
    };
    assert_eq!(
        seen,
        nothing_strayed(TINT),
        "the eye stands inside a cell whose block declares this colour reaching its full strength \
         at {REACHES_AT} blocks, and the wall it faces stands at exactly half of that along the \
         view direction. The frame's centre is therefore the even mix, in linear light, of the \
         wall's own colour and the declared one — {expected:?} — and every other sample is that \
         same mix taken at its own pixel's distance from the eye, which the lens carries up to \
         1.161 times further out at the grid's corner. The wall's own colour appearing is a \
         medium that reached no fragment; the declared colour appearing whole is a mix that \
         ignored how far away the wall is"
    );
    Ok(())
}

#[test]
fn a_wall_at_the_mediums_full_reach_is_drawn_wholly_at_the_declared_colour() -> TestResult {
    let expected = carried(WALL_COLOUR, TINT, REACHES_AT);
    told_apart(expected, WALL_COLOUR)?;

    let Some(seen) = what_the_eye_draws(
        TINT,
        REACHES_AT,
        "medium-whole",
        CANNOT_TELL_RADIUS_FROM_DEPTH,
    )?
    else {
        return Ok(());
    };
    assert_eq!(
        seen,
        nothing_strayed(TINT),
        "at the distance the declaration states, the medium hides what lies beyond it completely: \
         every sampled pixel is the declared colour {expected:?} with none of the wall's own \
         colour left in it — at every sample and not only at the centre, because a pixel whose \
         ray reaches further out is further still past the distance the ramp reaches one at. \
         This is the one reading here radial distance cannot move. Anything of the wall showing \
         through is a ramp that never reaches one"
    );
    Ok(())
}

#[test]
fn a_wall_a_tenth_of_the_mediums_reach_away_is_drawn_a_tenth_of_the_way_toward_it() -> TestResult {
    let a_tenth_of_the_reach = REACHES_AT / 10.0;
    let expected = carried(WALL_COLOUR, TINT, a_tenth_of_the_reach);
    let from_its_own = told_apart(expected, WALL_COLOUR)?;
    let from_the_even_mix = told_apart(expected, carried(WALL_COLOUR, TINT, REACHES_AT / 2.0))?;

    let Some(seen) = what_the_eye_draws(
        TINT,
        a_tenth_of_the_reach,
        "medium-tenth",
        CANNOT_TELL_RADIUS_FROM_DEPTH,
    )?
    else {
        return Ok(());
    };
    assert_eq!(
        seen,
        nothing_strayed(TINT),
        "a surface close to the eye is barely touched, and *barely* is a measured claim rather \
         than a small one: the centre's {expected:?} stands ΔE {from_its_own:.2} from the wall's \
         own colour and ΔE {from_the_even_mix:.2} from the even mix, so this frame is \
         distinguishable both from an untinted one and from one that ignored how near the wall \
         is. Every other sample is predicted at its own pixel's distance, which at a tenth of \
         the reach is a shift this tolerance would have forgiven — by 0.77 ΔE, a margin nobody \
         chose and none of these readings rests on"
    );
    Ok(())
}

#[test]
fn the_same_wall_seen_through_two_declared_colours_is_drawn_at_two_distinct_colours() -> TestResult
{
    let half_the_reach = REACHES_AT / 2.0;
    let one = carried(WALL_COLOUR, TINT, half_the_reach);
    let other = carried(WALL_COLOUR, OTHER_TINT, half_the_reach);
    let apart = told_apart(one, other)?;

    let Some((first, second)) = what_the_two_colours_draw(half_the_reach)? else {
        return Ok(());
    };
    assert_eq!(
        (first, second),
        (nothing_strayed(TINT), nothing_strayed(OTHER_TINT)),
        "the colour is the declaration's and not the engine's, so two roots differing in that one \
         field draw the same wall at two colours — {one:?} and {other:?}, which stand ΔE \
         {apart:.2} apart. Both frames matching one of the two would be a colour coming from \
         somewhere other than the block the eye is in"
    );
    Ok(())
}

/// The verdict a frame drawn through a medium declaring `colour` owes.
fn nothing_strayed(colour: [u8; 3]) -> Seen {
    Seen {
        the_medium_the_eye_stands_in: tinting(colour),
        samples_examined: SAMPLED,
        drawn_at_something_other_than_the_predicted_colour: Vec::new(),
    }
}

/// The same wall at `blocks`, drawn once through each of the two declared
/// colours, or `None` where the opt-in permitted the absence of a device.
///
/// # Errors
///
/// As [`what_the_eye_draws`].
fn what_the_two_colours_draw(blocks: f32) -> Result<Option<(Seen, Seen)>, Box<dyn Error>> {
    let Some(first) =
        what_the_eye_draws(TINT, blocks, "medium-one-colour", TELLS_RADIUS_FROM_DEPTH)?
    else {
        return Ok(None);
    };
    let Some(second) = what_the_eye_draws(
        OTHER_TINT,
        blocks,
        "medium-other-colour",
        TELLS_RADIUS_FROM_DEPTH,
    )?
    else {
        return Ok(None);
    };
    Ok(Some((first, second)))
}

/// What an eye standing `blocks` in front of the wall draws, over a world whose
/// medium declares `colour`.
///
/// `tells_radius_from_depth` says whether this pose's grid is one whose own
/// samples can distinguish a radial draw path from a depth-carrying one, which
/// the two poses at the extremes of the ramp cannot — see
/// [`the_grid_tells_radius_from_depth`]. Asking for the guard where it cannot
/// hold would refuse a correct fixture; not asking for it where it can would
/// leave the reading merely compatible with radial distance rather than a
/// witness on it.
///
/// `None` where the opt-in permitted the absence of a device.
fn what_the_eye_draws(
    colour: [u8; 3],
    blocks: f32,
    named: &str,
    tells_radius_from_depth: bool,
) -> Result<Option<Seen>, Box<dyn Error>> {
    let standing = Standing::behind_a_wall(Some(colour))?;
    let (eye, target) = eye_facing_the_wall(blocks);
    let Some(frame) = standing.drawn(eye, target, named)? else {
        return Ok(None);
    };
    let predict = |further: f32| carried(WALL_COLOUR, colour, blocks * further);
    if tells_radius_from_depth {
        the_grid_tells_radius_from_depth(predict)?;
    }
    let owed = owed_across_the_wall(&frame, predict)?;
    Ok(Some(Seen {
        the_medium_the_eye_stands_in: standing.about(eye),
        samples_examined: SAMPLED,
        drawn_at_something_other_than_the_predicted_colour: straying_at(&frame, &owed)?,
    }))
}
