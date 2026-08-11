//! A capture that matches its golden: passing, and leaving nothing behind.
//!
//! Artifacts are evidence of a failure. A directory still holding last run's
//! `diff.png` after today's pass is worse than an empty one, because the agent
//! reading it has no way to tell it is stale — so clearing is part of passing,
//! not housekeeping.

mod support;

use std::fs;
use std::path::Path;

use mc_testkit::frame::{CaptureId, GoldenOutcome, OptIns, verify_against_golden};
use support::{
    TestResult, artifact_dir, golden_settings, install_golden, reference_frame,
    synthetic_provenance,
};
use tempfile::TempDir;

const CAPTURE: &str = "clear-red-64";
/// A different capture, whose artifacts are none of this one's business.
const NEIGHBOUR: &str = "half-fill-64";
/// Everything the harness may ever write into a capture's artifact directory.
/// Deletion is by this allowlist and never recursive: the artifact root is
/// caller-supplied, so `remove_dir_all` on it points at whatever they passed.
const ARTIFACT_SET: [&str; 4] = ["expected.png", "actual.png", "diff.png", "report.json"];

fn stale_bytes() -> &'static [u8] {
    b"left behind by an earlier mismatch"
}

fn plant_artifacts(directory: &Path) -> Result<(), std::io::Error> {
    fs::create_dir_all(directory)?;
    for name in ARTIFACT_SET {
        fs::write(directory.join(name), stale_bytes())?;
    }
    Ok(())
}

fn entry_count(directory: &Path) -> Result<usize, std::io::Error> {
    if !directory.exists() {
        return Ok(0);
    }
    Ok(fs::read_dir(directory)?.count())
}

#[test]
fn a_capture_matching_its_golden_passes_and_leaves_its_artifact_directory_empty() -> TestResult {
    let goldens = TempDir::new()?;
    let artifacts = TempDir::new()?;
    let capture = CaptureId::new(CAPTURE)?;
    let frame = reference_frame()?;
    install_golden(goldens.path(), &capture, &frame)?;
    let settings = golden_settings(
        goldens.path(),
        artifacts.path(),
        capture.clone(),
        OptIns::default(),
    );

    let outcome = verify_against_golden(&frame, &synthetic_provenance(), &settings);

    assert!(
        matches!(outcome, GoldenOutcome::Pass),
        "a frame identical to its golden passes, got {outcome:?}"
    );
    assert_eq!(
        entry_count(&artifact_dir(artifacts.path(), &capture))?,
        0,
        "a pass writes no file into the capture's artifact directory"
    );
    Ok(())
}

#[test]
fn a_pass_removes_the_artifacts_an_earlier_mismatch_left_behind() -> TestResult {
    let goldens = TempDir::new()?;
    let artifacts = TempDir::new()?;
    let capture = CaptureId::new(CAPTURE)?;
    let frame = reference_frame()?;
    install_golden(goldens.path(), &capture, &frame)?;
    let directory = artifact_dir(artifacts.path(), &capture);
    plant_artifacts(&directory)?;
    let settings = golden_settings(goldens.path(), artifacts.path(), capture, OptIns::default());

    let outcome = verify_against_golden(&frame, &synthetic_provenance(), &settings);

    assert!(
        matches!(outcome, GoldenOutcome::Pass),
        "the golden still matches, got {outcome:?}"
    );
    let survivors: Vec<&str> = ARTIFACT_SET
        .iter()
        .filter(|name| directory.join(name).exists())
        .copied()
        .collect();
    assert!(
        survivors.is_empty(),
        "yesterday's mismatch must not read as today's; found {survivors:?}"
    );
    Ok(())
}

#[test]
fn clearing_one_captures_artifacts_leaves_another_captures_alone() -> TestResult {
    let goldens = TempDir::new()?;
    let artifacts = TempDir::new()?;
    let capture = CaptureId::new(CAPTURE)?;
    let neighbour = CaptureId::new(NEIGHBOUR)?;
    let frame = reference_frame()?;
    install_golden(goldens.path(), &capture, &frame)?;
    let neighbours_directory = artifact_dir(artifacts.path(), &neighbour);
    plant_artifacts(&neighbours_directory)?;
    let settings = golden_settings(goldens.path(), artifacts.path(), capture, OptIns::default());

    let outcome = verify_against_golden(&frame, &synthetic_provenance(), &settings);

    assert!(
        matches!(outcome, GoldenOutcome::Pass),
        "the golden still matches, got {outcome:?}"
    );
    let untouched: Vec<&str> = ARTIFACT_SET
        .iter()
        .filter(|name| neighbours_directory.join(name).exists())
        .copied()
        .collect();
    assert_eq!(
        untouched.len(),
        ARTIFACT_SET.len(),
        "one capture's pass must not delete another capture's evidence; only \
         {untouched:?} survived"
    );
    Ok(())
}
