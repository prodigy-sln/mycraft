//! The shipped game draws its sea, judged at every frame the golden set is shot
//! from.
//!
//! # What this is evidence for that no golden can be
//!
//! A golden says a frame is the frame that was minted. It cannot say the frame
//! ever held water, because the day the sea failed to mesh the golden would be
//! re-minted without it and go on passing forever. So this reading takes the
//! frame from a device drawing the shipped world, chooses its pixels by marching
//! the world's own voxels, and judges the colour against what fills the array
//! layer `base:water` draws from — three things that come from three places.
//!
//! # Every declared capture, each on its own
//!
//! The requirement is per frame and not across the set: a frame predicting no
//! water witnesses nothing, so a set in which one of three frames sees the sea
//! would satisfy an aggregate reading while two thirds of the evidence was
//! absent. Each declared tick therefore has to predict the sea at one sample at
//! least, and to draw it at every sample where it predicted it.
//!
//! **No budget for a sample that lands on a silhouette**, unlike the one-sided
//! comparison in `replay_oracle.rs`. The requirement says every predicted pixel,
//! and the remedy for a sample that straddles the sea's edge is to move that
//! sample and record why — which is what the declared grid's own doc comment
//! requires of it, and it is a change a reader can see. A budget is a change
//! nobody can see.
//!
//! # Where the tolerance comes from, in both directions
//!
//! Measured, and **re-measured on 2026-08-26 when `base:water` was baked**. Both
//! halves of the bracket moved that day and the tolerance did not have to: the
//! layer this reads is now the shipped image rather than the generated stand-in,
//! and its 256 texels sit within **ΔE 3.16** of the layer's linear-light mean —
//! one base tone at 87.9% and two accents eight bytes either side of it — so a
//! magnified face shows a colour at most that far off and a minified one
//! converges on the mean.
//!
//! The nearest *wrong* answer is `base:stone` at **ΔE 25.34**, then the sky at
//! 31.97, `base:dirt` at 51.30, the four grass sides at 51.54 to 51.87 and
//! `base:grass_top` at 71.85. So the tolerance sits anywhere in
//! (3.16 + the sRGB round trip, 25.34), and it is **8** — over twice the texel
//! spread and 17 ΔE clear of every other thing one of these pixels could be
//! showing. Not loosened until green; the guard below asserts the lower half of
//! that bracket rather than leaving it to this comment.
//!
//! **The upper half narrowed from ΔE 62.40 to 25.34 and that is the art
//! arriving, not the reading weakening.** The stand-in was magenta, which is
//! implausible on purpose and therefore far from everything; a blue that belongs
//! in the same palette as the ground it meets stands nearer to it. 8 was inside
//! both brackets, which is why it still stands — a fact worth stating, because
//! the alternative reading is that nobody checked.
//!
//! **What this reading cannot tell you.** It cannot tell one water texel from
//! another, and it would not notice the layer being reflected or shuffled. What
//! it tells apart is water's layer from every other thing these pixels could
//! hold — the sky above the sea, the grass at its shore, the dirt and stone under
//! it.

mod support;

use std::error::Error;
use std::sync::Arc;

use mc_core::id::{BlockName, TextureKey};
use mc_render::camera::camera_view;
use mc_render::color::CLEAR_COLOR_SRGB;
use mc_render::geometry::scene::SceneGeometry;
use mc_render::gpu::TerrainRenderer;
use mc_sim::camera::CameraPose;
use mc_testkit::frame::Rgba8Image;
use mc_testkit::frame::gpu::CaptureContext;

use support::art::{drawn_texels, linear_mean};
use support::frames::{CAPTURE_SIZE, ReplayFrame};
use support::goldens::DECLARED_TICKS;
use support::oracle::{Sighted, Voxels, sighted_samples};
use support::probe::{distance, pixel_color};
use support::swatch::require;
use support::{PreparedScene, TestResult, prepare_scene};

/// The block whose surface this is about, and the key its faces draw.
///
/// One key across all six facings, so a face of the sea seen from any direction
/// draws the same layer and the reading needs no facing of its own.
const WATER: &str = "base:water";

