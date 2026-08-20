//! Content roots whose built texture set has been put into one particular
//! state, each built from the shipped root and each a real directory of real
//! files.
//!
//! # Every root here is a copy, and the copy is what makes the fold portable
//!
//! An index records its sources relative to the manifest's own directory, which
//! is the content root itself. So a root copied into a temporary directory
//! re-folds to the value it was built with, and a fixture may edit whatever it
//! likes without touching `content/base/` — which is the product's own content
//! and would be left in whatever state a failed run ended in.
//!
//! # A fixture whose premise did not hold is a failure, never a no-op
//!
//! Every helper below refuses to hand back a root when the thing it was asked
//! to remove was not there to remove, or when the thing it was asked to add was
//! there already. A fixture that quietly did nothing would produce a verdict
//! about a root nobody described, and the test reading it would report the
//! fixture rather than the client.
//!
//! # The built set has to exist before any of this means anything
//!
//! The set is derived and is not committed, so a checkout that has not run the
//! build has no `textures/` to copy. [`a_root_with_a_built_set`] says so in one
//! sentence naming the command, because the alternative is eight verdict
//! mismatches that all read as a broken client.

use std::error::Error;
use std::fs;
use std::path::Path;

use mc_core::art::{INDEX_FILE_NAME, IndexEntry, TextureSetIndex};
use mc_core::id::TextureKey;

use super::content::{self, ContentRoot};

/// The subdirectory of a content root the built set is written into.
///
/// A convention on the client's side rather than something it reads: the
/// manifest states its own `output`, and the client never reads the manifest.
pub const SET_DIRECTORY: &str = "textures";

/// The file a content root states its art in.
///
/// Its presence is the whole of what separates a root whose set has not been
/// built from a root that declares no art at all.
pub const MANIFEST_FILE_NAME: &str = "textures.toml";

/// The subdirectory holding the models a manifest reaches.
pub const MODEL_DIRECTORY: &str = "models";

/// The subdirectory holding the materials every build folds.
pub const MATERIAL_DIRECTORY: &str = "materials";

/// A model the shipped manifest names, and therefore a source the index records.
pub const A_RECORDED_MODEL: &str = "models/grass-block.mcvox";

/// A material the shipped build folds, recorded like every other source.
pub const A_RECORDED_MATERIAL: &str = "materials/dirt.toml";

/// The image the shipped set holds one particular key's art in.
pub const A_RECORDED_IMAGE: &str = "base__stone.png";

/// The key that image belongs to.
pub const THE_KEY_THAT_IMAGE_BELONGS_TO: &str = "base:stone";

/// The record an index writes each of its keys as.
const KEY_RECORD: &str = "key ";

/// A source record a manifest naming a model outside its own directory produces.
///
/// `../shared/` is the shape the observation names: one model tree beside several
/// content roots is an ordinary thing to want, and it is refused rather than
/// supported.
pub const A_SOURCE_OUTSIDE_THE_ROOT: &str = "../shared/grass-block.mcvox";

/// An image name that is a path rather than a name.
///
/// It passes the index's own path rule — relative, `/`-separated, no parent — and
/// fails the rule for an image name, which is what makes it the one that says a
/// reader applies the second rule at all.
pub const AN_IMAGE_NAME_THAT_IS_A_PATH: &str = "elsewhere/base__stone.png";

/// A fold value nothing reaches: an index recording an unsafe source is refused
/// before a source is ever opened.
const A_FOLD_NOTHING_REACHES: u64 = 0x0123_4567_89ab_cdef;

/// A comment appended to a source file: bytes that change, meaning that does
/// not.
///
/// A model edited into something unreadable would be a second claim — that the
/// client refuses a broken model — and this phase makes no such claim. What has
/// to change is the fold, and the fold is over bytes.
const AN_EDIT_THAT_CHANGES_ONLY_BYTES: &str = "\n# edited after the set was built\n";

/// A manifest entry naming a model the manifest already reaches.
///
/// The new entry names no new file, so what changes is the manifest's own bytes
/// and nothing else. A root gaining a *file* as well would be stale for two
/// reasons and would say nothing about either.
const A_GAINED_MANIFEST_ENTRY: &str = "\n[[texture]]\nkey   = \"base:grass_top_again\"\nmodel = \"models/grass-block.mcvox\"\nface  = \"top\"\n";

