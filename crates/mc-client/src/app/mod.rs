//! The frame path: acquire a texture from the surface, record one frame into it,
//! present it, and advance the world by however long the frame took.
//!
//! **One frame is one call.** The world, the HUD content declared over it, and
//! the debug overlay over that, are ordered by the renderer and not here — so a
//! frame test that composes through that same call is exercising the path the
//! window takes, rather than a second one built to resemble it.
//!
//! **A frame advances the world by the time it took, never by one tick.** This
//! file used to do the opposite, and it is what made a walk cover 4.5 blocks a
//! second on a 60 Hz display and 10.8 on a 144 Hz one: a tick is a declared
//! sixtieth of a second, and delivering one per presented frame runs the world at
//! the display's rate divided by sixty. What is handed over below is an elapsed
//! reading; how many ticks it buys is the core's answer, and there is no longer
//! anything here that could name a per-tick advance.
//!
//! **The one wall clock this client reads is handed to the core once a frame, and
//! is read nowhere else.** That is a narrower claim than the one this file used to
//! make, and the narrowing is deliberate rather than a concession: pacing a frame
//! needs elapsed time, so the property worth having is not that no clock exists
//! but that exactly one reading a frame is taken and everything derived from it
//! agrees. The rate the overlay reports and the time the simulation spent are that
//! same reading. A confinement scan holds the clock to one file mechanically, with
//! the adapter's own as its only exemption.
//!
//! Every decision below is somebody else's. Whether a size is drawable, whether a
//! failed acquire is recovered from or fatal, which surface format is configured
//! — all of them are pure functions in `mc_render::surface`, each with a test that
//! never opened a window. What is left here is the wiring that carries their
//! answers to the graphics API, which is the whole reason this crate holds no
//! coverage of its own.

use std::sync::Arc;

use mc_core::hud::HudLayout;
use mc_core::id::TextureKey;
use mc_render::camera::{camera_view, waiting_view};
use mc_render::geometry::scene::SceneGeometry;
use mc_render::gpu::{FrameRenderer, FrameSnapshot, RecordTarget};
use mc_render::hud::{HudFrame, held_swatch};
use mc_render::snapshot::{ScenePhase, TerrainSnapshot};
use mc_render::surface::{
    FrameAction, ResizeAction, SurfaceErrorKind, SurfaceSize, resize_action, surface_error_action,
};
use mc_render::time::clock::SystemFrameClock;
use mc_render::window::{Ending, rendered};
use mc_sim::reload::watching_shipped_content;

mod reload;
mod report;

use crate::gpu_startup::Gpu;
use crate::launch::{PreparationHandle, Starting, collect};
use crate::notice::{self, Notices};
use crate::remesh::{Remesher, Retained};
use crate::session::Session;

use crate::session::reload::Remeshing;
use crate::startup::{PreparationError, empty_hud, empty_scene};
use crate::surface_setup::{SetupError, chosen_format, configuration_for, renderer_for};
use report::SaidOnce;

/// What a player is told when the re-mesh worker has gone.
///
/// **Said rather than swallowed**: no edit will be drawn for the rest of the run, and
/// a world that silently stops showing what a player breaks is the worst outcome the
/// re-mesh path has.
const WORKER_GONE: &str = "the worker that draws your edits has stopped; \
                           edits will not be shown for the rest of this run";

/// The label the frame's command encoder carries in a driver capture.
const ENCODER_LABEL: &str = "mycraft frame";

/// The window's contents, and everything needed to keep drawing them.
#[derive(Debug)]
pub struct App {
    gpu: Gpu,
    surface: wgpu::Surface<'static>,
    configuration: wgpu::SurfaceConfiguration,
    renderer: FrameRenderer,
    /// The worker preparing the launch, until it is collected. `None` afterwards,
    /// which is also what says the collection already happened.
    preparation: Option<PreparationHandle>,
    phase: ScenePhase,
    /// A scene holding nothing, handed to the renderer while `phase` is
    /// `Preparing` so a frame's snapshot has the same shape in both phases.
    nothing: Arc<SceneGeometry>,
    /// The elements content declared, composed over every frame.
    ///
    /// It declares nothing until the preparation lands, for the same reason
    /// `nothing` above exists: the declarations are read from the content root
    /// on the worker, so before it lands there is no content to draw a HUD
    /// from. It is not an `Option`, because "there is no layout yet" and "the
    /// layout declares no element" are the same picture and a second spelling
    /// of it would be a second thing to keep in step.
    hud: Arc<HudLayout>,
    /// The one wall clock this client reads, measured from the moment the window
    /// opened.
    ///
    /// **Owned here rather than by the session**, which is what keeps a session
    /// drivable by a clock a test owns: every scenario about what a key does or
    /// where a walk ends hands over its own elapsed readings, so none of them
    /// waits on a scheduler or depends on how fast the machine ran them. Reading
    /// it is the frame path's business, and the frame path is here.
    ///
    /// It measures from the moment this was built, so the first frame's interval
    /// is the time the client took to get from a configured surface to a presented
    /// frame — which is genuinely what that frame cost, and is bounded by the
    /// core's catch-up limit like any other stall.
    frame_clock: SystemFrameClock,
    size: SurfaceSize,
    /// The worker that turns an edit into a scene, once there is a world to
    /// edit. `None` until the preparation lands, exactly as the simulation is.
    remesher: Option<Remesher>,
    /// Where every non-fatal notice this client writes goes.
    ///
    /// Supplied by whoever started the run and held for the whole of it, so a
    /// harness can read what a player would have read — and silence it by
    /// supplying a sink that discards.
    notices: Notices,
    /// The dropped-frame line, so a fault that recurs every frame is stated once
    /// instead of filling the terminal.
    reported: SaidOnce,
    /// The re-mesh line, for the same reason and separately: a dropped frame and
    /// an edit that could not be shown are different faults, and one recurring
    /// must not silence the other.
    reported_remesh: SaidOnce,
    /// The held-block line, for the same reason again. It recurs every frame for
    /// as long as that block is held, which is the whole run.
    reported_swatch: SaidOnce,
    /// The content refusal, so a recurring one is said once.
    reported_reload: SaidOnce,
}

