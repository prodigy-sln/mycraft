//! Assertions about a captured frame that do not come from a committed image.
//!
//! A golden re-shot from a broken renderer is a golden of a broken renderer,
//! and it passes forever. The only thing that catches that is a statement about
//! the picture derived from somewhere else — here, from `spec.md`'s declared
//! camera, world and colours, and from the arithmetic `architecture.md`'s
//! screen-space budget did over them before any of this was rendered.
//!
//! **Nothing in this file reads a colour, a count or a position out of a
//! frame and then compares the frame against it.** The sky is
//! `mc_render::color::CLEAR_COLOR_SRGB`, a declaration. The three block
//! colours are `placeholder_mean_color`, a function of a texture key. The
//! landmark's pixel is its declared world position pushed through the declared
//! camera. The coverage floor is a fifth below an analytic floor computed from
//! the island's silhouette.
//!
//! **The metric is the harness's, driven rather than copied** (D-I).
//! `delta_e` is `pub(crate)` in `mc-testkit` and is *"the sole place a distance
//! is computed, and the sole swap point if CIE76 is later replaced"*. So every
//! distance below is a `compare` against a uniform field of a declared colour:
//! a mask for one pixel, a failing fraction for an area, a one-pixel pair for a
//! scalar. A second implementation would let goldens and probes silently judge
//! by different metrics the day the metric changes.
//!
//! # Probes report; they do not assert
//!
//! Each probe hands back what it examined, what it measured *in words whether
//! or not it was satisfied*, and a list of failures each naming its probe and
//! the pixel it looked at. That shape is what makes the suite feedable: a blank
//! frame, a vertically flipped frame and a horizontally mirrored frame are the
//! three controls proving these probes can fail at all, and they need a value
//! to inspect rather than a panic to catch.
//!
//! Area probes examine every pixel, so "the pixel it examined" is a category
//! error for them. They anchor at the frame's centre and say so — a declared
//! anchor, never a discovered one, so the report is the same on every run.

use std::error::Error;

use glam::Vec4;
use mc_core::id::TextureKey;
use mc_render::camera::{CameraView, projection_for, view_projection};
use mc_render::color::CLEAR_COLOR_SRGB;
use mc_render::surface::SurfaceSize;
use mc_render::texture::placeholder::placeholder_mean_color;
use mc_testkit::frame::{Rgba8Image, Thresholds, compare};

/// The probe that says which way up the world is.
pub const ORIENTATION: &str = "orientation";
/// The probe that says the renderer drew more than a quad.
pub const COVERAGE: &str = "coverage";
/// The probe that says which way round the frame is.
pub const LANDMARK: &str = "landmark";
/// The probe that says the texture layers reached the shader.
pub const TEXTURE_VARIETY: &str = "texture-variety";

/// Every probe, in the order [`suite`] runs them.
pub const NAMES: [&str; 4] = [ORIENTATION, COVERAGE, LANDMARK, TEXTURE_VARIETY];

/// How close a pixel has to sit to a declared colour to be called that colour.
///
/// The harness's own per-pixel default, so "this is the sky" means here what it
/// means in a golden comparison.
pub const SAME_COLOR: f64 = 2.0;

/// How far a pixel has to sit from a declared colour to be called a different
/// one. The harness's own hard ceiling: a pixel this far off is a defect on its
/// own, whatever share of the frame it occupies.
pub const DIFFERENT_COLOR: f64 = 10.0;

/// The smallest share of the frame terrain may cover.
///
/// Derived, never measured. Projecting the island's bounding box through the
/// declared tick-60 camera and taking the shoelace area of the clipped
/// silhouette gives 18.8% for a hypothetical all-32 world, 21.9% at the mean
/// surface and 25.1% for an all-48 one. 15% sits a fifth below the *floor*, so
/// it absorbs the silhouette model's error, and it is still roughly four times
/// what the largest single quad covers — so "the renderer drew one quad" fails
/// it too.
pub const COVERAGE_FLOOR: f64 = 0.15;