/// How far a pixel may sit from the water layer's own mean, in ΔE.
///
/// Derived from both directions in this module's header: over twice the ΔE 3.16
/// that is the furthest any texel of that layer stands from its mean, and well
/// below the ΔE 25.34 that separates it from `base:stone`, the nearest wrong
/// answer one of these pixels could be showing.
///
/// **The ceiling is 25.34, not the 62.40 the magenta stand-in used to buy.**
/// Whoever widens this is eating into 17 ΔE of headroom, not 54: at 30 the guard
/// accepts a frame drawing stone where the sea should be and passes silently.
/// That narrowing is the whole subject of "a guard holding at ΔE 62 where it
/// needs ΔE 8" in `docs/technical/testing.md` — this is the constant it is about.
const SHOWS_THE_LAYER: f64 = 8.0;

/// Everything else one of these pixels could be showing, as a texture key, plus
/// the sky, which is a declared colour and not a layer at all.
///
/// **The guard's needles, and it is the guard that keeps the tolerance honest.**
/// A tolerance justified only in prose goes on being quoted after the colours
/// move; this list is compared against the water layer's mean on every run, so a
/// palette change that brought two of them within the tolerance reports itself
/// here instead of turning this reading into one that cannot tell them apart.
const EVERYTHING_ELSE: [&str; 7] = [
    "base:dirt",
    "base:stone",
    "base:grass_top",
    "base:grass_side_north",
    "base:grass_side_south",
    "base:grass_side_east",
    "base:grass_side_west",
];

#[test]
fn every_declared_capture_draws_the_sea_wherever_a_marched_ray_says_the_sea_is() -> TestResult {
    let prepared = prepare_scene()?;
    let key = TextureKey::parse(WATER)?;
    let mean = linear_mean(&drawn_texels(&key, &prepared.texels));
    require_nothing_else_is_that_colour(mean, &prepared)?;

    let Some(shown) = the_sea_in_each_capture(prepared, mean)? else {
        return Ok(());
    };

    assert_eq!(
        shown,
        DECLARED_TICKS
            .iter()
            .map(|tick| Shown {
                tick: *tick,
                predicted: AT_LEAST_ONE,
                drawn_as_something_else: Vec::new(),
            })
            .collect::<Vec<_>>(),
        "a player walking the shipped replay has to be able to see the sea, and these are the \
         three frames the committed goldens are shot from. Each of them has to predict water \
         somewhere — a frame that predicts none witnesses nothing, however many the others \
         predict — and every pixel it predicted water at has to be drawn in water's own layer, \
         within ΔE {SHOWS_THE_LAYER} of that layer's mean {mean:?}. A tick predicting none is a \
         camera that cannot see the sea; a tick with pixels listed is a sea the mesher or the \
         draw path lost between the world and the screen"
    );
    Ok(())
}

/// What one declared capture came to.
///
/// The predicted count is reported as "at least one" rather than as itself,
/// because the number is a property of where the camera happens to stand and
/// committing it would make a moved spawn a failure of this reading rather than
/// of the thing that moved. What must not vary is that it is not zero.
#[derive(Debug, PartialEq, Eq)]
struct Shown {
    tick: u16,
    predicted: &'static str,
    /// Every pixel the march called water that the frame drew as something else,
    /// with what it drew, so a failure names the pixels rather than counting
    /// them.
    drawn_as_something_else: Vec<String>,
}

/// What a capture that predicted the sea at one sample or more reports.
const AT_LEAST_ONE: &str = "the sea at one sample or more";

/// What a capture that predicted the sea nowhere reports.
const NONE_AT_ALL: &str = "the sea at no sample at all";

/// Each declared capture drawn from the player's own camera and judged against
/// `mean`, or `None` when the opt-in permitted the absence of a device.
fn the_sea_in_each_capture(
    prepared: PreparedScene,
    mean: [u8; 3],
) -> Result<Option<Vec<Shown>>, Box<dyn Error>> {
    let Some(context) = support::frames::device()? else {
        return Ok(None);
    };
    let mut renderer = support::frames::prepared_renderer(&context, &prepared)?;
    let voxels = Voxels {
        world: &prepared.world,
        registry: prepared.registry.as_ref(),
    };
    let water = Sighted::Terrain(BlockName::parse(WATER)?);
    let scene = Arc::new(prepared.scene);

    let mut shown = Vec::new();
    for tick in DECLARED_TICKS {
        let camera =
            support::frames::player_pose(u32::from(tick), &prepared.world, &prepared.registry)?;
        let predicted = predicted_sea(&camera, &voxels, &water)?;
        let frame = drawn(&context, &mut renderer, &scene, (tick, camera))?;
        shown.push(Shown {
            tick,
            predicted: if predicted.is_empty() {
                NONE_AT_ALL
            } else {
                AT_LEAST_ONE
            },
            drawn_as_something_else: off_colour(&frame, &predicted, mean)?,
        });
    }
    Ok(Some(shown))
}

