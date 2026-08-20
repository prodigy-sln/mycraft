//! What a rendered frame shows where the base game's own declarations say it
//! should, judged per pixel against a prediction that shares no code with the
//! composition it grades.
//!
//! # Why a golden comparison cannot do this
//!
//! The harness's default tolerance budgets `0.0001 × 1280 × 720` = **92** wrong
//! pixels, and the base crosshair's fill is **17**. Every pixel of it could
//! therefore be a different colour and a default comparison would still return
//! Match. So the golden is *preceded* by the reading below: every pixel the
//! declarations predict, judged against the colour predicted for it, **with no
//! area budget** — one failing pixel is a mismatch — and paired with two controls
//! that must fail.
//!
//! # The prediction is a second derivation, and this file is a third
//!
//! `support::prediction` re-derives `architecture.md`'s Decision 8 from the
//! declarations alone; the rectangles below are worked out by hand from the same
//! text, and a fixture guard refuses to go on unless the two agree. Nothing here
//! is read back from a rendered frame.
//!
//! At the declared capture size of 1280 × 720 the scale is `720 / 720 = 1`, so a
//! UI unit is a physical pixel, `round` is half away from zero, and the safe-area
//! insets are `round(0.05 × 1280) = 64` and `round(0.05 × 720) = 36`. Ranges
//! below are half open.
//!
//! - **`base:crosshair-horizontal`**, `[9, 1]` at `center`:
//!   `left = round(640 − 4.5) = 636`, `top = round(360 − 0.5) = 360`. Fill
//!   `636..645 × 360..361` = 9 px; grown by its one unit of outline,
//!   `635..646 × 359..362` = 11 × 3 = **33**.
//! - **`base:crosshair-vertical`**, `[1, 9]` at `center`:
//!   `left = round(640 − 0.5) = 640`, `top = round(360 − 4.5) = 356`. Fill
//!   `640..641 × 356..365` = 9 px; footprint `639..642 × 355..366` = 3 × 11 =
//!   **33**.
//! - The two footprints overlap in the 3 × 3 square `639..642 × 359..362`, so
//!   they cover `33 + 33 − 9` = **57** pixels between them, of which
//!   `9 + 9 − 1` = **17** are fill.
//! - **`base:held-block`**, `[24, 24]` at `bottom`: `left = round(640 − 12) =
//!   628`, `top = 720 − 36 − 24 = 660`. Fill `628..652 × 660..684` = 576;
//!   footprint `627..653 × 659..685` = 26 × 26 = **676**, of which 100 are ring.
//! - All three footprints are disjoint, so they cover `57 + 676` = **733** of the
//!   frame's `1280 × 720` = 921 600 pixels, leaving **920 867** outside.
//!
//! # Which colour is due where, and why the two bars decide it together
//!
//! Both bars declare `color = "#FFFFFFFF"` and `outline = "#000000FF"`, both
//! opaque — so the composite is the declared colour itself, and the sRGB decode
//! and the target's re-encode are inverse operations around it. Rings are
//! composed in one pass and fills in a second, so at `(640, 359)` the upright's
//! **fill** wins over the crossbar's **ring**: white, not black. A composition
//! that outlined and filled per element paints a black notch there, which is why
//! the prediction models the two passes rather than one element at a time.
//!
//! # What this reading cannot see, measured rather than assumed
//!
//! **It does not grade the CPU-side sRGB decode of a declared colour.** Both
//! colours the shipped crosshair declares are built from bytes 0 and 255, and both
//! are fixed points of the transfer function, so `decode(255/255)` and `255/255`
//! are the same number. Measured: replacing the decode in `mc_render::hud`'s
//! uniform with a bare `channel / 255` leaves every reading here green. That hole
//! is closed elsewhere — by the `#808080` scenario in
//! `crates/mc-render/tests/hud_offscreen.rs`, which is the one colour this spec
//! declares that the transfer function moves — and a reader must not take a green
//! reading here as evidence about it.
//!
//! # Why the frames hold a block
//!
//! A prediction from declarations alone predicts what a declaration draws when its
//! draw resolves, and a textured swatch resolves only while a session holds a
//! block whose texture occupies a layer. So every frame here is captured holding
//! the block a client of `content/base/` would hold, through the client's own
//! `default_held_block` and `held_swatch`.

mod support;

use std::error::Error;

