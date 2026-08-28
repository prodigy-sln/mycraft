//! What a texture key's layer is actually filled with, and the colours a frame
//! drawn from it may hold.
//!
//! # Why this exists at all
//!
//! Every colour assertion in this suite used to come from `placeholder_texels`,
//! a pure function of a texture key. That was the whole of what filled a layer.
//! It is not any more: a key the built set covers is filled from a PNG on disk,
//! and the two are different colours entirely — the generated mean for
//! `base:dirt` stands **ΔE 62.94** from the baked dirt it replaces. So "what is
//! this layer made of" became a question with two answers depending on the key,
//! and [`drawn_texels`] is the one place that is decided.
//!
//! # It reads a file, and it never reads a frame
//!
//! The texels come through the client's own reader — the same decode a launch
//! performs — and the frame is only ever the thing being judged. That is the
//! separation `probe.rs`'s header states and it is unchanged here: an expectation
//! derived from a rendered picture is how a broken renderer certifies itself.
//!
//! # Two means, and this file declares the linear-light one
//!
//! A texture has a mean in linear light and a mean over its stored bytes, and
//! they are **not the same colour** — up to ΔE 2.38 apart on the grass sides,
//! which is past what this project calls two colours the same. The one declared
//! here is the linear-light mean, for the reason the array texture is
//! `Rgba8UnormSrgb`: a texel is decoded to linear on sample, so the mean a
//! minified face converges to, and the colour the smallest mip level holds, are
//! both this one. Averaging the stored bytes is the classic sRGB fault and gives
//! a darker answer.
//!
//! **The transfer function below is written from IEC 61966-2-1 and shares no
//! code with `mc_render::texture::mip`.** That is deliberate: the mip chain is
//! what *produces* the pixels a minified face shows, so an oracle calling into it
//! would be checking an arithmetic against itself. [`means_agree`] is the second
//! check on it — the stored-byte mean is integer arithmetic with no transfer
//! function anywhere in it, and a transfer applied backwards or inverted moves
//! the two means far further apart than any texture this repository ships does.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::path::Path;

use mc_client::textures::built_set;
use mc_core::content::TEXTURE_EDGE;
use mc_core::id::TextureKey;
use mc_render::texture::placeholder::placeholder_texels;
use mc_render::texture::supplied::SuppliedTexels;
use mc_testkit::frame::{Rgba8Image, Thresholds, compare};

use super::probe::{distance, uniform};

/// How far apart the two means of one shipped texture may stand.
///
/// **Measured, not chosen.** Over the **eight** images the shipped manifest
/// bakes the widest separation is ΔE 2.38, on `base:grass_side_north` and
/// `_south`. Three is the next whole number above it, which leaves this a check
/// on the transfer function rather than a restatement of a measurement: a decode
/// applied in the wrong direction puts the two tens of ΔE apart, and a decode
/// that is simply absent puts them at zero — which this does not catch, and
/// which [`drawn_colors`] having more than one colour does.
///
/// **Re-measured over eight when `base:water` was baked, and the figure did not
/// move.** It could not have gone down and it did not go up: water's two means
/// are byte-identical, ΔE 0.00, because 87.9% of its texels are one colour and
/// the two accents are symmetric about it, so its flat average lands on the base
/// tone in both arithmetics. The count in this sentence is not decoration — a
/// tolerance whose stated derivation no longer describes the set it was derived
/// from is a number nobody can re-check, however right it still happens to be.
pub const MEANS_AGREE_WITHIN: f64 = 3.0;

/// The texels supplied for `key`, and the generated texture where none were.
///
/// **This is the fallback, written once.** A key the built set does not cover is
/// a mod author's ordinary first block and it draws a generated texture; a key it
/// covers draws the art. Every reading in this suite that names a colour goes
/// through here, so no reading can be about the wrong one of the two.
#[must_use]
pub fn drawn_texels(key: &TextureKey, supplied: &SuppliedTexels) -> Vec<[u8; 4]> {
    supplied
        .covering(key)
        .map_or_else(|| placeholder_texels(key, TEXTURE_EDGE), <[_]>::to_vec)
}

