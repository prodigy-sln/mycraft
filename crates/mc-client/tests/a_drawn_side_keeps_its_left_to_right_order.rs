//! Which way *round* a grass side is drawn — the half turf-above-dirt cannot see.
//!
//! # Why a second drawn reading exists
//!
//! `a_drawn_side_shows_turf_above_dirt.rs` asks which edge the turf sits on, and
//! **a pure lateral reversal preserves that answer**: mirror a side face
//! left-to-right and the turf is still along the top. So the vertical half of a
//! face's orientation has a witness and the horizontal half has none — which is
//! the same shape of gap that let the original defect through, one axis at a time.
//!
//! # What it reads, and what makes it about laterality alone
//!
//! Every one of the face's texel cells is sampled at its own centre, and each
//! column is reduced to **how many of its texels are turf**. That number is
//! unchanged by a vertical flip, so this reading is orthogonal to its sibling
//! rather than a stronger version of it: the two together pin both axes, and
//! neither pins the other's.
//!
//! The column vector is then compared against the baked image's own — and against
//! that image reversed. **The image is the right oracle for this question and no
//! golden is**: the claim is that the renderer draws the image in the image's own
//! left-to-right order, and the goldens were minted from the broken output.
//! `the_baked_faces_are_the_model_seen_from_outside.rs` is what ties the image
//! itself to the hand-written model, so the chain runs model → image → frame with
//! no step judged against itself.
//!
//! # Where "left to right" comes from
//!
//! From `support::model::image_basis` — the same right-handed `(right, up, normal)`
//! triple the bake half walks. The eye stands square in front of the face with
//! world up as screen up, so the image's `right` is the screen's right, and each
//! texel cell's centre is a world position projected through the landmark probe's
//! own projection. The projection says *where to look*; the palette and the image
//! say *what must be there*.
//!
//! # The expectation for the second number is derived, not written down
//!
//! A palindromic column vector cannot tell a face from its mirror, and no reading
//! whatever could. So how many columns must disagree with the reversed vector is
//! measured **from the image alone**, and a fixture premise refuses a face whose
//! vector is palindromic rather than passing vacuously over it.

mod support;

use std::error::Error;
use std::sync::Arc;

use glam::IVec3;
use mc_core::content::TEXTURE_EDGE;
use mc_core::id::TextureKey;
use mc_render::camera::CameraView;
use mc_render::texture::supplied::SuppliedTexels;
use mc_testkit::frame::Rgba8Image;
use mc_world::mesh::Facing;

use support::art::{built_texels, declared_material_colors, drawn_texels};
use support::faces::{named, outward_of, silhouette_of, square_in_front_of, where_on_frame};
use support::model::{face_showing, image_basis};
use support::probe::pixel_color;
use support::swatch::require;
use support::{PreparedScene, TestResult, content_root, frames, prepare_scene};

/// The materials the model's turf courses are made of.
const TURF: [&str; 5] = [
    "grass",
    "grass_dark",
    "grass_deep",
    "grass_light",
    "grass_pale",
];

/// How many texels a block texture is on a side.
const EDGE: u32 = TEXTURE_EDGE;

/// How a drawn face's columns stand against the image's own.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Order {
    /// Which facing, so a failure names it.
    facing: &'static str,
    /// Columns whose turf count differs from the image's column of the same index.
    unlike_the_image: usize,
    /// Columns whose turf count differs from the image read right to left.
    unlike_it_reversed: usize,
}

#[test]
fn every_drawn_grass_side_runs_the_way_its_image_does() -> TestResult {
    let prepared = prepare_scene()?;
    let turf = declared_material_colors(&TURF)?;
    let texels = built_texels(&content_root()?)?;
    let exposed = support::faces::exposed_side_faces(&prepared)?;
    let Some(context) = frames::device()? else {
        return Ok(());
    };

    let (read, must_read) = read_every_face(&context, &prepared, &exposed, (&turf, &texels))?;

    assert_eq!(
        read, must_read,
        "a grass side is drawn in its image's own left-to-right order. The first number counts \
         columns whose turf count differs from the image's; the second counts those differing from \
         the image reversed, and it is the discrimination proof — a palindromic column vector could \
         not tell a face from its mirror, so what that number must be is measured from the image \
         alone rather than written down. Per-column turf counts survive a vertical flip untouched, \
         which is what keeps this reading about laterality and leaves the other axis to \
         `a_drawn_side_shows_turf_above_dirt.rs`"
    );
    Ok(())
}

