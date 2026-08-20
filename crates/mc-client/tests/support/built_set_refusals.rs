//! The eight refusals a built texture set raises, each as a person running the
//! client from their own game directory reads it.
//!
//! **Eight roots and not one**, for the reason [`crate::printed_refusals`] builds
//! eight: a root is refused whole, so a root carrying two faults is refused for
//! whichever the reader reaches first and the second refusal would be one no run
//! ever prints.
//!
//! Nothing here writes out what a refusal is expected to say. Each is produced by
//! the client's own preparation over a real content root and rendered through the
//! shipped reporting; the wording is the implementer's, and the whole point of the
//! guard these feed is that the page and the program are compared against each
//! other rather than each against somebody's belief about the other.
//!
//! # Why this is a module of its own
//!
//! [`crate::printed_refusals`] is within forty non-blank lines of the size the
//! gate allows a test file, and eight more producers do not fit — the same reason
//! [`crate::per_facing_refusals`] exists, and the same responsibility boundary:
//! everything here is one requirement's refusals.
//!
//! # Six of these name no path at all, and that is the difference from the eight
//!
//! Every refusal [`crate::printed_refusals`] produces names the declaration it is
//! about, so each goes through `as_read_from_a_game_directory`, whose rewrite
//! **refuses** text that does not name the fixture root — a rewrite that quietly
//! did nothing would leave a temporary directory in a string compared against a
//! page.
//!
//! A built set's refusals mostly name no path: they name the command to run, or a
//! source as the *index* records it, which is already relative to the root. Two
//! name a file — the index recording a source outside the root, and the image
//! that would not decode. So six of the eight are produced through
//! [`as_read_anywhere`], which carries the **inverse** premise check —
//! it fails if the text names the fixture root, because that is the day one of
//! these grows a path and starts leaking a directory that exists for a hundred
//! milliseconds into a page comparison. Neither direction is hoped for.

// Each test binary linking this module drives a subset of it.
#![allow(dead_code)]

use std::error::Error;

use crate::printed_refusals::{as_read_from_a_game_directory, normalised};
use crate::support::{self, built_sets, content};

/// Every line the built set can put in front of a person, each produced by a run.
///
/// The order is the order a mod author meets them in: the set that was never
/// built, then the three ways a built one falls behind what it was built from,
/// then the two ways an index says something no reader can act on, then the two
/// ways an image the index promises is not one a layer can be filled from.
///
/// # Errors
///
/// Returns an error if a fixture root cannot be built, if a root that must refuse
/// is accepted, or if a refusal names the fixture's own temporary directory where
/// it was not expected to.
pub fn built_set_refusals() -> Result<Vec<String>, Box<dyn Error>> {
    Ok(vec![
        the_set_was_never_built()?,
        a_model_moved_under_the_set()?,
        a_source_the_set_was_built_from_is_gone()?,
        an_image_the_index_promises_is_gone()?,
        an_index_recording_a_source_outside_the_root()?,
        an_index_naming_an_image_that_is_not_a_name()?,
        an_image_no_layer_can_hold()?,
        an_image_that_never_decoded()?,
    ])
}

/// The contributor who has cloned the repository and not run the art build.
///
/// **The one everybody meets**, and the reason this whole refusal exists: without
/// it a fresh checkout fails somewhere further in, on something that does not
/// mention the build at all.
fn the_set_was_never_built() -> Result<String, Box<dyn Error>> {
    let root = built_sets::a_root_with_a_built_set()?;
    built_sets::without_the_index(root.path())?;
    as_read_anywhere(&root)
}

/// The author edits a model the manifest reaches and launches without rebuilding.
fn a_model_moved_under_the_set() -> Result<String, Box<dyn Error>> {
    let root = built_sets::a_root_with_a_built_set()?;
    built_sets::with_a_model_edited(root.path())?;
    as_read_anywhere(&root)
}

/// The author deletes a material the set was folded over.
///
/// A different sentence from the one above and deliberately so: it names the file
/// that went, because "stale against its sources" would send somebody diffing a
/// directory to find which one.
fn a_source_the_set_was_built_from_is_gone() -> Result<String, Box<dyn Error>> {
    let root = built_sets::a_root_with_a_built_set()?;
    built_sets::without_a_recorded_source(root.path(), built_sets::A_RECORDED_MATERIAL)?;
    as_read_anywhere(&root)
}

/// An image the index names is not beside it — a half-copied set, or a build
/// interrupted.
fn an_image_the_index_promises_is_gone() -> Result<String, Box<dyn Error>> {
    let root = built_sets::a_root_with_a_built_set()?;
    built_sets::without_a_recorded_image(root.path(), built_sets::A_RECORDED_IMAGE)?;
    as_read_anywhere(&root)
}

