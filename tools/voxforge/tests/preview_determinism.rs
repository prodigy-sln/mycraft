//! Whether the same model always encodes to the same bytes, and whether a
//! different model encodes to different ones.
//!
//! The third scenario is what makes the first two mean anything. A renderer that
//! emits one constant image satisfies "twice is identical" and "declaration
//! order does not matter" perfectly and for ever, and only "a changed voxel
//! changes the bytes" can tell it from a working one. They are three readings of
//! one property and none of them is worth much alone.
//!
//! The encodings are compared as bytes rather than through an image metric on
//! purpose: this is a within-one-process comparison, where the encoder's version
//! cannot vary between the two sides. A *committed* golden is the other case
//! entirely and belongs to the reference sheet, which is compared through a
//! perceptual metric because a PNG encoder is free to change its bytes across a
//! version bump.

mod common;

use std::error::Error;

use common::preview::{Encodings, Paint, compared, painted, paints, png_of};
use common::{TestResult, assembled};
use voxforge::format::FilledCell;
use voxforge::render::View;
use voxforge::volume::{StateSelection, Volume};

/// A solid `[2, 2, 2]` model, blue below and red above, whose two layers are
/// declared in `order`.
///
/// Two layers is the whole of what the scenario needs: the question is whether
/// the order they were *written* in survives into the raster, and a document
/// with one layer cannot ask it.
fn stacked(order: [u32; 2]) -> String {
    let layers: String = order
        .iter()
        .map(|plane| {
            let paint = if *plane == 0 { 'b' } else { 'r' };
            format!(
                "\n[[layers]]\ny = {plane}\ngrid = \"\"\"\n{paint}{paint}\n{paint}{paint}\n\"\"\"\n"
            )
        })
        .collect();
    format!(
        r#"schema = 1
name = "base:stacked"
scale = 16
size = [2, 2, 2]
origin = [0, 0, 0]
slice = "y"

[palette]
"r" = "base:ruby"
"b" = "base:lapis"
{layers}"#
    )
}

/// A solid `[2, 2, 2]` blue model with one voxel of `dot` at `(0, 0, 1)`.
///
/// `z = 1` is the plane a front view reaches first, so the one voxel that
/// differs between the two models is one a front view can actually see. A change
/// hidden behind another voxel would leave the bytes identical for a reason that
/// has nothing to do with determinism.
fn dotted(dot: Paint) -> String {
    painted((2, 2, 2), &|x, y, z| match (x, y, z) {
        (0, 0, 1) => Some(dot),
        _ => Some(Paint::Blue),
    })
}

/// The two documents of [`stacked`], ascending and descending.
///
/// # Errors
///
/// Returns an error when the two texts are the same, which would make a
/// byte-identical answer a statement about nothing.
fn declared_both_ways() -> Result<(String, String), Box<dyn Error>> {
    let (ascending, descending) = (stacked([0, 1]), stacked([1, 0]));
    if ascending == descending {
        return Err(
            "both documents came out identical, so their layer order differs in neither".into(),
        );
    }
    Ok((ascending, descending))
}

/// The two volumes of [`dotted`], differing in exactly one voxel's material.
///
/// # Errors
///
/// Returns an error unless they hold the same filled positions and disagree
/// about exactly one of them — the scenario is about a *material* change, and a
/// fixture that moved a voxel as well would be answering a different question.
fn differing_in_one_voxel() -> Result<(Volume, Volume), Box<dyn Error>> {
    let plain = assembled(&dotted(Paint::Blue), &StateSelection::default())?;
    let altered = assembled(&dotted(Paint::Red), &StateSelection::default())?;
    let (left, right) = (plain.filled(), altered.filled());
    let positions: Vec<_> = left.iter().map(|cell| cell.position).collect();
    let moved: Vec<_> = right.iter().map(|cell| cell.position).collect();
    if positions != moved {
        return Err(
            "the two fixtures do not fill the same voxels, so they differ in more than a material"
                .into(),
        );
    }
    let differing = left
        .iter()
        .zip(right.iter())
        .filter(|(left, right): &(&FilledCell, &FilledCell)| left.material != right.material)
        .count();
    if differing != 1 {
        return Err(
            format!("the two fixtures differ in {differing} voxels' materials, not one").into(),
        );
    }
    Ok((plain, altered))
}

#[test]
fn rendering_one_model_twice_encodes_to_the_same_bytes_both_times() -> TestResult {
    let volume = assembled(&dotted(Paint::Red), &StateSelection::default())?;
    let materials = paints()?;

    assert_eq!(
        compared(
            &png_of(&volume, &materials, View::Front)?,
            &png_of(&volume, &materials, View::Front)?
        ),
        Encodings::Identical,
        "nothing in this tool reads a clock, a random source or a hash order, so two runs of one render have nothing to disagree about"
    );
    Ok(())
}

#[test]
fn two_documents_whose_layers_are_declared_in_opposite_orders_encode_to_the_same_bytes()
-> TestResult {
    let (ascending, descending) = declared_both_ways()?;
    let materials = paints()?;
    let first = assembled(&ascending, &StateSelection::default())?;
    let second = assembled(&descending, &StateSelection::default())?;

    assert_eq!(
        compared(
            &png_of(&first, &materials, View::Front)?,
            &png_of(&second, &materials, View::Front)?
        ),
        Encodings::Identical,
        "the order layers were written in is a fact about the document, not about the model, and it must not reach the image"
    );
    Ok(())
}

#[test]
fn changing_one_voxels_material_changes_the_encoded_bytes() -> TestResult {
    let (plain, altered) = differing_in_one_voxel()?;
    let materials = paints()?;

    assert_ne!(
        compared(
            &png_of(&plain, &materials, View::Front)?,
            &png_of(&altered, &materials, View::Front)?
        ),
        Encodings::Identical,
        "a renderer that emits one constant image passes every other determinism scenario there is, and this is the one it cannot"
    );
    Ok(())
}
