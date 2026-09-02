//! A medium filling the cells an eye stands in, with an opaque wall a stated
//! distance beyond it, drawn through the client's own mesher, packer and draw
//! path.
//!
//! # What is the fixture's, and what is the shipped path
//!
//! Two things are the fixture's: the **declarations** — a copy of the shipped
//! root holding nothing but the blocks a reading names — and the **voxels**,
//! because no seed generates a world with a flat wall six blocks from an eye
//! standing inside a medium. Everything between is the product's own:
//! `mc_sim::content::load` reads the root, [`World::mesh`] is the shipped
//! mesher, `mc_client::startup::scene_of` is the shipped packer, and
//! `TerrainRenderer` is the shipped draw path. `support/reload_opacity.rs` is
//! the same shape and reached it the same way.
//!
//! **The tint is resolved from the same `World` the geometry was meshed from,
//! through [`eye_medium`].** Nothing here states a tint on a snapshot. A fixture
//! that did would be asserting about a frame the product never draws: the
//! shipped client copies a field the simulation published, and the simulation
//! published it by asking which block fills the cell the eye is in.
//!
//! # Which instrument answers which question
//!
//! This is the division the phase turns on, and it is a judgement rather than a
//! convenience. The **arithmetic** readings — is the mix `0.5` where `d/D` is
//! `0.5`, is a pixel off the frame's centre carried further than the centre is,
//! is each layer carried by its own distance — stand here, over a world whose
//! identity is irrelevant and whose distances are the difference of two declared
//! numbers. What has to be shipped for those is the **mesher, the packer and the
//! draw path**, and all three are.
//!
//! The **can-a-player-reach-this** readings stand over the shipped world, where
//! its identity is the entire question: `support/submerged.rs` holds the pose
//! for those, and the reading about the sea's own top face lands on the shipped
//! sea exactly as its scenario words it. **A fixture world is the wrong
//! instrument for that question and the right one for this**; neither is a
//! shortcut past the other.
//!
//! # Why the shipped world will not serve for these, measured rather than
//! assumed
//!
//! **The measurement below is the justification, not a convenience** — a fixture
//! whose reason is measured is a different artefact from one that is merely
//! easier, and this paragraph is what stops a later reader moving these readings
//! back onto the shipped world and quietly losing the one that tells radial
//! distance from depth.
//!
//! Every reading in FR-2 that *can* stand over the shipped world does — the
//! ones about the sea take their pose there. These cannot. The shipped sea is
//! 178 cells; its footprint at `y = 34` is `x 60..63 × z 0..34` and the two-deep
//! channel is `x 62..63 × z 0..30` over a flat bed at `y = 33.0`. From an eye
//! inside it the bed stands 1.5 blocks below and the opaque distances sideways
//! are the half-integers 2.5 … 34.5, so **six blocks is not among them**;
//! squarely faced walls at whole distances do exist down the channel, and the
//! widest of them runs **two columns** across the centre row, while a pixel a
//! quarter of the frame's width from the centre at six blocks stands
//! `6.0 · tan 27.17° = 3.08` blocks off it. There is no wall in that world both
//! six blocks away and wide enough to be looked at.
//!
//! # The geometry, and why every distance below is exact
//!
//! One 64 × 64 footprint. The medium fills `x ∈ [0, 20)` and the opaque wall the
//! single layer `x = 20`, both across `z ∈ [0, 64)` and `y ∈ [118, 140)`. The
//! wall's `−X` face therefore stands at exactly `x = 20.0`, and an eye at
//! `x = e` looking along `+X` sees it at exactly `20.0 − e` blocks. Nothing is
//! measured off a march: the distance is the difference of two declared
//! numbers.
//!
//! **The eye sits on a cell face at the whole distances, and that is safe by
//! construction rather than by luck.** `20.0 − 6.0 = 14.0` is a boundary, and
//! `containing` floors, so the eye's cell is 14 — but cells 13 and 14 both hold
//! the medium, so the answer does not turn on the rounding. [`Standing::about`]
//! reports which block the resolver found, so a reading says so rather than
//! assuming it.
//!
//! **The band is 22 blocks tall and 64 wide because the frame has to land inside
//! it.** At the declared capture size the lens takes in `tan 30° = 0.577` of the
//! distance vertically and `1.026` of it horizontally, so at the furthest pose
//! here — twelve blocks — the frame's corners reach `±6.93` blocks in `y` and
//! `±12.3` in `z` of the eye. From `y = 128.5` and `z = 32.5` that is
//! `y ∈ [121.6, 135.4]` and `z ∈ [20.2, 44.8]`, inside the wall on both axes.
//!
//! # Every face the eye could see but the wall's is back-facing
//!
//! The eye stands inside the medium's box, so each of that box's boundary faces
//! turns its outward normal away from the eye and is culled — which is the same
//! reason a camera inside the sea gains nothing from the sea. The one face left
//! along the view is the wall's `−X`, emitted because the medium declares
//! `occludes = false`.
//!
//! # Every layer is one flat colour
//!
//! A tint expectation is an exact triple only if the colour going into it is.
//! A flat layer also takes minification out of the question: nearest
//! magnification, linear minification and every mip level answer the same byte
//! for a texture of one colour, so the same expectation holds at 1.2 blocks and
//! at 12.

