//! The HUD fixtures this suite composes, and the two ways it gets a frame.
//!
//! Split out of the scenario file by responsibility: what a declaration looks
//! like, what a terrain fixture looks like, and how a frame is captured are one
//! job, and the scenarios in `hud_offscreen.rs` are another.
//!
//! **Both frame paths are here on purpose.** [`render_frame`] drives
//! `record_frame`, which is the client's own frame call and therefore the path
//! a scenario about the product has to take. [`compose_over`] drives
//! `compose_hud` onto a target this suite cleared itself, which is what the
//! composition entry point is public for: the terrain pass clears to the
//! declared sky, and a composite stated over black needs a backdrop the sky
//! cannot supply.

use std::error::Error;
use std::sync::Arc;

use mc_core::hud::source::InMemoryHudSource;
use mc_core::hud::{DeclaredValue, HudLayout, HudOrigin, RawHudElement};
use mc_core::id::BlockName;
use mc_render::camera::{CameraView, camera_view};
use mc_render::color::srgb8_to_linear;
use mc_render::geometry::SectionOrigin;
use mc_render::gpu::{FrameRenderer, FrameSnapshot, RecordTarget};
use mc_render::hud::HudFrame;
use mc_render::pass::TerrainPassConfig;
use mc_render::snapshot::ScenePhase;
use mc_render::surface::SurfaceSize;
use mc_render::texture::TextureResolution;
use mc_testkit::frame::gpu::{CaptureContext, draw_fn};
use mc_testkit::frame::{Rgba8Image, wgpu};
use mc_world::mesh::{Facing, PlaneExtent, PlanePos, Quad};

use super::Fixture;
use super::frame::require;

/// The target every scenario here composes onto: the reference height, where
/// one UI unit is exactly one physical pixel and the derived rectangles the
/// scenarios state are the declared numbers.
pub const REFERENCE: SurfaceSize = SurfaceSize {
    width: 1280,
    height: 720,
};

/// The tick the terrain fixture is rendered at. Nothing asserts it; it is what a
/// snapshot carries.
const TICK: u32 = 60;

/// One declaration as this suite writes it: an anchored, filled rectangle with
/// an optional outline.
///
/// Every element is anchored `center`, because what these scenarios grade is
/// composition rather than placement — where an anchor puts a rectangle is
/// settled by the pure derivation and its own suite.
#[derive(Debug, Clone, Copy)]
pub struct Declared {
    pub name: &'static str,
    pub size: [i64; 2],
    pub color: &'static str,
    pub outline: Option<&'static str>,
}

impl Declared {
    /// This declaration in the form a source hands over.
    fn raw(self) -> RawHudElement {
        let [across, down] = self.size;
        let mut fields = vec![
            ("name".to_owned(), text(self.name)),
            ("anchor".to_owned(), text("center")),
            (
                "size".to_owned(),
                DeclaredValue::List(vec![
                    DeclaredValue::Integer(across),
                    DeclaredValue::Integer(down),
                ]),
            ),
            ("draw".to_owned(), text("fill")),
            ("color".to_owned(), text(self.color)),
        ];
        if let Some(outline) = self.outline {
            fields.push(("outline".to_owned(), text(outline)));
        }
        RawHudElement::new(fields)
    }
}

fn text(spelled: &str) -> DeclaredValue {
    DeclaredValue::Text(spelled.to_owned())
}

/// A frame whose layout holds exactly `declarations`, each attributed to the
/// origin beside it and in the order given.
///
/// # Errors
///
/// Fails when the layout refused a declaration or registered a different number
/// of them: a fixture that registered nothing makes every "equal to the
/// zero-element frame" assertion below vacuous.
pub fn hud_frame(declarations: &[(&str, Declared)]) -> Result<HudFrame, Box<dyn Error>> {
    let stated = declarations
        .iter()
        .map(|(origin, declared)| (HudOrigin::new(*origin), declared.raw()))
        .collect();
    let layout = HudLayout::load(&InMemoryHudSource::new(
        HudOrigin::new("this suite"),
        stated,
    ))?;
    require(
        layout.elements().len() == declarations.len(),
        format!(
            "this fixture has to register all {} of its declarations, or what is composed from it \
             is not what it states, but it registered {}",
            declarations.len(),
            layout.elements().len()
        ),
    )?;
    Ok(HudFrame {
        layout: Arc::new(layout),
        held: None,
    })
}

/// What a capture reports when the draw work never ran at all.
const DRAW_WORK_NEVER_RAN: &str = "the capture returned a frame without ever running the draw work, so the pixels below \
     would be the harness's own blank target rather than anything this suite composed";

