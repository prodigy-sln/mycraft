//! What a frame of [`super`]'s fixture is predicted to hold, and how a reading
//! judges it against that prediction.
//!
//! **The seam is the question each half answers.** The parent module builds the
//! world — a medium filling the cells an eye stands in, an opaque wall a stated
//! distance beyond it, meshed and drawn through the client's own path — and its
//! header carries why every distance there is exact. This module never touches a
//! world: it holds the **law applied to a distance**, the **grid** a reading
//! samples, the **tolerance** it judges by, and the **guard** that asks whether
//! a grid can see the thing its reading is about.
//!
//! They were one file until it crossed the 600-line cap, and the cap is what
//! forced the question rather than what answered it: a fixture and the
//! instrument that reads it are two responsibilities, and the day the wall moves
//! is not the day the tolerance changes.

use std::error::Error;

use mc_render::camera::projection_for;
use mc_testkit::frame::Rgba8Image;

use crate::support::frames::CAPTURE_SIZE;
use crate::support::probe::{distance, pixel_color};

use super::{ACROSS, COLUMNS, tinting};

/// How far a rendered pixel may sit from the colour predicted for it, in ΔE.
///
/// **Measured from both directions, against a correct tinted frame.** The
/// number has not moved; what it is derived *from* has. It was argued from
/// `support/translucency.rs`'s ΔE 2.68 attachment-rounding term "over
/// comparable colours" to "a floor near ΔE 3.0" — and that term is over a
/// **different palette**. It over-states this fixture's own measured floor by
/// **2.25×**. The value survived because the borrowed number erred
/// conservatively, which is luck rather than derivation, and a comment nobody
/// can re-run is not a derivation at all.
///
/// The admissible band is **1.19 < T < 7.45**, and each bound was taken by a
/// named instrument a later reader can drive again:
///
/// - **Floor ΔE 1.19**, by a downward search: lower this constant until a
///   correct frame strays, and two samples stray at **1.15** — both in the
///   two-layer reading, which composites two blends and therefore carries two
///   roundings through the eight-bit sRGB attachment rather than one. That is
///   what a correct frame can still be off by *here*, on this palette.
/// - **Ceiling ΔE 7.45**, by an upward search: raise it until a reading stops
///   being able to tell its own colours apart. The pair that binds first is
///   [`told_apart`]'s in FR-2.1-S4 — the mix for six blocks against the mix for
///   the 6.746 a quarter of the frame's width away — and it binds well before
///   the grid separations of 9.21 and 9.75 do.
/// - **Independently, ΔE 12.90**: forcing the draw path's reach to no tint at
///   all reddens nine of the ten readings, and the smallest per-sample signal
///   any of them loses is 12.90. So the thing this tolerance exists to see
///   stands an order of magnitude clear of it.
///
/// `3.0` sits within 1% of `√(1.19 × 7.45) = 2.98`, the geometric mean of its
/// own bounds. **That is where it lands, not why it was chosen** — it was
/// chosen before any of this could be measured, and the measurement is what
/// now says it was a defensible choice rather than a lucky one.
///
/// [`told_apart`] still measures the ceiling's binding pair on every run, so
/// that half is asserted rather than argued.
pub const TELLS_THEM_APART: f64 = 3.0;

/// How much further from the eye everything along the ray through `pixel`
/// stands than the same surface would at the frame's centre, as a multiple.
///
/// **The one number the whole file's radial arithmetic reduces to.** A wall
/// faced squarely stands at one *depth* along the view direction, and the ray
/// through a pixel `t` off the axis in tangent reaches that plane `√(1 + t²)`
/// times further out. That multiple is a property of the *pixel* and not of the
/// surface, so every layer a ray crosses is further out by the same factor —
/// which is what lets a two-layer prediction be written as one factor applied
/// to both distances.
///
/// **Derived from the declared camera rather than stated**, so a widened field
/// of view moves the prediction with the renderer instead of becoming a
/// disagreement nothing could tell from a draw-path defect. The pixel's own
/// centre is where the rasteriser samples, hence the half.
#[must_use]
pub fn further_at(pixel: (u32, u32)) -> f32 {
    let projection = projection_for(CAPTURE_SIZE);
    let half = (projection.fov_y_radians / 2.0).tan();
    let across =
        half * projection.aspect * (2.0 * (pixel.0 as f32 + 0.5) / CAPTURE_SIZE.width as f32 - 1.0);
    let down = half * (1.0 - 2.0 * (pixel.1 as f32 + 0.5) / CAPTURE_SIZE.height as f32);
    (1.0 + across * across + down * down).sqrt()
}

