//! The mip chain, as arithmetic: the sRGB transfer pair, the box average in
//! linear light, the chain of levels, and the levels one layer offers.
//!
//! **Nothing here is wired.** These are pure functions over plain values; the
//! array texture still declares one level and the sampler is still nearest.
//! What uploads a chain arrives with the GPU layer.
//!
//! # Why the average is taken in linear light
//!
//! The array texture is `Rgba8UnormSrgb`, so a texel is decoded to linear on
//! sample — `gpu::buffers` records why that choice is load-bearing. A level
//! averaged over the **stored** bytes is therefore not the average of what the
//! sampler will see: stored 0 and 255 average to stored 128, which decodes to
//! linear 0.216 rather than 0.5, and every level comes out darker than the one
//! above it. That is the classic sRGB mipping fault, and it is plausible-looking
//! and wrong in the direction nothing notices. Decoding first, averaging, and
//! re-encoding puts the same pair at stored **188**.
//!
//! The transfer pair is IEC 61966-2-1 itself and not an approximation of it: a
//! gamma-2.2 curve answers 186 for the same pair, which is close enough to look
//! right and is a different function.

use mc_core::id::TextureKey;

use super::placeholder::placeholder_texels;
use super::supplied::SuppliedTexels;
use super::{MIP_LEVELS, TextureError};

/// IEC 61966-2-1, both directions: the linear segment's slope, the stored and
/// linear values where it gives way to the power segment, and that segment's
/// scale, offset and exponent.
const TRANSFER_SLOPE: f32 = 12.92;
const STORED_KNEE: f32 = 0.04045;
const LINEAR_KNEE: f32 = 0.0031308;
const TRANSFER_SCALE: f32 = 1.055;
const TRANSFER_OFFSET: f32 = 0.055;
const TRANSFER_EXPONENT: f32 = 2.4;

/// The largest value a stored byte holds, as the scale between the two.
const STORED_MAX: f32 = 255.0;

/// The light a stored sRGB byte stands for, in linear light on `0.0..=1.0`.
#[must_use]
pub fn to_linear(stored: u8) -> f32 {
    let encoded = f32::from(stored) / STORED_MAX;
    if encoded <= STORED_KNEE {
        encoded / TRANSFER_SLOPE
    } else {
        ((encoded + TRANSFER_OFFSET) / TRANSFER_SCALE).powf(TRANSFER_EXPONENT)
    }
}

/// The stored sRGB byte that stands for `linear`.
///
/// Rounds to nearest and clamps. Truncating instead would lose the round trip —
/// a uniform image would darken by up to a byte a level, which is exactly the
/// fault this module exists to avoid, arrived at from the other side.
#[must_use]
pub fn to_stored(linear: f32) -> u8 {
    let clamped = linear.clamp(0.0, 1.0);
    let encoded = if clamped <= LINEAR_KNEE {
        clamped * TRANSFER_SLOPE
    } else {
        TRANSFER_SCALE * clamped.powf(1.0 / TRANSFER_EXPONENT) - TRANSFER_OFFSET
    };
    (encoded * STORED_MAX).round() as u8
}

/// The level below `level`, each of its texels the average of the four it
/// covers. `size` is the **source** edge, so the answer holds `(size/2)²`
/// texels, row-major.
///
/// **Colour is averaged in linear light and alpha is not**, because
/// `Rgba8UnormSrgb` decodes RGB through the transfer function and alpha
/// linearly — the format's own definition, not a preference.
///
/// **That treatment is discriminated now.** Both rules answer 255 for a constant
/// 255, so the reading that tells them apart stands where they disagree most:
/// two clear texels beside two opaque ones, which average where they stand to
/// **128** and in linear light to **188**.
/// `two_clear_texels_and_two_opaque_ones_reduce_to_the_stored_mean_and_not_the_lit_one`
/// asserts the first and rejects the second, and it computes the second out of
/// the transfer pair above rather than quoting it — so a pair that ever brought
/// the two rules within a byte of each other reports itself instead of leaving a
/// reading that can no longer tell them apart.
#[must_use]
pub fn reduced(level: &[[u8; 4]], size: u32) -> Vec<[u8; 4]> {
    let half = size >> 1;
    (0..half)
        .flat_map(|row| (0..half).map(move |column| (row, column)))
        .filter_map(|(row, column)| covered_average(level, size, row, column))
        .collect()
}