/// The distinct colours `key`'s layer is made of, ascending.
#[must_use]
pub fn drawn_colors(key: &TextureKey, supplied: &SuppliedTexels) -> Vec<[u8; 3]> {
    drawn_texels(key, supplied)
        .into_iter()
        .map(|[red, green, blue, _]| [red, green, blue])
        .collect::<BTreeSet<[u8; 3]>>()
        .into_iter()
        .collect()
}

/// The colours a pixel drawn from `key`'s layer may be judged against: every
/// colour the layer holds, and the layer's own linear-light mean.
///
/// **The mean belongs in the set and it is not decoration.** A magnified face
/// shows one texel and a minified one converges towards the mean, so a reading
/// that admitted only the texel colours would call a distant face wrong and a
/// reading that admitted only the mean would call a near one wrong. Both regimes
/// appear in one frame of this world.
///
/// **A blend between two of these stays inside the set**, which is what makes it
/// a reading rather than a list. Measured over the three strata the replay is
/// made of: the widest a midpoint between two landmarks of one texture strays
/// from the nearest landmark is **ΔE 5.70**, on `base:stone`, against the ΔE 10
/// this suite calls two colours told apart.
#[must_use]
pub fn landmarks(key: &TextureKey, supplied: &SuppliedTexels) -> Vec<[u8; 3]> {
    let mut found = drawn_colors(key, supplied);
    let mean = linear_mean(&drawn_texels(key, supplied));
    if !found.contains(&mean) {
        found.push(mean);
    }
    found
}

/// Every colour a face drawn from `texels` may show at any distance, ascending.
///
/// **A superset of [`landmarks`] and deliberately not a replacement for it.** A
/// magnified face shows one texel and a fully minified one shows the layer's
/// mean, which is what `landmarks` answers; between those two a face shows a
/// *reduced* texel — the average of four, then of sixteen, and so on — and a
/// reduction lands nowhere near either end for a layer whose texels differ.
/// Measured on the shipped set: a grass side is four fifths dirt with a strip of
/// turf across its top, and a pixel of one at middle distance stands as far as
/// **ΔE 16.14** from every colour `landmarks` offers. That is not a renderer
/// fault and it is not a tolerance's business; it is a colour the layer really
/// does show.
///
/// **The reduction is written here and shares no code with
/// `mc_render::texture::mip`**, for the reason this module's header gives about
/// the transfer function: the mip chain is what *produces* those pixels, so an
/// oracle calling into it would be checking an arithmetic against itself. What
/// is written here is the obvious repeated 2 × 2 average in linear light, and
/// `mip_test.rs` is where the shipped reduction is judged.
///
/// **`landmarks` is left alone on purpose.** Every reading that already judges a
/// pixel against a layer accepts a narrower set than this, and widening that set
/// under them would loosen controls this change has no business loosening.
#[must_use]
pub fn landmarks_at_every_scale(texels: &[[u8; 4]], edge: u32) -> Vec<[u8; 3]> {
    let mut found: BTreeSet<[u8; 3]> = BTreeSet::new();
    let mut level = texels.to_vec();
    let mut side = edge;
    loop {
        found.extend(
            level
                .iter()
                .map(|[red, green, blue, _]| [*red, *green, *blue]),
        );
        if side <= 1 || level.len() < (side * side) as usize {
            break;
        }
        level = halved(&level, side);
        side /= 2;
    }
    found.into_iter().collect()
}

/// `level`, a `side` by `side` image, reduced to one of half that edge by
/// averaging each 2 × 2 block in linear light.
fn halved(level: &[[u8; 4]], side: u32) -> Vec<[u8; 4]> {
    let half = side.div_euclid(2);
    (0..half * half)
        .map(|at| quad_mean(level, side, at.div_euclid(half), at % half))
        .collect()
}