use std::error::Error;
use std::sync::Arc;

use mc_core::block::{BlockRegistry, MediumTint};
use mc_core::content::TEXTURE_EDGE;
use mc_core::id::{BlockName, TextureKey};
use mc_render::camera::camera_view;
use mc_render::geometry::scene::SceneGeometry;
use mc_render::gpu::{RecordTarget, TerrainRenderer, TerrainTextures};
use mc_render::pass::TerrainPassConfig;
use mc_render::snapshot::{ScenePhase, TerrainSnapshot};
use mc_render::texture::sampler::TERRAIN_SAMPLER;
use mc_render::texture::supplied::SuppliedTexels;
use mc_sim::world::{World, eye_medium};
use mc_testkit::frame::Rgba8Image;
use mc_testkit::frame::gpu::{CaptureContext, draw_fn};
use mc_world::world::{VoxelWorld, WorldPos};

use super::content::{ContentRoot, shipped_copy};
use super::frames::{CAPTURE_SIZE, request};

/// What a frame of this fixture is predicted to hold, and how a reading judges
/// it against that prediction.
///
/// **Re-exported whole, so every path that named this module still does.** The
/// split is by responsibility and not by import surface: a scenario asking for
/// `medium::TELLS_THEM_APART` is asking the medium fixture what it calls two
/// pixels the same, and which file that answer lives in is this module's
/// business rather than the caller's. `reload_remesh.rs` re-exports its three
/// children the same way and for the same reason.
mod reading;
pub use reading::*;

/// The blocks these readings declare, and the files they are declared in.
pub const MEDIUM: &str = "example:medium";
pub const WALL: &str = "example:wall";
pub const PANE: &str = "example:pane";
const MEDIUM_FILE: &str = "example__medium.luau";
const WALL_FILE: &str = "example__wall.luau";
const PANE_FILE: &str = "example__pane.luau";

/// The one flat colour each block's layer is filled with.
///
/// **Chosen against the two declared tints and against each other**, so that an
/// absent tint, a wrong tint and a tint applied to the wrong layer are three
/// distinguishable pictures. The wall is a saturated yellow and both tints are
/// dark, which is what puts the untinted wall furthest from every mix of it;
/// the pane is a green standing clear of both. `told_apart` measures the
/// separation on every run rather than leaving it to this paragraph.
pub const MEDIUM_COLOUR: [u8; 3] = [20, 40, 30];
pub const WALL_COLOUR: [u8; 3] = [252, 214, 24];
pub const PANE_COLOUR: [u8; 3] = [36, 176, 92];

/// The colours a medium declares in these readings, and the distance it reaches
/// full strength at.
///
/// **Neither is a colour any engine constant could supply.** The sky is
/// `#87CEEB`, and neither of these is that, black, white, or any layer above.
pub const TINT: [u8; 3] = [0x3A, 0x6E, 0xA5];
pub const OTHER_TINT: [u8; 3] = [0x8A, 0x44, 0x00];
pub const REACHES_AT: f32 = 12.0;

/// The degree the pane declares, and the one an ordinary opaque block does.
pub const HALF: f32 = 0.5;
const WHOLE: f32 = 1.0;

/// Where the medium's box ends and the wall's single layer begins, and how far
/// the filled band runs.
const WALL_PLANE: u32 = 20;
const BAND: std::ops::Range<u32> = 118..140;
const ACROSS: u32 = 64;
/// Four columns of `SECTION_SIZE`, which is [`ACROSS`]. Stated rather than
/// divided out, and multiplied back in [`the_geometry_holds`].
const COLUMNS: u32 = 4;

/// Where the eye stands across the frame, and the row it looks along.
pub const EYE_Y: f32 = 128.5;
pub const EYE_Z: f32 = 32.5;

/// A world of a medium and a wall, meshed, with the registry it was resolved
/// against.
pub struct Standing {
    world: World,
    scene: Arc<SceneGeometry>,
    texels: SuppliedTexels,
    resolution: mc_render::texture::TextureResolution,
    /// Kept so the temporary content root outlives every reading over it.
    _root: ContentRoot,
}

