//! What a name resolves to, and the single door definitions come in through.

use std::collections::{BTreeSet, HashMap};

use thiserror::Error;

use super::definition::{BlockDefinition, BlockId, DefinitionOrigin};
use super::source::{DefinitionSource, DefinitionSourceError};
use crate::id::{BlockName, TextureKey};

/// The blocks a running game knows about.
///
/// A registry starts empty and there is no way to hand it a definition from Rust:
/// [`apply`](BlockRegistry::apply) takes a [`DefinitionSource`] and is the only
/// door in. That is deliberate and structural — an engine that shipped blocks of
/// its own would not compile, rather than being caught by a test someone has to
/// remember to keep.
#[derive(Debug, Default)]
pub struct BlockRegistry {
    /// Indexed by runtime id, which is therefore registration order.
    definitions: Vec<BlockDefinition>,
    ids: HashMap<BlockName, BlockId>,
}

impl BlockRegistry {
    /// A registry holding nothing.
    pub fn new() -> Self {
        Self::default()
    }

    /// How many blocks are registered.
    pub fn registered_count(&self) -> usize {
        self.definitions.len()
    }

    /// Every registered definition, in registration order.
    ///
    /// **What a registry declared, rather than what any world happens to
    /// hold.** A consumer deriving a per-voxel table from block properties needs
    /// every answer that is *writable* to be in that table before the first
    /// write, not only the answers a world already contains — so it asks here
    /// and never walks a world.
    pub fn definitions(&self) -> impl Iterator<Item = &BlockDefinition> {
        self.definitions.iter()
    }

    /// Every texture key the registered definitions declare.
    ///
    /// **A pure function of what content declared, and the answer to "which keys
    /// exist" for anything that has to assign them array-texture layers.** A layer
    /// index is assigned positionally over the sorted set and then travels inside
    /// every packed vertex, so a key set derived from the blocks a particular world
    /// happens to draw would make every layer index depend on that world — and a
    /// world that lost its last stone would silently renumber the array texture.
    /// Asking the registry instead cannot move with a world, because it never sees
    /// one.
    ///
    /// It reads each definition's declared keys and never its `name`. The two
    /// coincide across the blocks the base game happens to ship, which is exactly
    /// why the distinction has to be made here rather than discovered the first
    /// time a mod declares them differently.
    ///
    /// A set rather than a list, and a union rather than one key per block: two
    /// blocks may legitimately draw the same texture and so may two faces of one
    /// block, and each such pair shares a layer.
    #[must_use]
    pub fn texture_keys(&self) -> BTreeSet<TextureKey> {
        self.definitions
            .iter()
            .flat_map(|definition| definition.textures.keys())
            .collect()
    }

    /// Registers every definition the source yields, or none of them.
    ///
    /// Atomicity is structural rather than careful: everything fallible happens
    /// in a validation pass over a staging buffer, and the commit that follows
    /// returns nothing, so there is no point at which it could stop half way.
    ///
    /// # Errors
    ///
    /// Returns [`RegistryError::Source`] if the source fails while yielding,
    /// [`RegistryError::AlreadyRegistered`] if a name is already registered or is
    /// declared twice within this source, and [`RegistryError::NoDefinitions`] if
    /// the source declares nothing at all. In every case the registry is
    /// unchanged.
    pub fn apply(&mut self, source: &dyn DefinitionSource) -> Result<(), RegistryError> {
        let validated = self.validate(source)?;
        self.commit(validated);
        Ok(())
    }

    /// Resolves a name to the definition registered under it.
    ///
    /// # Errors
    ///
    /// Returns [`RegistryError::UnknownName`] if nothing is registered under
    /// `name`.
    pub fn resolve(&self, name: &BlockName) -> Result<&BlockDefinition, RegistryError> {
        let id = self.id_of(name)?;
        self.definition(id)
    }

    /// The runtime id assigned to `name` by this registry.
    ///
    /// # Errors
    ///
    /// Returns [`RegistryError::UnknownName`] if nothing is registered under
    /// `name`.
    pub fn id_of(&self, name: &BlockName) -> Result<BlockId, RegistryError> {
        self.ids
            .get(name)
            .copied()
            .ok_or_else(|| RegistryError::UnknownName { name: name.clone() })
    }

