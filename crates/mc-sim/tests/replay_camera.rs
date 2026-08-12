//! The camera pose is a total pure function of the tick index.
//!
//! The mistake this file exists to catch is a path that accumulates per-tick
//! deltas: it looks right, it is smooth, and it makes the pose depend on float
//! summation order and on having started at tick 0 — which is the exact class of
//! irreproducibility a committed golden cannot survive. So the comparison below
//! is between a pose *reached* by advancing tick by tick and the same pose
//! *asked for* directly, and it is a comparison of bits rather than of nearly
//! equal floats: "the same pose" here means the same value, not a close one.
//!
//! Bit comparison is also why the first test does not trip `clippy::float_cmp` —
//! `f32::to_bits` turns "is this the same value" into a question about integers,
//! which is what that question actually is.
//!
//! **The wrap is asked of `Simulation`, not of a helper.** The replay restarting
//! at its first tick is a property of the thing that produces the replay's ticks,
//! and what produces them is `advance()` — so the test below advances to the last
//! tick, advances once more, and reads what was published. Asserting the pose as
//! well as the number is the point: a counter that wrapped while the pose it
//! published came from somewhere else would satisfy an assertion about the number
//! alone, and a replay that restarts is only useful if the picture restarts with
//! it.
//!
//! **The last test asks a different question and answers it differently.** Where
//! the orbit *is* is a question about distance, not about bit patterns, and it
//! has to be: the declared positions are all exactly representable in binary,
//! but the trigonometry that reaches them is not exact. At tick 60 the eye's z
//! comes out at 31.999992 rather than 32 — `f32::consts::PI` sits a hair above
//! π, so its sine is −8.7e−8 instead of 0, two units in the last place of 32.
//! A bit comparison there would fail against a perfectly correct camera, and the
//! way that gets "fixed" is by rounding or special-casing a tick, which would be
//! a real defect introduced by a bad test. So that test uses a tolerance, and
//! the tolerance is derived rather than tuned until it passed.

mod support;

use mc_sim::TICK_COUNT;
use mc_sim::replay::{CameraPose, TickError, TickIndex, pose};
use mc_sim::simulation::Simulation;

use support::{TestResult, exactly};

/// The tick the direct and the advanced pose are compared at. Halfway round the
/// orbit, so a path that happened to agree at the ends would not agree here.
const SAMPLE_TICK: u32 = 60;

/// The last tick the replay has a pose for, and the tick it restarts at.
///
/// Asking for tick [`TICK_COUNT`] is refused by `TickIndex`, so the tick after
/// the last one has to be inside the replay again.
const LAST_TICK: u32 = TICK_COUNT - 1;
const FIRST_TICK: u32 = 0;

/// The smallest distance the eye must travel between two consecutive ticks.
///
/// Derived from the declared orbit rather than measured: 120 ticks around a
/// circle of radius 96 is 3 degrees per tick, a chord of
/// `2 * 96 * sin(1.5 deg) = 5.03` blocks. One block is a fifth of that, which
/// leaves the assertion about a camera that does not move rather than about
/// floating-point noise.
const MINIMUM_STEP: f32 = 1.0;

/// Where the orbit's phase puts the eye at the two ticks the declaration names.
///
/// **These two differ in x and share z, and that is what makes them
/// falsifiable.** Exchanging the orbit's sine and cosine swings the eye a
/// quarter turn — to (32, 56, 128) and (32, 56, −64) — so it moves *both* of
/// these, on the axis they disagree about. A pair that differed only on the axis
/// a swap leaves alone could not catch it at all, which is the same trap the
/// axis-swap scenario's deliberately non-square extent was written to avoid.
///
/// The whole screen-space budget — the horizon at row 282, the landmark at pixel
/// (478, 215), its mirror at 801 — is computed from exactly these two positions.
const DECLARED_EYES: [(u32, [f32; 3]); 2] = [(0, [128.0, 56.0, 32.0]), (60, [-64.0, 56.0, 32.0])];

/// How far from a declared position the eye may sit, per axis, in blocks.
///
/// Derived from both sides rather than tuned until it passed. Below it: the
/// trigonometry's own error at this magnitude, measured at 8.4e-6 blocks — two
/// units in the last place of 32 — where this tolerance is a thousandfold
/// larger. Above it: the smallest wrong answer that would matter, which is one
/// tick of phase at 5.03 blocks, five hundredfold larger than this. A quarter
/// turn, the mistake actually being hunted, is 96 * sqrt(2) = 135.8 blocks out.
const TOLERANCE: f32 = 0.01;

