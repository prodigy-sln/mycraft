//! The device, the scenes and the measuring instruments the offscreen terrain
//! suite is built from.
//!
//! Three things are settled here rather than in each test.
//!
//! **The device may be skipped, and only because the opt-in said so.** These
//! scenarios are the ones `spec.md` lets `MYCRAFT_ALLOW_NO_GPU` downgrade to an
//! announced skip. No test in this project sets an environment variable —
//! `set_var` is `unsafe` in edition 2024 — so the opt-in is *read*, through
//! [`OptIns::from_environment`], and the harness prints the notice itself.
//!
//! **The statistics have to escape the draw closure.** `capture` takes
//! `&mut dyn DrawWork` and hands back an image, so a `FrameStats` computed
//! inside the closure is otherwise trapped. [`render`] is the one place that
//! pattern is written, and the `ok_or` at the end of it is load-bearing: draw
//! work that never ran leaves `None`, and a test that proceeded on default
//! statistics would be exactly the green-but-blind shape this spec exists to
//! remove.
//!
//! **Every expected quantity is derived, never read back from the thing under
//! test.** The admitted set comes from the pure frustum function; the quad
//! totals come from the section table the scene was assembled from; the colours
//! come from `placeholder_mean_color`, which is a *declaration* — the deliberately
//! implausible teal-and-tan palette is correct and nothing here compares a frame
//! against what a block "should" look like.

// Each test binary links this whole module and uses a subset of it.
#![allow(dead_code)]

pub mod frame;
pub mod hud;

use std::collections::BTreeSet;
use std::error::Error;
use std::sync::{Arc, OnceLock};

use mc_core::content::FaceTextures;
use mc_core::id::{BlockName, TextureKey};
use mc_render::camera::{CameraView, Frustum, projection_for, view_projection, visible_sections};
use mc_render::geometry::scene::SceneGeometry;
use mc_render::geometry::{SectionOrigin, build_section_geometry};
use mc_render::gpu::{RecordTarget, TerrainRenderer, TerrainTextures};
use mc_render::pass::TerrainPassConfig;
use mc_render::snapshot::{FrameStats, ScenePhase, TerrainSnapshot};
use mc_render::surface::SurfaceSize;
use mc_render::texture::sampler::TERRAIN_SAMPLER;
use mc_render::texture::supplied::SuppliedTexels;
use mc_render::texture::{TextureLayers, TextureResolution};
use mc_testkit::frame::gpu::{
    AcquireOptions, Acquisition, CAPTURE_FORMAT, CaptureContext, CaptureRequest, draw_fn,
};
use mc_testkit::frame::{
    CaptureId, OptIns, Rgba8Image, Thresholds, compare, validate_frame_size, wgpu,
};
use mc_world::mesh::{Facing, PlaneExtent, PlanePos, Quad};
use mc_world::section::SECTION_SIZE;

/// The error type every test in this suite propagates with `?`.
pub type TestResult = Result<(), Box<dyn Error>>;

/// What a capture reports when the draw work never ran at all.
const DRAW_WORK_NEVER_RAN: &str = "the capture returned a frame without ever running the draw work, so no frame statistics \
     exist and every assertion below would be about a default value";

/// Where the replay's camera stands at tick 60, and what it looks at.
///
/// Declared in `spec.md`'s binding table and written out here rather than read
/// from the simulation: the renderer may not resolve `mc-sim` in any dependency
/// kind, and a camera taken from the code under test would not be an independent
/// statement of where tick 60 puts the eye.
pub const REPLAY_EYE_AT_TICK_60: [f32; 3] = [-64.0, 56.0, 32.0];
pub const REPLAY_LOOK_AT: [f32; 3] = [32.0, 44.0, 32.0];

/// The frame a test that reads counts rather than pixels renders into.
///
/// 16:9, exactly the declared aspect — `1280 / 720` and `128 / 72` are the same
/// rational, so the projection this size derives is the projection the declared
/// capture size derives, at a fraction of the pixels.
pub const COUNTING_FRAME: SurfaceSize = SurfaceSize {
    width: 128,
    height: 72,
};

/// The frame the depth scenarios render into. Square, so the centre pixel sits
/// on the view axis on both axes.
pub const DEPTH_FRAME: SurfaceSize = SurfaceSize {
    width: 256,
    height: 256,
};

/// How far apart two colours may be and still be called the same texture.
///
/// Every placeholder texel sits about ΔE 3.6 from its layer's declared mean and
/// the declared means used here stand ΔE 96 apart, so this separates them by a
/// wide margin without asserting anything about filtering.
pub const SAME_TEXTURE: f64 = 10.0;

