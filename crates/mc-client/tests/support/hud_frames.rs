//! One frame of the replay with a HUD composed over it, through the client's
//! own frame call.
//!
//! **A sibling of [`frames`](super::frames) rather than an edit of it.** The
//! three committed terrain goldens are shot through the terrain pass alone and
//! are frozen; everything here goes through the one frame call the windowed
//! client makes, which is the only path a scenario about the product's HUD may
//! take. Keeping both is deliberate: the terrain set says the world is drawn
//! right, this one says the HUD reaches the frame the client actually draws, and
//! neither covers the other's half.
//!
//! **The zero-element frame is a layout that declares nothing, not a frame with
//! the HUD stage skipped.** Both are available, and this one is the harder
//! comparison: a stage that ran and painted nothing is what the scenarios here
//! are entitled to assume, so a difference they report cannot be explained by a
//! pass that never ran.

use std::error::Error;
use std::path::Path;
use std::sync::Arc;

use mc_client::startup::empty_scene;
use mc_core::block::BlockRegistry;
use mc_core::hud::source::InMemoryHudSource;
use mc_core::hud::{HudLayout, HudOrigin};
use mc_core::id::BlockName;
use mc_render::camera::waiting_view;
use mc_render::gpu::{FrameRenderer, FrameSnapshot, RecordTarget};
use mc_render::hud::{HudFrame, held_swatch};
use mc_render::pass::TerrainPassConfig;
use mc_render::snapshot::{ScenePhase, TerrainSnapshot};
use mc_render::texture::TextureLayers;
use mc_sim::action::default_held_block;
use mc_sim::replay::ReplayWorld;
use mc_testkit::frame::gpu::{
    CaptureContext, CaptureRequest, DrawWork, capture_and_verify, draw_fn,
};
use mc_testkit::frame::{GoldenOutcome, GoldenSettings, Rgba8Image};
use mc_world::content::TomlFileHudSource;

use super::PreparedScene;
use super::frames::CAPTURE_SIZE;

/// A rectangle of a frame, in physical pixels, the way a HUD plan states one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl Rect {
    /// Whether `(x, y)` falls inside this rectangle.
    #[must_use]
    pub const fn holds(self, x: u32, y: u32) -> bool {
        x >= self.x && x < self.x + self.width && y >= self.y && y < self.y + self.height
    }

    /// This rectangle grown by `margin` pixels on every side, held at the frame
    /// origin so a rectangle at the edge does not wrap into a huge one.
    #[must_use]
    pub const fn grown_by(self, margin: u32) -> Self {
        Self {
            x: self.x.saturating_sub(margin),
            y: self.y.saturating_sub(margin),
            width: self.width + 2 * margin,
            height: self.height + 2 * margin,
        }
    }

    /// How many pixels this rectangle covers.
    #[must_use]
    pub const fn area(self) -> u64 {
        self.width as u64 * self.height as u64
    }
}

/// What a capture reports when the draw work never ran at all.
const DRAW_WORK_NEVER_RAN: &str = "the capture returned a frame without ever running the draw work, so every pixel below \
     would be about a target nothing drew into";

/// The HUD the declarations under `root` compose to.
///
/// Read through the same source the client reads a content root with, so a
/// fixture root and the shipped root are read by one implementation.
///
/// # Errors
///
/// Returns the refusal when the root's declarations do not load. A scenario
/// about what a root draws has learned nothing from a root that was refused.
pub fn hud_of(root: &Path) -> Result<HudFrame, Box<dyn Error>> {
    Ok(HudFrame {
        layout: Arc::new(HudLayout::load(&TomlFileHudSource::new(root))?),
        held: None,
    })
}

