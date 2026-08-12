//! Which array-texture layer a texture key resolves to.
//!
//! Layer indices are assigned in **lexicographic order of the texture key**, and
//! that is a decision rather than an accident of the container. A layer index
//! travels inside a packed vertex and therefore inside every golden frame, so
//! insertion order, registry-id order or hash order would each make the goldens
//! depend on something nothing in the project pins. A `BTreeMap` keyed by the
//! key itself makes the order structural: there is no comparator here that could
//! disagree with the assignment.
//!
//! The array texture itself arrives with the GPU layer. What lives here is the
//! two questions that can be answered without one: which layer a key occupies,
//! and whether every key a snapshot's blocks reference occupies one at all.

pub mod placeholder;

use std::collections::{BTreeMap, BTreeSet};

use mc_core::id::TextureKey;
use thiserror::Error;

/// The layer each texture key occupies in the array texture.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TextureLayers {
    layers: BTreeMap<TextureKey, u16>,
}

impl TextureLayers {
    /// Assigns one layer to each of `keys`, in lexicographic order.
    #[must_use]
    pub fn resolve(keys: &BTreeSet<TextureKey>) -> Self {
        let layers = keys
            .iter()
            .cloned()
            .zip(0..)
            .collect::<BTreeMap<TextureKey, u16>>();
        Self { layers }
    }

    /// The layer `key` occupies, or `None` when it was never resolved.
    #[must_use]
    pub fn layer_of(&self, key: &TextureKey) -> Option<u16> {
        self.layers.get(key).copied()
    }

    /// Every key and the layer it occupies, in lexicographic order.
    ///
    /// The array texture is filled one layer at a time and a layer's texels are
    /// generated from the key that occupies it, so whoever builds it needs the
    /// mapping in the direction `layer_of` does not answer.
    pub fn entries(&self) -> impl Iterator<Item = (&TextureKey, u16)> {
        self.layers.iter().map(|(key, layer)| (key, *layer))
    }

    /// Checks that every key the snapshot's blocks reference occupies a layer.
    ///
    /// Validates and builds nothing — the array texture itself belongs to the
    /// GPU layer, and this is the question that has to be answered before one
    /// can be built at all.
    ///
    /// # Errors
    ///
    /// Returns [`LayerError::UnresolvedKey`] naming the first unresolved key in
    /// lexicographic order. Refusing is the whole point: resolving an unknown
    /// key to layer 0 draws that block as whichever block owns layer 0, and a
    /// picture that is wrong in a plausible way is the failure nothing
    /// downstream can report.
    pub fn validate_covers(&self, block_keys: &BTreeSet<TextureKey>) -> Result<(), LayerError> {
        match block_keys
            .iter()
            .find(|key| !self.layers.contains_key(*key))
        {
            Some(key) => Err(LayerError::UnresolvedKey { key: key.clone() }),
            None => Ok(()),
        }
    }
}

/// Why the array texture cannot be built.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum LayerError {
    #[error("the texture key `{key}` has no array layer", key = key.as_str())]
    UnresolvedKey { key: TextureKey },
}

#[cfg(test)]
#[path = "mod_test.rs"]
mod tests;