use mc_render::hud::held_swatch;
use mc_render::surface::SurfaceSize;
use mc_testkit::frame::Rgba8Image;
use mc_testkit::frame::gpu::CaptureContext;

use support::hud_frames::{
    Comparison, HudCapture, Rect, compare_frames, default_block_held, hud_holding_default_block,
    hud_of, no_hud,
};
use support::prediction::{PixelVerdict, PredictedPaint, Prediction, per_pixel_reading};
use support::swatch::{SwatchReading, drawn_colors_of, require, swatch_reading};
use support::{TestResult, content, content_root, frames};

/// The block declarations moved aside so that a client holds **stone**.
///
/// **A fixture the shipped art forced, and the reason is worth keeping.** A
/// client holds the first solid block in registration order, which is dirt, and
/// the indicator then draws the baked dirt palette — three browns that are also
/// four fifths of every grass side, and the terrain behind the indicator at this
/// pose is grass side. So every pixel of the footprint already read as a colour
/// the indicator draws, and `every pixel of the footprint moves` was red against
/// a correct renderer.
///
/// Moving dirt and grass aside leaves `stone`, `water`, `zz-dirt`, `zz-grass`
/// and a client holding stone. Stone's greys share no colour with anything the
/// ground is made of: the nearest pair stands ΔE 21.13 apart, against the ΔE 2.0
/// that calls two colours the same.
///
/// **The assertion did not move and must not.** What moved is which block is
/// held, which this file has no claim about at all — its subject is where the
/// indicator lands and what it covers.
const HELD_MOVED_ASIDE: [(&str, &str); 2] = [
    ("dirt.luau", "zz-dirt.luau"),
    ("grass.luau", "zz-grass.luau"),
];

/// The tick every frame here is drawn at.
const TICK: u32 = 0;

/// The size every declared capture is taken at, and the target every rectangle
/// in this file's header is derived for.
const TARGET: SurfaceSize = frames::CAPTURE_SIZE;

/// The three elements `content/base/` declares, in the order a file-name sorted
/// read of `hud/` yields them.
const CROSSBAR: &str = "base:crosshair-horizontal";
const UPRIGHT: &str = "base:crosshair-vertical";
const INDICATOR: &str = "base:held-block";
const DECLARED_ELEMENTS: [&str; 3] = [CROSSBAR, UPRIGHT, INDICATOR];

/// The two elements the crosshair is composed from — the ones FR-4.1's per-pixel
/// reading is stated over.
const CROSSHAIR: [&str; 2] = [CROSSBAR, UPRIGHT];

/// The files the two bars are declared in.
const CROSSHAIR_DECLARATIONS: [&str; 2] = ["crosshair-horizontal.toml", "crosshair-vertical.toml"];

/// Where the three declarations' fills land, derived by hand in this file's
/// header, and their footprints — each fill grown by the one unit of outline the
/// declaration states.
const CROSSBAR_FILL: Rect = Rect {
    x: 636,
    y: 360,
    width: 9,
    height: 1,
};
const UPRIGHT_FILL: Rect = Rect {
    x: 640,
    y: 356,
    width: 1,
    height: 9,
};
const INDICATOR_FILL: Rect = Rect {
    x: 628,
    y: 660,
    width: 24,
    height: 24,
};
const CROSSBAR_FOOTPRINT: Rect = CROSSBAR_FILL.grown_by(1);
const UPRIGHT_FOOTPRINT: Rect = UPRIGHT_FILL.grown_by(1);
const INDICATOR_FOOTPRINT: Rect = INDICATOR_FILL.grown_by(1);

/// How many pixels the two bars' footprints cover between them: `33 + 33 − 9`.
const CROSSHAIR_PIXELS: u64 = 57;

/// How many pixels all three footprints cover: `57 + 676`.
const PREDICTED_PIXELS: u64 = 733;

/// How many pixels one declared capture holds: `1280 × 720`.
const FRAME_PIXELS: u64 = 921_600;

/// The one-pixel border immediately outside the indicator's footprint:
/// `28 × 28 − 26 × 26`.
const BORDER_PIXELS: u64 = 108;

