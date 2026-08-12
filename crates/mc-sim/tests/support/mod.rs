//! What every replay test builds its registry, its world and its expectations
//! from.
//!
//! Block names appear here in full. Files under `tests/` are not read by the
//! hardcoded-name scan in `mc-world`, and a test about the *declared* world has
//! to be able to say which blocks the declaration names — asserting the strata
//! without naming them would be asserting nothing.
//!
//! Every constant below is read off the specification's declaration of the
//! replay and never off a run: the footprint, the surface band, the sea level,
//! the strata depth and the landmark are what the world is *required* to be, so
//! they are this fixture's input rather than its output.

// Each test binary links this whole module and uses a subset of it.
#![allow(dead_code)]

pub mod oracle;

use std::error::Error;
use std::path::{Path, PathBuf};

use mc_core::block::BlockRegistry;
use mc_core::id::BlockName;
use mc_sim::replay::{CameraPose, ReplayWorld};
use mc_world::content::TomlFileDefinitionSource;

/// The error type every replay test propagates with `?`.
pub type TestResult = Result<(), Box<dyn Error>>;

/// The blocks the declaration names, spelled as content spells them.
pub const GRASS: &str = "base:grass";
pub const DIRT: &str = "base:dirt";
pub const STONE: &str = "base:stone";
pub const WATER: &str = "base:water";

/// How many blocks the replay spans along x and z: four columns of sixteen.
pub const FOOTPRINT: u32 = 64;

/// The band the declaration puts every column's surface height in.
pub const LOWEST_SURFACE: u32 = 32;
pub const HIGHEST_SURFACE: u32 = 48;

/// The height water fills up to where a column's surface is lower than it.
pub const SEA_LEVEL: u32 = 34;

/// How many blocks of dirt the declaration puts directly under a surface.
pub const DIRT_DEPTH: u32 = 3;

/// The column the landmark stands in, and the height its stone reaches.
pub const LANDMARK: (u32, u32) = (12, 12);
pub const LANDMARK_TOP: u32 = 64;

/// The repository's own root, located upwards from the crate this test binary
/// was built for.
///
/// # Errors
///
/// Returns an error if the manifest directory has no grandparent.
pub fn repository_root() -> Result<PathBuf, Box<dyn Error>> {
    Ok(Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .ok_or("the crate manifest directory has no repository root above it")?
        .to_owned())
}

/// A registry holding exactly what this repository ships as content.
///
/// The real content root rather than a fixture: the declared world is declared
/// in terms of blocks that content defines, and a registry assembled in Rust
/// would be the engine describing content to itself.
///
/// # Errors
///
/// Returns an error if the content root cannot be read or does not apply.
pub fn content_registry() -> Result<BlockRegistry, Box<dyn Error>> {
    let root = repository_root()?.join("content").join("base");
    let mut registry = BlockRegistry::new();
    registry.apply(&TomlFileDefinitionSource::new(root))?;
    Ok(registry)
}

/// The replay world, generated from the seed the declaration fixes.
///
/// # Errors
///
/// Returns an error if generation refuses.
pub fn replay_world(registry: &BlockRegistry) -> Result<ReplayWorld, Box<dyn Error>> {
    Ok(ReplayWorld::generate(mc_sim::REPLAY_SEED, registry)?)
}

/// A block name, parsed.
///
/// # Errors
///
/// Returns an error if `text` is not a namespaced name.
pub fn block_name(text: &str) -> Result<BlockName, Box<dyn Error>> {
    Ok(BlockName::parse(text)?)
}

/// A pose as the integers its floats are.
///
/// "The same pose" means the same value, not a nearly equal one, and asking the
/// question about bits is both the exact form of it and the form
/// `clippy::float_cmp` has no quarrel with.
pub fn exactly(camera: &CameraPose) -> ([u32; 3], [u32; 3]) {
    (
        camera.eye.map(f32::to_bits),
        camera.target.map(f32::to_bits),
    )
}

/// Every column of the footprint, x fastest.
pub fn every_column() -> impl Iterator<Item = (u32, u32)> {
    (0..FOOTPRINT).flat_map(|z| (0..FOOTPRINT).map(move |x| (x, z)))
}

/// The surface height the world reports for a column.
///
/// A column the world has no answer for is an error rather than a skipped
/// entry: a world that answered `None` everywhere would otherwise satisfy every
/// assertion written about its heights by having none of them.
///
/// # Errors
///
/// Returns an error if the world reports no height for that column.
pub fn surface_height(world: &ReplayWorld, x: u32, z: u32) -> Result<u32, Box<dyn Error>> {
    world.surface_height(x, z).ok_or_else(|| {
        format!("the replay world reports no surface height for column ({x}, {z})").into()
    })
}

/// The block the world holds at a world position.
///
/// # Errors
///
/// Returns an error if the world holds no voxel there.
pub fn block_at(world: &ReplayWorld, x: u32, y: u32, z: u32) -> Result<&BlockName, Box<dyn Error>> {
    world
        .block_at(x, y, z)
        .ok_or_else(|| format!("the replay world holds no voxel at ({x}, {y}, {z})").into())
}

/// The whole surface heightmap, one entry per column, in [`every_column`] order.
///
/// # Errors
///
/// Returns an error if the world reports no height for some column.
pub fn heightmap(world: &ReplayWorld) -> Result<Vec<u32>, Box<dyn Error>> {
    every_column()
        .map(|(x, z)| surface_height(world, x, z))
        .collect()
}
