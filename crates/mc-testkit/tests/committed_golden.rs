//! The one golden this spec commits, read from its real path in the repository.
//!
//! Every other lifecycle test writes its golden into a `TempDir` moments before
//! reading it back, which never exercises the part that actually breaks: bytes
//! that went through git — checkout, line-ending policy, a `.gitattributes`
//! that may or may not exist — and came back. Running that round trip for the
//! first time in the renderer spec would make a failure ambiguous between a
//! wrong renderer and a wrong golden workflow, which is exactly the ambiguity
//! this harness exists to remove.
//!
//! The fixture is **generated on the CPU**, never captured. A GPU-produced
//! golden would bake this machine's adapter into the repository and pre-empt
//! the per-adapter-golden deferral, and its sidecar says so rather than naming
//! an adapter that never rendered it.

mod support;

use std::path::PathBuf;

use mc_testkit::frame::{
    CaptureId, GoldenOutcome, ImageShapeError, OptIns, Rgba8Image, verify_against_golden,
};
use support::{TestResult, UPDATING, golden_settings, synthetic_provenance};
use tempfile::TempDir;

const CAPTURE: &str = "synthetic-gradient-64";
const EDGE: u32 = 64;
const BYTES_PER_PIXEL: usize = 4;
/// A fixed blue channel, so the frame is unmistakably synthetic rather than
/// something that could have come off a render target by accident.
const BLUE: u8 = 128;
const OPAQUE: u8 = 255;
/// Spreads 64 steps across the whole 0–252 range, so neighbouring rows and
/// columns are perceptually distinct rather than a rounding apart.
const STEP: u8 = 4;

/// The committed golden's pixels, from first principles.
///
/// Every channel is a function of the pixel's own coordinates and nothing else,
/// so the file is byte-reproducible from this function alone — a contributor
/// can rebuild the golden rather than take it on trust. Green varies down the
/// rows and red across the columns, so neither a row inversion nor a
/// transposition survives the comparison.
fn synthetic_frame() -> Result<Rgba8Image, ImageShapeError> {
    let mut pixels = Vec::with_capacity(EDGE as usize * EDGE as usize * BYTES_PER_PIXEL);
    for row in 0..EDGE {
        let green = u8::try_from(row).unwrap_or(u8::MAX).wrapping_mul(STEP);
        for column in 0..EDGE {
            let red = u8::try_from(column).unwrap_or(u8::MAX).wrapping_mul(STEP);
            pixels.extend_from_slice(&[red, green, BLUE, OPAQUE]);
        }
    }
    Rgba8Image::from_rgba(EDGE, EDGE, pixels)
}

/// The golden set's home. `CARGO_MANIFEST_DIR` is a compile-time constant
/// expanded in this crate, so nothing here depends on the process working
/// directory — and a future crate's golden test resolves to its own directory.
fn committed_golden_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("goldens")
}

#[test]
fn the_golden_committed_to_the_repository_matches_the_frame_that_produced_it() -> TestResult {
    // The artifact root is temporary, so a passing run writes nothing at all
    // into the working tree.
    let artifacts = TempDir::new()?;
    let settings = golden_settings(
        &committed_golden_root(),
        artifacts.path(),
        CaptureId::new(CAPTURE)?,
        OptIns::default(),
    );

    let outcome = verify_against_golden(&synthetic_frame()?, &synthetic_provenance(), &settings);

    assert!(
        matches!(outcome, GoldenOutcome::Pass),
        "the bytes in the repository must still be the bytes the generator \
         produces, got {outcome:?}"
    );
    Ok(())
}

/// Mints the committed golden and its sidecar through the harness's own update
/// path, so what lands in the repository is a product of the real code rather
/// than of a writer built for the occasion.
///
/// Ignored because it writes into the working tree. Run it deliberately:
/// `cargo nextest run -p mc-testkit --no-default-features --run-ignored ignored-only`
#[test]
#[ignore = "writes into the repository; run deliberately to regenerate the golden"]
fn regenerating_the_committed_golden_leaves_it_matching_the_generator() -> TestResult {
    let artifacts = TempDir::new()?;
    let settings = golden_settings(
        &committed_golden_root(),
        artifacts.path(),
        CaptureId::new(CAPTURE)?,
        UPDATING,
    );

    let outcome = verify_against_golden(&synthetic_frame()?, &synthetic_provenance(), &settings);

    assert!(
        matches!(
            outcome,
            GoldenOutcome::GoldenWritten { .. } | GoldenOutcome::GoldenUnchanged
        ),
        "regeneration is idempotent: it either writes the golden or finds it \
         already correct, got {outcome:?}"
    );
    Ok(())
}
