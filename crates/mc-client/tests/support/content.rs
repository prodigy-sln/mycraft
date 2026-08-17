//! Content roots built from the shipped one, for scenarios about what a root
//! declares — and about what it stops declaring.
//!
//! Every fixture here is a real directory of real files. What these scenarios
//! ask about is the reading of a content root by the client's own startup, and a
//! mocked filesystem would assert nothing about that.
//!
//! **A root is always copied, never edited in place.** `content/base/` is the
//! product's own content: a fixture that removed a declaration from it would
//! leave the repository in whatever state the run ended in, and a run that
//! failed half way would leave it broken.
//!
//! **Removing a declaration that was never there is a failure, not a no-op.** A
//! root that never declared a crosshair is not a root with the crosshair taken
//! out, and a scenario about what its removal changes would be comparing two
//! frames that were never going to differ.

use std::error::Error;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

use mc_core::block::{BlockRegistry, RegistryError};
use mc_core::hud::{HudLayout, HudLoadError};
use mc_world::content::{LuauFileDefinitionSource, TomlFileHudSource};
use tempfile::TempDir;

use super::content_root;

/// The subdirectory of a content root that HUD declarations live in.
pub const HUD_DIRECTORY: &str = "hud";

/// The subdirectory of a content root that block definitions live in.
pub const BLOCK_DIRECTORY: &str = "blocks";

/// The extension a block declaration is written with, and the one a HUD
/// declaration is written with.
///
/// **Two constants where there was one, and that split is a trap rather than
/// tidying.** A single constant served both directories for as long as both were
/// spelled the same way, and the day block declarations became Luau an edit of it
/// would have retargeted every HUD fixture in this module at the same moment —
/// silently, because a HUD fixture that stops finding its declarations reports a
/// root that never declared one rather than a fixture that went looking for the
/// wrong thing. The two are separate because they answer to separate decisions:
/// blocks are declared in the language a mod author writes, and the HUD format is
/// deliberately untouched by that change.
///
/// **The silent version of that mistake is structurally impossible here, and
/// keeping it impossible is a decision rather than luck.** [`empty`] refuses a
/// directory that declared nothing to begin with — the rule this module's header
/// states, that removing a declaration which was never there is a failure rather
/// than a no-op — so a `declaring_no_hud` handed the block extension finds no
/// declaration and says so, loudly, instead of emptying nothing and passing. The
/// pairing is therefore checked by the fixture that uses it and not only by
/// whoever edits these two lines. A future helper that took an extension and
/// tolerated finding nothing would give that up without anything going red.
pub const BLOCK_DECLARATION_EXTENSION: &str = "luau";
pub const HUD_DECLARATION_EXTENSION: &str = "toml";

/// A content root written into a temporary directory, removed when this is
/// dropped.
///
/// The directory is held inside rather than handed back beside the path, because
/// a `TempDir` dropped one line early deletes the tree the test is still reading
/// from and the failure reads as a missing content root.
#[derive(Debug)]
pub struct ContentRoot {
    directory: TempDir,
}

impl ContentRoot {
    /// Where this root sits.
    #[must_use]
    pub fn path(&self) -> &Path {
        self.directory.path()
    }

    /// This root with one more block declaration written into `blocks/`.
    ///
    /// **Taken and handed back by value so that roots compose**: a scenario needing
    /// a root that declares one block and stops declaring another chains the two
    /// helpers rather than asking for a third one that does both.
    ///
    /// # Errors
    ///
    /// Returns an error if the write fails, or if the root already declares that
    /// file — a root that declared the block all along is not a root a block was
    /// added to, and a scenario about what the addition changes would be reading a
    /// root it never built.
    pub fn declaring_block(
        self,
        file_name: &str,
        declaration: &str,
    ) -> Result<Self, Box<dyn Error>> {
        let blocks = self.path().join(BLOCK_DIRECTORY);
        let declared = blocks.join(file_name);
        if declared.exists() {
            return Err(format!(
                "this fixture has to add `{BLOCK_DIRECTORY}/{file_name}` to a copy of the shipped \
                 content root, but the shipped root already declares it. What it would build is a \
                 root whose block came from the shipped content rather than from this fixture, and \
                 the declaration a scenario is about would be one nobody here wrote"
            )
            .into());
        }
        fs::create_dir_all(&blocks)?;
        fs::write(&declared, declaration)?;
        Ok(self)
    }