/// Reads all four faces, one capture each, and what each must read as.
fn read_every_face(
    context: &mc_testkit::frame::gpu::CaptureContext,
    prepared: &PreparedScene,
    exposed: &[(Facing, IVec3)],
    sources: (&[[u8; 3]], &SuppliedTexels),
) -> Result<(Vec<Order>, Vec<Order>), Box<dyn Error>> {
    let scene = Arc::new(prepared.scene.clone());
    let mut read = Vec::new();
    let mut must_read = Vec::new();
    for (facing, at) in exposed {
        let camera = square_in_front_of(*facing, *at);
        let frame = support::faces::captured(context, prepared, &scene, (camera, named(*facing)))?;
        let (found, wanted) = order_of((*facing, *at), &camera, &frame, sources)?;
        read.push(found);
        must_read.push(wanted);
    }
    Ok((read, must_read))
}

/// How the columns of the face on `facing` stand in `frame`, and how they must.
///
/// # Errors
///
/// Returns an error when the facing is none of the six, when the world offers no
/// texture key for it, when the face projects too small to sample a texel from, or
/// when the image's own columns are palindromic — the last of which is a reading
/// that cannot discriminate rather than a picture that is right.
fn order_of(
    face: (Facing, IVec3),
    camera: &CameraView,
    frame: &Rgba8Image,
    sources: (&[[u8; 3]], &SuppliedTexels),
) -> Result<(Order, Order), Box<dyn Error>> {
    let (facing, at) = face;
    let (turf, texels) = sources;
    support::faces::require_large_enough(silhouette_of(facing, at, camera)?, EDGE * 3)?;
    let baked = image_columns(facing, texels, turf)?;
    let reversed: Vec<usize> = baked.iter().rev().copied().collect();
    require(
        baked != reversed,
        format!(
            "the image drawn on the {} face has palindromic turf counts across its columns, so no \
             reading whatever could tell it from its own mirror and this one would pass over a \
             reversed face in silence: {baked:?}",
            named(facing)
        ),
    )?;
    let drawn = drawn_columns((facing, at), camera, frame, turf)?;
    Ok((
        Order {
            facing: named(facing),
            unlike_the_image: unlike(&drawn, &baked),
            unlike_it_reversed: unlike(&drawn, &reversed),
        },
        Order {
            facing: named(facing),
            unlike_the_image: 0,
            unlike_it_reversed: unlike(&baked, &reversed),
        },
    ))
}

/// How many turf texels each column of the *drawn* face holds.
///
/// Each texel cell is sampled at its own centre, a world position walked out along
/// the face's own image basis, so a sample never lands on the cell next door: the
/// face spans 415 pixels of a 720-pixel frame at the declared distance, which is
/// 26 pixels to a texel.
fn drawn_columns(
    face: (Facing, IVec3),
    camera: &CameraView,
    frame: &Rgba8Image,
    turf: &[[u8; 3]],
) -> Result<Vec<usize>, Box<dyn Error>> {
    let (facing, at) = face;
    let showing = face_showing(outward_of(facing)).ok_or_else(|| {
        format!(
            "the {} facing is none of the six a block has, so it has no image basis",
            named(facing)
        )
    })?;
    let (right, down) = image_basis(showing);
    let corner = top_left_of(facing, at, (right, down));
    let mut columns = Vec::with_capacity(EDGE as usize);
    for column in 0..EDGE {
        columns.push(turf_down_column(
            (corner, (right, column), down),
            camera,
            frame,
            turf,
        )?);
    }
    Ok(columns)
}