/// The 2 × 2 block of `level` at `down` and `across`, averaged in linear light.
///
/// Opaque by construction: a reduction is about colour, and the alpha channel
/// is reduced where it stands by the module this one shares no code with.
fn quad_mean(level: &[[u8; 4]], side: u32, down: u32, across: u32) -> [u8; 4] {
    let quad: Vec<[u8; 4]> = [(0, 0), (1, 0), (0, 1), (1, 1)]
        .iter()
        .filter_map(|(right, below)| {
            level.get((((2 * down + below) * side) + (2 * across + right)) as usize)
        })
        .copied()
        .collect();
    let [red, green, blue] = linear_mean(&quad);
    [red, green, blue, 255]
}

/// The colour `texels` average to in linear light.
///
/// Each stored byte is decoded through the sRGB transfer function, the channel
/// is averaged over the whole layer, and the average is encoded back.
///
/// **This is the flat average, and the smallest mip level is not always the same
/// byte.** Measured over the built set: `base:dirt`'s 1 x 1 level reads
/// `[139, 106, 71]` against this function's `[138, 106, 70]`, `base:grass_top`
/// `[105, 165, 79]` against `[104, 165, 78]`, and `base:stone` `[127, 127, 127]`
/// against `[126, 126, 126]` — **one byte on one or two channels, ΔE 0.39 to
/// 0.68**, because a chain rounds at each of four halvings where this rounds
/// once. Neither is wrong. Every tolerance in this suite is an order of
/// magnitude above that separation, so which of the two is declared decides
/// nothing here — but a reading tight enough to care about a byte would have to
/// say, and this one says: it is the flat average.
#[must_use]
pub fn linear_mean(texels: &[[u8; 4]]) -> [u8; 3] {
    let count = texels.len().max(1) as f64;
    [0, 1, 2].map(|channel| {
        let summed: f64 = texels
            .iter()
            .filter_map(|texel| texel.get(channel).copied())
            .map(to_linear)
            .sum();
        to_stored(summed / count)
    })
}

/// The colour `texels` average to over their stored bytes.
///
/// **Here as a check on the one above and for nothing else.** It is integer
/// arithmetic with no transfer function in it, so the two agreeing to within a
/// few ΔE is evidence the transfer function runs in the direction it says it
/// does. Nothing declares this as what a face shows.
#[must_use]
pub fn stored_mean(texels: &[[u8; 4]]) -> [u8; 3] {
    let count = f64::from(texels.len().max(1) as u32);
    [0, 1, 2].map(|channel| {
        let summed: f64 = texels
            .iter()
            .filter_map(|texel| texel.get(channel).copied())
            .map(f64::from)
            .sum();
        (summed / count).round() as u8
    })
}

/// How far `key`'s two means stand apart.
///
/// # Errors
///
/// Returns the distance metric's own failure, or a separation past
/// [`MEANS_AGREE_WITHIN`] — which is a broken oracle rather than a failed
/// behaviour, and says so.
pub fn means_agree(key: &TextureKey, supplied: &SuppliedTexels) -> Result<f64, Box<dyn Error>> {
    let texels = drawn_texels(key, supplied);
    let apart = distance(linear_mean(&texels), stored_mean(&texels))?;
    if apart > MEANS_AGREE_WITHIN {
        return Err(format!(
            "`{key}`'s linear-light mean and its stored-byte mean stand ΔE {apart:.2} apart, past \
             the ΔE {MEANS_AGREE_WITHIN} the eight shipped images were measured to hold. The two \
             are computed by different arithmetic — one through the sRGB transfer function and one \
             by integer averaging — so a separation this wide is a transfer function running the \
             wrong way rather than a texture that happens to be unusual, and every colour this \
             file declares would be derived through it",
            key = key.as_str()
        )
        .into());
    }
    Ok(apart)
}

