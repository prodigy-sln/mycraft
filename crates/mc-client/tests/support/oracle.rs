//! An independent prediction of what the player's camera sees, marched through
//! the world's own voxels.
//!
//! This is the judge, never the thing judged. The assertion it serves is
//! one-sided — *every sample this predicts as terrain has to be something other
//! than sky in the frame* — because a ray passing within a pixel of a silhouette
//! cannot be trusted to predict **sky** correctly, while a ray that entered a
//! drawn voxel a long way from any edge can be trusted to predict terrain.
//!
//! # It shares no code with the renderer's projection
//!
//! That is the whole point of it, and it is a constraint no assertion can
//! enforce, so it is written here where a reader meets it. The basis below is
//! built by hand — forward from the pose, right from `forward × up`, up from
//! `right × forward` — and a pixel is turned into a direction by hand from the
//! frame's own dimensions and the lens's half-angle. Nothing here calls
//! [`view_projection`](mc_render::camera::view_projection), builds a matrix, or
//! inverts one.
//!
//! **Reading the field of view and the aspect off `projection_for` is reading a
//! declaration, not sharing the projection.** Those two numbers are what the
//! renderer was *told*; the matrix that turns them into pixels is what it does
//! with them, and that is the thing under test. Restating 60° here instead would
//! make a widened field of view a disagreement between the oracle and the
//! renderer that this suite could not tell from a draw-path defect — and would
//! be a committed number besides. The near and far distances are deliberately
//! **not** read: a march has no near plane, and geometry closer to the eye than
//! one is exactly what FR-4.6 is about.
//!
//! # It is the slow, obvious implementation
//!
//! One voxel at a time, one registry lookup per voxel, no bitset and no
//! acceleration structure — `crates/mc-sim/tests/support/oracle.rs` is the same
//! shape and for the same reason. Being obviously right is the only property it
//! needs. In particular it reads drawnness through
//! [`BlockDefinition::drawn`](mc_core::block::BlockDefinition) and never through
//! the pre-resolved bitset the physics uses, so an oracle and a subject that
//! were both wrong about a block would still have to be wrong in two separate
//! places.
//!
//! # Why marching on `drawn` is not marching on the mesher's decision
//!
//! The mesher decides a face by three questions at once — is this block drawn,
//! does what lies beyond it fail to occlude, and is what lies beyond it a
//! different kind — computed over a resolved key table and a boundary plane
//! carrying one key per cell. **This judge is given none of those.** It has no
//! boundary plane, no key table and no `occludes` answer; it asks one question,
//! of one voxel at a time, and never looks at a neighbour at all. Two different
//! questions, answered by two implementations that share nothing but the
//! registry lookup — which is the same relationship the two had when both read
//! solidity.
//!
//! # The two directions that make the prediction non-vacuous
//!
//! A judge that had quietly stopped reading the registry would once have gone on
//! agreeing about water for free, and nothing would have said so. Two readings
//! close that, and they are the reason this module's old paragraph about water
//! is gone rather than softened:
//!
//! - every declared sample is classified as exactly one of sky or a block the
//!   world holds, the classes sum to the whole grid, and the sea is predicted at
//!   one sample at least — so a march that collapsed, or that came to answer
//!   "nothing" everywhere, is reported rather than passing quietly;
//! - a world holding a block declared `drawn = false, solid = true` is marched
//!   *through* — which no judge reading solidity can do.
//!
//! **The named breaker.** A first-drawn-voxel march is right only while every
//! drawn block is opaque: the nearest drawn surface is then the one a pixel
//! shows. The day a translucent block exists — PRO-952 — this judge needs a
//! second rule, because the nearest drawn surface will no longer be the only
//! thing the pixel is of. Recorded here, at the assumption, rather than
//! reconstructed later.

use std::error::Error;

use glam::{IVec3, Vec3};
use mc_core::block::{BlockRegistry, RegistryError};
use mc_core::id::BlockName;
use mc_render::camera::projection_for;
use mc_render::color::CLEAR_COLOR_SRGB;
use mc_render::surface::SurfaceSize;
use mc_sim::camera::CameraPose;
use mc_sim::replay::ReplayWorld;
use mc_testkit::frame::Rgba8Image;
use mc_world::mesh::Facing;
use mc_world::section::Contents;

use super::probe::{DIFFERENT_COLOR, distance, pixel_color};