/// The device this suite draws on, or `None` when the opt-in permitted its
/// absence.
///
/// # Errors
///
/// Returns the acquisition failure when no adapter answered and the opt-in did
/// not permit saying so.
pub fn device() -> Result<Option<Box<CaptureContext>>, Box<dyn Error>> {
    match CaptureContext::acquire(&OptIns::from_environment(), &AcquireOptions::default())? {
        Acquisition::Ready(context) => Ok(Some(context)),
        Acquisition::Skipped(_) => Ok(None),
    }
}

/// A request for a `size` capture named `name`, on `context`'s device.
///
/// # Errors
///
/// Returns the name failure for an invalid capture name, or the size failure
/// when the device cannot render a frame that large.
pub fn request(
    context: &CaptureContext,
    name: &str,
    size: SurfaceSize,
) -> Result<CaptureRequest, Box<dyn Error>> {
    let maximum = context.limits().max_texture_dimension_2d;
    let frame = validate_frame_size(size.width, size.height, maximum)?;
    Ok(CaptureRequest::new(CaptureId::new(name)?, frame))
}

/// A scene, and the texture layers its blocks resolved to.
#[derive(Debug)]
pub struct Fixture {
    pub scene: Arc<SceneGeometry>,
    pub resolution: TextureResolution,
}

/// Assembles `sections` — each an origin and the quads it shows — into one
/// scene, in the order given.
///
/// # Errors
///
/// Returns the parse, geometry or assembly failure.
pub fn assemble(sections: &[(SectionOrigin, Vec<Quad>)]) -> Result<Fixture, Box<dyn Error>> {
    let mut keys = BTreeSet::new();
    let mut stated = Vec::new();
    for quad in sections.iter().flat_map(|(_, quads)| quads.iter()) {
        // These fixtures declare `texture` equal to `name`, which is what every
        // block in this repository does; the readings that are about the two
        // differing state them apart for themselves.
        let key = TextureKey::parse(quad.block.as_str())?;
        keys.insert(key.clone());
        stated.push((quad.block.clone(), FaceTextures::uniform(key)));
    }
    let resolution = TextureResolution::stating(stated, TextureLayers::resolve(&keys));
    let built = sections
        .iter()
        .map(|(origin, quads)| build_section_geometry(quads, *origin, &resolution))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Fixture {
        scene: Arc::new(SceneGeometry::assemble(built)?),
        resolution,
    })
}

/// What the composition root hands a renderer when no content root has offered
/// any texels: the terrain sampler, and an empty supply.
///
/// Every layer is then filled from the generator, which is what this crate's own
/// fixtures want — they name `example:` blocks that no built set covers, and a
/// renderer test may not read a content root at all.
#[must_use]
pub fn production_textures() -> TerrainTextures<'static> {
    TerrainTextures {
        supplied: NO_TEXELS.get_or_init(SuppliedTexels::none),
        sampler: TERRAIN_SAMPLER,
    }
}

/// The empty supply every renderer above borrows from.
static NO_TEXELS: OnceLock<SuppliedTexels> = OnceLock::new();

/// A renderer with `fixture`'s textures and scene already uploaded, drawing
/// through the sampler the composition root asks for and supplied no texels.
///
/// Every scenario but the two about sampling wants exactly this: the production
/// request, and layers filled from the generator. Those two reach for
/// [`prepared_renderer_through`] instead, and the fact that they are the only
/// two is what keeps the second sampler configuration from leaking into readings
/// that are about something else.
///
/// # Errors
///
/// Returns the pipeline or upload failure.
pub fn prepared_renderer(
    context: &CaptureContext,
    fixture: &Fixture,
) -> Result<TerrainRenderer, Box<dyn Error>> {
    prepared_renderer_through(context, fixture, &production_textures())
}

/// That same renderer, built through `textures` — a sampler request and the
/// texels its array texture is filled from.
///
/// **The parameter is the decision, not a convenience.** `buffers` is private
/// and its sampler is a free function taking no request, so without a request
/// threaded from here nothing under `tests/` can build a renderer that samples
/// any other way — and a scenario comparing filtered minification against
/// unfiltered minification cannot be written at all.
///
/// # Errors
///
/// Returns the pipeline, sampler or upload failure.
pub fn prepared_renderer_through(
    context: &CaptureContext,
    fixture: &Fixture,
    textures: &TerrainTextures<'_>,
) -> Result<TerrainRenderer, Box<dyn Error>> {
    let mut renderer = TerrainRenderer::new(
        context.device(),
        context.queue(),
        &TerrainPassConfig::offscreen(),
        textures,
    )?;
    renderer.upload_textures(context.queue(), fixture.resolution.layers())?;
    renderer.upload_scene(context.queue(), &fixture.scene)?;
    Ok(renderer)
}

