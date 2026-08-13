//! The simulation publishes a tick at a time, and never rewrites what a reader
//! is already holding.
//!
//! **The concurrent publish is no longer expressible, which is why the old test
//! here is gone — but not because the signature fends off a race.** `advance`
//! takes `&mut self` because a tick assigns `self.player`, a plain field sitting
//! *beside* the `ArcSwap` rather than inside it, and an `&self` method cannot
//! assign to it at all; the signature follows from where the state lives. The
//! lost update it looks like it is guarding against is unreachable anyway:
//! `Solidity` declares no `Sync` supertrait, so `Simulation` is `Send` but *not*
//! `Sync` and no two threads can hold `&Simulation` at all. Either fact on its
//! own sinks the old test, which held `&Simulation` on one thread while another
//! published. "Publishing never waits on a reader" survives as a property of
//! publication and stops being observable until a second thread reads a snapshot,
//! which is a deferred decision rather than this feature's. `latest(&self)` is
//! unchanged, so no reader is affected.
//!
//! What remains is the other half of the same guarantee: holding a snapshot is
//! only useful if nothing can rewrite it underneath the holder. It compares the
//! held snapshot against a second simulation advanced to the same tick rather
//! than against a copy of itself, so a snapshot quietly rewritten to the *newer*
//! tick's contents fails rather than matching its own new ones.
//!
//! **The tick counter is free-running, and the run below is deliberately longer
//! than the script it used to wrap at.** 120 is the length of the declared
//! *intent script*, not a period of the simulation: a windowed client runs for as
//! long as its window is open, and a tick that restarted at 0 would republish an
//! old tick number to everything downstream that reads one.

mod support;

use glam::Vec3;
use mc_sim::player::{MovementIntent, PlayerState};
use mc_sim::replay::SCRIPT_TICKS;
use mc_sim::simulation::Simulation;

use support::solidity::Ground;
use support::{TestResult, exactly, exactly_player};

/// The tick the reader is holding while the next one is published.
const HELD_TICK: u32 = 60;

/// How many intents are submitted one after another.
///
/// Past the declared script's length on purpose: every tick inside it is a tick
/// the old wrapping counter also numbered correctly, so a run that stopped at
/// the end of the script could not tell a free-running counter from one that
/// restarts.
const SUBMISSIONS: u32 = SCRIPT_TICKS + 5;

/// The declared world these ticks run over: a floor, and a player standing on
/// it. Nothing here is about the terrain — it is about what publication does
/// with whatever the tick produced.
const FLOOR: i32 = 40;

/// Where the player stands. The two horizontal coordinates differ so that a
/// snapshot rewritten from a transposed state is not equal by coincidence.
const FEET: Vec3 = Vec3::new(10.5, 41.0, 3.5);

#[test]
fn every_submitted_intent_publishes_the_tick_after_the_one_before_it() -> TestResult {
    let mut simulation = Simulation::new(standing(), Box::new(Ground::Flat { surface: FLOOR }));
    let mut published = vec![simulation.latest().tick];

    for _ in 0..SUBMISSIONS {
        simulation.advance(MovementIntent::default());
        published.push(simulation.latest().tick);
    }

    let broken: Vec<(u32, u32)> = published
        .iter()
        .zip(published.iter().skip(1))
        .filter(|(before, after)| **after != **before + 1)
        .map(|(before, after)| (*before, *after))
        .collect();
    assert!(
        broken.is_empty(),
        "submitting an intent publishes a snapshot one tick past the last one published, at \
         every one of {SUBMISSIONS} submissions — these pairs are the published ticks before \
         and after a submission that did something else: {broken:?}. A pair ending at 0 is a \
         counter still wrapping at the script's {SCRIPT_TICKS} ticks, and a pair that stood \
         still is a tick nothing advanced"
    );
    Ok(())
}

/// This test carries a second guarantee that no assertion in it mentions, and the
/// mechanism is the *order* of the two lines below, not the comparison at the end.
///
/// Taking `held` before the advance and reading it after means a snapshot outlives
/// a `&mut self` call on the simulation it came from. The borrow checker permits
/// that only while `Simulation::latest` returns an owned `Arc`, rather than a guard
/// borrowing `&self`. So "publishing never waits on a reader" — a reader holds
/// nothing a publisher could be waiting on — is pinned here by this test compiling
/// at all. Change `latest` to hand back a guard and this file stops building.
///
/// **Do not simplify this by fetching the snapshot after the advance.** Both
/// assertions can be made to pass that way, and the guarantee disappears with no
/// test going red. There was a direct concurrent-publish test; `&mut self` on
/// `advance` makes its shape inexpressible, so this is what carries the property
/// in its place.
#[test]
fn a_later_publish_leaves_the_snapshot_the_renderer_holds_unchanged() -> TestResult {
    let mut simulation = advanced_to(HELD_TICK);
    let held = simulation.latest();
    let independent = advanced_to(HELD_TICK);

    simulation.advance(MovementIntent::default());

    assert_eq!(
        simulation.latest().tick,
        HELD_TICK + 1,
        "a later publish has to have happened, or nothing could have disturbed the held \
         snapshot in the first place"
    );
    assert_eq!(
        (
            held.tick,
            exactly(&held.camera),
            exactly_player(&held.player)
        ),
        (
            HELD_TICK,
            exactly(&independent.latest().camera),
            exactly_player(&independent.latest().player)
        ),
        "the held snapshot still describes its own tick after a later one was published"
    );
    Ok(())
}

/// A simulation that has been advanced `tick` times from its start.
fn advanced_to(tick: u32) -> Simulation {
    let mut simulation = Simulation::new(standing(), Box::new(Ground::Flat { surface: FLOOR }));
    for _ in 0..tick {
        simulation.advance(MovementIntent::default());
    }
    simulation
}

/// A player standing still on the declared floor.
fn standing() -> PlayerState {
    PlayerState {
        position: FEET,
        velocity: Vec3::ZERO,
        yaw: 0.0,
        pitch: 0.0,
        on_ground: true,
    }
}
