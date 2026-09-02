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
use std::path::{Path, PathBuf};
use std::sync::Arc;

use mc_render::capture::{SCENE_REVISION, capture_id};
use mc_testkit::frame::gpu::CaptureContext;
use mc_testkit::frame::{CaptureId, GoldenOutcome, GoldenSettings, OptIns, Rgba8Image, Thresholds};

use super::frames::ReplayFrame;
use super::repository_root;

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

/// That same verdict, over a content root a fixture built.
///
/// **The root is a parameter so that a reading about the art's *sources* can be
/// taken through the golden path rather than beside it.** A model edited since
/// the set was built has to stop a golden run before any pixel is compared, and
/// a test asking that of `prepare_scene` alone would prove nothing about the
/// suites the goldens are actually shot by — those go through here.
///
/// # Errors
///
/// Returns the preparation, pipeline, spawn or capture failure. For a root whose
/// set is not current that failure is the set's own, raised before a device is
/// asked for, which is why this answers without one.
pub fn verified_over(root: &Path, tick: u16) -> Result<Option<GoldenOutcome>, Box<dyn Error>> {
    judged_over(root, tick, tick, artifact_root()?)
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
    judged_over(&super::content_root()?, tick, judged_against, artifact_root)
}

/// That same verdict, prepared from the content root at `root`.
///
/// # Errors
///
/// Returns the preparation, pipeline, spawn or capture failure.
pub fn judged_over(
    root: &Path,
    tick: u16,
    judged_against: u16,
    artifact_root: PathBuf,
) -> Result<Option<GoldenOutcome>, Box<dyn Error>> {
    shot(root, tick, |context, frame| {
        let request = super::frames::request(context, &capture_id(tick, SCENE_REVISION)?)?;
        let settings = settings(judged_against, artifact_root)?;
        frame.verify(&request, &settings)
    })
}

/// The pixels `tick`'s capture draws, prepared from the content root at `root`,
/// with no golden read at all.
///
/// **The same shot as [`judged_over`], through the same five steps**, so a
/// reading comparing these bytes against a committed blob is looking at the
/// picture the mint path produces and not at a second one assembled beside it.
/// That module header's rule — a mint path differing from a verify path is the
/// one thing the golden discipline cannot survive — is what [`shot`] exists to
/// keep, and it applies to a third reader exactly as it does to the first two.
///
/// # Errors
///
/// Returns the preparation, pipeline, spawn or capture failure.
pub fn drawn_over(root: &Path, tick: u16) -> Result<Option<Rgba8Image>, Box<dyn Error>> {
    shot(root, tick, |context, frame| {
        let request = super::frames::request(context, &capture_id(tick, SCENE_REVISION)?)?;
        frame.capture(&request)
    })
}

/// The five steps every declared terrain capture is shot through, with `run`
/// deciding what is made of the frame.
///
/// **The tint is resolved by the simulation's own resolver against the world
/// this capture is of**, through [`super::frames::snapshot_in`]. A hard-coded
/// `None` here would make every golden reading pass about a renderer that cannot
/// tint at all: the frame would be untinted because this line said so, and
/// nothing would redden the day a tint reached a dry camera. Resolving is what
/// makes the answer `None` because the eye stands in open air — which is the
/// property `replay_oracle.rs` asserts separately, and the premise the whole
/// golden set rests on.
fn shot<T>(
    root: &Path,
    tick: u16,
    run: impl FnOnce(&CaptureContext, &mut ReplayFrame<'_>) -> Result<T, Box<dyn Error>>,
) -> Result<Option<T>, Box<dyn Error>> {
    let prepared = super::prepare_scene_at(root)?;
    let Some(context) = super::frames::device()? else {
        return Ok(None);
    };
    let mut renderer = super::frames::prepared_renderer(&context, &prepared)?;
    let scene = Arc::new(prepared.scene.clone());
    let camera =
        super::frames::replay_camera(u32::from(tick), &prepared.world, &prepared.registry)?;
    let snapshot = super::frames::snapshot_in(&prepared, u32::from(tick), camera, &scene)?;
    let mut frame = ReplayFrame {
        context: &context,
        renderer: &mut renderer,
        snapshot: &snapshot,
    };
    let produced = run(&context, &mut frame)?;
    Ok(Some(produced))
}

/// The golden lifecycle's settings for the capture declared at `tick`.
fn settings(tick: u16, artifact_root: PathBuf) -> Result<GoldenSettings, Box<dyn Error>> {
    settings_for(&capture_id(tick, SCENE_REVISION)?, artifact_root)
}

/// Where this repository's committed captures live.
///
/// **One statement of it, shared with the settings below**, so a reading that
/// opens a committed blob by hand and a reading that judges through the golden
/// lifecycle cannot end up looking in two directories.
///
/// # Errors
///
/// Returns an error if the repository root cannot be located.
pub fn golden_root() -> Result<PathBuf, Box<dyn Error>> {
    Ok(repository_root()?
        .join("crates")
        .join("mc-render")
        .join("goldens"))
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
        golden_root: golden_root()?,
        artifact_root,
        capture: CaptureId::new(id)?,
        thresholds: Thresholds::default(),
        opt_ins: OptIns::from_environment(),
    })
}
