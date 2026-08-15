//! What colour a block texture carries, and what it does not carry.
//!
//! A texture is sampled by a mesh, not read by a human, so a face factor baked
//! into it would light every block twice. Flatness is therefore an input to the
//! **render** rather than a property of a material — a material table is shared
//! across a whole art set and cannot express the intent of one emission.
//!
//! Every expected byte here is hand derived through the sRGB transfer function
//! and none is read off a render. Mid grey, `#808080`, on the two faces this
//! file names:
//!
//! ```text
//! decode(128 / 255) = ((0.50196 + 0.055) / 1.055) ^ 2.4 = 0.2158605   // linear
//!            × 0.80  (the ±x factor)                    = 0.1726884
//! encode(0.1726884) × 255                               = 115.40 → 115
//!            × 0.50  (the −y factor)                    = 0.1079303
//! encode(0.1079303) × 255                               =  92.37 →  92
//! ```
//!
//! **That is why `left` and `bottom` are the faces named.** They are the two the
//! shaded path darkens furthest and by different amounts, so a flat render that
//! quietly applied a factor lands on 115 and 92 rather than on 128 twice. `top`
//! could never grade it: its factor is 1.00, and a fully shaded implementation
//! emits the declared byte there and passes.

mod common;

use common::preview::{EIGHT_PER_VOXEL, drawn_colours, table_of};
use common::texture::{GREY, Tone, model, table, tone};
use common::{TestResult, assembled};
use voxforge::fault::Fault;
use voxforge::material::MaterialTable;
use voxforge::render::{Pixel, View, render, render_texture};
use voxforge::texture::AxisAlignedView;
use voxforge::volume::{StateSelection, Volume};

/// A material lighting none of itself.
const DULL: Tone = tone('p', "base:cast_dull", 0x808080);

/// One declaring the same colour and lighting most of itself.
///
/// 0.8 rather than 1.0 because the blend has to be *partly* applied: at 1.0 the
/// shaded path already returns the declared colour, and a texture that read
/// emissive would pass anyway.
const LIT: Tone = tone('q', "base:cast_lit", 0x808080);

/// The byte both materials declare, and the byte a texture of either must carry.
const DECLARED: Pixel = Pixel {
    red: 128,
    green: 128,
    blue: 128,
    alpha: 255,
};

/// What the shaded path makes of that byte on a `−x` face.
const DARKENED_ACROSS: Pixel = Pixel {
    red: 115,
    green: 115,
    blue: 115,
    alpha: 255,
};

/// A solid `[2, 3, 4]` model of one mid grey.
///
/// Non-cubic so that no two of its faces project to the same image shape, and
/// solid so that every ray stops on the face pointing at the camera.
fn grey_block() -> String {
    model((2, 3, 4), 16, &[GREY], &|_, _, _| Some(GREY))
}

/// A solid `[2, 2, 2]` model whose lower half declares no self-illumination and
/// whose upper half declares most of it.
///
/// Halved on `y` so that both materials are visible from `left`, which is the
/// face the scenarios name: an orthographic view of this block's `−x` side sees
/// one voxel of each.
fn two_materials() -> String {
    model((2, 2, 2), 16, &[DULL, LIT], &|_, y, _| {
        Some(if y == 0 { DULL } else { LIT })
    })
}

/// The distinct colours the flat texture of `face` holds.
fn flat(volume: &Volume, materials: &MaterialTable, face: View) -> Result<Vec<Pixel>, Fault> {
    let face = AxisAlignedView::parse(face)?;
    Ok(drawn_colours(&render_texture(
        volume,
        materials,
        face,
        EIGHT_PER_VOXEL,
    )))
}

#[test]
fn a_flat_texture_carries_the_declared_byte_on_the_faces_a_shaded_render_darkens() -> TestResult {
    let volume = assembled(&grey_block(), &StateSelection::default())?;
    let materials = table(&[GREY])?;

    assert_eq!(
        (
            flat(&volume, &materials, View::Left)?,
            flat(&volume, &materials, View::Bottom)?
        ),
        (vec![DECLARED], vec![DECLARED]),
        "a texture is sampled rather than read, so no face factor reaches it — the shaded path makes 115 of this byte on `left` and 92 on `bottom`, and neither is 128"
    );
    Ok(())
}

#[test]
fn flattening_one_render_leaves_a_preview_of_the_same_materials_shaded() -> TestResult {
    let volume = assembled(&grey_block(), &StateSelection::default())?;
    let materials = table(&[GREY])?;

    // One run, one table, two renders of the same face: the flat one and the
    // shaded one. A flattened *material table* would make both of them 128.
    let shaded = drawn_colours(&render(&volume, &materials, View::Left, EIGHT_PER_VOXEL));

    assert_eq!(
        (flat(&volume, &materials, View::Left)?, shaded),
        (vec![DECLARED], vec![DARKENED_ACROSS]),
        "flatness belongs to the render, so a texture and a preview sharing one material table disagree about the same face in the same run"
    );
    Ok(())
}

#[test]
fn two_materials_of_one_colour_differing_only_in_emissive_carry_one_texture_byte() -> TestResult {
    let volume = assembled(&two_materials(), &StateSelection::default())?;
    let materials = table_of(&[(DULL.key, DULL.colour, 0.0), (LIT.key, LIT.colour, 0.8)])?;

    // `left` and not `top`: the shaded path darkens a `−x` face to 115 for the
    // first material and, under any blend, to something below 128 for the
    // second, so one colour coming back is decidable. On `top` the factor is
    // 1.00 and a fully shaded implementation emits 128 for both.
    assert_eq!(
        flat(&volume, &materials, View::Left)?,
        vec![DECLARED],
        "no path exists from a material's self-illumination into a texture, so two materials of one colour are one colour whatever their emissive says"
    );
    Ok(())
}

/// Additional coverage: the same claim over every facing there is.
///
/// The three scenarios above name `left` and `bottom`, which leaves the four
/// remaining facings ungraded — an implementation resolving flatness *per face*
/// rather than once per render passes all three by flattening exactly the two
/// faces they look at. This reads the same property through all six, and over
/// both materials at once, so it also says the emissive independence is not a
/// fact about one facing.
///
/// It grades nothing about the factors themselves, which are deliberately still
/// an open decision: what it fixes is that a texture carries the declared byte,
/// which no factor is allowed to change.
#[test]
fn every_axis_aligned_face_of_a_flat_texture_carries_the_declared_byte() -> TestResult {
    let volume = assembled(&two_materials(), &StateSelection::default())?;
    let materials = table_of(&[(DULL.key, DULL.colour, 0.0), (LIT.key, LIT.colour, 0.8)])?;

    let faces: Vec<(&str, Vec<Pixel>)> = AxisAlignedView::ALL
        .into_iter()
        .map(|face| {
            (
                face.as_str(),
                drawn_colours(&render_texture(&volume, &materials, face, EIGHT_PER_VOXEL)),
            )
        })
        .collect();

    assert_eq!(
        faces,
        ["front", "back", "left", "right", "top", "bottom"]
            .map(|face| (face, vec![DECLARED]))
            .to_vec(),
        "the colour function is resolved once per render rather than per pixel or per facing, so every face of a flat texture carries the one declared byte"
    );
    Ok(())
}
