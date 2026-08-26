//! What fills `base:water`'s array-texture layer, and what the image filling it
//! is made of.
//!
//! # Not one reading here goes through the suite's colour oracle, deliberately
//!
//! `support::art::drawn_texels` answers `covering(key)` **or else**
//! `placeholder_texels(key)`, which is the decision the product makes and is
//! correct. It is also why nothing in this suite could see that water had no art:
//! every colour assertion asked "does water draw what an uncovered key should
//! draw" and was truthfully told yes. So every reading below takes
//! `covering(key)` and **refuses the absence** rather than falling back — the
//! fallback is the thing being ruled out, and an oracle sharing it would rule it
//! out against itself.
//!
//! # The bounds come from the seven images that already shipped
//!
//! Every figure below was measured over `base:dirt`, `base:grass_top`, the four
//! `base:grass_side_*` and `base:stone` on 2026-08-26, before any water art
//! existed, and not one of them is a number this candidate produced. ΔE 10 is
//! what this project already calls two colours told apart
//! (`support/probe.rs::DIFFERENT_COLOR`); ΔE 2 is what it calls two texels
//! distinguishable; 16.10 is `base:dirt`'s own spread, the flattest image the set
//! ships.
//!
//! # A separation bound may not be restated as a set-wide invariant
//!
//! "Every shipped mean stands ΔE 10 from every other" is **false** and would be
//! the tempting generalisation: the four grass sides stand ΔE 0.47 to 1.05 from
//! each other and `base:dirt` stands ΔE 9.59 from `base:grass_side_west`. What is
//! claimed is about water against each of the seven, one at a time, because water
//! is a *material* the others are not and the set has room for it.

mod support;

use std::collections::BTreeSet;
use std::error::Error;

use mc_core::content::TEXTURE_EDGE;
use mc_core::id::TextureKey;
use mc_render::texture::mip::levels_for;
use mc_render::texture::placeholder::placeholder_texels;
use mc_render::texture::supplied::SuppliedTexels;

use support::art::{declared_material_colors, linear_mean};
use support::probe::distance;
use support::swatch::require;
use support::{TestResult, prepare_scene};

/// The key this whole file is about.
const WATER: &str = "base:water";

/// The seven keys the manifest baked before this one, each judged against water
/// on its own.
const THE_SEVEN_THAT_SHIPPED: [&str; 7] = [
    "base:dirt",
    "base:grass_side_east",
    "base:grass_side_north",
    "base:grass_side_south",
    "base:grass_side_west",
    "base:grass_top",
    "base:stone",
];

/// The materials `water-block.mcvox` is built from, by the file name each is
/// declared in.
///
/// **A palette a person wrote, sharing nothing with a decoded PNG.** A face bakes
/// its material colour unshaded, so what the image is made of can be stated as
/// material names and read out of `content/base/materials/` rather than
/// snapshotted from a run of the decoder.
const WATER_MATERIALS: [&str; 3] = ["water", "water_dark", "water_light"];

/// How many texels a layer of the array texture holds.
const TEXELS_IN_A_LAYER: usize = (TEXTURE_EDGE * TEXTURE_EDGE) as usize;

/// The two colours the generated stand-in checkerboards `base:water` between.
///
/// Stated as well as derived. The derivation below is what the assertion uses —
/// it cannot go stale — and these two are the control on it: they are the values
/// a human read off the screen and reported, and the values the four committed
/// goldens were measured for.
const THE_STAND_INS_TEXELS: [[u8; 3]; 2] = [[140, 38, 131], [160, 58, 151]];

/// What this project already calls two colours told apart.
const TOLD_APART: f64 = 10.0;

/// What it calls two texels distinguishable.
const DISTINGUISHABLE: f64 = 2.0;

/// `base:dirt`'s own widest pairwise separation — the flattest image the set
/// already ships, and the ceiling a smoother surface may not pass.
///
/// Measured over the shipped seven: dirt 16.10, `grass_top` 17.01, stone 22.87,
/// the four grass sides 55.51.
const NO_MORE_MOTTLED_THAN_DIRT: f64 = 16.10;

#[test]
fn waters_layer_is_filled_with_the_image_the_manifest_bakes_for_it() -> TestResult {
    let prepared = prepare_scene()?;
    let water = TextureKey::parse(WATER)?;

    let baked = baked_image(&water, &prepared.texels)?;
    let filled = filled_layer(&water, &prepared.texels)?;

    assert_eq!(
        (
            baked.len(),
            filled == baked,
            first_disagreement(&filled, &baked)
        ),
        (TEXELS_IN_A_LAYER, true, None),
        "the level zero the upload loop writes for water has to be the image the manifest bakes \
         for it, decoded by the client's own reader. Before this fix the manifest baked nothing \
         for this key, so `baked_image` refuses above and this never reaches its assertion — \
         which is the defect, not a missing test. The third element names the first texel that \
         disagrees rather than printing two hundred and fifty-six of them"
    );
    Ok(())
}

