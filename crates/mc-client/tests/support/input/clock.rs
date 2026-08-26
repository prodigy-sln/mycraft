//! A clock the suite moves by hand.
//!
//! Every frame a scenario draws costs exactly what the scenario says it cost.
//! Nothing here reads a system clock, waits, or sleeps — a harness that had to
//! would be measuring the machine the suite runs on rather than the pacing under
//! test, and would be the flaky test the discipline exists to prevent.
//!
//! It answers with time *elapsed* rather than with an interval, because that is
//! what the port answers with: the client is what turns two readings into a frame
//! time, and a harness handing over intervals directly would be doing the
//! subtraction the client is supposed to be watched doing.

use std::cell::Cell;
use std::time::Duration;

use mc_render::time::clock::FrameClock;

/// A clock that reads whatever it has been advanced to.
///
/// Interior mutability because the port answers through `&self` — a clock is
/// something you read, not something reading changes — while a driver has to be
/// able to move it between two readings.
#[derive(Debug, Default)]
pub struct DrivenClock {
    elapsed: Cell<Duration>,
}

impl DrivenClock {
    /// Moves this clock forward by `by`.
    ///
    /// `move_on` rather than `advance`: the seam guard next door forbids
    /// `.advance(` anywhere outside the client's core, because advancing is what
    /// a simulation does and a harness that did it would be deciding what it is
    /// supposed to be watching. A clock moving on is a different verb for a
    /// different act, and spelling it as one keeps the guard's needle honest
    /// rather than dodged by a suffix.
    pub fn move_on(&self, by: Duration) {
        self.elapsed.set(self.elapsed.get() + by);
    }
}

impl FrameClock for DrivenClock {
    fn now_elapsed(&self) -> Duration {
        self.elapsed.get()
    }
}