/// How many texels of one drawn column are a turf colour.
fn turf_down_column(
    walk: ([f32; 3], ([i32; 3], u32), [i32; 3]),
    camera: &CameraView,
    frame: &Rgba8Image,
    turf: &[[u8; 3]],
) -> Result<usize, Box<dyn Error>> {
    let (corner, across, down) = walk;
    let mut found = 0;
    for row in 0..EDGE {
        let (x, y) = where_on_frame(walked(corner, across, (down, row)), camera)?;
        if turf.contains(&pixel_color(frame, (x, y))?) {
            found += 1;
        }
    }
    Ok(found)
}

/// How many turf texels each column of the *baked* image holds.
///
/// # Errors
///
/// Returns an error when the facing has no texture key here, or when the key does
/// not parse.
fn image_columns(
    facing: Facing,
    texels: &SuppliedTexels,
    turf: &[[u8; 3]],
) -> Result<Vec<usize>, Box<dyn Error>> {
    let key = TextureKey::parse(key_drawn_on(facing)?)?;
    let held = drawn_texels(&key, texels);
    let edge = EDGE as usize;
    Ok((0..edge)
        .map(|column| {
            (0..edge)
                .filter_map(|row| held.get(row * edge + column))
                .filter(|[red, green, blue, _]| turf.contains(&[*red, *green, *blue]))
                .count()
        })
        .collect())
}

/// The key the grass block declares for `facing`.
///
/// Spelled from the facing rather than read out of the declaration, because what
/// the declaration says is `the_shipped_blocks_draw_their_baked_art.rs`'s subject
/// and not this one's — here it is only the address of the image whose order is in
/// question.
fn key_drawn_on(facing: Facing) -> Result<&'static str, Box<dyn Error>> {
    match facing {
        Facing::NegZ => Ok("base:grass_side_north"),
        Facing::PosZ => Ok("base:grass_side_south"),
        Facing::PosX => Ok("base:grass_side_east"),
        Facing::NegX => Ok("base:grass_side_west"),
        _ => Err(format!(
            "the {} face draws no grass side, so this reading has no image to compare against",
            named(facing)
        )
        .into()),
    }
}

/// The world position of the corner of the face's image at column 0, row 0.
///
/// The face is the unit square of the block at `at` on its outward side, and the
/// image's first texel sits at whichever of its corners the two walks start from.
fn top_left_of(facing: Facing, at: IVec3, basis: ([i32; 3], [i32; 3])) -> [f32; 3] {
    let (right, down) = basis;
    let normal = outward_of(facing);
    let block = [at.x as f32, at.y as f32, at.z as f32];
    [0, 1, 2].map(|axis| {
        let outward = if on(normal, axis) == 1 { 1.0 } else { 0.0 };
        let started = |direction: i32| if direction == -1 { 1.0 } else { 0.0 };
        along(block, axis) + outward + started(on(right, axis)) + started(on(down, axis))
    })
}

/// One component of an axis-aligned direction.
fn on(vector: [i32; 3], axis: usize) -> i32 {
    vector.get(axis).copied().unwrap_or(0)
}

/// One component of a world position.
fn along(position: [f32; 3], axis: usize) -> f32 {
    position.get(axis).copied().unwrap_or(0.0)
}

/// A world position `column` and `row` texel cell centres along the face's basis.
fn walked(from: [f32; 3], across: ([i32; 3], u32), down: ([i32; 3], u32)) -> [f32; 3] {
    let (right, column) = across;
    let (downward, row) = down;
    let step = |steps: u32| (steps as f32 + 0.5) / EDGE as f32;
    [0, 1, 2].map(|axis| {
        along(from, axis)
            + on(right, axis) as f32 * step(column)
            + on(downward, axis) as f32 * step(row)
    })
}

/// How many columns two vectors of counts disagree on.
fn unlike(found: &[usize], wanted: &[usize]) -> usize {
    found
        .iter()
        .zip(wanted)
        .filter(|(left, right)| left != right)
        .count()
}
