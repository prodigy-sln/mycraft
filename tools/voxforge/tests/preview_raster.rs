//! What a view actually looks at, and what shape the image it produces is.
//!
//! The occlusion fixtures are **solid**, and that is load-bearing rather than
//! convenient: a hollow model lets a ray reach the far half, so a first-hit
//! assertion would pass against a renderer that returns the *last* hit. No
//! assertion here can enforce that shape — it is held by
//! [`common::preview::halved_on_x`] and by whoever reads it.
//!
//! Every expected pixel figure is arithmetic over the fixture's own declared
//! extent and the eight pixels per voxel asked for, never a number read off a
//! rendered image. A solid `4 × 4 × 4` seen along `x` is 4 voxels across by 4
//! down, so 32 × 32 pixels, so 1024 of them.

mod common;

use std::error::Error;
use std::num::NonZeroU32;

use common::preview::{
    Coverage, EIGHT_PER_VOXEL, MadeOf, Paint, coverage, halved_on_x, halved_on_y, made_of, painted,
    paints, solid,
};
use common::{FIXTURE_FILE, TestResult, all_named, assembled, unnamed};
use voxforge::fault::{Fault, Origin};
use voxforge::render::{View, pixels_per_voxel, render, view_named};
use voxforge::volume::StateSelection;

/// A solid model whose `−x` half is blue and whose `+x` half is red.
///
/// `[4, 6, 2]`: three different extents, so a transposed raster is a differently
/// shaped image rather than a plausible one, and an even width so the two halves
/// are the same size.
fn halved_across() -> String {
    halved_on_x((4, 6, 2), Paint::Blue, Paint::Red)
}

/// A solid model whose `−y` half is blue and whose `+y` half is red.
fn halved_upward() -> String {
    halved_on_y((4, 6, 2), Paint::Blue, Paint::Red)
}

/// Two voxels at opposite corners of a `[3, 3, 1]` box, so that most of the
/// image is background and the drawn part is derivable: two voxels, 8 × 8 pixels
/// each, is 128 pixels of the 576 a 24 × 24 image holds.
fn two_corners() -> String {
    painted((3, 3, 1), &|x, y, _| match (x, y) {
        (0, 0) | (2, 2) => Some(Paint::Red),
        _ => None,
    })
}

/// A `[4, 4, 4]` box holding a 2 × 2 × 2 blue block at `(0..1, 2..3, 2..3)` and
/// one red voxel at `(3, 0, 0)`, with nothing else.
///
/// The red voxel sits **exactly behind** the block along `iso-fl`'s ray: its
/// centre is `(3.5, 0.5, 0.5)` and the block's is `(1, 3, 3)`, a difference of
/// `2.5 · (1, −1, −1)`, which is the view direction. Two shapes offset along the
/// view direction project onto the same place, and the block is twice the red
/// voxel's size on every axis, so its silhouette contains the red one's with a
/// margin rather than coinciding with it — a coincident pair would leave the
/// answer to a rounding decision at the silhouette edge.
///
/// The block is nearer: depth along the ray is `p · d`, which is `−4.5` for the
/// block's centre against `+1.5` for the red voxel's, and rays travel along `+d`.
fn one_behind_the_other() -> String {
    painted((4, 4, 4), &|x, y, z| match (x, y, z) {
        (3, 0, 0) => Some(Paint::Red),
        (0 | 1, 2 | 3, 2 | 3) => Some(Paint::Blue),
        _ => None,
    })
}

/// The refusal naming a view `text` earns.
///
/// # Errors
///
/// Returns an error when the name was accepted — a scenario about a refusal
/// asserts nothing if the name resolved.
fn view_refusal(text: &str) -> Result<Fault, Box<dyn Error>> {
    match view_named(text, Origin::new(FIXTURE_FILE)) {
        Ok(view) => Err(format!(
            "`{text}` must be refused, but resolved to the {} view",
            view.as_str()
        )
        .into()),
        Err(fault) => Ok(fault),
    }
}

