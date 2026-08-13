//! What a content root gives a registry, and what it says when it cannot.
//!
//! Two halves. The first is the happy path: a root of definition files is the
//! sole origin of everything a registry knows, including the blocks this
//! repository itself ships. The second is the diagnostic contract, and it is the
//! half a mod author actually lives in — every refusal has to say which file,
//! which block and which field was wrong, because a mod author debugging blind
//! is the failure mode this feature is judged on.

mod common;

use std::collections::BTreeSet;
use std::error::Error;
use std::fs;
use std::path::Path;

use common::{
    TestResult, block_file, content_root, directory_label, registered_names, repository_root,
};
use mc_core::block::source::{DefinitionFault, DefinitionSourceError};
use mc_core::block::{BlockRegistry, RegistryError};
use mc_core::id::BlockName;
use mc_world::content::TomlFileDefinitionSource;
use tempfile::TempDir;

/// The blocks this repository ships, with the solidity each declares.
///
/// Four, and none of them means empty space. A cell of the world holds one of
/// these or it holds nothing at all, and nothing is not a block a content author
/// declares, textures or reasons about.
const SHIPPED_BLOCKS: [(&str, bool); 4] = [
    ("base:stone", true),
    ("base:dirt", true),
    ("base:grass", true),
    ("base:water", false),
];

/// A registry populated from the content root at `root`.
fn registry_from_root(root: &Path) -> Result<BlockRegistry, Box<dyn Error>> {
    let mut registry = BlockRegistry::new();
    registry.apply(&TomlFileDefinitionSource::new(root))?;
    Ok(registry)
}

/// The registry and the refusal that applying `root` produced, for the roots
/// that must not apply at all.
fn rejection_from_root(root: &Path) -> Result<(BlockRegistry, RegistryError), Box<dyn Error>> {
    let mut registry = BlockRegistry::new();
    let error = registry
        .apply(&TomlFileDefinitionSource::new(root))
        .err()
        .ok_or("this content root must not be accepted, or the assertion below is vacuous")?;
    Ok((registry, error))
}

/// The fault reported for a root holding one badly-declared block.
fn fault_from_root(root: &Path) -> Result<DefinitionFault, Box<dyn Error>> {
    let (_, error) = rejection_from_root(root)?;
    let RegistryError::Source(DefinitionSourceError::Malformed(fault)) = &error else {
        return Err(format!("expected a malformed-definition refusal, got {error:?}").into());
    };
    Ok(fault.clone())
}

/// A root holding three declared blocks and no other file whatsoever.
fn three_block_root(directory: &TempDir) -> Result<std::path::PathBuf, Box<dyn Error>> {
    content_root(
        directory,
        &[
            ("air.toml", block_file("base:air", "base:air", false)),
            ("stone.toml", block_file("base:stone", "base:stone", true)),
            ("grass.toml", block_file("base:grass", "base:grass", true)),
        ],
    )
}

/// The content root this repository ships.
fn shipped_content_root() -> Result<std::path::PathBuf, Box<dyn Error>> {
    Ok(repository_root()?.join("content").join("base"))
}

#[test]
fn a_content_root_registers_exactly_the_blocks_it_declares() -> TestResult {
    let directory = TempDir::new()?;
    let root = three_block_root(&directory)?;

    let registry = registry_from_root(&root)?;

    let expected: BTreeSet<String> = ["base:air", "base:stone", "base:grass"]
        .into_iter()
        .map(String::from)
        .collect();
    assert_eq!(
        registered_names(&registry)?,
        expected,
        "a content root is the sole origin of what a registry holds"
    );
    Ok(())
}

#[test]
fn the_content_this_repository_ships_registers_the_blocks_it_declares_and_no_other() -> TestResult {
    let registry = registry_from_root(&shipped_content_root()?)?;

    let mut registered = BTreeSet::new();
    for name in registered_names(&registry)? {
        let is_solid = registry.resolve(&BlockName::parse(&name)?)?.is_solid;
        registered.insert((name, is_solid));
    }

    assert_eq!(
        registered,
        declared_blocks(),
        "the shipped content root is the sole origin of what a registry holds, and every block \
         in it is one a content author declared, textured and gave a solidity to. The set is \
         walked *out of the registry* rather than looked up name by name, so a block the root \
         declares and this list does not is a difference here rather than something nothing \
         reads; and each solidity is read back through the registry, so a block whose file says \
         one thing and whose registration says another is a difference too"
    );
    Ok(())
}

/// The blocks this repository ships, as a set of name and solidity.
fn declared_blocks() -> BTreeSet<(String, bool)> {
    SHIPPED_BLOCKS
        .into_iter()
        .map(|(name, is_solid)| (String::from(name), is_solid))
        .collect()
}

