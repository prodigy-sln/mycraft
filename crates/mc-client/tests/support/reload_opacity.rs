//! A pane of one flat colour lying on a floor of another, played by a client
//! whose content root a mod author edits while it runs.
//!
//! # What is the fixture's and what is the shipped path
//!
//! Two things are the fixture's: the **declarations** — a copy of the shipped
//! root holding nothing but the two blocks a reading names — and the **flat
//! layers** the array texture is filled from. Everything between is the
//! product's own: `mc_sim::content::load` reads the root, the client watches it,
//! a tick boundary builds the candidate and reports what it made of it,
//! `ContentView::of` turns what is now serving into a resolution, `scene_of`
//! packs the sections against it and `TerrainRenderer` draws them.
//!
//! **The world is meshed once and never again.** The same `Vec<SectionQuads>`
//! is packed before and after the reload, which is what the architecture claims
//! a changed degree costs — a re-pack and no re-mesh. A fixture that re-meshed
//! between the two would be asserting about a path a running client never takes
//! for this edit, and would hide a packer that resolved the degree at mesh time.
//!
//! # Where the reading is taken, and the half it cannot reach
//!
//! `crates/mc-client/src/app/` needs a real window that nothing in this
//! workspace constructs, so the value is read where it crosses out of the
//! client's core: the resolution the accepted reload's own report handed over,
//! and — after a refusal — the resolution the client is still publishing. That
//! is `crate::reload_upload`'s rule and it is inherited unchanged. What no test
//! here can reach is the upload `App` performs with it.
//!
//! # Both layers are one flat colour, and both bounds of the tolerance
//!
//! A blend expectation is an exact triple only if its two operands are, so each
//! layer is filled with a single colour — which also takes minification out of
//! the question, since nearest magnification, linear minification and every mip
//! level answer the same byte for a texture of one colour.
//!
//! The tolerance is [`crate::support::translucency::TELLS_THEM_APART`], and both
//! halves of its derivation hold here. Its **floor** is a property of the
//! mechanism rather than of that fixture's palette: a flat layer's texel spread
//! is 0.00, the degree is quantised to a byte before a fragment reads it, and an
//! 8-bit sRGB attachment rounds — the three terms it states. Its **ceiling** is
//! half the closest separation a reading has to preserve, and that is not
//! inherited on faith: `require_told_apart` measures it over *these* four
//! colours on every run.
//!
//! # The eye, and why every ray meets both surfaces
//!
//! The pane lies in the layer directly over the floor, so a ray crosses the
//! pane's upward face at `y = 11` and meets the floor's own upward face at
//! `y = 10` one block later — the floor's face is emitted because the pane
//! declares `occludes = false`, and the pane's downward face is not because the
//! floor occludes.
//!
//! The eye stands one block over that face and `tan 30° = 0.577` blocks along
//! `+x`, so the view is 30° off vertical: straight down, a look-at matrix has no
//! unique answer because the view direction is the world's own up. At 1.155
//! blocks from the face and a sixty-degree field of view over a square frame,
//! the half-extent is `1.155 · tan 30° = 0.667` blocks each way, and the widest
//! corner ray reaches the floor's face 3.46 blocks along `x` and 2.0 along `z`
//! from the eye — well inside a column that runs 0 to 16 in both. So **every
//! pixel of the frame is the pane over the floor**, and the frame holding any of
//! the other three named colours is a defect rather than an edge.
//!
//! # Why this is reached by `#[path]` and not declared inside `support`
//!
//! It names the reload fixtures, which are themselves reached that way. A binary
//! including this must declare `mod support;`, the input harness,
//! [`crate::reload`], [`crate::reload_remesh`], [`crate::reload_upload`],
//! [`crate::reload_watch`] and [`crate::reload_world`] as well.

// Each scenario binary links this whole module and drives a subset of it.
#![allow(dead_code)]

use std::error::Error;
use std::sync::Arc;

use glam::Vec3;
use mc_client::startup::scene_of;
use mc_core::block::BlockRegistry;
use mc_core::id::{BlockName, TextureKey};
use mc_render::camera::camera_view;
use mc_render::geometry::scene::SceneGeometry;
use mc_render::gpu::{RecordTarget, TerrainRenderer, TerrainTextures};
use mc_render::pass::TerrainPassConfig;
use mc_render::snapshot::{ScenePhase, TerrainSnapshot};
use mc_render::surface::SurfaceSize;
use mc_render::texture::TextureResolution;
use mc_render::texture::sampler::TERRAIN_SAMPLER;
use mc_render::texture::supplied::SuppliedTexels;
use mc_sim::replay::SectionQuads;
use mc_sim::world::World;
use mc_testkit::frame::gpu::{CaptureContext, CaptureRequest, draw_fn};
use mc_testkit::frame::{CaptureId, Rgba8Image, validate_frame_size};
use mc_world::world::{VoxelWorld, WorldPos};

use crate::input::InputHarness;
use crate::reload_watch::{Reports, a_client_over};
use crate::reload_world::{ACROSS, FLOOR, floor_of, registry_of, standing_at};
use crate::support::content::{ContentRoot, shipped_copy};
use crate::support::pixel_census::Expected;

