//! What the client draws and meshes from, built from the content it was handed.
//!
//! # Built from the resolved value and from nothing else
//!
//! The simulation reads a content root; the client receives what came back.
//! `docs/planning/client-server-split.md` is the binding reasoning and is not
//! re-derived here — the rule it settles is that **the client never evaluates
//! anything any other participant, the server included, must agree with**, and a
//! content set is the sharpest case of that there is.
//!
//! So this type takes a [`mc_core::content::ResolvedContent`] and reads nothing
//! else: no registry,
//! no path, no scripting host. That is the single property distinguishing this
//! seam from a rename, and the two failures it rules out are worth naming
//! because each would leave every scenario about content green while nothing had
//! been cut — a resolved value that is a newtype over the registry, and a view
//! that reaches back through one.
//!
//! # The assignment is honoured, never re-derived
//!
//! Layers used to be handed out as a key's position in the lexicographically
//! sorted key set. **A layer index rides inside every packed vertex**, so under
//! derivation inserting one block renumbers every index after it and the whole
//! world is textured wrong — silently, with no error anywhere, and not localised
//! to the block that caused it. That is a live defect on hot reload, in one
//! process, rather than anything to do with networking.
//!
//! Nothing here checks the stated assignment against a sort. Checking would be
//! the same derivation written a second time, and it would refuse exactly the
//! assignments this exists to accept.

use std::collections::BTreeMap;

use mc_core::content::ResolvedContent;
use mc_core::id::BlockName;
use mc_render::texture::TextureLayers;

/// The client's view of the content it was handed.
#[derive(Debug)]
pub struct ContentView {
    layers: TextureLayers,
    /// Solidity by block name, which is what a mesher culls faces by.
    solidity: BTreeMap<BlockName, bool>,
}

impl ContentView {
    /// The view of `content`.
    #[must_use]
    pub fn of(content: &ResolvedContent) -> Self {
        Self {
            layers: TextureLayers::stated(
                content
                    .layer_assignment()
                    .map(|(key, layer)| (key.clone(), layer)),
            ),
            solidity: content
                .blocks()
                .map(|block| (block.name.clone(), block.is_solid))
                .collect(),
        }
    }

    /// The layers the array texture is filled from and packed against.
    #[must_use]
    pub fn layers(&self) -> &TextureLayers {
        &self.layers
    }

    /// The layers, taken out of the view.
    ///
    /// A scene records the layers it was packed against, so the two travel
    /// together from here rather than the caller asking twice.
    #[must_use]
    pub fn into_layers(self) -> TextureLayers {
        self.layers
    }

    /// Whether `block` is solid, or `None` where the content states no such
    /// block.
    ///
    /// **`None` rather than a default**, because a view that answered for a
    /// block it was never handed would be inventing content — which is the whole
    /// failure this seam exists to make impossible.
    ///
    /// **No production caller yet, and that is stated rather than left to be
    /// found.** The mesher still culls against the `BlockRegistry` that is still
    /// travelling to the client, which is the residue of one binary hosting both
    /// halves that the seam's own record already states. This is what the mesher
    /// reads once the registry stops travelling.
    #[must_use]
    pub fn is_solid(&self, block: &BlockName) -> Option<bool> {
        self.solidity.get(block).copied()
    }
}
