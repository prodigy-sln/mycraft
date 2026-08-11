//! The registry is populated through a port, and the port has two
//! implementations that agree.
//!
//! This is the test that keeps the seam honest. MVP 2 replaces the file reader
//! with a scripting host, and the whole bet is that the registry does not notice.
//! Both fixtures below are therefore written out **by hand**: building the
//! in-memory one by draining the file reader would make this assert that a value
//! equals itself, and the seam would be asserted rather than tested.

mod common;

use std::error::Error;

use common::{TestResult, content_root};
use mc_core::block::source::InMemoryDefinitionSource;
use mc_core::block::{BlockDefinition, BlockRegistry, DefinitionOrigin};
use mc_core::id::{BlockName, TextureKey};
use mc_world::content::TomlFileDefinitionSource;
use tempfile::TempDir;

/// The three blocks, written as a mod author would write them.
///
/// Air is deliberately non-solid and grass's texture key deliberately differs
/// from its own name, so neither a hard-coded solidity nor a key derived from the
/// block's name can pass.
const AIR_TOML: &str = "name = \"base:air\"\ntexture = \"base:air\"\nsolid = false\n";
const STONE_TOML: &str = "name = \"base:stone\"\ntexture = \"base:stone\"\nsolid = true\n";
const GRASS_TOML: &str = "name = \"base:grass\"\ntexture = \"base:grass_top\"\nsolid = true\n";

/// The same three blocks, stated independently in Rust — not derived from the
/// files above, and not derived from anything that read them.
fn hand_written_source() -> Result<InMemoryDefinitionSource, Box<dyn Error>> {
    Ok(InMemoryDefinitionSource::new(
        DefinitionOrigin::new("hand-written"),
        vec![
            Ok(BlockDefinition {
                name: BlockName::parse("base:air")?,
                texture: TextureKey::parse("base:air")?,
                is_solid: false,
                origin: DefinitionOrigin::new("hand-written air"),
            }),
            Ok(BlockDefinition {
                name: BlockName::parse("base:stone")?,
                texture: TextureKey::parse("base:stone")?,
                is_solid: true,
                origin: DefinitionOrigin::new("hand-written stone"),
            }),
            Ok(BlockDefinition {
                name: BlockName::parse("base:grass")?,
                texture: TextureKey::parse("base:grass_top")?,
                is_solid: true,
                origin: DefinitionOrigin::new("hand-written grass"),
            }),
        ],
    ))
}

#[test]
fn definitions_held_in_memory_register_exactly_as_the_same_definitions_in_files_do() -> TestResult {
    let directory = TempDir::new()?;
    let root = content_root(
        &directory,
        &[
            ("air.toml", AIR_TOML.to_owned()),
            ("stone.toml", STONE_TOML.to_owned()),
            ("grass.toml", GRASS_TOML.to_owned()),
        ],
    )?;

    let mut from_files = BlockRegistry::new();
    from_files.apply(&TomlFileDefinitionSource::new(&root))?;
    let mut from_memory = BlockRegistry::new();
    from_memory.apply(&hand_written_source()?)?;

    let mut held_in_memory = Vec::new();
    let mut read_from_files = Vec::new();
    for text in ["base:air", "base:stone", "base:grass"] {
        let name = BlockName::parse(text)?;
        let in_memory = from_memory.resolve(&name)?;
        let in_files = from_files.resolve(&name)?;
        held_in_memory.push((text, in_memory.texture.as_str(), in_memory.is_solid));
        read_from_files.push((text, in_files.texture.as_str(), in_files.is_solid));
    }
    assert_eq!(
        held_in_memory, read_from_files,
        "where a definition came from changes nothing about what gets registered"
    );
    Ok(())
}
