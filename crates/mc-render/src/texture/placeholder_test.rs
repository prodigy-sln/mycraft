//! Procedural stand-ins for block textures: the same texels every run, three
//! colours a viewer can tell apart, and none of them a flat fill.
//!
//! **Two different things are asserted here and the difference matters.**
//! `placeholder_texels` is a *generator* — it produces the pixels that reach the
//! array texture. `placeholder_mean_color` is a *declaration* — it is the value
//! the frame probes later cluster against, and nothing in this project ever
//! reads a colour out of a rendered frame to find out what it should have been.
//! A test that only compared declarations to each other would hold just as well
//! over a generator that emits nothing at all, so the separation scenario below
//! checks the declaration against the mean of the generated texels first and
//! only then checks the three declarations against each other. Each assertion
//! below says which of the two it is about.
//!
//! **Distance is the harness's, never this crate's.** `mc-testkit`'s `delta_e`
//! is the single place a perceptual distance is computed and the single swap
//! point if CIE76 is ever replaced, so the checks here drive its public
//! `compare` rather than reimplementing the metric: two 1x1 images give the
//! exact distance between two colours in `max_delta_e`, and a comparison against
//! a uniform field of one texel gives the share of a texture that differs from
//! it in `failing_fraction`. A second implementation here would let the goldens
//! and these thresholds silently judge by different metrics.

use std::cmp::Reverse;
use std::collections::BTreeMap;
use std::error::Error;

use mc_core::id::TextureKey;
use mc_testkit::frame::{Rgba8Image, Thresholds, compare};

use super::{placeholder_mean_color, placeholder_texels};

type TestResult = Result<(), Box<dyn Error>>;

