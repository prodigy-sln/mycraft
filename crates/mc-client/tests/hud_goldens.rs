//! The HUD's own declared capture, shot through the frame call the windowed
//! client makes, against its committed golden.
//!
//! **Ordering is binding and it is not a preference.** This golden is minted from
//! the renderer it verifies, so on its own it is a photograph of whatever the
//! renderer happened to do that day and it then passes forever. What makes
//! shooting it safe is `hud_prediction.rs`, which judges the declarations' own
//! rectangles per pixel against a derivation that shares no code with the
//! composition. Nothing here substitutes for that: the default tolerance this
//! file applies budgets `0.0001 × 1280 × 720` = 92 wrong pixels, and the base
//! crosshair's fill is 17.
//!
//! So the two scenarios here are the **whole-frame backstop** and the assertion
//! that the HUD is in the **captured** frame at all — which is not a given: the
//! three committed terrain goldens are shot through `record_terrain`, below
//! `App::draw`, and would never have seen a HUD. A capture that quietly took the
//! same road would carry a golden nothing about the HUD could ever move.
//!
//! **One golden, at tick 0.** The HUD does not animate and the held block is set
//! once, so ticks 59 and 119 would assert the same rectangles against different
//! terrain. Tick 0 is the frame with the least terrain coverage (77.91%,
//! measured), so the crosshair stands against the most sky.
//!
//! **The three terrain goldens are not touched and `SCENE_REVISION` is not
//! bumped.** Nothing about the mesh contract changes here, and bumping the
//! revision would rename and force a re-shoot of exactly the frames being
//! preserved. The HUD gets a capture id of its own instead.
//!
//! Each scenario runs with `MYCRAFT_UPDATE_GOLDENS` **unset**. With it set the
//! golden is minted and matched in the same run and the scenario asserts nothing,
//! so the opt-in is *read* through `OptIns::from_environment` rather than assumed
//! either way. No test in this project sets an environment variable; `set_var` is
//! `unsafe` in edition 2024.
//!
//! **This binary is safe to mint through**, which is what lets the re-shoot
//! procedure name it: the judgement below is against the capture's *own* golden,
//! and the frame-to-frame comparison beside it reads no golden at all.
//!
//! # Why the capture path is here and the settings are not
//!
//! The golden root, the thresholds and the opt-in reading come from
//! `support::goldens::settings_for`, so this capture and the three terrain ones
//! cannot be judged by different rules — that module's header says why a mint path
//! differing from a verify path is the one thing the discipline cannot survive.
//! What is built here is the *capture*: it is named by `hud_capture_id` rather
//! than by `capture_id` and it is recorded through `record_frame` rather than
//! `record_terrain`, and both of those differences are the point of it.

mod support;

use std::error::Error;

use mc_render::capture::{HUD_CAPTURE_TICKS, SCENE_REVISION, hud_capture_id};
use mc_testkit::frame::{GoldenOutcome, OptIns, Rgba8Image};

use support::TestResult;
use support::goldens::{artifact_root, settings_for};
use support::hud_frames::{HudCapture, compare_frames, hud_holding_default_block, no_hud};
use support::{content_root, frames};

/// How many pixels one declared capture holds: `1280 × 720`.
const FRAME_PIXELS: u64 = 921_600;

#[test]
fn the_hud_capture_differs_from_the_same_capture_with_nothing_declared() -> TestResult {
    for tick in HUD_CAPTURE_TICKS {
        let Some(captures) = hud_captures(tick)? else {
            return Ok(());
        };

        let seen = compare_frames(&captures.declared, &captures.bare, |_, _| true);
        assert_eq!(
            (seen.considered, seen.different == 0),
            (FRAME_PIXELS, false),
            "the frame the golden is shot from has to be a frame with a HUD in it. The three \
             terrain captures are recorded below the client's frame call and would never have seen \
             one, so a HUD capture that took the same road would commit a reference no HUD \
             regression could ever move — and it would pass forever. Tick {tick}: {seen:?}"
        );
    }
    Ok(())
}

