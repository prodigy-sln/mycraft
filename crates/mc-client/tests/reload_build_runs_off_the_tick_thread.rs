//! A candidate is built somewhere other than the ticking thread, and no tick
//! waits for it.
//!
//! # Why this file exists, and what nothing else in this spec can see
//!
//! Building the candidate off the tick thread is the whole reason a reload does not
//! stall the game, and **no scenario in this spec grades it.** The scenario whose
//! wording comes closest — the ticks a candidate is built over putting the player
//! where a run with no reload would — was measured to pass with the build
//! *blocking* the tick: its observable is *where* the ticks put the player and its
//! property is *when* they happened, and a blocking collect changes only the
//! second. The latency benchmark cannot see it either, and worse: a blocking
//! collect is *faster* than a polled one, because it drops the polling overhead. So
//! the property being violated makes the number look better.
//!
//! Left there, a later change moves the build onto the tick thread, every test
//! stays green, and one save freezes the game for as long as a content root takes
//! to read.
//!
//! # The discrimination is identity and ordering, never duration
//!
//! Elapsed time is refused, and so is any count that proxies it: a polled collect
//! spanning many boundaries against a blocking one spanning exactly one is a timing
//! assertion in a count's clothing, and it would go red on a machine where a tick
//! costs a millisecond. What is asked instead are two facts a build can only report
//! about itself:
//!
//! - **Which thread it ran on**, as a `ThreadId`. Deterministic, no window, and it
//!   discriminates exactly: the build either ran somewhere other than the thread
//!   that was ticking or it did not.
//! - **Whether the tick came back before it was let go.** The injected build
//!   announces itself and then waits for this test to release it. The test crosses
//!   a boundary *while it is waiting* and only then releases it, so a run that
//!   reaches the release at all is a run whose ticks returned with a build
//!   outstanding. A tick that waited instead is the build's own answer: nobody
//!   released it, because the thread that would have was inside the wait.
//!
//! Neither fact covers the other, and each catches a different spelling of the same
//! defect — the build called inline, and the build spawned and immediately joined.
//! A third spelling, a collect that blocks at the *next* boundary rather than the
//! one that started the build, is what the second crossed boundary is for.
//!
//! # The build is injected and the spawn is not
//!
//! `ContentReload::building` is the product's own door for a build a fixture
//! supplies — the same one the lost-worker scenario goes through — and it changes
//! *what* is built, never *where*. Both it and the shipped
//! `watching_shipped_content` reach `begin_a_build`, which is the one place a build
//! is started, so a build that came to run on the tick thread would run there for
//! the shipped door too. This is not a fixture grading its own call: nothing here
//! spawns anything.
//!
//! # The candidate is byte-identical to the root already serving
//!
//! What is graded is where the build ran, not what it produced, and a root read
//! twice is accepted the second time under a later serial. The attempt list is what
//! says the build ran to its end and was taken up at all.
//!
//! # The one bound, and which way it fails
//!
//! The wait is denominated in settling windows, like every other bound in this
//! suite, and it is only ever reached by a tick that is holding the build up: in a
//! healthy run the release arrives after one channel receive and one tick step —
//! microseconds against seconds of patience. **A run that hits it reports a tick
//! that waited**, which is red, so the bound cannot green anything; what it costs
//! is that a machine stalled for whole seconds between two statements of this test
//! would report a defect it does not have.

#[path = "support/input/mod.rs"]
mod input;
#[path = "support/reload.rs"]
mod reload;
#[path = "support/reload_watch.rs"]
mod reload_watch;
#[path = "support/reload_world.rs"]
mod reload_world;
mod support;

use std::error::Error;
use std::path::Path;
use std::sync::mpsc::{Receiver, Sender, TryRecvError, channel};
use std::sync::{Mutex, OnceLock};
use std::thread::{self, ThreadId};
use std::time::Duration;

use mc_core::content::LayerAssignment;
use mc_sim::content::{ContentError, LoadedContent};
use mc_sim::reload::ContentReload;

