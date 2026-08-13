//! The half of the golden lifecycle a passing suite never exercises: one
//! capture judged against a golden that is not its own, and the evidence that
//! failure has to leave behind.
//!
//! **This test is alone in its own binary, and that is the point.** It judges
//! the tick-59 capture against tick 0's committed golden. Under
//! `MYCRAFT_UPDATE_GOLDENS` a judgement mints instead of comparing, so running
//! *this* test with the opt-in set writes a tick-59 frame as tick 0's ground
//! truth, and every later run then passes forever against the wrong reference —
//! with the diff that would have shown it being a binary blob nobody can read.
//!
//! Keeping it out of `terrain_goldens.rs` is what makes that binary safe to mint
//! wholesale, and therefore what lets `docs/technical/rendering.md` name the
//! mint target as a *binary* rather than as a test function whose name a
//! refactor moves silently. **`MYCRAFT_UPDATE_GOLDENS` must never be set for a
//! run that selects this binary.**

mod support;

use std::collections::BTreeSet;
use std::path::PathBuf;

use mc_testkit::frame::{GoldenFailureReason, GoldenOutcome};
use tempfile::TempDir;

use support::TestResult;
use support::goldens::{OPENING, WALKED, judged};

/// The four files a mismatch has to leave behind for a reader.
const MISMATCH_ARTIFACTS: [&str; 4] = ["actual.png", "diff.png", "expected.png", "report.json"];

#[test]
fn a_capture_of_the_walk_judged_against_the_spawns_golden_reports_the_mismatch_and_its_evidence()
-> TestResult {
    let workspace = TempDir::new()?;
    let Some(outcome) = judged(WALKED, OPENING, workspace.path().join("artifacts"))? else {
        return Ok(());
    };

    let GoldenOutcome::Failed(failure) = outcome else {
        return Err(format!("{TWO_DIFFERENT_FRAMES} Got {outcome:?}").into());
    };
    let GoldenFailureReason::Mismatch(comparison) = &failure.reason else {
        return Err(
            format!("the comparison has to be what failed, not the lookup: {failure}").into(),
        );
    };
    let written = failure
        .artifacts
        .as_ref()
        .map_err(|error| error.to_string())?;

    assert_eq!(
        (comparison.failing_pixels > 0, names_of(written)),
        (true, BTreeSet::from(MISMATCH_ARTIFACTS.map(str::to_owned))),
        "the compare-and-fail path is the half of the golden lifecycle a passing suite never \
         exercises, and a reader who cannot see what differed cannot judge whether the change \
         was deliberate. It reported {} failing pixels at worst distance {:.3} and wrote \
         {written:?}",
        comparison.failing_pixels,
        comparison.max_delta_e
    );
    Ok(())
}

/// Why one golden cannot stand in for another's tick.
///
/// The eye at tick 59 stands exactly two blocks below the eye at tick 0 — the
/// spawn's own fall, which is arithmetic over the declared spawn and not a
/// number read off a run — and, measured, about 1.7 blocks away from it
/// horizontally with a different terrain under it. No perceptual budget confuses
/// those two frames.
const TWO_DIFFERENT_FRAMES: &str = "the spawn's frame and the frame at the end of the walk are two different pictures, taken \
     two blocks apart vertically, so judging one against the other has to fail.";

/// The file names `written` holds, so the assertion is about which evidence
/// landed rather than about where the run happened to put it.
fn names_of(written: &[PathBuf]) -> BTreeSet<String> {
    written
        .iter()
        .filter_map(|path| path.file_name())
        .map(|name| name.to_string_lossy().into_owned())
        .collect()
}
