//! Stand-in block textures, generated from the texture key and nothing else.
//!
//! Real block art does not exist yet and is not what this spec is about, but
//! *something* has to reach the array texture — and whatever reaches it has to
//! be the same on every machine on every run, or a golden frame means nothing.
//! So a texture is a pure function of its key: same key, same 256 texels,
//! forever.
//!
//! # A declaration and a generator, kept apart
//!
//! [`placeholder_mean_color`] is a **declaration**. The frame probes cluster
//! pixels against it, and nothing in this project ever reads a colour out of a
//! rendered frame to find out what that frame should have contained — that is
//! how a broken renderer certifies itself. [`placeholder_texels`] is the
//! **generator**. The two agree because the variation the generator applies sums
//! to zero, not because either one measures the other: every texel is the
//! declared colour plus or minus one fixed step, on a checkerboard, so an
//! even-sided layer holds exactly as many of one as of the other and its mean is
//! the declared colour exactly. Break that balance and the scenario tying the
//! two together goes red, which is the only reason it is worth asserting.
//!
//! # Why a checkerboard and not hashed noise
//!
//! The first version drew each texel's direction from a hash bit, and a mutation
//! of the pairing turned exactly *one* of the three keys red — because for the
//! other two the branch it broke was never taken. FNV-1a's low bit is
//! `bit0(state ^ byte)`, so a hash keyed on an even counter yields a constant,
//! and multiplying once does not carry a small input change into the high bits
//! either. The pattern was a fixed stripe and the mixing was dead code that
//! looked like variety. A checkerboard makes the balance **structural**: it is
//! half and half by counting, not by a hash behaving.
//!
//! # Why one step and not a spread
//!
//! Every texel sits one step either side of the declared mean. That is far
//! enough to be visibly not-flat — a quarter of a texture must differ from its
//! own commonest texel, and half of it does — and near enough that every texel
//! stays well inside the tolerance the frame probes allow around the declared
//! colour. The two requirements pull in opposite directions, which is why the
//! step is named once, here.

use mc_core::id::TextureKey;

/// How far a texel sits from its texture's declared mean, on every channel.
///
/// Two texels of opposite parity differ by twice this — around ΔE 7, against a
/// floor of 2 — while one texel sits around ΔE 3.6 from the mean, against the
/// ΔE 10 the frame probes cluster within.
const VARIATION: i8 = 10;

/// The darkest and lightest a declared mean channel may be.
///
/// The band is inset from 0 and 255 by more than [`VARIATION`], so no texel ever
/// clips — a clipped channel would pull the layer's true mean away from the
/// declared one silently, on some keys and not others.
const BAND_LOW: u32 = 40;
const BAND_SPAN: u32 = 176;

/// Fully opaque. Block textures have no transparency in this increment.
const OPAQUE: u8 = 255;

/// FNV-1a, 64-bit: the offset basis and the prime.
///
/// A named, published hash rather than an ad-hoc mix, so "the same texels on
/// every machine" rests on a specification instead of on nobody having touched
/// it. Nothing here is security-sensitive; what is needed is determinism and
/// enough avalanche across the *whole* word that different keys land on
/// distinguishable colours. Note what it is deliberately **not** used for: no
/// single bit of this hash decides anything.
const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// The texels of `key`'s placeholder layer, row-major, `size` by `size`.
///
/// Deterministic in the key and the size alone: no clock, no address, no
/// iteration order of anything.
///
/// An even `size` — which every caller uses, and the only size any scenario
/// names — makes the layer's mean the declared colour exactly. An odd one
/// leaves a single unpaired texel, worth under a tenth of a channel across the
/// layer.
#[must_use]
pub fn placeholder_texels(key: &TextureKey, size: u32) -> Vec<[u8; 4]> {
    let mean = placeholder_mean_color(key);
    let lighter = varied(mean, VARIATION);
    let darker = varied(mean, -VARIATION);
    (0..size)
        .flat_map(|row| (0..size).map(move |column| (row + column) & 1 == 0))
        .map(|light| if light { lighter } else { darker })
        .collect()
}

/// The colour `key`'s placeholder layer averages to.
///
/// Declared, not measured. This is the value frame probes look for, so it is
/// derived from the key directly and never from a texture, a frame or a golden.
#[must_use]
pub fn placeholder_mean_color(key: &TextureKey) -> [u8; 3] {
    let hash = key_hash(key);
    [0, 1, 2].map(|channel| band(byte_of(hash, channel)))
}

/// One channel of a hash, as a value in the declared band.
///
/// A multiply and a shift rather than a division: `clippy::integer_division` is
/// denied workspace-wide, and this is the same arithmetic without the operator.
fn band(byte: u8) -> u8 {
    let scaled = (u32::from(byte) * BAND_SPAN) >> 8;
    (BAND_LOW + scaled) as u8
}

/// The `index`-th byte of `hash`.
fn byte_of(hash: u64, index: u32) -> u8 {
    (hash >> (index * 8)) as u8
}

/// `mean`, moved `step` on every channel.
///
/// Saturating, though the band above means it never has to saturate — a clamp
/// that is never reached is cheaper than a proof obligation that it is not.
fn varied(mean: [u8; 3], step: i8) -> [u8; 4] {
    let [red, green, blue] = mean.map(|channel| channel.saturating_add_signed(step));
    [red, green, blue, OPAQUE]
}

/// FNV-1a over the key's own text.
fn key_hash(key: &TextureKey) -> u64 {
    key.as_str().bytes().fold(FNV_OFFSET_BASIS, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(FNV_PRIME)
    })
}

#[cfg(test)]
#[path = "placeholder_test.rs"]
mod tests;