/// The HUD the declarations under `root` compose, holding the block a client
/// reading `root` would hold.
///
/// **The client's own two answers rather than a stand-in for them.**
/// `default_held_block` is what decides which block a session starts holding and
/// `held_swatch` is what decides which layer that draws from, and those are the
/// two `App::draw` composes its own frame with. A capture that claimed to be the
/// client's frame while holding something else would be a frame of a client that
/// does not exist.
///
/// An unresolved swatch is not refused here, for the same reason the client does
/// not refuse one: the indicator is simply not drawn, and a scenario that needs
/// one drawn says so in its own fixture guard.
///
/// # Errors
///
/// Returns the refusal when the root's declarations do not load, or when the
/// prepared content registers no solid block for a client to hold.
pub fn hud_holding_default_block(
    root: &Path,
    content: &PreparedContent,
) -> Result<HudFrame, Box<dyn Error>> {
    let held = default_block_held(content)?;
    Ok(HudFrame {
        held: held_swatch(Some(&held), &content.layers).texture(),
        ..hud_of(root)?
    })
}

/// The block a client of `content` would hold, by the simulation's own policy.
///
/// # Errors
///
/// Returns a failure when the content registers no solid block, which is a
/// fixture that cannot hold anything rather than a client that holds nothing.
pub fn default_block_held(content: &PreparedContent) -> Result<BlockName, Box<dyn Error>> {
    default_held_block(&content.registry).ok_or_else(|| {
        "this content root has to register a solid block for a client to hold".into()
    })
}

/// A HUD that declares nothing at all.
///
/// Built in memory rather than from an empty directory on disk: what these
/// scenarios need is a layout holding no element, and routing that through a
/// filesystem would make the fixture depend on a directory read none of them is
/// about.
///
/// # Errors
///
/// Returns the refusal if a source declaring nothing is ever refused, which is
/// the one thing HUD loading is specified never to do.
pub fn no_hud() -> Result<HudFrame, Box<dyn Error>> {
    Ok(HudFrame {
        layout: Arc::new(HudLayout::load(&InMemoryHudSource::new(
            HudOrigin::new("a content root declaring no HUD"),
            Vec::new(),
        ))?),
        held: None,
    })
}

/// A renderer of the kind the windowed client owns, with the replay's array
/// texture and scene already uploaded.
///
/// # Errors
///
/// Returns the pipeline or upload failure.
pub fn prepared_renderer(
    context: &CaptureContext,
    prepared: &PreparedScene,
) -> Result<FrameRenderer, Box<dyn Error>> {
    let mut renderer = FrameRenderer::new(
        context.device(),
        context.queue(),
        &TerrainPassConfig::offscreen(),
    )?;
    renderer.upload_textures(context.queue(), &prepared.layers)?;
    renderer.upload_scene(context.queue(), &prepared.scene)?;
    Ok(renderer)
}

/// One prepared scene, ready to be drawn with whichever HUD a scenario asks
/// for.
///
/// The renderer and the scene are held rather than handed in per call, so two
/// frames a scenario compares cannot differ in anything but their HUD — which is
/// what every comparison here is stated about.
///
/// The renderer is public because it is the object the windowed client owns, and
/// how many times it has composed a HUD is an observation one scenario is
/// entirely about.
#[derive(Debug)]
pub struct HudCapture<'a> {
    pub context: &'a CaptureContext,
    pub renderer: FrameRenderer,
    pub snapshot: TerrainSnapshot,
    /// What the preparation produced beside the picture.
    pub content: PreparedContent,
}

/// What one preparation produced beside the scene: what the world is made of,
/// which layer each texture key occupies, and the registry both were resolved
/// against.
///
/// Retained rather than dropped because a scenario about the block a session
/// holds needs all three — the registry decides which block that is, the world
/// is what a simulation of it is built over, and the layers are what its texture
/// is looked for in. Taking them from a second preparation instead would be a
/// second world, and the frame would be of the first.
#[derive(Debug)]
pub struct PreparedContent {
    pub layers: TextureLayers,
    pub registry: Arc<BlockRegistry>,
    pub world: ReplayWorld,
}

