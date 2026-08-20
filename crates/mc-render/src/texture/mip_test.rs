//! The mip chain as arithmetic: the sRGB transfer pair, the box average in
//! linear light, the chain of levels, and the layer that offers too few of them.
//!
//! **Nothing here is wired, and every number here is derived rather than
//! observed.** The array texture is `Rgba8UnormSrgb` and decodes to linear on
//! sample, so a level averaged over *stored* bytes comes out darker than the
//! level above it — the classic sRGB mipping fault, plausible-looking and wrong
//! in the direction nothing notices. The two implementations answer differently
//! at exactly one place a test can stand: the stored byte halfway between 0 and
//! 255. Averaging in linear light gives 188; averaging the stored bytes gives
//! 128. That byte is the whole reason `two_stored_zeroes_and_two_stored_maxima`
//! exists, and it is written from the specification rather than softened to a
//! range that would span the two.
//!
//! **Where the expected bytes come from.** Every expected value below was
//! derived offline from IEC 61966-2-1 — decode each stored byte to linear,
//! average, re-encode, round to nearest — by a program sharing no code with this
//! crate, and the arithmetic path was measured before any assertion was chosen.
//! Exact byte equality is what the readings support: the transfer pair
//! round-trips all 256 stored bytes with a worst pre-rounding error of
//! **1.53e-5 of a byte**, at stored 132. The narrowest margin any expected value
//! below holds against a rounding boundary is **0.016 of a byte** — that is 188
//! itself, whose pre-rounding value is 187.516 — and the narrowest among the
//! sixteen channel readings of the 4 x 4 fixture is 0.15, at 85.349. Three
//! orders of magnitude clear at the tightest point, so a tolerance would blunt
//! these assertions without buying anything. The derivations are recorded in the
//! spec folder's `test-map.md`.
//!
//! **The pair that matters most.** A uniform image averages to itself however
//! the four texels are chosen, so `an_image_of_one_colour` stays green under an
//! off-by-one in the selection. `each_texel_of_the_reduced_level` is what
//! catches it, which is why its fixture holds sixteen pairwise-distinct texels
//! and why the scenario it comes from says *and not of any other four*. Neither
//! reading covers the other.

use std::error::Error;

use mc_core::content::TEXTURE_EDGE;
use mc_core::id::TextureKey;

use super::{chain, levels_for, reduced, to_linear, to_stored};
use crate::texture::placeholder::placeholder_texels;
use crate::texture::supplied::SuppliedTexels;
use crate::texture::{MIP_LEVELS, TextureError};

type TestResult = Result<(), Box<dyn Error>>;

/// The edge lengths a 16 x 16 image's chain holds, written from the scenario.
const DECLARED_EDGES: [u32; 5] = [16, 8, 4, 2, 1];

/// The stored byte the linear-light average of 0 and 255 re-encodes to.
///
/// Averaging the stored bytes instead gives 128, which is why this is pinned
/// exactly rather than as "midway between".
const HALFWAY_IN_LINEAR_LIGHT: u8 = 188;

/// Channels held constant across a fixture, so that a reading is about the
/// channel that varies. Both survive a round trip through linear and back.
const HELD_GREEN: u8 = 64;
const HELD_BLUE: u8 = 128;

/// Block textures carry no transparency in this increment.
const OPAQUE: u8 = 255;

/// A key the shipped manifest bakes, and one the spec names as unauthored.
const AUTHORED: &str = "base:grass_top";
const UNAUTHORED: &str = "example:undrawn";

/// A 4 x 4 image reduces to 4, 2 and 1 — three levels against the five the
/// array texture declares.
const SHORT_EDGE: u32 = 4;
const LEVELS_A_FOUR_EDGE_OFFERS: usize = 3;

#[test]
fn a_sixteen_texel_image_prepares_five_levels_sized_sixteen_eight_four_two_and_one() {
    let level_zero = climbing(TEXTURE_EDGE);

    let levels = chain(&level_zero, TEXTURE_EDGE);

    let counted: Vec<usize> = levels.iter().map(Vec::len).collect();
    let declared: Vec<usize> = DECLARED_EDGES.iter().map(|edge| texels_in(*edge)).collect();
    assert_eq!(counted, declared);
}

#[test]
fn two_stored_zeroes_and_two_stored_maxima_reduce_to_stored_one_eight_eight() {
    // Red is the channel that varies; the other three are held, so what is read
    // is one channel's average rather than a texel that happens to be right.
    let level_zero = [
        [0, HELD_GREEN, HELD_BLUE, OPAQUE],
        [255, HELD_GREEN, HELD_BLUE, OPAQUE],
        [255, HELD_GREEN, HELD_BLUE, OPAQUE],
        [0, HELD_GREEN, HELD_BLUE, OPAQUE],
    ];

    let level_one = reduced(&level_zero, 2);

    assert_eq!(
        level_one,
        vec![[HALFWAY_IN_LINEAR_LIGHT, HELD_GREEN, HELD_BLUE, OPAQUE]]
    );
}

#[test]
fn an_image_of_one_colour_reduces_to_that_colour_at_every_level() {
    // Three channels no two of which are equal, so a level that copied one
    // channel across the others cannot read as the colour having survived.
    let colour = [37, 158, 211, OPAQUE];
    let level_zero = vec![colour; texels_in(TEXTURE_EDGE)];

    let levels = chain(&level_zero, TEXTURE_EDGE);

    let unchanged: Vec<Vec<[u8; 4]>> = DECLARED_EDGES
        .iter()
        .map(|edge| vec![colour; texels_in(*edge)])
        .collect();
    assert_eq!(levels, unchanged);
}