/// The smallest share of the frame one declared block colour may cover.
///
/// Dirt is the binding case: it is exposed only where a step between adjacent
/// columns is two blocks tall, which the heightmap's own `<= 2` coherence bound
/// makes the minority case, and the budget puts it near 0.7% of the frame with
/// a factor-of-two modelling error either way. 0.25% restores the margin and is
/// still around 2 300 pixels at 1280 × 720, far above any noise floor.
pub const COLOR_SHARE_FLOOR: f64 = 0.0025;

/// The three blocks the replay's strata are made of, and therefore the three
/// texture layers a correct frame shows.
pub const STRATA: [&str; 3] = ["base:dirt", "base:grass", "base:stone"];

/// The centre of the landmark pillar's cap.
///
/// Column (12, 12) filled with stone to y = 64, so the cap's centre is
/// (12.5, 64, 12.5). The pillar is the only thing in the world above the eye's
/// y = 56, which is what puts it above the horizon and its mirror on empty sky.
pub const LANDMARK_TOP_CENTRE: [f32; 3] = [12.5, 64.0, 12.5];

/// A declared block colour, and the block it was declared for.
type Mean = (&'static str, [u8; 3]);

/// One thing a probe found wrong.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbeFailure {
    /// Which probe found it.
    pub probe: &'static str,
    /// The pixel it was looking at when it did.
    pub pixel: (u32, u32),
    /// What it expected there and what it found.
    pub detail: String,
}

/// What one probe concluded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbeOutcome {
    pub probe: &'static str,
    /// Every pixel this probe examined, in order. Area probes report the pixel
    /// their measurement is anchored at.
    pub examined: Vec<(u32, u32)>,
    /// What it measured, whether or not that satisfied it — a passing probe's
    /// figures are what its red neighbour is read against.
    pub detail: String,
    /// Empty when the probe was satisfied.
    pub failures: Vec<ProbeFailure>,
}

/// Every probe, run over `frame`.
///
/// # Errors
///
/// Returns the image-shape or threshold failure, or a position outside the
/// frame.
pub fn suite(frame: &Rgba8Image, camera: &CameraView) -> Result<Vec<ProbeOutcome>, Box<dyn Error>> {
    Ok(vec![
        orientation(frame)?,
        coverage(frame)?,
        landmark(frame, camera)?,
        texture_variety(frame)?,
    ])
}

/// Sky at the top of the frame, terrain at the bottom.
///
/// The camera looks *down* at the island from above its horizon, so this is the
/// probe a clip-space y inversion turns red — and a vertically mirrored world
/// is entirely plausible in a committed PNG, which is why this is derived
/// rather than left to a golden.
///
/// # Errors
///
/// Returns the image-shape or threshold failure, or a position outside the
/// frame.
pub fn orientation(frame: &Rgba8Image) -> Result<ProbeOutcome, Box<dyn Error>> {
    let column = frame.width() >> 1;
    let top = (column, 0);
    let bottom = (column, frame.height().saturating_sub(1));
    let to_sky = distance(pixel_color(frame, top)?, CLEAR_COLOR_SRGB)?;
    let to_ground = distance(pixel_color(frame, bottom)?, CLEAR_COLOR_SRGB)?;

    Ok(ProbeOutcome {
        probe: ORIENTATION,
        examined: vec![top, bottom],
        detail: format!(
            "top-centre {top:?} stands ΔE {to_sky:.1} from the declared clear colour and \
             bottom-centre {bottom:?} stands ΔE {to_ground:.1} from it"
        ),
        failures: orientation_faults(top, bottom, to_sky, to_ground),
    })
}

/// Which of the orientation probe's two sides was not satisfied.
fn orientation_faults(
    top: (u32, u32),
    bottom: (u32, u32),
    to_sky: f64,
    to_ground: f64,
) -> Vec<ProbeFailure> {
    [
        (to_sky > SAME_COLOR).then(|| {
            let detail = format!(
                "stands ΔE {to_sky:.1} from the declared clear colour, past the ΔE \
                 {SAME_COLOR} that calls a pixel sky"
            );
            fault(ORIENTATION, top, detail)
        }),
        (to_ground <= DIFFERENT_COLOR).then(|| {
            let detail = format!(
                "stands ΔE {to_ground:.1} from the declared clear colour, short of the ΔE \
                 {DIFFERENT_COLOR} that calls a pixel terrain"
            );
            fault(ORIENTATION, bottom, detail)
        }),
    ]
    .into_iter()
    .flatten()
    .collect()
}

