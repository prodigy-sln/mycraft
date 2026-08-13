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
const AIR_TOML: &str =
    "name = \"base:air\"\ntexture = \"base:air\"\nsolid = false\nreplaceable = true\n";
const STONE_TOML: &str =
    "name = \"base:stone\"\ntexture = \"base:stone\"\nsolid = true\nbreaks_into = \"base:air\"\n";
const GRASS_TOML: &str =
    "name = \"base:grass\"\ntexture = \"base:grass_top\"\nsolid = true\nbreakable = false\n";

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
            definition.texture.as_str(),
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
        texture: TextureKey::parse("base:air")?,
        is_solid: false,
        replaceable: true,
        breakable: true,
        breaks_into: None,
        origin: DefinitionOrigin::new("hand-written air"),
    })
}

/// Stone: the one of the three naming a block it breaks into.
fn hand_written_stone() -> Result<BlockDefinition, Box<dyn Error>> {
    Ok(BlockDefinition {
        name: BlockName::parse("base:stone")?,
        texture: TextureKey::parse("base:stone")?,
        is_solid: true,
        replaceable: false,
        breakable: true,
        breaks_into: Some(BlockName::parse("base:air")?),
        origin: DefinitionOrigin::new("hand-written stone"),
    })
}

/// Grass: the one of the three declaring itself unbreakable, and the one whose
/// texture key differs from its own name.
fn hand_written_grass() -> Result<BlockDefinition, Box<dyn Error>> {
    Ok(BlockDefinition {
        name: BlockName::parse("base:grass")?,
        texture: TextureKey::parse("base:grass_top")?,
        is_solid: true,
        replaceable: false,
        breakable: false,
        breaks_into: None,
        origin: DefinitionOrigin::new("hand-written grass"),
    })
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

    assert_eq!(
        registered(&from_memory, &DECLARED)?,
        registered(&from_files, &DECLARED)?,
        "where a definition came from changes nothing about what gets registered — the block a \
         definition names as what it breaks into included, both where one is named and where the \
         declaration is silent"
    );
    Ok(())
}