/// The refusal asking for `requested` pixels per voxel earns.
///
/// # Errors
///
/// Returns an error when the figure was accepted.
fn scale_refusal(requested: u32) -> Result<Fault, Box<dyn Error>> {
    match pixels_per_voxel(requested, Origin::new(FIXTURE_FILE)) {
        Ok(scale) => Err(format!(
            "{requested} pixels per voxel must be refused, but resolved to {scale}"
        )
        .into()),
        Err(fault) => Ok(fault),
    }
}

#[test]
fn a_view_along_minus_x_shows_only_the_material_on_the_models_plus_x_side() -> TestResult {
    let volume = assembled(&halved_across(), &StateSelection::default())?;

    assert_eq!(
        made_of(
            &render(&volume, &paints()?, View::Right, EIGHT_PER_VOXEL),
            Paint::Red
        ),
        MadeOf::OnlyThePaint,
        "the model is solid, so every ray from the right stops on the red half and none of the blue one behind it is ever reached"
    );
    Ok(())
}

#[test]
fn a_view_along_plus_x_shows_only_the_material_on_the_models_minus_x_side() -> TestResult {
    let volume = assembled(&halved_across(), &StateSelection::default())?;

    assert_eq!(
        made_of(
            &render(&volume, &paints()?, View::Left, EIGHT_PER_VOXEL),
            Paint::Blue
        ),
        MadeOf::OnlyThePaint,
        "the same model from the opposite side shows the other half, which is what makes the first answer a first hit rather than a coincidence"
    );
    Ok(())
}

#[test]
fn a_view_from_above_shows_only_the_material_on_the_models_upper_half() -> TestResult {
    let volume = assembled(&halved_upward(), &StateSelection::default())?;

    assert_eq!(
        made_of(
            &render(&volume, &paints()?, View::Top, EIGHT_PER_VOXEL),
            Paint::Red
        ),
        MadeOf::OnlyThePaint,
        "a plan view stops on the topmost voxel of every column, so nothing of the lower half reaches the image"
    );
    Ok(())
}

#[test]
fn a_solid_four_voxel_cube_covers_exactly_its_silhouette_at_eight_pixels_per_voxel() -> TestResult {
    // 4 voxels across the view by 4 down, at 8 pixels each: a 32 × 32 image
    // wholly covered, so 1024 drawn pixels. Counted off the declared extent,
    // never off a render.
    let volume = assembled(&solid((4, 4, 4), Paint::Red), &StateSelection::default())?;

    assert_eq!(
        coverage(&render(&volume, &paints()?, View::Right, EIGHT_PER_VOXEL)),
        Coverage::Drawn(1024),
        "an orthographic view of a solid cube is a filled square of its silhouette, with nothing over-covered and nothing missed"
    );
    Ok(())
}

#[test]
fn a_view_name_that_is_not_one_of_the_ten_is_refused_listing_every_one_that_is() -> TestResult {
    let fault = view_refusal("oblique")?;
    let expected: Vec<&str> = ["oblique"]
        .into_iter()
        .chain(View::ALL.iter().map(|view| view.as_str()))
        .collect();

    assert_eq!(
        (fault.field.as_deref(), unnamed(&fault, &expected)),
        (Some("view"), all_named()),
        "the reader is an agent repairing its own command line, and the ten names it may write are the whole of the repair; cause was: {}",
        fault.cause
    );
    Ok(())
}

#[test]
fn every_pixel_no_voxel_lands_on_is_left_fully_transparent() -> TestResult {
    // Two voxels of a 3 × 3 × 1 box, 8 × 8 pixels each: 128 of the image's 576
    // pixels are drawn and the remaining 448 must be background. An image that
    // painted its own backdrop, or one that drew nothing at all, misses this
    // figure in opposite directions.
    let volume = assembled(&two_corners(), &StateSelection::default())?;

    assert_eq!(
        coverage(&render(&volume, &paints()?, View::Front, EIGHT_PER_VOXEL)),
        Coverage::Drawn(128),
        "one opaque sample per pixel leaves no half-covered pixel anywhere, and everything no voxel reached stays at alpha 0"
    );
    Ok(())
}

