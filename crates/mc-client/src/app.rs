//! The frame path: acquire a texture from the surface, record the terrain pass
//! into it, present it, advance one tick.
//!
//! **One tick per rendered frame, never elapsed time.** No wall clock is read
//! anywhere in this client, which is what makes the replay the same run on a
//! machine that draws it at 300 frames a second and on one that manages 30.
//!
//! Every decision below is somebody else's. Whether a size is drawable, whether a
//! failed acquire is recovered from or fatal, which surface format is configured
//! — all of them are pure functions in `mc_render::surface`, each with a test that
//! never opened a window. What is left here is the wiring that carries their
//! answers to the graphics API, which is the whole reason this crate holds no
//! coverage of its own.

use std::sync::Arc;

use mc_render::camera::{camera_view, waiting_view};
use mc_render::geometry::scene::SceneGeometry;
use mc_render::gpu::{FrameError, RecordTarget, RendererError, TerrainRenderer};
use mc_render::pass::{ColorFormat, TerrainPassConfig};
use mc_render::snapshot::{ScenePhase, TerrainSnapshot};
use mc_render::surface::{
    FormatError, FrameAction, ResizeAction, SurfaceErrorKind, SurfaceFormatFacts, SurfaceSize,
    resize_action, select_surface_format, surface_error_action,
};
use mc_render::window::Ending;
use mc_sim::replay::simulation_for;
use thiserror::Error;

use crate::gpu_startup::Gpu;
use crate::session::Session;
use crate::startup::{PreparationError, PreparedScene, collect, empty_scene};

/// The label the frame's command encoder carries in a driver capture.
const ENCODER_LABEL: &str = "mycraft frame";

/// Why the client could not be built around the window it was given.
#[derive(Debug, Error)]
pub enum SetupError {
    #[error("the surface offers no format this client can present through")]
    Format(#[from] FormatError),
    #[error(
        "the surface's first sRGB format is `{name}`, which this renderer has no pass \
         configuration for"
    )]
    UnsupportedFormat { name: String },
    #[error("the surface reported no default configuration for this adapter")]
    NoDefaultConfiguration,
    #[error(
        "the surface's format list no longer holds the format at index {index} that was chosen \
         from it"
    )]
    FormatVanished { index: usize },
    #[error("the terrain pass could not be built")]
    Renderer(#[from] RendererError),
}

/// The window's contents, and everything needed to keep drawing them.
#[derive(Debug)]
pub struct App {
    gpu: Gpu,
    surface: wgpu::Surface<'static>,
    configuration: wgpu::SurfaceConfiguration,
    renderer: TerrainRenderer,
    /// The worker preparing the replay, until it is collected. `None` afterwards,
    /// which is also what says the collection already happened.
    preparation: Option<PreparationHandle>,
    phase: ScenePhase,
    /// A scene holding nothing, handed to the renderer while `phase` is
    /// `Preparing` so a frame's snapshot has the same shape in both phases.
    nothing: Arc<SceneGeometry>,
    size: SurfaceSize,
    /// The last frame error reported, so a fault that recurs every frame is
    /// stated once instead of filling the terminal.
    reported: Option<FrameError>,
}

/// The worker handle, named so the type above reads.
type PreparationHandle = std::thread::JoinHandle<Result<PreparedScene, PreparationError>>;

impl App {
    /// Configures `surface` for the window it came from and builds the terrain
    /// pass that draws into it.
    ///
    /// # Errors
    ///
    /// Returns [`SetupError`] when the surface offers no sRGB format, offers one
    /// this renderer has no pass for, or the pass cannot be built.
    pub fn new(
        gpu: Gpu,
        surface: wgpu::Surface<'static>,
        size: SurfaceSize,
        preparation: PreparationHandle,
    ) -> Result<Self, SetupError> {
        let capabilities = surface.get_capabilities(&gpu.adapter);
        let format = chosen_format(&capabilities.formats)?;
        let configuration = configuration_for(&surface, &gpu, size, format)?;
        surface.configure(&gpu.device, &configuration);

        let renderer = TerrainRenderer::new(
            &gpu.device,
            &gpu.queue,
            &TerrainPassConfig::windowed(color_format(format)?),
        )?;

        Ok(Self {
            gpu,
            surface,
            configuration,
            renderer,
            preparation: Some(preparation),
            phase: ScenePhase::Preparing,
            nothing: empty_scene(),
            size,
            reported: None,
        })
    }

