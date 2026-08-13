//! What a client accumulates between ticks, and the intent it hands over.
//!
//! This is client-side behaviour that lives in the authority's crate on purpose
//! (architecture D-5). In MVP 3 the client still holds the keys down and still
//! sends a [`MovementIntent`] over the wire, and the server still clamps what
//! arrives — so accumulating here costs the authority nothing and buys the
//! accumulation a test. The client's whole contribution is the one line that
//! submits `input.take_intent()`.
//!
//! **Held keys survive a tick; the look delta does not.** A key is held until it
//! is released, so every tick between those two events carries its contribution.
//! Pointer motion is a *quantity* that arrived once, so the tick it feeds
//! consumes it — two motions before one tick add up, and the tick after them
//! starts from zero. An accumulator that got either the wrong way round is
//! smooth, total and reproducible while being wrong: a drained key stops the
//! player on the tick after the press, and a look delta that is never drained
//! turns the camera further on every tick that follows one flick of the pointer.
//!
//! **A key nothing binds cannot reach here.** [`InputState::press`] takes a
//! [`PlayerAction`], not a key, so the binding table is what refuses an unbound
//! key and [`InputState::apply`] is where that refusal is spent: the adapter
//! hands over whatever the table made of the key, `None` included, and the
//! branch that does nothing about `None` is here rather than in the crate
//! `ADR-013` leaves out of the coverage denominator.

use crate::player::MovementIntent;

/// How much of a turn one raw pointer count asks for, in radians.
///
/// Declared rather than measured, and deliberately not configurable: a settings
/// UI is Out of Scope, and a sensitivity read from anywhere but here would make
/// the same pointer motion mean different things in two runs of a replay that is
/// supposed to be reproducible.
const LOOK_SENSITIVITY: f32 = 0.0022;

/// What a bound key asks of the player.
///
/// The five the specification's binding table names, and nothing else. Which
/// *key* produces which action is the client's table; what an action does is
/// this crate's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerAction {
    Forward,
    Back,
    StrafeLeft,
    StrafeRight,
    Jump,
}

/// The keys a client is holding, and the pointer motion it has seen since the
/// last tick.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct InputState {
    held: HeldKeys,
    yaw_delta: f32,
    pitch_delta: f32,
}

/// Which of the five bound keys are down.
///
/// A field per key rather than a set, because the walk request is two *opposed
/// pairs* and both halves of a pair are held at once often enough to matter: a
/// player rocking between W and S releases them in either order, and only
/// remembering both separately gets that right.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct HeldKeys {
    forward: bool,
    back: bool,
    strafe_left: bool,
    strafe_right: bool,
    jump: bool,
}

impl InputState {
    /// Records that `action`'s key went down.
    pub const fn press(&mut self, action: PlayerAction) {
        self.hold(action, true);
    }

    /// Records that `action`'s key came up.
    ///
    /// Only that key's own contribution goes: the others are still being held,
    /// and a release that cleared them all would drop a walk the player never
    /// stopped asking for.
    pub const fn release(&mut self, action: PlayerAction) {
        self.hold(action, false);
    }

    /// One key transition, as the binding table read it.
    ///
    /// `None` is a key no row of the table names, and it changes nothing. This
    /// is the only decision in the whole key path, which is why it is here and
    /// not in the adapter.
    pub const fn apply(&mut self, action: Option<PlayerAction>, pressed: bool) {
        if let Some(action) = action {
            self.hold(action, pressed);
        }
    }

    /// Forgets every key that is being held.
    ///
    /// What a window losing focus asks for: the key-up events for the keys the
    /// player was holding are delivered to whatever has focus now and never
    /// arrive here, so a client that kept them would walk into a wall for as
    /// long as it was looked away from — and would still be walking when the
    /// player came back.
    pub const fn clear_held(&mut self) {
        self.held = HeldKeys {
            forward: false,
            back: false,
            strafe_left: false,
            strafe_right: false,
            jump: false,
        };
    }

    /// Accumulates raw pointer motion into the pending look delta.
    ///
    /// `raw_x` and `raw_y` are device counts in screen directions: +x is right
    /// and +y is *down*, which is why the pitch takes the opposite sign — a
    /// pointer pushed down looks down. Adding rather than replacing is what
    /// makes a fast flick, which the pointer reports in several events before
    /// one tick, arrive as the one turn it was.
    pub const fn look(&mut self, raw_x: f32, raw_y: f32) {
        self.yaw_delta += raw_x * LOOK_SENSITIVITY;
        self.pitch_delta -= raw_y * LOOK_SENSITIVITY;
    }

    /// The intent one tick is advanced with.
    ///
    /// Draining the look delta and keeping the held keys is the difference
    /// between the two kinds of input, and it is the whole reason this is a
    /// method rather than a conversion.
    pub const fn take_intent(&mut self) -> MovementIntent {
        let intent = MovementIntent {
            forward: deflection(self.held.forward, self.held.back),
            strafe: deflection(self.held.strafe_right, self.held.strafe_left),
            yaw_delta: self.yaw_delta,
            pitch_delta: self.pitch_delta,
            jump: self.held.jump,
        };
        self.yaw_delta = 0.0;
        self.pitch_delta = 0.0;
        intent
    }

    /// Puts `action`'s key into the state `pressed` describes.
    ///
    /// A press and a release are the same transition in opposite directions, and
    /// writing the state rather than toggling it is what makes a repeat — which
    /// the operating system delivers for as long as a key is held — cost
    /// nothing.
    const fn hold(&mut self, action: PlayerAction, pressed: bool) {
        match action {
            PlayerAction::Forward => self.held.forward = pressed,
            PlayerAction::Back => self.held.back = pressed,
            PlayerAction::StrafeLeft => self.held.strafe_left = pressed,
            PlayerAction::StrafeRight => self.held.strafe_right = pressed,
            PlayerAction::Jump => self.held.jump = pressed,
        }
    }
}

/// One axis of the walk request, from the two keys that deflect it.
///
/// The pair is one axis rather than two requests, so holding both is a walk of
/// zero rather than a walk of two: they are opposite deflections of equal size
/// and they cancel. Summing magnitudes instead, or taking whichever key arrived
/// later, walks a player who is asking to stand still.
const fn deflection(positive: bool, negative: bool) -> f32 {
    match (positive, negative) {
        (true, false) => 1.0,
        (false, true) => -1.0,
        (true, true) | (false, false) => 0.0,
    }
}
