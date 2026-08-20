//! What the client makes of the texture set under a content root, one verdict
//! per root.
//!
//! # Why every reading here is an enumerated arm and never an absence
//!
//! `assert!(no_refusal_printed)` cannot tell a healthy set from a client that
//! has lost the ability to check: both write nothing. `SetVerdict` is a total
//! enumeration returned in `Ok`, so `assert_eq!(verdict, Current)` rejects every
//! other arm *including* the ones that mean "I could not look" — and a check
//! that stops checking reddens for free rather than going quiet.
//!
//! That is also why the verdict is returned rather than raised. Three of the six
//! arms let a launch continue, so an answer that only existed as an error would
//! leave those three unconstructible and the totality the suite is holding would
//! not be the one the reasoning claims.
//!
//! # The refusal is asked for separately, and this file only asks whether there
//! is one
//!
//! `refusal_for` maps a verdict to what a player reads. What that text says is
//! `set_refusal_and_key_fallback_differ.rs`'s subject; what this file adds is
//! which verdicts produce a refusal at all, since "report the set as current and
//! **complete the launch**" is half of what four of these scenarios claim.
//!
//! # The set is derived, and a checkout that has not built one has nothing to
//! read
//!
//! Every fixture copies the shipped root, built set included. A tree that has
//! not run `cargo run -p voxforge -- build content/base/textures.toml` therefore
//! has no set to copy, and the fixture says so in one sentence rather than
//! letting eight verdict mismatches read as a broken client. The gate builds the
//! set before it runs anything, which is why the gate is green and a bare
//! `cargo nextest run` on a fresh checkout is not.

use std::error::Error;
use std::path::{Path, PathBuf};

use mc_client::textures::{SetVerdict, built_set, refusal_for};
use mc_core::id::TextureKey;

mod support;

use support::{TestResult, built_sets, content_root};

/// The error type every test in this file propagates with `?`.
type SetVerdictResult = Result<SetVerdict, Box<dyn Error>>;

#[test]
fn an_absent_index_reports_the_set_absent_and_the_refusal_names_the_build_command() -> TestResult {
    let root = built_sets::a_root_with_a_built_set()?;
    built_sets::without_the_index(root.path())?;

    let verdict = verdict_over(root.path())?;

    assert_eq!(verdict, SetVerdict::Absent);
    assert!(
        names_the_build_command(&verdict)?,
        "a root whose index is gone has to be told what to run to get one, and the refusal for \
         {verdict:?} does not name it"
    );
    Ok(())
}

#[test]
fn a_model_edited_since_the_build_reports_the_set_stale_and_names_the_rebuild_command() -> TestResult
{
    let root = built_sets::a_root_with_a_built_set()?;
    built_sets::with_a_model_edited(root.path())?;

    let verdict = verdict_over(root.path())?;

    assert_eq!(verdict, SetVerdict::StaleAgainstSources);
    assert!(
        names_the_build_command(&verdict)?,
        "a set built from a model that has since been edited is rebuilt by one command, and the \
         refusal for {verdict:?} does not name it"
    );
    Ok(())
}

#[test]
fn a_manifest_that_gained_an_entry_reports_the_set_stale_and_refuses_the_launch() -> TestResult {
    let root = built_sets::a_root_with_a_built_set()?;
    built_sets::with_a_gained_manifest_entry(root.path())?;

    let verdict = verdict_over(root.path())?;

    assert_eq!(verdict, SetVerdict::StaleAgainstSources);
    assert!(
        refusal_for(&verdict).is_some(),
        "a manifest naming a key the set does not hold has to turn the launch away, and \
         {verdict:?} let it through"
    );
    Ok(())
}

#[test]
fn a_recorded_source_that_is_no_longer_present_reports_the_set_stale_and_names_it() -> TestResult {
    let root = built_sets::a_root_with_a_built_set()?;
    built_sets::without_a_recorded_source(root.path(), built_sets::A_RECORDED_MATERIAL)?;

    let verdict = verdict_over(root.path())?;

    assert_eq!(
        verdict,
        SetVerdict::SourceMissing {
            source: PathBuf::from(built_sets::A_RECORDED_MATERIAL),
        }
    );
    assert!(
        refusal_for(&verdict).is_some(),
        "a source the set was built from and that is no longer there has to turn the launch \
         away, and {verdict:?} let it through"
    );
    Ok(())
}