/// What the medium reaching full strength at [`REACHES_AT`] carries.
#[must_use]
pub fn tinting(colour: [u8; 3]) -> Option<MediumTint> {
    MediumTint::new(colour, REACHES_AT)
}

/// How far along the ray the wall's own face stands from an eye at `x`.
#[must_use]
pub fn wall_stands_at(eye_x: f32) -> f32 {
    WALL_PLANE as f32 - eye_x
}

/// The eye standing `blocks` in front of the wall, looking squarely at it.
#[must_use]
pub fn eye_facing_the_wall(blocks: f32) -> ([f32; 3], [f32; 3]) {
    let x = WALL_PLANE as f32 - blocks;
    ([x, EYE_Y, EYE_Z], [x + 1.0, EYE_Y, EYE_Z])
}

impl Standing {
    /// The world in which the medium fills every cell up to the wall, declaring
    /// `tint` at [`REACHES_AT`] where one is given.
    ///
    /// # Errors
    ///
    /// Returns the root's own refusal, the world's, the mesher's or the
    /// packer's.
    pub fn behind_a_wall(tint: Option<[u8; 3]>) -> Result<Self, Box<dyn Error>> {
        Self::of(tint, None)
    }

    /// That same world with one layer of a pane declaring [`HALF`] standing at
    /// `x = 14`, and the medium stopping short of it.
    ///
    /// # Errors
    ///
    /// As [`behind_a_wall`](Self::behind_a_wall).
    pub fn behind_a_pane_and_a_wall(tint: Option<[u8; 3]>) -> Result<Self, Box<dyn Error>> {
        Self::of(tint, Some(PANE_PLANE))
    }

    /// What the simulation's own resolver answers for an eye at `at`, and which
    /// block it found there.
    ///
    /// # Errors
    ///
    /// Returns the registry's refusal for a block this world's registry does not
    /// register.
    pub fn about(&self, at: [f32; 3]) -> Option<MediumTint> {
        eye_medium(&self.world, glam::Vec3::from_array(at))
    }

    /// The frame this world draws from `eye` looking at `target`, or `None` when
    /// the opt-in permitted the absence of a device.
    ///
    /// # Errors
    ///
    /// Returns the pipeline, upload or capture failure.
    pub fn drawn(
        &self,
        eye: [f32; 3],
        target: [f32; 3],
        named: &str,
    ) -> Result<Option<Rgba8Image>, Box<dyn Error>> {
        let Some(context) = super::frames::device()? else {
            return Ok(None);
        };
        let mut renderer = TerrainRenderer::new(
            context.device(),
            context.queue(),
            &TerrainPassConfig::offscreen(),
            &TerrainTextures {
                supplied: &self.texels,
                sampler: TERRAIN_SAMPLER,
            },
        )?;
        renderer.upload_textures(context.queue(), self.resolution.layers())?;
        renderer.upload_scene(context.queue(), &self.scene)?;
        let snapshot = TerrainSnapshot {
            tick: 0,
            camera: camera_view(eye, target),
            scene: Arc::clone(&self.scene),
            tint: self.about(eye),
        };
        Ok(Some(captured(&context, &mut renderer, &snapshot, named)?))
    }

    /// The world and the scene, built from a root declaring the blocks below.
    fn of(tint: Option<[u8; 3]>, pane_at: Option<u32>) -> Result<Self, Box<dyn Error>> {
        let root = declaring(tint, pane_at.is_some())?;
        let loaded =
            mc_sim::content::load(root.path(), &mc_core::content::LayerAssignment::none())?;
        let registry = Arc::new(loaded.registry);
        let world = World::new(voxels(&registry, pane_at)?, Arc::clone(&registry))?;
        let resolution = mc_client::content::ContentView::of(&loaded.resolved).into_resolution();
        let scene = Arc::new(mc_client::startup::scene_of(&world.mesh()?, &resolution)?);
        Ok(Self {
            world,
            scene,
            texels: flat_layers(pane_at.is_some())?,
            resolution,
            _root: root,
        })
    }
}

/// Where the pane stands when a reading asks for one.
const PANE_PLANE: u32 = 14;

/// The cells of the world: the medium up to the wall, or up to the pane where
/// there is one, and the wall's single layer at [`WALL_PLANE`].
fn voxels(registry: &BlockRegistry, pane_at: Option<u32>) -> Result<VoxelWorld, Box<dyn Error>> {
    let mut blocks = VoxelWorld::empty(COLUMNS);
    let along = one_row(pane_at)?;
    for (y, z) in BAND.flat_map(|y| (0..ACROSS).map(move |z| (y, z))) {
        for (x, held) in &along {
            blocks.set_block(WorldPos { x: *x, y, z }, held, registry)?;
        }
    }
    Ok(blocks)
}

