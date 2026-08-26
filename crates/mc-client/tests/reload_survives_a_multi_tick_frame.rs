//! What a reload the client answered reaches, when the frame that answered it
//! spent more than one tick.
//!
//! # A tick produces this and a frame consumes it, and those stopped being the
//! same thing
//!
//! Crossing the content root's boundary is a *tick's* work, and reading what it
//! produced is a *frame's*. While a frame was exactly one tick the two cadences
//! were one and nothing could fall between them. A frame now spends as many ticks
//! as the elapsed time it was handed buys, so the tick that answers is followed by
//! ticks that have nothing to say — and "nothing to say" must not read as "the
//! answer is withdrawn".
//!
//! # Why both arms and not only the one that swaps content
//!
//! An accepted candidate and a refused one are two arms of the producer and one
//! arm of the consumer. Losing the acceptance is the louder failure — the
//! simulation serves content the device was never given — but losing the refusal
//! is the one a mod author cannot work around, because what they get back from a
//! save they know they made is silence.
//!
//! # The reading is one frame, and the boundaries after it are what say whether it
//! means anything
//!
//! A candidate is built on a thread, and nothing here can ask whether that build
//! has finished without collecting it — the ask *is* the collect. So the run waits
//! out the bound an attempt may not outlast, takes its reading with a single
//! multi-tick frame, and goes on crossing ordinary one-tick boundaries afterwards.
//! Which of the two saw the attempt is the whole verdict: the multi-tick frame is
//! the capability, nobody at all is the defect, and a later boundary means this
//! machine's build had not finished in time and the reading says nothing either
//! way. **An enumerated verdict rather than an absence**, so a run that could not
//! look cannot answer under the good arm's name.

#[path = "support/input/mod.rs"]
mod input;
#[path = "support/reload.rs"]
mod reload;
#[path = "support/reload_watch.rs"]
mod reload_watch;
#[path = "support/reload_world.rs"]
mod reload_world;
mod support;

use std::thread;
use std::time::{Duration, Instant};

use mc_sim::player::TICK_QUANTUM;

use input::InputHarness;
use reload::{AMBER_FILE, GRASS, STONE_FILE, restating, shipped, stone_that_is_not_solid};
use reload_watch::{
    A_RUN_EXPECTING_ONE_MAY_NOT_OUTLAST, AN_ATTEMPT_MAY_NOT_OUTLAST, Attempt, NOT_A_CHUNK,
    a_client_on, attempt_of, block_path, boundary, pause_between_boundaries,
};
use support::TestResult;

/// How many ticks the frame that takes the reading spends.
///
/// Three rather than two, so a defect dropping only the *last* tick's answer would
/// still be caught and so the frame is unambiguously the multi-tick regime rather
/// than a boundary case of the single-tick one. At sixty ticks a second this is a
/// frame drawn at twenty a second: a machine playing badly, not one that has
/// stopped.
const TICKS_IN_THE_FRAME: u32 = 3;

/// How long the run crosses ordinary boundaries after the reading before it accepts
/// that no straggler is coming.
///
/// A **minimum**: what it denies is a late attempt, and being short here would let a
/// slow machine's inconclusive reading be recorded as the defect — and the defect it
/// would be recorded as is [`WhoSawIt::NobodyTheAnswerWasOverwritten`], the one this
/// spec exists to have fixed.
///
/// So it is the bound a run *expecting* an attempt is given, not the bound the
/// attempt itself is allowed: [`AN_ATTEMPT_MAY_NOT_OUTLAST`] is already spent
/// sleeping before the reading, and a window merely equal to it leaves a build that
/// legitimately outlasted that sleep no margin at all. `runs.rs` states the rule
/// where it cannot drift — a run's window has to exceed what an attempt may take, or
/// a slow attempt is reported as no attempt — and this is that margin, four times
/// over.
const A_STRAGGLER_MAY_NOT_OUTLAST: Duration = A_RUN_EXPECTING_ONE_MAY_NOT_OUTLAST;

/// Who saw the attempt the run provoked.
///
/// Total, so an assertion against [`WhoSawIt::TheMultiTickFrame`] rejects every
/// other answer — including the one that means this reading could not be taken.
#[derive(Debug, PartialEq, Eq)]
enum WhoSawIt {
    /// The multi-tick frame reported it. This is the capability.
    TheMultiTickFrame,
    /// No boundary reported it at all: a tick produced the answer and a later tick
    /// of the same frame overwrote it before the frame path could read it.
    NobodyTheAnswerWasOverwritten,
    /// An ordinary boundary after the reading reported it, so the build was still
    /// running when the multi-tick frame went past. **This reading says nothing
    /// about the defect**, and it is its own arm for exactly that reason.
    ALaterBoundarySoTheBuildWasStillRunning,
    /// Something the run did not provoke was reported, which is a fixture that has
    /// stopped being about what it says it is about.
    SomethingElse(String),
}