/// How many sample pixels stand across the frame, and how many down it.
pub const SAMPLE_COLUMNS: u32 = 32;
pub const SAMPLE_ROWS: u32 = 18;

/// How far apart the samples stand, and how far the first one's centre sits from
/// the frame's top-left corner.
///
/// A declared fixture, not a discovered one: 32 × 18 centres at
/// `(40k + 20, 40m + 20)` covers a 1280 × 720 frame edge to edge, the last
/// column at 1260 and the last row at 700, each sample the centre of its own
/// 40 × 40 cell. Moving one of these is permitted when a sample lands within a
/// pixel of a silhouette — and then the move and its reason are recorded in
/// `test-map.md`, because a grid quietly nudged until a suite went green is the
/// same defect as a threshold quietly lowered.
pub const SAMPLE_SPACING: u32 = 40;
pub const SAMPLE_ORIGIN: u32 = 20;

/// How many samples the declared grid holds.
pub const SAMPLE_COUNT: usize = (SAMPLE_COLUMNS * SAMPLE_ROWS) as usize;

/// How far a ray is marched before the world is called empty along it, in
/// blocks.
///
/// The longest chord of the loaded world: √(64² + 64² + 256²) = 271.5 blocks. A
/// ray that has travelled further than that, from an eye inside the footprint,
/// has left everything solidity can be asked about — whichever way it left.
const MARCH_LIMIT: f32 = 272.0;

/// What one marched ray is looking at.
///
/// **Two arms, and no third for "I could not tell".** A ray either meets a drawn
/// voxel inside the march limit or it does not, and there is no answer in
/// between: a block the registry does not register is a [`RegistryError`] the
/// caller receives, never a sample quietly classified as nothing. That is what
/// lets a reading compare the whole classification of a grid — an answer outside
/// the set the world can offer, and a grid that stopped summing to itself, are
/// both failures of the same comparison.
///
/// `Sky` is a **prediction and not a reading of any frame**: it means the march
/// met no drawn voxel, which is a statement about the world and the camera
/// alone.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Sighted {
    /// The march met no drawn voxel inside [`MARCH_LIMIT`].
    Sky,
    /// The nearest drawn voxel along the ray holds this block.
    Terrain(BlockName),
}

/// One declared sample pixel and what a ray through it is looking at.
pub type SightedSample = ((u32, u32), Sighted);

/// What a sample is called wherever this suite tallies classifications as text.
///
/// **It is not a block name and cannot become one**: every namespaced name
/// carries a colon, so sky and a block can sit in one tally without either being
/// able to impersonate the other.
pub const SKY: &str = "sky";

impl Sighted {
    /// This classification as the one word a tally is keyed by.
    #[must_use]
    pub fn described(&self) -> String {
        match self {
            Self::Sky => SKY.to_owned(),
            Self::Terrain(block) => block.as_str().to_owned(),
        }
    }
}

/// The world the oracle marches, and the definitions it reads drawnness from.
///
/// Both by reference and both re-read per voxel. This is the pair
/// `crates/mc-sim/tests/support/overlap.rs` walks for the same reason: the
/// physics reads a pre-resolved bitset, so a judge reading the world and the
/// registry directly cannot inherit a mistake made while resolving it.
#[derive(Debug)]
pub struct Voxels<'a> {
    pub world: &'a ReplayWorld,
    pub registry: &'a BlockRegistry,
}

impl Voxels<'_> {
    /// The block at `voxel` where the registry says that block is drawn, and
    /// nothing where the cell is empty, outside the loaded world, or holds a
    /// block nothing draws.
    ///
    /// # Errors
    ///
    /// Returns [`RegistryError`] when the world holds a block the registry does
    /// not register. Reported rather than read as not drawn: a silent not-drawn
    /// would shrink the prediction, and a shrinking prediction is exactly what a
    /// one-sided comparison cannot see.
    pub fn drawn_block(&self, voxel: IVec3) -> Result<Option<&BlockName>, RegistryError> {
        let (Ok(x), Ok(y), Ok(z)) = (
            u32::try_from(voxel.x),
            u32::try_from(voxel.y),
            u32::try_from(voxel.z),
        ) else {
            return Ok(None);
        };
        // Three answers, three arms, and never two of them folded together. A
        // position the world does not reach and a cell holding nothing both mean
        // nothing to stop a ray — which is what would make writing them as one
        // arm invisible in the output. This judge re-reads the world and the
        // registry and consults nothing the simulation resolved, so an empty
        // answer reached here is reached independently of the one the subject
        // reached.
        match self.world.block_at(x, y, z) {
            None => Ok(None),
            Some(Contents::Empty) => Ok(None),
            Some(Contents::Holds(name)) => Ok(self.registry.resolve(name)?.drawn.then_some(name)),
        }
    }

    /// Whether the voxel at `voxel` is drawn, reading anything outside the
    /// loaded world as not drawn.
    ///
    /// # Errors
    ///
    /// Returns [`RegistryError`] when the world holds a block the registry does
    /// not register — see [`drawn_block`](Self::drawn_block), which this is the
    /// yes-or-no form of.
    pub fn is_drawn(&self, voxel: IVec3) -> Result<bool, RegistryError> {
        Ok(self.drawn_block(voxel)?.is_some())
    }
}

