//! The tint reaches the world and stops at the overlay.
//!
//! # What this catches, and it is an ordering mistake rather than a colour one
//!
//! The HUD composites over the terrain frame in a later pass, so anything that
//! tints has to act **inside** the terrain pass. A mechanism that acted after it
//! — a full-screen wash over the composited image, a clear applied at the wrong
//! point — draws a picture in which the crosshair is under water too, and the
//! only thing that reports it is a reading of the HUD's own pixels while the eye
//! is submerged. The crosshair's fill is white and its outline black; a medium
//! carrying either toward a declared colour is the defect.
//!
//! # The reading is an enumerated verdict, not an absence
//!
//! [`PixelVerdict::EveryPredictedPixelShowsIt`] rejects every other answer
//! *including* the ones that mean "I could not look" — a prediction covering no
//! pixel, a predicted pixel outside the frame, a colour that was not
//! predictable. An `is_empty` over a list of strays could not tell any of those
//! from a clean frame.
//!
//! # A hundred pixels needs one more element than the shipped HUD has
//!
//! The scenario asks for at least a hundred HUD pixels examined and the shipped
//! crosshair covers **57** — `hud_prediction.rs` states that figure and derives
//! it. The held-block indicator would supply the rest and cannot be judged: its
//! swatch draws a block's own art, so its pixels carry no predictable colour and
//! a reading over them answers `APredictedColourWasNotPredictable` rather than
//! anything about a tint.
//!
//! So the fixture root declares **one more element of its own**, a flat panel in
//! a colour no layer of this scene shows, and the reading covers it beside both
//! shipped crosshair bars. What is under test is whether *a declared HUD pixel*
//! keeps its declared colour while the eye is under water, and a fixture element
//! is as much a declared HUD pixel as a shipped one — while the two shipped bars
//! stay in the reading, so the scenario's own `#FFFFFF` fill and `#000000`
//! outline are covered by the same verdict.
//!
//! # And it needs a control, because the dry frame satisfies it
//!
//! A build that never writes the tint into any frame draws a HUD at its declared
//! colours, over terrain at its own colours, and passes the paragraph above
//! without the eye's medium having reached anything. So the verdict also states
//! that the **terrain** moved when the sea declared a tint — the same pose and
//! the same HUD over two roots, one declaring a tint in the eye's own cell and
//! one declaring none.

mod support;

use std::error::Error;

use mc_core::block::MediumTint;

use support::TestResult;
use support::content::{ContentRoot, SEA_FILE, shipped_copy};
use support::frames;
use support::hud_frames::{HudCapture, hud_of};
use support::medium::{REACHES_AT, TINT, tinting};
use support::prediction::{PixelVerdict, Prediction, per_pixel_reading};
use support::submerged::{EYE, LOOK_AT, differing};

/// How many HUD pixels a reading about the HUD needs before it means anything.
const MANY: u64 = 100;

/// What the composited frame came to.
#[derive(Debug, PartialEq)]
struct Composited {
    /// What the resolver answered for the eye the frame was drawn from.
    the_medium_the_eye_stands_in: Option<MediumTint>,
    /// Whether every pixel of every HUD element shows the colour its own
    /// declaration states.
    every_declared_hud_pixel: PixelVerdict,
    at_least_a_hundred_hud_pixels_were_examined: bool,
    /// Whether **any** pixel of the frame moved when the sea declared a tint.
    ///
    /// **Named for what it checks and not for what it is for.** It is a
    /// whole-frame comparison, so it would be true if only the HUD had moved —
    /// the terrain is what it is *about*, and the element above is what makes
    /// the conjunction sound, a tinted HUD failing that one in the same verdict.
    /// A name saying more than the assertion checks is a small lie a later
    /// reader relies on.
    the_frame_moved_when_the_sea_declared_one: bool,
}

