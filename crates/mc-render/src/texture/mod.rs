//! Which array-texture layer a texture key occupies.
//!
//! **The assignment is stated where the content is read, and honoured here.**
//! [`TextureLayers::stated`] is the production path: it takes the layer each key
//! occupies as given and checks it against nothing, because checking would be a
//! derivation written a second time and would refuse exactly the assignments the
//! arrangement exists to accept. [`TextureLayers::resolve`], which does hand out
//! indices in lexicographic order of the key, survives as a convenience for tests
//! that need *an* assignment rather than a particular one — it has no production
//! caller.
//!
//! **A layer index travels inside a packed vertex and therefore inside every
//! golden frame.** That used to be the argument for the sort being a decision
//! rather than an accident of the container; it is now the argument for the
//! *assignment* having to be stable. A renderer that worked its own out from a
//! key set would renumber every index after any block it and the content's reader
//! disagreed about — silently, with no error anywhere, and not localised to the
//! block they disagreed about. `docs/technical/architecture.md` §"The layer
//! assignment is stated, not derived" holds the reasoning.
//!
//! A `BTreeMap` keyed by the key itself is still what backs both constructors, so
//! iteration order is structural and no comparator here can disagree with what it
//! was handed.
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
    ///
    /// **No production caller, and that is stated here rather than left for a
    /// reviewer to notice.** What ships is [`stated`](Self::stated); this is for
    /// tests that need *an* assignment rather than a particular one. Reaching for
    /// it on a path a frame is drawn through is the derivation the seam exists to
    /// remove.
    #[must_use]
    pub fn resolve(keys: &BTreeSet<TextureKey>) -> Self {
        let layers = keys
            .iter()
            .cloned()
            .zip(0..)
            .collect::<BTreeMap<TextureKey, u16>>();
        Self { layers }
    }

    /// Layers exactly as `assignment` states them.
    ///
    /// **Honouring an answer rather than reproducing a decision**, which is the
    /// difference between this and [`resolve`](Self::resolve). A layer index
    /// rides inside every packed vertex, so a renderer that worked its own out
    /// from a key set would renumber every index after any block it and the
    /// content's reader disagreed about — silently, with no error anywhere, and
    /// not localised to the block they disagreed about.
    ///
    /// Nothing here checks the assignment against a sort. Checking would be the
    /// same derivation written twice and would refuse exactly the assignments
    /// this exists to accept.
    #[must_use]
    pub fn stated(assignment: impl IntoIterator<Item = (TextureKey, u16)>) -> Self {
        Self {
            layers: assignment.into_iter().collect(),
        }
    }

    /// The layer `key` occupies, or `None` where the assignment names none for
    /// it.
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
