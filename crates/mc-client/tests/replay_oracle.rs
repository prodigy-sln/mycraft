//! What the player's own camera sees, judged against a ray marched through the
//! world's own voxels.
//!
//! # Why a coverage floor was not enough
//!
//! The assertion that stood here was "at least 15% of the frame is not sky",
//! derived for SPEC-004's orbit camera and carried unchanged to a camera it was
//! never derived for. Measured from the player's camera it is met with three to
//! five times of slack — a renderer drawing a third of what it should would still
//! pass it — and a threshold no plausible regression can cross is decoration.
//!
//! So the expected value is computed from the world instead. For each of 576
//! declared sample pixels a ray is cast from the **published** camera and marched
//! through `ReplayWorld::block_at` and `BlockDefinition::is_solid`, and the frame
//! has to agree wherever that march met a solid voxel. Nothing is declared, no
//! number is committed, and the assertion is valid for any seed and any spawn.
//!
//! # What this can and cannot catch
//!
//! The frame and the prediction are shot from the *same* published camera, so a
//! camera that is wrong moves both and they still agree. That is deliberate: the
//! camera's own correctness is FR-6.2's and FR-6.5's, asserted against the
//! world's heightmap and against exact per-tick arithmetic. What this suite is
//! the only evidence for is that the *renderer* draws the world the camera is
//! looking at — a mirrored frame, an inverted clip-space y, a field of view that
//! does not match the one the lens declares, a section wrongly culled, a mesh
//! built from the wrong voxels.
//!
//! # The comparison is one-sided, and that is what makes the floor mandatory
//!
//! Only "a sample predicted as terrain is not sky in the frame" is asserted. A
//! ray passing within a pixel of a silhouette cannot be trusted to predict *sky*
//! correctly, so the other direction is not asked for. The cost is that an oracle
//! predicting nothing would satisfy it perfectly, which is the strongest argument
//! against this whole approach — and the prediction floor below is the answer to
//! it. The two scenarios are one pair and were authored together.
//!
//! # Why the control is a downward pitch and not a yaw
//!
//! The dominant edge in these frames is the terrain horizon, which a yaw rotation
//! slides *along* rather than across. Only a downward pitch predicts terrain
//! where the frame has sky, which is the one direction a one-sided check can see.
//! Tick 0 is the perturbed frame because its horizon sits highest in the frame of
//! the three; by tick 59 the frame is almost entirely terrain and a small pitch
//! lands inside terrain either way.

mod support;

use std::error::Error;
use std::sync::Arc;

use mc_core::block::BlockRegistry;
use mc_render::camera::camera_view;
use mc_render::color::CLEAR_COLOR_SRGB;
use mc_render::geometry::scene::SceneGeometry;
use mc_render::gpu::TerrainRenderer;
use mc_sim::camera::CameraPose;
use mc_sim::replay::ReplayWorld;
use mc_testkit::frame::Rgba8Image;
use mc_testkit::frame::gpu::CaptureContext;

use support::frames::{CAPTURE_SIZE, ReplayFrame};
use support::oracle::{self, Voxels};
use support::{TestResult, prepare_scene, probe};

/// The ticks the goldens are declared at, and therefore the frames this judges.
const JUDGED_TICKS: [u32; 3] = [0, 59, 119];

/// The tick the two controls are run against.
const OPENING: [u32; 1] = [0];

/// How many predicted-terrain samples one frame may draw as the sky.
///
/// `spec.md`'s declared budget. **It may not be raised to make a test pass**: if
/// a sample lands within a pixel of a silhouette the remedy is to move that
/// sample, which is a recorded change to a declared fixture, and the control
/// below is what keeps the budget honest — a three-degree pitch puts roughly a
/// grid row across the horizon, an order of magnitude above this.
const DISAGREEMENT_BUDGET: usize = 2;

/// The fewest samples a march may predict as terrain in any judged frame.
///
/// Slack by design and unlike the budget above. Its job is to catch a
/// *collapsed* oracle rather than to be tight: a uniform grid over a frame that
/// is 78% not sky predicts on the order of 450 of the 576, so any value well
/// above zero and well below that serves.
const PREDICTION_FLOOR: usize = 100;

/// How far below the camera the control's prediction is marched from.
const CONTROL_PITCH_DEGREES: f32 = 3.0;

