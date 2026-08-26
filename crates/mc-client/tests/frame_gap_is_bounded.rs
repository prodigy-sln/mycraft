//! A frame that reports a pathological gap advances a bounded number of ticks,
//! and the bound sits above every interval a working machine produces.
//!
//! # The bound has two sides and both are scenarios here
//!
//! A debugger pause, a breakpoint or a laptop resuming from sleep hands the frame
//! path unbounded elapsed time. Spending all of it would replay it — a hang whose
//! length is however long the machine was away — so a frame's elapsed time is
//! clamped, and the surplus is *discarded* rather than carried: a single-player
//! client losing that time is the right answer, because nobody was playing.
//!
//! **The other side is what makes the first falsifiable.** A bound of one tick
//! satisfies "a ten-second frame advances at most the bound" and "the surplus does
//! not leak into the frames after it" perfectly well, and would make the world
//! crawl on any machine below 60 frames a second — this defect with the sign
//! flipped, and harder to notice, because a slow game reads as a slow machine. The
//! third scenario is what forbids it: at ten frames a second, the slowest rate at
//! which a game is arguably being *played* rather than hung, a walk covers exactly
//! the ground it covers at sixty.
//!
//! # Nothing here spells the bound
//!
//! Neither the 250 ms nor the fifteen ticks appears below. What a bounded frame
//! advances is asserted against a run of whole-quantum frames — [`AT_THE_BOUND`],
//! which is a count this file states and the implementation states separately —
//! and if the two disagree the assertion says which number each side used. A test
//! that reached for the constant would agree with the implementation by
//! construction and would go green the day the bound became one tick.
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

/// A frame gap no working machine produces: ten seconds, which is a debugger, a
/// breakpoint or a lid that was closed.
const A_PATHOLOGICAL_GAP: Duration = Duration::from_secs(10);

/// How many ticks a frame may ever buy, as this file states it independently of
/// the implementation.
///
/// Fifteen quanta is 250 ms. The derivation is the floor rather than the ceiling:
/// ten frames a second is the slowest rate at which the game is arguably being
/// played, that is a 100 ms interval, and the bound has to sit above it or a slow
/// machine loses simulated time systematically. 2.5× headroom over that floor.
const AT_THE_BOUND: u32 = 15;

/// The elapsed time every rate below delivers.
///
/// Half a second, near enough — the exact figure is what makes both partitions
/// land on it as whole frames: 30 at a simulated 60 a second, 5 a tenth of a
/// second apart.
const THE_STRETCH: Duration = Duration::from_nanos(500_004_000);

/// A simulated 60 frames a second over [`THE_STRETCH`].
const AT_60: Rate = Rate {
    frames: 30,
    each: Duration::from_nanos(16_666_800),
};

/// Frames a tenth of a second apart over the same stretch: a machine managing ten
/// frames a second, which is slow and is not stalled.
///
/// The interval is 100.0008 ms rather than a round tenth for the reason
/// [`THE_STRETCH`] is not a round half second: five of it has to be the same
/// integer nanosecond count as thirty of [`AT_60`]'s, or the two runs would be
/// comparing different stretches of time and the equality below would be an
/// accident either way.
const AT_10: Rate = Rate {
    frames: 5,
    each: Duration::from_nanos(100_000_800),
};

/// Half of [`AT_60`]'s frames at the same interval: half the stretch, which is
/// the control the equality below is read against.
///
/// The count is written rather than divided. Half of thirty is a fact about this
/// fixture and not an arithmetic step, and a division here would silently round
/// the day the count became odd — half of thirty-one is fifteen, which is not
/// half of anything.
const HALF_OF_AT_60: Rate = Rate {
    frames: 15,
    each: AT_60.each,
};

/// A rate a stretch of elapsed time is delivered at.
#[derive(Debug, Clone, Copy)]
struct Rate {
    frames: u32,
    each: Duration,
}

