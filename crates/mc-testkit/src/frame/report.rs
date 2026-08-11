//! The machine-readable record of a verdict, and of every golden that is written.
//!
//! Both files exist for a reader who cannot look at a screen. The mismatch
//! report answers "what drifted, by how much, against which thresholds, on which
//! adapter" without re-running anything; the golden sidecar answers "which
//! adapter produced this ground truth", which is what makes the deferred
//! per-adapter golden variants an exercise in adding files rather than
//! migrating them.

use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;
use thiserror::Error;

use super::compare::{Comparison, MismatchReason, Thresholds, Verdict};
use super::layout::CaptureId;

/// What an adapter that reports no driver description is recorded as.
///
/// The field is always present: omitting it would leave a reader unable to tell
/// "the adapter did not say" from "nobody looked".
const UNKNOWN_DRIVER: &str = "unknown";

/// A graphics backend, as far as this crate is concerned.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Backend {
    Vulkan,
    Dx12,
    Metal,
    Gl,
    #[serde(rename = "browser_webgpu")]
    BrowserWebGpu,
    Other,
}

/// Which adapter produced a frame.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AdapterProvenance {
    pub name: String,
    pub backend: Backend,
    pub driver_description: String,
}

impl AdapterProvenance {
    /// Records an adapter, normalising a missing driver description to the
    /// literal `unknown`.
    ///
    /// The normalisation is a pure core function rather than something the GPU
    /// layer does inline, because the scenario that asserts it is one of the
    /// ones that must hold without hardware.
    #[must_use]
    pub fn new(name: &str, backend: Backend, driver_description: Option<&str>) -> Self {
        let described = driver_description
            .map(str::trim)
            .filter(|described| !described.is_empty())
            .unwrap_or(UNKNOWN_DRIVER);
        Self {
            name: name.to_owned(),
            backend,
            driver_description: described.to_owned(),
        }
    }
}

/// A report, a golden sidecar, or the directory holding one, that could not be
/// produced.
#[derive(Debug, Error)]
pub enum ReportError {
    #[error("could not serialise the report for the capture `{capture}`")]
    Serialize {
        capture: String,
        #[source]
        cause: serde_json::Error,
    },
    #[error("could not create the directory `{path}` to hold a report")]
    Directory {
        path: PathBuf,
        #[source]
        cause: std::io::Error,
    },
    #[error("could not write the report `{path}`")]
    Write {
        path: PathBuf,
        #[source]
        cause: std::io::Error,
    },
}

/// The three thresholds a verdict was judged against.
#[derive(Debug, Clone, Copy, Serialize)]
struct ReportThresholds {
    per_pixel_delta_e: f64,
    max_failing_fraction: f64,
    hard_ceiling_delta_e: f64,
}

impl From<&Thresholds> for ReportThresholds {
    fn from(thresholds: &Thresholds) -> Self {
        Self {
            per_pixel_delta_e: thresholds.per_pixel_delta_e(),
            max_failing_fraction: thresholds.max_failing_fraction(),
            hard_ceiling_delta_e: thresholds.hard_ceiling_delta_e(),
        }
    }
}

/// Which files accompany the report, and why one of them does not.
#[derive(Debug, Clone, Serialize)]
struct ReportArtifacts {
    expected: &'static str,
    actual: &'static str,
    diff: Option<&'static str>,
    diff_omitted_reason: Option<String>,
}

/// The record of one comparison, as it lands on disk beside the images.
#[derive(Debug, Clone, Serialize)]
pub struct FrameReport {
    capture: String,
    verdict: &'static str,
    reason: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    expected_size: Option<[u32; 2]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    actual_size: Option<[u32; 2]>,
    thresholds: ReportThresholds,
    failing_pixels: u64,
    total_pixels: u64,
    failing_fraction: f64,
    max_delta_e: f64,
    adapter: AdapterProvenance,
    artifacts: ReportArtifacts,
}

