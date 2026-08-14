//! What the debug overlay shows: the player's place in the world, the column
//! holding them, and how fast frames are arriving.
//!
//! No device, no window and no toolkit anywhere in this file. Everything below is
//! a value the overlay publishes and the lines it spells from one, which is the
//! whole reason those two are plain functions over plain data.
//!
//! # Every expected number here is derived, and the derivations are these
//!
//! A section is [`SECTION_SIZE`](mc_world::section::SECTION_SIZE) voxels across,
//! so the column holding a coordinate is that coordinate divided by the section
//! size and **floored** — floored, not truncated, which is a distinction with no
//! visible difference at a positive coordinate and the whole of the answer at a
//! negative one:
//!
//! - `32.0 / 16 = 2.0`, floored **2**. A truncation agrees, which is why this
//!   fixture alone cannot grade the rule.
//! - `-0.5 / 16 = -0.03125`, floored **-1**. A truncation reports **0** — the
//!   column on the other side of the origin, one whole column away from where
//!   the player is standing.
//!
//! A frame time is the interval between two readings of the overlay's clock, and
//! a frame rate is how many such intervals fit in a second:
//!
//! - ten frames [`ONE_FIFTIETH`] apart read a mean interval of **20 ms**, and
//!   `1000 / 20` = **50** frames a second.
//! - three frames [`ONE_TWENTY_FIFTH`] apart read **40 ms** and `1000 / 40` =
//!   **25** frames a second.
//!
//! Both expectations are computed from the driven interval in this file rather
//! than written down, so a run of the code under test cannot be where they came
//! from. The mean is the interval whatever the ring's length is and whether or
//! not the first reading contributes an interval of its own, so neither number
//! moves with a decision this suite is not about.
//!
//! # The clock is driven, not waited on
//!
//! [`DrivenClock`] is a [`Duration`] this file owns, advanced by hand. It names
//! no system clock at all — a fake that had to read one would be measuring the
//! machine this suite runs on rather than the arithmetic under test, and would be
//! the flaky test that discipline exists to prevent.
//!
//! # One limitation of the position fixture, stated because a green run cannot
//!
//! The scenario the coordinate ordering comes from stands the player at
//! `(32.0, 41.62, 32.0)`, whose **x and z are equal**. So an implementation that
//! transposed those two axes spells the identical line, and the assertion below
//! cannot see it. What it does see is the vertical sitting between them, which is
//! the plausible failure — a readout showing the height first, as a great many of
//! them do. Recorded rather than closed: the fixture is the scenario's, and
//! widening it would be inventing a scenario inside a phase.

use std::cell::Cell;
use std::time::Duration;

use glam::Vec3;

use crate::overlay::clock::OverlayClock;

use super::{DebugOverlay, readout_lines};

/// A clock the caller moves.
///
/// Interior mutability because the port answers through `&self` — a clock is
/// something you read, not something reading changes — while a test has to be
/// able to move it between two readings.
#[derive(Debug, Default)]
struct DrivenClock {
    elapsed: Cell<Duration>,
}

impl DrivenClock {
    /// Moves this clock forward by `by`.
    fn advance(&self, by: Duration) {
        self.elapsed.set(self.elapsed.get() + by);
    }
}

impl OverlayClock for DrivenClock {
    fn now_elapsed(&self) -> Duration {
        self.elapsed.get()
    }
}

/// How many milliseconds a second holds, named rather than spelled so the two
/// derivations below read as the arithmetic they are.
const MILLIS_PER_SECOND: f64 = 1000.0;

/// A fiftieth and a twenty-fifth of a second, as the interval between two frames.
const ONE_FIFTIETH: Duration = Duration::from_millis(20);
const ONE_TWENTY_FIFTH: Duration = Duration::from_millis(40);

/// How many frames each of the two timing scenarios draws.
const TEN_FRAMES: u32 = 10;
const THREE_FRAMES: u32 = 3;

