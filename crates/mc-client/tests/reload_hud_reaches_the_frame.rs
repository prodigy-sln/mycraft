//! The HUD an accepted reload published, composed over a real frame on a device
//! with no window.
//!
//! # This is the other half of one scenario, and neither half covers it alone
//!
//! That the widened element is in the layout the client publishes is asserted
//! where the value crosses the boundary, with no device
//! (`reload_publishes_content.rs`). That says nothing about whether anything ever
//! draws it. This says the reverse: the layout the reload published, handed to the
//! one frame call the windowed client makes, paints the wider bar. The residue
//! between them — `App` assigning the published layout to its own field — needs a
//! real window and is held by review, as `App`'s share of an edit already is.
//!
//! # The frame is judged against a prediction, never against a golden
//!
//! The harness's default area budget forgives 92 wrong pixels at the declared
//! capture size and the widened crossbar adds 36, so a golden comparison could
//! accept a frame that drew the shipped bar instead. The reading here is per pixel
//! with no budget, against a prediction that re-derives the placement rule from
//! the declarations and shares no code with the composition it grades.
//!
//! # The shipped frame is the control, in the same run
//!
//! A prediction of the widened bar is satisfied by nothing the shipped bar draws:
//! six columns either side of it are world where the wider bar is white. So the
//! same prediction is read against a frame composed from the shipped layout and
//! has to **stray** — which is what stops the first reading passing over a
//! prediction that covered no pixel, or over two frames that were never going to
//! differ. A fixture guard ahead of both compares the two footprints, so a widened
//! declaration that landed on the same pixels as the shipped one fails there
//! rather than passing here.

#[path = "support/input/mod.rs"]
mod input;
#[path = "support/reload.rs"]
mod reload;
#[path = "support/reload_content.rs"]
mod reload_content;
#[path = "support/reload_world.rs"]
mod reload_world;
mod support;

use std::error::Error;
use std::sync::Arc;

use mc_render::surface::SurfaceSize;
use mc_sim::simulation::PublishedContent;
use mc_testkit::frame::Rgba8Image;
use mc_testkit::frame::gpu::CaptureContext;

use input::InputHarness;
use reload::{DIRT, STONE, accepted, adoption};
use reload_content::{CROSSBAR, candidate_against, shipped_widening_the_crossbar};
use reload_world::{floor_of, playing, standing};
use support::hud_frames::{HudCapture, hud_of, hud_published};
use support::prediction::{PixelVerdict, Prediction, per_pixel_reading};
use support::{TestResult, content_root, frames};

/// The tick every frame here is drawn at.
const TICK: u32 = 0;

/// The size every capture is taken at, and the target both predictions are made
/// for.
const TARGET: SurfaceSize = frames::CAPTURE_SIZE;

/// The one element this scenario's edit widens.
const WIDENED: [&str; 1] = [CROSSBAR];

#[test]
fn the_layout_a_reload_published_paints_the_widened_crossbar_on_a_drawn_frame() -> TestResult {
    let Some(context) = frames::device()? else {
        return Ok(());
    };
    let widened = shipped_widening_the_crossbar()?;
    require_a_wider_footprint(&widened)?;
    let mut client = a_client_over(STONE)?;
    let candidate = candidate_against(&widened, client.content())?;
    let verdict = adoption(client.adopt(candidate));

    let predicted = Prediction::of(widened.path(), TARGET)?;
    let (published, as_shipped) = one_scene_two_huds(&context, &client)?;

    assert_eq!(
        (
            verdict,
            shown(&published, &predicted)?,
            shown(&as_shipped, &predicted)?
        ),
        (
            accepted(DIRT),
            PixelVerdict::EveryPredictedPixelShowsIt,
            PixelVerdict::Strayed
        ),
        "the author widened one HUD declaration and saved. The layout the reload published is what \
         the client's own frame call was handed, and every pixel that declaration predicts has to \
         carry the colour it declares — a published layout nothing composes leaves the author's edit \
         invisible with the boundary assertion next door still green. The third element is the \
         control in the same run: the shipped bar is six columns narrower either side, so a frame \
         composed from it cannot satisfy this prediction, and a reading that accepted both would be \
         accepting a prediction that covers nothing"
    );
    Ok(())
}

