//! The one wall clock this client reads, and the only production source allowed
//! to name one.
//!
//! # Why a port for a single implementation
//!
//! A wall clock is a source of nondeterminism, and the port is mandatory at
//! first use for that reason alone — the "no abstraction before three uses" rule
//! governs reuse-driven generalization and explicitly exempts boundaries like
//! this one. What it buys here is concrete rather than architectural: a frame
//! rate no test can drive is a readout no test can grade, and a *pacing* no test
//! can drive is a movement speed no test can grade — which is the defect this
//! port was renamed for. A suite that wanted to know where a walk ends after a
//! second of frames arriving 144 to the second would otherwise have to wait a
//! second and hope the scheduler agreed.
//!
//! # It is the frame's clock, not the overlay's
//!
//! It used to be named for the debug overlay, which was its first consumer and
//! for a while its only one. What it actually answers is how long the last frame
//! took, and *that* is what paces the simulation: the client spends whole tick
//! quanta out of the interval this port reports, so a walk covers the same ground
//! per second of elapsed time whatever rate the frames arrive at. A port is named
//! for its capability (`standards/global/code-quality.md` §3), and a reader
//! asking "what paces the simulation?" would not have looked under an overlay.
//!
//! It sits in `time/` rather than under `overlay/` for the same reason: the
//! renderer does not simulate, and a port carrying pacing must not read as
//! belonging to the subsystem that merely displays a number derived from it.
//!
//! # The port and the adapter share one file, on purpose
//!
//! This is the single file the wall-clock confinement scan exempts. Splitting
//! the adapter into a second file would need a second exemption, and an
//! exemption is the thing that guard exists to avoid needing — a scan with two
//! of them is one edit away from having three.
//!
//! # It reports elapsed time, never a date
//!
//! A frame time is the difference between two readings and nothing else, so the
//! port answers with a [`Duration`] since some fixed earlier moment rather than
//! with a point in time. That leaves no way to spell "what day is it" through
//! it, and it is what makes a driven fake a `Duration` a test owns rather than a
//! clock a test has to imitate.
//!
//! **`mc_testkit::frame::clock` is a different capability and is deliberately
//! not reused here.** That one is a readback deadline in a test-support crate;
//! reusing it would put test scaffolding on the product path.

use std::time::{Duration, Instant};

/// How much time has passed, as whoever is being asked measures it.
pub trait FrameClock {
    /// How long since the moment this clock measures from.
    ///
    /// Monotonic: a later call never answers with less than an earlier one, and
    /// the difference between two answers is what a frame time is made of.
    fn now_elapsed(&self) -> Duration;
}

/// The operating system's monotonic clock, measured from the moment this was
/// built.
///
/// [`Instant`] rather than [`std::time::SystemTime`]: a frame time is an
/// interval, and a clock a user can set backwards would report a negative one.
#[derive(Debug)]
pub struct SystemFrameClock {
    started: Instant,
}

impl SystemFrameClock {
    /// A clock measuring from now.
    ///
    /// Named for what it does rather than `new`, because the moment it captures
    /// is the whole of its state and a reader of the call site should see that
    /// the moment is *this* one.
    #[must_use]
    pub fn started_now() -> Self {
        Self {
            started: Instant::now(),
        }
    }
}

impl FrameClock for SystemFrameClock {
    fn now_elapsed(&self) -> Duration {
        self.started.elapsed()
    }
}
