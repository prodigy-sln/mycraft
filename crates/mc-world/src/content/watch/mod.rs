//! Whether a content root has changed since it was last asked, and which paths
//! under it are content at all.
//!
//! **The port speaks in paths that changed, not in a platform's event
//! taxonomy.** The loader reads the whole root on any change, so *which kind* of
//! change happened is information nothing above here has a use for — a port
//! mirroring create/modify/remove would be carrying the vendor's vocabulary
//! across a seam built to stop exactly that.
//!
//! **A path's *spelling* is the one vendor detail that does cross, and the
//! relevance rule is what absorbs it.** A watcher reports the paths the platform
//! gives it — absolute on Windows, and built by concatenating the caller's own
//! spelling of the root onto the working directory, `./` and forward slashes
//! included. So `ContentChanges::Changed` carries the platform's spelling, and
//! `declares_content` is responsible for recognising any spelling of the root it
//! is handed rather than one. A future consumer of `ContentChanges` inherits that
//! obligation; relativising inside the adapter instead is the better shape and is
//! recorded as scope on the issue that moves this watcher.
//!
//! **The relevance rule is built from the loaders' own constants.** A second list
//! of directories and extensions would go on answering for two the day a third
//! declaration kind arrived. The two extension constants share a name in
//! different modules and are disambiguated at the import below.

mod notify_watch;

use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use super::hud_toml_source::{DECLARATION_EXTENSION as HUD_DECLARATION, HUD_DIRECTORY};
use super::luau_source::{BLOCKS_DIRECTORY, DECLARATION_EXTENSION as BLOCK_DECLARATION};

pub use notify_watch::NotifyContentWatch;

/// How long an editor's save is allowed to settle before a change is reported.
///
/// **Declared here and nowhere else.** An editor's save is commonly a
/// write-then-rename or several partial writes, and this is long enough to absorb
/// one of them while leaving the rest of the one-second target to the engine.
pub const SETTLING_WINDOW: Duration = Duration::from_millis(150);

/// What has changed under a content root since this was last asked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContentChanges {
    Nothing,
    /// Paths under the root, in whatever order the platform reported them.
    Changed(Vec<PathBuf>),
    /// The root could not be watched at all.
    Unwatchable {
        directory: PathBuf,
        cause: String,
    },
}

/// A source of changes to a content root.
///
/// `Debug` because the policy that consumes this holds one boxed, and a type
/// holding a trait object that cannot be shown is a type that cannot be shown.
pub trait ContentWatch: fmt::Debug {
    /// What has changed since this was last asked.
    fn changes(&mut self) -> ContentChanges;
}

/// Whether `path` is one the content loaders would read: directly under
/// `blocks/` with the block declaration extension, or directly under `hud/` with
/// the HUD one.
///
/// Anything else under the root — a material, a model, an editor's scratch file,
/// a declaration nested one directory deeper — is not content, because it is not
/// something either loader reads.
///
/// **`root` and `path` need not be spelled the same way, and that is the whole
/// difficulty.** A vendor reports the paths the platform gives it, which are
/// absolute, while a caller may hand over any spelling of the same directory — the
/// shipped client's is relative. A rule that compared the two as written held for
/// one spelling and called every save under any other one "not content", which made
/// the reload inert while every test over an absolute fixture root passed.
#[must_use]
pub fn declares_content(root: &Path, path: &Path) -> bool {
    declared_directly_in(root.join(BLOCKS_DIRECTORY), path, BLOCK_DECLARATION)
        || declared_directly_in(root.join(HUD_DIRECTORY), path, HUD_DECLARATION)
}

/// Whether `path` sits immediately inside `directory` and carries `extension`.
fn declared_directly_in(directory: PathBuf, path: &Path, extension: &str) -> bool {
    path.extension().is_some_and(|found| found == extension)
        && is_the_same_directory(&directory, path.parent())
}

/// Whether `holding` is `declared`, however either is spelled.
///
/// **The written comparison first, and the filesystem only if it fails.** Two paths
/// spelled alike are the same directory without asking anything, which is the case
/// every absolute-root caller is in; the ask is what makes a relative or a
/// `./`-prefixed root work.
///
/// **The file's own parent is canonicalised, never the file.** A removal reports a
/// path that no longer exists, and its directory still does.
///
/// Both sides are canonicalised or neither is, which is what keeps this a rule about
/// paths rather than a preference for one spelling: canonicalising a root at the call
/// site would fix the shipped client and leave the port's contract exactly as narrow
/// as it was.
fn is_the_same_directory(declared: &Path, holding: Option<&Path>) -> bool {
    let Some(holding) = holding else {
        return false;
    };
    if declared == holding {
        return true;
    }
    match (fs::canonicalize(declared), fs::canonicalize(holding)) {
        (Ok(declared), Ok(holding)) => declared == holding,
        _ => false,
    }
}
