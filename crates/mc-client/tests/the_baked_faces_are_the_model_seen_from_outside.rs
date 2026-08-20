//! Whether each baked image is the plane of the model it claims to be a view of,
//! in the orientation a viewer outside the block would see it.
//!
//! # The gap this exists to close
//!
//! Every other colour reading in this spec compares **means, histograms or set
//! membership**, and every one of those is invariant under rotation, reflection
//! and permutation. FR-8.1-S3 asks the four sides to be pairwise *unequal*, which
//! a rotation preserves; FR-8.1-S5 judges the *top* face, which was correct; the
//! ΔE figures, the distinct-colour counts and `share_within_any` all ask *which
//! colours are present* and never *where they sit*. The first mint of the golden
//! set drew the sides rotated, **not one of 1366 tests could see it**, and it was
//! found by the project owner looking at the picture.
//!
//! **A colour-based reading cannot see geometry.** This is the bake half of the
//! feature's two orientation claims; `a_drawn_side_shows_turf_above_dirt.rs` and
//! `a_drawn_side_keeps_its_left_to_right_order.rs` are the drawn half.
//!
//! # The oracle is the model, and the comparison is texel for texel
//!
//! A face texture is a flat axis-aligned view, so each of the six images **is**
//! one outermost plane of `grass-block.mcvox` — a file a person wrote, read by
//! `support::model`, which shares no line with the baker. A texel is turned into
//! a material name and back through `content/base/materials/`, which is possible
//! at all because a face bakes its material colour **unshaded**.
//!
//! **No golden is used as an oracle and no committed row index appears.** The
//! goldens were minted from the broken output, so they are a picture of the
//! defect; a committed boundary row would be the same mistake as a committed RGB
//! triple, and would agree with whatever the bake did on the day it was written.
//!
//! # Why the second number in the assertion is the whole point
//!
//! An image agreeing with its plane under the identity is not by itself an
//! orientation claim: a laterally symmetric image agrees under a mirror too, and
//! a comparison that cannot tell the eight dihedral transforms apart would report
//! agreement whichever one the baker had applied. So the reading also counts how
//! many of the seven other transforms it *can* tell apart, and the expectation
//! for that count is derived from the image's own symmetries rather than written
//! down — a transform that maps an image to itself cannot be discriminated, and
//! every other one must be. Art that became symmetric would lower the expectation
//! by itself instead of reddening a correct bake.
//!
//! **What this half cannot see**: whether the renderer draws the image the right
//! way up or the right way round. When it was written the bake was **correct** on
//! all six faces and the draw was wrong on five.

mod support;

use std::error::Error;

use mc_core::id::TextureKey;

use support::art::{built_texels, declared_palette};
use support::model::{Face, VoxelModel};
use support::swatch::require;
use support::{TestResult, content_root, repository_root};

/// The model every one of the grass block's six images is baked from, including
/// `base:dirt`, which is the grass block's underside.
const MODEL: &str = "grass-block.mcvox";

/// How one baked image stands against the plane it is a view of.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Compared {
    /// The key, so a failure names the image.
    key: String,
    /// The face word the manifest bakes it from.
    word: &'static str,
    /// Texels where the image and the plane disagree, seen from outside.
    disagreeing_texels: usize,
    /// How many of the seven other dihedral transforms this comparison tells
    /// apart from the identity.
    transforms_told_apart: usize,
}

#[test]
fn every_baked_face_agrees_with_the_model_plane_it_is_a_view_of() -> TestResult {
    let texels = built_texels(&content_root()?)?;
    let palette = declared_palette()?;
    let model = VoxelModel::read(&repository_root()?.join("content/base/models").join(MODEL))?;
    let baked = support::model::baked_from(MODEL)?;
    require_all_six_faces(&baked)?;

    let (read, must_read): (Vec<Compared>, Vec<Compared>) = baked
        .iter()
        .map(|(face, key)| compared(*face, key, (&model, &texels, &palette)))
        .collect::<Result<Vec<(Compared, Compared)>, _>>()?
        .into_iter()
        .unzip();

    assert_eq!(
        read, must_read,
        "each of the six images is one outermost plane of `{MODEL}` as a viewer outside the block \
         sees it, and the oracle is the model file rather than any golden — the goldens were minted \
         from the broken output. The first number is texels where the image and the plane disagree. \
         The second is the discrimination proof, and without it the first is not an orientation \
         claim at all: it counts how many of the seven other dihedral transforms this comparison \
         can tell apart, and it is expected to be every transform that does not map the image to \
         itself. A reading that compared means, histograms or set membership would score zero there"
    );
    Ok(())
}