use input::InputHarness;
use reload::{GRASS, STONE_FILE, shipped};
use reload_watch::{Reports, block_path, boundary, ended, taken_up_once, until_settled, watch};
use reload_world::{floor_of, playing, standing};
use support::TestResult;
use support::content::ContentRoot;

/// How long a build waits to be released before it concludes that whoever would have
/// released it is waiting for it instead.
///
/// **A maximum with a number of its own, not the production settling window times a
/// count.** Reached only by a tick that is holding the build up: in a healthy run the
/// release arrives after one channel receive and one tick step, microseconds against
/// the seconds here. Reaching it produces a *red* — the verdict then says a tick
/// waited — so a generous value cannot hide anything, and the only cost is the
/// wall-clock of a run that was going to fail.
const A_BLOCKED_TICK_MAY_NOT_HOLD_A_BUILD_LONGER_THAN: Duration = Duration::from_secs(6);

/// The whole of that patience, as a duration.
const fn before_a_tick_must_have_come_back() -> Duration {
    A_BLOCKED_TICK_MAY_NOT_HOLD_A_BUILD_LONGER_THAN
}

/// What the injected build said about itself.
#[derive(Debug, PartialEq, Eq)]
enum Announced {
    /// It has begun, on this thread.
    Begun(ThreadId),
    /// It has stopped waiting, and whether it was released rather than giving up.
    StoppedWaiting { released: bool },
}

/// Where a candidate was built, and whether a tick waited for it.
///
/// **A total verdict**, so an assertion against the good arm rejects a build on the
/// tick thread, a build a tick waited for, a build that never happened, and a run
/// that produced some other number of announcements — `assert!` on either fact
/// alone would let the other through.
#[derive(Debug, PartialEq, Eq)]
enum WhereItRan {
    /// On a thread of its own, and the boundaries crossed while it waited came
    /// back before anything released it.
    SomewhereElseAndNoTickWaited,
    /// On the very thread that was ticking.
    OnTheTickingThread,
    /// On a thread of its own, but nothing released it before it gave up waiting —
    /// so the thread that would have was inside the build.
    SomewhereElseButATickWaitedForIt,
    /// No build announced itself at all.
    NoBuildBegan,
    /// Neither one build's two announcements nor none of them.
    Announcements(usize),
}

#[test]
fn a_candidate_is_built_off_the_ticking_thread_and_no_tick_waits_for_it() -> TestResult {
    let root = shipped()?;
    let (mut client, reports) = a_client_whose_build_announces_itself(&root)?;
    let (announced, release) = a_rendezvous()?;
    let ticking = thread::current().id();

    reports.changed(&[block_path(&root, STONE_FILE)])?;
    let mut crossed = vec![boundary(&mut client)];
    let begun = announced.recv_timeout(before_a_tick_must_have_come_back())?;
    // A boundary crossed while the build is still waiting. Without it, a collect
    // that blocks at the boundary *after* the one that started the build would
    // never be asked to.
    crossed.push(boundary(&mut client));
    release.send(())?;
    crossed.extend(until_settled(&mut client));

    assert_eq!(
        (where_it_ran(begun, &announced, ticking), ended(&crossed)),
        (WhereItRan::SomewhereElseAndNoTickWaited, taken_up_once()),
        "reading a content root is milliseconds of work and a tick is microseconds, so a build the \
         tick thread runs — or waits for — stutters the game on every save. Nothing else in this \
         spec can see it: the scenario whose wording comes closest passes with the build blocking \
         the tick, and the latency benchmark is *faster* when it does. The thread is asked by \
         identity because a duration is a flake and a count of boundaries is a duration in \
         disguise; the release is what says a tick came back with a build still outstanding"
    );
    Ok(())
}

/// What the build's announcements amount to, against the thread that was ticking.
fn where_it_ran(
    begun: Announced,
    announced: &Receiver<Announced>,
    ticking: ThreadId,
) -> WhereItRan {
    let mut said = vec![begun];
    said.extend(drained(announced));
    match said.as_slice() {
        [] => WhereItRan::NoBuildBegan,
        [Announced::Begun(on), Announced::StoppedWaiting { released }] => {
            if *on == ticking {
                WhereItRan::OnTheTickingThread
            } else if *released {
                WhereItRan::SomewhereElseAndNoTickWaited
            } else {
                WhereItRan::SomewhereElseButATickWaitedForIt
            }
        }
        other => WhereItRan::Announcements(other.len()),
    }
}