#[test]
fn a_frame_reporting_ten_seconds_advances_no_further_than_the_catch_up_bound() -> TestResult {
    let after_the_gap = walked_over(&[A_PATHOLOGICAL_GAP])?;
    let at_the_bound = walked_over(&whole_quanta(AT_THE_BOUND))?;
    let one_tick_further = walked_over(&whole_quanta(AT_THE_BOUND + 1))?;

    assert_ne!(
        at_the_bound, one_tick_further,
        "the control this scenario is read under: one more tick has to leave the player somewhere \
         else, or 'no further than {AT_THE_BOUND} ticks' is satisfied by any number of them"
    );
    assert_eq!(
        after_the_gap, at_the_bound,
        "{A_PATHOLOGICAL_GAP:?} of elapsed time is a debugger, a breakpoint or a lid that was \
         closed — never a frame. Spending all of it would replay it: 600 ticks of walking in one \
         frame, and a client that hung for as long as the machine had been away. It is clamped to \
         {AT_THE_BOUND} ticks, which is 250 ms, which is 2.5× the interval of the slowest machine \
         anybody is playing on"
    );
    Ok(())
}

#[test]
fn the_surplus_a_bounded_frame_discarded_does_not_reach_the_frames_after_it() -> TestResult {
    let mut after_the_gap = vec![A_PATHOLOGICAL_GAP];
    after_the_gap.push(TICK_QUANTUM);
    let resumed = walked_over(&after_the_gap)?;
    let never_stalled = walked_over(&whole_quanta(AT_THE_BOUND + 1))?;
    let still_at_the_bound = walked_over(&whole_quanta(AT_THE_BOUND))?;

    assert_ne!(
        never_stalled, still_at_the_bound,
        "the control this scenario is read under: the frame after the gap has to be worth a tick \
         at all, or 'exactly one tick follows' and 'nothing follows' are the same place"
    );
    assert_eq!(
        resumed, never_stalled,
        "what the bound refused is thrown away, not held. A client that carried the surplus would \
         owe the world 585 ticks after a ten-second pause and would pay them fifteen at a time for \
         the next forty frames — a stutter that arrives *after* the stall and lasts longer than it \
         did. The frame after a clamped one is worth exactly what it took: one quantum, one tick"
    );
    Ok(())
}

#[test]
fn frames_a_tenth_of_a_second_apart_walk_the_same_distance_as_the_same_time_at_60() -> TestResult {
    let at_10 = walked_at(AT_10)?;
    let at_60 = walked_at(AT_60)?;
    let control = walked_at(HALF_OF_AT_60)?;

    assert_ne!(
        at_60, control,
        "the control this scenario is read under: half the stretch has to leave the player \
         somewhere else, or the equality below is two runs that both stopped"
    );
    assert_eq!(
        at_10, at_60,
        "ten frames a second is a slow machine and not a stalled one, and {THE_STRETCH:?} of it is \
         the same {THE_STRETCH:?} of walking. This is the half of the bound that a bound of one \
         tick would break: the two scenarios above pass under it, and a player on a machine like \
         this one would find the world running at a sixth speed with the whole suite green"
    );
    Ok(())
}

/// `ticks` frames, each of exactly one tick quantum.
///
/// The oracle every bounded run is read against, and it is an *independent* one:
/// it reaches the same tick count through frames that never touch the clamp.
fn whole_quanta(ticks: u32) -> Vec<Duration> {
    vec![TICK_QUANTUM; ticks as usize]
}

/// Where walking forward through frames that took `took` leaves the player.
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

/// Where walking forward through `rate`'s frames leaves the player.
fn walked_at(rate: Rate) -> Result<Vec3, Box<dyn Error>> {
    walked_over(&vec![rate.each; rate.frames as usize])
}

/// A client over the declared floor, holding the walk key and not yet given a
/// frame.
fn walking() -> Result<InputHarness, Box<dyn Error>> {
    let mut harness = InputHarness::started();
    harness.start_world()?;
    harness.press(FORWARD);
    Ok(harness)
}