impl App {
    /// Configures `surface` for the window it came from and builds the frame
    /// path that draws into it.
    ///
    /// # Errors
    ///
    /// Returns [`SetupError`] when the surface offers no sRGB format, offers one
    /// this renderer has no pass for, or the pass cannot be built.
    pub fn new(
        gpu: Gpu,
        surface: wgpu::Surface<'static>,
        size: SurfaceSize,
        starting: Starting,
    ) -> Result<Self, SetupError> {
        let capabilities = surface.get_capabilities(&gpu.adapter);
        let format = chosen_format(&capabilities.formats)?;
        let configuration = configuration_for(&surface, &gpu, size, format)?;
        surface.configure(&gpu.device, &configuration);
        let renderer = renderer_for(&gpu, format, &starting.texels)?;
        let notices = starting.notices;

        Ok(Self {
            gpu,
            surface,
            configuration,
            renderer,
            preparation: Some(starting.preparation),
            phase: ScenePhase::Preparing,
            nothing: empty_scene(),
            hud: empty_hud()?,
            frame_clock: SystemFrameClock::started_now(),
            size,
            remesher: None,
            notices,
            reported: SaidOnce::default(),
            reported_remesh: SaidOnce::default(),
            reported_swatch: SaidOnce::default(),
            reported_reload: SaidOnce::default(),
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
            return Some(Ending::failed(&failure));
        }
        self.exchange_remesh(session);
        self.present(session)
    }

    /// Shows whatever the re-mesh worker has finished, and gives it whatever the
    /// last tick left to do.
    ///
    /// **In that order, and only one batch at a time.** A worker that is still
    /// busy is not asked, so the sections edited meanwhile stay in the world's
    /// own dirty set and arrive together — a set keyed per section, so a player
    /// digging through a slow batch accumulates sections rather than batches.
    ///
    /// This is the frame path's entire share of an edit: one upload of a scene
    /// somebody else assembled.
    fn exchange_remesh(&mut self, session: &mut Session) {
        // Computed before the match, because `show` needs `self` while the collect
        // needs the worker out of it.
        let collected = self
            .remesher
            .as_mut()
            .map(|remesher| session.collect_remesh(remesher));
        // What to say is `said_about`'s decision and never this path's: the words a
        // player reads when meshing stops are only assertable because nothing
        // decides them inside a redraw. A worker that is gone is said through the
        // same dedup a re-mesh fault uses, since it recurs on every frame left in
        // the run and is the one absence waiting will not repair.
        match collected {
            Some(Remeshing::Show(scene)) => self.show(scene),
            Some(other) => self.say_about(&other),
            None => {}
        }
        self.submit_remesh(session);
    }

    /// Hands the worker whatever the last tick left to re-mesh, if it is free to
    /// take it.
    ///
    /// A busy worker is not asked at all, which is what leaves the edits made
    /// meanwhile accumulating in the world rather than queued here.
    fn submit_remesh(&mut self, session: &mut Session) {
        let Some(remesher) = self.remesher.as_mut().filter(|remesher| remesher.is_free()) else {
            return;
        };
        if let Some(work) = session.take_remesh_work() {
            remesher.submit(work);
        }
    }

    /// Gives a re-meshed scene to the device and draws from it thereafter.
    ///
    /// An upload that fails is reported and dropped, not fatal: the picture the
    /// player already has is a stale world rather than no world, which is the
    /// same trade the failed batch above makes.
    fn show(&mut self, scene: Arc<SceneGeometry>) {
        if let Err(failure) = self.renderer.upload_scene(&self.gpu.queue, &scene) {
            self.report_remesh(&rendered(&failure));
            return;
        }
        self.phase = ScenePhase::Ready(scene);
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
        // Here rather than in `redraw`, because this is the first point at which a
        // frame is certainly drawn: every skip, reconfigure and fatal path above
        // has already returned, and elapsed time attributed to a frame nobody drew
        // would advance the world for a frame that never happened.
        //
        // After the present, so the interval measured spans a whole frame's work.
        // The readout drawn above is therefore the previous frame's reading, which
        // is what a mean over sixty frames is anyway.
        session.advance_frame(&self.frame_clock);
        // After the ticks, because a tick is what crosses the reload boundary.
        if let Err(refused) = self.take_up_reloaded_content(session) {
            return Some(Ending::failed_under(
                "the reloaded content could not be drawn",
                &refused,
            ));
        }
        None
    }

    /// Records and submits one frame into `view`.
    ///
    /// **One call, and the whole frame is behind it.** The world, the HUD
    /// content declared and — later — the debug overlay are ordered by the
    /// renderer rather than here, so a frame test composing through that same
    /// call is exercising what the product does. A client that recorded the
    /// passes itself would give the HUD a second entry point, and every scenario
    /// about the composition would stay green while the window drew a world with
    /// nothing over it.
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
        let terrain = self.snapshot(session);
        let hud = HudFrame {
            layout: Arc::clone(&self.hud),
            held: self.swatch(session),
        };
        let target = RecordTarget {
            device: &self.gpu.device,
            queue: &self.gpu.queue,
            encoder: &mut encoder,
            color: view,
            size: self.size,
        };
        // Asked for, not decided here. Whether the overlay is being shown, and
        // what it says if it is, are the session's answers — this hands one on
        // and holds no opinion about it, which is what leaves nothing here for a
        // scenario to have to reach.
        let readout = session.overlay_readout();
        let frame = FrameSnapshot {
            terrain: &terrain,
            hud: &hud,
            overlay: readout.as_ref(),
        };
        if let Err(failure) = self.renderer.record_frame(target, &self.phase, &frame) {
            self.report(failure);
        }
        self.gpu.queue.submit([encoder.finish()]);
    }