/// How far a reading may sit from the interval that produced it.
///
/// The scenario's own bar. It is far above the arithmetic's error — a mean of
/// exact 20 ms intervals is exact — and far below the smallest difference the
/// assertion has to catch, which is a whole frame's worth of misattributed time.
const WITHIN: f64 = 1.0;

/// Where the player stands in the scenarios that place one.
const STANDING: Vec3 = Vec3::new(32.0, 41.62, 32.0);
const BEFORE_THE_ORIGIN: Vec3 = Vec3::new(-0.5, 41.62, -0.5);

/// The line a player standing at [`STANDING`] is shown as.
///
/// Written out rather than assembled with the formatting under test: an
/// expectation built by the code it grades is the subject agreeing with itself.
const STANDING_SHOWN_AS: &str = "position  x 32.000  y 41.620  z 32.000";

/// The columns the two positions above sit in, spelled as the overlay spells
/// them. Derived in this file's header, and both are one line of arithmetic a
/// reviewer can redo.
const COLUMN_OF_STANDING: &str = "column  2, 2";
const COLUMN_BEFORE_THE_ORIGIN: &str = "column  -1, -1";

/// What each of the four lines opens with.
///
/// The labels are the contract this suite binds, and they are what lets a claim
/// about one reading be made without reading the others: "no position is shown"
/// is a statement about the line that would have shown one.
const POSITION_LABEL: &str = "position  ";
const COLUMN_LABEL: &str = "column  ";
const FRAME_RATE_LABEL: &str = "frame rate  ";
const FRAME_TIME_LABEL: &str = "frame time  ";

/// An overlay that has drawn `frames` frames, each `apart` after the one before.
///
/// The clock starts at zero and is advanced before every reading, so what the
/// overlay is handed is exactly `frames` readings spaced by `apart` and nothing
/// else — no wall clock, no scheduler, and no sleeping.
fn drawing(frames: u32, apart: Duration) -> DebugOverlay {
    let clock = DrivenClock::default();
    let mut overlay = DebugOverlay::default();
    for _ in 0..frames {
        clock.advance(apart);
        overlay.record_frame_time(&clock);
    }
    overlay
}

/// The one line of `lines` opening with `label`, or nothing when none does.
fn line_opening_with<'a>(lines: &'a [String], label: &str) -> Option<&'a str> {
    lines
        .iter()
        .map(String::as_str)
        .find(|line| line.starts_with(label))
}

#[test]
fn the_readout_states_the_players_three_coordinates_in_x_y_z_order() {
    let shown = readout_lines(&DebugOverlay::default().readout(Some(STANDING)));

    assert_eq!(
        line_opening_with(&shown, POSITION_LABEL),
        Some(STANDING_SHOWN_AS),
        "a debug overlay's first job is to say where the player is, and the order of the three \
         numbers is what makes it readable at a glance: somebody comparing a position against a \
         chunk boundary reads x, then the height, then z, in the order every coordinate in this \
         engine is written. A readout that showed the height first would be read wrong every time \
         rather than obviously wrong once. It showed: {shown:?}"
    );
}

#[test]
fn the_readout_names_the_column_holding_a_player_standing_two_sections_out() {
    let shown = readout_lines(&DebugOverlay::default().readout(Some(STANDING)));

    assert_eq!(
        line_opening_with(&shown, COLUMN_LABEL),
        Some(COLUMN_OF_STANDING),
        "the column is the coordinate somebody diagnosing chunk loading, meshing or persistence \
         actually works in, and it is derived from the position rather than carried beside it so \
         the two can never name different places. A player standing at {STANDING:?} is inside the \
         column this file's header derives, and an overlay that showed the raw position twice \
         would be showing nothing new. It showed: {shown:?}"
    );
}

#[test]
fn the_readout_names_the_column_below_zero_for_a_player_a_half_block_before_the_origin() {
    let shown = readout_lines(&DebugOverlay::default().readout(Some(BEFORE_THE_ORIGIN)));

    assert_eq!(
        line_opening_with(&shown, COLUMN_LABEL),
        Some(COLUMN_BEFORE_THE_ORIGIN),
        "half of any world sits at a negative coordinate, and the column holding one is the \
         division **floored** rather than truncated: a truncation answers 0 for both axes here, \
         which is the column on the far side of the origin and a whole column away from where the \
         player is standing. It is also the failure nothing else in this suite can see — at a \
         positive coordinate the two agree exactly. It showed: {shown:?}"
    );
}

