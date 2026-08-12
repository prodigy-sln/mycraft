//! Which array-texture layer a key occupies, and what happens to a block whose
//! key occupies none.
//!
//! A layer index travels inside a packed vertex and therefore inside every
//! golden frame, so *which* key gets which index is a contract rather than an
//! implementation detail. The keys below are handed over in an order that is
//! deliberately **not** lexicographic — stone, grass, dirt — and the expected
//! indices are the lexicographic ones, so an assignment that followed the order
//! it was given, a registry id, or a hash bucket lands on different numbers than
//! this test names. That the parameter is a sorted set is what makes the answer
//! structural; asserting the concrete indices is what makes it checked.
//!
//! The refusal is the other half. A block whose texture key resolved to no layer
//! has no honest index: layer 0 is stone, so substituting it draws every
//! unresolved block as stone, and a picture that is wrong in a plausible way is
//! the one failure nothing downstream can report.

use std::collections::BTreeSet;
use std::error::Error;

use mc_core::id::TextureKey;

use super::placeholder::placeholder_texels;
use super::{LayerError, TextureLayers};

type TestResult = Result<(), Box<dyn Error>>;

/// The edge length of one array layer.
const PLACEHOLDER_SIZE: u32 = 16;

/// The three keys the snapshot's blocks reference, in the order the caller
/// happens to hand them over. Lexicographically this is exactly backwards.
const SUPPLIED_ORDER: [&str; 3] = ["base:stone", "base:grass", "base:dirt"];

/// The same three keys in lexicographic order, which is the order the layers are
/// assigned in.
const LEXICOGRAPHIC_ORDER: [&str; 3] = ["base:dirt", "base:grass", "base:stone"];

/// A key no layer is generated for in the refusal scenario.
const UNGENERATED_KEY: &str = "base:grass";

/// Every key in `names`, as the set the resolver takes.
fn key_set(names: &[&str]) -> Result<BTreeSet<TextureKey>, Box<dyn Error>> {
    let mut keys = BTreeSet::new();
    for name in names {
        keys.insert(TextureKey::parse(name)?);
    }
    Ok(keys)
}

#[test]
fn three_texture_keys_take_three_distinct_layers_in_lexicographic_order() -> TestResult {
    let layers = TextureLayers::resolve(&key_set(&SUPPLIED_ORDER)?);

    let mut assigned = Vec::new();
    for name in LEXICOGRAPHIC_ORDER {
        assigned.push(layers.layer_of(&TextureKey::parse(name)?));
    }
    assert_eq!(
        assigned,
        vec![Some(0), Some(1), Some(2)],
        "three keys resolve to three distinct layers, numbered in lexicographic order of the \
         key regardless of the order they were supplied in ({SUPPLIED_ORDER:?})"
    );

    let mut sizes = BTreeSet::new();
    for name in LEXICOGRAPHIC_ORDER {
        sizes.insert(placeholder_texels(&TextureKey::parse(name)?, PLACEHOLDER_SIZE).len());
    }
    assert_eq!(
        sizes.into_iter().collect::<Vec<_>>(),
        vec![(PLACEHOLDER_SIZE as usize) * (PLACEHOLDER_SIZE as usize)],
        "an array texture has one size for all of its layers, so the three keys must produce \
         equally sized textures — one distinct size across the three, and it is 16 x 16"
    );
    Ok(())
}

#[test]
fn a_block_key_that_resolved_to_no_layer_refuses_the_array_and_names_the_key() -> TestResult {
    let layers = TextureLayers::resolve(&key_set(&["base:stone", "base:dirt"])?);
    let referenced = key_set(&["base:stone", "base:dirt", UNGENERATED_KEY])?;

    let refusal = layers.validate_covers(&referenced).err().ok_or(
        "a block referencing a texture key no layer was generated for must refuse the array; \
         resolving it to layer 0 draws that block as whichever block layer 0 belongs to",
    )?;

    match refusal {
        LayerError::UnresolvedKey { key } => assert_eq!(
            key.as_str(),
            UNGENERATED_KEY,
            "the refusal must name the key that resolved to no layer"
        ),
    }
    Ok(())
}
