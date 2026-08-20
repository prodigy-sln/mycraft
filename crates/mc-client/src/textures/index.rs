//! Where a content root's built set sits, and reading what is under it.
//!
//! # The set's directory is a convention here and a statement there
//!
//! A manifest names its own `output`, and this side never reads the manifest
//! (`architecture.md` D7): two hand-rolled readers of one format agreeing forever
//! is the drift that decision exists to make unspellable. So the client looks for
//! the set under [`SET_DIRECTORY`] and nothing tells it otherwise.
//!
//! **The consequence is a real dead end and it is documented rather than
//! papered over**: a root whose manifest states an `output` other than
//! `textures` builds cleanly into that other directory and is then judged
//! [`Absent`](super::SetVerdict::Absent) forever, telling whoever wrote it to run
//! the build they have just run. `docs/modding/voxel-models.md` says so in the
//! author's own terms. Closing it needs the output directory recorded *in the
//! index*, which is a change to a format two programs share and belongs to the
//! spec that makes it.
//!
//! # Reading is separated from judging
//!
//! What is here answers "what is on disk"; [`super`] answers "what does that
//! make the set". A missing file is not a failure at this level — it is one of
//! the answers — so each of these hands back an absence the caller turns into a
//! verdict, and reserves [`TextureSetError`] for a set that admits no answer at
//! all.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use mc_core::art::{INDEX_FILE_NAME, TextureSetIndex};

use super::TextureSetError;

/// The subdirectory of a content root a build writes its set into.
pub const SET_DIRECTORY: &str = "textures";

/// The file a content root states its art in.
///
/// Its presence is the whole of what separates a root whose set has not been
/// built from a root that declares no art at all — the split `architecture.md`
/// D6 added, because telling a mod author who ships no art to run the art build
/// blames the wrong party.
pub const MANIFEST_FILE_NAME: &str = "textures.toml";

/// Whether the root at `root` states any art to build.
pub fn declares_art(root: &Path) -> bool {
    root.join(MANIFEST_FILE_NAME).is_file()
}

/// The index of the set under `root`, or `None` where no set has been built.
///
/// # Errors
///
/// Returns [`TextureSetError::Unreadable`] where an index is there and cannot be
/// read, and [`TextureSetError::Index`] where it is not an index this client can
/// make sense of. Neither is an absence: a set that is there and unreadable must
/// not be reported as one that was never built.
pub fn under(root: &Path) -> Result<Option<TextureSetIndex>, TextureSetError> {
    let at = root.join(SET_DIRECTORY).join(INDEX_FILE_NAME);
    let text = match fs::read_to_string(&at) {
        Ok(read) => read,
        Err(cause) if cause.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(cause) => return Err(TextureSetError::Unreadable { path: at, cause }),
    };
    TextureSetIndex::parse(&text)
        .map(Some)
        .map_err(|cause| TextureSetError::Index { path: at, cause })
}

/// The bytes of the source `recorded` under `root`, or `None` where it is no
/// longer there.
///
/// # Errors
///
/// Returns [`TextureSetError::Unreadable`] where the source is there and cannot
/// be read. A source that folds as empty because nobody could open it would
/// report a set current against a file nothing can consume.
pub fn source_bytes(root: &Path, recorded: &str) -> Result<Option<Vec<u8>>, TextureSetError> {
    let at = resolved(root, recorded);
    match fs::read(&at) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(cause) if cause.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(cause) => Err(TextureSetError::Unreadable { path: at, cause }),
    }
}

/// Whether the set under `root` holds the image named `image`.
///
/// # Errors
///
/// Returns [`TextureSetError::Unreadable`] where the image cannot be looked at,
/// which is a different answer from its not being there.
pub fn holds_image(root: &Path, image: &str) -> Result<bool, TextureSetError> {
    let at = image_path(root, image);
    match fs::metadata(&at) {
        Ok(found) => Ok(found.is_file()),
        Err(cause) if cause.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(cause) => Err(TextureSetError::Unreadable { path: at, cause }),
    }
}

/// Where the image named `image` sits under `root`.
///
/// The name has already passed the rule the build derived it under — see
/// [`TextureSetError::UnusableImageName`](super::TextureSetError::UnusableImageName)
/// — so this joins and nothing more.
pub fn image_path(root: &Path, image: &str) -> PathBuf {
    root.join(SET_DIRECTORY).join(image)
}

/// The bytes of the image named `image` under `root`.
///
/// # Errors
///
/// Returns [`TextureSetError::Unreadable`] where it cannot be read, its absence
/// included. Absence is a failure here and not an answer, which is the
/// difference from [`holds_image`]: by the time anything reads an image the
/// verdict has already established every one the index names is there, so a
/// disappearance between the two is a set being written while it is read rather
/// than a set that was never built.
pub fn image_bytes(root: &Path, image: &str) -> Result<Vec<u8>, TextureSetError> {
    let at = image_path(root, image);
    fs::read(&at).map_err(|cause| TextureSetError::Unreadable { path: at, cause })
}

/// Where the path an index recorded sits under `root`.
///
/// An index records `/`-separated relative paths whatever platform built it
/// (`architecture.md` D8), so the components are joined one at a time rather
/// than handed to `join` whole: that is what makes a set built on one platform
/// readable on the other, and what lets a root copied anywhere re-fold to the
/// value it was built with.
fn resolved(root: &Path, recorded: &str) -> PathBuf {
    recorded
        .split('/')
        .fold(root.to_owned(), |below, part| below.join(part))
}
