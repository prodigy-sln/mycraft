//! A walk covers the same ground per second of elapsed time whatever rate the
//! frames arrive at.
//!
//! This is the defect a player found and reported as *"I just warp around with
//! super speed"*: the client advanced exactly one simulation tick per rendered
//! frame, and a tick is a declared sixtieth of a second, so the world ran at
//! `frames_per_second / 60` times real speed — right by coincidence on a 60 Hz
//! display and 2.4× too fast on a 144 Hz one. Every scenario below drives frames
//! at a *simulated* rate, with no wall clock, no sleeping and no scheduler.
//!
//! # The three rates deliver one total, exactly, and that is what makes equality
//! the right assertion
//!
//! [`AT_144`], [`AT_1000`] and [`AT_60`] each hand over [`THE_STRETCH`]
//! nanoseconds — the same integer, reached by three different partitions. The
//! pacing carries unspent time as a `Duration`, which is whole nanoseconds, so
//! nothing is lost at a partition boundary: the ticks a stretch buys are
//! `floor(total / quantum)` however it was cut up. Equal totals therefore give
//! *equal tick counts*, and equal tick counts give a bit-identical position.
//!
//! **That is a stronger reason than the spec's own, and worth stating because it
//! replaces a constraint.** The spec derived the same assertion from a residue of
//! about a microsecond over a one-second drive and asked the fixture to choose a
//! total clear of a quantum boundary. With integer nanoseconds the residue is not
//! small — it is *zero* — and the boundary cannot be straddled by two runs that
//! deliver the same nanosecond count. The margin is recorded anyway, because a
//! reader should be able to check the claim rather than take it: `THE_STRETCH`
//! sits 3 990 ns above the 30-quantum boundary, which is 0.02 % of a quantum. An
//! implementation accumulating `f64` seconds instead would err by around
//! 10⁻⁷ ns at this magnitude — ten orders of magnitude inside that margin — so the
//! fixture is not tuned to the implementation it happens to have.
//!
//! # Why the drive is half a second and not longer
//!
//! The declared floor is one chunk column, 16 blocks across, and the player
//! spawns centred at x 8.5 facing +x. Half a second of walking is 2.25 blocks,
//! leaving 5.25 to the edge — and the *broken* client, which spends 2.4× as many
//! ticks over the same stretch, walks 5.4 and still lands inside the world. That
//! matters: a red produced by two runs both piling into the same edge would be
//! evidence of nothing, and a red that reads as 2.4× is evidence of this defect.
//!
//! # Nothing here is asserted against a written-down coordinate
//!
//! Gravity acts on every tick and a walk covers a fixed fraction of a block on
//! each, so a position spelled in this file would be a number copied out of a run
//! of the code it judges. Every assertion is a comparison between two runs of the
//! same harness, so the oracle is independent of the walk speed, the gravity and
//! the tick duration — none of which this fix touches.
//!
//! # Every equality carries a control
//!
//! Two runs that both moved nobody are equal, and so are two runs that both
//! walked into the same wall. So each scenario that asserts sameness also
//! requires that the walk was still progressing: half the stretch has to leave
//! the player somewhere else. A client that stopped advancing altogether fails
//! that control rather than passing the assertion.
//!
//! # The harness is included by path
//!
//! Not through `tests/support/mod.rs`, which links `support/frames.rs` and with it
//! the whole graphics stack — into a binary whose entire premise is that no
//! adapter is acquired (conductor ruling 56).

#[path = "support/input/mod.rs"]
mod input;

use std::error::Error;
use std::time::Duration;

use glam::Vec3;
use mc_sim::player::TICK_QUANTUM;
use winit::keyboard::KeyCode;

use input::InputHarness;

type TestResult = Result<(), Box<dyn Error>>;

/// The key the declared table binds to walking forward.
const FORWARD: KeyCode = KeyCode::KeyW;

/// The key the declared table binds to showing the debug overlay.
const DECLARED_TOGGLE: KeyCode = KeyCode::F3;

/// The elapsed time every rate below delivers, in nanoseconds.
///
/// Half a second, near enough — 500 004 000 ns. The exact figure is what makes
/// all four partitions land on it as whole frames: 500 at a simulated 1000 a
/// second, 72 at 144, 30 at 60 and 5 a tenth of a second apart. See the header
/// for why it is half a second and not longer, and for where it sits relative to
/// a quantum boundary.
const THE_STRETCH: Duration = Duration::from_nanos(500_004_000);

