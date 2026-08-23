//! The one judgement of this spec's picture that does not come from a committed
//! image.
//!
//! # Why a golden is not enough, said once and plainly
//!
//! The golden set is re-shot in this phase. A golden minted from a renderer
//! nobody checked is a photograph of whatever that renderer did that day, and it
//! then passes forever — so the re-shoot is the moment this spec is least able
//! to catch itself. This is the reading that can: the pixel comes from a device
//! drawing the shipped world, and the colour it is judged against comes from the
//! **built PNG**, decoded by the client's own reader and never by the draw.
//!
//! **If the two ever come to share a path the pair collapses into one
//! snapshot.** No mutation detects that. It is held by whoever reads this file,
//! which is why it is the first thing the file says.
//!
//! # Which pixel, and how it is chosen without looking at the frame
//!
//! A ray is marched through the world's own voxels from the declared pose,
//! through each pixel of the declared sample grid in order, and the first pixel
//! whose ray meets a `base:grass` voxel through its **upward** face — with its
//! four neighbours meeting the same face of the same voxel — is the one read.
//! Nothing about that choice consults the picture. The neighbour agreement is
//! what keeps the pixel off a silhouette: a top face at this pose is between
//! five and sixteen pixels tall, so a pixel with four agreeing neighbours is one
//! well inside a face rather than one on its edge.
//!
//! # Where the tolerance comes from, in both directions
//!
//! Measured over the built `base:grass_top` image: **every one of its texels sits
//! within ΔE 9.09 of that image's linear-light mean**, so a nearest-magnified
//! face shows a colour at most that far off and a minified one converges towards
//! the mean itself. The nearest *wrong* answer is a grass side at ΔE 38.00, the
//! generated texture for `base:grass` at 42.35 and `base:dirt` at 48.49. So the
//! tolerance sits anywhere in (9.09 + the sRGB round trip, 38.00) and it is 12 —
//! not loosened until green, and 26 ΔE clear of every other thing this pixel
//! could be showing.
//!
//! **What this reading cannot tell you**, stated because an instrument whose
//! limit is unwritten gets read as stronger than it is: it cannot tell one
//! grass-top texel from another, and it would not notice the image being
//! reflected, rotated or shuffled. What it tells apart is grass-top art from
//! every other thing that pixel could hold — the dirt underneath it, the sides
//! beside it, the stone pillar, the sky, and the generated stand-in this spec
//! replaces.

mod support;

use std::error::Error;
use std::sync::Arc;

use glam::IVec3;
use mc_core::id::{BlockName, TextureKey};
use mc_render::camera::camera_view;
use mc_sim::camera::CameraPose;
use mc_testkit::frame::Rgba8Image;
use mc_world::mesh::Facing;
use mc_world::section::Contents;

use support::art::{drawn_texels, linear_mean, means_agree};
use support::frames::{CAPTURE_SIZE, ReplayFrame};
use support::oracle::{self, Voxels, first_drawn_face};
use support::probe::{distance, pixel_color};
use support::swatch::require;
use support::{TestResult, prepare_scene};

/// The tick the observed snapshot is labelled with. The scene is static and the
/// pose is declared rather than reached, so nothing here depends on it.
const TICK: u32 = 0;

/// The declared observation pose, from `spec.md`, written out rather than taken
/// from the simulation — for the reason every other figure derived for it is.
const EYE: [f32; 3] = [44.0, 56.0, 44.0];
const LOOK_AT: [f32; 3] = [12.0, 52.0, 20.0];

/// The block whose upward face this is about, and the key that face draws.
const GRASS: &str = "base:grass";
const GRASS_TOP: &str = "base:grass_top";

/// How far the pixel may sit from the image's own mean, in ΔE.
///
/// Derived from both directions in this module's header: above the ΔE 9.09 that
/// is the furthest any texel of the image stands from its mean, and far below
/// the ΔE 38.00 that separates it from the nearest thing it could be confused
/// with.
const SHOWS_THE_IMAGE: f64 = 12.0;

/// How many pixels either side of the chosen one have to be looking at the same
/// face of the same voxel.
const NEIGHBOURS_AGREE_WITHIN: u32 = 1;

#[test]
fn the_pixel_the_declared_camera_puts_a_grass_top_on_shows_that_images_own_colour() -> TestResult {
    let prepared = prepare_scene()?;
    let key = TextureKey::parse(GRASS_TOP)?;
    let mean = linear_mean(&drawn_texels(&key, &prepared.texels));
    require_the_set_covers_it(&key, &prepared)?;

    let voxels = Voxels {
        world: &prepared.world,
        registry: prepared.registry.as_ref(),
    };
    let looking_at = a_grass_top_pixel(&declared_pose(), &voxels)?;
    let Some(frame) = observed_frame(prepared)? else {
        return Ok(());
    };

    let shown = pixel_color(&frame, looking_at)?;
    let stands = distance(shown, mean)?;
    assert!(
        stands <= SHOWS_THE_IMAGE,
        "the world's own voxels say pixel {looking_at:?} is the upward face of a grass block, and \
         the built PNG for `{GRASS_TOP}` says what colour that is. The frame drew {shown:?}, ΔE \
         {stands:.2} from the image's mean {mean:?}, against the ΔE {SHOWS_THE_IMAGE} every texel \
         of that image sits inside. The nearest thing it could be instead is a grass side at ΔE \
         38, so this is not a near miss whatever it is"
    );
    Ok(())
}

