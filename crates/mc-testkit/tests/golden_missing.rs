//! Goldens that are absent or unreadable.
//!
//! A broken renderer must never mint its own ground truth. Both cases here fail
//! loudly and leave the golden set exactly as it was, which is what keeps a
//! green suite meaningful: the only way a golden changes is somebody asking for
//! it.

mod support;

use std::fs;

use mc_testkit::frame::{
    CaptureId, GoldenFailureReason, GoldenOutcome, OptIns, read_png, verify_against_golden,
};
use support::{
    TestResult, UPDATING, artifact_dir, golden_image_path, golden_settings, reference_frame,
    synthetic_provenance,
};
use tempfile::TempDir;

const CAPTURE: &str = "clear-red-64";
/// Not a PNG by any reading of the signature.
const CORRUPT: &[u8] = b"this was a golden once";

#[test]
fn a_missing_golden_fails_naming_the_path_and_is_not_created() -> TestResult {
    let goldens = TempDir::new()?;
    let artifacts = TempDir::new()?;
    let capture = CaptureId::new(CAPTURE)?;
    let settings = golden_settings(
        goldens.path(),
        artifacts.path(),
        capture.clone(),
        OptIns::default(),
    );

    let outcome = verify_against_golden(&reference_frame()?, &synthetic_provenance(), &settings);

    let GoldenOutcome::Failed(failure) = outcome else {
        return Err(format!("a capture with no golden has not passed, got {outcome:?}").into());
    };
    let GoldenFailureReason::MissingGolden { path } = &failure.reason else {
        return Err(format!(
            "expected a missing-golden failure, got {:?}",
            failure.reason
        )
        .into());
    };
    assert_eq!(
        path,
        &golden_image_path(goldens.path(), &capture),
        "the failure names the golden it went looking for"
    );
    assert!(
        !path.exists(),
        "the run that noticed a golden was missing must not be the run that \
         creates it"
    );
    Ok(())
}

#[test]
fn a_missing_golden_writes_the_captured_frame_into_the_artifact_directory() -> TestResult {
    let goldens = TempDir::new()?;
    let artifacts = TempDir::new()?;
    let capture = CaptureId::new(CAPTURE)?;
    let settings = golden_settings(
        goldens.path(),
        artifacts.path(),
        capture.clone(),
        OptIns::default(),
    );
    // Split across rows, so a frame written upside-down is a different file.
    let captured = reference_frame()?;

    let outcome = verify_against_golden(&captured, &synthetic_provenance(), &settings);

    assert!(
        matches!(outcome, GoldenOutcome::Failed(_)),
        "a missing golden is still a failure, got {outcome:?}"
    );
    let written = read_png(&artifact_dir(artifacts.path(), &capture).join("actual.png"))?;
    assert!(
        written.as_bytes() == captured.as_bytes(),
        "the artifact must be the frame that was captured, the same way up — \
         it is the only record of what the run actually produced"
    );
    Ok(())
}

#[test]
fn a_golden_that_is_not_a_decodable_png_fails_without_being_replaced() -> TestResult {
    let goldens = TempDir::new()?;
    let artifacts = TempDir::new()?;
    let capture = CaptureId::new(CAPTURE)?;
    let golden = golden_image_path(goldens.path(), &capture);
    fs::create_dir_all(
        golden
            .parent()
            .ok_or("a golden path always has a directory")?,
    )?;
    fs::write(&golden, CORRUPT)?;
    // The update opt-in is deliberately set: "must not overwrite" is only a
    // restraint where overwriting is otherwise permitted.
    let settings = golden_settings(goldens.path(), artifacts.path(), capture, UPDATING);

    let outcome = verify_against_golden(&reference_frame()?, &synthetic_provenance(), &settings);

    let GoldenOutcome::Failed(failure) = outcome else {
        return Err(format!("an unreadable golden is not a pass, got {outcome:?}").into());
    };
    let GoldenFailureReason::UndecodableGolden { path, cause } = &failure.reason else {
        return Err(format!(
            "expected an undecodable-golden failure, got {:?}",
            failure.reason
        )
        .into());
    };
    assert_eq!(
        path, &golden,
        "the failure names the file it could not read"
    );
    assert!(
        !cause.is_empty(),
        "the failure carries the decode error, so the reader knows what is \
         wrong with the file"
    );
    assert!(
        fs::read(&golden)? == CORRUPT,
        "a corrupt golden is never silently replaced, opt-in or no opt-in"
    );
    Ok(())
}
