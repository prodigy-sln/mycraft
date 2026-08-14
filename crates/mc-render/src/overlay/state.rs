//! What the debug overlay knows, and the lines it publishes for whoever paints
//! them.
//!
//! # The readout is a value, so it can be graded without a device
//!
//! Nothing here draws. What the overlay produces is four readings and the lines
//! they spell, which is what lets every claim about *what the overlay shows* be
//! an assertion over plain values — no adapter, no window, no toolkit. The
//! painting is one file in `gpu/` and is the only thing that needs any of those.
//!
//! # The two world readings are `Option`, and that is structural rather than
//! cosmetic
//!
//! A client draws frames before its world lands: the preparation runs on a
//! worker and the frame path draws until it finishes. During those frames there
//! is no player, so there is no position and no column — and the overlay says so
//! by publishing neither, rather than by publishing a zero or the last one it
//! saw. A zeroed field would read as a player standing at the origin, which is a
//! place a player can genuinely stand, so nothing downstream could tell the two
//! apart. `None` is the only spelling of "there is no answer yet" that a reader
//! cannot mistake for an answer.
//!
//! # The frame readings do not wait for the world
//!
//! They come from the overlay's own clock rather than from the simulation, so
//! they are exactly as available before the world lands as after — which is the
//! point of an instrument for diagnosing a client that is taking too long to
//! start.

use std::collections::VecDeque;
use std::time::Duration;

use glam::Vec3;
use mc_world::column::{ColumnCoordinate, column_containing};

use crate::overlay::clock::OverlayClock;

/// How many milliseconds a second holds.
const MILLIS_PER_SECOND: f64 = 1000.0;

/// How many decimal places each reading is shown to.
///
/// Each is the precision that reading is *worth*, which is a different question
/// per line. A position to a thousandth of a block is finer than anything a
/// player can stand on and is what makes a coordinate comparable against a
/// section boundary. A frame rate to a tenth is as much as a number that moves
/// every frame can support; a frame time to a hundredth of a millisecond is the
/// scale a stutter is argued at, and it is the same reading shown the other way
/// round, so it is shown more finely rather than less.
const POSITION_PLACES: usize = 3;
/// See [`POSITION_PLACES`].
const FRAME_RATE_PLACES: usize = 1;
/// See [`POSITION_PLACES`].
const FRAME_TIME_PLACES: usize = 2;

/// How many frame times the ring remembers.
///
/// One second of frames on a client drawing sixty a second, which is what makes
/// the mean a reading of the frame rate *now*: a stutter reaches the mean within
/// a second of starting and has left it a second after it stopped. The bound is
/// the whole point — a mean over every frame since launch is a number nothing can
/// move, and somebody watching the overlay is watching it because something just
/// changed.
///
/// **Nothing grades the value**, measured rather than assumed: every interval
/// this crate's suite drives is the same length, so a mean over one of them is
/// the mean over nine and a ring of any capacity reads alike. Recorded where the
/// number is, because the day a scenario cares about a stutter it is this
/// constant it will be about.
const FRAMES_REMEMBERED: usize = 60;

/// Everything the debug overlay knows: whether it is being shown, and how long
/// the frames just before this one took.
///
/// [`Default`] is the hidden overlay with nothing timed yet, which is the state
/// every client starts in.
#[derive(Debug, Default)]
pub struct DebugOverlay {
    visible: bool,
    /// How long each of the last frames took, bounded at
    /// [`FRAMES_REMEMBERED`] — which is where the reason for bounding it is.
    frames: VecDeque<Duration>,
    /// What the clock read when the previous frame was timed, or nothing before
    /// any frame has been.
    ///
    /// The port answers with time elapsed rather than with an interval, so the
    /// interval is the difference between two readings and something has to hold
    /// the earlier one. **One reading is not a frame time**, which is why the
    /// first one contributes nothing to the ring above: the interval it would
    /// have to be measured against is the moment the clock started, which is not
    /// when the previous frame was drawn.
    previous: Option<Duration>,
}

impl DebugOverlay {
    /// Whether the overlay is being shown.
    #[must_use]
    pub const fn visible(&self) -> bool {
        self.visible
    }

