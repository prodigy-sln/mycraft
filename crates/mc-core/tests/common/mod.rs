//! Fixture builders shared by `mc-core`'s behavioural tests.
//!
//! There is no public way to hand a registry a definition directly: a definition
//! source is the only way in, so every fixture here is built through
//! `InMemoryDefinitionSource` — the same seam a file-backed loader and, later, a
//! scripting host go through. A block the engine ships in Rust is not
//! expressible, which is the point.

// Each test binary links this whole module and uses a subset of it.
#![allow(dead_code)]

use mc_core::block::source::InMemoryDefinitionSource;
use mc_core::block::{BlockDefinition, BlockRegistry, DefinitionOrigin, RegistryError};
use mc_core::id::{BlockName, NamespacedIdError, TextureKey};

/// The error type every test in this suite propagates with `?`.
pub type TestResult = Result<(), Box<dyn std::error::Error>>;

/// A definition of `name`, textured `texture`, declared at `origin`.
///
/// Solidity is fixed at `true` because nothing in this suite observes it; the
/// suites that do observe it build registries whose solidity is deliberately
/// inverted, and cannot use this builder.
///
/// It names no block it breaks into for the same reason and for one more: this
/// suite is about registration — which name resolves to which definition, and what
/// a duplicate does — and a residue is a thing a *break* reads. Naming one here
/// would put a second block name in every fixture that nothing ever resolves.
///
/// # Errors
///
/// Returns [`NamespacedIdError`] if `name` or `texture` is not a namespaced id.
pub fn definition(
    name: &str,
    texture: &str,
    origin: &str,
) -> Result<BlockDefinition, NamespacedIdError> {
    Ok(BlockDefinition {
        name: BlockName::parse(name)?,
        texture: TextureKey::parse(texture)?,
        is_solid: true,
        replaceable: false,
        breakable: true,
        breaks_into: None,
        origin: DefinitionOrigin::new(origin),
    })
}

/// A source labelled `origin` that yields `definitions` and fails at none of
/// them.
#[must_use]
pub fn source(origin: &str, definitions: Vec<BlockDefinition>) -> InMemoryDefinitionSource {
    InMemoryDefinitionSource::new(
        DefinitionOrigin::new(origin),
        definitions.into_iter().map(Ok).collect(),
    )
}

/// A registry populated by applying exactly one source holding `definitions`,
/// in the order given.
///
/// # Errors
///
/// Returns [`RegistryError`] if the registry rejects the source.
pub fn registry_from(
    origin: &str,
    definitions: Vec<BlockDefinition>,
) -> Result<BlockRegistry, RegistryError> {
    let mut registry = BlockRegistry::new();
    registry.apply(&source(origin, definitions))?;
    Ok(registry)
}
