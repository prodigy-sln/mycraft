//! What a content root's art offers, asked of the value that carries it.
//!
//! **Nothing consumes this yet, and that is exactly why it is tested now.** The
//! type exists because the client's `built_set` returns it, and in this increment
//! only `none()` is ever constructed — so `stating` and `covering` are a surface
//! nobody has driven. A surface nobody has driven is one whose first consumer
//! finds out what it does by writing the consumer, which is the wrong order and
//! the reason this crate's own rules give a pure function no exemption for being
//! unwired.
//!
//! **`covering` answering `None` is an ordinary answer, not a failure**, and the
//! two readings below are the pair that says so: a key that was supplied comes
//! back with its texels, a key that was not comes back empty, and a supply that
//! covers nothing answers empty for a key that exists elsewhere. Without the
//! third, a `covering` that had come to answer `None` unconditionally would still
//! satisfy the second.

use std::error::Error;

use mc_core::id::TextureKey;

use super::SuppliedTexels;

type TestResult = Result<(), Box<dyn Error>>;

/// A key a set covers, and one it does not.
const SUPPLIED: &str = "base:grass_top";
const UNSUPPLIED: &str = "base:nobody_baked_this";

/// The texels the supplied key is given: four, and pairwise distinct, so a
/// `covering` handing back the wrong entry, a truncated one, or one in the wrong
/// order cannot read as the right answer.
const FOUR_DISTINCT_TEXELS: [[u8; 4]; 4] = [
    [10, 20, 30, 255],
    [40, 50, 60, 255],
    [70, 80, 90, 255],
    [100, 110, 120, 128],
];

#[test]
fn texels_stated_for_a_key_come_back_for_that_key_unchanged() -> TestResult {
    let supplied = SuppliedTexels::stating([(key(SUPPLIED)?, FOUR_DISTINCT_TEXELS.to_vec())]);

    let covered = supplied.covering(&key(SUPPLIED)?);

    assert_eq!(covered, Some(FOUR_DISTINCT_TEXELS.as_slice()));
    Ok(())
}

#[test]
fn a_key_nothing_supplied_texels_for_is_covered_by_nothing() -> TestResult {
    // Stated alongside a key that *is* supplied, so what is being read is the
    // absence of one entry rather than the absence of every entry — which is
    // what the reading below covers instead.
    let supplied = SuppliedTexels::stating([(key(SUPPLIED)?, FOUR_DISTINCT_TEXELS.to_vec())]);

    let covered = supplied.covering(&key(UNSUPPLIED)?);

    assert_eq!(covered, None);
    Ok(())
}

#[test]
fn a_root_declaring_no_art_supplies_nothing_for_a_key_another_root_would_cover() -> TestResult {
    // The key is one the shipped manifest does bake, so this says the emptiness
    // is the supply's and not the key's.
    let supplied = SuppliedTexels::none();

    let covered = supplied.covering(&key(SUPPLIED)?);

    assert_eq!(covered, None);
    Ok(())
}

/// The texture key `spelled` names.
///
/// # Errors
///
/// Returns an error if it is not a key a declaration could write, which would
/// make the reading above about this file's spelling rather than about the supply.
fn key(spelled: &str) -> Result<TextureKey, Box<dyn Error>> {
    Ok(TextureKey::parse(spelled)?)
}