/// Which block stands at each `x` of one row, nearest the eye first.
///
/// The row is worked out once and written into every cell of the band, which is
/// what makes the wall's face a single plane at [`WALL_PLANE`] rather than
/// something a reader has to reconstruct from three loops.
fn one_row(pane_at: Option<u32>) -> Result<Vec<(u32, BlockName)>, Box<dyn Error>> {
    let medium = BlockName::parse(MEDIUM)?;
    let filled = pane_at.unwrap_or(WALL_PLANE);
    let mut along: Vec<(u32, BlockName)> = (0..filled).map(|x| (x, medium.clone())).collect();
    if let Some(plane) = pane_at {
        along.push((plane, BlockName::parse(PANE)?));
    }
    along.push((WALL_PLANE, BlockName::parse(WALL)?));
    Ok(along)
}

/// A copy of the shipped root declaring these blocks and no others.
fn declaring(tint: Option<[u8; 3]>, with_a_pane: bool) -> Result<ContentRoot, Box<dyn Error>> {
    let mut root = shipped_copy()?
        .declaring_no_blocks()?
        .declaring_block(WALL_FILE, &opaque_declaration(WALL))?
        .declaring_block(MEDIUM_FILE, &medium_declaration(tint))?;
    if with_a_pane {
        root = root.declaring_block(PANE_FILE, &passing_declaration(PANE, HALF, None))?;
    }
    Ok(root)
}

/// The wall's declaration: an ordinary opaque block, saying nothing about either
/// occlusion or opacity.
fn opaque_declaration(name: &str) -> String {
    format!("return {{\n\tname = \"{name}\",\n\ttexture = \"{name}\",\n\tsolid = true,\n}}\n")
}

/// The medium's declaration, stating a tint pair where one is given.
///
/// **`occludes = false` is stated whatever the tint is**, so the two roots a
/// reading compares differ in the tint and in nothing else. `occludes` falls
/// back to `solid`, and a block that both passes light and hides what is behind
/// it is refused, taking the root with it.
fn medium_declaration(tint: Option<[u8; 3]>) -> String {
    passing_declaration(MEDIUM, HALF, tint)
}

/// A block that passes light at `degree`, declaring `tint` at [`REACHES_AT`]
/// where one is given.
fn passing_declaration(name: &str, degree: f32, tint: Option<[u8; 3]>) -> String {
    let stated = tint.map_or_else(String::new, |[red, green, blue]| {
        format!(
            "\ttint = \"#{red:02X}{green:02X}{blue:02X}\",\n\ttint_distance = {REACHES_AT:?},\n"
        )
    });
    format!(
        "return {{\n\tname = \"{name}\",\n\ttexture = \"{name}\",\n\tsolid = true,\n\toccludes = \
         false,\n\topacity = {degree:?},\n{stated}}}\n"
    )
}

/// A layer of one colour for every block declared.
fn flat_layers(with_a_pane: bool) -> Result<SuppliedTexels, Box<dyn Error>> {
    let mut stated = vec![
        (TextureKey::parse(WALL)?, filled(WALL_COLOUR)),
        (TextureKey::parse(MEDIUM)?, filled(MEDIUM_COLOUR)),
    ];
    if with_a_pane {
        stated.push((TextureKey::parse(PANE)?, filled(PANE_COLOUR)));
    }
    Ok(SuppliedTexels::stating(stated))
}

/// One layer's worth of texels, every one of them `colour` at full alpha.
fn filled(colour: [u8; 3]) -> Vec<[u8; 4]> {
    let [red, green, blue] = colour;
    vec![[red, green, blue, u8::MAX]; (TEXTURE_EDGE * TEXTURE_EDGE) as usize]
}

/// What a capture reports when the draw work never ran at all.
const DRAW_WORK_NEVER_RAN: &str = "the capture returned a frame without ever running the draw work, so every pixel read back \
     would be about a target nothing drew into";

/// The frame `snapshot` draws, at the declared capture size.
fn captured(
    context: &CaptureContext,
    renderer: &mut TerrainRenderer,
    snapshot: &TerrainSnapshot,
    named: &str,
) -> Result<Rgba8Image, Box<dyn Error>> {
    let phase = ScenePhase::Ready(Arc::clone(&snapshot.scene));
    let mut ran = false;
    let image;
    {
        let mut work = draw_fn(|encoder, color| {
            let target = RecordTarget {
                device: context.device(),
                queue: context.queue(),
                encoder,
                color,
                size: CAPTURE_SIZE,
            };
            renderer.record_terrain(target, &phase, snapshot)?;
            ran = true;
            Ok(())
        });
        image = context.capture(&request(context, named)?, &mut work)?.image;
    }
    if !ran {
        return Err(DRAW_WORK_NEVER_RAN.into());
    }
    Ok(image)
}
