//! What colour a voxel's face reaches the image as.
//!
//! Every expected byte here is hand derived through the sRGB transfer function
//! and none is read off a render. The derivation, once, for the one that matters:
//!
//! ```text
//! decode(128 / 255) = ((0.50196 + 0.055) / 1.055) ^ 2.4 = 0.2158605   // linear
//!            × 0.80                                    = 0.1726884    // the ±x factor
//! encode(0.1726884) = 1.055 × 0.1726884 ^ (1 / 2.4) − 0.055 = 0.452509
//!            × 255                                    = 115.40 → 115
//! ```
//!
//! **Byte 128 against byte 188 is this project's recurring trap**, met twice
//! before this feature existed: shading a mid grey by 0.8 in *encoded* space
//! gives 102, and skipping the decode on the way in gives 188. Only the decode,
//! the multiply and the re-encode give 115. 0 and 255 are fixed points of the
//! transfer function, so a fixture built from them would grade none of this,
//! which is why every material below is a mid tone.
//!
//! An orthographic view of a solid box shows exactly one face normal, so each
//! render below must hold exactly **one** distinct colour. That is what makes
//! "unmodified on every visible face" decidable rather than a smoke test: three
//! views of an emissive material are three different face factors, and all three
//! must come back as the one declared colour.

mod common;

use common::preview::{EIGHT_PER_VOXEL, drawn_colours, rgb, table_of};
use common::{TestResult, assembled};
use voxforge::material::MaterialTable;
use voxforge::render::{Pixel, View, render};
use voxforge::volume::{StateSelection, Volume};

/// The material every fixture here is made of.
const MATERIAL: &str = "base:cast_block";

/// A solid `[2, 3, 4]` model of [`MATERIAL`], sliced on `y`.
///
/// Non-cubic so that no two of its faces project to the same image shape, and
/// solid so that every ray stops on the face pointing at the camera.
const BLOCK: &str = r#"schema = 1
name = "base:block"
scale = 16
size = [2, 3, 4]
origin = [0, 0, 0]
slice = "y"

[palette]
"m" = "base:cast_block"

[[layers]]
y = 0
grid = """
mm
mm
mm
mm
"""

[[layers]]
y = 1
grid = """
mm
mm
mm
mm
"""

[[layers]]
y = 2
grid = """
mm
mm
mm
mm
"""
"#;

#[test]
fn an_emissive_material_reaches_every_face_it_is_seen_from_as_its_declared_colour() -> TestResult {
    let declared = rgb(0x40, 0x80, 0xc0);
    let volume = assembled(BLOCK, &StateSelection::default())?;
    let materials = table_of(&[(MATERIAL, declared, 1.0)])?;
    let lit = Pixel::opaque(declared.red, declared.green, declared.blue);

    // Three views, three different face normals, three different factors — 1.00
    // up, 0.80 on ±x and 0.65 on ±z — and self-illumination means none of them
    // is applied.
    let faces: Vec<(&str, Vec<Pixel>)> = [View::Top, View::Left, View::Front]
        .into_iter()
        .map(|view| {
            (
                view.as_str(),
                drawn_colours(&render(&volume, &materials, view, EIGHT_PER_VOXEL)),
            )
        })
        .collect();

    assert_eq!(
        faces,
        vec![
            ("top", vec![lit]),
            ("left", vec![lit]),
            ("front", vec![lit])
        ],
        "a material that makes its own light is not lit by anything, so no face of it is darker than another"
    );
    Ok(())
}

#[test]
fn an_emissive_mid_grey_reaches_the_image_as_the_byte_it_was_declared_with() -> TestResult {
    let volume = assembled(BLOCK, &StateSelection::default())?;
    let materials = table_of(&[(MATERIAL, rgb(0x80, 0x80, 0x80), 1.0)])?;
    let grey = Pixel::opaque(128, 128, 128);

    let faces: Vec<(&str, Vec<Pixel>)> = [View::Top, View::Left, View::Front]
        .into_iter()
        .map(|view| {
            (
                view.as_str(),
                drawn_colours(&render(&volume, &materials, view, EIGHT_PER_VOXEL)),
            )
        })
        .collect();

    assert_eq!(
        faces,
        vec![
            ("top", vec![grey]),
            ("left", vec![grey]),
            ("front", vec![grey])
        ],
        "128 is what was declared and 128 is what comes out; 188 is what a colour that went through a decode without a matching encode looks like"
    );
    Ok(())
}

#[test]
fn a_shaded_mid_grey_is_darkened_by_its_face_factor_in_linear_space() -> TestResult {
    let volume = assembled(BLOCK, &StateSelection::default())?;
    let materials = table_of(&[(MATERIAL, rgb(0x80, 0x80, 0x80), 0.0)])?;

    // 1.00 upward leaves the declared byte alone; 0.80 on a `−x` face is 115,
    // derived at the top of this file. Multiplying the byte instead of the
    // linear value gives 102, and forgetting the encode gives 188 — both are
    // arithmetic a reader can check without running anything.
    let faces: Vec<(&str, Vec<Pixel>)> = [View::Top, View::Left]
        .into_iter()
        .map(|view| {
            (
                view.as_str(),
                drawn_colours(&render(&volume, &materials, view, EIGHT_PER_VOXEL)),
            )
        })
        .collect();

    assert_eq!(
        faces,
        vec![
            ("top", vec![Pixel::opaque(128, 128, 128)]),
            ("left", vec![Pixel::opaque(115, 115, 115)])
        ],
        "the face factors apply to light, and light is linear — a factor applied to the encoded byte lands somewhere else entirely"
    );
    Ok(())
}

