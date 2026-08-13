//! The client's own input dispatch, driven with no event loop, no window and no
//! graphics device.
//!
//! # Every method here is one call into the client's dispatch
//!
//! That is the whole design constraint, and it is the single most likely way the
//! feature this harness serves fails. A harness that read a key table itself, or
//! decided for itself whether pointer motion counts, or drained the accumulator
//! and stepped the simulation, would agree with the client by construction: every
//! scenario written against it would be green while the client submitted nothing.
//! So nothing below matches on a key, weighs a capture, or builds an intent —
//! each method hands a value to `mc_client::events` and stops. A sibling text
//! guard, `tests/seam_boundaries.rs`, is what keeps it that way.
//!
//! # Why it enters where it does
//!
//! Real window and device events are constructed and dispatched whole, so the
//! client's own translation of them runs under test. The keyboard is the one
//! exception and it is the library's doing rather than a choice: a key event
//! cannot be built outside the window library, so the keyboard enters one level
//! below, at the reduction to a key code and a pressed flag.
//!
//! # The world it drives is declared, not empty
//!
//! Gravity acts on every tick, so a world that answered "nothing is solid" would
//! put the player in free fall and turn every comparison against a no-input
//! control into a comparison of two falls. The fixture is a floor to stand on and
//! nothing else — see [`world`].

// Each scenario binary links the whole harness and drives a subset of it.
#![allow(dead_code)]

/// The platform the session asks for the pointer, recording what it was asked.
mod platform;
/// The world a driven tick resolves the player's motion against.
mod world;

use std::sync::Arc;

use mc_client::events::{dispatch_device_event, dispatch_key, dispatch_window_event};
use mc_client::session::{PointerAsk, Session};
use mc_render::window::CaptureState;
use mc_sim::simulation::SimSnapshot;
use winit::event::{DeviceEvent, WindowEvent};
use winit::keyboard::KeyCode;

use platform::{PointerLog, RecordingPlatform};

/// A client's dispatch, driven with no window and no adapter.
#[derive(Debug)]
pub struct InputHarness {
    session: Session,
    log: PointerLog,
}

impl InputHarness {
    /// A session over a platform granting exactly `granted` and refusing
    /// everything else, with no world yet.
    ///
    /// The session asks for its first capture while it is being built, so the
    /// asks this records begin before any event has been dispatched.
    #[must_use]
    pub fn granting(granted: &[CaptureState]) -> Self {
        let (platform, log) = RecordingPlatform::granting(granted);
        Self {
            session: Session::new(Box::new(platform)),
            log,
        }
    }

    /// The common case: a platform that grants a locked pointer.
    #[must_use]
    pub fn started() -> Self {
        Self::granting(&[CaptureState::Locked])
    }

    /// Puts a player on the declared ground plane and hands it to the session.
    pub fn start_world(&mut self) {
        self.session.attach_simulation(world::ground_plane());
    }

    /// A key going down.
    pub fn press(&mut self, key: KeyCode) {
        dispatch_key(&mut self.session, key, true);
    }

    /// The same key coming back up.
    pub fn release(&mut self, key: KeyCode) {
        dispatch_key(&mut self.session, key, false);
    }

    /// The pointer moving by a number of raw device counts.
    ///
    /// A real device event handed to the whole device-event entry, so the pair
    /// the client destructures out of it is destructured under test: the two
    /// axes are one type and one call apart, and a swap between them or a
    /// flipped vertical sign is an upside-down world that nothing here would
    /// notice if the harness handed the two numbers over already separated.
    ///
    /// It is dispatched whatever the client is doing with the cursor. Whether
    /// motion arriving now is the player looking around is the client's question
    /// to answer, and a harness that asked it first would agree with the client
    /// by construction and go on agreeing after the client stopped asking.
    pub fn move_pointer(&mut self, raw_x: f64, raw_y: f64) {
        dispatch_device_event(
            &mut self.session,
            &DeviceEvent::MouseMotion {
                delta: (raw_x, raw_y),
            },
        );
    }

    /// The window telling the client it no longer has focus.
    ///
    /// A real `Focused(false)` handed to the whole window-event entry rather than
    /// a direct call to the session's own clear: what a lost window *means* is
    /// the client's translation to make, and a harness that made it here would
    /// leave `kind_of` and the loop's own policy asserted by nothing.
    pub fn lose_focus(&mut self) {
        let _ = dispatch_window_event(&mut self.session, &WindowEvent::Focused(false));
    }

    /// One tick step, and whatever it left published.
    ///
    /// `None` when no world has been started: a tick step with nothing to
    /// advance publishes nothing, which is a state the client is genuinely in
    /// for the first frames of every run.
    pub fn tick(&mut self) -> Option<Arc<SimSnapshot>> {
        self.session.tick();
        self.session.latest()
    }

    /// What `ticks` tick steps publish, in order.
    ///
    /// Shorter than `ticks` exactly when some of them had no world to advance,
    /// so a caller that expected a snapshot per step can see that it did not get
    /// one rather than reading a repeat of the last.
    pub fn ticks(&mut self, ticks: u32) -> Vec<Arc<SimSnapshot>> {
        (0..ticks).filter_map(|_| self.tick()).collect()
    }

    /// The captures the platform was asked for, in order.
    ///
    /// Derived from the ask log by filtering rather than counted beside it, so
    /// there is one record of what happened and a released pointer can never be
    /// miscounted as a captured one.
    #[must_use]
    pub fn grabs(&self) -> Vec<CaptureState> {
        self.log
            .asks()
            .into_iter()
            .filter_map(|ask| match ask {
                PointerAsk::Grab(capture) => Some(capture),
                PointerAsk::Release | PointerAsk::CursorVisible(_) => None,
            })
            .collect()
    }
}
