//! The two namespaced names VoxForge introduces.
//!
//! Both are newtypes over [`NamespacedId`] rather than reuses of
//! `mc_core::id::BlockName`: `namespaced.rs` argues that types over one rule are
//! deliberately not interchangeable, and a model is not a block, so reusing
//! `BlockName` here would be that file's own argument ignored. Materials are not
//! engine content yet, which is why both types live in this crate; the day they
//! become engine content, they move.

use mc_core::id::{NamespacedId, NamespacedIdError};

/// The namespaced key a material is declared under, such as `base:oak_plank`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct MaterialKey(NamespacedId);

impl MaterialKey {
    /// Parses `text` as a material key.
    ///
    /// # Errors
    ///
    /// Returns [`NamespacedIdError`] if `text` is not `namespace:path` with both
    /// sides non-empty.
    pub fn parse(text: &str) -> Result<Self, NamespacedIdError> {
        Ok(Self(NamespacedId::parse(text)?))
    }

    /// The key exactly as it was written.
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

/// The namespaced name a model document declares, such as `base:torch`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ModelName(NamespacedId);

impl ModelName {
    /// Parses `text` as a model name.
    ///
    /// # Errors
    ///
    /// Returns [`NamespacedIdError`] if `text` is not `namespace:path` with both
    /// sides non-empty.
    pub fn parse(text: &str) -> Result<Self, NamespacedIdError> {
        Ok(Self(NamespacedId::parse(text)?))
    }

    /// The name exactly as it was written.
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}
