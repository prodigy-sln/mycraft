//! Assertions about a captured frame that do not come from a committed image.
//!
//! A golden re-shot from a broken renderer is a golden of a broken renderer,
//! and it passes forever. The only thing that catches that is a statement about
//! the picture derived from somewhere else — here, from `spec.md`'s declared
//! observation pose, world and colours, and from the arithmetic done over them
//! by hand before any of this was rendered.
//!
//! **Every screen-space figure below was re-derived for that pose, and none was
//! inherited.** The suite this grew out of was shot through an orbit camera at
//! `eye = (−64, 56, 32)`; a figure derived for one camera is a statement about
//! that camera and about nothing else, so the landmark's sample point, the
//! coverage floor and the per-colour floors were all worked out again from
//! scratch. The derivations live beside the constants they produced.
//!
//! **Nothing in this file reads a colour, a count or a position out of a
//! frame and then compares the frame against it.** The sky is
//! `mc_render::color::CLEAR_COLOR_SRGB`, a declaration. The three strata's
//! colours come from `support::art`, which reads the built set's own images
//! through the client's decoder and the generator for a key the set does not
//! cover — a file on disk, never a picture. The landmark's pixel is its declared
//! world position pushed through the declared camera. The coverage floor is a
//! fifth below an analytic floor computed from the island's silhouette.
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
use mc_render::texture::supplied::SuppliedTexels;
use mc_testkit::frame::{Rgba8Image, Thresholds, compare};

use super::art::{drawn_texels, landmarks, linear_mean, share_within_any};

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
/// Derived, never measured. The eye stands *inside* the 64 × 64 footprint at
/// y = 56, with every surface between the declared 32 and 48 below it, so the
/// island's silhouette is bracketed by projecting the flat plane each of those
/// two bounds describes and taking the area below the footprint's two far edges
/// — the near two are behind the camera. That gives 10.34% of the frame for an
/// all-32 world, 25.59% at the mean surface and 41.88% for an all-48 one. The
/// all-32 figure is a genuine lower bound over every admissible heightmap: no
/// column's surface is under 32, so a ray reaching that plane inside the
/// footprint has already met a solid voxel. 8% sits a fifth below it, which is
/// the rule the orbit's own floor was derived by.
///
/// The slack is deliberate rather than unexamined. What this probe answers is
/// "the renderer drew more than a quad"; the tight statement about what the
/// frame shows is the ray-marched oracle, which judges the player's camera
/// against the world's own voxels.
pub const COVERAGE_FLOOR: f64 = 0.08;

/// The three texture keys the replay's strata draw from — and therefore the
/// three array layers a correct frame shows — each with the smallest share of
/// the frame it may cover.
///
/// **Texture keys, not block names, and that distinction now bites.** The grass
/// block declares six facings: its top draws `base:grass_top`, its underside
/// draws `base:dirt`, and its four sides draw four keys of their own. There is
/// no key called `base:grass` any more.
///
/// **Which three, and why not a grass side.** `base:dirt`, `base:grass_top` and
/// `base:stone` stand ΔE 26.89 to 53.70 apart, comfortably clear of the ΔE 10
/// that tells two textures apart. A grass side is not one of them and must not
/// become one: a grass side *is* mostly dirt, and `base:dirt` against
/// `base:grass_side_west` measures **ΔE 9.59** — already under that ceiling — so
/// adding one would turn [`distinct_means`] red against a correct renderer, and
/// the cheapest way to green it would be to raise the constant that lets this
/// probe tell two textures apart at all.
///
/// One floor per key rather than one for all three, because this pose shows the
/// three strata in wildly different amounts and no single number is both
/// meaningful for the largest and satisfiable for the smallest. **The three
/// floors are the geometric bounds they always were and none of them moved**;
/// what changed is which colours a pixel is judged against.
///
/// - **`base:grass_top`, 7.5%**: every column's surface block is grass and its
///   upward face is the one this pose sees, so the all-32 silhouette less every
///   visible face that is not a grass top bounds it from below.
/// - **`base:stone`, 0.4%**: the pillar alone projects to 0.54% of the frame
///   above y = 48, where nothing in the world can occlude it, and 0.4% is a
///   fifth below that.
/// - **`base:dirt`, presence, and now less than it was.** Dirt was exposed only
///   on the side of a two-block step, which from this pose came to 7 pixels of
///   921 600 — emptiness was the only threshold the pose supported. It is now
///   *also* what four fifths of every grass side is made of: **81.25% to 82.81%**
///   of a grass side's texels sit within ΔE 10 of a `base:dirt` landmark,
///   measured over all four sides of the built set — east lowest, west highest.
///   A range rather than one side's figure, because the claim is about every
///   side. **So this reading no longer witnesses the dirt texture
///   specifically; it witnesses that the dirt palette reaches the frame**, which
///   it does through the sides of every grass block in view. Stated rather than
///   left for a reader to discover, because a floor that is easy to satisfy and
///   reads as though it were hard is worse than no floor.
pub const STRATA: [(&str, f64); 3] = [
    ("base:dirt", 0.0),
    ("base:grass_top", 0.075),
    ("base:stone", 0.004),
];