/// A texture key and the mean colour it declares.
type DeclaredMean = (&'static str, [u8; 3]);

/// A texture key, the mean colour it declares, and the mean of the texels it
/// actually generated.
type MeanDrift = (&'static str, [u8; 3], [u8; 3]);

/// Two texture keys and the distance between their declared mean colours.
type MeanPair = (&'static str, &'static str, f64);

/// The edge length of one array layer.
const PLACEHOLDER_SIZE: u32 = 16;

/// How many texels a 16 x 16 layer holds. A generator that emitted nothing
/// would satisfy "generated twice, identical" without this.
const TEXELS_PER_LAYER: usize = 256;

/// The three keys the replay's blocks reference.
const DECLARED_KEYS: [&str; 3] = ["base:stone", "base:dirt", "base:grass"];

/// The three pairs of those keys, written out rather than nested loops.
const PAIRS: [(usize, usize); 3] = [(0, 1), (0, 2), (1, 2)];

/// How far a declared mean colour may sit from the mean of the texels actually
/// generated. The harness's own just-noticeable bound: a declaration further
/// from its texture than this is a colour the probes would hunt for in a frame
/// that never contains it.
const DECLARATION_TOLERANCE: f64 = 2.0;

/// How far apart two placeholder mean colours must be.
const PAIRWISE_FLOOR: f64 = 10.0;

/// How far a texel must sit from its texture's modal texel to count as varying.
const VARIATION_TOLERANCE: f64 = 2.0;

/// What share of a texture's texels must vary by that much.
const VARIETY_FLOOR: f64 = 0.25;

/// Fully opaque, which is what a block texture is.
const OPAQUE: u8 = 255;

/// One colour as an image, so the harness's comparison can measure it.
fn one_pixel(color: [u8; 3]) -> Result<Rgba8Image, Box<dyn Error>> {
    let [red, green, blue] = color;
    Ok(Rgba8Image::from_rgba(1, 1, vec![red, green, blue, OPAQUE])?)
}

/// The perceptual distance between two colours, taken from the harness's metric
/// rather than computed here.
fn distance(left: [u8; 3], right: [u8; 3]) -> Result<f64, Box<dyn Error>> {
    let every_difference = Thresholds::new(0.0, 1.0, f64::MAX)?;
    Ok(compare(&one_pixel(left)?, &one_pixel(right)?, &every_difference).max_delta_e)
}

/// The mean of generated texels, alpha ignored.
///
/// Errors on an empty texture rather than reporting a mean of nothing, because
/// "the generator emitted no texels" and "the generator emitted black" are
/// different defects and only one of them is visible in a colour.
fn mean_color(texels: &[[u8; 4]]) -> Result<[u8; 3], Box<dyn Error>> {
    let count = texels.len();
    if count == 0 {
        return Err("a placeholder texture with no texels has no mean colour".into());
    }
    let mut sums = [0.0_f64; 3];
    for texel in texels {
        for (sum, channel) in sums.iter_mut().zip(texel) {
            *sum += f64::from(*channel);
        }
    }
    Ok(sums.map(|sum| (sum / count as f64).round() as u8))
}

/// The texel a texture holds most of, ties broken by value so the answer does
/// not depend on iteration order.
fn modal_texel(texels: &[[u8; 4]]) -> Result<[u8; 4], Box<dyn Error>> {
    let mut tally: BTreeMap<[u8; 4], usize> = BTreeMap::new();
    for texel in texels {
        *tally.entry(*texel).or_insert(0) += 1;
    }
    tally
        .into_iter()
        .max_by_key(|&(texel, count)| (count, Reverse(texel)))
        .map(|(texel, _)| texel)
        .ok_or_else(|| "a placeholder texture with no texels has no modal texel".into())
}

/// Generated texels as an image the harness can compare.
fn as_image(texels: &[[u8; 4]], size: u32) -> Result<Rgba8Image, Box<dyn Error>> {
    let pixels = texels.iter().flatten().copied().collect();
    Ok(Rgba8Image::from_rgba(size, size, pixels)?)
}

/// A field of one texel at the same size, which is what a flat texture would be.
fn uniform_image(texel: [u8; 4], size: u32) -> Result<Rgba8Image, Box<dyn Error>> {
    let bytes = (size as usize) * (size as usize) * texel.len();
    let pixels = texel.iter().copied().cycle().take(bytes).collect();
    Ok(Rgba8Image::from_rgba(size, size, pixels)?)
}

#[test]
fn generating_a_placeholder_texture_twice_produces_the_same_texels() -> TestResult {
    let key = TextureKey::parse("base:stone")?;

    let first = placeholder_texels(&key, PLACEHOLDER_SIZE);
    let second = placeholder_texels(&key, PLACEHOLDER_SIZE);

    assert_eq!(
        (first.len(), &first),
        (TEXELS_PER_LAYER, &second),
        "a 16 x 16 layer is 256 texels and generating it twice must give the same 256; the \
         count is asserted alongside the equality because two empty textures are also equal"
    );
    Ok(())
}

/// Each key's declared mean colour, in the order the keys are listed.
fn declared_means() -> Result<Vec<DeclaredMean>, Box<dyn Error>> {
    let mut means = Vec::new();
    for name in DECLARED_KEYS {
        means.push((name, placeholder_mean_color(&TextureKey::parse(name)?)));
    }
    Ok(means)
}

/// The keys whose declared mean colour is not the mean of the texels they
/// actually generate — the tie between the declaration and the generator.
fn misdeclared_means() -> Result<Vec<MeanDrift>, Box<dyn Error>> {
    let mut misdeclared = Vec::new();
    for (name, declaration) in declared_means()? {
        let key = TextureKey::parse(name)?;
        let generated = mean_color(&placeholder_texels(&key, PLACEHOLDER_SIZE))?;
        if distance(declaration, generated)? > DECLARATION_TOLERANCE {
            misdeclared.push((name, declaration, generated));
        }
    }
    Ok(misdeclared)
}

/// The pairs of declared mean colours that sit no further apart than `floor`.
fn pairs_no_further_apart_than(floor: f64) -> Result<Vec<MeanPair>, Box<dyn Error>> {
    let means = declared_means()?;
    let mut too_close = Vec::new();
    for (left, right) in PAIRS {
        let (left_key, left_mean) = *means.get(left).ok_or("three keys make three pairs")?;
        let (right_key, right_mean) = *means.get(right).ok_or("three keys make three pairs")?;
        let apart = distance(left_mean, right_mean)?;
        if apart <= floor {
            too_close.push((left_key, right_key, apart));
        }
    }
    Ok(too_close)
}

#[test]
fn the_three_placeholder_textures_declare_mean_colours_a_viewer_can_tell_apart() -> TestResult {
    // First that each declaration is about its own generated texture, because a
    // declaration checked only against the other declarations would hold over a
    // generator that emits nothing at all.
    let misdeclared = misdeclared_means()?;
    assert!(
        misdeclared.is_empty(),
        "a declared mean colour is what the frame probes look for, so it must be the mean of \
         the texels actually generated; these were not, as (key, declared, generated): \
         {misdeclared:?}"
    );

    let too_close = pairs_no_further_apart_than(PAIRWISE_FLOOR)?;
    assert!(
        too_close.is_empty(),
        "the three declared mean colours must be pairwise more than {PAIRWISE_FLOOR} apart, \
         or a frame drawn entirely in one of them satisfies a probe looking for another; \
         these were not, as (key, key, distance): {too_close:?}"
    );
    Ok(())
}

#[test]
fn every_placeholder_texture_varies_across_more_than_a_quarter_of_its_texels() -> TestResult {
    let varying = Thresholds::new(VARIATION_TOLERANCE, 1.0, f64::MAX)?;

    let mut flat = Vec::new();
    for name in DECLARED_KEYS {
        let key = TextureKey::parse(name)?;
        let texels = placeholder_texels(&key, PLACEHOLDER_SIZE);
        let modal = uniform_image(modal_texel(&texels)?, PLACEHOLDER_SIZE)?;
        let share =
            compare(&modal, &as_image(&texels, PLACEHOLDER_SIZE)?, &varying).failing_fraction;
        if share <= VARIETY_FLOOR {
            flat.push((name, share));
        }
    }

    assert!(
        flat.is_empty(),
        "more than {VARIETY_FLOOR} of a placeholder's texels must differ from its own most \
         common texel by more than {VARIATION_TOLERANCE}, or the texture is a flat colour \
         with a pattern nobody can see; these were flatter, as (key, varying share): {flat:?}"
    );
    Ok(())
}