#[test]
fn every_pixel_the_crosshair_declarations_predict_shows_the_colour_predicted_for_it() -> TestResult
{
    let Some(context) = frames::device()? else {
        return Ok(());
    };
    let prediction = shipped_prediction()?;
    require_placed_where_derived(&prediction)?;
    let shot = captured(&context)?;

    let reading = per_pixel_reading(&shot.declared, &prediction, &CROSSHAIR)?;
    assert_eq!(
        (reading.verdict, reading.considered),
        (PixelVerdict::EveryPredictedPixelShowsIt, CROSSHAIR_PIXELS),
        "every pixel the two bars declare has to show the colour their declaration states there, \
         judged one pixel at a time with no area budget: the golden comparison this precedes \
         forgives 92 wrong pixels and the fill is 17, so a crosshair drawn in the wrong colour, \
         the wrong place or the wrong shape passes it. {reading:?}"
    );
    Ok(())
}

#[test]
fn every_pixel_outside_the_predicted_footprints_is_what_a_frame_with_no_hud_shows_there()
-> TestResult {
    let Some(context) = frames::device()? else {
        return Ok(());
    };
    let prediction = shipped_prediction()?;
    require_placed_where_derived(&prediction)?;
    let shot = captured(&context)?;
    require_something_was_drawn(&compare_frames(&shot.declared, &shot.bare, |x, y| {
        prediction.covers(x, y)
    }))?;

    let seen = compare_frames(&shot.declared, &shot.bare, |x, y| !prediction.covers(x, y));
    assert_eq!(
        (seen.considered, seen.different),
        (FRAME_PIXELS - PREDICTED_PIXELS, 0),
        "a HUD paints where its declarations say and nowhere else: a stage that cleared the \
         target, tinted the frame or drew a rectangle the declarations do not predict would move \
         a pixel out here, and every one of them is a pixel of the world the player is looking at. \
         {seen:?}"
    );
    Ok(())
}

#[test]
fn the_per_pixel_reading_reports_a_mismatch_against_a_frame_with_no_hud_element_at_all()
-> TestResult {
    let Some(context) = frames::device()? else {
        return Ok(());
    };
    let prediction = shipped_prediction()?;
    let shot = captured(&context)?;

    let reading = per_pixel_reading(&shot.bare, &prediction, &CROSSHAIR)?;
    assert_eq!(
        (reading.verdict, reading.considered),
        (PixelVerdict::Strayed, CROSSHAIR_PIXELS),
        "the reading has to be able to fail: applied to a frame rendered with zero HUD elements it \
         must report a mismatch, because a check that cannot fail on an empty frame is not \
         evidence that a crosshair is there. It looked at the same {CROSSHAIR_PIXELS} pixels it \
         judges the shipped frame by. {reading:?}"
    );
    Ok(())
}

#[test]
fn the_per_pixel_reading_reports_a_mismatch_when_the_bars_are_filled_with_their_outline_colour()
-> TestResult {
    let Some(context) = frames::device()? else {
        return Ok(());
    };
    let prediction = shipped_prediction()?;
    require_placed_where_derived(&prediction)?;
    let root = content::shipped_filling_with_the_outline_color(&CROSSHAIR_DECLARATIONS)?;
    let mut frames_of = HudCapture::ready(&context, TICK)?;
    let request = frames::request(&context, "hud-prediction-filled-with-its-outline")?;
    let frame = frames_of.capture(&hud_of(root.path())?, &request)?;
    require_filled_with_the_outline_colour(&frame, &prediction)?;

    let reading = per_pixel_reading(&frame, &prediction, &CROSSHAIR)?;
    assert_eq!(
        (reading.verdict, reading.considered),
        (PixelVerdict::Strayed, CROSSHAIR_PIXELS),
        "the reading grades the colour a declaration states and not merely that something was \
         painted: these bars are drawn, in the very colour the same declaration reserves for its \
         outline, and a check satisfied by paint rather than by the right paint would call a \
         crosshair that has gone black against a cave wall correct. {reading:?}"
    );
    Ok(())
}

#[test]
fn the_indicator_covers_the_footprint_its_declaration_predicts_and_not_the_pixel_outside_it()
-> TestResult {
    let Some(context) = frames::device()? else {
        return Ok(());
    };
    let prediction = shipped_prediction()?;
    require_placed_where_derived(&prediction)?;
    let shot = captured(&context)?;
    require_nothing_already_reads_as_the_indicator(&shot, &prediction)?;

    let inside = compare_frames(&shot.declared, &shot.bare, |x, y| {
        INDICATOR_FOOTPRINT.holds(x, y)
    });
    let border = compare_frames(&shot.declared, &shot.bare, just_outside_the_indicator);
    assert_eq!(
        (
            (inside.considered, inside.same),
            (border.considered, border.different)
        ),
        ((INDICATOR_FOOTPRINT.area(), 0), (BORDER_PIXELS, 0)),
        "the indicator covers exactly what its declaration predicts: every pixel of the footprint \
         moves and the ring of pixels immediately outside it does not. An indicator one pixel too \
         large satisfies an inside-only claim, and one pixel too small leaves the world showing \
         through a square a player reads as a swatch. inside {inside:?}, border {border:?}"
    );
    Ok(())
}