/// Everything a channel has to hand over right now.
fn drained(announced: &Receiver<Announced>) -> Vec<Announced> {
    let mut said = Vec::new();
    loop {
        match announced.try_recv() {
            Ok(one) => said.push(one),
            Err(TryRecvError::Empty | TryRecvError::Disconnected) => return said,
        }
    }
}

/// Where the injected build announces itself and waits.
///
/// A static because [`mc_sim::reload::CandidateBuild`] is a function pointer rather
/// than a closure — which is what keeps `ContentReload` `Debug` and `Send` — so a
/// build cannot carry a channel with it. The lost-worker fixture reaches for a
/// static for the same reason. Each test binary runs in its own process, so one
/// rendezvous is one run.
static RENDEZVOUS: OnceLock<Rendezvous> = OnceLock::new();

/// The two channel ends the build itself holds.
struct Rendezvous {
    /// What the build says about itself, as it happens.
    announcing: Mutex<Sender<Announced>>,
    /// What the build waits for before it goes on.
    release: Mutex<Receiver<()>>,
}

/// The two ends the test holds: what the build announced, and the release.
///
/// # Errors
///
/// Returns an error if a rendezvous has already been set up, which is two runs in
/// one process rather than one.
fn a_rendezvous() -> Result<(Receiver<Announced>, Sender<()>), Box<dyn Error>> {
    let (announcing, announced) = channel();
    let (release, released) = channel();
    RENDEZVOUS
        .set(Rendezvous {
            announcing: Mutex::new(announcing),
            release: Mutex::new(released),
        })
        .map_err(
            |_already| "a rendezvous is already set up, so this build would announce itself to it",
        )?;
    Ok((announced, release))
}

/// A build that says which thread it is on, waits to be released, and then reads
/// the root exactly as the shipped build does.
///
/// **It decides nothing.** Where it runs is `begin_a_build`'s answer and what it
/// produces is the one content door's; all this adds is that it says so and then
/// holds still long enough for a boundary to be crossed around it.
fn a_build_that_announces_itself_and_waits(
    root: &Path,
    spent: &LayerAssignment,
) -> Result<LoadedContent, ContentError> {
    let Some(meeting) = RENDEZVOUS.get() else {
        return mc_sim::content::load(root, spent);
    };
    announce(meeting, Announced::Begun(thread::current().id()));
    let released = meeting.release.lock().is_ok_and(|waiting| {
        waiting
            .recv_timeout(before_a_tick_must_have_come_back())
            .is_ok()
    });
    announce(meeting, Announced::StoppedWaiting { released });
    mc_sim::content::load(root, spent)
}

/// Says one thing about the build, dropping the failure to say it.
///
/// Nothing listening is a test that has already finished; there is nobody left for
/// an error here to be reported to, and the assertion is what speaks.
fn announce(meeting: &Rendezvous, said: Announced) {
    if let Ok(announcing) = meeting.announcing.lock() {
        drop(announcing.send(said));
    }
}

/// A client on a floor of grass playing the root at `root`, watching that same root
/// through a double, and building candidates through the announcing build.
///
/// # Errors
///
/// Returns an error if the root does not read, if the world does not build, or if
/// the content declares no solid block at all.
fn a_client_whose_build_announces_itself(
    root: &ContentRoot,
) -> Result<(InputHarness, Reports), Box<dyn Error>> {
    let (simulation, holding) = playing(root.path(), standing(), |registry| {
        floor_of(registry, GRASS)
    })?;
    let (watching, reports) = watch();
    let mut client = InputHarness::started();
    client.play(simulation, holding);
    client.attach_reload(ContentReload::building(
        root.path().to_owned(),
        Box::new(watching),
        a_build_that_announces_itself_and_waits,
    ));
    Ok((client, reports))
}
