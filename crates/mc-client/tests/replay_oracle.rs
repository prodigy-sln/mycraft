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
//!
//! **Which tick is perturbed no longer selects anything, and that is a change
//! rather than a detail.** The reason used to be that tick 0's horizon sat
//! highest of the three — when the sky counts were 135 against tick 59's 32, a
//! fourfold gap that genuinely picked a frame. Since the declared spawn moved to
//! the coast they are **241 / 168 / 259** of 576, and tick 0 is not even the
//! roomiest: tick 119 is. Measured at the same time, the 3° control finds
//! **22 / 26 / 25** disagreements at the three ticks — an order of magnitude over
//! the budget of 2 at every one of them, and near enough the same order at each.
//!
//! So any of the three would serve, and tick 0 is kept because it is the opening
//! frame and because keeping it leaves the control unchanged in every respect but
//! the spawn. **The reason is recorded as no longer discriminating rather than
//! repaired to a fresher number**, because a justification that sounds decisive
//! over an 18-sample margin is worse than one that admits it is arbitrary.

mod support;

use std::error::Error;
use std::sync::Arc;

use glam::Vec3;
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
/// *collapsed* oracle rather than to be tight, so it is derived from both
/// directions and from neither tightly: **above zero**, because an oracle
/// predicting nothing is the one failure the one-sided comparison beside it
/// cannot see, and **below the tightest of the judged frames**, or a correct
/// march fails it.
///
/// Re-measured after the declared spawn moved to the coast: the three frames are
/// 58 % / 71 % / 55 % not sky and predict **335 / 408 / 317** of 576, so the
/// tightest is 317 and 100 sits 3.2× under it. The figures the coastal spawn
/// replaced were 78 % and around 450; the conclusion survived the move and the
/// sentence supporting it did not, which is why it is restated rather than left
/// standing beside a value it no longer describes.
const PREDICTION_FLOOR: usize = 100;

/// How far below the camera the control's prediction is marched from.
const CONTROL_PITCH_DEGREES: f32 = 3.0;

/// Everything a declared sample of these frames may be classified as: the sky,
/// and the four blocks the replay's declaration places.
///
/// **Written out rather than read off the registry**, which is the thing the
/// classification resolves through — a list discovered from the registry would
/// agree with it whatever it came to hold. The sea is among them and is the one
/// that was unreachable before this feature: for as long as the mesher and the
/// judge both decided by solidity, no ray could stop at water and no sample could
/// be classified as it.
const THE_CLASSES: [&str; 5] = [
    oracle::SKY,
    "base:dirt",
    "base:grass",
    "base:stone",
    "base:water",
];

/// The class a frame has to show at one sample at least.
const WATER: &str = "base:water";

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

/// The one reading here that judges the march itself rather than judging a frame
/// by it, and it needs no device for the same reason the floor above does not: a
/// classification is a statement about the world and the camera and about nothing
/// drawn.
///
/// **An enumerated verdict rather than a filter for the class of interest.** A
/// reading that counted water samples and asked for one could not see a march
/// that had come to answer nothing everywhere else, or one that classified a
/// sample as a block the declaration never places. Every sample is classified,
/// the classes are compared against the whole of what the world can offer, and
/// the count is compared against the grid's own size — so a march that stopped
/// short and a march that answered outside the world are two distinct failures
/// of one comparison.
#[test]
fn every_declared_sample_of_every_judged_frame_is_sky_or_a_block_the_world_places_and_some_is_sea()
-> TestResult {
    let classified = classifications(&JUDGED_TICKS)?;

    assert_eq!(
        classified,
        JUDGED_TICKS
            .iter()
            .map(|tick| Classified {
                tick: *tick,
                outside_the_declared_classes: Vec::new(),
                samples: oracle::SAMPLE_COUNT,
                sea: AT_LEAST_ONE,
            })
            .collect::<Vec<_>>(),
        "every one of the {} declared samples is looking at exactly one of {THE_CLASSES:?}, and \
         the sea is among them in every judged frame. A class outside that list is a march \
         answering about a world the declaration does not describe; a count short of the grid is \
         a march that stopped classifying; and a frame without the sea is a camera that cannot \
         see it, which is the state this feature found the declared spawn in",
        oracle::SAMPLE_COUNT
    );
    Ok(())
}

