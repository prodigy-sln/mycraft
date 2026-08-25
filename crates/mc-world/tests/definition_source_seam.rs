//! The registry is populated through a port, and the port has two
//! implementations that agree.
//!
//! This is the test that keeps the seam honest. The file reader is a scripting
//! host now, and the whole bet was that the registry would not notice — the
//! definitions below are the ones this file compared before the swap, moved into
//! the language a declaration is written in and otherwise untouched, so what a
//! reader hands the registry is held to the same value it was held to then.
//! Both fixtures below are therefore written out **by hand**: building the
//! in-memory one by draining the file reader would make this assert that a value
//! equals itself, and the seam would be asserted rather than tested.

mod common;

use std::error::Error;

use common::{TestResult, content_root};
use mc_core::block::source::InMemoryDefinitionSource;
use mc_core::block::{BlockDefinition, BlockRegistry, DefinitionOrigin};
use mc_core::content::FaceTextures;
use mc_core::id::{BlockName, TextureKey};
use mc_world::content::LuauFileDefinitionSource;
use tempfile::TempDir;

/// The three blocks, written as a mod author would write them.
///
/// Air is deliberately non-solid and grass's texture key deliberately differs
/// from its own name, so neither a hard-coded solidity nor a key derived from the
/// block's name can pass.
///
/// Each of the three optional keys is declared by exactly one file, and by a
/// different one. A field only ever declared, or only ever omitted, crosses this
/// seam unobserved: an absence that arrives as an absence proves nothing about a
/// present value, and a reader that dropped the key entirely would agree with a
/// hand-written source that never named one either. Spreading them over all
/// three files is also what stops a reader that stops after the first definition
/// — or after the first two — from being the thing that makes the pair agree.
///
/// Each is declared *away from* the meaning an absent key carries, which is the
/// half that matters: a reader that read `breakable` as absent-means-false would
/// still agree with a hand-written `false`.
const AIR_DECLARATION: &str = "return {\n\tname = 'base:air',\n\ttexture = 'base:air',\n\tsolid = false,\n\treplaceable = \
     true,\n}\n";
const STONE_DECLARATION: &str = "return {\n\tname = 'base:stone',\n\ttexture = 'base:stone',\n\tsolid = true,\n\tbreaks_into = \
     'base:air',\n}\n";
const GRASS_DECLARATION: &str = "return {\n\tname = 'base:grass',\n\ttexture = 'base:grass_top',\n\tsolid = true,\n\tbreakable = \
     false,\n}\n";

/// The three names both registries are asked about, in declaration order.
const DECLARED: [&str; 3] = ["base:air", "base:stone", "base:grass"];

/// Everything a registry holds for each of `names`, one line per block.
///
/// Rendered rather than tupled so that a mismatch reads as the two declarations
/// side by side and names the field that disagrees. Both registries are read
/// through the same function, and neither is built from the other.
///
/// # Errors
///
/// Returns an error if a name is not a namespaced id or the registry does not
/// hold it.
fn registered(registry: &BlockRegistry, names: &[&str]) -> Result<Vec<String>, Box<dyn Error>> {
    let mut held = Vec::new();
    for text in names {
        let definition = registry.resolve(&BlockName::parse(text)?)?;
        held.push(format!(
            "{text}: textured {}, solid {}, replaceable {}, breakable {}, breaks into {:?}",
            textured(&definition.textures),
            definition.is_solid,
            definition.replaceable,
            definition.breakable,
            definition.breaks_into.as_ref().map(BlockName::as_str),
        ));
    }
    Ok(held)
}

/// The same three blocks, stated independently in Rust — not derived from the
/// files above, and not derived from anything that read them.
///
/// One function per block rather than one loop over a table: every field is
/// spelled out at each of the three, so a reader compares a declaration against
/// the TOML it is supposed to mirror rather than against a row of booleans whose
/// meaning is fixed somewhere else.
fn hand_written_source() -> Result<InMemoryDefinitionSource, Box<dyn Error>> {
    Ok(InMemoryDefinitionSource::new(
        DefinitionOrigin::new("hand-written"),
        vec![
            Ok(hand_written_air()?),
            Ok(hand_written_stone()?),
            Ok(hand_written_grass()?),
        ],
    ))
}

/// Air: not solid, and the one of the three declaring itself replaceable.
fn hand_written_air() -> Result<BlockDefinition, Box<dyn Error>> {
    Ok(BlockDefinition {
        name: BlockName::parse("base:air")?,
        textures: FaceTextures::uniform(TextureKey::parse("base:air")?),
        is_solid: false,
        replaceable: true,
        breakable: true,
        breaks_into: None,
        drawn: false,
        occludes: false,
        targetable: false,
        swimmable: false,
        move_resistance: 0.0,
        origin: DefinitionOrigin::new("hand-written air"),
    })
}

/// Stone: the one of the three naming a block it breaks into.
fn hand_written_stone() -> Result<BlockDefinition, Box<dyn Error>> {
    Ok(BlockDefinition {
        name: BlockName::parse("base:stone")?,
        textures: FaceTextures::uniform(TextureKey::parse("base:stone")?),
        is_solid: true,
        replaceable: false,
        breakable: true,
        breaks_into: Some(BlockName::parse("base:air")?),
        drawn: true,
        occludes: true,
        targetable: true,
        swimmable: false,
        move_resistance: 0.0,
        origin: DefinitionOrigin::new("hand-written stone"),
    })
}

/// Grass: the one of the three declaring itself unbreakable, and the one whose
/// texture key differs from its own name.
fn hand_written_grass() -> Result<BlockDefinition, Box<dyn Error>> {
    Ok(BlockDefinition {
        name: BlockName::parse("base:grass")?,
        textures: FaceTextures::uniform(TextureKey::parse("base:grass_top")?),
        is_solid: true,
        replaceable: false,
        breakable: false,
        breaks_into: None,
        drawn: true,
        occludes: true,
        targetable: true,
        swimmable: false,
        move_resistance: 0.0,
        origin: DefinitionOrigin::new("hand-written grass"),
    })
}

#[test]
fn definitions_held_in_memory_register_exactly_as_the_same_definitions_in_files_do() -> TestResult {
    let directory = TempDir::new()?;
    let root = content_root(
        &directory,
        &[
            ("air.luau", AIR_DECLARATION.to_owned()),
            ("stone.luau", STONE_DECLARATION.to_owned()),
            ("grass.luau", GRASS_DECLARATION.to_owned()),
        ],
    )?;

    let mut from_files = BlockRegistry::new();
    from_files.apply(&LuauFileDefinitionSource::new(&root))?;
    let mut from_memory = BlockRegistry::new();
    from_memory.apply(&hand_written_source()?)?;

    assert_eq!(
        registered(&from_memory, &DECLARED)?,
        registered(&from_files, &DECLARED)?,
        "where a definition came from changes nothing about what gets registered — the block a \
         definition names as what it breaks into included, both where one is named and where the \
         declaration is silent"
    );
    Ok(())
}

/// Every key a block's six facings draw from, joined — one key where all six
/// agree, and a list where they do not.
///
/// **Total over the six rather than a reading of one of them.** Every fixture in
/// this file states its texture as a single string, so the answer is one key; a
/// resolver that lost five facings, or that answered one facing's key for all six
/// while the declaration said otherwise, changes this string rather than hiding
/// behind whichever facing happened to be read.
fn textured(textures: &FaceTextures) -> String {
    textures
        .keys()
        .iter()
        .map(TextureKey::as_str)
        .collect::<Vec<_>>()
        .join(", ")
}