/// One frame: what it reported, and what it looked like.
#[derive(Debug)]
pub struct Rendered {
    pub stats: FrameStats,
    pub image: Rgba8Image,
}

/// Records `snapshot`'s terrain into a capture and hands back both halves.
///
/// The `ok_or` below is the load-bearing line: it is what turns "the draw work
/// never ran" into a failure instead of into a default `FrameStats` nobody
/// looked at.
///
/// # Errors
///
/// Returns the recording failure the renderer reported, the capture failure, or
/// the absence of any statistics at all.
pub fn render(
    context: &CaptureContext,
    renderer: &mut TerrainRenderer,
    snapshot: &TerrainSnapshot,
    request: &CaptureRequest,
) -> Result<Rendered, Box<dyn Error>> {
    let size = SurfaceSize {
        width: request.size.width(),
        height: request.size.height(),
    };
    let phase = ScenePhase::Ready(Arc::clone(&snapshot.scene));
    let mut stats = None;
    let captured;
    {
        let mut work = draw_fn(|encoder, color| {
            let target = RecordTarget {
                device: context.device(),
                queue: context.queue(),
                encoder,
                color,
                size,
            };
            stats = Some(renderer.record_terrain(target, &phase, snapshot)?);
            Ok(())
        });
        captured = context.capture(request, &mut work)?;
    }
    let stats = stats.ok_or(DRAW_WORK_NEVER_RAN)?;
    Ok(Rendered {
        stats,
        image: captured.image,
    })
}

/// A colour target of `size` that nothing reads back.
///
/// For the one scenario that is about a recording failure: the pass never
/// reaches submission, so the frame is a place for the encoder to point rather
/// than a picture.
#[must_use]
pub fn unread_target(device: &wgpu::Device, size: SurfaceSize) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("terrain recording failure"),
        size: wgpu::Extent3d {
            width: size.width,
            height: size.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: CAPTURE_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    })
}

/// A snapshot of `fixture` at `tick`, seen from `camera`.
#[must_use]
pub fn snapshot(tick: u32, camera: CameraView, fixture: &Fixture) -> TerrainSnapshot {
    TerrainSnapshot {
        tick,
        camera,
        scene: Arc::clone(&fixture.scene),
    }
}

/// The section indices the pure frustum function admits for `camera` at `size`.
///
/// The independent answer every GPU-side count in this suite is judged against.
#[must_use]
pub fn admitted(scene: &SceneGeometry, camera: &CameraView, size: SurfaceSize) -> Vec<u32> {
    let frustum = Frustum::from_view_projection(&view_projection(camera, &projection_for(size)));
    visible_sections(&frustum, scene.sections())
}

/// How many quads `indices` name, summed over the scene's section table.
#[must_use]
pub fn quads_of(scene: &SceneGeometry, indices: &[u32]) -> u32 {
    indices
        .iter()
        .filter_map(|index| scene.sections().get(*index as usize))
        .map(|section| section.quad_count)
        .sum()
}

/// How many quads the whole scene holds.
#[must_use]
pub fn quads_of_everything(scene: &SceneGeometry) -> u32 {
    scene
        .sections()
        .iter()
        .map(|section| section.quad_count)
        .sum()
}

/// One solid block at `local`, as the six faces it shows.
///
/// A quad's `plane` is the emitting voxel's own coordinate on the facing's axis,
/// and its in-plane axes are the other two in `x < y < z` order — so an `X` face
/// runs primary along y and secondary along z, a `Y` face primary along x and
/// secondary along z, and a `Z` face primary along x and secondary along y.
#[must_use]
pub fn solid_block(local: [u32; 3], block: &BlockName) -> Vec<Quad> {
    let [x, y, z] = local;
    [
        (Facing::NegX, x, y, z),
        (Facing::PosX, x, y, z),
        (Facing::NegY, y, x, z),
        (Facing::PosY, y, x, z),
        (Facing::NegZ, z, x, y),
        (Facing::PosZ, z, x, y),
    ]
    .into_iter()
    .map(|(facing, plane, primary, secondary)| Quad {
        facing,
        plane,
        origin: PlanePos { primary, secondary },
        extent: PlaneExtent {
            primary: 1,
            secondary: 1,
        },
        block: block.clone(),
    })
    .collect()
}

/// One to three upward faces, so that sections do not all hold the same number
/// of quads.
///
/// A uniform quad count would let a compaction that gathered the *wrong*
/// sections still produce the right index total whenever it gathered the right
/// number of them.
#[must_use]
pub fn filler_quads(index: usize, block: &BlockName) -> Vec<Quad> {
    (0..=(index % 3) as u32)
        .map(|plane| Quad {
            facing: Facing::PosY,
            plane,
            origin: PlanePos {
                primary: 0,
                secondary: 0,
            },
            extent: PlaneExtent {
                primary: 1,
                secondary: 1,
            },
            block: block.clone(),
        })
        .collect()
}

