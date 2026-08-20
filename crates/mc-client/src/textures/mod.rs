//! What the built texture set under a content root is, and what a launch makes
//! of it.
//!
//! # One verdict per root, and it is total
//!
//! `assert!(nothing_was_refused)` cannot tell a healthy set from a client that
//! has lost the ability to check: both say nothing. [`SetVerdict`](crate::textures::SetVerdict) is a total
//! enumeration, so every arm is an answer somebody can be shown and a check that
//! stops checking cannot report the arm that lets a launch through.
//!
//! **It is returned in `Ok`, never raised.** Three of the six arms let a launch
//! continue — a root declaring no art, a set that is current, and a set that is
//! current while covering nothing — so a verdict that only existed as an error
//! would leave those three unconstructible and the totality would be a claim
//! nothing held.
//!
//! [`TextureSetError`](crate::textures::TextureSetError) is the other axis: not what the set *is*, but a set that
//! admits no answer. An index that will not parse and an index naming an image
//! by something that is not an image name are both of that kind.
//!
//! # The sources are re-folded as the index recorded them, and the manifest is
//! never read
//!
//! The build writes the list of sources it folded, in fold order, each relative
//! to the manifest's own directory. This side reads that list back and folds the
//! same bytes in the same order (`architecture.md` D7, D8). It does **not** work
//! the list out again from the manifest: two independent derivations of one list
//! agree on the shipped tree and part company the first time a build changes what
//! it reaches, and the client would then call a set stale that is not — or worse,
//! current when it is not.
//!
//! Resolving the recorded paths against the root the client was given is also
//! what makes a content root copied anywhere still current, which every fixture
//! in this workspace depends on.
//!
//! # The refusal lives here, beside the enum it is total over
//!
//! [`refusal_for`](crate::textures::refusal_for) maps a verdict to what a player reads. It sits next to
//! [`SetVerdict`](crate::textures::SetVerdict) rather than in [`crate::startup`] so that adding an arm and
//! forgetting to say what it means is a non-exhaustive match in this file rather
//! than a silent `None` in another one.

/// Turning the set's images into the texels a layer is filled from. The only
/// file in this crate that names an image decoder.
mod decode;
/// Where the set sits under a content root, and reading what is there.
mod index;

use std::io;
use std::path::{Path, PathBuf};

use mc_core::art::{IndexError, TextureSetIndex, folded_sources, is_an_ordinary_image_name};
use mc_core::content::TEXTURE_EDGE;
use mc_core::id::TextureKey;
use mc_render::texture::supplied::SuppliedTexels;
use thiserror::Error;

use crate::startup::PreparationError;

/// What the built texture set under a content root is.
///
/// Total: a check that stops looking cannot report [`Current`](Self::Current).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SetVerdict {
    /// The root states no texture manifest, so there is nothing to build and
    /// nothing to be stale against.
    NoArtDeclared,
    /// A manifest is there and no set has been built from it.
    Absent,
    /// The sources the index recorded no longer fold to the value it recorded.
    StaleAgainstSources,
    /// A source the index recorded is no longer there.
    ///
    /// Its own arm rather than part of staleness, because it names a different
    /// thing to whoever reads the message: a source that moved, rather than one
    /// that changed.
    SourceMissing {
        /// The source, spelled as the index records it — searchable in that
        /// file, and the same text on either platform.
        source: PathBuf,
    },
    /// The index names an image the set does not hold.
    ImageMissing {
        /// The key that image is the art for.
        key: TextureKey,
        /// The image, spelled as the index records it.
        image: PathBuf,
    },
    /// The set is there and folds to what its sources say.
    Current,
}

/// Why the set could not be read at all — a different axis from what it *is*.
#[derive(Debug, Error)]
pub enum TextureSetError {
    /// A file the set is made of that is there and will not open.
    ///
    /// **What the filesystem said is a layer beneath this and never inside it.**
    /// A message interpolating its own cause reports one flattened sentence and
    /// drops whatever sits under that, which is the defect
    /// `tests/reporting_seam.rs` exists over — and it caught this variant's first
    /// spelling.
    #[error("`{path}` could not be read", path = path.display())]
    Unreadable {
        /// What could not be read.
        path: PathBuf,
        /// What the filesystem said about it.
        #[source]
        cause: io::Error,
    },
    #[error("`{path}` is not a texture set index this client can read", path = path.display())]
    Index {
        /// The index at fault.
        path: PathBuf,
        /// What is wrong with it, which is where the offending record is named.
        #[source]
        cause: IndexError,
    },
    /// An image name a set may not be written under.
    ///
    /// **The client takes the name from the index and joins it onto a path**, so
    /// it applies the rule the build derived that name under rather than
    /// deriving it a second time (`architecture.md` D9). A relative,
    /// `/`-separated path with no parent component passes the index's own rule
    /// and is still not an image name; without this it would be joined onto the
    /// set's directory and read.
    #[error(
        "the texture set index names the art for `{key}` as `{image}`, which is not a name a set's \
         image may be written under",
        key = key.as_str()
    )]
    UnusableImageName {
        /// The key whose record is at fault.
        key: TextureKey,
        /// The name, exactly as the index spells it.
        image: String,
    },
    /// An image the array texture has no layer the shape of.
    ///
    /// **Both edges are named as well as the key**, because whoever meets this
    /// has one image to redraw and needs three things to do it: which key it is
    /// the art for, what size it is, and what size a layer holds. The declared
    /// edge is read from the same constant the array texture is allocated at, so
    /// a message and an allocation cannot come to disagree.
    #[error(
        "the art for `{key}` is {wide}x{high} and a layer of the array texture holds \
         {TEXTURE_EDGE}x{TEXTURE_EDGE}",
        key = key.as_str(),
        wide = found.0,
        high = found.1
    )]
    Size {
        /// The key that image is the art for.
        key: TextureKey,
        /// What the image measures, in texels: width then height.
        found: (u32, u32),
    },
    /// A file the index names as art that no PNG decoder can read.
    ///
    /// The format is stated to the decoder rather than guessed, so this says
    /// "not a PNG" rather than "some other image": a set holds PNGs because that
    /// is what the build writes, and a file that is not one is a file somebody
    /// has to go and look at. It is named for that reason, and the key is named
    /// beside it because the file name is one nobody typed.
    #[error(
        "the art for `{key}` at `{image}` is not a PNG this client can read",
        key = key.as_str(),
        image = image.display()
    )]
    NotAPng {
        /// The key that file is the art for.
        key: TextureKey,
        /// Where the file is.
        image: PathBuf,
    },
}