/// A material file no index records, and the name it is written under.
///
/// Sorted after every material the shipped root ships, so a build that folded
/// the directory afresh would fold it last rather than in the middle — which is
/// what makes a re-derived source list differ from the recorded one in a way a
/// reader of the failure can see.
pub const AN_UNRECORDED_MATERIAL: &str = "zzz_added_after_the_build.toml";

/// What that material declares. A real material, so a later build would accept
/// it; nothing in this phase reads it.
const AN_UNRECORDED_MATERIALS_DECLARATION: &str =
    "name  = \"base:added_after_the_build\"\ncolor = \"#123456\"\n";

/// The shipped content root, copied whole, with its built set beside it.
///
/// # Errors
///
/// Returns an error if the copy fails, or if the shipped root carries no built
/// set — which is what a checkout that has not run the art build looks like.
pub fn a_root_with_a_built_set() -> Result<ContentRoot, Box<dyn Error>> {
    let copied = content::shipped_copy()?;
    let index = copied.path().join(SET_DIRECTORY).join(INDEX_FILE_NAME);
    if !index.is_file() {
        return Err(
            "this fixture needs a built texture set to copy, and the shipped content root \
                    has none. The set is derived and is never committed, so run `cargo run -p \
                    voxforge -- build content/base/textures.toml` and try again"
                .into(),
        );
    }
    Ok(copied)
}

/// That root with its index taken away, the manifest and the images left where
/// they were.
///
/// # Errors
///
/// Returns an error if the index was not there to remove.
pub fn without_the_index(root: &Path) -> Result<(), Box<dyn Error>> {
    let index = root.join(SET_DIRECTORY).join(INDEX_FILE_NAME);
    removing(&index)
}

/// That root with the whole set and the manifest taken away — a root that
/// declares no art at all, which is what a mod author's first root is.
///
/// # Errors
///
/// Returns an error if either was not there to remove.
pub fn without_any_art(root: &Path) -> Result<(), Box<dyn Error>> {
    let manifest = root.join(MANIFEST_FILE_NAME);
    if !manifest.is_file() {
        return Err(a_premise_that_did_not_hold(
            &manifest,
            "was not there to remove",
        ));
    }
    fs::remove_file(&manifest)?;
    let set = root.join(SET_DIRECTORY);
    if !set.is_dir() {
        return Err(a_premise_that_did_not_hold(&set, "was not there to remove"));
    }
    fs::remove_dir_all(&set)?;
    Ok(())
}

/// That root with one of the models the manifest reaches edited since the build.
///
/// # Errors
///
/// Returns an error if the model was not there to edit.
pub fn with_a_model_edited(root: &Path) -> Result<(), Box<dyn Error>> {
    appending(
        &root.join(A_RECORDED_MODEL),
        AN_EDIT_THAT_CHANGES_ONLY_BYTES,
    )
}

/// That root with one more entry in its manifest than the set was built from.
///
/// # Errors
///
/// Returns an error if the manifest was not there to extend.
pub fn with_a_gained_manifest_entry(root: &Path) -> Result<(), Box<dyn Error>> {
    appending(&root.join(MANIFEST_FILE_NAME), A_GAINED_MANIFEST_ENTRY)
}

/// That root with one source the index records no longer present.
///
/// **This fixture carries more than the scenario that asks for it.** Measured: a
/// client that re-derived its source list by scanning the manifest's directories
/// instead of re-folding the list the index recorded reports the root
/// `StaleAgainstSources` here, not `SourceMissing` — because a directory scan
/// cannot see a file that is *gone*, so the removed source simply drops out of
/// the derived list. So this is also the evidence that **recording the list is
/// what makes "a source went missing" observable as that**, rather than as
/// generic staleness with nothing naming which file went.
///
/// Keep the removal a *recorded* source and keep the assertion on the arm.
/// Widening it to `StaleAgainstSources`, or removing a file the index never
/// recorded, gives that property up without anything going red.
///
/// # Errors
///
/// Returns an error if the source was not there to remove.
pub fn without_a_recorded_source(root: &Path, source: &str) -> Result<(), Box<dyn Error>> {
    removing(&root.join(source))
}

