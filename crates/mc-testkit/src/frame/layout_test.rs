//! Where a capture's golden and its artifacts live on disk.
//!
//! The layout is the one thing in this spec that is expensive to reverse: every
//! golden this project ever commits is found through it, and a variant added
//! later must be a new file rather than a migration of the whole set. These
//! assertions therefore spell the shape out literally instead of asking the
//! constructors under test what they think it is — a relocated file has to be a
//! failing test, not a silent move.

use std::path::Path;

use super::{ArtifactPaths, CaptureId, CaptureIdError, GoldenPaths};

const CAPTURE: &str = "clear-red-64";
const GOLDEN_ROOT: &str = "goldens";
const ARTIFACT_ROOT: &str = "mycraft-frames";

fn capture() -> Result<CaptureId, CaptureIdError> {
    CaptureId::new(CAPTURE)
}

#[test]
fn a_golden_is_the_default_image_inside_a_directory_named_for_its_capture()
-> Result<(), CaptureIdError> {
    let root = Path::new(GOLDEN_ROOT);

    let paths = GoldenPaths::new(root, &capture()?);

    assert_eq!(
        paths.image(),
        root.join(CAPTURE).join("default.png"),
        "a directory per capture is what lets an adapter variant be a new file"
    );
    Ok(())
}

#[test]
fn a_goldens_provenance_sidecar_sits_beside_it() -> Result<(), CaptureIdError> {
    let root = Path::new(GOLDEN_ROOT);

    let paths = GoldenPaths::new(root, &capture()?);

    assert_eq!(
        paths.provenance(),
        root.join(CAPTURE).join("default.provenance.json"),
        "which adapter produced a golden travels next to the golden"
    );
    Ok(())
}

#[test]
fn a_captures_four_artifacts_share_a_directory_named_for_it() -> Result<(), CaptureIdError> {
    let root = Path::new(ARTIFACT_ROOT);
    let directory = root.join(CAPTURE);

    let paths = ArtifactPaths::new(root, &capture()?);

    assert_eq!(
        [
            paths.directory().to_path_buf(),
            paths.expected(),
            paths.actual(),
            paths.diff(),
            paths.report(),
        ],
        [
            directory.clone(),
            directory.join("expected.png"),
            directory.join("actual.png"),
            directory.join("diff.png"),
            directory.join("report.json"),
        ],
        "the directory carries the capture's name and nothing run-specific, so \
         today's pass can find yesterday's stale files"
    );
    Ok(())
}
