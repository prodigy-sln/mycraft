//! Standing square in front of one face of one block in the shipped replay world.
//!
//! # Why this is shared
//!
//! Two readings ask where a drawn face's colours sit — one about which edge the
//! turf is on, one about which way round the image runs — and both need the same
//! three things: a grass block whose face on a given side the world exposes, an
//! eye placed square in front of it with world up as screen up, and the face's own
//! silhouette found by projecting its corners. Written once, so the two cannot
//! disagree about *which* face they are reading while disagreeing about what is on
//! it.
//!
//! # What it is not
//!
//! **No expectation about colour comes from here.** The projection says *where to
//! look* and nothing else; what must be at that pixel comes from
//! `content/base/materials/` and from the voxel model, neither of which the
//! renderer touches. That is the same separation `probe.rs`'s header states.

use std::error::Error;
use std::sync::Arc;

use glam::IVec3;
use mc_core::id::BlockName;
use mc_render::camera::{CameraView, camera_view};
use mc_testkit::frame::Rgba8Image;
use mc_world::mesh::Facing;
use mc_world::section::Contents;

use super::PreparedScene;
use super::frames::{self, CAPTURE_SIZE, ReplayFrame};
use super::oracle::Voxels;
use super::probe::project;
use super::swatch::require;

/// The tick every capture through here is labelled with. The scene is static and
/// the poses are chosen rather than reached, so nothing depends on it.
const TICK: u32 = 0;

/// The block whose faces these readings are about.
const GRASS: &str = "base:grass";

/// The four horizontal facings a grass side is drawn on.
pub const SIDES: [Facing; 4] = [Facing::NegZ, Facing::PosZ, Facing::PosX, Facing::NegX];

/// How far in front of a face the eye stands, in blocks.
///
/// **Chosen for texel size, and the arithmetic is here so it can be moved
/// deliberately.** The lens's vertical focal length is `cot 30° = √3`, so a face
/// one block tall at `d` blocks covers `√3 / d` of the frame's half-height. At
/// `d = 1.5` that is 0.577, so the face spans 415 of a 720-pixel frame — **26
/// pixels to a texel row**, which is what lets a reading sample inside one texel
/// without touching its neighbour.
pub const IN_FRONT: f32 = 1.5;

/// How many cells outward of the face have to hold nothing drawn for the eye to
/// stand there.
///
/// Two: the eye is 1.5 blocks out, so the cell it occupies and the one between it
/// and the face both have to hold nothing drawn, or something nearer than the
/// face is what the frame is of.
const CLEAR_CELLS_NEEDED: i32 = 2;

/// One grass block per facing whose face on that facing is exposed and reachable.
pub type Exposed = Vec<(Facing, IVec3)>;

/// Where a face lands on the frame: left, right, top and bottom in pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Silhouette {
    /// Smallest column the face covers.
    pub left: u32,
    /// Largest column the face covers.
    pub right: u32,
    /// Smallest row the face covers.
    pub top: u32,
    /// Largest row the face covers.
    pub bottom: u32,
}

/// One grass block per facing whose face on that facing the world exposes and in
/// front of which the eye can stand.
///
/// **Searched in a declared order, so the answer is the same on every run and on
/// every machine**, and each hit is checked for the two things a reading needs:
/// the neighbouring cell holds nothing drawn, so the mesher emitted that face at
/// all, and the two cells outward hold nothing drawn either, so nothing nearer
/// than the face is what the frame shows.
///
/// # Errors
///
/// Returns an error naming any facing the world offers no such block for. A pose a
/// reading cannot be taken from is a fixture failure and says so, rather than
/// being read as a picture that is wrong.
pub fn exposed_side_faces(prepared: &PreparedScene) -> Result<Exposed, Box<dyn Error>> {
    let voxels = Voxels {
        world: &prepared.world,
        registry: prepared.registry.as_ref(),
    };
    let grass = BlockName::parse(GRASS)?;
    let mut found = Vec::new();
    for facing in SIDES {
        let at = first_exposed(facing, &voxels, &grass).ok_or_else(|| {
            format!(
                "the replay world exposes no `{GRASS}` face on its {} side with \
                 {CLEAR_CELLS_NEEDED} cells in front of it holding nothing drawn, so this reading \
                 has no pose to be taken from",
                named(facing)
            )
        })?;
        found.push((facing, at));
    }
    Ok(found)
}

