//! A world saved before this repository's declarations changed loads afterwards,
//! and what it reports about its blocks is exactly what really moved.
//!
//! **This is the only fixed oracle in the whole feature over a whole resolved
//! definition.** Every other reading of a declaration compares one field against
//! an expectation written beside it; this one compares the lot at once, against
//! numbers nobody here computed. A save records each block it holds as two folds —
//! its declared behaviour over `name`, `is_solid`, `replaceable`, `breakable` and
//! `breaks_into`, and its declared appearance over `name` and the keys its faces
//! draw from — and deliberately excludes the origin, precisely so that a save does
//! not depend on the path a definition was read from. So a field mapped to the
//! wrong place, a default resolved differently by a new reader, or a texture key
//! that quietly became the block's own name all show up here and nowhere else.
//!
//! **The oracle is a save written against the declarations that came before, and
//! it is committed rather than generated.** A save this suite wrote from the
//! declarations under test would agree with them by construction and could not
//! fail. `tests/fixtures/world_saved_against_the_toml_declarations.mcw` was
//! written from `content/base/` while its four blocks were still TOML, holding
//! all four of them, with the shipped reader of the day. It is not regenerated:
//! the day it is, this test stops being evidence about anything.
//!
//! # What a block's appearance now folds, and why that shows up here
//!
//! A block's recorded appearance folds **every key it declares**, one per facing,
//! under a revision byte of its own. Every save written before that — this fixture
//! included — recorded an appearance over one key under the previous revision, so
//! every block in it is reported **retextured**. That is correct rather than a
//! migration defect: every block's appearance really did change, and a retexture
//! is loadable without asking anybody anything.
//!
//! **What must not have moved is the other half.** A block's behaviour is folded
//! over its own field list under its own revision, this feature adds nothing to
//! it, and a save reporting its blocks as *changed* would stop a player at a
//! prompt about a rebalance that never happened. The revision byte is therefore
//! per field list and not one number shared between the two: bumping a shared one
//! would move every behaviour fold in existence as a side effect of adding a
//! texture key. The second reading below is what says it did not.
//!
//! # An empty verdict is what the wrong answer looks like too
//!
//! "No block is missing" is an absence, and a comparison against it passes over a
//! save that needs no blocks at all just as happily as over one whose blocks all
//! resolve. So two further tests stand behind these: one saying the fixture really
//! does need the four blocks by name, and one saying this same comparison reports
//! a block whose declaration has gone. Without the pair, a fixture that lost its
//! name table would read as agreement forever.

mod common;

use std::error::Error;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

use common::TestResult;
use mc_core::block::BlockRegistry;
use mc_core::id::BlockName;
use mc_world::content::LuauFileDefinitionSource;
use mc_world::persistence::{Acceptance, RegistryVerdict, requirements, resolve};
use tempfile::TempDir;

/// The save the swap is judged against, written before it.
const OLDER_SAVE: &str = "tests/fixtures/world_saved_against_the_toml_declarations.mcw";

/// Every block that save holds, in the ascending order a requirements report
/// lists them in.
const NEEDED_BY_THE_OLDER_SAVE: [&str; 4] = ["base:dirt", "base:grass", "base:stone", "base:water"];

/// The block whose declaration the falsifying control takes away — by the name of
/// the file that declares it, and by the name the save knows it as.
const WITHDRAWN: &str = "water";
const WITHDRAWN_BLOCK: &str = "base:water";

/// What the shipped content declares its blocks in, and what it declares its HUD
/// in.
///
/// The two are deliberately different and stay different: this feature moves
/// block declarations into the language a mod author writes and leaves the HUD
/// format alone, so a change that swept both would be doing something nobody
/// asked for.
const BLOCK_EXTENSION: &str = "luau";
const HUD_EXTENSION: &str = "toml";

#[test]
fn a_save_written_before_appearances_folded_facing_keys_reports_its_blocks_as_retextured()
-> TestResult {
    let needed = requirements(&older_save()?)?;

    let verdict = resolve(&needed, &shipped_registry()?);

    assert_eq!(
        verdict,
        RegistryVerdict {
            missing: Vec::new(),
            changed: Vec::new(),
            retextured: every_block_the_older_save_holds()?,
        },
        "an appearance recorded before it folded a key per facing was folded over different \
         fields under a different revision, so the two are not comparable and the honest answer \
         is that every block looks different. Comparing them anyway would be worse than either \
         answer: two values folded over different field lists agree or disagree for reasons \
         nothing in the save records"
    );
    Ok(())
}