/// The declared samples a march from `camera` says the sea is at.
///
/// **Chosen from the world and never from the picture**, which is what lets the
/// colour assertion be about the frame: a set of pixels found by looking for
/// water-coloured pixels would be a frame certifying itself.
fn predicted_sea(
    camera: &CameraPose,
    voxels: &Voxels<'_>,
    water: &Sighted,
) -> Result<Vec<(u32, u32)>, Box<dyn Error>> {
    Ok(sighted_samples(camera, CAPTURE_SIZE, voxels)?
        .into_iter()
        .filter(|(_, sighted)| sighted == water)
        .map(|(pixel, _)| pixel)
        .collect())
}

/// Every one of `predicted` that `frame` draws further than the tolerance from
/// `mean`, named with what it drew instead.
fn off_colour(
    frame: &Rgba8Image,
    predicted: &[(u32, u32)],
    mean: [u8; 3],
) -> Result<Vec<String>, Box<dyn Error>> {
    let mut off = Vec::new();
    for pixel in predicted {
        let colour = pixel_color(frame, *pixel)?;
        let stands = distance(colour, mean)?;
        if stands > SHOWS_THE_LAYER {
            off.push(format!("{pixel:?} drew {colour:?}, ΔE {stands:.2} away"));
        }
    }
    Ok(off)
}

/// Fails unless the water layer's mean stands further than the tolerance from
/// the sky and from every other layer one of these pixels could hold.
///
/// **The lower half of the tolerance's bracket, asserted rather than quoted.**
/// The upper half — that a water pixel sits within the tolerance of this mean —
/// is what the reading itself measures; this is the half that says the reading
/// can tell water from anything else at all. Without it the tolerance could be
/// widened until the assertion passed over a frame drawing stone.
fn require_nothing_else_is_that_colour(
    mean: [u8; 3],
    prepared: &PreparedScene,
) -> Result<(), Box<dyn Error>> {
    let mut too_close = Vec::new();
    let sky = distance(CLEAR_COLOR_SRGB, mean)?;
    if sky <= SHOWS_THE_LAYER {
        too_close.push(format!("the sky at ΔE {sky:.2}"));
    }
    for other in EVERYTHING_ELSE {
        let key = TextureKey::parse(other)?;
        let apart = distance(linear_mean(&drawn_texels(&key, &prepared.texels)), mean)?;
        if apart <= SHOWS_THE_LAYER {
            too_close.push(format!("`{other}` at ΔE {apart:.2}"));
        }
    }
    require(
        too_close.is_empty(),
        format!(
            "this reading calls a pixel water when it sits within ΔE {SHOWS_THE_LAYER} of the \
             water layer's mean {mean:?}, which is only a reading at all while nothing else one \
             of these pixels could hold sits that close to it. These do: {too_close:?}"
        ),
    )
}

/// The frame `shot`'s camera draws at `shot`'s tick, at the declared capture
/// size.
fn drawn(
    context: &CaptureContext,
    renderer: &mut TerrainRenderer,
    scene: &Arc<SceneGeometry>,
    shot: (u16, CameraPose),
) -> Result<Rgba8Image, Box<dyn Error>> {
    let (tick, camera) = shot;
    let view = camera_view(camera.eye, camera.target);
    let snapshot = support::frames::snapshot(u32::from(tick), view, scene);
    let request = support::frames::request(context, &format!("player-sea-t{tick:03}"))?;
    let mut frame = ReplayFrame {
        context,
        renderer,
        snapshot: &snapshot,
    };
    frame.capture(&request)
}
