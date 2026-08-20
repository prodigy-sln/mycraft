//! What the replay's scene has to look like from the declared observation pose,
//! asserted without reference to any committed image.
//!
//! This is the suite the whole spec leans on. A golden re-shot from a broken
//! renderer is a golden of a broken renderer and passes forever; only an
//! assertion that does not come from the renderer can catch that, and these are
//! those assertions. They land **before any golden is minted**, because a
//! reference shot from a renderer nobody checked is worth nothing.
//!
//! # Every figure here was re-derived for this pose. None was inherited.
//!
//! The suite that stood here was derived for SPEC-004's orbit camera at tick 60,
//! `eye = (−64, 56, 32)` looking at `(32, 44, 32)`. Not one screen-space figure
//! survives a change of camera, so not one was carried over — not the landmark
//! pixel, not its mirror, not the coverage floor, not the per-colour floors, and
//! not a single number in this prose. The design-time heightmap figure the
//! architecture recorded (`surface_height(32, 32) = 40`) is wrong as well — the
//! shipped generator answers 37 — so anything descending from it is untrusted
//! too. Everything below is derived from the declared pose by hand, or from the
//! world's own voxels, and the two sources are named apart wherever both appear.
//!
//! # The camera is written out, not imported
//!
//! `spec.md` declares the observation pose as `eye = [44, 56, 44]` looking at
//! `[12, 52, 20]` at 1280 × 720, and that is what is written below. Taking it
//! from the simulation instead would make the probes agree with whatever the
//! published camera happens to be, which is the one thing they are here not to
//! do. It is deliberately **not** a pose the player ever reaches: the probes
//! verify the renderer, and the player's own poses are verified elsewhere. It is
//! off-axis from the pillar so that the landmark pixel and its horizontal mirror
//! are distinct, which a centred pose would not give.
//!
//! # Where the landmark pixel comes from
//!
//! By hand, from the declared pose and the declared sample point `[12.5, 58,
//! 12.5]`, and from nothing else. With `d = target − eye = (−32, −4, −24)`:
//!
//! - forward `f = d/|d| = (−8, −1, −6)/√101`
//! - right `s = normalise(f × up) = (0.6, 0, −0.8)`
//! - up `u = s × f = (−0.8, 10, −0.6)/√101`
//! - `p − eye = (−31.5, 2, −31.5)`, so `s·(p−eye) = 6.3`,
//!   `u·(p−eye) = 64.1/√101` and `w = f·(p−eye) = 439/√101`
//! - focal `cot 30° = √3`, aspect `16/9`, so
//!   `ndc.x = 6.3 · √3 · (9/16) · √101 / 439 = 0.1405141` and
//!   `ndc.y = 64.1 · √3 / 439 = 0.2529031`
//! - `across = (1 + 0.1405141) · 640 = 729.929` and
//!   `down = (1 − 0.2529031) · 360 = 268.955`, which round to **(730, 269)**
//! - the mirror is `1279 − 730 = 549`
//!
//! Reading that pixel off a rendered frame, or asking the renderer's own
//! projection for it, is exactly what the agreement assertion below forbids: two
//! computations that share a step cannot check each other.
//!
//! # Where the sample point sits inside the silhouette
//!
//! The pillar occupies `x ∈ [12, 13]`, `z ∈ [12, 13]`, with stone from its
//! column's surface at y = 36 up to and including voxel y = 64 — so its box
//! reaches y = 65. Projecting its four vertical edges at the sample's height
//! puts the silhouette's left edge at `across = 719.76` (the edge at x = 12,
//! z = 13) and its right edge at `740.15` (x = 13, z = 12), and projecting the
//! axis at y = 65 and at y = 36 puts its top at `down = 164.3` and its base at
//! `570.3`. The sample pixel therefore sits **10 px inside each vertical edge,
//! 104 px below the cap and 304 px above the base** — where SPEC-004's cap-centre
//! sample sat about one pixel inside an edge. Marching the world's own voxels
//! from the same pose agrees to the pixel: stone spans columns 720..=740 on row
//! 269 and rows 165..=573 in column 730.
//!
//! # Why this suite is here and not beside the renderer
//!
//! It renders the replay: the world `mc-sim` generates, meshed and packed
//! through the draw path `mc-render` owns. Neither of those crates may resolve
//! the other in any dependency kind — the seam test walks dev-dependencies too
//! — so the composition root is the only crate that resolves both. Same reason
//! `replay_offscreen.rs` sits here. The goldens still live under
//! `crates/mc-render/goldens/`, as `spec.md`'s binding table requires.

mod support;

use std::error::Error;
use std::sync::Arc;

use mc_render::camera::{CameraView, camera_view};
use mc_render::color::CLEAR_COLOR_SRGB;
use mc_render::texture::supplied::SuppliedTexels;
use mc_testkit::frame::Rgba8Image;

use support::frames::ReplayFrame;
use support::probe::{self, ProbeOutcome};
use support::{TestResult, prepare_scene};

