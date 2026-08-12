//! The simulation publishes without waiting on whoever is reading.
//!
//! **The first test has to be genuinely concurrent, and that is the whole
//! reason it looks like this.** In the product one tick is advanced per rendered
//! frame on one thread, so publishing and reading are never simultaneous; a
//! single-threaded store-then-load would pass over any implementation at all,
//! including one that takes a writer lock the reader holds. So a second thread
//! publishes while this one is still holding a loaded snapshot, and the
//! assertion is that the publish *finished*.
//!
//! The deadline below is a test-side clock, not a product one. Nothing in this
//! feature reads a wall clock; a deadline is simply the only way to turn "did
//! not wait" into a failure rather than into a hang.
//!
//! The second test is the other half of the same guarantee: holding a snapshot
//! is only useful if nothing can rewrite it underneath the holder. It compares
//! the held snapshot against the pose function rather than against a copy of
//! itself, so a snapshot quietly rewritten to the *newer* tick's pose fails
//! rather than matching its own new contents.

mod support;

use std::sync::Arc;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use mc_sim::replay::{TickIndex, pose};
use mc_sim::simulation::Simulation;

use support::{TestResult, exactly};

/// The tick the reader is holding while the next one is published.
const HELD_TICK: u32 = 60;

/// How long the publish may take before it counts as having waited.
///
/// Generous by three orders of magnitude: swapping a pointer takes nanoseconds,
/// so anything near this deadline is a publisher that blocked, not a slow
/// machine.
const PUBLISH_DEADLINE: Duration = Duration::from_secs(5);

#[test]
fn publishing_a_snapshot_completes_while_the_renderer_still_holds_the_previous_one() -> TestResult {
    let simulation = Arc::new(advanced_to(HELD_TICK));
    let held = simulation.latest();
    let publisher = Arc::clone(&simulation);
    let (finished, waiting) = mpsc::channel();

    let publish = thread::spawn(move || {
        publisher.advance();
        finished.send(publisher.latest().tick).is_ok()
    });
    let published = waiting.recv_timeout(PUBLISH_DEADLINE).ok();

    assert_eq!(
        held.tick, HELD_TICK,
        "the reader has to be holding the snapshot the publish runs against"
    );
    assert_eq!(
        published,
        Some(HELD_TICK + 1),
        "the publish has to complete inside {PUBLISH_DEADLINE:?} while a reader still holds \
         the previous snapshot; stalling the simulation to serve a reader is the one thing \
         this seam may not do"
    );
    publish.join().or(Err("the publishing thread panicked"))?;
    Ok(())
}

#[test]
fn a_later_publish_leaves_the_snapshot_the_renderer_holds_unchanged() -> TestResult {
    let simulation = advanced_to(HELD_TICK);
    let held = simulation.latest();

    simulation.advance();

    assert_eq!(
        simulation.latest().tick,
        HELD_TICK + 1,
        "a later publish has to have happened, or nothing could have disturbed the held \
         snapshot in the first place"
    );
    assert_eq!(
        (held.tick, exactly(&held.camera)),
        (HELD_TICK, exactly(&pose(TickIndex::new(HELD_TICK)?))),
        "the held snapshot still describes its own tick after a later one was published"
    );
    Ok(())
}

/// A simulation that has been advanced `tick` times from its start.
fn advanced_to(tick: u32) -> Simulation {
    let simulation = Simulation::new();
    for _ in 0..tick {
        simulation.advance();
    }
    simulation
}
