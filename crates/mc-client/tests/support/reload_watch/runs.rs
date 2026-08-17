//! How long a run of tick boundaries lasts, and how its boundaries are crossed.
//!
//! # The bounds a run needs are not all the same kind of number
//!
//! Collapsing them into one is what made three fixtures flaky under the gate's load.
//! A run expecting *no* attempt needs a **minimum** — a window long enough for the
//! presence it denies — and being short there is **silent**. A run expecting *an*
//! attempt needs a **maximum**, polled to rather than slept through, where being
//! generous costs nothing and can never make a green.
//!
//! **Neither is `SETTLING_WINDOW`.** That constant is production policy: how long an
//! editor's save is given to settle. These are statements about how slow this machine
//! may be while the suite runs, so they carry their own numbers with their own
//! derivations — and each derivation says which direction it came from.

use std::error::Error;
use std::thread;
use std::time::{Duration, Instant};

use crate::input::InputHarness;

use super::{Attempt, attempt_of};

/// How long a boundary waits for the next one while nothing is being reported.
///
/// **A run's patience is time and not a count of boundaries, and the first draft of
/// this module got that wrong in both directions.** A boundary in a tight test loop
/// costs about a hundred nanoseconds while a boundary in the shipped client is a
/// rendered frame, so a boundary count is a proxy for elapsed time whose conversion
/// factor differs by five orders of magnitude between the two — and a candidate
/// build is *time*: a scripting host and seven declaration files, milliseconds of
/// work on a thread that is not the tick's. Two thousand tight boundaries came to
/// 0.2 ms against a build of 0.7–1.7 ms, so a run counting attempts gave up an
/// order of magnitude before the thing it was waiting for could happen, and a run
/// expecting *no* attempt could not have seen a wrongly-started one land either.
/// Pacing the quiet boundaries makes the bound below a duration and the boundary
/// count fall out of it.
const BETWEEN_QUIET_BOUNDARIES: Duration = Duration::from_millis(1);

/// How long an attempt may take to run to its end on a machine under load.
///
/// **A test bound, not production policy, and that is why it has a number of its
/// own.** `SETTLING_WINDOW` says how long an editor's save is given to settle; this
/// says how slow this machine may be while the suite runs. Denominating the two in
/// one number made them move together, which is what let a fixture bound be short
/// exactly where it had to be long.
///
/// **Derived from both directions, as `testing.md` §2 requires.**
///
/// - *From below, measured:* under the gate's own instrumented run of 1 190 tests
///   with the GPU suites in flight, an attempt had **not** completed inside 300 ms,
///   while the same test completed in 0.70 s alone and 0.73 s under coverage. So a
///   bound has to be above 300 ms, and 1.5 s is five times that observed
///   insufficiency and twice the whole observed duration of the test under coverage.
/// - *From above:* nothing about correctness — waiting longer cannot turn a
///   wrongly-begun attempt or an unwanted second one into a pass, because what is
///   asserted against this bound is that neither happened. The ceiling is **runtime**
///   alone, which is why it is one and a half seconds rather than a minute.
/// - *The smallest difference it must still catch:* one attempt beginning and ending
///   inside it. That is the same floor stated from the other side, and it is what a
///   number reached by raising it until the suite went quiet would have lost.
///
/// It was **not** reached by loosening until green: the run above was measured first
/// and the bound derived from it, and a slow-build fixture then reproduced the
/// failure at 1.2 s and the pass at 1.5 s.
pub const AN_ATTEMPT_MAY_NOT_OUTLAST: Duration = Duration::from_millis(1_500);

/// The most a run that expects an attempt waits for one before it gives up.
///
/// **A maximum, so it is polled to rather than slept through** — the run crosses
/// boundaries until it sees the attempt or reaches this, which is what keeps "the
/// change arrives within a bound" from becoming an equality against machine load.
///
/// - *From below:* it has to exceed [`AN_ATTEMPT_MAY_NOT_OUTLAST`] by a margin, or a
///   slow attempt would be reported as no attempt. Four times over.
/// - *From above:* only the cost of a genuine failure. Reaching it **never** makes a
///   green — it produces an empty attempt list, which is what every assertion here
///   compares against a non-empty one — so a generous value asserts nothing and
///   cannot hide anything.
pub const A_RUN_EXPECTING_ONE_MAY_NOT_OUTLAST: Duration = Duration::from_secs(6);

/// The relationship between the two, stated where it cannot drift: a run that gives
/// up sooner than an attempt may legitimately take is a run that reports load as a
/// defect.
const _: () = assert!(
    A_RUN_EXPECTING_ONE_MAY_NOT_OUTLAST.as_millis() > AN_ATTEMPT_MAY_NOT_OUTLAST.as_millis()
);

/// How long a run that expects **no** attempt keeps crossing boundaries before it is
/// satisfied that none is going to end.
///
/// A **minimum**, and being short here is silent: a run that gave up before a
/// wrongly-begun attempt could have completed reports "nothing began" for a reason
/// about the machine rather than about the client.
fn long_enough_for_a_wrongly_begun_attempt_to_end() -> Duration {
    AN_ATTEMPT_MAY_NOT_OUTLAST
}

/// How long a run that has already seen an attempt goes on before it accepts that no
/// **second** one is following.
///
/// **This is an absence bound too, and treating it as a short tail was the silent
/// half of the same defect.** What it denies is a second attempt, so it needs a
/// window long enough for one to begin and end — the same floor as the bound above,
/// not a debouncer window. An earlier form of this split gave it two settling windows
/// on the argument that a second report would have to arrive inside one window of the
/// first; that is true of the *report* and false of the *attempt it starts*, which
/// still has to build.
fn long_enough_for_another_to_follow() -> Duration {
    AN_ATTEMPT_MAY_NOT_OUTLAST
}

