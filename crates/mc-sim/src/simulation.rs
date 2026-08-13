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
//! **`advance` takes `&mut self` because of what a tick mutates, not to fend off
//! a race.** A tick assigns `self.player` — a plain field sitting *beside* the
//! `ArcSwap` rather than inside it — and now also edits the world, and an
//! `&self` method can do neither. `arc_swap` does offer `rcu` on `&self`; what
//! rules it out is its retry semantics, since a tick's effect is not confined to
//! the swapped cell and a re-run closure would step the player, and break a
//! block, more than once for one tick number.
//!
//! **The `!Sync` half of that argument no longer holds, and the conclusion
//! survives without it.** The world used to be a `Box<dyn Solidity + Send>`,
//! which left this struct `Send` but not `Sync`, so no two threads could hold
//! `&Simulation` at all. It is a concrete `World` now — a block store, a bitset
//! and an `Arc<BlockRegistry>`, all `Sync` — so `Simulation` is `Send + Sync`
//! and the borrow is doing real work again. Read the `&mut` as recording that a
//! tick mutates state outside the published cell. `latest` still takes `&self`,
//! so readers are unaffected.
//!
//! **The tick counter is free-running and never wraps.** 120 is the length of the
//! declared *intent script*, not a period of the simulation: a windowed client
//! runs for as long as its window is open, and a counter that restarted would
//! republish an old tick number to everything downstream that reads one.

use std::fmt;
use std::sync::Arc;

use arc_swap::ArcSwap;

use crate::camera::CameraPose;
use crate::player::{PlayerState, advance_player, eye_pose};
use crate::world::action::{EditReport, TickIntent, resolve};
use crate::world::{RemeshWork, World};

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
    /// The world a tick resolves the player's motion against, and edits.
    ///
    /// Concrete and not a trait object. A trait object would re-open the hole
    /// [`World`] closes: any implementor could be *asked* to keep its block
    /// store and its collision view in agreement, and that is precisely the
    /// thing that must not be askable.
    world: World,
}

impl Simulation {
    /// A simulation of `world`, with the player at `spawn` and its first
    /// snapshot already published.
    ///
    /// The spawn's own snapshot exists before any intent is submitted, because
    /// the state before the first tick is a state a reader can be shown — the
    /// frame drawn while nothing has been asked for yet is drawn from it.
    #[must_use]
    pub fn new(spawn: PlayerState, world: World) -> Self {
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
    /// **Movement and look are applied first, and the action is resolved against
    /// the state the tick ends with.** So the ray is cast from the eye the tick
    /// arrived at, along the orientation it has already turned and limited, and
    /// the snapshot this publishes shows the edit and the view that produced it
    /// together.
    ///
    /// The camera is derived from the player the tick produced rather than
    /// carried beside it, so there is nothing to keep in step: an eye that moved
    /// is an eye whose player moved.
    ///
    /// Returns nothing when the tick asked for no action — which is almost every
    /// tick, and is not a refusal. Deliberately not `#[must_use]`: every caller
    /// that only walks the player ignores it.
    pub fn advance(&mut self, intent: impl Into<TickIntent>) -> Option<EditReport> {
        let intent = intent.into();
        self.player = advance_player(self.player, &intent.movement, &self.world);
        let report = intent
            .action
            .as_ref()
            .map(|action| resolve(action, &self.player, &mut self.world));
        self.published.store(Arc::new(SimSnapshot {
            tick: self.published.load().tick.saturating_add(1),
            camera: eye_pose(&self.player),
            player: self.player,
        }));
        report
    }

    /// What has to be re-meshed for this simulation's edits to be seen, or
    /// nothing when there have been none since it was last asked.
    ///
    /// **An owned, `Send` batch and never a borrow**, which is what lets a
    /// re-mesh run on a thread of its own without pinning the tick behind it —
    /// the same property the session that owns this simulation already has.
    pub fn take_remesh_work(&mut self) -> Option<RemeshWork> {
        self.world.take_remesh_work()
    }

    /// Whatever was published most recently.
    #[must_use]
    pub fn latest(&self) -> Arc<SimSnapshot> {
        self.published.load_full()
    }

    /// The world this simulation owns.
    ///
    /// Read-only, and that is a property of the type rather than of this
    /// signature: the one function that writes anything is private to
    /// `mc_sim::world`, so a `&World` is a view and can be nothing else.
    #[must_use]
    pub const fn world(&self) -> &World {
        &self.world
    }
}

/// A world's blocks would bury whatever a panic message was about, so what is
/// shown is what a reader of one can use: which tick was published and where the
/// player is.
impl fmt::Debug for Simulation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Simulation")
            .field("published", &self.published.load())
            .field("player", &self.player)
            .finish_non_exhaustive()
    }
}
