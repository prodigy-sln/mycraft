//! The `namespace:path` id every kind of definition is named by.
//!
//! One parse rule, three public types over it. They are deliberately not
//! interchangeable: handing a texture key to something expecting a block name is
//! a compile error rather than a debugging session, and the shared core is what
//! keeps a single rule from drifting into three.

use std::sync::Arc;

use thiserror::Error;

/// Why a `namespace:path` id could not be parsed.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum NamespacedIdError {
    #[error("`{text}` has no namespace — a namespaced id is written `namespace:path`")]
    MissingNamespace { text: String },
    #[error("`{text}` has an empty namespace")]
    EmptyNamespace { text: String },
    #[error("`{text}` has an empty path")]
    EmptyPath { text: String },
    #[error(
        "`{text}` has more than one namespace separator — a namespaced id is written `namespace:path`"
    )]
    MultipleSeparators { text: String },
}

/// A validated `namespace:path` id.
///
/// `Arc<str>` because names are cloned per palette entry and per registry lookup
/// and never mutated. It is private, so the representation is reversible.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct NamespacedId(Arc<str>);

impl NamespacedId {
    /// The rule is *exactly* one separator with both sides non-empty, and
    /// nothing else.
    ///
    /// Exactly one, not at least one: splitting on the first separator would
    /// turn `example:granite:top` into the path `granite:top`, so a typo becomes
    /// a plausible-looking id that resolves to nothing and no diagnostic points
    /// at the colon.
    ///
    /// No character set is imposed. Which characters a mod id may use is a
    /// decision the scripting layer makes when mod ids become real; guessing it
    /// here would be a compatibility promise made blind. Refusing a second
    /// separator is safe in a way that guess would not be: a rule can be relaxed
    /// later without invalidating content, never tightened.
    fn parse(text: &str) -> Result<Self, NamespacedIdError> {
        let Some((namespace, path)) = text.split_once(':') else {
            return Err(NamespacedIdError::MissingNamespace {
                text: text.to_owned(),
            });
        };
        if path.contains(':') {
            return Err(NamespacedIdError::MultipleSeparators {
                text: text.to_owned(),
            });
        }
        if namespace.is_empty() {
            return Err(NamespacedIdError::EmptyNamespace {
                text: text.to_owned(),
            });
        }
        if path.is_empty() {
            return Err(NamespacedIdError::EmptyPath {
                text: text.to_owned(),
            });
        }
        Ok(Self(Arc::from(text)))
    }

    fn as_str(&self) -> &str {
        &self.0
    }
}

/// The namespaced name a block is registered under, such as `example:granite`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BlockName(NamespacedId);

impl BlockName {
    /// Parses `text` as a block name.
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

/// The namespaced key a definition names its texture by, such as
/// `example:granite`.
///
/// A key, never a path: what pixels it resolves to is the renderer's question,
/// and a definition that named a file would answer it in the wrong place.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TextureKey(NamespacedId);

impl TextureKey {
    /// Parses `text` as a texture key.
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

/// The namespaced name a HUD element is declared under, such as
/// `example:crosshair-horizontal`.
///
/// A third type over the same rule rather than a reuse of [`BlockName`]: HUD
/// element names and block names occupy separate namespaces and may collide
/// without consequence, so a value of one that reached the other's registry
/// would be a mistake the compiler should be able to see.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct HudElementName(NamespacedId);

impl HudElementName {
    /// Parses `text` as a HUD element name.
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
