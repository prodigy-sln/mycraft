//! The one file in this crate that knows what a window is.
//!
//! Everything the loop does with an event is decided by
//! `mc_render::window::window_event_action`, a pure function tested without a
//! display server. What is here is the translation into its vocabulary and the
//! handful of calls that need the real thing — creating the window, asking it for
//! its size, telling it to redraw.
//!
//! `tests/winit_boundary.rs` fails the build if a second file in this crate names
//! the library. That is not decoration: ADR-013 leaves this whole crate out of the
//! coverage denominator on the stated grounds that it holds the adapter and the
//! wiring and no policy, and a decision that drifted in here would leave the
//! denominator with it, moving no number as it went.

use std::sync::Arc;

use mc_render::surface::SurfaceSize;
use mc_render::window::{Ending, LoopAction, WindowEventKind, window_event_action};
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowId};

use crate::app::App;
use crate::gpu_startup::{Gpu, create_surface};
use crate::startup::{PreparationError, PreparedScene};

/// What the window is called, and how large it opens.
///
/// The declared capture size, so what the window shows is framed as the goldens
/// framed it — the grass rim near the horizon sits where the screen-space budget
/// put it only at this aspect.
const TITLE: &str = "MyCraft";
const INITIAL_WIDTH: u32 = 1280;
const INITIAL_HEIGHT: u32 = 720;

/// The worker preparing the replay, handed on to the app once one exists.
type PreparationHandle = std::thread::JoinHandle<Result<PreparedScene, PreparationError>>;

/// Runs the client until the window closes or something ends it.
///
/// Returns the ending rather than exiting, so the one place that turns an ending
/// into a status stays the one place.
pub fn run(gpu: Gpu, preparation: PreparationHandle) -> Ending {
    let event_loop = match EventLoop::new() {
        Ok(event_loop) => event_loop,
        Err(failure) => {
            return failed(&format!(
                "no event loop could be created, so there is no window to draw in: {failure}"
            ));
        }
    };
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut client = Client {
        gpu: Some(gpu),
        preparation: Some(preparation),
        window: None,
        app: None,
        ending: None,
    };
    if let Err(failure) = event_loop.run_app(&mut client) {
        return failed(&format!("the event loop stopped: {failure}"));
    }
    client.ending.unwrap_or(Ending::Closed)
}

/// The client as the windowing library sees it.
struct Client {
    /// Taken when the window arrives and the app is built from it.
    gpu: Option<Gpu>,
    preparation: Option<PreparationHandle>,
    window: Option<Arc<Window>>,
    app: Option<App>,
    ending: Option<Ending>,
}

impl ApplicationHandler for Client {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.app.is_some() {
            return;
        }
        match self.start(event_loop) {
            Ok(app) => self.app = Some(app),
            Err(ending) => self.stop(event_loop, ending),
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        match window_event_action(&kind_of(&event)) {
            LoopAction::Exit => self.stop(event_loop, Ending::Closed),
            LoopAction::Resize(size) => self.on_resize(size),
            LoopAction::Redraw => self.on_redraw(event_loop),
            LoopAction::Ignore => {}
        }
    }

    fn about_to_wait(&mut self, _: &ActiveEventLoop) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}

impl Client {
    /// Opens the window, makes a surface for it, and builds everything that draws
    /// into that surface.
    fn start(&mut self, event_loop: &ActiveEventLoop) -> Result<App, Ending> {
        let (gpu, preparation) = self.taken()?;
        let window = Arc::new(
            event_loop
                .create_window(window_attributes())
                .map_err(|failure| failed(&format!("no window could be opened: {failure}")))?,
        );
        let size = size_of(&window);
        let surface = create_surface(&gpu.instance, Arc::clone(&window)).map_err(|failure| {
            failed(&format!(
                "no surface could be made for the window: {failure}"
            ))
        })?;

        self.window = Some(window);
        App::new(gpu, surface, size, preparation)
            .map_err(|failure| failed(&format!("the client could not be built: {failure}")))
    }

    /// The device and the worker, each of which is handed on exactly once.
    fn taken(&mut self) -> Result<(Gpu, PreparationHandle), Ending> {
        let gpu = self
            .gpu
            .take()
            .ok_or_else(|| failed("the device was already handed to a window"))?;
        let preparation = self
            .preparation
            .take()
            .ok_or_else(|| failed("the replay was already handed to a window"))?;
        Ok((gpu, preparation))
    }

    fn on_resize(&mut self, size: SurfaceSize) {
        if let Some(app) = &mut self.app {
            app.resize(size);
        }
    }

    fn on_redraw(&mut self, event_loop: &ActiveEventLoop) {
        let ended = self.app.as_mut().and_then(App::redraw);
        if let Some(ending) = ended {
            self.stop(event_loop, ending);
        }
    }

    /// Records how the run ended and leaves the loop.
    ///
    /// The first ending wins: whatever stopped the run is what is reported, not
    /// the close that follows from stopping it.
    fn stop(&mut self, event_loop: &ActiveEventLoop, ending: Ending) {
        self.ending.get_or_insert(ending);
        event_loop.exit();
    }
}

/// An ending carrying `report`, for the failures that are the client's own rather
/// than a startup verdict or a lost device.
fn failed(report: &str) -> Ending {
    Ending::Failed {
        report: report.to_owned(),
    }
}

/// How the window opens.
fn window_attributes() -> winit::window::WindowAttributes {
    Window::default_attributes()
        .with_title(TITLE)
        .with_inner_size(winit::dpi::LogicalSize::new(INITIAL_WIDTH, INITIAL_HEIGHT))
}

/// The window's size in physical pixels, which is the only size a surface is
/// configured at.
fn size_of(window: &Window) -> SurfaceSize {
    let inner = window.inner_size();
    SurfaceSize {
        width: inner.width,
        height: inner.height,
    }
}

/// One window event, in the renderer's vocabulary.
///
/// The catch-all is deliberate and is the reason this translation exists at all:
/// the library grows variants between versions, and every one of them arrives
/// here as something the loop already knows how to ignore.
fn kind_of(event: &WindowEvent) -> WindowEventKind {
    match event {
        WindowEvent::CloseRequested => WindowEventKind::CloseRequested,
        WindowEvent::Resized(size) => WindowEventKind::Resized(SurfaceSize {
            width: size.width,
            height: size.height,
        }),
        WindowEvent::RedrawRequested => WindowEventKind::RedrawRequested,
        _ => WindowEventKind::Other,
    }
}
