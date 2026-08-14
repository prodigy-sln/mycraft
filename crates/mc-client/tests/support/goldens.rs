//! Judging one of the replay's declared captures against a committed golden.
//!
//! Shared by the two golden binaries because they must judge by the same
//! settings — same golden root, same capture ids, same thresholds, same opt-in
//! reading. Two statements of that would be two lifecycles, and the one thing
//! the golden discipline cannot survive is a mint path that differs from the
//! verify path.
//!
//! **Why there are two binaries at all.** Under `MYCRAFT_UPDATE_GOLDENS` a
//! judgement mints rather than compares, so a test that deliberately judges one
//! tick's frame against another tick's golden writes the wrong frame as ground
//! truth. That test lives alone in `golden_mismatch.rs` precisely so the mint
//! command can name a *binary* — `terrain_goldens` — instead of a test function
//! whose name a refactor moves silently. See `docs/technical/rendering.md`
//! §"Re-shooting a golden set".
//!
//! **The HUD capture is a third binary and it is a mint target too.** Its two
//! scenarios are safe to mint through: one judges the HUD capture against *its
//! own* golden, and the other compares two frames of the same run against each
//! other and reads no golden at all. The terrain captures below are shot through
//! `record_terrain`; the HUD capture is shot through `record_frame`, the one call
//! the windowed client makes — which is the whole point of it, since a frame
//! recorded below `App::draw` would never have seen a HUD.

use std::error::Error;
use std::path::PathBuf;
use std::sync::Arc;

use mc_render::capture::{SCENE_REVISION, capture_id};
use mc_testkit::frame::{CaptureId, GoldenOutcome, GoldenSettings, OptIns, Thresholds};

use super::frames::ReplayFrame;
use super::{prepare_scene, repository_root};

/// The three ticks `spec.md` declares captures for: the spawn before it has
/// fallen, the end of the straight walk, and the last tick of the script.
pub const OPENING: u16 = 0;
pub const WALKED: u16 = 59;
pub const CLOSING: u16 = 119;

/// Every declared capture, in tick order.
pub const DECLARED_TICKS: [u16; 3] = [OPENING, WALKED, CLOSING];

/// The verdict on `tick`'s capture against its own committed golden, or `None`
/// when the opt-in permitted the absence of a device.
///
/// # Errors
///
/// Returns the preparation, pipeline, spawn or capture failure.
pub fn verified(tick: u16) -> Result<Option<GoldenOutcome>, Box<dyn Error>> {
    judged(tick, tick, artifact_root()?)
}

/// The verdict on `tick`'s capture against the golden committed for
/// `judged_against`, with the evidence written under `artifact_root`.
///
/// # Errors
///
/// Returns the preparation, pipeline, spawn or capture failure.
pub fn judged(
    tick: u16,
    judged_against: u16,
    artifact_root: PathBuf,
) -> Result<Option<GoldenOutcome>, Box<dyn Error>> {
    let prepared = prepare_scene()?;
    let Some(context) = super::frames::device()? else {
        return Ok(None);
    };
    let mut renderer = super::frames::prepared_renderer(&context, &prepared)?;
    let scene = Arc::new(prepared.scene);
    let camera =
        super::frames::replay_camera(u32::from(tick), &prepared.world, &prepared.registry)?;
    let snapshot = super::frames::snapshot(u32::from(tick), camera, &scene);

    let request = super::frames::request(&context, &capture_id(tick, SCENE_REVISION)?)?;
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
    settings_for(&capture_id(tick, SCENE_REVISION)?, artifact_root)
}

/// Where a golden comparison writes its evidence.
///
/// # Errors
///
/// Returns an error if the repository root cannot be located.
pub fn artifact_root() -> Result<PathBuf, Box<dyn Error>> {
    Ok(repository_root()?.join("artifacts").join("frames"))
}

/// The golden lifecycle's settings for the capture called `id`.
///
/// **One statement of the golden root, the thresholds and the opt-in reading for
/// every capture this repository commits**, terrain and HUD alike. The capture id
/// is a parameter because a HUD capture is named by a different function than a
/// terrain one; everything else about the lifecycle is the same or the mint path
/// and the verify path have parted company.
///
/// # Errors
///
/// Returns the name failure for an id that cannot be a directory under the golden
/// root, or the failure to locate the repository.
pub fn settings_for(id: &str, artifact_root: PathBuf) -> Result<GoldenSettings, Box<dyn Error>> {
    Ok(GoldenSettings {
        golden_root: repository_root()?
            .join("crates")
            .join("mc-render")
            .join("goldens"),
        artifact_root,
        capture: CaptureId::new(id)?,
        thresholds: Thresholds::default(),
        opt_ins: OptIns::from_environment(),
    })
}
