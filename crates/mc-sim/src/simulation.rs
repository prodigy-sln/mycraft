//! Publishing what the replay is at, without waiting on whoever is reading it.
//!
//! Reading a stale snapshot is correct; stalling the simulation to serve a
//! reader is not. Publication is therefore a pointer swap: a reader holds an
//! `Arc` of whatever was current when it looked, and the next publish replaces
//! the pointer rather than the contents.
//!
//! **That the held snapshot cannot change underneath its holder is a property of
//! the type, not of a compiler error.** It holds a tick and a pose, both plain
//! values, and it must stay that way — a field carrying a `Mutex` or an
//! `AtomicU32` would silently reopen the hole while every test here still
//! passed.

use std::sync::Arc;

use arc_swap::ArcSwap;

use crate::TICK_COUNT;
use crate::replay::{CameraPose, TickIndex, pose};

/// What the simulation publishes: which tick it is at, and where the camera is.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SimSnapshot {
    pub tick: u32,
    pub camera: CameraPose,
}

impl SimSnapshot {
    /// The snapshot describing `tick`.
    fn at(tick: TickIndex) -> Self {
        Self {
            tick: tick.get(),
            camera: pose(tick),
        }
    }
}

/// The replay's tick counter, and the snapshot it publishes.
#[derive(Debug)]
pub struct Simulation {
    published: ArcSwap<SimSnapshot>,
}

impl Simulation {
    /// A simulation at the replay's first tick.
    #[must_use]
    pub fn new() -> Self {
        Self {
            published: ArcSwap::from_pointee(SimSnapshot::at(TickIndex::FIRST)),
        }
    }

    /// Advances one tick and publishes the result.
    ///
    /// Takes `&self` rather than `&mut self` on purpose: a publisher that needed
    /// exclusive access would make "publish while a reader holds a snapshot"
    /// inexpressible instead of merely correct.
    pub fn advance(&self) {
        let next = following(self.published.load().tick);
        self.published.store(Arc::new(SimSnapshot::at(next)));
    }

    /// Whatever was published most recently.
    #[must_use]
    pub fn latest(&self) -> Arc<SimSnapshot> {
        self.published.load_full()
    }
}

impl Default for Simulation {
    fn default() -> Self {
        Self::new()
    }
}

/// The tick after `current`, wrapping at the replay's end.
///
/// Returns a [`TickIndex`] rather than a number, so nothing downstream has to
/// re-check the bound. The wrap keeps it inside the replay by construction, and
/// the fallback below is therefore unreachable — it is the replay's first tick
/// rather than a panic, because a panic in the tick loop is the one failure this
/// project does not accept.
fn following(current: u32) -> TickIndex {
    let next = current + 1;
    let inside = if next < TICK_COUNT { next } else { 0 };
    TickIndex::new(inside).unwrap_or(TickIndex::FIRST)
}
