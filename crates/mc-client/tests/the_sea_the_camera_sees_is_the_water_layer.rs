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
//! # What the sea's colour became, and why the tolerance did not move
//!
//! A sea that passes light no longer draws its own colour. It draws itself
//! composed over whatever the ray meets beyond it, and this reading is
//! re-derived against exactly that — **at the tolerance it always had**.
//! Measured over the three declared captures with the sea at half a degree: the
//! worst a sample stands from the nearest colour predicted for it is **ΔE
//! 1.29**, and the nearest a predicted composite stands from any colour one of
//! its own operands draws *unblended* is **ΔE 11.95**. The bracket is therefore
//! (1.29, 11.95) and the ΔE 8 below sits inside it with room at both ends.
//!
//! That is the whole point of re-deriving the expectation instead of widening
//! the constant. A tolerance widened until a blended sea passed would have gone
//! on accepting an unblended one: the two stand only **ΔE 16.26** apart at their
//! closest, so anything past 16.26 accepts both and anything between 8 and 16.26
//! is a number nobody derived.
//!
//! **What did move is the set a pixel is judged against**, from the layer's own
//! mean to every colour the layer shows at any distance
//! (`support::art::landmarks_at_every_scale`). A magnified face shows one texel
//! and a fully minified one shows the mean; one at middle distance shows a
//! reduced texel that is neither. That is what keeps the measured error at 1.29
//! rather than at 7.13.
//!
//! # The ceiling is not layer-against-layer, and the figure that used to stand
//! here answered the wrong question
//!
//! This file used to derive 8.0's upper end from *"the nearest wrong answer is
//! `base:stone` at ΔE 25.34"* — the distance between two **layers'** means. That
//! is the right question for a reading that judges a pixel against one layer,
//! and this one no longer does: it judges a pixel against a **composition**, and
//! what a composition can be mistaken for is one of the two things it was
//! composed from. Two failure modes, and the second is the precise defect this
//! whole spec exists to fix:
//!
//! - the sea failed to draw and the surface behind it shows through — the
//!   composition against the under-layer, unblended;
//! - the sea drew and never blended — the composition against water's own
//!   colour, unblended.
//!
//! **Over the crossings these captures actually hold the nearest of those is ΔE
//! 11.95**, and `base:stone` cannot take it lower because stone lies under the
//! sea nowhere in this world. Enumerated rather than argued: the terminals
//! behind the sea are `base:grass` at 303 of the declared samples and the sky at
//! 69, and over a sweep of **103 680** poses across the whole footprint and the
//! integer direction lattice they are `base:grass` (106 156) and the sky
//! (71 817), with `base:stone` at **zero**. What stands directly under a water
//! cell is `base:grass` at 131 of them and more sea at the other 47.
//!
//! The arithmetic over a composite family that *included* a stone lakebed would
//! put the ceiling at **ΔE 9.46**, and that figure is stated here because a
//! bound over what a world could hold and a measurement over what it does are
//! different claims — 8.0 clears either, and neither licenses widening it.
//! **The reading does not rest on the enumeration**:
//! `too_close_to_what_it_is_made_of` measures that separation per run over the
//! crossings that occur, so the day a stone lakebed appears it reports itself
//! rather than being covered by a sentence written today.
//!
//! # A composition of the sea sits ΔE 2.86 from a colour `base:stone` shows
//!
//! Enumerated over all 372 predicted-water samples of the three declared
//! captures and all 478 distinct colours their compositions may take: **1 111
//! pairs of those colours and `base:stone`'s stand within ΔE 8 of each other,
//! the nearest at 2.86** — `(123, 119, 123)` against `(121, 121, 121)`. Half of
//! a blue-grey sea over anything lands in the grey a stone texture is made of,
//! and no tolerance this reading could carry separates them.
//!
//! **It is a different question from the ceiling above and it does not move
//! it.** That one asks whether a composition can be mistaken for something it
//! was composed *from*; this asks whether it can be mistaken for a layer
//! entirely absent from the pixel, which is a layer-index or a mesh defect and
//! not a blend one. Stone stands behind the sea nowhere, so the grey such a
//! pixel would arrive by is a colour nothing there can draw.
//!
//! **The fourth list below is what stops that being an argument.** Every
//! predicted-water pixel has to stand further than the tolerance from what that
//! same pixel would show *with no sea at all*, so a picture with the sea missing
//! is reported however near some third layer's colours the composition happens
//! to sit. Measured margin: the nearest any of them comes is **ΔE 11.95**, at a
//! sample looking through the sea at the sky.
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
//! it — and, since the sea began passing light, a sea composed over the lakebed
//! from a sea drawn as though it stopped everything.

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
use support::composite::{Palette, nearest_between};
use support::frames::{CAPTURE_SIZE, ReplayFrame};
use support::goldens::DECLARED_TICKS;
use support::oracle::{Crossed, CrossedSample, Voxels, crossed_samples};
use support::probe::{distance, pixel_color};
use support::swatch::require;
use support::{PreparedScene, TestResult, prepare_scene};

