//! A capture that does not match its golden: the artifact set, and where it is.
//!
//! The fixtures are split across rows, so a frame written upside-down into
//! `expected.png` or `actual.png` is a different file rather than the same one.
//! A capture path that inverted rows would make every golden this project ever
//! commits wrong in the same direction, consistently and therefore invisibly.

mod support;

use std::error::Error;
use std::fs;

use mc_testkit::frame::{
    CaptureId, GoldenFailureReason, GoldenOutcome, OptIns, verify_against_golden,
};
use support::{
    TestResult, artifact_dir, drifted_frame, golden_settings, install_golden, reference_frame,
    synthetic_provenance,
};
use tempfile::TempDir;

const CAPTURE: &str = "clear-red-64";
const ARTIFACT_SET: [&str; 4] = ["expected.png", "actual.png", "diff.png", "report.json"];

#[test]
fn a_mismatching_capture_writes_the_whole_artifact_set() -> TestResult {
    let goldens = TempDir::new()?;
    let artifacts = TempDir::new()?;
    let capture = CaptureId::new(CAPTURE)?;
    install_golden(goldens.path(), &capture, &reference_frame()?)?;
    let settings = golden_settings(
        goldens.path(),
        artifacts.path(),
        capture.clone(),
        OptIns::default(),
    );

    let outcome = verify_against_golden(&drifted_frame()?, &synthetic_provenance(), &settings);

    let GoldenOutcome::Failed(failure) = outcome else {
        return Err(
            format!("a drifted frame must fail against its golden, got {outcome:?}").into(),
        );
    };
    let directory = artifact_dir(artifacts.path(), &capture);
    let missing: Vec<&str> = ARTIFACT_SET
        .iter()
        .filter(|name| !directory.join(name).exists())
        .copied()
        .collect();
    assert!(
        missing.is_empty(),
        "the expected frame, the actual frame, the diff and the report are one \
         set; missing {missing:?} (artifacts: {:?})",
        failure.artifacts
    );
    Ok(())
}

#[test]
fn a_reported_mismatch_names_the_directory_holding_its_artifacts() -> TestResult {
    let goldens = TempDir::new()?;
    let artifacts = TempDir::new()?;
    let capture = CaptureId::new(CAPTURE)?;
    install_golden(goldens.path(), &capture, &reference_frame()?)?;
    let settings = golden_settings(
        goldens.path(),
        artifacts.path(),
        capture.clone(),
        OptIns::default(),
    );

    let outcome = verify_against_golden(&drifted_frame()?, &synthetic_provenance(), &settings);

    let GoldenOutcome::Failed(failure) = outcome else {
        return Err(
            format!("a drifted frame must fail against its golden, got {outcome:?}").into(),
        );
    };
    let directory = artifact_dir(artifacts.path(), &capture);
    assert!(
        failure
            .to_string()
            .contains(&directory.display().to_string()),
        "a failure the reader cannot follow to the evidence is half a failure; \
         got `{failure}`"
    );
    Ok(())
}

#[test]
fn an_artifact_directory_that_cannot_be_created_still_reports_the_mismatch() -> TestResult {
    let goldens = TempDir::new()?;
    let workspace = TempDir::new()?;
    // A plain file standing where the artifact root's parent would have to be a
    // directory: nothing beneath it can be created, on any platform.
    let blocker = workspace.path().join("occupied");
    fs::write(&blocker, b"a file, not a directory")?;
    let artifact_root = blocker.join("mycraft-frames");
    let capture = CaptureId::new(CAPTURE)?;
    install_golden(goldens.path(), &capture, &reference_frame()?)?;
    let settings = golden_settings(goldens.path(), &artifact_root, capture, OptIns::default());

    let outcome = verify_against_golden(&drifted_frame()?, &synthetic_provenance(), &settings);

    let GoldenOutcome::Failed(failure) = outcome else {
        return Err(format!("the frames still differ, whatever disk did, got {outcome:?}").into());
    };
    assert!(
        matches!(failure.reason, GoldenFailureReason::Mismatch(_)),
        "an artifact failure must not swallow the verdict it was recording, \
         got {:?}",
        failure.reason
    );
    let artifact_error = failure
        .artifacts
        .as_ref()
        .err()
        .ok_or("writing beneath a file must fail")?;
    assert!(
        artifact_error.source().is_some(),
        "the artifact failure carries its underlying cause, got \
         {artifact_error:?}"
    );
    Ok(())
}