/// A point deep inside the landmark pillar's silhouette.
///
/// Column (12, 12) is stone from its surface at y = 36 up to and including
/// y = 64, so (12.5, 58, 12.5) is stone with the whole width of the pillar
/// around it and 104 px of it above. The pillar is the only column in the world
/// holding anything above y = 48, which is what leaves this pixel's horizontal
/// mirror on empty sky.
///
/// It supersedes the cap's centre, (12.5, 64, 12.5), which sat about one pixel
/// inside the silhouette's edge — close enough that a sub-pixel drift decided
/// the answer.
pub const LANDMARK_SAMPLE_POINT: [f32; 3] = [12.5, 58.0, 12.5];

/// One declared stratum: the texture key, that layer's own mean, the colours a
/// pixel drawn from it may hold, and the smallest share of the frame it may
/// cover.
///
/// **The mean and the landmarks answer different questions.** The mean is what
/// [`distinct_means`] tells one stratum from another by; the landmarks are what
/// a pixel is judged to belong to a stratum by, and they include the mean
/// because a minified face converges to it.
#[derive(Debug, Clone)]
struct Stratum {
    block: &'static str,
    mean: [u8; 3],
    landmarks: Vec<[u8; 3]>,
    floor: f64,
}

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
pub fn suite(
    frame: &Rgba8Image,
    camera: &CameraView,
    supplied: &SuppliedTexels,
) -> Result<Vec<ProbeOutcome>, Box<dyn Error>> {
    Ok(vec![
        orientation(frame)?,
        coverage(frame)?,
        landmark(frame, camera)?,
        texture_variety(frame, supplied)?,
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
    let at = project(LANDMARK_SAMPLE_POINT, camera, size)?;
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
pub fn texture_variety(
    frame: &Rgba8Image,
    supplied: &SuppliedTexels,
) -> Result<ProbeOutcome, Box<dyn Error>> {
    let anchor = anchor_of(frame);
    let strata = declared_strata(supplied)?;
    let (detail, mut failures) = color_shares(frame, &strata, anchor)?;
    failures.extend(distinct_means(&strata, anchor)?);

    Ok(ProbeOutcome {
        probe: TEXTURE_VARIETY,
        examined: vec![anchor],
        detail,
        failures,
    })
}

/// Each of the replay's three strata, with the colours its layer is filled
/// from.
///
/// Read from the built set through the client's own decoder, or from the
/// generator for a key the set does not cover — never from a frame.
fn declared_strata(supplied: &SuppliedTexels) -> Result<Vec<Stratum>, Box<dyn Error>> {
    STRATA
        .into_iter()
        .map(|(block, floor)| {
            let key = TextureKey::parse(block)?;
            Ok(Stratum {
                block,
                mean: linear_mean(&drawn_texels(&key, supplied)),
                landmarks: landmarks(&key, supplied),
                floor,
            })
        })
        .collect()
}

/// What share of the frame each declared colour covers, and which of them fell
/// short of its own floor.
fn color_shares(
    frame: &Rgba8Image,
    strata: &[Stratum],
    anchor: (u32, u32),
) -> Result<(String, Vec<ProbeFailure>), Box<dyn Error>> {
    let mut described = Vec::new();
    let mut failures = Vec::new();
    for stratum in strata {
        let (block, floor) = (stratum.block, stratum.floor);
        let share = share_within_any(frame, &stratum.landmarks, DIFFERENT_COLOR)?;
        described.push(format!(
            "{block} {:.4}% (floor {:.4}%)",
            share * 100.0,
            floor * 100.0
        ));
        if share <= floor {
            failures.push(fault(
                TEXTURE_VARIETY,
                anchor,
                format!(
                    "{block}'s declared colours {colors:?} cover {:.4}% of the frame, at or under \
                     its own {:.4}% floor",
                    share * 100.0,
                    floor * 100.0,
                    colors = stratum.landmarks
                ),
            ));
        }
    }
    let detail = format!(
        "within ΔE {DIFFERENT_COLOR} of a colour of each: {}",
        described.join(", ")
    );
    Ok((detail, failures))
}

/// Which declared means a viewer could not tell apart.
fn distinct_means(
    strata: &[Stratum],
    anchor: (u32, u32),
) -> Result<Vec<ProbeFailure>, Box<dyn Error>> {
    let mut failures = Vec::new();
    for (one, other) in pairs(strata) {
        let apart = distance(one.mean, other.mean)?;
        if apart <= DIFFERENT_COLOR {
            failures.push(fault(
                TEXTURE_VARIETY,
                anchor,
                format!(
                    "the declared means of {} {:?} and {} {:?} stand ΔE {apart:.1} apart, at or \
                     under the ΔE {DIFFERENT_COLOR} that tells two textures apart — so finding \
                     both in a frame would say nothing",
                    one.block, one.mean, other.block, other.mean
                ),
            ));
        }
    }
    Ok(failures)
}

/// Every unordered pair of `strata`, flattened so the walk over them nests once.
fn pairs(strata: &[Stratum]) -> Vec<(&Stratum, &Stratum)> {
    strata
        .iter()
        .enumerate()
        .flat_map(|(index, one)| strata.iter().skip(index + 1).map(move |other| (one, other)))
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
///
/// Public so that a reading which has to *find* a face's silhouette can use the
/// same projection the landmark probe does. Finding a pixel this way is the
/// established idiom here and is not a shared expectation: what the caller then
/// asserts about that pixel has to come from somewhere other than the renderer.
pub fn project(
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
///
/// Public so that the ray-marched oracle reads a pixel the way every probe here
/// reads one.
///
/// # Errors
///
/// Returns a failure naming `at` when the frame has no pixel there.
pub fn pixel_color(frame: &Rgba8Image, at: (u32, u32)) -> Result<[u8; 3], Box<dyn Error>> {
    let [red, green, blue, _] = frame
        .pixel(at.0, at.1)
        .ok_or_else(|| format!("the frame has no pixel at {at:?}"))?;
    Ok([red, green, blue])
}

/// The perceptual distance between two colours, measured by the harness's own
/// metric on a pair of one-pixel frames.
///
/// Public so that the ray-marched oracle judges "this pixel is the sky" by the
/// same metric the probes and the goldens judge by. A second implementation of
/// it is how two suites come to disagree silently the day the metric changes,
/// which is the reason this file drives `compare` rather than computing a
/// distance itself.
///
/// # Errors
///
/// Returns the image-shape failure, which a one-pixel frame cannot produce.
pub fn distance(left: [u8; 3], right: [u8; 3]) -> Result<f64, Box<dyn Error>> {
    let one = uniform(1, 1, left)?;
    let other = uniform(1, 1, right)?;
    Ok(compare(&one, &other, &Thresholds::default()).max_delta_e)
}

/// What share of `frame` sits further than `tolerance` from `color`.
pub fn share_beyond(
    frame: &Rgba8Image,
    color: [u8; 3],
    tolerance: f64,
) -> Result<f64, Box<dyn Error>> {
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