/// How far a pixel a quarter of the frame's width from the centre stands from an
/// eye `depth` blocks from a wall it faces squarely, in blocks.
///
/// [`further_at`] at [`A_QUARTER_ACROSS`], which is what makes this the same
/// arithmetic every sample below is predicted through rather than a second
/// statement of it. At six blocks it is `6.746`.
#[must_use]
pub fn radially_a_quarter_across(depth: f32) -> f32 {
    depth * further_at(A_QUARTER_ACROSS)
}

/// The frame's own centre pixel, and one a quarter of its width to the right of
/// it on the same row.
///
/// **Written out rather than divided out**, because a `u32` division is a lint
/// this workspace refuses and a cast through `f32` would be a second rounding
/// nobody needs. [`the_geometry_holds`] multiplies them back up against the
/// declared capture size on every run, so the two cannot drift apart quietly.
pub const THE_CENTRE: (u32, u32) = (640, 360);
pub const A_QUARTER_ACROSS: (u32, u32) = (960, 360);

/// The corner the sample grid starts at and how far apart its samples stand.
///
/// A quarter in from each edge and half the frame across, so every sample lands
/// on the wall at every pose here. Same rule as above: stated, and checked back
/// against the capture size by multiplication.
const FIRST_SAMPLE: (u32, u32) = (320, 180);
const SAMPLE_STEP: (u32, u32) = (32, 36);
const SAMPLE_COLUMNS: u32 = 21;
const SAMPLE_ROWS: u32 = 11;

/// Refuses a capture size the stated pixels above no longer describe.
///
/// # Errors
///
/// Returns an error naming both when the frame is not twice the centre pixel in
/// each direction, or when the sample grid would run past its edge.
fn the_geometry_holds() -> Result<(), Box<dyn Error>> {
    let footprint = COLUMNS * mc_world::section::SECTION_SIZE == ACROSS;
    let centred = footprint
        && THE_CENTRE.0 * 2 == CAPTURE_SIZE.width
        && THE_CENTRE.1 * 2 == CAPTURE_SIZE.height;
    let quartered = A_QUARTER_ACROSS.0 * 4 == CAPTURE_SIZE.width * 3
        && A_QUARTER_ACROSS.1 == THE_CENTRE.1
        && FIRST_SAMPLE.0 * 4 == CAPTURE_SIZE.width
        && FIRST_SAMPLE.1 * 4 == CAPTURE_SIZE.height
        && SAMPLE_STEP.0 * SAMPLE_COLUMNS < CAPTURE_SIZE.width
        && SAMPLE_STEP.1 * SAMPLE_ROWS < CAPTURE_SIZE.height;
    if centred && quartered {
        return Ok(());
    }
    Err(format!(
        "the pixels these readings name are stated for a {} x {} capture and the declared size is          {} x {}. Every distance and every sample below was derived for the first, so a reading          over the second would be looking at pixels nobody worked out",
        THE_CENTRE.0 * 2,
        THE_CENTRE.1 * 2,
        CAPTURE_SIZE.width,
        CAPTURE_SIZE.height
    )
    .into())
}

/// The colour a surface of `own` is drawn at, seen `blocks` away through a
/// medium declaring `colour` and reaching full strength at [`REACHES_AT`].
///
/// The law, in linear light, through the transfer pair `support::art` declares
/// from IEC 61966-2-1 — which shares no code with the draw path.
#[must_use]
pub fn carried(own: [u8; 3], colour: [u8; 3], blocks: f32) -> [u8; 3] {
    crate::support::composite::carried(own, tinting(colour), blocks)
}

