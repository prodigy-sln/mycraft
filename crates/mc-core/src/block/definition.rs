//! What a block definition is, where it was declared, and the dense id a
//! registry assigns it.

use crate::content::FaceTextures;
use crate::id::BlockName;

/// Where a definition was declared, as an opaque human-readable label.
///
/// Opaque on purpose. This crate performs no I/O and must not learn what a file
/// or a content root is, so a script chunk name is exactly as expressible here as
/// a file path. It exists to be quoted back to whoever wrote the definition when
/// something about it is wrong.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefinitionOrigin(String);

impl DefinitionOrigin {
    /// Labels an origin.
    pub fn new(label: impl Into<String>) -> Self {
        Self(label.into())
    }

    /// The label as it was given.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Everything the engine knows about a block, and all of it comes from content.
///
/// The origin travels with the definition rather than with the batch it arrived
/// in, because a duplicate name must be reported against both the place that
/// declared it first and the place that declared it again — which needs the
/// first origin still to be known when the second arrives.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockDefinition {
    pub name: BlockName,
    /// The key each of its six faces draws from.
    ///
    /// A declaration states one key for all six or a key per facing, and the two
    /// forms arrive here as one value — so nothing downstream branches on which
    /// of them was written.
    pub textures: FaceTextures,
    /// Whether this block stops a player who walks into it.
    ///
    /// Collision and nothing else. It says nothing about whether the block is
    /// seen ([`drawn`](Self::drawn)), whether it hides what is behind it
    /// ([`occludes`](Self::occludes)), whether a swing can find it
    /// ([`targetable`](Self::targetable)), or whether a new block may be built
    /// over it ([`replaceable`](Self::replaceable)). Each of those is a separate
    /// declaration on purpose: they coincide across the blocks the base game
    /// happens to ship, and deriving any of them from this one would put that
    /// accident in the engine where content could not override it.
    pub is_solid: bool,
    /// Whether a placement may overwrite this block.
    ///
    /// Content's word, read by the placement rule and by nothing else. Absent in
    /// a declaration means `false` — the conservative half, so that a block
    /// which says nothing cannot be built through.
    pub replaceable: bool,
    /// Whether this block can be broken at all.
    ///
    /// Absent in a declaration means `true`: breakable is the ordinary case and
    /// a block that says nothing is an ordinary block. `false` is what makes a
    /// block indestructible, and any block may declare it — including one that
    /// also names a residue, which is then simply never reached.
    pub breakable: bool,
    /// What the cell holds once this block is broken, or nothing where breaking
    /// it leaves the cell empty.
    ///
    /// `None` is the common case and says the cell becomes empty, because the
    /// absence of a block is not a residue worth naming. Indestructibility is
    /// [`breakable`](Self::breakable) and never this field's silence. It is a
    /// [`BlockName`] rather than a [`BlockId`] because ids belong to a registry
    /// and definitions arrive in batches: a block may legitimately name a
    /// residue that a later batch registers, so the name is resolved where a
    /// break reads it and not where it is declared.
    pub breaks_into: Option<BlockName>,
    /// Whether any face of this block is emitted.
    ///
    /// Appearance and nothing else, read by the mesher. Absent in a declaration
    /// means whatever that declaration says about [`is_solid`](Self::is_solid) —
    /// which is what keeps every declaration written before this field existed
    /// meaning what it meant, since one bit used to answer this question too.
    pub drawn: bool,
    /// Whether this block hides the face of a neighbour that would meet it.
    ///
    /// Separate from [`drawn`](Self::drawn) because a block may be seen without
    /// hiding what is behind it, which is the whole of what makes water look like
    /// water. Absent in a declaration means whatever that declaration says about
    /// [`is_solid`](Self::is_solid).
    pub occludes: bool,
    /// Whether a swing can find this block.
    ///
    /// What the crosshair may settle on, read where a trace resolves what a
    /// player is aiming at. Absent in a declaration means whatever that
    /// declaration says about [`is_solid`](Self::is_solid). Whether the block
    /// then yields to that swing is [`breakable`](Self::breakable): this field
    /// decides only whether the swing arrives.
    pub targetable: bool,
    pub origin: DefinitionOrigin,
}

/// A block's dense runtime id, valid only for the registry that assigned it.
///
/// Never an on-disk or on-wire identity: ids are reassigned freely whenever the
/// definition set changes. `u32` rather than `u16` because an id is never stored
/// per voxel, so the width is free and a 65 535-block ceiling in a public
/// contract would only be a future migration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BlockId(u32);

impl BlockId {
    /// The id numbered `raw`.
    pub const fn from_raw(raw: u32) -> Self {
        Self(raw)
    }

    /// The id's number.
    pub const fn get(self) -> u32 {
        self.0
    }
}
