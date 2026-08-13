//! Publishing what the simulation is at, without waiting on whoever is reading
//! it.
//!
//! Reading a stale snapshot is correct; stalling the simulation to serve a
//! reader is not. Publication is therefore a pointer swap: a reader holds an
//! `Arc` of whatever was current when it looked, and the next publish replaces
//! the pointer rather than the contents.
//!
//! **That the held snapshot cannot change underneath its holder is a property of
//! the type, not of a compiler error.** It holds a tick, a pose and a player
//! state, all plain values, and it must stay that way — a field carrying a
//! `Mutex` or an `AtomicU32` would silently reopen the hole while every test here
//! still passed.
//!
//! **`advance` takes `&mut self` because of where the player's state lives, not
//! to fend off a race.** A tick assigns `self.player`, a plain field sitting
//! *beside* the `ArcSwap` rather than inside it, and an `&self` method cannot
//! assign to it at all. `arc_swap` does offer `rcu` on `&self`; what rules it
//! out is its retry semantics, since a tick's effect is not confined to the
//! swapped cell and a re-run closure would step the player more than once for
//! one tick number. Nor is the exclusive borrow guarding a reachable lost
//! update: `world: Box<dyn Solidity + Send>` leaves this struct `Send` but
//! *not* `Sync`, so no two threads can hold `&Simulation` to race through in
//! the first place. Read the `&mut` as recording that a tick mutates state
//! outside the published cell. `latest` still takes `&self`, so readers are
//! unaffected.
//!
//! **The tick counter is free-running and never wraps.** 120 is the length of the
//! declared *intent script*, not a period of the simulation: a windowed client
//! runs for as long as its window is open, and a counter that restarted would
//! republish an old tick number to everything downstream that reads one.

use std::fmt;
use std::sync::Arc;

use arc_swap::ArcSwap;

use crate::camera::CameraPose;
use crate::player::{MovementIntent, PlayerState, Solidity, advance_player, eye_pose};

/// What the simulation publishes: which tick it is at, where the camera is, and
/// everything it knows about the player.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SimSnapshot {
    pub tick: u32,
    pub camera: CameraPose,
    pub player: PlayerState,
}

/// The tick a simulation publishes before any intent has been submitted.
const FIRST_TICK: u32 = 0;

/// The simulation's own state, and the snapshot it publishes.
pub struct Simulation {
    published: ArcSwap<SimSnapshot>,
    player: PlayerState,
    /// The world a tick resolves the player's motion against.
    world: Box<dyn Solidity + Send>,
}

impl Simulation {
    /// A simulation of `world`, with the player at `spawn` and its first
    /// snapshot already published.
    ///
    /// The spawn's own snapshot exists before any intent is submitted, because
    /// the state before the first tick is a state a reader can be shown — the
    /// frame drawn while nothing has been asked for yet is drawn from it.
    #[must_use]
    pub fn new(spawn: PlayerState, world: Box<dyn Solidity + Send>) -> Self {
        Self {
            published: ArcSwap::from_pointee(SimSnapshot {
                tick: FIRST_TICK,
                camera: eye_pose(&spawn),
                player: spawn,
            }),
            player: spawn,
            world,
        }
    }

    /// Advances one tick under `intent` and publishes the result.
    ///
    /// The camera is derived from the player the tick produced rather than
    /// carried beside it, so there is nothing to keep in step: an eye that moved
    /// is an eye whose player moved.
    pub fn advance(&mut self, intent: MovementIntent) {
        self.player = advance_player(self.player, &intent, self.world.as_ref());
        self.published.store(Arc::new(SimSnapshot {
            tick: self.published.load().tick.saturating_add(1),
            camera: eye_pose(&self.player),
            player: self.player,
        }));
    }

    /// Whatever was published most recently.
    #[must_use]
    pub fn latest(&self) -> Arc<SimSnapshot> {
        self.published.load_full()
    }
}

/// The world is a trait object with no `Debug` of its own, so what is shown is
/// what a reader of a panic message can use: which tick was published and where
/// the player is.
impl fmt::Debug for Simulation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Simulation")
            .field("published", &self.published.load())
            .field("player", &self.player)
            .finish_non_exhaustive()
    }
}
