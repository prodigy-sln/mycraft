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

use std::fs;

use mc_testkit::frame::{
    AdapterProvenance, Backend, CaptureId, GoldenOutcome, read_png, verify_against_golden,
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
