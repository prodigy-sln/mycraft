//! A content root registers every declaration it holds or none of them, and a
//! name declared twice names both places that declared it.
//!
//! None of this is the loader's own machinery: all-or-nothing application, the
//! empty-source refusal and the duplicate-name refusal all come from the
//! registry, which this feature is not allowed to touch. What is asserted here
//! is that reading declarations out of Luau files reaches the registry on the
//! same terms the previous reader did — a loader that registered as it went, or
//! that swallowed a refusal and carried on, would be a new failure arriving
//! through a new door.
//!
//! # The registry these tests apply to is not empty
//!
//! "Leaves the registry holding exactly what it held before" is a claim about a
//! registry that held something. Against an empty one it is satisfied by any
//! implementation that fails, including one that failed for the wrong reason
//! after registering nothing it was ever going to register.

mod common;
mod luau_common;

use std::error::Error;
use std::path::PathBuf;

use common::{
    FIXTURE_ORIGIN, TestResult, content_root, directory_label, registered_names,
    registry_from as registry_holding,
};
use luau_common::{AMBER, declaring, refusal_from};
use mc_core::block::source::DefinitionSourceError;
use mc_core::block::{BlockRegistry, RegistryError};
use mc_world::content::LuauFileDefinitionSource;
use tempfile::TempDir;

/// The block a registry already holds before a content root is applied to it.
const ALREADY_HELD: &str = "fixture:granite";

/// The texture that block was registered with, which is not its own name.
const ALREADY_HELD_TEXTURE: &str = "fixture:granite_face";

/// A declaration stating a solidity that is not a boolean, which the loader must
/// refuse.
fn a_declaration_that_cannot_be_read() -> String {
    "return {\n\tname = 'example:zinc',\n\ttexture = 'example:quartz',\n\tsolid = 'yes',\n}\n"
        .to_owned()
}

/// The three names the mixed root below declares.
const THREE_DECLARED: [&str; 3] = ["example:amber", "example:cobalt", "example:zinc"];

/// The one file in that root that cannot be read.
const UNREADABLE_FILE: &str = "zinc.luau";

/// The declaration file a refusal blames, where it blames one at all.
///
/// `None` covers every other shape of refusal, which is what makes this
/// discriminating: a root turned away for some reason of the loader's own —
/// before the file this fixture broke was ever reached — leaves the same
/// registry behind and has to be told apart from a declaration being refused.
fn refused_declaration_in(refusal: &RegistryError) -> Option<String> {
    let RegistryError::Source(DefinitionSourceError::Malformed(fault)) = refusal else {
        return None;
    };
    fault
        .origin
        .as_str()
        .rsplit(['/', '\\'])
        .next()
        .map(str::to_owned)
}

/// A root of three declarations of which the last-named one cannot be read.
///
/// The two sound files are written first, so a loader that registered each
/// declaration as it read it would be holding two blocks by the time it met the
/// third — whichever order the directory happens to be handed back in.
fn root_with_one_unreadable_declaration(directory: &TempDir) -> Result<PathBuf, Box<dyn Error>> {
    content_root(
        directory,
        &[
            ("amber.luau", declaring("example:amber")),
            ("cobalt.luau", declaring("example:cobalt")),
            (UNREADABLE_FILE, a_declaration_that_cannot_be_read()),
        ],
    )
}

/// The three names, sorted, as a registry reports them.
fn three_declared() -> Vec<String> {
    let mut names: Vec<String> = THREE_DECLARED.into_iter().map(String::from).collect();
    names.sort();
    names
}