/// The two blocks these readings declare, and the files they are declared in.
pub const WALL: &str = "example:wall";
pub const PANE: &str = "example:pane";
pub const WALL_FILE: &str = "example__wall.luau";
pub const PANE_FILE: &str = "example__pane.luau";

/// The one flat colour each block's layer is filled with.
pub const WALL_COLOUR: [u8; 3] = [32, 200, 90];
pub const PANE_COLOUR: [u8; 3] = [235, 120, 40];

/// The degree a mod author edits the pane to, and the degree that means the
/// pane stops all the light reaching it.
pub const HALF: f32 = 0.5;
pub const WHOLE: f32 = 1.0;

/// A degree half again past the ceiling — the value somebody writes reaching for
/// "more opaque than opaque", which is what a percentage-shaped intuition
/// produces on a scale that runs to one.
pub const PAST_THE_CEILING: f32 = 1.5;

/// The layer the pane lies in: the one directly over the floor.
const PANE_LAYER: i32 = FLOOR + 1;

/// Where the player stands — on the pane's upward face, out of every block.
///
/// The camera below is a free one, so the player is here only because a client
/// needs somewhere to be: a spawn inside the pane's own cell would have the
/// client legitimately move them, which is a second thing happening in a frame
/// these readings want to be about one.
const ON_THE_PANE: Vec3 = Vec3::new(8.5, 11.0, 8.5);

/// The frame these readings are drawn into.
///
/// Square, and large enough that the one region it holds covers thousands of
/// pixels against the hundred a reading asks for. Not the declared capture size:
/// nothing here is a golden and nothing here is shot through the player's
/// camera.
pub const FRAME: SurfaceSize = SurfaceSize {
    width: 256,
    height: 256,
};

/// How many pixels one of these frames holds.
pub const PIXELS_IN_THE_FRAME: u64 = (FRAME.width as u64) * (FRAME.height as u64);

/// Where the eye stands and what it looks at — derived in this module's header.
pub const EYE: [f32; 3] = [8.577, 12.0, 8.0];
pub const LOOK_AT: [f32; 3] = [8.0, 11.0, 8.0];

/// What a census calls each of the four colours a frame here may hold.
pub const THE_PANE_OVER_THE_WALL: &str = "the floor seen through the pane";
pub const THE_PANE_ITSELF: &str = "the pane's own colour, unblended";
pub const THE_WALL_ITSELF: &str = "the floor's own colour, with no pane over it";

/// The four colours a frame here may hold, the blend among them named at
/// [`HALF`].
///
/// **The blend is always the half blend, whatever the pane currently declares.**
/// It is the colour the edit under test moves to and away from, so both
/// directions are read against one list: a reading about a pane returned to a
/// whole degree needs the half blend named in order to say it is *not* there.
/// Naming it at the degree in force would collapse two of the four onto each
/// other at a whole degree, which `require_told_apart` would refuse.
#[must_use]
pub fn the_four_colours() -> [Expected; 4] {
    [
        Expected::blend(
            THE_PANE_OVER_THE_WALL,
            PANE_COLOUR,
            WALL_COLOUR,
            f64::from(HALF),
        ),
        Expected::new(THE_PANE_ITSELF, PANE_COLOUR),
        Expected::new(THE_WALL_ITSELF, WALL_COLOUR),
        Expected::sky(),
    ]
}

/// The pane's declaration, stating `degree` where one is given and no `opacity`
/// line at all where none is.
///
/// **`occludes = false` is stated whatever the degree is, and that is the whole
/// reason this edit needs no re-mesh.** `occludes` falls back to `solid`, so a
/// pane that said nothing would resolve to a block that both passes light and
/// hides what lies behind it — refused, taking the root with it. Stating it at
/// every degree also keeps the two declarations differing in the degree and in
/// nothing else: a fixture that flipped `occludes` alongside would change which
/// faces the mesher emits, and a frame that moved for two reasons says nothing
/// about either.
#[must_use]
pub fn pane_declaring(degree: Option<f32>) -> String {
    let stated = match degree {
        // Debug rather than Display, so a whole number reaches the file as `1.0`
        // rather than as `1`. Both are numbers the loader accepts, and a fixture
        // that means "a degree" should not become one that means "an integer".
        Some(degree) => format!("\topacity = {degree:?},\n"),
        None => String::new(),
    };
    format!(
        "return {{\n\tname = \"{PANE}\",\n\ttexture = \"{PANE}\",\n\tsolid = true,\n\toccludes = \
         false,\n{stated}}}\n"
    )
}

/// The floor's declaration: an ordinary opaque block, saying nothing about
/// either occlusion or opacity.
#[must_use]
fn wall_declaration() -> String {
    format!("return {{\n\tname = \"{WALL}\",\n\ttexture = \"{WALL}\",\n\tsolid = true,\n}}\n")
}

/// A copy of the shipped root declaring these two blocks and no others, the pane
/// at `degree`.
///
/// # Errors
///
/// Returns the copy or write failure.
pub fn a_root_whose_pane_declares(degree: Option<f32>) -> Result<ContentRoot, Box<dyn Error>> {
    shipped_copy()?
        .declaring_no_blocks()?
        .declaring_block(WALL_FILE, &wall_declaration())?
        .declaring_block(PANE_FILE, &pane_declaring(degree))
}