/// What share of `frame` sits within `tolerance` of at least one of `colors`.
///
/// **Asking whether a texture's pixels cluster around its own *mean* is the
/// premise this spec measured false.** Of the four grass sides, **three have
/// 0.00% of their texels within ΔE 10 of their own mean and the fourth — west —
/// has 43.36%**; stone reaches only 66.41%. A mean falls in the gap between the
/// colours it averages, so a pixel belongs to a layer when it stands near *any*
/// colour that layer holds — the mean included, since a minified face converges
/// to it.
///
/// **Three and one, not four**, because a figure that flattens an extremum into
/// a general case is the shape `docs/technical/testing.md` warns about: the
/// premise is false either way, and stating 0% of all four would be a number
/// nobody could reproduce from the set.
///
/// The per-colour masks are intersected rather than the shares summed: a pixel
/// standing near two colours of one layer is one pixel. The distances themselves
/// are the harness's, driven through `compare` rather than computed here, for
/// the reason every other distance in this suite is.
///
/// # Errors
///
/// Returns the image-shape or threshold failure, or a comparison that reported
/// no per-pixel mask — which only a size mismatch produces, and which would
/// otherwise be read as a frame nothing strayed in.
pub fn share_within_any(
    frame: &Rgba8Image,
    colors: &[[u8; 3]],
    tolerance: f64,
) -> Result<f64, Box<dyn Error>> {
    let mut beyond: Option<Vec<bool>> = None;
    for color in colors {
        let field = uniform(frame.width(), frame.height(), *color)?;
        let mask = compare(&field, frame, &Thresholds::new(tolerance, 1.0, f64::MAX)?)
            .failing_mask
            .ok_or("the comparison reported no per-pixel mask, so nothing can be attributed")?;
        let flags = coordinates(frame.width(), frame.height())
            .map(|(x, y)| mask.is_failing(x, y))
            .collect::<Vec<bool>>();
        beyond = Some(match beyond {
            None => flags,
            Some(held) => held
                .into_iter()
                .zip(flags)
                .map(|(one, other)| one && other)
                .collect(),
        });
    }
    let Some(beyond) = beyond else { return Ok(0.0) };
    let outside = beyond.iter().filter(|failing| **failing).count() as f64;
    let pixels = f64::from(frame.width()) * f64::from(frame.height());
    Ok(1.0 - outside / pixels)
}

/// Every coordinate of a `width` by `height` frame, row by row.
fn coordinates(width: u32, height: u32) -> impl Iterator<Item = (u32, u32)> {
    (0..height).flat_map(move |y| (0..width).map(move |x| (x, y)))
}

/// The colours the named materials declare, ascending.
///
/// **A palette a person wrote, sharing nothing with a decoded PNG.** A face
/// bakes its material colour *unshaded* — measured across all eight shipped
/// images, every distinct texel colour is byte-identical to a declared material
/// — so what a texture is made of can be stated as a list of material *names*
/// and read out of `content/base/materials/`, rather than snapshotted as RGB
/// triples from a run of the decoder.
///
/// That is the difference between an expectation and a record of what happened.
/// A decoder that swapped two channels, applied a transfer function it should
/// not have, or shaded a face would each land on colours no material declares,
/// and not one of the three would move a committed triple.
///
/// # Errors
///
/// Returns an error when a named material is not there, or states no `color`, or
/// states one that is not `#rrggbb` — each of which is a palette nobody
/// described rather than a decoder that did anything.
pub fn declared_material_colors(names: &[&str]) -> Result<Vec<[u8; 3]>, Box<dyn Error>> {
    let materials = super::repository_root()?
        .join("content")
        .join("base")
        .join("materials");
    let mut colors = Vec::with_capacity(names.len());
    for name in names {
        let at = materials.join(format!("{name}.toml"));
        let written = std::fs::read_to_string(&at)
            .map_err(|cause| format!("`{}` could not be read: {cause}", at.display()))?;
        colors.push(color_stated_in(&written).ok_or_else(|| {
            format!(
                "`{}` states no `color` of the form `#rrggbb`, so there is no declared colour \
                 for this reading to be about",
                at.display()
            )
        })?);
    }
    colors.sort_unstable();
    Ok(colors)
}

