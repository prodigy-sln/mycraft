//! A declared volume of named voxels, and the registry that says what those
//! names are.
//!
//! The replay's own world can only hold the five blocks the scripted scene
//! places, so a scenario about a block whose *definition* disagrees with what
//! its name suggests cannot be stated against it: there is no way to put a
//! `mod:cloud` into a generated world, and no way to make its `base:stone`
//! anything other than what content declares. This module is the other half —
//! a volume a test declares block by block, resolved through a registry the same
//! test declares definition by definition.
//!
//! **Block names appear here in full, and that is allowed.** Files under
//! `tests/` are not read by `mc-world`'s hardcoded-name scan; the invariant it
//! enforces is that no *engine* code decides what a block is, and a fixture
//! asserting that solidity comes from a definition has to be able to say which
//! name carries which definition — otherwise it is asserting nothing.
//!
//! **What these fixtures would fail to catch if they were built differently.**
//! [`NamedSlab`] is uniform on x and z, so it cannot discriminate a query that
//! read a box's z where it meant its x; that is what the walls of
//! `crates/mc-sim/tests/support/chamber.rs` are for, and it is pinned in phase
//! 4. What it *is* built to catch is the only thing it can: the slab's name and
//! the filler's name are declared with opposite solidity in each of the two
//! scenarios that use it, and the two scenarios choose names that point the
//! wrong way round — a shipped name for a hollow block, an invented one for a
//! solid block — so neither a hardcoded name nor a hardcoded list of known names
//! survives both.

use std::error::Error;

use mc_core::block::source::InMemoryDefinitionSource;
use mc_core::block::{BlockDefinition, BlockRegistry, DefinitionOrigin};
use mc_core::content::FaceTextures;
use mc_core::id::{BlockName, TextureKey};
use mc_sim::replay::{BlockVolume, Extent};
use mc_world::section::Contents;

/// What every registry declared here is attributed to. Nothing asserts it; a
/// definition has to say where it came from.
const FIXTURE_ORIGIN: &str = "a solidity test's declared registry";

/// A volume filled with one named block up to and including a declared height,
/// and with another above it.
///
/// Two names and nothing else, because two is what the scenarios need: the block
/// being asked about, and something above it for the player to fall through
/// before it gets there.
#[derive(Debug, Clone)]
pub struct NamedSlab {
    extent: Extent,
    top: u32,
    filling: BlockName,
    above: BlockName,
}

impl NamedSlab {
    /// A volume of `extent` holding `filling` from `y = 0` up to and including
    /// `top`, and `above` everywhere over it.
    ///
    /// # Errors
    ///
    /// Returns an error if either name is not a namespaced block name.
    pub fn new(
        extent: Extent,
        top: u32,
        filling: &str,
        above: &str,
    ) -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            extent,
            top,
            filling: BlockName::parse(filling)?,
            above: BlockName::parse(above)?,
        })
    }
}

impl BlockVolume for NamedSlab {
    fn extent(&self) -> Extent {
        self.extent
    }

    /// **A declared volume says what is there, so its answer comes from the
    /// declaration and from nowhere else.** Every cell inside this volume holds
    /// one of the two blocks it was declared with — that is what makes it a
    /// *slab* — so `Contents::Empty` is not an answer it can give, and the
    /// outer `None` keeps the one meaning it has always had: this position is
    /// outside the volume. Deriving either answer from anything the simulation
    /// resolved would make this fixture agree with whatever it was handed.
    fn block_at(&self, x: u32, y: u32, z: u32) -> Option<Contents<&BlockName>> {
        let inside = x < self.extent.x && y < self.extent.y && z < self.extent.z;
        let held = if y <= self.top {
            &self.filling
        } else {
            &self.above
        };
        inside.then_some(Contents::Holds(held))
    }
}

/// A registry holding exactly `blocks`, each carrying the solidity declared
/// beside it and textured by its own name.
///
/// Built through the in-memory definition source because a registry has no other
/// door in — which is the same structural rule content goes through, so a
/// declared definition here is indistinguishable to the engine from a shipped
/// one.
///
/// # Errors
///
/// Returns an error if a name is not a namespaced id, or if the registry refuses
/// the batch.
pub fn registry_declaring(blocks: &[(&str, bool)]) -> Result<BlockRegistry, Box<dyn Error>> {
    let mut declared = Vec::with_capacity(blocks.len());
    for &(name, is_solid) in blocks {
        // Solidity is the one property these declarations are about, and it is
        // stated per block above. The scenarios this registry serves resolve
        // solidity and never breakability, replaceability or a residue, so each
        // of those is left at what a declaration saying nothing about it means;
        // the fixtures that do break blocks declare their own registry.
        declared.push(Ok(BlockDefinition {
            name: BlockName::parse(name)?,
            textures: FaceTextures::uniform(TextureKey::parse(name)?),
            is_solid,
            replaceable: false,
            breakable: true,
            breaks_into: None,
            drawn: is_solid,
            occludes: is_solid,
            targetable: is_solid,
            origin: DefinitionOrigin::new(FIXTURE_ORIGIN),
        }));
    }
    let mut registry = BlockRegistry::new();
    registry.apply(&InMemoryDefinitionSource::new(
        DefinitionOrigin::new(FIXTURE_ORIGIN),
        declared,
    ))?;
    Ok(registry)
}