/// One column whose floor is the wall and whose next layer up is the pane.
///
/// # Errors
///
/// Returns an error if a name does not parse or the world refuses a write.
pub fn a_pane_lying_on_the_floor(registry: &BlockRegistry) -> Result<VoxelWorld, Box<dyn Error>> {
    let mut blocks = floor_of(registry, WALL)?;
    let pane = BlockName::parse(PANE)?;
    let layer = u32::try_from(PANE_LAYER)?;
    for x in 0..ACROSS {
        for z in 0..ACROSS {
            blocks.set_block(WorldPos { x, y: layer, z }, &pane, registry)?;
        }
    }
    Ok(blocks)
}

/// A client playing that world over `root`, the handle its content changes are
/// reported through, and the sections it meshed at launch.
pub struct Playing {
    pub client: InputHarness,
    pub reports: Reports,
    /// Meshed once, packed on every reading. See this module's header.
    pub meshed: Vec<SectionQuads>,
}

/// A client launched on `root`, standing on the pane.
///
/// # Errors
///
/// Returns an error if the root does not read, if the world does not build, or
/// if the sections do not mesh.
pub fn a_client_playing(root: &ContentRoot) -> Result<Playing, Box<dyn Error>> {
    let at_launch = registry_of(root.path())?;
    let blocks = a_pane_lying_on_the_floor(&at_launch)?;
    let meshed = World::new(blocks, at_launch)?.mesh()?;
    let (client, reports) = a_client_over(root, standing_at(ON_THE_PANE), |registry| {
        a_pane_lying_on_the_floor(registry)
    })?;
    Ok(Playing {
        client,
        reports,
        meshed,
    })
}

/// The frame `meshed` draws when packed against `resolution`, or `None` when the
/// opt-in permitted the absence of a device.
///
/// # Errors
///
/// Returns the packing, pipeline, upload or capture failure.
pub fn drawn_against(
    meshed: &[SectionQuads],
    resolution: &TextureResolution,
) -> Result<Option<Rgba8Image>, Box<dyn Error>> {
    let scene = Arc::new(scene_of(meshed, resolution)?);
    let texels = flat_layers()?;
    let Some(context) = crate::support::frames::device()? else {
        return Ok(None);
    };
    let mut renderer = TerrainRenderer::new(
        context.device(),
        context.queue(),
        &TerrainPassConfig::offscreen(),
        &TerrainTextures {
            supplied: &texels,
            sampler: TERRAIN_SAMPLER,
        },
    )?;
    renderer.upload_textures(context.queue(), resolution.layers())?;
    renderer.upload_scene(context.queue(), &scene)?;
    Ok(Some(captured(&context, &mut renderer, &scene)?))
}

/// A layer of one colour for each of the two blocks.
fn flat_layers() -> Result<SuppliedTexels, Box<dyn Error>> {
    let mut stated = Vec::new();
    for (block, [red, green, blue]) in [(WALL, WALL_COLOUR), (PANE, PANE_COLOUR)] {
        stated.push((
            TextureKey::parse(block)?,
            vec![
                [red, green, blue, u8::MAX];
                (mc_core::content::TEXTURE_EDGE * mc_core::content::TEXTURE_EDGE) as usize
            ],
        ));
    }
    Ok(SuppliedTexels::stating(stated))
}

/// The frame `scene` draws through the declared eye.
fn captured(
    context: &CaptureContext,
    renderer: &mut TerrainRenderer,
    scene: &Arc<SceneGeometry>,
) -> Result<Rgba8Image, Box<dyn Error>> {
    let snapshot = TerrainSnapshot {
        tick: 0,
        camera: camera_view(EYE, LOOK_AT),
        scene: Arc::clone(scene),
    };
    let phase = ScenePhase::Ready(Arc::clone(scene));
    let mut ran = false;
    let image;
    {
        let mut work = draw_fn(|encoder, color| {
            let target = RecordTarget {
                device: context.device(),
                queue: context.queue(),
                encoder,
                color,
                size: FRAME,
            };
            renderer.record_terrain(target, &phase, &snapshot)?;
            ran = true;
            Ok(())
        });
        image = context.capture(&request(context)?, &mut work)?.image;
    }
    if !ran {
        return Err(DRAW_WORK_NEVER_RAN.into());
    }
    Ok(image)
}

/// What a capture reports when the draw work never ran at all.
const DRAW_WORK_NEVER_RAN: &str = "the capture returned a frame without ever running the draw work, so every pixel read back \
     would be about a target nothing drew into";

/// A request for one of these frames.
fn request(context: &CaptureContext) -> Result<CaptureRequest, Box<dyn Error>> {
    let maximum = context.limits().max_texture_dimension_2d;
    let size = validate_frame_size(FRAME.width, FRAME.height, maximum)?;
    Ok(CaptureRequest::new(
        CaptureId::new("reloaded-opacity")?,
        size,
    ))
}