#[test]
fn the_same_comparison_reports_every_face_whose_image_is_turned_a_quarter_turn() -> TestResult {
    let texels = built_texels(&content_root()?)?;
    let palette = declared_palette()?;
    let model = VoxelModel::read(&repository_root()?.join("content/base/models").join(MODEL))?;
    let baked = support::model::baked_from(MODEL)?;
    let edge = model.edge();

    let read = baked
        .iter()
        .map(|(face, key)| {
            let turned = quarter_turn(&colors_of(key, &texels)?, edge);
            let expected = colors_for(&model.plane(*face), &palette)?;
            Ok((key.clone(), disagreements(&expected, &turned) != 0))
        })
        .collect::<Result<Vec<(String, bool)>, Box<dyn Error>>>()?;

    assert_eq!(
        read,
        baked
            .iter()
            .map(|(_, key)| (key.clone(), true))
            .collect::<Vec<(String, bool)>>(),
        "**this is the reading above's positive control, and without it that file is an assertion \
         nobody has seen fail.** It arrives green — the bake was already correct when it was \
         written — so the only evidence the comparison can see orientation at all is that a quarter \
         turn of a real image makes it report disagreement. A reading built on means, histograms or \
         set membership would report **none** here, on every face, which is the entire reason this \
         file exists"
    );
    Ok(())
}

/// What the reading needs in hand: the model, the built set and the palette.
type Sources<'a> = (
    &'a VoxelModel,
    &'a mc_render::texture::supplied::SuppliedTexels,
    &'a std::collections::BTreeMap<String, [u8; 3]>,
);

/// How `key`'s image stands against the plane `face` shows, and how it must stand.
///
/// **The two counts come from two different comparisons, which is what stops the
/// second being a restatement of the first.** The reading measures the plane
/// against the image; the expectation measures the image against *itself* under
/// each transform. They agree only when the image is the plane: a bake that
/// applied a transform would leave the plane disagreeing under the identity and
/// the two counts diverging.
///
/// # Errors
///
/// Returns an error when the key does not parse, when the image is not the
/// model's own edge across, or when a voxel names a material the shipped palette
/// does not declare — each of which is a reading that cannot be taken rather than
/// a bake that is wrong.
fn compared(
    face: Face,
    key: &str,
    sources: Sources<'_>,
) -> Result<(Compared, Compared), Box<dyn Error>> {
    let (model, texels, palette) = sources;
    let image = colors_of(key, texels)?;
    let edge = model.edge();
    require(
        image.len() == edge * edge,
        format!(
            "`{key}` holds {} texels, where `{MODEL}` is {edge} voxels across and bakes at one \
             pixel per voxel — so there is no texel-for-texel comparison to make",
            image.len()
        ),
    )?;
    let expected = colors_for(&model.plane(face), palette)?;
    let stated = |disagreeing_texels, transforms_told_apart| Compared {
        key: key.to_owned(),
        word: face.word,
        disagreeing_texels,
        transforms_told_apart,
    };
    Ok((
        stated(
            disagreements(&expected, &image),
            told_apart(&expected, &image, edge),
        ),
        stated(0, asymmetries_of(&image, edge)),
    ))
}

/// How many of the seven non-identity transforms move `image` off itself.
///
/// The expectation for the discrimination count, and it is a property of the art
/// alone: a comparison can separate exactly the transforms the image is not
/// invariant under, and no comparison whatever can separate the others.
fn asymmetries_of(image: &[[u8; 3]], edge: usize) -> usize {
    dihedral(image, edge)
        .into_iter()
        .filter(|turned| turned != image)
        .count()
}

