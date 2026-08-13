//! What one voxel cell holds: a block, or nothing.

use mc_core::id::BlockName;

/// What one voxel cell holds: a block, or nothing.
///
/// A named type and never a bare `Option`, because every `Option` on this read
/// path already means something else — the palette's means "this palette
/// position exists", the simulation world's means "this position is inside the
/// world" — and nesting emptiness inside one of those is how a corrupt section,
/// or a position past the edge of the world, gets read as ordinary empty space.
///
/// Generic over the name the way `Option` is generic over its payload: storage
/// holds `Contents<BlockName>`, every accessor hands out `Contents<&BlockName>`,
/// and [`as_ref`](Contents::as_ref) is the one step between them. One type in
/// two forms rather than two types that can drift.
///
/// **The default type parameter does not participate in inference.** A bare
/// `Contents` means the owned form in a *type* position only; in an expression
/// position an unconstrained `Contents::Empty` is `type annotations needed`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Contents<N = BlockName> {
    /// The cell holds nothing. Not a block, not a name, not a content entity.
    Empty,
    /// The cell holds this block.
    Holds(N),
}

impl Contents<BlockName> {
    /// These contents, borrowed.
    #[must_use]
    pub fn as_ref(&self) -> Contents<&BlockName> {
        match self {
            Self::Empty => Contents::Empty,
            Self::Holds(name) => Contents::Holds(name),
        }
    }
}

impl Contents<&BlockName> {
    /// These contents, owned.
    ///
    /// One reference-count bump for a [`Holds`](Contents::Holds), and nothing at
    /// all for an [`Empty`](Contents::Empty).
    #[must_use]
    pub fn cloned(self) -> Contents<BlockName> {
        match self {
            Self::Empty => Contents::Empty,
            Self::Holds(name) => Contents::Holds(name.clone()),
        }
    }
}

impl<N> Contents<N> {
    /// Whether the cell holds nothing.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        matches!(self, Self::Empty)
    }
}