/// The manifest named a model outside its own directory, and the build wrote it
/// down.
///
/// **The one of the six that names a path**, so it is the one the fixture-root
/// rewrite has anything to do.
fn an_index_recording_a_source_outside_the_root() -> Result<String, Box<dyn Error>> {
    let root = built_sets::a_root_with_a_built_set()?;
    built_sets::recording_a_source_outside_the_root(root.path())?;
    as_read_from_a_game_directory(&root)
}

/// An index whose image record is a path rather than a name.
///
/// Reachable only from a hand-edited index or an older tool, and it belongs on the
/// page anyway: it is the one refusal that says out loud that the client checks a
/// name it was handed rather than deriving one of its own. Describing it in prose
/// while quoting the other five is what teaches a reader the fences are decorative.
fn an_index_naming_an_image_that_is_not_a_name() -> Result<String, Box<dyn Error>> {
    let root = built_sets::a_root_with_a_built_set()?;
    built_sets::naming_one_image(root.path(), built_sets::AN_IMAGE_NAME_THAT_IS_A_PATH)?;
    as_read_anywhere(&root)
}

/// An image twice the edge a layer holds, put where the index promises one.
///
/// **Defence in depth over a directory somebody can write into.** The build
/// refuses a model whose scale and pixels-per-voxel do not come to a block
/// texture's edge, naming the *model* — the thing its author can fix. The set on
/// disk is checked again anyway, because it is derived, git-ignored and therefore
/// not reviewed, and uploading a 32 x 32 image into a 16 x 16 layer is a buffer
/// overrun rather than a picture that looks odd.
///
/// The image is a committed fixture rather than one written here: producing a PNG
/// by hand needs a deflate stream and four checksums, and a second encoder is a
/// second thing to keep correct.
fn an_image_no_layer_can_hold() -> Result<String, Box<dyn Error>> {
    let root = built_sets::a_root_with_a_built_set()?;
    let oversized = std::fs::read(a_committed_image(AN_IMAGE_TWICE_THE_EDGE)?)?;
    built_sets::with_one_image_replaced(root.path(), built_sets::A_RECORDED_IMAGE, &oversized)?;
    as_read_anywhere(&root)
}

/// A file the decoder cannot read at all, where an image should be.
///
/// **This one names the file as well as the key**, which is why it goes through
/// the game-directory rewrite: a file that never decoded is a file somebody has
/// to go and look at, and the key alone would leave them with a name they never
/// typed.
fn an_image_that_never_decoded() -> Result<String, Box<dyn Error>> {
    let root = built_sets::a_root_with_a_built_set()?;
    built_sets::with_one_image_replaced(root.path(), built_sets::A_RECORDED_IMAGE, NOT_A_PNG)?;
    as_read_from_a_game_directory(&root)
}

/// Bytes that are not a PNG and are not close to being one.
///
/// Text rather than a truncated PNG: a truncated one is a decoder question — how
/// much of a file is enough — and this refusal is about a file that was never an
/// image, which is what a set assembled by hand actually contains.
const NOT_A_PNG: &[u8] = b"this file is not a PNG, and the client has to say so by name\n";

/// A 32 x 32 image committed beside this suite.
const AN_IMAGE_TWICE_THE_EDGE: [&str; 4] = ["tests", "fixtures", "set", "thirty-two-square.png"];

/// A file committed beside this suite, located from the crate rather than from
/// wherever the test binary was started.
///
/// # Errors
///
/// Returns an error when the fixture is not there, which is a fixture that moved
/// rather than a client that did anything.
fn a_committed_image(parts: [&str; 4]) -> Result<std::path::PathBuf, Box<dyn Error>> {
    let at = parts.iter().fold(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")),
        |at, part| at.join(part),
    );
    if !at.is_file() {
        return Err(format!(
            "this producer needs the committed image at {}, and it is not there",
            at.display()
        )
        .into());
    }
    Ok(at)
}

/// What the client writes for the content root at `root`, for a refusal whose
/// words name no path.
///
/// The inverse of [`as_read_from_a_game_directory`]'s premise, and it is a check
/// rather than an assumption for the same reason that one is: a refusal that grew
/// a path would put a temporary directory into a string a page is compared
/// against, and the page could then never match. Failing here says which refusal
/// grew it.
///
/// # Errors
///
/// Returns an error if the root was accepted, or if what was written names the
/// fixture's own directory.
fn as_read_anywhere(root: &content::ContentRoot) -> Result<String, Box<dyn Error>> {
    let printed = support::refusal_printed_over(root.path())?;
    let named = root.path().display().to_string();
    if printed.contains(&named) {
        return Err(format!(
            "this refusal is produced without the fixture-root rewrite because its words name no \
             path, and it now names `{named}`. Compared against a page it would be text no run \
             could ever reproduce, since that directory exists for a hundred milliseconds. What \
             was written was:\n{printed}"
        )
        .into());
    }
    Ok(normalised(&printed))
}