#[test]
fn waters_layer_holds_no_texel_of_the_generated_stand_in() -> TestResult {
    let prepared = prepare_scene()?;
    let water = TextureKey::parse(WATER)?;
    let stand_in = the_stand_in_this_reading_rules_out(&water)?;

    let filled = filled_layer(&water, &prepared.texels)?;

    let showing: Vec<[u8; 3]> = colors_of(&filled)
        .into_iter()
        .filter(|color| stand_in.contains(color))
        .collect();
    assert_eq!(
        showing,
        Vec::<[u8; 3]>::new(),
        "a single texel of either stand-in colour means the layer is still being filled from the \
         generator rather than from the set — which is what shipped, and what 8.46% to 20.81% of \
         every committed golden frame was made of. **This inspects the filled layer and not the \
         baked image**, which is the whole difference between the two: a set that covers the key \
         while the fill path ignores it satisfies every reading taken off `covering` and draws a \
         checkerboard anyway"
    );
    Ok(())
}

#[test]
fn every_texel_of_waters_baked_image_is_more_blue_than_it_is_red_or_green() -> TestResult {
    let prepared = prepare_scene()?;
    let water = TextureKey::parse(WATER)?;

    let baked = baked_image(&water, &prepared.texels)?;

    let not_blue: Vec<[u8; 3]> = colors_of(&baked)
        .into_iter()
        .filter(|[red, green, blue]| blue <= red || blue <= green)
        .collect();
    assert_eq!(
        (not_blue, colors_of(&baked).len() > 1),
        (Vec::<[u8; 3]>::new(), true),
        "water reads as water because its blue channel dominates at every texel, tone and accent \
         alike — a claim about all 256 rather than about a mean, which a grey image with one blue \
         speckle would also satisfy. The second element is what stops this passing over a layer \
         of one colour, where 'every texel' is one texel"
    );
    Ok(())
}

#[test]
fn waters_mean_stands_clear_of_every_other_shipped_images_mean() -> TestResult {
    let prepared = prepare_scene()?;
    let water = TextureKey::parse(WATER)?;

    let mean = linear_mean(&baked_image(&water, &prepared.texels)?);

    let mut too_close = Vec::new();
    for other in THE_SEVEN_THAT_SHIPPED {
        let key = TextureKey::parse(other)?;
        let apart = distance(linear_mean(&baked_image(&key, &prepared.texels)?), mean)?;
        if apart < TOLD_APART {
            too_close.push(format!("`{other}` at ΔE {apart:.2}"));
        }
    }

    assert_eq!(
        too_close,
        Vec::<String>::new(),
        "a sea the eye cannot tell from the ground it stands on is art that has not arrived, and \
         every frame-based reading in this suite that names water by its colour needs the same \
         separation to be a reading at all. Judged against each of the seven one at a time and \
         never as a set-wide invariant: the four grass sides stand ΔE 0.47 to 1.05 from each other"
    );
    Ok(())
}

#[test]
fn waters_tones_are_distinguishable_and_no_more_mottled_than_dirt() -> TestResult {
    let prepared = prepare_scene()?;
    let water = TextureKey::parse(WATER)?;

    let colors = colors_of(&baked_image(&water, &prepared.texels)?);
    let widest = widest_pairwise(&colors)?;

    assert!(
        widest > DISTINGUISHABLE && widest <= NO_MORE_MOTTLED_THAN_DIRT,
        "water is the smoothest surface in this set and still not flat: its widest pairwise \
         separation is ΔE {widest:.2}, and it has to stand above the ΔE {DISTINGUISHABLE} this \
         project calls two texels distinguishable and no higher than the ΔE \
         {NO_MORE_MOTTLED_THAN_DIRT} of `base:dirt`, the flattest image already shipped. Below the \
         floor the speckle is decoration nobody can see; above the ceiling a sea surface is more \
         mottled than soil. The image holds {} distinct colours",
        colors.len()
    );
    Ok(())
}

#[test]
fn waters_baked_image_is_made_of_the_colours_the_water_materials_declare() -> TestResult {
    let prepared = prepare_scene()?;
    let water = TextureKey::parse(WATER)?;

    let baked = baked_image(&water, &prepared.texels)?;

    assert_eq!(
        colors_of(&baked),
        declared_material_colors(&WATER_MATERIALS)?,
        "a face bakes its material colour unshaded, so the image is exactly the palette a person \
         wrote in `content/base/materials/`. This is the one reading in this file with a reference \
         outside the renderer: a decoder that swapped two channels, applied a transfer function it \
         should not have or shaded a face lands on colours no material declares, and none of the \
         three would move a triple committed from a run"
    );
    Ok(())
}