/// Every material the shipped content declares, by the name it declares, and the
/// colour it states.
///
/// The companion to [`declared_material_colors`] for a reading that has a
/// *material name* in hand rather than a file name — a voxel model's palette
/// spells `base:grass`, and what colour that is belongs to the material file.
///
/// # Errors
///
/// Returns an error when the directory cannot be read, or when a file states a
/// `name` without a `color` — a material half-declared is a palette nobody
/// described, and a reading that skipped it would silently classify its voxels as
/// nothing.
pub fn declared_palette() -> Result<BTreeMap<String, [u8; 3]>, Box<dyn Error>> {
    let materials = super::repository_root()?
        .join("content")
        .join("base")
        .join("materials");
    let mut declared = BTreeMap::new();
    for entry in std::fs::read_dir(&materials)
        .map_err(|cause| format!("`{}` could not be read: {cause}", materials.display()))?
    {
        let at = entry?.path();
        if at.extension().is_none_or(|kind| kind != "toml") {
            continue;
        }
        let written = std::fs::read_to_string(&at)
            .map_err(|cause| format!("`{}` could not be read: {cause}", at.display()))?;
        let Some(name) = stated_in(&written, "name") else {
            continue;
        };
        let color = color_stated_in(&written).ok_or_else(|| {
            format!(
                "`{}` declares the material `{name}` and states no `color` of the form `#rrggbb`, \
                 so a voxel of it has no colour for a reading to be about",
                at.display()
            )
        })?;
        declared.insert(name.trim_matches('"').to_owned(), color);
    }
    Ok(declared)
}

/// The unquoted value a material file states for `key`, or nothing.
fn stated_in(written: &str, key: &str) -> Option<String> {
    Some(
        written
            .lines()
            .map(str::trim)
            .filter(|line| !line.starts_with('#'))
            .find_map(|line| line.strip_prefix(key))?
            .trim_start()
            .strip_prefix('=')?
            .trim()
            .trim_matches('"')
            .to_owned(),
    )
}

/// The `#rrggbb` a material file states, or nothing where it states none.
///
/// Read as text rather than through a TOML parser, and the reason is the one
/// this suite gives elsewhere: what these files are is content, and a reading
/// about them should not go through the same crate the loader does.
fn color_stated_in(written: &str) -> Option<[u8; 3]> {
    let stated = stated_in(written, "color")?.strip_prefix('#')?.to_owned();
    let mut channels = [0u8; 3];
    for (channel, at) in channels.iter_mut().zip([0, 2, 4]) {
        *channel = u8::from_str_radix(stated.get(at..at + 2)?, 16).ok()?;
    }
    Some(channels)
}

/// `src` laid over `dst` at `alpha`, in linear light and re-encoded.
///
/// **`src-over`, and the arithmetic is the whole point of it being here.** The
/// colour attachment is `Rgba8UnormSrgb`, so the hardware decodes both operands
/// to linear light, mixes them, and encodes the answer back — an expectation
/// computed on the stored bytes is a different number, and at a half blend
/// between two of this suite's fixture colours it stands ΔE 15.60 from the right
/// one. That is far past what any reading here calls two colours the same, so it
/// is a red test whose cheapest green is a looser tolerance, which is exactly the
/// mistake this function exists to make impossible.
///
/// The transfer pair it goes through is the one declared at the foot of this
/// module, written from IEC 61966-2-1 and sharing no code with
/// `mc_render::texture::mip` or with anything in the draw path.
///
/// `alpha` is the **declared** degree and never the byte a packed vertex
/// carries. The quantisation between the two is a term on the measured-error
/// side of a tolerance — at most ΔE 0.47 over this suite's fixtures — and
/// folding it into the expectation instead would be the prediction quietly
/// adopting the encoding it is meant to be independent of.
#[must_use]
pub fn composited(src: [u8; 3], dst: [u8; 3], alpha: f64) -> [u8; 3] {
    let mut mixed = [0u8; 3];
    for ((channel, over), under) in mixed.iter_mut().zip(src).zip(dst) {
        *channel = to_stored(alpha.mul_add(to_linear(over), (1.0 - alpha) * to_linear(under)));
    }
    mixed
}