    /// Takes a new surface size, reconfiguring only when a frame of it could be
    /// drawn.
    ///
    /// The size is recorded either way, because a redraw arriving at a
    /// zero-dimension size has to skip too and this is what it asks.
    pub fn resize(&mut self, size: SurfaceSize) {
        self.size = size;
        if let ResizeAction::Reconfigure(target) = resize_action(size) {
            self.reconfigure(target);
        }
    }

    /// Draws one frame, and says whether the run can continue.
    ///
    /// `None` continues — including for every frame that was deliberately not
    /// drawn. `Some` is an ending the event loop leaves on.
    pub fn redraw(&mut self, session: &mut Session) -> Option<Ending> {
        if matches!(resize_action(self.size), ResizeAction::Skip) {
            return None;
        }
        if let Err(failure) = self.collect_preparation(session) {
            return Some(Ending::Failed {
                report: failure.to_string(),
            });
        }
        self.present(session)
    }

    /// Acquires a texture, draws into it and presents it — or does whatever the
    /// pure policy says a failed acquire calls for.
    fn present(&mut self, session: &mut Session) -> Option<Ending> {
        let acquired = match self.acquire() {
            Acquired::Texture(texture) => texture,
            Acquired::Act(FrameAction::Reconfigure) => {
                self.reconfigure(self.size);
                return None;
            }
            Acquired::Act(FrameAction::Fatal(reason)) => return Some(Ending::Frame(reason)),
            Acquired::Act(FrameAction::Skip | FrameAction::Render) => return None,
        };

        let view = acquired
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        self.draw(&view, session);
        self.gpu.queue.present(acquired);
        session.tick();
        None
    }

    /// Records and submits one frame into `view`.
    ///
    /// A frame that cannot be recorded is reported and dropped rather than ending
    /// the run: a dropped frame is recoverable and a crash is not, and the depth
    /// allocation this can fail on is already ruled out by the size check above.
    fn draw(&mut self, view: &wgpu::TextureView, session: &Session) {
        let mut encoder = self
            .gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some(ENCODER_LABEL),
            });
        let snapshot = self.snapshot(session);
        let target = RecordTarget {
            device: &self.gpu.device,
            queue: &self.gpu.queue,
            encoder: &mut encoder,
            color: view,
            size: self.size,
        };
        if let Err(failure) = self.renderer.record_terrain(target, &self.phase, &snapshot) {
            self.report(failure);
        }
        self.gpu.queue.submit([encoder.finish()]);
    }

    /// This frame's input: the tick and pose the simulation published, and
    /// whatever geometry there is to draw.
    ///
    /// Before the world lands there is no simulation to read, and nothing is
    /// drawn either: the frame is the clear colour and the camera below is never
    /// looked through.
    fn snapshot(&self, session: &Session) -> TerrainSnapshot {
        let published = session.latest();
        TerrainSnapshot {
            tick: published.as_ref().map_or(0, |published| published.tick),
            camera: published.as_ref().map_or_else(waiting_view, |published| {
                camera_view(published.camera.eye, published.camera.target)
            }),
            scene: match &self.phase {
                ScenePhase::Preparing => Arc::clone(&self.nothing),
                ScenePhase::Ready(scene) => Arc::clone(scene),
            },
        }
    }

    /// Asks the surface for this frame's texture, translating everything that is
    /// not a texture into the renderer's own vocabulary.
    fn acquire(&self) -> Acquired {
        match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(texture)
            | wgpu::CurrentSurfaceTexture::Suboptimal(texture) => Acquired::Texture(texture),
            wgpu::CurrentSurfaceTexture::Timeout => Acquired::acting_on(SurfaceErrorKind::Timeout),
            wgpu::CurrentSurfaceTexture::Occluded | wgpu::CurrentSurfaceTexture::Validation => {
                Acquired::acting_on(SurfaceErrorKind::Other)
            }
            wgpu::CurrentSurfaceTexture::Outdated => {
                Acquired::acting_on(SurfaceErrorKind::Outdated)
            }
            wgpu::CurrentSurfaceTexture::Lost => Acquired::acting_on(self.loss()),
        }
    }

    /// Which kind of loss a `Lost` acquire is.
    ///
    /// `wgpu` reports a lost surface and a lost device through the same answer and
    /// separates them only through the device-lost callback, so this is the one
    /// place the two are told apart — and getting it wrong in the recoverable
    /// direction would spin forever on a window that will never draw again.
    fn loss(&self) -> SurfaceErrorKind {
        if self.gpu.is_device_lost() {
            SurfaceErrorKind::DeviceLost
        } else {
            SurfaceErrorKind::Lost
        }
    }

    /// Takes the prepared scene when the worker has finished with it, uploads it,
    /// starts the simulation of the world it came with, and moves the frame path
    /// off the clear colour.
    ///
    /// The simulation is built here and nowhere earlier because the player's
    /// spawn is derived from the world, and the world arrives several frames
    /// after the window opens. Ticking before then would drop the player through
    /// a world that does not exist yet for the whole of the load.
    ///
    /// **The invariant this holds is now a cross-object one.** "There is a
    /// simulation exactly when the phase is `Ready`" used to relate two fields
    /// of this struct and could be checked by reading it alone. The phase still
    /// lives here and the simulation now lives in the session, so the two are
    /// set together by this one function and by nothing else — nothing
    /// structural enforces it, and a reader of either type on its own cannot see
    /// it.
    fn collect_preparation(&mut self, session: &mut Session) -> Result<(), PreparationError> {
        if !self
            .preparation
            .as_ref()
            .is_some_and(|handle| handle.is_finished())
        {
            return Ok(());
        }
        let Some(handle) = self.preparation.take() else {
            return Ok(());
        };
        let prepared = collect(handle)?;

        self.renderer
            .upload_textures(&self.gpu.queue, &prepared.layers)?;
        let scene = Arc::new(prepared.scene);
        self.renderer.upload_scene(&self.gpu.queue, &scene)?;
        session.attach_simulation(simulation_for(&prepared.world, &prepared.registry)?);
        self.phase = ScenePhase::Ready(scene);
        Ok(())
    }

    /// Reconfigures the surface at `size`, which is the recovery for a lost or
    /// outdated one as well as the answer to a resize.
    fn reconfigure(&mut self, size: SurfaceSize) {
        self.configuration.width = size.width;
        self.configuration.height = size.height;
        self.surface
            .configure(&self.gpu.device, &self.configuration);
    }

    /// States a frame failure once, however many frames it goes on to affect.
    fn report(&mut self, failure: FrameError) {
        if self.reported != Some(failure) {
            eprintln!("mycraft: a frame was dropped: {failure}");
            self.reported = Some(failure);
        }
    }
}