/// How far apart a grid's sections stand: one section, exactly.
const STRIDE: i32 = SECTION_SIZE as i32;

/// Sixty-four section origins the declared tick-60 camera can see all of.
///
/// A 4 × 4 × 4 block of sections filling `x, z` in `0..64` and `y` in `16..80`,
/// which the eye at (−64, 56, 32) looks straight into.
#[must_use]
pub fn origins_in_view() -> Vec<[i32; 3]> {
    grid([0, STRIDE, 0])
}

/// Sixty-four section origins standing behind that same eye.
///
/// The eye sits at x = −64 looking towards +x, and these fill `x` in
/// `-256..-192`, so every one of them fails the near plane.
#[must_use]
pub fn origins_behind_the_camera() -> Vec<[i32; 3]> {
    grid([-256, STRIDE, 0])
}

/// Sixty-four origins of which exactly half stand in view, alternating.
///
/// Alternating rather than partitioned: a visibility buffer addressed one
/// section out of step still divides 64 into 32 and 32, and would agree with the
/// pure function about the total while disagreeing about every member.
#[must_use]
pub fn origins_half_in_view() -> Vec<[i32; 3]> {
    origins_in_view()
        .into_iter()
        .zip(origins_behind_the_camera())
        .flat_map(|(seen, hidden)| [seen, hidden])
        .take(64)
        .collect()
}

/// A 4 × 4 × 4 arrangement of section origins, offset by `corner`.
fn grid(corner: [i32; 3]) -> Vec<[i32; 3]> {
    let [x, y, z] = corner;
    (0..4)
        .flat_map(move |i| {
            (0..4).flat_map(move |j| {
                (0..4).map(move |k| [x + i * STRIDE, y + j * STRIDE, z + k * STRIDE])
            })
        })
        .collect()
}

/// A grid scene: `origins`, each holding filler quads of `block`.
///
/// # Errors
///
/// Returns the parse, geometry or assembly failure.
pub fn grid_scene(origins: &[[i32; 3]], block: &str) -> Result<Fixture, Box<dyn Error>> {
    let block = BlockName::parse(block)?;
    let sections = origins
        .iter()
        .enumerate()
        .map(|(index, origin)| (SectionOrigin::new(*origin), filler_quads(index, &block)))
        .collect::<Vec<_>>();
    assemble(&sections)
}

/// The perceptual distance between two colours, measured by the harness's own
/// metric.
///
/// Driven through `compare` on a pair of one-pixel frames rather than
/// reimplemented: `delta_e` is the single place a distance is computed in this
/// project, and a second copy would let goldens and probes judge by different
/// metrics the day CIE76 is replaced.
///
/// # Errors
///
/// Returns the image-shape failure, which a 1 × 1 frame cannot produce.
pub fn delta_e(left: [u8; 3], right: [u8; 3]) -> Result<f64, Box<dyn Error>> {
    let one = uniform(1, 1, left)?;
    let other = uniform(1, 1, right)?;
    Ok(compare(&one, &other, &Thresholds::default()).max_delta_e)
}

/// A frame of `width` × `height` filled with `color`.
///
/// # Errors
///
/// Returns the image-shape failure when the dimensions do not match the pixels.
pub fn uniform(width: u32, height: u32, color: [u8; 3]) -> Result<Rgba8Image, Box<dyn Error>> {
    let [red, green, blue] = color;
    let pixels = std::iter::repeat_n([red, green, blue, 255], (width * height) as usize)
        .flatten()
        .collect();
    Ok(Rgba8Image::from_rgba(width, height, pixels)?)
}

/// How many of `frame`'s pixels sit further than `tolerance` from `color`.
///
/// # Errors
///
/// Returns the threshold or image-shape failure.
pub fn pixels_away_from(
    frame: &Rgba8Image,
    color: [u8; 3],
    tolerance: f64,
) -> Result<u64, Box<dyn Error>> {
    let field = uniform(frame.width(), frame.height(), color)?;
    let thresholds = Thresholds::new(tolerance, 1.0, f64::MAX)?;
    Ok(compare(&field, frame, &thresholds).failing_pixels)
}

/// The colour at the middle of `frame`, alpha dropped.
///
/// # Errors
///
/// Returns a failure when the frame has no pixel there, which an empty frame is
/// the only way to arrange.
pub fn centre_pixel(frame: &Rgba8Image) -> Result<[u8; 3], Box<dyn Error>> {
    let pixel = frame
        .pixel(frame.width() >> 1, frame.height() >> 1)
        .ok_or("the captured frame has no centre pixel")?;
    let [red, green, blue, _] = pixel;
    Ok([red, green, blue])
}