/// Every declared sample pixel, left to right and then top to bottom.
#[must_use]
pub fn sample_pixels() -> Vec<(u32, u32)> {
    (0..SAMPLE_ROWS)
        .flat_map(|row| {
            (0..SAMPLE_COLUMNS).map(move |column| {
                (
                    SAMPLE_SPACING * column + SAMPLE_ORIGIN,
                    SAMPLE_SPACING * row + SAMPLE_ORIGIN,
                )
            })
        })
        .collect()
}

/// What every declared sample pixel of a frame of `size` is looking at, from
/// `camera`, in [`sample_pixels`] order.
///
/// **Every sample is classified and none is skipped**, so the answers sum to
/// [`SAMPLE_COUNT`] by construction and a reading that came to find fewer has
/// found a march that stopped rather than a grid that shrank.
///
/// # Errors
///
/// Returns [`RegistryError`] when the world holds a block `voxels`' registry
/// does not register.
pub fn sighted_samples(
    camera: &CameraPose,
    size: SurfaceSize,
    voxels: &Voxels<'_>,
) -> Result<Vec<SightedSample>, RegistryError> {
    let basis = Basis::of(camera);
    let lens = Lens::of(size);
    let mut sighted = Vec::new();
    for pixel in sample_pixels() {
        let met = first_drawn(basis.eye, basis.ray_through(pixel, &lens), voxels)?;
        sighted.push((pixel, met));
    }
    Ok(sighted)
}

/// The sample pixels a ray cast from `camera` onto a frame of `size` meets a
/// drawn voxel through.
///
/// **The samples [`sighted_samples`] does not call sky**, rather than a second
/// march of its own. The one-sided comparison this feeds and the classification
/// the grid is judged by must not be able to disagree about which samples are
/// terrain: two marches would be two answers, and the day they parted the
/// difference would read as a renderer fault.
///
/// # Errors
///
/// Returns [`RegistryError`] when the world holds a block `voxels`' registry
/// does not register.
pub fn predicted_terrain(
    camera: &CameraPose,
    size: SurfaceSize,
    voxels: &Voxels<'_>,
) -> Result<Vec<(u32, u32)>, RegistryError> {
    Ok(sighted_samples(camera, size, voxels)?
        .into_iter()
        .filter(|(_, sighted)| *sighted != Sighted::Sky)
        .map(|(pixel, _)| pixel)
        .collect())
}

/// The predicted samples `frame` draws as the sky.
///
/// Sky means what it means everywhere else in this suite: within the harness's
/// own ΔE ceiling of the declared clear colour. The metric is
/// [`probe::distance`](super::probe::distance) rather than a second
/// implementation of it, so a frame the goldens call terrain is a frame this
/// calls terrain.
///
/// # Errors
///
/// Returns the image-shape or threshold failure, or a predicted pixel that is
/// not a pixel of `frame`.
pub fn disagreements(
    frame: &Rgba8Image,
    predicted: &[(u32, u32)],
) -> Result<Vec<(u32, u32)>, Box<dyn Error>> {
    let mut sky = Vec::new();
    for pixel in predicted {
        if distance(pixel_color(frame, *pixel)?, CLEAR_COLOR_SRGB)? <= DIFFERENT_COLOR {
            sky.push(*pixel);
        }
    }
    Ok(sky)
}