#[test]
fn the_declared_hud_capture_matches_the_golden_committed_for_it() -> TestResult {
    let mut unmatched = Vec::new();
    for tick in HUD_CAPTURE_TICKS {
        let Some(outcome) = hud_verified(tick)? else {
            return Ok(());
        };
        unmatched.extend(reported(tick, &outcome));
    }

    assert!(
        unmatched.is_empty(),
        "the world is a pure function of its seed, the camera a pure function of the declared \
         spawn and script, and the HUD a pure function of what content declares — so this tick \
         draws the same picture on every run and the committed golden is what says which picture \
         that is. It is the whole-frame backstop and nothing more: its tolerance forgives 92 wrong \
         pixels where the crosshair's fill is 17, which is why the per-pixel prediction comes \
         first. Unmatched: {}",
        unmatched.join("; ")
    );
    Ok(())
}

/// The two frames the HUD capture declared at `tick` is asked about: the capture
/// itself, and the same capture with a layout that declares nothing.
#[derive(Debug)]
struct HudCaptures {
    declared: Rgba8Image,
    bare: Rgba8Image,
}

/// The HUD capture declared at `tick` and the same capture with nothing declared,
/// or `None` when the opt-in permitted the absence of a device.
///
/// One preparation, one renderer and one snapshot, so the two frames cannot
/// differ in anything but the HUD.
///
/// # Errors
///
/// Returns the preparation, pipeline, spawn or capture failure.
fn hud_captures(tick: u16) -> Result<Option<HudCaptures>, Box<dyn Error>> {
    let Some(context) = frames::device()? else {
        return Ok(None);
    };
    let mut capture = HudCapture::ready(&context, u32::from(tick))?;
    let shipped = hud_holding_default_block(&content_root()?, &capture.content)?;
    let id = hud_capture_id(tick, SCENE_REVISION)?;
    let request = frames::request(&context, &id)?;
    let declared = capture.capture(&shipped, &request)?;
    let request = frames::request(&context, &format!("{id}-declaring-nothing"))?;
    let bare = capture.capture(&no_hud()?, &request)?;
    Ok(Some(HudCaptures { declared, bare }))
}

/// The verdict on the HUD capture declared at `tick` against its own committed
/// golden, or `None` when the opt-in permitted the absence of a device.
///
/// # Errors
///
/// Returns the preparation, pipeline, spawn or capture failure.
fn hud_verified(tick: u16) -> Result<Option<GoldenOutcome>, Box<dyn Error>> {
    let Some(context) = frames::device()? else {
        return Ok(None);
    };
    let mut capture = HudCapture::ready(&context, u32::from(tick))?;
    let shipped = hud_holding_default_block(&content_root()?, &capture.content)?;
    let id = hud_capture_id(tick, SCENE_REVISION)?;
    let request = frames::request(&context, &id)?;
    let settings = settings_for(&id, artifact_root()?)?;
    Ok(Some(capture.verify(&shipped, &request, &settings)?))
}

/// How `tick`'s verdict reads to a person, or nothing at all when it matched.
///
/// Never `{outcome:?}`. A mismatch carries the per-pixel failing mask, and
/// debug-printing one buries the sentence a reader needs under eleven megabytes
/// of booleans. `GoldenFailure`'s own `Display` says which golden, how many
/// pixels stood past the tolerance, the worst distance, and where it wrote the
/// evidence.
fn reported(tick: u16, outcome: &GoldenOutcome) -> Option<String> {
    if matched(outcome) {
        return None;
    }
    Some(match outcome {
        GoldenOutcome::Failed(failure) => format!("tick {tick}: {failure}"),
        _ => format!(
            "tick {tick}: the golden was minted rather than matched, with the update opt-in \
             unset: {outcome:?}"
        ),
    })
}

/// Whether `outcome` is the verdict a run with `MYCRAFT_UPDATE_GOLDENS` unset has
/// to reach.
fn matched(outcome: &GoldenOutcome) -> bool {
    match outcome {
        GoldenOutcome::Pass => true,
        GoldenOutcome::GoldenWritten { .. } | GoldenOutcome::GoldenUnchanged => {
            OptIns::from_environment().update_goldens
        }
        _ => false,
    }
}