#[test]
fn a_save_written_before_this_format_is_still_loaded_without_the_player_being_asked() -> TestResult
{
    let needed = requirements(&older_save()?)?;

    let verdict = resolve(&needed, &shipped_registry()?);

    assert_eq!(
        verdict.refusal(Acceptance::OnlyUnchangedBlocks),
        None,
        "every hash this save stores other than a block's appearance is unchanged, and a player \
         opening their world is not stopped. The way that breaks is one number: a revision byte \
         shared between the appearance field list and the behaviour one moves both when only the \
         first gained a field, and every save in existence then reports every block as behaving \
         differently — a prompt about a rebalance that never happened, on a world nothing is \
         wrong with"
    );
    Ok(())
}

/// Every block the committed save holds, as a `BlockName`, ascending.
///
/// # Errors
///
/// Returns an error if one of the four is not a namespaced id.
fn every_block_the_older_save_holds() -> Result<Vec<BlockName>, Box<dyn Error>> {
    NEEDED_BY_THE_OLDER_SAVE
        .into_iter()
        .map(|name| Ok(BlockName::parse(name)?))
        .collect()
}

/// The control the comparison above cannot supply for itself, in the direction
/// that empties it.
///
/// A verdict naming nothing is what a save needing nothing produces. If the
/// fixture's name table were ever lost — regenerated from an empty world, say, or
/// truncated — the test above would go on agreeing while comparing four blocks
/// against none of them.
#[test]
fn the_committed_save_really_does_need_all_four_of_the_blocks_the_base_game_ships() -> TestResult {
    let needed = requirements(&older_save()?)?;

    let mut names: Vec<String> = needed
        .names()
        .map(|name| name.as_str().to_owned())
        .collect();
    names.sort();

    assert_eq!(
        names, NEEDED_BY_THE_OLDER_SAVE,
        "this fixture is the only fixed oracle over a whole resolved definition, and it is one \
         only while it holds the blocks it is supposed to be about"
    );
    Ok(())
}

/// The control in the other direction: the same comparison, against a registry
/// one declaration short.
///
/// A verdict that came back empty whatever it was handed would agree with the
/// shipped content forever. This is the reading that says it does not.
#[test]
fn the_same_comparison_reports_a_block_whose_declaration_the_content_no_longer_holds() -> TestResult
{
    let needed = requirements(&older_save()?)?;
    let stripped = shipped_copy_without(WITHDRAWN)?;

    let verdict = resolve(&needed, &registry_over(stripped.path())?);

    assert_eq!(
        verdict,
        RegistryVerdict {
            missing: vec![BlockName::parse(WITHDRAWN_BLOCK)?],
            changed: Vec::new(),
            retextured: every_block_the_older_save_holds()?
                .into_iter()
                .filter(|name| name.as_str() != WITHDRAWN_BLOCK)
                .collect(),
        },
        "a save whose block nothing declares any more is a save nothing can put in that cell, and \
         it has to be named. A comparison that could not report it is one whose agreement above \
         means nothing. The three that remain are still reported retextured, which is what keeps \
         the two lists separate: a missing block is not a judgement a player is in a position to \
         make, and a retextured one needs no judgement at all"
    );
    Ok(())
}

/// What the shipped content root is made of, which is the fact both readings
/// above are about and neither of them states.
///
/// A root still holding the block declarations in the format that was replaced
/// leaves two readers able to answer for the same directory, and whichever one a
/// caller reaches for decides what a player gets. The HUD half is asserted beside
/// it because the same fixture constant once served both, and a swap that took
/// the HUD with it would be silent everywhere else.
#[test]
fn the_shipped_root_declares_its_blocks_in_luau_and_its_hud_in_the_format_that_did_not_change()
-> TestResult {
    let root = shipped_root()?;

    let extensions = (
        extensions_under(&root.join("blocks"))?,
        extensions_under(&root.join("hud"))?,
    );

    assert_eq!(
        extensions,
        (
            vec![BLOCK_EXTENSION.to_owned()],
            vec![HUD_EXTENSION.to_owned()]
        ),
        "a block declaration left behind in the format that was retired is a file no reader will \
         ever open again and every reader of the directory still sees, and the HUD declarations \
         beside it are not part of this change at all"
    );
    Ok(())
}