    /// Shows the overlay if it was hidden, and hides it if it was shown.
    ///
    /// **One call is one change of visibility**, which is why the toggle is a
    /// flip rather than a pair of show and hide calls: whoever binds a key to it
    /// has one key, and a key that could only show would be a key that stops
    /// doing anything the moment it has been pressed once.
    pub const fn toggle(&mut self) {
        self.visible = !self.visible;
    }

    /// Times the frame that has just been drawn, by asking `clock` how long it
    /// has been since the previous one.
    ///
    /// A subtraction that saturates rather than one that could go negative: the
    /// port's answer is monotonic, so a later reading below an earlier one is a
    /// broken clock rather than a frame that took less than no time, and a
    /// zero-length frame is the closest true thing to say about it.
    pub fn record_frame_time(&mut self, clock: &impl OverlayClock) {
        let reading = clock.now_elapsed();
        if let Some(previous) = self.previous.replace(reading) {
            self.remember(reading.saturating_sub(previous));
        }
    }

    /// Adds `took` to the ring, dropping the oldest frame time once it is full.
    fn remember(&mut self, took: Duration) {
        if self.frames.len() >= FRAMES_REMEMBERED {
            self.frames.pop_front();
        }
        self.frames.push_back(took);
    }

    /// What the overlay publishes for a player standing at `standing`, or for a
    /// client whose world has not landed when there is nobody standing anywhere.
    ///
    /// The column is derived from the position rather than carried beside it, so
    /// the two can never name different places.
    #[must_use]
    pub fn readout(&self, standing: Option<Vec3>) -> OverlayReadout {
        let frame_time_ms = self.mean_frame_time_ms();
        OverlayReadout {
            position: standing,
            column: standing.map(|at| column_containing(at.x, at.z)),
            frame_rate: frames_a_second(frame_time_ms),
            frame_time_ms,
        }
    }

    /// How long the remembered frames took on average, in milliseconds, and zero
    /// where none has been timed yet.
    fn mean_frame_time_ms(&self) -> f64 {
        if self.frames.is_empty() {
            return 0.0;
        }
        let total: f64 = self.frames.iter().map(Duration::as_secs_f64).sum();
        total * MILLIS_PER_SECOND / self.frames.len() as f64
    }
}

/// How many frames a second a mean frame time of `frame_time_ms` is, and zero
/// where nothing has been timed.
///
/// Zero rather than infinity for an untimed client: a frame rate is a
/// measurement, and a client that has drawn one frame has not measured one yet.
fn frames_a_second(frame_time_ms: f64) -> f64 {
    if frame_time_ms > 0.0 {
        MILLIS_PER_SECOND / frame_time_ms
    } else {
        0.0
    }
}

/// The four readings the overlay publishes for one frame.
///
/// A plain value carrying the readouts and nothing else, which is what keeps the
/// toolkit that paints it confined to one file: this is the whole of what crosses
/// that boundary.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OverlayReadout {
    /// Where the player is standing, or nothing while there is no player.
    pub position: Option<Vec3>,
    /// The column that position sits in, or nothing while there is no position.
    pub column: Option<ColumnCoordinate>,
    pub frame_rate: f64,
    pub frame_time_ms: f64,
}

/// The lines the overlay shows for `readout`, in the order it shows them.
///
/// The two world readings contribute a line each only when there is something to
/// show; the two frame readings always do. A placeholder line for an absent
/// position would be a second spelling of "there is no world yet" and one more
/// thing a reader could mistake for data.
#[must_use]
pub fn readout_lines(readout: &OverlayReadout) -> Vec<String> {
    let mut lines = Vec::new();
    if let Some(at) = readout.position {
        lines.push(format!(
            "position  x {x:.places$}  y {y:.places$}  z {z:.places$}",
            x = at.x,
            y = at.y,
            z = at.z,
            places = POSITION_PLACES
        ));
    }
    if let Some(column) = readout.column {
        lines.push(format!("column  {x}, {z}", x = column.x, z = column.z));
    }
    lines.push(format!(
        "frame rate  {rate:.places$} fps",
        rate = readout.frame_rate,
        places = FRAME_RATE_PLACES
    ));
    lines.push(format!(
        "frame time  {ms:.places$} ms",
        ms = readout.frame_time_ms,
        places = FRAME_TIME_PLACES
    ));
    lines
}

#[cfg(test)]
#[path = "state_test.rs"]
mod tests;