#[test]
fn a_root_whose_third_declaration_is_refused_leaves_the_registry_holding_what_it_held() -> TestResult
{
    let directory = TempDir::new()?;
    let root = root_with_one_unreadable_declaration(&directory)?;
    let mut registry = registry_holding(
        FIXTURE_ORIGIN,
        &[(ALREADY_HELD, ALREADY_HELD_TEXTURE, true)],
    )?;

    let refusal = registry
        .apply(&LuauFileDefinitionSource::new(&root))
        .err()
        .ok_or("a root holding a declaration that cannot be read must not be accepted")?;

    assert_eq!(
        (
            refused_declaration_in(&refusal),
            registered_names(&registry)?.into_iter().collect::<Vec<_>>()
        ),
        (
            Some(String::from(UNREADABLE_FILE)),
            vec![ALREADY_HELD.to_owned()]
        ),
        "one declaration a mod author got wrong costs them that mod and not half of it. A \
         registry left holding two of three blocks is worse than one holding none: the game \
         starts, the missing block is discovered by whoever tries to place it, and nothing says \
         which file was skipped. The refusal is checked as well as the registry's contents, \
         because a loader that failed for some reason of its own — never reaching the third file \
         at all — would leave exactly the same registry behind"
    );
    Ok(())
}

#[test]
fn a_declarations_directory_holding_nothing_at_all_is_refused_naming_the_root() -> TestResult {
    let directory = TempDir::new()?;
    let root = content_root(&directory, &[])?;

    let (_, refusal) = refusal_from(&root)?;

    let RegistryError::NoDefinitions { origin } = &refusal else {
        return Err(format!("expected an empty-source refusal, got {refusal:?}").into());
    };
    assert!(
        origin.as_str().contains(directory_label(&root)?),
        "a root that declared nothing is a mod that will not work, and saying so at load is the \
         only moment anybody is looking. Registering an empty set instead produces a running \
         game with no blocks in it and no refusal anywhere: got `{}`",
        origin.as_str()
    );
    Ok(())
}

#[test]
fn a_root_whose_refused_declaration_was_repaired_registers_all_of_its_blocks() -> TestResult {
    let directory = TempDir::new()?;
    let root = root_with_one_unreadable_declaration(&directory)?;
    let mut registry = BlockRegistry::new();
    let source = LuauFileDefinitionSource::new(&root);
    if registry.apply(&source).is_ok() {
        return Err(
            "the unrepaired root must be refused, or the repair below proves nothing".into(),
        );
    }

    std::fs::write(
        root.join("blocks").join(UNREADABLE_FILE),
        declaring("example:zinc"),
    )?;
    registry.apply(&source)?;

    assert_eq!(
        registered_names(&registry)?.into_iter().collect::<Vec<_>>(),
        three_declared(),
        "a refusal has to be a thing a mod author can fix and try again. The same registry and \
         the same source are used for both attempts, so a loader that kept anything from the \
         first reading — a half-filled registry, a host it could not reuse — arrives at the \
         second with it"
    );
    Ok(())
}

/// A root whose two declaration files both claim one name.
///
/// Written in this order and named in this order, so creation order and
/// file-name order agree. Which of the two a directory listing hands back first
/// is not this phase's contract — the loader imposes no order of its own yet —
/// and a fixture whose two candidate orderings disagreed would make the
/// assertion below a fact about the filesystem.
fn root_declaring_one_name_twice(directory: &TempDir) -> Result<PathBuf, Box<dyn Error>> {
    content_root(
        directory,
        &[
            (FIRST_CLAIM, declaring(AMBER)),
            (SECOND_CLAIM, declaring(AMBER)),
        ],
    )
}

/// The file that claims the name first, and the one that claims it after.
const FIRST_CLAIM: &str = "amber.luau";
const SECOND_CLAIM: &str = "zinc.luau";

#[test]
fn two_declarations_of_one_name_are_refused_naming_both_files() -> TestResult {
    let directory = TempDir::new()?;
    let root = root_declaring_one_name_twice(&directory)?;

    let (_, refusal) = refusal_from(&root)?;

    let RegistryError::AlreadyRegistered {
        name,
        first,
        second,
    } = &refusal
    else {
        return Err(format!("expected an already-registered refusal, got {refusal:?}").into());
    };
    assert_eq!(
        (
            name.as_str(),
            first.as_str().contains(FIRST_CLAIM),
            second.as_str().contains(SECOND_CLAIM)
        ),
        (AMBER, true, true),
        "two mods claiming one name is the failure a server operator meets, and the only thing \
         that helps them is being told which two files. A refusal naming one file twice, or \
         naming the directory, sends them reading every declaration in the pack: first `{}`, \
         second `{}`",
        first.as_str(),
        second.as_str()
    );
    Ok(())
}