#[test]
fn a_content_root_holding_no_texture_files_still_registers_its_blocks_and_their_keys() -> TestResult
{
    // The fixture writes definition files and nothing else — there is no texture
    // anywhere on disk for these keys to resolve to, which is the point: a key is
    // a reference the renderer will interpret, not a path that has to exist.
    let directory = TempDir::new()?;
    let root = three_block_root(&directory)?;

    let registry = registry_from_root(&root)?;

    assert_eq!(
        registry.registered_count(),
        3,
        "a block needs no texture file to be registered"
    );
    assert_eq!(
        registry
            .resolve(&BlockName::parse("base:stone")?)?
            .texture
            .as_str(),
        "base:stone",
        "a texture key is reported as declared, whether or not any pixels exist for it"
    );
    Ok(())
}

#[test]
fn a_content_root_declaring_no_blocks_at_all_is_refused_naming_that_root() -> TestResult {
    let directory = TempDir::new()?;
    let root = content_root(&directory, &[])?;

    let (_, error) = rejection_from_root(&root)?;

    let RegistryError::NoDefinitions { origin } = &error else {
        return Err(format!("expected an empty-source refusal, got {error:?}").into());
    };
    assert!(
        origin.as_str().contains(directory_label(&root)?),
        "the refusal names the root that declared nothing, got `{}`",
        origin.as_str()
    );
    Ok(())
}

#[test]
fn a_content_root_that_does_not_exist_is_refused_naming_that_path() -> TestResult {
    const ABSENT_ROOT: &str = "no-such-content-root";
    let directory = TempDir::new()?;
    let root = directory.path().join(ABSENT_ROOT);

    let (_, error) = rejection_from_root(&root)?;

    let RegistryError::Source(DefinitionSourceError::Unreadable { origin, .. }) = &error else {
        return Err(format!("expected an unreadable-root refusal, got {error:?}").into());
    };
    assert!(
        origin.as_str().contains(ABSENT_ROOT),
        "the refusal names the path that could not be read, got `{}`",
        origin.as_str()
    );
    Ok(())
}

#[test]
fn a_block_declared_without_a_texture_key_is_refused_naming_its_file_and_itself() -> TestResult {
    const FILE: &str = "stone.toml";
    let directory = TempDir::new()?;
    let root = content_root(
        &directory,
        &[(FILE, "name = \"base:stone\"\nsolid = true\n".to_owned())],
    )?;

    let fault = fault_from_root(&root)?;

    assert_eq!(
        (fault.origin.as_str().contains(FILE), fault.block.as_deref()),
        (true, Some("base:stone")),
        "a block with no texture key is refused naming its file and itself: {fault:?}"
    );
    Ok(())
}

#[test]
fn a_block_declared_without_a_solidity_field_is_refused_naming_its_file_and_itself() -> TestResult {
    const FILE: &str = "stone.toml";
    let directory = TempDir::new()?;
    let root = content_root(
        &directory,
        &[(
            FILE,
            "name = \"base:stone\"\ntexture = \"base:stone\"\n".to_owned(),
        )],
    )?;

    let fault = fault_from_root(&root)?;

    assert_eq!(
        (fault.origin.as_str().contains(FILE), fault.block.as_deref()),
        (true, Some("base:stone")),
        "a block with no solidity is refused naming its file and itself: {fault:?}"
    );
    Ok(())
}

#[test]
fn a_block_whose_solidity_is_written_as_text_is_refused_naming_that_field() -> TestResult {
    const FILE: &str = "stone.toml";
    let directory = TempDir::new()?;
    let root = content_root(
        &directory,
        &[(
            FILE,
            "name = \"base:stone\"\ntexture = \"base:stone\"\nsolid = \"yes\"\n".to_owned(),
        )],
    )?;

    let fault = fault_from_root(&root)?;

    assert_eq!(
        (
            fault.origin.as_str().contains(FILE),
            fault.block.as_deref(),
            fault.field.as_deref()
        ),
        (true, Some("base:stone"), Some("solid")),
        "a solidity that is not a boolean is refused naming file, block and field: {fault:?}"
    );
    Ok(())
}

