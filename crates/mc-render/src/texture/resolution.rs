//! What a block's faces draw and how much light they stop: the block→appearance
//! map and the layer assignment, as one value.
//!
//! **One type rather than two values travelling side by side**, and that is the
//! whole reason this module exists. Both consumers of the question — the packer
//! building a section's vertices and the held-block indicator — need
//! block + facing → key → layer, so the map and the assignment travel together
//! through `PreparedScene`, `Unuploaded`, `Retained`, the re-mesh worker's
//! retirement and `FrameRenderer`. Carried loose, a batch packed with a reload's
//! *new* keys against its *old* layers resolves to a wrong-but-valid layer: a
//! plausible wrong picture, with no error anywhere.
//!
//! [`TextureLayers`] is unchanged and is what the array texture is filled from.
//! What travels is the pair; what fills the device is still the half that always
//! did.
//!
//! **There is deliberately no content serial here, and the absence is
//! load-bearing.** A bundled value invites being stamped with one so that
//! "packed against the content serving" becomes checkable — and it must not be.
//! The re-mesh worker keeps the whole meshed list for the run and re-packs all
//! of it on every batch, against whatever resolution it currently holds, so a
//! section nobody re-meshed is re-packed under the *newer* content on purpose.
//! A serial checked here would refuse exactly the case that path exists for.
//!
//! A block the map states nothing about resolves to nothing rather than to a
//! default. A section may still hold quads for a block a reload dropped, and
//! inventing a key for one — from its name, or from any other block's row —
//! draws a picture that is wrong in an entirely plausible way.
//!
//! **The declared opacity travels in the same row as the keys, for the same
//! reason.** It decides which of the two terrain draws a face lands in, so a
//! resolution answering "this block's keys" and "this block's opacity" out of
//! two maps could answer the first and not the second — and a face resolved to a
//! layer but not to a degree is exactly the plausible wrong picture the paragraph
//! above is about. One row means the two answers are `Some` and `None` together.

use std::collections::BTreeMap;

use mc_core::block::Opacity;
use mc_core::content::{Face, FaceTextures};
use mc_core::id::{BlockName, TextureKey};

use super::TextureLayers;

/// Everything about one block a packer needs: what it draws on each facing, and
/// how much light it stops.
#[derive(Debug, Clone, PartialEq, Eq)]
struct BlockAppearance {
    textures: FaceTextures,
    opacity: Opacity,
}

/// What each block draws on each of its facings, how much light each stops, and
/// which layer each key occupies.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TextureResolution {
    blocks: BTreeMap<BlockName, BlockAppearance>,
    layers: TextureLayers,
}

impl TextureResolution {
    /// The resolution stating `blocks` over `layers`.
    ///
    /// Neither half is derived from the other: the declarations are what content
    /// wrote and the assignment is what the session handed out, and a
    /// constructor computing one from the other would make every consumer agree
    /// with it by construction.
    ///
    /// A block arrives as its name, its six keys and its declared opacity
    /// together, so there is no order of calls in which a block has one and not
    /// the other.
    #[must_use]
    pub fn stating(
        blocks: impl IntoIterator<Item = (BlockName, FaceTextures, Opacity)>,
        layers: TextureLayers,
    ) -> Self {
        Self {
            blocks: blocks
                .into_iter()
                .map(|(name, textures, opacity)| (name, BlockAppearance { textures, opacity }))
                .collect(),
            layers,
        }
    }

    /// The key `block` draws on `face`, or `None` where the content states no
    /// such block.
    ///
    /// Total in the face and partial in the block, which is the honest shape:
    /// every declaration states all six facings, and no declaration at all is
    /// the case that has to stay tellable.
    #[must_use]
    pub fn key_of(&self, block: &BlockName, face: Face) -> Option<&TextureKey> {
        self.blocks
            .get(block)
            .map(|appearance| appearance.textures.at(face))
    }

    /// How much light `block` stops, or `None` where the content states no such
    /// block.
    ///
    /// Partial in the same blocks [`key_of`](Self::key_of) is partial in, and
    /// deliberately not defaulted to [`Opacity::OPAQUE`]: a block the resolution
    /// never heard of is a block that draws nothing, and answering "it is
    /// opaque" for one would put a face nobody resolved into the opaque draw.
    #[must_use]
    pub fn opacity_of(&self, block: &BlockName) -> Option<Opacity> {
        self.blocks.get(block).map(|appearance| appearance.opacity)
    }

    /// The layers the array texture is filled from.
    #[must_use]
    pub const fn layers(&self) -> &TextureLayers {
        &self.layers
    }
}