/// `camera` tilted `degrees` downward about its own right axis, looking at the
/// same eye position.
///
/// Written as "lean from forward toward the camera's own down" rather than as a
/// rotation matrix, so the direction is legible in the expression instead of
/// resting on a handedness convention: at `degrees` of 0 it is the camera it was
/// given, and at 90 it looks straight at the ground.
#[must_use]
pub fn pitched_down(camera: &CameraPose, degrees: f32) -> CameraPose {
    let basis = Basis::of(camera);
    let angle = degrees.to_radians();
    let tilted = basis.forward * angle.cos() - basis.up * angle.sin();
    CameraPose {
        eye: camera.eye,
        target: (basis.eye + tilted).to_array(),
    }
}

/// Where the camera stands and the three directions it stands in.
///
/// Built by hand from the pose. `right = forward × up` and `up = right ×
/// forward` are the same two cross products the derived probes work the landmark
/// pixel out with, and they are written here rather than shared, because two
/// computations that share a step cannot check each other.
///
/// A forward direction parallel to the world's up axis would leave `right` with
/// no length. The player's pitch is clamped to ±89°, so it cannot arise from a
/// published camera; a caller handing this a degenerate pose gets a frame of
/// `NaN` rays and a prediction of nothing, which the prediction floor is what
/// catches.
struct Basis {
    eye: Vec3,
    forward: Vec3,
    right: Vec3,
    up: Vec3,
}

impl Basis {
    /// The basis `camera` implies.
    fn of(camera: &CameraPose) -> Self {
        let eye = Vec3::from_array(camera.eye);
        let forward = (Vec3::from_array(camera.target) - eye).normalize();
        let right = forward.cross(Vec3::Y).normalize();
        Self {
            eye,
            forward,
            right,
            up: right.cross(forward),
        }
    }

    /// The unit direction of the ray through the centre of `pixel`.
    ///
    /// The centre, at `pixel + 0.5`, because that is where a rasteriser samples
    /// the pixel it is filling — an oracle that marched through the pixel's
    /// corner would be judging the frame half a pixel away from where it looked.
    fn ray_through(&self, pixel: (u32, u32), lens: &Lens) -> Vec3 {
        let across = 2.0 * (pixel.0 as f32 + 0.5) / lens.width - 1.0;
        let down = 1.0 - 2.0 * (pixel.1 as f32 + 0.5) / lens.height;
        (self.forward
            + self.right * (across * lens.aspect * lens.tan_half_fov)
            + self.up * (down * lens.tan_half_fov))
            .normalize()
    }
}

/// How wide the frame is and how much of the world it takes in.
struct Lens {
    tan_half_fov: f32,
    aspect: f32,
    width: f32,
    height: f32,
}

impl Lens {
    /// The lens the renderer declares for a frame of `size`.
    fn of(size: SurfaceSize) -> Self {
        let projection = projection_for(size);
        Self {
            tan_half_fov: (projection.fov_y_radians * 0.5).tan(),
            aspect: projection.aspect,
            width: size.width as f32,
            height: size.height as f32,
        }
    }
}

/// The first drawn voxel a ray cast from `camera` through `pixel` meets on a
/// frame of `size`, and the facing of it the ray came in through.
///
/// **The independent answer to "what is this pixel looking at".** It reads the
/// world's own voxels and the registry's own drawnness and consults nothing the
/// renderer produced, which is what lets a reading assert a colour at a pixel
/// *and* say which block face that pixel is of without one of the two coming
/// from the other.
///
/// The facing is the one the ray entered by, taken from the axis the march last
/// crossed a boundary on — a march that steps one boundary at a time cannot
/// enter a voxel by two faces at once. `None` where the ray met nothing, and
/// also where the eye already stands inside a drawn voxel: there is no face to
/// have entered by, and answering one would be an invention.
///
/// # Errors
///
/// Returns [`RegistryError`] when the world holds a block `voxels`' registry
/// does not register.
pub fn first_drawn_face(
    camera: &CameraPose,
    size: SurfaceSize,
    pixel: (u32, u32),
    voxels: &Voxels<'_>,
) -> Result<Option<(IVec3, Facing)>, RegistryError> {
    let basis = Basis::of(camera);
    let mut march = March::of(basis.eye, basis.ray_through(pixel, &Lens::of(size)));
    if voxels.is_drawn(march.voxel)? {
        return Ok(None);
    }
    while march.travelled <= MARCH_LIMIT {
        let entered = march.step();
        if voxels.is_drawn(march.voxel)? {
            return Ok(Some((march.voxel, entered)));
        }
    }
    Ok(None)
}