/// The eye standing square in front of the face on `facing`, looking at its centre.
///
/// World up is the camera's up on all four, so **screen-up is world-up** in every
/// capture — which is what makes "turf high on the face" a claim a reader can
/// check by looking.
#[must_use]
pub fn square_in_front_of(facing: Facing, at: IVec3) -> CameraView {
    let centre = centre_of(facing, at);
    let [x, y, z] = centre;
    let eye = match facing {
        Facing::NegX => [x - IN_FRONT, y, z],
        Facing::PosX => [x + IN_FRONT, y, z],
        Facing::NegZ => [x, y, z - IN_FRONT],
        _ => [x, y, z + IN_FRONT],
    };
    camera_view(eye, centre)
}

/// Where the face on `facing` of the block at `at` lands on a frame.
///
/// # Errors
///
/// Returns an error when a corner does not project in front of the camera, which
/// is a pose the reading cannot be taken from rather than a picture that is wrong.
pub fn silhouette_of(
    facing: Facing,
    at: IVec3,
    camera: &CameraView,
) -> Result<Silhouette, Box<dyn Error>> {
    let mut across = Vec::new();
    let mut down = Vec::new();
    for corner in corners_of(facing, at) {
        let (x, y) = project(corner, camera, CAPTURE_SIZE)?;
        across.push(x);
        down.push(y);
    }
    Ok(Silhouette {
        left: min_of(&across),
        right: max_of(&across),
        top: min_of(&down),
        bottom: max_of(&down),
    })
}

/// Where a point on a face lands on a frame.
///
/// # Errors
///
/// Returns an error when the point does not project in front of the camera.
pub fn where_on_frame(world: [f32; 3], camera: &CameraView) -> Result<(u32, u32), Box<dyn Error>> {
    project(world, camera, CAPTURE_SIZE)
}

/// The four corners of the face on `facing` of the block at `at`, in world
/// coordinates.
#[must_use]
pub fn corners_of(facing: Facing, at: IVec3) -> Vec<[f32; 3]> {
    let (x, y, z) = (at.x as f32, at.y as f32, at.z as f32);
    let outward = match facing {
        Facing::NegX => x,
        Facing::PosX => x + 1.0,
        Facing::NegZ => z,
        _ => z + 1.0,
    };
    let (low, high) = (0.0, 1.0);
    match facing {
        Facing::NegX | Facing::PosX => vec![
            [outward, y + low, z + low],
            [outward, y + low, z + high],
            [outward, y + high, z + low],
            [outward, y + high, z + high],
        ],
        _ => vec![
            [x + low, y + low, outward],
            [x + high, y + low, outward],
            [x + low, y + high, outward],
            [x + high, y + high, outward],
        ],
    }
}

/// The centre of the face on `facing` of the block at `at`.
#[must_use]
pub fn centre_of(facing: Facing, at: IVec3) -> [f32; 3] {
    let (x, y, z) = (at.x as f32, at.y as f32, at.z as f32);
    match facing {
        Facing::NegX => [x, y + 0.5, z + 0.5],
        Facing::PosX => [x + 1.0, y + 0.5, z + 0.5],
        Facing::NegZ => [x + 0.5, y + 0.5, z],
        _ => [x + 0.5, y + 0.5, z + 1.0],
    }
}

/// The facing's outward direction, as the model's own faces state theirs.
#[must_use]
pub fn outward_of(facing: Facing) -> [i32; 3] {
    match facing {
        Facing::NegX => [-1, 0, 0],
        Facing::PosX => [1, 0, 0],
        Facing::NegY => [0, -1, 0],
        Facing::PosY => [0, 1, 0],
        Facing::NegZ => [0, 0, -1],
        Facing::PosZ => [0, 0, 1],
    }
}

/// The facing's own word, for a failure that names which face was wrong.
#[must_use]
pub fn named(facing: Facing) -> &'static str {
    match facing {
        Facing::NegX => "west (-X)",
        Facing::PosX => "east (+X)",
        Facing::NegY => "down (-Y)",
        Facing::PosY => "up (+Y)",
        Facing::NegZ => "north (-Z)",
        Facing::PosZ => "south (+Z)",
    }
}

/// One capture of the prepared replay scene from `camera`, named after `facing`.
///
/// # Errors
///
/// Returns the capture path's own failure.
pub fn captured(
    context: &mc_testkit::frame::gpu::CaptureContext,
    prepared: &PreparedScene,
    scene: &Arc<mc_render::geometry::scene::SceneGeometry>,
    from: (CameraView, &str),
) -> Result<Rgba8Image, Box<dyn Error>> {
    let (camera, facing) = from;
    let mut renderer = frames::prepared_renderer(context, prepared)?;
    let snapshot = frames::snapshot(TICK, camera, scene);
    let name = format!(
        "grass-side-{}",
        facing.split_whitespace().next().unwrap_or("side")
    );
    let mut frame = ReplayFrame {
        context,
        renderer: &mut renderer,
        snapshot: &snapshot,
    };
    frame.capture(&frames::request(context, &name)?)
}