/// Refuses a pair of predicted colours the tolerance cannot tell apart.
///
/// **The half of the tolerance no reading can measure for itself.** Two mixes
/// standing nearer than [`TELLS_THEM_APART`] are two mixes a frame cannot be
/// judged between, so a reading resting on the pair would accept a draw path
/// that had confused them.
///
/// # Errors
///
/// Returns an error naming both colours and the distance between them when they
/// stand no further apart than the tolerance.
pub fn told_apart(one: [u8; 3], other: [u8; 3]) -> Result<f64, Box<dyn Error>> {
    let apart = distance(one, other)?;
    if apart <= TELLS_THEM_APART {
        return Err(format!(
            "this fixture's own colours have to stand further apart than the {TELLS_THEM_APART} \
             ΔE a reading here calls two pixels the same, and {one:?} against {other:?} measures \
             {apart:.2}. Nothing asserted over them would be able to tell one from the other, so \
             the reading would pass for a draw path that had drawn either"
        )
        .into());
    }
    Ok(apart)
}

/// One examined pixel and the colour paired with it — drawn, where a reading is
/// reporting, and predicted, where it is owing.
pub type SampledPixel = ((u32, u32), [u8; 3]);

/// Every sample pixel of the declared region, and what `frame` drew at each.
///
/// A grid across the middle half of the frame, which the wall covers at every
/// pose here. **More than a hundred**, so a reading over it is over a region and
/// not over one lucky pixel.
///
/// # Errors
///
/// Returns an error for a pixel outside `frame`.
pub fn across_the_wall(frame: &Rgba8Image) -> Result<Vec<SampledPixel>, Box<dyn Error>> {
    the_geometry_holds()?;
    let mut read = Vec::new();
    for pixel in sampled_pixels() {
        read.push((pixel, pixel_color(frame, pixel)?));
    }
    Ok(read)
}

/// The pixels [`across_the_wall`] examines, left to right and then down.
///
/// Stated once so the grid a reading judges and the grid a fixture guard
/// measures its spread over cannot come apart.
fn sampled_pixels() -> impl Iterator<Item = (u32, u32)> {
    (0..SAMPLE_ROWS)
        .flat_map(|row| (0..SAMPLE_COLUMNS).map(move |column| (row, column)))
        .map(|(row, column)| {
            (
                FIRST_SAMPLE.0 + column * SAMPLE_STEP.0,
                FIRST_SAMPLE.1 + row * SAMPLE_STEP.1,
            )
        })
}

/// Refuses a grid whose own samples could not tell a radial draw path from one
/// carrying the tint by depth, and answers how far apart the two stand.
///
/// **The check whose absence let the contradiction compile.**
/// [`the_geometry_holds`] asks only that the grid fit inside the frame and the
/// wall; nothing asked whether the grid could *see* the thing its readings are
/// about. Because `predict` is handed a multiple, `predict(1.0)` is exactly the
/// colour a pass carrying the tint by **depth** would draw at every sample — so
/// comparing it against the worst sample's own prediction asks, on every run,
/// whether this reading is a witness on radial distance or merely compatible
/// with one.
///
/// **The same closure the prediction uses**, so a guard that passed and a
/// prediction that judged cannot be about two different laws.
///
/// Two of this file's readings deliberately do **not** call it, and both are
/// properties rather than omissions. At the declared reach `min(1, d / D)`
/// saturates and every sample predicts the declared colour exactly, spread
/// ΔE 0.00 — radial distance cannot reach that reading at all. At a tenth of the
/// reach the spread is ΔE 2.23, inside the tolerance by 0.77: that reading is
/// about how *little* a near surface is touched and it cannot tell radius from
/// depth, which is what [`radially_a_quarter_across`]'s own reading is for.
///
/// # Errors
///
/// Returns an error naming both colours and the distance between them when the
/// grid's widest spread is one the tolerance would call the same colour.
pub fn the_grid_tells_radius_from_depth(
    predict: impl Fn(f32) -> [u8; 3],
) -> Result<f64, Box<dyn Error>> {
    let by_depth = predict(1.0);
    let mut widest = (0.0, by_depth, (0, 0));
    for pixel in sampled_pixels() {
        let there = predict(further_at(pixel));
        let apart = distance(by_depth, there)?;
        if apart > widest.0 {
            widest = (apart, there, pixel);
        }
    }
    if widest.0 <= TELLS_THEM_APART {
        return Err(format!(
            "this reading judges {SAMPLED} samples of a squarely faced wall, and a pass carrying \
             the tint by depth along the view direction rather than by distance from the eye \
             would draw every one of them at {by_depth:?}. Its widest sample predicts \
             {:?}, ΔE {:.2} away, which is inside the ΔE {TELLS_THEM_APART} this fixture calls \
             two pixels the same — so nothing asserted over this grid could tell the two apart, \
             and a depth-carrying draw path would pass it",
            widest.1, widest.0
        )
        .into());
    }
    Ok(widest.0)
}

