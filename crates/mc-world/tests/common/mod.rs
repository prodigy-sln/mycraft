//! Fixture builders shared by `mc-world`'s behavioural tests.
//!
//! A content root is a directory of files, so every fixture here writes real
//! files into a temporary directory rather than mocking a filesystem: the thing
//! under test is precisely the reading of a directory, and a mock of it would
//! assert nothing.
//!
//! Origins are compared by the *name* of the file or directory they point at,
//! never by a whole path — a path renders with OS-specific separators and an
//! assertion on one would be a Windows-only or Unix-only test.
//!
//! The section fixtures below build their registries the same way: through the
//! in-memory definition source, because a registry has no other door in.

// Each test binary links this whole module and uses a subset of it.
#![allow(dead_code)]

pub mod assembled;
pub mod handbuilt;
pub mod hud;
// Not a fixture builder like its neighbours: the licence check itself, shared
// because two test binaries read the same declaration and a second copy of the
// extraction would be a second opinion about what "declares nothing" means.
pub mod license;
// The two places that declaration has to reach — every workspace member, and
// the README. Its own module because the gate caps a file at 600 lines, not
// because the concerns are unrelated.
pub mod license_consumers;
pub mod persistence;

use std::collections::BTreeSet;
use std::error::Error;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

use mc_core::block::source::InMemoryDefinitionSource;
use mc_core::block::{BlockDefinition, BlockId, BlockRegistry, DefinitionOrigin};
use mc_core::id::{BlockName, NamespacedIdError, TextureKey};
use mc_world::section::{Contents, LocalPos, SECTION_SIZE, Section};
use tempfile::TempDir;

/// The error type every test in this suite propagates with `?`.
pub type TestResult = Result<(), Box<dyn Error>>;

/// What a cell holding nothing is called wherever this suite compares contents
/// as text.
///
/// **It is not a block name and cannot become one.** Every namespaced name
/// carries a colon, so no registry can ever hand back something that reads like
/// this — which is what lets an expectation about an empty cell and one about a
/// named block sit side by side in the same list without either being able to
/// impersonate the other.
pub const NOTHING: &str = "nothing";

/// What `contents` holds, as text: the block's own name, or [`NOTHING`].
///
/// Written as two arms rather than as a default, because the two answers are
/// different facts about a cell and a fallback would let one arrive under the
/// other's name.
#[must_use]
pub fn described(contents: Contents<&BlockName>) -> String {
    match contents {
        Contents::Empty => NOTHING.to_owned(),
        Contents::Holds(name) => name.as_str().to_owned(),
    }
}

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

/// A content root inside `directory` declaring `blocks`, and holding nothing
/// else — no textures, no nested directories, no files outside `blocks/`.
///
/// The root is `directory` itself rather than a fixed subdirectory, so its final
/// component is unique per fixture and an origin that quotes it back can be told
/// apart from a constant label.
///
/// # Errors
///
/// Returns an error if the directory or any file cannot be written.
pub fn content_root(
    directory: &TempDir,
    blocks: &[(&str, String)],
) -> Result<PathBuf, Box<dyn Error>> {
    let root = directory.path().to_owned();
    let declarations = root.join("blocks");
    fs::create_dir_all(&declarations)?;
    for (file_name, body) in blocks {
        fs::write(declarations.join(file_name), body)?;
    }
    Ok(root)
}

/// The text of a well-formed block definition file.
#[must_use]
pub fn block_file(name: &str, texture: &str, solid: bool) -> String {
    format!("name = \"{name}\"\ntexture = \"{texture}\"\nsolid = {solid}\n")
}

/// Every name a registry holds, read back through the dense runtime ids it
/// assigned.
///
/// # Errors
///
/// Returns an error if an id the registry counted does not resolve, which would
/// mean the ids it assigns are not dense.
pub fn registered_names(registry: &BlockRegistry) -> Result<BTreeSet<String>, Box<dyn Error>> {
    let mut names = BTreeSet::new();
    for position in 0..registry.registered_count() {
        let id = BlockId::from_raw(u32::try_from(position)?);
        names.insert(registry.definition(id)?.name.as_str().to_owned());
    }
    Ok(names)
}

/// The final component of `path`, which is what an origin assertion looks for.
///
/// # Errors
///
/// Returns an error if `path` has no final component or it is not valid UTF-8.
pub fn directory_label(path: &Path) -> Result<&str, Box<dyn Error>> {
    path.file_name()
        .and_then(OsStr::to_str)
        .ok_or_else(|| format!("{} has no usable final component", path.display()).into())
}

/// The label every hand-built registry in this suite is attributed to unless a
/// fixture says otherwise.
///
/// Most suites never assert it — it exists because a definition has to say where
/// it came from. It is public because a fixture that varies the origin needs the
/// *other* registry to keep the ordinary one, so that the two differ in the
/// origin and in nothing else.
pub const FIXTURE_ORIGIN: &str = "a fixture registry";

