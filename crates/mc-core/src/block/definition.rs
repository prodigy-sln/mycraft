//! What a block definition is, where it was declared, and the dense id a
//! registry assigns it.

use crate::id::{BlockName, TextureKey};

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
    pub texture: TextureKey,
    pub is_solid: bool,
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