#[test]
fn a_block_carrying_a_field_the_loader_does_not_recognise_is_refused_naming_that_field()
-> TestResult {
    const UNRECOGNISED: &str = "hardness";
    let directory = TempDir::new()?;
    let root = content_root(
        &directory,
        &[(
            "stone.toml",
            format!(
                "name = \"base:stone\"\ntexture = \"base:stone\"\nsolid = true\n{UNRECOGNISED} = 3\n"
            ),
        )],
    )?;

    let fault = fault_from_root(&root)?;

    // Which slot carries the name is the loader's choice — a parser that refuses
    // unknown fields reports one inside its own message. What is not its choice
    // is whether the offending field is named at all: a mod author who is told
    // only "this file is wrong" has to bisect it by hand.
    assert!(
        fault.field.as_deref() == Some(UNRECOGNISED) || fault.cause.contains(UNRECOGNISED),
        "an unrecognised field is named in the refusal: {fault:?}"
    );
    Ok(())
}

#[test]
fn an_unparseable_file_read_after_well_formed_ones_registers_nothing_at_all() -> TestResult {
    let directory = TempDir::new()?;
    // Sorted by file name, so the unparseable one is genuinely read third and a
    // loader that registered as it went would already hold two blocks by then.
    let root = content_root(
        &directory,
        &[
            ("first.toml", block_file("base:air", "base:air", false)),
            ("second.toml", block_file("base:stone", "base:stone", true)),
            ("third.toml", "this line is not toml at all\n".to_owned()),
        ],
    )?;

    let (registry, _) = rejection_from_root(&root)?;

    assert_eq!(
        registry.registered_count(),
        0,
        "a content root registers every definition in it or none of them"
    );
    Ok(())
}

#[test]
fn two_files_declaring_the_same_block_are_refused_naming_both_of_them() -> TestResult {
    const FIRST_FILE: &str = "first_stone.toml";
    const SECOND_FILE: &str = "second_stone.toml";
    let directory = TempDir::new()?;
    let root = content_root(
        &directory,
        &[
            (FIRST_FILE, block_file("base:stone", "base:stone_a", true)),
            (SECOND_FILE, block_file("base:stone", "base:stone_b", true)),
        ],
    )?;

    let (_, error) = rejection_from_root(&root)?;

    let RegistryError::AlreadyRegistered { first, second, .. } = &error else {
        return Err(format!("expected an already-registered refusal, got {error:?}").into());
    };
    assert_eq!(
        (
            first.as_str().contains(FIRST_FILE),
            second.as_str().contains(SECOND_FILE)
        ),
        (true, true),
        "the refusal tells the two declarations apart, in the order the root is read: \
         first `{}`, second `{}`",
        first.as_str(),
        second.as_str()
    );
    Ok(())
}

/// Guard. The scenarios for a missing texture and a missing solidity ask only
/// that the file and the block be named, so the two tests above assert only
/// those. The loader also names the field, which is most of what makes the
/// message usable, and nothing would notice if that stopped happening. This is
/// what notices.
#[test]
fn a_required_field_that_was_not_declared_is_named_in_the_refusal() -> TestResult {
    let without_texture_directory = TempDir::new()?;
    let without_texture = content_root(
        &without_texture_directory,
        &[(
            "stone.toml",
            "name = \"base:stone\"\nsolid = true\n".to_owned(),
        )],
    )?;
    let without_solidity_directory = TempDir::new()?;
    let without_solidity = content_root(
        &without_solidity_directory,
        &[(
            "stone.toml",
            "name = \"base:stone\"\ntexture = \"base:stone\"\n".to_owned(),
        )],
    )?;

    let faults = (
        fault_from_root(&without_texture)?,
        fault_from_root(&without_solidity)?,
    );

    assert_eq!(
        (faults.0.field.as_deref(), faults.1.field.as_deref()),
        (Some("texture"), Some("solid")),
        "a mod author is told which field was left out, not merely that the file is \
         wrong somewhere: {faults:?}"
    );
    Ok(())
}

/// Guard. A root that exists but holds no block declarations directory is a
/// distinct case from a root that does not exist at all, and from a declarations
/// directory that exists and declares nothing. The first two are the same
/// unreadable path; the third is the empty-source refusal asserted above. The
/// distinction is deliberate and no scenario pins it.
#[test]
fn a_content_root_with_no_declarations_directory_is_refused_naming_that_path() -> TestResult {
    const ROOT_WITHOUT_DECLARATIONS: &str = "root-without-declarations";
    let directory = TempDir::new()?;
    let root = directory.path().join(ROOT_WITHOUT_DECLARATIONS);
    fs::create_dir_all(&root)?;

    let (_, error) = rejection_from_root(&root)?;

    let RegistryError::Source(DefinitionSourceError::Unreadable { origin, .. }) = &error else {
        return Err(format!("expected an unreadable-root refusal, got {error:?}").into());
    };
    assert!(
        origin.as_str().contains(ROOT_WITHOUT_DECLARATIONS),
        "a root with nothing to read is refused naming the path that could not be read, \
         got `{}`",
        origin.as_str()
    );
    Ok(())
}