/// What asking the surface for a texture came to.
enum Acquired {
    Texture(wgpu::SurfaceTexture),
    Act(FrameAction),
}

impl Acquired {
    /// Whatever the pure policy says about an acquire that failed with `kind`.
    fn acting_on(kind: SurfaceErrorKind) -> Self {
        Self::Act(surface_error_action(kind))
    }
}

/// Which of the formats a surface offers is configured.
fn chosen_format(offered: &[wgpu::TextureFormat]) -> Result<wgpu::TextureFormat, SetupError> {
    let facts: Vec<SurfaceFormatFacts> = offered
        .iter()
        .map(|format| SurfaceFormatFacts {
            name: format!("{format:?}"),
            is_srgb: format.is_srgb(),
        })
        .collect();
    let index = select_surface_format(&facts)?;
    offered
        .get(index)
        .copied()
        // Unreachable: the index came from the list this one was built from. It is
        // an error rather than a panic because this is a player's startup path.
        .ok_or(SetupError::FormatVanished { index })
}

/// The pass's colour target, as the renderer spells it.
fn color_format(format: wgpu::TextureFormat) -> Result<ColorFormat, SetupError> {
    match format {
        wgpu::TextureFormat::Rgba8UnormSrgb => Ok(ColorFormat::Rgba8UnormSrgb),
        wgpu::TextureFormat::Bgra8UnormSrgb => Ok(ColorFormat::Bgra8UnormSrgb),
        other => Err(SetupError::UnsupportedFormat {
            name: format!("{other:?}"),
        }),
    }
}

/// The surface's own default configuration, pointed at the format that was
/// chosen rather than the one it would have picked.
fn configuration_for(
    surface: &wgpu::Surface<'static>,
    gpu: &Gpu,
    size: SurfaceSize,
    format: wgpu::TextureFormat,
) -> Result<wgpu::SurfaceConfiguration, SetupError> {
    let mut configuration = surface
        .get_default_config(&gpu.adapter, size.width, size.height)
        .ok_or(SetupError::NoDefaultConfiguration)?;
    configuration.format = format;
    configuration.usage = wgpu::TextureUsages::RENDER_ATTACHMENT;
    Ok(configuration)
}