#[test]
fn the_pose_at_a_tick_is_the_same_asked_for_directly_and_reached_by_advancing() -> TestResult {
    let simulation = advanced_to(SAMPLE_TICK);
    let reached = simulation.latest();

    let asked = pose(TickIndex::new(SAMPLE_TICK)?);

    assert_eq!(
        reached.tick, SAMPLE_TICK,
        "advancing {SAMPLE_TICK} times has to arrive at tick {SAMPLE_TICK}, or the two \
         poses compared below are poses of different ticks"
    );
    assert_eq!(
        exactly(&reached.camera),
        exactly(&asked),
        "a pose reached by advancing has to be the same value as the pose asked for \
         directly; an accumulated path drifts instead"
    );
    Ok(())
}

#[test]
fn the_camera_moves_between_every_pair_of_consecutive_ticks() -> TestResult {
    let mut stalled = Vec::new();
    let mut previous = pose(TickIndex::new(0)?);

    for tick in 1..TICK_COUNT {
        let current = pose(TickIndex::new(tick)?);
        if travelled(&previous, &current) <= MINIMUM_STEP {
            stalled.push(tick);
        }
        previous = current;
    }

    assert!(
        stalled.is_empty(),
        "the eye has to move more than {MINIMUM_STEP} block between consecutive ticks, and \
         at these ticks it did not: {stalled:?}"
    );
    Ok(())
}

/// The last tick is constructed as well as the first one past it refused: a
/// constructor that refused *every* tick would satisfy the refusal alone.
#[test]
fn a_tick_at_the_replays_length_is_refused_and_names_the_length() -> TestResult {
    let last = TickIndex::new(TICK_COUNT - 1)?;

    let refused = TickIndex::new(TICK_COUNT)
        .err()
        .ok_or("a tick at the replay's length has to be refused, not accepted")?;

    assert_eq!(
        last.get(),
        TICK_COUNT - 1,
        "the tick before the end is a tick of this replay"
    );
    assert_eq!(
        refused,
        TickError::BeyondReplay {
            tick: TICK_COUNT,
            tick_count: TICK_COUNT,
        },
        "the refusal has to name the replay's length rather than extrapolate the path"
    );
    Ok(())
}

#[test]
fn advancing_past_the_replays_last_tick_publishes_the_first_tick_and_its_pose() -> TestResult {
    let simulation = advanced_to(LAST_TICK);
    let reached_the_end = simulation.latest();

    simulation.advance();

    let restarted = simulation.latest();
    assert_eq!(
        reached_the_end.tick, LAST_TICK,
        "the replay has to have reached its last tick before the advance under test, or the \
         wrap below is not the wrap that happens at the end of the replay — a counter stuck \
         at {FIRST_TICK} would answer {FIRST_TICK} here too"
    );
    assert_eq!(
        (restarted.tick, exactly(&restarted.camera)),
        (FIRST_TICK, exactly(&pose(TickIndex::new(FIRST_TICK)?))),
        "the tick after the replay's last has to be the replay's first, published with the \
         first tick's pose: the client renders whatever this publishes, so it starts the \
         replay again rather than asking for a pose beyond tick {LAST_TICK} or stopping"
    );
    Ok(())
}

#[test]
fn the_orbit_puts_the_eye_at_the_two_declared_points_of_its_half_turn() -> TestResult {
    let mut misplaced = Vec::new();

    for (tick, declared) in DECLARED_EYES {
        let placed = pose(TickIndex::new(tick)?).eye;
        if furthest_axis(placed, declared) > TOLERANCE {
            misplaced.push(format!(
                "tick {tick} places the eye at {placed:?}, not {declared:?}"
            ));
        }
    }

    assert!(
        misplaced.is_empty(),
        "the orbit's phase is what every projected figure in this feature is computed from, \
         so tick 0 belongs at (128, 56, 32) and tick 60 at (-64, 56, 32). Those two differ \
         in x and share z, so a path that exchanged the orbit's sine and cosine moves both \
         of them — and moves them a quarter turn, which no other pose test can see, since a \
         swapped path is equally total, equally moving and equally refusing: {misplaced:?}"
    );
    Ok(())
}

/// A simulation that has been advanced `tick` times from its start, and is
/// therefore at tick `tick`.
fn advanced_to(tick: u32) -> Simulation {
    let simulation = Simulation::new();
    for _ in 0..tick {
        simulation.advance();
    }
    simulation
}

/// The largest disagreement between two positions on any one axis.
fn furthest_axis(placed: [f32; 3], declared: [f32; 3]) -> f32 {
    placed
        .iter()
        .zip(declared.iter())
        .map(|(placed, declared)| (placed - declared).abs())
        .fold(0.0, f32::max)
}

/// How far the eye moved between two poses, in the plane the orbit runs in.
fn travelled(from: &CameraPose, to: &CameraPose) -> f32 {
    let [from_x, _, from_z] = from.eye;
    let [to_x, _, to_z] = to.eye;
    ((to_x - from_x).powi(2) + (to_z - from_z).powi(2)).sqrt()
}
