//! Which layer of the array texture the block a placement would use draws its
//! swatch from, and what to say when it draws from none.
//!
//! Pure, and deliberately **outside** `src/gpu/`. Resolving a block to a layer
//! is a lookup over plain values; putting it beside the pass would take it out
//! of the coverage denominator for no gain.
//!
//! # The spelling gap is inherited here, and inherited loudly
//!
//! `build_section_geometry` matches a `BlockName` to a `TextureKey` by identical
//! spelling and consults no registry, so a block whose name is not the key its
//! texture occupies resolves to nothing at all. The indicator inherits that
//! failure rather than a wrong picture: an unresolved block draws **no
//! indicator** and is stated by name, where resolving it to layer 0 would draw
//! whichever block owns layer 0 and would look entirely plausible while being
//! wrong. Closing the gap is another increment's; refusing to paper over it is
//! this one's.
//!
//! # The fault leaves as a value, not through the frame path
//!
//! Recording a frame reports no error for this: an indicator that cannot be
//! drawn is not a dropped frame, and a frame error recurring every frame is the
//! wrong vocabulary for a content-and-registry mismatch. So the answer below
//! carries both halves — what to draw, and what to say — and whoever composes a
//! frame states the second once, however many frames go on to meet it. That also
//! keeps the sentence somewhere a test can ask for it rather than somewhere only
//! a window could produce it.

use mc_core::id::{BlockName, TextureKey};

use crate::texture::TextureLayers;

/// What the held-block indicator draws, and what is wrong when it draws
/// nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HeldSwatch {
    /// Nothing is held, so the indicator draws nothing and nothing is wrong.
    /// A client is in this state for every frame before its world lands.
    NothingHeld,
    /// The texture key whose array layer the indicator samples.
    Shows {
        /// What the swatch is drawn from.
        key: TextureKey,
    },
    /// The held block occupies no layer of the array texture.
    Unresolved {
        /// What the session holds.
        block: BlockName,
        /// The key that block's own name spells, or nothing where its name is
        /// not a texture key at all.
        key: Option<TextureKey>,
    },
}

impl HeldSwatch {
    /// The texture a composed frame carries, or `None` where the indicator draws
    /// nothing.
    ///
    /// An owned key rather than a borrow, so a frame built from this answer is
    /// not tied to the lifetime of the lookup that produced it.
    #[must_use]
    pub fn texture(&self) -> Option<TextureKey> {
        match self {
            Self::Shows { key } => Some(key.clone()),
            Self::NothingHeld | Self::Unresolved { .. } => None,
        }
    }

    /// What to state about a held block that draws no indicator, or `None` when
    /// there is nothing to state.
    ///
    /// It names the block, because the block is what somebody looking at a
    /// missing swatch can act on: a key is derived from a name, and a report
    /// quoting only the derived half leaves the reader to guess which of them
    /// was wrong.
    #[must_use]
    pub fn unresolved_report(&self) -> Option<String> {
        match self {
            Self::NothingHeld | Self::Shows { .. } => None,
            Self::Unresolved {
                block,
                key: Some(key),
            } => Some(format!(
                "the held block `{block}` draws no indicator: its texture key `{key}` occupies no \
                 layer of the array texture",
                block = block.as_str(),
                key = key.as_str()
            )),
            Self::Unresolved { block, key: None } => Some(format!(
                "the held block `{block}` draws no indicator: its name is not a texture key",
                block = block.as_str()
            )),
        }
    }
}

/// What the indicator draws for `held`, against the layers the array texture was
/// filled from.
///
/// The key is the block's own name, spelled as one — the identical-spelling match
/// this module's header is about. A name that is not a namespaced id at all and a
/// key that occupies no layer are both unresolved, because both leave the same
/// thing to draw and the same thing to say.
#[must_use]
pub fn held_swatch(held: Option<&BlockName>, layers: &TextureLayers) -> HeldSwatch {
    let Some(block) = held else {
        return HeldSwatch::NothingHeld;
    };
    let Ok(key) = TextureKey::parse(block.as_str()) else {
        return HeldSwatch::Unresolved {
            block: block.clone(),
            key: None,
        };
    };
    if layers.layer_of(&key).is_some() {
        HeldSwatch::Shows { key }
    } else {
        HeldSwatch::Unresolved {
            block: block.clone(),
            key: Some(key),
        }
    }
}

#[cfg(test)]
#[path = "held_test.rs"]
mod tests;