impl<'a> HudCapture<'a> {
    /// The replay's scene at `tick`, prepared through the client's own startup
    /// and uploaded to a renderer of the kind the client owns.
    ///
    /// # Errors
    ///
    /// Returns the preparation, pipeline, upload or spawn failure.
    pub fn ready(context: &'a CaptureContext, tick: u32) -> Result<Self, Box<dyn Error>> {
        let prepared = super::prepare_scene()?;
        let renderer = prepared_renderer(context, &prepared)?;
        let camera = super::frames::replay_camera(tick, &prepared.world, &prepared.registry)?;
        let PreparedScene {
            scene,
            layers,
            world,
            registry,
            ..
        } = prepared;
        let scene = Arc::new(scene);
        Ok(Self {
            context,
            renderer,
            snapshot: super::frames::snapshot(tick, camera, &scene),
            content: PreparedContent {
                layers,
                registry,
                world,
            },
        })
    }

    /// One frame of the scene with `hud` over it, recorded through the client's
    /// frame call and read back.
    ///
    /// # Errors
    ///
    /// Returns the recording failure the renderer reported, the capture
    /// failure, or the absence of any drawn frame at all.
    pub fn capture(
        &mut self,
        hud: &HudFrame,
        request: &CaptureRequest,
    ) -> Result<Rgba8Image, Box<dyn Error>> {
        self.recorded(hud, |context, work| {
            Ok(context.capture(request, work)?.image)
        })
    }

    /// One frame of the scene with `hud` over it, recorded through the client's
    /// frame call and judged against the golden `settings` names.
    ///
    /// The **same** recording as [`capture`](Self::capture), so the frame a
    /// golden is minted from and the frame a scenario reads pixels out of are
    /// one frame drawn one way.
    ///
    /// # Errors
    ///
    /// Returns the recording failure the renderer reported, the capture
    /// failure, or the absence of any drawn frame at all. A frame that did not
    /// match its golden is a [`GoldenOutcome`], not an error.
    pub fn verify(
        &mut self,
        hud: &HudFrame,
        request: &CaptureRequest,
        settings: &GoldenSettings,
    ) -> Result<GoldenOutcome, Box<dyn Error>> {
        self.recorded(hud, |context, work| {
            Ok(capture_and_verify(context, request, work, settings)?)
        })
    }

    /// Records one frame with `hud` over the prepared scene and hands the draw
    /// work to `run`.
    fn recorded<T>(
        &mut self,
        hud: &HudFrame,
        run: impl FnOnce(&CaptureContext, &mut dyn DrawWork) -> Result<T, Box<dyn Error>>,
    ) -> Result<T, Box<dyn Error>> {
        let phase = ScenePhase::Ready(Arc::clone(&self.snapshot.scene));
        record_one(
            &mut self.renderer,
            &Recording {
                context: self.context,
                phase: &phase,
                snapshot: &self.snapshot,
                hud,
            },
            run,
        )
    }
}

/// The frame a client draws before its world has landed: no scene uploaded, no
/// array texture filled, and the phase that says so.
///
/// **The client's own waiting state rather than an imitation of it.** `App`
/// holds a scene declaring nothing and a phase of `Preparing` from the moment
/// the window opens until the preparation worker is collected, and draws every
/// frame in between through the same one call. What content declares is composed
/// over those frames too, which is the whole question this shape exists to ask.
#[derive(Debug)]
pub struct UnpreparedCapture<'a> {
    context: &'a CaptureContext,
    renderer: FrameRenderer,
    snapshot: TerrainSnapshot,
}