    /// This root with the named block declarations taken out of `blocks/`.
    ///
    /// **Taken and handed back by value for the same reason
    /// [`declaring_block`](Self::declaring_block) is**, and this is the pair that
    /// reason was written for: a root that declares a block of its own *and*
    /// stops declaring the ones a generator places is two operations on one copy,
    /// and neither of the two is a usable root on its own.
    ///
    /// **A root with nothing left in `blocks/` is not what this builds.**
    /// `BlockRegistry::apply` refuses a source that declares nothing at all, so
    /// such a root fails at *registration* — before anything that generates a
    /// world is reached, and a scenario about what a generator cannot place would
    /// then be passing or failing over a registry that was never built. Chain
    /// this after a `declaring_block` that leaves something behind.
    ///
    /// **Every name is checked before any file goes**, so a root this refuses is
    /// the root it was handed rather than a half-stripped one. The alternative
    /// leaves a temporary directory that declares some of what was asked for and
    /// not the rest, which is precisely the "root nobody built" this refuses in
    /// order to avoid.
    ///
    /// # Errors
    ///
    /// Returns an error if a named declaration was not there to remove — see this
    /// module's header for why that is a failure rather than nothing happening —
    /// or if a removal fails.
    pub fn not_declaring_blocks(self, file_names: &[&str]) -> Result<Self, Box<dyn Error>> {
        let blocks = self.path().join(BLOCK_DIRECTORY);
        if let Some(file_name) = file_names
            .iter()
            .find(|file_name| !blocks.join(file_name).is_file())
        {
            return Err(format!(
                "this fixture has to remove `{BLOCK_DIRECTORY}/{file_name}` from a copy of the \
                 shipped content root, but the shipped root does not declare it. What it would \
                 build is a root that never declared the block rather than one whose declaration \
                 was taken away, and a scenario about a world the generator can no longer build \
                 would be about a block nobody stopped declaring"
            )
            .into());
        }
        for file_name in file_names {
            fs::remove_file(blocks.join(file_name))?;
        }
        Ok(self)
    }

    /// This root with every block declaration but the one whose file is named
    /// `stem` taken out of `blocks/`.
    ///
    /// **The file's extension is deliberately not part of what this asks for.**
    /// Which extension a block declaration is written with is exactly what is
    /// changing under these fixtures, and a helper naming one would have to be
    /// rewritten at the moment of the swap — which is the moment a fixture is
    /// least likely to be looked at and most likely to retarget a scenario
    /// silently. What a scenario about the block a client holds is really about
    /// is which declarations are left, not what they are spelled in.
    ///
    /// # Errors
    ///
    /// Returns an error if `blocks/` cannot be read, if a removal fails, if the
    /// root declares no file named `stem`, or if it declares nothing else — a
    /// root that only ever declared one block is not a root the others were
    /// taken out of, and a scenario about what the remaining declaration cannot
    /// do would be about a root nobody stripped.
    pub fn declaring_only_the_block_file_named(self, stem: &str) -> Result<Self, Box<dyn Error>> {
        let blocks = self.path().join(BLOCK_DIRECTORY);
        let (kept, withdrawn): (Vec<PathBuf>, Vec<PathBuf>) = entries_in(&blocks)?
            .into_iter()
            .partition(|declared| declared.file_stem() == Some(OsStr::new(stem)));
        if kept.is_empty() || withdrawn.is_empty() {
            return Err(format!(
                "this fixture has to leave `{BLOCK_DIRECTORY}/{stem}` alone in a copy of the \
                 shipped content root and take every other declaration out, and the root it was \
                 given kept {kept:?} and would remove {withdrawn:?}. What it would build is a root \
                 that never declared the others, or one that does not declare `{stem}` at all, and \
                 a scenario about what the one remaining declaration cannot do would be about \
                 neither"
            )
            .into());
        }
        for declared in withdrawn {
            fs::remove_file(declared)?;
        }
        Ok(self)
    }

