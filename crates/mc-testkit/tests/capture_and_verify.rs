//! The composition root: capture a frame, then judge it against its golden.
//!
//! The golden root is a temporary directory and the frame is produced by this
//! machine's adapter, so **nothing here is ever committed**. Baking a
//! GPU-produced image into the repository would pin the project's ground truth
//! to one adapter and pre-empt the deferral of per-adapter golden variants; the
//! one committed golden in this crate is CPU-generated and stays out of this
//! test.
//!
//! The update opt-in is passed as a value. No test in this project sets an
//! environment variable — `std::env::set_var` is `unsafe` in edition 2024, and
//! an `#[allow(unsafe_code)]` in a test is exactly the escape hatch the quality
//! gate exists to make visible.

mod scene;

use std::error::Error;
use std::path::Path;

use mc_testkit::frame::gpu::capture_and_verify;
use mc_testkit::frame::{CaptureId, GoldenOutcome, GoldenSettings, OptIns, Thresholds};
use scene::TestResult;
use tempfile::TempDir;

const EDGE: u32 = 64;
const CAPTURE_NAME: &str = "capture-and-verify";

/// Opt-ins that permit writing a golden, and nothing else.
const UPDATING: OptIns = OptIns {
    allow_no_gpu: false,
    update_goldens: true,
};

#[test]
fn a_captured_frame_written_as_a_golden_matches_the_next_capture_of_that_scene() -> TestResult {
    let context = scene::device_context()?;
    let workspace = TempDir::new()?;
    let request = scene::request(&context, CAPTURE_NAME, EDGE, EDGE)?;
    let mut draw = scene::clear(scene::OPAQUE_RED);
    let updating = settings(workspace.path(), UPDATING)?;
    let verifying = settings(workspace.path(), OptIns::default())?;

    let written = capture_and_verify(&context, &request, &mut draw, &updating)?;
    let GoldenOutcome::GoldenWritten { paths } = &written else {
        return Err(format!("the update path must mint the golden, got {written:?}").into());
    };
    assert!(
        !paths.is_empty(),
        "the update path must report every golden path it wrote"
    );

    let verified = capture_and_verify(&context, &request, &mut draw, &verifying)?;
    assert!(
        matches!(verified, GoldenOutcome::Pass),
        "the same scene must match the golden it just produced, got {verified:?}"
    );
    Ok(())
}

/// Golden and artifact roots inside `workspace`, so the run reads no committed
/// file and leaves nothing behind.
fn settings(workspace: &Path, opt_ins: OptIns) -> Result<GoldenSettings, Box<dyn Error>> {
    Ok(GoldenSettings {
        golden_root: workspace.join("goldens"),
        artifact_root: workspace.join("artifacts"),
        capture: CaptureId::new(CAPTURE_NAME)?,
        thresholds: Thresholds::default(),
        opt_ins,
    })
}
