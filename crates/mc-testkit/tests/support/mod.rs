//! Fixtures for the golden-frame lifecycle.
//!
//! Deliberately separate from `common`, and deliberately self-contained: the
//! pixel-pipeline suite keeps compiling while the lifecycle is still being
//! built, so it stays a live regression signal rather than collateral damage.
//!
//! Every frame here is **split across rows**. The lifecycle writes captured
//! frames to disk and reads goldens back, so a row-order inversion is a
//! realistic bug in exactly this path — and a fixture that is symmetric down
//! the rows cannot witness one.

#![allow(dead_code)]

use std::path::{Path, PathBuf};

use mc_testkit::frame::{
    AdapterProvenance, Backend, CaptureId, GoldenSettings, ImageIoError, ImageShapeError, OptIns,
    Rgba8Image, Thresholds, write_png,
};

/// The error type every test in this suite propagates with `?`.
pub type TestResult = Result<(), Box<dyn std::error::Error>>;

/// Opt-ins with `MYCRAFT_UPDATE_GOLDENS` set, injected as a value.
///
/// Tests never set a real environment variable: `std::env::set_var` is `unsafe`
/// in edition 2024, and an `#[allow(unsafe_code)]` in a test is exactly the
/// escape hatch the quality gate exists to make visible.
pub const UPDATING: OptIns = OptIns {
    allow_no_gpu: false,
    update_goldens: true,
};

const EDGE: u32 = 64;
const OPAQUE: u8 = 255;
/// The light half, above the split.
const TOP: u8 = 200;
/// The dark half, below it.
const BOTTOM: u8 = 60;
/// Far enough from `TOP` that a single drifted pixel is past the hard ceiling.
const DRIFTED: u8 = 20;
/// How many pixels of the top-left corner the drifted frame recolours.
const DRIFTED_PIXELS: usize = 12;

const BYTES_PER_PIXEL: usize = 4;
/// The one golden filename this spec writes and reads. Variants are Out of
/// Scope; only the path *shape* leaves room for one.
const GOLDEN_IMAGE: &str = "default.png";
const GOLDEN_SIDECAR: &str = "default.provenance.json";

/// A provenance value naming no real adapter, for the tests where which adapter
/// produced a frame is beside the point.
const SYNTHETIC_ADAPTER: &str = "synthetic (cpu-generated fixture)";
const SYNTHETIC_DRIVER: &str = "no adapter; generated on the CPU by the test suite";

/// The frame the lifecycle tests treat as ground truth: a light top half over a
/// dark bottom half, so an inverted write or read is a *different* image and
/// not the same one.
///
/// # Errors
///
/// Returns [`ImageShapeError`] if the built buffer does not match the declared
/// dimensions.
pub fn reference_frame() -> Result<Rgba8Image, ImageShapeError> {
    let mut pixels = Vec::with_capacity(EDGE as usize * EDGE as usize * BYTES_PER_PIXEL);
    // `integer_division` is lint-denied workspace-wide; `midpoint` is the
    // halving that survives it.
    let midpoint = u32::midpoint(0, EDGE);
    for row in 0..EDGE {
        let level = if row < midpoint { TOP } else { BOTTOM };
        for _ in 0..EDGE {
            pixels.extend_from_slice(&[level, level, level, OPAQUE]);
        }
    }
    Rgba8Image::from_rgba(EDGE, EDGE, pixels)
}

/// [`reference_frame`] with its top-left corner drifted far past the hard
/// ceiling, so the mismatch is a verdict rather than a rounding accident.
///
/// # Errors
///
/// Returns [`ImageShapeError`] if the rebuilt buffer does not match the base
/// frame's dimensions.
pub fn drifted_frame() -> Result<Rgba8Image, ImageShapeError> {
    let base = reference_frame()?;
    let mut pixels = base.as_bytes().to_vec();
    for pixel in pixels
        .chunks_exact_mut(BYTES_PER_PIXEL)
        .take(DRIFTED_PIXELS)
    {
        pixel.copy_from_slice(&[DRIFTED, DRIFTED, DRIFTED, OPAQUE]);
    }
    Rgba8Image::from_rgba(base.width(), base.height(), pixels)
}

/// Where a capture's golden lives, spelled out rather than asked of the code
/// under test: the layout is what every future golden depends on.
#[must_use]
pub fn golden_image_path(golden_root: &Path, capture: &CaptureId) -> PathBuf {
    golden_root.join(capture.as_str()).join(GOLDEN_IMAGE)
}

/// Where that golden's provenance sidecar sits.
#[must_use]
pub fn golden_sidecar_path(golden_root: &Path, capture: &CaptureId) -> PathBuf {
    golden_root.join(capture.as_str()).join(GOLDEN_SIDECAR)
}

/// The directory a capture writes its artifacts into.
#[must_use]
pub fn artifact_dir(artifact_root: &Path, capture: &CaptureId) -> PathBuf {
    artifact_root.join(capture.as_str())
}

/// Places `frame` where the harness will look for this capture's golden.
///
/// # Errors
///
/// Returns [`ImageIoError`] if the golden cannot be written.
pub fn install_golden(
    golden_root: &Path,
    capture: &CaptureId,
    frame: &Rgba8Image,
) -> Result<PathBuf, ImageIoError> {
    let path = golden_image_path(golden_root, capture);
    write_png(frame, &path)?;
    Ok(path)
}

/// Settings pointing at throwaway roots, with the harness's default thresholds.
#[must_use]
pub fn golden_settings(
    golden_root: &Path,
    artifact_root: &Path,
    capture: CaptureId,
    opt_ins: OptIns,
) -> GoldenSettings {
    GoldenSettings {
        golden_root: golden_root.to_path_buf(),
        artifact_root: artifact_root.to_path_buf(),
        capture,
        thresholds: Thresholds::default(),
        opt_ins,
    }
}

/// Provenance for a frame that came from no adapter at all.
#[must_use]
pub fn synthetic_provenance() -> AdapterProvenance {
    AdapterProvenance::new(SYNTHETIC_ADAPTER, Backend::Other, Some(SYNTHETIC_DRIVER))
}