#[test]
fn a_candidate_a_multi_tick_frame_accepts_reaches_the_frame_path() -> TestResult {
    let root = shipped()?;
    let (mut client, reports) = a_client_on(&root, GRASS)?;
    let root = restating(root, STONE_FILE, &stone_that_is_not_solid())?;

    reports.changed(&[block_path(&root, STONE_FILE)])?;
    let saw = who_saw_it(&mut client, &Attempt::TakenUp);

    assert_eq!(
        saw,
        WhoSawIt::TheMultiTickFrame,
        "the tick that takes a candidate up is followed, inside the same frame, by ticks with \
         nothing to report — and a frame reads what its ticks produced exactly once, after all of \
         them. If those later ticks clear the answer the simulation serves the new content while \
         the frame path never uploads its layers, never retires the re-mesh worker's and never \
         says a word to the author: the one state the frame path's own header declares must end \
         the run, reached in silence instead"
    );
    Ok(())
}

#[test]
fn a_candidate_a_multi_tick_frame_refuses_reaches_the_frame_path() -> TestResult {
    let root = shipped()?;
    let (mut client, reports) = a_client_on(&root, GRASS)?;
    let root = root.declaring_block(AMBER_FILE, NOT_A_CHUNK)?;

    reports.changed(&[block_path(&root, AMBER_FILE)])?;
    let saw = who_saw_it(&mut client, &a_refusal());

    assert_eq!(
        saw,
        WhoSawIt::TheMultiTickFrame,
        "a refusal is the only thing a mod author gets back from a save that will not load, and it \
         leaves the client through the same field an acceptance does. Losing it to a later tick of \
         the same frame leaves the author looking at a file they know they saved, with the old \
         content still serving and nothing said about why"
    );
    Ok(())
}

/// A refusal, whatever it said.
///
/// The wording belongs to `reload_refuses_a_broken_declaration.rs` and not to this
/// file: what is asserted here is *that* a refusal crossed out of the client, so
/// the text is not spelled a second time.
fn a_refusal() -> Attempt {
    Attempt::Refused {
        said: String::new(),
    }
}

/// Whether two reports are the same kind of answer, a refusal's wording aside.
fn same_kind(one: &Attempt, other: &Attempt) -> bool {
    matches!(
        (one, other),
        (Attempt::TakenUp, Attempt::TakenUp) | (Attempt::Refused { .. }, Attempt::Refused { .. })
    )
}

/// Drives the run this file's header describes and reports which boundary saw
/// `expected`.
fn who_saw_it(client: &mut InputHarness, expected: &Attempt) -> WhoSawIt {
    // One ordinary boundary starts the build; it cannot have finished by the end of
    // that same tick, so this reports nothing and a report here is a broken fixture.
    let starting = boundary(client);
    // A wait rather than a poll: asking whether the build has finished is the same
    // act as collecting it. The straggler arm below is what stops a wait that was
    // too short for this machine from reading as the defect.
    thread::sleep(AN_ATTEMPT_MAY_NOT_OUTLAST);

    client.frame(TICK_QUANTUM * TICKS_IN_THE_FRAME);
    let reading = attempt_of(client.take_reload_report());
    let after = crossing_until_no_straggler(client);

    let elsewhere: Vec<Attempt> = after.into_iter().flatten().collect();
    let odd = starting
        .iter()
        .chain(reading.iter())
        .chain(elsewhere.iter())
        .find(|seen| !same_kind(seen, expected))
        .or(starting.as_ref());

    match (odd, reading.is_some(), elsewhere.is_empty()) {
        (Some(seen), _, _) => WhoSawIt::SomethingElse(format!("{seen:?}")),
        (None, true, _) => WhoSawIt::TheMultiTickFrame,
        (None, false, false) => WhoSawIt::ALaterBoundarySoTheBuildWasStillRunning,
        (None, false, true) => WhoSawIt::NobodyTheAnswerWasOverwritten,
    }
}

/// Ordinary one-tick boundaries, for long enough that an attempt still running when
/// the reading was taken would have been reported.
fn crossing_until_no_straggler(client: &mut InputHarness) -> Vec<Option<Attempt>> {
    let started = Instant::now();
    let mut crossed = Vec::new();
    while started.elapsed() < A_STRAGGLER_MAY_NOT_OUTLAST {
        crossed.push(boundary(client));
        pause_between_boundaries();
    }
    crossed
}
