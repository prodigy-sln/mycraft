//! The one file in this crate that knows what a window is.
//!
//! Everything the loop does with an event is decided by
//! `mc_render::window::window_event_action`, a pure function tested without a
//! display server, or by [`Session`](crate::session::Session), which a test
//! drives directly. What is here is the translation into their vocabulary and
//! the handful of calls that need the real thing — creating the window, asking
//! it for its size, telling it to redraw, and asking the operating system to
//! hold the pointer.
//!
//! # Half of this file is reachable by a test and half is not, and which half is
//! which is not visible in it
//!
//! The three `dispatch_*` entries below are crossed by the event loop **and** by
//! `tests/support/input/`, so everything they call — the key table's spelling,
//! the window-event translation, the mouse filter, the pointer destructure — runs
//! under test. What stays unreachable is the reduction of a `winit::event::KeyEvent`
//! to a key code and a pressed flag, the creation of the window and the surface,
//! and this file's forwarding into the entries.
//!
//! That split is a fact about `KeyEvent`'s privacy rather than anything a reader
//! can see here, which is why it is written down: the next reader who assumes the
//! whole file is unreachable will quietly move a decision into it. `Session` is
//! where a decision goes, and `tests/seam_boundaries.rs` fails the build if one
//! arrives here instead.
//!
//! `tests/winit_boundary.rs` fails the build if a second file in this crate names
//! the library. That is not decoration: ADR-013 leaves this whole crate out of the
//! coverage denominator on the stated grounds that it holds the adapter and the
//! wiring and no policy, and a decision that drifted in here would leave the
//! denominator with it, moving no number as it went.

use std::sync::Arc;