/// Enough of the frame is not sky.
///
/// The one probe that a renderer drawing nothing, drawing one quad, or drawing
/// with its winding inverted cannot satisfy.
///
/// # Errors
///
/// Returns the image-shape or threshold failure.
pub fn coverage(frame: &Rgba8Image) -> Result<ProbeOutcome, Box<dyn Error>> {
    let anchor = anchor_of(frame);
    let covered = share_beyond(frame, CLEAR_COLOR_SRGB, DIFFERENT_COLOR)?;
    let detail = format!(
        "{:.2}% of the frame stands more than ΔE {DIFFERENT_COLOR} from the declared clear \
         colour, against the {:.2}% floor",
        covered * 100.0,
        COVERAGE_FLOOR * 100.0
    );
    let failures = (covered < COVERAGE_FLOOR)
        .then(|| fault(COVERAGE, anchor, detail.clone()))
        .into_iter()
        .collect();

    Ok(ProbeOutcome {
        probe: COVERAGE,
        examined: vec![anchor],
        detail,
        failures,
    })
}

/// The landmark stands where the camera maths says it does, and its horizontal
/// mirror is sky.
///
/// Two-sided on purpose. A mirrored frame is exactly as self-asymmetric as a
/// correct one, so comparing a frame against its own mirror proves nothing
/// about which way round it is; only pushing a declared world position through
/// the declared camera does.
///
/// # Errors
///
/// Returns the image-shape or threshold failure, a position outside the frame,
/// or a landmark that does not project in front of the camera at all.
pub fn landmark(frame: &Rgba8Image, camera: &CameraView) -> Result<ProbeOutcome, Box<dyn Error>> {
    let size = SurfaceSize {
        width: frame.width(),
        height: frame.height(),
    };
    let at = project(LANDMARK_TOP_CENTRE, camera, size)?;
    let mirror = (size.width.saturating_sub(1).saturating_sub(at.0), at.1);
    let to_pillar = distance(pixel_color(frame, at)?, CLEAR_COLOR_SRGB)?;
    let to_sky = distance(pixel_color(frame, mirror)?, CLEAR_COLOR_SRGB)?;

    Ok(ProbeOutcome {
        probe: LANDMARK,
        examined: vec![at, mirror],
        detail: format!(
            "the landmark projects to {at:?}, ΔE {to_pillar:.1} from the declared clear \
             colour, and its mirror {mirror:?} stands ΔE {to_sky:.1} from it"
        ),
        failures: landmark_faults(at, mirror, to_pillar, to_sky),
    })
}

/// Which of the landmark probe's two sides was not satisfied.
fn landmark_faults(
    at: (u32, u32),
    mirror: (u32, u32),
    to_pillar: f64,
    to_sky: f64,
) -> Vec<ProbeFailure> {
    [
        (to_pillar <= DIFFERENT_COLOR).then(|| {
            fault(
                LANDMARK,
                at,
                format!(
                    "shows sky: it stands ΔE {to_pillar:.1} from the declared clear colour, short \
                 of the ΔE {DIFFERENT_COLOR} that would call it the pillar"
                ),
            )
        }),
        (to_sky > SAME_COLOR).then(|| {
            fault(
                LANDMARK,
                mirror,
                format!(
                    "shows the pillar: it stands ΔE {to_sky:.1} from the declared clear colour, \
                 past the ΔE {SAME_COLOR} that would call it sky"
                ),
            )
        }),
    ]
    .into_iter()
    .flatten()
    .collect()
}