/// What a ray leaving `origin` along `direction` is looking at: the block of the
/// first drawn voxel it meets, or sky where it met none inside [`MARCH_LIMIT`].
///
/// The voxel the eye already stands in counts, which is what makes an eye inside
/// terrain a prediction of that terrain rather than of the sky beyond it.
fn first_drawn(
    origin: Vec3,
    direction: Vec3,
    voxels: &Voxels<'_>,
) -> Result<Sighted, RegistryError> {
    let mut march = March::of(origin, direction);
    while march.travelled <= MARCH_LIMIT {
        if let Some(block) = voxels.drawn_block(march.voxel)? {
            return Ok(Sighted::Terrain(block.clone()));
        }
        let _entered = march.step();
    }
    Ok(Sighted::Sky)
}

/// The facing a voxel was entered by, given which way the march is travelling on
/// that axis: `lower` where it moves towards higher coordinates, and `higher`
/// where it moves the other way.
///
/// A march never steps zero on the axis it chose, so the middle case is
/// unreachable; it answers `lower` rather than carrying a fourth state nothing
/// can produce.
const fn entered_by(towards: i32, lower: Facing, higher: Facing) -> Facing {
    if towards < 0 { higher } else { lower }
}

/// A ray walking the voxel grid one boundary crossing at a time.
///
/// Exact rather than sampled: stepping along the ray in fixed increments can
/// pass through a voxel whose chord is shorter than the increment, which is
/// precisely what happens where a ray clips the corner of a block — the places
/// this oracle most needs to be right about. Crossing one boundary at a time
/// cannot skip a voxel at all.
struct March {
    /// The voxel the ray is inside.
    voxel: IVec3,
    /// Which way each axis's voxel coordinate moves, as −1, 0 or +1.
    towards: IVec3,
    /// How far along the ray each axis's next boundary stands.
    next: Vec3,
    /// How far apart successive boundaries stand on each axis.
    between: Vec3,
    /// How far along the ray the current voxel was entered.
    travelled: f32,
}

impl March {
    /// The march of a ray leaving `origin` along the unit vector `direction`.
    fn of(origin: Vec3, direction: Vec3) -> Self {
        let voxel = origin.floor().as_ivec3();
        Self {
            voxel,
            towards: IVec3::new(
                towards(direction.x),
                towards(direction.y),
                towards(direction.z),
            ),
            next: Vec3::new(
                boundary(origin.x, direction.x, voxel.x),
                boundary(origin.y, direction.y, voxel.y),
                boundary(origin.z, direction.z, voxel.z),
            ),
            between: direction.abs().recip(),
            travelled: 0.0,
        }
    }

    /// Crosses into the next voxel, which is the one on whichever axis's
    /// boundary stands nearest, and answers the facing of it that was entered
    /// through.
    ///
    /// A ray moving towards higher x enters the voxel it reaches by that
    /// voxel's **negative** x side, which is what the pairing below says.
    fn step(&mut self) -> Facing {
        let next = self.next;
        if next.x <= next.y && next.x <= next.z {
            self.travelled = next.x;
            self.voxel.x += self.towards.x;
            self.next.x += self.between.x;
            entered_by(self.towards.x, Facing::NegX, Facing::PosX)
        } else if next.y <= next.z {
            self.travelled = next.y;
            self.voxel.y += self.towards.y;
            self.next.y += self.between.y;
            entered_by(self.towards.y, Facing::NegY, Facing::PosY)
        } else {
            self.travelled = next.z;
            self.voxel.z += self.towards.z;
            self.next.z += self.between.z;
            entered_by(self.towards.z, Facing::NegZ, Facing::PosZ)
        }
    }
}

/// Which way a voxel coordinate moves as the ray advances along one axis.
fn towards(direction: f32) -> i32 {
    if direction > 0.0 {
        1
    } else if direction < 0.0 {
        -1
    } else {
        0
    }
}

/// How far along the ray the next voxel boundary on one axis stands.
///
/// Infinite for an axis the ray does not move along, which is what keeps that
/// axis from ever being the nearest boundary.
fn boundary(origin: f32, direction: f32, voxel: i32) -> f32 {
    if direction > 0.0 {
        (voxel as f32 + 1.0 - origin) / direction
    } else if direction < 0.0 {
        (voxel as f32 - origin) / direction
    } else {
        f32::INFINITY
    }
}
