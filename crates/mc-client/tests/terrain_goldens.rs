//! The replay's three declared captures, against their committed goldens.
//!
//! **Ordering is binding and it is not a preference.** These goldens are minted
//! from this renderer, so a golden shot before the derived probes pass is a
//! photograph of whatever the renderer happened to do that day, and it then
//! passes forever. `terrain_probes.rs` is what makes shooting them safe;
//! nothing here can substitute for it.
//!
//! Each scenario runs with `MYCRAFT_UPDATE_GOLDENS` **unset**. With it set the
//! golden is minted and matched in the same run and the scenario asserts
//! nothing — the spec's own wording — so the opt-in is *read* through
//! `OptIns::from_environment` rather than assumed either way. No test in this
//! project sets an environment variable; `set_var` is `unsafe` in edition 2024.
//!
//! # Where the goldens live, and where this file lives
//!
//! The golden root stays `crates/mc-render/goldens/`, as `spec.md`'s binding
//! table requires. This file cannot: it renders the replay, which needs the
//! world `mc-sim` generates and the draw path `mc-render` owns, and neither of
//! those crates may resolve the other in any dependency kind. The composition
//! root is the only crate that resolves both.

mod support;

use std::collections::BTreeSet;
use std::error::Error;
use std::path::PathBuf;
use std::sync::Arc;

use mc_render::capture::{SCENE_REVISION, capture_id};
use mc_testkit::frame::{
    CaptureId, GoldenFailureReason, GoldenOutcome, GoldenSettings, OptIns, Thresholds,
};
use tempfile::TempDir;

use support::frames::ReplayFrame;
use support::{TestResult, prepare_scene, repository_root};

/// The three ticks `spec.md` declares captures for.
const OPENING: u16 = 0;
const HALF_TURN: u16 = 60;
const CLOSING: u16 = 119;

/// The four files a mismatch has to leave behind for a reader.
const MISMATCH_ARTIFACTS: [&str; 4] = ["actual.png", "diff.png", "expected.png", "report.json"];

#[test]
fn the_replays_opening_capture_matches_its_committed_golden() -> TestResult {
    let Some(outcome) = verified(OPENING)? else {
        return Ok(());
    };
    assert!(
        matched(&outcome),
        "the replay is a pure function of a fixed seed and its camera a pure function of the \
         tick, so tick {OPENING} draws the same picture on every run and the committed golden \
         is what says which picture that is: {outcome:?}"
    );
    Ok(())
}

#[test]
fn the_replays_half_turn_capture_matches_its_committed_golden() -> TestResult {
    let Some(outcome) = verified(HALF_TURN)? else {
        return Ok(());
    };
    assert!(
        matched(&outcome),
        "tick {HALF_TURN} is the pose every derived probe is written against, so its golden is \
         the one a regression is most likely to move: {outcome:?}"
    );
    Ok(())
}

#[test]
fn the_replays_closing_capture_matches_its_committed_golden() -> TestResult {
    let Some(outcome) = verified(CLOSING)? else {
        return Ok(());
    };
    assert!(
        matched(&outcome),
        "tick {CLOSING} sits three degrees from tick {OPENING} on the same orbit, and the \
         landmark alone shifts 6.7 px between them — far outside the area budget, so the two \
         committed sets cannot stand in for each other: {outcome:?}"
    );
    Ok(())
}

#[test]
fn the_half_turn_capture_judged_against_the_opening_golden_reports_the_mismatch_and_its_evidence()
-> TestResult {
    let workspace = TempDir::new()?;
    let Some(outcome) = judged(HALF_TURN, OPENING, workspace.path().join("artifacts"))? else {
        return Ok(());
    };

    let GoldenOutcome::Failed(failure) = outcome else {
        return Err(format!(
            "two different poses of the same orbit cannot match one golden, got {outcome:?}"
        )
        .into());
    };
    let GoldenFailureReason::Mismatch(comparison) = &failure.reason else {
        return Err(format!(
            "the comparison has to be the thing that failed, not the lookup: {failure}"
        )
        .into());
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
         was deliberate. It reported {comparison:?} and wrote {written:?}"
    );
    Ok(())
}

/// Whether `outcome` is the verdict a run with `MYCRAFT_UPDATE_GOLDENS` unset
/// has to reach.
///
/// With the opt-in set, the golden is minted and matched in the same run and
/// the scenario asserts nothing — which is `spec.md`'s own wording for FR-8.1,
/// not a loophole invented here.
fn matched(outcome: &GoldenOutcome) -> bool {
    match outcome {
        GoldenOutcome::Pass => true,
        GoldenOutcome::GoldenWritten { .. } | GoldenOutcome::GoldenUnchanged => {
            OptIns::from_environment().update_goldens
        }
        _ => false,
    }
}

/// The verdict on `tick`'s capture against its own committed golden, or `None`
/// when the opt-in permitted the absence of a device.
fn verified(tick: u16) -> Result<Option<GoldenOutcome>, Box<dyn Error>> {
    judged(
        tick,
        tick,
        repository_root()?.join("artifacts").join("frames"),
    )
}

/// The verdict on `tick`'s capture against the golden committed for
/// `judged_against`, with the evidence written under `artifact_root`.
fn judged(
    tick: u16,
    judged_against: u16,
    artifact_root: PathBuf,
) -> Result<Option<GoldenOutcome>, Box<dyn Error>> {
    let prepared = prepare_scene()?;
    let Some(context) = support::frames::device()? else {
        return Ok(None);
    };
    let mut renderer = support::frames::prepared_renderer(&context, &prepared)?;
    let scene = Arc::new(prepared.scene);
    let camera = support::frames::replay_camera(u32::from(tick))?;
    let snapshot = support::frames::snapshot(u32::from(tick), camera, &scene);

    let request = support::frames::request(&context, &capture_id(tick, SCENE_REVISION)?)?;
    let settings = settings(judged_against, artifact_root)?;
    let mut frame = ReplayFrame {
        context: &context,
        renderer: &mut renderer,
        snapshot: &snapshot,
    };
    Ok(Some(frame.verify(&request, &settings)?))
}

/// The golden lifecycle's settings for the capture declared at `tick`.
fn settings(tick: u16, artifact_root: PathBuf) -> Result<GoldenSettings, Box<dyn Error>> {
    Ok(GoldenSettings {
        golden_root: repository_root()?
            .join("crates")
            .join("mc-render")
            .join("goldens"),
        artifact_root,
        capture: CaptureId::new(&capture_id(tick, SCENE_REVISION)?)?,
        thresholds: Thresholds::default(),
        opt_ins: OptIns::from_environment(),
    })
}

/// The file names `written` holds, so the assertion is about which evidence
/// landed rather than about where the run happened to put it.
fn names_of(written: &[PathBuf]) -> BTreeSet<String> {
    written
        .iter()
        .filter_map(|path| path.file_name())
        .map(|name| name.to_string_lossy().into_owned())
        .collect()
}
