//! The golden-frame lifecycle: judge a frame against committed ground truth,
//! and leave the right evidence behind.
//!
//! Two rules shape everything here.
//!
//! **A broken renderer never mints its own ground truth.** A golden is created
//! or overwritten only when someone explicitly asks, and a golden that cannot be
//! read is never silently replaced.
//!
//! **Artifacts are evidence of a failure, so they are cleared on every path that
//! is not one.** A directory still holding last run's `diff.png` after today's
//! pass is worse than an empty one, because the reader has no way to tell it is
//! stale. Deletion is by an explicit filename allowlist and never recursive: the
//! artifact root is caller-supplied, so `remove_dir_all` on it would be a
//! foot-gun aimed at whatever they passed.

use std::fs;
use std::path::{Path, PathBuf};

use thiserror::Error;

use super::compare::{Comparison, Verdict, compare};
use super::diff::render_diff;
use super::image::Rgba8Image;
use super::layout::{ArtifactPaths, GoldenPaths, GoldenSettings};
use super::png::{ImageIoError, read_png, write_png};
use super::report::{
    AdapterProvenance, FrameReport, ReportError, write_golden_provenance, write_report,
};

/// Why no diff image accompanies a report.
const DIFF_OMITTED: &str = "the frames are different sizes, so they share no positions to diff";

/// What a run of the lifecycle concluded.
#[derive(Debug)]
pub enum GoldenOutcome {
    /// The frame matched its golden.
    Pass,
    /// The update opt-in was set, and the golden already matched.
    GoldenUnchanged,
    /// The update opt-in was set, and these paths were written.
    GoldenWritten {
        paths: Vec<PathBuf>,
    },
    Failed(GoldenFailure),
}

/// What was wrong.
#[derive(Debug, Error)]
pub enum GoldenFailureReason {
    #[error("no golden exists at `{}`", path.display())]
    MissingGolden { path: PathBuf },
    #[error("the golden `{}` could not be read: {cause}", path.display())]
    UndecodableGolden { path: PathBuf, cause: String },
    #[error(
        "the capture differs from its golden: {} of {} pixels past the per-pixel \
         tolerance, worst distance {:.3}",
        .0.failing_pixels, .0.total_pixels, .0.max_delta_e
    )]
    Mismatch(Comparison),
}

/// A failed verification, and where to look for what it left behind.
///
/// `Display` is written by hand rather than derived because a write that failed
/// has to reach the reader. Deriving `{reason}; artifacts in {dir}` printed the
/// standing verdict and a directory, and said nothing about the write — which on
/// the update path meant a caller was shown "no golden exists" with no hint that
/// the golden they asked for had not been written.
#[derive(Debug, Error)]
pub struct GoldenFailure {
    pub reason: GoldenFailureReason,
    pub artifact_dir: PathBuf,
    /// The paths written, or the failure to write them.
    ///
    /// An `Err` here never replaces the verdict: a disk that would not take the
    /// evidence does not make the frames match.
    pub artifacts: Result<Vec<PathBuf>, ArtifactError>,
}

impl std::fmt::Display for GoldenFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.artifacts {
            Ok(_) => write!(
                formatter,
                "{}; artifacts in `{}`",
                self.reason,
                self.artifact_dir.display()
            ),
            // The whole cause chain, not just its head: `ArtifactError`'s own
            // `Display` names the file it could not write, and the reason it
            // could not is one link further down.
            Err(cause) => write!(formatter, "{}; {}", self.reason, describe(cause)),
        }
    }
}

/// An artifact, or the directory holding it, that could not be written.
#[derive(Debug, Error)]
pub enum ArtifactError {
    #[error("could not create the artifact directory `{}`", path.display())]
    Directory {
        path: PathBuf,
        #[source]
        cause: std::io::Error,
    },
    /// Boxed because an image error carries a decoder's own error type, which
    /// is large enough that inlining it would make every `Result` in this
    /// module pay for the failure path.
    #[error("could not write the image `{}`", path.display())]
    Image {
        path: PathBuf,
        #[source]
        cause: Box<ImageIoError>,
    },
    #[error("could not write the report `{}`", path.display())]
    Report {
        path: PathBuf,
        #[source]
        cause: ReportError,
    },
    /// The update path could not write the golden itself.
    ///
    /// Distinct from [`Self::Image`], and worded the way it is, because of who
    /// reads it. The verdict reported alongside this is the one that **still
    /// stands** — "no golden exists at X", or "the capture differs from its
    /// golden" — and a caller who set `MYCRAFT_UPDATE_GOLDENS` would otherwise
    /// read that verdict as the state the update had just fixed, and walk into
    /// the same wall on the next run. So the error says outright that it did
    /// not happen.
    ///
    /// It wraps only the **image** write. A sidecar that fails after the golden
    /// landed is [`Self::Report`], because in that case the golden *was*
    /// updated and claiming otherwise would be false.
    /// The path is deliberately not repeated here: the wrapped cause names the
    /// file it could not write, one link down the chain.
    #[error("the golden was NOT updated")]
    GoldenNotUpdated {
        path: PathBuf,
        #[source]
        cause: Box<ArtifactError>,
    },
}