/// The most a run waits before it gives up and lets the assertion speak.
fn before_giving_up() -> Duration {
    A_RUN_EXPECTING_ONE_MAY_NOT_OUTLAST
}

/// One tick boundary of `client`, and whatever it reported.
pub fn boundary(client: &mut InputHarness) -> Option<Attempt> {
    client.tick();
    attempt_of(client.take_reload_report())
}

/// What every boundary of a quiet run reported, over a run long enough that an
/// attempt begun at the first of them would have ended.
///
/// **For the scenarios that expect no attempt at all**, and the length of the run is
/// the whole of what makes those assertions mean anything: a run that ended before a
/// wrongly-started build could land would report nothing either way. Every boundary
/// is crossed whatever it reports, so the ticks really were advanced.
pub fn crossing_a_quiet_run(client: &mut InputHarness) -> Vec<Option<Attempt>> {
    let started = Instant::now();
    let mut crossed = Vec::new();
    while started.elapsed() < long_enough_for_a_wrongly_begun_attempt_to_end() {
        crossed.push(boundary(client));
        pause_between_boundaries();
    }
    crossed
}

/// The same, for a run that expects an attempt: it waits for one, and then goes on
/// long enough to see whether a second follows.
///
/// **The quiet stretch does not start until something has been reported.** An earlier
/// form started it at the first boundary, so a run gave up two settling windows after
/// it began whether or not anything had happened yet — which made every test using it
/// a hostage to how busy the machine was, and under the gate's load one of them
/// reported an empty presence half while passing alone twice.
///
/// Until an attempt is reported the only bound is [`before_giving_up`]. After one is
/// reported, a second would have to arrive inside a window of it. A run that never
/// sees one gives up and the assertion is what says so.
pub fn until_settled(client: &mut InputHarness) -> Vec<Option<Attempt>> {
    let started = Instant::now();
    let mut quiet: Option<Instant> = None;
    let mut crossed = Vec::new();
    while started.elapsed() < before_giving_up()
        && quiet.is_none_or(|since| since.elapsed() < long_enough_for_another_to_follow())
    {
        let reported = boundary(client);
        if reported.is_some() {
            quiet = Some(Instant::now());
        }
        crossed.push(reported);
        pause_between_boundaries();
    }
    crossed
}

/// Lets the thread a candidate is being built on have the machine for a moment.
///
/// **A test loop crosses boundaries about ten thousand times faster than a client
/// draws frames**, so a run that never paused would spend its whole patience before
/// a build could finish. Pacing is what makes the bounds above durations rather than
/// boundary counts in disguise.
pub fn pause_between_boundaries() {
    thread::sleep(BETWEEN_QUIET_BOUNDARIES);
}

/// Whether a run that began at `started` may cross another boundary before it gives
/// up.
///
/// For the two scenarios whose loops carry a script of their own — a walk to keep in
/// lockstep, a reading to take between boundaries — so that their patience is this
/// module's and not a second number beside it.
#[must_use]
pub fn may_cross_another(started: Instant) -> bool {
    started.elapsed() < before_giving_up()
}

/// Every attempt a run reported, in order.
///
/// The length is the number of attempts that ended, never the number of ticks.
#[must_use]
pub fn ended(crossed: &[Option<Attempt>]) -> Vec<Attempt> {
    crossed.iter().flatten().cloned().collect()
}

/// One taking up, for a scenario expecting exactly that and nothing else.
#[must_use]
pub fn taken_up_once() -> Vec<Attempt> {
    vec![Attempt::TakenUp]
}

/// How many boundaries were crossed before the first one reported anything.
#[must_use]
pub fn before_the_first_report(crossed: &[Option<Attempt>]) -> Option<usize> {
    crossed.iter().position(Option::is_some)
}

/// Refuses unless at least one boundary was crossed with a build in flight.
///
/// **The premise of every scenario about what happens *while* a candidate is being
/// built.** A build that ran on the tick thread reports its outcome at the boundary
/// that started it, so there is no such boundary and every assertion about those
/// ticks is vacuously true — which is the shape this refusal exists to turn into a
/// failure.
///
/// **What it does not grade, measured rather than assumed:** that no tick ever
/// *waited* on the build. A collect that blocks on the worker instead of polling it
/// still reports at the boundary after the one that started it, so this refusal
/// passes — and so does every scenario in the phase, because the difference between a
/// stalled tick and a free one is purely temporal and nothing here reads a clock into
/// an assertion. The scenario whose wording comes closest — the ticks a candidate is
/// built over putting the player where a run with no reload would — passes with the
/// build blocking the tick, because its observable is *where* the ticks put the player
/// while its property is *when* they happened. What does grade that is
/// `tests/reload_build_runs_off_the_tick_thread.rs`, by thread identity and ordering
/// rather than by any duration.
///
/// # Errors
///
/// Returns an error where nothing was reported at all, and where the first report
/// arrived at the boundary that must have started the build.
pub fn require_a_build_in_flight(crossed: &[Option<Attempt>]) -> Result<(), Box<dyn Error>> {
    match before_the_first_report(crossed) {
        None => Err(format!(
            "this scenario needs an attempt to run to its end inside {crossed} boundaries and \
             none of them reported anything at all",
            crossed = crossed.len()
        )
        .into()),
        Some(0) => Err(NOTHING_WAS_EVER_IN_FLIGHT.into()),
        Some(_) => Ok(()),
    }
}

/// What a run whose build never crossed a boundary is told.
const NOTHING_WAS_EVER_IN_FLIGHT: &str = "this scenario is about the ticks a candidate is built over, and the very boundary that saw \
     the change also reported the outcome — so no tick ran with a build in flight and every \
     assertion about those ticks would hold over none of them";
