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
use std::fs;
use std::path::{Path, PathBuf};

use tempfile::TempDir;

use super::content_root;

/// The subdirectory of a content root that HUD declarations live in.
pub const HUD_DIRECTORY: &str = "hud";

/// The subdirectory of a content root that block definitions live in.
pub const BLOCK_DIRECTORY: &str = "blocks";

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