/// Verifies a frame against its committed golden.
///
/// Fully GPU-free: it takes the frame and its provenance as values, so every
/// branch below is reachable in a process that holds no adapter.
#[must_use]
pub fn verify_against_golden(
    captured: &Rgba8Image,
    provenance: &AdapterProvenance,
    settings: &GoldenSettings,
) -> GoldenOutcome {
    Lifecycle {
        captured,
        provenance,
        settings,
        goldens: GoldenPaths::new(&settings.golden_root, &settings.capture),
        artifacts: ArtifactPaths::new(&settings.artifact_root, &settings.capture),
    }
    .run()
}

/// What was found where the golden should be.
enum GoldenState {
    Missing,
    Undecodable(String),
    Present(Box<Rgba8Image>),
}

/// One run of the lifecycle, with its inputs and its two path sets gathered so
/// that each step below takes no arguments beyond them.
struct Lifecycle<'a> {
    captured: &'a Rgba8Image,
    provenance: &'a AdapterProvenance,
    settings: &'a GoldenSettings,
    goldens: GoldenPaths,
    artifacts: ArtifactPaths,
}

impl Lifecycle<'_> {
    fn run(&self) -> GoldenOutcome {
        match read_golden(&self.goldens.image()) {
            GoldenState::Missing => self.on_missing(),
            GoldenState::Undecodable(cause) => self.on_undecodable(cause),
            GoldenState::Present(golden) => self.on_present(&golden),
        }
    }

    /// No golden yet. The run that notices one is missing is never the run that
    /// creates it — unless that is exactly what was asked for.
    fn on_missing(&self) -> GoldenOutcome {
        if self.settings.opt_ins.update_goldens {
            return self.on_update(GoldenFailureReason::MissingGolden {
                path: self.goldens.image(),
            });
        }
        self.clear_artifacts();
        let artifacts = self.write_captured_frame();
        self.fail(
            GoldenFailureReason::MissingGolden {
                path: self.goldens.image(),
            },
            artifacts,
        )
    }

    /// A golden that cannot be read is a defect in the golden set, and is never
    /// replaced on the strength of a frame that could not be compared to it.
    fn on_undecodable(&self, cause: String) -> GoldenOutcome {
        self.clear_artifacts();
        self.fail(
            GoldenFailureReason::UndecodableGolden {
                path: self.goldens.image(),
                cause,
            },
            Ok(Vec::new()),
        )
    }

    fn on_present(&self, golden: &Rgba8Image) -> GoldenOutcome {
        let comparison = compare(golden, self.captured, &self.settings.thresholds);
        match (comparison.verdict, self.settings.opt_ins.update_goldens) {
            (Verdict::Match, false) => {
                self.clear_artifacts();
                GoldenOutcome::Pass
            }
            (Verdict::Match, true) => {
                self.clear_artifacts();
                GoldenOutcome::GoldenUnchanged
            }
            (Verdict::Mismatch(_), true) => {
                self.on_update(GoldenFailureReason::Mismatch(comparison))
            }
            (Verdict::Mismatch(_), false) => self.on_mismatch(golden, comparison),
        }
    }

    /// Rewrites the golden and its sidecar. The mismatch artifact set is
    /// deliberately **not** written: recording a failure against ground truth
    /// that has just been replaced would document a comparison nobody made.
    ///
    /// `standing` is the verdict that still holds if the write itself fails —
    /// a golden that could not be written is still missing, or still wrong.
    fn on_update(&self, standing: GoldenFailureReason) -> GoldenOutcome {
        self.clear_artifacts();
        match self.write_golden() {
            Ok(paths) => GoldenOutcome::GoldenWritten { paths },
            Err(cause) => self.fail(standing, Err(cause)),
        }
    }

    fn on_mismatch(&self, golden: &Rgba8Image, comparison: Comparison) -> GoldenOutcome {
        self.clear_artifacts();
        let artifacts = self.write_artifact_set(golden, &comparison);
        self.fail(GoldenFailureReason::Mismatch(comparison), artifacts)
    }

    /// The golden and its provenance sidecar, reported together: an unexplained
    /// golden update is a review stop, so every path written has to be named.
    ///
    /// Only the image write is wrapped as [`ArtifactError::GoldenNotUpdated`].
    /// Past that line the golden is on disk, so a sidecar that then fails leaves
    /// a golden that *was* updated and a provenance record that was not — two
    /// different things to tell a reader.
    fn write_golden(&self) -> Result<Vec<PathBuf>, ArtifactError> {
        let path = self.goldens.image();
        let image = write_frame(self.captured, path.clone()).map_err(|cause| {
            ArtifactError::GoldenNotUpdated {
                path,
                cause: Box::new(cause),
            }
        })?;
        let sidecar = self.goldens.provenance();
        write_golden_provenance(&self.settings.capture, self.provenance, &sidecar).map_err(
            |cause| ArtifactError::Report {
                path: sidecar.clone(),
                cause,
            },
        )?;
        Ok(vec![image, sidecar])
    }

    /// The captured frame alone. A missing golden asks for the image that was
    /// produced, not for a diff against something that does not exist.
    fn write_captured_frame(&self) -> Result<Vec<PathBuf>, ArtifactError> {
        self.create_artifact_directory()?;
        Ok(vec![write_frame(self.captured, self.artifacts.actual())?])
    }

    fn write_artifact_set(
        &self,
        golden: &Rgba8Image,
        comparison: &Comparison,
    ) -> Result<Vec<PathBuf>, ArtifactError> {
        self.create_artifact_directory()?;
        // Each path joins the list only once its file is on disk, so a partial
        // failure never reports a file that is not there.
        let mut written = vec![
            write_frame(golden, self.artifacts.expected())?,
            write_frame(self.captured, self.artifacts.actual())?,
        ];

        let diff = render_diff(golden, comparison);
        if let Some(rendered) = &diff {
            written.push(write_frame(rendered, self.artifacts.diff())?);
        }

        let report = FrameReport::new(
            &self.settings.capture,
            comparison,
            self.provenance,
            diff.is_none().then_some(DIFF_OMITTED),
        );
        let path = self.artifacts.report();
        write_report(&report, &path).map_err(|cause| ArtifactError::Report {
            path: path.clone(),
            cause,
        })?;
        written.push(path);
        Ok(written)
    }

    fn create_artifact_directory(&self) -> Result<(), ArtifactError> {
        fs::create_dir_all(self.artifacts.directory()).map_err(|cause| ArtifactError::Directory {
            path: self.artifacts.directory().to_path_buf(),
            cause,
        })
    }

    /// Removes this capture's artifacts by name. Never recursive, and never
    /// anything outside the four files this crate writes.
    fn clear_artifacts(&self) {
        for path in [
            self.artifacts.expected(),
            self.artifacts.actual(),
            self.artifacts.diff(),
            self.artifacts.report(),
        ] {
            remove_if_present(&path);
        }
    }

    fn fail(
        &self,
        reason: GoldenFailureReason,
        artifacts: Result<Vec<PathBuf>, ArtifactError>,
    ) -> GoldenOutcome {
        GoldenOutcome::Failed(GoldenFailure {
            reason,
            artifact_dir: self.artifacts.directory().to_path_buf(),
            artifacts,
        })
    }
}

