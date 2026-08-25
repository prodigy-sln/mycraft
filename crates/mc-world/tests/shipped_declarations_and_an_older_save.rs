//! A world saved before this repository's declarations changed loads afterwards,
//! and what it reports about its blocks is exactly what really moved.
//!
//! **This is the only fixed oracle in the whole feature over a whole resolved
//! definition.** Every other reading of a declaration compares one field against
//! an expectation written beside it; this one compares the lot at once, against
//! numbers nobody here computed. A save records each block it holds as two folds —
//! its declared behaviour over `name`, `is_solid`, `replaceable`, `breakable`,
//! `breaks_into`, `targetable`, `swimmable` and `move_resistance`, and its declared
//! appearance over `name`, the keys its six faces draw from, `drawn` and
//! `occludes` — and deliberately excludes the origin, precisely so that a save does
//! not depend on the path a definition was read from. So a field mapped to the
//! wrong place, a default resolved differently by a new reader, or a texture key
//! that quietly became the block's own name all show up here and nowhere else.
//!
//! **Both lists are restated here in full rather than cited**, because a doc
//! comment naming five of eight is the same hand-maintained mirror this project
//! has twice found sitting short while the thing it mirrors grew. This one is
//! prose and no assertion rests on it, which makes it worse rather than better: a
//! stale list here misleads a reader with nothing to redden.
//!
//! **The oracle is a save written against the declarations that came before, and
//! it is committed rather than generated.** A save this suite wrote from the
//! declarations under test would agree with them by construction and could not
//! fail. `tests/fixtures/world_saved_against_the_toml_declarations.mcw` was
//! written from `content/base/` while its four blocks were still TOML, holding
//! all four of them, with the shipped reader of the day. It is not regenerated:
//! the day it is, this test stops being evidence about anything.
//!
//! # Both of a block's records have now moved, and the behaviour one moved on
//! purpose
//!
//! A block's recorded appearance folds **every key it declares**, one per facing,
//! and then whether it is drawn and whether it occludes. A block's recorded
//! behaviour folds its own field list, and now ends with whether a swing can find
//! the block and then with the two properties that say what its volume is to move
//! through. Each list carries a revision byte of its own and **each has moved
//! twice** — the appearance one for five texture keys and then for `drawn` and
//! `occludes`, the behaviour one for `targetable` and then for `swimmable` and
//! `move_resistance` together.
//!
//! **Both bytes now read 3, and that is a coincidence of counting rather than an
//! argument for one byte.** They arrived there by different routes, one growth
//! apart, for unrelated reasons, and the next change to either list moves one of
//! them alone. It is the least durable fact on this page, and it is stated so that
//! nobody reads today's equality as a case for collapsing the two constants into
//! one — which would move every behaviour fold in existence as a side effect of a
//! texture key. `docs/technical/world-format.md` says the same thing in the same
//! words, for the same reason.
//!
//! **So every block this save holds is reported as behaving differently, and that
//! is the designed answer rather than a defect.** A behaviour list that grows is a
//! list every existing save recorded under the old shape, and there is no reading
//! of those bytes that could say otherwise. The cost is paid once, deliberately,
//! and it is survivable because such a save *loads* and names its blocks instead
//! of being refused. What would be wrong now is a **list that failed to grow**: an
//! implementation folding the new fields into nothing at all reports water alone,
//! over its `breakable = false`, and three blocks as merely retextured — which is
//! the answer this file used to expect and is now the defect it catches.
//!
//! **What it cannot catch is a byte that failed to move, and that is measured
//! rather than argued.** Putting `BEHAVIOUR_REVISION` back to 2 with the list left
//! grown reddens exactly two tests in the workspace — the two that state a byte
//! sequence by hand — and every reading in this file stays green, because a fold
//! over a grown list disagrees with a recorded one whatever the leading byte says.
//! That is not a gap in the fixture: which *list* a field joined and which *byte*
//! it moved are different questions, and this file answers the first. It is stated
//! so that nobody later reads a committed save as closing the hole only a
//! byte-stating test closes.
//!
//! **The two revision bytes stay separate, and that is unchanged.** One number
//! shared between the lists would move every behaviour fold in existence as a
//! side effect of adding a texture key, which is a claim about how every block
//! behaves made on the strength of art. That the behaviour byte has moved here is
//! two decisions somebody took, about `targetable` and then about the medium pair;
//! a shared byte would have taken them for them, on every future retexture,
//! forever.
//!
//! # There are two committed saves now, and each answers what the other cannot
//!
//! This one predates the move to Luau, so **both** of its records are stale: it
//! can say that every block behaves differently and every block looks different,
//! and it cannot separate the two. `world_saved_against_behaviour_revision_2.mcw`,
//! read by `shipped_declarations_and_a_revision_2_save.rs`, was minted while the
//! behaviour list stood at 2 and the appearance list already stood at 3 — so under
//! today's declarations every block it holds behaves differently and **not one of
//! them looks different**, which is the asymmetry that catches a medium field
//! folded onto the appearance list. Neither fixture subsumes the other, and
//! neither can be minted a second time.
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
fn the_shipped_content_reports_every_block_the_older_save_holds_as_behaving_differently()
-> TestResult {
    let needed = requirements(&older_save()?)?;

    let verdict = resolve(&needed, &shipped_registry()?);

    assert_eq!(
        verdict,
        RegistryVerdict {
            missing: Vec::new(),
            changed: every_block_the_older_save_holds()?,
            retextured: Vec::new(),
        },
        "this is the one fixed oracle over whole resolved definitions, and what it says is what a \
         player pays for a behaviour list that grew: every block of every save written before it \
         is reported. `retextured` is necessarily empty because behaviour is asked first and \
         answers alone, so the appearance byte having moved as well is invisible here — which is \
         why the byte sequences have guards of their own. The verdict is compared whole, and the \
         near miss it rules out is the one that used to be right: water alone in `changed` over \
         its `breakable = false`, with the other three merely retextured, is exactly what an \
         implementation that folded the three new fields into nothing produces"
    );
    Ok(())
}

/// Both arms of the acceptance decision over the same save, in one reading.
///
/// **The strict arm is the one carrying the evidence**, and that is why both are
/// read here rather than only the default. Under the accepting default `refusal`
/// answers `None` for *any* changed list at all, so a reading of that arm alone
/// passes however badly the behaviour fold broke — every block reported changed,
/// no block reported changed, still `None`. The strict arm names the list, so it
/// disagrees with an implementation that folded nothing by three names.
#[test]
fn the_same_save_loads_by_default_and_is_refused_naming_all_four_when_strictness_is_asked_for()
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
                changed: every_block_the_older_save_holds()?,
            })
        ),
        "a player who has updated their content opens their world: the default answer over a save \
         whose blocks merely behave differently is no refusal at all, however many of them there \
         are. Somebody who asked for the strict answer is turned away and told every one — four \
         names and not one, which is the reading an implementation that folded no new field and \
         left the revision byte alone cannot satisfy"
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
            changed: every_block_the_older_save_holds()?
                .into_iter()
                .filter(|name| name.as_str() != WITHDRAWN_BLOCK)
                .collect(),
            retextured: Vec::new(),
        },
        "a save whose block nothing declares any more is a save nothing can put in that cell, and \
         it has to be named. A comparison that could not report it is one whose agreement above \
         means nothing. The three that remain move to `changed` rather than vanishing, which is \
         what keeps the two lists separate: a missing block is not a judgement a player is in a \
         position to make, and a changed one is exactly that"
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