/// The tick the observed snapshot is labelled with.
///
/// The scene is static and the pose is declared rather than reached, so nothing
/// about the picture depends on this number — it is the label a snapshot must
/// carry, not a camera path index. The orbit's tick 60 is gone with the orbit.
const TICK: u32 = 0;

/// The declared observation pose, from `spec.md`.
const EYE: [f32; 3] = [44.0, 56.0, 44.0];
const LOOK_AT: [f32; 3] = [12.0, 52.0, 20.0];

/// Where the pillar's declared interior sample point lands on a 1280 × 720
/// frame, and where its horizontal mirror falls.
///
/// Hand-derived in this module's header from the declared pose and the declared
/// sample point, and recomputed by the probe from the renderer's own projection,
/// so the two have to agree.
const LANDMARK_PIXEL: (u32, u32) = (730, 269);
const MIRROR_PIXEL: (u32, u32) = (549, 269);

/// The two pixels the orientation probe examines on a 1280 × 720 frame.
const TOP_CENTRE: (u32, u32) = (640, 0);
const BOTTOM_CENTRE: (u32, u32) = (640, 719);

#[test]
fn the_top_of_the_frame_is_the_declared_sky_and_the_bottom_is_not() -> TestResult {
    let Some(frame) = observed_frame("terrain-probe-orientation")? else {
        return Ok(());
    };
    let outcome = probe::orientation(&frame)?;

    assert_eq!(
        (outcome.examined.as_slice(), outcome.failures.is_empty()),
        ([TOP_CENTRE, BOTTOM_CENTRE].as_slice(), true),
        "the pose looks slightly down — its axis is depressed 5.7° and the lens takes in 30° \
         either side — so the top of the frame is above every horizon the island has and the \
         bottom lands on terrain 36 blocks out. Inverting clip-space y exchanges the two and \
         leaves a picture that looks entirely plausible in a committed PNG. Measured: {} — {:?}",
        outcome.detail,
        outcome.failures
    );
    Ok(())
}

#[test]
fn the_frame_shows_terrain_over_more_than_a_twelfth_of_its_pixels() -> TestResult {
    let Some(frame) = observed_frame("terrain-probe-coverage")? else {
        return Ok(());
    };
    let outcome = probe::coverage(&frame)?;

    assert!(
        outcome.failures.is_empty(),
        "the eye stands inside the footprint at y = 56 with every surface between 32 and 48 \
         below it, so the island's silhouette is bounded by projecting the flat planes those \
         two declared bounds describe: 10.34% of the frame for an all-32 world, 25.59% at the \
         mean surface and 41.88% for an all-48 one. The 8% floor sits a fifth below the \
         *lowest* of those, which is a bound no admissible heightmap can fall under — so a \
         renderer that drew nothing, drew one quad, or wound its faces inside out cannot reach \
         it. Measured: {}",
        outcome.detail
    );
    Ok(())
}

#[test]
fn the_pillars_interior_sample_point_is_drawn_as_something_other_than_sky() -> TestResult {
    let Some(frame) = observed_frame("terrain-probe-landmark")? else {
        return Ok(());
    };
    let outcome = probe::landmark(&frame, &declared_camera())?;

    assert_eq!(
        (
            outcome.examined.first().copied(),
            reported_at(&outcome, LANDMARK_PIXEL)
        ),
        (Some(LANDMARK_PIXEL), 0),
        "the sample point sits deep inside the pillar's stone — 10 px inside either vertical \
         edge and 104 px below the cap — so the pixel it projects to shows the pillar and not \
         the sky behind it. A sample taken at the cap's own centre would have stood a pixel \
         from the silhouette edge, where a sub-pixel drift decides the answer. Measured: {} — \
         {:?}",
        outcome.detail,
        outcome.failures
    );
    Ok(())
}

#[test]
fn the_horizontal_mirror_of_the_pillars_sample_pixel_is_drawn_as_sky() -> TestResult {
    let Some(frame) = observed_frame("terrain-probe-mirror")? else {
        return Ok(());
    };
    let outcome = probe::landmark(&frame, &declared_camera())?;

    assert_eq!(
        (
            outcome.examined.get(1).copied(),
            reported_at(&outcome, MIRROR_PIXEL)
        ),
        (Some(MIRROR_PIXEL), 0),
        "the ray through the mirror pixel leaves the eye climbing — its world direction is \
         (−0.895, +0.046, −0.489) — and the pillar is the only column in the world holding \
         anything above y = 48, 181 px away across the frame. So nothing can stand in that \
         pixel, and a frame mirrored left to right is the one thing that puts the pillar there. \
         Measured: {} — {:?}",
        outcome.detail,
        outcome.failures
    );
    Ok(())
}