/// Two frames of one scene differing in nothing but their HUD: the layout `client`
/// is publishing, and the one the shipped declarations compose.
///
/// **One `HudCapture` for both**, so the terrain behind them, the camera and the
/// uploaded array texture are the same in each and the only thing a comparison can
/// be reporting is the HUD. The published layout is asked of the client rather than
/// handed in, because a frame drawn from a second read of the widened root would
/// agree with this fixture by construction while the publication carried nothing.
///
/// # Errors
///
/// Returns the pipeline, upload, recording or capture failure, or the absence of
/// any published content to draw a HUD from.
fn one_scene_two_huds(
    context: &CaptureContext,
    client: &InputHarness,
) -> Result<(Rgba8Image, Rgba8Image), Box<dyn Error>> {
    let serving = serving(client)?;
    let mut frames_of = HudCapture::ready(context, TICK)?;
    let published = frames_of.capture(
        &hud_published(Arc::clone(&serving.hud)),
        &frames::request(context, "reload-hud-published")?,
    )?;
    let as_shipped = frames_of.capture(
        &hud_of(&content_root()?)?,
        &frames::request(context, "reload-hud-shipped")?,
    )?;
    Ok((published, as_shipped))
}

/// What `frame` shows at every pixel `predicted` covers for the widened element.
///
/// # Errors
///
/// Returns the distance metric's own failure.
fn shown(frame: &Rgba8Image, predicted: &Prediction) -> Result<PixelVerdict, Box<dyn Error>> {
    Ok(per_pixel_reading(frame, predicted, &WIDENED)?.verdict)
}

/// Refuses unless the widened declaration covers more of the frame than the
/// shipped one does.
///
/// A constraint no assertion in the scenario can enforce: an edit landing on
/// exactly the pixels the shipped declaration already covered would make the
/// control above unable to stray, and the reading would pass over two frames that
/// were never going to differ.
///
/// # Errors
///
/// Returns an error unless the widened footprint is strictly larger.
fn require_a_wider_footprint(
    widened: &support::content::ContentRoot,
) -> Result<(), Box<dyn Error>> {
    let after = footprint_of(&Prediction::of(widened.path(), TARGET)?)?;
    let before = footprint_of(&Prediction::of(&content_root()?, TARGET)?)?;
    if after > before {
        return Ok(());
    }
    Err(format!(
        "this scenario needs the widened declaration to cover more of the frame than the shipped \
         one, and it covers {after} pixels against {before}. The control below could not stray, so \
         the reading would pass over two frames that were never going to differ"
    )
    .into())
}

/// How many pixels the crossbar's footprint covers in `predicted`.
///
/// # Errors
///
/// Returns an error where the prediction states no such element, or none of it
/// lands on the target — a prediction of nothing is not one a comparison could be
/// made against.
fn footprint_of(predicted: &Prediction) -> Result<u64, Box<dyn Error>> {
    predicted
        .element(CROSSBAR)
        .and_then(|element| element.footprint)
        .map(|rect| rect.area())
        .ok_or_else(|| format!("nothing predicts a footprint for `{CROSSBAR}`").into())
}

/// A client playing a one-column floor of `floor`, with the shipped content root
/// serving.
fn a_client_over(floor: &'static str) -> Result<InputHarness, Box<dyn Error>> {
    let (simulation, holding) = playing(&content_root()?, standing(), |registry| {
        floor_of(registry, floor)
    })?;
    let mut client = InputHarness::started();
    client.play(simulation, holding);
    Ok(client)
}

/// The content `client` is publishing.
///
/// # Errors
///
/// Returns an error where the client publishes none, which is a client with no
/// world rather than one whose HUD a frame could be composed from.
fn serving(client: &InputHarness) -> Result<Arc<PublishedContent>, Box<dyn Error>> {
    client.content().ok_or_else(|| {
        "this fixture's client publishes no content, so it has no HUD to draw".into()
    })
}