    /// This root with every block declaration taken out of `blocks/`.
    ///
    /// **A root whose `blocks/` declares nothing is a refusal shape of its own.**
    /// Registration refuses a source that declared nothing at all, and what that
    /// refusal names is the root — no block and no field, because there is
    /// neither. Built by emptying a copy of the shipped root rather than by
    /// making a bare directory, so what a scenario reads is the refusal a mod
    /// author gets from a root they broke rather than from a directory nobody
    /// would point the client at.
    ///
    /// # Errors
    ///
    /// Returns an error if `blocks/` cannot be read, if a removal fails, or if
    /// the root declared nothing there to begin with — see this module's header
    /// for why that is a failure rather than nothing happening.
    pub fn declaring_no_blocks(self) -> Result<Self, Box<dyn Error>> {
        empty(
            &self.path().join(BLOCK_DIRECTORY),
            BLOCK_DIRECTORY,
            BLOCK_DECLARATION_EXTENSION,
        )?;
        Ok(self)
    }

    /// This root with every HUD declaration taken out of `hud/`.
    ///
    /// A root declaring no HUD is a **valid** root, which is what makes this the
    /// fixture for the one HUD scenario that is not about a refusal. The
    /// directory is emptied and left in place: a root with no `hud/` at all is a
    /// second thing, and a scenario about a root that declares no element would
    /// then be about a directory that is not there.
    ///
    /// # Errors
    ///
    /// Returns an error if `hud/` cannot be read, if a removal fails, or if the
    /// root declared no HUD to begin with.
    pub fn declaring_no_hud(self) -> Result<Self, Box<dyn Error>> {
        empty(
            &self.path().join(HUD_DIRECTORY),
            HUD_DIRECTORY,
            HUD_DECLARATION_EXTENSION,
        )?;
        Ok(self)
    }
}

/// The shipped content root, copied whole into a temporary directory.
///
/// # Errors
///
/// Returns an error if the repository's content root cannot be located or
/// copied.
pub fn shipped_copy() -> Result<ContentRoot, Box<dyn Error>> {
    let directory = TempDir::new()?;
    copy_tree(&content_root()?, directory.path())?;
    Ok(ContentRoot { directory })
}

/// The shipped content root copied with the named HUD declarations removed.
///
/// # Errors
///
/// Returns an error if the copy fails, or if a named declaration was not there
/// to remove — see this module's header for why that is a failure rather than
/// nothing happening.
pub fn shipped_without(declarations: &[&str]) -> Result<ContentRoot, Box<dyn Error>> {
    let copied = shipped_copy()?;
    for file_name in declarations {
        let declared = copied.path().join(HUD_DIRECTORY).join(file_name);
        if !declared.is_file() {
            return Err(format!(
                "this fixture has to remove `{HUD_DIRECTORY}/{file_name}` from a copy of the \
                 shipped content root, but the shipped root does not declare it. What it would \
                 build is a root that never had a crosshair rather than one whose crosshair was \
                 taken away, and the two are not the same claim"
            )
            .into());
        }
        fs::remove_file(&declared)?;
    }
    Ok(copied)
}

