//! What the replay's tick-60 frame has to look like, asserted without reference
//! to any committed image.
//!
//! This is the suite the whole spec leans on. A golden re-shot from a broken
//! renderer is a golden of a broken renderer and passes forever; only an
//! assertion that does not come from the renderer can catch that, and these are
//! those assertions. They land **in the same phase as the goldens, before the
//! goldens are minted**, because a reference shot from a renderer nobody
//! checked is worth nothing.
//!
//! Every figure here is derived in `architecture.md`'s screen-space budget,
//! which projected the declared camera before any of this was written: the
//! horizon at row 282, the landmark at pixel (478, 215) with its mirror at 801
//! landing on sky 323 px from an 8.3 px pillar, coverage near 21.9% against an
//! analytic floor of 18.8%, and per-colour shares near 18.1% / 3.0% / 0.7%.
//! **If a measured figure lands under its declared threshold the model is
//! wrong and the spec escalates — the threshold does not quietly move.**
//!
//! # Why this suite is here and not beside the renderer
//!
//! It renders the replay: the world `mc-sim` generates, meshed and packed
//! through the draw path `mc-render` owns. Neither of those crates may resolve
//! the other in any dependency kind — the seam test walks dev-dependencies too
//! — so the composition root is the only crate that resolves both. Same reason
//! `replay_offscreen.rs` sits here. The goldens still live under
//! `crates/mc-render/goldens/`, as `spec.md`'s binding table requires.
//!
//! # The camera is written out, not imported
//!
//! `spec.md`'s binding table puts the tick-60 eye at (−64, 56, 32) looking at
//! (32, 44, 32), and that is what the screen-space budget projected. Taking it
//! from `mc_sim::replay::pose` instead would make the probes agree with
//! whatever the replay's camera happens to be, which is the one thing they are
//! here not to do.

mod support;

use std::error::Error;
use std::sync::Arc;

use mc_render::camera::{CameraView, camera_view};
use mc_render::color::CLEAR_COLOR_SRGB;
use mc_testkit::frame::Rgba8Image;

use support::frames::ReplayFrame;
use support::probe::{self, ProbeOutcome};
use support::{TestResult, prepare_scene};

/// The tick every probe here examines.
const TICK: u32 = 60;

/// The declared tick-60 pose, from `spec.md`'s binding table.
const EYE: [f32; 3] = [-64.0, 56.0, 32.0];
const LOOK_AT: [f32; 3] = [32.0, 44.0, 32.0];

/// Where the landmark's cap centre lands on a 1280 × 720 frame at tick 60, and
/// where its horizontal mirror falls.
///
/// Derived in the screen-space budget from the declared camera: NDC
/// (−0.2536, +0.4029), which is 477.70 across and 214.95 down. The mirror is
/// `1279 − 478`. Both are hand-computed here and recomputed by the probe from
/// the renderer's own projection, so the two have to agree.
const LANDMARK_PIXEL: (u32, u32) = (478, 215);
const MIRROR_PIXEL: (u32, u32) = (801, 215);

/// The two pixels the orientation probe examines on a 1280 × 720 frame.
const TOP_CENTRE: (u32, u32) = (640, 0);
const BOTTOM_CENTRE: (u32, u32) = (640, 719);

#[test]
fn the_top_of_the_tick_sixty_frame_is_the_declared_sky_and_the_bottom_is_not() -> TestResult {
    let Some(frame) = tick_60_frame("terrain-probe-orientation")? else {
        return Ok(());
    };
    let outcome = probe::orientation(&frame)?;

    assert_eq!(
        (outcome.examined.as_slice(), outcome.failures.is_empty()),
        ([TOP_CENTRE, BOTTOM_CENTRE].as_slice(), true),
        "the camera looks down at the island from above its horizon, so the top-centre pixel \
         is the declared clear colour and the bottom-centre pixel is not. Inverting clip-space \
         y exchanges the two and leaves a picture that looks entirely plausible in a committed \
         PNG. Measured: {} — {:?}",
        outcome.detail,
        outcome.failures
    );
    Ok(())
}

#[test]
fn the_tick_sixty_frame_is_terrain_over_at_least_a_seventh_of_its_pixels() -> TestResult {
    let Some(frame) = tick_60_frame("terrain-probe-coverage")? else {
        return Ok(());
    };
    let outcome = probe::coverage(&frame)?;

    assert!(
        outcome.failures.is_empty(),
        "the declared camera's silhouette model puts the island over 21.9% of the frame with \
         an analytic floor of 18.8%, so a renderer that drew nothing, drew one quad, or wound \
         its faces inside out cannot reach the 15% this asks for. Measured: {}",
        outcome.detail
    );
    Ok(())
}

#[test]
fn the_landmark_stands_where_the_camera_maths_puts_it_and_its_mirror_is_sky() -> TestResult {
    let Some(frame) = tick_60_frame("terrain-probe-landmark")? else {
        return Ok(());
    };
    let outcome = probe::landmark(&frame, &declared_camera())?;

    assert_eq!(
        (outcome.examined.as_slice(), outcome.failures.is_empty()),
        ([LANDMARK_PIXEL, MIRROR_PIXEL].as_slice(), true),
        "the pillar's cap projects to the pixel the screen-space budget derived by hand, and \
         nothing in the world is tall enough to reach its mirror 323 px away. A frame mirrored \
         horizontally is exactly as self-asymmetric as a correct one, so this is the only \
         assertion that can tell them apart. Measured: {} — {:?}",
        outcome.detail,
        outcome.failures
    );
    Ok(())
}

