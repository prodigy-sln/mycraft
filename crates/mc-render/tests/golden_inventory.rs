//! What the golden set is called, that it is all there, and what happens when
//! it is not.
//!
//! Nothing here needs a device, and that is the point: a capture id is a string
//! derived from a tick and a revision, and the golden lifecycle judges an image
//! it is handed. Both are decisions, so both are testable in the configuration
//! where `wgpu` is not in this crate's dependency graph at all.
//!
//! # Why the revision is in the id
//!
//! Ambient occlusion will one day narrow the merge predicate and invalidate
//! every committed frame. Carrying the scene revision in the id turns that from
//! a silent re-shoot into a **rename**: the commit shows added and removed PNGs
//! instead of a modified binary blob nobody can read, and a bumped revision with
//! no new goldens fails as a *missing* golden naming the path it looked for.
//!
//! The inventory check below is the other half of that. Renaming the set only
//! helps if the previous set has to go, so `goldens/` is asserted to hold
//! exactly the directories the current revision declares — an orphaned
//! revision fails the gate rather than lingering beside its replacement.
//!
//! # Placement
//!
//! `tasks.md` names a sibling `capture_test.rs`. Every property here is
//! reachable from the public API, and `docs/technical/testing.md` binds the
//! choice: *"Test through the public API in `tests/` by default; write a
//! private-access unit test only where the property genuinely has no public
//! surface."* Nothing here needs private access. No signature is affected
//! either way.

use std::error::Error;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use mc_render::capture::{
    DECLARED_CAPTURE_TICKS, SCENE_REVISION, capture_id, declared_capture_ids,
};
use mc_testkit::frame::{
    AdapterProvenance, Backend, CaptureId, GoldenFailureReason, GoldenOutcome, GoldenSettings,
    OptIns, Rgba8Image, Thresholds, verify_against_golden,
};
use tempfile::TempDir;

type TestResult = Result<(), Box<dyn Error>>;

/// A revision nobody has captured anything for.
///
/// The scenarios ask what a *second* revision produces, so this is the one the
/// spec names rather than an arbitrary string: `r2` is what the day after
/// ambient occlusion looks like.
const SECOND_REVISION: &str = "r2";

/// The tick whose id the missing-golden refusal is asked for.
const TICK: u16 = 60;

/// The filename the golden lifecycle reads inside a capture's directory.
const GOLDEN_FILE: &str = "default.png";

/// Opt-ins with the update variable unset, stated rather than read.
///
/// `MYCRAFT_UPDATE_GOLDENS` is what the scenario says must be unset, and no
/// test in this project sets or unsets an environment variable — `set_var` is
/// `unsafe` in edition 2024. Constructing the value says the same thing and
/// keeps the assertion true even for a developer who is mid-re-shoot.
const NOT_UPDATING: OptIns = OptIns {
    allow_no_gpu: false,
    update_goldens: false,
};

#[test]
fn the_capture_ids_of_a_second_scene_revision_all_carry_it_and_none_repeats_the_first() -> TestResult
{
    let first = declared_capture_ids(SCENE_REVISION)?;
    let second = declared_capture_ids(SECOND_REVISION)?;

    let carrying = second
        .iter()
        .filter(|id| id.contains(SECOND_REVISION))
        .count();
    let repeated = first
        .iter()
        .zip(&second)
        .filter(|(one, other)| one == other)
        .count();

    assert_eq!(
        (second.len(), carrying, repeated),
        (
            DECLARED_CAPTURE_TICKS.len(),
            DECLARED_CAPTURE_TICKS.len(),
            0
        ),
        "a revision has to reach every declared capture's id and rename all of them: an id \
         that ignored the revision it was asked for would leave the new set colliding with \
         the old one, which is exactly the silent re-shoot the revision exists to prevent. \
         `{SCENE_REVISION}` gave {first:?} and `{SECOND_REVISION}` gave {second:?}"
    );
    Ok(())
}

#[test]
fn a_revision_whose_goldens_were_never_captured_fails_naming_the_path_it_looked_for() -> TestResult
{
    let workspace = TempDir::new()?;
    let (settings, looked_for) = settings_for_an_uncaptured_revision(workspace.path())?;

    let outcome = verify_against_golden(&a_frame()?, &an_adapter(), &settings);

    let GoldenOutcome::Failed(failure) = outcome else {
        return Err(format!("a revision with no goldens must fail, got {outcome:?}").into());
    };
    let reported = failure.to_string();
    let minted = looked_for.exists();
    assert!(
        matches!(failure.reason, GoldenFailureReason::MissingGolden { .. })
            && reported.contains(&looked_for.display().to_string())
            && !minted,
        "bumping the revision without capturing its goldens has to fail naming the path it \
         looked for, and must not mint one on the way past — a harness that captured a \
         replacement would make every future frame its own ground truth. It reported \
         `{reported}` and the golden at `{}` was {}",
        looked_for.display(),
        if minted { "written" } else { "still absent" }
    );
    Ok(())
}

/// The lifecycle's settings for a revision nothing was ever captured for, and
/// the golden path they will send it looking for.
///
/// Both come back together because the path is not a second guess at what the
/// settings imply — it is built from the same capture id and the same root, so
/// the assertion cannot pass by agreeing with a path the test made up.
fn settings_for_an_uncaptured_revision(
    workspace: &Path,
) -> Result<(GoldenSettings, PathBuf), Box<dyn Error>> {
    let capture = CaptureId::new(&capture_id(TICK, SECOND_REVISION)?)?;
    let golden_root = workspace.join("goldens");
    let looked_for = golden_root.join(capture.as_str()).join(GOLDEN_FILE);
    let settings = GoldenSettings {
        golden_root,
        artifact_root: workspace.join("artifacts"),
        capture,
        thresholds: Thresholds::default(),
        opt_ins: NOT_UPDATING,
    };
    Ok((settings, looked_for))
}

#[test]
fn the_committed_goldens_are_exactly_the_directories_the_current_revision_declares() -> TestResult {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("goldens");
    let mut committed = entries_of(&root)?;
    let mut declared = declared_capture_ids(SCENE_REVISION)?;
    committed.sort();
    declared.sort();

    assert_eq!(
        committed,
        declared,
        "`{}` has to hold exactly the captures revision `{SCENE_REVISION}` declares: a set \
         left behind by a previous revision keeps a frame nothing compares against, and a \
         directory for a capture nobody declared is a golden no test reads",
        root.display()
    );
    Ok(())
}

/// Everything `directory` holds, by name. A directory that is not there holds
/// nothing — which is a state the assertion is entitled to report rather than
/// an error that hides what was expected.
fn entries_of(directory: &Path) -> Result<Vec<String>, Box<dyn Error>> {
    let read = match fs::read_dir(directory) {
        Ok(read) => read,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error.into()),
    };
    let mut names = Vec::new();
    for entry in read {
        names.push(entry?.file_name().to_string_lossy().into_owned());
    }
    Ok(names)
}

/// A frame to hand the lifecycle. Its contents decide nothing here: the golden
/// it would be compared against does not exist.
fn a_frame() -> Result<Rgba8Image, Box<dyn Error>> {
    Ok(Rgba8Image::from_rgba(1, 1, vec![0, 0, 0, 255])?)
}

/// The provenance a golden would have been written with, had one been written.
fn an_adapter() -> AdapterProvenance {
    AdapterProvenance::new("test", Backend::Other, None)
}