/// Every level of `level_zero`'s chain, starting with `level_zero` verbatim and
/// halving until a single texel.
///
/// The first element is the input unchanged, so whoever uploads a chain uploads
/// level zero from here rather than from a second copy of it.
#[must_use]
pub fn chain(level_zero: &[[u8; 4]], size: u32) -> Vec<Vec<[u8; 4]>> {
    let mut level = level_zero.to_vec();
    let mut edge = size;
    let mut levels = Vec::new();
    while edge > 1 {
        let next = reduced(&level, edge);
        levels.push(level);
        level = next;
        edge >>= 1;
    }
    levels.push(level);
    levels
}

/// The mip levels `key`'s layer is filled from: the supplied art where the
/// built set covers the key, and its generated texels where nothing does.
///
/// **An uncovered key is an ordinary answer, not a failure.** A mod author's
/// first block declares a texture nothing has drawn yet, and what they get is a
/// generated one rather than a refused launch.
///
/// # Errors
///
/// [`TextureError::WrongTexelCount`] where the supplied texels are not
/// `size * size`, and [`TextureError::TooFewLevels`] where the chain is shorter
/// than the array texture declares. Both name the key: a layer refused without
/// one leaves a reader with 256 candidates.
pub fn levels_for(
    key: &TextureKey,
    supplied: &SuppliedTexels,
    size: u32,
) -> Result<Vec<Vec<[u8; 4]>>, TextureError> {
    let level_zero = supplied
        .covering(key)
        .map_or_else(|| placeholder_texels(key, size), <[[u8; 4]]>::to_vec);
    let texels_declared = (size * size) as usize;
    if level_zero.len() != texels_declared {
        return Err(TextureError::WrongTexelCount {
            key: key.clone(),
            offered: level_zero.len(),
            declared: texels_declared,
        });
    }
    let levels = chain(&level_zero, size);
    if levels.len() < MIP_LEVELS as usize {
        return Err(TextureError::TooFewLevels {
            key: key.clone(),
            offered: levels.len(),
            declared: MIP_LEVELS as usize,
        });
    }
    Ok(levels)
}

/// The average of the four texels the reduced texel at `row`, `column` covers,
/// or `None` where `level` is shorter than an edge of `size` says it is.
///
/// Answering `None` rather than substituting a texel is what stops a short level
/// reducing to a plausible-looking one: the caller's answer comes back the wrong
/// length instead of quietly padded.
fn covered_average(level: &[[u8; 4]], size: u32, row: u32, column: u32) -> Option<[u8; 4]> {
    let upper_left = (((row << 1) * size) + (column << 1)) as usize;
    let lower_left = upper_left + size as usize;
    let [
        [red_a, green_a, blue_a, alpha_a],
        [red_b, green_b, blue_b, alpha_b],
    ] = [*level.get(upper_left)?, *level.get(upper_left + 1)?];
    let [
        [red_c, green_c, blue_c, alpha_c],
        [red_d, green_d, blue_d, alpha_d],
    ] = [*level.get(lower_left)?, *level.get(lower_left + 1)?];
    Some([
        mean_in_linear_light([red_a, red_b, red_c, red_d]),
        mean_in_linear_light([green_a, green_b, green_c, green_d]),
        mean_in_linear_light([blue_a, blue_b, blue_c, blue_d]),
        mean_of_stored([alpha_a, alpha_b, alpha_c, alpha_d]),
    ])
}

/// Four stored bytes of one colour channel, decoded, averaged and re-encoded.
fn mean_in_linear_light(channel: [u8; 4]) -> u8 {
    let total: f32 = channel.iter().copied().map(to_linear).sum();
    to_stored(total * 0.25)
}

/// Four stored bytes averaged where they stand — the alpha channel, which
/// `Rgba8UnormSrgb` never puts through the transfer function.
fn mean_of_stored(channel: [u8; 4]) -> u8 {
    let total: u32 = channel.iter().copied().map(u32::from).sum();
    ((total + 2) >> 2) as u8
}

#[cfg(test)]
#[path = "mip_test.rs"]
mod tests;