/// A registry holding exactly `blocks`, in the order given, each carrying the
/// texture and solidity declared beside it, and every definition attributed to
/// `origin`.
///
/// **Name, texture and origin vary independently here, and that independence is
/// the whole reason this builder exists.** [`registry_declaring`] derives the
/// texture from the name and fixes the origin, so it cannot express "the same
/// block, retextured" or "the same definitions, read from somewhere else" — and
/// those two are exactly what tells a declaration's appearance apart from its
/// behaviour, and what proves a path-derived label is not part of either.
///
/// # Errors
///
/// Returns an error if a name or a texture is not a namespaced id, or if the
/// registry refuses the batch — a name repeated in `blocks`, for instance.
pub fn registry_from(
    origin: &str,
    blocks: &[(&str, &str, bool)],
) -> Result<BlockRegistry, Box<dyn Error>> {
    let mut declared = Vec::with_capacity(blocks.len());
    for &(name, texture, is_solid) in blocks {
        // Solidity and texture are the properties these fixtures declare per
        // block. Breakability, replaceability and a residue are read by a break
        // or a placement, which is not something this crate's suites drive, so
        // each is left at what a declaration saying nothing about it means.
        declared.push(Ok(BlockDefinition {
            name: BlockName::parse(name)?,
            texture: TextureKey::parse(texture)?,
            is_solid,
            replaceable: false,
            breakable: true,
            breaks_into: None,
            origin: DefinitionOrigin::new(origin),
        }));
    }
    let mut registry = BlockRegistry::new();
    registry.apply(&InMemoryDefinitionSource::new(
        DefinitionOrigin::new(origin),
        declared,
    ))?;
    Ok(registry)
}

/// A registry holding exactly `names`, in the order given, each block solid and
/// textured by its own name.
///
/// # Errors
///
/// Returns an error if a name is not a namespaced id, or if the registry refuses
/// the batch — a name repeated in `names`, for instance.
pub fn registry_of(names: &[&str]) -> Result<BlockRegistry, Box<dyn Error>> {
    let declared: Vec<(&str, bool)> = names.iter().map(|name| (*name, true)).collect();
    registry_declaring(&declared)
}

/// A registry holding exactly `blocks`, in the order given, each carrying the
/// solidity declared beside it and textured by its own name.
///
/// Solidity has to be sayable per block, and inverted from what the shipped
/// content declares, or the tests that assert a voxel's solidity could be passed
/// by an engine that recognised a name or a runtime id instead of reading the
/// property. [`registry_of`] is this builder with every block solid.
///
/// # Errors
///
/// Returns an error if a name is not a namespaced id, or if the registry refuses
/// the batch — a name repeated in `blocks`, for instance.
pub fn registry_declaring(blocks: &[(&str, bool)]) -> Result<BlockRegistry, Box<dyn Error>> {
    let declared: Vec<(&str, &str, bool)> = blocks
        .iter()
        .map(|&(name, is_solid)| (name, name, is_solid))
        .collect();
    registry_from(FIXTURE_ORIGIN, &declared)
}

/// A registry holding `count` generated blocks, `fixture:block_0000` upwards.
///
/// # Errors
///
/// Returns an error if the registry refuses the generated batch.
pub fn registry_of_size(count: u32) -> Result<BlockRegistry, Box<dyn Error>> {
    let names: Vec<String> = (0..count).map(generated_block_name).collect();
    let borrowed: Vec<&str> = names.iter().map(String::as_str).collect();
    registry_of(&borrowed)
}

/// What the `position`-th generated block is called.
///
/// The `fixture:` namespace is deliberate. A generated block carrying a `base:`
/// name would be indistinguishable from shipped content in a failure message,
/// and a reader would have no way to tell an invented block from a real one.
#[must_use]
pub fn generated_block_name(position: u32) -> String {
    format!("fixture:block_{position:04}")
}

/// The `position`-th generated block's name.
///
/// # Errors
///
/// Returns [`NamespacedIdError`] if the generated form ever stops being a
/// namespaced id.
pub fn generated_block(position: u32) -> Result<BlockName, NamespacedIdError> {
    BlockName::parse(&generated_block_name(position))
}

/// How many bits of a linear voxel index address one axis, and the mask that
/// reads them back.
///
/// Shifts rather than divisions throughout: `clippy::integer_division` is a gate
/// error and it applies to test targets too.
const AXIS_SHIFT: u32 = SECTION_SIZE.trailing_zeros();
const AXIS_MASK: u32 = SECTION_SIZE - 1;

/// A local position, spelled out.
#[must_use]
pub const fn at(x: u32, y: u32, z: u32) -> LocalPos {
    LocalPos { x, y, z }
}

/// The `linear`-th position of a section, counting x fastest, then y, then z.
///
/// Used wherever a test needs some number of distinct positions and does not
/// care which — writing `n` distinct blocks, for instance.
#[must_use]
pub const fn nth_position(linear: u32) -> LocalPos {
    LocalPos {
        x: linear & AXIS_MASK,
        y: (linear >> AXIS_SHIFT) & AXIS_MASK,
        z: (linear >> (AXIS_SHIFT * 2)) & AXIS_MASK,
    }
}

/// Every position a section has, x fastest, then y, then z.
pub fn all_positions() -> impl Iterator<Item = LocalPos> {
    (0..SECTION_SIZE).flat_map(|z| {
        (0..SECTION_SIZE).flat_map(move |y| (0..SECTION_SIZE).map(move |x| at(x, y, z)))
    })
}

/// What every position in `section` holds, in [`all_positions`] order — a block
/// by name, or [`NOTHING`] where the cell holds none.
///
/// # Errors
///
/// Returns an error if any position a section is supposed to have cannot be
/// read.
pub fn contents_at_every_position(section: &Section) -> Result<Vec<String>, Box<dyn Error>> {
    let mut held = Vec::new();
    for position in all_positions() {
        held.push(described(section.block_at(position)?));
    }
    Ok(held)
}