/// The texels the built set under `root` offers, read the way a launch reads
/// them.
///
/// # Errors
///
/// Returns the reader's own refusal for a set that admits no verdict at all.
pub fn built_texels(root: &Path) -> Result<SuppliedTexels, Box<dyn Error>> {
    let (_verdict, texels) = built_set(root)?;
    Ok(texels)
}

/// IEC 61966-2-1, both directions. The published constants, written here rather
/// than imported: see this module's header.
const TRANSFER_SLOPE: f64 = 12.92;
const STORED_KNEE: f64 = 0.040_45;
const LINEAR_KNEE: f64 = 0.003_130_8;
const TRANSFER_SCALE: f64 = 1.055;
const TRANSFER_OFFSET: f64 = 0.055;
const TRANSFER_EXPONENT: f64 = 2.4;
const STORED_MAX: f64 = 255.0;

/// The light a stored sRGB byte stands for.
fn to_linear(stored: u8) -> f64 {
    let encoded = f64::from(stored) / STORED_MAX;
    if encoded <= STORED_KNEE {
        encoded / TRANSFER_SLOPE
    } else {
        ((encoded + TRANSFER_OFFSET) / TRANSFER_SCALE).powf(TRANSFER_EXPONENT)
    }
}

/// The stored sRGB byte that stands for `linear`, rounded to nearest.
fn to_stored(linear: f64) -> u8 {
    let clamped = linear.clamp(0.0, 1.0);
    let encoded = if clamped <= LINEAR_KNEE {
        clamped * TRANSFER_SLOPE
    } else {
        TRANSFER_SCALE * clamped.powf(1.0 / TRANSFER_EXPONENT) - TRANSFER_OFFSET
    };
    (encoded * STORED_MAX).round() as u8
}

/// How far the furthest texel of `key`'s layer stands from that layer's own
/// linear-light mean, in ΔE.
///
/// **The layer's spread, measured rather than quoted.** It is the term every
/// tolerance in this suite carries on its measured-error side, and it is a
/// property of an image on disk — so a reading that states it can be checked
/// against the image instead of against a number somebody wrote down.
///
/// # Errors
///
/// Returns an error for a key with no texels at all, or the distance metric's
/// own failure.
pub fn spread_of(key: &str, supplied: &SuppliedTexels) -> Result<f64, Box<dyn Error>> {
    let texels = drawn_texels(&TextureKey::parse(key)?, supplied);
    let mean = linear_mean(&texels);
    let mut widest: Option<f64> = None;
    for [red, green, blue, _] in &texels {
        let apart = distance([*red, *green, *blue], mean)?;
        widest = Some(widest.map_or(apart, |held: f64| held.max(apart)));
    }
    widest.ok_or_else(|| {
        format!("`{key}`'s layer holds no texel, so it has no spread to measure").into()
    })
}

/// How far two keys' linear-light means stand apart, in ΔE.
///
/// # Errors
///
/// Returns the key's parse failure or the distance metric's own.
pub fn means_apart(
    one: &str,
    other: &str,
    supplied: &SuppliedTexels,
) -> Result<f64, Box<dyn Error>> {
    let mean_of = |key: &str| -> Result<[u8; 3], Box<dyn Error>> {
        Ok(linear_mean(&drawn_texels(
            &TextureKey::parse(key)?,
            supplied,
        )))
    };
    distance(mean_of(one)?, mean_of(other)?)
}
