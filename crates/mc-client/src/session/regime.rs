//! Whether the numbers a pointer reports are movements or positions, and what a
//! turn is when they are positions.
//!
//! # Why the client has to ask at all
//!
//! `winit` documents `DeviceEvent::MouseMotion`'s delta as a change in physical
//! position, and on Windows it is not always one. The raw-input path guards the
//! relative branch with `has_flag(mouse.usFlags, MOUSE_MOVE_RELATIVE)`, and
//! `MOUSE_MOVE_RELATIVE` is `0`: `usFlags & 0 == 0` holds for every packet, so
//! the guard admits all of them and `MOUSE_MOVE_ABSOLUTE` is consulted nowhere in
//! the crate. A physical mouse sets the relative bit and the defect is invisible.
//! Remote Desktop's virtual mouse — and tablets, touchscreens and some VM guest
//! drivers — set `MOUSE_MOVE_ABSOLUTE`, where `lLastX/lLastY` are screen
//! positions normalised to `0..65535` over the display. One such packet spent as
//! a device count is 49 radians of turn, or 7.8 revolutions, from a single event.
//!
//! # A decision, not a build flag
//!
//! The relative reading stays primary and the absolute one engages only when the
//! stream is *measured* to be absolute, twice over. Nothing here is configurable
//! and nothing is compiled out: a player who resumes a Remote Desktop session on
//! the local console, or reconnects a local session over Remote Desktop, changes
//! the shape of the stream under a client that is already playing, and neither
//! restarts the game.
//!
//! # Three properties, and each one is what a scenario buys
//!
//! - **No first-sample snap, in either direction.** A single position-shaped
//!   sample never turns the camera, so the very packet that produces today's
//!   49-radian spin produces nothing at all.
//! - **Two-sample corroboration both ways**, so one freak report cannot flip the
//!   reading — and the sample that *decides* a change is spent as the kind the
//!   new reading says it is, never differenced against a position it predates.
//! - **The position is forgotten while the cursor is the desktop's.** Otherwise
//!   the pointer's journey across the desktop, between an Escape and the click
//!   that comes back, arrives as one enormous turn.
//!
//! # The scale is a declared calibration and is wrong by a known amount
//!
//! The absolute range is declared to span a nominal 1920 × 1080. Measured against
//! the recorded session — 39.26 absolute units per pixel horizontally and 71.94
//! vertically, a session of about 1669 × 911 — this yields 1.150 counts per pixel
//! across and 1.185 down: the axes agree to 3.1%, and the whole thing runs 15-19%
//! faster than a one-count-per-pixel mouse *there*. A nominal is wrong by the
//! ratio of the real display to 1920 × 1080.
//!
//! Reading the real extent from `primary_monitor()` was weighed and declined, and
//! not on testability: the conversion is a pure function of the extent either
//! way. It was declined because nobody has measured what `primary_monitor()`
//! returns inside a Remote Desktop session — the remote session's extent or the
//! host machine's — and staking the scale on an unmeasured platform behaviour is
//! the exact mistake this defect already taught. A nominal's error is bounded,
//! deterministic and diagnosable from this comment; the exact route's error, if
//! that call answers with the host's display, is silent and machine-dependent.
//!
//! **A future change may not remove the two-sample corroboration in either
//! direction, and may not spend a deciding sample as the reading it is leaving.**

/// The width the absolute range is declared to span, in device counts.
const NOMINAL_WIDTH: f64 = 1920.0;

/// The height the absolute range is declared to span, in device counts.
const NOMINAL_HEIGHT: f64 = 1080.0;

/// How many steps the absolute range is divided into, per axis.
///
/// Per axis is the whole point: equal pointer travel is *not* equal in the two
/// axes' units, and differencing the raw stream into one sensitivity leaves the
/// vertical axis 1.83 times as fast as the horizontal on the measured session.
const ABSOLUTE_RANGE: f64 = 65_536.0;

/// The largest value the absolute range reports.
const ABSOLUTE_MAX: f64 = ABSOLUTE_RANGE - 1.0;

/// The smallest component that makes a sample look like a screen position.
///
/// The probe's own reading threshold, and *one* component clearing it is enough
/// rather than both: over the recording's 584 raw-motion samples the smallest
/// `|x|` is 21 093 while the smallest `|y|` is **35**, so requiring both would
/// have failed on the very recording the threshold is derived from.
const READING_THRESHOLD: f64 = 1000.0;

/// One raw pair, before anything has decided what it means.
#[derive(Debug, Clone, Copy)]
struct Sample {
    x: f64,
    y: f64,
}

impl Sample {
    /// Whether this is shaped like a screen position rather than a movement.
    ///
    /// A movement goes negative and stays small; a position never goes negative,
    /// never leaves the absolute range, and — for a pointer anywhere but the very
    /// corner of the display — has at least one large component.
    fn is_a_position(self) -> bool {
        let in_range =
            (0.0..=ABSOLUTE_MAX).contains(&self.x) && (0.0..=ABSOLUTE_MAX).contains(&self.y);
        in_range && (self.x >= READING_THRESHOLD || self.y >= READING_THRESHOLD)
    }
}

/// Which way the client is currently reading the stream.
#[derive(Debug, Clone, Copy)]
enum Reading {
    /// Every sample is a movement. `corroborating` is a lone position-shaped
    /// sample that has not yet been backed by a second.
    Relative { corroborating: Option<Sample> },
    /// Every sample is a position. `anchor` is what the next one is measured
    /// from — absent while there is nothing to measure from — and
    /// `corroborating` is a lone sample that did not look like a position.
    Absolute {
        anchor: Option<Sample>,
        corroborating: Option<Sample>,
    },
}