/// The image the built set bakes for `key`, refusing the absence.
///
/// **Never `support::art::drawn_texels`.** That helper falls back to the
/// generated stand-in, which is the product's own correct decision and is exactly
/// the state this file exists to rule out: an oracle sharing it reports the
/// defect as the expectation.
///
/// # Errors
///
/// Returns an error naming the key when the set covers nothing for it — which is
/// this spec's defect, said as a sentence rather than as a silent fallback.
fn baked_image(key: &TextureKey, texels: &SuppliedTexels) -> Result<Vec<[u8; 4]>, Box<dyn Error>> {
    Ok(texels
        .covering(key)
        .ok_or_else(|| {
            format!(
                "the shipped built set covers no image for `{key}`, so its layer is filled from \
                 the texture generated out of the key itself and every reading in this file would \
                 be about that generator. The manifest at `content/base/textures.toml` states no \
                 entry for this key",
                key = key.as_str()
            )
        })?
        .to_vec())
}

/// What actually fills `key`'s layer: the level zero the upload loop writes.
///
/// **Not the same question as [`baked_image`], and the pair is the point.** One
/// asks what the set offers and the other what the fill path did with it; a fill
/// path that stopped consulting the supply satisfies the first and draws a
/// checkerboard.
///
/// # Errors
///
/// Returns the refusal preparing that layer's levels raised, or the absence of a
/// level zero, which a chain cannot be built without.
fn filled_layer(key: &TextureKey, texels: &SuppliedTexels) -> Result<Vec<[u8; 4]>, Box<dyn Error>> {
    Ok(levels_for(key, texels, TEXTURE_EDGE)?
        .first()
        .cloned()
        .ok_or("water's filled layer has no level zero")?)
}

/// The distinct colours `texels` are made of, ascending.
fn colors_of(texels: &[[u8; 4]]) -> Vec<[u8; 3]> {
    texels
        .iter()
        .map(|[red, green, blue, _]| [*red, *green, *blue])
        .collect::<BTreeSet<[u8; 3]>>()
        .into_iter()
        .collect()
}

/// The colours the generator produces for `key` — what an uncovered key draws,
/// and therefore what the reading above rules out.
///
/// **The control on two literals.** They are a pure function of the key, so a
/// change to the generator would leave the reading hunting colours nothing draws
/// and passing over a layer full of the ones it does.
///
/// # Errors
///
/// Returns an error when the generator's colours are no longer the two a human
/// reported off the screen and the committed goldens were measured for.
fn the_stand_in_this_reading_rules_out(
    key: &TextureKey,
) -> Result<BTreeSet<[u8; 3]>, Box<dyn Error>> {
    let generated: BTreeSet<[u8; 3]> = colors_of(&placeholder_texels(key, TEXTURE_EDGE))
        .into_iter()
        .collect();
    require(
        generated
            == THE_STAND_INS_TEXELS
                .iter()
                .copied()
                .collect::<BTreeSet<_>>(),
        format!(
            "this reading is about the two colours a human saw on screen and the four committed \
             goldens were measured for, {THE_STAND_INS_TEXELS:?}. The generator now produces \
             {generated:?} for this key, so what is being ruled out is no longer what was reported"
        ),
    )?;
    Ok(generated)
}

/// The widest ΔE between any two of `colors`.
///
/// # Errors
///
/// Returns the distance metric's own failure, or the absence of a pair — a single
/// colour has no separation to report and a bound over it would be a bound over
/// nothing.
fn widest_pairwise(colors: &[[u8; 3]]) -> Result<f64, Box<dyn Error>> {
    let mut widest: Option<f64> = None;
    for (at, one) in colors.iter().enumerate() {
        for other in colors.iter().skip(at + 1) {
            let apart = distance(*one, *other)?;
            widest = Some(widest.map_or(apart, |held: f64| held.max(apart)));
        }
    }
    widest.ok_or_else(|| {
        format!(
            "this image is made of {} colour(s), so there is no pair to measure a spread over",
            colors.len()
        )
        .into()
    })
}

/// Where two layers first disagree, as an index and the two texels there.
fn first_disagreement(
    filled: &[[u8; 4]],
    expected: &[[u8; 4]],
) -> Option<(usize, [u8; 4], [u8; 4])> {
    filled
        .iter()
        .zip(expected)
        .enumerate()
        .find(|(_, (one, other))| one != other)
        .map(|(at, (one, other))| (at, *one, *other))
}