use mc_render::surface::SurfaceSize;
use mc_render::window::{CaptureState, Ending, LoopAction, WindowEventKind, window_event_action};
use winit::application::ApplicationHandler;
use winit::event::{DeviceEvent, DeviceId, ElementState, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{CursorGrabMode, Window, WindowId};

use crate::app::App;
use crate::gpu_startup::{Gpu, create_surface};
use crate::session::{KeyKind, MouseButtonKind, PointerPlatform, Session};
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
        session: None,
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
    /// Everything the client decides about input, once there is a window to ask
    /// for the pointer. Events arriving before then are dropped, as they always
    /// were.
    session: Option<Session>,
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

    /// One window event, handed to the session and then acted on.
    ///
    /// With no session yet — every event before `resumed` — the loop still needs
    /// an answer, so the same pure policy is asked directly and a close request
    /// arriving that early still exits.
    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        let action = self.session.as_mut().map_or_else(
            || window_event_action(&kind_of(&event)),
            |session| dispatch_window_event(session, &event),
        );
        match action {
            LoopAction::Exit => self.stop(event_loop, Ending::Closed),
            LoopAction::Resize(size) => self.on_resize(size),
            LoopAction::Redraw => self.on_redraw(event_loop),
            // The session has already dropped what a lost focus asks it to, and
            // an ignored event asks for nothing.
            LoopAction::ClearInput | LoopAction::Ignore => {}
        }
    }

    /// Raw pointer motion, which is relative and arrives whatever the cursor is
    /// doing — including while it is the desktop's.
    fn device_event(&mut self, _: &ActiveEventLoop, _: DeviceId, event: DeviceEvent) {
        if let Some(session) = self.session.as_mut() {
            dispatch_device_event(session, &event);
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

        // Built here, before the app, because building it is what asks the
        // platform for the pointer — the same point in startup that ask was made
        // at before there was a session to make it.
        self.session = Some(Session::new(Box::new(WindowPointer {
            window: Arc::clone(&window),
        })));
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
        let ended = match (self.app.as_mut(), self.session.as_mut()) {
            (Some(app), Some(session)) => app.redraw(session),
            _ => None,
        };
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

/// What the session makes of one window event, and what the loop does about the
/// rest.
///
/// **Both the event loop and the harness enter here.** The keyboard is routed
/// through the reduction below rather than handed on whole, because that
/// reduction is the one thing on this side of the seam a test cannot reach.
pub fn dispatch_window_event(session: &mut Session, event: &WindowEvent) -> LoopAction {
    if let WindowEvent::KeyboardInput { event: key, .. } = event
        && let PhysicalKey::Code(code) = key.physical_key
    {
        dispatch_key(session, code, key.state.is_pressed());
    }
    // A click is how the player takes the cursor back after Escape gave it
    // away, and it is also how they ask the world for something. Both are the
    // session's answers; what is decided here is only which of the library's
    // buttons this is, because the library's spelling cannot travel further.
    if let WindowEvent::MouseInput {
        state: ElementState::Pressed,
        button,
        ..
    } = event
    {
        session.on_mouse_pressed(mouse_button_kind_of(*button));
    }
    let action = window_event_action(&kind_of(event));
    if action == LoopAction::ClearInput {
        session.on_input_cleared();
    }
    action
}

/// Raw pointer motion, which is relative and arrives whatever the cursor is
/// doing — including while it is the desktop's.
///
/// The window's own cursor-moved event carries a *position*, which stops at the
/// edge of the screen and so cannot express a turn that keeps going.
///
/// **An unconditional destructure and forward, deciding nothing.** Whether
/// motion arriving now is the player looking around is the session's question,
/// and an adapter that filtered first would move that decision to the side of
/// the seam no test can reach.
pub fn dispatch_device_event(session: &mut Session, event: &DeviceEvent) {
    if let DeviceEvent::MouseMotion { delta: (x, y) } = event {
        session.on_pointer_motion(*x, *y);
    }
}

/// One key transition.
///
/// The harness enters here rather than at [`dispatch_window_event`] because
/// `winit::event::KeyEvent` cannot be constructed outside the library: its
/// `platform_specific` field is crate-private and it has neither a constructor
/// nor a `Default`. A real window would not help either, because the library
/// synthesizes no key events.
pub fn dispatch_key(session: &mut Session, key: KeyCode, pressed: bool) {
    session.on_key(key_kind_of(key), pressed);
}

/// One key code, in the session's vocabulary.
///
/// The catch-all absorbs every key the client cannot tell apart, so a library
/// upgrade that adds key codes changes nothing here. Physical key codes rather
/// than logical keys, so a row of the table below names a position under the
/// player's left hand rather than a letter: the same four keys walk the player
/// on a QWERTZ keyboard and on an AZERTY one, whatever is printed on them.
const fn key_kind_of(key: KeyCode) -> KeyKind {
    match key {
        KeyCode::KeyW => KeyKind::W,
        KeyCode::KeyS => KeyKind::S,
        KeyCode::KeyA => KeyKind::A,
        KeyCode::KeyD => KeyKind::D,
        KeyCode::Space => KeyKind::Space,
        KeyCode::Escape => KeyKind::Escape,
        _ => KeyKind::Other,
    }
}

/// One mouse button, in the session's vocabulary.
///
/// The catch-all absorbs every button the client cannot tell apart, so a library
/// upgrade that adds buttons changes nothing here. It is a translation and not a
/// decision: that a middle button asks the world for nothing is the session's
/// answer, and this arm only says which button arrived.
const fn mouse_button_kind_of(button: MouseButton) -> MouseButtonKind {
    match button {
        MouseButton::Left => MouseButtonKind::Left,
        MouseButton::Right => MouseButtonKind::Right,
        _ => MouseButtonKind::Other,
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
fn kind_of(event: &WindowEvent) -> WindowEventKind {
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

/// The pointer as the window library holds it.
///
/// One attempt per call and no ladder: which capture follows a refusal, and what
/// is left when nothing is granted, are the session's decisions. This is the
/// asking.
struct WindowPointer {
    window: Arc<Window>,
}

impl PointerPlatform for WindowPointer {
    /// The library refuses a grab mode a platform does not implement — a locked
    /// pointer on X11, a confined one on some Wayland compositors — and a
    /// refusal is an answer rather than a failure, so nothing here is
    /// propagated.
    fn grab(&mut self, capture: CaptureState) -> bool {
        grab_mode(capture).is_some_and(|mode| self.window.set_cursor_grab(mode).is_ok())
    }

    /// A failure here is reported rather than swallowed: it leaves the player
    /// with a cursor they asked to be let go of and cannot be, which is worth a
    /// line even though there is nothing further this client can do about it.
    fn release(&mut self) {
        if let Err(failure) = self.window.set_cursor_grab(CursorGrabMode::None) {
            eprintln!("mycraft: the cursor could not be released: {failure}");
        }
    }

    fn show_cursor(&mut self, visible: bool) {
        self.window.set_cursor_visible(visible);
    }
}