impl FrameReport {
    /// Builds the report for one comparison.
    ///
    /// `diff_omitted_reason` is `Some` when no diff image accompanies the
    /// report, which happens when the two frames had different sizes and so
    /// share no positions to diff.
    #[must_use]
    pub fn new(
        capture: &CaptureId,
        comparison: &Comparison,
        provenance: &AdapterProvenance,
        diff_omitted_reason: Option<&str>,
    ) -> Self {
        let (verdict, reason) = describe_verdict(comparison.verdict);
        let (expected_size, actual_size) = compared_sizes(comparison.verdict);
        Self {
            capture: capture.as_str().to_owned(),
            verdict,
            reason,
            expected_size,
            actual_size,
            thresholds: (&comparison.thresholds).into(),
            failing_pixels: comparison.failing_pixels,
            total_pixels: comparison.total_pixels,
            failing_fraction: comparison.failing_fraction,
            max_delta_e: comparison.max_delta_e,
            adapter: provenance.clone(),
            artifacts: ReportArtifacts {
                expected: "expected.png",
                actual: "actual.png",
                diff: diff_omitted_reason.map_or(Some("diff.png"), |_| None),
                diff_omitted_reason: diff_omitted_reason.map(str::to_owned),
            },
        }
    }

    /// Renders the report as JSON.
    ///
    /// # Errors
    ///
    /// Returns [`ReportError::Serialize`] if the report cannot be rendered.
    pub fn to_json(&self) -> Result<String, ReportError> {
        serde_json::to_string_pretty(self).map_err(|cause| ReportError::Serialize {
            capture: self.capture.clone(),
            cause,
        })
    }
}

/// The verdict and its reason, as the two JSON strings that carry them.
const fn describe_verdict(verdict: Verdict) -> (&'static str, Option<&'static str>) {
    match verdict {
        Verdict::Match => ("match", None),
        Verdict::Mismatch(MismatchReason::AreaBudget) => ("mismatch", Some("area_budget")),
        Verdict::Mismatch(MismatchReason::HardCeiling) => ("mismatch", Some("hard_ceiling")),
        Verdict::Mismatch(MismatchReason::Dimensions { .. }) => ("mismatch", Some("dimensions")),
    }
}

/// The two frame sizes, recorded only when the difference between them *is* the
/// reason for the verdict.
const fn compared_sizes(verdict: Verdict) -> (Option<[u32; 2]>, Option<[u32; 2]>) {
    match verdict {
        Verdict::Mismatch(MismatchReason::Dimensions { expected, actual }) => {
            (Some([expected.0, expected.1]), Some([actual.0, actual.1]))
        }
        _ => (None, None),
    }
}

/// The record written beside a golden: which adapter produced it.
#[derive(Debug, Clone, Serialize)]
struct GoldenProvenance<'a> {
    capture: &'a str,
    adapter: &'a str,
    backend: Backend,
    driver_description: &'a str,
}

/// Writes a mismatch report to `path`, creating its directory if needed.
///
/// # Errors
///
/// Returns [`ReportError::Serialize`] if the report cannot be rendered,
/// [`ReportError::Directory`] if its directory cannot be created, or
/// [`ReportError::Write`] if the file cannot be written.
pub fn write_report(report: &FrameReport, path: &Path) -> Result<(), ReportError> {
    write_json(&report.to_json()?, path)
}

/// Writes the provenance sidecar that travels with a golden.
///
/// # Errors
///
/// Returns [`ReportError::Serialize`], [`ReportError::Directory`] or
/// [`ReportError::Write`] as [`write_report`] does.
pub(crate) fn write_golden_provenance(
    capture: &CaptureId,
    provenance: &AdapterProvenance,
    path: &Path,
) -> Result<(), ReportError> {
    let sidecar = GoldenProvenance {
        capture: capture.as_str(),
        adapter: &provenance.name,
        backend: provenance.backend,
        driver_description: &provenance.driver_description,
    };
    let rendered =
        serde_json::to_string_pretty(&sidecar).map_err(|cause| ReportError::Serialize {
            capture: capture.as_str().to_owned(),
            cause,
        })?;
    write_json(&rendered, path)
}

/// Writes rendered JSON to `path`, creating the parent directory if needed.
fn write_json(rendered: &str, path: &Path) -> Result<(), ReportError> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent).map_err(|cause| ReportError::Directory {
            path: parent.to_path_buf(),
            cause,
        })?;
    }
    fs::write(path, rendered).map_err(|cause| ReportError::Write {
        path: path.to_path_buf(),
        cause,
    })
}
