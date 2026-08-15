//! A block's six faces, emitted from one invocation so that they cannot
//! disagree.
//!
//! The load-bearing property is that the set is **all-or-nothing**: every
//! verdict computed and every byte vector built before any file is opened. A
//! shell loop calling this tool six times cannot provide that — by the time the
//! fourth invocation refuses, three files are already on disk. This file
//! *enables* that property by emitting the whole set from one call; the file
//! that *asserts* it is the one driving the command line, because `emit` takes
//! no paths and writes nothing.
//!
//! **The fixture is the marker cube and that is not decoration.** A solid
//! uniform cube emits six byte-identical images, so a fixture without markers is
//! satisfied by an implementation that renders `front` six times and by one that
//! confuses any two faces. Each marker sits with exactly one coordinate at an
//! extreme, so each face sees its own and no other, and every boundary row and
//! column stays grey — the seam legs are untouched by the whole arrangement.

mod common;

use common::TestResult;
use common::preview::pixels;
use common::preview::{Encodings, compared};
use common::texture::{Emission, GREY, MARKERS, Outcome, Tone, Words, emitted, words};
use common::tiles::{marker_cube, not_a_cube};
use voxforge::fault::Origin;
use voxforge::material::Srgb8;
use voxforge::render::{Pixel, Preview, to_png};
use voxforge::texture::{AxisAlignedView, SeamVerdict};

/// The palette a one-grey fixture is painted from.
const PLAIN: [Tone; 1] = [GREY];

/// The name a face's bytes are attributed to when an encode has to name one.
const ENCODED_AS: &str = "face.png";

/// Whether the six images of a set are six different pictures.
#[derive(Debug, PartialEq, Eq)]
enum Distinctness {
    /// No two of the six are byte-identical.
    EverySixDiffer,
    /// Two of them are the same picture.
    Duplicate {
        /// One of the pair.
        one: String,
        /// The other.
        other: String,
    },
    /// Something other than six faces came out.
    NotSix(usize),
}

/// Whether a set's faces are each byte-identical to that same face emitted on
/// its own.
#[derive(Debug, PartialEq, Eq)]
enum Agreement {
    /// Every one of the six is.
    EveryFaceMatchesItsOwnEmission,
    /// One of them is not.
    Differs {
        /// Which face.
        face: String,
        /// How the two encodings differ.
        how: Encodings,
    },
    /// A face is missing from one side or the other, so nothing was compared.
    NotEmitted {
        /// Which face.
        face: String,
    },
}

/// The PNG encoding of `image`.
fn encoded(image: &Preview) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    Ok(to_png(image, Origin::new(ENCODED_AS))?)
}

/// Whether no two of `outcome`'s six images are the same picture.
fn distinctness(outcome: &Outcome) -> Result<Distinctness, Box<dyn std::error::Error>> {
    let faces = outcome.faces();
    if faces.len() != AxisAlignedView::ALL.len() {
        return Ok(Distinctness::NotSix(faces.len()));
    }
    let mut seen: Vec<(&str, Vec<u8>)> = Vec::new();
    for face in faces {
        let bytes = encoded(&face.image)?;
        if let Some((other, _)) = seen.iter().find(|(_, held)| *held == bytes) {
            return Ok(Distinctness::Duplicate {
                one: face.face.as_str().to_owned(),
                other: (*other).to_owned(),
            });
        }
        seen.push((face.face.as_str(), bytes));
    }
    Ok(Distinctness::EverySixDiffer)
}

/// Which face carried which verdicts, in the order the set reports them.
fn verdicts(outcome: &Outcome) -> Vec<(&str, Vec<SeamVerdict>)> {
    outcome
        .faces()
        .iter()
        .map(|face| (face.face.as_str(), face.verdicts.clone()))
        .collect()
}

/// What every face of a cube that tiles carries.
fn all_tiling() -> Vec<(&'static str, Vec<SeamVerdict>)> {
    AxisAlignedView::ALL
        .into_iter()
        .map(|face| (face.as_str(), vec![SeamVerdict::TilesAcrossEveryEdge]))
        .collect()
}

#[test]
fn a_face_set_emits_six_different_pictures_each_with_its_own_verdict() -> TestResult {
    let outcome = emitted(&marker_cube(), &marker_palette(), Emission::reported_set())?;

    assert_eq!(
        (distinctness(&outcome)?, verdicts(&outcome)),
        (Distinctness::EverySixDiffer, all_tiling()),
        "six faces of a cube are six pictures, and a set that rendered `front` six times or confused any two of them would hand back a duplicate"
    );
    Ok(())
}

#[test]
fn each_face_of_a_set_is_the_same_picture_as_that_face_emitted_on_its_own() -> TestResult {
    let set = emitted(&marker_cube(), &marker_palette(), Emission::reported_set())?;

    assert_eq!(
        agreement(&set)?,
        Agreement::EveryFaceMatchesItsOwnEmission,
        "one invocation fixes one volume, one material table and one scale, so emitting the six together can differ from emitting them one at a time in nothing at all"
    );
    Ok(())
}

/// Whether each of `set`'s faces matches that face emitted on its own.
fn agreement(set: &Outcome) -> Result<Agreement, Box<dyn std::error::Error>> {
    for face in AxisAlignedView::ALL {
        let alone = emitted(
            &marker_cube(),
            &marker_palette(),
            Emission::reported(face.view())?,
        )?;
        let (Some(within), Some(alone)) = (found(set, face), alone.only()) else {
            return Ok(Agreement::NotEmitted {
                face: face.as_str().to_owned(),
            });
        };
        let how = compared(&encoded(&within.image)?, &encoded(&alone.image)?);
        if how != Encodings::Identical {
            return Ok(Agreement::Differs {
                face: face.as_str().to_owned(),
                how,
            });
        }
    }
    Ok(Agreement::EveryFaceMatchesItsOwnEmission)
}