#[test]
fn all_three_declared_block_colours_reach_the_tick_sixty_frame() -> TestResult {
    let Some(frame) = tick_60_frame("terrain-probe-textures")? else {
        return Ok(());
    };
    let outcome = probe::texture_variety(&frame)?;

    assert!(
        outcome.failures.is_empty(),
        "the frame is clustered against the three *declared* placeholder means rather than \
         against clusters found in it, so a renderer that resolved the texture layers and then \
         ignored them leaves two of the three clusters empty. Measured: {}",
        outcome.detail
    );
    Ok(())
}

#[test]
fn a_frame_of_nothing_but_sky_fails_every_probe_at_a_pixel_each_of_them_names() -> TestResult {
    let size = support::frames::CAPTURE_SIZE;
    let blank = probe::uniform(size.width, size.height, CLEAR_COLOR_SRGB)?;

    let outcomes = probe::suite(&blank, &declared_camera())?;
    let silent = quiet_probes(&outcomes);
    let unlocated = outcomes
        .iter()
        .flat_map(|outcome| &outcome.failures)
        .filter(|failure| !probe::NAMES.contains(&failure.probe) || !inside(&blank, failure.pixel))
        .count();

    assert_eq!(
        (outcomes.len(), silent.is_empty(), unlocated),
        (probe::NAMES.len(), true, 0),
        "a probe suite that cannot fail makes every assertion above worthless, so a frame of \
         nothing but the declared clear colour has to turn all of them red, each naming itself \
         and a pixel of the frame it looked at. Silent: {silent:?}. Reported: {outcomes:#?}"
    );
    Ok(())
}

#[test]
fn flipping_the_tick_sixty_frame_upside_down_turns_the_orientation_probe_red() -> TestResult {
    let Some(frame) = tick_60_frame("terrain-probe-flipped")? else {
        return Ok(());
    };
    let flipped = probe::flipped_vertically(&frame)?;

    let outcomes = probe::suite(&flipped, &declared_camera())?;
    let reported = pixels_reported_by(&outcomes, probe::ORIENTATION);

    assert!(
        reported.contains(&TOP_CENTRE),
        "a vertically mirrored world is the failure a committed golden cannot see, so the \
         probe that watches which way up the frame is has to name the pixel that changed: the \
         orientation probe reported {reported:?}"
    );
    Ok(())
}

#[test]
fn mirroring_the_tick_sixty_frame_left_to_right_turns_only_the_landmark_probe_red() -> TestResult {
    let Some(frame) = tick_60_frame("terrain-probe-mirrored")? else {
        return Ok(());
    };
    let mirrored = probe::mirrored_horizontally(&frame)?;

    let outcomes = probe::suite(&mirrored, &declared_camera())?;
    let loud = noisy_probes(&outcomes);
    let reported = pixels_reported_by(&outcomes, probe::LANDMARK);

    assert_eq!(
        (loud.as_slice(), reported.contains(&LANDMARK_PIXEL)),
        ([probe::LANDMARK].as_slice(), true),
        "mirroring preserves every count and every row, so it moves nothing the other three \
         probes measure — the landmark is the one that has to notice, at the pixel it \
         examined. It reported {reported:?} and the suite reported {outcomes:#?}"
    );
    Ok(())
}

/// The camera the frames below are seen from, written out rather than imported.
fn declared_camera() -> CameraView {
    camera_view(EYE, LOOK_AT)
}

/// The replay's tick-60 frame at the declared capture size, or `None` when the
/// opt-in permitted the absence of a device.
fn tick_60_frame(name: &str) -> Result<Option<Rgba8Image>, Box<dyn Error>> {
    let prepared = prepare_scene()?;
    let Some(context) = support::frames::device()? else {
        return Ok(None);
    };
    let mut renderer = support::frames::prepared_renderer(&context, &prepared)?;
    let scene = Arc::new(prepared.scene);
    let snapshot = support::frames::snapshot(TICK, declared_camera(), &scene);
    let request = support::frames::request(&context, name)?;

    let mut frame = ReplayFrame {
        context: &context,
        renderer: &mut renderer,
        snapshot: &snapshot,
    };
    Ok(Some(frame.capture(&request)?))
}

/// The probes that found nothing wrong.
fn quiet_probes(outcomes: &[ProbeOutcome]) -> Vec<&'static str> {
    outcomes
        .iter()
        .filter(|outcome| outcome.failures.is_empty())
        .map(|outcome| outcome.probe)
        .collect()
}

/// The probes that found something wrong.
fn noisy_probes(outcomes: &[ProbeOutcome]) -> Vec<&'static str> {
    outcomes
        .iter()
        .filter(|outcome| !outcome.failures.is_empty())
        .map(|outcome| outcome.probe)
        .collect()
}

/// Every pixel `probe` reported a failure at.
fn pixels_reported_by(outcomes: &[ProbeOutcome], probe: &str) -> Vec<(u32, u32)> {
    outcomes
        .iter()
        .filter(|outcome| outcome.probe == probe)
        .flat_map(|outcome| &outcome.failures)
        .map(|failure| failure.pixel)
        .collect()
}

/// Whether `at` is a position `frame` has.
fn inside(frame: &Rgba8Image, at: (u32, u32)) -> bool {
    frame.pixel(at.0, at.1).is_some()
}