#[test]
fn an_index_naming_an_absent_image_refuses_the_launch_naming_the_image_and_its_key() -> TestResult {
    let root = built_sets::a_root_with_a_built_set()?;
    built_sets::without_a_recorded_image(root.path(), built_sets::A_RECORDED_IMAGE)?;

    let verdict = verdict_over(root.path())?;

    assert_eq!(
        verdict,
        SetVerdict::ImageMissing {
            key: TextureKey::parse(built_sets::THE_KEY_THAT_IMAGE_BELONGS_TO)?,
            image: PathBuf::from(built_sets::A_RECORDED_IMAGE),
        }
    );
    assert!(
        refusal_for(&verdict).is_some(),
        "an index promising art in a file that is not there has to turn the launch away, and \
         {verdict:?} let it through"
    );
    Ok(())
}

#[test]
fn a_present_and_current_set_reports_current_and_completes_the_launch() -> TestResult {
    // The repository's own content root, not a copy of it: this is the tree a
    // player launches against and the one the gate builds the set into, so it
    // is the reading that says the shipped arrangement works rather than that
    // the fixtures do.
    let verdict = verdict_over(&content_root()?)?;

    assert_eq!(verdict, SetVerdict::Current);
    assert!(
        refusal_for(&verdict).is_none(),
        "a set that is present and current is what a launch is supposed to proceed on, and \
         {verdict:?} produced a refusal"
    );
    Ok(())
}

#[test]
fn an_index_naming_no_keys_and_current_against_its_sources_reports_current() -> TestResult {
    let root = built_sets::a_root_with_a_built_set()?;
    built_sets::with_an_index_naming_no_keys(root.path())?;

    let verdict = verdict_over(root.path())?;

    assert_eq!(verdict, SetVerdict::Current);
    assert!(
        refusal_for(&verdict).is_none(),
        "an index that covers nothing is still an index that matches its sources, and \
         {verdict:?} produced a refusal"
    );
    Ok(())
}

#[test]
fn a_content_root_stating_no_texture_manifest_reports_no_art_declared_and_launches() -> TestResult {
    let root = built_sets::a_root_with_a_built_set()?;
    built_sets::without_any_art(root.path())?;

    let verdict = verdict_over(root.path())?;

    assert_eq!(verdict, SetVerdict::NoArtDeclared);
    assert!(
        refusal_for(&verdict).is_none(),
        "a root that declares no art has nothing to build and nothing to be stale against, and \
         being told to run the art build would blame the wrong party — {verdict:?} produced a \
         refusal"
    );
    Ok(())
}

#[test]
fn the_client_refolds_the_sources_the_index_recorded_and_never_reads_the_manifest() -> TestResult {
    let root = built_sets::a_root_with_a_built_set()?;
    // A build folds every `*.toml` under the manifest's materials directory, so
    // a client that went back to the manifest for its source list would fold
    // this file and call the set stale. One that re-folds what the index
    // recorded cannot see it — and on the shipped tree the two answers agree,
    // which is why no verdict scenario can tell them apart.
    built_sets::with_a_material_the_index_never_recorded(root.path())?;

    let verdict = verdict_over(root.path())?;

    assert_eq!(verdict, SetVerdict::Current);
    Ok(())
}

#[test]
fn a_content_root_copied_to_a_temporary_directory_is_still_current() -> TestResult {
    // Nothing is changed. What is being asked is whether the fold survives the
    // move, which it does only because the index records its sources relative
    // to the root. Absolute paths would make every copied root permanently
    // stale and would put whoever ran the build into a file the gate writes.
    let root = built_sets::a_root_with_a_built_set()?;

    let verdict = verdict_over(root.path())?;

    assert_eq!(verdict, SetVerdict::Current);
    Ok(())
}

/// What the client makes of the set under `root`.
///
/// The texels it offers are dropped here: no scenario in this phase asks what
/// they are, and the set is judged and then not used.
///
/// # Errors
///
/// Returns an error if the set could not be read at all, which is a different
/// axis from what it is and is read in
/// `an_unreadable_set_names_what_it_cannot_read.rs`.
fn verdict_over(root: &Path) -> SetVerdictResult {
    let (verdict, _texels) = built_set(root)?;
    Ok(verdict)
}

/// Whether the refusal `verdict` becomes quotes the command that builds a set.
///
/// # Errors
///
/// Returns an error if the verdict produced no refusal at all. A scenario asking
/// which command a refusal names has nothing to read when there is no refusal,
/// and a search of the empty string would report the absence as a wording
/// problem.
fn names_the_build_command(verdict: &SetVerdict) -> Result<bool, Box<dyn Error>> {
    let refused = refusal_for(verdict).ok_or_else(|| {
        format!(
            "this scenario needs {verdict:?} to refuse the launch so that the command it names \
             can be read, and it let the launch through instead"
        )
    })?;
    Ok(refused
        .to_string()
        .contains(mc_client::startup::BUILD_THE_TEXTURE_SET))
}