/// That face of `outcome`, where it emitted one.
fn found(outcome: &Outcome, wanted: AxisAlignedView) -> Option<&voxforge::texture::EmittedFace> {
    outcome.faces().iter().find(|face| face.face == wanted)
}

#[test]
fn a_face_set_of_a_model_that_is_not_a_cube_is_refused_under_either_policy() -> TestResult {
    let named = ["z axis", "3", "4"];
    let declared = emitted(&not_a_cube(), &PLAIN, Emission::seamless_set())?;
    let undeclared = emitted(&not_a_cube(), &PLAIN, Emission::reported_set())?;

    assert_eq!(
        (words(&declared, &named), words(&undeclared, &named)),
        (Words::NamedEverything, Words::NamedEverything),
        "a face set is by definition a block's six faces, so being a cube is a precondition of the request rather than a seam verdict — and a precondition does not wait to be asked"
    );
    Ok(())
}

#[test]
fn each_face_reports_the_two_model_axes_its_image_runs_along() -> TestResult {
    let outcome = emitted(&marker_cube(), &marker_palette(), Emission::reported_set())?;

    assert_eq!(
        mapping(&outcome),
        expected_mapping(),
        "the printed pair is the entire declared mitigation for a consumer mapping these onto a mesh, and code printing `(x, y)` six times satisfies a claim that only says a pair is printed — so the marker is what pins direction"
    );
    Ok(())
}

/// Each face's declared axis pair and where its own marker landed.
fn mapping(outcome: &Outcome) -> Vec<(&str, &str, &str, Block)> {
    outcome
        .faces()
        .iter()
        .zip(MARKERS)
        .map(|(face, (marker, _))| {
            (
                face.face.as_str(),
                face.face.columns().as_str(),
                face.face.rows().as_str(),
                block_of(&face.image, marker.colour),
            )
        })
        .collect()
}

/// Where FR-5.2's orientation contract and each face's axis pair put its marker.
///
/// Derived rather than measured. `front` runs its columns along `+x` from the
/// model's minimum and its rows *down* from the model's maximum `y`, so the
/// marker at `(1, 2, 3)` occupies columns `1 × 8 ..` and rows `(4 − 1 − 2) × 8
/// ..`. `back` and `right` run their columns backwards, so theirs are
/// `(4 − 1 − x) × 8`; `top` runs its rows along `+z` and `bottom` backwards
/// along it.
fn expected_mapping() -> Vec<(&'static str, &'static str, &'static str, Block)> {
    let at = |column, row| Block::At {
        column,
        row,
        width: 8,
        height: 8,
    };
    vec![
        ("front", "x", "y", at(8, 8)),
        ("back", "x", "y", at(8, 16)),
        ("left", "z", "y", at(8, 8)),
        ("right", "z", "y", at(8, 16)),
        ("top", "x", "z", at(8, 16)),
        ("bottom", "x", "z", at(16, 16)),
    ]
}

/// The marker cube's whole palette: the grey it is made of, and six markers.
fn marker_palette() -> Vec<Tone> {
    let mut palette = vec![GREY];
    palette.extend(MARKERS.iter().map(|(tone, _)| *tone));
    palette
}

/// Where a run of one colour sits in an image.
#[derive(Debug, PartialEq, Eq)]
pub enum Block {
    /// A solid rectangle of that colour, with this corner and this size.
    At {
        /// Its leftmost pixel column.
        column: u32,
        /// Its topmost pixel row.
        row: u32,
        /// How many pixels wide it is.
        width: u32,
        /// How many pixels tall it is.
        height: u32,
    },
    /// No pixel of the image holds that colour.
    NotDrawn,
    /// Pixels of that colour are there and do not form a solid rectangle.
    Scattered {
        /// How many of them there are.
        pixels: usize,
    },
}

/// Where `colour` sits in `preview`.
#[must_use]
pub fn block_of(preview: &Preview, colour: Srgb8) -> Block {
    let wanted = Pixel::opaque(colour.red, colour.green, colour.blue);
    let found: Vec<(u32, u32)> = pixels(preview)
        .filter(|(_, _, pixel)| *pixel == wanted)
        .map(|(column, row, _)| (column, row))
        .collect();
    let Some(((left, top), (right, bottom))) = corners(&found) else {
        return Block::NotDrawn;
    };
    let width = right.saturating_sub(left).saturating_add(1);
    let height = bottom.saturating_sub(top).saturating_add(1);
    if usize::try_from(width).ok().and_then(|across| {
        usize::try_from(height)
            .ok()
            .and_then(|down| across.checked_mul(down))
    }) != Some(found.len())
    {
        return Block::Scattered {
            pixels: found.len(),
        };
    }
    Block::At {
        column: left,
        row: top,
        width,
        height,
    }
}

/// The lowest and highest corner `found` reaches, or `None` where it is empty.
fn corners(found: &[(u32, u32)]) -> Option<((u32, u32), (u32, u32))> {
    let mut positions = found.iter().copied();
    let first = positions.next()?;
    Some(
        positions.fold((first, first), |(low, high), (column, row)| {
            (
                (low.0.min(column), low.1.min(row)),
                (high.0.max(column), high.1.max(row)),
            )
        }),
    )
}