/// The block whose surface this is about, and the key its faces draw.
///
/// One key across all six facings, so a face of the sea seen from any direction
/// draws the same layer and the reading needs no facing of its own.
const WATER: &str = "base:water";

/// How far a pixel may sit from the nearest colour the march predicts for it, in
/// ΔE.
///
/// Derived from both directions in this module's header: above the worst a
/// correct frame is measured to stand — ΔE 3.16 of texel spread while the sea
/// stopped all the light, ΔE 1.29 measured over the three captures now that it
/// does not — and below the ΔE 11.95 that separates a composed sea both from
/// the colours it was composed *from* and from what its own pixel would show
/// with no sea at all.
///
/// **The ceiling is 11.95, not the 25.34 a layer-against-layer reading buys and
/// not the 62.40 the magenta stand-in used to.** Whoever widens this is eating
/// into 3.95 ΔE of headroom, not 17 and not 54: past 11.95 the guard accepts a
/// frame in which the sea never drew at all. That narrowing is the whole subject
/// of "a guard holding at ΔE 62 where it needs ΔE 8" in
/// `docs/technical/testing.md` — this is the constant it is about, and the
/// headroom it has is smaller than that entry left a reader expecting.
///
/// **It did not move when the sea began to blend, and that was the point.** The
/// expectation was re-derived instead. A tolerance widened to fit a composite
/// would have gone on accepting the unblended sea it was supposed to start
/// refusing.
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

    let Some(shown) = the_sea_in_each_capture(prepared)? else {
        return Ok(());
    };

    assert_eq!(
        shown,
        every_capture_showing_the_sea(),
        "a player walking the shipped replay has to be able to see the sea, and these are the \
         three frames the committed goldens are shot from. Each of them has to predict water \
         somewhere — a frame that predicts none witnesses nothing, however many the others \
         predict — and every pixel it predicted water at has to be drawn as the sea composed over \
         whatever the ray meets beyond it, within ΔE {SHOWS_THE_LAYER}. The sea's own layer means \
         {mean:?}, and a pixel sitting *at* that colour where the sea passes light is a sea drawn \
         as though it did not — which is the failure this reading could not state while every \
         block was opaque. A tick predicting none is a camera that cannot see the sea; a tick with \
         pixels listed is a sea the mesher or the draw path lost between the world and the screen. \
         The third list is the reading's own premise rather than the frame's: a composition \
         standing within the tolerance of a colour one of its own operands draws unblended is one \
         this reading cannot tell from that operand"
    );
    Ok(())
}