/// That root with one image the index names no longer present.
///
/// # Errors
///
/// Returns an error if the image was not there to remove.
pub fn without_a_recorded_image(root: &Path, image: &str) -> Result<(), Box<dyn Error>> {
    removing(&root.join(SET_DIRECTORY).join(image))
}

/// That root with a material file the index does not record.
///
/// **What separates re-folding the recorded list from re-deriving one.** The
/// manifest names its materials directory and a build folds every `*.toml` in
/// it, so a client that went back to the manifest would fold this file and call
/// the set stale. A client re-folding what the index recorded does not see it.
///
/// # Errors
///
/// Returns an error if a file of that name is there already.
pub fn with_a_material_the_index_never_recorded(root: &Path) -> Result<(), Box<dyn Error>> {
    let added = root.join(MATERIAL_DIRECTORY).join(AN_UNRECORDED_MATERIAL);
    if added.exists() {
        return Err(a_premise_that_did_not_hold(&added, "is there already"));
    }
    fs::write(&added, AN_UNRECORDED_MATERIALS_DECLARATION)?;
    Ok(())
}

/// That root with every key record struck out of its index, its fold and its
/// sources untouched.
///
/// An index naming zero keys and current against the sources it recorded: the
/// vacuity control for the verdict that lets a launch through.
///
/// # Errors
///
/// Returns an error if the index was not there to rewrite, or if it recorded no
/// keys to strike out — an index that never named one is not an index whose keys
/// were removed.
pub fn with_an_index_naming_no_keys(root: &Path) -> Result<(), Box<dyn Error>> {
    let index = root.join(SET_DIRECTORY).join(INDEX_FILE_NAME);
    let recorded = fs::read_to_string(&index)?;
    if !recorded.contains(KEY_RECORD) {
        return Err(a_premise_that_did_not_hold(
            &index,
            "records no key to strike out",
        ));
    }
    let without_keys: String = recorded
        .lines()
        .filter(|line| !line.starts_with(KEY_RECORD))
        .map(|line| format!("{line}\n"))
        .collect();
    fs::write(&index, without_keys)?;
    Ok(())
}

/// That root with an index of its own, written whole.
///
/// For the two readings that need an index no build would produce: the client
/// takes what it is given, and what it is given is a file on disk.
///
/// # Errors
///
/// Returns an error if the set directory is not there to write into.
pub fn stating_the_index(root: &Path, text: &str) -> Result<(), Box<dyn Error>> {
    let set = root.join(SET_DIRECTORY);
    if !set.is_dir() {
        return Err(a_premise_that_did_not_hold(
            &set,
            "is not there to write into",
        ));
    }
    fs::write(set.join(INDEX_FILE_NAME), text)?;
    Ok(())
}

/// That root with an index recording one source that leaves the content root.
///
/// The shape a manifest naming `model = "../shared/x.mcvox"` builds cleanly into:
/// the writer states it without complaint and the reader refuses it. Rendered
/// through `stating` and `rendered` rather than typed out, so it is what a build
/// emits rather than this module's belief about what a build emits.
///
/// # Errors
///
/// Returns an error if the key this repository ships cannot be parsed as one, if
/// the index cannot be stated, or if the set directory is not there.
pub fn recording_a_source_outside_the_root(root: &Path) -> Result<(), Box<dyn Error>> {
    let written = TextureSetIndex::stating(
        A_FOLD_NOTHING_REACHES,
        vec![A_SOURCE_OUTSIDE_THE_ROOT.to_owned()],
        vec![IndexEntry {
            key: TextureKey::parse(THE_KEY_THAT_IMAGE_BELONGS_TO)?,
            image: A_RECORDED_IMAGE.to_owned(),
        }],
    )?
    .rendered();
    stating_the_index(root, &written)
}