/// How bright the table declares one class of face, ascending.
///
/// An ordering rather than four numbers: the factors themselves are still an
/// open question, and what this test grades is the shape of the table, not the
/// values in it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Class {
    /// The underside, darkest.
    Down,
    /// The two faces along `z`.
    Through,
    /// The two faces along `x`.
    Across,
    /// The top, brightest.
    Up,
}

/// The six axis views, each with the face it shows and that face's class.
const FACES: [(View, &str, Class); 6] = [
    (View::Top, "top", Class::Up),
    (View::Left, "left", Class::Across),
    (View::Right, "right", Class::Across),
    (View::Front, "front", Class::Through),
    (View::Back, "back", Class::Through),
    (View::Bottom, "bottom", Class::Down),
];

/// Which factor each face normal is shaded by.
#[derive(Debug, PartialEq, Eq)]
enum Faces {
    /// Every face of one class shows one byte, and the classes run darkest
    /// underneath to brightest overhead.
    ShadedByTheDeclaredTable,
    /// A view shows no single grey, so there was nothing to compare.
    NotOneColour(&'static str),
    /// Two faces the table gives one factor came out different.
    PairDisagrees {
        /// One of them.
        one: &'static str,
        /// The other.
        other: &'static str,
        /// What each showed.
        bytes: (u8, u8),
    },
    /// A face the table declares brighter is not brighter.
    OutOfOrder {
        /// The face the table puts above.
        brighter: &'static str,
        /// The face it puts below.
        darker: &'static str,
        /// What each showed.
        bytes: (u8, u8),
    },
}

/// One face, as measured.
type Face = (&'static str, Class, u8);

/// The one grey a view of the solid block shows, or `None` for anything else.
fn face_byte(volume: &Volume, materials: &MaterialTable, view: View) -> Option<u8> {
    match drawn_colours(&render(volume, materials, view, EIGHT_PER_VOXEL)).as_slice() {
        [only] if only.red == only.green && only.green == only.blue => Some(only.red),
        _ => None,
    }
}

/// Whether the six axis views shade by the table FR-5.4 declares.
fn shaded_by_face(volume: &Volume, materials: &MaterialTable) -> Faces {
    let mut measured: Vec<Face> = Vec::new();
    for (view, name, class) in FACES {
        match face_byte(volume, materials, view) {
            Some(byte) => measured.push((name, class, byte)),
            None => return Faces::NotOneColour(name),
        }
    }
    disagreeing_pair(&measured).unwrap_or_else(|| ordering(&measured))
}

/// Every ordered pair of measured faces.
fn pairs(measured: &[Face]) -> impl Iterator<Item = (&Face, &Face)> {
    measured
        .iter()
        .flat_map(move |one| measured.iter().map(move |other| (one, other)))
}

/// Two faces of one class showing different bytes, if any do.
fn disagreeing_pair(measured: &[Face]) -> Option<Faces> {
    pairs(measured)
        .find(|((_, one_class, one_byte), (_, other_class, other_byte))| {
            one_class == other_class && one_byte != other_byte
        })
        .map(
            |((one, _, one_byte), (other, _, other_byte))| Faces::PairDisagrees {
                one,
                other,
                bytes: (*one_byte, *other_byte),
            },
        )
}

/// Whether brightness runs the way the classes are ordered.
fn ordering(measured: &[Face]) -> Faces {
    let broken =
        pairs(measured).find(|((_, one_class, one_byte), (_, other_class, other_byte))| {
            one_class > other_class && one_byte <= other_byte
        });
    broken.map_or(
        Faces::ShadedByTheDeclaredTable,
        |((brighter, _, above), (darker, _, below))| Faces::OutOfOrder {
            brighter,
            darker,
            bytes: (*above, *below),
        },
    )
}

/// Additional coverage, and the second witness on this path.
///
/// The three scenarios above grade the transfer function, and they grade it
/// twice through one route: a single wrong exponent reddens FR-5.4-S2 and S3
/// identically, so between them they are one witness wearing two hats. They also
/// name only two of the four factors, which leaves **which normal gets which
/// factor** almost ungraded — swap the `±z` and `−y` rows of the table and every
/// one of the twenty scenarios stays green while every underside in every
/// preview is lit wrongly.
///
/// This reads the same shading by a different route: the *relations* the table
/// declares, over all six axis views. It pins no constant, so it forecloses
/// nothing about the factors' exact values, which are deliberately still open —
/// what it fixes is that `+x` and `−x` share a factor, that `+z` and `−z` share
/// one, and that up is brighter than the sides, the sides brighter than the
/// faces through `z`, and those brighter than the underside. Any permutation of
/// the four rows breaks one of those.
#[test]
fn each_face_normal_is_shaded_by_the_factor_its_own_row_of_the_table_gives_it() -> TestResult {
    let volume = assembled(BLOCK, &StateSelection::default())?;
    let materials = table_of(&[(MATERIAL, rgb(0x80, 0x80, 0x80), 0.0)])?;

    assert_eq!(
        shaded_by_face(&volume, &materials),
        Faces::ShadedByTheDeclaredTable,
        "the table gives one factor per face *class*, so the two `x` faces match each other, the two `z` faces match each other, and the four classes run brightest overhead to darkest underneath"
    );
    Ok(())
}