/// Reads what is at `path`, distinguishing "no golden yet" from "a golden that
/// is not readable" — the two call for opposite responses.
fn read_golden(path: &Path) -> GoldenState {
    if matches!(path.try_exists(), Ok(false)) {
        return GoldenState::Missing;
    }
    match read_png(path) {
        Ok(golden) => GoldenState::Present(Box::new(golden)),
        Err(cause) => GoldenState::Undecodable(describe(&cause)),
    }
}

/// Writes `frame` to `path` and reports where it landed, so a caller
/// accumulating written paths only ever names files that are on disk.
fn write_frame(frame: &Rgba8Image, path: PathBuf) -> Result<PathBuf, ArtifactError> {
    write_png(frame, &path).map_err(|cause| ArtifactError::Image {
        path: path.clone(),
        cause: Box::new(cause),
    })?;
    Ok(path)
}

/// Removes `path` if it is there.
///
/// Best effort by design: recovery from a failed deletion is Out of Scope
/// (audit #22). The consequence of one is a stale artifact beside a fresh
/// verdict, never a wrong verdict.
fn remove_if_present(path: &Path) {
    if fs::remove_file(path).is_err() {
        // Nothing to do, and deliberately not written `let _ = ...`: swallowing
        // a `must_use` is the habit this shape exists to avoid teaching.
    }
}

/// An error and everything underneath it, on one line.
///
/// A `Display` alone would say "could not decode `x` as a PNG" without saying
/// what is wrong with the file, which is the half a reader needs.
fn describe(error: &dyn std::error::Error) -> String {
    let mut described = error.to_string();
    let mut next = error.source();
    while let Some(cause) = next {
        described.push_str(": ");
        described.push_str(&cause.to_string());
        next = cause.source();
    }
    described
}