/// Fails unless the built set covers `key`, and checks the oracle's own
/// arithmetic while it is there.
///
/// Two things at once, and both are premises rather than behaviours. A set
/// covering nothing for this key would make the colour below the generated
/// stand-in, so the assertion would be about the fallback; and the two means
/// standing further apart than any shipped texture does would mean the transfer
/// function this file's mean is computed through is running the wrong way.
fn require_the_set_covers_it(
    key: &TextureKey,
    prepared: &support::PreparedScene,
) -> Result<(), Box<dyn Error>> {
    let apart = means_agree(key, &prepared.texels)?;
    require(
        prepared.texels.covering(key).is_some(),
        format!(
            "this reading judges a pixel against the built image for `{GRASS_TOP}`, and the set \
             offers none — so the colour below would be the generated stand-in and the assertion \
             would be about the fallback rather than about art reaching the screen. The two means \
             stood ΔE {apart:.2} apart"
        ),
    )
}

/// The declared pose, as the marching oracle takes one.
fn declared_pose() -> CameraPose {
    CameraPose {
        eye: EYE,
        target: LOOK_AT,
    }
}

/// The first pixel of the declared sample grid whose ray, and every neighbour's,
/// meets the upward face of one `base:grass` voxel.
///
/// **Chosen from the world and never from the picture.** The grid is the
/// oracle's own declared one, walked in its own order, so the answer is the same
/// on every run and on every machine.
///
/// # Errors
///
/// Returns an error when no pixel of the grid qualifies — a pose that shows no
/// grass top, or shows only slivers of them, is a fixture this reading cannot be
/// made from, and it says so rather than reading whatever pixel came closest.
fn a_grass_top_pixel(
    camera: &CameraPose,
    voxels: &Voxels<'_>,
) -> Result<(u32, u32), Box<dyn Error>> {
    let grass = BlockName::parse(GRASS)?;
    for pixel in oracle::sample_pixels() {
        let Some(hit) = grass_top_at(camera, pixel, voxels, &grass)? else {
            continue;
        };
        if neighbours_of(pixel)
            .into_iter()
            .map(|near| grass_top_at(camera, near, voxels, &grass))
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .all(|near| near == Some(hit))
        {
            return Ok(pixel);
        }
    }
    Err(format!(
        "no pixel of the declared {} sample grid looks at the upward face of a grass block with \
         all {NEIGHBOURS_AGREE_WITHIN}-pixel neighbours agreeing. A pose showing only slivers of \
         grass tops cannot carry this reading, and reading whichever pixel came closest would be \
         reading the frame to decide what the frame should hold",
        oracle::SAMPLE_COUNT
    )
    .into())
}

/// The voxel `pixel`'s ray meets through an upward face, when that voxel holds
/// `grass`.
fn grass_top_at(
    camera: &CameraPose,
    pixel: (u32, u32),
    voxels: &Voxels<'_>,
    grass: &BlockName,
) -> Result<Option<IVec3>, Box<dyn Error>> {
    let Some((voxel, entered)) = first_drawn_face(camera, CAPTURE_SIZE, pixel, voxels)? else {
        return Ok(None);
    };
    if entered != Facing::PosY {
        return Ok(None);
    }
    let (Ok(x), Ok(y), Ok(z)) = (
        u32::try_from(voxel.x),
        u32::try_from(voxel.y),
        u32::try_from(voxel.z),
    ) else {
        return Ok(None);
    };
    Ok(match voxels.world.block_at(x, y, z) {
        Some(Contents::Holds(name)) if name == grass => Some(voxel),
        _ => None,
    })
}

/// The four pixels one step from `pixel` on each axis.
fn neighbours_of(pixel: (u32, u32)) -> Vec<(u32, u32)> {
    let (x, y) = pixel;
    let step = NEIGHBOURS_AGREE_WITHIN;
    vec![
        (x.saturating_sub(step), y),
        (x + step, y),
        (x, y.saturating_sub(step)),
        (x, y + step),
    ]
}

/// The replay's scene from the declared pose at the declared capture size, or
/// `None` when the opt-in permitted the absence of a device.
fn observed_frame(prepared: support::PreparedScene) -> Result<Option<Rgba8Image>, Box<dyn Error>> {
    let Some(context) = support::frames::device()? else {
        return Ok(None);
    };
    let mut renderer = support::frames::prepared_renderer(&context, &prepared)?;
    let scene = Arc::new(prepared.scene);
    let snapshot = support::frames::snapshot(TICK, camera_view(EYE, LOOK_AT), &scene);
    let request = support::frames::request(&context, "grass-top-is-its-image")?;

    let mut frame = ReplayFrame {
        context: &context,
        renderer: &mut renderer,
        snapshot: &snapshot,
    };
    Ok(Some(frame.capture(&request)?))
}
