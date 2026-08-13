//! The client's input dispatch: everything a keystroke, a pointer motion or a
//! click decides, with no window and no graphics device anywhere in it.
//!
//! **This is the half of the client a test can drive.** `events.rs` on the other
//! side of the seam spells the window library's vocabulary and decides nothing;
//! what is here reads the policies `mc_render::window` declares, accumulates what
//! the player asked for, and advances the tick that spends it. A test drives it
//! by dispatching events at `events.rs`'s entries, so the translation runs too
//! and neither half is asserted against a copy of itself.
//!
//! **It hands out no borrow of what it owns.** There is no accessor returning the
//! accumulator or the simulation, by reference or otherwise, which is what makes
//! "nothing outside this file drains the input or advances the tick" a property of
//! the type rather than of a text scan. The failure that shape exists to prevent
//! is a test that asks the accumulator directly what the client should have asked
//! it — an agreement between two callers of one function, green while the client
//! it is supposed to be watching submits nothing at all.
//!
//! **The capture ladder is walked here rather than by the platform.** The port
//! below attempts exactly one grab and reports whether it was granted; which
//! attempt follows a refusal, when the walk stops, and what is left when nothing
//! was granted are decisions, and a port that took them would put them back on the
//! side of the seam no test can reach.

use std::fmt;
use std::sync::Arc;

use mc_render::window::{
    CaptureState, accepts_pointer_motion, capture_after_click, capture_after_escape,
    first_capture_attempt, next_capture_attempt,
};
use mc_sim::player::{InputState, PlayerAction};
use mc_sim::simulation::{SimSnapshot, Simulation};

/// What the platform can be asked to do with the pointer.
///
/// One attempt at a time, deliberately: a port that took a capture and answered
/// with the one it settled on would be walking the ladder, and the ladder is the
/// decision this trait exists to keep out of the platform.
pub trait PointerPlatform {
    /// Asks for `capture` and reports whether the platform granted it.
    ///
    /// Never called with [`CaptureState::Uncaptured`] — that is the bottom of
    /// the ladder, which is a state rather than something to ask a window for.
    fn grab(&mut self, capture: CaptureState) -> bool;

    /// Gives the pointer back to the desktop.
    fn release(&mut self);

    /// Shows or hides the cursor.
    fn show_cursor(&mut self, visible: bool);
}

/// One thing a platform was asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PointerAsk {
    Grab(CaptureState),
    Release,
    CursorVisible(bool),
}

/// One key of the keyboard, in the vocabulary the session decides in.
///
/// The same shape as `WindowEventKind`, and for the same reason: the catch-all
/// absorbs every key code the client cannot tell apart, so the window library
/// growing key codes between versions changes nothing on this side of the seam.
///
/// Named `KeyKind` rather than `Key` because the window library has a `Key` of
/// its own and the adapter imports both.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyKind {
    W,
    S,
    A,
    D,
    Space,
    Escape,
    Other,
}

/// What the player asked for by pressing `key`, if the binding table names it.
///
/// The declared table — W forward, S back, A strafe-left, D strafe-right, Space
/// jump. It stays a *table*: what an action does is `mc-sim`'s, and the one
/// branch that acts on a `None` is `InputState::apply`'s, so nothing here
/// decides anything a test cannot ask about.
///
/// **Private, and that is load-bearing.** Nothing outside this module calls the
/// table, and keeping it so is what makes a test that wanted to ask the table
/// the question the session asks it a compile error rather than a text-scan
/// finding.
#[must_use]
const fn bound_action(key: KeyKind) -> Option<PlayerAction> {
    match key {
        KeyKind::W => Some(PlayerAction::Forward),
        KeyKind::S => Some(PlayerAction::Back),
        KeyKind::A => Some(PlayerAction::StrafeLeft),
        KeyKind::D => Some(PlayerAction::StrafeRight),
        KeyKind::Space => Some(PlayerAction::Jump),
        KeyKind::Escape | KeyKind::Other => None,
    }
}

/// Everything the client decides about input, and the world it decides it over.
pub struct Session {
    /// What the player has asked for since the last tick. It outlives the
    /// simulation being absent: keys held and pointer motion made while the
    /// world is still generating are the player's input all the same, and the
    /// first tick is what spends them.
    input: InputState,
    /// How firmly the pointer is currently held. What the platform granted, not
    /// what was asked for — pointer motion is admitted against this, so a
    /// refused grab must not leave the client believing it has the cursor.
    capture: CaptureState,
    /// The simulation, once there is a world to place the player in. `None`
    /// while the preparation worker is still generating one — the spawn is
    /// derived from the world, so no tick can be advanced before it lands.
    simulation: Option<Simulation>,
    pointer: Box<dyn PointerPlatform>,
}

