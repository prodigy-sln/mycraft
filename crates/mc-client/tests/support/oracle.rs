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
//! - every declared sample is classified as exactly one of sky, a block the
//!   world holds, or a composition of them, the classes sum to the whole grid,
//!   and the sea is predicted at one sample at least — so a march that
//!   collapsed, or that came to answer "nothing" everywhere, is reported rather
//!   than passing quietly;
//! - a world holding a block declared `drawn = false, solid = true` is marched
//!   *through* — which no judge reading solidity can do.
//!
//! # The second rule, which the named breaker asked for and this is
//!
//! A first-drawn-voxel march was right only while every drawn block was opaque:
//! the nearest drawn surface was then the one a pixel showed. A block that
//! passes light no longer stops a ray. It is recorded as a **layer** and the
//! march goes on, so what a pixel shows is the composition of every layer
//! crossed over whatever finally stopped it.
//!
//! **A maximal run of one kind contributes exactly one layer, and the
//! distinction from one-per-voxel is the whole of the rule.** This judge walks
//! one voxel at a time, so a rule accumulating every voxel that passes light
//! would answer ten layers through ten cells of sea where the engine draws one
//! face — wrong in a way that reads exactly like a renderer defect. It is a
//! restatement of two engine rules arrived at independently, which is the
//! relationship this module already has with `drawn`: a block draws no face
//! against its own kind, and the face a ray *leaves* a run through has its
//! normal along the ray and is culled.
//!
//! **It is per run and not per volume.** An air gap ends a run, so a body
//! entered, left and entered again contributes two layers, which is what the
//! engine draws and what a volume rule gets wrong for a concave sea.
//!
//! **The run the eye stands in contributes nothing at all.** The eye is past
//! that run's entry face and in front of its exit face, and the exit face is
//! back-facing. That is why a camera inside a translucent volume gains no tint
//! from the volume it stands in, and it falls out of the run rule rather than
//! being a case beside it: the eye's own cell opens a run that the march is
//! already inside.
//!
//! **What this judge still does not read.** The degree comes from
//! [`BlockDefinition::opacity`](mc_core::block::BlockDefinition), the door
//! `drawn` comes through, never from the resolution the packer partitions on;
//! and composing those layers into a colour is [`super::composite`]'s.

use std::error::Error;

use glam::{IVec3, Vec3};
use mc_core::block::{BlockRegistry, Opacity, RegistryError};
use mc_core::id::BlockName;
use mc_render::color::CLEAR_COLOR_SRGB;
use mc_render::surface::SurfaceSize;
use mc_sim::camera::CameraPose;
use mc_sim::replay::ReplayWorld;
use mc_testkit::frame::Rgba8Image;
use mc_world::mesh::Facing;
use mc_world::section::Contents;

use super::march::{Basis, Lens, March};
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
/// **Three arms, and still no arm for "I could not tell".** A ray either meets a
/// drawn voxel inside the march limit or it does not, and there is no answer in
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
    /// The nearest drawn voxel along the ray holds this block, and it stops all
    /// the light reaching it.
    Terrain(BlockName),
    /// The ray crossed one run of a block that passes light per entry, nearest
    /// first, and met `beyond` past the last of them. `beyond` is `Sky` or
    /// `Terrain` and never another `Through`, because [`Crossed::sighted`] is
    /// the only thing that builds one and it flattens every layer into `layers`.
    Through {
        layers: Vec<BlockName>,
        beyond: Box<Sighted>,
    },
}

/// One declared sample pixel and what a ray through it is looking at.
pub type SightedSample = ((u32, u32), Sighted);

/// One declared sample pixel and everything the ray through it met.
pub type CrossedSample = ((u32, u32), Crossed);

/// What a sample is called wherever this suite tallies classifications as text.
///
/// **It is not a block name and cannot become one**: every namespaced name
/// carries a colon, so sky and a block can sit in one tally without either being
/// able to impersonate the other.
pub const SKY: &str = "sky";

/// What joins a layer to what stands behind it wherever a classification is
/// spelled out.
///
/// **Neither a block name nor [`SKY`] can contain it**, for the same reason the
/// colon keeps those two apart: a composite class cannot impersonate a simple
/// one, and a reading may split a class back into its parts and get exactly the
/// names that went in.
pub const OVER: &str = " over ";

