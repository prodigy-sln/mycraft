//! The index records one value folded over exactly the sources the manifest
//! reached — no fewer and no more.
//!
//! **The negative is the one that constrains this.** Every positive staleness
//! reading here is satisfied by folding the whole content tree, and a build that
//! did that would turn each unrelated content edit into a launch that refuses
//! with a message about textures. So a model under `models/` that no entry names
//! is edited and the value must not move, and a file in the materials directory
//! that is not a `.toml` is added and the value must not move either. The
//! second is the same rule met from the other side: the build reads every
//! `*.toml` in that directory and nothing else, so folding a stray note would
//! make the set stale for an input nothing consumed.
//!
//! Two of these edit a file without changing a pixel — a comment line at the top
//! of a model. That is deliberate: the fold is over the **bytes a build read**,
//! not over the picture it produced, and an implementation folding rendered
//! images would pass every staleness scenario and still let an edited comment
//! go unnoticed by a client deciding whether to rebuild.

#[path = "common/build.rs"]
mod build;
mod common;

use std::error::Error;
use std::fs;

use common::TestResult;
use common::texture::GREY;
use mc_core::content::TEXTURE_EDGE;
use voxforge::inspect::ExitCode;

use build::{
    CUBE_MODEL, MANIFEST_FILE, Recorded, Refused, Root, block_of, built, recorded,
    root_of_one_cube, two_faces_of_the_cube,
};

/// A model in the root that no manifest entry names.
const SPARE_MODEL: &str = "models/spare.mcvox";

/// The material file the cube is painted from.
const CUBE_MATERIAL: &str = "materials/tone_grey.toml";

/// A file in the materials directory that is not a material.
const A_NOTE_IN_THE_MATERIALS_DIRECTORY: &str = "materials/notes.md";

/// A name in the materials directory that ends `.toml` and cannot be read as
/// a file, because it is a directory.
///
/// A directory refuses a read on every platform this builds for, which a
/// permission bit does not.
const AN_UNREADABLE_MATERIAL: &str = "materials/locked.toml";

/// A root holding the fixture cube, a spare model nothing names, and a manifest.
fn folding_root() -> Result<Root, Box<dyn Error>> {
    let root = root_of_one_cube(GREY)?;
    let edge = TEXTURE_EDGE;
    root.holding(SPARE_MODEL, &block_of((edge, edge, edge), edge, GREY))?
        .holding(MANIFEST_FILE, &two_faces_of_the_cube())?;
    Ok(root)
}

/// The cube's material, declaring `colour` instead of the tone it was painted.
fn recoloured(colour: &str) -> String {
    format!(
        "name = \"{key}\"\ncolor = \"{colour}\"\nemissive = 0.0\n",
        key = GREY.key
    )
}

/// `text` with a comment line above it — an edit that moves a file's bytes and
/// not one pixel of what it renders to.
fn commented(text: &str) -> String {
    format!("# edited\n{text}")
}

/// The text of the file at `relative` inside `root`.
fn text_of(root: &Root, relative: &str) -> Result<String, Box<dyn Error>> {
    Ok(fs::read_to_string(root.path().join(relative))?)
}

#[test]
fn editing_a_material_the_manifest_reached_records_a_different_value() -> TestResult {
    let root = folding_root()?;
    let before = built(&root)?.recorded_fold();

    root.holding(CUBE_MATERIAL, &recoloured("#123456"))?;
    let after = built(&root)?;

    assert_eq!(
        (after.code, recorded(&before, &after.recorded_fold())),
        (ExitCode::Success, Recorded::Moved),
        "a material is what a model is painted from, so a client holding a set built before this \
         edit is holding the old colour. `Unavailable` is a third answer on purpose: a build that \
         stopped writing an index would otherwise read as *the value did not move*. It said: \
         {err}",
        err = after.err
    );
    Ok(())
}

#[test]
fn editing_a_model_the_manifest_names_records_a_different_value() -> TestResult {
    let root = folding_root()?;
    let before = built(&root)?.recorded_fold();

    let edited = commented(&text_of(&root, CUBE_MODEL)?);
    root.holding(CUBE_MODEL, &edited)?;
    let after = built(&root)?;

    assert_eq!(
        (after.code, recorded(&before, &after.recorded_fold())),
        (ExitCode::Success, Recorded::Moved),
        "the edit here is a comment line: the model renders to exactly the pixels it rendered to \
         before. The value has to move anyway, because it is a fold over what the build read and \
         not over what it drew — an implementation folding its own output would satisfy every \
         other reading in this file and miss this one. It said: {err}",
        err = after.err
    );
    Ok(())
}

#[test]
fn editing_a_model_no_entry_names_records_the_same_value() -> TestResult {
    let root = folding_root()?;
    let before = built(&root)?.recorded_fold();

    let edited = commented(&text_of(&root, SPARE_MODEL)?);
    root.holding(SPARE_MODEL, &edited)?;
    let after = built(&root)?;

    assert_eq!(
        (after.code, recorded(&before, &after.recorded_fold())),
        (ExitCode::Success, Recorded::Stayed),
        "this is the reading that fixes the fold's extent, and it is the only one that does. \
         Folding everything under the content root satisfies every staleness scenario in this \
         file and turns an edit to a model nobody bakes into a client that refuses to launch \
         until somebody rebuilds textures that did not change. It said: {err}",
        err = after.err
    );
    Ok(())
}

#[test]
fn a_material_that_is_not_a_toml_file_does_not_move_the_fold() -> TestResult {
    let root = folding_root()?;
    let before = built(&root)?.recorded_fold();

    root.holding(A_NOTE_IN_THE_MATERIALS_DIRECTORY, "a note to whoever\n")?;
    let after = built(&root)?;

    assert_eq!(
        (after.code, recorded(&before, &after.recorded_fold())),
        (ExitCode::Success, Recorded::Stayed),
        "the build reads every `*.toml` in the materials directory and nothing else, so that is \
         exactly what it folds. Folding the whole directory instead would be the same spurious \
         refusal as folding the whole content tree, arriving by the other door — and no scenario \
         reaches it. It said: {err}",
        err = after.err
    );
    Ok(())
}

#[test]
fn a_source_that_cannot_be_read_while_folding_refuses_the_build_naming_it() -> TestResult {
    let root = folding_root()?;
    fs::create_dir_all(root.path().join(AN_UNREADABLE_MATERIAL))?;

    let made = built(&root)?;

    assert_eq!(
        (made.refusal(&["locked.toml"]), made.images()),
        (Refused::NamingEverything, Vec::new()),
        "a source the build reaches and cannot read has to stop it. Treating it as empty records \
         a value that is not a function of what the build consumed, and the set that results \
         reports itself current forever against a file nobody can open. It said: {err}",
        err = made.err
    );
    Ok(())
}
