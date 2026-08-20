//! The texels a content root's built art offers, one entry per texture key.
//!
//! **The domain-shaped value between the decoder and the mip chain.** The client
//! reads the built set and decodes its PNGs; the array texture is filled from mip
//! levels computed here in `mc-render`. Neither half may name the other's crate,
//! so what crosses between them is this: a key, and the texels of its level zero
//! in `[R, G, B, A]` stored bytes.
//!
//! **A key nobody supplied is an ordinary answer, not a failure.** A mod author's
//! first block declares a texture the set does not cover, and what they get is a
//! generated texture rather than a refused launch — so [`covering`](SuppliedTexels::covering)
//! answers `None` and the caller falls back. The refusal a launch does make is
//! about the *set*, and it is the client's.
//!
//! Nothing consumes this yet. It exists now because the signature that returns it
//! is the one the client's tests already bind, and narrowing that signature would
//! make the phase which fills it widen a surface it does not own.

use std::collections::BTreeMap;

use mc_core::id::TextureKey;

/// The level-zero texels supplied for each key a built set covers.
///
/// A `BTreeMap` for the reason [`TextureLayers`](super::TextureLayers) is one:
/// iteration order is structural rather than a comparator somebody chose, so two
/// runs over the same set walk it the same way.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SuppliedTexels {
    supplied: BTreeMap<TextureKey, Vec<[u8; 4]>>,
}

impl SuppliedTexels {
    /// The texels `entries` states, one entry per key.
    ///
    /// A key stated twice keeps the last statement, which is the only answer a
    /// map can give; an index refuses a duplicate key before anything reaches
    /// here, so there is no second opinion for this to have to hold.
    #[must_use]
    pub fn stating(entries: impl IntoIterator<Item = (TextureKey, Vec<[u8; 4]>)>) -> Self {
        Self {
            supplied: entries.into_iter().collect(),
        }
    }

    /// What a content root declaring no art supplies.
    #[must_use]
    pub fn none() -> Self {
        Self::default()
    }

    /// The level-zero texels supplied for `key`, or `None` where nothing was.
    #[must_use]
    pub fn covering(&self, key: &TextureKey) -> Option<&[[u8; 4]]> {
        self.supplied.get(key).map(Vec::as_slice)
    }
}

#[cfg(test)]
#[path = "supplied_test.rs"]
mod tests;
