//! Rewriting goldens, which happens only when someone asks.
//!
//! The opt-in is injected as a value rather than set in the environment
//! (`std::env::set_var` is `unsafe` in edition 2024), so what these tests
//! exercise is the harness's decision, not the process's state.
//!
//! The fixtures are split across rows: an overwritten golden must be the frame
//! that was captured, the same way up, or every comparison from then on is
//! against ground truth that is itself inverted.

mod support;

use std::error::Error;
use std::fs;
use std::path::Path;

use mc_testkit::frame::{
    AdapterProvenance, Backend, CaptureId, GoldenFailure, GoldenFailureReason, GoldenOutcome,
    read_png, verify_against_golden,
};
use serde_json::Value;
use support::{
    TestResult, UPDATING, drifted_frame, golden_image_path, golden_settings, golden_sidecar_path,
    install_golden, reference_frame, synthetic_provenance,
};
use tempfile::TempDir;

const CAPTURE: &str = "clear-red-64";
const ADAPTER: &str = "NVIDIA GeForce RTX 4090";
const BACKEND: &str = "vulkan";

/// The words a caller who set the opt-in has to find. Asserted as a substring:
/// the operating system's own wording for the underlying failure follows it and
/// differs per platform.
const NOT_UPDATED: &str = "the golden was NOT updated";

fn identified_adapter() -> AdapterProvenance {
    AdapterProvenance::new(ADAPTER, Backend::Vulkan, Some("566.36"))
}

#[test]
fn the_update_path_overwrites_a_mismatching_golden_and_reports_every_path_it_wrote() -> TestResult {
    let goldens = TempDir::new()?;
    let artifacts = TempDir::new()?;
    let capture = CaptureId::new(CAPTURE)?;
    install_golden(goldens.path(), &capture, &reference_frame()?)?;
    let settings = golden_settings(goldens.path(), artifacts.path(), capture.clone(), UPDATING);
    let captured = drifted_frame()?;

    let outcome = verify_against_golden(&captured, &synthetic_provenance(), &settings);

    let GoldenOutcome::GoldenWritten { paths } = outcome else {
        return Err(
            format!("the opt-in asks for the golden to be rewritten, got {outcome:?}").into(),
        );
    };
    let golden = golden_image_path(goldens.path(), &capture);
    let sidecar = golden_sidecar_path(goldens.path(), &capture);
    assert!(
        paths.contains(&golden) && paths.contains(&sidecar),
        "an unexplained golden update is a review stop, so every path written \
         has to be reported; got {paths:?}"
    );
    assert!(
        read_png(&golden)?.as_bytes() == captured.as_bytes(),
        "the golden is now the captured frame, the same way up"
    );
    Ok(())
}

#[test]
fn the_update_path_leaves_a_matching_golden_byte_for_byte_alone() -> TestResult {
    let goldens = TempDir::new()?;
    let artifacts = TempDir::new()?;
    let capture = CaptureId::new(CAPTURE)?;
    let frame = reference_frame()?;
    let golden = install_golden(goldens.path(), &capture, &frame)?;
    let before = fs::read(&golden)?;
    let settings = golden_settings(goldens.path(), artifacts.path(), capture, UPDATING);

    let outcome = verify_against_golden(&frame, &synthetic_provenance(), &settings);

    assert!(
        matches!(outcome, GoldenOutcome::GoldenUnchanged),
        "a golden that already matches was not written, so no path is reported; \
         got {outcome:?}"
    );
    assert!(
        fs::read(&golden)? == before,
        "permission to rewrite a golden is not an instruction to churn it"
    );
    Ok(())
}