/// Fails unless a face projects large enough for `wanted` pixels to a texel.
///
/// # Errors
///
/// Returns an error stating the measured span, which is a pose the reading cannot
/// be taken from rather than a picture that is wrong.
pub fn require_large_enough(seen: Silhouette, wanted: u32) -> Result<(), Box<dyn Error>> {
    let across = seen.right.saturating_sub(seen.left);
    let down = seen.bottom.saturating_sub(seen.top);
    require(
        across >= wanted && down >= wanted,
        format!(
            "the face has to project large enough to read one texel from: it spans {}..{} across \
             and {}..{} down, where {wanted} pixels on each axis is the least this reading needs",
            seen.left, seen.right, seen.top, seen.bottom
        ),
    )
}

/// The first grass block, in the declared scan order, whose `facing` face is
/// exposed and reachable.
fn first_exposed(facing: Facing, voxels: &Voxels<'_>, grass: &BlockName) -> Option<IVec3> {
    let step = outward_step(facing);
    (0..64)
        .flat_map(|x| (0..64).map(move |z| (x, z)))
        .flat_map(|(x, z)| (32..66).rev().map(move |y| IVec3::new(x, y, z)))
        .find(|at| holds(voxels, *at, grass) && reachable(voxels, *at, step))
}

/// Whether the eye can stand in front of the face outward of `at` along `step`
/// with nothing drawn in the way.
///
/// Both cells matter: the one the eye occupies and the one between it and the
/// face. Anything drawn in either is nearer to the eye than the face this
/// reading is about, and the frame is then of that instead.
///
/// **The question is drawnness and not collision, and the two are no longer the
/// same question.** This helper used to ask whether either cell was solid, and
/// described what it was avoiding as "the camera inside terrain" — which was the
/// same set of cells while every drawn block was also an obstacle. It is not any
/// more: the sea is drawn and is not an obstacle, so an eye that may legitimately
/// *stand* in a cell of water still cannot read a face through one, because
/// water's own face toward the eye is emitted and is nearer than the face being
/// read.
///
/// **The two questions are incomparable rather than nested, and the difference
/// runs both ways.** Drawnness rejects a `drawn = true, solid = false` cell that
/// solidity admitted — the water case, and the tightening this reading needs.
/// But solidity rejected a `drawn = false, solid = true` cell that drawnness
/// admits, so a pose with an invisible obstacle standing between the eye and the
/// face is now accepted. **That is a real limit with a failure mode**: such a
/// block occludes unless its own declaration says otherwise, so the face this
/// reading was going to read is culled and never emitted at all, and the reading
/// would then be taken against a face that is not in the picture.
///
/// It cannot arise from the content as it ships — dirt, grass and stone state
/// nothing about being drawn and so answer it from their solidity, leaving water
/// the one block where the two part — but **that is a fact about what the four
/// declarations say and not about the two predicates**. A fixture declaring an
/// undrawn obstacle needs this helper to ask about occlusion as well, and it does
/// not ask today.
fn reachable(voxels: &Voxels<'_>, at: IVec3, step: IVec3) -> bool {
    (1..=CLEAR_CELLS_NEEDED).all(|out| voxels.is_drawn(at + step * out).ok() == Some(false))
}

/// Whether the cell at `at` holds `block`.
fn holds(voxels: &Voxels<'_>, at: IVec3, block: &BlockName) -> bool {
    let (Ok(x), Ok(y), Ok(z)) = (
        u32::try_from(at.x),
        u32::try_from(at.y),
        u32::try_from(at.z),
    ) else {
        return false;
    };
    matches!(voxels.world.block_at(x, y, z), Some(Contents::Holds(name)) if name == block)
}

/// Which way is outward of `facing`.
fn outward_step(facing: Facing) -> IVec3 {
    let [x, y, z] = outward_of(facing);
    IVec3::new(x, y, z)
}

/// The smallest and largest of a run of coordinates.
fn min_of(values: &[u32]) -> u32 {
    values.iter().copied().min().unwrap_or(0)
}
fn max_of(values: &[u32]) -> u32 {
    values.iter().copied().max().unwrap_or(0)
}