/// What every declared capture has to come to: the sea predicted somewhere, and
/// nothing reported against it.
fn every_capture_showing_the_sea() -> Vec<Shown> {
    DECLARED_TICKS
        .iter()
        .map(|tick| Shown {
            tick: *tick,
            predicted: AT_LEAST_ONE,
            drawn_as_something_else: Vec::new(),
            drawn_as_though_the_sea_were_absent: Vec::new(),
            too_close_to_what_it_is_made_of: Vec::new(),
        })
        .collect()
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
    /// Every pixel the march called water that the frame drew at what that same
    /// pixel would show with **no sea at all**.
    ///
    /// **The direct falsifier for a picture with the sea missing**, and it is
    /// not the claim the list above makes. A composition may legitimately sit
    /// near some third layer's colours — measured, half a blue-grey sea over
    /// anything lands ΔE 2.86 from a grey `base:stone` shows — so "the drawn
    /// colour is one the composition may take" is a sentence a picture nobody
    /// wants could satisfy. This asks the other question, about the one
    /// alternative that could actually stand at these pixels.
    drawn_as_though_the_sea_were_absent: Vec<String>,
    /// Every crossing whose predicted colour stands too near a colour one of its
    /// own operands draws unblended — the half of the tolerance no reading of
    /// the frame can make for itself.
    too_close_to_what_it_is_made_of: Vec<String>,
}

/// What a capture that predicted the sea at one sample or more reports.
const AT_LEAST_ONE: &str = "the sea at one sample or more";

/// What a capture that predicted the sea nowhere reports.
const NONE_AT_ALL: &str = "the sea at no sample at all";

/// Each declared capture drawn from the player's own camera and judged against
/// what the march predicts for it, or `None` when the opt-in permitted the
/// absence of a device.
fn the_sea_in_each_capture(prepared: PreparedScene) -> Result<Option<Vec<Shown>>, Box<dyn Error>> {
    let Some(context) = support::frames::device()? else {
        return Ok(None);
    };
    let mut renderer = support::frames::prepared_renderer(&context, &prepared)?;
    let (voxels, palette) = (
        Voxels {
            world: &prepared.world,
            registry: prepared.registry.as_ref(),
        },
        Palette::of(&prepared.registry, &prepared.resolution, &prepared.texels),
    );
    let sea = BlockName::parse(WATER)?;
    let scene = Arc::new(prepared.scene.clone());

    let mut shown = Vec::new();
    for tick in DECLARED_TICKS {
        let camera =
            support::frames::player_pose(tick.into(), &prepared.world, &prepared.registry)?;
        let predicted = predicted_sea(&camera, &voxels, &sea)?;
        let frame = drawn(&context, &mut renderer, &scene, (tick, camera))?;
        shown.push(judged(tick, &frame, &predicted, &palette)?);
    }
    Ok(Some(shown))
}

/// What one capture's predicted samples came to, drawn and judged.
fn judged(
    tick: u16,
    frame: &Rgba8Image,
    predicted: &[CrossedSample],
    palette: &Palette<'_>,
) -> Result<Shown, Box<dyn Error>> {
    Ok(Shown {
        tick,
        predicted: if predicted.is_empty() {
            NONE_AT_ALL
        } else {
            AT_LEAST_ONE
        },
        drawn_as_something_else: off_colour(frame, predicted, palette)?,
        drawn_as_though_the_sea_were_absent: as_though_absent(frame, predicted, palette)?,
        too_close_to_what_it_is_made_of: indistinguishable(predicted, palette)?,
    })
}

/// The declared samples a march from `camera` says show the sea, each with
/// everything its ray met.
///
/// **Chosen from the world and never from the picture**, which is what lets the
/// colour assertion be about the frame: a set of pixels found by looking for
/// water-coloured pixels would be a frame certifying itself.
fn predicted_sea(
    camera: &CameraPose,
    voxels: &Voxels<'_>,
    sea: &BlockName,
) -> Result<Vec<CrossedSample>, Box<dyn Error>> {
    Ok(crossed_samples(camera, CAPTURE_SIZE, voxels)?
        .into_iter()
        .filter(|(_, crossed)| shows(crossed, sea))
        .collect())
}