/// Every sampled pixel of `frame` paired with the colour `predict` gives for
/// how much further out that pixel's own ray reaches.
///
/// **One colour cannot serve the grid, and that is the whole of why this takes
/// a closure.** The wall is flat and faced squarely, so its *depth* is one
/// number and its *distance from the eye* is a different one at every pixel —
/// [`further_at`] by as much as `1.161` at the grid's corner, which at six
/// blocks is a predicted colour standing **ΔE 9.75** from the centre's against
/// the ΔE {`TELLS_THEM_APART`} that calls two pixels the same. A grid judged
/// against the centre's colour is therefore red against a *correct* radial draw
/// path, and its cheapest green is to carry the tint by depth — which is
/// precisely the defect `the_tint_measures_how_far_a_pixel_is_from_the_eye_...`
/// exists to catch. Measured before the repair: ΔE 9.75 at six blocks, 9.21 for
/// the two-layer reading, 2.23 at 1.2 blocks (inside the tolerance, but by
/// 0.77), and 0.00 at twelve where `min(1, d / D)` clamps every sample to one.
///
/// `predict` is handed the **multiple**, not a distance, because every layer a
/// ray crosses is further out by the same factor — so a two-layer prediction is
/// one factor applied to both of its distances.
///
/// # Errors
///
/// Returns an error for a pixel outside `frame`.
pub fn owed_across_the_wall(
    frame: &Rgba8Image,
    predict: impl Fn(f32) -> [u8; 3],
) -> Result<Vec<SampledPixel>, Box<dyn Error>> {
    Ok(across_the_wall(frame)?
        .into_iter()
        .map(|(pixel, _drawn)| (pixel, predict(further_at(pixel))))
        .collect())
}

/// Every named pixel of `frame` standing further than [`TELLS_THEM_APART`] from
/// the colour paired with it, named with what it drew.
///
/// # Errors
///
/// Returns an error for a pixel outside `frame`, or the distance metric's own.
pub fn straying_at(
    frame: &Rgba8Image,
    owed: &[SampledPixel],
) -> Result<Vec<String>, Box<dyn Error>> {
    the_geometry_holds()?;
    let mut strayed = Strays::default();
    for (pixel, expected) in owed {
        let drawn = pixel_color(frame, *pixel)?;
        let stands = distance(drawn, *expected)?;
        if stands <= TELLS_THEM_APART {
            continue;
        }
        strayed.note(format!(
            "{pixel:?} drew {drawn:?} where {expected:?} was predicted, ΔE {stands:.2} away"
        ));
    }
    Ok(strayed.named())
}

/// The straying pixels a failure names, and how many more it only counts.
///
/// **A whole frame of them says what a handful says and buries the verdict they
/// are attached to**, so the list is capped and the rest are reported as a
/// number — which is still an enumerated answer, because an empty list is the
/// only clean one.
#[derive(Debug, Default)]
pub struct Strays {
    named: Vec<String>,
    counted: usize,
}

impl Strays {
    /// Records one straying pixel.
    pub fn note(&mut self, stray: String) {
        self.counted += 1;
        if self.named.len() < NAMED_STRAYS {
            self.named.push(stray);
        }
    }

    /// The named strays, with a count of whatever would not fit.
    #[must_use]
    pub fn named(mut self) -> Vec<String> {
        if self.counted > self.named.len() {
            self.named.push(format!(
                "and {} more like them",
                self.counted - self.named.len()
            ));
        }
        self.named
    }
}

/// How many straying pixels a failure names before it starts counting them.
const NAMED_STRAYS: usize = 8;

/// How many pixels [`across_the_wall`] examines.
pub const SAMPLED: usize = (SAMPLE_ROWS * SAMPLE_COLUMNS) as usize;