/// The premise every reading in this file and in the sea's own rests on, which
/// nothing else here states.
///
/// # A saturated frame passes everything, and only the spawn's position prevents it
///
/// If the eye stood **inside** a drawn voxel, `first_drawn` tests the voxel the
/// eye occupies before it steps, so every one of the 576 samples would classify
/// as that block. Follow that through the file:
///
/// - the classification totals the grid, and 576 of one class totals it perfectly;
/// - [`PREDICTION_FLOOR`] is a **floor** — 576 clears 100 without trouble, so the
///   collapse detector is silent;
/// - the one-sided comparison asks that predicted terrain not be drawn as sky, and
///   576 water samples are all non-sky;
/// - and the sea's own reading would then judge a frame that genuinely *is* water
///   at every sample, and pass **honestly**.
///
/// **Nothing in the workspace bounds terrain from above.** So a saturated
/// classification is not unrepresentable — it is merely absent, and what makes it
/// absent is where the declared spawn happens to sit. That is a contingency, and
/// this test is what turns it into a property: the counts *cannot* be 576 because
/// the eye is not inside anything drawn, rather than happening not to be.
///
/// It matters here more than it would have before this phase. `predicted_terrain`
/// now derives from the classification instead of marching separately — which is
/// right, and stops the two disagreeing about which samples are terrain — but it
/// also removes the second opinion that would have reported this.
///
/// # Why it needs no positive control of its own
///
/// The way an absence assertion rots is the subject coming to answer "no" to
/// everything. Here that is a judge whose `is_drawn` answers false everywhere,
/// and that judge predicts **nothing** as terrain — which is exactly what the
/// prediction floor above catches, in this same file. The two guards fail in
/// opposite directions and neither can go quiet without the other speaking.
#[test]
fn the_camera_of_every_judged_frame_stands_in_open_air() -> TestResult {
    let prepared = prepare_scene()?;
    let voxels = Voxels {
        world: &prepared.world,
        registry: &prepared.registry,
    };

    let mut standing = Vec::new();
    for tick in JUDGED_TICKS {
        let camera = support::frames::player_pose(tick, &prepared.world, &prepared.registry)?;
        let eye = Vec3::from_array(camera.eye).floor().as_ivec3();
        standing.push((
            tick,
            voxels
                .drawn_block(eye)?
                .map(|block| block.as_str().to_owned()),
        ));
    }

    assert_eq!(
        standing,
        JUDGED_TICKS.map(|tick| (tick, None)).to_vec(),
        "the eye has to stand in open air at every judged tick, and the cell it occupies is \
         named here rather than merely denied. A block reported for any of them is an eye \
         inside terrain — and if that block were the sea, every reading in this file and the \
         one that judges water's own colour would pass on a frame classified 576 out of 576 as \
         water. The declared spawn stands one column from the sea, so this is near enough to \
         be worth stating"
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
///
/// The registry is a shared handle rather than a value of its own, because the
/// simulation the preparation built holds it for the life of the run. The oracle
/// below still reads it directly and re-resolves every name it finds, which is
/// what keeps its judgement a separate lookup chain from the pre-resolved bitset
/// the physics reads — sharing the registry is not sharing the resolution.
struct Session {
    world: ReplayWorld,
    registry: Arc<BlockRegistry>,
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

/// What one tick's classification of the declared grid came to.
#[derive(Debug, PartialEq, Eq)]
struct Classified {
    tick: u32,
    /// Every class the march used that the declaration does not place, ascending
    /// and without repeats, so a failure names them rather than counting them.
    outside_the_declared_classes: Vec<String>,
    /// How many samples were classified at all.
    samples: usize,
    /// Whether the sea was among them.
    sea: &'static str,
}

/// What a frame that classified the sea at one sample or more reports.
const AT_LEAST_ONE: &str = "the sea at one sample or more";

/// What a frame that classified the sea nowhere reports.
const NONE_AT_ALL: &str = "the sea at no sample at all";

/// What the march classifies each declared sample as, at each of `ticks`, with
/// no device involved.
fn classifications(ticks: &[u32]) -> Result<Vec<Classified>, Box<dyn Error>> {
    let prepared = prepare_scene()?;
    let voxels = Voxels {
        world: &prepared.world,
        registry: &prepared.registry,
    };
    let mut classified = Vec::new();
    for tick in ticks {
        let camera = support::frames::player_pose(*tick, &prepared.world, &prepared.registry)?;
        let sighted = oracle::sighted_samples(&camera, CAPTURE_SIZE, &voxels)?;
        classified.push(summarised(
            *tick,
            sighted.iter().map(|(_, sighted)| sighted.described()),
        ));
    }
    Ok(classified)
}

/// What one tick's classes add up to.
///
/// The classes arrive as the words a tally is keyed by rather than as the
/// classifications themselves, so the comparison this feeds is over the same
/// spelling on both sides and a block name cannot arrive under the sky's word.
fn summarised(tick: u32, classes: impl Iterator<Item = String>) -> Classified {
    let classes: Vec<String> = classes.collect();
    let mut outside: Vec<String> = classes
        .iter()
        .filter(|class| !THE_CLASSES.contains(&class.as_str()))
        .cloned()
        .collect();
    outside.sort();
    outside.dedup();
    Classified {
        tick,
        outside_the_declared_classes: outside,
        samples: classes.len(),
        sea: if classes.iter().any(|class| class == WATER) {
            AT_LEAST_ONE
        } else {
            NONE_AT_ALL
        },
    }
}

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