/// How many ticks [`THE_STRETCH`] buys: `floor(500_004_000 / 16_666_667)`.
///
/// Written down because it is the arithmetic a reader should be able to redo, and
/// asserted nowhere — every scenario below compares two runs rather than a run
/// against this number.
const TICKS_THE_STRETCH_BUYS: u32 = 30;

/// A rate a stretch of elapsed time is delivered at.
///
/// Frames and an interval rather than a frames-per-second figure, because what a
/// client is handed is intervals: a rate is what those *mean*, and naming the
/// rate would put a division between the fixture and what it drives.
#[derive(Debug, Clone, Copy)]
struct Rate {
    frames: u32,
    each: Duration,
}

/// A simulated 144 frames a second: 6.9445 ms a frame, 72 of them.
///
/// The interval is the nearest nanosecond to a 144th of a second, which is
/// 6 944 444.4 ns, and 72 of it is [`THE_STRETCH`] exactly. The rate it spells is
/// 143.9989 a second — a hundredth of a percent off nominal, and the price of the
/// three rates summing to one integer.
const AT_144: Rate = Rate {
    frames: 72,
    each: Duration::from_nanos(6_944_500),
};

/// A simulated 60 frames a second, which is the rate the broken client happened
/// to be correct at.
///
/// It is the control every other rate is read against for exactly that reason: a
/// fix that made the world run at some *other* wrong speed uniformly would satisfy
/// "the same at 144 and at 1000" and fail here.
const AT_60: Rate = Rate {
    frames: 30,
    each: Duration::from_nanos(16_666_800),
};

/// A simulated 1000 frames a second: far above any display, and where the
/// per-frame interval is a fraction of a quantum rather than a small multiple.
const AT_1000: Rate = Rate {
    frames: 500,
    each: Duration::from_nanos(1_000_008),
};

/// Half of [`AT_60`]'s frames at the same interval: half the stretch, which is
/// the control every equality in this file is read against.
///
/// The count is written rather than divided. Half of thirty is a fact about this
/// fixture and not an arithmetic step, and a division here would silently round
/// the day the count became odd — half of thirty-one is fifteen, which is not
/// half of anything.
const HALF_OF_AT_60: Rate = Rate {
    frames: 15,
    each: AT_60.each,
};

/// Half a tick quantum, rounded up.
///
/// The quantum is an odd number of nanoseconds, so it has no exact half. Rounded
/// *up*, because two of the rounded-down half come to one nanosecond short of a
/// quantum and correctly buy no tick at all — which is the scenario above this
/// one, not this one.
const HALF_QUANTUM: Duration = Duration::from_nanos(8_333_334);

/// A run of frame times that is not uniform, for the determinism scenario.
///
/// Deliberately jagged and deliberately straddling the quantum: two frames worth
/// no tick at all, two whose sums cross one, and one worth three. A sequence of
/// equal intervals would leave the carried remainder at zero throughout, which is
/// the one state in which carrying nothing and carrying correctly look alike.
///
/// **The last frame has to buy a tick, and the first draft's did not.** The
/// control drops it, so a final frame that bought nothing would leave the shorter
/// run ending exactly where the whole one did — which is what the control caught,
/// against a correct implementation. It buys one now: the run spends 0, 1, 3, 0,
/// 1 and 1, and the control spends five of those six.
const A_JAGGED_RUN: [Duration; 6] = [
    Duration::from_nanos(4_000_000),
    Duration::from_nanos(13_500_000),
    Duration::from_nanos(51_000_000),
    Duration::from_nanos(1),
    Duration::from_nanos(16_666_667),
    Duration::from_nanos(24_000_000),
];

/// How close two frame rates have to be to count as the same reading.
///
/// The overlay reports a mean over the frames it remembers and the drive below
/// hands it 72 identical intervals, so the mean is that interval exactly and the
/// arithmetic error is float noise — parts in 10¹⁵. The smallest difference the
/// assertion has to catch is the one this scenario exists for: a rate taken from a
/// second reading of the clock, which would differ by whole frames. A tenth of a
/// frame a second sits far above the one and far below the other.
const WITHIN_FPS: f64 = 0.1;