impl Sighted {
    /// This classification as the one word a tally is keyed by.
    #[must_use]
    pub fn described(&self) -> String {
        match self {
            Self::Sky => SKY.to_owned(),
            Self::Terrain(block) => block.as_str().to_owned(),
            Self::Through { layers, beyond } => {
                let mut named: Vec<String> = layers
                    .iter()
                    .map(|block| block.as_str().to_owned())
                    .collect();
                named.push(beyond.described());
                named.join(OVER)
            }
        }
    }
}

/// One surface a marched ray met: the block standing there, and the facing of it
/// the ray came in through.
///
/// The facing is `None` only where the eye already stood inside that voxel:
/// there is no face to have entered by, and answering one would be an invention
/// — which is why [`super::composite`] refuses such a surface.
///
/// **No [`Eq`], because `along` is a distance.** Whoever compares two of these
/// is comparing a measurement, and a reading that wanted an exact equality over
/// one would be asserting that two marches produced the same float — which is a
/// statement about arithmetic rather than about what a ray met.
#[derive(Debug, Clone, PartialEq)]
pub struct Surface {
    pub block: BlockName,
    pub facing: Option<Facing>,
    /// How far along the ray this surface stands, in blocks. Zero where the eye
    /// already stood inside the voxel.
    pub along: f32,
}

/// Everything one marched ray met, in the order it met it.
///
/// **The one answer, from the one march.** [`Sighted`] is a view of this with
/// the facings dropped, rather than a second march of its own: two marches would
/// be two answers, and the day they parted the difference would read as a
/// renderer fault.
#[derive(Debug, Clone, PartialEq)]
pub struct Crossed {
    /// One entry per maximal run of a block that passes light, nearest first.
    pub layers: Vec<Surface>,
    /// The surface that stopped the ray, or nothing where none did inside
    /// [`MARCH_LIMIT`].
    pub beyond: Option<Surface>,
}