/// The shipped content root copied with one block definition file renamed.
///
/// **The declaration inside is untouched; only its file name moves.** Blocks are
/// registered in file-name sorted order and a client holds the first solid block
/// in that order, so renaming one file is the smallest edit that changes which
/// block a run holds — and it changes nothing else: the same four blocks are
/// registered, the same world generates, and the same texture keys resolve to
/// the same layers. Deleting a definition instead would change the world as
/// well as the held block, and two frames differing for two reasons say nothing
/// about either.
///
/// # Errors
///
/// Returns an error if the copy or the rename fails, or if `from` was not there
/// to rename — a root that never declared it is not a root whose declaration
/// moved.
pub fn shipped_renaming_block(from: &str, to: &str) -> Result<ContentRoot, Box<dyn Error>> {
    let copied = shipped_copy()?;
    let blocks = copied.path().join(BLOCK_DIRECTORY);
    let declared = blocks.join(from);
    if !declared.is_file() {
        return Err(format!(
            "this fixture has to rename `{BLOCK_DIRECTORY}/{from}` inside a copy of the shipped \
             content root, but the shipped root does not declare it. What it would build is a \
             root that registers the same blocks in the same order, and the two frames a \
             scenario compares would then hold the same block for a reason nothing states"
        )
        .into());
    }
    fs::rename(&declared, blocks.join(to))?;
    Ok(copied)
}

/// The shipped content root copied with one more declaration written into
/// `hud/`.
///
/// # Errors
///
/// Returns an error if the copy or the write fails.
pub fn shipped_with(file_name: &str, declaration: &str) -> Result<ContentRoot, Box<dyn Error>> {
    let copied = shipped_copy()?;
    let declared = copied.path().join(HUD_DIRECTORY);
    fs::create_dir_all(&declared)?;
    fs::write(declared.join(file_name), declaration)?;
    Ok(copied)
}

/// The shipped content root copied with the named HUD declarations restating
/// their `outline` colour as their fill `color`.
///
/// **Both colours come out of the shipped declaration, so nothing here states a
/// colour of its own.** What this builds is the frame a negative control needs: a
/// crosshair whose fill pixels really are drawn, and drawn in the colour the same
/// declaration reserves for its outline. A prediction that accepted it would be
/// accepting "something was painted here" in place of "the declared colour was".
///
/// # Errors
///
/// Returns an error if the copy, the read or the write fails, or if a named
/// declaration states no `color` or no `outline` to move — a root that never
/// declared one is not a root whose fill colour changed.
pub fn shipped_filling_with_the_outline_color(
    declarations: &[&str],
) -> Result<ContentRoot, Box<dyn Error>> {
    let copied = shipped_copy()?;
    for file_name in declarations {
        let declared = copied.path().join(HUD_DIRECTORY).join(file_name);
        let stated = fs::read_to_string(&declared)?;
        let filled = line_of(&stated, COLOR_FIELD, file_name)?;
        let outlined = value_of(&stated, OUTLINE_FIELD, file_name)?;
        if filled.ends_with(&outlined) {
            return Err(format!(
                "`{HUD_DIRECTORY}/{file_name}` already fills with the colour it outlines with, so \
                 restating it changes nothing and the control below would be about the shipped \
                 declaration rather than about a fill in the wrong colour"
            )
            .into());
        }
        let restated = stated.replace(&filled, &format!("{COLOR_FIELD} = {outlined}"));
        fs::write(&declared, restated)?;
    }
    Ok(copied)
}

/// The field a declaration states its fill colour in, and the one it states its
/// contrast outline in, as a declaration spells them.
const COLOR_FIELD: &str = "color";
const OUTLINE_FIELD: &str = "outline";

/// The whole `field = value` line `stated` holds, matched from the start of the
/// line so the word inside one of these files' prose comments cannot be hit.
///
/// # Errors
///
/// Returns an error naming the field and the file when the declaration does not
/// state it.
fn line_of(stated: &str, field: &str, file_name: &str) -> Result<String, Box<dyn Error>> {
    let opening = format!("{field} = ");
    stated
        .lines()
        .find(|line| line.starts_with(&opening))
        .map(str::to_owned)
        .ok_or_else(|| {
            format!(
                "this fixture has to restate `{HUD_DIRECTORY}/{file_name}`'s `{field}`, but that \
                 declaration does not state it. What it would build is a root the control below \
                 was never going to be about"
            )
            .into()
        })
}