/// What [`AT_144`] means as frames a second, derived from its own interval rather
/// than written down, so the two cannot drift.
fn nominal_rate(rate: Rate) -> f64 {
    1.0 / rate.each.as_secs_f64()
}

#[test]
fn the_same_elapsed_time_walks_the_same_distance_at_144_frames_a_second_and_at_60() -> TestResult {
    let at_144 = walked_at(AT_144)?;
    let at_60 = walked_at(AT_60)?;
    let control = walked_at(HALF_OF_AT_60)?;

    assert_ne!(
        at_60, control,
        "the control this scenario is read under: half the stretch has to leave the player \
         somewhere else, or the equality below is two runs that both stopped — or both walked \
         into the same edge of the fixture's one column"
    );
    assert_eq!(
        at_144, at_60,
        "{THE_STRETCH:?} of elapsed time is {TICKS_THE_STRETCH_BUYS} ticks of walking, whether it \
         arrived as {} frames or as {}. A client that advanced one tick per frame walked 2.4× as \
         far on the faster display — which is the player report this fix comes from, in numbers",
        AT_144.frames, AT_60.frames
    );
    Ok(())
}

#[test]
fn the_same_elapsed_time_walks_the_same_distance_at_1000_frames_a_second_and_at_60() -> TestResult {
    let at_1000 = walked_at(AT_1000)?;
    let at_60 = walked_at(AT_60)?;
    let control = walked_at(HALF_OF_AT_60)?;

    assert_ne!(
        at_60, control,
        "the control this scenario is read under: half the stretch has to leave the player \
         somewhere else, or the equality below is two runs that both stopped"
    );
    assert_eq!(
        at_1000, at_60,
        "a frame far shorter than a tick quantum buys no tick of its own and is not thrown away \
         either — the {} frames of this run each add a sixteenth of a quantum to what is carried, \
         and the ticks come out of the sum. A client that spent a tick per frame ran this one \
         16.7× fast; one that discarded anything below a quantum would leave the player standing \
         still at any rate above 60",
        AT_1000.frames
    );
    Ok(())
}

#[test]
fn a_frame_delivering_three_tick_quanta_advances_the_player_by_three_ticks_walk() -> TestResult {
    let one_frame_of_three = walked_over(&[3 * TICK_QUANTUM])?;
    let three_frames_of_one = walked_over(&[TICK_QUANTUM, TICK_QUANTUM, TICK_QUANTUM])?;
    let one_frame_of_one = walked_over(&[TICK_QUANTUM])?;

    assert_ne!(
        three_frames_of_one, one_frame_of_one,
        "the control this scenario is read under: three ticks of walking has to be a different \
         place from one, or the equality below holds for a client that walks nobody anywhere"
    );
    assert_eq!(
        one_frame_of_three, three_frames_of_one,
        "a frame is not a tick. One frame carrying three quanta of elapsed time owes the world \
         three ticks, and it pays them before it returns — a client that paid one and dropped the \
         rest runs slow on exactly the machines that need the catch-up most"
    );
    Ok(())
}

#[test]
fn a_frame_delivering_less_than_one_tick_quantum_leaves_the_player_where_it_was() -> TestResult {
    let before = walked_over(&[])?;
    let after_a_short_frame = walked_over(&[TICK_QUANTUM - Duration::from_nanos(1)])?;
    let after_a_whole_one = walked_over(&[TICK_QUANTUM])?;

    assert_ne!(
        after_a_whole_one, before,
        "the control this scenario is read under: a whole quantum has to move the player, or \
         'a short frame moves nobody' is a statement about a client that moves nobody at all"
    );
    assert_eq!(
        after_a_short_frame, before,
        "a tick is indivisible: a frame one nanosecond short of a quantum has not bought one, and \
         the player stands exactly where they stood. The nanosecond is not lost — it is carried, \
         which is what the next scenario is about"
    );
    Ok(())
}