#[test]
fn a_written_golden_records_the_adapter_and_backend_that_produced_it() -> TestResult {
    let goldens = TempDir::new()?;
    let artifacts = TempDir::new()?;
    let capture = CaptureId::new(CAPTURE)?;
    install_golden(goldens.path(), &capture, &reference_frame()?)?;
    let settings = golden_settings(goldens.path(), artifacts.path(), capture.clone(), UPDATING);

    let outcome = verify_against_golden(&drifted_frame()?, &identified_adapter(), &settings);

    assert!(
        matches!(outcome, GoldenOutcome::GoldenWritten { .. }),
        "the golden was rewritten, got {outcome:?}"
    );
    let sidecar_path = golden_sidecar_path(goldens.path(), &capture);
    let sidecar: Value = serde_json::from_str(&fs::read_to_string(&sidecar_path)?)?;
    assert_eq!(
        (
            sidecar.get("adapter").and_then(Value::as_str),
            sidecar.get("backend").and_then(Value::as_str),
        ),
        (Some(ADAPTER), Some(BACKEND)),
        "the day a second adapter runs the gate, the existing set's provenance \
         has to already be known"
    );
    Ok(())
}

#[test]
fn an_update_that_cannot_write_the_golden_reports_that_it_was_not_updated() -> TestResult {
    let goldens = TempDir::new()?;
    let artifacts = TempDir::new()?;

    let failure = update_into_a_blocked_golden_directory(goldens.path(), artifacts.path())?;

    assert!(
        failure.to_string().contains(NOT_UPDATED),
        "a caller who asked for the golden to be rewritten is otherwise shown \
         only the verdict the update was supposed to have fixed, and walks into \
         the same wall on the next run; got `{failure}`"
    );
    Ok(())
}

#[test]
fn a_failed_golden_update_still_reports_the_verdict_that_stands() -> TestResult {
    let goldens = TempDir::new()?;
    let artifacts = TempDir::new()?;

    let failure = update_into_a_blocked_golden_directory(goldens.path(), artifacts.path())?;

    assert!(
        matches!(failure.reason, GoldenFailureReason::MissingGolden { .. }),
        "a golden that could not be written is still missing, and the write \
         failure must not swallow that; got {:?}",
        failure.reason
    );
    Ok(())
}

#[test]
fn a_golden_written_without_its_sidecar_still_reports_the_path_it_wrote() -> TestResult {
    let goldens = TempDir::new()?;
    let artifacts = TempDir::new()?;
    let capture = CaptureId::new(CAPTURE)?;
    install_golden(goldens.path(), &capture, &reference_frame()?)?;
    // A directory standing exactly where the sidecar's file has to go. The
    // golden beside it is written normally, so only the JSON fails — which is
    // the narrow case: the image landed, the record of which adapter produced
    // it did not.
    fs::create_dir(golden_sidecar_path(goldens.path(), &capture))?;
    let settings = golden_settings(goldens.path(), artifacts.path(), capture.clone(), UPDATING);
    let captured = drifted_frame()?;

    let outcome = verify_against_golden(&captured, &synthetic_provenance(), &settings);

    let golden = golden_image_path(goldens.path(), &capture);
    assert!(
        read_png(&golden)?.as_bytes() == captured.as_bytes(),
        "this test means nothing unless the image write succeeded and the \
         sidecar was the only casualty"
    );
    let GoldenOutcome::GoldenWrittenWithoutProvenance { paths, .. } = &outcome else {
        return Err(format!(
            "a golden that was replaced must not be reported as one that never was, got {outcome:?}"
        )
        .into());
    };
    assert!(
        paths.contains(&golden),
        "the golden on disk was replaced a moment earlier, so its path is one \
         the run has to report; got {paths:?}"
    );
    Ok(())
}

/// Runs the update path against a golden directory that cannot be created,
/// and returns the failure it reported.
///
/// The golden is never installed — it is missing, and writing it is exactly
/// what the opt-in asked for. A plain file stands where the capture's golden
/// directory would have to go, so creating it fails on every platform with no
/// lock-injection machinery.
fn update_into_a_blocked_golden_directory(
    goldens: &Path,
    artifacts: &Path,
) -> Result<GoldenFailure, Box<dyn Error>> {
    let capture = CaptureId::new(CAPTURE)?;
    fs::write(goldens.join(capture.as_str()), b"a file, not a directory")?;
    let settings = golden_settings(goldens, artifacts, capture, UPDATING);

    let outcome = verify_against_golden(&reference_frame()?, &synthetic_provenance(), &settings);

    let GoldenOutcome::Failed(failure) = outcome else {
        return Err(
            format!("a golden that could not be written is a failure, got {outcome:?}").into(),
        );
    };
    Ok(failure)
}