/// What a client makes of the pointer stream it is being handed.
#[derive(Debug)]
pub(super) struct PointerRegime {
    reading: Reading,
}

impl Default for PointerRegime {
    /// A session opens reading movements, which is what a physical mouse on a
    /// local console reports and what every client before this one assumed.
    fn default() -> Self {
        Self {
            reading: Reading::Relative {
                corroborating: None,
            },
        }
    }
}

impl PointerRegime {
    /// The device counts one raw sample asks the camera to turn by, if it asks
    /// for anything at all.
    ///
    /// `None` is a sample spent on deciding what the stream is rather than on a
    /// turn — the first position of a session, the first after the cursor came
    /// back, and a lone sample contradicting the current reading.
    pub(super) fn counts_of(&mut self, raw_x: f64, raw_y: f64) -> Option<(f64, f64)> {
        let sample = Sample { x: raw_x, y: raw_y };
        let (counts, next) = Self::step(self.reading, sample);
        self.reading = next;
        counts
    }

    /// Forgets the position the next turn would be measured from, keeping the
    /// reading itself.
    ///
    /// What a pointer the game does not hold leaves behind: nothing. The stream's
    /// *shape* does not change because the player alt-tabbed, but every position
    /// it reported while the cursor was the desktop's is somewhere the game was
    /// not told about.
    ///
    /// **Two different things free the pointer and only one of them has an
    /// event**, which is why the session calls this from both the capture path
    /// and the lost-window path. Escape hands the cursor to the desktop while the
    /// window keeps focus, so motion keeps arriving and the first sample admitted
    /// against a refused capture clears the position on its way past. A window
    /// that loses focus delivers no motion at all — `winit` registers raw input
    /// without `RIDEV_INPUTSINK`, so nothing is sent while the window is not
    /// foreground — and the capture state, which is what the other route is
    /// guarded on, does not change when focus does. Left to that route alone the
    /// position would survive the player's whole time away and be differenced
    /// against wherever the pointer turned up on their return: on this spec's own
    /// recording, an anchor at `x = 34475` meeting `x = 65477` is 908 counts, or
    /// **two radians of yaw from one event**.
    pub(super) const fn forget_position(&mut self) {
        self.reading = match self.reading {
            Reading::Relative { .. } => Reading::Relative {
                corroborating: None,
            },
            Reading::Absolute { .. } => Reading::Absolute {
                anchor: None,
                corroborating: None,
            },
        };
    }

    /// One transition, dispatched on which way the stream is currently being
    /// read. The two arms below spell out every row of the table between them.
    fn step(reading: Reading, sample: Sample) -> (Option<(f64, f64)>, Reading) {
        match reading {
            Reading::Relative { corroborating } => Self::reading_movements(corroborating, sample),
            Reading::Absolute {
                anchor,
                corroborating,
            } => Self::reading_positions(anchor, corroborating, sample),
        }
    }

    /// What a sample means while the stream is being read as movements.
    ///
    /// The first position-shaped sample buys nothing but a memory of itself; the
    /// second is what settles the question, and the travel between the two is
    /// what it is worth. Anything that is not a position is the movement it
    /// appears to be, and it discards whatever was waiting for corroboration.
    fn reading_movements(
        corroborating: Option<Sample>,
        sample: Sample,
    ) -> (Option<(f64, f64)>, Reading) {
        match (corroborating, sample.is_a_position()) {
            (None, true) => (None, waiting_on(sample)),
            (Some(first), true) => (Some(travel(first, sample)), anchored_at(sample)),
            (_, false) => (Some((sample.x, sample.y)), MOVEMENTS),
        }
    }

    /// What a sample means while the stream is being read as positions.
    ///
    /// A position is worth the travel since the last one, and worth nothing at
    /// all when there is no last one — the opening of a session, and the moment
    /// after the cursor came back from the desktop. A sample that is not a
    /// position is one report against the reading and is held; a second in a row
    /// hands the stream back, and *that* second one is spent as the movement the
    /// new reading says it is rather than differenced against a stale position.
    fn reading_positions(
        anchor: Option<Sample>,
        corroborating: Option<Sample>,
        sample: Sample,
    ) -> (Option<(f64, f64)>, Reading) {
        match (corroborating, sample.is_a_position()) {
            (_, true) => (anchor.map(|from| travel(from, sample)), anchored_at(sample)),
            (None, false) => (
                None,
                Reading::Absolute {
                    anchor,
                    corroborating: Some(sample),
                },
            ),
            (Some(_), false) => (Some((sample.x, sample.y)), MOVEMENTS),
        }
    }
}

/// Reading movements, with nothing held over.
const MOVEMENTS: Reading = Reading::Relative {
    corroborating: None,
};

/// Reading movements, holding one position-shaped sample that a second would
/// corroborate.
const fn waiting_on(sample: Sample) -> Reading {
    Reading::Relative {
        corroborating: Some(sample),
    }
}

/// Reading positions, measuring the next turn from `sample`.
const fn anchored_at(sample: Sample) -> Reading {
    Reading::Absolute {
        anchor: Some(sample),
        corroborating: None,
    }
}

/// The device counts between two screen positions, converted per axis.
fn travel(from: Sample, to: Sample) -> (f64, f64) {
    (
        (to.x - from.x) * NOMINAL_WIDTH / ABSOLUTE_RANGE,
        (to.y - from.y) * NOMINAL_HEIGHT / ABSOLUTE_RANGE,
    )
}