    /// Which texture this frame's held-block indicator draws from, stating once
    /// whatever leaves it drawing nothing.
    ///
    /// The block is the session's answer and the resolution is the renderer's,
    /// and neither is second-guessed here: one lookup and one report,
    /// both of which are somebody else's decision spelled out where a test can
    /// reach it. The lookup answers with an owned key, so the renderer's borrow is
    /// over before the report needs this type mutably.
    fn swatch(&mut self, session: &Session) -> Option<TextureKey> {
        let swatch = held_swatch(
            session.held_block().as_ref(),
            self.renderer.texture_resolution(),
        );
        if let Some(report) = swatch.unresolved_report() {
            self.report_swatch(&report);
        }
        swatch.texture()
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
            tint: published.as_ref().and_then(|published| published.tint),
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

    /// Takes the prepared launch when the worker has finished with it, uploads
    /// its scene, attaches the simulation it came with, and moves the frame path
    /// off the clear colour.
    ///
    /// **It decides nothing.** Which world is played and which block is held are
    /// both answered on the worker, where the registry the world was resolved
    /// against is in hand — so what is left here is an upload and an attach. That
    /// ordering is why a save that cannot be read now refuses before anything
    /// reaches the device rather than after.
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
        // The serial the launch published under, so the worker's first batch is
        // judged against the content it was actually meshed with.
        let serving = prepared.simulation.content().serial;

        self.renderer
            .upload_textures(&self.gpu.queue, &prepared.resolution)?;
        let scene = Arc::new(prepared.scene);
        self.renderer.upload_scene(&self.gpu.queue, &scene)?;
        notice::say_entering(prepared.clearing, &self.notices);
        session.attach_simulation(prepared.simulation, prepared.holding);
        // What makes the whole reload path reachable by the person it is for: the
        // root the launch was prepared from goes under watch, and the session
        // crosses its boundaries from the next tick on. Attached after the
        // simulation because a boundary needs one.
        session.attach_reload(watching_shipped_content(prepared.root));
        // The meshed sections and the resolution are handed to the worker rather
        // than kept here: they are what a re-mesh works on, and a copy on each
        // side would be a second answer waiting to disagree. No registry — a
        // batch carries the one its own world was resolved against.
        self.remesher = Some(Remesher::spawn(
            Retained {
                meshed: prepared.meshed,
                resolution: prepared.resolution,
            },
            serving,
        ));
        // The HUD arrives with the scene because it was read from the same
        // content root, on the same worker. Until this moment the frame path
        // composed a layout declaring nothing, which is what a client that has
        // not read its content yet has to draw.
        self.hud = prepared.hud;
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