#[test]
fn the_base_game_declares_exactly_the_three_hud_elements_the_prediction_predicts_from() -> TestResult
{
    let prediction = shipped_prediction()?;

    assert_eq!(
        prediction.names(),
        DECLARED_ELEMENTS,
        "the prediction is derived from what `content/base/` declares, so what it declares is the \
         prediction's input contract: a fourth element would be painted onto every frame these \
         scenarios compare while no rectangle predicted it, and a renamed one would be predicted \
         by nothing and drawn all the same"
    );
    Ok(())
}

/// The prediction the shipped declarations make on a declared capture.
///
/// # Errors
///
/// Returns the refusal when the shipped root's declarations do not load.
fn shipped_prediction() -> Result<Prediction, Box<dyn Error>> {
    Prediction::of(&content_root()?, TARGET)
}

/// One capture of the replay with the shipped HUD over it and the same capture
/// with a layout that declares nothing, plus the block the first one holds.
#[derive(Debug)]
struct Shot {
    declared: Rgba8Image,
    bare: Rgba8Image,
    /// The colours the held block's indicator is drawn from.
    ///
    /// **Carried rather than derived later, because deriving it needs the
    /// content root that took this shot.** A key the built set covers draws its
    /// image and a key it does not draws the generated texture, and only the
    /// preparation behind these two frames knows which of the two this block is.
    drawn: Vec<[u8; 3]>,
}

/// The two frames every comparison here is between.
///
/// One preparation, one renderer and one snapshot, so the two cannot differ in
/// anything but the HUD — and the zero-element frame is a layout that declares
/// nothing rather than a frame with the HUD stage skipped, which is the harder
/// comparison of the two.
///
/// # Errors
///
/// Returns the preparation, pipeline, upload or capture failure, or the refusal
/// when the shipped declarations do not load.
fn captured(context: &CaptureContext) -> Result<Shot, Box<dyn Error>> {
    let holding_stone = content::shipped_renaming_blocks(&HELD_MOVED_ASIDE)?;
    let mut frames_of = HudCapture::over(context, TICK, holding_stone.path())?;
    let held = default_block_held(&frames_of.content)?;
    let shipped = hud_holding_default_block(&content_root()?, &frames_of.content)?;
    let request = frames::request(context, "hud-prediction-declared")?;
    let declared = frames_of.capture(&shipped, &request)?;
    let request = frames::request(context, "hud-prediction-nothing-declared")?;
    let bare = frames_of.capture(&no_hud()?, &request)?;
    let drawn = drawn_colors_of(
        &held_swatch(Some(&held), &frames_of.content.resolution)
            .texture()
            .ok_or("the block a client holds has to resolve to a key for its indicator to draw")?,
        &frames_of.content.texels,
    )?;
    Ok(Shot {
        declared,
        bare,
        drawn,
    })
}

/// Whether `(x, y)` is one of the pixels immediately outside the indicator's
/// footprint.
fn just_outside_the_indicator(x: u32, y: u32) -> bool {
    INDICATOR_FOOTPRINT.grown_by(1).holds(x, y) && !INDICATOR_FOOTPRINT.holds(x, y)
}