impl<'a> UnpreparedCapture<'a> {
    /// A client that has opened its window and is still waiting for a world.
    ///
    /// # Errors
    ///
    /// Returns the pipeline failure.
    pub fn waiting(context: &'a CaptureContext) -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            context,
            renderer: FrameRenderer::new(
                context.device(),
                context.queue(),
                &TerrainPassConfig::offscreen(),
            )?,
            snapshot: TerrainSnapshot {
                tick: 0,
                camera: waiting_view(),
                scene: empty_scene(),
            },
        })
    }

    /// One waiting frame with `hud` over it, through the client's frame call.
    ///
    /// # Errors
    ///
    /// Returns the recording failure the renderer reported, the capture
    /// failure, or the absence of any drawn frame at all.
    pub fn capture(
        &mut self,
        hud: &HudFrame,
        request: &CaptureRequest,
    ) -> Result<Rgba8Image, Box<dyn Error>> {
        record_one(
            &mut self.renderer,
            &Recording {
                context: self.context,
                phase: &ScenePhase::Preparing,
                snapshot: &self.snapshot,
                hud,
            },
            |context, work| Ok(context.capture(request, work)?.image),
        )
    }
}

/// Everything one recorded frame reads.
#[derive(Debug)]
struct Recording<'a> {
    context: &'a CaptureContext,
    phase: &'a ScenePhase,
    snapshot: &'a TerrainSnapshot,
    hud: &'a HudFrame,
}

/// Records one frame through the client's own frame call and hands the draw work
/// to `run`.
///
/// The `recorded` flag is the load-bearing part: a capture that returned without
/// ever invoking the draw work would hand back a target nothing wrote into, and
/// every comparison would then be between two blank textures.
///
/// # Errors
///
/// Returns the recording failure the renderer reported, the capture failure, or
/// the absence of any drawn frame at all.
fn record_one<T>(
    renderer: &mut FrameRenderer,
    recording: &Recording<'_>,
    run: impl FnOnce(&CaptureContext, &mut dyn DrawWork) -> Result<T, Box<dyn Error>>,
) -> Result<T, Box<dyn Error>> {
    let context = recording.context;
    let frame = FrameSnapshot {
        terrain: recording.snapshot,
        hud: recording.hud,
        // No capture this suite shoots is taken with the overlay shown, and none
        // ever should be: the overlay is engine tooling whose text is rasterised
        // by a toolkit, and a golden containing rasterised text makes that
        // toolkit the ground truth every driver then has to agree with.
        overlay: None,
    };
    let mut recorded = false;
    let mut work = draw_fn(|encoder, color| {
        let target = RecordTarget {
            device: context.device(),
            queue: context.queue(),
            encoder,
            color,
            size: CAPTURE_SIZE,
        };
        renderer.record_frame(target, recording.phase, &frame)?;
        recorded = true;
        Ok(())
    });
    let produced = run(context, &mut work)?;
    drop(work);
    if !recorded {
        return Err(DRAW_WORK_NEVER_RAN.into());
    }
    Ok(produced)
}

/// How two frames stand at the pixels a region accepts.
///
/// `considered` is reported beside the verdict on purpose: a region that accepts
/// nothing makes every count zero, and a test asserting `different == 0` over an
/// empty region asserts nothing at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Comparison {
    pub considered: u64,
    pub same: u64,
    pub different: u64,
    pub first_different: Option<(u32, u32)>,
}

/// Compares `left` and `right` at every pixel `chosen` accepts.
///
/// A pixel one frame has and the other does not counts as a disagreement, which
/// is what two frames of different sizes are.
pub fn compare_frames(
    left: &Rgba8Image,
    right: &Rgba8Image,
    chosen: impl Fn(u32, u32) -> bool,
) -> Comparison {
    let mut seen = Comparison {
        considered: 0,
        same: 0,
        different: 0,
        first_different: None,
    };
    for (x, y) in pixels(left).filter(|(x, y)| chosen(*x, *y)) {
        seen.considered += 1;
        if left.pixel(x, y) == right.pixel(x, y) {
            seen.same += 1;
        } else {
            seen.different += 1;
            seen.first_different = seen.first_different.or(Some((x, y)));
        }
    }
    seen
}

/// Every pixel coordinate of `frame`, row by row.
fn pixels(frame: &Rgba8Image) -> impl Iterator<Item = (u32, u32)> {
    let (width, height) = (frame.width(), frame.height());
    (0..height).flat_map(move |y| (0..width).map(move |x| (x, y)))
}