#[test]
fn the_hand_derived_landmark_pixel_is_where_the_renderers_own_projection_lands() -> TestResult {
    let size = support::frames::CAPTURE_SIZE;
    let unrendered = probe::uniform(size.width, size.height, CLEAR_COLOR_SRGB)?;
    let outcome = probe::landmark(&unrendered, &declared_camera())?;

    assert_eq!(
        outcome.examined.as_slice(),
        [LANDMARK_PIXEL, MIRROR_PIXEL].as_slice(),
        "the pixel above was worked out by hand from the declared pose and the declared sample \
         point, and the probe works it out again by pushing that point through the renderer's \
         own view-projection. Two computations sharing no step have to land on the same pixel; \
         reading the pixel off a rendered frame instead would make this assertion a statement \
         about nothing. The probe needs no device to say where it looks, so no frame is drawn \
         here. Measured: {}",
        outcome.detail
    );
    Ok(())
}

#[test]
fn all_three_declared_block_colours_reach_the_frame() -> TestResult {
    let Some(frame) = observed_frame("terrain-probe-textures")? else {
        return Ok(());
    };
    let outcome = probe::texture_variety(&frame, &art_of_the_shipped_root()?)?;

    assert!(
        outcome.failures.is_empty(),
        "the frame is clustered against the three *declared* placeholder means rather than \
         against clusters found in it, so a renderer that resolved the texture layers and then \
         ignored them leaves two of the three clusters empty. Each declared mean carries its \
         own floor, because this pose shows the three strata in wildly different amounts. \
         Measured: {}",
        outcome.detail
    );
    Ok(())
}

#[test]
fn a_frame_of_nothing_but_sky_fails_every_probe_at_a_pixel_each_of_them_names() -> TestResult {
    let size = support::frames::CAPTURE_SIZE;
    let blank = probe::uniform(size.width, size.height, CLEAR_COLOR_SRGB)?;

    let outcomes = probe::suite(&blank, &declared_camera(), &art_of_the_shipped_root()?)?;
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
fn flipping_the_frame_upside_down_turns_the_orientation_probe_red() -> TestResult {
    let Some(frame) = observed_frame("terrain-probe-flipped")? else {
        return Ok(());
    };
    let flipped = probe::flipped_vertically(&frame)?;

    let outcomes = probe::suite(&flipped, &declared_camera(), &art_of_the_shipped_root()?)?;
    let reported = pixels_reported_by(&outcomes, probe::ORIENTATION);

    assert!(
        reported.contains(&TOP_CENTRE),
        "a vertically mirrored world is the failure a committed golden cannot see, so the \
         probe that watches which way up the frame is has to name the pixel that changed. \
         Under this pose the whole of row 0 is sky and the whole of row 719 is terrain, so \
         the flip exchanges two rows that could not be confused: the orientation probe \
         reported {reported:?}"
    );
    Ok(())
}

#[test]
fn mirroring_the_frame_left_to_right_turns_only_the_landmark_probe_red() -> TestResult {
    let Some(frame) = observed_frame("terrain-probe-mirrored")? else {
        return Ok(());
    };
    let mirrored = probe::mirrored_horizontally(&frame)?;

    let outcomes = probe::suite(&mirrored, &declared_camera(), &art_of_the_shipped_root()?)?;
    let loud = noisy_probes(&outcomes);
    let reported = pixels_reported_by(&outcomes, probe::LANDMARK);

    assert_eq!(
        (loud.as_slice(), reported.contains(&LANDMARK_PIXEL)),
        ([probe::LANDMARK].as_slice(), true),
        "mirroring preserves every count, so the two area probes cannot see it; and under this \
         pose row 0 is sky across its full width and row 719 is terrain across its full width, \
         so the orientation probe's two pixels keep their answers for a reason rather than by \
         luck. The landmark is the one that has to notice, at the pixel it examined. It \
         reported {reported:?} and the suite reported {outcomes:#?}"
    );
    Ok(())
}

/// The texels the shipped root's built set offers, read through the client's own
/// reader.
///
/// **A file on disk, never a frame.** The strata a correct picture shows are the
/// art the manifest bakes, and the three colours a pixel is clustered against
/// have to come from that art rather than from a generator that no longer
/// describes it — the generated mean for `base:dirt` stands ΔE 62.94 from the
/// dirt this root actually draws.
fn art_of_the_shipped_root() -> Result<SuppliedTexels, Box<dyn Error>> {
    support::art::built_texels(&support::content_root()?)
}

/// The camera the frames above are seen from, written out rather than imported.
fn declared_camera() -> CameraView {
    camera_view(EYE, LOOK_AT)
}

/// The replay's scene from the declared pose at the declared capture size, or
/// `None` when the opt-in permitted the absence of a device.
fn observed_frame(name: &str) -> Result<Option<Rgba8Image>, Box<dyn Error>> {
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

/// How many failures one probe reported at `pixel`.
fn reported_at(outcome: &ProbeOutcome, pixel: (u32, u32)) -> usize {
    outcome
        .failures
        .iter()
        .filter(|failure| failure.pixel == pixel)
        .count()
}

/// Whether `at` is a position `frame` has.
fn inside(frame: &Rgba8Image, at: (u32, u32)) -> bool {
    frame.pixel(at.0, at.1).is_some()
}