/// All three declared block colours reach the frame, and they are three
/// colours.
///
/// Clustered against the **declared** means rather than against clusters
/// discovered in the frame: a renderer that resolved the texture layers and
/// then ignored them leaves two of the three clusters empty, which discovering
/// clusters would never have noticed.
///
/// # Errors
///
/// Returns the image-shape or threshold failure, or an unparseable texture key.
pub fn texture_variety(frame: &Rgba8Image) -> Result<ProbeOutcome, Box<dyn Error>> {
    let anchor = anchor_of(frame);
    let means = declared_means()?;
    let (detail, mut failures) = color_shares(frame, &means, anchor)?;
    failures.extend(distinct_means(&means, anchor)?);

    Ok(ProbeOutcome {
        probe: TEXTURE_VARIETY,
        examined: vec![anchor],
        detail,
        failures,
    })
}

/// The declared mean colour of each of the replay's three strata.
fn declared_means() -> Result<Vec<Mean>, Box<dyn Error>> {
    STRATA
        .into_iter()
        .map(|block| Ok((block, placeholder_mean_color(&TextureKey::parse(block)?))))
        .collect()
}

/// What share of the frame each declared colour covers, and which of them fell
/// short.
fn color_shares(
    frame: &Rgba8Image,
    means: &[Mean],
    anchor: (u32, u32),
) -> Result<(String, Vec<ProbeFailure>), Box<dyn Error>> {
    let mut described = Vec::new();
    let mut failures = Vec::new();
    for (block, mean) in means {
        let share = share_within(frame, *mean, DIFFERENT_COLOR)?;
        described.push(format!("{block} {:.3}%", share * 100.0));
        if share <= COLOR_SHARE_FLOOR {
            failures.push(fault(
                TEXTURE_VARIETY,
                anchor,
                format!(
                    "{block}'s declared mean {mean:?} covers {:.3}% of the frame, at or under the \
                 {:.2}% floor",
                    share * 100.0,
                    COLOR_SHARE_FLOOR * 100.0
                ),
            ));
        }
    }
    let detail = format!(
        "within ΔE {DIFFERENT_COLOR} of each declared mean: {} (floor {:.2}%)",
        described.join(", "),
        COLOR_SHARE_FLOOR * 100.0
    );
    Ok((detail, failures))
}

/// Which declared means a viewer could not tell apart.
fn distinct_means(means: &[Mean], anchor: (u32, u32)) -> Result<Vec<ProbeFailure>, Box<dyn Error>> {
    let mut failures = Vec::new();
    for ((block, mean), (other_block, other)) in pairs(means) {
        let apart = distance(mean, other)?;
        if apart <= DIFFERENT_COLOR {
            failures.push(fault(
                TEXTURE_VARIETY,
                anchor,
                format!(
                    "the declared means of {block} {mean:?} and {other_block} {other:?} stand ΔE \
                 {apart:.1} apart, at or under the ΔE {DIFFERENT_COLOR} that tells two \
                 textures apart — so finding both in a frame would say nothing"
                ),
            ));
        }
    }
    Ok(failures)
}

/// Every unordered pair of `means`, flattened so the walk over them nests once.
fn pairs(means: &[Mean]) -> Vec<(Mean, Mean)> {
    means
        .iter()
        .enumerate()
        .flat_map(|(index, one)| {
            means
                .iter()
                .skip(index + 1)
                .map(move |other| (*one, *other))
        })
        .collect()
}

/// `frame` with its rows reversed.
///
/// # Errors
///
/// Returns the image-shape failure.
pub fn flipped_vertically(frame: &Rgba8Image) -> Result<Rgba8Image, Box<dyn Error>> {
    let last = frame.height().saturating_sub(1);
    remapped(frame, |x, y| (x, last.saturating_sub(y)))
}

/// `frame` with its columns reversed.
///
/// # Errors
///
/// Returns the image-shape failure.
pub fn mirrored_horizontally(frame: &Rgba8Image) -> Result<Rgba8Image, Box<dyn Error>> {
    let last = frame.width().saturating_sub(1);
    remapped(frame, |x, y| (last.saturating_sub(x), y))
}

