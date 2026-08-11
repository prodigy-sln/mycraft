//! A capture that does not match its golden: the artifact set, and where it is.
//!
//! The fixtures are split across rows, so a frame written upside-down into
//! `expected.png` or `actual.png` is a different file rather than the same one.
//! A capture path that inverted rows would make every golden this project ever
//! commits wrong in the same direction, consistently and therefore invisibly.

mod support;

use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use mc_testkit::frame::{
    ArtifactError, CaptureId, GoldenFailure, GoldenFailureReason, GoldenOutcome, OptIns,
    verify_against_golden,
};
use support::{
    TestResult, artifact_dir, drifted_frame, golden_settings, install_golden, reference_frame,
    synthetic_provenance,
};
use tempfile::TempDir;

const CAPTURE: &str = "clear-red-64";
/// The first artifact written, and so the first write that can fail.
const EXPECTED_FRAME: &str = "expected.png";
const ARTIFACT_SET: [&str; 4] = [EXPECTED_FRAME, "actual.png", "diff.png", "report.json"];

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
fn an_artifact_set_that_cannot_reach_disk_still_reports_the_mismatch() -> TestResult {
    for obstruction in [
        Obstruction::UncreatableDirectory,
        Obstruction::UnwritableFile,
    ] {
        let goldens = TempDir::new()?;
        let workspace = TempDir::new()?;
        let capture = CaptureId::new(CAPTURE)?;
        let (artifact_root, obstructed) = obstruction.place(workspace.path(), &capture)?;
        install_golden(goldens.path(), &capture, &reference_frame()?)?;
        let settings = golden_settings(goldens.path(), &artifact_root, capture, OptIns::default());

        let outcome = verify_against_golden(&drifted_frame()?, &synthetic_provenance(), &settings);

        let GoldenOutcome::Failed(failure) = outcome else {
            return Err(
                format!("the frames still differ, whatever disk did, got {outcome:?}").into(),
            );
        };
        let artifact_error = artifact_failure_under(&failure, obstruction)?;
        assert_reported_alongside_the_verdict(&failure, artifact_error, &obstructed)?;
    }
    Ok(())
}

/// The two ways the artifact set fails to reach disk.
///
/// `spec.md` names both — "cannot be created **or** written" — and they fail at
/// different points in the write path, so provoking only the first leaves the
/// second unobserved. Neither needs a permission bit, so both work on any
/// platform.
#[derive(Clone, Copy, Debug)]
enum Obstruction {
    /// A plain file standing where the artifact root's parent would have to be
    /// a directory: nothing beneath it can be created.
    UncreatableDirectory,
    /// The artifact directory is created without trouble and the first artifact
    /// is a directory, so the *write* into it is what fails.
    UnwritableFile,
}

impl Obstruction {
    /// Sets the obstruction up under `workspace`, and returns the artifact root
    /// to run against together with the path the failure has to name.
    ///
    /// # Errors
    ///
    /// Returns the filesystem error if the obstruction cannot be placed.
    fn place(
        self,
        workspace: &Path,
        capture: &CaptureId,
    ) -> Result<(PathBuf, PathBuf), Box<dyn Error>> {
        match self {
            Self::UncreatableDirectory => {
                let blocker = workspace.join("occupied");
                fs::write(&blocker, b"a file, not a directory")?;
                let root = blocker.join("mycraft-frames");
                let directory = artifact_dir(&root, capture);
                Ok((root, directory))
            }
            Self::UnwritableFile => {
                let root = workspace.join("mycraft-frames");
                let expected = artifact_dir(&root, capture).join(EXPECTED_FRAME);
                fs::create_dir_all(&expected)?;
                Ok((root, expected))
            }
        }
    }

    /// Whether `error` is the failure this obstruction was aimed at, rather
    /// than some other one that would make the run prove something else.
    fn explains(self, error: &ArtifactError) -> bool {
        match self {
            Self::UncreatableDirectory => matches!(error, ArtifactError::Directory { .. }),
            Self::UnwritableFile => matches!(error, ArtifactError::Image { .. }),
        }
    }
}

/// The artifact failure recorded under a verdict that still stands.
///
/// # Errors
///
/// Returns a message if the verdict was swallowed, if the artifact set reached
/// disk anyway, or if the run failed somewhere other than where it was aimed.
fn artifact_failure_under(
    failure: &GoldenFailure,
    obstruction: Obstruction,
) -> Result<&ArtifactError, Box<dyn Error>> {
    if !matches!(failure.reason, GoldenFailureReason::Mismatch(_)) {
        return Err(format!(
            "an artifact failure must not swallow the verdict it was recording, got {:?}",
            failure.reason
        )
        .into());
    }
    let artifact_error = failure
        .artifacts
        .as_ref()
        .err()
        .ok_or_else(|| format!("{obstruction:?} must stop the artifact set reaching disk"))?;
    if !obstruction.explains(artifact_error) {
        return Err(
            format!("{obstruction:?} failed somewhere else instead: {artifact_error:?}").into(),
        );
    }
    Ok(artifact_error)
}

/// Everything the reader is owed on one line: the verdict, the artifact failure
/// named against the path it actually happened to, and the cause underneath it.
fn assert_reported_alongside_the_verdict(
    failure: &GoldenFailure,
    artifact_error: &ArtifactError,
    obstructed: &Path,
) -> TestResult {
    let cause = artifact_error
        .source()
        .ok_or("the artifact failure must carry its underlying cause")?;
    let reported = failure.to_string();
    assert!(
        reported.contains(&failure.reason.to_string()),
        "the mismatch has to survive the write that failed; got `{reported}`"
    );
    assert!(
        names_whole_path(&reported, obstructed),
        "`{}` must be named as a whole path of its own — not a longer one that \
         merely contains it, and not the directory holding it; got `{reported}`",
        obstructed.display()
    );
    assert!(
        reported.contains(&cause.to_string()),
        "the underlying cause `{cause}` must be named alongside the verdict; \
         got `{reported}`"
    );
    Ok(())
}

/// Whether `message` names `path` as a whole path rather than only as the
/// prefix of a longer one.
///
/// A directory is a substring of every file inside it, so a plain `contains`
/// cannot tell "the failure named the directory" from "the failure named a file
/// that happens to live in it" — and stays green for an implementation that
/// never mentions the directory at all.
fn names_whole_path(message: &str, path: &Path) -> bool {
    let rendered = path.display().to_string();
    message.match_indices(&rendered).any(|(at, _)| {
        message
            .get(at.saturating_add(rendered.len())..)
            .is_none_or(|tail| !tail.starts_with(['/', '\\']))
    })
}