impl Crossed {
    /// This crossing as the classification a tally is keyed by.
    #[must_use]
    pub fn sighted(&self) -> Sighted {
        let beyond = self
            .beyond
            .as_ref()
            .map_or(Sighted::Sky, |met| Sighted::Terrain(met.block.clone()));
        if self.layers.is_empty() {
            return beyond;
        }
        Sighted::Through {
            layers: self.layers.iter().map(|met| met.block.clone()).collect(),
            beyond: Box::new(beyond),
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
        Ok(self.drawn_degree(voxel)?.map(|(block, _degree)| block))
    }

    /// That same block together with how much light the registry says it stops.
    ///
    /// The degree comes through
    /// [`BlockDefinition`](mc_core::block::BlockDefinition), the door `drawn`
    /// comes through and never the resolution the packer partitions on, so a
    /// judge and a subject that were both wrong about a block would still have
    /// to be wrong in two separate places.
    ///
    /// # Errors
    ///
    /// Returns [`RegistryError`] as [`drawn_block`](Self::drawn_block) does.
    pub fn drawn_degree(
        &self,
        voxel: IVec3,
    ) -> Result<Option<(&BlockName, Opacity)>, RegistryError> {
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
            Some(Contents::Holds(name)) => {
                let definition = self.registry.resolve(name)?;
                Ok(definition.drawn.then_some((name, definition.opacity)))
            }
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
    Ok(crossed_samples(camera, size, voxels)?
        .into_iter()
        .map(|(pixel, crossed)| (pixel, crossed.sighted()))
        .collect())
}

/// Everything the ray through `pixel` met, on a frame of `size` seen from
/// `camera`.
///
/// The same march [`crossed_samples`] runs, over one pixel of a caller's own
/// choosing rather than over the declared grid — which is what lets a reading
/// ask whether a sample stands *inside* a silhouette by marching its immediate
/// neighbours, instead of reading the answer off the frame it is judging.
///
/// # Errors
///
/// Returns [`RegistryError`] when the world holds a block `voxels`' registry
/// does not register.
pub fn crossed_at(
    camera: &CameraPose,
    size: SurfaceSize,
    pixel: (u32, u32),
    voxels: &Voxels<'_>,
) -> Result<Crossed, RegistryError> {
    let basis = Basis::of(camera);
    crossed(basis.eye, basis.ray_through(pixel, &Lens::of(size)), voxels)
}

/// Everything the ray through each declared sample pixel met, in
/// [`sample_pixels`] order.
///
/// The richer answer [`sighted_samples`] is a view of, and what a reading needs
/// to predict a *colour*: a composition wants the facing each surface was
/// entered by, because a block may draw a different image on each of its six.
///
/// # Errors
///
/// Returns [`RegistryError`] when the world holds a block `voxels`' registry
/// does not register.
pub fn crossed_samples(
    camera: &CameraPose,
    size: SurfaceSize,
    voxels: &Voxels<'_>,
) -> Result<Vec<CrossedSample>, RegistryError> {
    let basis = Basis::of(camera);
    let lens = Lens::of(size);
    let mut crossings = Vec::new();
    for pixel in sample_pixels() {
        let met = crossed(basis.eye, basis.ray_through(pixel, &lens), voxels)?;
        crossings.push((pixel, met));
    }
    Ok(crossings)
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

/// Everything a ray leaving `origin` along `direction` meets: one entry per
/// maximal run of a block that passes light, and then whatever stopped it.
///
/// The voxel the eye already stands in counts, which is what makes an eye inside
/// opaque terrain a prediction of that terrain rather than of the sky beyond it.
/// Where that voxel passes light it counts differently — it *opens* a run the
/// march is already inside, so that run contributes no layer, which is this
/// module's header's last paragraph falling out of one state variable rather
/// than being a case beside the rule.
fn crossed(origin: Vec3, direction: Vec3, voxels: &Voxels<'_>) -> Result<Crossed, RegistryError> {
    let mut march = March::of(origin, direction);
    let mut layers: Vec<Surface> = Vec::new();
    // The kind of the cell the ray is inside, where that cell holds a block
    // passing light. It is what makes a run maximal: a second cell of the same
    // kind adds no layer, and anything else — air included — ends the run.
    let mut running: Option<BlockName> = None;
    let mut entered: Option<Facing> = None;
    loop {
        match met(voxels, &march, entered)? {
            Met::Nothing => running = None,
            Met::Stopping(surface) => {
                return Ok(Crossed {
                    layers,
                    beyond: Some(surface),
                });
            }
            Met::Passing(surface) => running = opening(&mut layers, running, surface),
        }
        if march.travelled > MARCH_LIMIT {
            return Ok(Crossed {
                layers,
                beyond: None,
            });
        }
        entered = Some(march.step());
    }
}

/// What the cell a march stands in is to the ray crossing it.
enum Met {
    /// Nothing drawn, so any run the ray was inside ends here.
    Nothing,
    /// A block that passes light, which opens or continues a run.
    Passing(Surface),
    /// A block that stops the ray.
    Stopping(Surface),
}

/// What the cell `march` stands in is to a ray that entered it by `entered`.
fn met(voxels: &Voxels<'_>, march: &March, entered: Option<Facing>) -> Result<Met, RegistryError> {
    let Some((block, degree)) = voxels.drawn_degree(march.voxel)? else {
        return Ok(Met::Nothing);
    };
    let surface = Surface {
        block: block.clone(),
        facing: entered,
        along: march.travelled,
    };
    Ok(if degree.passes_light() {
        Met::Passing(surface)
    } else {
        Met::Stopping(surface)
    })
}

/// `surface` folded into the run the ray is already inside, answering the kind
/// it is now inside.
///
/// A cell of the kind the ray was already crossing adds nothing — that is the
/// run rule. A cell with no facing is the eye's own, which opens a run the march
/// is already inside and contributes no layer, which is why a camera standing in
/// a translucent volume gains nothing from it.
fn opening(
    layers: &mut Vec<Surface>,
    running: Option<BlockName>,
    surface: Surface,
) -> Option<BlockName> {
    if running.as_ref() == Some(&surface.block) {
        return running;
    }
    let block = surface.block.clone();
    if surface.facing.is_some() {
        layers.push(surface);
    }
    Some(block)
}