/// Fails unless the prediction places all three declarations exactly where this
/// file's header derives them by hand.
///
/// Two derivations of one rule are evidence; one derivation nobody checked is a
/// rectangle the whole suite then trusts.
///
/// # Errors
///
/// Returns a failure naming the first element the two disagree about.
fn require_placed_where_derived(prediction: &Prediction) -> Result<(), Box<dyn Error>> {
    let derived = [
        (CROSSBAR, CROSSBAR_FILL, CROSSBAR_FOOTPRINT),
        (UPRIGHT, UPRIGHT_FILL, UPRIGHT_FOOTPRINT),
        (INDICATOR, INDICATOR_FILL, INDICATOR_FOOTPRINT),
    ];
    for (name, fill, footprint) in derived {
        let placed = prediction.element(name).ok_or_else(|| {
            format!("`{name}` has to be declared for anything below to be about it")
        })?;
        require(
            (placed.fill, placed.footprint) == (Some(fill), Some(footprint)),
            format!(
                "the prediction has to place `{name}` where this file's header derives it by hand, \
                 or every claim below is stated over a rectangle nothing checked: it predicts \
                 {placed:?} against the derived fill {fill:?} and footprint {footprint:?}"
            ),
        )?;
    }
    require(
        prediction.covered_pixels() == PREDICTED_PIXELS,
        format!(
            "the three footprints have to cover the {PREDICTED_PIXELS} pixels the header derives, \
             or the count of pixels outside them is a count of something else: {covered}",
            covered = prediction.covered_pixels()
        ),
    )
}

/// Fails unless the shipped HUD drew something inside its own predicted
/// footprints.
///
/// # Errors
///
/// Returns a failure carrying the comparison when it did not.
fn require_something_was_drawn(inside: &Comparison) -> Result<(), Box<dyn Error>> {
    require(
        inside.considered == PREDICTED_PIXELS && inside.different > 0,
        format!(
            "the shipped declarations have to change the frame somewhere inside their own \
             footprints, or 'nothing outside them moved' is a claim about two frames neither of \
             which has a HUD: {inside:?}"
        ),
    )
}

/// Fails unless every pixel of the two bars' fills reads as the outline colour
/// their own declaration states.
///
/// This is what makes FR-4.1-S4 a control rather than a second copy of the
/// empty-frame one: the bars really are drawn in that frame, and drawn in a
/// colour the declaration itself names.
///
/// # Errors
///
/// Returns a failure naming the first fill that does not.
fn require_filled_with_the_outline_colour(
    frame: &Rgba8Image,
    prediction: &Prediction,
) -> Result<(), Box<dyn Error>> {
    for (name, fill) in [(CROSSBAR, CROSSBAR_FILL), (UPRIGHT, UPRIGHT_FILL)] {
        let outlined = outline_colour(prediction, name)?;
        let seen = swatch_reading(frame, fill, &[outlined])?;
        require(
            (seen.considered, seen.strayed) == (fill.area(), 0),
            format!(
                "`{name}`'s fill has to be drawn in the {outlined:?} it declares as its outline \
                 for this control to be about a wrong colour rather than about nothing being \
                 drawn: {seen:?}"
            ),
        )?;
    }
    Ok(())
}

/// The opaque colour `name`'s declaration states as its outline.
///
/// # Errors
///
/// Returns a failure when the element declares no outline, or one whose composite
/// a declaration cannot state on its own.
fn outline_colour(prediction: &Prediction, name: &str) -> Result<[u8; 3], Box<dyn Error>> {
    match prediction.element(name).and_then(|element| element.outline) {
        Some(PredictedPaint::Opaque(colour)) => Ok(colour),
        other => Err(format!(
            "`{name}` has to declare an opaque outline colour for a fill drawn in it to be a \
             stated colour at all: {other:?}"
        )
        .into()),
    }
}

/// Fails unless nothing already inside the indicator's footprint could be
/// mistaken for something the indicator draws.
///
/// The assertion this guards says **every** pixel of that footprint moves, and
/// that is only derivable when no pixel of the world behind it already reads as
/// the ring's colour or as one of the two colours the swatch is made of. Measured
/// from the declaration and from the texture generator, never hoped for: an
/// over-tight assertion is red against a correct renderer, and the cheapest way
/// to green one is to break the renderer.
///
/// # Errors
///
/// Returns a failure carrying the reading when something does.
fn require_nothing_already_reads_as_the_indicator(
    shot: &Shot,
    prediction: &Prediction,
) -> Result<(), Box<dyn Error>> {
    let mut drawn = shot.drawn.clone();
    drawn.push(outline_colour(prediction, INDICATOR)?);
    let seen: SwatchReading = swatch_reading(&shot.bare, INDICATOR_FOOTPRINT, &drawn)?;
    require(
        (seen.considered, seen.strayed) == (INDICATOR_FOOTPRINT.area(), seen.considered),
        format!(
            "no pixel of the world behind the indicator may already read as one of the \
             {drawn:?} the indicator draws, or 'every pixel of the footprint moves' is red against \
             a correct renderer for a pixel that was already that colour: {seen:?}"
        ),
    )
}
