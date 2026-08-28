//! The block registry contract: what a block definition is, where definitions
//! come from, and how a name resolves to one.
//!
//! No block is defined here. The engine holds the shape of a definition and
//! nothing about any particular block — every name, texture and property arrives
//! through [`source::DefinitionSource`], which is why the base game can be
//! content on exactly the terms a third-party mod is.

mod definition;
mod opacity;
mod registry;
pub mod source;

pub use definition::{BlockDefinition, BlockId, DefinitionOrigin};
pub use opacity::Opacity;
pub use registry::{BlockRegistry, RegistryError};
