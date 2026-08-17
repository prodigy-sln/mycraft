//! One reader stands between a content root and the block registry, and it is
//! the one that reads declarations written in Luau.
//!
//! Two readers of one directory is the failure this is about, and it is quiet:
//! both answer, neither errors, and which one a caller happens to reach for
//! decides what a player is handed. A file in the format that was retired is
//! then a declaration to one of them and nothing at all to the other.
//!
//! **These assert through the client's own preparation and not through the
//! reader.** Asked of the reader, a directory of retired declarations is refused
//! today and was refused the day the reader was written — the reader never knew
//! that format. What is worth asserting is that the *client* no longer has a
//! second way to read one, and that is a fact about the path from a content root
//! to a registry rather than about any reader on it.
//!
//! **The scene the golden frames are shot from is a second door onto that
//! path**, and a second door is untested until something asserts through it
//! rather than through the one that was already covered. That is what the last
//! test here is for: it reads the shipped content through the very call every
//! committed frame is captured from.

mod support;

use std::error::Error;
use std::ffi::OsStr;
use std::fs;
use std::path::Path;

use mc_client::startup::{PreparationError, prepare_scene};
use mc_core::block::{BlockId, BlockRegistry, DefinitionOrigin, RegistryError};
use support::{TestResult, content, content_root};

/// A block declaration in the format this feature retires, and the file it is
/// written into.
const RETIRED_FILE: &str = "brass.toml";
const RETIRED_DECLARATION: &str =
    "name = \"example:brass\"\ntexture = \"example:quartz\"\nsolid = true\n";

/// The same declaration under the name a root that holds nothing else uses.
const ONLY_RETIRED_FILE: &str = "amber.toml";
const ONLY_RETIRED_DECLARATION: &str =
    "name = \"example:amber\"\ntexture = \"example:quartz\"\nsolid = true\n";

/// A block declaration in the language that replaces it, beside the retired one.
const DECLARATION_FILE: &str = "amber.luau";
const DECLARATION: &str =
    "return {\n\tname = 'example:amber',\n\ttexture = 'example:quartz',\n\tsolid = true,\n}\n";

/// What the shipped content declares, in registration order, each with the file
/// that declares it.
const SHIPPED: [&str; 4] = [
    "base:dirt declared by dirt.luau",
    "base:grass declared by grass.luau",
    "base:stone declared by stone.luau",
    "base:water declared by water.luau",
];

/// What a preparation came to.
///
/// Three answers rather than "it refused", because a root the client reads
/// through the retired reader does not fail quietly — it registers a block and
/// then fails somewhere further down, over a world it could not generate. A
/// comparison that only asked whether something was refused would accept that
/// as the refusal this is about.
#[derive(Debug, PartialEq, Eq)]
enum Prepared {
    /// The blocks it registered, in registration order, each with the file that
    /// declared it.
    Registered(Vec<String>),
    /// It refused because the root declares no block at all, naming the root.
    RefusedForDeclaringNoBlocks(String),
    /// It refused for some other reason, rendered as it renders itself.
    RefusedOtherwise(String),
}

#[test]
fn a_root_holding_only_a_declaration_in_the_retired_format_declares_no_block_at_all() -> TestResult
{
    let root = tempfile::tempdir()?;
    a_root_declaring(
        root.path(),
        &[(ONLY_RETIRED_FILE, ONLY_RETIRED_DECLARATION)],
    )?;

    let prepared = prepared_over(root.path());

    assert_eq!(
        prepared,
        Prepared::RefusedForDeclaringNoBlocks(root.path().display().to_string()),
        "a directory whose only block file is written in the format that was retired declares no \
         block, and the refusal has to say so by naming the root. A client that read the file \
         anyway would be the second path this feature exists to close, and whoever wrote the file \
         would have no way to learn that nothing reads it"
    );
    Ok(())
}

#[test]
fn a_root_declaring_in_both_formats_registers_the_luau_declaration_and_never_the_other()
-> TestResult {
    let root = content::shipped_copy()?
        .declaring_block(DECLARATION_FILE, DECLARATION)?
        .declaring_block(RETIRED_FILE, RETIRED_DECLARATION)?;

    let prepared = prepared_over(root.path());

    assert_eq!(
        prepared,
        Prepared::Registered(
            ["example:amber declared by amber.luau"]
                .into_iter()
                .map(str::to_owned)
                .chain(SHIPPED.iter().map(|reading| (*reading).to_owned()))
                .collect()
        ),
        "the two files sit in one directory and only one of them is a declaration. A registry \
         that answered for `example:brass` would mean the retired reader is still on the path, \
         and every promise made about what refuses a bad declaration would be made about the \
         wrong reader"
    );
    Ok(())
}

#[test]
fn the_scene_the_golden_frames_are_shot_through_registers_the_blocks_the_shipped_declarations_state()
-> TestResult {
    let prepared = prepared_over(&content_root()?);

    assert_eq!(
        prepared,
        Prepared::Registered(
            SHIPPED
                .iter()
                .map(|reading| (*reading).to_owned())
                .collect()
        ),
        "every committed frame is captured through this call, so it is where the claim that the \
         world renders identically is decided. A door onto the content path that the other door's \
         tests happen to cover is a door nothing has ever read through"
    );
    Ok(())
}

/// What preparing the scene from `root` came to.
fn prepared_over(root: &Path) -> Prepared {
    match prepare_scene(root) {
        Ok(prepared) => Prepared::Registered(registered_in(&prepared.registry)),
        Err(PreparationError::Content(RegistryError::NoDefinitions { origin })) => {
            Prepared::RefusedForDeclaringNoBlocks(origin.as_str().to_owned())
        }
        Err(other) => Prepared::RefusedOtherwise(other.to_string()),
    }
}

/// Every block `registry` holds, in the order it registered them, each with the
/// file that declared it.
fn registered_in(registry: &BlockRegistry) -> Vec<String> {
    (0..registry.registered_count())
        .map(|id| {
            registry
                .definition(BlockId::from_raw(id as u32))
                .map_or_else(
                    |failure| failure.to_string(),
                    |definition| {
                        format!(
                            "{} declared by {}",
                            definition.name.as_str(),
                            declaring_file(&definition.origin)
                        )
                    },
                )
        })
        .collect()
}

/// The file an origin points at, by its name alone — a whole path renders with
/// the platform's own separators and could not be written down here portably.
fn declaring_file(origin: &DefinitionOrigin) -> String {
    Path::new(origin.as_str())
        .file_name()
        .and_then(OsStr::to_str)
        .unwrap_or(origin.as_str())
        .to_owned()
}

/// A content root at `at` whose `blocks/` holds exactly `declarations`, and
/// which declares no HUD at all.
///
/// A root declaring no HUD is a valid root, so nothing here is refused for a
/// reason these scenarios are not about.
///
/// # Errors
///
/// Returns an error if the directory or a file cannot be written.
fn a_root_declaring(at: &Path, declarations: &[(&str, &str)]) -> Result<(), Box<dyn Error>> {
    let blocks = at.join("blocks");
    fs::create_dir_all(&blocks)?;
    for (file_name, declaration) in declarations {
        fs::write(blocks.join(file_name), declaration)?;
    }
    Ok(())
}
