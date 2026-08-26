//! How long the last frame took, and what the core owes the simulation for it.
//!
//! **Not an accumulator.** That word already means the *input* accumulator in
//! the module above — what the player has asked for since the last tick — and a
//! second meaning four lines away would make both unreadable. What is held here
//! is unspent frame time, and the operation on it is spending whole quanta.
//!
//! # The first frame is measured from the moment the frame path began
//!
//! There is no earlier frame to measure the first one against, and the two
//! honest answers are "no interval" and "the interval since the clock started".
//! This takes the second, because the clock is started when the object that
//! draws frames is built: the time until the first frame is presented is time
//! that first frame genuinely took. It also removes an `Option` that every
//! caller and every driver would otherwise have to have an opinion about, and
//! whatever a slow start delivers is bounded like any other stall.

use std::time::Duration;

use mc_sim::player::TICK_QUANTUM;

/// How many ticks one frame may ever buy.
///
/// **Derived from the floor, not the ceiling.** The bound has to sit *above*
/// every frame interval a working machine produces, or a slow machine silently
/// loses simulated time and the world crawls — which is this fix's own defect
/// with the sign flipped, and harder to notice, because a slow game reads as a
/// slow machine. Ten frames a second is the slowest rate at which a game is
/// arguably being played rather than hung, and that is a 100 ms interval;
/// fifteen quanta is 250 ms, 2.5× above it.
///
/// The ceiling is that the work stays bounded: a tick is arithmetic over a small
/// voxel neighbourhood, so fifteen of them cost a fraction of a frame budget and
/// a frame that hits the cap cannot spiral into the next one.
///
/// Spelled as a count of quanta rather than as 250 ms so that a frame at the cap
/// spends exactly this many and leaves nothing over. Written as a duration,
/// 250 ms is 15.000 002 quanta or 14.999 998 of them depending on which way a
/// nanosecond falls, and one of those spends fourteen.
const CATCH_UP_TICKS: u32 = 15;

/// What one frame's elapsed time buys, and what is left of it.
#[derive(Debug, Default)]
pub struct FramePacing {
    /// What the clock read when the previous frame was timed.
    ///
    /// Zero before any has been, which is what makes the first frame's interval
    /// the time since the clock started rather than a special case.
    previous: Duration,
    /// Elapsed time delivered to this core and not yet spent on a tick.
    ///
    /// Below one quantum at all times once a frame has been spent, because
    /// spending takes every whole quantum there is. It is what makes two frames
    /// of half a quantum buy the tick that one frame of a whole quantum does —
    /// and, more to the point, what makes the ticks a stretch of time buys
    /// depend on the stretch rather than on how many frames it was cut into.
    unspent: Duration,
}

impl FramePacing {
    /// How long the frame just drawn took, given what the clock reads now.
    ///
    /// A subtraction that saturates rather than one that could go negative: the
    /// port's answer is monotonic, so a later reading below an earlier one is a
    /// broken clock rather than a frame that took less than no time, and a
    /// zero-length frame is the closest true thing to say about it.
    pub fn timed(&mut self, reading: Duration) -> Duration {
        let took = reading.saturating_sub(self.previous);
        self.previous = reading;
        took
    }

    /// How many whole ticks `took` of elapsed time buys, bounded at
    /// [`CATCH_UP_TICKS`].
    ///
    /// **The bound is applied before the time is carried, and the surplus is
    /// discarded rather than held.** A debugger pause, a breakpoint or a laptop
    /// resuming from sleep delivers unbounded elapsed time, and carrying it would
    /// replay it — a hang whose length is however long the machine was away,
    /// arriving *after* the stall and lasting longer than it did. A single-player
    /// client losing that time is the right answer: nobody was playing.
    ///
    /// Whatever is below a whole quantum stays. The arithmetic is over whole
    /// nanoseconds throughout, so nothing is lost at a frame boundary and the
    /// ticks a stretch of elapsed time buys are the same however it was cut up.
    ///
    /// **Subtracted one quantum at a time rather than divided**, which is this
    /// repository's standing answer to `clippy::integer_division` being a gate
    /// error. Elsewhere that answer is a shift, because the divisor is a power of
    /// two; a sixtieth of a second is not, and a float division would round —
    /// exactly three quanta can come back as 2.999 999 999 999 999 6 and floor to
    /// two, which is a tick silently lost per frame. The loop is bounded by the
    /// clamp above: what is carried in is below one quantum and what is added is
    /// at most [`CATCH_UP_TICKS`] of them, so it turns at most that many times.
    pub fn spend(&mut self, took: Duration) -> u32 {
        self.unspent += took.min(CATCH_UP_TICKS * TICK_QUANTUM);
        let mut ticks = 0;
        while let Some(left) = self.unspent.checked_sub(TICK_QUANTUM) {
            self.unspent = left;
            ticks += 1;
        }
        ticks
    }
}