/// The save the readings above are judged against.
///
/// # Errors
///
/// Returns an error if the repository root cannot be located.
fn older_save() -> Result<PathBuf, Box<dyn Error>> {
    Ok(Path::new(env!("CARGO_MANIFEST_DIR")).join(OLDER_SAVE))
}

/// The content root the game ships.
///
/// # Errors
///
/// Returns an error if the repository root cannot be located.
fn shipped_root() -> Result<PathBuf, Box<dyn Error>> {
    Ok(common::repository_root()?.join("content").join("base"))
}

/// A registry holding what the shipped content declares.
///
/// # Errors
///
/// Returns an error if the shipped root cannot be located or is refused.
fn shipped_registry() -> Result<BlockRegistry, Box<dyn Error>> {
    registry_over(&shipped_root()?)
}

/// A registry holding what the content root at `root` declares.
///
/// # Errors
///
/// Returns an error if the root is refused — which is a fact about the content
/// and is reported as one rather than folded into a verdict.
fn registry_over(root: &Path) -> Result<BlockRegistry, Box<dyn Error>> {
    let mut registry = BlockRegistry::new();
    registry.apply(&LuauFileDefinitionSource::new(root.to_owned()))?;
    Ok(registry)
}

/// A copy of the shipped content root with the block file named `stem` taken
/// out, whatever it is spelled with.
///
/// # Errors
///
/// Returns an error if the copy fails, or if the shipped root declares no such
/// file — a root that never declared the block is not a root whose declaration
/// was withdrawn, and the control would then be about a save requirement nothing
/// was ever going to answer.
fn shipped_copy_without(stem: &str) -> Result<TempDir, Box<dyn Error>> {
    let copied = TempDir::new()?;
    copy_tree(&shipped_root()?, copied.path())?;
    let withdrawn: Vec<PathBuf> = entries_in(&copied.path().join("blocks"))?
        .into_iter()
        .filter(|declared| declared.file_stem() == Some(OsStr::new(stem)))
        .collect();
    for declared in &withdrawn {
        fs::remove_file(declared)?;
    }
    if withdrawn.is_empty() {
        return Err(format!(
            "this control has to take the declaration of `{stem}` out of a copy of the shipped \
             content root, and the root declares no such file. What it would build is a root the \
             save's requirements are all answerable from, and the reading below would be the one \
             above written twice"
        )
        .into());
    }
    Ok(copied)
}

/// The distinct extensions the files directly under `directory` carry, sorted.
///
/// # Errors
///
/// Returns an error if the directory cannot be read, or if it holds a file with
/// no extension at all — which is neither of the two answers this is asking
/// about and must not be reported as either.
fn extensions_under(directory: &Path) -> Result<Vec<String>, Box<dyn Error>> {
    let mut found: Vec<String> = entries_in(directory)?
        .iter()
        .filter(|declared| !declared.is_dir())
        .map(PathBuf::as_path)
        .map(extension_of)
        .collect::<Result<Vec<_>, _>>()?;
    found.sort();
    found.dedup();
    Ok(found)
}

/// What `path` is spelled with.
///
/// # Errors
///
/// Returns an error if it carries no extension at all — which is neither of the
/// two answers this file is asking about and must not be reported as either.
fn extension_of(path: &Path) -> Result<String, Box<dyn Error>> {
    path.extension()
        .and_then(OsStr::to_str)
        .map(str::to_owned)
        .ok_or_else(|| format!("{} carries no extension at all", path.display()).into())
}

/// Every entry directly under `directory`, whatever it is called.
///
/// # Errors
///
/// Returns an error if the directory cannot be read.
fn entries_in(directory: &Path) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    let mut found = Vec::new();
    for entry in fs::read_dir(directory)? {
        found.push(entry?.path());
    }
    Ok(found)
}

/// Copies every file and directory under `from` into `into`.
fn copy_tree(from: &Path, into: &Path) -> Result<(), Box<dyn Error>> {
    fs::create_dir_all(into)?;
    for entry in fs::read_dir(from)? {
        let source: PathBuf = entry?.path();
        let Some(name) = source.file_name() else {
            continue;
        };
        let destination = into.join(name);
        if source.is_dir() {
            copy_tree(&source, &destination)?;
        } else {
            fs::copy(&source, &destination)?;
        }
    }
    Ok(())
}
