//! The content a participant receives once somebody else has read a content
//! root.
//!
//! # What this carries, and what it deliberately does not
//!
//! A block's **name**, its **texture key** and its **solidity**, plus the
//! **layer assignment** the renderer draws against. Those are what a client
//! draws and predicts with.
//!
//! It carries none of `replaceable`, `breakable` or `breaks_into`. Those are the
//! rules by which a world is **mutated**, the server recomputes every one of
//! them, and a client holding them would be holding rules it may not apply. That
//! absence is asserted by discrimination rather than by inspection — two content
//! roots differing in nothing but those three resolve to values that compare
//! equal — because a type that simply has no such field cannot fail a test about
//! not having one.
//!
//! # Why the layer assignment travels rather than being derived
//!
//! **A layer index rides inside every packed vertex.** Derived as a key's
//! position in a sorted key set, inserting one block renumbers every index after
//! it and the whole world is textured wrong — silently, with no error anywhere,
//! and not localised to the block that caused it. That is not a networking
//! concern: it is a live defect on hot reload, in one process.
//!
//! So the assignment is **stated** by whoever read the content and **honoured**
//! by whoever draws. The falsifier is the one thing separating that from a
//! rename: hand a client an assignment that is deliberately *not* the positional
//! order and the rendered indices follow it.
//!
//! # Why this lives in `mc-core`
//!
//! It is a content primitive with no I/O, which is what this crate is for — and
//! `mc-render` has to be able to accept a stated assignment while never naming
//! the simulation. Putting the value in `mc-sim` would have made the renderer
//! reach for a crate the dependency rules forbid it.
//!
//! # What is deliberately absent
//!
//! No identity, digest or hash of the content set. With one process nothing can
//! disagree, so nothing could falsify such a field, and a test that cannot fail
//! reads as evidence and is not. The moment it becomes falsifiable is the moment
//! a second participant exists. **This is the opposite of the layer assignment**,
//! which is here precisely because its consumer exists today: the mesher and the
//! packer read it on every frame this project already draws.

use crate::id::{BlockName, TextureKey};

/// One block, as a participant that only draws needs it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedBlock {
    /// What the block is called.
    pub name: BlockName,
    /// The key its faces are drawn from. **Never a file path**: what pixels a
    /// key resolves to is the renderer's concern.
    pub texture: TextureKey,
    /// Whether it stops a player, which is also what the mesher culls faces by.
    pub is_solid: bool,
}

/// Everything a content root declares that a client needs, and nothing it does
/// not.
///
/// See the module note for what is absent and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedContent {
    /// In registration order, which is the order the reader established.
    blocks: Vec<ResolvedBlock>,
    /// Key to layer. A pair list rather than a map, because the order it is
    /// stated in is the stater's and nothing reading it may depend on a map's.
    layers: Vec<(TextureKey, u16)>,
}

impl ResolvedContent {
    /// Content stating `blocks` and assigning `layers`.
    ///
    /// **Both are stated rather than derived from each other**, which is the
    /// whole point: a constructor that assigned layers by sorting the blocks
    /// would make every consumer agree with a sort by construction, and the
    /// disagreement a test needs to be able to express would be unexpressible.
    pub fn stating(
        blocks: impl IntoIterator<Item = ResolvedBlock>,
        layers: impl IntoIterator<Item = (TextureKey, u16)>,
    ) -> Self {
        Self {
            blocks: blocks.into_iter().collect(),
            layers: layers.into_iter().collect(),
        }
    }

    /// Every block, in registration order.
    pub fn blocks(&self) -> impl Iterator<Item = &ResolvedBlock> {
        self.blocks.iter()
    }

    /// Which layer each texture key occupies.
    pub fn layer_assignment(&self) -> impl Iterator<Item = (&TextureKey, u16)> {
        self.layers.iter().map(|(key, layer)| (key, *layer))
    }
}
