//! What one `voxforge build` of the shipped manifest leaves behind.
//!
//! **These run against a copy of the shipped content root**, not against a
//! fixture root assembled here, because the shipped manifest is what the
//! command a mod author types is pointed at. A fixture that handed the builder
//! a hand-built manifest would be correct and would assert about a world the
//! product does not inhabit — and it is the only thing that could tell anybody
//! the committed manifest bakes what it claims to.
//!
//! Nothing here writes into the repository: the copy is what a build writes
//! into, and it also grades D8 for free, since an index recording absolute
//! paths would re-fold against nothing inside a temporary directory.

#[path = "common/build.rs"]
mod build;
mod common;

use std::collections::BTreeMap;
use std::error::Error;
use std::path::Path;

use mc_core::art::INDEX_FILE_NAME;
use tempfile::TempDir;
use voxforge::inspect::ExitCode;

use build::{
    Index, MANIFEST_FILE, Root, built, entries_stated, files_in, image_named, keys_stated, manifest,
};
use common::TestResult;
use common::cli::{invoke, nothing_missing, unnamed_in};
use common::texture::GREY;

/// How many entries the committed manifest states.
///
/// The one number in this file that is not read off the fixture, and it is the
/// scenario's own: without it a manifest that came to state nothing would
/// satisfy every assertion below, since each is derived from whatever the
/// manifest happens to hold.
const ENTRIES_THE_SHIPPED_MANIFEST_STATES: usize = 7;

/// The key the grass model's underside is baked to.
const DIRT_KEY: &str = "base:dirt";

/// The model the grass block's six faces come from, as the manifest names it.
const GRASS_MODEL: &str = "models/grass-block.mcvox";

/// The six faces of `model` as the `texture` command emits them.
///
/// The independent oracle: the same pixels reached by a different command, so
/// that "the build wrote the bottom face there" is a comparison against
/// something and not the build agreeing with itself. One pixel per voxel,
/// which is what the manifest states and the only value a sixteen-voxel model
/// may be baked at.
fn faces_of(root: &Root, model: &str) -> Result<BTreeMap<String, Vec<u8>>, Box<dyn Error>> {
    let emitted = TempDir::new()?;
    invoke(&[
        "texture",
        &spelled(&root.path().join(model)),
        "--out",
        &spelled(emitted.path()),
        "--all-faces",
        "--pixels-per-voxel",
        "1",
        "--materials",
        &spelled(&root.path().join("materials")),
    ])?;
    files_in(emitted.path())
}

/// `path` as a command line spells it.
fn spelled(path: &Path) -> String {
    path.display().to_string()
}

#[test]
fn a_manifest_of_seven_entries_writes_seven_images_each_named_for_its_key() -> TestResult {
    let root = Root::shipped()?;
    let keys = keys_stated(&root.manifest())?;
    let mut owed: Vec<String> = keys.iter().map(|key| image_named(key)).collect();
    owed.sort();

    let made = built(&root)?;

    assert_eq!(
        (made.code, keys.len(), made.images()),
        (ExitCode::Success, ENTRIES_THE_SHIPPED_MANIFEST_STATES, owed),
        "one image per entry, each named for the key its entry states. The names on the right are \
         derived from the manifest's own keys by the rule a key's image name follows — a colon \
         becomes two underscores — rather than listed beside it, so a manifest that gains an \
         eighth entry stays graded. What is stated outright is the count, because every other \
         assertion here is derived from the manifest and would pass for one holding nothing. \
         stderr said: {err}",
        err = made.err
    );
    Ok(())
}

#[test]
fn a_manifest_of_seven_entries_writes_one_index_naming_all_seven_keys() -> TestResult {
    let root = Root::shipped()?;
    let mut owed = keys_stated(&root.manifest())?;
    owed.sort();

    let made = built(&root)?;

    assert_eq!(
        (made.code, owed.len(), made.index().sorted()),
        (
            ExitCode::Success,
            ENTRIES_THE_SHIPPED_MANIFEST_STATES,
            Index::Naming(owed)
        ),
        "the index is what a client reads instead of the manifest, so every key the build baked \
         has to be in it. `Naming` and `Absent` are separate answers on purpose: an index nobody \
         wrote and an index naming nothing are the same file listing and must never compare \
         equal. stderr said: {err}",
        err = made.err
    );
    Ok(())
}

#[test]
fn every_written_path_is_reported_on_the_builds_output() -> TestResult {
    let root = Root::shipped()?;
    let keys = keys_stated(&root.manifest())?;
    let mut owed: Vec<String> = keys
        .iter()
        .map(|key| spelled(&root.output().join(image_named(key))))
        .collect();
    owed.push(spelled(&root.output().join(INDEX_FILE_NAME)));
    let expected: Vec<&str> = owed.iter().map(String::as_str).collect();

    let made = built(&root)?;

    assert_eq!(
        (made.code, keys.len(), unnamed_in(&made.out, &expected)),
        (
            ExitCode::Success,
            ENTRIES_THE_SHIPPED_MANIFEST_STATES,
            nothing_missing()
        ),
        "the paths on stdout are the answer: a build that wrote eight files and named six of them \
         leaves whoever ran it unable to say what changed. The index counts — it is a written \
         path too, and the client reads it. stderr said: {err}",
        err = made.err
    );
    Ok(())
}

/// What the committed manifest has to state for that scenario to grade anything.
fn the_entry_the_scenario_needs() -> Option<(String, String, String)> {
    Some((
        DIRT_KEY.to_owned(),
        GRASS_MODEL.to_owned(),
        "bottom".to_owned(),
    ))
}

#[test]
fn the_bottom_face_of_the_grass_model_is_written_as_the_dirt_keys_art() -> TestResult {
    let root = Root::shipped()?;
    let stated = entries_stated(&root.manifest())?;
    let carried = stated.iter().find(|(key, _, _)| key == DIRT_KEY).cloned();

    let made = built(&root)?;
    let faces = faces_of(&root, GRASS_MODEL)?;
    let art = made.written.get(&image_named(DIRT_KEY));

    assert_eq!(
        (
            carried,
            faces.len(),
            art.is_some(),
            art == faces.get("bottom.png"),
            art == faces.get("top.png")
        ),
        (the_entry_the_scenario_needs(), 6, true, true, false),
        "dirt is the underside of the grass block and nothing else — one model paints two blocks, \
         which is the whole reason a manifest names a face rather than a model. The first member \
         is the scenario's premise read off the committed manifest, without which the rest grades \
         a set nobody asked for; the last says the image is the bottom face and not simply some \
         face the build happened to write. stderr said: {err}",
        err = made.err
    );
    Ok(())
}

#[test]
fn a_manifest_with_no_entries_writes_an_index_naming_no_keys_and_reports_no_images() -> TestResult {
    let root = Root::bare()?;
    root.painted(&[GREY])?
        .holding(MANIFEST_FILE, &manifest(1, &[]))?;

    let made = built(&root)?;

    assert_eq!(
        (made.code, made.images(), made.index()),
        (ExitCode::Success, Vec::new(), Index::Naming(Vec::new())),
        "a manifest asking for nothing is answered, not ignored. This is the control the whole \
         phase is shaped against: a build that writes nothing at all also reports zero images, so \
         the assertion that carries it is the index being **there** and naming nothing — \
         `Naming([])` against `Absent`, which a count cannot tell apart. stderr said: {err}",
        err = made.err
    );
    Ok(())
}