#[test]
fn a_four_by_eight_by_two_model_renders_thirty_two_pixels_wide_and_sixty_four_tall() -> TestResult {
    // From the front the image spans x across and y down: 4 × 8 voxels at 8
    // pixels each. The three extents differ, so a transposed raster is 64 × 32
    // and fails rather than looking plausible.
    let volume = assembled(&solid((4, 8, 2), Paint::Red), &StateSelection::default())?;
    let preview = render(&volume, &paints()?, View::Front, EIGHT_PER_VOXEL);

    assert_eq!(
        (preview.width(), preview.height()),
        (32, 64),
        "the image is the model's own box projected onto the view's axes, at the scale asked for"
    );
    Ok(())
}

#[test]
fn a_pixels_per_voxel_of_zero_is_refused_naming_the_value_and_the_minimum() -> TestResult {
    let fault = scale_refusal(0)?;

    assert_eq!(
        (
            fault.field.as_deref(),
            unnamed(&fault, &["0", "minimum is 1"])
        ),
        (Some("pixels-per-voxel"), all_named()),
        "an image of no pixels is not a smaller preview, it is no preview; cause was: {}",
        fault.cause
    );
    Ok(())
}

/// Additional coverage, on the one part of the march no scenario reaches.
///
/// The three occlusion scenarios above are all **axis** views, where a ray runs
/// down one grid axis and "first hit" is the near end of a straight line of
/// cells. A corner view is the other case entirely: the ray crosses cells
/// diagonally, and the order it visits them in is where a depth comparison
/// reversed, or a march that steps past a cell, actually lives. FR-5.2-S2 cannot
/// see any of that — its two voxels deliberately share one column *so that* no
/// depth term can reorder them, which is exactly what leaves the depth term
/// ungraded.
///
/// A reversed comparison shows red here; a march that tunnelled through the
/// block shows red; a march that lost the red voxel entirely still shows blue,
/// which is why the verdict distinguishes "only this paint" from "nothing drawn".
#[test]
fn a_corner_view_shows_the_nearer_of_two_voxels_one_directly_behind_the_other() -> TestResult {
    let volume = assembled(&one_behind_the_other(), &StateSelection::default())?;

    assert_eq!(
        made_of(
            &render(&volume, &paints()?, View::IsoFl, EIGHT_PER_VOXEL),
            Paint::Blue
        ),
        MadeOf::OnlyThePaint,
        "the block stands between the corner and the red voxel, and a ray march stops where it first meets something rather than where it last does"
    );
    Ok(())
}

/// Additional coverage, beyond the eight scenarios above.
///
/// Every other test in this phase reaches the renderer with a [`View`] value in
/// hand, so none of them calls [`view_named`] at all, and the one that does asks
/// it to refuse. A reader that refused every name there is would leave all
/// twenty of the phase's scenarios green while `preview --view front` reported
/// that `front` is not a view. Asserting the round trip also grades the two
/// tables against each other: a name spelled one way by [`View::as_str`] and
/// another by the reader is a defect neither table shows on its own.
#[test]
fn every_canonical_view_answers_to_the_name_it_reports() -> TestResult {
    let read_back: Vec<Option<View>> = View::ALL
        .iter()
        .map(|view| view_named(view.as_str(), Origin::new(FIXTURE_FILE)).ok())
        .collect();

    assert_eq!(
        read_back,
        View::ALL.iter().copied().map(Some).collect::<Vec<_>>(),
        "the ten names a refusal offers are the ten a request may use, or the message sends its reader somewhere the tool will not follow"
    );
    Ok(())
}

/// Additional coverage, for the same reason and on the same shape of path.
///
/// One is the boundary the refusal above names, and nothing else here asks for
/// it: every other test uses the eight-pixel default, which arrives as a
/// constant rather than through this reader.
#[test]
fn the_smallest_accepted_pixels_per_voxel_is_the_minimum_the_refusal_names() -> TestResult {
    let accepted: Vec<Option<u32>> = [1, 8]
        .into_iter()
        .map(|requested| {
            pixels_per_voxel(requested, Origin::new(FIXTURE_FILE))
                .ok()
                .map(NonZeroU32::get)
        })
        .collect();

    assert_eq!(
        accepted,
        vec![Some(1), Some(8)],
        "one pixel per voxel is a coarse preview rather than an illegal one, and a reader that refused it would contradict its own refusal message"
    );
    Ok(())
}