#[test]
fn each_texel_of_the_reduced_level_averages_exactly_the_four_texels_it_covers() {
    // Row-major, so the four output texels cover sources 0/1/4/5, 2/3/6/7,
    // 8/9/12/13 and 10/11/14/15. All sixteen sources are pairwise distinct, so
    // any other four — a selection offset by one, or four consecutive in
    // row-major order — averages to something else. Both were measured.
    let level_zero = four_by_four();

    let level_one = reduced(&level_zero, SHORT_EDGE);

    assert_eq!(
        level_one,
        vec![
            [55, 216, HELD_BLUE, OPAQUE],
            [85, 183, HELD_BLUE, OPAQUE],
            [183, 85, HELD_BLUE, OPAQUE],
            [216, 55, HELD_BLUE, OPAQUE],
        ]
    );
}

#[test]
fn a_layer_offered_fewer_levels_than_declared_is_refused_naming_the_key_and_the_count() -> TestResult
{
    let key = key(AUTHORED)?;
    let supplied = SuppliedTexels::stating([(key.clone(), climbing(SHORT_EDGE))]);

    let refused = levels_for(&key, &supplied, SHORT_EDGE);

    let expected = TextureError::TooFewLevels {
        key: key.clone(),
        offered: LEVELS_A_FOUR_EDGE_OFFERS,
        declared: MIP_LEVELS as usize,
    };
    assert_eq!(refused, Err(expected.clone()));
    let named = expected.to_string();
    assert!(
        named.contains(AUTHORED) && named.contains(&LEVELS_A_FOUR_EDGE_OFFERS.to_string()),
        "the refusal must name the key and the levels offered, and reads `{named}`"
    );
    Ok(())
}

#[test]
fn a_stored_byte_survives_a_round_trip_through_linear_and_back() {
    let every_stored_byte: Vec<u8> = (0..=u8::MAX).collect();

    let round_tripped: Vec<u8> = every_stored_byte
        .iter()
        .map(|stored| to_stored(to_linear(*stored)))
        .collect();

    // Compared whole rather than counted, so a transfer pair that collapsed a
    // range onto one byte is read as the bytes it lost and not as a tally.
    assert_eq!(round_tripped, every_stored_byte);
}

#[test]
fn the_declared_level_count_is_derived_from_the_texture_edge() {
    let level_zero = climbing(TEXTURE_EDGE);

    let levels = chain(&level_zero, TEXTURE_EDGE);

    // Against what the halving actually produced, never against `ilog2` written
    // a second time: a level count that can disagree with the edge it is taken
    // from is a copy that overruns.
    assert_eq!(MIP_LEVELS as usize, levels.len());
}

#[test]
fn supplied_texels_of_the_wrong_count_are_refused_naming_the_key() -> TestResult {
    let key = key(AUTHORED)?;
    let mut one_short = climbing(TEXTURE_EDGE);
    one_short.pop();
    let supplied = SuppliedTexels::stating([(key.clone(), one_short)]);

    let refused = levels_for(&key, &supplied, TEXTURE_EDGE);

    assert_eq!(
        refused,
        Err(TextureError::WrongTexelCount {
            key,
            offered: texels_in(TEXTURE_EDGE) - 1,
            declared: texels_in(TEXTURE_EDGE),
        })
    );
    Ok(())
}

#[test]
fn a_key_the_supply_covers_is_levelled_from_the_supplied_texels() -> TestResult {
    let key = key(AUTHORED)?;
    let art = climbing(TEXTURE_EDGE);
    let supplied = SuppliedTexels::stating([(key.clone(), art.clone())]);

    let levels = levels_for(&key, &supplied, TEXTURE_EDGE)?;

    assert_ne!(
        art,
        placeholder_texels(&key, TEXTURE_EDGE),
        "the supplied art must differ from what the key generates, or a levelling ignoring the supply reads as correct"
    );
    assert_eq!(levels.first(), Some(&art));
    Ok(())
}

#[test]
fn a_key_the_supply_does_not_cover_falls_back_to_its_generated_texels() -> TestResult {
    let unauthored = key(UNAUTHORED)?;
    // Stated alongside a key that *is* covered, so the fallback is reached by
    // this key's absence rather than by the supply being empty.
    let supplied = SuppliedTexels::stating([(key(AUTHORED)?, climbing(TEXTURE_EDGE))]);

    let levels = levels_for(&unauthored, &supplied, TEXTURE_EDGE)?;

    assert_eq!(
        levels.first(),
        Some(&placeholder_texels(&unauthored, TEXTURE_EDGE))
    );
    Ok(())
}

/// How many texels an `edge` x `edge` level holds.
fn texels_in(edge: u32) -> usize {
    (edge * edge) as usize
}

/// An `edge` x `edge` image whose texels climb through the stored range.
///
/// No level of its chain is uniform, so a level copied from another — or a chain
/// answering with its own input at every level — cannot pass for it.
fn climbing(edge: u32) -> Vec<[u8; 4]> {
    (0..edge * edge)
        .map(|index| {
            let step = (index % 256) as u8;
            [step, 255 - step, HELD_BLUE, OPAQUE]
        })
        .collect()
}

/// Sixteen pairwise-distinct texels: red climbs by 17 a step from 0 to 255 and
/// green falls by the same, so no four of them average to what another four do.
fn four_by_four() -> Vec<[u8; 4]> {
    (0..SHORT_EDGE * SHORT_EDGE)
        .map(|index| {
            let step = index * 17;
            [step as u8, (255 - step) as u8, HELD_BLUE, OPAQUE]
        })
        .collect()
}

/// The texture key `spelled` names.
///
/// # Errors
///
/// Returns an error if it is not a key a declaration could write, which would
/// make a reading above about this file's spelling rather than about the chain.
fn key(spelled: &str) -> Result<TextureKey, Box<dyn Error>> {
    Ok(TextureKey::parse(spelled)?)
}