#[test]
fn ten_frames_a_fiftieth_of_a_second_apart_read_twenty_milliseconds_and_fifty_frames_a_second() {
    let due_ms = ONE_FIFTIETH.as_secs_f64() * MILLIS_PER_SECOND;
    let due_rate = MILLIS_PER_SECOND / due_ms;
    let read = drawing(TEN_FRAMES, ONE_FIFTIETH).readout(None);
    let off_by = |reading: f64, due: f64| (reading - due).abs();

    assert!(
        off_by(read.frame_time_ms, due_ms) <= WITHIN && off_by(read.frame_rate, due_rate) <= WITHIN,
        "the frame time is the interval between two readings of the overlay's own clock and the \
         frame rate is how many of those fit in a second, so {TEN_FRAMES} frames {ONE_FIFTIETH:?} \
         apart read {due_ms} ms and {due_rate} frames a second — both computed from the driven \
         interval, never read back from a run. An overlay that timed nothing reads zero, and one \
         that timed the wrong pair of readings reads a plausible number nothing can check. It read \
         {read_ms} ms and {read_rate} frames a second",
        read_ms = read.frame_time_ms,
        read_rate = read.frame_rate
    );
}

#[test]
fn a_readout_taken_before_the_world_lands_still_states_the_frame_rate_and_the_frame_time() {
    let due_ms = ONE_TWENTY_FIFTH.as_secs_f64() * MILLIS_PER_SECOND;
    let due_rate = MILLIS_PER_SECOND / due_ms;
    let due_rate_line = format!("{FRAME_RATE_LABEL}{due_rate:.1} fps");
    let due_time_line = format!("{FRAME_TIME_LABEL}{due_ms:.2} ms");
    let shown = readout_lines(&drawing(THREE_FRAMES, ONE_TWENTY_FIFTH).readout(None));

    assert_eq!(
        (
            line_opening_with(&shown, FRAME_RATE_LABEL),
            line_opening_with(&shown, FRAME_TIME_LABEL)
        ),
        (Some(due_rate_line.as_str()), Some(due_time_line.as_str())),
        "the two frame readings come from the overlay's own clock rather than from the simulation, \
         so they are exactly as available before the world lands as after — which is the point of \
         an instrument for diagnosing a client that is taking too long to start. An overlay that \
         waited for a player before showing anything would be blank for precisely the frames \
         somebody is watching it for. It showed: {shown:?}"
    );
}

#[test]
fn a_readout_taken_before_the_world_lands_names_neither_a_position_nor_a_column() {
    let overlay = drawing(THREE_FRAMES, ONE_TWENTY_FIFTH);
    let landed = readout_lines(&overlay.readout(Some(STANDING)));
    let waiting = readout_lines(&overlay.readout(None));

    assert!(
        line_opening_with(&landed, POSITION_LABEL).is_some()
            && line_opening_with(&landed, COLUMN_LABEL).is_some(),
        "the control this scenario needs, and the same overlay is asked both times so a reading \
         that remembered the last player it saw is caught here: once there is a player, this \
         reading has to name both a position and a column — or the absence below is a readout that \
         never names either and the assertion is about nothing. It showed: {landed:?}"
    );
    assert_eq!(
        (
            line_opening_with(&waiting, POSITION_LABEL),
            line_opening_with(&waiting, COLUMN_LABEL)
        ),
        (None, None),
        "before the world lands there is no player, so there is no position and no column — and \
         the overlay says so by showing neither rather than by showing a zero or the last one it \
         saw. The origin is a place a player can genuinely stand, so a zeroed readout is one \
         nothing downstream can tell from a real one, and a stale one is worse: it reads as a \
         player who has stopped moving. It showed: {waiting:?}"
    );
}