/// One frame of `hud` over `fixture`, recorded through the client's own frame
/// call.
pub fn render_frame(
    context: &CaptureContext,
    fixture: &Fixture,
    hud: &HudFrame,
    name: &str,
) -> Result<Rgba8Image, Box<dyn Error>> {
    let mut renderer = FrameRenderer::new(
        context.device(),
        context.queue(),
        &TerrainPassConfig::offscreen(),
        &super::production_textures(),
    )?;
    renderer.upload_textures(context.queue(), &fixture.resolution)?;
    renderer.upload_scene(context.queue(), &fixture.scene)?;
    let request = super::request(context, name, REFERENCE)?;
    let snapshot = super::snapshot(TICK, wall_camera(), fixture);
    let phase = ScenePhase::Ready(Arc::clone(&fixture.scene));
    let frame = FrameSnapshot {
        terrain: &snapshot,
        hud,
        // Every scenario in this suite is about what a *content declaration*
        // composes, and the debug overlay is not one — it is engine tooling no
        // declaration can reach. A readout here would put toolkit-rasterised
        // text over the rectangles these fixtures measure.
        overlay: None,
    };
    let mut recorded = false;
    let captured;
    {
        let mut work = draw_fn(|encoder, color| {
            renderer.record_frame(record_into(context, encoder, color), &phase, &frame)?;
            recorded = true;
            Ok(())
        });
        captured = context.capture(&request, &mut work)?;
    }
    require(recorded, DRAW_WORK_NEVER_RAN.to_owned())?;
    Ok(captured.image)
}

/// The same scene through the terrain pass alone, with no HUD stage at all.
pub fn terrain_alone(
    context: &CaptureContext,
    fixture: &Fixture,
    name: &str,
) -> Result<Rgba8Image, Box<dyn Error>> {
    let mut renderer = super::prepared_renderer(context, fixture)?;
    let request = super::request(context, name, REFERENCE)?;
    let snapshot = super::snapshot(TICK, wall_camera(), fixture);
    Ok(super::render(context, &mut renderer, &snapshot, &request)?.image)
}

/// One frame of `hud` composed onto a target this suite cleared to `backdrop`.
///
/// Through `compose_hud` rather than through a whole frame, which is the reason
/// that entry point is public: the terrain pass clears to the declared sky, and
/// a composite stated over black needs a backdrop the sky cannot supply.
pub fn compose_over(
    context: &CaptureContext,
    hud: &HudFrame,
    backdrop: [u8; 3],
    name: &str,
) -> Result<Rgba8Image, Box<dyn Error>> {
    let mut renderer = FrameRenderer::new(
        context.device(),
        context.queue(),
        &TerrainPassConfig::offscreen(),
        &super::production_textures(),
    )?;
    renderer.upload_textures(context.queue(), &TextureResolution::default())?;
    let request = super::request(context, name, REFERENCE)?;
    let mut composed = false;
    let captured;
    {
        let mut work = draw_fn(|encoder, color| {
            clear_to(encoder, color, backdrop);
            renderer.compose_hud(record_into(context, encoder, color), hud)?;
            composed = true;
            Ok(())
        });
        captured = context.capture(&request, &mut work)?;
    }
    require(composed, DRAW_WORK_NEVER_RAN.to_owned())?;
    Ok(captured.image)
}

/// The target one pass records into.
fn record_into<'a>(
    context: &'a CaptureContext,
    encoder: &'a mut wgpu::CommandEncoder,
    color: &'a wgpu::TextureView,
) -> RecordTarget<'a> {
    RecordTarget {
        device: context.device(),
        queue: context.queue(),
        encoder,
        color,
        size: REFERENCE,
    }
}

/// Clears `color` to `backdrop`, in the linear space `wgpu` takes a clear value
/// in and the target encodes back out of.
fn clear_to(encoder: &mut wgpu::CommandEncoder, color: &wgpu::TextureView, backdrop: [u8; 3]) {
    let [red, green, blue] = srgb8_to_linear(backdrop);
    let attachments = [Some(wgpu::RenderPassColorAttachment {
        view: color,
        depth_slice: None,
        resolve_target: None,
        ops: wgpu::Operations {
            load: wgpu::LoadOp::Clear(wgpu::Color {
                r: red,
                g: green,
                b: blue,
                a: 1.0,
            }),
            store: wgpu::StoreOp::Store,
        },
    })];
    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("hud backdrop"),
        color_attachments: &attachments,
        ..wgpu::RenderPassDescriptor::default()
    });
}

/// The block the wall is made of. An `example:` namespace: a fixture borrowing a
/// shipped block name would be the engine describing itself in terms of content.
const WALL_BLOCK: &str = "example:wall";

/// One section-wide face, standing across the camera's view axis.
///
/// Real terrain rather than a clear-only phase: two frames that are both a bare
/// clear are equal for a reason that has nothing to do with a HUD, and a
/// footprint asserted against a uniform backdrop cannot tell a pass that
/// repainted the screen from one that painted a rectangle.
pub fn wall_scene() -> Result<Fixture, Box<dyn Error>> {
    let quad = Quad {
        facing: Facing::PosZ,
        plane: 8,
        origin: PlanePos {
            primary: 0,
            secondary: 0,
        },
        extent: PlaneExtent {
            primary: 16,
            secondary: 16,
        },
        block: BlockName::parse(WALL_BLOCK)?,
    };
    super::assemble(&[(SectionOrigin::new([0, 0, 0]), vec![quad])])
}

/// The eye the wall is seen from: square on to it, close enough that it fills
/// the frame's height and leaves sky at either side.
fn wall_camera() -> CameraView {
    camera_view([8.0, 8.0, 20.0], [8.0, 8.0, 0.0])
}