#[test]
fn a_hud_over_a_submerged_frame_is_drawn_at_the_colours_its_declarations_state() -> TestResult {
    let Some(composited) = what_the_composited_frame_draws()? else {
        return Ok(());
    };
    assert_eq!(
        composited,
        Composited {
            the_medium_the_eye_stands_in: tinting(TINT),
            every_declared_hud_pixel: PixelVerdict::EveryPredictedPixelShowsIt,
            at_least_a_hundred_hud_pixels_were_examined: true,
            the_frame_moved_when_the_sea_declared_one: true,
        },
        "the eye stands inside a cell whose block declares a tint reaching its full strength at \
         {REACHES_AT} blocks, and the HUD composites over the terrain frame in a later pass. So \
         the crosshair's fill is still drawn at the white its declaration states and its outline \
         at the black — the medium reaches the world and stops at the overlay. The last element \
         is the control: without it a renderer that tinted nothing at all would satisfy every \
         word above"
    );
    Ok(())
}

/// What the declared submerged pose draws with the shipped HUD over it, over a
/// sea that declares a tint and over one that declares none.
///
/// `None` where the opt-in permitted the absence of a device.
fn what_the_composited_frame_draws() -> Result<Option<Composited>, Box<dyn Error>> {
    let Some(context) = frames::device()? else {
        return Ok(None);
    };
    let tinted = a_sea_that_tints()?;
    let plain = a_sea_declaring_nothing()?;
    let prediction = Prediction::of(tinted.path(), frames::CAPTURE_SIZE)?;
    let named: Vec<&str> = [CROSSBAR, UPRIGHT, PANEL].to_vec();

    let mut under = HudCapture::from_a_declared_eye(&context, tinted.path(), (EYE, LOOK_AT))?;
    let over_the_tint = under.capture(
        &hud_of(tinted.path())?,
        &frames::request(&context, "hud-tinted")?,
    )?;
    let mut without = HudCapture::from_a_declared_eye(&context, plain.path(), (EYE, LOOK_AT))?;
    let over_no_tint = without.capture(
        &hud_of(plain.path())?,
        &frames::request(&context, "hud-plain")?,
    )?;

    let reading = per_pixel_reading(&over_the_tint, &prediction, &named)?;
    Ok(Some(Composited {
        the_medium_the_eye_stands_in: under.published_tint(),
        every_declared_hud_pixel: reading.verdict,
        at_least_a_hundred_hud_pixels_were_examined: reading.considered >= MANY,
        the_frame_moved_when_the_sea_declared_one: differing(&over_the_tint, &over_no_tint) > 0,
    }))
}

/// The two shipped crosshair bars, and the panel this fixture adds beside them.
const CROSSBAR: &str = "base:crosshair-horizontal";
const UPRIGHT: &str = "base:crosshair-vertical";
const PANEL: &str = "example:panel";
const PANEL_FILE: &str = "example-panel.toml";

/// The panel's declaration.
///
/// **A colour no layer of this scene shows**, so a panel drawn at the terrain's
/// colour and a panel drawn at its own are two pictures a reading can tell
/// apart. Anchored away from the centre so it does not sit over the crosshair
/// the same verdict is about.
const PANEL_DECLARATION: &str = "name = \"example:panel\"
anchor = \"top-left\"
size = [24,                                  24]
draw = \"fill\"
color = \"#C81E78FF\"
outline =                                  \"#000000FF\"
";

/// A copy of the shipped root whose sea declares [`TINT`] at [`REACHES_AT`], and
/// one whose sea declares no tint at all — both carrying the fixture's panel, so
/// the two differ in the sea's declaration and in nothing else.
fn a_sea_that_tints() -> Result<ContentRoot, Box<dyn Error>> {
    with_the_panel(shipped_copy()?.whose_block_declares(SEA_FILE, Some((TINT, REACHES_AT)))?)
}
fn a_sea_declaring_nothing() -> Result<ContentRoot, Box<dyn Error>> {
    with_the_panel(shipped_copy()?.whose_block_declares(SEA_FILE, None)?)
}

/// That root with the fixture's own HUD element written into it.
fn with_the_panel(root: ContentRoot) -> Result<ContentRoot, Box<dyn Error>> {
    let declared = root.path().join(support::content::HUD_DIRECTORY);
    std::fs::create_dir_all(&declared)?;
    std::fs::write(declared.join(PANEL_FILE), PANEL_DECLARATION)?;
    Ok(root)
}