#[test]
fn every_sample_a_marched_ray_calls_terrain_is_drawn_as_something_other_than_sky() -> TestResult {
    let Some(session) = judged(&JUDGED_TICKS)? else {
        return Ok(());
    };

    let mut reported = Vec::new();
    let mut over_budget = Vec::new();
    for frame in &session.frames {
        let sky = oracle::disagreements(&frame.frame, &frame.predicted)?;
        if sky.len() > DISAGREEMENT_BUDGET {
            over_budget.push(frame.tick);
        }
        reported.push(describe(frame, &sky));
    }

    assert!(
        over_budget.is_empty(),
        "a ray that entered a solid voxel is looking at terrain, so the pixel it was cast \
         through cannot be the sky. The budget of {DISAGREEMENT_BUDGET} is for a sample that \
         lands within a pixel of a silhouette, where the march and the rasteriser can \
         legitimately round the other way — it is not room for a frame that shows something \
         else. Over budget at ticks {over_budget:?}. Measured: {}",
        reported.join("; ")
    );
    Ok(())
}

#[test]
fn every_frames_march_predicts_terrain_at_a_hundred_of_the_declared_samples_or_more() -> TestResult
{
    let predicted = predictions(&JUDGED_TICKS)?;
    let counted: Vec<(u32, usize)> = predicted
        .iter()
        .map(|(tick, samples)| (*tick, samples.len()))
        .collect();
    let collapsed: Vec<(u32, usize)> = counted
        .iter()
        .copied()
        .filter(|(_, count)| *count < PREDICTION_FLOOR)
        .collect();

    assert!(
        collapsed.is_empty(),
        "the agreement above is one-sided, so an oracle that predicted nothing would satisfy \
         it perfectly while checking nothing at all — this is what stops that being invisible. \
         The floor of {PREDICTION_FLOOR} of {} is deliberately far below what these frames \
         hold; it is a collapse detector and not a coverage assertion. Predicted per tick: \
         {counted:?}, short of the floor at {collapsed:?}",
        oracle::SAMPLE_COUNT
    );
    Ok(())
}

#[test]
fn a_prediction_marched_three_degrees_below_the_camera_disagrees_with_the_frame() -> TestResult {
    let Some(session) = judged(&OPENING)? else {
        return Ok(());
    };
    let opening = session.opening()?;
    let control = oracle::pitched_down(&opening.camera, CONTROL_PITCH_DEGREES);
    let predicted = oracle::predicted_terrain(&control, CAPTURE_SIZE, &session.voxels())?;
    let sky = oracle::disagreements(&opening.frame, &predicted)?;

    assert!(
        sky.len() > DISAGREEMENT_BUDGET,
        "a frame the oracle agrees with unconditionally is worse than the floor it replaced, \
         because it reads as evidence. Tilting the *prediction* {CONTROL_PITCH_DEGREES}° below \
         the camera the frame was drawn from moves every ray about {} pixels down the frame, \
         which puts roughly a whole grid row of samples across the terrain horizon — so a \
         comparison that can fail has to notice. It predicted {} samples as terrain and found \
         {} of them drawn as sky",
        pitch_in_pixels(),
        predicted.len(),
        sky.len()
    );
    Ok(())
}

#[test]
fn a_frame_of_nothing_but_sky_disagrees_with_the_prediction_the_world_gives() -> TestResult {
    let predicted = predictions(&OPENING)?;
    let (_, opening) = predicted.first().ok_or(NOTHING_JUDGED)?;
    let blank = probe::uniform(CAPTURE_SIZE.width, CAPTURE_SIZE.height, CLEAR_COLOR_SRGB)?;
    let sky = oracle::disagreements(&blank, opening)?;

    assert!(
        sky.len() > DISAGREEMENT_BUDGET,
        "the other way this comparison could pass for the wrong reason is a judgement that \
         never reports anything, and a frame of nothing but the declared clear colour is what \
         a renderer that drew nothing at all leaves behind. Every one of the {} samples the \
         world predicts as terrain has to be reported against it; {} were",
        opening.len(),
        sky.len()
    );
    Ok(())
}

/// What a run with no judged tick in it reports, which no declared tick list can
/// produce.
const NOTHING_JUDGED: &str =
    "no tick was judged at all, so the assertion below would be about an empty set";

/// One judged tick: the camera the player published, the frame drawn through it,
/// and the samples a march from that same camera called terrain.
struct Judged {
    tick: u32,
    camera: CameraPose,
    frame: Rgba8Image,
    predicted: Vec<(u32, u32)>,
}

