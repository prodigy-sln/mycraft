//! Which way up a grass side is *drawn* — the half no colour statistic can see.
//!
//! # Why this exists
//!
//! The first mint of the golden set drew the grass sides **rotated**: turf along
//! the bottom edge on the Z-facing pair and running vertically on the X-facing
//! pair. **Not one of 1366 tests could see it**, and it was found by the project
//! owner looking at the picture. FR-8.1-S3 asks the four sides to be pairwise
//! *unequal*, which a rotation preserves; FR-8.1-S5 judges the *top* face, which
//! was correct; and every remaining colour reading in this spec compares means,
//! histograms or set membership, all of which are invariant under rotation,
//! reflection and permutation. The goldens were minted from the broken output and
//! so enshrined it.
//!
//! **So no golden and no committed image is used as an oracle here.** What a
//! pixel is judged against is `content/base/materials/` — TOML a person wrote —
//! and what makes the reading about *geometry* is that it samples two bands
//! chosen by position and asks a different question of each.
//!
//! # It reads all four facings where the scenario asks for one
//!
//! FR-8.1-S7 asks for one side face of one grass block. All four are read,
//! because the defect was **two different rotations**: the Z-facing pair was
//! upside down and the X-facing pair was turned a quarter. One facing would
//! witness one of those and leave the other with no instrument at all — and a
//! reading that catches half a defect is how the other half survives a fix.
//!
//! # Every band is derived, and nothing here is a committed pixel
//!
//! The camera is placed square in front of one exposed face at a stated distance,
//! so the face's silhouette is found by projecting its own four corners through
//! the same projection the landmark probe uses. Finding a pixel that way is this
//! suite's established idiom and is not a shared expectation: the projection says
//! *where to look*, and the palette says *what must be there*. The bands are then
//! the middle third of the topmost and bottommost texel rows of that silhouette —
//! a fraction of a measured extent, never a pixel somebody wrote down.
//!
//! # What this half cannot see
//!
//! Two things. Whether the *image* carries turf at its top, which is
//! `the_baked_faces_are_the_model_seen_from_outside.rs`; and whether the face is
//! drawn the right way **round**, since turf above dirt is preserved by a pure
//! lateral reversal — that is `a_drawn_side_keeps_its_left_to_right_order.rs`.
//! When this was written the bake was correct and this was red on all four.

mod support;

use std::error::Error;
use std::sync::Arc;

use glam::IVec3;
use mc_render::camera::CameraView;
use mc_testkit::frame::Rgba8Image;
use mc_world::mesh::Facing;

use support::art::declared_material_colors;
use support::faces::{Exposed, SIDES, named, silhouette_of, square_in_front_of};
use support::hud_frames::Rect;
use support::swatch::swatch_reading;
use support::{PreparedScene, TestResult, frames, prepare_scene};

/// The materials the model's turf courses are made of, and the ones its body is.
const TURF: [&str; 5] = [
    "grass",
    "grass_dark",
    "grass_deep",
    "grass_light",
    "grass_pale",
];
const BODY: [&str; 3] = ["dirt", "dirt_dark", "dirt_light"];

/// How many texels a block texture is on a side, as the divisor that turns a
/// silhouette into rows.
const TEXEL_ROWS: u32 = mc_core::content::TEXTURE_EDGE;

/// How a drawn face's two bands stand.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Bands {
    /// Which facing, so a failure names it.
    facing: &'static str,
    /// Pixels of the top band that are no turf colour.
    top_not_turf: u64,
    /// Pixels of the bottom band that are no body colour.
    bottom_not_body: u64,
    /// Whether both bands were read whole.
    both_bands_read: bool,
}

#[test]
fn every_drawn_grass_side_shows_turf_high_on_it_and_dirt_low() -> TestResult {
    let prepared = prepare_scene()?;
    let turf = declared_material_colors(&TURF)?;
    let body = declared_material_colors(&BODY)?;
    let exposed = support::faces::exposed_side_faces(&prepared)?;
    let Some(context) = frames::device()? else {
        return Ok(());
    };

    let read = read_every_face(&context, &prepared, &exposed, (&turf, &body))?;

    assert_eq!(
        read,
        SIDES.map(|facing| upright(named(facing))).to_vec(),
        "a grass side carries turf along its top edge over a body of dirt, and that is a claim \
         about *where* colours sit which no mean, histogram or membership test can make. All four \
         facings are read because the defect this closes was two different rotations at once — the \
         Z pair upside down and the X pair turned a quarter — and one facing would have witnessed \
         one of them. The bands are the middle third of the topmost and bottommost texel rows of \
         each face's own projected silhouette, so nothing here is a pixel anybody wrote down"
    );
    Ok(())
}

/// What an upright face reads as.
fn upright(facing: &'static str) -> Bands {
    Bands {
        facing,
        top_not_turf: 0,
        bottom_not_body: 0,
        both_bands_read: true,
    }
}

/// Reads all four faces, one capture each.
fn read_every_face(
    context: &mc_testkit::frame::gpu::CaptureContext,
    prepared: &PreparedScene,
    exposed: &Exposed,
    palettes: (&[[u8; 3]], &[[u8; 3]]),
) -> Result<Vec<Bands>, Box<dyn Error>> {
    let scene = Arc::new(prepared.scene.clone());
    let mut read = Vec::new();
    for (facing, at) in exposed {
        let camera = square_in_front_of(*facing, *at);
        let frame = support::faces::captured(context, prepared, &scene, (camera, named(*facing)))?;
        read.push(bands_of(*facing, *at, &frame, palettes)?);
    }
    Ok(read)
}

/// How the two bands of the face on `facing` at `at` stand in `frame`.
fn bands_of(
    facing: Facing,
    at: IVec3,
    frame: &Rgba8Image,
    palettes: (&[[u8; 3]], &[[u8; 3]]),
) -> Result<Bands, Box<dyn Error>> {
    let (turf, body) = palettes;
    let camera = square_in_front_of(facing, at);
    let (top, bottom) = the_two_bands(facing, at, &camera)?;
    let high = swatch_reading(frame, top, turf)?;
    let low = swatch_reading(frame, bottom, body)?;
    Ok(Bands {
        facing: named(facing),
        top_not_turf: high.strayed,
        bottom_not_body: low.strayed,
        both_bands_read: high.considered == top.area() && low.considered == bottom.area(),
    })
}

/// The middle third of the topmost and bottommost texel rows of the face's own
/// projected silhouette.
///
/// # Errors
///
/// Returns an error when a corner does not project in front of the camera, or when
/// the silhouette is too small to hold a band — both of which are a pose this
/// reading cannot be taken from rather than a picture that is wrong.
fn the_two_bands(
    facing: Facing,
    at: IVec3,
    camera: &CameraView,
) -> Result<(Rect, Rect), Box<dyn Error>> {
    let seen = silhouette_of(facing, at, camera)?;
    support::faces::require_large_enough(seen, TEXEL_ROWS * 3)?;
    let row = (seen.bottom - seen.top).div_euclid(TEXEL_ROWS);
    let third = row.div_euclid(3);
    let inset = (seen.right - seen.left).div_euclid(3);
    let band = |y: u32| Rect {
        x: seen.left + inset,
        y,
        width: inset,
        height: third,
    };
    Ok((band(seen.top + third), band(seen.bottom - row + third)))
}
