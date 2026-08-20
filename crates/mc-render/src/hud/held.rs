//! Which layer of the array texture the block a placement would use draws its
//! swatch from, and what to say when it draws from none.
//!
//! Pure, and deliberately **outside** `src/gpu/`. Resolving a block to a layer
//! is a lookup over plain values; putting it beside the pass would take it out
//! of the coverage denominator for no gain.
//!
//! # One lookup, two consumers, and this is the second site
//!
//! A drawn face and this indicator ask one question: which key does *this block*
//! draw on *this facing*. Both read it out of the block's declaration through a
//! [`TextureResolution`](crate::texture::TextureResolution), and neither parses
//! a block's name. Closing one site and leaving the other would leave a block
//! drawing correctly in the world with a blank indicator beside it, which reads
//! as a HUD fault and sends whoever chases it to the wrong module.
//!
//! A block the resolution states nothing about, and a declared key that occupies
//! no layer, are both **unresolved**: the indicator draws nothing and says so by
//! name. Resolving either to layer 0 would draw whichever block owns layer 0 and
//! would look entirely plausible while being wrong.
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

use mc_core::content::Face;
use mc_core::id::{BlockName, TextureKey};

use crate::texture::TextureResolution;

/// The facing the indicator draws.
///
/// **A side face, chosen once and written down.** A side is what makes the
/// canonical block recognisable — a grass block's side carries both the growth
/// and the earth, where its top is a green square that says "grass" only to
/// somebody who already knows. The four sides are interchangeable for that
/// purpose, so `north` is arbitrary; what matters is that it is stated rather
/// than implied by whichever facing a lookup happened to reach for.
pub const INDICATOR_FACE: Face = Face::North;

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
        /// The facing the key was declared against.
        face: Face,
    },
    /// The held block draws nothing on the facing the indicator looks at.
    Unresolved {
        /// What the session holds.
        block: BlockName,
        /// The facing that was looked at.
        face: Face,
        /// The key the block declares there, or nothing where the content states
        /// no such block at all.
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
            Self::Shows { key, .. } => Some(key.clone()),
            Self::NothingHeld | Self::Unresolved { .. } => None,
        }
    }

    /// What to state about a held block that draws no indicator, or `None` when
    /// there is nothing to state.
    ///
    /// It names the block, the facing and the key. A block declares up to six
    /// keys and only one of them is the one without art, so a sentence naming
    /// the block and the key but not the facing leaves the author unable to tell
    /// a mistake in their declaration from a missing image.
    #[must_use]
    pub fn unresolved_report(&self) -> Option<String> {
        match self {
            Self::NothingHeld | Self::Shows { .. } => None,
            Self::Unresolved {
                block,
                face,
                key: Some(key),
            } => Some(format!(
                "the held block `{block}` draws no indicator: the key `{key}` it declares \
                 against `{face}` occupies no layer of the array texture",
                block = block.as_str(),
                key = key.as_str(),
                face = face.as_str()
            )),
            Self::Unresolved {
                block,
                face,
                key: None,
            } => Some(format!(
                "the held block `{block}` draws no indicator: the content states no such block, \
                 so it declares nothing against `{face}`",
                block = block.as_str(),
                face = face.as_str()
            )),
        }
    }
}

/// What the indicator draws for `held`, resolved through the same declarations
/// the world's faces are packed from.
///
/// The key is the one `held` declares against [`INDICATOR_FACE`]. A block the
/// content states nothing about and a declared key that occupies no layer are
/// both unresolved, because both leave the same thing to draw and the same thing
/// to say — but the two sentences differ, and which one a reader gets is the
/// difference between a wrong declaration and a missing assignment.
#[must_use]
pub fn held_swatch(held: Option<&BlockName>, resolution: &TextureResolution) -> HeldSwatch {
    let Some(block) = held else {
        return HeldSwatch::NothingHeld;
    };
    let declared = resolution.key_of(block, INDICATOR_FACE);
    match declared.filter(|key| resolution.layers().layer_of(key).is_some()) {
        Some(key) => HeldSwatch::Shows {
            key: key.clone(),
            face: INDICATOR_FACE,
        },
        None => HeldSwatch::Unresolved {
            block: block.clone(),
            face: INDICATOR_FACE,
            key: declared.cloned(),
        },
    }
}

#[cfg(test)]
#[path = "held_test.rs"]
mod tests;