/// That root with the image one key is held in renamed to `image`, keeping the
/// fold and the sources the build recorded.
///
/// **The fold is kept deliberately.** A set that were also stale would be refused
/// for that instead, and the reading this serves would never reach the name.
///
/// # Errors
///
/// Returns an error if the index cannot be read, parsed or re-stated, or if it
/// names no key whose image could be renamed.
pub fn naming_one_image(root: &Path, image: &str) -> Result<(), Box<dyn Error>> {
    let at = root.join(SET_DIRECTORY).join(INDEX_FILE_NAME);
    let recorded = TextureSetIndex::parse(&fs::read_to_string(&at)?)?;
    let mut entries: Vec<IndexEntry> = recorded.entries().to_vec();
    let first = entries
        .first_mut()
        .ok_or_else(|| a_premise_that_did_not_hold(&at, "names no key to rename the image of"))?;
    first.image = image.to_owned();
    let rewritten =
        TextureSetIndex::stating(recorded.fold(), recorded.sources().to_vec(), entries)?.rendered();
    stating_the_index(root, &rewritten)
}

/// That root with one more key in its index than any block declares, its art a
/// copy of an image the set already holds.
///
/// **The fold is left exactly as the build wrote it**, which is what makes this
/// a current set naming a key nothing draws rather than a stale one: an index is
/// folded over the manifest, the models and the materials, and none of those is
/// what this touches.
///
/// # Errors
///
/// Returns an error if the index cannot be read, parsed or re-stated, if the
/// image it copies is not there, or if the key is one the index already names.
pub fn also_naming(root: &Path, key: &str, image: &str) -> Result<(), Box<dyn Error>> {
    let set = root.join(SET_DIRECTORY);
    let at = set.join(INDEX_FILE_NAME);
    let recorded = TextureSetIndex::parse(&fs::read_to_string(&at)?)?;
    let named = TextureKey::parse(key)?;
    if recorded.entries().iter().any(|entry| entry.key == named) {
        return Err(a_premise_that_did_not_hold(
            &at,
            "names that key already, so nothing would be added",
        ));
    }
    let copied = set.join(image);
    if copied.exists() {
        return Err(a_premise_that_did_not_hold(&copied, "is there already"));
    }
    fs::copy(set.join(A_RECORDED_IMAGE), &copied)?;
    let mut entries: Vec<IndexEntry> = recorded.entries().to_vec();
    entries.push(IndexEntry {
        key: named,
        image: image.to_owned(),
    });
    let rewritten =
        TextureSetIndex::stating(recorded.fold(), recorded.sources().to_vec(), entries)?.rendered();
    stating_the_index(root, &rewritten)
}

/// That root with the bytes of one image the index names replaced by `bytes`.
///
/// **The index is untouched and so is the fold**, so the set is still current and
/// the image is still where the index says it is: what is wrong with it is what
/// is inside it. That is the only way to reach a reading about an image the
/// array texture cannot hold, because every other fixture here would be refused
/// one step earlier as stale or as missing.
///
/// # Errors
///
/// Returns an error if the image was not there to overwrite.
pub fn with_one_image_replaced(
    root: &Path,
    image: &str,
    bytes: &[u8],
) -> Result<(), Box<dyn Error>> {
    let at = root.join(SET_DIRECTORY).join(image);
    if !at.is_file() {
        return Err(a_premise_that_did_not_hold(
            &at,
            "was not there to overwrite",
        ));
    }
    fs::write(&at, bytes)?;
    Ok(())
}

/// Removes `path`, refusing where it was not there to remove.
fn removing(path: &Path) -> Result<(), Box<dyn Error>> {
    if !path.is_file() {
        return Err(a_premise_that_did_not_hold(path, "was not there to remove"));
    }
    fs::remove_file(path)?;
    Ok(())
}

/// Appends `text` to `path`, refusing where it was not there to append to.
fn appending(path: &Path, text: &str) -> Result<(), Box<dyn Error>> {
    if !path.is_file() {
        return Err(a_premise_that_did_not_hold(path, "was not there to edit"));
    }
    let held = fs::read_to_string(path)?;
    fs::write(path, format!("{held}{text}"))?;
    Ok(())
}

/// The failure a fixture whose premise did not hold hands back.
fn a_premise_that_did_not_hold(path: &Path, what: &str) -> Box<dyn Error> {
    format!(
        "this fixture describes a content root by changing `{}`, and that path {what}. What it \
         would build is a root nobody described, and the verdict read from it would be about the \
         fixture rather than about the client",
        path.display()
    )
    .into()
}