/// Whether a ray that met `crossed` shows `sea` at all — as the surface it
/// stopped at, or as a run it passed through on the way.
///
/// **Both arms, because this spec moves the sea from one to the other.** A
/// reading looking only at the surface a ray stopped at would find the sea
/// nowhere the day it began passing light, report "the sea at no sample at all"
/// for all three captures, and be reporting the wrong thing.
fn shows(crossed: &Crossed, sea: &BlockName) -> bool {
    crossed
        .beyond
        .as_ref()
        .is_some_and(|surface| &surface.block == sea)
        || crossed.layers.iter().any(|layer| &layer.block == sea)
}

/// Every one of `predicted` that `frame` draws further than the tolerance from
/// the nearest colour its own crossing predicts, named with what it drew.
fn off_colour(
    frame: &Rgba8Image,
    predicted: &[CrossedSample],
    palette: &Palette<'_>,
) -> Result<Vec<String>, Box<dyn Error>> {
    let mut off = Vec::new();
    for (pixel, crossed) in predicted {
        let colour = pixel_color(frame, *pixel)?;
        let stands = palette.stands_from(crossed, colour)?;
        if stands > SHOWS_THE_LAYER {
            off.push(format!(
                "{pixel:?} looks at {} and drew {colour:?}, ΔE {stands:.2} from anything that can \
                 be, against a mean-over-mean of {:?}",
                crossed.sighted().described(),
                palette.predicted_mean(crossed)?
            ));
        }
    }
    Ok(off)
}

/// Every one of `predicted` that `frame` draws at what the same pixel would show
/// with no sea at all: the surface behind it unblended, or the declared clear
/// colour where nothing stands behind it.
fn as_though_absent(
    frame: &Rgba8Image,
    predicted: &[CrossedSample],
    palette: &Palette<'_>,
) -> Result<Vec<String>, Box<dyn Error>> {
    let mut absent = Vec::new();
    for (pixel, crossed) in predicted {
        let colour = pixel_color(frame, *pixel)?;
        let without = match &crossed.beyond {
            None => vec![CLEAR_COLOR_SRGB],
            Some(surface) => palette.landmarks_of(surface)?,
        };
        let stands = nearest_between(&[colour], &without)?;
        if stands <= SHOWS_THE_LAYER {
            absent.push(format!(
                "{pixel:?} looks through the sea at {} and drew {colour:?}, ΔE {stands:.2} from \
                 what that pixel shows with no sea in front of it at all",
                crossed.sighted().described()
            ));
        }
    }
    Ok(absent)
}

/// Every one of `predicted` whose composition stands within the tolerance of a
/// colour one of its own operands draws unblended.
///
/// **The half of the tolerance no measurement of the frame can make**, and the
/// half this spec added. A composition sitting on top of the sea's own colour,
/// or on top of the lakebed's, is one that a reading accepting either would pass
/// over — so an implementation that lost the blend, or lost what stands behind
/// it, would go unreported. While the sea stopped all the light there was no
/// composition to be confused with anything, which is why the old form of this
/// file had nothing to say here.
fn indistinguishable(
    predicted: &[CrossedSample],
    palette: &Palette<'_>,
) -> Result<Vec<String>, Box<dyn Error>> {
    let mut too_close = Vec::new();
    for (pixel, crossed) in predicted {
        let apart = palette.unblended_stands_from(crossed)?;
        if apart <= SHOWS_THE_LAYER {
            too_close.push(format!(
                "{pixel:?} looks at {} whose composition stands ΔE {apart:.2} from a colour one of \
                 its own layers draws unblended",
                crossed.sighted().described()
            ));
        }
    }
    Ok(too_close)
}

/// Fails unless the water layer's mean stands further than the tolerance from
/// the sky and from every other layer one of these pixels could hold.
///
/// **The lower half of the tolerance's bracket, asserted rather than quoted.**
/// The upper half — that a water pixel sits within the tolerance of what the
/// march predicts for it — is what the reading itself measures; this is the half
/// that says the reading can tell water from anything else at all. Without it
/// the tolerance could be widened until the assertion passed over a frame
/// drawing stone.
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