#[test]
fn two_frames_of_half_a_quantum_advance_the_player_by_one_ticks_walk() -> TestResult {
    let two_halves = walked_over(&[HALF_QUANTUM, HALF_QUANTUM])?;
    let one_whole = walked_over(&[TICK_QUANTUM])?;
    let one_half = walked_over(&[HALF_QUANTUM])?;

    assert_ne!(
        one_half, one_whole,
        "the control this scenario is read under: half a quantum has to buy no tick on its own, \
         or the two frames below could each have bought one and the total would still agree"
    );
    assert_eq!(
        two_halves, one_whole,
        "what a frame does not spend it carries. Two frames of half a quantum owe the world one \
         tick between them, and a client that dropped each remainder would run permanently slow on \
         any machine whose frame time is not a whole multiple of a sixtieth of a second — which is \
         every machine"
    );
    Ok(())
}

#[test]
fn the_rate_the_overlay_reports_and_the_ticks_the_world_spent_come_from_one_reading() -> TestResult
{
    let mut harness = InputHarness::started();
    harness.start_world()?;
    harness.press(DECLARED_TOGGLE);
    let published = harness.frames(AT_144.frames, AT_144.each);

    let ticks = published
        .last()
        .ok_or("a run over a started world publishes a snapshot per frame")?
        .tick;
    let shown = harness
        .overlay_frame_rate()
        .ok_or("a client showing its overlay publishes a reading for whoever paints it")?;

    assert_eq!(
        (ticks, near_enough(shown, nominal_rate(AT_144))),
        (TICKS_THE_STRETCH_BUYS, true),
        "the frame rate on screen and the time the world spent are two views of one measurement, \
         and this is where that stops being a claim: {} frames of {:?} is {THE_STRETCH:?}, which \
         the overlay reports as {:.1} frames a second and the simulation spends as \
         {TICKS_THE_STRETCH_BUYS} ticks. A client that read its clock twice — once to time the \
         frame and once to pace it — could show 144 while the world ran at some other speed, and \
         no assertion about either reading alone would say so. It reported {shown}",
        AT_144.frames,
        AT_144.each,
        nominal_rate(AT_144)
    );
    Ok(())
}

#[test]
fn the_same_sequence_of_frame_times_leaves_the_player_in_the_same_place_twice() -> TestResult {
    let once = walked_over(&A_JAGGED_RUN)?;
    let again = walked_over(&A_JAGGED_RUN)?;
    let a_frame_shorter = A_JAGGED_RUN
        .split_last()
        .ok_or("the jagged run has to have a frame to drop")?
        .1;
    let a_different_run = walked_over(a_frame_shorter)?;

    assert_ne!(
        once, a_different_run,
        "the control this scenario is read under: a sequence one frame shorter has to end \
         somewhere else, or 'the same sequence ends in the same place' is true of a client that \
         ends everywhere in the same place"
    );
    assert_eq!(
        once, again,
        "the pacing carries state between frames, and state is a new way to break a replay. The \
         same frame times from the same start leave the player at the same coordinates — not \
         nearly, exactly — because what is carried is whole nanoseconds and nothing about the \
         spending consults anything outside them"
    );
    Ok(())
}

/// Whether the rate the overlay reported is [`WITHIN_FPS`] of `rate`.
fn near_enough(shown: f64, rate: f64) -> bool {
    (shown - rate).abs() < WITHIN_FPS
}

/// Where walking forward through `rate`'s frames leaves the player.
fn walked_at(rate: Rate) -> Result<Vec3, Box<dyn Error>> {
    let mut harness = walking()?;
    let published = harness.frames(rate.frames, rate.each);
    Ok(published
        .last()
        .ok_or("a run over a started world publishes a snapshot per frame")?
        .player
        .position)
}

/// Where walking forward through frames that took exactly `took` leaves the
/// player.
///
/// A slice rather than a count and an interval, because the scenarios that use it
/// are about frames of *different* lengths, and the one that uses none of them is
/// the state a client is in before it has drawn anything.
fn walked_over(took: &[Duration]) -> Result<Vec3, Box<dyn Error>> {
    let mut harness = walking()?;
    for one in took {
        harness.frame(*one);
    }
    Ok(harness
        .published()
        .ok_or("a client over a started world publishes where its player is standing")?
        .player
        .position)
}

/// A client over the declared floor, holding the walk key and not yet given a
/// frame.
///
/// The key is pressed before any frame so that every frame of every run below
/// carries the same input: what separates the runs is the elapsed time they are
/// handed and nothing else.
fn walking() -> Result<InputHarness, Box<dyn Error>> {
    let mut harness = InputHarness::started();
    harness.start_world()?;
    harness.press(FORWARD);
    Ok(harness)
}
