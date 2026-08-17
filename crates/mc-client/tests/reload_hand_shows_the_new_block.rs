//! The held-block indicator, once a reload has put a newly declared block in the
//! player's hand: drawn from the layer that reload appended.
//!
//! # A device, because the indicator is a picture
//!
//! Which block a client holds after a reload is already graded with no device. What
//! is not, and cannot be, is that the square at the bottom of the screen shows *that
//! block's* texture: the swatch is composed by the renderer from the layers it was
//! uploaded, so a layer that was appended to the assignment and never filled draws
//! whatever was in the array texture before — a picture that is wrong in an
//! entirely plausible way.
//!
//! # The renderer is handed the layers the report handed over, through the one route
//! there is
//!
//! `HudCapture` opens with the layers a launch produced, which is exactly the state
//! a client is in before a reload. The report hands its own over **wrapped**, and
//! `Unuploaded::uploaded_to` is the only way to an owned `TextureLayers` — so this
//! scenario drives the frame path's own upload call rather than making one of its
//! own, and the frame is then composed through the client's own frame call rather
//! than through a second path built to resemble it.
//!
//! That matters beyond tidiness: the wrapper exists because deleting the upload
//! outright left 234 of 234 `mc-client` tests green, and if no test drove
//! `uploaded_to` the route it guards would be reachable from nothing in this
//! workspace.
//!
//! # Where the swatch lands is predicted, not written down
//!
//! The rectangle comes from an independent prediction over the content root's own
//! declarations, so a declaration an author moves or resizes moves the reading with
//! it instead of leaving a stale literal that reads pixels of the world.

#[path = "support/input/mod.rs"]
mod input;
#[path = "support/reload.rs"]
mod reload;
#[path = "support/reload_content.rs"]
mod reload_content;
#[path = "support/reload_remesh.rs"]
mod reload_remesh;
#[path = "support/reload_upload.rs"]
mod reload_upload;
#[path = "support/reload_watch.rs"]
mod reload_watch;
#[path = "support/reload_world.rs"]
mod reload_world;
mod support;

use std::error::Error;
use std::path::Path;

use mc_core::id::{BlockName, TextureKey};
use mc_render::hud::{HudFrame, held_swatch};
use mc_render::texture::TextureLayers;
use mc_testkit::frame::Rgba8Image;
use mc_testkit::frame::gpu::CaptureContext;

use reload::{AMBER, AMBER_FILE, STONE, amber, shipped};
use reload_content::THE_NEXT_UNUSED_LAYER;
use reload_upload::{declaring_after_launch, layers_handed_over, until_taken_up};
use reload_watch::a_client_on;
use support::content::ContentRoot;
use support::hud_frames::{HudCapture, Rect, hud_of};
use support::prediction::Prediction;
use support::swatch::{TEXEL_COLORS, swatch_reading, texel_colors};
use support::{TestResult, frames};

/// The tick the frame is drawn at. Nothing about the indicator depends on it.
const A_TICK: u32 = 0;

/// The name the base game's held-block indicator registers under.
const THE_INDICATOR: &str = "base:held-block";

#[test]
fn the_held_block_indicator_draws_the_new_block_from_the_layer_the_reload_appended() -> TestResult {
    let Some(context) = frames::device()? else {
        return Ok(());
    };
    let root = shipped()?;
    let indicator = the_indicators_fill(root.path())?;
    let Shown {
        held,
        layers,
        frame,
    } = a_run_that_declares_the_new_block(&context, &root)?;

    let seen = swatch_reading(&frame, indicator, &texel_colors(&BlockName::parse(AMBER)?)?)?;
    let holds = held.as_ref().map(BlockName::as_str);
    let appended = layers.layer_of(&TextureKey::parse(AMBER)?);
    let read = (holds, appended, seen.strayed, seen.shown, seen.considered);
    let owed = (
        Some(AMBER),
        Some(THE_NEXT_UNUSED_LAYER),
        0,
        TEXEL_COLORS,
        indicator.area(),
    );
    assert_eq!(
        read, owed,
        "the block the reload put in the hand is the block the indicator draws, and it draws it from \
         the layer that reload appended: every pixel of the swatch is one of the two colours that \
         block's placeholder layer is made of, and both of them are there. A flat square shows one \
         colour, another block's texture shows neither, and an appended layer nothing filled shows \
         whatever the array texture held before the reload"
    );
    Ok(())
}

/// What one run produced: what the client holds, the layers its report handed over,
/// and the frame drawn with both.
struct Shown {
    held: Option<BlockName>,
    layers: TextureLayers,
    frame: Rgba8Image,
}

/// A run that launches over a stone floor, has an author declare a block, and draws
/// one frame with the layers the reload's report handed over.
///
/// **The renderer's textures are replaced with those layers and nothing else is**,
/// which is the one thing the frame path does with a reload's textures. The block
/// the swatch shows and the layer it draws from are the client's own two answers,
/// carried rather than decided here.
///
/// # Errors
///
/// Returns the read, write, pipeline, upload or capture failure, and the refusal
/// where no candidate was taken up.
fn a_run_that_declares_the_new_block(
    context: &CaptureContext,
    root: &ContentRoot,
) -> Result<Shown, Box<dyn Error>> {
    let (mut client, reports) = a_client_on(root, STONE)?;
    let declared = declaring_after_launch(root, AMBER_FILE, &amber())?;
    reports.changed(&[declared])?;
    let unuploaded = layers_handed_over(until_taken_up(&mut client))?;

    let mut frames_of = HudCapture::ready(context, A_TICK)?;
    // The product's own route, and the only one there is: an owned `TextureLayers`
    // can be had no other way, so a frame path that forgot the upload could not
    // reach the line below either.
    let layers = unuploaded.uploaded_to(&mut frames_of.renderer, context.queue())?;
    let held = client.held_block();
    let showing = held_swatch(held.as_ref(), &layers).texture();
    let request = frames::request(context, "reload-held-block-appended-layer")?;
    let frame = frames_of.capture(&holding(root.path(), showing)?, &request)?;
    Ok(Shown {
        held,
        layers,
        frame,
    })
}

/// The HUD the declarations under `root` compose, holding `showing`.
///
/// The layout is read from the root and the swatch is the client's own answer
/// carried through — the two `App::draw` composes its own frame from.
fn holding(root: &Path, showing: Option<TextureKey>) -> Result<HudFrame, Box<dyn Error>> {
    Ok(HudFrame {
        held: showing,
        ..hud_of(root)?
    })
}

/// Where the indicator's fill lands on a frame of the declared capture size, from
/// an independent prediction over the root's own declarations.
///
/// # Errors
///
/// Returns an error if the root declares no such element, or if none of it falls on
/// the frame — either of which would leave the reading below over a rectangle of
/// the world rather than of the swatch.
fn the_indicators_fill(root: &Path) -> Result<Rect, Box<dyn Error>> {
    Prediction::of(root, frames::CAPTURE_SIZE)?
        .element(THE_INDICATOR)
        .and_then(|element| element.fill)
        .ok_or_else(|| {
            format!(
                "this scenario reads the pixels of `{THE_INDICATOR}`'s fill, so the content root has \
                 to declare it and it has to land on the frame"
            )
            .into()
        })
}
