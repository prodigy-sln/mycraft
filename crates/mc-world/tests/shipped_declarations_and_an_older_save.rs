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
//! every block in it looks different now. That is correct rather than a migration
//! defect, and a retexture is loaded without a word said about it.
//!
//! **What must not have moved is the other half, and one block's half really
//! did.** A block's behaviour is folded over its own field list under its own
//! revision. `content/base/blocks/water.luau` declares `breakable = false`, which
//! is one of the five fields that fold goes over, so water — and water alone — is
//! reported as behaving differently. That is the two-hash separation firing on a
//! real content edit rather than on a fixture, and it is the whole reason the
//! expectations below name four blocks in two lists rather than four in one.
//!
//! The revision byte is per field list and not one number shared between the two:
//! bumping a shared one would move every behaviour fold in existence as a side
//! effect of adding a texture key, and the split below is what says it did not.
//! **A shared byte reports all four blocks as changed and none as retextured**,
//! which is a different answer from every expectation here rather than a nearly
//! equal one.
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
use mc_world::persistence::{Acceptance, LoadError, RegistryVerdict, requirements, resolve};
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

/// The one shipped block whose declared *behaviour* moved after this save was
/// written: `content/base/blocks/water.luau` states `breakable = false`, and
/// `breakable` is one of the five fields a behaviour fold goes over.
const BEHAVES_DIFFERENTLY: &str = "base:water";

/// The three whose *appearance* alone moved, ascending.
const LOOK_DIFFERENT: [&str; 3] = ["base:dirt", "base:grass", "base:stone"];

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
fn the_shipped_content_reports_water_as_behaving_differently_and_the_other_three_as_retextured()
-> TestResult {
    let needed = requirements(&older_save()?)?;

    let verdict = resolve(&needed, &shipped_registry()?);

    assert_eq!(
        verdict,
        RegistryVerdict {
            missing: Vec::new(),
            changed: vec![BlockName::parse(BEHAVES_DIFFERENTLY)?],
            retextured: named(&LOOK_DIFFERENT)?,
        },
        "this is the one fixed oracle over whole resolved definitions, and what it says is that \
         the two halves of a block's record moved independently. `{BEHAVES_DIFFERENTLY}` states \
         `breakable = false`, which is a behaviour field, so it lands in `changed`; the other \
         three declare the same behaviour they always did and differ only in folding a key per \
         facing, so they land in `retextured`. A revision byte shared between the two field \
         lists puts all four in `changed` and leaves `retextured` empty, and a field mapped to \
         the wrong list swaps a name between them — neither of which is a near miss of this \
         expectation"
    );
    Ok(())
}

/// Both arms of the acceptance decision over the same save, in one reading.
///
/// **The argument is not simply flipped from what this test used to pass**, and
/// that matters. Under the accepting default `refusal` answers `None` for *any*
/// changed list at all, so a reading that asserted only that arm would pass
/// however badly the behaviour fold broke — a shared revision byte, every block
/// reported changed, still `None`. The strict arm is what carries the evidence:
/// it names the changed blocks, so it disagrees with a shared byte by three names.
#[test]
fn the_same_save_loads_by_default_and_is_refused_naming_water_alone_when_strictness_is_asked_for()
-> TestResult {
    let needed = requirements(&older_save()?)?;

    let verdict = resolve(&needed, &shipped_registry()?);

    assert_eq!(
        (
            verdict.refusal(Acceptance::ChangedBlocksToo),
            verdict.refusal(Acceptance::OnlyUnchangedBlocks)
        ),
        (
            None,
            Some(LoadError::Unresolvable {
                missing: Vec::new(),
                changed: vec![BlockName::parse(BEHAVES_DIFFERENTLY)?],
            })
        ),
        "a player who has updated their content opens their world: the default answer over a save \
         whose blocks merely behave differently is no refusal at all. Somebody who asked for the \
         strict answer is turned away and told which block it was — one name and not four, which \
         is the reading a revision byte shared between the two field lists cannot satisfy"
    );
    Ok(())
}

/// Every block the committed save holds, as a `BlockName`, ascending.
///
/// # Errors
///
/// Returns an error if one of the four is not a namespaced id.
fn every_block_the_older_save_holds() -> Result<Vec<BlockName>, Box<dyn Error>> {
    named(&NEEDED_BY_THE_OLDER_SAVE)
}

/// `texts` as block names, in the order given.
///
/// # Errors
///
/// Returns an error if one of them is not a namespaced id.
fn named(texts: &[&str]) -> Result<Vec<BlockName>, Box<dyn Error>> {
    texts
        .iter()
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