    /// The definition this registry assigned `id` to.
    ///
    /// # Errors
    ///
    /// Returns [`RegistryError::UnknownRuntimeId`] if `id` is outside the range
    /// this registry has assigned.
    pub fn definition(&self, id: BlockId) -> Result<&BlockDefinition, RegistryError> {
        self.definitions
            .get(id.get() as usize)
            .ok_or(RegistryError::UnknownRuntimeId {
                id,
                registered: self.definitions.len(),
            })
    }

    /// Drains the source into a staging buffer, rejecting a name that is already
    /// registered or that the source declares twice, and rejecting a source that
    /// declared nothing.
    ///
    /// A source yielding nothing is refused rather than accepted as a no-op, and
    /// deciding that here is what keeps the rule where it belongs: whether a
    /// place that was asked for definitions and produced none is broken is a
    /// question about registration, not about files. It is answered without this
    /// crate ever learning what the source was reading — [`DefinitionSource::origin`]
    /// hands back a label to quote, and nothing more.
    fn validate(
        &self,
        source: &dyn DefinitionSource,
    ) -> Result<Vec<BlockDefinition>, RegistryError> {
        let mut staged: Vec<BlockDefinition> = Vec::new();
        for yielded in source.definitions() {
            let definition = yielded?;
            self.reject_if_already_declared(&definition, &staged)?;
            staged.push(definition);
        }
        if staged.is_empty() {
            return Err(RegistryError::NoDefinitions {
                origin: source.origin(),
            });
        }
        Ok(staged)
    }

    /// Refuses a name that is already registered or that the batch under
    /// validation has already claimed, naming both places that declared it.
    fn reject_if_already_declared(
        &self,
        definition: &BlockDefinition,
        staged: &[BlockDefinition],
    ) -> Result<(), RegistryError> {
        match self.first_origin_of(&definition.name, staged) {
            Some(first) => Err(RegistryError::AlreadyRegistered {
                name: definition.name.clone(),
                first,
                second: definition.origin.clone(),
            }),
            None => Ok(()),
        }
    }

    /// Where `name` was first declared, whether that was a previous `apply` or
    /// earlier in the batch now being validated.
    fn first_origin_of(
        &self,
        name: &BlockName,
        staged: &[BlockDefinition],
    ) -> Option<DefinitionOrigin> {
        self.ids
            .get(name)
            .and_then(|id| self.definitions.get(id.get() as usize))
            .or_else(|| staged.iter().find(|candidate| &candidate.name == name))
            .map(|declared| declared.origin.clone())
    }

    /// Assigns ids in registration order. Infallible, and returning nothing is
    /// what makes a partial application unexpressible.
    fn commit(&mut self, validated: Vec<BlockDefinition>) {
        for definition in validated {
            // A registry cannot hold more than u32::MAX definitions long before
            // it runs out of memory, so the width is not a real bound.
            let id = BlockId::from_raw(self.definitions.len() as u32);
            self.ids.insert(definition.name.clone(), id);
            self.definitions.push(definition);
        }
    }
}

/// Why a registry refused.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum RegistryError {
    #[error("no block is registered under the name `{name}`", name = name.as_str())]
    UnknownName { name: BlockName },
    #[error(
        "`{name}` is already registered: declared by {first} and again by {second}",
        name = name.as_str(),
        first = first.as_str(),
        second = second.as_str()
    )]
    AlreadyRegistered {
        name: BlockName,
        first: DefinitionOrigin,
        second: DefinitionOrigin,
    },
    #[error(
        "runtime id {id} does not exist: {registered} blocks are registered",
        id = id.get()
    )]
    UnknownRuntimeId { id: BlockId, registered: usize },
    /// The field is `origin` and not `source` because `thiserror` reads a field
    /// of that name as the underlying [`std::error::Error`], which an origin
    /// label is not.
    #[error("{origin} declared no block definitions", origin = origin.as_str())]
    NoDefinitions { origin: DefinitionOrigin },
    #[error(transparent)]
    Source(#[from] DefinitionSourceError),
}
