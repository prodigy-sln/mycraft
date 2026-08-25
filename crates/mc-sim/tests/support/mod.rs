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

pub mod chamber;
pub mod launch;
pub mod medium;
pub mod oracle;
pub mod overlap;
pub mod sea;
pub mod solidity;
pub mod volume;

use std::error::Error;
use std::path::{Path, PathBuf};

use mc_core::block::{BlockId, BlockRegistry};
use mc_core::content::{LayerAssignment, ResolvedBlock, ResolvedContent};
use mc_core::id::BlockName;
use mc_sim::player::PlayerState;
use mc_sim::replay::{CameraPose, ReplayWorld};
use mc_sim::simulation::PublishedContent;
use mc_world::content::LuauFileDefinitionSource;
use mc_world::section::Contents;

/// The error type every replay test propagates with `?`.
pub type TestResult = Result<(), Box<dyn Error>>;

/// What a cell holding nothing is called wherever this suite compares contents
/// as text.
///
/// **It is not a block name and cannot become one**: every namespaced name
/// carries a colon, so an expectation of an empty cell and one of a named block
/// can sit in the same list without either being able to impersonate the other.
pub const NOTHING: &str = "nothing";

/// What `contents` holds, as text: the block's own name, or [`NOTHING`].
///
/// Two arms rather than a fallback, because "this cell holds nothing" and "this
/// cell holds a block" are different facts and a default would let one arrive
/// under the other's name.
#[must_use]
pub fn described(contents: Contents<&BlockName>) -> String {
    match contents {
        Contents::Empty => NOTHING.to_owned(),
        Contents::Holds(name) => name.as_str().to_owned(),
    }
}

/// The blocks the declaration names, spelled as content spells them.
///
/// Four names and no fifth for the space above the ground: the declaration puts
/// nothing there, and nothing has no name to spell.
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
    registry.apply(&LuauFileDefinitionSource::new(root))?;
    Ok(registry)
}

/// The content a simulation over `registry` publishes at launch.
///
/// **The reader's own share, written out again rather than asked of
/// `mc_sim::content::load`.** Some of the registries these fixtures build are
/// assembled in memory, so there is no content root for that door to read — and
/// writing out the three fields a participant that only draws receives makes this
/// an independent statement of what crosses the seam rather than a second call to
/// the thing that decides it.
///
/// The layers are the ones a session that has spent nothing hands out, because a
/// launch has spent nothing. The HUD is a client's own empty layout: nothing here
/// declares an element, and a source declaring nothing is a valid answer.
///
/// # Errors
///
/// Returns an error if a registered id cannot be read back, if the layers do not
/// fit a session's budget, or if an empty HUD source is refused.
pub fn published_content(registry: &BlockRegistry) -> Result<PublishedContent, Box<dyn Error>> {
    let mut blocks = Vec::new();
    for position in 0..registry.registered_count() {
        let declared = registry.definition(BlockId::from_raw(u32::try_from(position)?))?;
        blocks.push(ResolvedBlock {
            name: declared.name.clone(),
            textures: declared.textures.clone(),
            is_solid: declared.is_solid,
        });
    }
    let layers = LayerAssignment::none().appending(&registry.texture_keys())?;
    Ok(PublishedContent::first(
        ResolvedContent::stating(blocks, layers),
        mc_sim::content::hud_before_content_is_read()?,
    ))
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

/// A player state as the integers its floats are.
///
/// Everything the specification asks two runs to agree about — where the player
/// is, which way it faces, how fast it is going and whether it is on the ground
/// — as one comparable value. "The same state" means the same value, not a
/// nearly equal one, and asking the question about bits is both the exact form
/// of it and the form `clippy::float_cmp` has no quarrel with.
pub fn exactly_player(player: &PlayerState) -> ([u32; 3], [u32; 3], u32, u32, bool) {
    (
        player.position.to_array().map(f32::to_bits),
        player.velocity.to_array().map(f32::to_bits),
        player.yaw.to_bits(),
        player.pitch.to_bits(),
        player.on_ground,
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

/// What the world holds at a world position: the block's own name, or
/// [`NOTHING`] where the cell holds none.
///
/// **A position the world does not reach is a third answer and is an error**,
/// never one of the first two — a world answering "outside" everywhere would
/// otherwise satisfy every assertion written about its contents by having none
/// of them.
///
/// # Errors
///
/// Returns an error if the world reaches no cell there.
pub fn block_at(world: &ReplayWorld, x: u32, y: u32, z: u32) -> Result<String, Box<dyn Error>> {
    let held = world
        .block_at(x, y, z)
        .ok_or_else(|| format!("the replay world reaches no cell at ({x}, {y}, {z})"))?;
    Ok(described(held))
}

/// How many columns of the declared world stand below the declared sea level.
///
/// **Counted from the surface heights and [`SEA_LEVEL`] alone** — no mesh, no
/// per-voxel walk, and no reading of what any cell holds. That is what makes it
/// an independent statement of how many upward water faces the world owes: the
/// sea fills a submerged column from one block above its surface up to the sea
/// level and stops there, and the cell over it holds nothing, which the replay's
/// own contents suite asserts separately. So each submerged column shows exactly
/// one upward water face and no other column shows any.
///
/// # Errors
///
/// Returns an error if the world reports no height for some column.
pub fn submerged_columns(world: &ReplayWorld) -> Result<u64, Box<dyn Error>> {
    let mut submerged = 0;
    for (x, z) in every_column() {
        if surface_height(world, x, z)? < SEA_LEVEL {
            submerged += 1;
        }
    }
    Ok(submerged)
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
