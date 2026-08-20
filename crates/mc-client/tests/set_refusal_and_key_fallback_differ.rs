//! The two things a missing texture can mean, and why they must never come to
//! be said the same way.
//!
//! # The pair
//!
//! *The build step was not run* is a refusal: nothing about the content is
//! wrong, the derived half of it simply is not there, and one command produces
//! it. *This key was never authored* is an ordinary state of a content root a
//! mod author meets on their first block, and it costs them a generated texture
//! rather than a launch. A client that said the same thing for both would send
//! somebody who forgot to run a build looking for a declaration they never wrote
//! — or, worse, teach them to ignore the sentence that tells them what to run.
//!
//! # What this phase can honestly assert about the second half
//!
//! The per-key fallback does not exist yet: no texel reaches the array texture
//! in this phase, so there is no key for the set to fail to cover in any way a
//! frame can see. What is observable now is that a current set covering no key
//! at all lets the launch through **without** the refusal that names the build
//! command, and that the refusal for an absent set names no key. Those are the
//! two halves of "these must not collapse" that exist today; the fallback itself
//! is asserted where it is built.
//!
//! # Both readings go through the launch, not past it
//!
//! A pure function that maps a verdict to a refusal proves nothing about whether
//! the client consults it. Both readings here run `prepare_scene` — the sequence
//! the goldens are shot through and the one a window opens on — so a preparation
//! that stopped judging the set altogether reddens rather than staying quiet.

use std::error::Error;
use std::path::Path;

use mc_client::startup::{BUILD_THE_TEXTURE_SET, prepare_scene};
use mc_client::textures::{SetVerdict, built_set};
use mc_core::art::{INDEX_FILE_NAME, TextureSetIndex};

mod support;

use support::{TestResult, built_sets, refusal_printed_over, repository_root};

/// The command a contributor runs to produce the set, as `tasks.md` states it
/// for `README.md`.
///
/// Spelled here **from the task and not from a run**: this is the one value in
/// the phase that four places have to agree on — the constant the refusal
/// interpolates, the two pages that tell somebody to run it, and this. A test
/// that read the constant and compared it to itself would agree with whatever
/// the constant happened to say.
const THE_COMMAND: &str = "cargo run -p voxforge -- build content/base/textures.toml";

/// The page that tells a contributor how to get a tree that runs.
const CONTRIBUTOR_PAGE: [&str; 1] = ["README.md"];

/// The page that tells a mod author what the set is and what refuses without it.
const MOD_AUTHOR_PAGE: [&str; 3] = ["docs", "modding", "voxel-models.md"];

#[test]
fn an_absent_set_emits_the_build_command_refusal_and_not_the_unauthored_key_wording() -> TestResult
{
    let root = built_sets::a_root_with_a_built_set()?;
    let covered = keys_the_set_covers(root.path())?;
    built_sets::without_the_index(root.path())?;

    let printed = refusal_printed_over(root.path())?;

    assert!(
        printed.contains(BUILD_THE_TEXTURE_SET),
        "a launch stopped by an unbuilt set is stopped by one command not having been run, and \
         the refusal has to say which. It said: {printed}"
    );
    let named: Vec<&String> = covered
        .iter()
        .filter(|key| printed.contains(*key))
        .collect();
    assert!(
        named.is_empty(),
        "the refusal for a set that was never built names {named:?}, which is the shape of the \
         other message entirely — a key the set does not cover is not why this launch stopped, \
         and somebody reading this would go looking for a declaration they never wrote. It said: \
         {printed}"
    );
    Ok(())
}

#[test]
fn a_current_set_covering_no_declared_key_launches_without_the_build_command_refusal() -> TestResult
{
    let root = built_sets::a_root_with_a_built_set()?;
    built_sets::with_an_index_naming_no_keys(root.path())?;
    // Stated rather than assumed: what makes this reading about the *fallback*
    // and not about staleness is that the set is genuinely current while
    // covering nothing the blocks declare.
    let (verdict, _texels) = built_set(root.path())?;
    assert_eq!(verdict, SetVerdict::Current);

    let prepared = prepare_scene(root.path());

    assert!(
        prepared.is_ok(),
        "a set that covers no key a block declares is a mod author's ordinary first state and \
         must not stop a launch. It stopped with: {}",
        prepared
            .err()
            .map_or_else(String::new, |refused| refused.to_string())
    );
    Ok(())
}

#[test]
fn the_command_the_refusal_quotes_is_the_one_the_pages_tell_a_contributor_to_run() -> TestResult {
    // A message quoting a command nothing accepts reads as a way out and is not
    // one, and a page quoting a different command is the same failure one step
    // further away from whoever hits it.
    assert_eq!(BUILD_THE_TEXTURE_SET, THE_COMMAND);
    let repository = repository_root()?;
    for page in [CONTRIBUTOR_PAGE.as_slice(), MOD_AUTHOR_PAGE.as_slice()] {
        let path = page
            .iter()
            .fold(repository.clone(), |at, part| at.join(part));
        let written = std::fs::read_to_string(&path)?;
        assert!(
            written.contains(THE_COMMAND),
            "{} tells somebody how to get a tree that runs, and it does not quote `{THE_COMMAND}`",
            path.display()
        );
    }
    Ok(())
}

/// Every texture key the set under `root` covers, as its index records them.
///
/// **Read from the fixture rather than written out here**, so that the refusal
/// is checked against every key the shipped art actually holds instead of
/// against one somebody remembered. A key added to the manifest later is
/// checked the day it is built.
///
/// # Errors
///
/// Returns an error if the index cannot be read or parsed, or if it covers no
/// key — a set covering nothing gives this reading nothing to look for and it
/// would pass for the wrong reason.
fn keys_the_set_covers(root: &Path) -> Result<Vec<String>, Box<dyn Error>> {
    let index = root.join(built_sets::SET_DIRECTORY).join(INDEX_FILE_NAME);
    let parsed = TextureSetIndex::parse(&std::fs::read_to_string(&index)?)?;
    let covered: Vec<String> = parsed
        .entries()
        .iter()
        .map(|entry| entry.key.as_str().to_owned())
        .collect();
    if covered.is_empty() {
        return Err(format!(
            "this reading asks that a refusal names none of the keys the set covers, and the \
             index at {} covers none. There would be nothing to look for and the assertion would \
             hold for a reason that has nothing to do with what was printed",
            index.display()
        )
        .into());
    }
    Ok(covered)
}
