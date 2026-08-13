//! Headless-capable simulation core: ECS schedule, physics, inventory, crafting, NPC runtime, quests. This is the server.
//!
//! What lives here today is the scripted replay the renderer is verified
//! against: a fixed world generated from a fixed seed, a camera path that is a
//! total function of the tick index, and the publication seam a renderer reads
//! through. None of it knows what a vertex is — the quad to vertex conversion
//! belongs to `mc-render`, and the two crates are asserted never to resolve each
//! other.

pub mod camera;
pub mod persistence;
pub mod player;
pub mod replay;
pub mod simulation;
pub mod world;

/// The vocabulary a client fills in to ask for an edit, and what the server
/// answers with.
///
/// A curated re-export and not the module itself: [`world::World`]'s write is
/// private to its own module tree, and what belongs out here is the vocabulary,
/// not the door.
pub mod action {
    pub use crate::world::action::{
        ActionIntent, EditReport, Hit, REACH, Refusal, TickIntent, default_held_block, targeted,
    };
}

/// The seed the replay world is generated from.
///
/// A world that is a pure function of this number is what makes a committed
/// golden frame mean anything: everything the camera sees has to be the same on
/// every machine, on every run.
pub const REPLAY_SEED: u64 = 0x4D79_4372_6166_7431;