/// How many of the seven non-identity dihedral transforms this comparison
/// separates from the identity.
///
/// A transform is told apart when applying it to the expected plane changes what
/// the comparison reports. That is a property of the *art*, not of the bake: a
/// laterally symmetric image is told apart by nothing, which is the case the
/// derived expectation exists to allow for.
fn told_apart(expected: &[[u8; 3]], image: &[[u8; 3]], edge: usize) -> usize {
    let straight = disagreements(expected, image);
    dihedral(expected, edge)
        .into_iter()
        .filter(|turned| disagreements(turned, image) != straight)
        .count()
}

/// The seven transforms of `grid` other than the identity: three quarter turns,
/// each with and without a left-to-right mirror, and the mirror alone.
fn dihedral(grid: &[[u8; 3]], edge: usize) -> Vec<Vec<[u8; 3]>> {
    let mut turned = grid.to_vec();
    let mut all = vec![mirrored(&turned, edge)];
    for _ in 0..3 {
        turned = quarter_turn(&turned, edge);
        all.push(turned.clone());
        all.push(mirrored(&turned, edge));
    }
    all
}

/// `grid` turned a quarter turn.
fn quarter_turn(grid: &[[u8; 3]], edge: usize) -> Vec<[u8; 3]> {
    let last = edge.saturating_sub(1);
    (0..edge)
        .flat_map(|row| (0..edge).map(move |column| (row, column)))
        .filter_map(|(row, column)| grid.get((last - column) * edge + row).copied())
        .collect()
}

/// `grid` with its columns reversed.
fn mirrored(grid: &[[u8; 3]], edge: usize) -> Vec<[u8; 3]> {
    let last = edge.saturating_sub(1);
    (0..edge)
        .flat_map(|row| (0..edge).map(move |column| (row, column)))
        .filter_map(|(row, column)| grid.get(row * edge + (last - column)).copied())
        .collect()
}

/// Texels where two grids of the same size hold different colours.
fn disagreements(expected: &[[u8; 3]], image: &[[u8; 3]]) -> usize {
    expected
        .iter()
        .zip(image)
        .filter(|(wanted, found)| wanted != found)
        .count()
}

/// The colours `key`'s baked image holds, row-major.
fn colors_of(
    key: &str,
    texels: &mc_render::texture::supplied::SuppliedTexels,
) -> Result<Vec<[u8; 3]>, Box<dyn Error>> {
    Ok(support::art::drawn_texels(&TextureKey::parse(key)?, texels)
        .into_iter()
        .map(|[red, green, blue, _]| [red, green, blue])
        .collect())
}

/// The colours a plane of material names stands for.
///
/// # Errors
///
/// Returns an error naming a material the shipped palette does not declare. A
/// voxel of a material nobody described has no colour for a texel to be compared
/// against, and calling it a disagreement would report a missing palette entry as
/// a wrongly oriented bake.
fn colors_for(
    plane: &[String],
    palette: &std::collections::BTreeMap<String, [u8; 3]>,
) -> Result<Vec<[u8; 3]>, Box<dyn Error>> {
    plane
        .iter()
        .map(|material| {
            palette.get(material).copied().ok_or_else(|| {
                format!(
                    "`{MODEL}` spells a voxel of `{material}`, which `content/base/materials/` \
                     declares no colour for"
                )
                .into()
            })
        })
        .collect()
}

/// Fails unless the manifest bakes all six of the block's faces from this model.
///
/// **A premise, not a claim about orientation.** The reading is over the faces the
/// manifest names, so a manifest that came to bake five of them would quietly
/// narrow what this covers — and the face it stopped baking is the one nobody
/// would notice, exactly as the bottom face went unnoticed for being invisible in
/// this world.
fn require_all_six_faces(baked: &[(Face, String)]) -> Result<(), Box<dyn Error>> {
    let mut words: Vec<&str> = baked.iter().map(|(face, _)| face.word).collect();
    words.sort_unstable();
    words.dedup();
    require(
        words.len() == support::model::FACES.len() && baked.len() == words.len(),
        format!(
            "this reading covers the faces `content/base/textures.toml` bakes from `{MODEL}`, and \
             a block has six: it bakes {} entries spelling {words:?}",
            baked.len()
        ),
    )
}