/// One preparation of the replay, and everything judged out of it.
///
/// The world and the registry outlive the frames because the controls march a
/// second time, from a camera the player never published.
struct Session {
    world: ReplayWorld,
    registry: BlockRegistry,
    frames: Vec<Judged>,
}

impl Session {
    /// The world as the oracle reads it.
    fn voxels(&self) -> Voxels<'_> {
        Voxels {
            world: &self.world,
            registry: &self.registry,
        }
    }

    /// The first judged tick.
    fn opening(&self) -> Result<&Judged, Box<dyn Error>> {
        Ok(self.frames.first().ok_or(NOTHING_JUDGED)?)
    }
}

/// What one tick's march predicted: the tick, and the sample pixels it called
/// terrain.
type Predicted = (u32, Vec<(u32, u32)>);

/// What the march predicts at each of `ticks`, with no device involved.
///
/// A prediction is a statement about the world and the camera and about nothing
/// drawn, which is why the two scenarios that rest on it alone run on a machine
/// with no GPU at all.
fn predictions(ticks: &[u32]) -> Result<Vec<Predicted>, Box<dyn Error>> {
    let prepared = prepare_scene()?;
    let voxels = Voxels {
        world: &prepared.world,
        registry: &prepared.registry,
    };
    let mut predicted = Vec::new();
    for tick in ticks {
        let camera = support::frames::player_pose(*tick, &prepared.world, &prepared.registry)?;
        predicted.push((
            *tick,
            oracle::predicted_terrain(&camera, CAPTURE_SIZE, &voxels)?,
        ));
    }
    Ok(predicted)
}

/// Each of `ticks` rendered from the player's own camera and marched from it, or
/// `None` when the opt-in permitted the absence of a device.
fn judged(ticks: &[u32]) -> Result<Option<Session>, Box<dyn Error>> {
    let prepared = prepare_scene()?;
    let Some(context) = support::frames::device()? else {
        return Ok(None);
    };
    let mut renderer = support::frames::prepared_renderer(&context, &prepared)?;
    let scene = Arc::new(prepared.scene);
    let voxels = Voxels {
        world: &prepared.world,
        registry: &prepared.registry,
    };

    let mut frames = Vec::new();
    for tick in ticks {
        let camera = support::frames::player_pose(*tick, &prepared.world, &prepared.registry)?;
        let frame = drawn(&context, &mut renderer, &scene, (*tick, camera))?;
        let predicted = oracle::predicted_terrain(&camera, CAPTURE_SIZE, &voxels)?;
        frames.push(Judged {
            tick: *tick,
            camera,
            frame,
            predicted,
        });
    }
    Ok(Some(Session {
        world: prepared.world,
        registry: prepared.registry,
        frames,
    }))
}

/// The frame `shot`'s camera draws at `shot`'s tick, at the declared capture
/// size.
fn drawn(
    context: &CaptureContext,
    renderer: &mut TerrainRenderer,
    scene: &Arc<SceneGeometry>,
    shot: (u32, CameraPose),
) -> Result<Rgba8Image, Box<dyn Error>> {
    let (tick, camera) = shot;
    let view = camera_view(camera.eye, camera.target);
    let snapshot = support::frames::snapshot(tick, view, scene);
    let request = support::frames::request(context, &format!("player-camera-t{tick:03}"))?;
    let mut frame = ReplayFrame {
        context,
        renderer,
        snapshot: &snapshot,
    };
    frame.capture(&request)
}

/// What one judged tick measured, in words, whether or not it was satisfied.
fn describe(judged: &Judged, sky: &[(u32, u32)]) -> String {
    format!(
        "tick {} predicted {} of {} samples as terrain and the frame drew {} of them as sky \
         {sky:?}",
        judged.tick,
        judged.predicted.len(),
        oracle::SAMPLE_COUNT,
        sky.len()
    )
}

/// How far down the frame the control's pitch moves a ray, in pixels.
///
/// Derived rather than measured, from the declared 60° vertical field of view
/// over 720 rows: near the frame's centre a degree is twelve rows, so three
/// degrees is about a grid row of samples. Reported in the failure message so a
/// reader can tell a perturbation that was too small from one the frame did not
/// notice.
fn pitch_in_pixels() -> f32 {
    CONTROL_PITCH_DEGREES * CAPTURE_SIZE.height as f32 / 60.0
}