/// What `stated` states `field` as, quotes included.
///
/// # Errors
///
/// Returns an error naming the field and the file when the declaration does not
/// state it.
fn value_of(stated: &str, field: &str, file_name: &str) -> Result<String, Box<dyn Error>> {
    let line = line_of(stated, field, file_name)?;
    Ok(line
        .split_once(" = ")
        .map_or(line.clone(), |(_, value)| value.to_owned()))
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

/// Every block declaration directly under `directory`.
///
/// # Errors
///
/// Returns an error if the directory cannot be read.
pub fn block_declarations_in(directory: &Path) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    declarations_in(directory, BLOCK_DECLARATION_EXTENSION)
}

/// Every declaration file directly under `directory` written with `extension`.
///
/// The search is one directory deep and reads the extension rather than the
/// name, which is how both loaders decide what they are looking at — a fixture
/// counting anything else would be counting files the client never reads. The
/// extension is a parameter rather than a constant read here, so that every
/// caller states which of the two directories it is talking about at the point
/// it asks.
///
/// # Errors
///
/// Returns an error if the directory cannot be read.
fn declarations_in(directory: &Path, extension: &str) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    let mut declared = Vec::new();
    for entry in fs::read_dir(directory)? {
        let found = entry?.path();
        if found.extension() == Some(OsStr::new(extension)) {
            declared.push(found);
        }
    }
    Ok(declared)
}

/// Takes every declaration written with `extension` out of `directory`, which
/// the root names `named`.
///
/// # Errors
///
/// Returns an error if the directory cannot be read, if a removal fails, or if
/// there was no declaration there to take out.
fn empty(directory: &Path, named: &str, extension: &str) -> Result<(), Box<dyn Error>> {
    let declared = declarations_in(directory, extension)?;
    if declared.is_empty() {
        return Err(format!(
            "this fixture has to take every declaration out of `{named}/` in a copy of the \
             shipped content root, but the shipped root declares none there. What it would build \
             is a root that never declared anything rather than one whose declarations were taken \
             away, and the two are not the same claim"
        )
        .into());
    }
    for declaration in declared {
        fs::remove_file(declaration)?;
    }
    Ok(())
}

/// Why the block registry refuses the content root at `root`, asked of the
/// registry itself.
///
/// **The oracle a scenario about printed text is compared against.** The refusal
/// a mod author reads has to carry this value's own words, and a test that spelled
/// a parser's diagnostic out by hand would be asserting that parser's wording
/// rather than that any of it reached the author.
///
/// # Errors
///
/// Returns an error if the root was accepted — a root that registers is not a
/// root a refusal can be read from, and a scenario comparing printed text against
/// nothing would be asserting nothing.
pub fn block_refusal_over(root: &Path) -> Result<RegistryError, Box<dyn Error>> {
    let mut registry = BlockRegistry::new();
    match registry.apply(&LuauFileDefinitionSource::new(root.to_owned())) {
        Ok(()) => Err(format!(
            "this scenario needs the blocks declared under {} to be refused, and they registered \
             instead. There is no refusal to compare the printed text against",
            root.display()
        )
        .into()),
        Err(refused) => Ok(refused),
    }
}

/// Why the HUD loader refuses the content root at `root`, asked of the loader
/// itself. The HUD half of [`block_refusal_over`], and it is here for the same
/// reason.
///
/// # Errors
///
/// Returns an error if the root's HUD declarations were accepted.
pub fn hud_refusal_over(root: &Path) -> Result<HudLoadError, Box<dyn Error>> {
    match HudLayout::load(&TomlFileHudSource::new(root)) {
        Ok(_) => Err(format!(
            "this scenario needs the HUD declared under {} to be refused, and it loaded instead. \
             There is no refusal to compare the printed text against",
            root.display()
        )
        .into()),
        Err(refused) => Ok(refused),
    }
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
