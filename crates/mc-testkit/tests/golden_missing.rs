//! Goldens that are absent or unreadable.
//!
//! A broken renderer must never mint its own ground truth. Both cases here fail
//! loudly and leave the golden set exactly as it was, which is what keeps a
//! green suite meaningful: the only way a golden changes is somebody asking for
//! it.

mod support;

use std::error::Error;
use std::fs;
use std::path::PathBuf;

use mc_testkit::frame::{
    CaptureId, GoldenFailure, GoldenFailureReason, GoldenOutcome, GoldenSettings, OptIns, read_png,
    verify_against_golden,
};
use support::{
    TestResult, UPDATING, artifact_dir, golden_image_path, golden_settings, reference_frame,
    synthetic_provenance,
};
use tempfile::TempDir;

const CAPTURE: &str = "clear-red-64";
/// Not a PNG by any reading of the signature.
const CORRUPT: &[u8] = b"this was a golden once";

/// Throwaway golden and artifact roots for one capture.
///
/// Both scenarios in this file are about the *state of the golden set* rather
/// than about how the roots were arranged, so the arranging is named once here
/// and each test says only what makes it different from the others.
#[derive(Debug)]
struct Workspace {
    goldens: TempDir,
    artifacts: TempDir,
    capture: CaptureId,
}

impl Workspace {
    fn new() -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            goldens: TempDir::new()?,
            artifacts: TempDir::new()?,
            capture: CaptureId::new(CAPTURE)?,
        })
    }

    fn settings(&self, opt_ins: OptIns) -> GoldenSettings {
        golden_settings(
            self.goldens.path(),
            self.artifacts.path(),
            self.capture.clone(),
            opt_ins,
        )
    }

    /// Where this capture's golden belongs, whether or not it is there.
    fn golden(&self) -> PathBuf {
        golden_image_path(self.goldens.path(), &self.capture)
    }

    /// Where a run writes the frame it captured.
    fn written_frame(&self) -> PathBuf {
        artifact_dir(self.artifacts.path(), &self.capture).join("actual.png")
    }

    /// Puts something that is not a PNG where the golden belongs.
    fn plant_corrupt_golden(&self) -> Result<PathBuf, Box<dyn Error>> {
        let golden = self.golden();
        fs::create_dir_all(
            golden
                .parent()
                .ok_or("a golden path always has a directory")?,
        )?;
        fs::write(&golden, CORRUPT)?;
        Ok(golden)
    }
}

/// The failure inside an outcome, or a message naming what came back instead.
fn failure_of(outcome: GoldenOutcome) -> Result<GoldenFailure, String> {
    match outcome {
        GoldenOutcome::Failed(failure) => Ok(failure),
        other => Err(format!("expected a failure, got {other:?}")),
    }
}

#[test]
fn a_missing_golden_fails_naming_the_path_and_is_not_created() -> TestResult {
    let workspace = Workspace::new()?;
    let settings = workspace.settings(OptIns::default());

    let outcome = verify_against_golden(&reference_frame()?, &synthetic_provenance(), &settings);

    let failure = failure_of(outcome)?;
    let GoldenFailureReason::MissingGolden { path } = &failure.reason else {
        return Err(format!("expected a missing golden, got {:?}", failure.reason).into());
    };
    assert_eq!(
        path,
        &workspace.golden(),
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
    let workspace = Workspace::new()?;
    let settings = workspace.settings(OptIns::default());
    // Split across rows, so a frame written upside-down is a different file.
    let captured = reference_frame()?;

    let outcome = verify_against_golden(&captured, &synthetic_provenance(), &settings);

    assert!(
        matches!(outcome, GoldenOutcome::Failed(_)),
        "a missing golden is still a failure, got {outcome:?}"
    );
    let written = read_png(&workspace.written_frame())?;
    assert!(
        written.as_bytes() == captured.as_bytes(),
        "the artifact must be the frame that was captured, the same way up — \
         it is the only record of what the run actually produced"
    );
    Ok(())
}

#[test]
fn a_golden_that_is_not_a_decodable_png_fails_without_being_replaced() -> TestResult {
    let workspace = Workspace::new()?;
    let golden = workspace.plant_corrupt_golden()?;
    // The update opt-in is deliberately set: "must not overwrite" is only a
    // restraint where overwriting is otherwise permitted.
    let settings = workspace.settings(UPDATING);

    let outcome = verify_against_golden(&reference_frame()?, &synthetic_provenance(), &settings);

    let failure = failure_of(outcome)?;
    let GoldenFailureReason::UndecodableGolden { path, cause } = &failure.reason else {
        return Err(format!("expected an undecodable golden, got {:?}", failure.reason).into());
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
