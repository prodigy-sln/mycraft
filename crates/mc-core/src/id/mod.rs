//! Namespaced ids — `namespace:path` values shared by every kind of definition.

mod namespaced;

pub use namespaced::{BlockName, HudElementName, NamespacedIdError, TextureKey};