/// A frame whose pixel at `(x, y)` is `frame`'s at `source(x, y)`.
fn remapped(
    frame: &Rgba8Image,
    source: impl Fn(u32, u32) -> (u32, u32),
) -> Result<Rgba8Image, Box<dyn Error>> {
    let (width, height) = (frame.width(), frame.height());
    let mut pixels = Vec::with_capacity(width as usize * height as usize * 4);
    for y in 0..height {
        for x in 0..width {
            let from = source(x, y);
            let pixel = frame
                .pixel(from.0, from.1)
                .ok_or_else(|| format!("the frame has no pixel at {from:?}"))?;
            pixels.extend_from_slice(&pixel);
        }
    }
    Ok(Rgba8Image::from_rgba(width, height, pixels)?)
}

/// One failure, named.
fn fault(probe: &'static str, pixel: (u32, u32), detail: String) -> ProbeFailure {
    ProbeFailure {
        probe,
        pixel,
        detail,
    }
}

/// The pixel an area probe reports, declared rather than discovered.
fn anchor_of(frame: &Rgba8Image) -> (u32, u32) {
    (frame.width() >> 1, frame.height() >> 1)
}

/// Where `world` lands on a frame of `size` seen through `camera`.
fn project(
    world: [f32; 3],
    camera: &CameraView,
    size: SurfaceSize,
) -> Result<(u32, u32), Box<dyn Error>> {
    let [x, y, z] = world;
    let clip = view_projection(camera, &projection_for(size)) * Vec4::new(x, y, z, 1.0);
    if clip.w <= 0.0 {
        return Err(format!("{world:?} does not project in front of the camera").into());
    }
    let across = (clip.x / clip.w + 1.0) * 0.5 * size.width as f32;
    let down = (1.0 - clip.y / clip.w) * 0.5 * size.height as f32;
    Ok((across.round() as u32, down.round() as u32))
}

/// The colour at `at`, alpha dropped.
fn pixel_color(frame: &Rgba8Image, at: (u32, u32)) -> Result<[u8; 3], Box<dyn Error>> {
    let [red, green, blue, _] = frame
        .pixel(at.0, at.1)
        .ok_or_else(|| format!("the frame has no pixel at {at:?}"))?;
    Ok([red, green, blue])
}

/// The perceptual distance between two colours, measured by the harness's own
/// metric on a pair of one-pixel frames.
fn distance(left: [u8; 3], right: [u8; 3]) -> Result<f64, Box<dyn Error>> {
    let one = uniform(1, 1, left)?;
    let other = uniform(1, 1, right)?;
    Ok(compare(&one, &other, &Thresholds::default()).max_delta_e)
}

/// What share of `frame` sits within `tolerance` of `color`.
fn share_within(frame: &Rgba8Image, color: [u8; 3], tolerance: f64) -> Result<f64, Box<dyn Error>> {
    Ok(1.0 - share_beyond(frame, color, tolerance)?)
}

/// What share of `frame` sits further than `tolerance` from `color`.
fn share_beyond(frame: &Rgba8Image, color: [u8; 3], tolerance: f64) -> Result<f64, Box<dyn Error>> {
    let field = uniform(frame.width(), frame.height(), color)?;
    let thresholds = Thresholds::new(tolerance, 1.0, f64::MAX)?;
    Ok(compare(&field, frame, &thresholds).failing_fraction)
}

/// A frame of `width` × `height` filled with `color`.
///
/// Public because the blank control is one: a frame of nothing but the declared
/// clear colour is what every probe here has to be able to fail against.
///
/// # Errors
///
/// Returns the image-shape failure when the dimensions do not match the pixels.
pub fn uniform(width: u32, height: u32, color: [u8; 3]) -> Result<Rgba8Image, Box<dyn Error>> {
    let [red, green, blue] = color;
    let pixels = std::iter::repeat_n([red, green, blue, 255], (width * height) as usize)
        .flatten()
        .collect();
    Ok(Rgba8Image::from_rgba(width, height, pixels)?)
}