/// What the built set under `root` is, and the texels it offers.
///
/// **The verdict is returned rather than raised**; [`refusal_for`] is what turns
/// one into the sentence a player reads.
///
/// **Only a current set is decoded**, and the order is the whole of why: every
/// other verdict either refuses the launch or says there is no art, so decoding
/// first would spend the work and — worse — would report a broken image out of a
/// set the client was going to refuse for a reason its author can act on.
///
/// # Errors
///
/// Returns [`TextureSetError`] where the set admits no verdict at all — an index
/// that cannot be read or parsed, one naming an image by something that is not
/// an image name, or an image no layer can be filled from.
pub fn built_set(root: &Path) -> Result<(SetVerdict, SuppliedTexels), TextureSetError> {
    let (verdict, recorded) = judged(root)?;
    let Some(recorded) = recorded.filter(|_| verdict == SetVerdict::Current) else {
        return Ok((verdict, SuppliedTexels::none()));
    };
    let texels = decode::texels_of(root, &recorded)?;
    Ok((verdict, texels))
}

/// The refusal `verdict` becomes, or `None` where the launch goes on.
///
/// Two arms let a launch through and they are not the same thing: a root that
/// declares no art has nothing to build, and a current set is one that was
/// built. Both draw every face from a generated texture; only one of them would
/// be told to run a build if the split were dropped.
#[must_use]
pub fn refusal_for(verdict: &SetVerdict) -> Option<PreparationError> {
    match verdict {
        SetVerdict::NoArtDeclared | SetVerdict::Current => None,
        SetVerdict::Absent => Some(PreparationError::TextureSetAbsent),
        SetVerdict::StaleAgainstSources => Some(PreparationError::TextureSetStale),
        SetVerdict::SourceMissing { source } => Some(PreparationError::TextureSetSourceMissing {
            missing: source.clone(),
        }),
        SetVerdict::ImageMissing { key, image } => Some(PreparationError::TextureSetImageMissing {
            key: key.clone(),
            image: image.clone(),
        }),
    }
}

/// What the set under `root` is.
///
/// The order of the questions is the order of the answers: a root that declares
/// no art is never stale, a set that was never built has no sources to check,
/// and a set whose sources have moved is not judged on images it may not have
/// baked yet.
fn judged(root: &Path) -> Result<(SetVerdict, Option<TextureSetIndex>), TextureSetError> {
    if !index::declares_art(root) {
        return Ok((SetVerdict::NoArtDeclared, None));
    }
    let Some(recorded) = index::under(root)? else {
        return Ok((SetVerdict::Absent, None));
    };
    if let Some(verdict) = against_its_sources(root, &recorded)? {
        return Ok((verdict, Some(recorded)));
    }
    if let Some(verdict) = over_its_images(root, &recorded)? {
        return Ok((verdict, Some(recorded)));
    }
    Ok((SetVerdict::Current, Some(recorded)))
}

/// What the sources `recorded` names say about the set under `root`, or `None`
/// where they fold to the value it recorded.
fn against_its_sources(
    root: &Path,
    recorded: &TextureSetIndex,
) -> Result<Option<SetVerdict>, TextureSetError> {
    let mut read: Vec<(&str, Vec<u8>)> = Vec::with_capacity(recorded.sources().len());
    for source in recorded.sources() {
        let Some(bytes) = index::source_bytes(root, source)? else {
            return Ok(Some(SetVerdict::SourceMissing {
                source: PathBuf::from(source),
            }));
        };
        read.push((source.as_str(), bytes));
    }
    let folding: Vec<(&str, &[u8])> = read
        .iter()
        .map(|(source, bytes)| (*source, bytes.as_slice()))
        .collect();
    Ok((folded_sources(&folding) != recorded.fold()).then_some(SetVerdict::StaleAgainstSources))
}

/// What the art `recorded` names says about the set under `root`, or `None`
/// where every key's image is there.
///
/// # Errors
///
/// Returns [`TextureSetError::UnusableImageName`] before looking for an image
/// whose name is not one a set may hold: a name that fails the rule is not a
/// missing file, and looking for it would mean joining it onto a path first.
fn over_its_images(
    root: &Path,
    recorded: &TextureSetIndex,
) -> Result<Option<SetVerdict>, TextureSetError> {
    for entry in recorded.entries() {
        if !is_an_ordinary_image_name(&entry.image) {
            return Err(TextureSetError::UnusableImageName {
                key: entry.key.clone(),
                image: entry.image.clone(),
            });
        }
        if !index::holds_image(root, &entry.image)? {
            return Ok(Some(SetVerdict::ImageMissing {
                key: entry.key.clone(),
                image: PathBuf::from(&entry.image),
            }));
        }
    }
    Ok(None)
}