impl Session {
    /// A session over `pointer`, which it has already asked for the first
    /// capture.
    ///
    /// The ask is part of construction rather than a second call, and that is
    /// not tidiness: a separate `start()` would let "the client never asks the
    /// platform to hold the pointer" be spelled by deleting one line from the
    /// window-facing side, where no test could see it. Folded in here, there is
    /// nowhere to spell it that a scenario does not watch.
    #[must_use]
    pub fn new(pointer: Box<dyn PointerPlatform>) -> Self {
        let mut session = Self {
            input: InputState::default(),
            capture: CaptureState::Uncaptured,
            simulation: None,
            pointer,
        };
        session.hold(first_capture_attempt());
        session
    }

    /// One key transition, below the reduction of a key event to a key and a
    /// pressed flag.
    ///
    /// Escape is not in the binding table and is not the player asking to move:
    /// it is how they get their desktop back, so it is spent on the capture
    /// policy and never reaches the accumulator. Every other key is whatever the
    /// table made of it, `None` included.
    /// Only Escape's press does anything. Releasing it is the player letting go
    /// of a key they have already spent, and asking for the capture back a
    /// moment after giving it up is exactly what the release policy exists to
    /// not do.
    pub fn on_key(&mut self, key: KeyKind, pressed: bool) {
        match key {
            KeyKind::Escape if pressed => self.hold(capture_after_escape(self.capture)),
            KeyKind::Escape => {}
            bindable => self.input.apply(bound_action(bindable), pressed),
        }
    }

    /// Raw pointer motion in device counts.
    ///
    /// Admitted against the capture the platform actually granted, never the one
    /// that was asked for: a refused grab leaves the cursor the desktop's, and
    /// turning the camera with it would be the game reading input it was not
    /// given — and holding the turn ready for the player when they came back.
    ///
    /// The narrowing to `f32` loses nothing a pointer can report, and the
    /// accumulator is `f32` because the angle it becomes is.
    pub fn on_pointer_motion(&mut self, raw_x: f64, raw_y: f64) {
        if !accepts_pointer_motion(self.capture) {
            return;
        }
        self.input.look(raw_x as f32, raw_y as f32);
    }

    /// A mouse button going down, which is how the player takes the cursor back
    /// after Escape gave it away.
    ///
    /// It walks the ladder unconditionally, including when the pointer is
    /// already held. The re-ask is not redundant: nothing here tracks a capture
    /// the compositor silently dropped, so asking again is the only thing that
    /// recovers one.
    pub fn on_mouse_pressed(&mut self) {
        self.hold(capture_after_click(self.capture));
    }

    /// Drops every key the player was holding when the window went away.
    pub const fn on_input_cleared(&mut self) {
        self.input.clear_held();
    }

    /// The world landed and there is something to advance.
    pub fn attach_simulation(&mut self, simulation: Simulation) {
        self.simulation = Some(simulation);
    }

    /// Advances one tick under everything accumulated since the last one.
    ///
    /// **Nothing is drained when there is no simulation.** A tick step taken
    /// before the world lands has nothing to advance, and the input made while
    /// it was still generating is the player's input all the same — the first
    /// tick after it lands is what spends it.
    pub fn tick(&mut self) {
        if let Some(simulation) = self.simulation.as_mut() {
            simulation.advance(self.input.take_intent());
        }
    }

    /// Whatever the simulation published most recently, if there is one.
    #[must_use]
    pub fn latest(&self) -> Option<Arc<SimSnapshot>> {
        self.simulation.as_ref().map(Simulation::latest)
    }

    /// Holds the pointer as firmly as `wanted` asks and the platform allows.
    ///
    /// The refusals are the point: a platform refuses a grab mode it does not
    /// implement — a locked pointer on X11, a confined one on some Wayland
    /// compositors — and every refusal walks one rung down the declared ladder
    /// rather than ending the run. The walk terminates because that ladder
    /// descends and its bottom rung is a state rather than a grab, which is what
    /// makes a refused pointer a degraded game rather than a failed start.
    fn hold(&mut self, wanted: CaptureState) {
        let mut attempt = wanted;
        // Walk on while there is still a rung to try and the platform refused
        // the one being tried. The bottom rung is never asked for, because it is
        // what is left when nothing was granted rather than a grab.
        while attempt != CaptureState::Uncaptured && !self.pointer.grab(attempt) {
            attempt = next_capture_attempt(attempt);
        }
        let held = attempt != CaptureState::Uncaptured;
        if !held {
            self.pointer.release();
        }
        self.pointer.show_cursor(!held);
        self.capture = attempt;
    }
}

/// The platform is a trait object with no `Debug` of its own, so what is shown
/// is what a reader of a panic message can use: what the player has asked for,
/// how firmly the pointer is held, and what is being advanced.
impl fmt::Debug for Session {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Session")
            .field("input", &self.input)
            .field("capture", &self.capture)
            .field("simulation", &self.simulation)
            .finish_non_exhaustive()
    }
}
