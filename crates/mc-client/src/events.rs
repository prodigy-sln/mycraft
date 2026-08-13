//! The one file in this crate that knows what a window is.
//!
//! Everything the loop does with an event is decided by
//! `mc_render::window::window_event_action`, a pure function tested without a
//! display server. What is here is the translation into its vocabulary and the
//! handful of calls that need the real thing — creating the window, asking it for
//! its size, telling it to redraw, and asking the operating system to hold the
//! pointer.
//!
//! **The binding table is the one decision this file holds**, and it is data
//! rather than a procedure: a key code becomes a `PlayerAction` or nothing, and
//! what either of those means is decided elsewhere and tested there. It is here
//! because a key code cannot be spelled anywhere else, and it is admitted as a
//! narrow recorded exception to this crate holding no policy. The capture ladder
//! is the same shape — every rung is `mc_render::window`'s answer, and what is
//! here is the asking.
//!
//! `tests/winit_boundary.rs` fails the build if a second file in this crate names
//! the library. That is not decoration: ADR-013 leaves this whole crate out of the
//! coverage denominator on the stated grounds that it holds the adapter and the
//! wiring and no policy, and a decision that drifted in here would leave the
//! denominator with it, moving no number as it went.

use std::sync::Arc;

use mc_render::surface::SurfaceSize;
use mc_render::window::{
    CaptureState, Ending, LoopAction, WindowEventKind, accepts_pointer_motion, capture_after_click,
    capture_after_escape, first_capture_attempt, next_capture_attempt, window_event_action,
};
use mc_sim::player::PlayerAction;
use winit::application::ApplicationHandler;
use winit::event::{DeviceEvent, DeviceId, ElementState, KeyEvent, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{CursorGrabMode, Window, WindowId};

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
        capture: CaptureState::Uncaptured,
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
    /// How firmly the pointer is currently held. What the platform granted, not
    /// what was asked for — pointer motion is admitted against this, so a
    /// refused grab must not leave the client believing it has the cursor.
    capture: CaptureState,
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
        // Keys are read here rather than through `WindowEventKind`, because the
        // binding table is the one piece of this feature the renderer may not
        // hold: it is spelled in key codes (architecture D-10). Everything that
        // follows from a key is still a call into a tested pure function, and a
        // keyboard event reaches the table below as `Other`.
        if let WindowEvent::KeyboardInput { event: key, .. } = &event {
            self.on_key(key);
        }
        // A click is how the player takes the cursor back after Escape gave it
        // away, and it is read here for the same reason a key is: which button
        // was pressed cannot be spelled anywhere else. Which capture follows is
        // `mc_render::window`'s answer, not this file's.
        if let WindowEvent::MouseInput {
            state: ElementState::Pressed,
            ..
        } = &event
        {
            self.set_capture(capture_after_click(self.capture));
        }
        match window_event_action(&kind_of(&event)) {
            LoopAction::Exit => self.stop(event_loop, Ending::Closed),
            LoopAction::Resize(size) => self.on_resize(size),
            LoopAction::Redraw => self.on_redraw(event_loop),
            LoopAction::ClearInput => self.on_input_cleared(),
            LoopAction::Ignore => {}
        }
    }

    /// Raw pointer motion, which is relative and arrives whatever the cursor is
    /// doing — including while it is the desktop's.
    ///
    /// The window's own cursor-moved event carries a *position*, which stops at
    /// the edge of the screen and so cannot express a turn that keeps going.
    ///
    /// The motion is admitted against the capture the platform actually granted,
    /// not the one that was asked for: a refused grab leaves the cursor the
    /// desktop's, and turning the camera with it would be the game reading input
    /// it was not given.
    fn device_event(&mut self, _: &ActiveEventLoop, _: DeviceId, event: DeviceEvent) {
        let DeviceEvent::MouseMotion { delta: (x, y) } = event else {
            return;
        };
        if !accepts_pointer_motion(self.capture) {
            return;
        }
        if let Some(app) = self.app.as_mut() {
            // Device counts, which are small whole numbers per event — the
            // narrowing loses nothing a pointer can report, and the accumulator
            // is `f32` because the angle it becomes is.
            app.input().look(x as f32, y as f32);
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

        self.capture = hold_pointer(&window, first_capture_attempt());
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

    /// One key going down or coming up.
    ///
    /// Escape is not in the binding table and is not the player asking to move:
    /// it is how they get their desktop back, so it is spent on the capture
    /// policy and never reaches the accumulator. Every other key is whatever the
    /// table made of it, `None` included.
    fn on_key(&mut self, key: &KeyEvent) {
        let PhysicalKey::Code(code) = key.physical_key else {
            return;
        };
        let pressed = key.state.is_pressed();
        if code == KeyCode::Escape {
            self.on_escape(pressed);
            return;
        }
        if let Some(app) = self.app.as_mut() {
            app.input().apply(bound_action(code), pressed);
        }
    }

    /// Escape, going down or coming up.
    ///
    /// Only the press does anything. Releasing Escape is the player letting go
    /// of a key they have already spent, and asking for the capture back a
    /// moment after giving it up is exactly what the release policy exists to
    /// not do.
    fn on_escape(&mut self, pressed: bool) {
        if pressed {
            self.set_capture(capture_after_escape(self.capture));
        }
    }

    /// Asks the platform for `wanted` and records what it granted.
    fn set_capture(&mut self, wanted: CaptureState) {
        if let Some(window) = self.window.as_ref() {
            self.capture = hold_pointer(window, wanted);
        }
    }

    /// Drops every key the player was holding when the window went away.
    fn on_input_cleared(&mut self) {
        if let Some(app) = self.app.as_mut() {
            app.input().clear_held();
        }
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

/// What the player asked for by pressing `key`, if the binding table names it.
///
/// The declared table — W forward, S back, A strafe-left, D strafe-right, Space
/// jump — and it lives here because this is the only file that may name the
/// window library's key codes. `ADR-013` leaves this crate out of the coverage
/// denominator on the grounds that it holds no policy, and a five-row data table
/// is admitted as a narrow, recorded exception (specification §"Where each piece
/// lives"). It stays a *table*: what an action does is `mc-sim`'s, and the one
/// branch that acts on a `None` is `InputState::apply`'s, so nothing here
/// decides anything a test cannot ask about.
///
/// Physical key codes rather than logical keys, so a row names a position under
/// the player's left hand rather than a letter: the same four keys walk the
/// player on a QWERTZ keyboard and on an AZERTY one, whatever is printed on
/// them.
#[must_use]
pub const fn bound_action(key: KeyCode) -> Option<PlayerAction> {
    match key {
        KeyCode::KeyW => Some(PlayerAction::Forward),
        KeyCode::KeyS => Some(PlayerAction::Back),
        KeyCode::KeyA => Some(PlayerAction::StrafeLeft),
        KeyCode::KeyD => Some(PlayerAction::StrafeRight),
        KeyCode::Space => Some(PlayerAction::Jump),
        _ => None,
    }
}

/// One window event, in the renderer's vocabulary.
///
/// The catch-all is deliberate and is the reason this translation exists at all:
/// the library grows variants between versions, and every one of them arrives
/// here as something the loop already knows how to ignore.
///
/// Regaining focus is one of those: it needs nothing done, because the keys were
/// dropped when focus went and a key still held is pressed again by the player
/// or is not held at all.
#[must_use]
pub fn kind_of(event: &WindowEvent) -> WindowEventKind {
    match event {
        WindowEvent::CloseRequested => WindowEventKind::CloseRequested,
        WindowEvent::Resized(size) => WindowEventKind::Resized(SurfaceSize {
            width: size.width,
            height: size.height,
        }),
        WindowEvent::RedrawRequested => WindowEventKind::RedrawRequested,
        WindowEvent::Focused(false) => WindowEventKind::FocusLost,
        _ => WindowEventKind::Other,
    }
}

/// The grab the window library is asked for, to hold the pointer as `state`
/// describes.
///
/// `None` for the uncaptured state, which is the bottom of the ladder and is not
/// something to ask a window for — it is what is left when nothing was granted.
const fn grab_mode(state: CaptureState) -> Option<CursorGrabMode> {
    match state {
        CaptureState::Locked => Some(CursorGrabMode::Locked),
        CaptureState::Confined => Some(CursorGrabMode::Confined),
        CaptureState::Uncaptured => None,
    }
}

/// Holds the pointer as firmly as `wanted` asks and the platform allows, and
/// reports what was actually granted.
///
/// The refusals are the point: `winit` refuses a grab mode a platform does not
/// implement — a locked pointer on X11, a confined one on some Wayland
/// compositors — and every refusal walks one rung down
/// [`next_capture_attempt`]'s ladder rather than ending the run. The loop
/// terminates because that ladder descends and its bottom rung has no grab to
/// ask for.
fn hold_pointer(window: &Window, wanted: CaptureState) -> CaptureState {
    let mut attempt = wanted;
    while let Some(mode) = grab_mode(attempt) {
        if window.set_cursor_grab(mode).is_ok() {
            window.set_cursor_visible(false);
            return attempt;
        }
        attempt = next_capture_attempt(attempt);
    }
    release_pointer(window);
    CaptureState::Uncaptured
}

/// Gives the pointer back to the desktop.
///
/// A failure here is reported rather than swallowed: it leaves the player with a
/// cursor they asked to be let go of and cannot be, which is worth a line even
/// though there is nothing further this client can do about it.
fn release_pointer(window: &Window) {
    if let Err(failure) = window.set_cursor_grab(CursorGrabMode::None) {
        eprintln!("mycraft: the cursor could not be released: {failure}");
    }
    window.set_cursor_visible(true);
}
